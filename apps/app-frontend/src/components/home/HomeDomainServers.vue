<script setup lang="ts">
import {
	ChevronRightIcon,
	ClipboardCopyIcon,
	DownloadIcon,
	GlobeIcon,
	LoaderIcon,
	PackageIcon,
	PlayIcon,
	ServerIcon,
	UpdatedIcon,
	UserIcon,
} from '@modrinth/assets'
import {
	Avatar,
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'

import type { HomeWidgetSize } from '@/components/home/home-dashboard'
import JoinServerModal from '@/components/ymcl/JoinServerModal.vue'
import { useHomeDashboardRuntime } from '@/components/home/home-dashboard-runtime'
import type { DomainServerRow } from '@/composables/useDomainServers'
import { useDomainServers } from '@/composables/useDomainServers'

/**
 * Launcher-built domain servers home card. Servers are the hard dependency
 * (packs bind to servers); this unifies the old domain-packs card into a
 * server-first list with address, backup lines, online players, pack/instance
 * status and join/download actions.
 */

const props = defineProps<{
	dashboardSize?: HomeWidgetSize | null
}>()

const { formatMessage } = useVIntl()
const { addNotification, handleError } = injectNotificationManager()
const router = useRouter()
const runtime = useHomeDashboardRuntime()
const joinModal = ref<InstanceType<typeof JoinServerModal> | null>(null)

const {
	loading,
	loadError,
	servers,
	hasDomainFace,
	download,
	update,
	downloadingServer,
	updatingInstance,
	packEntryFor,
} = useDomainServers()

const messages = defineMessages({
	title: {
		id: 'app.home.servers.domain-title',
		defaultMessage: '服务器',
	},
	empty: {
		id: 'app.home.servers.domain-empty',
		defaultMessage: '当前域没有可加入的服务器。',
	},
	loadFailed: {
		id: 'app.home.servers.domain-load-failed',
		defaultMessage: '无法加载域服务器列表。',
	},
	playersOnline: {
		id: 'app.home.servers.players-online',
		defaultMessage: '{online}/{max} online',
	},
	offline: {
		id: 'app.home.servers.offline',
		defaultMessage: 'Offline',
	},
	checking: {
		id: 'app.home.servers.checking',
		defaultMessage: 'Checking…',
	},
	season: {
		id: 'app.ymcl.renderer.season',
		defaultMessage: 'Season',
	},
	backupAddresses: {
		id: 'app.home.servers.backup-addresses',
		defaultMessage: 'Backup',
	},
	players: {
		id: 'app.home.servers.players',
		defaultMessage: 'Players',
	},
	join: {
		id: 'app.ymcl.join.title',
		defaultMessage: 'Join server',
	},
	download: {
		id: 'app.ymcl.packs.download',
		defaultMessage: 'Download pack',
	},
	downloading: {
		id: 'app.ymcl.packs.downloading',
		defaultMessage: 'Downloading…',
	},
	updating: {
		id: 'app.ymcl.manage.updating',
		defaultMessage: 'Updating…',
	},
	updateNow: {
		id: 'app.ymcl.packs.update-now',
		defaultMessage: 'Update pack',
	},
	openInstance: {
		id: 'app.ymcl.packs.open-instance',
		defaultMessage: 'Open instance',
	},
	installedOn: {
		id: 'app.ymcl.packs.installed-on',
		defaultMessage: 'Installed on “{name}”',
	},
	updateAvailable: {
		id: 'app.ymcl.packs.update-available',
		defaultMessage: 'Update available to {version}',
	},
	copyAddress: {
		id: 'app.ymcl.renderer.copy-address',
		defaultMessage: 'Copy address',
	},
	copied: {
		id: 'app.home.servers.copied-address',
		defaultMessage: 'Address copied',
	},
	noPack: {
		id: 'app.home.servers.no-pack',
		defaultMessage: 'No modpack bound',
	},
	mcVersion: {
		id: 'app.ymcl.packs.mc-version',
		defaultMessage: 'MC {version}',
	},
})

const isCompact = computed(
	() => props.dashboardSize === '1x1' || props.dashboardSize === '1x2' || props.dashboardSize === '2x1',
)

const visibleServers = computed(() => (isCompact.value ? servers.value.slice(0, 4) : servers.value))

function serverKey(row: DomainServerRow): string {
	return row.serverId
}

function onlinePlayers(row: DomainServerRow): number | null {
	if (row.status === 'offline') return null
	if (row.onlinePlayers != null) return row.onlinePlayers
	return null
}

function maxPlayers(row: DomainServerRow): number {
	return row.maxPlayers != null && row.maxPlayers > 0 ? row.maxPlayers : 0
}

function playerSample(row: DomainServerRow): string[] {
	return row.players.map((player) => player.name).filter(Boolean)
}

async function copyAddress(row: DomainServerRow) {
	const address = row.primaryAddress
	if (!address) return
	try {
		await navigator.clipboard.writeText(address)
		addNotification({
			title: formatMessage(messages.copied),
			text: address,
			type: 'success',
		})
	} catch {
		// Clipboard may be unavailable; ignore silently.
	}
}

function join(row: DomainServerRow) {
	void joinModal.value?.show({ serverId: row.serverId, address: row.primaryAddress })
}

function openInstance(row: DomainServerRow) {
	if (row.install) {
		router.push(`/instance/${encodeURIComponent(row.install.instanceId)}`)
	}
}

async function downloadPack(row: DomainServerRow) {
	const entry = packEntryFor(row)
	if (!entry) return
	try {
		await download(entry)
	} catch (error) {
		handleError(error)
	}
}

async function updatePack(row: DomainServerRow) {
	const entry = packEntryFor(row)
	if (!entry) return
	try {
		await update(entry)
	} catch (error) {
		handleError(error)
	}
}

function warmPings() {
	for (const row of servers.value) {
		if (!row.primaryAddress) continue
		// Domain rows may not have a local instance yet; serverId is a stable
		// cache key and protocol lookup degrades to null for non-instances.
		void runtime.refreshServer(row.serverId, row.primaryAddress).catch(() => undefined)
	}
}

function liveData(row: DomainServerRow) {
	if (!row.primaryAddress) return null
	return runtime.getServerData(row.serverId, row.primaryAddress)
}

function liveOnline(row: DomainServerRow): { online: number; max: number } | null {
	const fromYap = onlinePlayers(row)
	const max = maxPlayers(row)
	if (fromYap != null) {
		return { online: fromYap, max: max > 0 ? max : 0 }
	}
	const data = liveData(row)
	const status = data?.status
	if (!status?.players) return null
	return { online: status.players.online, max: status.players.max }
}

function livePlayers(row: DomainServerRow): string[] {
	const names = playerSample(row)
	if (names.length > 0) return names
	const sample = liveData(row)?.status?.players?.sample ?? []
	return sample.map((player) => player.name).filter(Boolean)
}

function liveFavicon(row: DomainServerRow): string | undefined {
	return row.favicon || liveData(row)?.status?.favicon || undefined
}

function liveStatus(row: DomainServerRow): 'online' | 'offline' | 'checking' | 'unknown' {
	if (row.status === 'online') return 'online'
	const data = liveData(row)
	if (data?.refreshing) return 'checking'
	if (data?.status) return 'online'
	if (row.status === 'offline' || (data && !data.status && !data.refreshing)) return 'offline'
	return row.status === 'unknown' ? 'unknown' : row.status
}

function liveVersion(row: DomainServerRow): string | undefined {
	return row.versionName || liveData(row)?.status?.version?.name || undefined
}

function subtitleParts(row: DomainServerRow): string[] {
	const parts: string[] = []
	if (row.primaryAddress) parts.push(row.primaryAddress)
	const online = liveOnline(row)
	if (online) {
		parts.push(
			formatMessage(messages.playersOnline, {
				online: String(online.online),
				max: String(online.max > 0 ? online.max : '?'),
			}),
		)
	} else if (liveStatus(row) === 'offline') {
		parts.push(formatMessage(messages.offline))
	} else if (liveStatus(row) === 'checking') {
		parts.push(formatMessage(messages.checking))
	}
	if (row.seasonName) parts.push(`${formatMessage(messages.season)}: ${row.seasonName}`)
	const version = liveVersion(row)
	if (version) parts.push(version)
	return parts
}

onMounted(() => {
	warmPings()
})

watch(
	() => servers.value.map((row) => `${row.serverId}:${row.primaryAddress ?? ''}`).join(','),
	() => warmPings(),
)
</script>

<template>
	<section
		v-if="hasDomainFace"
		class="home-domain-servers flex min-w-0 min-h-0 h-full flex-col gap-2"
		:data-size="props.dashboardSize"
	>
		<div class="home-widget-heading flex min-w-0 h-8 flex-none items-center gap-2">
			<ServerIcon class="size-5 shrink-0 text-brand" aria-hidden="true" />
			<h2>{{ formatMessage(messages.title) }}</h2>
		</div>

		<div v-if="loading" class="home-widget-empty">
			<LoaderIcon class="animate-spin" aria-hidden="true" />
		</div>
		<div v-else-if="loadError" class="home-widget-empty">
			<ServerIcon aria-hidden="true" />
			<span>{{ formatMessage(messages.loadFailed) }}</span>
			<span class="text-xs opacity-80">{{ loadError }}</span>
		</div>
		<div v-else-if="servers.length === 0" class="home-widget-empty">
			<ServerIcon aria-hidden="true" />
			<span>{{ formatMessage(messages.empty) }}</span>
		</div>
		<ul
			v-else
			class="home-server-list m-0 flex min-w-0 min-h-0 flex-1 list-none flex-col gap-1 overflow-x-hidden overflow-y-auto p-0 pr-1"
		>
			<li
				v-for="row in visibleServers"
				:key="serverKey(row)"
				class="home-server-row group hover:bg-button-bg"
			>
				<div class="relative shrink-0">
					<Avatar
						:src="liveFavicon(row) || undefined"
						:tint-by="row.primaryAddress || row.serverId"
						size="36px"
					/>
					<span
						class="absolute -bottom-0.5 -right-0.5 size-2.5 rounded-full border-2 border-solid border-bg-raised"
						:class="
							liveStatus(row) === 'checking'
								? 'animate-pulse bg-secondary'
								: liveStatus(row) === 'online'
									? 'bg-brand-green'
									: liveStatus(row) === 'offline'
										? 'bg-red'
										: 'bg-secondary'
						"
						aria-hidden="true"
					/>
				</div>

				<div class="flex min-w-0 flex-1 flex-col gap-0.5">
					<div class="flex min-w-0 items-center gap-1.5">
						<span class="truncate text-sm font-semibold text-contrast">
							{{ row.name }}
						</span>
						<span
							v-if="row.hasPack"
							class="inline-flex shrink-0 items-center gap-0.5 rounded-full bg-button-bg px-1.5 py-0.5 text-[0.625rem] font-semibold text-secondary"
						>
							<PackageIcon class="size-2.5" aria-hidden="true" />
							<span v-if="row.targetVersion">v{{ row.targetVersion }}</span>
						</span>
					</div>

					<span
						v-if="subtitleParts(row).length"
						class="truncate text-xs text-secondary"
						:title="subtitleParts(row).join(' · ')"
					>
						{{ subtitleParts(row).join(' · ') }}
					</span>

					<span
						v-if="row.backupAddresses.length"
						class="flex min-w-0 items-center gap-1 text-[0.6875rem] text-secondary opacity-80"
					>
						<GlobeIcon class="size-3 shrink-0" aria-hidden="true" />
						<span class="truncate">
							{{ formatMessage(messages.backupAddresses) }}: {{ row.backupAddresses.join(', ') }}
						</span>
					</span>

					<span
						v-if="livePlayers(row).length"
						class="flex min-w-0 items-center gap-1 text-[0.6875rem] text-secondary"
						:title="livePlayers(row).join(', ')"
					>
						<UserIcon class="size-3 shrink-0" aria-hidden="true" />
						<span class="truncate">{{ livePlayers(row).join(', ') }}</span>
					</span>

					<span
						v-if="row.install"
						class="truncate text-xs"
						:class="row.hasUpdate ? 'text-orange' : 'text-secondary'"
					>
						<template v-if="row.hasUpdate">
							{{ formatMessage(messages.updateAvailable, { version: row.targetVersion || '?' }) }}
						</template>
						{{ formatMessage(messages.installedOn, { name: row.install.instanceName }) }}
					</span>
					<span v-else-if="row.hasPack" class="truncate text-xs text-secondary">
						<template v-if="row.targetVersion">v{{ row.targetVersion }}</template>
						<template v-if="row.requiredVersion">
							· {{ formatMessage(messages.mcVersion, { version: row.requiredVersion }) }}
						</template>
					</span>
					<span v-else-if="row.requiredVersion" class="truncate text-xs text-secondary">
						{{ formatMessage(messages.mcVersion, { version: row.requiredVersion }) }}
					</span>
					<span v-else class="truncate text-xs text-secondary">
						{{ formatMessage(messages.noPack) }}
					</span>
				</div>

				<div class="ml-auto flex shrink-0 items-center gap-0.5">
					<ButtonStyled
						v-if="row.primaryAddress"
						circular
						size="small"
						type="transparent"
					>
						<button
							v-tooltip="formatMessage(messages.copyAddress)"
							class="opacity-60 transition-opacity group-hover:opacity-100"
							@click="copyAddress(row)"
						>
							<ClipboardCopyIcon />
						</button>
					</ButtonStyled>

					<ButtonStyled
						v-if="!row.install && row.hasPack"
						circular
						size="small"
						type="transparent"
					>
						<button
							v-tooltip="
								formatMessage(
									downloadingServer === row.serverId ? messages.downloading : messages.download,
								)
							"
							:disabled="!!downloadingServer"
							class="!text-brand opacity-60 transition-opacity group-hover:opacity-100"
							@click="downloadPack(row)"
						>
							<LoaderIcon v-if="downloadingServer === row.serverId" class="animate-spin" />
							<DownloadIcon v-else />
						</button>
					</ButtonStyled>

					<ButtonStyled
						v-else-if="row.install && row.hasUpdate"
						circular
						size="small"
						type="transparent"
					>
						<button
							v-tooltip="
								formatMessage(
									updatingInstance === row.install?.instanceId
										? messages.updating
										: messages.updateNow,
								)
							"
							:disabled="!!updatingInstance"
							class="!text-brand opacity-60 transition-opacity group-hover:opacity-100"
							@click="updatePack(row)"
						>
							<LoaderIcon
								v-if="updatingInstance === row.install?.instanceId"
								class="animate-spin"
							/>
							<UpdatedIcon v-else />
						</button>
					</ButtonStyled>

					<ButtonStyled v-else-if="row.install" circular size="small" type="transparent">
						<button
							v-tooltip="formatMessage(messages.openInstance)"
							class="opacity-60 transition-opacity group-hover:opacity-100"
							@click="openInstance(row)"
						>
							<ChevronRightIcon />
						</button>
					</ButtonStyled>

					<ButtonStyled circular size="small" type="transparent">
						<button
							v-tooltip="formatMessage(messages.join)"
							class="!text-brand opacity-60 transition-opacity group-hover:opacity-100"
							@click="join(row)"
						>
							<PlayIcon />
						</button>
					</ButtonStyled>
				</div>
			</li>
		</ul>
	</section>
	<JoinServerModal ref="joinModal" />
</template>

<style scoped>
.home-widget-heading h2 {
	min-width: 0;
	overflow: hidden;
	margin: 0;
	color: var(--color-contrast);
	font-size: 1rem;
	font-weight: 700;
	letter-spacing: 0;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.home-server-row {
	display: flex;
	min-width: 0;
	align-items: center;
	gap: 0.625rem;
	padding: 0.5rem;
	border-radius: 6px;
	transition: background-color 120ms ease;
}

.home-domain-servers[data-size='2x1'] .home-server-list,
.home-domain-servers[data-size='2x2'] .home-server-list {
	grid-template-columns: none;
}

.home-domain-servers[data-size='1x1'] {
	gap: 0.375rem;
}

.home-domain-servers[data-size='1x1'] .home-widget-heading {
	height: 1.5rem;
}

.home-domain-servers[data-size='1x1'] .home-server-row {
	gap: 0.5rem;
	padding: 0.375rem;
}

.home-widget-empty {
	display: flex;
	max-width: 20rem;
	margin: auto;
	flex-direction: column;
	align-items: center;
	gap: 0.5rem;
	color: var(--color-secondary);
	font-size: 0.8125rem;
	line-height: 1.4;
	text-align: center;
}

.home-widget-empty svg {
	width: 1.5rem;
	height: 1.5rem;
	opacity: 0.7;
}
</style>
