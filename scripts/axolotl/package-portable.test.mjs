import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'
import {
	archivePortable,
	selectRuntime,
	stagePortable,
	windowsPowerShellEnvironment,
} from './package-portable.mjs'

test('Windows PowerShell rebuilds its module path when launched through Node from pwsh', () => {
	const source = {
		Path: 'tools',
		PSModulePath: 'PowerShell 7 modules',
		PSMODULEPATH: 'other',
		YMCL_PORTABLE_ZIP: 'portable.zip',
	}
	const environment = windowsPowerShellEnvironment(source)
	assert.deepEqual(environment, { Path: 'tools', YMCL_PORTABLE_ZIP: 'portable.zip' })
	assert.equal(source.PSModulePath, 'PowerShell 7 modules')
	if (process.platform === 'win32') {
		const command = execFileSync(
			'powershell.exe',
			[
				'-NoProfile',
				'-NonInteractive',
				'-Command',
				'$ErrorActionPreference = "Stop"; (Get-Command Get-AuthenticodeSignature).Name',
			],
			{
				encoding: 'utf8',
				env: windowsPowerShellEnvironment({
					...process.env,
					PSModulePath: 'Z:\\nonexistent-ps7-modules',
				}),
			},
		)
		assert.equal(command.trim(), 'Get-AuthenticodeSignature')
	}
})

test('fixed runtime comes from Microsoft and matches the requested architecture', () => {
	const release = (version) => ({
		version,
		builds: [
			{
				architecture: 'x64',
				url: `https://msedge.sf.dl.delivery.mp.microsoft.com/files/Microsoft.WebView2.FixedVersionRuntime.${version}.x64.cab`,
			},
		],
	})
	assert.equal(selectRuntime([release('155.0.1.1'), release('156.0.2.1')]).version, '156.0.2.1')
	assert.throws(() => selectRuntime([release('156.0.2.1')], 'arm64'), /No fixed/)
	const invalid = release('156.0.2.1')
	invalid.builds[0].url = invalid.builds[0].url.replace(
		'msedge.sf.dl.delivery.mp.microsoft.com',
		'example.org',
	)
	assert.throws(() => selectRuntime([invalid]), /Microsoft-hosted/)
})

test('portable layout includes the full app resources, runtime and a nonempty activation marker', async () => {
	const root = await mkdtemp(join(tmpdir(), 'ymcl-portable-'))
	try {
		const resources = join(root, 'input-resources')
		const runtime = join(root, 'input-runtime')
		const executable = join(root, 'input.exe')
		const destination = join(root, 'YMCL')
		await mkdir(join(resources, 'dist'), { recursive: true })
		await mkdir(runtime)
		await writeFile(executable, 'app binary')
		await writeFile(join(resources, 'index.html'), 'skin editor')
		await writeFile(join(resources, 'dist/skin.bundle.js.gz'), 'bundle')
		await writeFile(join(resources, '.resource'), 'hidden resource')
		await writeFile(join(runtime, 'msedgewebview2.exe'), 'runtime')
		await writeFile(join(runtime, 'icudtl.dat'), 'runtime data')
		await stagePortable({ executable, resources, runtime, destination })
		for (const [path, content] of [
			['YMCL.exe', 'app binary'],
			['resources/blockbench-skin/.resource', 'hidden resource'],
			['WebView2/icudtl.dat', 'runtime data'],
		])
			assert.equal(await readFile(join(destination, path), 'utf8'), content)
		assert.match(await readFile(join(destination, 'portable.txt'), 'utf8'), /portable/)
		await writeFile(join(destination, '.Axolotl/app.db'), 'existing data')
		await assert.rejects(stagePortable({ executable, resources, runtime, destination }), /EEXIST/)
		assert.equal(await readFile(join(destination, '.Axolotl/app.db'), 'utf8'), 'existing data')
		if (process.platform === 'win32') {
			await mkdir(join(destination, '.Axolotl/empty'))
			execFileSync('attrib.exe', ['+H', join(destination, '.Axolotl')])
			execFileSync('attrib.exe', ['+H', join(destination, 'resources/blockbench-skin/.resource')])
			const zip = join(root, 'portable.zip')
			archivePortable(destination, zip)
			const entries = execFileSync(
				'powershell.exe',
				[
					'-NoProfile',
					'-NonInteractive',
					'-Command',
					`Add-Type -AssemblyName System.IO.Compression.FileSystem; $zip = [System.IO.Compression.ZipFile]::OpenRead($env:YMCL_TEST_ZIP); try { $zip.Entries.FullName } finally { $zip.Dispose() }`,
				],
				{ encoding: 'utf8', env: { ...process.env, YMCL_TEST_ZIP: zip } },
			).replaceAll('\\', '/')
			assert.match(entries, /YMCL\/portable.txt/)
			assert.match(entries, /YMCL\/\.Axolotl\/app.db/)
			assert.match(entries, /YMCL\/\.Axolotl\/empty\//)
			assert.match(entries, /YMCL\/resources\/blockbench-skin\/\.resource/)
		}
	} finally {
		await rm(root, { recursive: true, force: true })
	}
})
