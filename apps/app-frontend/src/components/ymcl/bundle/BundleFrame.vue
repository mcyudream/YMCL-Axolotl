<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import { computed, onBeforeUnmount, ref, watch } from 'vue'

import { type BridgePermission,createBridgeListener } from '@/helpers/ymcl-bridge'

/**
 * BundleFrame (YAP §6.8): hosts an extension page bundle inside a sandboxed
 * iframe (opaque origin, no launcher APIs). All interaction goes through
 * the typed bridge in helpers/ymcl-bridge; permissions come from the
 * bundle manifest as declared by the domain's page descriptor.
 */

const props = defineProps<{
	/** The manifest page descriptor with renderer "extension". */
	page: {
		id: string
		bundle?: {
			id: string
			version: string
			entry: string
			url: string
			sha256: string
			permissions?: string[]
		} | null
	}
}>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	loading: { id: 'app.ymcl.bundle.loading', defaultMessage: 'Loading extension page…' },
	failed: {
		id: 'app.ymcl.bundle.failed',
		defaultMessage: 'This extension page could not be loaded.',
	},
})

const emit = defineEmits<{ reload: [] }>()

const entryUrl = ref<string | null>(null)
const failed = ref(false)
const loading = ref(true)

const permissions = computed<BridgePermission[]>(() =>
	(props.page.bundle?.permissions ?? []).filter((permission): permission is BridgePermission =>
		['data.fetch', 'action.execute', 'open-url', 'theme.read', 'nav.context'].includes(permission),
	),
)

let cleanup: (() => void) | null = null

async function loadBundle() {
	loading.value = true
	failed.value = false
	entryUrl.value = null
	cleanup?.()
	if (!props.page.bundle) {
		failed.value = true
		loading.value = false
		return
	}
	try {
		const bundle = await import('@tauri-apps/api/core').then((core) =>
			core.invoke('plugin:ymcl|ymcl_bundle_get', { pageId: props.page.id }),
		)
		entryUrl.value = convertFileSrc(bundle.entry_path)
		cleanup = createBridgeListener({
			bundleId: bundle.bundle_id,
			permissions: permissions.value,
			actionAllow: [],
			onReload: () => emit('reload'),
		})
	} catch {
		failed.value = true
	} finally {
		loading.value = false
	}
}

watch(
	() => props.page.id,
	() => void loadBundle(),
	{ immediate: true },
)

onBeforeUnmount(() => cleanup?.())
</script>

<template>
	<div class="flex min-h-96 flex-1 flex-col">
		<iframe
			v-if="entryUrl"
			:src="entryUrl"
			sandbox="allow-scripts"
			class="min-h-96 w-full flex-1 rounded-xl border-0 bg-bg-raised"
			@load="loading = false"
		></iframe>
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
	</div>
</template>
