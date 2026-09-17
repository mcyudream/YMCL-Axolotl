// YAP §6.5 member override (unlocked domain homes): members may locally
// reorder/hide/add widgets on top of the domain-hosted layout. Overrides are
// stored per domain in localStorage — frontend-only, no settings migration.

import {
	HOME_DASHBOARD_VERSION,
	type HomeDashboardConfig,
	type HomeWidgetLayout,
	type HomeWidgetPlacement,
	normalizeHomeDashboard,
} from '@/components/home/home-dashboard'

export interface YmclHomeOverride {
	version: number
	layout: HomeWidgetLayout
	/** The member's full widget list (domain widgets in edited order plus their own). */
	widgets: HomeWidgetPlacement[]
	/** Domain widget ids the member removed; guards against re-appending. */
	removed: string[]
}

const OVERRIDES_STORAGE_KEY = 'axolotl-ymcl-home-overrides'

function readOverrides(): Record<string, unknown> {
	try {
		return JSON.parse(globalThis.localStorage?.getItem(OVERRIDES_STORAGE_KEY) ?? '{}')
	} catch {
		return {}
	}
}

export function loadYmclHomeOverride(domainId: string): YmclHomeOverride | null {
	const value = readOverrides()[domainId]
	if (!value || typeof value !== 'object') return null
	const candidate = value as Partial<YmclHomeOverride>
	const normalized = normalizeHomeDashboard({
		version: HOME_DASHBOARD_VERSION,
		layout: candidate.layout,
		widgets: candidate.widgets,
	})
	if (!normalized) return null
	return {
		version: HOME_DASHBOARD_VERSION,
		layout: normalized.layout,
		widgets: normalized.widgets,
		removed: Array.isArray(candidate.removed)
			? candidate.removed.filter((id): id is string => typeof id === 'string')
			: [],
	}
}

export function saveYmclHomeOverride(domainId: string, override: YmclHomeOverride): void {
	const all = readOverrides()
	all[domainId] = override
	try {
		globalThis.localStorage?.setItem(OVERRIDES_STORAGE_KEY, JSON.stringify(all))
	} catch {
		// Storage full/unavailable: the override stays session-only.
	}
}

export function clearYmclHomeOverride(domainId: string): void {
	const { [domainId]: _removed, ...rest } = readOverrides()
	try {
		globalThis.localStorage?.setItem(OVERRIDES_STORAGE_KEY, JSON.stringify(rest))
	} catch {
		// Ignore write failures; the in-memory state is already cleared.
	}
}

/**
 * Renders the domain layout through a member's override: the override's
 * widget list wins, and domain cards added after the override was saved are
 * appended (per YAP §6.5 "新增的卡片按默认位置插入") unless explicitly removed.
 */
export function applyHomeOverride(
	domain: HomeDashboardConfig,
	override: YmclHomeOverride | null | undefined,
): HomeDashboardConfig {
	if (!override) return domain
	const known = new Set(override.widgets.map((widget) => widget.id))
	const removed = new Set(override.removed)
	const appended = domain.widgets.filter((widget) => !known.has(widget.id) && !removed.has(widget.id))
	if (override.layout !== 'free') {
		return { version: HOME_DASHBOARD_VERSION, layout: 'grid', widgets: [...override.widgets, ...appended] }
	}
	// Free layout needs explicit coordinates; stack newcomers below the rest.
	let nextRow = override.widgets.reduce(
		(max, widget) => Math.max(max, (widget.position?.row ?? 0) + 1),
		0,
	)
	const placed = appended.map((widget) => {
		const height = Number(widget.size.split('x')[1] ?? 1)
		const placement = { ...widget, position: { column: 0, row: nextRow } }
		nextRow += height
		return placement
	})
	return {
		version: HOME_DASHBOARD_VERSION,
		layout: 'free',
		widgets: [...override.widgets, ...placed],
	}
}
