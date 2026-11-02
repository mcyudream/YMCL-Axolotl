import { copyFile, lstat, mkdir } from 'node:fs/promises'
import { basename, join } from 'node:path'
import { pathToFileURL } from 'node:url'

export async function stageReleaseAssets(artifacts, directory, target) {
	if (!Array.isArray(artifacts) || !artifacts.length) throw new Error('No release artifacts')
	const names = new Set()
	const files = []
	for (const artifact of artifacts) {
		if (typeof artifact !== 'string' || !(await lstat(artifact)).isFile())
			throw new Error('Release artifacts must be regular files')
		let name = basename(artifact)
		if (target === 'universal-apple-darwin') {
			name = name.replace(/(?<!_universal)\.app\.tar\.gz(\.sig)?$/, '_universal.app.tar.gz$1')
		}
		if (names.has(name)) throw new Error(`Duplicate release artifact ${name}`)
		names.add(name)
		files.push({ source: artifact, destination: join(directory, name) })
	}
	await mkdir(directory, { recursive: true })
	for (const file of files) await copyFile(file.source, file.destination)
	return files.map((file) => file.destination)
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	const [directory, target] = process.argv.slice(2)
	if (!directory) throw new Error('Usage: stage-release-assets.mjs <directory> [target]')
	await stageReleaseAssets(JSON.parse(process.env.ARTIFACT_PATHS ?? '[]'), directory, target)
}
