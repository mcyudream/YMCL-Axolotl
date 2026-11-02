<script setup lang="ts">
import { FolderOpenIcon } from '@modrinth/assets'
import { defineMessages, NewButton, useFormatBytes, useVIntl } from '@modrinth/ui'
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { open } from '@tauri-apps/plugin-dialog'
import { openPath } from '@tauri-apps/plugin-opener'
import { computed } from 'vue'

import {
	getBackupRepositoryStatus,
	isBackupOperationActive,
	listBackupOperations,
	moveBackupRepository,
} from '@/helpers/instance-backup'
import { handleError } from '@/store/notifications'

const { formatMessage } = useVIntl()
const formatBytes = useFormatBytes()
const queryClient = useQueryClient()
const status = useQuery({
	queryKey: ['instance-backup-repository'],
	queryFn: getBackupRepositoryStatus,
	refetchInterval: 5000,
})
const operations = useQuery({
	queryKey: ['instance-backup-operations'],
	queryFn: () => listBackupOperations(),
	refetchInterval: 2000,
})
const activeOperations = computed(
	() => operations.data.value?.filter(isBackupOperationActive) ?? [],
)
const latestMove = computed(() =>
	operations.data.value?.find((operation) => operation.operation_type === 'repository_move'),
)
const moving = computed(() =>
	activeOperations.value.some((operation) => operation.operation_type === 'repository_move'),
)
const messages = defineMessages({
	cancelled: {
		id: 'settings.instanceBackups.cancelled',
		defaultMessage: 'The backup storage move was cancelled.',
	},
	completed: {
		id: 'settings.instanceBackups.completed',
		defaultMessage: 'Backup storage moved successfully.',
	},
	title: { id: 'settings.instanceBackups.title', defaultMessage: 'Instance backup storage' },
	description: {
		id: 'settings.instanceBackups.description',
		defaultMessage:
			'Snapshots share identical files to save disk space. Enable backups from each instance’s settings.',
	},
	usage: {
		id: 'settings.instanceBackups.usage',
		defaultMessage: '{count} snapshots · {stored} stored · {logical} before deduplication',
	},
	move: { id: 'settings.instanceBackups.move', defaultMessage: 'Move backup storage' },
	moving: {
		id: 'settings.instanceBackups.moving',
		defaultMessage: 'Moving backup storage in the background. Keep the launcher open.',
	},
	open: { id: 'settings.instanceBackups.open', defaultMessage: 'Open backup folder' },
	error: { id: 'settings.instanceBackups.error', defaultMessage: 'Could not read backup storage.' },
	retry: { id: 'settings.instanceBackups.retry', defaultMessage: 'Retry' },
})
const move = useMutation({
	mutationFn: moveBackupRepository,
	onSuccess: () => {
		void queryClient.invalidateQueries({ queryKey: ['instance-backup-operations'] })
	},
	onError: handleError,
})
async function chooseDestination() {
	if (move.isPending.value || moving.value) return
	try {
		const destination = await open({
			directory: true,
			multiple: false,
			title: formatMessage(messages.move),
		})
		if (typeof destination === 'string') move.mutate(destination)
	} catch (error) {
		handleError(error as Error)
	}
}
</script>

<template>
	<section data-onboarding-id="instance-backup-repository" class="flex flex-col gap-3">
		<h2
			id="settings-target-instance-backup-repository"
			class="m-0 text-lg font-semibold text-contrast"
		>
			{{ formatMessage(messages.title) }}
		</h2>
		<p class="m-0 text-secondary">{{ formatMessage(messages.description) }}</p>
		<div v-if="status.isError.value" role="alert" class="flex items-center gap-3">
			<span>{{ formatMessage(messages.error) }}</span
			><NewButton @click="status.refetch()">{{ formatMessage(messages.retry) }}</NewButton>
		</div>
		<div v-if="operations.isError.value" role="alert" class="flex items-center gap-3">
			<span>{{ formatMessage(messages.error) }}</span
			><NewButton @click="operations.refetch()">{{ formatMessage(messages.retry) }}</NewButton>
		</div>
		<template v-if="status.data.value">
			<p class="m-0 break-all font-mono text-sm">{{ status.data.value.path }}</p>
			<p class="m-0 text-sm text-secondary">
				{{
					formatMessage(messages.usage, {
						count: status.data.value.snapshot_count,
						stored: formatBytes(status.data.value.stored_size),
						logical: formatBytes(status.data.value.logical_size),
					})
				}}
			</p>
			<p v-if="status.data.value.error" role="alert" class="m-0 break-words text-red">
				{{ status.data.value.error }}
			</p>
			<p v-if="latestMove?.error" role="alert" class="m-0 break-words text-red">
				{{ latestMove.error }}
			</p>
			<p v-else-if="latestMove?.state === 'cancelled'" class="m-0">
				{{ formatMessage(messages.cancelled) }}
			</p>
			<p v-else-if="latestMove?.state === 'completed'" class="m-0">
				{{ formatMessage(messages.completed) }}
			</p>
			<p v-if="moving" role="status" class="m-0">{{ formatMessage(messages.moving) }}</p>
			<div class="flex flex-wrap gap-2">
				<NewButton
					:loading="move.isPending.value"
					:disabled="
						moving ||
						activeOperations.length > 0 ||
						operations.isError.value ||
						operations.isPending.value
					"
					@click="chooseDestination"
					>{{ formatMessage(messages.move) }}</NewButton
				>
				<NewButton
					:disabled="!status.data.value.initialized || !status.data.value.available"
					@click="openPath(status.data.value.path).catch(handleError)"
					><FolderOpenIcon />{{ formatMessage(messages.open) }}</NewButton
				>
			</div>
		</template>
	</section>
</template>
