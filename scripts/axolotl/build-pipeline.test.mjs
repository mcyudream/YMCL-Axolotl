import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdtemp, mkdir, readFile, rm, stat, utimes, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test } from 'node:test'
import { parse } from 'yaml'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..')
const pnpm = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm'

function buildPlan(packageName, environment = {}) {
	const env = { ...process.env, TURBO_TELEMETRY_DISABLED: '1' }
	delete env.TAURI_ENV_PLATFORM
	delete env.TAURI_ENV_DEBUG
	return JSON.parse(
		execFileSync(pnpm, ['exec', 'turbo', 'run', 'build', `--filter=${packageName}`, '--dry=json'], {
			cwd: root,
			env: { ...env, ...environment },
			encoding: 'utf8',
			maxBuffer: 16 * 1024 * 1024,
			timeout: 30_000,
			shell: process.platform === 'win32',
			stdio: ['ignore', 'pipe', 'pipe'],
		}),
	).tasks
}

test('desktop packaging does not prebuild app-lib with a second Cargo feature graph', () => {
	const tasks = buildPlan('@modrinth/app')
	assert.deepEqual(
		tasks.map((task) => task.taskId),
		['@modrinth/app#build'],
	)
	const definition = tasks[0].resolvedTaskDefinition
	assert.equal(definition.cache, false)
	for (const pattern of [
		'CARGO_*',
		'RUST*',
		'SCCACHE_*',
		'TAURI_*',
		'APPLE_*',
		'JAVA_HOME',
		'GRADLE_USER_HOME',
		'NPM_CONFIG_*',
	]) {
		assert.ok(definition.passThroughEnv.includes(pattern), `missing ${pattern}`)
	}
})

test('frontend cache separates platform targets and debug/release builds', () => {
	const frontend = (environment) =>
		buildPlan('@modrinth/app-frontend', environment).find(
			(task) => task.taskId === '@modrinth/app-frontend#build',
		)
	const linux = frontend({ TAURI_ENV_PLATFORM: 'linux' })
	const windows = frontend({ TAURI_ENV_PLATFORM: 'windows' })
	const debug = frontend({ TAURI_ENV_PLATFORM: 'linux', TAURI_ENV_DEBUG: 'true' })
	assert.notEqual(linux.hash, windows.hash)
	assert.notEqual(linux.hash, debug.hash)
	assert.ok(Object.keys(linux.inputs).some((path) => path.endsWith('/app/tauri.conf.json')))
	assert.ok(linux.dependencies.includes('@modrinth/ui#build'))
	assert.ok(linux.dependencies.includes('@modrinth/api-client#build'))
	assert.deepEqual(linux.resolvedTaskDefinition.outputs, ['dist/**'])
})

test('normal frontend builds remain checked and do not refresh remote source data', async () => {
	const { scripts } = JSON.parse(
		await readFile(join(root, 'apps/app-frontend/package.json'), 'utf8'),
	)
	assert.match(scripts.build, /vue-tsc --noEmit/)
	assert.match(scripts.build, /vite build/)
	assert.doesNotMatch(scripts.build, /contributors:sync/)
	assert.match(scripts['contributors:sync'], /sync-contributors\.mjs/)
})

test('Windows installer variants reuse a binary without weakening release optimization', async () => {
	const ci = parse(await readFile(join(root, '.github/workflows/axolotl-ci.yml'), 'utf8'))
	const release = parse(await readFile(join(root, '.github/workflows/axolotl-release.yml'), 'utf8'))
	for (const job of [ci.jobs.desktop, release.jobs.build]) {
		const commands = job.steps.map((step) => step.run ?? '').join('\n')
		assert.match(
			commands,
			/tauri bundle --config tauri-release\.conf\.json --config tauri-native\.conf\.json/,
		)
		assert.doesNotMatch(commands, /tauri build[^\n]*tauri-native/)
		assert.equal(job.env.CARGO_INCREMENTAL, '0')
	}
	assert.equal(release.jobs.build.env.CARGO_PROFILE_RELEASE_LTO, 'true')
	assert.ok(ci.jobs.desktop.steps.some((step) => step.run?.includes('turbo run build')))
})

test('release and branch cache warming use the same full-LTO targets and setup', async () => {
	const release = parse(await readFile(join(root, '.github/workflows/axolotl-release.yml'), 'utf8'))
	const warm = parse(
		await readFile(join(root, '.github/workflows/prepare-release-cache.yml'), 'utf8'),
	)
	const setup = parse(
		await readFile(join(root, '.github/actions/setup-release-build/action.yml'), 'utf8'),
	)
	assert.deepEqual(
		warm.jobs.warm.strategy.matrix.include,
		release.jobs.build.strategy.matrix.include,
	)
	for (const job of [release.jobs.build, warm.jobs.warm]) {
		assert.equal(job.env.CARGO_PROFILE_RELEASE_LTO, 'true')
		assert.equal(job.env.CARGO_INCREMENTAL, '0')
		const checkout = job.steps.find((step) => step.uses?.startsWith('actions/checkout@'))
		assert.equal(checkout.with['fetch-depth'], 1)
		assert.equal(checkout.with.submodules, 'recursive')
		const step = job.steps.find((step) => step.uses === './.github/actions/setup-release-build')
		assert.deepEqual(step.with, {
			runner: '${{ matrix.runner }}',
			targets: '${{ matrix.targets }}',
		})
	}
	const cache = setup.runs.steps.find((step) => step.uses?.startsWith('Swatinem/rust-cache@'))
	assert.match(cache.with['shared-key'], /release-lto/)
	assert.equal(cache.with['cache-workspace-crates'], false)
	assert.equal(cache.with['cache-on-failure'], true)
	assert.equal(cache.env?.CARGO_PROFILE_RELEASE_LTO, undefined)
	assert.match(cache.with['save-if'], /refs\/heads\/main/)
	assert.deepEqual(warm.on.push.branches, ['main'])
	assert.equal(warm.permissions.contents, 'read')
	const preflightCheckout = release.jobs.preflight.steps.find((step) =>
		step.uses?.startsWith('actions/checkout@'),
	)
	assert.equal(preflightCheckout.with['fetch-depth'], 0)
	assert.equal(preflightCheckout.with.submodules, undefined)
	assert.ok(warm.jobs.warm.steps.some((step) => step.run?.includes('tauri bundle --no-sign')))
	assert.ok(
		release.jobs.build.steps.some((step) => step.with?.args?.includes('--locked --timings')),
	)
})

test('release restores download and resource caches before preparation', async () => {
	const {
		runs: { steps },
	} = parse(await readFile(join(root, '.github/actions/setup-release-build/action.yml'), 'utf8'))
	const prepare = steps.findIndex((step) => step.run?.includes('pnpm app:prepare'))
	const caches = steps.filter((step) => step.uses?.startsWith('actions/cache@'))
	const paths = caches.map((step) => step.with.path).join('\n')
	for (const path of [
		'.cache/npm',
		'~/.gradle/jdks',
		'~/.gradle/wrapper',
		'tauri-tools',
		'build/libs',
		'resources/blockbench-skin',
		'third-party/blockbench/dist/skin.bundle.js',
		'.cache/apt',
		'.turbo',
	]) {
		assert.ok(paths.includes(path), `missing cached path ${path}`)
	}
	for (const cache of caches) assert.ok(steps.indexOf(cache) < prepare)
	assert.ok(!paths.includes('node_modules'))
	assert.ok(!paths.includes('.env'))
	assert.ok(
		steps.some((step) => step.run?.includes('pnpm install --frozen-lockfile --prefer-offline')),
	)
})

test('byte-identical Blockbench resource sync advances its completion stamp', async () => {
	const fixture = await mkdtemp(join(tmpdir(), 'ymcl-build-sync-'))
	try {
		const script = join(fixture, 'scripts/axolotl/sync-blockbench-skin-editor.mjs')
		await mkdir(dirname(script), { recursive: true })
		await writeFile(
			script,
			await readFile(join(root, 'scripts/axolotl/sync-blockbench-skin-editor.mjs')),
		)
		const blockbench = join(fixture, 'third-party/blockbench')
		for (const directory of ['dist', 'assets', 'css', 'font']) {
			await mkdir(join(blockbench, directory), { recursive: true })
		}
		await writeFile(join(blockbench, 'dist/skin.bundle.js'), 'console.log("skin")\n')
		await writeFile(join(blockbench, 'index.html'), '<html></html>')
		await writeFile(join(blockbench, 'css/main.css'), 'body { color: red; }')
		const sync = () => execFileSync(process.execPath, [script], { timeout: 10_000 })
		sync()
		const bundle = join(fixture, 'apps/app/resources/blockbench-skin/dist/skin.bundle.js.gz')
		const stamp = join(blockbench, 'dist/.axolotl-skin-sync')
		const original = await readFile(bundle)
		const oldTime = new Date('2020-01-01T00:00:00Z')
		await utimes(bundle, oldTime, oldTime)
		await utimes(stamp, oldTime, oldTime)
		sync()
		assert.deepEqual(await readFile(bundle), original)
		assert.equal((await stat(bundle)).mtimeMs, oldTime.getTime())
		assert.ok(
			(await stat(stamp)).mtimeMs > (await stat(join(blockbench, 'dist/skin.bundle.js'))).mtimeMs,
		)
		await writeFile(join(blockbench, 'css/main.css'), 'body { color: blue; }')
		sync()
		assert.equal(
			await readFile(join(fixture, 'apps/app/resources/blockbench-skin/css/main.css'), 'utf8'),
			'body { color: blue; }',
		)
	} finally {
		await rm(fixture, { recursive: true, force: true })
	}
})
