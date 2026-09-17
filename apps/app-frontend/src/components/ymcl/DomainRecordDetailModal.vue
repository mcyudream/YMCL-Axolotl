<script setup lang="ts">
import { ButtonStyled, defineMessages, injectNotificationManager, NewModal, useVIntl } from '@modrinth/ui'
import { computed, ref } from 'vue'

import DomainImg from '@/components/ymcl/DomainImg.vue'
import {
	executeAction,
	type YmclAction,
	type YmclServerActionResponse,
} from '@/helpers/ymcl-actions'
import {
	ymclRecordBody,
	ymclRecordCover,
	ymclRecordSummary,
	ymclRecordText,
	ymclRecordTitle,
} from '@/helpers/ymcl-envelope'
import { ymclDisplayText, ymclErrorMessage } from '@/helpers/ymcl'

/**
 * Launcher-side record detail for YAP card-grid / pack-catalog pages.
 * Adapters that do not declare a detail page or detail action still get a
 * usable surface: cover, body text, key metadata, and every executable
 * item action.
 */

const props = defineProps<{
	record: Record<string, unknown> | null
	itemActions?: YmclAction[]
	allow?: string[]
	pageTitle?: string | null
}>()

const emit = defineEmits<{
	reload: []
	actionExecuted: [action: YmclAction]
}>()

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const { addNotification } = injectNotificationManager()
const { formatMessage } = useVIntl()
const busy = ref(false)

const messages = defineMessages({
	header: { id: 'app.ymcl.record-detail.header', defaultMessage: '活动详情' },
	untitled: { id: 'app.ymcl.record-detail.untitled', defaultMessage: '未命名' },
	meta: { id: 'app.ymcl.record-detail.meta', defaultMessage: '信息' },
	actions: { id: 'app.ymcl.record-detail.actions', defaultMessage: '操作' },
	actionUnavailable: {
		id: 'app.ymcl.record-detail.action-unavailable',
		defaultMessage: '该操作暂不可用',
	},
	emptyBody: { id: 'app.ymcl.record-detail.empty-body', defaultMessage: '暂无详细介绍' },
})

const META_LABELS: Record<string, { id: string; defaultMessage: string }> = {
	version: { id: 'app.ymcl.record-detail.meta.version', defaultMessage: '版本' },
	channel: { id: 'app.ymcl.record-detail.meta.channel', defaultMessage: '渠道' },
	status: { id: 'app.ymcl.record-detail.meta.status', defaultMessage: '状态' },
	date: { id: 'app.ymcl.record-detail.meta.date', defaultMessage: '日期' },
	start: { id: 'app.ymcl.record-detail.meta.start', defaultMessage: '开始' },
	end: { id: 'app.ymcl.record-detail.meta.end', defaultMessage: '结束' },
	startAt: { id: 'app.ymcl.record-detail.meta.start-at', defaultMessage: '开始时间' },
	endAt: { id: 'app.ymcl.record-detail.meta.end-at', defaultMessage: '结束时间' },
	start_at: { id: 'app.ymcl.record-detail.meta.start_at', defaultMessage: '开始时间' },
	end_at: { id: 'app.ymcl.record-detail.meta.end_at', defaultMessage: '结束时间' },
	address: { id: 'app.ymcl.record-detail.meta.address', defaultMessage: '地址' },
	packId: { id: 'app.ymcl.record-detail.meta.pack-id', defaultMessage: '整合包' },
	pack_id: { id: 'app.ymcl.record-detail.meta.pack_id', defaultMessage: '整合包' },
	author: { id: 'app.ymcl.record-detail.meta.author', defaultMessage: '作者' },
	category: { id: 'app.ymcl.record-detail.meta.category', defaultMessage: '分类' },
	reward: { id: 'app.ymcl.record-detail.meta.reward', defaultMessage: '奖励' },
	quota: { id: 'app.ymcl.record-detail.meta.quota', defaultMessage: '名额' },
}

const META_FIELD_KEYS = Object.keys(META_LABELS)

const title = computed(() => {
	const fromRecord = ymclRecordTitle(props.record ?? {})
	if (fromRecord) return fromRecord
	if (props.pageTitle?.trim()) return props.pageTitle.trim()
	return formatMessage(messages.untitled)
})

const cover = computed(() => ymclRecordCover(props.record ?? {}))
const summary = computed(() => ymclRecordSummary(props.record ?? {}))
const body = computed(() => ymclRecordBody(props.record ?? {}))
const bodyLines = computed(() =>
	body.value
		.split('\n')
		.map((line) => line.trim())
		.filter((line) => line.length > 0),
)

const metaRows = computed(() => {
	const record = props.record
	if (!record) return []
	return META_FIELD_KEYS.map((key) => {
		const label = META_LABELS[key]
		const value = ymclRecordText(record[key])
		if (!value.trim()) return null
		return {
			key,
			label: formatMessage(label),
			value: value.trim(),
		}
	}).filter((row): row is { key: string; label: string; value: string } => row !== null)
})

const detailActions = computed(() => {
	if (!props.record) return []
	return props.itemActions ?? []
})

function actionTitle(action: YmclAction): string {
	return ymclDisplayText(action.title) || ymclDisplayText(action.code) || '操作'
}

function show(event?: unknown) {
	void modal.value?.show(event as MouseEvent | undefined)
}

function hide() {
	modal.value?.hide()
}

defineExpose({ show, hide })

async function run(action: YmclAction) {
	const record = props.record
	if (!record || busy.value) return
	busy.value = true
	const title = actionTitle(action)
	try {
		if (action.kind === 'client:reload') {
			emit('reload')
			return
		}
		const response = (await executeAction(action, {
			allow: props.allow ?? [],
			record,
			onReload: () => emit('reload'),
		})) as boolean | YmclServerActionResponse
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
		emit('actionExecuted', action)
		if (typeof response !== 'boolean' && response.refresh !== false) emit('reload')
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
	} finally {
		busy.value = false
	}
}
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.header)"
		max-width="min(36rem, calc(95vw - 10rem))"
		scrollable
		max-content-height="70vh"
	>
		<div class="flex flex-col gap-4">
			<div v-if="cover" class="overflow-hidden rounded-xl">
				<DomainImg :src="cover" :alt="title" class="h-40 w-full object-cover" />
			</div>
			<div class="flex flex-col gap-2">
				<h2 class="m-0 text-xl font-bold text-contrast">{{ title }}</h2>
				<p v-if="summary" class="m-0 text-sm leading-relaxed text-secondary">{{ summary }}</p>
			</div>
			<div v-if="metaRows.length > 0" class="flex flex-col gap-2">
				<div class="text-xs font-bold uppercase tracking-wide text-secondary">
					{{ formatMessage(messages.meta) }}
				</div>
				<div class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1.5 text-sm">
					<template v-for="row in metaRows" :key="row.key">
						<span class="text-secondary">{{ row.label }}</span>
						<span class="min-w-0 break-words text-contrast">{{ row.value }}</span>
					</template>
				</div>
			</div>
			<div class="flex flex-col gap-2">
				<div class="text-xs font-bold uppercase tracking-wide text-secondary">
					{{ formatMessage(messages.header) }}
				</div>
				<p
					v-if="bodyLines.length === 0"
					class="m-0 rounded-xl bg-bg-raised p-4 text-sm text-secondary"
				>
					{{ formatMessage(messages.emptyBody) }}
				</p>
				<div v-else class="flex flex-col gap-2">
					<p
						v-for="(line, index) in bodyLines"
						:key="index"
						class="m-0 text-sm leading-relaxed text-secondary"
					>
						{{ line }}
					</p>
				</div>
			</div>
		</div>
		<template v-if="detailActions.length > 0" #actions>
			<div class="flex flex-wrap gap-2">
				<ButtonStyled
					v-for="action in detailActions"
					:key="action.code"
					:color="action.primary ? 'brand' : 'standard'"
				>
					<button :disabled="busy" @click="run(action)">
						{{ actionTitle(action) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>
