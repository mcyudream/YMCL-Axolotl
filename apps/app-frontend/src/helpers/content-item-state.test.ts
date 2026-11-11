import assert from 'node:assert/strict'
import test from 'node:test'

import type { ContentItem } from '@modrinth/ui'

import {
	applyContentItemUpdates,
	contentItemProvider,
	contentItemReference,
} from './content-item-state.ts'

test('recognized MRPACK files use their verified source without inventing an install origin', () => {
	assert.equal(
		contentItemProvider({
			origin_provider: null,
			provider_refs: [{ provider: 'curseforge', project_id: 123, file_id: 456 }],
		}),
		'curseforge',
	)
})

test('an explicit install origin wins over a recognized alias', () => {
	assert.equal(
		contentItemProvider({
			origin_provider: 'modrinth',
			provider_refs: [{ provider: 'curseforge', project_id: 123, file_id: 456 }],
		}),
		'modrinth',
	)
})

test('an exact file identity wins over a project-only reference', () => {
	assert.equal(
		contentItemProvider({
			origin_provider: null,
			provider_refs: [
				{ provider: 'curseforge', project_id: 123, file_id: null },
				{ provider: 'modrinth', project_id: 'AANobbMI', version_id: '7pwil2dy' },
			],
		}),
		'modrinth',
	)
})

test('project-only origins keep their project when another exact alias is recognized', () => {
	const origin = { provider: 'curseforge' as const, project_id: 999, file_id: null }
	const alias = { provider: 'curseforge' as const, project_id: 123, file_id: 456 }
	assert.equal(
		contentItemReference({ origin_provider: 'curseforge', provider_refs: [origin, alias] }),
		origin,
	)
	const verifiedOrigin = { ...origin, file_id: 789 }
	assert.equal(
		contentItemReference({
			origin_provider: 'curseforge',
			provider_refs: [origin, alias, verifiedOrigin],
		}),
		verifiedOrigin,
	)
})

test('a missing origin reference never borrows an alias from another provider', () => {
	assert.equal(
		contentItemReference({
			origin_provider: 'modrinth',
			provider_refs: [{ provider: 'curseforge', project_id: 123, file_id: 456 }],
		}),
		null,
	)
})

function contentItem(path: string): ContentItem {
	const fileName = path.split('/').pop() ?? path
	return {
		id: 'content',
		file_name: fileName,
		file_path: path,
		size: 1,
		enabled: true,
		project_type: 'mod',
		project: {
			id: 'local:content',
			slug: 'content',
			title: 'Content',
			icon_url: 'C:/icons/content.png',
		},
		version: {
			id: 'local:content',
			version_number: '1.0.0',
			file_name: fileName,
		},
		update: null,
		provider_refs: [],
	} as ContentItem
}

test('toggle updates survive recomputing an icon-bearing display clone', () => {
	const source = contentItem('mods/content.jar')
	const rendered = {
		...source,
		project: { ...source.project!, icon_url: 'asset://localhost/icons/content.png' },
	}

	applyContentItemUpdates([source], rendered, source.file_name, source.file_path, {
		file_name: 'content.jar.disabled',
		file_path: 'mods/content.jar.disabled',
		enabled: false,
	})

	assert.equal(source.enabled, false)
	assert.equal(source.file_name, 'content.jar.disabled')
	assert.equal(source.file_path, 'mods/content.jar.disabled')
	assert.equal(rendered.enabled, false)
	assert.equal(rendered.file_path, 'mods/content.jar.disabled')
})
