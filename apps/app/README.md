# YMCL (YuDream Launcher)

YMCL is the Tauri desktop shell for the YMCL Minecraft launcher.

Run `pnpm install` and `pnpm app:dev` from the workspace root to start a development build.

## Windows portable edition

Download `YMCL_<version>_x64_portable.zip`, extract the **whole** `YMCL` folder to
a writable USB drive, and run `YMCL.exe`. The launcher, games and downloaded
Java runtimes run from that folder; no launcher installation is needed.

```text
YMCL/
  YMCL.exe
  portable.txt
  resources/blockbench-skin/
  WebView2/
  .Axolotl/
    release/ or beta/     app.db and account/settings data
    profiles/            instances, mods, saves and game configuration
    meta/                game assets, libraries and managed Java runtimes
    Backups/             default instance backup repository
    webview/             WebView2 user data
```

`portable.txt` activates portable mode and creates `.Axolotl` on first use. An
existing `.Axolotl` directory also activates it, including copies made with
older releases. A write failure stops startup with an error instead of opening
an unrelated system data directory. The data root is fixed in portable mode.
Each portable folder has its own single-instance identifier, so launching it
does not hand control to an already running installed copy or another folder.

Paths to files inside this folder are stored relative to the executable's
directory and resolved at runtime, independent of the process working directory.
This follows HMCL's [local-directory model](https://github.com/HMCL-dev/HMCL/blob/v3.16.4/HMCL/src/main/java/org/jackhuang/hmcl/Metadata.java)
and [relative-path storage](https://github.com/HMCL-dev/HMCL/blob/v3.16.4/HMCLCore/src/main/java/org/jackhuang/hmcl/util/PortablePath.java).
Changing the USB drive letter does not require editing settings or reimporting
managed instances. Automatic Java selection uses runtimes inside the portable
folder, and missing runtimes are downloaded into `.Axolotl/meta/java_versions`.
Initial game/Java downloads require an internet connection. Direct associations
with external launchers, filesystem links, explicit Java overrides, hooks and
other custom paths can still depend on files outside the USB drive.
Remembered Yggdrasil passwords and AI provider credentials use the operating
system's credential store; they remain on each computer and may need to be
entered again. Minecraft account records remain in the portable database.

The ZIP includes Microsoft's fixed WebView2 runtime, so a target computer does
not need a separate WebView2 installation. Runtime data is stored in
`.Axolotl/webview`. The packager downloads the runtime from Microsoft's official
manifest and validates its Microsoft Authenticode signature. Fixed runtimes are
updated by replacing the portable program files with a newer release.
On NTFS volumes, startup grants the runtime directory the inherited
AppContainer read/execute permissions required by [Microsoft's Windows 10
deployment instructions](https://learn.microsoft.com/microsoft-edge/webview2/concepts/distribution).
FAT/exFAT volumes have no persistent ACLs to configure.

Close both Minecraft and YMCL before safely ejecting the USB drive. For updates,
exit YMCL, replace `YMCL.exe`, `resources/`, `WebView2/` and the marker from the new
ZIP, and **retain the existing `.Axolotl/` directory**. Portable mode disables the
installer updater to preserve this layout.

Build the full portable ZIP on Windows x64 with `pnpm app:build:portable`. It is
written to `target/release/bundle/portable/`. To package an already built release
binary, run `node scripts/axolotl/package-portable.mjs`; use `--runtime <directory>`
to supply an extracted, Microsoft-signed fixed runtime locally. Release CI reuses
the same application executable as the installers and packages resources and the
runtime without a second Rust build.

## Building efficiently

- `pnpm app:build` produces the normal optimized release build and installers.
- `pnpm app:prepare` prefetches locked Rust dependencies for the host and prepares the Java agents and Blockbench resources in parallel. Run it ahead of a release build; for macOS universal builds, use `pnpm app:prepare aarch64-apple-darwin x86_64-apple-darwin`.
- `pnpm app:build:debug` builds a standalone debug executable without installers or signing. Use it for local iteration when release optimization and packaging are unnecessary; the executable is in `target/debug/`.
- `pnpm exec turbo run build --filter=@modrinth/app-frontend` builds and caches the frontend, including its existing type-check step. Repeated unchanged builds restore `dist/` from the cache. Tauri already uses this command internally.
- Contribution counts are checked-in build inputs. Refresh them explicitly with `pnpm --filter @modrinth/app-frontend contributors:sync`; normal builds do not need GitHub access or rewrite source files.

Development builds retain filename/line-number backtraces with reduced debug information. For full debugger variable inspection, set `CARGO_PROFILE_DEV_DEBUG=2`. Local release builds retain incremental compiler state; CI sets `CARGO_INCREMENTAL=0` and published releases retain full LTO. The first build after changing compiler profiles will rebuild dependencies once.

The C seed-map engine lives in `packages/seed-map-native` and uses Cargo's jobserver for parallel compilation. Tauri asset/configuration changes no longer recompile its C sources. Initialize the pinned submodules with `git submodule update --init --recursive --depth 1` before a first desktop build or resource preparation.

Normal Tauri builds also prepare shared resources automatically. Java agents live in `packages/app-lib/java/build/libs/` instead of per-profile Cargo output directories. Both the Java and Blockbench caches validate source and output contents, so changing profiles, targets, or checkout timestamps does not rebuild unchanged resources. Missing or modified outputs are rebuilt; npm still verifies the lockfile and Gradle still runs the Java checks on a cache miss. The web-only Blockbench build skips the unused Electron binary download, but retains dependency install scripts such as esbuild's. Node.js is required for direct Cargo builds of `theseus` as well as desktop builds.

### Release CI caches

Release jobs restore pnpm/npm packages, Cargo dependencies, Gradle/JDK downloads, Java agents, Blockbench resources, frontend build results, Linux package downloads, and Tauri packaging tools. Linux package installation and signature/checksum verification are retained. Build checkouts are shallow; only release preflight fetches history for its guardrails.

`Prepare release caches` builds the same full-LTO profiles and platform matrix on `main` when dependencies or build configuration change. It warms installer tools without signing or publishing. Published jobs reuse these dependency artifacts, but never cache workspace Rust outputs that may embed release secrets. Windows installer variants still reuse one compiled application binary.

The first cold build still downloads and compiles dependencies: prewarming moves that work ahead of the release, at the cost of an extra CI build when its inputs change. To prepare a release after a long idle period or an evicted cache, run `Prepare release caches` manually on `main` and wait for it to finish before tagging. GitHub allows tags to restore default-branch caches, but not caches created by other tags. Cargo timing reports are written to `target/cargo-timings/`; compare real release runs before claiming an end-to-end speedup.

For Windows distribution variants, build the modern installer once and then use `tauri bundle --config tauri-release.conf.json --config tauri-native.conf.json`. Both installers reuse the same updater-enabled executable; do not change compile-time features or application configuration between build and bundle. Published workflows keep release signing and updater verification enabled.
