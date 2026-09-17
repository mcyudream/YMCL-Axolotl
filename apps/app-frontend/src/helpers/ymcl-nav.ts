// Normalize YAP manifest navigation/pages for the launcher sidebar so the
// rail matches the admin console tree regardless of adapter field dialect.

import type { YmclManifest, YmclNavigationItem, YmclPageDescriptor } from '@/helpers/ymcl'

type RawNav = Record<string, unknown>

/**
 * Sidebar visibility for a nav node the adapter already put in `navigation`.
 * The manifest is the preset: show enabled entries as-is. Do NOT re-check
 * `required_permission` here — that hid admin-configured plugin pages when
 * the local session permission list didn't mirror the adapter's RBAC codes.
 * Data endpoints still enforce permissions server-side.
 */
export function canShowYmclNavItem(item: {
	enabled?: boolean
	disabled?: boolean
}): boolean {
	if (item.disabled === true) return false
	if (item.enabled === false) return false
	return true
}

function asString(value: unknown): string | null {
	return typeof value === 'string' && value.trim() ? value.trim() : null
}

function asBool(value: unknown): boolean | undefined {
	if (typeof value === 'boolean') return value
	return undefined
}

function pickChildren(raw: RawNav): unknown[] {
	const keys = ['children', 'subMenus', 'sub_menus', 'items', 'tabs', 'nodes']
	for (const key of keys) {
		const value = raw[key]
		if (Array.isArray(value)) return value
	}
	return []
}

export function normalizeYmclNavItem(raw: unknown): YmclNavigationItem | null {
	if (!raw || typeof raw !== 'object') return null
	const item = raw as RawNav
	const id = asString(item.id) ?? asString(item.key) ?? asString(item.code)
	if (!id) return null
	// Explicit hide flags from admin dialects.
	if (item.hidden === true || item.visible === false || asString(item.status) === 'hidden') {
		return null
	}
	const children = pickChildren(item)
		.map((child) => normalizeYmclNavItem(child))
		.filter((child): child is YmclNavigationItem => child !== null)
	const sort = typeof item.sort === 'number' ? item.sort : Number(item.sort ?? 0)
	const enabled = asBool(item.enabled)
	const disabled =
		asBool(item.disabled) ??
		(asString(item.status) === 'disabled' ? true : undefined)
	return {
		id,
		type:
			asString(item.type) ??
			asString(item.kind) ??
			(children.length ? 'directory' : 'page'),
		page_id:
			asString(item.page_id) ??
			asString(item.pageId) ??
			asString(item.page) ??
			// Plugin nav entries sometimes reuse the page/dataSource id only.
			(asString(item.type)?.includes('page') || asString(item.kind)?.includes('page')
				? id
				: null),
		route: asString(item.route) ?? asString(item.path),
		title: asString(item.title) ?? asString(item.name) ?? asString(item.label),
		icon: asString(item.icon) ?? asString(item.iconName),
		sort: Number.isFinite(sort) ? sort : 0,
		required_permission:
			asString(item.required_permission) ?? asString(item.requiredPermission),
		enabled,
		disabled,
		children,
	}
}

export function normalizeYmclPage(raw: unknown): YmclPageDescriptor | null {
	if (!raw || typeof raw !== 'object') return null
	const page = raw as RawNav
	const id = asString(page.id)
	if (!id) return null
	const permissions = Array.isArray(page.permissions)
		? page.permissions.filter((value): value is string => typeof value === 'string')
		: []
	return {
		id,
		renderer: asString(page.renderer) ?? asString(page.rendererId),
		title: asString(page.title) ?? asString(page.name),
		data_source:
			asString(page.data_source) ?? asString(page.dataSource) ?? asString(page.dataSourceId),
		permissions,
		params: page.params,
		bundle: page.bundle as YmclPageDescriptor['bundle'],
	}
}

/** Canonical renderer ids used by DomainPageHost. */
export function normalizeRendererId(raw: string | null | undefined): string | null {
	if (!raw) return null
	const key = raw.trim().toLowerCase().replace(/[_\s]+/g, '-')
	const aliases: Record<string, string> = {
		'serverlist': 'server-list',
		'server-list': 'server-list',
		'servers': 'server-list',
		'server': 'server-list',
		'list': 'server-list',
		'cardgrid': 'card-grid',
		'card-grid': 'card-grid',
		'grid': 'card-grid',
		'cards': 'card-grid',
		'activity': 'card-grid',
		'activities': 'card-grid',
		'articlelist': 'article-list',
		'article-list': 'article-list',
		'articles': 'article-list',
		'news': 'article-list',
		'richtext': 'rich-text',
		'rich-text': 'rich-text',
		'markdown': 'rich-text',
		'packcatalog': 'pack-catalog',
		'pack-catalog': 'pack-catalog',
		'packs': 'pack-catalog',
		'stat': 'stats',
		'stats': 'stats',
		'statistics': 'stats',
	}
	return aliases[key] ?? key
}

function pickNavArrays(root: RawNav): unknown[] {
	const candidates = [
		root.navigation,
		root.nav,
		root.menu,
		root.customNavigation,
		root.custom_navigation,
		root.navTree,
		root.nav_tree,
	]
	for (const value of candidates) {
		if (Array.isArray(value)) return value
		if (value && typeof value === 'object') {
			const nested = value as RawNav
			for (const key of ['items', 'nodes', 'tree', 'list', 'children']) {
				if (Array.isArray(nested[key])) return nested[key] as unknown[]
			}
		}
	}
	return []
}

/**
 * Re-normalize a manifest coming off Tauri so camelCase admin dialects and
 * alternate nav child keys still produce a complete navigation tree.
 */
export function normalizeYmclManifest(raw: YmclManifest | null | undefined): YmclManifest | null {
	if (!raw) return null
	const root = raw as unknown as RawNav
	const navSource = pickNavArrays(root)
	const pagesSource = root.pages
	const sourcesSource = [root.data_sources, root.dataSources].find((value) => Array.isArray(value))
	const navigation = navSource
		.map((item) => normalizeYmclNavItem(item))
		.filter((item): item is YmclNavigationItem => item !== null)
	const pages = (Array.isArray(pagesSource) ? pagesSource : [])
		.map((page) => normalizeYmclPage(page))
		.filter((page): page is YmclPageDescriptor => page !== null)
	return {
		...raw,
		navigation,
		pages,
		data_sources: (Array.isArray(sourcesSource) ? sourcesSource : []) as YmclManifest['data_sources'],
	}
}
