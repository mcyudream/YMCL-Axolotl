<script setup lang="ts">
import {
	ChevronRightIcon,
	IssuesIcon,
	PlayIcon,
	RefreshCwIcon,
	SpinnerIcon,
} from '@modrinth/assets'
import { ButtonStyled, defineMessages, useVIntl } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'

import type { HomeWidgetPlacement, HomeWidgetSize } from '@/components/home/home-dashboard'
import DomainImg from '@/components/ymcl/DomainImg.vue'
import DomainRecordDetailModal from '@/components/ymcl/DomainRecordDetailModal.vue'
import JoinServerModal from '@/components/ymcl/JoinServerModal.vue'
import {
	isYmclActionExecutable,
	type YmclAction,
} from '@/helpers/ymcl-actions'
import { domainPageRoute } from '@/helpers/ymcl-domain'
import {
	normalizeYmclEnvelope,
	ymclRecordCover,
	ymclRecordSummary,
	ymclRecordTitle,
} from '@/helpers/ymcl-envelope'
import { resolveDataSourceTitle } from '@/helpers/ymcl-home'
import { ymclDisplayText } from '@/helpers/ymcl'
import { createYmclActionRunner, primaryYmclAction } from '@/helpers/ymcl-item-action'
import { useYmclStore } from '@/store/ymcl'

/**
 * data-card home widget (YAP §6.5 adapter card contribution): fetches the
 * bound manifest dataSource and renders a compact stats/list/hero view.
 * Rows open a launcher-side record detail; "All" still jumps to the host page.
 * Uses ymcl-envelope + ymcl-item-action only — do not reintroduce local
 * isActionAllowed/renderActionParams helpers (they break after HMR).
 */

const props = defineProps<{
	placement: HomeWidgetPlacement
	dashboardSize: HomeWidgetSize
}>()

const { formatMessage } = useVIntl()
const ymclStore = useYmclStore()
const { runYmclAction } = createYmclActionRunner()

const messages = defineMessages({
	viewAll: { id: 'app.home.data-card.view-all', defaultMessage: 'All' },
	refresh: { id: 'app.ymcl.domain-page.refresh', defaultMessage: 'Refresh' },
	empty: { id: 'app.home.data-card.empty', defaultMessage: 'No data yet' },
	loadFailed: { id: 'app.home.data-card.load-failed', defaultMessage: 'Failed to load data' },
	online: {
		id: 'app.home.data-card.players-online',
		defaultMessage: '{players}/{max} online',
	},
	offline: { id: 'app.home.data-card.offline', defaultMessage: 'Offline' },
	actionUnavailable: {
		id: 'app.ymcl.renderer.action-unavailable',
		defaultMessage: '该操作暂不可用',
	},
})

const rawEnvelope = ref<unknown>(null)
const loading = ref(false)
const loadFailed = ref(false)
const actionBusy = ref(false)
const joinModal = ref<InstanceType<typeof JoinServerModal> | null>(null)
const detailModal = ref<InstanceType<typeof DomainRecordDetailModal> | null>(null)
const activeRecord = ref<Record<string, unknown> | null>(null)

const source = computed(() => props.placement.dataSource)
const variant = computed(() => source.value?.variant ?? 'list')
/**
 * Page hosting this dataSource for click-through. Falls back to the dataSource
 * id itself — DomainPageHost synthesizes a page for bare source ids.
 */
const boundPageId = computed(() => {
	const id = source.value?.id
	if (!id) return null
	const page = (ymclStore.manifest?.pages ?? []).find((candidate) => candidate.data_source === id)
	return page?.id ?? id
})

function navigationTitleForPage(pageId: string): string | null {
	const items = ymclStore.manifest?.navigation ?? []
	const stack = [...items]
	while (stack.length) {
		const item = stack.shift()
		if (!item) continue
		if (item.page_id === pageId && item.title?.trim()) return item.title.trim()
		if (item.children?.length) stack.push(...item.children)
	}
	return null
}

function declarationTitle(id: string): string | null {
	const declared = (ymclStore.manifest?.data_sources ?? []).find((item) => item.id === id)
	if (!declared) return null
	return declared.title ?? declared.name ?? declared.label ?? null
}

/** Card header uses adapter-provided titles, then well-known/humanized fallbacks. */
const title = computed(() =>
	resolveDataSourceTitle({
		id: source.value?.id ?? '',
		cardTitle: source.value?.title,
		declarationTitle: source.value?.id ? declarationTitle(source.value.id) : null,
		pageTitle: ymclStore.pageById(boundPageId.value ?? '')?.title,
		navigationTitle: boundPageId.value ? navigationTitleForPage(boundPageId.value) : null,
	}),
)

const manifestAllow = computed(() => {
	const actions = ymclStore.manifest?.actions as { allow?: string[] } | undefined
	return actions?.allow ?? []
})
const envelope = computed(() => normalizeYmclEnvelope(rawEnvelope.value, manifestAllow.value))
const itemActions = computed(() => envelope.value.itemActions)
const allow = computed(() => envelope.value.allow)

function recordTitle(record: Record<string, unknown>): string {
	return ymclRecordTitle(record)
}

function recordSubtitle(record: Record<string, unknown>): string {
	const parts: string[] = []
	if (record.address) parts.push(String(record.address))
	// 在线人数约定为数字（onlinePlayers 优先）；players 若是玩家名列表则忽略。
	const players =
		typeof record.players === 'number'
			? record.players
			: typeof record.onlinePlayers === 'number'
				? record.onlinePlayers
				: null
	if (record.online === false) {
		parts.push(formatMessage(messages.offline))
	} else if (players != null) {
		// maxPlayers 为 0 视为未知上限，避免显示成 "1/0 online"。
		const max = Number(record.maxPlayers) > 0 ? String(record.maxPlayers) : '?'
		parts.push(formatMessage(messages.online, { players: String(players), max }))
	}
	const summary = ymclRecordSummary(record)
	if (summary) parts.push(summary)
	return parts.join(' · ')
}

function recordCover(record: Record<string, unknown>): string | null {
	return ymclRecordCover(record)
}

function primaryAction(record: Record<string, unknown>): YmclAction | null {
	return primaryYmclAction(itemActions.value, record, allow.value)
}

function openDetail(record: Record<string, unknown>) {
	activeRecord.value = record
	detailModal.value?.show()
}

async function runAction(action: YmclAction, record: Record<string, unknown>) {
	if (actionBusy.value) return
	actionBusy.value = true
	try {
		await runYmclAction(action, {
			allow: allow.value,
			record,
			onReload: () => void load(),
			onOpenDetail: () => openDetail(record),
			onLaunchServerWithoutInstance: (address) => {
				void joinModal.value?.show({ address })
			},
			actionUnavailableText: formatMessage(messages.actionUnavailable),
		})
	} finally {
		actionBusy.value = false
	}
}

function onRecordClick(record: Record<string, unknown>) {
	const action = primaryAction(record)
	if (action && isYmclActionExecutable(action, record, allow.value)) {
		void runAction(action, record)
		return
	}
	openDetail(record)
}

async function load() {
	const binding = source.value
	if (!binding || ymclStore.isPersonal) return
	loading.value = true
	loadFailed.value = false
	try {
		rawEnvelope.value = await invoke<unknown>('plugin:ymcl|ymcl_data_fetch', {
			providerCode: binding.providerCode,
			sourceCode: binding.sourceCode,
			query: null,
		})
	} catch {
		rawEnvelope.value = null
		loadFailed.value = true
	} finally {
		loading.value = false
	}
}

watch(
	[() => props.placement.dataSource, () => ymclStore.dataEpoch, () => ymclStore.activeDomainId],
	() => void load(),
	{ immediate: true },
)

const records = computed(() => envelope.value.records)
const isStats = computed(() => variant.value === 'stats')
const isHero = computed(() => variant.value === 'hero')
const isCompact = computed(() => props.dashboardSize === '1x1' || props.dashboardSize === '1x2')
const visibleRecords = computed(() => {
	if (isHero.value) return records.value.slice(0, 1)
	return records.value.slice(0, isCompact.value ? 2 : 4)
})
</script>

<template>
	<div class="flex min-w-0 min-h-0 h-full flex-col gap-2" :data-size="dashboardSize">
		<div class="flex min-w-0 items-center gap-2">
			<span class="truncate text-xs font-bold leading-none text-secondary">
				{{ title }}
			</span>
			<div class="flex-1"></div>
			<SpinnerIcon v-if="loading" class="size-3.5 shrink-0 animate-spin text-secondary" />
			<button
				v-else
				v-tooltip="formatMessage(messages.refresh)"
				class="border-0 bg-transparent p-0 text-secondary hover:text-contrast"
				@click="load"
			>
				<RefreshCwIcon class="size-3.5" />
			</button>
			<router-link
				v-if="boundPageId"
				class="flex shrink-0 items-center gap-0.5 text-xs font-semibold text-secondary no-underline hover:text-brand"
				:to="domainPageRoute(boundPageId)"
			>
				{{ formatMessage(messages.viewAll) }}
				<ChevronRightIcon class="size-3" aria-hidden="true" />
			</router-link>
		</div>

		<template v-if="loadFailed">
			<div class="flex min-h-0 flex-1 flex-col items-center justify-center gap-2 text-center">
				<IssuesIcon class="size-5 text-secondary" aria-hidden="true" />
				<span class="text-xs text-secondary">{{ formatMessage(messages.loadFailed) }}</span>
			</div>
		</template>
		<p
			v-else-if="visibleRecords.length === 0"
			class="m-0 flex min-h-0 flex-1 items-center justify-center text-xs text-secondary"
		>
			{{ formatMessage(messages.empty) }}
		</p>

		<div
			v-else-if="isStats"
			class="grid min-h-0 flex-1 gap-2"
			:class="isCompact ? 'grid-cols-1' : 'grid-cols-2'"
		>
			<div
				v-for="record in visibleRecords"
				:key="String(record.label ?? record.id)"
				class="flex min-w-0 flex-col justify-center gap-0.5 rounded-lg bg-bg-raised px-3 py-2"
			>
				<span class="truncate text-lg font-bold leading-tight text-contrast">
					{{ record.value ?? '—' }}
				</span>
				<span class="truncate text-xs text-secondary">{{ record.label }}</span>
			</div>
		</div>

		<div v-else class="flex min-h-0 flex-1 flex-col gap-1.5 overflow-hidden">
			<div
				v-for="(record, index) in visibleRecords"
				:key="String(record.id ?? record.title ?? index)"
				class="flex min-w-0 cursor-pointer items-center gap-2 rounded-lg bg-bg-raised px-2.5 py-1.5 transition-colors hover:bg-button-bg"
				:class="isHero ? 'flex-1 py-2' : ''"
				role="button"
				tabindex="0"
				@click="onRecordClick(record)"
				@keydown.enter.prevent="onRecordClick(record)"
			>
				<DomainImg
					:src="recordCover(record)"
					:alt="recordTitle(record)"
					class="size-8 shrink-0 rounded-md object-cover"
				/>
				<div class="flex min-w-0 flex-1 flex-col">
					<span class="flex min-w-0 items-center gap-2">
						<span
							v-if="record.status"
							class="inline-block size-2 shrink-0 rounded-full"
							:class="record.status === 'online' ? 'bg-green' : 'bg-red'"
						></span>
						<span
							class="truncate text-sm font-semibold"
							:class="isHero ? 'text-base text-contrast' : 'text-contrast'"
						>
							{{ recordTitle(record) }}
						</span>
					</span>
					<span v-if="recordSubtitle(record)" class="truncate text-xs text-secondary">
						{{ recordSubtitle(record) }}
					</span>
				</div>
				<ButtonStyled v-if="primaryAction(record)" color="brand" size="small">
					<button
						class="shrink-0"
						type="button"
						:disabled="actionBusy"
						@click.stop="runAction(primaryAction(record)!, record)"
					>
						<PlayIcon class="size-3.5" />
						{{
							ymclDisplayText(primaryAction(record)!.title) ||
							ymclDisplayText(primaryAction(record)!.code)
						}}
					</button>
				</ButtonStyled>
			</div>
		</div>
		<JoinServerModal ref="joinModal" />
		<DomainRecordDetailModal
			ref="detailModal"
			:record="activeRecord"
			:item-actions="itemActions"
			:allow="allow"
			:page-title="title"
			@reload="() => load()"
		/>
	</div>
</template>
