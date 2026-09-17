<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import {
	type Component,
	computed,
	onBeforeUnmount,
	onErrorCaptured,
	ref,
	shallowRef,
	watch,
} from 'vue'
import * as VueNamespace from 'vue'

import { executeAction } from '@/helpers/ymcl-actions'
import { useYmclStore } from '@/store/ymcl'

import JoinServerModal from '@/components/ymcl/JoinServerModal.vue'

/**
 * ModuleFrame (YAP §6.8 "module" renderer): hot-delivered page as an ESM
 * remote entry. Unlike BundleFrame's sandboxed iframe, the module runs
 * in-process — it is trusted first-party domain content (same trust level
 * as MIP game content). The entry must default-export a factory:
 *
 *   export default function (ctx) {
 *     const { h, ref, onMounted } = ctx.vue
 *     return { component: { setup: () => () => h('div', '…') }, unmount() {} }
 *   }
 *
 * Returning a bare component is also accepted. ctx.host exposes the typed
 * launcher API, gated by the bundle's declared `permissions`.
 */

const props = defineProps<{
	/** The manifest page descriptor with renderer "module". */
	page: {
		id: string
		params?: unknown
		bundle?: {
			id: string
			version: string
			entry: string
			url: string
			sha256: string
			permissions?: string[]
		} | null
	}
	actionAllow?: string[]
}>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	loading: { id: 'app.ymcl.module.loading', defaultMessage: '正在加载模块页面…' },
	failed: {
		id: 'app.ymcl.module.failed',
		defaultMessage: '模块页面加载失败。',
	},
})

const emit = defineEmits<{ reload: [] }>()

const { addNotification } = injectNotificationManager()
const ymclStore = useYmclStore()

const loading = ref(true)
const failed = ref(false)
const moduleComponent = shallowRef<Component | null>(null)
const joinModal = ref<InstanceType<typeof JoinServerModal>>()

const permissions = computed(() => new Set(props.page.bundle?.permissions ?? []))

interface ModuleHandle {
	component?: Component
	unmount?: () => void
}

let cleanup: (() => void) | null = null

function buildHostApi() {
	return {
		permissions: [...permissions.value],
		page: { id: props.page.id, params: props.page.params ?? null },
		reload: () => emit('reload'),
		async dataFetch(providerCode: string, sourceCode: string, query?: unknown) {
			if (!permissions.value.has('data.fetch')) throw new Error('permission denied: data.fetch')
			return invoke('plugin:ymcl|ymcl_data_fetch', { providerCode, sourceCode, query: query ?? null })
		},
			async executeAction(action: {
				code: string
				title: string
				kind: string
				params: Record<string, unknown>
			}) {
				if (!permissions.value.has('action.execute'))
					throw new Error('permission denied: action.execute')
				try {
					const result = await executeAction(action, {
						allow: props.actionAllow ?? [],
						onReload: () => emit('reload'),
					})
					if (!result) throw new Error('action rejected')
					// Server actions may answer {toast, refresh} (YAP §6.7) — surface the
					// toast here and hand the response back so the module can refetch.
					if (typeof result === 'object' && result.toast) {
						addNotification({ type: 'success', title: String(result.toast) })
					}
					return result
				} catch (error) {
					// 失败必须可见：模块侧 catch 只解锁按钮（静默），错误统一在这里
					// 以 toast 透出，否则动作 404/会话失效表现为「按钮点了没反应」。
					addNotification({
						type: 'error',
						title: action.title ? `${action.title}失败` : '操作失败',
						text: error instanceof Error ? error.message : String(error),
					})
					throw error
				}
			},
		async joinServer(address: string) {
			if (!permissions.value.has('action.execute'))
				throw new Error('permission denied: action.execute')
			await joinModal.value?.show({ address })
		},
		async openExternal(url: string) {
			if (!permissions.value.has('open-url')) throw new Error('permission denied: open-url')
			if (!/^https:\/\//.test(url)) throw new Error('only https urls are allowed')
			await openUrl(url)
		},
		/**
		 * Resolve a server-relative URL (e.g. /api/files/{id}/content) against
		 * the active domain origin. Absolute/data/blob URLs pass through.
		 * Images on the domain host are anonymous and img-src already allows
		 * http:/https:, so the result is directly usable as <img src>.
		 */
		resolveUrl(url: string) {
			if (!url) return ''
			if (/^(?:[a-z][a-z0-9+.-]*:)?\/\//i.test(url) || /^(?:data|blob):/i.test(url)) return url
			const origin = ymclStore.activeDomain?.origin
			if (!origin) return url
			return origin.replace(/\/+$/, '') + (url.startsWith('/') ? url : `/${url}`)
		},
		async theme() {
			if (!permissions.value.has('theme.read')) throw new Error('permission denied: theme.read')
			const { useYmclThemeStore } = await import('@/store/ymcl-theme')
			return useYmclThemeStore().profile
		},
	}
}

async function loadModule() {
	loading.value = true
	failed.value = false
	moduleComponent.value = null
	cleanup?.()
	cleanup = null
	if (!props.page.bundle) {
		failed.value = true
		loading.value = false
		return
	}
	let blobUrl: string | null = null
	try {
		const bundle = await invoke<{ entry_path: string; entry_source: string }>(
			'plugin:ymcl|ymcl_bundle_get',
			{
				pageId: props.page.id,
			},
		)
		// Blob URL keeps the dynamic import inside the page origin so the module
		// can construct components against the launcher's own Vue instance. The
		// source comes over IPC — fetching entry_path via asset.localhost is
		// blocked by CSP connect-src.
		const source = bundle.entry_source
		blobUrl = URL.createObjectURL(new Blob([source], { type: 'text/javascript' }))
		const mod = (await import(/* @vite-ignore */ blobUrl)) as {
			default?: (ctx: { vue: typeof VueNamespace; host: ReturnType<typeof buildHostApi> }) =>
				| ModuleHandle
				| Component
		}
		const factory = mod.default
		if (typeof factory !== 'function') throw new Error('module has no default factory export')
		const handle = factory({ vue: VueNamespace, host: buildHostApi() })
		const component =
			handle && typeof handle === 'object' && 'component' in handle
				? (handle as ModuleHandle).component
				: (handle as Component)
		if (!component) throw new Error('module factory returned no component')
		moduleComponent.value = component
		cleanup = () => {
			if (handle && typeof handle === 'object' && 'unmount' in handle) {
				try {
					(handle as ModuleHandle).unmount?.()
				} catch {
					// module cleanup must not break navigation away
				}
			}
			if (blobUrl) URL.revokeObjectURL(blobUrl)
		}
	} catch {
		failed.value = true
		if (blobUrl) URL.revokeObjectURL(blobUrl)
	} finally {
		loading.value = false
	}
}

onErrorCaptured(() => {
	failed.value = true
	return false
})

watch(
	[() => props.page.id, () => props.page.bundle?.version],
	() => void loadModule(),
	{ immediate: true },
)

onBeforeUnmount(() => cleanup?.())
</script>

<template>
	<div class="flex min-h-96 flex-1 flex-col">
		<component :is="moduleComponent" v-if="moduleComponent && !failed" />
		<div
			v-else-if="loading"
			class="flex min-h-96 flex-1 items-center justify-center rounded-xl bg-bg-raised text-sm text-secondary"
		>
			{{ formatMessage(messages.loading) }}
		</div>
		<div
			v-else
			class="flex min-h-96 flex-1 items-center justify-center rounded-xl bg-bg-raised text-sm text-secondary"
		>
			{{ formatMessage(messages.failed) }}
		</div>
		<JoinServerModal ref="joinModal" />
	</div>
</template>
