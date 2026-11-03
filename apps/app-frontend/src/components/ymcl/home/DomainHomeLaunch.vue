<script setup lang="ts">
import {
	DownloadIcon,
	FolderOpenIcon,
	PlayIcon,
	RefreshCwIcon,
	SpinnerIcon,
	TerminalSquareIcon,
	TriangleAlertIcon,
} from '@modrinth/assets'
import { ButtonStyled, defineMessages, ProgressBar, useFormatBytes, useVIntl } from '@modrinth/ui'
import { computed, onUnmounted, ref, watch } from 'vue'
import { RouterLink } from 'vue-router'

import {
	type DomainPackOperationScope,
	useDomainPackOperations,
} from '@/composables/domain-pack-operations'
import type { DomainPackEntry } from '@/composables/useDomainPacks'
import { useDomainServers } from '@/composables/useDomainServers'
import { process_listener } from '@/helpers/events'
import {
	install_create_instance,
	install_job_get,
	install_job_retry,
	installJobInstanceId,
	wait_for_install_job,
} from '@/helpers/install'
import { installProgressTextSource } from '@/helpers/install-progress'
import { list, run } from '@/helpers/instance'
import { get_by_instance_id } from '@/helpers/process'
import type { GameInstance } from '@/helpers/types'
import { showInstanceInFolder } from '@/helpers/utils'
import { ymcl, ymclErrorMessage, type YmclJoinPreview, type YmclUpdateCheck } from '@/helpers/ymcl'
import { injectDownloadManager } from '@/providers/download-manager'
import { useYmclStore } from '@/store/ymcl'

import { domainHomeTransferProgress } from './domain-home-transfer'
import { useDomainHomeSession } from './domain-home-session'
import DomainHomeServerPicker from './DomainHomeServerPicker.vue'

const props = defineProps<{ featuredServerId: string; preview?: boolean }>()
const store = useYmclStore()
const manager = injectDownloadManager()
const { formatMessage } = useVIntl()
const formatBytes = useFormatBytes()
const { servers, loading, loadError, refresh, download, update } = useDomainServers()
const operations = useDomainPackOperations()
const session = useDomainHomeSession()
const previewServerId = ref('')
const chosenServerId = computed({
	get: () =>
		props.preview
			? previewServerId.value
			: session.serverFor(store.activeDomainId, store.session?.session.user_id ?? null),
	set: (serverId: string) => {
		if (props.preview) previewServerId.value = serverId
		else
			session.rememberServer(store.activeDomainId, store.session?.session.user_id ?? null, serverId)
	},
})
const server = computed(
	() =>
		servers.value.find(
			(row) => row.serverId === (chosenServerId.value || props.featuredServerId),
		) ??
		servers.value.find((row) => row.available) ??
		servers.value[0],
)
const preview = ref<YmclJoinPreview | null>(null)
const updateCheck = ref<YmclUpdateCheck | null>(null)
const instances = ref<GameInstance[]>([])
const localChoice = ref('')
const checking = ref(false)
const executing = ref(false)
const launching = ref(false)
const launched = ref(false)
const requestError = ref('')
const fileError = ref('')
const operationScope = computed(() => ({
	domainId: store.activeDomainId,
	userId: store.session?.session.user_id ?? null,
	serverId: server.value?.serverId ?? '',
}))
const task = computed(() => (props.preview ? null : operations.operationFor(operationScope.value)))
const operation = computed(() => task.value?.progress ?? null)
const outcome = computed(() => (task.value?.result?.ok ? task.value.result : null))
const working = computed(() => executing.value || !!task.value?.running)
const error = computed(
	() =>
		requestError.value ||
		(!launched.value && task.value?.result?.ok === false ? task.value.result.error : ''),
)
let generation = 0
let contextRevision = 0
let disposed = false

const messages = defineMessages({
	title: { id: 'app.ymcl.landing.quick-play', defaultMessage: 'Quick play' },
	checking: { id: 'app.ymcl.landing.transfer.checking', defaultMessage: 'Checking for updates' },
	downloading: {
		id: 'app.ymcl.landing.transfer.downloading',
		defaultMessage: 'Downloading the server pack',
	},
	installing: { id: 'app.ymcl.landing.transfer.installing', defaultMessage: 'Installing the game' },
	finalizing: {
		id: 'app.ymcl.landing.transfer.finalizing',
		defaultMessage: 'Saving instance configuration',
	},
	updating: {
		id: 'app.ymcl.landing.transfer.updating',
		defaultMessage: 'Updating the server pack',
	},
	launching: { id: 'app.ymcl.landing.launching', defaultMessage: 'Launching…' },
	launched: { id: 'app.ymcl.landing.launched', defaultMessage: 'Game launched' },
	ready: { id: 'app.ymcl.landing.transfer.ready', defaultMessage: 'Ready to play' },
	available: {
		id: 'app.ymcl.landing.transfer.available',
		defaultMessage: 'Download to start playing',
	},
	updateAvailable: {
		id: 'app.ymcl.landing.transfer.update-available',
		defaultMessage: 'An update is available',
	},
	choose: {
		id: 'app.ymcl.landing.choose-instance',
		defaultMessage: 'Choose a local instance to join',
	},
	instance: { id: 'app.ymcl.landing.transfer.instance', defaultMessage: 'Game instance' },
	vanilla: {
		id: 'app.ymcl.landing.transfer.vanilla',
		defaultMessage: 'Download Minecraft {version}',
	},
	enhanced: {
		id: 'app.ymcl.landing.transfer.enhanced',
		defaultMessage: 'Download the enhanced vanilla pack',
	},
	download: { id: 'app.ymcl.packs.download', defaultMessage: 'Download pack' },
	update: { id: 'app.ymcl.packs.update-now', defaultMessage: 'Update pack' },
	launch: { id: 'app.ymcl.landing.launch', defaultMessage: 'Launch and join' },
	create: { id: 'app.ymcl.landing.transfer.create', defaultMessage: 'Create an instance' },
	retry: { id: 'app.ymcl.landing.retry', defaultMessage: 'Retry' },
	failed: { id: 'app.ymcl.landing.transfer.failed', defaultMessage: 'Could not prepare the game' },
	none: {
		id: 'app.ymcl.landing.no-servers',
		defaultMessage: 'No servers are available in this domain yet.',
	},
	serverError: { id: 'app.ymcl.landing.server-error', defaultMessage: 'Could not load servers.' },
	versions: { id: 'app.ymcl.landing.transfer.versions', defaultMessage: '{current} → {target}' },
	version: { id: 'app.ymcl.landing.transfer.version', defaultMessage: 'Pack version {version}' },
	defaultFeatures: {
		id: 'app.ymcl.landing.transfer.default-features',
		defaultMessage: 'Creates a separate instance with the server’s default components.',
	},
	files: { id: 'app.ymcl.landing.transfer.files', defaultMessage: '{current} / {total} files' },
	changed: {
		id: 'app.ymcl.landing.transfer.changed',
		defaultMessage: 'Updated {count} files; removed {deleted} old files.',
	},
	skipped: {
		id: 'app.ymcl.landing.transfer.skipped',
		defaultMessage: '{count} files could not be updated. Their local copies were kept.',
	},
	pending: {
		id: 'app.ymcl.landing.transfer.pending',
		defaultMessage: '{count} files to update; {deleted} files to remove.',
	},
	waiting: {
		id: 'app.ymcl.landing.transfer.waiting',
		defaultMessage: 'Some required files need your attention',
	},
	queued: { id: 'app.downloads.status.queued', defaultMessage: 'Queued' },
	verifying: { id: 'app.downloads.phase.verifying', defaultMessage: 'Verifying' },
	java: { id: 'app.downloads.phase.preparing-java', defaultMessage: 'Preparing Java' },
	gameFiles: {
		id: 'app.downloads.phase.downloading-minecraft',
		defaultMessage: 'Downloading Minecraft',
	},
	content: { id: 'app.downloads.phase.downloading-content', defaultMessage: 'Downloading content' },
	downloads: { id: 'app.ymcl.landing.transfer.downloads', defaultMessage: 'Download details' },
	notes: { id: 'app.ymcl.landing.transfer.notes', defaultMessage: 'Update notes' },
	logs: { id: 'app.ymcl.landing.transfer.logs', defaultMessage: 'Game logs' },
	localFiles: { id: 'app.ymcl.landing.transfer.local-files', defaultMessage: 'Local files' },
	check: { id: 'app.ymcl.landing.transfer.check', defaultMessage: 'Check for updates' },
	addressMissing: {
		id: 'app.ymcl.landing.address-missing',
		defaultMessage: 'This server has no connection address.',
	},
	unavailable: {
		id: 'app.ymcl.landing.unavailable',
		defaultMessage: 'This server is currently unavailable.',
	},
	domainChanged: {
		id: 'app.ymcl.landing.transfer.domain-changed',
		defaultMessage: 'The active domain changed. Continue from the original domain.',
	},
})

const installed = computed(() => {
	if (preview.value?.installed) return preview.value.installed
	const local = server.value?.install
	return local
		? {
				instance_id: local.instanceId,
				pack_id: local.packId,
				version: local.currentVersion,
				up_to_date: !server.value?.hasUpdate,
			}
		: null
})
const matchingInstances = computed(() =>
	instances.value.filter(
		(instance) =>
			!preview.value?.required_version || instance.game_version === preview.value.required_version,
	),
)
const instanceOptions = computed(() => [
	...matchingInstances.value.map((instance) => ({
		value: `instance:${instance.id}`,
		label: `${instance.name} · ${instance.game_version}`,
	})),
	...(preview.value?.required_version
		? [
				{
					value: 'vanilla',
					label: formatMessage(messages.vanilla, { version: preview.value.required_version }),
				},
			]
		: []),
	...(preview.value?.optional_pack_id
		? [{ value: 'enhanced', label: formatMessage(messages.enhanced) }]
		: []),
])
const instanceId = computed(
	() =>
		installed.value?.instance_id ??
		(localChoice.value.startsWith('instance:') ? localChoice.value.slice(9) : ''),
)
const needsUpdate = computed(
	() =>
		!!installed.value &&
		(!installed.value.up_to_date ||
			!!outcome.value?.warning ||
			(updateCheck.value?.pending_changes ?? 0) > 0 ||
			(updateCheck.value?.pending_deletions ?? 0) > 0),
)
const busy = computed(() => working.value || checking.value)
const transfer = computed(() =>
	domainHomeTransferProgress(
		task.value?.running ? operation.value : null,
		manager.jobs.value,
		manager.legacyDownloads.value,
	),
)
const waiting = computed(() => transfer.value.job?.status === 'waiting_for_user')
const percentage = computed(() =>
	transfer.value.fraction === null ? null : Math.round(transfer.value.fraction * 100),
)
const action = computed(() => {
	if (preview.value?.has_pack)
		return !installed.value ? 'download' : needsUpdate.value ? 'update' : 'launch'
	if (localChoice.value === 'vanilla' || localChoice.value === 'enhanced') return 'download'
	return instanceId.value ? 'launch' : 'create'
})
const statusTitle = computed(() => {
	if (waiting.value) return messages.waiting
	if (launching.value) return messages.launching
	if (task.value?.running && operation.value) return messages[operation.value.stage]
	if (checking.value || loading.value) return messages.checking
	if (error.value) return messages.failed
	if (launched.value) return messages.launched
	if (preview.value && !preview.value.available) return messages.unavailable
	if (needsUpdate.value) return messages.updateAvailable
	if (action.value === 'download') return messages.available
	return instanceId.value ? messages.ready : messages.choose
})
const progressDetail = computed(() => {
	const job = transfer.value.job
	if (!job) return ''
	if (job.status === 'waiting_for_user') return formatMessage(messages.waiting)
	if (job.status === 'queued') return formatMessage(messages.queued)
	const text = installProgressTextSource(job)
	if (text.type === 'bytes') return `${formatBytes(text.current)} / ${formatBytes(text.total)}`
	if (text.type === 'items') return formatMessage(messages.files, text)
	if (job.phase === 'preparing_java') return formatMessage(messages.java)
	if (job.phase === 'downloading_minecraft') return formatMessage(messages.gameFiles)
	if (job.phase === 'downloading_content') return formatMessage(messages.content)
	if (job.phase === 'verifying' || job.phase === 'finalizing')
		return formatMessage(messages.verifying)
	return ''
})
const versionText = computed(() => {
	const current = outcome.value?.version || installed.value?.version
	const target = preview.value?.target_version
	if (current && target && current !== target)
		return formatMessage(messages.versions, { current, target })
	return current || target ? formatMessage(messages.version, { version: current || target }) : ''
})

async function inspect() {
	const selected = server.value
	if (!selected || props.preview) return
	const request = ++generation
	const domainId = store.activeDomainId
	const userId = store.session?.session.user_id
	checking.value = true
	try {
		const [next, local] = await Promise.all([ymcl.joinPreview(selected.serverId), list()])
		const localId = next.installed?.instance_id ?? selected.install?.instanceId
		const check = localId ? await ymcl.preLaunchCheck(localId) : null
		if (
			disposed ||
			request !== generation ||
			store.activeDomainId !== domainId ||
			store.session?.session.user_id !== userId
		)
			return
		preview.value = next
		updateCheck.value = check
		instances.value = local
		if (
			!local.some((instance) => `instance:${instance.id}` === localChoice.value) &&
			!(localChoice.value === 'vanilla' && next.required_version) &&
			!(localChoice.value === 'enhanced' && next.optional_pack_id)
		) {
			const matching = local.find(
				(instance) => !next.required_version || instance.game_version === next.required_version,
			)
			localChoice.value = matching
				? `instance:${matching.id}`
				: next.required_version
					? 'vanilla'
					: next.optional_pack_id
						? 'enhanced'
						: ''
		}
		requestError.value = ''
	} catch (cause) {
		if (!disposed && request === generation) requestError.value = ymclErrorMessage(cause)
	} finally {
		if (request === generation) checking.value = false
	}
}

function packEntry(enhanced = false): DomainPackEntry | null {
	const selected = server.value
	const packId = enhanced ? preview.value?.optional_pack_id : preview.value?.pack_id
	if (!selected || !packId) return null
	return {
		serverId: selected.serverId,
		serverName: selected.name,
		packId,
		targetVersion:
			(enhanced ? preview.value?.optional_target_version : preview.value?.target_version) ?? '',
		requiredVersion: preview.value?.required_version ?? '',
		seasonName: selected.seasonName ?? '',
	}
}

async function installVanilla(name: string, gameVersion: string, scope: DomainPackOperationScope) {
	const previous = operations.operationFor(scope)
	const previousJobId = previous?.progress.packId === '' ? previous.progress.jobId : undefined
	const handle = operations.begin(scope, { stage: 'installing', packId: '' })
	if (!handle) return null
	try {
		let job = previousJobId ? await install_job_get(previousJobId) : null
		if (job && ['failed', 'interrupted', 'canceled'].includes(job.status))
			job = await install_job_retry(job.job_id)
		if (!job)
			job = await install_create_instance({
				name: `${name} ${gameVersion}`,
				gameVersion,
				loader: 'vanilla',
				loaderVersion: null,
				adjuncts: [],
				iconPath: null,
			})
		operations.advance(handle, { stage: 'installing', packId: '', jobId: job.job_id })
		const finished = await wait_for_install_job(job.job_id)
		const instanceId = installJobInstanceId(finished)
		if (!instanceId) throw new Error(formatMessage(messages.failed))
		operations.finish(handle, { ok: true, instanceId, version: gameVersion })
		return instanceId
	} catch (cause) {
		operations.finish(handle, { ok: false, error: ymclErrorMessage(cause) })
		throw cause
	}
}

async function execute() {
	if (busy.value || props.preview || !server.value) return
	if (!preview.value || (error.value && !preview.value.available)) {
		await inspect()
		return
	}
	const domainId = store.activeDomainId
	const userId = store.session?.session.user_id
	const selectedId = server.value.serverId
	chosenServerId.value = selectedId
	const revision = contextRevision
	const intent = action.value
	const sameContext = () =>
		!disposed &&
		contextRevision === revision &&
		store.activeDomainId === domainId &&
		store.session?.session.user_id === userId &&
		server.value?.serverId === selectedId
	const requireContext = () => {
		if (!sameContext()) throw new Error(formatMessage(messages.domainChanged))
	}
	executing.value = true
	requestError.value = ''
	try {
		await inspect()
		requireContext()
		if (requestError.value) return
		if (!preview.value?.available)
			throw new Error(preview.value?.reason || formatMessage(messages.unavailable))
		if (intent === 'download' || intent === 'update' || needsUpdate.value) {
			if (intent === 'download' && installed.value) return
			if (!preview.value.has_pack && localChoice.value === 'vanilla') {
				const gameVersion = preview.value.required_version
				if (!gameVersion) throw new Error(formatMessage(messages.choose))
				const createdId = await installVanilla(server.value.name, gameVersion, operationScope.value)
				requireContext()
				if (!createdId) return
				localChoice.value = `instance:${createdId}`
			} else {
				const enhanced =
					!preview.value.has_pack &&
					(localChoice.value === 'enhanced' ||
						installed.value?.pack_id === preview.value.optional_pack_id)
				const entry = packEntry(enhanced)
				if (!entry) throw new Error(formatMessage(messages.unavailable))
				const options = {
					notify: false,
					instanceId: installed.value?.instance_id,
					selectedFeatures: (enhanced
						? (preview.value.optional_features ?? [])
						: preview.value.features
					)
						.filter((feature) => feature.default)
						.map((feature) => feature.id),
				}
				const result = installed.value
					? await update(entry, options)
					: await download(entry, options)
				requireContext()
				if (!result) throw new Error(formatMessage(messages.failed))
				if (!result.ok) throw new Error(result.error)
				if (enhanced) localChoice.value = `instance:${result.instanceId}`
			}
			await refresh()
			requireContext()
			await inspect()
			requireContext()
			if (
				intent !== 'launch' ||
				error.value ||
				outcome.value?.warning ||
				outcome.value?.update?.skipped_files.length
			)
				return
		}
		if (!instanceId.value) return
		launching.value = true
		const next = await ymcl.joinPreview(selectedId)
		const targetInstance = instanceId.value
		const running = await get_by_instance_id(targetInstance)
		const check = await ymcl.preLaunchCheck(targetInstance)
		requireContext()
		if (!next.available) throw new Error(next.reason || formatMessage(messages.unavailable))
		if ((check.pending_changes ?? 0) > 0 || (check.pending_deletions ?? 0) > 0) {
			updateCheck.value = check
			launching.value = false
			const entry = packEntry(!next.has_pack && check.pack_id === next.optional_pack_id)
			if (!entry) throw new Error(formatMessage(messages.unavailable))
			const result = await update(entry, { notify: false, instanceId: targetInstance })
			requireContext()
			if (!result) throw new Error(formatMessage(messages.failed))
			if (!result.ok) throw new Error(result.error)
			await refresh()
			requireContext()
			await inspect()
			requireContext()
			if (error.value || result.warning || result.update?.skipped_files.length) return
			launching.value = true
		}
		const address = next.launch_address || next.mc_address || server.value.primaryAddress
		if (!address) throw new Error(formatMessage(messages.addressMissing))
		if (!running.length) await run(targetInstance, address)
		if (sameContext()) launched.value = true
	} catch (cause) {
		if (sameContext()) requestError.value = ymclErrorMessage(cause)
	} finally {
		if (sameContext()) {
			executing.value = false
			launching.value = false
		}
	}
}

async function openLocalFiles() {
	const target = instanceId.value
	const revision = contextRevision
	if (!target) return
	fileError.value = ''
	try {
		await showInstanceInFolder(target)
	} catch (cause) {
		if (!disposed && revision === contextRevision && instanceId.value === target)
			fileError.value = ymclErrorMessage(cause)
	}
}

watch(
	[() => store.activeDomainId, () => server.value?.serverId, () => store.session?.session.user_id],
	() => {
		generation++
		contextRevision++
		preview.value = null
		updateCheck.value = null
		requestError.value = ''
		fileError.value = ''
		localChoice.value = ''
		launched.value = false
		executing.value = false
		launching.value = false
		checking.value = false
		void inspect()
	},
	{ immediate: true },
)
watch(
	() => task.value?.result,
	(result) => {
		if (!result) return
		if (result.ok) localChoice.value = `instance:${result.instanceId}`
		void inspect()
	},
	{ immediate: true },
)
watch(instanceId, () => {
	launched.value = false
	fileError.value = ''
})
watch(
	() => store.dataEpoch,
	() => {
		if (!working.value) void inspect()
	},
)
let unlistenProcess: (() => void) | undefined
void process_listener(() => {
	if (launched.value && !working.value && instanceId.value) {
		const target = instanceId.value
		const revision = contextRevision
		void get_by_instance_id(target)
			.then((processes) => {
				if (!disposed && revision === contextRevision && target === instanceId.value)
					launched.value = processes.length > 0
			})
			.catch(() => {})
	}
})
	.then((unlisten) => {
		if (disposed) unlisten()
		else unlistenProcess = unlisten
	})
	.catch(() => {})
onUnmounted(() => {
	disposed = true
	generation++
	contextRevision++
	unlistenProcess?.()
})
</script>

<template>
	<section class="min-w-0 text-white" :aria-label="formatMessage(messages.title)">
		<template v-if="server">
			<DomainHomeServerPicker
				:servers="servers"
				:selected-id="server.serverId"
				:disabled="working || props.preview"
				@select="chosenServerId = $event"
			/>

			<fieldset
				v-if="preview && !preview.has_pack && !installed && !busy"
				class="m-0 mt-4 min-w-0 border-0 p-0"
			>
				<legend class="mb-2 text-xs text-white/60">{{ formatMessage(messages.instance) }}</legend>
				<div class="flex gap-2 overflow-x-auto pb-1">
					<button
						v-for="option in instanceOptions"
						:key="option.value"
						type="button"
						:aria-pressed="localChoice === option.value"
						class="shrink-0 cursor-pointer rounded-lg border px-3 py-2 text-xs transition-colors active:bg-white/20"
						:class="
							localChoice === option.value
								? 'border-white/30 bg-white/15 text-white'
								: 'border-white/10 bg-black/15 text-white/60 hover:bg-white/10 hover:text-white'
						"
						:disabled="props.preview"
						@click="localChoice = option.value"
					>
						{{ option.label }}
					</button>
				</div>
			</fieldset>

			<div
				v-if="busy"
				class="mt-4 rounded-2xl border border-white/15 bg-black/30 p-5 backdrop-blur-xl"
			>
				<div class="flex items-center justify-between gap-4" role="status" aria-live="polite">
					<h2 class="m-0 flex min-w-0 items-center gap-2 text-lg font-semibold">
						<TriangleAlertIcon v-if="waiting" class="size-5 shrink-0 text-orange" />
						<SpinnerIcon
							v-else
							class="size-5 shrink-0 animate-spin text-white/60 motion-reduce:animate-none"
						/>
						{{ formatMessage(statusTitle) }}
					</h2>
					<span
						v-if="working && percentage !== null && !waiting && !launching"
						class="shrink-0 text-lg font-medium tabular-nums"
						>{{ percentage }}%</span
					>
				</div>
				<p v-if="versionText && operation" class="mb-0 mt-2 text-xs text-white/60">
					{{ versionText }}
				</p>
				<ProgressBar
					v-if="!waiting"
					class="mt-4"
					:progress="working ? (transfer.fraction ?? 0) : 0"
					:waiting="!working || transfer.fraction === null"
					:gradient-border="false"
					full-width
				/>
				<div
					v-if="progressDetail || operation"
					class="mt-3 flex flex-wrap items-center justify-between gap-3 text-xs text-white/60"
				>
					<span v-if="progressDetail && !waiting">{{ progressDetail }}</span>
					<RouterLink
						v-if="operation"
						:to="
							operation.jobId
								? { path: '/downloads', query: { job: operation.jobId } }
								: '/downloads'
						"
						class="ml-auto cursor-pointer text-white/80 hover:text-white hover:underline"
					>
						{{ formatMessage(messages.downloads) }} →
					</RouterLink>
				</div>
			</div>
			<template v-else>
				<div class="mt-5 flex flex-wrap items-center justify-between gap-x-5 gap-y-3">
					<div class="min-w-0 flex-1" role="status" aria-live="polite">
						<h2
							class="m-0 flex items-center gap-2 text-lg font-semibold"
							:class="error ? 'text-orange' : 'text-white'"
						>
							<TriangleAlertIcon v-if="error" class="size-5 shrink-0" />
							<span v-else-if="launched" class="size-2 shrink-0 rounded-full bg-green" />
							{{ formatMessage(statusTitle) }}
						</h2>
						<p v-if="versionText && !error && !launched" class="mb-0 mt-1.5 text-xs text-white/60">
							{{ versionText }}
						</p>
						<p
							v-else-if="action === 'download' && !error"
							class="mb-0 mt-1.5 text-xs text-white/60"
						>
							{{ formatMessage(messages.defaultFeatures) }}
						</p>
					</div>
					<ButtonStyled v-if="launched && instanceId" size="large" color="brand">
						<RouterLink :to="`/instance/${encodeURIComponent(instanceId)}/logs`"
							><TerminalSquareIcon />{{ formatMessage(messages.logs) }}</RouterLink
						>
					</ButtonStyled>
					<ButtonStyled
						v-else-if="action === 'create' && !error && preview?.available"
						size="large"
						color="brand"
					>
						<RouterLink to="/create">{{ formatMessage(messages.create) }}</RouterLink>
					</ButtonStyled>
					<ButtonStyled v-else size="large" color="brand">
						<button
							type="button"
							:disabled="(!error && (!preview || !preview.available)) || props.preview"
							@click="execute"
						>
							<RefreshCwIcon v-if="error || action === 'update'" /><DownloadIcon
								v-else-if="action === 'download'"
							/><PlayIcon v-else />
							{{ formatMessage(error ? messages.retry : messages[action]) }}
						</button>
					</ButtonStyled>
				</div>
				<p
					v-if="error"
					role="alert"
					class="mb-0 mt-3 break-words text-sm leading-relaxed text-orange"
				>
					{{ error }}
				</p>
				<p v-else-if="preview && !preview.available" class="mb-0 mt-3 text-sm text-white/60">
					{{ preview.reason || formatMessage(messages.unavailable) }}
				</p>
				<p v-if="outcome?.warning" class="mb-0 mt-3 text-xs leading-relaxed text-orange">
					{{ outcome.warning }}
				</p>
				<p v-if="outcome?.update?.skipped_files.length" class="mb-0 mt-3 text-xs text-orange">
					{{ formatMessage(messages.skipped, { count: outcome.update.skipped_files.length }) }}
				</p>
				<p v-else-if="outcome?.update" class="mb-0 mt-3 text-xs text-white/60">
					{{
						formatMessage(messages.changed, {
							count: outcome.update.staged_files,
							deleted: outcome.update.deleted_files,
						})
					}}
				</p>
				<p
					v-else-if="
						needsUpdate &&
						updateCheck &&
						((updateCheck.pending_changes ?? 0) > 0 || (updateCheck.pending_deletions ?? 0) > 0)
					"
					class="mb-0 mt-3 text-xs text-white/60"
				>
					{{
						formatMessage(messages.pending, {
							count: updateCheck.pending_changes ?? 0,
							deleted: updateCheck.pending_deletions ?? 0,
						})
					}}
				</p>
			</template>

			<details
				v-if="updateCheck?.notes?.trim() && (needsUpdate || working || outcome)"
				class="mt-3 text-xs text-white/60"
			>
				<summary class="cursor-pointer hover:text-white">
					{{ formatMessage(messages.notes) }}
				</summary>
				<p class="mb-0 max-h-32 overflow-y-auto whitespace-pre-line leading-relaxed">
					{{ updateCheck.notes.trim() }}
				</p>
			</details>
			<nav
				v-if="!busy && (instanceId || operation)"
				class="mt-3 flex flex-wrap items-center gap-x-4 gap-y-2 text-xs text-white/50"
			>
				<RouterLink
					v-if="operation"
					:to="
						operation.jobId ? { path: '/downloads', query: { job: operation.jobId } } : '/downloads'
					"
					class="cursor-pointer hover:text-white"
					>{{ formatMessage(messages.downloads) }}</RouterLink
				>
				<button
					v-if="instanceId"
					type="button"
					class="flex cursor-pointer items-center gap-1.5 border-0 bg-transparent p-0 font-[inherit] text-inherit hover:text-white"
					@click="openLocalFiles"
				>
					<FolderOpenIcon class="size-3.5" />{{ formatMessage(messages.localFiles) }}
				</button>
				<button
					v-if="instanceId && !launched"
					type="button"
					class="ml-auto flex cursor-pointer items-center gap-1.5 border-0 bg-transparent p-0 font-[inherit] text-inherit hover:text-white"
					:disabled="loading || props.preview"
					@click="
						async () => {
							await refresh()
							await inspect()
						}
					"
				>
					<RefreshCwIcon class="size-3.5" />{{ formatMessage(messages.check) }}
				</button>
			</nav>
			<p v-if="fileError" role="alert" class="mb-0 mt-2 text-xs text-orange">{{ fileError }}</p>
		</template>
		<p v-else-if="loading" class="m-0 flex items-center gap-2 text-sm text-white/60" role="status">
			<SpinnerIcon class="size-4 animate-spin motion-reduce:animate-none" />{{
				formatMessage(messages.checking)
			}}
		</p>
		<div v-else-if="loadError" class="flex items-center justify-between gap-4">
			<p class="m-0 text-sm text-white/60">{{ formatMessage(messages.serverError) }}</p>
			<ButtonStyled type="outlined"
				><button type="button" @click="refresh">
					<RefreshCwIcon />{{ formatMessage(messages.retry) }}
				</button></ButtonStyled
			>
		</div>
		<p v-else class="m-0 text-sm text-white/60">{{ formatMessage(messages.none) }}</p>
	</section>
</template>
