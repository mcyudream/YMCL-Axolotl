<script setup lang="ts">
import { ButtonStyled, commonMessages, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { computed, ref } from 'vue'

import {
	getAnnouncementByVersion,
	type LauncherAnnouncement,
} from '@/announcements/catalog'
import { AxolotlBrandConfig } from '@/config'
import { isYmclUpdateConfigured } from '@/helpers/ymcl-content'
import {
	findYmclAnnouncementByVersion,
	loadYmclUpdateAnnouncements,
	resolveUpdateChangelogUrl,
} from '@/helpers/ymcl-update-history'

import UpdateAnnouncementContent from './UpdateAnnouncementContent.vue'

const emit = defineEmits<{
	closed: [version: string]
}>()

const { formatMessage } = useVIntl()
const modal = ref<InstanceType<typeof NewModal>>()
const version = ref<string | null>(null)
const remoteAnnouncements = ref<LauncherAnnouncement[] | null>(null)
const announcement = computed(() => {
	if (remoteAnnouncements.value) {
		return findYmclAnnouncementByVersion(remoteAnnouncements.value, version.value)
	}
	return getAnnouncementByVersion(version.value)
})

const messages = defineMessages({
	header: {
		id: 'app.update-announcement.modal.header',
		defaultMessage: "What's new",
	},
})

async function show(nextVersion: string) {
	version.value = nextVersion
	if (isYmclUpdateConfigured()) {
		try {
			const remote = await loadYmclUpdateAnnouncements({ channel: 'all', limit: 50 })
			if (remote) remoteAnnouncements.value = remote
		}
		catch {
			// 保留本地 catalog 回退
		}
	}
	modal.value?.show()
}

function close() {
	modal.value?.hide()
}

function handleHide() {
	if (version.value) emit('closed', version.value)
}

defineExpose({ show, close })
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.header)"
		:on-hide="handleHide"
		max-width="760px"
		max-content-height="min(68vh, 42rem)"
		scrollable
		actions-divider
	>
		<UpdateAnnouncementContent
			:announcement="announcement"
			:version="version"
			:external-url="announcement?.externalUrl ?? resolveUpdateChangelogUrl()"
		/>

		<template #actions>
			<div class="flex justify-end">
				<ButtonStyled color="brand">
					<button type="button" @click="close">
						{{ formatMessage(commonMessages.closeButton) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>
