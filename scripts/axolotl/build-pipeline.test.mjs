import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdtemp, mkdir, readFile, rm, stat, utimes, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test } from 'node:test'
import { parse } from 'yaml'
import { stageReleaseAssets } from './stage-release-assets.mjs'

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

test('CI builds the app once and leaves distribution packaging to releases', async () => {
	const ci = parse(await readFile(join(root, '.github/workflows/axolotl-ci.yml'), 'utf8'))
	const release = parse(await readFile(join(root, '.github/workflows/axolotl-release.yml'), 'utf8'))
	const commands = ci.jobs.desktop.steps.map((step) => step.run ?? '').join('\n')
	assert.match(commands, /tauri build --no-bundle --config tauri-release\.conf\.json/)
	assert.doesNotMatch(commands, /cargo check|turbo run build|tauri bundle|package-portable\.mjs/)
	assert.ok(release.jobs.build.steps.some((step) => step.run?.includes('package-portable.mjs')))
	const preflight = release.jobs.preflight.steps.map((step) => step.run ?? '').join('\n')
	assert.doesNotMatch(preflight, /turbo run build/)
	assert.match(preflight, /downgrade-app-db\.test\.mjs/)
})

test('Windows installer variants reuse one release binary', async () => {
	const release = parse(await readFile(join(root, '.github/workflows/axolotl-release.yml'), 'utf8'))
	const commands = release.jobs.build.steps.map((step) => step.run ?? '').join('\n')
	assert.match(
		commands,
		/tauri bundle --config tauri-release\.conf\.json --config tauri-native\.conf\.json/,
	)
	assert.doesNotMatch(commands, /tauri build[^\n]*tauri-native/)
})

test('native Windows installers do not publish stale modern updater signatures', async () => {
	const release = parse(await readFile(join(root, '.github/workflows/axolotl-release.yml'), 'utf8'))
	const native = JSON.parse(await readFile(join(root, 'apps/app/tauri-native.conf.json'), 'utf8'))
	assert.equal(native.bundle.createUpdaterArtifacts, false)
	const variants = release.jobs.build.steps.find(
		(step) => step.name === 'Prepare Windows distribution variants',
	)
	const modern = release.jobs.build.steps.find(
		(step) => step.name === 'Name the signed Windows modern installer',
	)
	assert.ok(modern.run.includes('$modernSignaturePath'))
	assert.doesNotMatch(variants.run, /\$nativeSignature/)
	const staging = release.jobs.build.steps.findIndex((step) =>
		step.run?.includes('stage-release-assets.mjs'),
	)
	const publish = release.jobs.build.steps.findIndex(
		(step) => step.name === 'Publish Windows installer and updater immediately',
	)
	assert.ok(staging >= 0 && staging < release.jobs.build.steps.indexOf(modern))
	assert.ok(publish > release.jobs.build.steps.indexOf(modern))
	assert.ok(publish < release.jobs.build.steps.indexOf(variants))
	assert.equal(release.jobs.build.steps[publish].if, "runner.os == 'Windows'")
	assert.equal(release.jobs.build.steps[publish].uses, './.github/actions/publish-release-assets')
	assert.equal(release.jobs.build.strategy.matrix.include[0].name, 'Windows x64')
	assert.doesNotMatch(variants.run, /gh release (upload|delete-asset)/)
})

test('CI and releases share Windows dependency caches without caching embedded secrets', async () => {
	const release = parse(await readFile(join(root, '.github/workflows/axolotl-release.yml'), 'utf8'))
	const ci = parse(await readFile(join(root, '.github/workflows/axolotl-ci.yml'), 'utf8'))
	const setup = parse(
		await readFile(join(root, '.github/actions/setup-release-build/action.yml'), 'utf8'),
	)
	assert.equal(ci.jobs.desktop['runs-on'], release.jobs.build.strategy.matrix.include[0].runner)
	for (const job of [ci.jobs.desktop, release.jobs.build]) {
		assert.equal(job.env.CARGO_PROFILE_RELEASE_LTO, 'thin')
		assert.equal(job.env.CARGO_INCREMENTAL, '0')
		const checkout = job.steps.find((step) => step.uses?.startsWith('actions/checkout@'))
		assert.equal(checkout.with['fetch-depth'], 1)
		assert.equal(checkout.with.submodules, 'recursive')
		assert.ok(job.steps.some((step) => step.uses === './.github/actions/setup-release-build'))
	}
	const cache = setup.runs.steps.find((step) => step.uses?.startsWith('Swatinem/rust-cache@'))
	assert.match(cache.with['shared-key'], /release-lto/)
	assert.equal(cache.with['cache-workspace-crates'], false)
	assert.equal(cache.with['cache-on-failure'], true)
	assert.equal(cache.env?.CARGO_PROFILE_RELEASE_LTO, undefined)
	assert.match(cache.with['save-if'], /github\.event\.repository\.default_branch/)
	assert.match(cache.with['save-if'], /refs\/tags\/v/)
	const checkout = release.jobs.preflight.steps.find((step) =>
		step.uses?.startsWith('actions/checkout@'),
	)
	assert.equal(checkout.with['fetch-depth'], 0)
	assert.equal(checkout.with.submodules, undefined)
	assert.ok(
		release.jobs.build.steps.some((step) => step.with?.args?.includes('--locked --timings')),
	)
})

test('completed platforms publish independently while update manifests require the complete release', async () => {
	const release = parse(await readFile(join(root, '.github/workflows/axolotl-release.yml'), 'utf8'))
	assert.equal(release.jobs.build.strategy['fail-fast'], false)
	assert.equal(release.jobs.build.needs, 'preflight')
	for (const job of [release.jobs.build]) {
		const verification = job.steps.findIndex((step) =>
			step.run?.includes('verify-embedded-curseforge-key.mjs'),
		)
		const publish = job.steps.findIndex(
			(step) => step.uses === './.github/actions/publish-release-assets',
		)
		assert.ok(verification >= 0 && publish > verification)
		assert.equal(job.steps[publish].env.GH_TOKEN, '${{ secrets.GITHUB_TOKEN }}')
	}
	const build = release.jobs.build.steps.find((step) =>
		step.uses?.startsWith('tauri-apps/tauri-action@'),
	)
	assert.equal(build.with.tagName, undefined)
	assert.equal(build.with.releaseId, undefined)
	assert.deepEqual(release.jobs['verify-and-publish'].needs, ['preflight', 'build'])
	const publish = parse(
		await readFile(join(root, '.github/actions/publish-release-assets/action.yml'), 'utf8'),
	)
	assert.match(publish.runs.steps[0].run, /gh release upload/)
	assert.match(publish.runs.steps[1].run, /--draft=false --prerelease --latest=false/)
	assert.doesNotMatch(publish.runs.steps[1].run, /exit 0/)
	assert.match(publish.runs.steps[1].run, /--draft=false --latest=false/)
	const finalize = release.jobs['verify-and-publish'].steps
	const upload = finalize.findIndex((step) =>
		step.run?.includes('release-check/latest.json --clobber'),
	)
	const latest = finalize.findIndex((step) => step.run?.includes('gh release edit'))
	assert.ok(upload >= 0 && latest > upload)
})

test('manual recovery validates source artifacts and skips successful platform compilations', async () => {
	const release = parse(await readFile(join(root, '.github/workflows/axolotl-release.yml'), 'utf8'))
	assert.equal(release.on.workflow_dispatch.inputs.release_tag.required, true)
	assert.equal(release.on.workflow_dispatch.inputs.source_run_id.required, false)
	assert.equal(release.on.workflow_dispatch.inputs.rebuild.default, false)
	assert.equal(release.env.RELEASE_TAG, '${{ inputs.release_tag || github.ref_name }}')
	assert.equal(release.permissions.actions, 'read')
	const validation = release.jobs.preflight.steps.find((step) =>
		step.run?.includes('validate-release-recovery.mjs'),
	)
	assert.equal(validation.if, "github.event_name == 'workflow_dispatch'")
	assert.equal(validation.env.SOURCE_RUN_ID, '${{ inputs.source_run_id }}')
	for (const name of ['build'])
		assert.equal(
			release.jobs[name].if,
			"github.event_name != 'workflow_dispatch' || inputs.rebuild",
		)
	for (const name of ['verify-and-publish']) {
		assert.match(release.jobs[name].if, /!cancelled\(\)/)
		assert.match(release.jobs[name].if, /needs.preflight.result == 'success'/)
		assert.match(release.jobs[name].if, /github.event_name == 'workflow_dispatch'/)
	}
	const publish = release.jobs['verify-and-publish'].steps
	for (const name of [
		'Promote complete stable release to latest',
		'Publish website download metadata',
	])
		assert.equal(
			publish.find((step) => step.name === name).if,
			"${{ !contains(env.RELEASE_TAG, '-') }}",
		)
	const mirror = publish.find((step) => step.name === 'Mirror published release to CNB')
	assert.doesNotMatch(mirror.run, /push --force/)
})

test('manual publication reuses Windows assets and generates the missing update manifest', async () => {
	const release = parse(await readFile(join(root, '.github/workflows/axolotl-release.yml'), 'utf8'))
	const condition = release.jobs['verify-and-publish'].if
	const evaluate = ({
		event = 'workflow_dispatch',
		rebuild = false,
		preflight = 'success',
		build = 'skipped',
	} = {}) =>
		Function(
			'github',
			'inputs',
			'needs',
			'cancelled',
			`return ${condition.slice(3, -3)}`,
		)(
			{ event_name: event },
			{ rebuild },
			{ preflight: { result: preflight }, build: { result: build } },
			() => false,
		)
	assert.equal(evaluate(), true)
	assert.equal(evaluate({ event: 'push' }), false)
	assert.equal(evaluate({ event: 'push', build: 'success' }), true)
	assert.equal(evaluate({ preflight: 'failure' }), false)
	assert.equal(evaluate({ build: 'failure' }), false)
	assert.equal(evaluate({ rebuild: true }), false)
	const steps = release.jobs['verify-and-publish'].steps
	for (const name of ['Generate update manifest', 'Upload update manifest'])
		assert.equal(steps.find((step) => step.name === name).if, undefined)
	assert.ok(!steps.some((step) => step.name === 'Download existing update manifest'))
})

test('release publication uses YuDream without requiring platform selection', async () => {
	const release = parse(await readFile(join(root, '.github/workflows/axolotl-release.yml'), 'utf8'))
	assert.equal(release.on.workflow_dispatch.inputs.update_platforms, undefined)
	assert.equal(release.env.YMCL_UPDATE_PLATFORMS, undefined)
})

test('stable website metadata updates the default branch from a tag checkout', async () => {
	const release = parse(await readFile(join(root, '.github/workflows/axolotl-release.yml'), 'utf8'))
	const publish = release.jobs['verify-and-publish'].steps.find(
		(step) => step.name === 'Publish website download metadata',
	)
	assert.equal(publish.env.DEFAULT_BRANCH, '${{ github.event.repository.default_branch }}')
	assert.match(
		publish.run,
		/git switch --create release-website-metadata "origin\/\$\{DEFAULT_BRANCH\}"/,
	)
	assert.match(publish.run, /git push origin "HEAD:\$\{DEFAULT_BRANCH\}"/)
	assert.doesNotMatch(publish.run, /push origin main|--force/)
})

test('staging preserves signed updater bytes across rebundling and keeps universal asset names', async () => {
	const directory = await mkdtemp(join(tmpdir(), 'ymcl-release-staging-'))
	try {
		const source = join(directory, 'original')
		await mkdir(source)
		const archive = join(source, 'YMCL.app.tar.gz')
		const signature = `${archive}.sig`
		await writeFile(archive, 'signed updater bytes')
		await writeFile(signature, 'signature bytes')
		const staged = await stageReleaseAssets(
			[archive, signature],
			join(directory, 'assets'),
			'universal-apple-darwin',
		)
		assert.deepEqual(staged, [
			join(directory, 'assets/YMCL_universal.app.tar.gz'),
			join(directory, 'assets/YMCL_universal.app.tar.gz.sig'),
		])
		await writeFile(archive, 'native variant overwrote source')
		assert.equal(await readFile(staged[0], 'utf8'), 'signed updater bytes')
		assert.equal(await readFile(staged[1], 'utf8'), 'signature bytes')
		await assert.rejects(stageReleaseAssets([], join(directory, 'empty')), /No release artifacts/)
		await assert.rejects(stageReleaseAssets([source], join(directory, 'invalid')), /regular files/)
		await assert.rejects(
			stageReleaseAssets([archive, archive], join(directory, 'duplicates')),
			/Duplicate release artifact/,
		)
	} finally {
		await rm(directory, { recursive: true, force: true })
	}
})

test(
	'publishing exposes successful uploads, keeps latest intact, and stops on upload failure',
	{
		skip: process.platform === 'win32',
	},
	async () => {
		const directory = await mkdtemp(join(tmpdir(), 'ymcl-release-publish-'))
		try {
			await mkdir(join(directory, 'release-assets'))
			await writeFile(join(directory, 'release-assets/YMCL.dmg'), 'verified package')
			const log = join(directory, 'commands.jsonl')
			await writeFile(
				join(directory, 'gh'),
				`#!/usr/bin/env node
const fs = require('node:fs')
const args = process.argv.slice(2)
fs.appendFileSync(process.env.PUBLISH_TEST_LOG, JSON.stringify(args) + '\\n')
if (args[1] === 'upload' && process.env.PUBLISH_TEST_FAIL === '1') process.exit(1)
if (args[1] === 'view') console.log(process.env.PUBLISH_TEST_DRAFT)
`,
				{ mode: 0o755 },
			)
			const {
				runs: { steps },
			} = parse(
				await readFile(join(root, '.github/actions/publish-release-assets/action.yml'), 'utf8'),
			)
			for (const { tag, draft, fail, recovery } of [
				{ tag: 'v1.2.3', draft: true },
				{ tag: 'v1.2.3-beta.1', draft: true },
				{ tag: 'v1.2.3', draft: false },
				{ tag: 'v1.2.3-beta.1', draft: false },
				{ tag: 'v1.2.3', draft: true, fail: true },
				{ tag: 'v1.2.3-beta.1', draft: false, recovery: true },
			]) {
				await writeFile(log, '')
				const run = () => {
					for (const step of steps)
						execFileSync('bash', ['-e', '-o', 'pipefail', '-c', step.run], {
							env: {
								...process.env,
								PATH: `${directory}:${process.env.PATH}`,
								RUNNER_TEMP: directory,
								GITHUB_REF_NAME: recovery ? 'ymcl-dev' : tag,
								RELEASE_TAG: recovery ? tag : '',
								PUBLISH_TEST_LOG: log,
								PUBLISH_TEST_DRAFT: String(draft),
								PUBLISH_TEST_FAIL: fail ? '1' : '0',
							},
							stdio: ['ignore', 'pipe', 'pipe'],
						})
				}
				if (fail) assert.throws(run)
				else run()
				const commands = (await readFile(log, 'utf8')).trim().split('\n').map(JSON.parse)
				assert.equal(commands[0][1], 'upload')
				assert.equal(commands[0][2], tag)
				assert.ok(commands[0].includes(join(directory, 'release-assets/YMCL.dmg')))
				if (fail || (!draft && !tag.includes('-')))
					assert.equal(
						commands.some((args) => args[1] === 'edit'),
						false,
					)
				else
					assert.deepEqual(commands.at(-1), [
						'release',
						'edit',
						tag,
						'--draft=false',
						...(tag.includes('-') ? ['--prerelease'] : []),
						'--latest=false',
					])
			}
		} finally {
			await rm(directory, { recursive: true, force: true })
		}
	},
)

test('release refuses missing credentials and verifies the key in each production binary', async () => {
	const release = parse(await readFile(join(root, '.github/workflows/axolotl-release.yml'), 'utf8'))
	const secretGuard = release.jobs.preflight.steps.find((step) =>
		step.run?.includes('check-release-secrets.mjs'),
	)
	for (const name of ['CURSEFORGE_API_KEY', 'TAURI_SIGNING_PRIVATE_KEY', 'YUDREAM_API_KEY']) {
		assert.equal(secretGuard.env[name], `\${{ secrets.${name} }}`)
	}
	assert.equal(secretGuard.env.SWUSTMC_API_KEY, undefined)
	for (const job of [release.jobs.build]) {
		assert.equal(job.env.AXOLOTL_REQUIRE_CURSEFORGE_API_KEY, '1')
		const verification = job.steps.find((step) =>
			step.run?.includes('verify-embedded-curseforge-key.mjs'),
		)
		assert.equal(verification.env.CURSEFORGE_API_KEY, '${{ secrets.CURSEFORGE_API_KEY }}')
	}
	const publish = release.jobs['verify-and-publish'].steps.find((step) =>
		step.run?.includes('publish-ymcl-platforms.mjs'),
	)
	assert.equal(publish.env.YUDREAM_API_KEY, '${{ secrets.YUDREAM_API_KEY }}')
	assert.equal(publish.env.SWUSTMC_API_KEY, undefined)
})

test('production update defaults consistently use the YuDream platform', async () => {
	const production = await readFile(join(root, 'apps/app-frontend/.env.production'), 'utf8')
	const tauri = JSON.parse(await readFile(join(root, 'apps/app/tauri-release.conf.json'), 'utf8'))
	const updater = await readFile(join(root, 'apps/app/src/updater_impl.rs'), 'utf8')
	const base = 'https://www.yudream.online/api/plugins/ymcl-content/v1/update'
	assert.ok(production.includes(`VITE_YMCL_UPDATE_URL=${base}`))
	assert.equal(tauri.plugins.updater.endpoints[0], `${base}/latest?channel=beta`)
	assert.ok(updater.includes(`${base}/manifest`))
	assert.ok(updater.includes(`${base}/versions`))
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
