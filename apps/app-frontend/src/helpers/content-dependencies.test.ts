import assert from 'node:assert/strict'
import test from 'node:test'

import { dependencyRefMatchesContentItem } from './content-dependencies.ts'

test('dependency references match either provider identity of a recognized mod', () => {
	const item = {
		project: { id: 'modrinth-id' },
		provider_refs: [
			{ provider: 'modrinth' as const, project_id: 'modrinth-id', version_id: null },
			{ provider: 'curseforge' as const, project_id: 123, file_id: null },
		],
	}
	assert.equal(
		dependencyRefMatchesContentItem({ provider: 'curseforge', projectId: '123' }, item),
		true,
	)
	assert.equal(
		dependencyRefMatchesContentItem({ provider: 'curseforge', projectId: 'curseforge:123' }, item),
		true,
	)
	assert.equal(
		dependencyRefMatchesContentItem({ provider: 'modrinth', projectId: 'other' }, item),
		false,
	)
})

test('dependency references retain local and legacy CurseForge project matching', () => {
	for (const projectId of ['123', 'curseforge:123']) {
		assert.equal(
			dependencyRefMatchesContentItem(
				{ provider: 'curseforge', projectId: '123' },
				{ project: { id: projectId }, provider_refs: [] },
			),
			true,
		)
	}
	assert.equal(
		dependencyRefMatchesContentItem(
			{ provider: 'local', projectId: 'local:library' },
			{ project: { id: 'local:library' }, provider_refs: [] },
		),
		true,
	)
})

test('provider references prevent matching an ID owned by a different provider', () => {
	assert.equal(
		dependencyRefMatchesContentItem(
			{ provider: 'curseforge', projectId: '123' },
			{
				project: { id: '123' },
				provider_refs: [{ provider: 'modrinth', project_id: '123', version_id: null }],
			},
		),
		false,
	)
})

test('local project identity remains usable with partial online references', () => {
	assert.equal(
		dependencyRefMatchesContentItem(
			{ provider: 'local', projectId: 'local:library' },
			{
				project: { id: 'local:library' },
				provider_refs: [{ provider: 'modrinth', project_id: 'online-id', version_id: null }],
			},
		),
		true,
	)
})

test('stable entries survive online display identities and distinguish duplicate copies', () => {
	const reference = { entryId: 'library-b', provider: 'local' as const, projectId: 'local:library' }
	const item = {
		instanceEntryId: 'library-b',
		project: { id: '123' },
		provider_refs: [{ provider: 'curseforge' as const, project_id: 123, file_id: null }],
	}
	assert.equal(dependencyRefMatchesContentItem(reference, item), true)
	assert.equal(
		dependencyRefMatchesContentItem(reference, { ...item, instanceEntryId: 'library-a' }),
		false,
	)
})

test('discovered file references survive online display metadata', () => {
	const reference = {
		fileId: 'library-file',
		provider: 'local' as const,
		projectId: 'local:library',
	}
	const item = {
		instanceFileId: 'library-file',
		instanceEntryId: 'recognized-entry',
		project: { id: 'online-library' },
		provider_refs: [
			{ provider: 'modrinth' as const, project_id: 'online-library', version_id: 'online-version' },
		],
	}
	assert.equal(dependencyRefMatchesContentItem(reference, item), true)
	assert.equal(
		dependencyRefMatchesContentItem(reference, { ...item, instanceFileId: 'other-file' }),
		false,
	)
})
