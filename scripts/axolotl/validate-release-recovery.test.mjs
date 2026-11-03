import assert from 'node:assert/strict'
import { test } from 'node:test'
import { validateRecoveryInputs, validateRecoverySource } from './validate-release-recovery.mjs'

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
		{ workflow_run: { id: 124, head_sha: 'a'.repeat(40), repository_id: 1, head_repository_id: 1 } },
		{ workflow_run: { id: 123, head_sha: 'c'.repeat(40), repository_id: 1, head_repository_id: 1 } },
		{ workflow_run: { id: 123, head_sha: 'a'.repeat(40), repository_id: 2, head_repository_id: 2 } },
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
