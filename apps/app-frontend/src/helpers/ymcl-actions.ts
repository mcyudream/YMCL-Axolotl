// YAP action executor (YAP §6.7): actions are data (kind + parameter
// templates), executed generically by the launcher. An empty allow list
// trusts adapter-declared actions; an explicit whitelist is enforced.
// Template values resolve from the record via {{item.path}} strings.
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'

import { isOfflineMode } from '@/composables/useNetworkStatus'
import { resolveGcLaunchIntent } from '@/helpers/instance'
import type { YmclNavigationItem } from '@/helpers/ymcl'
import { interpretYmclServerActionResponse } from '@/helpers/ymcl'

export interface YmclAction {
	code: string
	title: string
	kind: string
	primary?: boolean
	params?: Record<string, unknown>
	icon?: string
}

/**
 * Allowed kinds for a page. An empty/missing allow list treats every
 * adapter-declared action as executable — the envelope itself is the
 * preset. When the manifest (or envelope) supplies a whitelist, it is
 * enforced for both client and server kinds.
 */
export function isActionAllowed(action: YmclAction, allow: string[] = []): boolean {
	if (allow.length === 0) return true
	return allow.some((pattern) =>
		pattern.endsWith(':*')
			? action.kind.startsWith(pattern.slice(0, -1))
			: action.kind === pattern,
	)
}

/**
 * Renders `{{item.path}}` templates against a record. Values are coerced to
 * strings/numbers only — no expressions (YAP §6.6).
 */
export function renderTemplate(value: unknown, record: Record<string, unknown>): unknown {
	if (typeof value !== 'string') return value
	const match = /^\{\{item\.([\w.]+)\}\}$/.exec(value.trim())
	if (!match) return value
	let current: unknown = record
	for (const segment of match[1].split('.')) {
		if (current == null || typeof current !== 'object') return undefined
		current = (current as Record<string, unknown>)[segment]
	}
	return current
}

export function renderActionParams(
	action: YmclAction,
	record: Record<string, unknown>,
): Record<string, unknown> {
	const rendered: Record<string, unknown> = {}
	for (const [key, value] of Object.entries(action.params ?? {})) {
		rendered[key] = renderTemplate(value, record)
	}
	return rendered
}

export interface ActionContext {
	/** manifest actions.allow whitelist for the active domain */
	allow: string[]
	record?: Record<string, unknown>
	navigation?: YmclNavigationItem[]
	onReload?: () => void
}

/**
 * Preflight used by renderers to decide whether a primary/secondary button
 * should be interactive. Unknown `server:*` kinds stay clickable when the
 * allow list permits them — the adapter owns the final check.
 */
export function isYmclActionExecutable(
	action: YmclAction,
	record: Record<string, unknown>,
	allow: string[] = [],
): boolean {
	if (!isActionAllowed(action, allow)) return false
	if (action.kind.startsWith('server:')) return true
	const params = renderActionParams(action, record)
	switch (action.kind) {
		case 'client:reload':
		case 'client:open-detail':
			return true
		case 'client:open-url':
			return /^https:\/\//.test(String(params.url ?? ''))
		case 'client:copy':
			return String(params.text ?? '').length > 0
		case 'client:launch-instance':
			return !!params.instanceId
		case 'client:launch-server':
			return !!params.address
		case 'client:install-pack':
			return !!params.instanceId || !!params.packId
		case 'client:open-page':
			return !!(params.pageId ?? params.page ?? params.id)
		case 'client:navigate': {
			const route = String(params.route ?? params.path ?? params.url ?? '')
			return route.startsWith('/') || route.startsWith('native:') || /^https:\/\//.test(route)
		}
		default:
			return action.kind.startsWith('client:') && Object.keys(params).length > 0
	}
}

/**
 * YAP §6.7 server action response contract: the adapter may answer with
 * `{toast, refresh}` feedback; both fields are optional and the body may
 * be empty.
 */
export interface YmclServerActionResponse {
	toast?: string
	refresh?: boolean
}

/**
 * Parses `server:*` kind dialects into the provider/action pair the adapter
 * expects: `server:provider:action`, `server:provider.action`,
 * `server:provider/action`, or a bare action code.
 */
export function parseServerActionKind(
	action: YmclAction,
	record: Record<string, unknown> = {},
): { providerCode: string; actionCode: string } {
	const rest = action.kind.startsWith('server:')
		? action.kind.slice('server:'.length)
		: action.kind
	const recordProvider =
		(typeof record.providerCode === 'string' && record.providerCode) ||
		(typeof record.provider_code === 'string' && record.provider_code) ||
		(typeof record.provider === 'string' && record.provider) ||
		null
	if (!rest) {
		return { providerCode: recordProvider ?? '', actionCode: action.code }
	}
	const colon = rest.split(':')
	if (colon.length >= 2 && colon[0] && colon.slice(1).join(':')) {
		return { providerCode: colon[0], actionCode: colon.slice(1).join(':') }
	}
	for (const sep of ['.', '/'] as const) {
		const index = rest.indexOf(sep)
		if (index > 0 && index < rest.length - 1) {
			return { providerCode: rest.slice(0, index), actionCode: rest.slice(index + 1) }
		}
	}
	// Single segment: prefer record/adapter provider, treat rest as action code.
	return {
		providerCode: recordProvider ?? rest,
		actionCode: action.code && action.code !== rest ? action.code : rest,
	}
}

/**
 * Fills missing identity keys adapters commonly need for 报名/加入 style
 * actions when the envelope omitted `{{item.*}}` templates.
 */
export function withRecordIdentity(
	params: Record<string, unknown>,
	record: Record<string, unknown>,
): Record<string, unknown> {
	const result = { ...params }
	const id = record.id ?? record.activityId ?? record.activity_id ?? record.code
	const activityId = record.activityId ?? record.activity_id ?? record.id
	const fill = (key: string, value: unknown) => {
		const current = result[key]
		if (current == null || current === '') result[key] = value
	}
	fill('id', id)
	fill('activityId', activityId)
	fill('activity_id', activityId)
	fill('recordId', record.id)
	fill('record_id', record.id)
	fill('item_id', record.id)
	fill('targetId', record.targetId ?? id)
	return result
}

/**
 * Launch-time binding check (YAP §7): a MIP-managed instance is brought up
 * to the domain binding's current version before launching. Non-managed
 * instances and domains without MIP pass straight through.
 */
async function preLaunchUpdate(instanceId: string): Promise<void> {
	const check = await invoke<{
		managed: boolean
		target_version?: string | null
		pending_changes?: number | null
	}>('plugin:ymcl|ymcl_pre_launch_check', { instanceId })
	if (check.managed && check.pending_changes && check.pending_changes > 0) {
		await invoke('plugin:ymcl|ymcl_pack_apply_update', { instanceId })
	}
}

/**
 * Executes an action. Returns `false` when the kind is not permitted;
 * `server:*` actions resolve with the adapter's response envelope (or
 * `true` when it answers empty). Errors reject — callers surface them.
 */
export async function executeAction(
	action: YmclAction,
	context: ActionContext,
): Promise<boolean | YmclServerActionResponse> {
	if (!isActionAllowed(action, context.allow)) return false
	const params = renderActionParams(action, context.record ?? {})

	if (action.kind.startsWith('server:')) {
		const { providerCode, actionCode } = parseServerActionKind(action, context.record ?? {})
		const params = withRecordIdentity(
			renderActionParams(action, context.record ?? {}),
			context.record ?? {},
		)
		const response = await invoke<unknown>('plugin:ymcl|ymcl_action_execute', {
			providerCode,
			actionCode,
			params,
		})
		const interpreted = interpretYmclServerActionResponse(response)
		if (!interpreted.ok) {
			const error = new Error(interpreted.toast || '该操作暂不可用')
			// Preserve adapter feedback for the notification layer.
			;(error as Error & { ymclToast?: string }).ymclToast = interpreted.toast
			throw error
		}
		return {
			...(interpreted.toast ? { toast: interpreted.toast } : {}),
			...(interpreted.refresh != null ? { refresh: interpreted.refresh } : {}),
		}
	}

	switch (action.kind) {
		case 'client:open-url': {
			const url = String(params.url ?? '')
			if (!/^https:\/\//.test(url)) return false
			await openUrl(url)
			return true
		}
		case 'client:open-page': {
			const pageId = String(params.pageId ?? params.page ?? params.id ?? '')
			if (!pageId) return false
			const { domainPageRoute } = await import('@/helpers/ymcl-domain')
			const router = (await import('@/routes')).default
			await router.push(domainPageRoute(pageId))
			return true
		}
		case 'client:navigate': {
			const raw = String(params.route ?? params.path ?? params.url ?? '')
			if (!raw) return false
			if (/^https:\/\//.test(raw)) {
				await openUrl(raw)
				return true
			}
			const path = raw.startsWith('native:') ? raw.slice('native:'.length) : raw
			if (!path.startsWith('/')) return false
			const router = (await import('@/routes')).default
			await router.push(path)
			return true
		}
		case 'client:open-detail':
			// Renderers open their record detail surface; the action itself is a no-op.
			return true
		case 'client:copy': {
			await navigator.clipboard.writeText(String(params.text ?? ''))
			return true
		}
		case 'client:launch-server': {
			const address = String(params.address ?? '')
			const instanceId = String(params.instanceId ?? '')
			// Server join always runs through a local instance (quick play);
			// without a bound instance the action is a no-op.
			if (!address || !instanceId) return false
			// Hot update before launch (YAP §7 step 4): failures abort the
			// launch so the player never joins an under-updated server.
			await preLaunchUpdate(instanceId)
			const { args, gcIntent } = await resolveGcLaunchIntent(instanceId)
			await invoke('plugin:instance|instance_run', {
				instanceId,
				serverAddress: address,
				offlineMode: isOfflineMode(),
				extraLaunchArgs: args,
				gcIntent,
			})
			return true
		}
		case 'client:launch-instance': {
			const instanceId = String(params.instanceId ?? '')
			if (!instanceId) return false
			await preLaunchUpdate(instanceId)
			await invoke('plugin:instance|instance_run', { instanceId })
			return true
		}
		case 'client:reload': {
			context.onReload?.()
			return true
		}
		case 'client:install-pack': {
			// Applies the bound pack version to an existing MIP instance
			// (WF-5). First-time installation into a fresh instance is part
			// of the publish flow, not the join action.
			const instanceId = String(params.instanceId ?? '')
			if (!instanceId) return false
			await preLaunchUpdate(instanceId)
			await invoke('plugin:ymcl|ymcl_pack_apply_update', { instanceId })
			return true
		}
		case 'client:copy-server':
			return false
		default:
			return false
	}
}
