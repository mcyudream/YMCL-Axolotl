import { execFileSync } from 'node:child_process'
import { pathToFileURL } from 'node:url'

const recoveryPaths = new Set([
	'.github/workflows/axolotl-ci.yml',
	'scripts/axolotl/create-update-manifest.mjs',
	'scripts/axolotl/verify-update-manifest.mjs',
	'scripts/axolotl/create-update-server-catalog.mjs',
	'scripts/axolotl/create-update-server-catalog.test.mjs',
	'scripts/axolotl/create-website-release-metadata.mjs',
	'scripts/axolotl/windows-release.test.mjs',
	'apps/website/releases/latest.json',
	'apps/website/releases/catalog.json',

	'.github/workflows/axolotl-release.yml',
	'.github/actions/publish-release-assets/action.yml',
	'.github/actions/setup-release-build/action.yml',
	'.github/workflows/prepare-release-cache.yml',
	'scripts/axolotl/build-pipeline.test.mjs',
	'scripts/axolotl/validate-release-recovery.mjs',
	'scripts/axolotl/validate-release-recovery.test.mjs',
	'scripts/axolotl/publish-ymcl-platforms.mjs',
	'scripts/axolotl/release-platforms.test.mjs',
	'scripts/axolotl/check-release-secrets.mjs',
])

const requiredJobs = ['Release preflight', 'Build (Windows x64)']

function validateReleaseTag(tag) {
	if (!/^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(tag ?? ''))
		throw new Error('Recovery requires an existing version tag')
}

export function validateRecoveryInputs(tag, sourceRunId) {
	validateReleaseTag(tag)
	if (!/^[1-9]\d*$/.test(sourceRunId ?? ''))
		throw new Error('Recovery requires a numeric source run ID')
}

export function validateRebuildSource({ tag, changedPaths, release }) {
	validateReleaseTag(tag)
	const unexpectedPaths = changedPaths.filter((path) => !recoveryPaths.has(path))
	if (unexpectedPaths.length)
		throw new Error(`Rebuild cannot change release source files: ${unexpectedPaths.join(', ')}`)
	if (release && (!release.draft || release.assets?.length))
		throw new Error('Rebuild cannot overwrite an existing published release or uploaded assets')
}

export function validateRecoverySource({
	tag,
	sourceRunId,
	repository,
	releaseSha,
	changedPaths,
	run,
	jobs,
	sourceChangedPaths,
}) {
	validateRecoveryInputs(tag, sourceRunId)
	const taggedSource =
		run.event === 'push' && run.head_branch === tag && run.head_sha === releaseSha
	const repairedSource =
		run.event === 'workflow_dispatch' &&
		/^[a-f0-9]{40}$/.test(run.head_sha ?? '') &&
		Array.isArray(sourceChangedPaths) &&
		sourceChangedPaths.every((path) => recoveryPaths.has(path))
	if (
		String(run.id) !== sourceRunId ||
		run.repository?.full_name !== repository ||
		run.head_repository?.full_name !== repository ||
		run.path !== '.github/workflows/axolotl-release.yml' ||
		run.status !== 'completed' ||
		(!taggedSource && !repairedSource)
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
}

export function validatePublishedRelease(release, tag, repository) {
	const version = tag.replace(/^v/, '')
	if (release.tag_name !== tag || release.draft || !release.published_at)
		throw new Error('Publish-only recovery requires the existing published release')
	const required = new Set()
	for (const [filename, signed] of [
		[`YMCL_${version}_x64_modern-setup.exe`, true],
		[`YMCL_${version}_x64_nsis-setup.exe`, false],
		[`YMCL_${version}_x64_portable.zip`, false],
		[`YMCL_${version}_x64-setup.nsis.zip`, true],
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

async function readGitHub(path, optional = false) {
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
	if (optional && response.status === 404) return null
	if (!response.ok) throw new Error(`Recovery source lookup failed: HTTP ${response.status}`)
	return response.json()
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	const tag = process.env.RELEASE_TAG
	const sourceRunId = process.env.SOURCE_RUN_ID
	const publishOnly = process.env.PUBLISH_ONLY === 'true'
	const rebuild = process.env.REBUILD_RELEASE === 'true'
	validateReleaseTag(tag)
	if (!process.env.GH_TOKEN || !process.env.GITHUB_REPOSITORY)
		throw new Error('Recovery requires GitHub credentials and repository context')
	const git = (...args) => execFileSync('git', args, { encoding: 'utf8' }).trim()
	const releaseSha = git('rev-parse', '--verify', '--end-of-options', `refs/tags/${tag}^{commit}`)
	const changedPaths = git('diff', '--name-only', releaseSha, 'HEAD', '--')
		.split('\n')
		.filter(Boolean)
	if (rebuild) {
		if (publishOnly || sourceRunId)
			throw new Error('Rebuild cannot reuse artifacts or publish-only mode')
		validateRebuildSource({
			tag,
			changedPaths,
			release: await readGitHub(`/releases/tags/${encodeURIComponent(tag)}`, true),
		})
		console.log(`Validated ${tag} at ${releaseSha}; rebuilding with CI-only repairs`)
	} else {
		validateRecoveryInputs(tag, sourceRunId)
		const [run, jobData] = await Promise.all([
			readGitHub(`/actions/runs/${sourceRunId}`),
			readGitHub(`/actions/runs/${sourceRunId}/jobs?filter=latest&per_page=100`),
		])
		if (jobData.total_count > 100)
			throw new Error('Recovery source exceeds the supported job count')
		validateRecoverySource({
			tag,
			sourceRunId,
			repository: process.env.GITHUB_REPOSITORY,
			releaseSha,
			changedPaths,
			run,
			jobs: jobData.jobs,
			sourceChangedPaths:
				run.event === 'workflow_dispatch' && /^[a-f0-9]{40}$/.test(run.head_sha ?? '')
					? git('diff', '--name-only', releaseSha, run.head_sha, '--').split('\n').filter(Boolean)
					: undefined,
		})
		validatePublishedRelease(
			await readGitHub(`/releases/tags/${encodeURIComponent(tag)}`),
			tag,
			process.env.GITHUB_REPOSITORY,
		)
		console.log(`Validated signed Windows assets for ${tag} at ${releaseSha}; publication only`)
	}
}
