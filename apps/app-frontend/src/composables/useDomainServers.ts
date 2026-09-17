import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref, watch } from 'vue'

import { list } from '@/helpers/instance'
import { PERSONAL_DOMAIN_ID, ymcl, ymclErrorMessage } from '@/helpers/ymcl'
import { useYmclStore } from '@/store/ymcl'
import { useDomainPacks } from '@/composables/useDomainPacks'

/**
 * Launcher-side view of every domain server (MIP appendix B.2 + YAP servers
 * dataSource). Unifies what used to be the domain-packs card: servers are the
 * player-facing unit; pack install/update and local instances attach to the
 * server row instead of being a separate card.
 */

export interface DomainServerPlayer {
	id?: string
	name: string
}

export interface DomainServerEndpoint {
	address: string
	primary: boolean
	edition?: string
}

export interface DomainServerInstall {
	packId: string
	serverId: string
	instanceId: string
	instanceName: string
	currentVersion: string
}

export interface DomainServerRow {
	serverId: string
	name: string
	status: 'online' | 'offline' | 'unknown'
	primaryAddress?: string
	backupAddresses: string[]
	endpoints: DomainServerEndpoint[]
	seasonName?: string
	hasPack: boolean
	packId?: string
	targetVersion?: string
	requiredVersion?: string
	install?: DomainServerInstall
	hasUpdate: boolean
	description?: string
	motd?: string
	onlinePlayers?: number
	maxPlayers?: number
	players: DomainServerPlayer[]
	favicon?: string
	versionName?: string
	ping?: number
	available: boolean
	unavailableReason?: string
}

interface ServerOption {
	serverId: string
	name?: string | null
	status?: string | null
	mcAddress?: string | null
	endpoints?: Array<{
		address?: string | null
		primary?: boolean | null
		edition?: string | null
		name?: string | null
	}> | null
	currentSeason?: { name?: string | null } | null
	binding?: {
		packId?: string
		mcVersion?: string | null
	} | null
}

interface PackPreview {
	available: boolean
	reason?: string | null
	server_id?: string | null
	pack_id?: string | null
	target_version?: string | null
	required_version?: string | null
	season_name?: string | null
}

interface YapServerRecord {
	id?: unknown
	serverId?: unknown
	name?: unknown
	title?: unknown
	address?: unknown
	description?: unknown
	online?: unknown
	onlinePlayers?: unknown
	maxPlayers?: unknown
	players?: unknown
	ping?: unknown
	motd?: unknown
	favicon?: unknown
	versionName?: unknown
	endpoints?: unknown
	currentSeason?: { name?: unknown } | null
}

function asString(value: unknown): string | undefined {
	return typeof value === 'string' && value ? value : undefined
}

function asNumber(value: unknown): number | undefined {
	return typeof value === 'number' && Number.isFinite(value) ? value : undefined
}

function parseEndpoints(value: unknown): DomainServerEndpoint[] {
	if (!Array.isArray(value)) return []
	const endpoints: DomainServerEndpoint[] = []
	for (const item of value) {
		if (!item || typeof item !== 'object') continue
		const record = item as Record<string, unknown>
		const address = asString(record.address)
		if (!address) continue
		endpoints.push({
			address,
			primary: record.primary === true || record.primaryLine === true,
			...(asString(record.edition) ? { edition: asString(record.edition) } : {}),
		})
	}
	return endpoints
}

function parsePlayers(value: unknown): DomainServerPlayer[] {
	if (!Array.isArray(value)) return []
	const players: DomainServerPlayer[] = []
	for (const item of value) {
		if (typeof item === 'string' && item) {
			players.push({ name: item })
			continue
		}
		if (!item || typeof item !== 'object') continue
		const record = item as Record<string, unknown>
		const name = asString(record.name)
		if (!name) continue
		players.push({ ...(asString(record.id) ? { id: asString(record.id) } : {}), name })
	}
	return players
}

function normalizeStatus(value: unknown): DomainServerRow['status'] {
	if (typeof value === 'boolean') return value ? 'online' : 'offline'
	if (typeof value === 'string') {
		const lower = value.toLowerCase()
		if (lower === 'online' || lower === 'true') return 'online'
		if (lower === 'offline' || lower === 'false') return 'offline'
	}
	return 'unknown'
}

function formatAddress(host: string, port?: number | string | null): string {
	if (!host) return ''
	const parsed = typeof port === 'string' ? Number(port) : port
	if (parsed == null || !Number.isFinite(parsed) || parsed === 25565) return host
	return `${host}:${parsed}`
}

function matchYapRecord(
	server: ServerOption,
	records: YapServerRecord[],
): YapServerRecord | undefined {
	const address = server.mcAddress?.trim().toLowerCase()
	return records.find((record) => {
		const id = asString(record.id) ?? asString(record.serverId)
		if (id && id === server.serverId) return true
		const recordAddress = asString(record.address)?.trim().toLowerCase()
		if (address && recordAddress && recordAddress === address) return true
		return false
	})
}

export function useDomainServers() {
	const packsApi = useDomainPacks()
	const ymclStore = useYmclStore()

	const loading = ref(false)
	const loadError = ref('')
	const servers = ref<DomainServerRow[]>([])

	const hasDomainFace = computed(() => ymclStore.activeDomainId !== PERSONAL_DOMAIN_ID)

	function installFor(serverId: string): DomainServerInstall | undefined {
		const row = servers.value.find((item) => item.serverId === serverId)
		return row?.install
	}

	function hasUpdateFor(serverId: string): boolean {
		return servers.value.find((item) => item.serverId === serverId)?.hasUpdate ?? false
	}

	async function loadYapEnrichment(): Promise<YapServerRecord[]> {
		const manifest = ymclStore.manifest
		if (!manifest || ymclStore.isPersonal) return []
		const source = (manifest.data_sources ?? []).find((candidate) => {
			const sourceCode = candidate.sourceCode ?? candidate.source_code
			return sourceCode === 'servers' || sourceCode === 'servers.list'
		})
		if (!source) return []
		const providerCode = source.providerCode ?? source.provider_code
		const sourceCode = source.sourceCode ?? source.source_code
		if (!providerCode || !sourceCode) return []
		try {
			const envelope = await invoke<{ records?: unknown }>('plugin:ymcl|ymcl_data_fetch', {
				providerCode,
				sourceCode,
				query: null,
			})
			const records = envelope?.records
			if (!Array.isArray(records)) return []
			return records.filter(
				(record): record is YapServerRecord => !!record && typeof record === 'object',
			)
		} catch {
			// Enrichment is optional: MIP list alone still renders the card.
			return []
		}
	}

	async function loadPackPreviews(
		bound: ServerOption[],
	): Promise<Map<string, PackPreview>> {
		const previews = await Promise.all(
			bound.map(async (server) => {
				try {
					const preview = await invoke<PackPreview>('plugin:ymcl|ymcl_join_preview', {
						serverId: server.serverId,
					})
					return [server.serverId, preview] as const
				} catch {
					return [server.serverId, null] as const
				}
			}),
		)
		return new Map(previews.filter((entry): entry is readonly [string, PackPreview] => !!entry[1]))
	}

	async function loadInstalls(): Promise<Map<string, DomainServerInstall>> {
		const byPack = packsApi.installsByPack.value
		const byServer = new Map<string, DomainServerInstall>()
		for (const installs of byPack.values()) {
			for (const install of installs) {
				if (install.serverId && !byServer.has(install.serverId)) {
					byServer.set(install.serverId, install)
				}
			}
		}
		// Pack-id matching is the primary identity; also scan local instances when
		// the shared pack map is empty (first paint / personal cache miss).
		if (byServer.size === 0) {
			const instances = await list().catch(() => [])
			for (const instance of instances) {
				try {
					const check = await ymcl.preLaunchCheck(instance.id)
					if (!check.managed || !check.server_id) continue
					if (byServer.has(check.server_id)) continue
					byServer.set(check.server_id, {
						packId: check.pack_id ?? '',
						serverId: check.server_id,
						instanceId: instance.id,
						instanceName: instance.name,
						currentVersion: check.current_version ?? '',
					})
				} catch {
					// Unmanaged or unreadable instance: skip.
				}
			}
		}
		return byServer
	}

	function applyYapRecord(row: DomainServerRow, record: YapServerRecord | undefined) {
		if (!record) return
		const description = asString(record.description)
		if (description) row.description = description
		const motd = asString(record.motd)
		if (motd) row.motd = motd
		const favicon = asString(record.favicon)
		if (favicon) row.favicon = favicon
		const versionName = asString(record.versionName)
		if (versionName) row.versionName = versionName
		const ping = asNumber(record.ping)
		if (ping != null) row.ping = ping
		const onlinePlayers = asNumber(record.onlinePlayers) ?? asNumber(record.players)
		if (onlinePlayers != null) row.onlinePlayers = onlinePlayers
		const maxPlayers = asNumber(record.maxPlayers)
		if (maxPlayers != null) row.maxPlayers = maxPlayers
		const players = parsePlayers(record.players)
		if (players.length > 0) row.players = players
		if (record.online != null) row.status = normalizeStatus(record.online)
		const seasonName = asString(record.currentSeason?.name)
		if (seasonName) row.seasonName = seasonName
		const endpoints = parseEndpoints(record.endpoints)
		if (endpoints.length > 0) {
			row.endpoints = endpoints
			const primary = endpoints.find((endpoint) => endpoint.primary) ?? endpoints[0]
			if (primary?.address) row.primaryAddress = primary.address
			row.backupAddresses = endpoints
				.filter((endpoint) => !endpoint.primary && endpoint.address !== row.primaryAddress)
				.map((endpoint) => endpoint.address)
		}
	}

	function applyMipEndpoints(row: DomainServerRow, server: ServerOption) {
		const endpoints: DomainServerEndpoint[] = []
		for (const item of server.endpoints ?? []) {
			const address = item.address?.trim()
			if (!address) continue
			endpoints.push({
				address,
				primary: item.primary === true,
				...(item.edition ? { edition: item.edition } : {}),
			})
		}
		if (endpoints.length === 0) {
			if (row.primaryAddress) {
				row.endpoints = [{ address: row.primaryAddress, primary: true }]
			}
			return
		}
		endpoints.sort((left, right) => Number(right.primary) - Number(left.primary))
		const primary = endpoints.find((endpoint) => endpoint.primary) ?? endpoints[0]
		if (primary?.address) row.primaryAddress = primary.address
		row.endpoints = endpoints
		row.backupAddresses = endpoints
			.filter((endpoint) => !endpoint.primary && endpoint.address !== row.primaryAddress)
			.map((endpoint) => endpoint.address)
	}

	async function refresh() {
		if (!hasDomainFace.value) {
			servers.value = []
			loadError.value = ''
			return
		}
		loading.value = true
		loadError.value = ''
		try {
			// Keep the shared pack catalog warm so install identity matches the library.
			await packsApi.refresh()
			const rawServers = await invoke<ServerOption[]>('plugin:ymcl|ymcl_mip_servers')
			const bound = rawServers.filter((server) => !!server.binding?.packId)
			const [previews, installs, yapRecords] = await Promise.all([
				loadPackPreviews(bound),
				loadInstalls(),
				loadYapEnrichment(),
			])

			servers.value = rawServers.map((server): DomainServerRow => {
				const name = server.name || server.serverId
				const primaryAddress = server.mcAddress || undefined
				const preview = previews.get(server.serverId)
				const packId = server.binding?.packId || preview?.pack_id || undefined
				const targetVersion = preview?.target_version ?? undefined
				const requiredVersion =
					preview?.required_version ?? server.binding?.mcVersion ?? undefined
				const install = installs.get(server.serverId)
				const hasPack = !!packId
				const hasUpdate =
					!!install &&
					hasPack &&
					(install.packId !== packId ||
						(!!targetVersion && install.currentVersion !== targetVersion))

				const row: DomainServerRow = {
					serverId: server.serverId,
					name,
					status: normalizeStatus(server.status),
					...(primaryAddress ? { primaryAddress } : {}),
					backupAddresses: [],
					endpoints: primaryAddress
						? [{ address: primaryAddress, primary: true }]
						: [],
					...(server.currentSeason?.name ? { seasonName: server.currentSeason.name } : {}),
					hasPack,
					...(packId ? { packId } : {}),
					...(targetVersion ? { targetVersion } : {}),
					...(requiredVersion ? { requiredVersion } : {}),
					...(install ? { install } : {}),
					hasUpdate,
					players: [],
					available: preview ? preview.available : true,
					...(preview?.reason ? { unavailableReason: preview.reason } : {}),
				}

				applyMipEndpoints(row, server)

				// Bound packs need a successful preview to expose a resolvable version;
				// unbound servers still list from the MIP face alone.
				if (bound.some((item) => item.serverId === server.serverId) && preview && !preview.available) {
					row.available = false
					row.unavailableReason = preview.reason ?? undefined
				}

				applyYapRecord(row, matchYapRecord(server, yapRecords))
				return row
			})
		} catch (error) {
			servers.value = []
			loadError.value = ymclErrorMessage(error)
		} finally {
			loading.value = false
		}
	}

	watch(
		() => ymclStore.activeDomainId,
		() => void refresh(),
	)
	watch(
		() => ymclStore.dataEpoch,
		() => void refresh(),
	)
	// Do not watch packsApi.installsByPack: refresh() already awaits packsApi.refresh()
	// and would otherwise re-enter through this watcher.

	onMounted(() => void refresh())

	return {
		loading,
		loadError,
		servers,
		hasDomainFace,
		installFor,
		hasUpdateFor,
		refresh,
		download: packsApi.download,
		update: packsApi.update,
		downloadingServer: packsApi.downloadingServer,
		updatingInstance: packsApi.updatingInstance,
		/** Pack-oriented entry shape for download/update actions. */
		packEntryFor(row: DomainServerRow) {
			if (!row.hasPack || !row.packId) return null
			return {
				serverId: row.serverId,
				serverName: row.name,
				packId: row.packId,
				targetVersion: row.targetVersion ?? '',
				requiredVersion: row.requiredVersion ?? '',
				seasonName: row.seasonName ?? '',
			}
		},
	}
}
