<script setup lang="ts">
import { DownloadIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, useVIntl } from '@modrinth/ui'
import { computed, nextTick, ref } from 'vue'

import DomainImg from '@/components/ymcl/DomainImg.vue'
import DomainRecordDetailModal from '@/components/ymcl/DomainRecordDetailModal.vue'
import { type YmclAction } from '@/helpers/ymcl-actions'
import { ymclRecordCover, ymclRecordSummary, ymclRecordTitle } from '@/helpers/ymcl-envelope'
import { ymclDisplayText } from '@/helpers/ymcl'
import { createYmclActionRunner, primaryYmclAction } from '@/helpers/ymcl-item-action'

/**
 * pack-catalog renderer (YAP §6.6): MIP pack / project entries with version
 * and change summary. Primary actions execute through the shared YAP runner;
 * cards open a detail surface when the adapter does not provide a working
 * install action (or omits instance bindings).
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
	empty: { id: 'app.ymcl.renderer.pack-empty', defaultMessage: '暂无可用整合包/项目' },
	version: { id: 'app.ymcl.renderer.pack-version', defaultMessage: '版本' },
})

const records = computed(() => props.envelope.records ?? [])
const itemActions = computed(() => props.envelope.itemActions ?? [])
const allow = computed(() => props.envelope.allow ?? [])

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

function subtitle(record: Record<string, unknown>): string {
	const parts: string[] = []
	const version = record.version ?? record.versionId ?? record.version_id
	if (version != null) {
		parts.push(`${formatMessage(messages.version)} ${String(version)}`)
	}
	if (record.channel) parts.push(String(record.channel))
	const summary = ymclRecordSummary(record)
	if (summary) parts.push(summary)
	return parts.join(' · ')
}

async function run(action: YmclAction, record: Record<string, unknown>) {
	await runYmclAction(action, {
		allow: allow.value,
		record,
		onReload: () => emit('reload'),
		onOpenDetail: () => openDetail(record),
		actionUnavailableText: formatMessage(messages.actionUnavailable),
	})
}

function onRowClick(record: Record<string, unknown>) {
	const action = primaryAction(record)
	if (action) {
		void run(action, record)
		return
	}
	openDetail(record)
}
</script>

<template>
	<div class="flex flex-col gap-2">
		<div
			v-for="record in records"
			:key="String(record.packId ?? record.pack_id ?? record.id ?? record.title)"
			class="flex cursor-pointer items-center gap-3 rounded-xl border border-solid border-surface-5 bg-bg-raised p-3 transition-colors hover:bg-button-bg"
			role="button"
			tabindex="0"
			@click="onRowClick(record)"
			@keydown.enter.prevent="onRowClick(record)"
		>
			<DomainImg
				:src="ymclRecordCover(record)"
				:alt="ymclRecordTitle(record)"
				class="h-10 w-10 shrink-0 rounded-lg object-contain"
			/>
			<div class="min-w-0 flex-1">
				<div class="truncate font-semibold text-contrast">
					{{ ymclRecordTitle(record) }}
				</div>
				<div class="truncate text-xs text-secondary">{{ subtitle(record) }}</div>
			</div>
			<div class="flex shrink-0 items-center gap-2">
				<template v-if="primaryAction(record)">
					<ButtonStyled color="brand">
						<button
							type="button"
							@click.stop="run(primaryAction(record)!, record)"
						>
							<DownloadIcon />
							{{ actionTitle(primaryAction(record)!) }}
						</button>
					</ButtonStyled>
				</template>
				<ButtonStyled type="standard">
					<button type="button" @click.stop="openDetail(record)">
						{{ formatMessage(messages.detail) }}
					</button>
				</ButtonStyled>
			</div>
		</div>
		<p v-if="records.length === 0" class="m-0 p-6 text-center text-sm text-secondary">
			{{ formatMessage(messages.empty) }}
		</p>
	</div>
	<DomainRecordDetailModal
		ref="detailModal"
		:record="activeRecord"
		:item-actions="itemActions"
		:allow="allow"
		@reload="emit('reload')"
	/>
</template>
