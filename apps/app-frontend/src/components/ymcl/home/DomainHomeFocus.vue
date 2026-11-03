<script setup lang="ts">
import {
	ArrowUpRightIcon,
	ChevronLeftIcon,
	ChevronRightIcon,
	RefreshCwIcon,
	SpinnerIcon,
} from '@modrinth/assets'
import { defineMessages, useFormatDateTime, useVIntl } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { computed, onUnmounted, ref, watch } from 'vue'

import DomainRecordDetailModal from '@/components/ymcl/DomainRecordDetailModal.vue'
import {
	type NormalizedYmclEnvelope,
	normalizeYmclEnvelope,
	ymclRecordSummary,
	ymclRecordTitle,
} from '@/helpers/ymcl-envelope'
import { resolveDataCardSource, resolveDataSourceTitle } from '@/helpers/ymcl-home'
import { useYmclStore } from '@/store/ymcl'

const props = defineProps<{ sources: string[]; preview?: boolean }>()
const { formatMessage } = useVIntl()
const formatDate = useFormatDateTime({ dateStyle: 'medium' })
const store = useYmclStore()
const PAGE_SIZE = 4
const messages = defineMessages({
	title: { id: 'app.ymcl.landing.focus', defaultMessage: 'Domain highlights' },
	refresh: { id: 'app.ymcl.landing.focus-refresh', defaultMessage: 'Refresh highlights' },
	all: { id: 'app.ymcl.landing.focus-all', defaultMessage: 'All' },
	empty: { id: 'app.ymcl.landing.focus-empty', defaultMessage: 'No highlights yet.' },
	error: { id: 'app.ymcl.landing.focus-error', defaultMessage: 'Could not load highlights.' },
	retry: { id: 'app.ymcl.landing.retry', defaultMessage: 'Retry' },
	untitled: { id: 'app.ymcl.landing.untitled', defaultMessage: 'Untitled' },
	pinned: { id: 'app.ymcl.landing.focus-pinned', defaultMessage: 'Pinned' },
	previous: { id: 'app.ymcl.landing.focus-previous', defaultMessage: 'Previous highlights' },
	next: { id: 'app.ymcl.landing.focus-next', defaultMessage: 'Next highlights' },
	page: { id: 'app.ymcl.landing.focus-page', defaultMessage: 'Page {page}' },
})
type Source = { id: string; providerCode: string; sourceCode: string; title: string }
type SourceState = {
	envelope: NormalizedYmclEnvelope
	page: number
	loading: boolean
	failed: boolean
	request: number
}
const selected = ref('')
const states = ref<Record<string, SourceState>>({})
const active = ref<{
	record: Record<string, unknown>
	envelope: NormalizedYmclEnvelope
	title: string
} | null>(null)
const detail = ref<InstanceType<typeof DomainRecordDetailModal>>()
let generation = 0
const sources = computed<Source[]>(() =>
	[...new Set(props.sources)].flatMap((id) => {
		const declaration = store.manifest?.data_sources.find((source) => source.id === id)
		if (!declaration) return []
		const binding = resolveDataCardSource(id, [declaration])
		if (!binding) return []
		const page = store.manifest?.pages.find((page) => page.data_source === id)
		return [
			{
				...binding,
				id,
				title: resolveDataSourceTitle({
					id,
					declarationTitle: declaration.title ?? declaration.name ?? declaration.label,
					pageTitle: page?.title,
				}),
			},
		]
	}),
)
const visibleSources = computed(() =>
	sources.value.filter((item) => {
		if (selected.value) return item.id === selected.value
		if (sources.value.length === 1) return true
		const state = states.value[item.id]
		return state && (state.loading || state.failed || state.envelope.records.length > 0)
	}),
)
const loading = computed(() => Object.values(states.value).some((state) => state.loading))

async function loadPage(binding: Source, page = 1) {
	const state = states.value[binding.id]
	if (!state) return
	const context = generation
	const request = ++state.request
	state.loading = true
	state.failed = false
	try {
		const raw = await invoke<unknown>('plugin:ymcl|ymcl_data_fetch', {
			providerCode: binding.providerCode,
			sourceCode: binding.sourceCode,
			query: new URLSearchParams({ page: String(page), pageSize: String(PAGE_SIZE) }).toString(),
		})
		if (context !== generation || request !== state.request) return
		state.envelope = normalizeYmclEnvelope(
			raw,
			(store.manifest?.actions as { allow?: string[] })?.allow ?? [],
		)
		state.page = page
	} catch {
		if (context === generation && request === state.request) state.failed = true
	} finally {
		if (context === generation && request === state.request) state.loading = false
	}
}

function reload() {
	generation++
	detail.value?.hide()
	active.value = null
	states.value = {}
	if (!sources.value.some((item) => item.id === selected.value)) selected.value = ''
	if (store.isPersonal || store.loginRequired || store.loadingManifest) return
	for (const binding of sources.value) {
		states.value[binding.id] = {
			envelope: normalizeYmclEnvelope(null),
			page: 1,
			loading: false,
			failed: false,
			request: 0,
		}
	}
	void Promise.allSettled(sources.value.map((binding) => loadPage(binding)))
}

watch(
	[
		() => JSON.stringify(sources.value),
		() => store.activeDomainId,
		() => store.session,
		() => store.dataEpoch,
		() => store.loginRequired,
		() => store.loadingManifest,
	],
	reload,
	{ immediate: true },
)
onUnmounted(() => {
	generation++
})

function hasNext(state: SourceState) {
	return state.envelope.total == null
		? state.envelope.records.length >= PAGE_SIZE
		: state.page * PAGE_SIZE < state.envelope.total
}
function recordDate(record: Record<string, unknown>) {
	const value = record.publishedAt ?? record.published_at ?? record.date
	if (typeof value !== 'string' && typeof value !== 'number') return ''
	const timestamp =
		typeof value === 'number' || /^\d+$/.test(value) ? Number(value) : Date.parse(value)
	return Number.isFinite(timestamp) && timestamp > 0 && !Number.isNaN(new Date(timestamp).getTime())
		? formatDate(timestamp)
		: ''
}
function open(record: Record<string, unknown>, binding: Source) {
	if (props.preview) return
	active.value = { record, envelope: states.value[binding.id].envelope, title: binding.title }
	detail.value?.show()
}
</script>

<template>
	<aside
		class="flex min-h-0 min-w-0 max-h-full flex-col overflow-hidden rounded-[var(--radius-xl)] border border-white/15 bg-black/40 p-5 text-white backdrop-blur-xl"
		:aria-label="formatMessage(messages.title)"
	>
		<div class="flex shrink-0 items-center justify-between gap-4">
			<h2 class="m-0 text-sm font-semibold">{{ formatMessage(messages.title) }}</h2>
			<SpinnerIcon v-if="loading" class="h-4 w-4 animate-spin" aria-hidden="true" />
			<button
				v-else
				type="button"
				class="border-0 bg-transparent p-0 text-white/60 hover:text-white"
				:aria-label="formatMessage(messages.refresh)"
				@click="reload"
			>
				<RefreshCwIcon class="h-4 w-4" />
			</button>
		</div>
		<div v-if="sources.length > 1" class="mt-4 flex shrink-0 flex-wrap gap-2">
			<button type="button" class="focus-tab" :aria-pressed="!selected" @click="selected = ''">
				{{ formatMessage(messages.all) }}
			</button>
			<button
				v-for="item in sources"
				:key="item.id"
				type="button"
				class="focus-tab"
				:aria-pressed="item.id === selected"
				@click="selected = item.id"
			>
				{{ item.title }}
			</button>
		</div>
		<div class="mt-3 min-h-0 max-h-96 flex-1 overflow-y-auto overscroll-contain">
			<p v-if="!visibleSources.length" class="m-0 py-4 text-sm text-white/70">
				{{ formatMessage(messages.empty) }}
			</p>
			<section
				v-for="binding in visibleSources"
				:key="binding.id"
				class="border-b border-white/10 py-3 first:pt-0 last:border-0 last:pb-0"
			>
				<template v-if="states[binding.id]">
					<div
						v-if="!selected && sources.length > 1"
						class="flex items-center justify-between gap-3"
					>
						<button
							type="button"
							class="border-0 bg-transparent p-0 text-xs text-white/60 hover:text-white"
							@click="selected = binding.id"
						>
							{{ binding.title }}
						</button>
						<button
							v-if="states[binding.id].envelope.records.length > 2"
							type="button"
							class="border-0 bg-transparent p-0 text-white/60 hover:text-white"
							:aria-label="binding.title"
							@click="selected = binding.id"
						>
							<ChevronRightIcon class="h-4 w-4" />
						</button>
					</div>
					<p
						v-if="states[binding.id].failed"
						class="m-0 flex flex-wrap items-center gap-3 py-3 text-xs text-white/70"
						role="status"
					>
						{{ formatMessage(messages.error) }}
						<button
							type="button"
							class="inline-flex items-center gap-1 border-0 bg-transparent p-0 text-white"
							@click="loadPage(binding, states[binding.id].page)"
						>
							<RefreshCwIcon class="h-3 w-3" />{{ formatMessage(messages.retry) }}
						</button>
					</p>
					<div
						v-if="states[binding.id].loading && !states[binding.id].envelope.records.length"
						class="flex flex-col gap-3 py-4"
						aria-hidden="true"
					>
						<span class="h-3 rounded bg-white/10" /><span class="h-3 w-3/4 rounded bg-white/10" />
					</div>
					<p
						v-else-if="!states[binding.id].failed && !states[binding.id].envelope.records.length"
						class="m-0 py-3 text-xs text-white/60"
					>
						{{ formatMessage(messages.empty) }}
					</p>
					<div class="divide-y divide-white/10" :aria-busy="states[binding.id].loading">
						<button
							v-for="(record, index) in states[binding.id].envelope.records.slice(
								0,
								selected || sources.length === 1 ? PAGE_SIZE : 2,
							)"
							:key="`${states[binding.id].page}:${index}`"
							type="button"
							class="group flex w-full min-w-0 flex-col gap-2 border-0 bg-transparent px-0 py-3 text-left text-white last:pb-0 disabled:cursor-default"
							:disabled="preview || states[binding.id].loading"
							@click="open(record, binding)"
						>
							<span
								v-if="recordDate(record) || record.pinned === true"
								class="flex items-center gap-2 text-[0.6875rem] text-white/55"
								><span v-if="record.pinned === true">{{ formatMessage(messages.pinned) }}</span
								><span v-if="recordDate(record)">{{ recordDate(record) }}</span></span
							>
							<span class="flex justify-between gap-3 text-sm font-semibold"
								><span
									class="min-w-0 break-words group-hover:underline group-hover:underline-offset-4"
									>{{ ymclRecordTitle(record) || formatMessage(messages.untitled) }}</span
								><ArrowUpRightIcon v-if="!preview" class="h-4 w-4 shrink-0 text-white/50"
							/></span>
							<span
								v-if="ymclRecordSummary(record)"
								class="line-clamp-2 break-words text-xs leading-relaxed text-white/65"
								>{{ ymclRecordSummary(record) }}</span
							>
						</button>
					</div>
					<div
						v-if="
							(selected || sources.length === 1) &&
							(states[binding.id].page > 1 || hasNext(states[binding.id]))
						"
						class="mt-4 flex items-center justify-between gap-3 text-xs text-white/65"
					>
						<button
							type="button"
							class="focus-tab"
							:disabled="states[binding.id].loading || states[binding.id].page <= 1"
							:aria-label="formatMessage(messages.previous)"
							@click="loadPage(binding, states[binding.id].page - 1)"
						>
							<ChevronLeftIcon class="h-4 w-4" />
						</button>
						<span>{{ formatMessage(messages.page, { page: states[binding.id].page }) }}</span>
						<button
							type="button"
							class="focus-tab"
							:disabled="states[binding.id].loading || !hasNext(states[binding.id])"
							:aria-label="formatMessage(messages.next)"
							@click="loadPage(binding, states[binding.id].page + 1)"
						>
							<ChevronRightIcon class="h-4 w-4" />
						</button>
					</div>
				</template>
			</section>
		</div>
		<DomainRecordDetailModal
			ref="detail"
			:record="active?.record ?? null"
			:item-actions="active?.envelope.itemActions"
			:allow="active?.envelope.allow"
			:page-title="active?.title"
			@reload="reload"
		/>
	</aside>
</template>

<style scoped>
.focus-tab {
	@apply rounded-full border border-white/15 bg-transparent px-3 py-1 text-xs text-white/65 transition-colors hover:bg-white/10 disabled:cursor-default disabled:opacity-40;
}
.focus-tab[aria-pressed='true'] {
	@apply bg-white/15 text-white;
}
</style>
