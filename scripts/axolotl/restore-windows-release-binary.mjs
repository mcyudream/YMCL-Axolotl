import { execFileSync } from 'node:child_process'
import { cp, mkdir, mkdtemp, rm, stat } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { downloadArtifact } from './publish-ymcl-platforms.mjs'
import { validatePublishedRelease } from './validate-release-recovery.mjs'

const tag = process.env.RELEASE_TAG
const repository = process.env.GITHUB_REPOSITORY
if (
	process.platform !== 'win32' ||
	!/^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(tag ?? '') ||
	!repository
)
	throw new Error('Restoring a release binary requires Windows and a validated release tag')
const release = JSON.parse(
	execFileSync('gh', ['api', `repos/${repository}/releases/tags/${tag}`], { encoding: 'utf8' }),
)
validatePublishedRelease(release, tag, repository, { requireVariants: false })
const asset = release.assets.find(
	(asset) => asset.name === `YMCL_${tag.slice(1)}_x64_modern-setup.exe`,
)
const staging = await mkdtemp(join(tmpdir(), 'ymcl-release-binary-'))
try {
	const installer = await downloadArtifact(
		{
			filename: asset.name,
			size: asset.size,
			sha256: asset.digest.slice('sha256:'.length),
			downloadUrl: asset.browser_download_url,
		},
		staging,
	)
	const extracted = join(staging, 'extracted')
	const sevenZip = join(process.env.ProgramFiles, '7-Zip/7z.exe')
	execFileSync(sevenZip, ['x', '-y', installer, `-o${extracted}`], {
		stdio: 'pipe',
		timeout: 120_000,
	})
	const executable = join(extracted, 'YMCL.exe')
	if (!(await stat(executable)).isFile())
		throw new Error('The verified installer contains no YMCL.exe')
	const resources = join(extracted, 'resources/blockbench-skin')
	for (const file of ['index.html', 'dist/skin.bundle.js.gz']) {
		if (!(await stat(join(resources, file))).isFile())
			throw new Error(`The verified installer is missing ${file}`)
	}
	await mkdir('target/release', { recursive: true })
	await cp(executable, 'target/release/YMCL.exe')
	await rm('apps/app/resources/blockbench-skin', { recursive: true, force: true })
	await cp(resources, 'apps/app/resources/blockbench-skin', { recursive: true })
	console.log(
		`Restored the existing ${tag} executable from its verified installer; no Rust compilation`,
	)
} finally {
	await rm(staging, { recursive: true, force: true })
}
