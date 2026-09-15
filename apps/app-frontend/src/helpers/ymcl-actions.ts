// YAP action executor (YAP §6.7): actions are data (kind + parameter
// templates), executed generically by the launcher. Only whitelisted kinds
// run; template values resolve from the record via {{item.path}} strings.
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'

import { isOfflineMode } from '@/composables/useNetworkStatus'
import { resolveGcLaunchIntent } from '@/helpers/instance'
import type { YmclNavigationItem } from '@/helpers/ymcl'

export interface YmclAction {
	code: string
	title: string
	kind: string
	primary?: boolean
	params?: Record<string, unknown>
	icon?: string
}

const CLIENT_ACTION_KINDS = new Set([
	'client:launch-server',
	'client:open-url',
	'client:copy',
	'client:install-pack',
	'client:launch-instance',
	'client:reload',
])

/** Allowed kinds for a page, intersected with the built-in client set. */
export function isActionAllowed(action: YmclAction, allow: string[] = []): boolean {
	if (!CLIENT_ACTION_KINDS.has(action.kind)) {
		// server:* and extension actions are forwarded to the adapter instead
		return allow.some((pattern) =>
			pattern.endsWith(':*')
				? action.kind.startsWith(pattern.slice(0, -1))
				: action.kind === pattern,
		)
	}
	return (
		allow.length === 0 ||
		allow.some((pattern) =>
			pattern.endsWith(':*')
				? action.kind.startsWith(pattern.slice(0, -1))
				: action.kind === pattern,
		)
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

/** Executes an action or returns false when the kind is not permitted. */
export async function executeAction(action: YmclAction, context: ActionContext): Promise<boolean> {
	if (!isActionAllowed(action, context.allow)) return false
	const params = renderActionParams(action, context.record ?? {})

	if (action.kind.startsWith('server:')) {
		const [providerCode, actionCode] = action.kind.split(':').slice(1)
		await invoke('plugin:ymcl|ymcl_action_execute', {
			providerCode: providerCode ?? '',
			actionCode: actionCode ?? action.code,
			params,
		})
		return true
	}

	switch (action.kind) {
		case 'client:open-url': {
			const url = String(params.url ?? '')
			if (!/^https:\/\//.test(url)) return false
			await openUrl(url)
			return true
		}
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
			await invoke('plugin:ymcl|ymcl_pack_apply_update', { instanceId })
			return true
		}
		case 'client:copy-server':
			return false
		default:
			return false
	}
}
