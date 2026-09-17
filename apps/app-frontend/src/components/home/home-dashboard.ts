export const HOME_DASHBOARD_VERSION = 1 as const
export const HOME_WIDGET_LAYOUTS = ['grid', 'free'] as const
export const HOME_WIDGET_STANDARD_SIZES = ['1x1', '2x1', '1x2', '2x2'] as const
export const HOME_WIDGET_SIZES = [...HOME_WIDGET_STANDARD_SIZES, '3x1', '3x2'] as const
export const HOME_WIDGET_GRID_GAP = 16
export const HOME_WIDGET_GRID_ROW_HEIGHT = 160
export const HOME_RECENT_LIMIT_OPTIONS = [2, 4, 6, 8] as const
export const HOME_RECENT_DEFAULT_LIMIT = 4
export const HOME_GREETING_MODES = ['greeting', 'text-and-greeting', 'text'] as const
export const HOME_GREETING_DEFAULT_MODE = 'greeting'
export const HOME_GREETING_FONTS = ['sans', 'minecraft', 'mono', 'serif'] as const
export const HOME_GREETING_DEFAULT_FONT = 'sans'
export const HOME_GREETING_FONT_SIZE_MIN = 16
export const HOME_GREETING_FONT_SIZE_MAX = 32
export const HOME_GREETING_DEFAULT_FONT_SIZE = 22

export type HomeWidgetSize = (typeof HOME_WIDGET_SIZES)[number]
export type HomeWidgetLayout = (typeof HOME_WIDGET_LAYOUTS)[number]
export type HomeRecentLimit = (typeof HOME_RECENT_LIMIT_OPTIONS)[number]
export type HomeGreetingMode = (typeof HOME_GREETING_MODES)[number]
export type HomeGreetingFont = (typeof HOME_GREETING_FONTS)[number]
export type HomeWidgetKind =
	| 'greeting'
	| 'recent'
	| 'calendar'
	| 'pinned-instances'
	| 'pinned-worlds'
	| 'pinned-servers'
	| 'instance'
	| 'world'
	| 'server'
	| 'page-shortcut'
	| 'data-card'
	| 'domain-servers'

export type HomeWidgetTarget = {
	instanceId: string
	path?: string
	address?: string
	fallbackLabel: string
}

/**
 * Domain card payloads (YAP §6.5 adapter card contributions). `target` is the
 * raw protocol value: "page:<pageId>" or "native:<route>"; the data binding
 * mirrors manifest.dataSources entries so the widget survives manifest churn.
 */
export type HomeShortcutTarget = {
	target: string
	title?: string
}

export type HomeDataCardSource = {
	id: string
	providerCode: string
	sourceCode: string
	variant?: 'stats' | 'list' | 'hero'
	title?: string
}

export type HomeWidgetOptions = {
	recentLimit?: HomeRecentLimit
	greetingMode?: HomeGreetingMode
	greetingText?: string
	greetingFont?: HomeGreetingFont
	greetingFontSize?: number
}

export type HomeWidgetPosition = {
	column: number
	row: number
}

export type HomeWidgetPlacement = {
	id: string
	kind: HomeWidgetKind
	size: HomeWidgetSize
	target?: HomeWidgetTarget
	options?: HomeWidgetOptions
	position?: HomeWidgetPosition
	shortcut?: HomeShortcutTarget
	dataSource?: HomeDataCardSource
}

export type HomeDashboardConfig = {
	version: typeof HOME_DASHBOARD_VERSION
	layout: HomeWidgetLayout
	widgets: HomeWidgetPlacement[]
}

export type PackedHomeWidget = HomeWidgetPlacement & {
	column: number
	row: number
	effectiveColumns: number
	effectiveRows: number
}

export type HomeDashboardSaveQueue = {
	enqueue: (config: HomeDashboardConfig, rollback: HomeDashboardConfig) => Promise<void>
	flush: () => Promise<void>
}

export const HOME_WIDGET_SIZE_OPTIONS: Record<HomeWidgetKind, readonly HomeWidgetSize[]> = {
	greeting: ['2x1'],
	recent: ['2x1', '2x2', '3x1', '3x2'],
	calendar: ['1x2'],
	'pinned-instances': HOME_WIDGET_STANDARD_SIZES,
	'pinned-worlds': HOME_WIDGET_STANDARD_SIZES,
	'pinned-servers': HOME_WIDGET_STANDARD_SIZES,
	instance: ['1x1', '2x1'],
	world: ['1x1', '2x1'],
	server: ['1x1', '2x1'],
	'page-shortcut': ['1x1', '2x1'],
	'data-card': HOME_WIDGET_STANDARD_SIZES,
	'domain-servers': ['2x1', '1x2', '2x2'],
}

export const HOME_WIDGET_DEFAULT_SIZE: Record<HomeWidgetKind, HomeWidgetSize> = {
	greeting: '2x1',
	recent: '2x2',
	calendar: '1x2',
	'pinned-instances': '2x2',
	'pinned-worlds': '1x2',
	'pinned-servers': '1x2',
	instance: '1x1',
	world: '1x1',
	server: '1x1',
	'page-shortcut': '1x1',
	'data-card': '2x1',
	'domain-servers': '2x2',
}

export function getHomeWidgetCardDensity(
	dashboardSize: HomeWidgetSize | null | undefined,
): 'compact' | 'comfortable' {
	return dashboardSize === '1x1' ||
		dashboardSize === '1x2' ||
		dashboardSize === '2x1' ||
		dashboardSize === '2x2'
		? 'compact'
		: 'comfortable'
}

const HOME_WIDGET_KINDS = new Set<HomeWidgetKind>(
	Object.keys(HOME_WIDGET_DEFAULT_SIZE) as HomeWidgetKind[],
)

function createPlacement(
	kind: HomeWidgetKind,
	size = HOME_WIDGET_DEFAULT_SIZE[kind],
): HomeWidgetPlacement {
	return {
		id: crypto.randomUUID(),
		kind,
		size,
		...(kind === 'recent' ? { options: { recentLimit: HOME_RECENT_DEFAULT_LIMIT } } : {}),
		...(kind === 'greeting'
			? {
					options: {
						greetingMode: HOME_GREETING_DEFAULT_MODE,
						greetingFont: HOME_GREETING_DEFAULT_FONT,
						greetingFontSize: HOME_GREETING_DEFAULT_FONT_SIZE,
					},
				}
			: {}),
	}
}

export function createDefaultHomeDashboard(includeRecent = true): HomeDashboardConfig {
	return {
		version: HOME_DASHBOARD_VERSION,
		layout: 'grid',
		widgets: [
			createPlacement('greeting'),
			createPlacement('calendar'),
			...(includeRecent ? [createPlacement('recent')] : []),
			createPlacement('pinned-worlds'),
			createPlacement('pinned-servers', '2x2'),
			createPlacement('pinned-instances', '2x1'),
		],
	}
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function normalizeTarget(value: unknown): HomeWidgetTarget | undefined {
	if (!isRecord(value) || typeof value.instanceId !== 'string' || !value.instanceId)
		return undefined
	if (typeof value.fallbackLabel !== 'string' || !value.fallbackLabel) return undefined

	return {
		instanceId: value.instanceId,
		...(typeof value.path === 'string' ? { path: value.path } : {}),
		...(typeof value.address === 'string' ? { address: value.address } : {}),
		fallbackLabel: value.fallbackLabel,
	}
}

function normalizePosition(value: unknown): HomeWidgetPosition | undefined {
	if (!isRecord(value)) return undefined
	if (typeof value.column !== 'number' || !Number.isFinite(value.column)) return undefined
	if (typeof value.row !== 'number' || !Number.isFinite(value.row)) return undefined

	return {
		column: Math.min(100, Math.max(0, Math.round(value.column))),
		row: Math.min(10_000, Math.max(0, Math.round(value.row))),
	}
}

const HOME_DATA_CARD_VARIANTS = ['stats', 'list', 'hero'] as const

function normalizeShortcut(value: unknown): HomeShortcutTarget | undefined {
	if (!isRecord(value) || typeof value.target !== 'string') return undefined
	const target = value.target.trim()
	if (!target) return undefined
	return {
		target,
		...(typeof value.title === 'string' && value.title ? { title: value.title } : {}),
	}
}

function normalizeDataSource(value: unknown): HomeDataCardSource | undefined {
	if (!isRecord(value)) return undefined
	const id = typeof value.id === 'string' ? value.id.trim() : ''
	const providerCode = typeof value.providerCode === 'string' ? value.providerCode.trim() : ''
	const sourceCode = typeof value.sourceCode === 'string' ? value.sourceCode.trim() : ''
	if (!id || !providerCode || !sourceCode) return undefined
	const variant = HOME_DATA_CARD_VARIANTS.includes(
		value.variant as (typeof HOME_DATA_CARD_VARIANTS)[number],
	)
		? (value.variant as (typeof HOME_DATA_CARD_VARIANTS)[number])
		: undefined
	return {
		id,
		providerCode,
		sourceCode,
		...(variant ? { variant } : {}),
		...(typeof value.title === 'string' && value.title ? { title: value.title } : {}),
	}
}

function normalizeOptions(kind: HomeWidgetKind, value: unknown): HomeWidgetOptions | undefined {
	if (kind === 'recent') {
		const recentLimit =
			isRecord(value) && HOME_RECENT_LIMIT_OPTIONS.includes(value.recentLimit as HomeRecentLimit)
				? (value.recentLimit as HomeRecentLimit)
				: HOME_RECENT_DEFAULT_LIMIT
		return { recentLimit }
	}

	if (kind === 'greeting') {
		const greetingMode =
			isRecord(value) && HOME_GREETING_MODES.includes(value.greetingMode as HomeGreetingMode)
				? (value.greetingMode as HomeGreetingMode)
				: HOME_GREETING_DEFAULT_MODE
		const greetingText =
			isRecord(value) && typeof value.greetingText === 'string'
				? value.greetingText.trim().slice(0, 120)
				: ''
		const greetingFont =
			isRecord(value) && HOME_GREETING_FONTS.includes(value.greetingFont as HomeGreetingFont)
				? (value.greetingFont as HomeGreetingFont)
				: HOME_GREETING_DEFAULT_FONT
		const greetingFontSize = normalizeGreetingFontSize(
			isRecord(value) ? value.greetingFontSize : undefined,
		)
		return {
			greetingMode,
			...(greetingText ? { greetingText } : {}),
			greetingFont,
			greetingFontSize,
		}
	}

	return undefined
}

function normalizeGreetingFontSize(value: unknown): number {
	if (typeof value !== 'number' || !Number.isFinite(value)) {
		return HOME_GREETING_DEFAULT_FONT_SIZE
	}

	return Math.min(
		HOME_GREETING_FONT_SIZE_MAX,
		Math.max(HOME_GREETING_FONT_SIZE_MIN, Math.round(value)),
	)
}

export function normalizeHomeDashboard(value: unknown): HomeDashboardConfig | null {
	if (
		!isRecord(value) ||
		value.version !== HOME_DASHBOARD_VERSION ||
		!Array.isArray(value.widgets)
	) {
		return null
	}

	const usedIds = new Set<string>()
	const layout = HOME_WIDGET_LAYOUTS.includes(value.layout as HomeWidgetLayout)
		? (value.layout as HomeWidgetLayout)
		: 'grid'
	const widgets = value.widgets.flatMap((candidate): HomeWidgetPlacement[] => {
		if (!isRecord(candidate) || typeof candidate.kind !== 'string') return []
		// domain-packs was renamed to domain-servers (server-first home card).
		const rawKind = candidate.kind === 'domain-packs' ? 'domain-servers' : candidate.kind
		if (!HOME_WIDGET_KINDS.has(rawKind as HomeWidgetKind)) return []

		const kind = rawKind as HomeWidgetKind
		const target = normalizeTarget(candidate.target)
		const options = normalizeOptions(kind, candidate.options)
		const position = normalizePosition(candidate.position)
		const shortcut = normalizeShortcut(candidate.shortcut)
		const dataSource = normalizeDataSource(candidate.dataSource)
		if ((kind === 'instance' || kind === 'world' || kind === 'server') && !target) return []
		if (kind === 'world' && !target?.path) return []
		if (kind === 'server' && !target?.address) return []
		if (kind === 'page-shortcut' && !shortcut) return []
		if (kind === 'data-card' && !dataSource) return []

		let id = typeof candidate.id === 'string' && candidate.id ? candidate.id : crypto.randomUUID()
		if (usedIds.has(id)) id = crypto.randomUUID()
		usedIds.add(id)

		const requestedSize = typeof candidate.size === 'string' ? candidate.size : ''
		const size = HOME_WIDGET_SIZE_OPTIONS[kind].includes(requestedSize as HomeWidgetSize)
			? (requestedSize as HomeWidgetSize)
			: HOME_WIDGET_DEFAULT_SIZE[kind]

		return [
			{
				id,
				kind,
				size,
				...(target ? { target } : {}),
				...(options ? { options } : {}),
				...(position ? { position } : {}),
				...(shortcut ? { shortcut } : {}),
				...(dataSource ? { dataSource } : {}),
			},
		]
	})

	return { version: HOME_DASHBOARD_VERSION, layout, widgets }
}

export function replaceHomeDashboardWidgets(
	config: HomeDashboardConfig,
	widgets: HomeWidgetPlacement[],
): HomeDashboardConfig {
	return { ...config, widgets }
}

export function addHomeWidget(
	config: HomeDashboardConfig,
	widget: HomeWidgetPlacement,
): HomeDashboardConfig {
	return replaceHomeDashboardWidgets(config, [...config.widgets, widget])
}

export function setHomeDashboardLayout(
	config: HomeDashboardConfig,
	layout: HomeWidgetLayout,
): HomeDashboardConfig {
	return { ...config, layout }
}

export function setHomeWidgetPosition(
	config: HomeDashboardConfig,
	id: string,
	position: HomeWidgetPosition,
): HomeDashboardConfig {
	return replaceHomeDashboardWidgets(
		config,
		config.widgets.map((widget) => (widget.id === id ? { ...widget, position } : widget)),
	)
}

export function removeHomeWidget(config: HomeDashboardConfig, id: string): HomeDashboardConfig {
	return replaceHomeDashboardWidgets(
		config,
		config.widgets.filter((widget) => widget.id !== id),
	)
}

export function resizeHomeWidget(
	config: HomeDashboardConfig,
	id: string,
	size: HomeWidgetSize,
): HomeDashboardConfig {
	return replaceHomeDashboardWidgets(
		config,
		config.widgets.map((widget) =>
			widget.id === id && HOME_WIDGET_SIZE_OPTIONS[widget.kind].includes(size)
				? { ...widget, size }
				: widget,
		),
	)
}

export function setHomeRecentLimit(
	config: HomeDashboardConfig,
	id: string,
	recentLimit: HomeRecentLimit,
): HomeDashboardConfig {
	return replaceHomeDashboardWidgets(
		config,
		config.widgets.map((widget) =>
			widget.id === id && widget.kind === 'recent'
				? { ...widget, options: { ...widget.options, recentLimit } }
				: widget,
		),
	)
}

export function setHomeGreetingOptions(
	config: HomeDashboardConfig,
	id: string,
	greetingMode: HomeGreetingMode,
	greetingText: string,
	greetingFont: HomeGreetingFont,
	greetingFontSize: number,
): HomeDashboardConfig {
	const normalizedText = greetingText.trim().slice(0, 120)
	const normalizedFont = HOME_GREETING_FONTS.includes(greetingFont)
		? greetingFont
		: HOME_GREETING_DEFAULT_FONT
	return replaceHomeDashboardWidgets(
		config,
		config.widgets.map((widget) =>
			widget.id === id && widget.kind === 'greeting'
				? {
						...widget,
						options: {
							greetingMode,
							...(normalizedText ? { greetingText: normalizedText } : {}),
							greetingFont: normalizedFont,
							greetingFontSize: normalizeGreetingFontSize(greetingFontSize),
						},
					}
				: widget,
		),
	)
}

export function moveHomeWidget(
	config: HomeDashboardConfig,
	index: number,
	direction: -1 | 1,
): HomeDashboardConfig {
	const target = index + direction
	if (
		index < 0 ||
		index >= config.widgets.length ||
		target < 0 ||
		target >= config.widgets.length
	) {
		return config
	}

	const widgets = [...config.widgets]
	const [widget] = widgets.splice(index, 1)
	widgets.splice(target, 0, widget)
	return replaceHomeDashboardWidgets(config, widgets)
}

export function createHomeDashboardSaveQueue(
	persist: (config: HomeDashboardConfig) => Promise<void>,
	onRollback: (config: HomeDashboardConfig) => void,
	onError: (error: unknown) => void,
): HomeDashboardSaveQueue {
	let queue = Promise.resolve()
	let version = 0

	return {
		enqueue(config, rollback) {
			const operationVersion = ++version
			queue = queue.then(async () => {
				try {
					await persist(config)
				} catch (error) {
					if (operationVersion === version) onRollback(rollback)
					onError(error)
				}
			})
			return queue
		},
		flush: () => queue,
	}
}

export function getHomeGridColumnCount(width: number): number {
	const minimumColumnWidth = 240
	const gap = 16
	return Math.min(
		4,
		Math.max(1, Math.floor((Math.max(0, width) + gap) / (minimumColumnWidth + gap))),
	)
}

export function getHomeWidgetDimensions(
	size: HomeWidgetSize,
	columnCount: number,
	containerWidth: number,
) {
	const columns = Math.max(1, columnCount)
	const span = getHomeWidgetSpan(size, columns)
	const columnWidth = Math.max(0, (containerWidth - HOME_WIDGET_GRID_GAP * (columns - 1)) / columns)

	return {
		width: columnWidth * span.columns + HOME_WIDGET_GRID_GAP * (span.columns - 1),
		height: HOME_WIDGET_GRID_ROW_HEIGHT * span.rows + HOME_WIDGET_GRID_GAP * (span.rows - 1),
	}
}

export function getHomeWidgetSpan(size: HomeWidgetSize, columnCount: number) {
	const [columns, rows] = size.split('x').map(Number)
	return {
		columns: Math.min(columns, Math.max(1, columnCount)),
		rows,
	}
}

function homeWidgetRect(
	widget: HomeWidgetPlacement,
	position: HomeWidgetPosition,
	columnCount: number,
) {
	const span = getHomeWidgetSpan(widget.size, columnCount)
	return {
		left: position.column,
		top: position.row,
		right: position.column + span.columns,
		bottom: position.row + span.rows,
	}
}

function homeWidgetRectsOverlap(
	left: ReturnType<typeof homeWidgetRect>,
	right: ReturnType<typeof homeWidgetRect>,
) {
	return (
		left.left < right.right &&
		left.right > right.left &&
		left.top < right.bottom &&
		left.bottom > right.top
	)
}

export function findNearestFreeHomeWidgetPosition(
	widgets: readonly HomeWidgetPlacement[],
	movingWidget: HomeWidgetPlacement,
	desiredPosition: HomeWidgetPosition,
	columnCount: number,
): HomeWidgetPosition {
	const columns = Math.max(1, columnCount)
	const movingSpan = getHomeWidgetSpan(movingWidget.size, columns)
	const desired = {
		column: Math.min(
			Math.max(0, Math.round(desiredPosition.column)),
			Math.max(0, columns - movingSpan.columns),
		),
		row: Math.max(0, Math.round(desiredPosition.row)),
	}
	const occupied = widgets.flatMap((widget) =>
		widget.id !== movingWidget.id && widget.position
			? [homeWidgetRect(widget, widget.position, columns)]
			: [],
	)
	const isAvailable = (position: HomeWidgetPosition) => {
		const candidate = homeWidgetRect(movingWidget, position, columns)
		return occupied.every((rect) => !homeWidgetRectsOverlap(candidate, rect))
	}

	if (isAvailable(desired)) return desired

	const lastOccupiedRow = occupied.reduce((last, rect) => Math.max(last, rect.bottom), 0)
	const lastSearchRow = Math.max(
		desired.row + widgets.length * 2,
		lastOccupiedRow + movingSpan.rows,
	)
	let closest: HomeWidgetPosition | null = null
	let closestDistance = Number.POSITIVE_INFINITY
	for (let row = 0; row <= lastSearchRow; row += 1) {
		for (let column = 0; column <= columns - movingSpan.columns; column += 1) {
			const candidate = { column, row }
			if (!isAvailable(candidate)) continue
			const distance = Math.abs(column - desired.column) + Math.abs(row - desired.row)
			if (distance >= closestDistance) continue
			closest = candidate
			closestDistance = distance
		}
	}

	return closest ?? desired
}

export function packHomeWidgets(
	widgets: readonly HomeWidgetPlacement[],
	columnCount: number,
): PackedHomeWidget[] {
	const columns = Math.max(1, columnCount)
	const occupied: boolean[][] = []

	const fits = (column: number, row: number, width: number, height: number) => {
		if (column + width > columns) return false
		for (let y = row; y < row + height; y += 1) {
			for (let x = column; x < column + width; x += 1) {
				if (occupied[y]?.[x]) return false
			}
		}
		return true
	}

	return widgets.map((widget) => {
		const span = getHomeWidgetSpan(widget.size, columns)
		let row = 0
		let column = 0
		while (!fits(column, row, span.columns, span.rows)) {
			column += 1
			if (column >= columns) {
				column = 0
				row += 1
			}
		}

		for (let y = row; y < row + span.rows; y += 1) {
			occupied[y] ??= Array.from({ length: columns }, () => false)
			for (let x = column; x < column + span.columns; x += 1) occupied[y][x] = true
		}

		return {
			...widget,
			column: column + 1,
			row: row + 1,
			effectiveColumns: span.columns,
			effectiveRows: span.rows,
		}
	})
}

export function enableFreeHomeDashboard(
	config: HomeDashboardConfig,
	columnCount: number,
): HomeDashboardConfig {
	const columns = Math.max(1, columnCount)
	const packedById = new Map(
		packHomeWidgets(config.widgets, columns).map((widget) => [widget.id, widget]),
	)

	return {
		...config,
		layout: 'free',
		widgets: config.widgets.map((widget) => {
			if (widget.position) return widget
			const packed = packedById.get(widget.id)
			if (!packed) return widget

			return {
				...widget,
				position: {
					column: packed.column - 1,
					row: packed.row - 1,
				},
			}
		}),
	}
}
