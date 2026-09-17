import { spawnSync } from 'node:child_process'
import { mkdirSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

// The tauri CLI sets TAURI_CONFIG for its cargo run, while plain cargo
// (check/clippy/rust-analyzer) leaves it unset — tauri-build declares
// rerun-if-env-changed on it, so the two environments invalidate each other's
// fingerprints and every switch rebuilds the tauri plugin chain plus theseus.
// Keeping dev output in its own target tree makes the two caches independent.
const scriptDir = dirname(fileURLToPath(import.meta.url))
process.env.CARGO_TARGET_DIR ??= resolve(scriptDir, '../../target-dev')

// `--test-data[=<dir>]` launches an isolated test instance: the data location
// is pointed at `<repo>/.test-data` (or the given dir) via THESEUS_CONFIG_DIR,
// so the launcher keeps its database, instances and caches there instead of
// the installed launcher's data.
const tauriArgs = ['dev']
const testDataArg = process.argv
	.slice(2)
	.find((arg) => arg === '--test-data' || arg.startsWith('--test-data='))
if (testDataArg) {
	const customDir = testDataArg.slice('--test-data'.length).replace(/^=/, '')
	const dataDir = resolve(customDir || resolve(scriptDir, '../../.test-data'))
	mkdirSync(dataDir, { recursive: true })
	process.env.THESEUS_CONFIG_DIR = dataDir
	// Full isolation from a concurrently running `app:dev`: own build tree (a
	// relink would fight the exe the other instance has running), own vite
	// port, own app identifier (bypasses the single-instance mutex) and own
	// window title — the latter three come via tauri.dev-test.conf.json.
	process.env.CARGO_TARGET_DIR = resolve(scriptDir, '../../target-dev-test')
	process.env.YMCL_DEV_PORT = '5202'
	tauriArgs.push('--config', resolve(scriptDir, 'tauri.dev-test.conf.json'))
	console.log(
		`[dev:test] data=${dataDir} port=5202 target=target-dev-test identifier=red.ghs.axolotl.test`,
	)
}

const result = spawnSync('tauri', tauriArgs, {
	stdio: 'inherit',
	shell: process.platform === 'win32',
})
process.exit(result.status ?? 1)
