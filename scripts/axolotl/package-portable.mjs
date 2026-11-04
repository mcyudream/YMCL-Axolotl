import { execFileSync } from 'node:child_process'
import { createWriteStream } from 'node:fs'
import {
	cp,
	mkdir,
	mkdtemp,
	readFile,
	readdir,
	rename,
	rm,
	stat,
	writeFile,
} from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { Readable } from 'node:stream'
import { pipeline } from 'node:stream/promises'
import { fileURLToPath } from 'node:url'
import { parseArgs } from 'node:util'
import { run } from './prepare-build-resources.mjs'

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..')
const runtimeManifest = 'https://developer.microsoft.com/microsoft-edge/api/webview2'

export function selectRuntime(manifest, architecture = 'x64') {
	const releases = [...manifest].filter((release) => /^\d+(\.\d+){3}$/.test(release.version))
	releases.sort((a, b) => b.version.localeCompare(a.version, 'en', { numeric: true }))
	for (const release of releases) {
		const build = release.builds.find((build) => build.architecture === architecture)
		if (!build) continue
		const url = new URL(build.url)
		if (
			url.protocol !== 'https:' ||
			!url.hostname.endsWith('.microsoft.com') ||
			!url.pathname.endsWith(
				`/Microsoft.WebView2.FixedVersionRuntime.${release.version}.${architecture}.cab`,
			)
		)
			throw new Error('Expected a Microsoft-hosted fixed WebView2 runtime cabinet')
		return { version: release.version, url: url.href }
	}
	throw new Error(`No fixed WebView2 runtime for ${architecture}`)
}

function powershell(script, environment = {}, timeout = 120_000) {
	return execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', script], {
		encoding: 'utf8',
		stdio: ['ignore', 'pipe', 'pipe'],
		env: { ...process.env, ...environment },
		timeout,
	})
}

function verifyRuntime(directory) {
	powershell(
		`
$ErrorActionPreference = 'Stop'
$signature = Get-AuthenticodeSignature -LiteralPath (Join-Path $env:YMCL_WEBVIEW2_PATH 'msedgewebview2.exe')
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notlike '*Microsoft Corporation*') {
  throw 'Fixed WebView2 runtime must have a valid Microsoft signature'
}
`,
		{ YMCL_WEBVIEW2_PATH: directory },
	)
}

async function prepareRuntime(cacheDirectory) {
	const response = await fetch(runtimeManifest, { signal: AbortSignal.timeout(60_000) })
	if (!response.ok) throw new Error(`WebView2 manifest request failed: ${response.status}`)
	const runtime = selectRuntime(await response.json())
	const directory = join(cacheDirectory, `${runtime.version}-x64`)
	try {
		await stat(join(directory, 'msedgewebview2.exe'))
		verifyRuntime(directory)
		return directory
	} catch {}
	await mkdir(cacheDirectory, { recursive: true })
	const staging = await mkdtemp(join(cacheDirectory, '.download-'))
	try {
		console.log(`[portable] Downloading fixed WebView2 ${runtime.version} from Microsoft`)
		const download = await fetch(runtime.url, { signal: AbortSignal.timeout(10 * 60_000) })
		if (!download.ok || !download.body)
			throw new Error(`WebView2 download failed: ${download.status}`)
		const cabinet = join(staging, 'runtime.cab')
		await pipeline(Readable.fromWeb(download.body), createWriteStream(cabinet))
		const extracted = join(staging, 'extracted')
		await mkdir(extracted)
		execFileSync('expand.exe', [cabinet, '-F:*', extracted], { stdio: 'pipe', timeout: 120_000 })
		let runtimeDirectory = extracted
		try {
			await stat(join(runtimeDirectory, 'msedgewebview2.exe'))
		} catch {
			const entries = await readdir(extracted, { withFileTypes: true })
			const candidates = entries.filter(
				(entry) =>
					entry.isDirectory() && entry.name.startsWith('Microsoft.WebView2.FixedVersionRuntime.'),
			)
			if (candidates.length !== 1) throw new Error('Unexpected fixed WebView2 cabinet layout')
			runtimeDirectory = join(extracted, candidates[0].name)
		}
		verifyRuntime(runtimeDirectory)
		await writeFile(
			join(runtimeDirectory, 'ymcl-runtime.json'),
			`${JSON.stringify(runtime, null, 2)}\n`,
		)
		await rm(directory, { recursive: true, force: true })
		await rename(runtimeDirectory, directory)
		return directory
	} finally {
		await rm(staging, { recursive: true, force: true })
	}
}

// This accepts explicit inputs so the exact release layout can also be tested
// on non-Windows hosts, without building the desktop app or downloading a runtime.
export async function stagePortable({ executable, resources, runtime, destination }) {
	for (const path of [
		executable,
		join(resources, 'index.html'),
		join(resources, 'dist/skin.bundle.js.gz'),
		join(runtime, 'msedgewebview2.exe'),
	]) {
		if (!(await stat(path)).isFile()) throw new Error(`Missing portable build input: ${path}`)
	}
	// Refuse an existing destination: a package build must never empty user data.
	await mkdir(destination)
	await cp(executable, join(destination, 'YMCL.exe'))
	await mkdir(join(destination, 'resources'))
	await cp(resources, join(destination, 'resources/blockbench-skin'), { recursive: true })
	await cp(runtime, join(destination, 'WebView2'), { recursive: true })
	await mkdir(join(destination, '.Axolotl'))
	await writeFile(
		join(destination, 'portable.txt'),
		'YMCL portable installation. Keep this file beside YMCL.exe.\n',
	)
	await writeFile(
		join(destination, 'README.txt'),
		`YMCL Windows 便携版 / Windows portable edition

把整个 YMCL 文件夹解压到 U 盘或其他可写目录，双击 YMCL.exe。
实例、存档、模组、配置和自动下载的 Java 保存在 .Axolotl 中。
首次安装游戏需要联网下载；准备好游戏和 Java 后可在另一台兼容的 Windows 电脑上继续使用。
便携包自带 WebView2，无需安装启动器或 WebView2。
请在启动器和游戏完全退出后再安全移除 U 盘。

更新：退出启动器，用新版便携 ZIP 中的程序、resources 和 WebView2 替换旧文件。
保留现有 .Axolotl 文件夹，其中包含全部便携数据。
直接关联的外部实例和手动指定的电脑本地路径仍依赖原位置；需要随盘使用的文件请放在此目录内。
系统凭据库中的已记住密码和 AI 密钥保存在各电脑上，换电脑可能需要重新输入。

Extract the whole YMCL directory to a writable USB drive and run YMCL.exe.
Portable data, games and managed Java runtimes live in .Axolotl.
WebView2 is included. Initial game/Java downloads require an internet connection.
Close both the launcher and the game before safely ejecting the drive.
To update, replace the program files, resources and WebView2; retain .Axolotl.
External linked instances and custom paths still depend on their original locations.
Remembered passwords and AI credentials in the OS credential store stay on each computer.
`,
	)
}

export function archivePortable(directory, destination) {
	// .NET includes hidden files and empty directories, unlike Compress-Archive.
	powershell(
		`
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
[System.IO.Compression.ZipFile]::CreateFromDirectory($env:YMCL_PORTABLE_SOURCE, $env:YMCL_PORTABLE_ZIP, [System.IO.Compression.CompressionLevel]::Optimal, $true)
`,
		{ YMCL_PORTABLE_SOURCE: directory, YMCL_PORTABLE_ZIP: destination },
		10 * 60_000,
	)
}

async function main() {
	if (process.platform !== 'win32') throw new Error('Build the Windows portable ZIP on Windows')
	if (process.arch !== 'x64') throw new Error('The portable package currently targets Windows x64')
	const { values } = parseArgs({
		options: {
			build: { type: 'boolean' },
			version: { type: 'string' },
			'output-dir': { type: 'string' },
			runtime: { type: 'string' },
		},
	})
	if (values.build) {
		await run('pnpm.cmd', [
			'--filter',
			'@modrinth/app',
			'tauri',
			'build',
			'--no-bundle',
			'--config',
			'tauri-release.conf.json',
		])
	}
	const version =
		values.version ??
		JSON.parse(await readFile(join(repositoryRoot, 'apps/app-frontend/package.json'), 'utf8'))
			.version
	if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version))
		throw new Error('Invalid portable release version')
	const target = join(repositoryRoot, 'target')
	await mkdir(target, { recursive: true })
	const runtime = values.runtime
		? resolve(values.runtime)
		: await prepareRuntime(join(target, 'portable-webview2'))
	verifyRuntime(runtime)
	const staging = await mkdtemp(join(target, 'portable-build-'))
	const outputDirectory = resolve(values['output-dir'] ?? join(target, 'release/bundle/portable'))
	await mkdir(outputDirectory, { recursive: true })
	const output = join(outputDirectory, `YMCL_${version}_x64_portable.zip`)
	const temporaryZip = join(staging, 'portable.zip')
	try {
		const directory = join(staging, 'YMCL')
		await stagePortable({
			executable: join(target, 'release/YMCL.exe'),
			resources: join(repositoryRoot, 'apps/app/resources/blockbench-skin'),
			runtime,
			destination: directory,
		})
		archivePortable(directory, temporaryZip)
		await cp(temporaryZip, output)
		console.log(`[portable] Created ${output}`)
	} finally {
		await rm(staging, { recursive: true, force: true })
	}
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	await main()
}
