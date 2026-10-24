import assert from 'node:assert/strict'
import { access, mkdir, mkdtemp, readFile, rm, stat, utimes, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { test } from 'node:test'
import {
	cachedBuild,
	fingerprint,
	prepareBlockbench,
	prepareJava,
	run,
} from './prepare-build-resources.mjs'

async function fixture(t, files) {
	const root = await mkdtemp(join(tmpdir(), 'ymcl-resource-cache-'))
	t.after(() => rm(root, { recursive: true, force: true }))
	for (const [path, content] of Object.entries(files)) {
		await mkdir(dirname(join(root, path)), { recursive: true })
		await writeFile(join(root, path), content)
	}
	return root
}

test('cache ignores checkout timestamps but invalidates content changes and deletions', async (t) => {
	const root = await fixture(t, { 'src/a': 'one', 'src/b': 'two' })
	let builds = 0
	const build = () =>
		cachedBuild({
			root,
			name: 'fixture',
			inputs: ['src'],
			outputs: ['out'],
			build: async () => {
				builds++
				await writeFile(join(root, 'out'), await fingerprint(root, ['src']))
			},
		})
	assert.equal(await build(), true)
	await utimes(join(root, 'src/a'), new Date(), new Date('2030-01-01'))
	assert.equal(await build(), false)
	const mtime = (await stat(join(root, 'src/a'))).mtime
	await writeFile(join(root, 'src/a'), 'new')
	await utimes(join(root, 'src/a'), mtime, mtime)
	assert.equal(await build(), true)
	await rm(join(root, 'src/b'))
	assert.equal(await build(), true)
	assert.equal(builds, 3)
})

test('missing and corrupted outputs are rebuilt, not accepted by a source-only cache', async (t) => {
	const root = await fixture(t, { input: 'input' })
	const options = {
		root,
		name: 'fixture',
		inputs: ['input'],
		outputs: ['out'],
		required: ['out'],
		build: () => writeFile(join(root, 'out'), 'correct'),
	}
	await cachedBuild(options)
	await writeFile(join(root, 'out'), 'corrupt')
	assert.equal(await cachedBuild(options), true)
	assert.equal(await readFile(join(root, 'out'), 'utf8'), 'correct')
	await rm(join(root, 'out'))
	assert.equal(await cachedBuild(options), true)
})

test('failed or concurrently edited builds never leave a valid cache manifest', async (t) => {
	const root = await fixture(t, { input: 'before' })
	const options = { root, name: 'fixture', inputs: ['input'], outputs: ['out'] }
	await assert.rejects(
		cachedBuild({
			...options,
			build: async () => {
				throw new Error('compiler failed')
			},
		}),
		/compiler failed/,
	)
	await assert.rejects(access(join(root, '.cache/build-resources/fixture.json')))
	await assert.rejects(
		cachedBuild({
			...options,
			build: async () => {
				await writeFile(join(root, 'out'), 'output')
				await writeFile(join(root, 'input'), 'after')
			},
		}),
		/inputs changed/,
	)
	await assert.rejects(access(join(root, '.cache/build-resources/fixture.json')))
	await assert.rejects(access(join(root, '.cache/build-resources/fixture.json.lock')))
})

test('concurrent callers prepare one resource only once', async (t) => {
	const root = await fixture(t, { input: 'source' })
	let builds = 0
	const options = {
		root,
		name: 'fixture',
		inputs: ['input'],
		outputs: ['out'],
		build: async () => {
			builds++
			await writeFile(join(root, 'out'), 'output')
		},
	}
	const results = await Promise.all([
		cachedBuild(options),
		cachedBuild(options),
		cachedBuild(options),
	])
	assert.equal(builds, 1)
	assert.equal(results.filter(Boolean).length, 1)
})

test('Java agents share one verified output directory across Cargo profiles and targets', async (t) => {
	const java = 'packages/app-lib/java'
	const root = await fixture(t, {
		'.nvmrc': '24.15.0',
		'scripts/axolotl/prepare-build-resources.mjs': 'script',
		[`${java}/src/Agent.java`]: 'source',
		[`${java}/gradle/wrapper.properties`]: 'wrapper',
		[`${java}/gradlew`]: 'wrapper',
		[`${java}/gradlew.bat`]: 'wrapper',
		[`${java}/build.gradle.kts`]: 'build',
		[`${java}/settings.gradle.kts`]: 'settings',
		[`${java}/gradle.properties`]: 'settings',
	})
	const calls = []
	const execute = async (command, args, cwd) => {
		calls.push({ command, args, cwd })
		const libs = join(root, java, 'build/libs')
		await mkdir(libs, { recursive: true })
		await writeFile(join(libs, 'theseus.jar'), 'agent')
		await writeFile(join(libs, 'authlib-injector.jar'), 'authlib')
	}
	await prepareJava(root, execute)
	await prepareJava(root, execute)
	assert.equal(calls.length, 1)
	assert.ok(calls[0].args.includes('build'))
	assert.ok(calls[0].args.includes('--no-daemon'))
	assert.ok(calls[0].args[0].endsWith(join(java, 'build')))
	assert.ok(!calls[0].args.join(' ').includes('target/release'))
})

test('a restored Blockbench resource cache skips npm install and bundling', async (t) => {
	const blockbench = 'third-party/blockbench'
	const root = await fixture(t, {
		'.nvmrc': '24.15.0',
		'scripts/axolotl/prepare-build-resources.mjs': 'prepare',
		'scripts/axolotl/sync-blockbench-skin-editor.mjs': 'sync',
		[`${blockbench}/package.json`]: '{}',
		[`${blockbench}/package-lock.json`]: '{}',
		[`${blockbench}/build.js`]: 'build',
		[`${blockbench}/tsconfig.json`]: '{}',
		[`${blockbench}/js/app.js`]: 'source',
		[`${blockbench}/lib/dependency.js`]: 'dependency',
		[`${blockbench}/lang/en.json`]: '{}',
		[`${blockbench}/keymaps/default.bbkeymap`]: '{}',
		[`${blockbench}/themes/default.bbtheme`]: '{}',
		[`${blockbench}/css/main.css`]: 'style',
		[`${blockbench}/assets/icon.png`]: 'icon',
		[`${blockbench}/font/test.woff`]: 'font',
		[`${blockbench}/index.html`]: 'html',
	})
	let installs = 0
	const execute = async (_command, args, _cwd, environment) => {
		if (args[0] === 'ci') {
			installs++
			assert.ok(args.includes('--prefer-offline'))
			assert.ok(!args.includes('--ignore-scripts'))
			assert.deepEqual(environment, { ELECTRON_SKIP_BINARY_DOWNLOAD: '1' })
		} else if (args[0].endsWith('.mjs')) {
			const resources = join(root, 'apps/app/resources/blockbench-skin')
			await mkdir(join(resources, 'dist'), { recursive: true })
			await writeFile(join(resources, 'index.html'), 'html')
			await writeFile(join(resources, 'dist/skin.bundle.js.gz'), 'bundle')
			await mkdir(join(root, blockbench, 'dist'), { recursive: true })
			await writeFile(join(root, blockbench, 'dist/skin.bundle.js'), 'raw bundle')
		}
	}
	assert.equal(await prepareBlockbench(root, execute), true)
	assert.equal(await prepareBlockbench(root, execute), false)
	assert.equal(installs, 1)
	await writeFile(join(root, blockbench, 'package-lock.json'), '{"changed":true}')
	assert.equal(await prepareBlockbench(root, execute), true)
	assert.equal(installs, 2)
	await writeFile(join(root, blockbench, 'lib/dependency.js'), 'updated dependency')
	assert.equal(await prepareBlockbench(root, execute), true)
	assert.equal(installs, 3)
	await writeFile(join(root, '.nvmrc'), '26')
	assert.equal(await prepareBlockbench(root, execute), true)
	assert.equal(installs, 4)
	await rm(join(root, blockbench, 'dist/skin.bundle.js'))
	assert.equal(await prepareBlockbench(root, execute), true)
	assert.equal(installs, 5)
})

test('preparation commands preserve the environment and propagate process errors', async (t) => {
	const root = await fixture(t, {})
	await run(
		process.execPath,
		[
			'-e',
			'if (process.env.YMCL_PREPARE_ENV_TEST !== "verified" || !process.env.PATH) process.exit(1)',
		],
		root,
		{ YMCL_PREPARE_ENV_TEST: 'verified' },
	)
	await assert.rejects(run(process.execPath, ['-e', 'process.exit(7)'], root), /failed \(7\)/)
	await assert.rejects(run(join(root, 'missing-command'), [], root), { code: 'ENOENT' })
})
