// Shared UI mappings for domain adapter surfaces (YAP §6.5/§6.6): renderer
// icons, domain page routes, and page-shortcut target parsing. Used by the
// home dashboard widgets and the widget picker.

import {
	BookTextIcon,
	BoxIcon,
	CalendarIcon,
	ChartIcon,
	CompassIcon,
	GridIcon,
	ListIcon,
	ServerIcon,
} from '@modrinth/assets'
import type { Component } from 'vue'

import { canShowYmclNavItem, normalizeRendererId } from '@/helpers/ymcl-nav'

/** Icon for a built-in declarative renderer id (unknown ids → compass). */
export const DOMAIN_RENDERER_ICONS: Record<string, Component> = {
	'server-list': ServerIcon,
	'card-grid': GridIcon,
	'article-list': BookTextIcon,
	'rich-text': BookTextIcon,
	'pack-catalog': BoxIcon,
	stats: ChartIcon,
}

/** Well-known dataSource code → rail icon when the page has no nav icon. */
const DATA_SOURCE_CODE_ICONS: Record<string, Component> = {
	servers: ServerIcon,
	'servers.list': ServerIcon,
	activities: CalendarIcon,
	activity: CalendarIcon,
	'my-tasks': ListIcon,
	task: ListIcon,
	tasks: ListIcon,
	'claimable-tasks': ListIcon,
	'my-stats': ChartIcon,
	stats: ChartIcon,
}

export function domainRendererIcon(renderer: string | null | undefined): Component {
	const id = normalizeRendererId(renderer)
	return (id && DOMAIN_RENDERER_ICONS[id]) || CompassIcon
}

export function dataSourceNavIcon(
	sourceCode: string | null | undefined,
	renderer?: string | null,
): Component {
	const code = sourceCode?.trim() ?? ''
	if (code && DATA_SOURCE_CODE_ICONS[code]) return DATA_SOURCE_CODE_ICONS[code]
	return domainRendererIcon(renderer)
}

export function domainPageRoute(pageId: string): string {
	return `/domain/${encodeURIComponent(pageId)}`
}

/** Launcher native routes for domain nav items that omit `route`. */
export const YMCL_NATIVE_ROUTE_FALLBACK: Record<string, string> = {
	home: '/',
	homepage: '/',
	'首页': '/',
	discover: '/browse',
	explore: '/browse',
	'发现': '/browse',
	'发现内容': '/browse',
	library: '/library',
	instances: '/library',
	'实例库': '/library',
	skins: '/skins',
	skin: '/skins',
	'皮肤': '/skins',
	lab: '/lab',
	'实验': '/lab',
	'实验内容': '/lab',
	downloads: '/downloads',
	'下载': '/downloads',
	'下载管理': '/downloads',
	settings: '/settings',
	'设置': '/settings',
	worlds: '/worlds',
	'世界': '/worlds',
}

export function ymclNativeRoute(item: {
	id: string
	title?: string | null
	route?: string | null
}): string {
	if (item.route?.trim()) return item.route.trim()
	const keys = [item.id, item.title?.trim() ?? '']
	for (const key of keys) {
		const mapped = YMCL_NATIVE_ROUTE_FALLBACK[key]
		if (mapped) return mapped
		const lower = key.toLowerCase()
		const mappedLower = YMCL_NATIVE_ROUTE_FALLBACK[lower]
		if (mappedLower) return mappedLower
	}
	return '/'
}

export function ymclNavTo(item: {
	type: string
	route?: string | null
	page_id?: string | null
	id: string
	title?: string | null
}): string {
	const type = (item.type || '').toLowerCase()
	if (type === 'native' || type === 'link' || type === 'route') return ymclNativeRoute(item)
	// page / directory-with-target / unknown → domain page route
	return domainPageRoute(item.page_id ?? item.id)
}

/** Admin "已停用" hides the entry. Manifest navigation is the preset — no local RBAC re-filter. */
export function isYmclNavVisible(item: { enabled?: boolean; disabled?: boolean }): boolean {
	return canShowYmclNavItem(item)
}

const YMCL_NAV_DIRECTORY_TYPES = new Set([
	'directory',
	'catalog',
	'group',
	'folder',
	'menu',
	'section',
])

export function isYmclNavDirectory(item: {
	type: string
	route?: string | null
	page_id?: string | null
}): boolean {
	if (YMCL_NAV_DIRECTORY_TYPES.has(item.type)) return true
	if (item.type === 'separator') return false
	// Pure container: no native route and no page target.
	return item.type !== 'native' && !item.route && !item.page_id && item.type !== 'page'
}

/** Item navigates somewhere on click (explicit route/page, or a known native alias). */
export function ymclNavHasTarget(item: {
	id: string
	type: string
	route?: string | null
	page_id?: string | null
	title?: string | null
}): boolean {
	return (
		Boolean(item.route) ||
		Boolean(item.page_id) ||
		item.type === 'page' ||
		item.type === 'native' ||
		Boolean(YMCL_NATIVE_ROUTE_FALLBACK[item.id]) ||
		Boolean(item.title && YMCL_NATIVE_ROUTE_FALLBACK[item.title])
	)
}

export type YmclShortcutTarget =
	{ kind: 'page'; pageId: string } | { kind: 'native'; route: string }

/**
 * Parses a YAP page-shortcut target: "page:<pageId>" or "native:<route>".
 * Returns null for unknown formats so callers can render an unavailable state.
 */
export function parseShortcutTarget(target: string): YmclShortcutTarget | null {
	if (target.startsWith('page:')) {
		const pageId = target.slice('page:'.length)
		return pageId ? { kind: 'page', pageId } : null
	}
	if (target.startsWith('native:')) {
		const route = target.slice('native:'.length)
		return route ? { kind: 'native', route } : null
	}
	return null
}
