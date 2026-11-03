import { execFileSync } from 'node:child_process'
import { pathToFileURL } from 'node:url'

const recoveryPaths = new Set([
	'.github/workflows/axolotl-release.yml',
	'.github/actions/publish-release-assets/action.yml',
	'scripts/axolotl/build-pipeline.test.mjs',
	'scripts/axolotl/validate-release-recovery.mjs',
	'scripts/axolotl/validate-release-recovery.test.mjs',
	'scripts/axolotl/publish-ymcl-platforms.mjs',
	'scripts/axolotl/release-platforms.test.mjs',
])

const requiredJobs = [
	'Release preflight',
	'Build (Linux x64)',
	'Build (Linux arm64)',
	'Build (Windows x64)',
	'Compile (macOS Apple Silicon)',
	'Compile (macOS Intel)',
]

const requiredArtifacts = ['macos-aarch64-apple-darwin', 'macos-x86_64-apple-darwin']

export function validateRecoveryInputs(tag, sourceRunId) {
	if (!/^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(tag ?? ''))
		throw new Error('Recovery requires an existing version tag')
	if (!/^[1-9]\d*$/.test(sourceRunId ?? ''))
		throw new Error('Recovery requires a numeric source run ID')
}

export function validateRecoverySource({
	tag,
	sourceRunId,
	repository,
	releaseSha,
	changedPaths,
	run,
	jobs,
	artifacts,
	publishOnly = false,
}) {
	validateRecoveryInputs(tag, sourceRunId)
	if (
		String(run.id) !== sourceRunId ||
		run.repository?.full_name !== repository ||
		run.head_repository?.full_name !== repository ||
		run.path !== '.github/workflows/axolotl-release.yml' ||
		run.event !== 'push' ||
		run.status !== 'completed' ||
		run.head_branch !== tag ||
		run.head_sha !== releaseSha
	)
		throw new Error('Source run must be a completed release run for the exact tag and commit')
	const unexpectedPaths = changedPaths.filter((path) => !recoveryPaths.has(path))
	if (unexpectedPaths.length)
		throw new Error(`Recovery cannot change release source files: ${unexpectedPaths.join(', ')}`)
	for (const name of requiredJobs) {
		const matches = jobs.filter((job) => job.name === name)
		if (
			matches.length !== 1 ||
			matches[0].status !== 'completed' ||
			matches[0].conclusion !== 'success'
		)
			throw new Error(`Source run must have one successful ${name} job`)
	}
	if (publishOnly) return
	const macosArtifacts = artifacts.filter((artifact) => artifact.name.startsWith('macos-'))
	if (macosArtifacts.length !== requiredArtifacts.length)
		throw new Error('Source run must contain exactly two native macOS artifacts')
	for (const name of requiredArtifacts) {
		const matches = macosArtifacts.filter((artifact) => artifact.name === name)
		const artifact = matches[0]
		if (
			matches.length !== 1 ||
			artifact.expired ||
			!(artifact.size_in_bytes > 0) ||
			!/^sha256:[a-f0-9]{64}$/.test(artifact.digest ?? '') ||
			String(artifact.workflow_run?.id) !== sourceRunId ||
			artifact.workflow_run?.head_sha !== releaseSha ||
			artifact.workflow_run?.repository_id !== run.repository.id ||
			artifact.workflow_run?.head_repository_id !== run.repository.id
		)
			throw new Error(`Missing, expired, or mismatched native artifact: ${name}`)
	}
}

export function validatePublishedRelease(release, tag, repository) {
	const version = tag.replace(/^v/, '')
	if (release.tag_name !== tag || release.draft || !release.published_at)
		throw new Error('Publish-only recovery requires the existing published release')
	const required = new Set(['latest.json'])
	for (const [filename, signed] of [
		[`YMCL_${version}_amd64.AppImage`, true],
		[`YMCL_${version}_aarch64.AppImage`, true],
		[`YMCL_${version}_amd64.deb`, true],
		[`YMCL_${version}_arm64.deb`, true],
		[`YMCL-${version}-1.x86_64.rpm`, true],
		[`YMCL-${version}-1.aarch64.rpm`, true],
		[`YMCL_${version}_amd64.AppImage.tar.gz`, true],
		[`YMCL_${version}_aarch64.AppImage.tar.gz`, true],
		[`YMCL_${version}_x64_modern-setup.exe`, true],
		[`YMCL_${version}_x64_nsis-setup.exe`, false],
		[`YMCL_${version}_x64_portable.zip`, false],
		[`YMCL_${version}_x64-setup.nsis.zip`, true],
		[`YMCL_${version}_universal.dmg`, false],
		['YMCL_universal.app.tar.gz', true],
	]) {
		required.add(filename)
		if (signed) required.add(`${filename}.sig`)
	}
	const names = new Set()
	for (const asset of release.assets ?? []) {
		if (
			names.has(asset.name) ||
			asset.state !== 'uploaded' ||
			!Number.isSafeInteger(asset.size) ||
			asset.size <= 0 ||
			!/^sha256:[a-f0-9]{64}$/.test(asset.digest ?? '') ||
			asset.browser_download_url !==
				`https://github.com/${repository}/releases/download/${tag}/${asset.name}`
		)
			throw new Error(`Invalid existing release asset: ${asset.name}`)
		names.add(asset.name)
	}
	for (const name of required)
		if (!names.has(name)) throw new Error(`Publish-only recovery is missing ${name}`)
}

async function readGitHub(path) {
	const response = await fetch(
		`${process.env.GITHUB_API_URL ?? 'https://api.github.com'}/repos/${process.env.GITHUB_REPOSITORY}${path}`,
		{
			headers: {
				Authorization: `Bearer ${process.env.GH_TOKEN}`,
				Accept: 'application/vnd.github+json',
				'X-GitHub-Api-Version': '2022-11-28',
				'User-Agent': 'YMCL-release-recovery',
			},
			signal: AbortSignal.timeout(30_000),
		},
	)
	if (!response.ok) throw new Error(`Recovery source lookup failed: HTTP ${response.status}`)
	return response.json()
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	const tag = process.env.RELEASE_TAG
	const sourceRunId = process.env.SOURCE_RUN_ID
	const publishOnly = process.env.PUBLISH_ONLY === 'true'
	validateRecoveryInputs(tag, sourceRunId)
	if (!process.env.GH_TOKEN || !process.env.GITHUB_REPOSITORY)
		throw new Error('Recovery requires GitHub credentials and repository context')
	const git = (...args) => execFileSync('git', args, { encoding: 'utf8' }).trim()
	const releaseSha = git('rev-parse', '--verify', '--end-of-options', `refs/tags/${tag}^{commit}`)
	const changedPaths = git('diff', '--name-only', releaseSha, 'HEAD', '--')
		.split('\n')
		.filter(Boolean)
	const [run, jobData, artifactData] = await Promise.all([
		readGitHub(`/actions/runs/${sourceRunId}`),
		readGitHub(`/actions/runs/${sourceRunId}/jobs?filter=latest&per_page=100`),
		readGitHub(`/actions/runs/${sourceRunId}/artifacts?per_page=100`),
	])
	if (jobData.total_count > 100 || artifactData.total_count > 100)
		throw new Error('Recovery source exceeds the supported job or artifact count')
	validateRecoverySource({
		tag,
		sourceRunId,
		repository: process.env.GITHUB_REPOSITORY,
		releaseSha,
		changedPaths,
		run,
		jobs: jobData.jobs,
		artifacts: artifactData.artifacts,
		publishOnly,
	})
	if (publishOnly) {
		validatePublishedRelease(
			await readGitHub(`/releases/tags/${encodeURIComponent(tag)}`),
			tag,
			process.env.GITHUB_REPOSITORY,
		)
		console.log(
			`Validated complete published assets for ${tag} at ${releaseSha}; final synchronization only`,
		)
	} else {
		console.log(
			`Validated ${tag} at ${releaseSha}; reusing native artifacts from run ${sourceRunId}`,
		)
	}
}
