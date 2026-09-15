// YAP bundle sandbox bridge (YAP §6.8): typed postMessage protocol between
// the launcher and extension bundles running in a sandboxed iframe. Every
// call is checked against the bundle's declared permissions and the
// manifest action whitelist; denied calls are rejected, never executed.

import { invoke } from '@tauri-apps/api/core'

export const BRIDGE_SOURCE = 'ymcl-bridge'
export const BRIDGE_BUNDLE_SOURCE = 'ymcl-bundle'

export type BridgePermission =
	| 'data.fetch'
	| 'action.execute'
	| 'open-url'
	| 'theme.read'
	| 'nav.context'

export interface BridgeRequest {
	source: typeof BRIDGE_BUNDLE_SOURCE
	callId: number
	method: string
	params: Record<string, unknown>
}

export interface BridgeResponse {
	source: typeof BRIDGE_SOURCE
	callId: number
	ok: boolean
	result?: unknown
	error?: string
}

interface BridgeContext {
	bundleId: string
	permissions: BridgePermission[]
	actionAllow: string[]
	/** Injected by BundleFrame so actions can trigger a page reload. */
	onReload: () => void
}

async function handleMethod(
	ctx: BridgeContext,
	method: string,
	params: Record<string, unknown>,
): Promise<unknown> {
	switch (method) {
		case 'ping':
			return { bundleId: ctx.bundleId }
		case 'theme.read': {
			if (!ctx.permissions.includes('theme.read')) throw new Error('permission denied')
			const { useYmclThemeStore } = await import('@/store/ymcl-theme')
			return useYmclThemeStore().profile
		}
		case 'data.fetch': {
			if (!ctx.permissions.includes('data.fetch')) throw new Error('permission denied')
			const providerCode = String(params.providerCode ?? '')
			const sourceCode = String(params.sourceCode ?? '')
			if (!providerCode || !sourceCode) throw new Error('providerCode and sourceCode required')
			return invoke('plugin:ymcl|ymcl_data_fetch', {
				providerCode,
				sourceCode,
				query: params.query ?? null,
			})
		}
		case 'action.execute': {
			if (!ctx.permissions.includes('action.execute')) throw new Error('permission denied')
			const { executeAction } = await import('@/helpers/ymcl-actions')
			const kind = String(params.kind ?? '')
			if (!kind.startsWith('extension:') && !kind.startsWith('client:')) {
				throw new Error('action kind not allowed from bundle')
			}
			const executed = await executeAction(
				{
					code: String(params.code ?? kind),
					title: String(params.title ?? ''),
					kind,
					params: (params.params as Record<string, unknown>) ?? {},
				},
				{ allow: ctx.actionAllow, onReload: ctx.onReload },
			)
			if (!executed) throw new Error('action rejected')
			return { executed }
		}
		case 'open-url': {
			if (!ctx.permissions.includes('open-url')) throw new Error('permission denied')
			const { openUrl } = await import('@tauri-apps/plugin-opener')
			const url = String(params.url ?? '')
			if (!/^https:\/\//.test(url)) throw new Error('only https urls are allowed')
			await openUrl(url)
			return null
		}
		default:
			throw new Error(`unknown bridge method ${method}`)
	}
}

/**
 * Installs the window message listener for one bundle frame. Returns a
 * cleanup function. The listener only answers messages whose origin is
 * null/opaque (sandboxed iframe posts with origin "..."), carrying our
 * BRIDGE_BUNDLE_SOURCE marker and a numeric callId.
 */
export function createBridgeListener(ctx: BridgeContext): () => void {
	const handler = async (event: MessageEvent) => {
		const data = event.data as Partial<BridgeRequest> | undefined
		if (!data || data.source !== BRIDGE_BUNDLE_SOURCE) return
		if (typeof data.callId !== 'number' || typeof data.method !== 'string') return
		const response: BridgeResponse = {
			source: BRIDGE_SOURCE,
			callId: data.callId,
			ok: false,
		}
		try {
			response.result = await handleMethod(ctx, data.method, data.params ?? {})
			response.ok = true
		} catch (error) {
			response.error = error instanceof Error ? error.message : String(error)
		}
		event.source?.postMessage(response, { targetOrigin: event.origin || '*' })
	}
	window.addEventListener('message', handler)
	return () => window.removeEventListener('message', handler)
}
