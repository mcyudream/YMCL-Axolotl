#!/usr/bin/env python3
"""Prepare an exported YMCL server pack, then start its installed server."""
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import subprocess
import sys
import tempfile
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parent
MANIFEST = "ymcl-server-manifest.json"


def safe_path(relative):
    parts = PurePosixPath(relative)
    if (
        not relative
        or "\\" in relative
        or ":" in relative
        or parts.is_absolute()
        or ".." in parts.parts
        or parts.as_posix() != relative
    ):
        raise ValueError("Unsafe manifest path: " + relative)
    target = ROOT.joinpath(*parts.parts)
    if not target.resolve().is_relative_to(ROOT):
        raise ValueError("Path leaves the server directory: " + relative)
    for parent in [target, *target.parents]:
        if parent == ROOT:
            break
        if parent.is_symlink():
            raise ValueError("Symlink in managed path: " + relative)
    return target


def matches(path, record):
    if not path.is_file() or path.stat().st_size != record["size"]:
        return False
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(256 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest() == record["hashes"]["sha256"]


def prepare(manifest):
    if manifest.get("format") != "ymcl-server-pack" or manifest.get("formatVersion") != 1:
        raise ValueError("Unsupported YMCL server pack manifest")
    for record in manifest["files"]:
        target = safe_path(record["path"])
        urls = record.get("downloads", [])
        if not urls:
            # Configurations may be edited after extraction; bundled mods must
            # still match the exported pack before the server is launched.
            if record["path"].startswith("mods/") and not matches(target, record):
                raise ValueError("Missing or modified bundled mod: " + record["path"])
            continue
        if not record["path"].startswith("mods/") or not record["path"].endswith(".jar"):
            raise ValueError("Only mod JARs may be downloaded by this bootstrap")
        if matches(target, record):
            continue
        target.parent.mkdir(parents=True, exist_ok=True)
        errors = []
        for url in urls:
            temporary = None
            try:
                if urllib.parse.urlparse(url).scheme != "https":
                    raise ValueError("Mod downloads require HTTPS")
                request = urllib.request.Request(url, headers={"User-Agent": "YMCL-Server-Pack/1"})
                descriptor, temporary = tempfile.mkstemp(prefix=".ymcl-download-", dir=target.parent)
                with os.fdopen(descriptor, "wb") as output, urllib.request.urlopen(request, timeout=60) as response:
                    if urllib.parse.urlparse(response.geturl()).scheme != "https":
                        raise ValueError("Download redirected to an insecure URL")
                    size = 0
                    for chunk in iter(lambda: response.read(256 * 1024), b""):
                        size += len(chunk)
                        if size > record["size"]:
                            raise ValueError("Download exceeds expected size")
                        output.write(chunk)
                if not matches(Path(temporary), record):
                    raise ValueError("Download hash or size does not match")
                os.replace(temporary, target)
                print("Downloaded " + record["path"], flush=True)
                break
            except Exception as error:
                errors.append(str(error))
            finally:
                if temporary and Path(temporary).exists():
                    Path(temporary).unlink()
        else:
            raise RuntimeError("Could not download " + record["path"] + ": " + "; ".join(errors))


def launch(manifest, arguments):
    run_script = ROOT / ("run.bat" if os.name == "nt" else "run.sh")
    if run_script.is_file():
        command = ["cmd", "/c", str(run_script)] if os.name == "nt" else ["sh", str(run_script)]
    else:
        jars = [ROOT / name for name in ["fabric-server-launch.jar", "quilt-server-launch.jar", "server.jar"]]
        jars.extend(sorted(ROOT.glob("fabric-server-mc.*-launcher.*.jar")))
        jar = next((path for path in jars if path.is_file()), None)
        if jar is None:
            minecraft = manifest["minecraft"]
            print(
                "Mods are ready. Install the dedicated server for Minecraft "
                + minecraft["version"] + " / " + minecraft["loader"]
                + " " + (minecraft.get("loaderVersion") or "")
                + " in this directory, then run start-server again.\n"
                + "Mod 文件已准备好。请在此目录安装对应版本的服务端核心后重新运行启动脚本。",
                flush=True,
            )
            return 1
        command = ["java", "-jar", str(jar), "nogui"]
    return subprocess.call(command + arguments, cwd=ROOT)


def main():
    if sys.version_info < (3, 9):
        raise RuntimeError("Python 3.9 or newer is required")
    manifest = json.loads((ROOT / MANIFEST).read_text(encoding="utf-8"))
    prepare(manifest)
    if "--prepare-only" in sys.argv[1:]:
        return 0
    return launch(manifest, sys.argv[1:])


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as error:
        print("Server preparation failed: " + str(error), file=sys.stderr)
        sys.exit(1)
