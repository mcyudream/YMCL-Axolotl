import assert from 'node:assert/strict'
import { test } from 'node:test'
import {
	validatePublishedRelease,
	validateRecoveryInputs,
	validateRecoverySource,
	validateRebuildSource,
} from './validate-release-recovery.mjs'

function fixture() {
	const releaseSha = 'a'.repeat(40)
	return {
		tag: 'v2.0.0-beta.3',
		sourceRunId: '123',
		repository: 'owner/YMCL',
		releaseSha,
		changedPaths: ['.github/workflows/axolotl-release.yml'],
		run: {
			id: 123,
			repository: { id: 1, full_name: 'owner/YMCL' },
			head_repository: { full_name: 'owner/YMCL' },
			path: '.github/workflows/axolotl-release.yml',
			event: 'push',
			status: 'completed',
			head_branch: 'v2.0.0-beta.3',
			head_sha: releaseSha,
		},
		jobs: [
			'Release preflight',
			'Build (Linux x64)',
			'Build (Linux arm64)',
			'Build (Windows x64)',
			'Compile (macOS Apple Silicon)',
			'Compile (macOS Intel)',
		].map((name) => ({ name, status: 'completed', conclusion: 'success' })),
		artifacts: ['macos-aarch64-apple-darwin', 'macos-x86_64-apple-darwin'].map((name) => ({
			name,
			expired: false,
			size_in_bytes: 100,
			digest: `sha256:${'b'.repeat(64)}`,
			workflow_run: { id: 123, head_sha: releaseSha, repository_id: 1, head_repository_id: 1 },
		})),
	}
}

test('recovery accepts verified native artifacts and only packaging workflow changes', () => {
	assert.doesNotThrow(() => validateRecoverySource(fixture()))
	const source = fixture()
	source.changedPaths.push('scripts/axolotl/publish-ymcl-platforms.mjs')
	assert.doesNotThrow(() => validateRecoverySource(source))
})

test('rebuild allows CI-only repairs before assets exist and preserves published releases', () => {
	const source = fixture()
	assert.doesNotThrow(() => validateRebuildSource({ ...source, release: null }))
	assert.doesNotThrow(() =>
		validateRebuildSource({ ...source, release: { draft: true, assets: [] } }),
	)
	for (const release of [
		{ draft: false, assets: [] },
		{ draft: true, assets: [{}] },
	])
		assert.throws(() => validateRebuildSource({ ...source, release }), /cannot overwrite/)
	source.changedPaths.push('packages/app-lib/src/launcher/mod.rs')
	assert.throws(() => validateRebuildSource(source), /cannot change release source/)
})

test('recovery accepts repaired build runs only when their source still matches the release tag', () => {
	const source = fixture()
	Object.assign(source.run, {
		event: 'workflow_dispatch',
		head_branch: 'ymcl-dev',
		head_sha: 'c'.repeat(40),
	})
	source.sourceChangedPaths = ['.github/workflows/axolotl-release.yml']
	for (const artifact of source.artifacts) artifact.workflow_run.head_sha = source.run.head_sha
	assert.doesNotThrow(() => validateRecoverySource(source))
	source.sourceChangedPaths.push('apps/app-frontend/src/pages/Index.vue')
	assert.throws(() => validateRecoverySource(source), /exact tag and commit/)
})

function publishedRelease() {
	const tag = 'v2.0.0-beta.3'
	const names = [
		'latest.json',
		'YMCL-2.0.0-beta.3-1.aarch64.rpm',
		'YMCL-2.0.0-beta.3-1.aarch64.rpm.sig',
		'YMCL-2.0.0-beta.3-1.x86_64.rpm',
		'YMCL-2.0.0-beta.3-1.x86_64.rpm.sig',
		'YMCL_2.0.0-beta.3_aarch64.AppImage',
		'YMCL_2.0.0-beta.3_aarch64.AppImage.sig',
		'YMCL_2.0.0-beta.3_aarch64.AppImage.tar.gz',
		'YMCL_2.0.0-beta.3_aarch64.AppImage.tar.gz.sig',
		'YMCL_2.0.0-beta.3_amd64.AppImage',
		'YMCL_2.0.0-beta.3_amd64.AppImage.sig',
		'YMCL_2.0.0-beta.3_amd64.AppImage.tar.gz',
		'YMCL_2.0.0-beta.3_amd64.AppImage.tar.gz.sig',
		'YMCL_2.0.0-beta.3_amd64.deb',
		'YMCL_2.0.0-beta.3_amd64.deb.sig',
		'YMCL_2.0.0-beta.3_arm64.deb',
		'YMCL_2.0.0-beta.3_arm64.deb.sig',
		'YMCL_2.0.0-beta.3_universal.dmg',
		'YMCL_2.0.0-beta.3_x64-setup.nsis.zip',
		'YMCL_2.0.0-beta.3_x64-setup.nsis.zip.sig',
		'YMCL_2.0.0-beta.3_x64_modern-setup.exe',
		'YMCL_2.0.0-beta.3_x64_modern-setup.exe.sig',
		'YMCL_2.0.0-beta.3_x64_nsis-setup.exe',
		'YMCL_2.0.0-beta.3_x64_portable.zip',
		'YMCL_universal.app.tar.gz',
		'YMCL_universal.app.tar.gz.sig',
	]
	return {
		tag_name: tag,
		draft: false,
		published_at: '2026-11-03T00:20:07Z',
		assets: names.map((name) => ({
			name,
			size: 100,
			state: 'uploaded',
			digest: `sha256:${'b'.repeat(64)}`,
			browser_download_url: `https://github.com/owner/YMCL/releases/download/${tag}/${name}`,
		})),
	}
}

test('final synchronization requires all published installers, updates, signatures and manifest', () => {
	const source = fixture()
	source.publishOnly = true
	source.artifacts = []
	assert.doesNotThrow(() => validateRecoverySource(source))
	const published = publishedRelease()
	const validate = (value) => validatePublishedRelease(value, source.tag, source.repository)
	assert.doesNotThrow(() => validate(published))
	for (const removed of published.assets) {
		assert.throws(
			() =>
				validate({ ...published, assets: published.assets.filter((asset) => asset !== removed) }),
			/missing/,
		)
	}
	for (const patch of [{ tag_name: 'v2.0.0-beta.2' }, { draft: true }, { published_at: null }])
		assert.throws(() => validate({ ...published, ...patch }), /existing published release/)
	for (const patch of [
		{ size: 0 },
		{ state: 'new' },
		{ digest: null },
		{ browser_download_url: 'https://untrusted.example/latest.json' },
	]) {
		assert.throws(
			() =>
				validate({
					...published,
					assets: [{ ...published.assets[0], ...patch }, ...published.assets.slice(1)],
				}),
			/Invalid existing release asset/,
		)
	}
	assert.throws(
		() => validate({ ...published, assets: [...published.assets, published.assets[0]] }),
		/Invalid existing release asset/,
	)
	const failedBuild = fixture()
	failedBuild.publishOnly = true
	failedBuild.jobs[1].conclusion = 'failure'
	assert.throws(() => validateRecoverySource(failedBuild), /successful Build/)
})

test('recovery rejects invalid tag and run inputs', () => {
	for (const tag of ['ymcl-dev', '../v2.0.0', 'v2.0.0;echo unsafe', undefined])
		assert.throws(() => validateRecoveryInputs(tag, '123'), /version tag/)
	for (const runId of ['0', '-1', '123/jobs', '', undefined])
		assert.throws(() => validateRecoveryInputs('v2.0.0-beta.3', runId), /numeric source run/)
})

test('recovery rejects source runs from a different commit, tag, repository or workflow', () => {
	for (const patch of [
		{ id: 124 },
		{ head_sha: 'c'.repeat(40) },
		{ head_branch: 'v2.0.0-beta.2' },
		{ repository: { id: 2, full_name: 'untrusted/YMCL' } },
		{ head_repository: { full_name: 'untrusted/YMCL' } },
		{ path: '.github/workflows/axolotl-ci.yml' },
		{ event: 'workflow_dispatch' },
		{ status: 'in_progress' },
	]) {
		const source = fixture()
		Object.assign(source.run, patch)
		assert.throws(() => validateRecoverySource(source), /exact tag and commit/)
	}
	const changedSource = fixture()
	changedSource.changedPaths.push('apps/app/tauri.conf.json')
	assert.throws(() => validateRecoverySource(changedSource), /cannot change release source/)
})

test('recovery requires successful builds and cannot accept a failed key verification', () => {
	for (const state of ['failure', 'cancelled', 'skipped', null]) {
		const source = fixture()
		source.jobs.at(-1).conclusion = state
		assert.throws(() => validateRecoverySource(source), /successful Compile/)
	}
	const missing = fixture()
	missing.jobs.pop()
	assert.throws(() => validateRecoverySource(missing), /successful Compile/)
	const duplicate = fixture()
	duplicate.jobs.push({ ...duplicate.jobs[0] })
	assert.throws(() => validateRecoverySource(duplicate), /one successful/)
})

test('recovery refuses incomplete, expired or substituted architecture artifacts', () => {
	for (const patch of [
		{ expired: true },
		{ size_in_bytes: 0 },
		{ digest: null },
		{
			workflow_run: { id: 124, head_sha: 'a'.repeat(40), repository_id: 1, head_repository_id: 1 },
		},
		{
			workflow_run: { id: 123, head_sha: 'c'.repeat(40), repository_id: 1, head_repository_id: 1 },
		},
		{
			workflow_run: { id: 123, head_sha: 'a'.repeat(40), repository_id: 2, head_repository_id: 2 },
		},
	]) {
		const source = fixture()
		Object.assign(source.artifacts[0], patch)
		assert.throws(() => validateRecoverySource(source), /native artifact/)
	}
	for (const count of [1, 3]) {
		const source = fixture()
		if (count === 1) source.artifacts.pop()
		else source.artifacts.push({ ...source.artifacts[0] })
		assert.throws(() => validateRecoverySource(source), /exactly two/)
	}
})
