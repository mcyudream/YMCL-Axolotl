<script setup lang="ts">
import { DownloadIcon, LoadIcon, PlusIcon, TriangleAlertIcon, UploadIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref } from 'vue'

import { useYmclStore } from '@/store/ymcl'

/**
 * Publish console (YAP §7, admin): pick a local MIP-managed instance,
 * inspect the diff against the domain binding's current version, and push
 * it as an incremental delta publish (diff → upload → publish → bind →
 * notify, in one action).
 */

const { handleError, addNotification } = injectNotificationManager()
const { formatMessage } = useVIntl()
const ymclStore = useYmclStore()

const messages = defineMessages({
	title: {
		id: 'app.ymcl.publish.title',
		defaultMessage: '发布控制台',
	},
	description: {
		id: 'app.ymcl.publish.description',
		defaultMessage:
			'Push a local modpack instance to the domain as an initial package or incremental update. Publishing binds the pack to the selected server and notifies members.',
	},
	instance: { id: 'app.ymcl.publish.instance', defaultMessage: '实例' },
	loading: { id: 'app.ymcl.publish.loading', defaultMessage: 'Loading…' },
	checkDiff: { id: 'app.ymcl.publish.check', defaultMessage: '检查变更' },
	versionLabel: { id: 'app.ymcl.publish.version', defaultMessage: '新版本号' },
	channelLabel: { id: 'app.ymcl.publish.channel', defaultMessage: '渠道' },
	push: { id: 'app.ymcl.publish.push', defaultMessage: '发布更新' },
	nothingToDo: {
		id: 'app.ymcl.publish.nothing',
		defaultMessage: '没有可发布的变更：实例与绑定版本一致。',
	},
	needsDomain: {
		id: 'app.ymcl.publish.needs-domain',
		defaultMessage: 'Publishing requires an active domain and a signed-in session.',
	},
	notManaged: {
		id: 'app.ymcl.publish.not-managed',
		defaultMessage: 'This instance is not a MIP-managed instance.',
	},
	changed: { id: 'app.ymcl.publish.changed', defaultMessage: 'Changed' },
	added: { id: 'app.ymcl.publish.added', defaultMessage: 'Added' },
	deleted: { id: 'app.ymcl.publish.deleted', defaultMessage: 'Deleted' },
	moved: { id: 'app.ymcl.publish.moved', defaultMessage: 'Moved' },
	excluded: {
		id: 'app.ymcl.publish.excluded',
		defaultMessage: 'Excluded (player-side files)',
	},
	published: {
		id: 'app.ymcl.publish.published',
		defaultMessage: '更新已发布并绑定到服务器。',
	},
})

interface PublishDiffEntry {
	path: string
	sha512?: string | null
	from?: string | null
}

interface PublishDiff {
	managed: boolean
	pack_id?: string | null
	base_version?: string | null
	changed: PublishDiffEntry[]
	added: PublishDiffEntry[]
	deleted: string[]
	moved: PublishDiffEntry[]
	excluded: string[]
}

interface InstanceSummary {
	id: string
	name: string
}

const instances = ref<InstanceSummary[]>([])
const selectedInstanceId = ref<string | null>(null)
const diff = ref<PublishDiff | null>(null)
const checking = ref(false)
const pushing = ref(false)
const version = ref('')
const channel = ref('stable')
const serverId = ref('')

const canPublish = computed(
	() => !ymclStore.isPersonal && !!ymclStore.session && selectedInstanceId.value != null,
)

const totalChanges = computed(() =>
	diff.value
		? diff.value.changed.length +
			diff.value.added.length +
			diff.value.deleted.length +
			diff.value.moved.length
		: 0,
)

onMounted(async () => {
	await ymclStore.init()
	const { list } = await import('@/helpers/instance')
	const all = await list().catch(() => [])
	instances.value = all.map((instance) => ({
		id: instance.instance_id,
		name: instance.name,
	}))
})

async function checkDiff() {
	if (!selectedInstanceId.value) return
	checking.value = true
	diff.value = null
	try {
		diff.value = await invoke<PublishDiff>('plugin:ymcl|ymcl_publish_diff', {
			instanceId: selectedInstanceId.value,
		})
		if (!version.value && diff.value.base_version) {
			version.value = bumpPatch(diff.value.base_version)
		}
	} catch (error) {
		handleError(error)
	} finally {
		checking.value = false
	}
}

function bumpPatch(versionString: string): string {
	const parts = versionString.split('.')
	const last = Number(parts[parts.length - 1])
	if (!Number.isNaN(last)) {
		parts[parts.length - 1] = String(last + 1)
		return parts.join('.')
	}
	return versionString
}

async function push() {
	if (!selectedInstanceId.value || !version.value || pushing.value) return
	pushing.value = true
	try {
		await invoke('plugin:ymcl|ymcl_publish_push', {
			instanceId: selectedInstanceId.value,
			version: version.value,
			channel: channel.value || null,
			bind: serverId.value ? { serverId: serverId.value } : null,
		})
		addNotification({
			title: formatMessage(messages.published),
			text: `${diff.value?.pack_id ?? ''} → ${version.value}`,
			type: 'success',
		})
		diff.value = null
	} catch (error) {
		handleError(error)
	} finally {
		pushing.value = false
	}
}
</script>

<template>
	<div class="flex min-h-full flex-col gap-4 p-6">
		<h1 class="m-0 text-2xl font-bold text-contrast">
			{{ formatMessage(messages.title) }}
		</h1>
		<p class="m-0 text-sm leading-relaxed text-secondary">
			{{ formatMessage(messages.description) }}
		</p>

		<div
			v-if="!canPublish"
			class="flex items-center gap-2 rounded-xl bg-bg-raised p-4 text-sm text-secondary"
		>
			<TriangleAlertIcon class="h-5 w-5 shrink-0" />
			{{ formatMessage(messages.needsDomain) }}
		</div>

		<template v-else>
			<div class="flex flex-wrap items-end gap-3">
				<label class="flex min-w-60 flex-1 flex-col gap-1">
					<span class="text-xs font-medium text-secondary">
						{{ formatMessage(messages.instance) }}
					</span>
					<select
						v-model="selectedInstanceId"
						class="w-full rounded-xl border-0 bg-bg-raised px-3 py-2 text-sm text-contrast"
					>
						<option v-for="instance in instances" :key="instance.id" :value="instance.id">
							{{ instance.name }}
						</option>
					</select>
				</label>
				<ButtonStyled>
					<button :disabled="checking || !selectedInstanceId" @click="checkDiff">
						<LoadIcon v-if="checking" class="animate-spin" />
						<DownloadIcon v-else />
						{{ formatMessage(messages.checkDiff) }}
					</button>
				</ButtonStyled>
			</div>

			<template v-if="diff">
				<div v-if="!diff.managed" class="rounded-xl bg-bg-raised p-4 text-sm text-secondary">
					{{ formatMessage(messages.notManaged) }}
				</div>
				<template v-else>
					<div class="grid grid-cols-2 gap-2 sm:grid-cols-4">
						<div class="rounded-xl bg-bg-raised p-3">
							<div class="text-xl font-bold text-contrast">{{ diff.changed.length }}</div>
							<div class="text-xs text-secondary">{{ formatMessage(messages.changed) }}</div>
						</div>
						<div class="rounded-xl bg-bg-raised p-3">
							<div class="text-xl font-bold text-green">{{ diff.added.length }}</div>
							<div class="text-xs text-secondary">{{ formatMessage(messages.added) }}</div>
						</div>
						<div class="rounded-xl bg-bg-raised p-3">
							<div class="text-xl font-bold text-red">{{ diff.deleted.length }}</div>
							<div class="text-xs text-secondary">{{ formatMessage(messages.deleted) }}</div>
						</div>
						<div class="rounded-xl bg-bg-raised p-3">
							<div class="text-xl font-bold text-contrast">{{ diff.moved.length }}</div>
							<div class="text-xs text-secondary">{{ formatMessage(messages.moved) }}</div>
						</div>
					</div>

					<p v-if="diff.excluded.length > 0" class="m-0 text-xs text-secondary">
						{{ formatMessage(messages.excluded) }}: {{ diff.excluded.join(', ') }}
					</p>

					<div v-if="totalChanges > 0" class="flex flex-wrap items-end gap-3">
						<label class="flex min-w-40 flex-col gap-1">
							<span class="text-xs font-medium text-secondary">
								{{ formatMessage(messages.versionLabel) }}
							</span>
							<StyledInput v-model="version" class="w-full" />
						</label>
						<label class="flex min-w-40 flex-col gap-1">
							<span class="text-xs font-medium text-secondary">
								{{ formatMessage(messages.channelLabel) }}
							</span>
							<StyledInput v-model="channel" class="w-full" />
						</label>
						<ButtonStyled>
							<button :disabled="pushing || !version" @click="push">
								<LoadIcon v-if="pushing" class="animate-spin" />
								<UploadIcon v-else />
								{{ formatMessage(messages.push) }}
							</button>
						</ButtonStyled>
					</div>
					<p v-else class="m-0 text-sm text-secondary">
						{{ formatMessage(messages.nothingToDo) }}
					</p>
				</template>
			</template>

			<div v-else-if="!checking && selectedInstanceId" class="flex justify-center p-6">
				<ButtonStyled type="standard">
					<button disabled>
						<PlusIcon />
						{{ formatMessage(messages.checkDiff) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</div>
</template>
