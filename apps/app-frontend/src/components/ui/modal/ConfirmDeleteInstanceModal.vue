<template>
	<NewModal
		ref="modal"
		:header="formatMessage(count > 1 ? messages.batchHeader : messages.header)"
		fade="danger"
		:disable-close="busy"
		max-width="500px"
	>
		<Admonition
			v-if="!symlinkTarget && count <= 1"
			type="critical"
			:header="formatMessage(messages.admonitionHeader)"
		>
			{{ formatMessage(messages.admonitionBody) }}
		</Admonition>
		<Admonition
			v-else-if="!symlinkTarget"
			type="critical"
			:header="formatMessage(messages.admonitionHeader)"
		>
			{{ formatMessage(messages.batchAdmonitionBody, { count }) }}
		</Admonition>
		<Admonition v-else type="critical">
			{{ formatMessage(messages.symlinkDeleteWarning, { path: symlinkTarget }) }}
		</Admonition>

		<p v-if="snapshotCount > 0" class="mb-0 text-red">{{ formatMessage(messages.backupDeletion, { count: snapshotCount }) }}</p>
		<p v-if="error" role="alert" class="mb-0 break-words text-red">{{ error }}</p>
		<ButtonStyled v-if="!summaryReady && !loadingSummary"><button type="button" @click="show">{{ formatMessage(commonMessages.retryButton) }}</button></ButtonStyled>
		<template #actions>
			<div class="flex gap-2 justify-end">
				<ButtonStyled type="outlined">
					<button :disabled="busy" @click="modal?.hide()">
						<XIcon />
						{{ formatMessage(commonMessages.cancelButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="red">
					<button :disabled="busy || loadingSummary || !summaryReady" @click="confirm">
						<TrashIcon />
						{{
							formatMessage(count > 1 ? messages.batchDeleteButton : messages.deleteButton, {
								count,
							})
						}}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { TrashIcon, XIcon } from '@modrinth/assets'
import {
	Admonition,
	ButtonStyled,
	commonMessages,
	defineMessages,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { nextTick, ref } from 'vue'
import { getBackupDeleteSummary } from '@/helpers/instance-backup'

const { formatMessage } = useVIntl()

const props = withDefaults(
	defineProps<{
		symlinkTarget?: string | null
		instanceIds: string[]
		onConfirm: () => Promise<void>
		count?: number
	}>(),
	{
		symlinkTarget: null,
		count: 1,
	},
)

const messages = defineMessages({
	backupDeletion: { id: 'app.instance.confirm-delete.backups', defaultMessage: 'This also permanently deletes {count, plural, one {# backup snapshot} other {# backup snapshots}}.' },
	header: {
		id: 'app.instance.confirm-delete.header',
		defaultMessage: 'Delete instance',
	},
	batchHeader: {
		id: 'app.instance.confirm-delete.batch-header',
		defaultMessage: 'Delete instances',
	},
	admonitionHeader: {
		id: 'app.instance.confirm-delete.admonition-header',
		defaultMessage: 'This action cannot be undone',
	},
	admonitionBody: {
		id: 'app.instance.confirm-delete.admonition-body',
		defaultMessage:
			'All data for your instance will be permanently deleted, including your worlds, configs, and all installed content.',
	},
	batchAdmonitionBody: {
		id: 'app.instance.confirm-delete.batch-admonition-body',
		defaultMessage:
			'{count, plural, one {# instance} other {# instances}} will be permanently deleted, including worlds, configs, and all installed content.',
	},
	symlinkDeleteWarning: {
		id: 'app.instance.confirm-delete.symlink-warning',
		defaultMessage:
			'This is a shared instance linked to "{path}". Only the link will be removed; the original files will not be deleted.',
	},
	deleteButton: {
		id: 'app.instance.confirm-delete.delete-button',
		defaultMessage: 'Delete instance',
	},
	batchDeleteButton: {
		id: 'app.instance.confirm-delete.batch-delete-button',
		defaultMessage: 'Delete {count, plural, one {# instance} other {# instances}}',
	},
})

const modal = ref<InstanceType<typeof NewModal>>()
const busy = ref(false)
const loadingSummary = ref(false)
const summaryReady = ref(false)
const error = ref('')
const snapshotCount = ref(0)
let generation = 0

async function show() {
	if (busy.value) return
	const current = ++generation
	error.value = ''
	snapshotCount.value = 0
	summaryReady.value = false
	loadingSummary.value = true
	modal.value?.show()
	try {
		await nextTick()
		const instanceIds = [...props.instanceIds]
		if (current !== generation) return
		const summaries = await Promise.all(instanceIds.map(getBackupDeleteSummary))
		if (current === generation) {
			snapshotCount.value = summaries.reduce((sum, item) => sum + item.snapshot_count, 0)
			summaryReady.value = true
		}
	} catch (caught) {
		if (current === generation) error.value = String(caught)
	} finally { if (current === generation) loadingSummary.value = false }
}

async function confirm() {
	if (busy.value || loadingSummary.value || !summaryReady.value) return
	busy.value = true
	error.value = ''
	try {
		await props.onConfirm()
		busy.value = false
		await nextTick()
		modal.value?.hide()
	} catch (caught) { error.value = String(caught) }
	finally { busy.value = false }
}

defineExpose({
	show,
})
</script>
