<script setup lang="ts">
import { CheckIcon, DownloadIcon, LoaderIcon, PlayIcon, TriangleAlertIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	Checkbox,
	Combobox,
	defineMessages,
	injectNotificationManager,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref, watch } from 'vue'

import InstancePickerList from '@/components/ui/instance/InstancePickerList.vue'
import {
	install_create_instance,
	install_create_modpack_instance,
	install_pack_to_existing_instance,
	wait_for_install_job,
} from '@/helpers/install'
import { list, run } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'
import { catchUpPackUpdate, ymcl, ymclErrorMessage } from '@/helpers/ymcl'

/**
 * Join-flow modal (YAP §7 进服分流): a domain server requires either its
 * bound modpack or just a game version (pure/vanilla servers).
 *
 * Pack servers keep the required-pack path only: when the backend reports a
 * local instance already holding the bound pack (`.pack-state.json` scan)
 * the modal defaults to a direct launch (pre-launch authoritative update
 * covers version drift), otherwise it offers installing into a picked
 * instance or creating a new one — the pure-vanilla and generic-instance
 * bypasses are reserved for pack-free servers (three-way choice with the
 * optional vanilla-enhanced pack). Every path ends in a quick-play launch.
 * Pack downloads go through the composed mrpack + built-in importer (WF-4).
 */

const { addNotification } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	loading: { id: 'app.ymcl.publish.loading', defaultMessage: '加载中…' },
	title: {
		id: 'app.ymcl.join.title',
		defaultMessage: '加入服务器',
	},
	serverLabel: {
		id: 'app.ymcl.install.server-label',
		defaultMessage: '服务器',
	},
	noServers: {
		id: 'app.ymcl.join.no-servers',
		defaultMessage: '当前域没有可加入的服务器。',
	},
	unavailable: {
		id: 'app.ymcl.install.unavailable',
		defaultMessage: '无法加入',
	},
	versionLabel: { id: 'app.ymcl.install.version', defaultMessage: '版本' },
	requiredVersion: {
		id: 'app.ymcl.join.required-version',
		defaultMessage: '版本要求：{version}',
	},
	noRequirement: {
		id: 'app.ymcl.join.no-requirement',
		defaultMessage: '该服务器未声明版本要求，可使用任意实例加入。',
	},
	modePack: {
		id: 'app.ymcl.join.mode-pack',
		defaultMessage: '安装整合包',
	},
	modePackHint: {
		id: 'app.ymcl.join.mode-pack-hint',
		defaultMessage: '下载服务器绑定的整合包并装入下方实例，安装完成后自动进服。',
	},
	modePackNew: {
		id: 'app.ymcl.join.mode-pack-new',
		defaultMessage: '新建实例并安装整合包',
	},
	modePackNewHint: {
		id: 'app.ymcl.join.mode-pack-new-hint',
		defaultMessage: '下载整合包并自动创建一个新实例（版本与加载器由整合包决定），装好后进服。',
	},
	modeEnhanced: {
		id: 'app.ymcl.join.mode-enhanced',
		defaultMessage: '下载原版增强整合包',
	},
	modeEnhancedHint: {
		id: 'app.ymcl.join.mode-enhanced-hint',
		defaultMessage: '下载管理员上传的原版增强整合包，自动创建实例后进服。',
	},
	noVanillaVersion: {
		id: 'app.ymcl.join.no-vanilla-version',
		defaultMessage: '该服务器未声明游戏版本，无法自动创建纯净原版实例。',
	},
	modeExisting: {
		id: 'app.ymcl.join.mode-existing',
		defaultMessage: '使用本地实例启动',
	},
	modeExistingHint: {
		id: 'app.ymcl.join.mode-existing-hint',
		defaultMessage: '选择一个本地实例直接进服。',
	},
	modeVanilla: {
		id: 'app.ymcl.join.mode-vanilla',
		defaultMessage: '下载纯净原版',
	},
	modeVanillaHint: {
		id: 'app.ymcl.join.mode-vanilla-hint',
		defaultMessage: '按版本要求自动创建一个纯净原版实例并进服，不安装任何整合包内容。',
	},
	installedTitle: {
		id: 'app.ymcl.join.installed-title',
		defaultMessage: '已安装服务器整合包',
	},
	installedDetail: {
		id: 'app.ymcl.join.installed-detail',
		defaultMessage: '实例「{instance}」已安装 {pack} {version}，可直接进服。',
	},
	installedOutdated: {
		id: 'app.ymcl.join.installed-outdated',
		defaultMessage: '本地为 {version}，启动时会自动更新到 {target}。',
	},
	pickInstance: {
		id: 'app.ymcl.join.pick-instance',
		defaultMessage: '选择实例',
	},
	noInstances: {
		id: 'app.ymcl.join.no-instances',
		defaultMessage: '还没有本地实例。',
	},
	noMatches: {
		id: 'app.ymcl.join.no-matches',
		defaultMessage: '没有匹配的实例。',
	},
	matching: {
		id: 'app.ymcl.join.matching',
		defaultMessage: '只看匹配版本',
	},
	noMatching: {
		id: 'app.ymcl.join.no-matching',
		defaultMessage: '没有版本为 {version} 的本地实例，可切换「只看匹配版本」或选择下载纯净原版。',
	},
	join: { id: 'app.ymcl.join.join', defaultMessage: '进服' },
	joining: { id: 'app.ymcl.join.joining', defaultMessage: '正在准备并启动…' },
	joined: {
		id: 'app.ymcl.join.joined',
		defaultMessage: '已启动并连接服务器。',
	},
	joinFailed: {
		id: 'app.ymcl.join.join-failed',
		defaultMessage: '进服失败',
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
	instanceRequired: {
		id: 'app.ymcl.join.instance-required',
		defaultMessage: '请先选择一个实例。',
	},
	addressMissing: {
		id: 'app.ymcl.join.address-missing',
		defaultMessage: '该服务器没有可用地址，无法自动进服。',
	},
})

interface ServerOption {
	serverId: string
	name?: string | null
	mcAddress?: string | null
	binding?: { packId?: string; mcVersion?: string | null } | null
}

interface JoinFeature {
	id: string
	name?: string | null
	default: boolean
	conflicts: string[]
}

interface InstalledPack {
	instance_id: string
	pack_id: string
	version?: string | null
	up_to_date: boolean
}

interface JoinPreview {
	available: boolean
	reason?: string | null
	server_id?: string | null
	server_name?: string | null
	mc_address?: string | null
	status?: string | null
	has_pack: boolean
	pack_id?: string | null
	channel?: string | null
	target_version?: string | null
	required_version?: string | null
	season_id?: string | null
	season_name?: string | null
	features: JoinFeature[]
	optional_pack_id?: string | null
	optional_target_version?: string | null
	optional_features?: JoinFeature[]
	installed?: InstalledPack | null
}

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const loading = ref(false)
const working = ref(false)
const servers = ref<ServerOption[]>([])
const serverId = ref('')
const preview = ref<JoinPreview | null>(null)
const mode = ref<'pack' | 'pack-new' | 'existing' | 'vanilla' | 'enhanced'>('existing')
const instances = ref<GameInstance[]>([])
const selectedInstanceId = ref('')
const onlyMatching = ref(true)
const checked = ref<Set<string>>(new Set())
const vanillaName = ref('')

const serverOptions = computed(() =>
	servers.value.map((server) => ({
		value: server.serverId,
		label: server.name || server.serverId,
	})),
)

const requiredVersion = computed(() => preview.value?.required_version ?? null)
const hasAddress = computed(() => !!preview.value?.mc_address)

/** Server-bound pack already installed locally (backend pack-state scan). */
const installedPack = computed(() =>
	preview.value?.has_pack ? (preview.value.installed ?? null) : null,
)

const installedInstanceName = computed(() => {
	const id = installedPack.value?.instance_id
	if (!id) return ''
	return instances.value.find((instance) => instance.id === id)?.name ?? id
})

/** Feature list driving the checkboxes: required pack features for pack
 * modes, the vanilla-enhanced pack's features for `enhanced`. */
const activeFeatures = computed<JoinFeature[]>(() =>
	mode.value === 'enhanced'
		? (preview.value?.optional_features ?? [])
		: (preview.value?.features ?? []),
)

// 进入 enhanced 模式时勾选项切换为可选包的默认集合（与必装包互不影响：
// 两种模式不会同时出现在同一台服务器的预览里）。
watch(mode, (value) => {
	if (value === 'enhanced') {
		checked.value = new Set(activeFeatures.value.filter((f) => f.default).map((f) => f.id))
	}
})

function disabledReason(feature: JoinFeature): string | null {
	if (checked.value.has(feature.id)) return null
	for (const other of activeFeatures.value) {
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
		const feature = activeFeatures.value.find((item) => item.id === featureId)
		for (const conflict of feature?.conflicts ?? []) next.delete(conflict)
	} else {
		next.delete(featureId)
	}
	checked.value = next
}

/** Instances matching the server's version requirement, when one exists. */
const candidateInstances = computed(() => {
	if (!onlyMatching.value || !requiredVersion.value) return instances.value
	return instances.value.filter((instance) => instance.game_version === requiredVersion.value)
})

const instanceMatches = computed(
	() =>
		!requiredVersion.value ||
		instances.value.find((instance) => instance.id === selectedInstanceId.value)?.game_version ===
			requiredVersion.value,
)

const canJoin = computed(() => {
	if (working.value || !hasAddress.value) return false
	if (mode.value === 'vanilla') return !!requiredVersion.value
	if (mode.value === 'pack-new' || mode.value === 'enhanced') return true
	return !!selectedInstanceId.value
})

async function loadPreview(id: string) {
	preview.value = null
	checked.value = new Set()
	try {
		const result = await invoke<JoinPreview>('plugin:ymcl|ymcl_join_preview', {
			serverId: id || null,
		})
		preview.value = result
		checked.value = new Set(
			result.features.filter((feature) => feature.default).map((feature) => feature.id),
		)
		// 已装当前绑定包 → 默认直接进服（版本落后由启动前权威更新兜底）；
		// 否则有整合包要求时走装包路径（无本地实例则直接新建），纯服默认本地。
		if (result.installed) {
			selectedInstanceId.value = result.installed.instance_id
		}
		mode.value = result.has_pack
			? result.installed
				? 'existing'
				: instances.value.length === 0
					? 'pack-new'
					: 'pack'
			: 'existing'
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
	await loadPreview(value)
}

async function loadInstances() {
	try {
		instances.value = await list()
	} catch {
		instances.value = []
	}
}

async function join() {
	if (!canJoin.value || working.value) return
	if (!hasAddress.value) {
		addNotification({
			title: formatMessage(messages.addressMissing),
			type: 'error',
		})
		return
	}
	working.value = true
	try {
		let instanceId = selectedInstanceId.value
		if (mode.value === 'vanilla') {
			if (!requiredVersion.value) {
				addNotification({
					title: formatMessage(messages.noVanillaVersion),
					type: 'error',
				})
				return
			}
			const name =
				vanillaName.value.trim() ||
				`${preview.value?.server_name || 'Server'} ${requiredVersion.value}`.trim()
			const job = await install_create_instance({
				name,
				gameVersion: requiredVersion.value,
				loader: 'vanilla',
				loaderVersion: null,
				adjuncts: [],
				iconPath: null,
			})
			const settled = await wait_for_install_job(job.job_id)
			instanceId = settled.target.instance_id ?? ''
			if (!instanceId) {
				throw new Error('实例创建未返回 instanceId')
			}
		} else if (
			mode.value === 'enhanced' ||
			(preview.value?.has_pack && (mode.value === 'pack' || mode.value === 'pack-new'))
		) {
			// WF-4 首装走 mrpack：归档自带 dependencies，游戏版本/加载器由内置导入器物化；
			// enhanced 模式显式传可选包 id（绑定里的 optionalPackId），其余走必装包。
			// 目标版本没有完整包（纯增量版本）时后端回退为 parent 链上的基线
			// mrpack，adopt 之后增量追平到目标版本再进服。
			const selected = [...checked.value]
			const archive = await ymcl.downloadPackMrpack(
				serverId.value,
				mode.value === 'enhanced' ? (preview.value?.optional_pack_id ?? null) : null,
				selected,
			)
			if (mode.value === 'pack') {
				const job = await install_pack_to_existing_instance(
					instanceId,
					{ type: 'fromFile', path: archive.path },
					null,
				)
				await wait_for_install_job(job.job_id)
			} else {
				const name = preview.value?.server_name || archive.pack_id
				const job = await install_create_modpack_instance(
					{ type: 'fromFile', path: archive.path },
					{ name },
				)
				const settled = await wait_for_install_job(job.job_id)
				instanceId = settled.target.instance_id ?? ''
				if (!instanceId) {
					throw new Error('实例创建未返回 instanceId')
				}
			}
			await ymcl.adoptPackState(
				instanceId,
				serverId.value,
				archive.pack_id,
				archive.version,
				archive.channel ?? null,
				selected,
			)
			await catchUpPackUpdate(instanceId, archive, (error) => {
				addNotification({
					title: formatMessage(messages.catchupFailed, {
						version: archive.version,
						target: archive.target_version ?? '',
					}),
					text: ymclErrorMessage(error),
					type: 'error',
				})
			})
		}
		await run(instanceId, preview.value?.mc_address ?? null)
		addNotification({
			title: formatMessage(messages.joined),
			text: `${preview.value?.server_name || serverId.value}`,
			type: 'success',
		})
		modal.value?.hide()
	} catch (error) {
		addNotification({
			title: formatMessage(messages.joinFailed),
			text: ymclErrorMessage(error),
			type: 'error',
		})
	} finally {
		working.value = false
	}
}

defineExpose({
	show: async (prefill?: { serverId?: string; address?: string }) => {
		loading.value = true
		servers.value = []
		serverId.value = ''
		preview.value = null
		instances.value = []
		selectedInstanceId.value = ''
		checked.value = new Set()
		modal.value?.show()
		await loadInstances()
		try {
			servers.value = await invoke<ServerOption[]>('plugin:ymcl|ymcl_mip_servers')
			const pre = prefill?.serverId
				? servers.value.find((server) => server.serverId === prefill.serverId)
				: prefill?.address
					? servers.value.find((server) => server.mcAddress === prefill.address)
					: undefined
			if (pre) {
				serverId.value = pre.serverId
			} else if (!prefill && servers.value.length > 0) {
				serverId.value = servers.value[0]!.serverId
			}
			if (serverId.value) {
				await loadPreview(serverId.value)
			}
		} catch {
			// 无 MIP 面：模板显示 noServers。
		} finally {
			loading.value = false
		}
	},
})
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.title)"
		scrollable
		width="38rem"
		max-width="calc(100vw - 2rem)"
	>
		<div class="flex min-h-40 flex-col gap-3">
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
							:disabled="working"
							class="w-full"
							@update:model-value="(value: string) => onServerChange(value)"
						/>
					</div>

					<div
						v-if="preview && !preview.available"
						class="flex items-center gap-2 rounded-xl bg-bg-raised p-4 text-sm text-secondary"
					>
						<TriangleAlertIcon class="h-5 w-5 shrink-0" />
						{{ formatMessage(messages.unavailable) }}：{{ preview.reason ?? '' }}
					</div>

					<template v-else-if="preview">
						<div class="rounded-xl bg-bg-raised p-3 text-sm">
							<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
								<span class="font-semibold text-contrast">
									{{ preview.server_name || preview.server_id }}
								</span>
								<code v-if="preview.mc_address" class="font-mono text-xs text-secondary">
									{{ preview.mc_address }}
								</code>
							</div>
							<div class="mt-1 text-xs text-secondary">
								<template v-if="requiredVersion">
									{{
										formatMessage(messages.requiredVersion, {
											version: requiredVersion,
										})
									}}
									<template v-if="preview.has_pack && preview.pack_id">
										· {{ preview.pack_id }} → {{ preview.target_version }}
									</template>
								</template>
								<template v-else>
									{{ formatMessage(messages.noRequirement) }}
								</template>
							</div>
						</div>

						<div
							v-if="preview.has_pack && installedPack"
							class="rounded-xl bg-bg-raised p-3 text-sm ring-1 ring-brand"
						>
							<div class="flex items-center gap-2 font-medium text-contrast">
								<CheckIcon class="h-4 w-4 shrink-0 text-brand" />
								{{ formatMessage(messages.installedTitle) }}
							</div>
							<p class="m-0 mt-1 text-xs text-secondary">
								{{
									formatMessage(messages.installedDetail, {
										instance: installedInstanceName,
										pack: installedPack.pack_id,
										version: installedPack.version ?? '',
									})
								}}
							</p>
							<p
								v-if="!installedPack.up_to_date && preview.target_version"
								class="m-0 mt-1 text-xs text-orange"
							>
								{{
									formatMessage(messages.installedOutdated, {
										version: installedPack.version ?? '',
										target: preview.target_version,
									})
								}}
							</p>
						</div>

						<div class="flex flex-col gap-2">
							<label
								v-if="preview.has_pack"
								class="flex cursor-pointer items-start gap-3 rounded-xl bg-bg-raised p-3"
								:class="mode === 'pack' ? 'ring-1 ring-brand' : ''"
							>
								<input v-model="mode" type="radio" value="pack" class="mt-1" :disabled="working" />
								<span class="flex flex-col gap-0.5">
									<span class="font-medium text-contrast">
										{{ formatMessage(messages.modePack) }}
									</span>
									<span class="text-xs text-secondary">
										{{ formatMessage(messages.modePackHint) }}
									</span>
								</span>
							</label>
							<label
								v-if="preview.has_pack"
								class="flex cursor-pointer items-start gap-3 rounded-xl bg-bg-raised p-3"
								:class="mode === 'pack-new' ? 'ring-1 ring-brand' : ''"
							>
								<input
									v-model="mode"
									type="radio"
									value="pack-new"
									class="mt-1"
									:disabled="working"
								/>
								<span class="flex flex-col gap-0.5">
									<span class="font-medium text-contrast">
										{{ formatMessage(messages.modePackNew) }}
									</span>
									<span class="text-xs text-secondary">
										{{ formatMessage(messages.modePackNewHint) }}
									</span>
								</span>
							</label>
							<label
								v-if="!preview.has_pack"
								class="flex cursor-pointer items-start gap-3 rounded-xl bg-bg-raised p-3"
								:class="mode === 'existing' ? 'ring-1 ring-brand' : ''"
							>
								<input
									v-model="mode"
									type="radio"
									value="existing"
									class="mt-1"
									:disabled="working"
								/>
								<span class="flex flex-col gap-0.5">
									<span class="font-medium text-contrast">
										{{ formatMessage(messages.modeExisting) }}
									</span>
									<span class="text-xs text-secondary">
										{{ formatMessage(messages.modeExistingHint) }}
									</span>
								</span>
							</label>
							<label
								v-if="!preview.has_pack && preview.optional_pack_id"
								class="flex cursor-pointer items-start gap-3 rounded-xl bg-bg-raised p-3"
								:class="mode === 'enhanced' ? 'ring-1 ring-brand' : ''"
							>
								<input
									v-model="mode"
									type="radio"
									value="enhanced"
									class="mt-1"
									:disabled="working"
								/>
								<span class="flex flex-col gap-0.5">
									<span class="font-medium text-contrast">
										{{ formatMessage(messages.modeEnhanced) }}
										<code
											v-if="preview.optional_target_version"
											class="ml-1 font-mono text-xs font-normal text-secondary"
										>
											{{ preview.optional_pack_id }} → {{ preview.optional_target_version }}
										</code>
									</span>
									<span class="text-xs text-secondary">
										{{ formatMessage(messages.modeEnhancedHint) }}
									</span>
								</span>
							</label>
							<label
								v-if="!preview.has_pack"
								class="flex items-start gap-3 rounded-xl bg-bg-raised p-3"
								:class="[
									mode === 'vanilla' ? 'ring-1 ring-brand' : '',
									requiredVersion ? 'cursor-pointer' : 'cursor-not-allowed opacity-60',
								]"
							>
								<input
									v-model="mode"
									type="radio"
									value="vanilla"
									class="mt-1"
									:disabled="working || !requiredVersion"
								/>
								<span class="flex flex-col gap-0.5">
									<span class="font-medium text-contrast">
										{{ formatMessage(messages.modeVanilla) }}
									</span>
									<span class="text-xs text-secondary">
										{{ formatMessage(messages.modeVanillaHint) }}
									</span>
								</span>
							</label>
						</div>

						<!-- Instance picker: install target for pack installs, or the
							generic matching-instance path on pack-free servers. An
							already-installed pack instance skips it (direct launch). -->
						<template
							v-if="mode === 'pack' || (mode === 'existing' && !preview.has_pack)"
						>
							<div class="flex items-center justify-between">
								<p class="m-0 text-sm font-semibold text-contrast">
									{{ formatMessage(messages.pickInstance) }}
								</p>
								<label
									v-if="requiredVersion"
									class="flex items-center gap-1 text-xs text-secondary"
								>
									<input v-model="onlyMatching" type="checkbox" :disabled="working" />
									{{ formatMessage(messages.matching) }}
								</label>
							</div>
							<InstancePickerList
								:instances="candidateInstances"
								search-placeholder=""
								:no-instances-message="formatMessage(messages.noInstances)"
								:no-matches-message="formatMessage(messages.noMatches)"
								:select-label="(instance: GameInstance) => instance.name"
								@select="selectedInstanceId = $event.id"
							/>
							<p
								v-if="selectedInstanceId && requiredVersion && !instanceMatches"
								class="m-0 text-xs text-orange"
							>
								{{
									formatMessage(messages.noMatching, {
										version: requiredVersion,
									})
								}}
							</p>
						</template>

						<template
							v-if="
								(mode === 'pack' || mode === 'pack-new' || mode === 'enhanced') &&
								activeFeatures.length > 0
							"
						>
							<p class="m-0 text-sm font-semibold text-contrast">
								{{ formatMessage(mode === 'enhanced' ? messages.modeEnhanced : messages.modePack) }}
							</p>
							<label
								v-for="feature in activeFeatures"
								:key="feature.id"
								class="flex cursor-pointer items-start gap-3 rounded-xl bg-bg-raised p-3"
								:class="disabledReason(feature) ? 'opacity-60' : ''"
							>
								<Checkbox
									:model-value="checked.has(feature.id)"
									:disabled="working || !!disabledReason(feature)"
									class="mt-0.5"
									@update:model-value="(value: boolean) => toggle(feature.id, value)"
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
						</template>

						<ButtonStyled color="brand">
							<button :disabled="!canJoin" class="flex items-center gap-2" @click="join">
								<LoaderIcon v-if="working" class="animate-spin" />
								<DownloadIcon
									v-else-if="
										mode === 'vanilla' ||
										mode === 'pack' ||
										mode === 'pack-new' ||
										mode === 'enhanced'
									"
								/>
								<PlayIcon v-else />
								{{ formatMessage(working ? messages.joining : messages.join) }}
							</button>
						</ButtonStyled>
					</template>
				</template>
			</template>
		</div>
	</NewModal>
</template>
