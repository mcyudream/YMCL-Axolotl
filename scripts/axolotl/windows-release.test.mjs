import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'

test('Windows updates and website downloads can publish without Linux or macOS assets', async () => {
	const directory = await mkdtemp(join(tmpdir(), 'ymcl-windows-release-'))
	try {
		const version = '2.1.0'
		const tag = `v${version}`
		const archive = `YMCL_${version}_x64-setup.nsis.zip`
		const signature = Buffer.from('signed Windows updater test fixture').toString('base64')
		const names = [archive, `${archive}.sig`, `YMCL_${version}_x64_modern-setup.exe`]
		const release = {
			tag_name: tag,
			body: 'Release notes',
			assets: names.map((name) => ({
				name,
				browser_download_url: `https://github.com/mcyudream/YMCL-Axolotl/releases/download/${tag}/${name}`,
			})),
		}
		const releasePath = join(directory, 'release.json')
		const manifestPath = join(directory, 'latest.json')
		const websitePath = join(directory, 'website.json')
		await writeFile(releasePath, JSON.stringify(release))
		await writeFile(join(directory, `${archive}.sig`), signature)
		const run = (script, ...args) => execFileSync(process.execPath, [`scripts/axolotl/${script}`, ...args], { stdio: 'pipe' })
		run('create-update-manifest.mjs', releasePath, directory, tag, manifestPath)
		run('verify-update-manifest.mjs', manifestPath, tag)
		const manifest = JSON.parse(await readFile(manifestPath, 'utf8'))
		assert.equal(manifest.version, version)
		assert.deepEqual(Object.keys(manifest.platforms), ['windows-x86_64'])
		assert.equal(manifest.platforms['windows-x86_64'].signature, signature)
		release.assets.push({ name: `YMCL_${version}_amd64.AppImage` }, { name: `YMCL_${version}_universal.dmg` })
		await writeFile(releasePath, JSON.stringify(release))
		run('create-website-release-metadata.mjs', releasePath, websitePath)
		assert.deepEqual(JSON.parse(await readFile(websitePath, 'utf8')).assets, names)
		manifest.platforms['linux-x86_64'] = manifest.platforms['windows-x86_64']
		await writeFile(manifestPath, JSON.stringify(manifest))
		assert.throws(() => run('verify-update-manifest.mjs', manifestPath, tag), /Only Windows/)
		await rm(join(directory, `${archive}.sig`))
		assert.throws(() => run('create-update-manifest.mjs', releasePath, directory, tag, manifestPath), /Missing updater signature/)
	} finally {
		await rm(directory, { recursive: true, force: true })
	}
})
