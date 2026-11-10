import assert from 'node:assert/strict'
import test from 'node:test'

import type { YmclNavigationItem } from './ymcl.ts'
import {
	filterSimpleModeNavigation,
	isSimpleModeRouteHidden,
	matchesSimpleModeUnlockKey,
	normalizeSimpleMode,
	simpleModeHiddenRoutes,
} from './ymcl-simple-mode.ts'

const navTarget = (item: YmclNavigationItem) =>
	item.type === 'native'
		? (item.route ?? '/')
		: `/domain/${encodeURIComponent(item.page_id ?? item.id)}`

test('missing configuration stays disabled; enabling alone uses F12 and conservative hiding defaults', () => {
	assert.equal(normalizeSimpleMode(undefined).enabled, false)
	const profile = normalizeSimpleMode({ enabled: true })
	assert.equal(profile.unlockKey, 'F12')
	assert.ok(profile.hiddenScopes.includes('domain-pages'))
	assert.ok(profile.hiddenEntrances.includes('domain-switcher'))
	assert.ok(!profile.hiddenEntrances.includes('navigation'))
	assert.deepEqual(
		normalizeSimpleMode({
			enabled: 'true',
			unlockKey: 'F13',
			hiddenScopes: [],
			hiddenEntrances: [],
		}),
		{
			enabled: false,
			unlockKey: 'F12',
			hiddenScopes: [],
			hiddenEntrances: [],
			hiddenNavigationIds: [],
			hiddenRoutes: [],
		},
	)
	assert.deepEqual(
		normalizeSimpleMode({ hiddenScopes: ['skins', 'skins', 'unknown', 5], unlockKey: ' f9 ' })
			.hiddenScopes,
		['skins'],
	)
	assert.equal(normalizeSimpleMode({ unlockKey: ' f9 ' }).unlockKey, 'F9')
})

test('scope routes cover descendants and instance subpages without matching neighbouring paths', () => {
	const profile = normalizeSimpleMode({
		enabled: true,
		hiddenScopes: ['worlds', 'screenshots', 'lab'],
	})
	const routes = simpleModeHiddenRoutes(profile, [], navTarget)
	for (const path of [
		'/worlds',
		'/worlds/',
		'/WORLDS',
		'/lab/skin-editor?tab=edit#selection',
		'/instance/a/worlds/save/edit',
		'/INSTANCE/a%2Fb/WORLDS',
		'/instance/a/screenshots',
	]) {
		assert.ok(isSimpleModeRouteHidden(path, routes), path)
	}
	for (const path of [
		'/',
		'/instance/a',
		'/instance/a/logs',
		'/labelling',
		'/screenshots-old',
		'/worldscape',
	]) {
		assert.equal(isSimpleModeRouteHidden(path, routes), false, path)
	}
	assert.equal(isSimpleModeRouteHidden('/', ['/']), false)
	assert.ok(isSimpleModeRouteHidden('/INSTANCE/Case/LOGS', ['/instance/Case/logs']))
	assert.equal(isSimpleModeRouteHidden('/instance/case/logs', ['/instance/Case/logs']), false)
})

test('hidden domain pages also hide their data-source aliases in either navigation direction', () => {
	const pages = [{ id: 'mc.servers', data_source: 'minecraft-server.servers' }]
	for (const pageId of ['mc.servers', 'minecraft-server.servers']) {
		const navigation: YmclNavigationItem[] = [{ id: 'servers', type: 'page', page_id: pageId }]
		const profile = normalizeSimpleMode({
			enabled: true,
			hiddenScopes: [],
			hiddenNavigationIds: ['servers'],
		})
		const hidden = simpleModeHiddenRoutes(profile, navigation, navTarget, pages)
		assert.ok(isSimpleModeRouteHidden('/domain/mc.servers', hidden))
		assert.ok(isSimpleModeRouteHidden('/DOMAIN/minecraft-server.servers?tab=list', hidden))
		assert.equal(isSimpleModeRouteHidden('/domain/MC.servers', hidden), false)
		assert.equal(isSimpleModeRouteHidden('/domain/other-page', hidden), false)
	}
})

test('encoded page identifiers stay distinct segments and are decoded only once', () => {
	const pageId = 'mc/servers%2F?active#all'
	const alias = 'source/servers%2F?active#all'
	const profile = normalizeSimpleMode({
		enabled: true,
		hiddenScopes: [],
		hiddenNavigationIds: ['servers'],
	})
	const hidden = simpleModeHiddenRoutes(
		profile,
		[{ id: 'servers', type: 'page', page_id: pageId }],
		navTarget,
		[{ id: pageId, data_source: alias }],
	)
	assert.ok(isSimpleModeRouteHidden(`/domain/${encodeURIComponent(alias)}`, hidden))
	assert.equal(isSimpleModeRouteHidden('/domain/mc%2Fservers', ['/domain/mc']), false)
	assert.equal(isSimpleModeRouteHidden('/domain/mc%252Fservers', ['/domain/mc%2Fservers']), false)
})

test('selected page IDs remain hidden when a disabled entry is omitted from navigation', () => {
	const profile = normalizeSimpleMode({
		enabled: true,
		hiddenScopes: [],
		hiddenNavigationIds: ['mc.servers'],
	})
	const hidden = simpleModeHiddenRoutes(profile, [], navTarget, [
		{ id: 'mc.servers', data_source: 'minecraft-server.servers' },
	])
	assert.ok(isSimpleModeRouteHidden('/domain/mc.servers', hidden))
	assert.ok(isSimpleModeRouteHidden('/domain/minecraft-server.servers', hidden))
})

test('domain-resolved routes hide disabled directories and native pages omitted from the manifest', () => {
	const profile = normalizeSimpleMode({
		enabled: true,
		hiddenScopes: [],
		hiddenNavigationIds: ['tools', 'skins'],
		hiddenRoutes: ['/domain/mc.servers', '/skins'],
	})
	const hidden = simpleModeHiddenRoutes(profile, [], navTarget, [
		{ id: 'mc.servers', data_source: 'minecraft-server.servers' },
	])
	assert.ok(isSimpleModeRouteHidden('/domain/mc.servers', hidden))
	assert.ok(isSimpleModeRouteHidden('/domain/minecraft-server.servers', hidden))
	assert.ok(isSimpleModeRouteHidden('/SKINS', hidden))
	assert.equal(isSimpleModeRouteHidden('/settings', hidden), false)
})

test('hiding a directory suppresses every descendant and its alternate route', () => {
	const navigation: YmclNavigationItem[] = [
		{ id: 'home', type: 'native', route: '/' },
		{
			id: 'tools',
			type: 'group',
			children: [
				{ id: 'nested', type: 'group', children: [{ id: 'mc.tasks', type: 'page' }] },
				{ id: 'skins', type: 'native', route: '/skins' },
			],
		},
		{ id: 'other-link', type: 'page', page_id: 'mc.tasks' },
		{ id: 'news', type: 'page' },
	]
	const original = structuredClone(navigation)
	const profile = normalizeSimpleMode({
		enabled: true,
		hiddenScopes: [],
		hiddenNavigationIds: ['tools'],
	})
	const hidden = simpleModeHiddenRoutes(profile, navigation, navTarget)
	assert.ok(isSimpleModeRouteHidden('/domain/mc.tasks', hidden))
	assert.ok(isSimpleModeRouteHidden('/skins', hidden))
	assert.deepEqual(
		filterSimpleModeNavigation(navigation, profile, hidden, navTarget).map((item) => item.id),
		['home', 'news'],
	)
	assert.deepEqual(navigation, original)
})

test('empty directories disappear when all child routes are hidden', () => {
	const navigation: YmclNavigationItem[] = [
		{ id: 'pages', type: 'group', children: [{ id: 'mc.news', type: 'page' }] },
		{ id: 'home', type: 'native', route: '/' },
	]
	const profile = normalizeSimpleMode({ enabled: true, hiddenScopes: ['domain-pages'] })
	const hidden = simpleModeHiddenRoutes(profile, navigation, navTarget)
	assert.deepEqual(
		filterSimpleModeNavigation(navigation, profile, hidden, navTarget).map((item) => item.id),
		['home'],
	)
	assert.equal(filterSimpleModeNavigation(navigation, profile, [], navTarget).length, 2)
})

test('only the configured plain function key unlocks, including input and dialog events', () => {
	const event = {
		key: 'F12',
		code: 'F12',
		ctrlKey: false,
		metaKey: false,
		altKey: false,
		shiftKey: false,
		isComposing: false,
	}
	assert.ok(matchesSimpleModeUnlockKey(event, 'F12'))
	assert.equal(matchesSimpleModeUnlockKey(event, 'F9'), false)
	for (const modifier of ['ctrlKey', 'metaKey', 'altKey', 'shiftKey', 'isComposing']) {
		assert.equal(matchesSimpleModeUnlockKey({ ...event, [modifier]: true }, 'F12'), false, modifier)
	}
})
