// Shared runner for YAP item/page actions used by domain renderers.
// Surfaces adapter/Tauri failures as readable toasts (never "[object Object]").

import { injectNotificationManager } from '@modrinth/ui'

import {
	executeAction,
	isYmclActionExecutable,
	renderActionParams,
	type YmclAction,
	type YmclServerActionResponse,
} from '@/helpers/ymcl-actions'
import { ymclDisplayText, ymclErrorMessage } from '@/helpers/ymcl'

export interface RunYmclActionOptions {
	allow: string[]
	record?: Record<string, unknown>
	onReload?: () => void
	/** Called for `client:open-detail` so the host can open its detail surface. */
	onOpenDetail?: () => void
	/** Localized fallback text when the launcher refuses or the action no-ops. */
	actionUnavailableText: string
	/** Optional join modal hook for `client:launch-server` without instanceId. */
	onLaunchServerWithoutInstance?: (address: string) => void
}

function actionLabel(action: YmclAction): string {
	return ymclDisplayText(action.title) || ymclDisplayText(action.code) || '操作'
}

export function createYmclActionRunner() {
	const { addNotification } = injectNotificationManager()

	async function runYmclAction(
		action: YmclAction,
		options: RunYmclActionOptions,
	): Promise<boolean> {
		const title = actionLabel(action)
		if (action.kind === 'client:reload') {
			options.onReload?.()
			return true
		}
		if (action.kind === 'client:open-detail') {
			options.onOpenDetail?.()
			return true
		}
		if (action.kind === 'client:launch-server') {
			const params = renderActionParams(action, options.record ?? {})
			if (!params.instanceId && params.address && options.onLaunchServerWithoutInstance) {
				options.onLaunchServerWithoutInstance(String(params.address))
				return true
			}
		}
		try {
			const response = (await executeAction(action, {
				allow: options.allow,
				record: options.record,
				onReload: options.onReload,
			})) as boolean | YmclServerActionResponse
			if (response === false) {
				addNotification({
					title,
					text: options.actionUnavailableText,
					type: 'warning',
				})
				return false
			}
			const toast =
				typeof response === 'object' && response
					? ymclDisplayText(response.toast) || ymclDisplayText((response as { message?: unknown }).message)
					: ''
			if (toast) {
				addNotification({ title, text: toast, type: 'success' })
			}
			if (typeof response !== 'boolean' && response.refresh !== false) options.onReload?.()
			return true
		} catch (error) {
			const toast =
				(error as { ymclToast?: string } | null)?.ymclToast ||
				ymclErrorMessage(error) ||
				options.actionUnavailableText
			addNotification({
				title,
				text: toast === '[object Object]' ? options.actionUnavailableText : toast,
				type: 'error',
			})
			return false
		}
	}

	return { runYmclAction }
}

/**
 * Preferred primary action for a record. Adapter-declared `primary` wins so
 * buttons like 报名 stay visible even when local preflight cannot prove params.
 */
export function primaryYmclAction(
	itemActions: YmclAction[],
	record: Record<string, unknown>,
	allow: string[],
): YmclAction | null {
	return (
		itemActions.find((action) => action.primary && action.kind !== 'client:open-detail') ??
		itemActions.find((action) => action.primary) ??
		itemActions.find((action) => isYmclActionExecutable(action, record, allow)) ??
		null
	)
}

export function secondaryYmclActions(
	itemActions: YmclAction[],
	record: Record<string, unknown>,
	allow: string[],
	primary: YmclAction | null,
): YmclAction[] {
	return itemActions.filter(
		(action) => action.code !== primary?.code && isYmclActionExecutable(action, record, allow),
	)
}
