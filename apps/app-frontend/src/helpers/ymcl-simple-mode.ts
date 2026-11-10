import type { YmclNavigationItem, YmclPageDescriptor } from './ymcl'

export const SIMPLE_MODE_SCOPES = {
	'domain-pages': ['/domain'],
	discover: ['/browse', '/project'],
	library: ['/library', '/instance'],
	skins: ['/skins'],
	worlds: ['/worlds', '/instance/:instanceId/worlds'],
	screenshots: ['/screenshots', '/instance/:instanceId/screenshots'],
	lab: ['/lab'],
	downloads: ['/downloads'],
	settings: ['/settings'],
	'create-instance': ['/create'],
} as const

export type SimpleModeScope = keyof typeof SIMPLE_MODE_SCOPES
export const SIMPLE_MODE_ENTRANCES = [
	'navigation',
	'quick-instances',
	'create-instance',
	'settings',
	'account-sidebar',
	'domain-switcher',
	'home-customization',
] as const
export type SimpleModeEntrance = (typeof SIMPLE_MODE_ENTRANCES)[number]

/** Domain-owned presentation policy; unlocking never grants server permissions. */
export interface YmclSimpleModeProfile {
	enabled: boolean
	unlockKey: string
	hiddenScopes: SimpleModeScope[]
	hiddenEntrances: SimpleModeEntrance[]
	hiddenNavigationIds: string[]
	/** Resolved by the domain from its full, unfiltered navigation configuration. */
	hiddenRoutes: string[]
}

const DEFAULT_ENTRANCES: SimpleModeEntrance[] = [
	'quick-instances',
	'create-instance',
	'settings',
	'account-sidebar',
	'domain-switcher',
	'home-customization',
]

function stringList(value: unknown, fallback: readonly string[]): string[] {
	return Array.isArray(value)
		? [
				...new Set(
					value
						.filter((item): item is string => typeof item === 'string')
						.map((item) => item.trim())
						.filter(Boolean),
				),
			]
		: [...fallback]
}

export function normalizeSimpleMode(raw: unknown): YmclSimpleModeProfile {
	const value = raw && typeof raw === 'object' ? (raw as Record<string, unknown>) : {}
	const key = typeof value.unlockKey === 'string' ? value.unlockKey.trim().toUpperCase() : ''
	return {
		enabled: value.enabled === true,
		unlockKey: /^F(?:[1-9]|1[0-2])$/.test(key) ? key : 'F12',
		hiddenScopes: stringList(value.hiddenScopes, Object.keys(SIMPLE_MODE_SCOPES)).filter(
			(scope): scope is SimpleModeScope => Object.keys(SIMPLE_MODE_SCOPES).includes(scope),
		),
		hiddenEntrances: stringList(value.hiddenEntrances, DEFAULT_ENTRANCES).filter(
			(entrance): entrance is SimpleModeEntrance =>
				(SIMPLE_MODE_ENTRANCES as readonly string[]).includes(entrance),
		),
		hiddenNavigationIds: stringList(value.hiddenNavigationIds, []),
		hiddenRoutes: stringList(value.hiddenRoutes, []).filter((route) => route.startsWith('/')),
	}
}

function routeSegments(route: string): string[] {
	const path = route.split(/[?#]/, 1)[0].replace(/\/+$/, '') || '/'
	return path.split('/').map((segment) => {
		try {
			return decodeURIComponent(segment)
		} catch {
			return segment
		}
	})
}

/** Matches route segments, keeping /labelling distinct from /lab. Home stays reachable. */
export function isSimpleModeRouteHidden(route: string, hiddenRoutes: readonly string[]): boolean {
	const segments = routeSegments(route)
	if (segments.length === 2 && segments[1] === '') return false
	return hiddenRoutes.some((route) => {
		const hidden = routeSegments(route)
		if (hidden.length === 2 && hidden[1] === '') return false
		const root = hidden[1]?.toLowerCase()
		return hidden.every((segment, index) => {
			const value = segments[index]
			if (root === 'instance' && index === 2 && segment === ':instanceId') return Boolean(value)
			const isParameter = (root === 'domain' && index >= 2) || (root === 'instance' && index === 2)
			return isParameter ? segment === value : segment.toLowerCase() === value?.toLowerCase()
		})
	})
}

type NavTarget = (item: YmclNavigationItem) => string

function hasNavTarget(item: YmclNavigationItem): boolean {
	return Boolean(
		item.route || item.page_id || ['page', 'native', 'link', 'route'].includes(item.type),
	)
}

/** A hidden directory also hides its descendants from alternate navigation paths. */
export function simpleModeHiddenRoutes(
	profile: YmclSimpleModeProfile,
	navigation: readonly YmclNavigationItem[],
	target: NavTarget,
	pages: readonly Pick<YmclPageDescriptor, 'id' | 'data_source'>[] = [],
): string[] {
	const routes: string[] = [
		...profile.hiddenScopes.flatMap((scope) => [...SIMPLE_MODE_SCOPES[scope]]),
		...profile.hiddenRoutes,
	]
	const hiddenIds = new Set(profile.hiddenNavigationIds)
	const visit = (items: readonly YmclNavigationItem[], parentHidden = false) => {
		for (const item of items) {
			const hidden = parentHidden || hiddenIds.has(item.id)
			if (hidden && hasNavTarget(item)) routes.push(target(item))
			visit(item.children ?? [], hidden)
		}
	}
	visit(navigation)
	// Disabled navigation may be omitted from the manifest while its page remains routable.
	for (const id of hiddenIds) {
		const page = pages.find((page) => page.id === id)
		if (page) routes.push(target({ id, type: 'page' }))
	}
	const resolvePage = (id: string) =>
		pages.find((page) => page.id === id) ?? pages.find((page) => page.data_source === id)
	for (const route of [...routes]) {
		const segments = routeSegments(route)
		if (segments.length !== 3 || segments[1].toLowerCase() !== 'domain') continue
		const page = resolvePage(segments[2])
		if (!page) continue
		routes.push(`/domain/${encodeURIComponent(page.id)}`)
		if (page.data_source && resolvePage(page.data_source) === page)
			routes.push(`/domain/${encodeURIComponent(page.data_source)}`)
	}
	return [...new Set(routes)]
}

export function filterSimpleModeNavigation(
	navigation: readonly YmclNavigationItem[],
	profile: YmclSimpleModeProfile,
	hiddenRoutes: readonly string[],
	target: NavTarget,
): YmclNavigationItem[] {
	const hiddenIds = new Set(profile.hiddenNavigationIds)
	const visit = (items: readonly YmclNavigationItem[]): YmclNavigationItem[] =>
		items.flatMap((item) => {
			if (
				hiddenIds.has(item.id) ||
				(hasNavTarget(item) && isSimpleModeRouteHidden(target(item), hiddenRoutes))
			)
				return []
			const children = visit(item.children ?? [])
			if (
				item.children?.length &&
				!children.length &&
				!item.route &&
				!item.page_id &&
				item.type !== 'page' &&
				item.type !== 'native'
			)
				return []
			return [{ ...item, children }]
		})
	return visit(navigation)
}

/** Plain function keys remain usable while entering text or while a dialog is open. */
export function matchesSimpleModeUnlockKey(
	event: Pick<
		KeyboardEvent,
		'key' | 'code' | 'ctrlKey' | 'metaKey' | 'altKey' | 'shiftKey' | 'isComposing'
	>,
	key: string,
): boolean {
	return (
		!event.isComposing &&
		!event.ctrlKey &&
		!event.metaKey &&
		!event.altKey &&
		!event.shiftKey &&
		(event.code === key || event.key === key)
	)
}
