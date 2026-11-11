import assert from 'node:assert/strict'
import { registerHooks } from 'node:module'
import test, { type TestContext } from 'node:test'

import { createApp, effectScope, nextTick, ref } from 'vue'

import type { ContentItem } from '../types.ts'

const hooks = registerHooks({
	resolve(specifier, context, nextResolve) {
		try {
			return nextResolve(specifier, context)
		} catch (error) {
			if (
				(error as NodeJS.ErrnoException).code === 'ERR_MODULE_NOT_FOUND' &&
				(specifier.startsWith('.') || specifier.startsWith('#ui/'))
			) {
				return nextResolve(`${specifier}.ts`, context)
			}
			throw error
		}
	},
})

Object.defineProperty(globalThis, 'window', { value: {}, configurable: true })
const { I18N_INJECTION_KEY } = await import('../../../../providers/i18n.ts')
const { useContentPipeline } = await import('./content-pipeline.ts')
const { useContentSelection } = await import('./content-selection.ts')
const { canUpdateContentItem, getUpdatableContentItems } = await import('./content-filtering.ts')
hooks.deregister()

function contentItem(
	id: string,
	ownership: ContentItem['instanceOwnershipKind'],
	update: ContentItem['update'],
): ContentItem {
	return {
		id,
		file_name: `${id}.jar`,
		file_path: `mods/${id}.jar`,
		project_type: 'mod',
		enabled: true,
		instanceOwnershipKind: ownership,
		instanceMemberId: ownership === 'pack_managed' ? `member:${id}` : undefined,
		instanceOverrideKind: 'none',
		instanceMaterializationState: 'present',
		origin_provider: update?.provider ?? null,
		provider_refs: [],
		update,
	}
}

function modrinthUpdate(id: string): ContentItem['update'] {
	return {
		provider: 'modrinth',
		project_id: id,
		current_version_id: 'old',
		target_version_id: 'new',
	}
}

function setupPipeline(t: TestContext, regular: ContentItem[], pack: ContentItem[]) {
	t.mock.timers.enable({ apis: ['setTimeout'] })
	const app = createApp({})
	app.provide(I18N_INJECTION_KEY, {
		locale: ref('en-US'),
		t: (key) => key,
		setLocale: () => {},
	})
	const scope = effectScope()
	t.after(() => scope.stop())
	const items = ref(regular)
	const modpackItems = ref(pack)
	const pipeline = app.runWithContext(() =>
		scope.run(() =>
			useContentPipeline({
				items,
				modpackItems,
				duplicateItems: ref([]),
				sortItems: (items) => [...items].sort((a, b) => a.id.localeCompare(b.id)),
				getItemId: (item) => item.id,
				showUpdateFilter: true,
			}),
		),
	)!
	async function flush() {
		await nextTick()
		t.mock.timers.tick(100)
		await nextTick()
	}
	return { pipeline, items, modpackItems, scope, flush }
}

test('retains MRPACK member updates, ownership and update-filter counts', async (t) => {
	const packUpdate = modrinthUpdate('pack-mod')
	const { pipeline, flush } = setupPipeline(
		t,
		[contentItem('user-mod', 'user_added', modrinthUpdate('user-mod'))],
		[
			contentItem('pack-mod', 'pack_managed', packUpdate),
			contentItem('pack-current', 'pack_managed', null),
		],
	)
	await flush()

	const member = pipeline.filteredModpackItems.value.find((item) => item.id === 'pack-mod')!
	assert.deepEqual(member.update, packUpdate)
	assert.equal(member.instanceOwnershipKind, 'pack_managed')
	assert.equal(member.instanceMemberId, 'member:pack-mod')
	assert.equal(member.instanceOverrideKind, 'none')
	assert.equal(pipeline.filterCounts.value.updates, 2)
	assert.ok(pipeline.row2FilterOptions.value.some((option) => option.id === 'updates'))

	pipeline.toggleStatusFilter('updates')
	await flush()
	assert.deepEqual(
		pipeline.filteredModpackItems.value.map((item) => item.id),
		['pack-mod'],
	)
	assert.deepEqual(
		pipeline.filteredItems.value.map((item) => item.id),
		['user-mod'],
	)
})

test('keeps update filtering when all available updates belong to the pack group', async (t) => {
	const { pipeline, flush } = setupPipeline(
		t,
		[],
		[contentItem('pack-mod', 'pack_managed', modrinthUpdate('pack-mod'))],
	)
	await flush()
	pipeline.toggleStatusFilter('updates')
	await flush()

	assert.deepEqual(pipeline.selectedStatusFilters.value, ['updates'])
	assert.equal(pipeline.filterCounts.value.updates, 1)
	assert.equal(pipeline.filteredModpackItems.value.length, 1)

	pipeline.searchQuery.value = 'pack-mod'
	await flush()
	assert.equal(pipeline.filteredModpackItems.value.length, 1)
	assert.ok(pipeline.filteredModpackItems.value[0].update)
})

test('pack updates remain available for selected-item bulk updates', async (t) => {
	const update: ContentItem['update'] = {
		provider: 'curseforge',
		project_id: 1,
		current_file_id: 10,
		target_file_id: 11,
	}
	const { pipeline, scope, flush } = setupPipeline(
		t,
		[],
		[contentItem('pack-mod', 'pack_managed', update)],
	)
	const selection = scope.run(() =>
		useContentSelection(pipeline.sortedModpackItems, (item) => item.id),
	)!
	await flush()
	selection.selectedIds.value = ['pack-mod']

	const updatable = selection.selectedItems.value.filter(canUpdateContentItem)
	assert.equal(updatable.length, 1)
	assert.deepEqual(updatable[0].update, update)
	assert.equal(updatable[0].instanceMemberId, 'member:pack-mod')
})

test('refreshing pack member data updates the filter count', async (t) => {
	const { pipeline, modpackItems, flush } = setupPipeline(
		t,
		[],
		[contentItem('pack-mod', 'pack_managed', null)],
	)
	await flush()
	assert.equal(pipeline.filterCounts.value.updates, 0)

	modpackItems.value = [contentItem('pack-mod', 'pack_managed', modrinthUpdate('pack-mod'))]
	await flush()
	assert.equal(pipeline.filterCounts.value.updates, 1)
	assert.ok(pipeline.filteredModpackItems.value[0].update)
})

test('bulk update candidates include pack mods and resources once across both groups', () => {
	const mod = contentItem('pack-mod', 'pack_managed', modrinthUpdate('pack-mod'))
	const resource = {
		...contentItem('pack-resource', 'pack_managed', modrinthUpdate('pack-resource')),
		project_type: 'resourcepack' as const,
		file_name: 'pack-resource.zip',
		file_path: 'resourcepacks/pack-resource.zip',
	}
	const added = contentItem('added-mod', 'user_added', modrinthUpdate('added-mod'))
	const candidates = getUpdatableContentItems([mod, resource, added, { ...mod }], (item) => item.id)

	assert.deepEqual(
		candidates.map((item) => item.id),
		['pack-mod', 'pack-resource', 'added-mod'],
	)
	assert.equal(candidates[1].instanceMemberId, 'member:pack-resource')
})

test('bulk updates exclude absent and unsupported items while retaining disabled installed mods', () => {
	const mod = contentItem('pack-mod', 'pack_managed', modrinthUpdate('pack-mod'))
	const candidates = getUpdatableContentItems(
		[
			{ ...mod, id: 'disabled', enabled: false },
			{ ...mod, id: 'missing', instanceMaterializationState: 'missing' },
			{ ...mod, id: 'removed', instanceMaterializationState: 'removed' },
			{ ...mod, id: 'pending', instanceMaterializationState: 'pending_manual' },
			{
				...mod,
				id: 'unsupported',
				instanceCapabilities: {
					canToggle: true,
					canDelete: true,
					canUpdate: false,
					canChangeVersion: false,
					canRestorePackDefault: false,
				},
			},
			{ ...mod, id: 'current', update: null },
		],
		(item) => item.id,
	)

	assert.deepEqual(
		candidates.map((item) => item.id),
		['disabled'],
	)
})
