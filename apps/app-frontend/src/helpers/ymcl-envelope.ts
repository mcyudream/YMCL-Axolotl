// Normalize YAP data envelopes across adapter dialects (snake_case, Chinese
// admin consoles, yda `{code,data}` wrappers) so renderers always see
// records + actions even when the adapter omits the camelCase contract.
// Keep this module free of Tauri/path-alias imports so node --test can load it.

import type { YmclAction } from '@/helpers/ymcl-actions'

export interface NormalizedYmclEnvelope {
	records: Record<string, unknown>[]
	actions: YmclAction[]
	itemActions: YmclAction[]
	allow: string[]
	schemaVersion?: number
}

function asRecordArray(value: unknown): Record<string, unknown>[] {
	if (!Array.isArray(value)) return []
	return value.filter(
		(item): item is Record<string, unknown> =>
			!!item && typeof item === 'object' && !Array.isArray(item),
	)
}

function asString(value: unknown): string | null {
	if (typeof value !== 'string') return null
	const trimmed = value.trim()
	return trimmed ? trimmed : null
}

/** Adapter labels may be plain strings or locale maps ({ "zh-CN": "报名" }). */
export function ymclEnvelopeText(value: unknown): string {
	if (value == null) return ''
	if (typeof value === 'string') return value.trim()
	if (typeof value === 'number' || typeof value === 'boolean') return String(value)
	if (typeof value === 'object' && !Array.isArray(value)) {
		const obj = value as Record<string, unknown>
		for (const key of [
			'zh-CN',
			'zh',
			'cn',
			'zh-TW',
			'en-US',
			'en',
			'default',
			'text',
			'value',
			'name',
			'title',
			'label',
			'message',
		] as const) {
			const nested = obj[key]
			if (typeof nested === 'string' && nested.trim()) return nested.trim()
		}
	}
	return ''
}

export function normalizeYmclAction(raw: unknown): YmclAction | null {
	if (!raw || typeof raw !== 'object') return null
	const item = raw as Record<string, unknown>
	const kind =
		ymclEnvelopeText(item.kind) ||
		ymclEnvelopeText(item.type) ||
		ymclEnvelopeText(item.action_kind) ||
		ymclEnvelopeText(item.actionKind)
	if (!kind) return null
	const code =
		ymclEnvelopeText(item.code) || ymclEnvelopeText(item.id) || ymclEnvelopeText(item.key) || kind
	const title =
		ymclEnvelopeText(item.title) ||
		ymclEnvelopeText(item.name) ||
		ymclEnvelopeText(item.label) ||
		code
	const paramsSource = item.params ?? item.parameters ?? item.payload
	const params =
		paramsSource && typeof paramsSource === 'object' && !Array.isArray(paramsSource)
			? (paramsSource as Record<string, unknown>)
			: undefined
	const icon = ymclEnvelopeText(item.icon)
	return {
		code,
		title,
		kind,
		primary: item.primary === true || item.is_primary === true || item.isPrimary === true,
		...(params ? { params } : {}),
		...(icon ? { icon } : {}),
	}
}

function pickActions(source: Record<string, unknown>, keys: string[]): YmclAction[] {
	for (const key of keys) {
		const value = source[key]
		if (!Array.isArray(value)) continue
		return value
			.map((entry) => normalizeYmclAction(entry))
			.filter((entry): entry is YmclAction => entry !== null)
	}
	return []
}

function pickAllow(source: Record<string, unknown>): string[] {
	const actions = source.actions
	const nested =
		actions && typeof actions === 'object' && !Array.isArray(actions)
			? (actions as Record<string, unknown>)
			: null
	const candidates = [
		source.allow,
		source.allowed,
		source.actionAllow,
		source.action_allow,
		nested?.allow,
	]
	for (const value of candidates) {
		if (!Array.isArray(value)) continue
		return value
			.map((entry) => asString(entry))
			.filter((entry): entry is string => entry !== null)
	}
	return []
}

function pickRecords(source: Record<string, unknown>): Record<string, unknown>[] {
	const candidates = [
		source.records,
		source.list,
		source.items,
		source.rows,
		source.entries,
		source.data,
	]
	for (const value of candidates) {
		const records = asRecordArray(value)
		if (records.length > 0) return records
	}
	// Last pass: accept an empty-but-present list so "loaded empty" ≠ "missing key".
	for (const key of ['records', 'list', 'items', 'rows', 'entries', 'data'] as const) {
		if (Array.isArray(source[key])) return []
	}
	return []
}

/**
 * yda-style business envelopes wrap the payload in `{code, data}` /
 * `{success, payload}`. Only unwrap when the outer object itself has no
 * records-shaped keys, so a flat envelope is never double-unwrapped.
 */
function unwrapPayload(raw: unknown): unknown {
	if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return raw
	const root = raw as Record<string, unknown>
	const hasRecordKeys = ['records', 'list', 'items', 'rows', 'itemActions', 'item_actions'].some(
		(key) => key in root,
	)
	if (hasRecordKeys) return raw
	for (const key of ['data', 'payload', 'result'] as const) {
		const nested = root[key]
		if (nested && typeof nested === 'object' && !Array.isArray(nested)) return nested
	}
	// `{code, data: [...]}` — treat the array payload as the record list.
	for (const key of ['data', 'payload', 'result'] as const) {
		if (Array.isArray(root[key])) return { records: root[key] }
	}
	return raw
}

function pickSchemaVersion(source: Record<string, unknown>): number | undefined {
	for (const key of ['schemaVersion', 'schema_version'] as const) {
		const value = source[key]
		if (typeof value === 'number' && Number.isFinite(value)) return value
	}
	return undefined
}

/**
 * Canonical envelope for renderers. `fallbackAllow` (manifest actions.allow)
 * is merged with any envelope-level allow so a data-source whitelist is never
 * discarded just because the manifest omitted `actions`.
 */
export function normalizeYmclEnvelope(
	raw: unknown,
	fallbackAllow: string[] = [],
): NormalizedYmclEnvelope {
	const payload = unwrapPayload(raw)
	const root = (payload && typeof payload === 'object' && !Array.isArray(payload)
		? payload
		: {}) as Record<string, unknown>
	const allow = [...new Set([...pickAllow(root), ...fallbackAllow.map((v) => v.trim()).filter(Boolean)])]
	return {
		records: pickRecords(root),
		actions: pickActions(root, ['actions', 'pageActions', 'page_actions']),
		itemActions: pickActions(root, [
			'itemActions',
			'item_actions',
			'recordActions',
			'record_actions',
			'buttons',
			'cardActions',
			'card_actions',
		]),
		allow,
		...(pickSchemaVersion(root) != null ? { schemaVersion: pickSchemaVersion(root) as number } : {}),
	}
}

export function ymclRecordTitle(record: Record<string, unknown>): string {
	for (const key of ['title', 'name', 'label', 'address', 'id'] as const) {
		const value = record[key]
		if (typeof value === 'string' && value.trim()) return value.trim()
		if (typeof value === 'number') return String(value)
	}
	return ''
}

export function ymclRecordCover(record: Record<string, unknown>): string | null {
	for (const key of ['cover', 'icon', 'image', 'banner', 'thumbnail', 'logo'] as const) {
		const value = record[key]
		if (typeof value === 'string' && value.trim()) return value.trim()
	}
	return null
}

export function ymclRecordText(value: unknown): string {
	if (value == null) return ''
	if (typeof value === 'string') return value
	if (typeof value === 'number' || typeof value === 'boolean') return String(value)
	if (Array.isArray(value)) {
		return value
			.map((entry) => ymclRecordText(entry))
			.filter(Boolean)
			.join(', ')
	}
	if (typeof value === 'object') {
		const obj = value as Record<string, unknown>
		for (const key of ['name', 'title', 'label', 'text', 'value'] as const) {
			const nested = ymclRecordText(obj[key])
			if (nested) return nested
		}
	}
	return ''
}

/** Detail-view body field preference order. */
export function ymclRecordBody(record: Record<string, unknown>): string {
	for (const key of ['body', 'content', 'detail', 'description', 'markdown', 'text'] as const) {
		const text = ymclRecordText(record[key])
		if (text.trim()) return text
	}
	return ''
}

/** Secondary summary shown on cards / detail headers. */
export function ymclRecordSummary(record: Record<string, unknown>): string {
	for (const key of ['summary', 'subtitle', 'brief', 'excerpt'] as const) {
		const text = ymclRecordText(record[key])
		if (text.trim()) return text
	}
	const body = ymclRecordBody(record)
	return body.length > 160 ? `${body.slice(0, 160)}…` : body
}
