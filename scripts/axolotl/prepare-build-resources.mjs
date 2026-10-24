import { spawn } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
	access,
	mkdir,
	open,
	readFile,
	readdir,
	rename,
	rm,
	lstat,
	writeFile,
} from 'node:fs/promises'
import { hostname } from 'node:os'
import { dirname, relative, resolve } from 'node:path'
import { setTimeout as delay } from 'node:timers/promises'
import { fileURLToPath, pathToFileURL } from 'node:url'

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..')
const cacheDirectory = '.cache/build-resources'

export async function fingerprint(root, paths) {
	const hash = createHash('sha256')
	async function visit(path) {
		const info = await lstat(path)
		hash.update(relative(root, path).replaceAll('\\', '/')).update('\0')
		if (info.isDirectory()) {
			hash.update('directory\0')
			for (const name of (await readdir(path)).sort()) await visit(resolve(path, name))
		} else if (info.isFile()) {
			hash
				.update(`${info.size}\0`)
				.update(await readFile(path))
				.update('\0')
		} else {
			throw new Error(`Unsupported build input: ${path}`)
		}
	}
	for (const path of [...paths].sort()) await visit(resolve(root, path))
	return hash.digest('hex')
}

async function withLock(path, callback) {
	await mkdir(dirname(path), { recursive: true })
	const deadline = Date.now() + 15 * 60_000
	while (true) {
		let handle
		try {
			handle = await open(path, 'wx')
		} catch (error) {
			if (error.code !== 'EEXIST') throw error
			try {
				const owner = JSON.parse(await readFile(path, 'utf8'))
				if (owner.host === hostname()) {
					try {
						process.kill(owner.pid, 0)
					} catch (error) {
						if (error.code === 'ESRCH') await rm(path, { force: true })
					}
				}
			} catch {}
			if (Date.now() >= deadline) throw new Error(`Timed out waiting for build lock: ${path}`)
			await delay(200)
			continue
		}
		try {
			await handle.writeFile(JSON.stringify({ pid: process.pid, host: hostname() }))
			return await callback()
		} finally {
			await handle.close()
			await rm(path, { force: true })
		}
	}
}

export async function cachedBuild({ root, name, inputs, outputs, required = [], build }) {
	const manifest = resolve(root, cacheDirectory, `${name}.json`)
	return withLock(`${manifest}.lock`, async () => {
		const inputHash = await fingerprint(root, inputs)
		try {
			const cached = JSON.parse(await readFile(manifest, 'utf8'))
			await Promise.all(required.map((path) => access(resolve(root, path))))
			if (cached.inputs === inputHash && cached.outputs === (await fingerprint(root, outputs))) {
				console.log(`[prepare] ${name}: content cache hit`)
				return false
			}
		} catch {}
		console.log(`[prepare] ${name}: cache miss, building`)
		await rm(manifest, { force: true })
		await build()
		await Promise.all(required.map((path) => access(resolve(root, path))))
		if (inputHash !== (await fingerprint(root, inputs))) {
			throw new Error(`${name} inputs changed during the build; retry before packaging`)
		}
		const contents = JSON.stringify({
			inputs: inputHash,
			outputs: await fingerprint(root, outputs),
		})
		const temporary = `${manifest}.${process.pid}.tmp`
		await writeFile(temporary, contents)
		await rename(temporary, manifest)
		return true
	})
}

export function run(command, args, cwd = repositoryRoot, environment = {}) {
	return new Promise((accept, reject) => {
		const batch = process.platform === 'win32' && /\.(bat|cmd)$/.test(command)
		const child = spawn(
			batch ? `"${command}"` : command,
			batch ? args.map((arg) => `"${arg}"`) : args,
			{
				cwd,
				env: { ...process.env, ...environment },
				stdio: 'inherit',
				shell: batch,
			},
		)
		child.once('error', reject)
		child.once('exit', (code, signal) => {
			if (code === 0) accept()
			else reject(new Error(`${command} failed (${signal ?? code})`))
		})
	})
}

export function prepareJava(root = repositoryRoot, execute = run) {
	const java = 'packages/app-lib/java'
	const outputs = [`${java}/build/libs/theseus.jar`, `${java}/build/libs/authlib-injector.jar`]
	return cachedBuild({
		root,
		name: 'java',
		inputs: [
			'.nvmrc',
			'scripts/axolotl/prepare-build-resources.mjs',
			`${java}/src`,
			`${java}/gradle`,
			`${java}/gradlew`,
			`${java}/gradlew.bat`,
			`${java}/build.gradle.kts`,
			`${java}/settings.gradle.kts`,
			`${java}/gradle.properties`,
		],
		outputs,
		required: outputs,
		build: () =>
			execute(
				resolve(root, java, process.platform === 'win32' ? 'gradlew.bat' : 'gradlew'),
				[
					`-Dorg.gradle.project.buildDir=${resolve(root, java, 'build')}`,
					'build',
					'--no-daemon',
					'--console=plain',
				],
				resolve(root, java),
			),
	})
}

export function prepareBlockbench(root = repositoryRoot, execute = run) {
	const blockbench = 'third-party/blockbench'
	const resources = 'apps/app/resources/blockbench-skin'
	const developmentBundle = `${blockbench}/dist/skin.bundle.js`
	return cachedBuild({
		root,
		name: 'blockbench',
		inputs: [
			'.nvmrc',
			'scripts/axolotl/prepare-build-resources.mjs',
			'scripts/axolotl/sync-blockbench-skin-editor.mjs',
			`${blockbench}/package.json`,
			`${blockbench}/package-lock.json`,
			`${blockbench}/build.js`,
			`${blockbench}/tsconfig.json`,
			`${blockbench}/js`,
			`${blockbench}/lib`,
			`${blockbench}/lang`,
			`${blockbench}/keymaps`,
			`${blockbench}/themes`,
			`${blockbench}/css`,
			`${blockbench}/assets`,
			`${blockbench}/font`,
			`${blockbench}/index.html`,
		],
		outputs: [resources, developmentBundle],
		required: [`${resources}/index.html`, `${resources}/dist/skin.bundle.js.gz`, developmentBundle],
		build: async () => {
			const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm'
			await execute(
				npm,
				['ci', '--prefer-offline', '--no-audit', '--no-fund'],
				resolve(root, blockbench),
				{ ELECTRON_SKIP_BINARY_DOWNLOAD: '1' },
			)
			await execute(npm, ['run', 'build-skin'], resolve(root, blockbench))
			await execute(process.execPath, ['scripts/axolotl/sync-blockbench-skin-editor.mjs'], root)
		},
	})
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
	const mode = process.argv[2] ?? 'all'
	if (!['all', 'java', 'blockbench'].includes(mode))
		throw new Error(`Unknown resource group: ${mode}`)
	const tasks = []
	if (mode !== 'blockbench') tasks.push(prepareJava())
	if (mode !== 'java') tasks.push(prepareBlockbench())
	const results = await Promise.allSettled(tasks)
	for (const result of results) {
		if (result.status === 'rejected') {
			console.error(result.reason)
			process.exitCode = 1
		}
	}
}
