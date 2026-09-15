// YAP home card layout mapping (YAP §6.5 home capability): converts the
// domain's ThemeProfile-style home config into the launcher's existing
// HomeDashboardConfig so the real dashboard renders it. Shared by the
// domain-home store getter and the /ymcl/design designer preview.

import {
	HOME_DASHBOARD_VERSION,
	type HomeDashboardConfig,
	type HomeWidgetPlacement,
} from '@/components/home/home-dashboard'

export interface YmclHomeCard {
	id: string
	type: string
	span?: number
	sort?: number
	title?: string
	params?: Record<string, unknown>
	requiredPermission?: string
}

export interface YmclHomeProfile {
	schemaVersion?: number
	locked?: boolean
	columns?: number
	cards?: YmclHomeCard[]
}

/** Card types the launcher natively renders; unknown types are dropped. */
export const HOME_CARD_TYPE_OPTIONS = [
	{ value: 'recent-instances', label: 'Recent instances', kind: 'recent', size: '2x2' },
	{ value: 'pinned-instances', label: 'Pinned instances', kind: 'pinned-instances', size: '2x1' },
	{ value: 'pinned-servers', label: 'Pinned servers', kind: 'pinned-servers', size: '2x2' },
] as const

/** Types declared by the protocol but without a native dashboard widget. */
export const UNMAPPED_CARD_TYPES = ['page-shortcut', 'data-card', 'announcement', 'quick-actions']

/**
 * Maps a domain home profile to the dashboard config. Cards without a
 * native equivalent are dropped per the protocol whitelist; returns null
 * when nothing renderable remains.
 */
export function mapHomeProfileToDashboard(
	profile: YmclHomeProfile | null | undefined,
): HomeDashboardConfig | null {
	if (!profile?.cards?.length) return null
	const widgets: HomeWidgetPlacement[] = []
	let row = 0
	for (const card of profile.cards) {
		const mapping = HOME_CARD_TYPE_OPTIONS.find((option) => option.value === card.type)
		if (!mapping) continue
		widgets.push({
			id: card.id,
			kind: mapping.kind,
			size: mapping.size,
			position: { column: 0, row },
		})
		row += Number(mapping.size.split('x')[1] ?? 1)
	}
	if (widgets.length === 0) return null
	return { version: HOME_DASHBOARD_VERSION, layout: 'grid', widgets }
}
