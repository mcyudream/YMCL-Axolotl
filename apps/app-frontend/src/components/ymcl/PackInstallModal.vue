<script setup lang="ts">
import { DownloadIcon, LoaderIcon, TriangleAlertIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	Checkbox,
	Combobox,
	defineMessages,
	injectNotificationManager,
	NewModal,
	useFormatBytes,
	useVIntl,
} from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'

import { install_pack_to_existing_instance, wait_for_install_job } from '@/helpers/install'
import { catchUpPackUpdate, ymcl, ymclErrorMessage } from '@/helpers/ymcl'

/**
 * WF-4 first install (MIP appendix B.3): pick the domain server whose
 * binding to install, opt into optional content, then download the composed
 * mrpack and materialize it into this instance via the built-in importer.
 */

const props = defineProps<{
	instanceId: string
}>()

const { addNotification } = injectNotificationManager()
const { formatMessage } = useVIntl()
const formatBytes = useFormatBytes()

const messages = defineMessages({
	title: {
		id: 'app.ymcl.install.title',
		defaultMessage: '安装域整合包',
	},
	description: {
		id: 'app.ymcl.install.description',
		defaultMessage: '选择服务器并勾选可选内容，启动器会下载该服务器绑定的整合包并装入此实例。',
	},
	loading: { id: 'app.ymcl.publish.loading', defaultMessage: '加载中…' },
	serverLabel: {
		id: 'app.ymcl.install.server-label',
		defaultMessage: '服务器',
	},
	noServers: {
		id: 'app.ymcl.install.no-servers',
		defaultMessage: '当前域没有绑定整合包的服务器。',
	},
	unavailable: {
		id: 'app.ymcl.install.unavailable',
		defaultMessage: '无法安装',
	},
	versionLabel: { id: 'app.ymcl.install.version', defaultMessage: '版本' },
	filesLabel: { id: 'app.ymcl.install.files', defaultMessage: '{count} 个文件 · {size}' },
	seasonLabel: { id: 'app.ymcl.install.season', defaultMessage: '当前周目：{name}' },
	featuresTitle: {
		id: 'app.ymcl.publish.features-title',
		defaultMessage: '可选内容',
	},
	install: {
		id: 'app.ymcl.install.install',
		defaultMessage: '安装',
	},
	installing: {
		id: 'app.ymcl.install.installing',
		defaultMessage: '正在安装…',
	},
	installed: {
		id: 'app.ymcl.install.installed',
		defaultMessage: '整合包已安装到实例。',
	},
	installFailed: {
		id: 'app.ymcl.install.install-failed',
		defaultMessage: '安装失败',
	},
	catchupFailed: {
		id: 'app.ymcl.packs.catchup-failed',
		defaultMessage:
			'已安装基线版本 {version}，增量更新到 {target} 失败；启动时会自动重试。',
	},
	loadFailed: {
		id: 'app.ymcl.features.load-failed',
		defaultMessage: '读取可选组件失败',
	},
	conflictsWith: {
		id: 'app.ymcl.features.conflicts-with',
		defaultMessage: '与 {id} 互斥',
	},
})

interface ServerOption {
	serverId: string
	name?: string | null
	binding?: { packId: string } | null
}

interface PackFeature {
	id: string
	name?: string | null
	default: boolean
	conflicts: string[]
}

interface InstallPreview {
	available: boolean
	reason?: string | null
	server_id?: string | null
	pack_id?: string | null
	channel?: string | null
	target_version?: string | null
	season_id?: string | null
	season_name?: string | null
	install_files: number
	install_bytes: number
	features: PackFeature[]
}

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const loading = ref(false)
const installing = ref(false)
const servers = ref<ServerOption[]>([])
const serverId = ref('')
const preview = ref<InstallPreview | null>(null)
const checked = ref<Set<string>>(new Set())

const serverOptions = computed(() =>
	servers.value.map((server) => ({
		value: server.serverId,
		label: server.name || server.serverId,
	})),
)

function disabledReason(feature: PackFeature): string | null {
	if (checked.value.has(feature.id)) return null
	for (const other of preview.value?.features ?? []) {
		if (other.id === feature.id) continue
		if (checked.value.has(other.id) && other.conflicts.includes(feature.id)) {
			return formatMessage(messages.conflictsWith, {
				id: other.name || other.id,
			})
		}
	}
	return null
}

function toggle(featureId: string, value: boolean) {
	const next = new Set(checked.value)
	if (value) {
		next.add(featureId)
		const feature = preview.value?.features.find((item) => item.id === featureId)
		for (const conflict of feature?.conflicts ?? []) next.delete(conflict)
	} else {
		next.delete(featureId)
	}
	checked.value = next
}

async function loadPreview() {
	preview.value = null
	try {
		const result = await invoke<InstallPreview>(
			'plugin:ymcl|ymcl_pack_install_preview',
			{ instanceId: props.instanceId, serverId: serverId.value || null },
		)
		preview.value = result
		checked.value = new Set(
			result.features.filter((feature) => feature.default).map((feature) => feature.id),
		)
	} catch (error) {
		addNotification({
			title: formatMessage(messages.loadFailed),
			text: ymclErrorMessage(error),
			type: 'error',
		})
	}
}

async function onServerChange(value: string) {
	serverId.value = value
	await loadPreview()
}

	async function install() {
		if (installing.value || !preview.value?.available) return
		installing.value = true
		try {
			// WF-4 首装走 mrpack：下载归档（勾选的特性在服务端合成进包）→ 内置导入器装入 → adopt MIP 状态。
			// 目标版本没有完整包（纯增量版本）时后端回退为 parent 链上的基线 mrpack，
			// adopt 之后再用既有更新通道增量追平到目标版本。
			const selected = [...checked.value]
			const archive = await ymcl.downloadPackMrpack(serverId.value, null, selected)
			const job = await install_pack_to_existing_instance(
				props.instanceId,
				{ type: 'fromFile', path: archive.path },
				null,
			)
			await wait_for_install_job(job.job_id)
			await ymcl.adoptPackState(
				props.instanceId,
				serverId.value,
				archive.pack_id,
				archive.version,
				archive.channel ?? null,
				selected,
			)
			const finalVersion = await catchUpPackUpdate(props.instanceId, archive, (error) => {
				addNotification({
					title: formatMessage(messages.catchupFailed, {
						version: archive.version,
						target: archive.target_version ?? '',
					}),
					text: ymclErrorMessage(error),
					type: 'error',
				})
			})
			addNotification({
				title: formatMessage(messages.installed),
				text: `${archive.pack_id} → ${finalVersion}`,
				type: 'success',
			})
			modal.value?.hide()
		} catch (error) {
			addNotification({
				title: formatMessage(messages.installFailed),
				text: ymclErrorMessage(error),
				type: 'error',
			})
		} finally {
			installing.value = false
		}
	}

defineExpose({
	show: () => {
		loading.value = true
		servers.value = []
		serverId.value = ''
		preview.value = null
		checked.value = new Set()
		modal.value?.show()
		void (async () => {
			try {
				const all = await invoke<ServerOption[]>('plugin:ymcl|ymcl_mip_servers')
				servers.value = all.filter((server) => server.binding?.packId)
				if (servers.value.length > 0) {
					serverId.value = servers.value[0]!.serverId
					await loadPreview()
				}
			} catch {
				// 无 MIP 面：保持空列表，模板显示 noServers。
			} finally {
				loading.value = false
			}
		})()
	},
})
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.title)"
		scrollable
		width="32rem"
		max-width="calc(100vw - 2rem)"
	>
		<div class="flex min-h-40 flex-col gap-3">
			<p class="m-0 text-sm leading-relaxed text-secondary">
				{{ formatMessage(messages.description) }}
			</p>

			<div v-if="loading" class="flex items-center gap-2 text-sm text-secondary">
				<LoaderIcon class="h-5 w-5 animate-spin" />
				{{ formatMessage(messages.loading) }}
			</div>

			<template v-else>
				<div
					v-if="servers.length === 0"
					class="flex items-center gap-2 rounded-xl bg-bg-raised p-4 text-sm text-secondary"
				>
					<TriangleAlertIcon class="h-5 w-5 shrink-0" />
					{{ formatMessage(messages.noServers) }}
				</div>

				<template v-else>
					<div class="labeled_input w-full">
						<p class="text-contrast font-semibold">
							{{ formatMessage(messages.serverLabel) }}
						</p>
						<Combobox
							:model-value="serverId"
							:options="serverOptions"
							class="w-full"
							@update:model-value="(value: string) => onServerChange(value)"
						/>
					</div>

					<div
						v-if="preview && !preview.available"
						class="flex items-center gap-2 rounded-xl bg-bg-raised p-4 text-sm text-secondary"
					>
						<TriangleAlertIcon class="h-5 w-5 shrink-0" />
						{{
							formatMessage(messages.unavailable) }}：{{
							preview.reason ?? ''
						}}
					</div>

					<template v-else-if="preview">
						<div class="rounded-xl bg-bg-raised p-3 text-sm">
							<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
								<span class="font-semibold text-contrast">
									{{ preview.pack_id }}
								</span>
								<span class="text-secondary">
									{{
										formatMessage(messages.versionLabel)
									}}
									{{ preview.target_version }}
									<template v-if="preview.channel">
										（{{ preview.channel }}）</template
									>
								</span>
							</div>
							<div class="mt-1 text-xs text-secondary">
								{{
									formatMessage(messages.filesLabel, {
										count: preview.install_files,
										size: formatBytes(preview.install_bytes),
									})
								}}
								<template v-if="preview.season_name">
									·
									{{
										formatMessage(messages.seasonLabel, {
											name: preview.season_name,
										})
									}}
								</template>
							</div>
						</div>

						<div v-if="preview.features.length > 0" class="flex flex-col gap-2">
							<p class="m-0 text-sm font-semibold text-contrast">
								{{ formatMessage(messages.featuresTitle) }}
							</p>
							<label
								v-for="feature in preview.features"
								:key="feature.id"
								class="flex cursor-pointer items-start gap-3 rounded-xl bg-bg-raised p-3"
								:class="disabledReason(feature) ? 'opacity-60' : ''"
							>
								<Checkbox
									:model-value="checked.has(feature.id)"
									:disabled="installing || !!disabledReason(feature)"
									class="mt-0.5"
									@update:model-value="
										(value: boolean) => toggle(feature.id, value)
									"
								/>
								<span class="flex min-w-0 flex-col gap-0.5">
									<span class="font-medium text-contrast">
										{{ feature.name || feature.id }}
									</span>
									<span v-if="disabledReason(feature)" class="text-xs text-orange">
										{{ disabledReason(feature) }}
									</span>
								</span>
							</label>
						</div>

						<ButtonStyled color="brand">
							<button
								:disabled="installing"
								class="flex items-center gap-2"
								@click="install"
							>
								<LoaderIcon v-if="installing" class="animate-spin" />
								<DownloadIcon v-else />
								{{
									formatMessage(
										installing ? messages.installing : messages.install,
									)
								}}
							</button>
						</ButtonStyled>
					</template>
				</template>
			</template>
		</div>
	</NewModal>
</template>
