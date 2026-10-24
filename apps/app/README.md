# YMCL (YuDream Launcher)

YMCL is the Tauri desktop shell for the YMCL Minecraft launcher.

Run `pnpm install` and `pnpm app:dev` from the workspace root to start a development build.

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
