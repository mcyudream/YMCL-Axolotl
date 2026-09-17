<script setup lang="ts">
import { PlayIcon } from '@modrinth/assets'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed, nextTick, ref } from 'vue'

import DomainImg from '@/components/ymcl/DomainImg.vue'
import DomainRecordDetailModal from '@/components/ymcl/DomainRecordDetailModal.vue'
import JoinServerModal from '@/components/ymcl/JoinServerModal.vue'
import { type YmclAction } from '@/helpers/ymcl-actions'
import { ymclRecordCover, ymclRecordSummary, ymclRecordTitle } from '@/helpers/ymcl-envelope'
import { ymclDisplayText } from '@/helpers/ymcl'
import {
	createYmclActionRunner,
	primaryYmclAction,
} from '@/helpers/ymcl-item-action'

/**
 * card-grid renderer (YAP §6.6): a generic wall of cover cards with a
 * primary action per card (activities, entries, recommendations). Cards
 * always open a launcher-side detail surface when the adapter does not
 * declare an executable primary action; action buttons stay independent so
 * a missing detail page never dead-ends the whole card.
 */
const props = defineProps<{
	envelope: {
		records?: Record<string, unknown>[]
		itemActions?: YmclAction[]
		actions?: YmclAction[]
		allow?: string[]
	}
}>()

const emit = defineEmits<{ reload: [] }>()

const { formatMessage } = useVIntl()
const { runYmclAction } = createYmclActionRunner()

const messages = defineMessages({
	detail: { id: 'app.ymcl.renderer.detail', defaultMessage: '详情' },
	actionUnavailable: {
		id: 'app.ymcl.renderer.action-unavailable',
		defaultMessage: '该操作暂不可用',
	},
	empty: { id: 'app.ymcl.renderer.card-empty', defaultMessage: '暂无数据' },
})

const records = computed(() => props.envelope.records ?? [])
const itemActions = computed(() => props.envelope.itemActions ?? [])
const allow = computed(() => props.envelope.allow ?? [])

const joinModal = ref<InstanceType<typeof JoinServerModal> | null>(null)
const detailModal = ref<InstanceType<typeof DomainRecordDetailModal> | null>(null)
const activeRecord = ref<Record<string, unknown> | null>(null)

function openDetail(record: Record<string, unknown>) {
	activeRecord.value = record
	void nextTick(() => {
		detailModal.value?.show()
	})
}

function primaryAction(record: Record<string, unknown>): YmclAction | null {
	return primaryYmclAction(itemActions.value, record, allow.value)
}

function actionTitle(action: YmclAction): string {
	return ymclDisplayText(action.title) || ymclDisplayText(action.code) || '操作'
}

function showPrimaryCta(action: YmclAction | null): boolean {
	return !!action && action.kind !== 'client:open-detail'
}

async function run(action: YmclAction, record: Record<string, unknown>) {
	await runYmclAction(action, {
		allow: allow.value,
		record,
		onReload: () => emit('reload'),
		onOpenDetail: () => openDetail(record),
		onLaunchServerWithoutInstance: (address) => {
			void joinModal.value?.show({ address })
		},
		actionUnavailableText: formatMessage(messages.actionUnavailable),
	})
}

function onCardClick(record: Record<string, unknown>) {
	const action = primaryAction(record)
	if (action && showPrimaryCta(action)) {
		void run(action, record)
		return
	}
	openDetail(record)
}

function recordTitle(record: Record<string, unknown>) {
	return ymclRecordTitle(record)
}

function recordSummary(record: Record<string, unknown>) {
	return ymclRecordSummary(record)
}

function recordCover(record: Record<string, unknown>) {
	return ymclRecordCover(record)
}
</script>

<template>
	<div
		v-if="records.length > 0"
		class="grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-3"
	>
		<div
			v-for="record in records"
			:key="String(record.id ?? record.title ?? record.name)"
			class="group relative cursor-pointer overflow-hidden rounded-2xl border border-solid border-surface-5 bg-bg-raised"
			role="button"
			tabindex="0"
			@click="onCardClick(record)"
			@keydown.enter.prevent="onCardClick(record)"
			@keydown.space.prevent="onCardClick(record)"
		>
			<div class="absolute inset-0 transition-transform duration-200 group-hover:scale-[1.02]">
				<DomainImg
					:src="recordCover(record)"
					:alt="recordTitle(record)"
					class="h-full w-full object-cover"
				>
					<template #fallback>
						<div class="h-full w-full bg-bg-raised"></div>
					</template>
				</DomainImg>
			</div>
			<div
				class="relative flex min-h-36 flex-col justify-end gap-1 bg-gradient-to-t from-[rgba(0,0,0,0.85)] to-transparent p-4"
			>
				<div class="font-semibold text-white">
					{{ recordTitle(record) }}
				</div>
				<div v-if="recordSummary(record)" class="line-clamp-2 text-xs text-white/70">
					{{ recordSummary(record) }}
				</div>
				<div class="mt-2 flex flex-wrap items-center gap-2">
					<button
						v-if="primaryAction(record) && showPrimaryCta(primaryAction(record))"
						type="button"
						class="inline-flex items-center gap-1 rounded-lg border-0 bg-white/95 px-2.5 py-1 text-xs font-semibold text-black"
						@click.stop="run(primaryAction(record)!, record)"
					>
						<PlayIcon class="h-3 w-3" />
						{{ actionTitle(primaryAction(record)!) }}
					</button>
					<button
						type="button"
						class="inline-flex items-center gap-1 rounded-lg border border-white/30 bg-black/30 px-2.5 py-1 text-xs font-medium text-white"
						@click.stop="openDetail(record)"
					>
						{{ formatMessage(messages.detail) }}
					</button>
				</div>
			</div>
		</div>
	</div>
	<div v-else class="rounded-xl bg-bg-raised p-6 text-center text-sm text-secondary">
		{{ formatMessage(messages.empty) }}
	</div>
	<JoinServerModal ref="joinModal" />
	<DomainRecordDetailModal
		ref="detailModal"
		:record="activeRecord"
		:item-actions="itemActions"
		:allow="allow"
		@reload="emit('reload')"
	/>
</template>
