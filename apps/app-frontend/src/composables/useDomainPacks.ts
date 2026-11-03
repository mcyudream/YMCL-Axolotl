import { defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref, watch } from 'vue'

import {
	install_create_modpack_instance,
	install_job_get,
	install_job_retry,
	install_pack_to_existing_instance,
	installJobInstanceId,
	wait_for_install_job,
} from '@/helpers/install'
import { list } from '@/helpers/instance'
import {
	PERSONAL_DOMAIN_ID,
	ymcl,
	ymclErrorMessage,
	type YmclMrpackDownload,
	type YmclUpdateResult,
} from '@/helpers/ymcl'
import { useYmclStore } from '@/store/ymcl'

import { useDomainPackOperations } from './domain-pack-operations'

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

export interface DomainPackOperationProgress {
	stage: 'downloading' | 'installing' | 'finalizing' | 'updating'
	packId: string
	instanceId?: string
	jobId?: string
}

export interface DomainPackOperationOptions {
	notify?: boolean
	instanceId?: string
	selectedFeatures?: string[]
	onProgress?: (progress: DomainPackOperationProgress) => void
}

export type DomainPackOperationResult =
	| { ok: true; instanceId: string; version: string; warning?: string; update?: YmclUpdateResult }
	| { ok: false; error: string }

const pendingDownloads = new Map<
	string,
	{
		archive: YmclMrpackDownload
		selected: string[]
		jobId: string
	}
>()

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
	const operations = useDomainPackOperations()

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
			defaultMessage: '已安装基线版本 {version}，增量更新到 {target} 失败；启动时会自动重试。',
		},
		contextChanged: {
			id: 'app.ymcl.packs.context-changed',
			defaultMessage: 'The active domain or account changed. Continue from the original domain.',
		},
		operationInProgress: {
			id: 'app.ymcl.packs.operation-in-progress',
			defaultMessage: 'A task for this server is already running. View its progress in Downloads.',
		},
	})

	/** 基线安装后的增量追平；失败只提示，不推翻已完成的安装。 */
	async function catchUpAfterBaseline(
		instanceId: string,
		archive: YmclMrpackDownload,
		options: DomainPackOperationOptions,
	) {
		let warning = ''
		if (archive.target_version && archive.target_version !== archive.version) {
			options.onProgress?.({ stage: 'updating', packId: archive.pack_id, instanceId })
		}
		let version = archive.version
		let update: YmclUpdateResult | undefined
		try {
			if (archive.target_version && archive.target_version !== archive.version) {
				update = await ymcl.applyPackUpdate(instanceId)
				version = update.new_version || archive.version
			}
		} catch (error) {
			warning = ymclErrorMessage(error)
			if (options.notify !== false)
				addNotification({
					title: formatMessage(messages.catchupFailed, {
						version: archive.version,
						target: archive.target_version ?? '',
					}),
					text: ymclErrorMessage(error),
					type: 'error',
				})
		}
		return { version, warning, update }
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
					async (
						server,
					): Promise<{
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

	async function download(
		pack: DomainPackEntry,
		options: DomainPackOperationOptions = {},
	): Promise<DomainPackOperationResult | undefined> {
		if (downloadingServer.value || updatingInstance.value) return
		const domainId = ymclStore.activeDomainId
		const userId = ymclStore.session?.session.user_id
		const pendingKey = JSON.stringify([domainId, userId, pack.serverId, pack.packId])
		const handle = operations.begin(
			{ domainId, userId: userId ?? null, serverId: pack.serverId },
			{ stage: 'downloading', packId: pack.packId },
		)
		if (!handle) return operationInProgress(options)
		const onProgress = options.onProgress
		options = {
			...options,
			onProgress(progress) {
				operations.advance(handle, progress)
				onProgress?.(progress)
			},
		}
		const requireSameDomain = () => {
			if (ymclStore.activeDomainId !== domainId || ymclStore.session?.session.user_id !== userId) {
				throw new Error(formatMessage(messages.contextChanged))
			}
		}
		downloadingServer.value = pack.serverId
		try {
			const name = pack.serverName || pack.packId
			// WF-4 首装走 mrpack：归档自带 dependencies，游戏版本/加载器由内置
			// 导入器物化，不再需要面板上的 requiredVersion。目标版本没有完整
			// 包（纯增量版本）时后端回退为 parent 链上的基线 mrpack。
			let pending = pendingDownloads.get(pendingKey)
			if (!pending) {
				const selected = options.selectedFeatures ?? []
				options.onProgress?.({ stage: 'downloading', packId: pack.packId })
				const archive = await ymcl.downloadPackMrpack(pack.serverId, pack.packId, selected)
				requireSameDomain()
				const job = await install_create_modpack_instance(
					{ type: 'fromFile', path: archive.path },
					{ name },
				)
				pending = { archive, selected, jobId: job.job_id }
				pendingDownloads.set(pendingKey, pending)
			} else {
				const job = await install_job_get(pending.jobId)
				requireSameDomain()
				if (['failed', 'interrupted', 'canceled'].includes(job.status))
					await install_job_retry(job.job_id)
			}
			const { archive, selected, jobId } = pending
			options.onProgress?.({ stage: 'installing', packId: archive.pack_id, jobId })
			const settled = await wait_for_install_job(jobId)
			const instanceId = installJobInstanceId(settled) ?? ''
			if (!instanceId) throw new Error('实例创建未返回 instanceId')
			requireSameDomain()
			options.onProgress?.({ stage: 'finalizing', packId: archive.pack_id, instanceId, jobId })
			await ymcl.adoptPackState(
				instanceId,
				pack.serverId,
				archive.pack_id,
				archive.version,
				archive.channel ?? null,
				selected,
			)
			pendingDownloads.delete(pendingKey)
			requireSameDomain()
			const { version, warning, update } = await catchUpAfterBaseline(instanceId, archive, options)
			if (options.notify !== false)
				addNotification({
					title: formatMessage(messages.downloaded, { name }),
					text: `${archive.pack_id} → ${version}`,
					type: 'success',
				})
			if (ymclStore.activeDomainId === domainId) await refresh()
			const result = { ok: true, instanceId, version, warning, update } as const
			operations.finish(handle, result)
			return result
		} catch (error) {
			if (options.notify !== false)
				addNotification({
					title: formatMessage(messages.installFailed),
					text: ymclErrorMessage(error),
					type: 'error',
				})
			const result = { ok: false, error: ymclErrorMessage(error) } as const
			operations.finish(handle, result)
			return result
		} finally {
			downloadingServer.value = ''
		}
	}

	async function update(
		pack: DomainPackEntry,
		options: DomainPackOperationOptions = {},
	): Promise<DomainPackOperationResult | undefined> {
		let install = options.instanceId
			? [...installsByPack.value.values()]
					.flat()
					.find((item) => item.instanceId === options.instanceId)
			: installFor(pack)
		if ((!install && !options.instanceId) || updatingInstance.value || downloadingServer.value)
			return
		const domainId = ymclStore.activeDomainId
		const userId = ymclStore.session?.session.user_id
		const handle = operations.begin(
			{ domainId, userId: userId ?? null, serverId: pack.serverId },
			{
				stage: 'updating',
				packId: pack.packId,
				instanceId: options.instanceId ?? install?.instanceId,
			},
		)
		if (!handle) return operationInProgress(options)
		const onProgress = options.onProgress
		options = {
			...options,
			onProgress(progress) {
				operations.advance(handle, progress)
				onProgress?.(progress)
			},
		}
		const requireSameDomain = () => {
			if (ymclStore.activeDomainId !== domainId || ymclStore.session?.session.user_id !== userId) {
				throw new Error(formatMessage(messages.contextChanged))
			}
		}
		updatingInstance.value = options.instanceId ?? install!.instanceId
		try {
			if (!install) {
				const check = await ymcl.preLaunchCheck(options.instanceId!)
				requireSameDomain()
				if (
					!check.managed ||
					!check.pack_id ||
					(check.pack_id !== pack.packId && check.server_id !== pack.serverId)
				)
					throw new Error(formatMessage(messages.updateFailed))
				install = {
					instanceId: options.instanceId!,
					instanceName: '',
					serverId: check.server_id ?? pack.serverId,
					packId: check.pack_id,
					currentVersion: check.current_version ?? '',
				}
			}
			let outcome: DomainPackOperationResult
			if (install.packId === pack.packId) {
				// 同一包的版本推进：MIP WF-5 增量更新。
				options.onProgress?.({
					stage: 'updating',
					packId: pack.packId,
					instanceId: install.instanceId,
				})
				const result = await ymcl.applyPackUpdate(install.instanceId)
				if (options.notify !== false)
					addNotification({
						title: formatMessage(messages.updated, { version: result.new_version }),
						type: 'success',
					})
				outcome = {
					ok: true,
					instanceId: install.instanceId,
					version: result.new_version,
					update: result,
				}
			} else {
				// 服务器删包重传（packId 变化）：下载当前绑定包并装入原实例。
				options.onProgress?.({
					stage: 'downloading',
					packId: pack.packId,
					instanceId: install.instanceId,
				})
				const archive = await ymcl.downloadPackMrpack(pack.serverId, null, [])
				requireSameDomain()
				const job = await install_pack_to_existing_instance(
					install.instanceId,
					{ type: 'fromFile', path: archive.path },
					null,
				)
				options.onProgress?.({
					stage: 'installing',
					packId: archive.pack_id,
					instanceId: install.instanceId,
					jobId: job.job_id,
				})
				await wait_for_install_job(job.job_id)
				requireSameDomain()
				options.onProgress?.({
					stage: 'finalizing',
					packId: archive.pack_id,
					instanceId: install.instanceId,
					jobId: job.job_id,
				})
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
				requireSameDomain()
				const { version, warning, update } = await catchUpAfterBaseline(
					install.instanceId,
					archive,
					options,
				)
				if (options.notify !== false)
					addNotification({
						title: formatMessage(messages.updated, { version }),
						type: 'success',
					})
				outcome = { ok: true, instanceId: install.instanceId, version, warning, update }
			}
			if (ymclStore.activeDomainId === domainId) await refresh()
			operations.finish(handle, outcome)
			return outcome
		} catch (error) {
			if (options.notify !== false)
				addNotification({
					title: formatMessage(messages.updateFailed),
					text: ymclErrorMessage(error),
					type: 'error',
				})
			const result = { ok: false, error: ymclErrorMessage(error) } as const
			operations.finish(handle, result)
			return result
		} finally {
			updatingInstance.value = ''
		}
	}

	function operationInProgress(options: DomainPackOperationOptions): DomainPackOperationResult {
		const error = formatMessage(messages.operationInProgress)
		if (options.notify !== false) addNotification({ title: error, type: 'error' })
		return { ok: false, error }
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
