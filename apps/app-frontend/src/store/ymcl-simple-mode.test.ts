import assert from 'node:assert/strict'
import { registerHooks } from 'node:module'
import test, { type TestContext } from 'node:test'

import { createPinia } from 'pinia'

import type { YmclManifest } from '../helpers/ymcl.ts'

// The domain mapping also exports UI icons; the store only consumes its route mapping.
const hooks = registerHooks({
	resolve(specifier, context, nextResolve) {
		if (specifier === '@modrinth/assets') {
			const icons = [
				'BookTextIcon',
				'BoxIcon',
				'CalendarIcon',
				'ChartIcon',
				'CompassIcon',
				'GridIcon',
				'ListIcon',
				'ServerIcon',
			]
			return {
				url: `data:text/javascript,${encodeURIComponent(`export const ${icons.map((icon) => `${icon}=null`).join(',')}`)}`,
				shortCircuit: true,
			}
		}
		if (specifier.startsWith('@/'))
			return nextResolve(new URL(`../${specifier.slice(2)}.ts`, import.meta.url).href, context)
		return nextResolve(specifier, context)
	},
})
const { useYmclStore } = await import('./ymcl.ts')
const { ymcl } = await import('../helpers/ymcl.ts')
hooks.deregister()

function mockRuntime(t: TestContext, initialDomains: string[] = []) {
	const unlocked = new Set(initialDomains)
	t.mock.method(ymcl, 'unlockSimpleMode', async (domainId: string) => {
		unlocked.add(domainId)
		return [...unlocked]
	})
	t.mock.method(ymcl, 'simpleModeUnlocked', async () => [...unlocked])
}

function manifest(enabled = true): YmclManifest {
	return {
		protocol_version: 1,
		navigation: [
			{ id: 'home', type: 'native', route: '/' },
			{ id: 'settings', type: 'native', route: '/settings' },
		],
		pages: [],
		data_sources: [],
		simple_mode: { enabled, hiddenScopes: ['settings'], hiddenEntrances: ['domain-switcher'] },
	}
}

test('unlock restores domain routes, navigation and entrances and never toggles back', async (t) => {
	mockRuntime(t)
	const store = useYmclStore(createPinia())
	store.activeDomainId = 'domain-a'
	store.setManifest(manifest())
	assert.ok(store.simpleModeActive)
	assert.ok(store.isRouteHidden('/settings'))
	assert.ok(store.isEntranceHidden('domain-switcher'))
	assert.deepEqual(
		store.navigation.map((item) => item.id),
		['home'],
	)
	await store.unlockSimpleMode()
	await store.unlockSimpleMode()
	assert.equal(store.simpleModeActive, false)
	assert.equal(store.isRouteHidden('/settings'), false)
	assert.equal(store.isEntranceHidden('domain-switcher'), false)
	assert.deepEqual(
		store.navigation.map((item) => item.id),
		['home', 'settings'],
	)
	assert.deepEqual(store.unlockedSimpleModeDomains, ['domain-a'])
})

test('unlock is isolated by domain and survives switching back and manifest refresh', async (t) => {
	mockRuntime(t)
	const store = useYmclStore(createPinia())
	store.activeDomainId = 'domain-a'
	store.setManifest(manifest())
	await store.unlockSimpleMode()
	store.applyDomainsState({ domains: [], active_domain_id: 'domain-b' })
	assert.equal(store.manifest, null)
	store.setManifest(manifest())
	assert.ok(store.simpleModeActive)
	store.applyDomainsState({ domains: [], active_domain_id: 'domain-a' })
	store.setManifest(manifest())
	assert.equal(store.simpleModeActive, false)
	store.setManifest({
		...manifest(),
		simple_mode: { enabled: true, hiddenScopes: ['domain-pages'] },
	})
	assert.equal(store.simpleModeActive, false)
})

test('a fresh launcher run resets unlocks; personal and disabled domains remain unaffected', async (t) => {
	mockRuntime(t)
	const previous = useYmclStore(createPinia())
	previous.activeDomainId = 'domain-a'
	previous.setManifest(manifest())
	await previous.unlockSimpleMode()
	const fresh = useYmclStore(createPinia())
	fresh.activeDomainId = 'domain-a'
	fresh.setManifest(manifest())
	assert.ok(fresh.simpleModeActive)
	fresh.activeDomainId = 'personal'
	assert.equal(fresh.simpleModeActive, false)
	assert.equal(fresh.isRouteHidden('/settings'), false)
	fresh.activeDomainId = 'domain-b'
	fresh.setManifest(manifest(false))
	await fresh.unlockSimpleMode()
	assert.equal(fresh.simpleModeActive, false)
	assert.deepEqual(fresh.unlockedSimpleModeDomains, [])
})

test('recreating a WebView restores process-owned unlocks during initialization', async (t) => {
	mockRuntime(t, ['domain-a'])
	t.mock.method(ymcl, 'domainsState', async () => ({ domains: [], active_domain_id: 'domain-a' }))
	t.mock.method(ymcl, 'refreshManifest', async () => manifest())
	t.mock.method(ymcl, 'session', async () => null)
	const recreated = useYmclStore(createPinia())
	await recreated.init()
	assert.equal(recreated.simpleModeActive, false)
	assert.deepEqual(recreated.unlockedSimpleModeDomains, ['domain-a'])
})

test('a stale initialization snapshot cannot relock a domain unlocked while startup was pending', async (t) => {
	mockRuntime(t)
	let finishStartup!: (value: { domains: []; active_domain_id: string }) => void
	t.mock.method(
		ymcl,
		'domainsState',
		() => new Promise((resolve) => { finishStartup = resolve }),
	)
	t.mock.method(ymcl, 'activateDomain', async (id: string) => ({ domains: [], active_domain_id: id }))
	t.mock.method(ymcl, 'refreshManifest', async () => manifest())
	t.mock.method(ymcl, 'session', async () => null)
	const store = useYmclStore(createPinia())
	const pending = store.init()
	await store.activateDomain('domain-a')
	await store.unlockSimpleMode()
	assert.equal(store.simpleModeActive, false)
	finishStartup({ domains: [], active_domain_id: 'personal' })
	await pending
	assert.equal(store.activeDomainId, 'domain-a')
	assert.equal(store.simpleModeActive, false)
	assert.deepEqual(store.unlockedSimpleModeDomains, ['domain-a'])
})

test('unlocking presentation never dismisses the domain authentication requirement', async (t) => {
	mockRuntime(t)
	const store = useYmclStore(createPinia())
	store.activeDomainId = 'domain-a'
	store.domains = [
		{
			id: 'domain-a',
			display_name: 'A',
			is_personal: false,
			is_active: true,
			capabilities: {
				protocol_version: 1,
				capabilities: [],
				auth: { required: true, methods: [] },
			},
		},
	]
	store.setManifest(manifest())
	assert.ok(store.loginRequired)
	await store.unlockSimpleMode()
	assert.equal(store.simpleModeActive, false)
	assert.ok(store.loginRequired)
})

test('a late manifest response from the previous domain cannot replace the current policy', async (t) => {
	let finishOldRequest!: (value: YmclManifest) => void
	const oldResponse = new Promise<YmclManifest>((resolve) => {
		finishOldRequest = resolve
	})
	let first = true
	t.mock.method(ymcl, 'refreshManifest', () => {
		if (first) {
			first = false
			return oldResponse
		}
		return Promise.resolve(manifest(false))
	})
	t.mock.method(ymcl, 'activateDomain', async (id: string) => ({
		domains: [],
		active_domain_id: id,
	}))
	t.mock.method(ymcl, 'session', async () => null)
	const store = useYmclStore(createPinia())
	store.activeDomainId = 'domain-a'
	store.setManifest(manifest())
	const pending = store.refreshManifest()
	await store.activateDomain('domain-b')
	finishOldRequest({ ...manifest(), simple_mode: { enabled: true, unlockKey: 'F9' } })
	await pending
	assert.equal(store.activeDomainId, 'domain-b')
	assert.equal(store.simpleMode.enabled, false)
	assert.equal(store.simpleMode.unlockKey, 'F12')
})
