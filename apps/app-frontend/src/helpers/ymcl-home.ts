// YAP home card layout mapping (YAP §6.5 home capability): converts the
// domain's ThemeProfile-style home config into the launcher's existing
// HomeDashboardConfig so the real dashboard renders it. Shared by the
// domain-home store getter and the /ymcl/design designer preview.

import {
	HOME_DASHBOARD_VERSION,
	HOME_WIDGET_DEFAULT_SIZE,
	HOME_WIDGET_SIZE_OPTIONS,
	type HomeDashboardConfig,
	type HomeWidgetKind,
	type HomeWidgetLayout,
	type HomeWidgetOptions,
	type HomeWidgetPlacement,
	type HomeWidgetPosition,
	type HomeWidgetSize,
	normalizeHomeDashboard,
} from '../components/home/home-dashboard.ts'

export interface YmclHomeCard {
	id: string
	type: string
	span?: number
	sort?: number
	title?: string
	params?: Record<string, unknown>
	requiredPermission?: string
	/** Launcher extension: free-layout coordinates; the adapter persists them verbatim. */
	position?: HomeWidgetPosition
	/** Launcher extension: widget size ('2x1' 等)；适配器原样透传，缺省回退类型默认尺寸。 */
	size?: HomeWidgetSize
}

export interface YmclHomeProfile {
	schemaVersion?: number
	locked?: boolean
	columns?: number
	/** Launcher extension (protocol v1 default is grid): free widget layout. */
	layout?: HomeWidgetLayout
	cards?: YmclHomeCard[]
}

/** Card types the launcher natively renders; unknown types are dropped. */
export const HOME_CARD_TYPE_OPTIONS = [
	{ value: 'recent-instances', label: '最近实例', kind: 'recent', size: '2x2' },
	{ value: 'pinned-instances', label: '固定实例', kind: 'pinned-instances', size: '2x1' },
	{ value: 'pinned-servers', label: '固定服务器', kind: 'pinned-servers', size: '2x2' },
	// 问候/日历/固定世界/域服务器列表没有用户私有绑定，域布局同样可以承载
	// （问候语可由管理员定制；域服务器列表随域数据展示）。
	{ value: 'greeting', label: '问候', kind: 'greeting', size: '2x1' },
	{ value: 'calendar', label: '日历', kind: 'calendar', size: '1x2' },
	{ value: 'pinned-worlds', label: '固定的世界', kind: 'pinned-worlds', size: '1x2' },
	{ value: 'domain-servers', label: '域服务器列表', kind: 'domain-servers', size: '2x2' },
] as const

/**
 * Adapter-contributed card types (YAP §6.5 v1): a `page-shortcut` targets a
 * domain page (`page:<id>`) or native route (`native:<route>`); a `data-card`
 * binds a manifest dataSource. Both render as real dashboard widgets, so they
 * round-trip through the editor instead of being preserved untouched.
 */
const DOMAIN_CARD_KINDS: Record<string, string> = {
	'page-shortcut': 'page-shortcut',
	'data-card': 'data-card',
}

/** Every protocol card type the launcher can map onto a dashboard widget. */
export const MAPPABLE_HOME_CARD_TYPES: readonly string[] = [
	...HOME_CARD_TYPE_OPTIONS.map((option) => option.value),
	...Object.keys(DOMAIN_CARD_KINDS),
]

/** Types declared by the protocol but without a native dashboard widget. */
export const UNMAPPED_CARD_TYPES = ['announcement', 'quick-actions']

function firstNonEmpty(...values: (string | null | undefined)[]): string | null {
	for (const value of values) {
		const trimmed = value?.trim()
		if (trimmed) return trimmed
	}
	return null
}

/** Well-known YAP sourceCode → display label when the adapter omits titles. */
const WELL_KNOWN_SOURCE_CODE_LABELS: Record<string, string> = {
	servers: '服务器',
	'servers.list': '服务器',
	activities: '活动',
	'my-tasks': '我的任务',
	'claimable-tasks': '可认领任务',
	'my-stats': '我的统计',
}

/** First segment is providerCode; the remainder (may contain dots) is sourceCode. */
export function splitDataSourceId(id: string): {
	providerCode: string
	sourceCode: string
} | null {
	const separator = id.indexOf('.')
	if (separator <= 0 || separator === id.length - 1) return null
	return { providerCode: id.slice(0, separator), sourceCode: id.slice(separator + 1) }
}

/** Readable fallback for a raw `<provider>.<source>` id (e.g. "My Tasks"). */
export function humanizeDataSourceId(id: string): string {
	const sourceCode = splitDataSourceId(id)?.sourceCode ?? id
	return sourceCode
		.split(/[-_.]/)
		.filter(Boolean)
		.map((part) => part.charAt(0).toUpperCase() + part.slice(1))
		.join(' ')
}

/**
 * Last-resort display label: prefer a well-known sourceCode mapping, else
 * humanize the source segment. Used when the adapter provides no titles.
 */
export function dataSourceFallbackLabel(id: string): string {
	const sourceCode = splitDataSourceId(id)?.sourceCode ?? id
	return WELL_KNOWN_SOURCE_CODE_LABELS[sourceCode] ?? humanizeDataSourceId(id)
}

/**
 * Human-readable label for a dataSource. Titles are adapter-provided
 * (declaration / bound page / navigation / home card). A cached title equal
 * to the raw dataSource id is treated as missing so page/declaration titles
 * win. When the adapter provides nothing, fall back to a well-known
 * sourceCode label or a humanized source segment — never the raw id.
 */
export function resolveDataSourceTitle(options: {
	id: string
	cardTitle?: string | null
	declarationTitle?: string | null
	pageTitle?: string | null
	navigationTitle?: string | null
}): string {
	const id = options.id
	const usable = (value?: string | null) => {
		const trimmed = value?.trim()
		if (!trimmed || trimmed === id) return null
		return trimmed
	}
	return (
		usable(options.cardTitle) ??
		usable(options.declarationTitle) ??
		usable(options.pageTitle) ??
		usable(options.navigationTitle) ??
		dataSourceFallbackLabel(id)
	)
}

/**
 * Resolves a data-card's dataSource id into the provider/source pair that
 * `ymcl_data_fetch` needs, preferring the manifest declaration and falling
 * back to splitting the adapter-generated `<providerCode>.<sourceCode>` id.
 * Manifest dialects vary (snake/camel), so both spellings are accepted.
 */
export function resolveDataCardSource(
	id: string,
	dataSources?: readonly {
		id: string
		providerCode?: string
		sourceCode?: string
		provider_code?: string
		source_code?: string
		title?: string
		name?: string
		label?: string
	}[],
): { providerCode: string; sourceCode: string; title: string | null } | null {
	const declared = dataSources?.find((source) => source.id === id)
	if (declared) {
		const providerCode = declared.providerCode ?? declared.provider_code
		const sourceCode = declared.sourceCode ?? declared.source_code
		const title = firstNonEmpty(declared.title, declared.name, declared.label)
		if (providerCode && sourceCode) return { providerCode, sourceCode, title }
	}
	// 旧版绑定迁移：裸 sourceCode（无点号）按声明里的 sourceCode 匹配。
	const legacy = dataSources?.find((source) => (source.source_code ?? source.sourceCode) === id)
	if (legacy) {
		const providerCode = legacy.providerCode ?? legacy.provider_code
		const sourceCode = legacy.sourceCode ?? legacy.source_code
		const title = firstNonEmpty(legacy.title, legacy.name, legacy.label)
		if (providerCode && sourceCode) return { providerCode, sourceCode, title }
	}
	const split = splitDataSourceId(id)
	if (!split) return null
	return { ...split, title: null }
}

/**
 * Validates a card's persisted size against the widget kind's allowed sizes;
 * unknown/missing values fall back to the kind default so stale or foreign
 * profiles never produce an unrenderable widget.
 */
function resolveCardSize(
	size: unknown,
	kind: HomeWidgetKind,
	fallback?: HomeWidgetSize,
): HomeWidgetSize {
	if (typeof size === 'string' && (HOME_WIDGET_SIZE_OPTIONS[kind] as readonly string[]).includes(size)) {
		return size as HomeWidgetSize
	}
	return fallback ?? HOME_WIDGET_DEFAULT_SIZE[kind]
}

/**
 * Maps a domain home profile to the dashboard config. Cards without a
 * native equivalent are dropped per the protocol whitelist; returns null
 * when nothing renderable remains.
 */
export function mapHomeProfileToDashboard(
	profile: YmclHomeProfile | null | undefined,
	dataSources?: readonly {
		id: string
		providerCode?: string
		sourceCode?: string
		provider_code?: string
		source_code?: string
		title?: string
		name?: string
		label?: string
	}[],
): HomeDashboardConfig | null {
	if (!profile?.cards?.length) return null
	const free = profile.layout === 'free'
	const widgets: HomeWidgetPlacement[] = []
	let stackedRow = 0
	for (const card of profile.cards) {
		const native = HOME_CARD_TYPE_OPTIONS.find((option) => option.value === card.type)
		const domainKind = DOMAIN_CARD_KINDS[card.type]
		if (!native && !domainKind) continue

		let widget: HomeWidgetPlacement
		if (card.type === 'page-shortcut') {
			const target = typeof card.params?.target === 'string' ? (card.params.target as string) : ''
			if (!target) continue
			widget = {
				id: card.id,
				kind: 'page-shortcut',
				size: resolveCardSize(card.size, 'page-shortcut'),
				shortcut: { target, ...(card.title ? { title: card.title } : {}) },
			}
		} else if (card.type === 'data-card') {
			const sourceId = typeof card.params?.dataSource === 'string' ? card.params.dataSource : ''
			const resolved = sourceId ? resolveDataCardSource(sourceId, dataSources) : null
			if (!sourceId || !resolved) continue
			const variant =
				card.params?.variant === 'stats' || card.params?.variant === 'hero'
					? card.params.variant
					: 'list'
			widget = {
				id: card.id,
				kind: 'data-card',
				size: resolveCardSize(card.size, 'data-card'),
				dataSource: {
					id: sourceId,
					providerCode: resolved.providerCode,
					sourceCode: resolved.sourceCode,
					variant,
					// Prefer adapter-declared titles; ignore a card title equal to the
					// raw id so page/declaration labels still win at render time.
					...((card.title && card.title !== sourceId) || resolved.title
						? {
								title:
									card.title && card.title !== sourceId
										? card.title
										: (resolved.title as string),
							}
						: {}),
				},
			}
		} else {
			widget = {
				id: card.id,
				kind: native!.kind,
				size: resolveCardSize(card.size, native!.kind, native!.size),
				// 问候语/最近实例选项随 params 往返；最终统一 normalize 消毒。
				...(card.params && (card.type === 'greeting' || card.type === 'recent-instances')
					? { options: card.params as unknown as HomeWidgetOptions }
					: {}),
			}
		}

		const rows = Number(widget.size.split('x')[1] ?? 1)
		if (isFreePosition(card.position)) {
			widget.position = card.position
		} else {
			widget.position = { column: 0, row: stackedRow }
			stackedRow += rows
		}
		widgets.push(widget)
	}
	if (widgets.length === 0) return null
	// normalize 消毒问候语选项等载荷，非法字段回退默认值。
	return (
		normalizeHomeDashboard({
			version: HOME_DASHBOARD_VERSION,
			layout: free ? 'free' : 'grid',
			widgets,
		}) ?? { version: HOME_DASHBOARD_VERSION, layout: free ? 'free' : 'grid', widgets }
	)
}

function isFreePosition(position: YmclHomeCard['position']): position is HomeWidgetPosition {
	return (
		!!position &&
		Number.isFinite(position.column) &&
		Number.isFinite(position.row) &&
		position.column >= 0 &&
		position.row >= 0
	)
}

/**
 * Reverse of mapHomeProfileToDashboard for in-place domain home editing:
 * cards that still exist keep their original payload and get their sort
 * reassigned by dashboard order, new widgets become fresh cards, widgets
 * without a domain card type are skipped (reported via skippedWidgets), and
 * original protocol cards the launcher cannot render are preserved at the
 * end so a republish never silently deletes them.
 */
export function mapDashboardToHomeProfile(
	config: HomeDashboardConfig,
	original?: YmclHomeProfile | null,
): { profile: YmclHomeProfile; skippedWidgets: number; skippedKinds: string[] } {
	const typeByKind = new Map<string, string>([
		...HOME_CARD_TYPE_OPTIONS.map((option) => [option.kind, option.value] as const),
		...Object.entries(DOMAIN_CARD_KINDS),
	])
	const untouched = new Map((original?.cards ?? []).map((card) => [card.id, card]))
	const free = config.layout === 'free'
	const cards: YmclHomeCard[] = []
	let skippedWidgets = 0
	const skippedKinds: string[] = []
	const noteSkip = (kind: string) => {
		skippedWidgets += 1
		if (!skippedKinds.includes(kind)) skippedKinds.push(kind)
	}
	for (const widget of config.widgets) {
		const type = typeByKind.get(widget.kind)
		if (!type) {
			noteSkip(widget.kind)
			continue
		}
		const existing = untouched.get(widget.id)
		let card: YmclHomeCard
		if (existing) {
			untouched.delete(widget.id)
			card = { ...existing, sort: cards.length }
		} else if (widget.kind === 'page-shortcut' && widget.shortcut) {
			card = {
				id: widget.id,
				type,
				sort: cards.length,
				...(widget.shortcut.title ? { title: widget.shortcut.title } : {}),
				params: { target: widget.shortcut.target },
			}
		} else if (widget.kind === 'data-card' && widget.dataSource) {
			card = {
				id: widget.id,
				type,
				sort: cards.length,
				...(widget.dataSource.title ? { title: widget.dataSource.title } : {}),
				params: {
					dataSource: widget.dataSource.id,
					...(widget.dataSource.variant ? { variant: widget.dataSource.variant } : {}),
				},
			}
		} else if (widget.kind !== 'page-shortcut' && widget.kind !== 'data-card') {
			card = { id: widget.id, type, sort: cards.length }
			// 问候语定制文案随卡片发布，成员端原样渲染。
			if (widget.kind === 'greeting' && widget.options) {
				card.params = { ...widget.options }
			}
			// 最近实例的展示数量同样是卡片配置，随 params 往返。
			if (widget.kind === 'recent' && widget.options?.recentLimit) {
				card.params = { recentLimit: widget.options.recentLimit }
			}
		} else {
			noteSkip(widget.kind)
			continue
		}
		// 尺寸随卡片持久化（网格布局同样按它还原跨度）；自由坐标只在自由布局
		// 下保留，切回网格时清掉避免残留复活。
		card.size = widget.size
		if (free && widget.position) {
			card.position = { ...widget.position }
		} else {
			delete card.position
		}
		cards.push(card)
	}
	// Known-type originals left over were removed by the editor and stay
	// removed; unknown-type originals were never visible and are kept.
	for (const card of untouched.values()) {
		if (!MAPPABLE_HOME_CARD_TYPES.includes(card.type)) {
			const kept = { ...card, sort: cards.length }
			if (!free) delete kept.position
			cards.push(kept)
		}
	}
	return {
		profile: {
			schemaVersion: original?.schemaVersion,
			locked: original?.locked,
			columns: original?.columns,
			layout: config.layout,
			cards,
		},
		skippedWidgets,
		skippedKinds,
	}
}
