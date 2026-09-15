<script setup lang="ts">
import { CheckIcon, ClipboardCopyIcon, PlayIcon, RefreshCwIcon } from '@modrinth/assets'
import { ButtonStyled, useVIntl } from '@modrinth/ui'
import { computed, ref } from 'vue'

import { executeAction, renderActionParams, type YmclAction } from '@/helpers/ymcl-actions'

/**
 * server-list renderer (YAP §6.6): server records with status, current
 * season and declarative join/copy actions.
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

const refreshing = ref<string | null>(null)

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
})

const records = computed(() => props.envelope.records ?? [])
const itemActions = computed(() => props.envelope.itemActions ?? [])
const allow = computed(() => props.envelope.allow ?? [])

function primaryAction(record: Record<string, unknown>): YmclAction | null {
	return itemActions.value.find((action) => action.primary && isExecutable(action, record)) ?? null
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
	return itemActions.value.filter((action) => !action.primary && isExecutable(action, record))
}

function isExecutable(action: YmclAction, record: Record<string, unknown>): boolean {
	const params = renderActionParams(action, record)
	if (action.kind === 'client:launch-server') return !!params.address && !!params.instanceId
	if (action.kind === 'client:open-url') return /^https:\/\//.test(String(params.url ?? ''))
	if (action.kind === 'client:copy') return !!params.text
	if (action.kind === 'client:launch-instance') return !!params.instanceId
	return false
}

async function run(action: YmclAction, record: Record<string, unknown>) {
	if (action.kind === 'client:reload') {
		refreshing.value = 'page'
		emit('reload')
		refreshing.value = null
		return
	}
	await executeAction(action, { allow: allow.value, record })
}
</script>

<template>
	<div class="flex flex-col gap-2">
		<div
			v-for="record in records"
			:key="String(record.id ?? record.title)"
			class="flex items-center gap-3 rounded-xl border border-solid border-surface-5 bg-bg-raised p-3"
		>
			<img
				v-if="record.icon"
				:src="String(record.icon)"
				:alt="String(record.title ?? '')"
				class="h-10 w-10 shrink-0 rounded-lg object-contain"
			/>
			<div class="min-w-0 flex-1">
				<div class="flex items-center gap-2">
					<span
						class="inline-block h-2 w-2 shrink-0 rounded-full"
						:class="record.status === 'online' ? 'bg-green' : 'bg-red'"
					></span>
					<span class="truncate font-semibold text-contrast">
						{{ record.title }}
					</span>
				</div>
				<div class="truncate text-xs text-secondary">
					{{ subtitle(record) }}
				</div>
			</div>
			<div class="flex shrink-0 items-center gap-2">
				<template v-if="primaryAction(record)">
					<ButtonStyled>
						<button @click="run(primaryAction(record)!, record)">
							<PlayIcon />
							{{ primaryAction(record)!.title }}
						</button>
					</ButtonStyled>
				</template>
				<ButtonStyled
					v-for="action in secondaryActions(record)"
					:key="action.code"
					type="standard"
					circular
				>
					<button v-tooltip="action.title" @click="run(action, record)">
						<ClipboardCopyIcon v-if="action.kind === 'client:copy'" />
						<RefreshCwIcon v-else-if="action.kind === 'client:reload'" />
						<CheckIcon v-else />
					</button>
				</ButtonStyled>
			</div>
		</div>
		<p v-if="records.length === 0" class="m-0 p-6 text-center text-sm text-secondary">
			{{ formatMessage(messages.empty) }}
		</p>
	</div>
</template>
