<script setup lang="ts">
import { CompassIcon, RefreshCwIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { computed, defineAsyncComponent, ref, watch } from 'vue'
import { useRoute } from 'vue-router'

import type { YmclAction } from '@/helpers/ymcl-actions'
import { normalizeYmclEnvelope } from '@/helpers/ymcl-envelope'
import { resolveDataSourceTitle, splitDataSourceId } from '@/helpers/ymcl-home'
import { normalizeRendererId } from '@/helpers/ymcl-nav'
import { ymclDisplayText, ymclErrorMessage } from '@/helpers/ymcl'
import { useYmclStore } from '@/store/ymcl'

import DomainSubNav from '@/components/ymcl/DomainSubNav.vue'

const route = useRoute()
const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()
const ymclStore = useYmclStore()

const messages = defineMessages({
	title: {
		id: 'app.ymcl.domain-page.title',
		defaultMessage: '域页面',
	},
	pageUnavailable: {
		id: 'app.ymcl.domain-page.unavailable',
		defaultMessage:
			'This page is not available. It may have been removed by the domain, or requires a newer launcher version.',
	},
	notInDomain: {
		id: 'app.ymcl.domain-page.not-in-domain',
		defaultMessage: '域页面仅在加入域后可用。',
	},
	rendererUnknown: {
		id: 'app.ymcl.domain-page.renderer-unknown',
		defaultMessage:
			'This page uses the "{renderer}" renderer, which is not available in this launcher build.',
	},
	loadFailed: {
		id: 'app.ymcl.domain-page.load-failed',
		defaultMessage: '无法从域加载此页面。',
	},
	refresh: {
		id: 'app.ymcl.domain-page.refresh',
		defaultMessage: '刷新',
	},
	actionUnavailable: {
		id: 'app.ymcl.renderer.action-unavailable',
		defaultMessage: '该操作暂不可用',
	},
})

/** Built-in declarative renderers (YAP §6.6); extension bundles land in P4. */
const renderers: Record<string, ReturnType<typeof defineAsyncComponent>> = {
	'server-list': defineAsyncComponent(
		() => import('@/components/ymcl/renderers/ServerListRenderer.vue'),
	),
	'card-grid': defineAsyncComponent(
		() => import('@/components/ymcl/renderers/CardGridRenderer.vue'),
	),
	'article-list': defineAsyncComponent(
		() => import('@/components/ymcl/renderers/ArticleListRenderer.vue'),
	),
	'rich-text': defineAsyncComponent(
		() => import('@/components/ymcl/renderers/RichTextRenderer.vue'),
	),
	'pack-catalog': defineAsyncComponent(
		() => import('@/components/ymcl/renderers/PackCatalogRenderer.vue'),
	),
	stats: defineAsyncComponent(() => import('@/components/ymcl/renderers/StatsRenderer.vue')),
}

const BundleFrame = defineAsyncComponent(() => import('@/components/ymcl/bundle/BundleFrame.vue'))
const ModuleFrame = defineAsyncComponent(() => import('@/components/ymcl/bundle/ModuleFrame.vue'))

const pageId = computed(() => String(route.params.pageId ?? ''))
/**
 * Registry page, or a synthetic descriptor when the route targets a bare
 * dataSource id (adapters may declare data without a matching page node).
 */
const page = computed(() => {
	const registered = ymclStore.pageById(pageId.value)
	if (registered) return registered
	const source = (ymclStore.manifest?.data_sources ?? []).find(
		(candidate) => candidate.id === pageId.value,
	)
	if (!source?.id) return null
	const sourceCode = source.source_code ?? source.sourceCode ?? ''
	// Prefer a registry page that already binds this dataSource (admin renderer).
	const bound = (ymclStore.manifest?.pages ?? []).find(
		(candidate) => candidate.data_source === source.id,
	)
	if (bound) return bound
	return {
		id: source.id,
		renderer: normalizeRendererId(
			sourceCode.includes('server') ? 'server-list' : 'card-grid',
		),
		title: null as string | null,
		data_source: source.id,
		permissions: [] as string[],
	}
})
const pageTitle = computed(() => {
	if (page.value?.title?.trim()) return page.value.title.trim()
	const sourceId = page.value?.data_source
	if (sourceId) {
		const declared = (ymclStore.manifest?.data_sources ?? []).find(
			(candidate) => candidate.id === sourceId,
		)
		const declarationTitle = declared?.title ?? declared?.name ?? declared?.label
		if (declarationTitle?.trim() && declarationTitle.trim() !== sourceId) {
			return declarationTitle.trim()
		}
		const boundPage = (ymclStore.manifest?.pages ?? []).find(
			(candidate) => candidate.data_source === sourceId && candidate.title?.trim(),
		)
		if (boundPage?.title?.trim()) return boundPage.title.trim()
		return resolveDataSourceTitle({
			id: sourceId,
			cardTitle: null,
			declarationTitle,
			pageTitle: null,
			navigationTitle: null,
		})
	}
	return page.value?.id ?? formatMessage(messages.title)
})
const rendererComponent = computed(() => {
	const id = normalizeRendererId(page.value?.renderer)
	return id ? (renderers[id] ?? null) : null
})
const isExtensionRenderer = computed(
	() => normalizeRendererId(page.value?.renderer) === 'extension',
)
const isModuleRenderer = computed(() => normalizeRendererId(page.value?.renderer) === 'module')

const rawEnvelope = ref<unknown>(null)
const loading = ref(false)
const loadError = ref<string | null>(null)
const reloadTick = ref(0)

const manifestAllow = computed(() => {
	const actions = ymclStore.manifest?.actions as { allow?: string[] } | undefined
	return actions?.allow ?? []
})

/** Adapter dialects + manifest allow merged so item buttons stay clickable. */
const envelope = computed(() => normalizeYmclEnvelope(rawEnvelope.value, manifestAllow.value))

const allow = computed(() => envelope.value.allow)

const pageActions = computed(() => envelope.value.actions as YmclAction[])

async function loadEnvelope() {
	if (!page.value?.data_source) return
	// module 页面自持数据拉取（ModuleFrame host.dataFetch），这里再拉一遍
	// 纯属浪费；dataSource 仅作数据源→页面绑定供首页卡片「全部」反查。
	if (isModuleRenderer.value) return
	// `<providerCode>.<sourceCode>` — sourceCode may contain dots (e.g. servers.list).
	const split = splitDataSourceId(page.value.data_source)
	if (!split) return
	loading.value = true
	loadError.value = null
	try {
		const { invoke } = await import('@tauri-apps/api/core')
		rawEnvelope.value = await invoke<unknown>('plugin:ymcl|ymcl_data_fetch', {
			providerCode: split.providerCode,
			sourceCode: split.sourceCode,
			query: null,
		})
	} catch (error) {
		loadError.value = ymclErrorMessage(error)
		rawEnvelope.value = null
	} finally {
		loading.value = false
	}
}

async function runPageAction(action: YmclAction) {
	const { executeAction } = await import('@/helpers/ymcl-actions')
	const title = ymclDisplayText(action.title) || ymclDisplayText(action.code) || '操作'
	try {
		const response = await executeAction(action, {
			allow: allow.value,
			onReload: () => {
				reloadTick.value += 1
				void loadEnvelope()
			},
		})
		if (response === false) {
			addNotification({
				title,
				text: formatMessage(messages.actionUnavailable),
				type: 'warning',
			})
			return
		}
		const toast =
			typeof response === 'object' && response
				? ymclDisplayText(response.toast) ||
					ymclDisplayText((response as { message?: unknown }).message)
				: ''
		if (toast) {
			addNotification({ title, text: toast, type: 'success' })
		}
		if (typeof response !== 'boolean' && response.refresh !== false) {
			reloadTick.value += 1
			void loadEnvelope()
		}
	} catch (error) {
		const toast =
			(error as { ymclToast?: string } | null)?.ymclToast ||
			ymclErrorMessage(error) ||
			formatMessage(messages.actionUnavailable)
		addNotification({
			title,
			text: toast === '[object Object]' ? formatMessage(messages.actionUnavailable) : toast,
			type: 'error',
		})
	}
}

function handlePageReload() {
	reloadTick.value += 1
	void loadEnvelope()
}

watch(
	[pageId, () => ymclStore.manifest, reloadTick, () => ymclStore.dataEpoch],
	() => {
		if (!ymclStore.isPersonal && page.value?.data_source) {
			void loadEnvelope()
		} else {
			rawEnvelope.value = null
			loadError.value = null
		}
	},
	{ immediate: true },
)
</script>

<template>
	<div class="domain-page-layout flex min-h-full gap-6 p-6">
		<DomainSubNav v-if="!ymclStore.isPersonal" :page-id="pageId" />
		<div class="flex min-h-full min-w-0 flex-1 flex-col">
			<template v-if="ymclStore.isPersonal">
			<div class="flex min-h-full flex-col items-center justify-center gap-3 text-center">
				<CompassIcon class="h-12 w-12 text-secondary" />
				<h1 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.notInDomain) }}
				</h1>
			</div>
		</template>
		<template v-else-if="!page">
			<div class="flex min-h-full flex-col items-center justify-center gap-3 text-center">
				<CompassIcon class="h-12 w-12 text-secondary" />
				<h1 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.pageUnavailable) }}
				</h1>
			</div>
		</template>
		<template v-else>
			<div class="mb-4 flex items-center gap-3">
				<h1 class="m-0 text-2xl font-bold text-contrast">{{ pageTitle }}</h1>
				<div class="flex-1"></div>
				<ButtonStyled
					v-for="action in pageActions"
					:key="action.code"
					:color="action.primary ? 'brand' : 'standard'"
				>
					<button @click="runPageAction(action)">
						{{ action.title }}
					</button>
				</ButtonStyled>
				<ButtonStyled v-if="page.data_source && !isModuleRenderer" type="standard" circular>
					<button
						v-tooltip="formatMessage(messages.refresh)"
						:disabled="loading"
						@click="loadEnvelope"
					>
						<RefreshCwIcon :class="{ 'animate-spin': loading }" />
					</button>
				</ButtonStyled>
			</div>

			<div v-if="loadError" class="rounded-xl bg-bg-raised p-6 text-center text-sm text-secondary">
				{{ formatMessage(messages.loadFailed) }}
			</div>
			<BundleFrame
				v-else-if="isExtensionRenderer && page"
				:page="page"
				@reload="handlePageReload"
			/>
			<ModuleFrame
				v-else-if="isModuleRenderer && page"
				:page="page"
				:action-allow="allow"
				@reload="handlePageReload"
			/>
			<div v-else-if="rendererComponent && rawEnvelope !== null">
				<component
					:is="rendererComponent"
					:envelope="envelope"
					@reload="handlePageReload"
				/>
			</div>
			<div
				v-else-if="rendererComponent && loading"
				class="rounded-xl bg-bg-raised p-6 text-center text-sm text-secondary"
			>
				{{ formatMessage(messages.title) }}…
			</div>
			<div
				v-else-if="!rendererComponent"
				class="rounded-xl bg-bg-raised p-6 text-center text-sm text-secondary"
			>
				{{
					formatMessage(messages.rendererUnknown, {
						renderer: page.renderer ?? page.id,
					})
				}}
			</div>
			</template>
		</div>
	</div>
</template>

<style scoped>
@media (max-width: 800px) {
	.domain-page-layout {
		flex-direction: column;
	}
}
</style>