import { defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref, watch } from 'vue'

import {
	install_create_modpack_instance,
	install_pack_to_existing_instance,
	wait_for_install_job,
} from '@/helpers/install'
import { list } from '@/helpers/instance'
import {
	catchUpPackUpdate,
	PERSONAL_DOMAIN_ID,
	ymcl,
	ymclErrorMessage,
	type YmclMrpackDownload,
} from '@/helpers/ymcl'
import { useYmclStore } from '@/store/ymcl'

/**
 * Shared player-side view of the domain's published modpacks (MIP appendix
 * B): browse what the domain ships, download a pack as a brand-new local
 * instance, update an installed one. Backs the library panel, the home
 * widget and the create-page section. Hidden outside a domain or when the
 * domain has no MIP face.
 */

export interface DomainPackEntry {
	serverId: string
	serverName: string
	packId: string
	targetVersion: string
	requiredVersion: string
	seasonName: string
}

export interface DomainPackInstall {
	packId: string
	serverId: string
	instanceId: string
	instanceName: string
	currentVersion: string
}

interface ServerOption {
	serverId: string
	name?: string | null
	binding?: { packId?: string } | null
}

interface PackPreview {
	available: boolean
	has_pack: boolean
	reason?: string | null
	server_id?: string | null
	server_name?: string | null
	pack_id?: string | null
	target_version?: string | null
	required_version?: string | null
	season_name?: string | null
}

/** 每个已绑定服务器的解析结果（面板空态时逐行展示，方便定位）。 */
export interface DomainPackServerDiagnostic {
	serverName: string
	detail: string
	ok: boolean
}

export function useDomainPacks() {
	const { addNotification } = injectNotificationManager()
	const { formatMessage } = useVIntl()
	const ymclStore = useYmclStore()

	const messages = defineMessages({
		downloaded: {
			id: 'app.ymcl.packs.downloaded',
			defaultMessage: '整合包已下载，实例「{name}」已创建。',
		},
		updated: {
			id: 'app.ymcl.packs.updated',
			defaultMessage: '整合包已更新到 {version}。',
		},
		installFailed: {
			id: 'app.ymcl.install.install-failed',
			defaultMessage: '安装失败',
		},
		updateFailed: {
			id: 'app.ymcl.packs.update-failed',
			defaultMessage: '更新失败',
		},
		catchupFailed: {
			id: 'app.ymcl.packs.catchup-failed',
			defaultMessage:
				'已安装基线版本 {version}，增量更新到 {target} 失败；启动时会自动重试。',
		},
	})

	/** 基线安装后的增量追平；失败只提示，不推翻已完成的安装。 */
	function catchUpAfterBaseline(
		instanceId: string,
		archive: YmclMrpackDownload,
	): Promise<string> {
		return catchUpPackUpdate(instanceId, archive, (error) => {
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

	const loading = ref(false)
	const packs = ref<DomainPackEntry[]>([])
	const installsByPack = ref<Map<string, DomainPackInstall[]>>(new Map())
	const downloadingServer = ref('')
	const updatingInstance = ref('')
	/** 面板空态的原因，让"看不到包"可自诊断而不是整块消失。 */
	const emptyReason = ref<'none' | 'load-failed' | 'no-bound' | 'no-pack'>('none')
	const loadError = ref('')
	/** 逐服务器的解析诊断（空态时展示，定位绑定/包状态）。 */
	const serverDiagnostics = ref<DomainPackServerDiagnostic[]>([])

	const hasDomainFace = computed(() => ymclStore.activeDomainId !== PERSONAL_DOMAIN_ID)

	/** 同一服务器重传包会换新 packId，安装身份按 packId 优先、serverId 兜底。 */
	function installFor(pack: DomainPackEntry): DomainPackInstall | undefined {
		const byPack = installsByPack.value.get(pack.packId)?.[0]
		if (byPack) return byPack
		for (const installs of installsByPack.value.values()) {
			const match = installs.find(
				(install) => install.serverId && install.serverId === pack.serverId,
			)
			if (match) return match
		}
		return undefined
	}

	function hasUpdate(pack: DomainPackEntry): boolean {
		const install = installFor(pack)
		if (!install) return false
		// 服务器删包重传 → packId 变化，也视为需要更新（重装为当前绑定包）。
		return (
			install.packId !== pack.packId ||
			(!!pack.targetVersion && install.currentVersion !== pack.targetVersion)
		)
	}

	async function refresh() {
		if (!hasDomainFace.value) {
			packs.value = []
			installsByPack.value = new Map()
			serverDiagnostics.value = []
			emptyReason.value = 'none'
			return
		}
		loading.value = true
		emptyReason.value = 'none'
		loadError.value = ''
		try {
			const servers = await invoke<ServerOption[]>('plugin:ymcl|ymcl_mip_servers')
			const bound = servers.filter((server) => server.binding?.packId)
			if (bound.length === 0) {
				packs.value = []
				serverDiagnostics.value = []
				emptyReason.value = 'no-bound'
				return
			}
			const previews = await Promise.all(
				bound.map(
					async (server): Promise<{
						diagnostic: DomainPackServerDiagnostic
						entry: DomainPackEntry | null
					}> => {
						const serverName = server.name || server.serverId
						try {
							const preview = await invoke<PackPreview>('plugin:ymcl|ymcl_join_preview', {
								serverId: server.serverId,
							})
							if (!preview.available) {
								return {
									diagnostic: {
										serverName,
										detail: preview.reason ?? '暂时不可用',
										ok: false,
									},
									entry: null,
								}
							}
							if (!preview.has_pack || !preview.pack_id) {
								return {
									diagnostic: {
										serverName,
										detail: '该服务器未绑定整合包',
										ok: false,
									},
									entry: null,
								}
							}
							return {
								diagnostic: {
									serverName,
									detail: `已绑定 ${preview.pack_id} → ${preview.target_version ?? '未知版本'}`,
									ok: true,
								},
								entry: {
									serverId: server.serverId,
									serverName: preview.server_name || server.serverId,
									packId: preview.pack_id,
									targetVersion: preview.target_version ?? '',
									requiredVersion: preview.required_version ?? '',
									seasonName: preview.season_name ?? '',
								},
							}
						} catch (error) {
							return {
								diagnostic: { serverName, detail: ymclErrorMessage(error), ok: false },
								entry: null,
							}
						}
					},
				),
			)
			serverDiagnostics.value = previews.map((item) => item.diagnostic)
			// 同一个包可能被多个服务器绑定：只保留首个入口。
			const seen = new Set<string>()
			packs.value = previews
				.map((item) => item.entry)
				.filter((pack) => {
					if (!pack || seen.has(pack.packId)) return false
					seen.add(pack.packId)
					return true
				})
			if (packs.value.length === 0) {
				// 有绑定但全部解析不出可用包：多半是绑定指向的包/版本已被删除。
				emptyReason.value = 'no-pack'
			}

			const instances = await list().catch(() => [])
			const installs = await Promise.all(
				instances.map(async (instance): Promise<DomainPackInstall | null> => {
					try {
						const check = await ymcl.preLaunchCheck(instance.id)
						if (!check.managed || !check.pack_id) return null
						return {
							packId: check.pack_id,
							serverId: check.server_id ?? '',
							instanceId: instance.id,
							instanceName: instance.name,
							currentVersion: check.current_version ?? '',
						}
					} catch {
						return null
					}
				}),
			)
			const grouped = new Map<string, DomainPackInstall[]>()
			for (const install of installs) {
				if (!install) continue
				const group = grouped.get(install.packId) ?? []
				group.push(install)
				grouped.set(install.packId, group)
			}
			installsByPack.value = grouped
		} catch (error) {
			// 服务器列表都拿不到（无 MIP 面/会话失效/权限点未授权）：面板显示
			// 失败原因而不是无声消失。
			packs.value = []
			emptyReason.value = 'load-failed'
			loadError.value = ymclErrorMessage(error)
		} finally {
			loading.value = false
		}
	}

	async function download(pack: DomainPackEntry) {
		if (downloadingServer.value) return
		downloadingServer.value = pack.serverId
		try {
			const name = pack.serverName || pack.packId
			// WF-4 首装走 mrpack：归档自带 dependencies，游戏版本/加载器由内置
			// 导入器物化，不再需要面板上的 requiredVersion。目标版本没有完整
			// 包（纯增量版本）时后端回退为 parent 链上的基线 mrpack。
			const archive = await ymcl.downloadPackMrpack(pack.serverId, pack.packId, [])
			const job = await install_create_modpack_instance(
				{ type: 'fromFile', path: archive.path },
				{ name },
			)
			const settled = await wait_for_install_job(job.job_id)
			const instanceId = settled.target.instance_id ?? ''
			if (!instanceId) throw new Error('实例创建未返回 instanceId')
			await ymcl.adoptPackState(
				instanceId,
				pack.serverId,
				archive.pack_id,
				archive.version,
				archive.channel ?? null,
				[],
			)
			const finalVersion = await catchUpAfterBaseline(instanceId, archive)
			addNotification({
				title: formatMessage(messages.downloaded, { name }),
				text: `${archive.pack_id} → ${finalVersion}`,
				type: 'success',
			})
			await refresh()
		} catch (error) {
			addNotification({
				title: formatMessage(messages.installFailed),
				text: ymclErrorMessage(error),
				type: 'error',
			})
		} finally {
			downloadingServer.value = ''
		}
	}

	async function update(pack: DomainPackEntry) {
		const install = installFor(pack)
		if (!install || updatingInstance.value) return
		updatingInstance.value = install.instanceId
		try {
			if (install.packId === pack.packId) {
				// 同一包的版本推进：MIP WF-5 增量更新。
				const result = await ymcl.applyPackUpdate(install.instanceId)
				addNotification({
					title: formatMessage(messages.updated, { version: result.new_version }),
					type: 'success',
				})
			} else {
				// 服务器删包重传（packId 变化）：下载当前绑定包并装入原实例。
				const archive = await ymcl.downloadPackMrpack(pack.serverId, null, [])
				const job = await install_pack_to_existing_instance(
					install.instanceId,
					{ type: 'fromFile', path: archive.path },
					null,
				)
				await wait_for_install_job(job.job_id)
				// 服务器删包重传换了 packId：force 接管旧包实例。
				await ymcl.adoptPackState(
					install.instanceId,
					pack.serverId,
					archive.pack_id,
					archive.version,
					archive.channel ?? null,
					[],
					true,
				)
				const finalVersion = await catchUpAfterBaseline(install.instanceId, archive)
				addNotification({
					title: formatMessage(messages.updated, { version: finalVersion }),
					type: 'success',
				})
			}
			await refresh()
		} catch (error) {
			addNotification({
				title: formatMessage(messages.updateFailed),
				text: ymclErrorMessage(error),
				type: 'error',
			})
		} finally {
			updatingInstance.value = ''
		}
	}

	watch(
		() => ymclStore.activeDomainId,
		() => void refresh(),
	)
	// 发布/绑定推送到达后重拉：否则面板停留在发布前的旧条目上
	// （旧条目缺 requiredVersion，下载会被误判为"未声明游戏版本"）。
	watch(
		() => ymclStore.dataEpoch,
		() => void refresh(),
	)

	onMounted(() => void refresh())

	return {
		loading,
		packs,
		installsByPack,
		downloadingServer,
		updatingInstance,
		hasDomainFace,
		emptyReason,
		loadError,
		serverDiagnostics,
		installFor,
		hasUpdate,
		refresh,
		download,
		update,
	}
}
