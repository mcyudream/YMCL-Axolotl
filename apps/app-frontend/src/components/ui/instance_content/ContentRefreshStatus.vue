<template>
	<section
		v-if="job && job.state !== 'completed'"
		data-onboarding-id="content-refresh"
		class="mb-4 rounded-xl bg-surface-2 p-4"
		aria-live="polite"
	>
		<div class="flex flex-wrap items-center justify-between gap-3">
			<div class="min-w-0 flex-1">
				<p class="m-0 font-semibold text-contrast">{{ formatMessage(messages[job.state]) }}</p>
				<p v-if="job.total" class="m-0 mt-1 text-sm text-secondary">
					{{
						formatMessage(messages.progress, {
							processed: job.processed,
							total: job.total,
							failed: job.failed,
						})
					}}
				</p>
				<p
					v-if="job.error && job.state !== 'cancelled'"
					class="m-0 mt-1 break-words text-sm text-secondary"
				>
					{{ job.error }}
				</p>
			</div>
			<ButtonStyled v-if="active" size="small"
				><button type="button" :disabled="busy" @click="cancel">
					{{ formatMessage(messages.cancel) }}
				</button></ButtonStyled
			>
			<ButtonStyled v-else size="small"
				><button type="button" :disabled="busy" @click="retry">
					{{ formatMessage(messages.retry) }}
				</button></ButtonStyled
			>
		</div>
		<progress
			v-if="active && job.total"
			:value="job.processed"
			:max="job.total"
			class="mt-3 h-1 w-full accent-brand"
		/>
		<details v-if="job.failures.length" class="mt-3 text-sm">
			<summary class="cursor-pointer">{{ formatMessage(messages.failures) }}</summary>
			<ul class="mb-0 flex max-h-48 flex-col gap-2 overflow-auto pl-5">
				<li v-for="failure in job.failures" :key="failure.path">
					<span class="break-all font-semibold">{{ failure.path }}</span
					><span class="block break-words text-secondary">{{ failure.error }}</span>
				</li>
			</ul>
		</details>
	</section>
</template>

<script setup lang="ts">
import { ButtonStyled, defineMessages, useVIntl } from '@modrinth/ui'
import { computed, onBeforeUnmount, ref, watch } from 'vue'

import { handleError } from '@/store/notifications'
import {
	cancelContentRefresh,
	getContentRefresh,
	isContentRefreshActive,
	startContentRefresh,
	type ContentRefreshJob,
} from '@/helpers/content-refresh'

const props = defineProps<{ instanceId: string }>()
const { formatMessage } = useVIntl()
const job = ref<ContentRefreshJob | null>(null)
const busy = ref(false)
const active = computed(() => isContentRefreshActive(job.value))
let generation = 0
let timer: ReturnType<typeof setTimeout> | undefined
const messages = defineMessages({
	queued: { id: 'instance.contentRefresh.queued', defaultMessage: 'Content scan queued' },
	scanning: {
		id: 'instance.contentRefresh.scanning',
		defaultMessage: 'Scanning content in the background',
	},
	enriching: {
		id: 'instance.contentRefresh.enriching',
		defaultMessage: 'Updating content information in the background',
	},
	completed: { id: 'instance.contentRefresh.completed', defaultMessage: 'Content scan complete' },
	partial: {
		id: 'instance.contentRefresh.partial',
		defaultMessage: 'Some content could not be refreshed',
	},
	failed: { id: 'instance.contentRefresh.failed', defaultMessage: 'Content scan failed' },
	cancelled: { id: 'instance.contentRefresh.cancelled', defaultMessage: 'Content scan cancelled' },
	interrupted: {
		id: 'instance.contentRefresh.interrupted',
		defaultMessage: 'Content scan interrupted',
	},
	progress: {
		id: 'instance.contentRefresh.progress',
		defaultMessage: '{processed} / {total} files scanned · {failed} failed',
	},
	cancel: { id: 'instance.contentRefresh.cancel', defaultMessage: 'Cancel scan' },
	retry: { id: 'instance.contentRefresh.retry', defaultMessage: 'Retry unfinished files' },
	failures: { id: 'instance.contentRefresh.failures', defaultMessage: 'Show failed files' },
})

async function poll(id: string, current: number) {
	try {
		const result = await getContentRefresh(id)
		if (current !== generation) return
		job.value = result
	} catch (error) {
		if (current === generation) handleError(error as Error)
		return
	}
	if (current === generation)
		timer = setTimeout(() => void poll(id, current), active.value ? 1500 : 10000)
}

async function cancel() {
	if (busy.value) return
	busy.value = true
	const id = props.instanceId
	try {
		await cancelContentRefresh(id)
	} catch (error) {
		handleError(error as Error)
	} finally {
		busy.value = false
	}
}

async function retry() {
	if (busy.value) return
	busy.value = true
	const id = props.instanceId
	try {
		const result = await startContentRefresh(id, (job.value?.total ?? 0) > 0)
		if (id === props.instanceId) job.value = result
	} catch (error) {
		handleError(error as Error)
	} finally {
		busy.value = false
	}
}

watch(
	() => props.instanceId,
	(id) => {
		clearTimeout(timer)
		job.value = null
		void poll(id, ++generation)
	},
	{ immediate: true },
)
onBeforeUnmount(() => {
	generation++
	clearTimeout(timer)
})
</script>
