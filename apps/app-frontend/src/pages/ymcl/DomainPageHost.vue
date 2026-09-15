<script setup lang="ts">
import { CompassIcon, RefreshCwIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, useVIntl } from '@modrinth/ui'
import { computed, defineAsyncComponent, ref, watch } from 'vue'
import { useRoute } from 'vue-router'

import type { YmclAction } from '@/helpers/ymcl-actions'
import { useYmclStore } from '@/store/ymcl'

const route = useRoute()
const { formatMessage } = useVIntl()
const ymclStore = useYmclStore()

const messages = defineMessages({
	title: {
		id: 'app.ymcl.domain-page.title',
		defaultMessage: 'Domain page',
	},
	pageUnavailable: {
		id: 'app.ymcl.domain-page.unavailable',
		defaultMessage:
			'This page is not available. It may have been removed by the domain, or requires a newer launcher version.',
	},
	notInDomain: {
		id: 'app.ymcl.domain-page.not-in-domain',
		defaultMessage: 'Domain pages are only available while a domain is active.',
	},
	rendererUnknown: {
		id: 'app.ymcl.domain-page.renderer-unknown',
		defaultMessage:
			'This page uses the "{renderer}" renderer, which is not available in this launcher build.',
	},
	loadFailed: {
		id: 'app.ymcl.domain-page.load-failed',
		defaultMessage: 'Could not load this page from the domain.',
	},
	refresh: {
		id: 'app.ymcl.domain-page.refresh',
		defaultMessage: 'Refresh',
	},
})

interface Envelope {
	records?: Record<string, unknown>[]
	actions?: YmclAction[]
	itemActions?: YmclAction[]
	allow?: string[]
	schemaVersion?: number
}

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

const pageId = computed(() => String(route.params.pageId ?? ''))
const page = computed(() => ymclStore.pageById(pageId.value))
const pageTitle = computed(
	() => page.value?.title ?? page.value?.id ?? formatMessage(messages.title),
)
const rendererComponent = computed(() => {
	const id = page.value?.renderer
	return id ? (renderers[id] ?? null) : null
})
const isExtensionRenderer = computed(() => page.value?.renderer === 'extension')

const envelope = ref<Envelope | null>(null)
const loading = ref(false)
const loadError = ref<string | null>(null)
const reloadTick = ref(0)

async function loadEnvelope() {
	if (!page.value?.data_source) return
	const [providerCode, sourceCode] = page.value.data_source.split('.')
	if (!providerCode || !sourceCode) return
	loading.value = true
	loadError.value = null
	try {
		const { invoke } = await import('@tauri-apps/api/core')
		envelope.value = await invoke<Envelope>('plugin:ymcl|ymcl_data_fetch', {
			providerCode,
			sourceCode,
			query: null,
		})
	} catch (error) {
		loadError.value = String(error)
		envelope.value = null
	} finally {
		loading.value = false
	}
}

const allow = computed(() => {
	const actions = ymclStore.manifest?.actions as { allow?: string[] } | undefined
	return actions?.allow ?? []
})

const pageActions = computed(() => (envelope.value?.actions ?? []) as YmclAction[])

async function runPageAction(action: YmclAction) {
	const { executeAction } = await import('@/helpers/ymcl-actions')
	await executeAction(action, {
		allow: allow.value,
		onReload: () => {
			reloadTick.value += 1
			void loadEnvelope()
		},
	})
}

watch(
	[pageId, () => ymclStore.manifest, reloadTick, () => ymclStore.dataEpoch],
	() => {
		if (!ymclStore.isPersonal && page.value?.data_source) {
			void loadEnvelope()
		} else {
			envelope.value = null
			loadError.value = null
		}
	},
	{ immediate: true },
)
</script>

<template>
	<div class="flex min-h-full flex-col p-6">
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
					:type="action.primary ? 'brand' : 'standard'"
				>
					<button @click="runPageAction(action)">
						{{ action.title }}
					</button>
				</ButtonStyled>
				<ButtonStyled v-if="page.data_source" type="standard" circular>
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
				@reload="
					reloadTick++
					loadEnvelope()
				"
			/>
			<div v-else-if="rendererComponent && envelope">
				<component
					:is="rendererComponent"
					:envelope="{ ...envelope, allow }"
					@reload="
						reloadTick++
						loadEnvelope()
					"
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
</template>
