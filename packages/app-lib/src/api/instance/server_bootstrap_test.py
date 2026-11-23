import hashlib
import importlib.util
import io
import tempfile
from pathlib import Path
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "server_bootstrap", Path(__file__).with_name("server_bootstrap.py")
)
bootstrap = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bootstrap)


class Response(io.BytesIO):
    def geturl(self):
        return "https://cdn.example.test/mod.jar"


class ServerBootstrapTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root_patch = patch.object(bootstrap, "ROOT", Path(self.temp.name))
        self.root_patch.start()
        self.addCleanup(self.root_patch.stop)

    def record(self, data=b"mod jar", downloads=None):
        return {
            "path": "mods/example.jar",
            "size": len(data),
            "hashes": {"sha256": hashlib.sha256(data).hexdigest()},
            "downloads": downloads if downloads is not None else ["https://cdn.example.test/mod.jar"],
        }

    def manifest(self, record):
        return {"format": "ymcl-server-pack", "formatVersion": 1, "files": [record]}

    def test_download_is_verified_and_second_launch_uses_cached_mod(self):
        record = self.record()
        with patch.object(bootstrap.urllib.request, "urlopen", return_value=Response(b"mod jar")) as request:
            bootstrap.prepare(self.manifest(record))
            bootstrap.prepare(self.manifest(record))
        self.assertEqual(request.call_count, 1)
        self.assertEqual((bootstrap.ROOT / record["path"]).read_bytes(), b"mod jar")
        self.assertEqual(list(bootstrap.ROOT.rglob(".ymcl-download-*")), [])

    def test_hash_failure_preserves_existing_mod_and_cleans_temporary_file(self):
        path = bootstrap.ROOT / "mods/example.jar"
        path.parent.mkdir()
        path.write_bytes(b"old mod")
        with patch.object(bootstrap.urllib.request, "urlopen", return_value=Response(b"bad jar")):
            with self.assertRaisesRegex(RuntimeError, "hash or size"):
                bootstrap.prepare(self.manifest(self.record()))
        self.assertEqual(path.read_bytes(), b"old mod")
        self.assertEqual(list(bootstrap.ROOT.rglob(".ymcl-download-*")), [])

    def test_falls_back_to_another_download_url(self):
        record = self.record(downloads=["https://first.test/mod.jar", "https://second.test/mod.jar"])
        with patch.object(bootstrap.urllib.request, "urlopen", side_effect=[OSError("offline"), Response(b"mod jar")]) as request:
            bootstrap.prepare(self.manifest(record))
        self.assertEqual(request.call_count, 2)
        self.assertEqual((bootstrap.ROOT / record["path"]).read_bytes(), b"mod jar")

    def test_rejects_traversal_symlinks_and_insecure_downloads(self):
        for path in ["../escape.jar", "/tmp/escape.jar", "mods/../../escape.jar", "C:/escape.jar", "mods\\escape.jar", "./mods/a.jar"]:
            with self.assertRaises(ValueError, msg=path):
                bootstrap.safe_path(path)
        with tempfile.TemporaryDirectory() as outside:
            (bootstrap.ROOT / "mods").symlink_to(outside, target_is_directory=True)
            with self.assertRaisesRegex(ValueError, "directory|Symlink"):
                bootstrap.safe_path("mods/escape.jar")
        (bootstrap.ROOT / "mods").unlink()
        record = self.record(downloads=["http://insecure.test/mod.jar"])
        with patch.object(bootstrap.urllib.request, "urlopen") as request:
            with self.assertRaisesRegex(RuntimeError, "HTTPS"):
                bootstrap.prepare(self.manifest(record))
        request.assert_not_called()

    def test_preserves_edited_config_and_verifies_bundled_mods(self):
        config = bootstrap.ROOT / "config/server.toml"
        config.parent.mkdir()
        config.write_bytes(b"edited config")
        record = self.record(downloads=[])
        record["path"] = "config/server.toml"
        bootstrap.prepare(self.manifest(record))
        self.assertEqual(config.read_bytes(), b"edited config")
        with self.assertRaisesRegex(ValueError, "bundled mod"):
            bootstrap.prepare(self.manifest(self.record(downloads=[])))

    def test_rejects_oversized_downloads_before_replacing_any_mod(self):
        with patch.object(bootstrap.urllib.request, "urlopen", return_value=Response(b"too many bytes")):
            with self.assertRaisesRegex(RuntimeError, "expected size"):
                bootstrap.prepare(self.manifest(self.record()))
        self.assertFalse((bootstrap.ROOT / "mods/example.jar").exists())


if __name__ == "__main__":
    unittest.main()
