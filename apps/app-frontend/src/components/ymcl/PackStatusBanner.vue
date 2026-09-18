<script setup lang="ts">
import { LoaderIcon, PackageIcon, UpdatedIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { onMounted, ref, watch } from 'vue'

import { ymcl, ymclErrorMessage, type YmclUpdateCheck } from '@/helpers/ymcl'

/**
 * Managed-pack status for a MIP instance (YAP §7): shows the binding's pack
 * and version on the instance page and offers a manual update — the launch
 * path hot-updates on join, but a standalone launch never did, so this is
 * the only explicit "update modpack" entry for the player.
 */

const props = defineProps<{
	instanceId: string
}>()

const { addNotification } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	managedBy: {
		id: 'app.ymcl.manage.managed-by',
		defaultMessage: '此实例由域整合包 {pack} 管理',
	},
	currentVersion: {
		id: 'app.ymcl.manage.current-version',
		defaultMessage: '当前版本 {version}',
	},
	upToDate: {
		id: 'app.ymcl.manage.up-to-date',
		defaultMessage: '已是最新版本',
	},
	updateAvailable: {
		id: 'app.ymcl.manage.update-available',
		defaultMessage: '可更新到 {version}',
	},
	updateNotes: {
		id: 'app.ymcl.manage.update-notes',
		defaultMessage: '本次更新：{notes}',
	},
	updateNow: {
		id: 'app.ymcl.manage.update-now',
		defaultMessage: '更新整合包',
	},
	updating: {
		id: 'app.ymcl.manage.updating',
		defaultMessage: '正在更新…',
	},
	updated: {
		id: 'app.ymcl.manage.updated',
		defaultMessage: '整合包已更新到 {version}。',
	},
	updateFailed: {
		id: 'app.ymcl.manage.update-failed',
		defaultMessage: '更新失败',
	},
	checkFailed: {
		id: 'app.ymcl.manage.check-failed',
		defaultMessage: '域整合包状态检查失败',
	},
	retry: {
		id: 'app.ymcl.manage.retry',
		defaultMessage: '重试',
	},
})

const check = ref<YmclUpdateCheck | null>(null)
const checkError = ref<string | null>(null)
const updating = ref(false)

const hasUpdate = () =>
	!!check.value?.managed &&
	!!check.value.target_version &&
	check.value.current_version !== check.value.target_version

async function refresh() {
	check.value = null
	checkError.value = null
	try {
		check.value = await ymcl.preLaunchCheck(props.instanceId)
	} catch (error) {
		// 检查失败必须可见：静默隐藏会让"未绑定/没版本信息"这类问题无从排查。
		checkError.value = ymclErrorMessage(error)
	}
}

async function applyUpdate() {
	if (updating.value || !hasUpdate()) return
	updating.value = true
	try {
		const result = await ymcl.applyPackUpdate(props.instanceId)
		addNotification({
			title: formatMessage(messages.updated, { version: result.new_version }),
			text: `${result.staged_files} 个文件更新，${result.deleted_files} 个删除`,
			type: 'success',
		})
		await refresh()
	} catch (error) {
		addNotification({
			title: formatMessage(messages.updateFailed),
			text: ymclErrorMessage(error),
			type: 'error',
		})
	} finally {
		updating.value = false
	}
}

watch(
	() => props.instanceId,
	() => void refresh(),
)

onMounted(() => void refresh())
</script>

<template>
	<div
		v-if="checkError"
		class="mb-3 flex flex-wrap items-center gap-x-3 gap-y-2 rounded-xl bg-bg-raised p-3 text-sm"
	>
		<PackageIcon class="h-5 w-5 shrink-0 text-secondary" />
		<span class="min-w-0 flex-1 text-orange">
			{{ formatMessage(messages.checkFailed) }}：{{ checkError }}
		</span>
		<ButtonStyled size="small" type="outlined">
			<button class="flex items-center gap-2" @click="refresh">
				{{ formatMessage(messages.retry) }}
			</button>
		</ButtonStyled>
	</div>
	<div
		v-else-if="check?.managed && check.pack_id"
		class="mb-3 flex flex-wrap items-center gap-x-3 gap-y-2 rounded-xl bg-bg-raised p-3 text-sm"
	>
		<PackageIcon class="h-5 w-5 shrink-0 text-secondary" />
		<span class="flex min-w-0 flex-col gap-1">
			<span class="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-0.5">
				<span class="font-semibold text-contrast">
					{{ formatMessage(messages.managedBy, { pack: check.pack_id }) }}
				</span>
				<span class="text-secondary">
					{{ formatMessage(messages.currentVersion, { version: check.current_version }) }}
				</span>
				<span v-if="!hasUpdate()" class="text-secondary">
					· {{ formatMessage(messages.upToDate) }}
				</span>
				<span v-else class="flex items-center gap-1 text-orange">
					<UpdatedIcon class="h-4 w-4" />
					{{ formatMessage(messages.updateAvailable, { version: check.target_version }) }}
				</span>
			</span>
			<span
				v-if="hasUpdate() && check.notes?.trim()"
				class="whitespace-pre-line text-xs text-secondary"
			>
				{{ formatMessage(messages.updateNotes, { notes: check.notes.trim() }) }}
			</span>
		</span>
		<ButtonStyled v-if="hasUpdate()" color="brand" size="small">
			<button :disabled="updating" class="flex items-center gap-2" @click="applyUpdate">
				<LoaderIcon v-if="updating" class="animate-spin" />
				{{ formatMessage(updating ? messages.updating : messages.updateNow) }}
			</button>
		</ButtonStyled>
	</div>
</template>
