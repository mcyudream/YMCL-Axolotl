<script setup lang="ts">
import { CheckIcon, ClipboardCopyIcon, PlayIcon, RefreshCwIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, useVIntl } from '@modrinth/ui'
import { computed, nextTick, ref } from 'vue'

import DomainImg from '@/components/ymcl/DomainImg.vue'
import DomainRecordDetailModal from '@/components/ymcl/DomainRecordDetailModal.vue'
import JoinServerModal from '@/components/ymcl/JoinServerModal.vue'
import { renderActionParams, type YmclAction } from '@/helpers/ymcl-actions'
import { ymclRecordCover, ymclRecordTitle } from '@/helpers/ymcl-envelope'
import { ymclDisplayText } from '@/helpers/ymcl'
import {
	createYmclActionRunner,
	primaryYmclAction,
	secondaryYmclActions,
} from '@/helpers/ymcl-item-action'

/**
 * server-list renderer (YAP §6.6): server records with status, current
 * season and declarative join/copy actions. Rows open a detail surface so
 * records without an executable action remain inspectable.
 */
const props = defineProps<{
	envelope: {
		records?: Record<string, unknown>[]
		actions?: YmclAction[]
		itemActions?: YmclAction[]
		allow?: string[]
	}
}>()

const emit = defineEmits<{ reload: [] }>()

const { formatMessage } = useVIntl()
const { runYmclAction } = createYmclActionRunner()

const messages = defineMessages({
	join: { id: 'app.ymcl.renderer.join', defaultMessage: 'Join' },
	copyAddress: { id: 'app.ymcl.renderer.copy-address', defaultMessage: 'Copy address' },
	playersOnline: {
		id: 'app.ymcl.renderer.players-online',
		defaultMessage: '{players}/{max} online',
	},
	offline: { id: 'app.ymcl.renderer.offline', defaultMessage: 'Offline' },
	season: { id: 'app.ymcl.renderer.season', defaultMessage: 'Season' },
	empty: { id: 'app.ymcl.renderer.empty', defaultMessage: 'Nothing here yet.' },
	detail: { id: 'app.ymcl.renderer.detail', defaultMessage: '详情' },
	actionUnavailable: {
		id: 'app.ymcl.renderer.action-unavailable',
		defaultMessage: '该操作暂不可用',
	},
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

function subtitle(record: Record<string, unknown>): string {
	const parts: string[] = []
	const season = record.currentSeason as Record<string, unknown> | undefined
	if (season?.name) parts.push(`${formatMessage(messages.season)}: ${String(season.name)}`)
	if (record.address) parts.push(String(record.address))
	if (record.players != null) {
		parts.push(
			formatMessage(messages.playersOnline, {
				players: String(record.players),
				max: String(record.maxPlayers ?? '?'),
			}),
		)
	}
	return parts.join(' · ')
}

function secondaryActions(record: Record<string, unknown>): YmclAction[] {
	return secondaryYmclActions(itemActions.value, record, allow.value, primaryAction(record)).filter(
		(action) => action.kind !== 'client:open-detail',
	)
}

function actionIconKind(action: YmclAction): string {
	if (action.kind === 'client:copy') return 'copy'
	if (action.kind === 'client:reload') return 'reload'
	if (action.kind === 'client:launch-server' || action.kind === 'client:launch-instance')
		return 'play'
	return 'check'
}

function showJoinFallback(action: YmclAction, record: Record<string, unknown>): boolean {
	if (action.kind !== 'client:launch-server') return false
	const params = renderActionParams(action, record)
	return !params.instanceId && !!params.address
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

function onRowClick(record: Record<string, unknown>) {
	const action = primaryAction(record)
	if (action && !showJoinFallback(action, record)) {
		void run(action, record)
		return
	}
	if (action && showJoinFallback(action, record)) {
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
			:key="String(record.id ?? record.title)"
			class="flex cursor-pointer items-center gap-3 rounded-xl border border-solid border-surface-5 bg-bg-raised p-3 transition-colors hover:bg-button-bg"
			role="button"
			tabindex="0"
			@click="onRowClick(record)"
			@keydown.enter.prevent="onRowClick(record)"
		>
			<DomainImg
				:src="ymclRecordCover(record) ?? (typeof record.icon === 'string' ? record.icon : null)"
				:alt="ymclRecordTitle(record)"
				class="h-10 w-10 shrink-0 rounded-lg object-contain"
			/>
			<div class="min-w-0 flex-1">
				<div class="flex items-center gap-2">
					<span
						class="inline-block h-2 w-2 shrink-0 rounded-full"
						:class="record.status === 'online' ? 'bg-green' : 'bg-red'"
					></span>
					<span class="truncate font-semibold text-contrast">
						{{ ymclRecordTitle(record) }}
					</span>
				</div>
				<div class="truncate text-xs text-secondary">
					{{ subtitle(record) }}
				</div>
			</div>
			<div class="flex shrink-0 items-center gap-2">
				<template v-if="primaryAction(record)">
					<ButtonStyled>
						<button type="button" @click.stop="run(primaryAction(record)!, record)">
							<PlayIcon />
							{{ actionTitle(primaryAction(record)!) }}
						</button>
					</ButtonStyled>
				</template>
				<ButtonStyled
					v-for="action in secondaryActions(record)"
					:key="action.code"
					type="standard"
					circular
				>
					<button
						type="button"
						v-tooltip="actionTitle(action)"
						@click.stop="run(action, record)"
					>
						<ClipboardCopyIcon v-if="actionIconKind(action) === 'copy'" />
						<RefreshCwIcon v-else-if="actionIconKind(action) === 'reload'" />
						<PlayIcon v-else-if="actionIconKind(action) === 'play'" />
						<CheckIcon v-else />
					</button>
				</ButtonStyled>
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
	<JoinServerModal ref="joinModal" />
	<DomainRecordDetailModal
		ref="detailModal"
		:record="activeRecord"
		:item-actions="itemActions"
		:allow="allow"
		@reload="emit('reload')"
	/>
</template>
