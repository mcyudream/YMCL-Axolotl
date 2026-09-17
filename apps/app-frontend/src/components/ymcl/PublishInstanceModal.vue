<script setup lang="ts">
import {
	FolderOpenIcon,
	LoaderIcon,
	PlusIcon,
	TrashIcon,
	TriangleAlertIcon,
	UploadIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	Checkbox,
	Combobox,
	defineMessages,
	FileTreeSelect,
	injectNotificationManager,
	NewModal,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { join } from '@tauri-apps/api/path'
import { readDir, stat } from '@tauri-apps/plugin-fs'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'

import PublishChangeListModal from '@/components/ymcl/PublishChangeListModal.vue'
import { get_full_path, get_pack_export_candidates } from '@/helpers/instance'
import { ymcl, ymclErrorMessage } from '@/helpers/ymcl'
import { useYmclStore } from '@/store/ymcl'

/**
 * Publish modal for a single instance (YAP §7): inspect the diff against the
 * domain binding's current version and push it as an incremental delta
 * publish (diff → upload → publish → bind → notify, in one action).
 */

const props = defineProps<{
	instanceId: string
	instanceName: string
}>()

const { addNotification } = injectNotificationManager()
const { formatMessage } = useVIntl()
const ymclStore = useYmclStore()

const messages = defineMessages({
	title: {
		id: 'app.ymcl.publish.title',
		defaultMessage: '发布到域',
	},
	description: {
		id: 'app.ymcl.publish.description',
		defaultMessage: '检查实例与域绑定版本的差异，作为初始包或增量更新发布，并通知全体成员。',
	},
	loading: { id: 'app.ymcl.publish.loading', defaultMessage: '加载中…' },
	checkDiff: { id: 'app.ymcl.publish.check', defaultMessage: '检查变更' },
	recheck: { id: 'app.ymcl.publish.recheck', defaultMessage: '重新检查' },
	versionLabel: { id: 'app.ymcl.publish.version', defaultMessage: '新版本号' },
	channelLabel: { id: 'app.ymcl.publish.channel', defaultMessage: '渠道' },
	push: { id: 'app.ymcl.publish.push', defaultMessage: '发布更新' },
	initialMode: {
		id: 'app.ymcl.publish.initial-mode',
		defaultMessage: '该实例未绑定域整合包，将作为初始包发布并创建整合包。',
	},
	pushInitial: {
		id: 'app.ymcl.publish.push-initial',
		defaultMessage: '发布初始包',
	},
	publishedInitial: {
		id: 'app.ymcl.publish.published-initial',
		defaultMessage: '初始包已发布并绑定到服务器。',
	},
	bindLabel: { id: 'app.ymcl.publish.bind-label', defaultMessage: '绑定服务器' },
	bindNone: { id: 'app.ymcl.publish.bind-none', defaultMessage: '不绑定' },
	bindModeLabel: {
		id: 'app.ymcl.publish.bind-mode',
		defaultMessage: '绑定方式',
	},
	bindModeRequired: {
		id: 'app.ymcl.publish.bind-mode-required',
		defaultMessage: '必装整合包（玩家进服前必须安装）',
	},
	bindModeOptional: {
		id: 'app.ymcl.publish.bind-mode-optional',
		defaultMessage: '原版增强包（玩家可选，适合需要原版的服务器）',
	},
	bindHintRequired: {
		id: 'app.ymcl.publish.bind-hint-required',
		defaultMessage:
			'玩家进服前必须安装此整合包：进服窗口会引导装入已有的对应实例，或自动新建实例并安装。',
	},
	bindHintOptional: {
		id: 'app.ymcl.publish.bind-hint-optional',
		defaultMessage:
			'玩家进服时三选一：用本地对应版本实例直接启动 / 下载此原版增强整合包 / 下载纯净原版。',
	},
	serversErrorLabel: {
		id: 'app.ymcl.publish.servers-error',
		defaultMessage: '服务器列表加载失败',
	},
	serversEmpty: {
		id: 'app.ymcl.publish.servers-empty',
		defaultMessage: '该域还没有可绑定的服务器，可先不绑定直接发布。',
	},
	retry: {
		id: 'app.ymcl.publish.retry',
		defaultMessage: '重试',
	},
	contentTitle: {
		id: 'app.ymcl.publish.content-title',
		defaultMessage: '包内容',
	},
	contentHint: {
		id: 'app.ymcl.publish.content-hint',
		defaultMessage: '勾选要打进初始包的文件，玩家侧文件（存档/日志等）已自动排除。',
	},
	publishedOptional: {
		id: 'app.ymcl.publish.published-optional',
		defaultMessage: '原版增强包已发布并绑定到服务器。',
	},
	publishedNoBind: {
		id: 'app.ymcl.publish.published-no-bind',
		defaultMessage: '已发布，但未绑定服务器——绑定前玩家看不到这个包。',
	},
	detailsTitle: { id: 'app.ymcl.publish.details', defaultMessage: '变更明细' },
	detailsTruncated: {
		id: 'app.ymcl.publish.details-truncated',
		defaultMessage: '仅显示前 {count} 项，其余以统计数为准。',
	},
	checkFailed: { id: 'app.ymcl.publish.check-failed', defaultMessage: '检查变更失败' },
	pushFailed: { id: 'app.ymcl.publish.push-failed', defaultMessage: '发布失败' },
	withdrawTitle: {
		id: 'app.ymcl.publish.withdraw-title',
		defaultMessage: '已发布版本',
	},
	withdrawHint: {
		id: 'app.ymcl.publish.withdraw-hint',
		defaultMessage: '撤回后客户端不再把该版本当作更新目标；已安装的实例保持当前版本。',
	},
	withdraw: {
		id: 'app.ymcl.publish.withdraw',
		defaultMessage: '撤回',
	},
	withdrawing: {
		id: 'app.ymcl.publish.withdrawing',
		defaultMessage: '撤回中…',
	},
	withdrawConfirm: {
		id: 'app.ymcl.publish.withdraw-confirm',
		defaultMessage: '确认撤回版本 {version}？客户端将不再收到此更新。',
	},
	withdrawn: {
		id: 'app.ymcl.publish.withdrawn',
		defaultMessage: '版本已撤回',
	},
	withdrawFailed: {
		id: 'app.ymcl.publish.withdraw-failed',
		defaultMessage: '撤回失败',
	},
	versionsEmpty: {
		id: 'app.ymcl.publish.versions-empty',
		defaultMessage: '暂无已发布版本。',
	},
	needsDomain: {
		id: 'app.ymcl.publish.needs-domain',
		defaultMessage: '发布前请先在顶栏选择一个域。',
	},
	needsLogin: {
		id: 'app.ymcl.publish.needs-login',
		defaultMessage: '发布前请先登录当前域。',
	},
	nothingToDo: {
		id: 'app.ymcl.publish.nothing',
		defaultMessage: '没有可发布的变更：实例与绑定版本一致。',
	},
	changed: { id: 'app.ymcl.publish.changed', defaultMessage: '修改' },
	added: { id: 'app.ymcl.publish.added', defaultMessage: '新增' },
	deleted: { id: 'app.ymcl.publish.deleted', defaultMessage: '删除' },
	moved: { id: 'app.ymcl.publish.moved', defaultMessage: '移动' },
	viewChanges: {
		id: 'app.ymcl.publish.view-changes',
		defaultMessage: '查看全部',
	},
	featuresTitle: {
		id: 'app.ymcl.publish.features-title',
		defaultMessage: '可选内容',
	},
	featuresHint: {
		id: 'app.ymcl.publish.features-hint',
		defaultMessage:
			'用 glob 声明可选内容（如 mods/iris-*.jar，一行一个）。玩家安装时按需勾选，默认关闭的内容不会自动下发。',
	},
	addFeature: {
		id: 'app.ymcl.publish.add-feature',
		defaultMessage: '添加可选内容',
	},
	featureNameLabel: {
		id: 'app.ymcl.publish.feature-name',
		defaultMessage: '名称',
	},
	featureGlobsLabel: {
		id: 'app.ymcl.publish.feature-globs',
		defaultMessage: '文件 glob',
	},
	pickFiles: {
		id: 'app.ymcl.publish.pick-files',
		defaultMessage: '从文件选择',
	},
	featureDefaultLabel: {
		id: 'app.ymcl.publish.feature-default',
		defaultMessage: '默认勾选',
	},
	featureConflictsLabel: {
		id: 'app.ymcl.publish.feature-conflicts',
		defaultMessage: '互斥项',
	},
	featureMatchCount: {
		id: 'app.ymcl.publish.feature-match-count',
		defaultMessage: '匹配 {count} 个文件',
	},
	featureNoMatch: {
		id: 'app.ymcl.publish.feature-no-match',
		defaultMessage: 'glob 未匹配到任何文件',
	},
	featureIdRequired: {
		id: 'app.ymcl.publish.feature-id-required',
		defaultMessage: '可选内容必须填写 ID',
	},
	featureIdDuplicate: {
		id: 'app.ymcl.publish.feature-id-duplicate',
		defaultMessage: '可选内容 ID 重复：{id}',
	},
	featureConflictUnknown: {
		id: 'app.ymcl.publish.feature-conflict-unknown',
		defaultMessage: '互斥项引用了不存在的 ID：{id}',
	},
	featuresHintManaged: {
		id: 'app.ymcl.publish.features-hint-managed',
		defaultMessage:
			'为本版新增/变更的文件声明可选内容（glob，一行一个）。新声明的可选内容必须至少匹配到一个本版新文件；重新声明的可选项会更新其名称/默认值/互斥项。',
	},
	policiesTitle: {
		id: 'app.ymcl.publish.policies-title',
		defaultMessage: '文件策略',
	},
	policiesHint: {
		id: 'app.ymcl.publish.policies-hint',
		defaultMessage:
			'为匹配的文件声明策略：seed 不覆盖玩家已有文件，merge 冲突时保留玩家版本并生成 .pack-new 副本，managed 直接替换。',
	},
	addPolicy: {
		id: 'app.ymcl.publish.add-policy',
		defaultMessage: '添加策略',
	},
	excluded: {
		id: 'app.ymcl.publish.excluded',
		defaultMessage: '已排除（玩家侧文件）',
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
	size?: number | null
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

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const diff = ref<PublishDiff | null>(null)
const checking = ref(false)
const pushing = ref(false)
const version = ref('')
const channel = ref('stable')
const servers = ref<MipServerOption[]>([])
const serverId = ref('')
/** 服务器列表加载失败的原因（呈现给管理员并允许重试，而不是静默藏掉绑定区）。 */
const serversError = ref<string | null>(null)
/** 绑定方式：false=必装整合包（覆盖 packId），true=原版增强包（合并写 optionalPackId）。 */
const bindOptional = ref(false)

interface MipServerOption {
	serverId: string
	name?: string | null
}

const serverOptions = computed(() => [
	{ value: '', label: formatMessage(messages.bindNone) },
	...servers.value.map((server) => ({
		value: server.serverId,
		label: server.name || server.serverId,
	})),
])

const gate = computed(() => {
	if (ymclStore.isPersonal) return 'domain'
	if (!ymclStore.session) return 'login'
	return null
})

const changeListModal = ref<InstanceType<typeof PublishChangeListModal> | null>(
	null,
)

/** Editable optional-content declaration (mip.json features, MIP §3.5). */
interface FeatureDraft {
	id: string
	name: string
	defaultChecked: boolean
	conflicts: string[]
	globsText: string
}
const features = ref<FeatureDraft[]>([])

/** For an unmanaged instance every pack file shows up as "added"; for a
 * delta the adapter annotates changed/added entries too. */
const candidatePaths = computed(() =>
	diff.value
		? [
				...diff.value.changed.map((entry) => entry.path),
				...diff.value.added.map((entry) => entry.path),
			]
		: [],
)

function globToRegExp(glob: string): RegExp {
	const escaped = glob
		.replace(/[.+^${}()|[\]\\]/g, '\\$&')
		.replace(/\*\*/g, '\0')
		.replace(/\*/g, '[^/]*')
		.replace(/\?/g, '[^/]')
		.replace(/\0/g, '.*')
	return new RegExp(`^${escaped}$`)
}

function featureMatchCount(feature: FeatureDraft): number {
	const patterns = feature.globsText
		.split('\n')
		.map((line) => line.trim())
		.filter(Boolean)
		.map(globToRegExp)
	if (patterns.length === 0) return 0
	return candidatePaths.value.filter((path) =>
		patterns.some((regexp) => regexp.test(path)),
	).length
}

/** Structural problems that block publishing in any mode. */
const featureStructuralIssue = computed(() => {
	const ids = features.value.map((feature) => feature.id.trim())
	for (const feature of features.value) {
		const id = feature.id.trim()
		if (!id) return formatMessage(messages.featureIdRequired)
		if (ids.filter((other) => other === id).length > 1) {
			return formatMessage(messages.featureIdDuplicate, { id })
		}
		for (const conflict of feature.conflicts) {
			if (!ids.includes(conflict)) {
				return formatMessage(messages.featureConflictUnknown, { id: conflict })
			}
		}
	}
	return null
})

/** 0-match is blocking for an initial publish; a delta redeclaration may
 * legitimately match no new file (e.g. only changing the default). */
const featureNoMatchWarning = computed(() => {
	for (const feature of features.value) {
		const id = feature.id.trim()
		if (id && featureMatchCount(feature) === 0) {
			return `${id}: ${formatMessage(messages.featureNoMatch)}`
		}
	}
	return null
})

const featureIssue = computed(() => {
	const structural = featureStructuralIssue.value
	if (structural) return structural
	if (!diff.value?.managed) return featureNoMatchWarning.value
	return null
})

/** Editable file-policy rules (mip.json policies, MIP §3.5). */
interface PolicyDraft {
	glob: string
	policy: string
}
const policies = ref<PolicyDraft[]>([])
const policyOptions = [
	{ value: 'managed', label: 'managed' },
	{ value: 'seed', label: 'seed' },
	{ value: 'merge', label: 'merge' },
]

function addPolicy() {
	policies.value.push({ glob: '', policy: 'seed' })
}

function removePolicy(index: number) {
	policies.value.splice(index, 1)
}

/** Live match count so a policy rule's glob isn't a shot in the dark. */
function policyMatchCount(rule: PolicyDraft): number {
	const glob = rule.glob.trim()
	if (!glob) return 0
	const regexp = globToRegExp(glob)
	return candidatePaths.value.filter((path) => regexp.test(path)).length
}

function addFeature() {
	features.value.push({
		id: `feature-${features.value.length + 1}`,
		name: '',
		defaultChecked: false,
		conflicts: [],
		globsText: '',
	})
}

function removeFeature(index: number) {
	features.value.splice(index, 1)
}

function showChanges(options?: { pickMode?: boolean; selected?: string[] }) {
	if (!diff.value) return
	changeListModal.value?.show(
		{
			changed: diff.value.changed,
			added: diff.value.added,
			deleted: diff.value.deleted,
			moved: diff.value.moved,
		},
		options,
	)
}

/** 从变更明细里勾选文件，精确路径填入该可选内容的 glob 列表。 */
const pickingFeatureIndex = ref<number | null>(null)

function openFilePicker(index: number) {
	pickingFeatureIndex.value = index
	const feature = features.value[index]
	const literal = feature
		? feature.globsText
				.split('\n')
				.map((line) => line.trim())
				.filter((line) => line && !/[*?]/.test(line))
		: []
	showChanges({ pickMode: true, selected: literal })
}

function onPickFiles(paths: string[]) {
	const index = pickingFeatureIndex.value
	pickingFeatureIndex.value = null
	if (index == null || paths.length === 0) return
	const feature = features.value[index]
	if (!feature) return
	const lines = feature.globsText
		.split('\n')
		.map((line) => line.trim())
		.filter(Boolean)
	for (const path of paths) {
		if (!lines.includes(path)) lines.push(path)
	}
	feature.globsText = lines.join('\n')
}

const totalChanges = computed(() =>
	diff.value
		? diff.value.changed.length +
			diff.value.added.length +
			diff.value.deleted.length +
			diff.value.moved.length
		: 0,
)

type DiffRowKind = 'changed' | 'added' | 'deleted' | 'moved'

interface DiffRow {
	kind: DiffRowKind
	path: string
	from?: string | null
	size?: number | null
}

/** Per-file detail rows for the change list, ordered by operation type. */
const diffRows = computed<DiffRow[]>(() => {
	const d = diff.value
	if (!d) return []
	return [
		...d.changed.map((entry) => ({
			kind: 'changed' as const,
			path: entry.path,
			from: null,
			size: entry.size,
		})),
		...d.added.map((entry) => ({
			kind: 'added' as const,
			path: entry.path,
			from: null,
			size: entry.size,
		})),
		...d.deleted.map((path) => ({
			kind: 'deleted' as const,
			path,
			from: null,
			size: null,
		})),
		...d.moved.map((entry) => ({
			kind: 'moved' as const,
			path: entry.path,
			from: entry.from,
			size: entry.size,
		})),
	]
})

function bumpPatch(versionString: string): string {
	const parts = versionString.split('.')
	const last = Number(parts[parts.length - 1])
	if (!Number.isNaN(last)) {
		parts[parts.length - 1] = String(last + 1)
		return parts.join('.')
	}
	return versionString
}

async function checkDiff() {
	checking.value = true
	diff.value = null
	try {
		diff.value = await invoke<PublishDiff>('plugin:ymcl|ymcl_publish_diff', {
			instanceId: props.instanceId,
		})
		if (!version.value && diff.value.base_version) {
			version.value = bumpPatch(diff.value.base_version)
		}
		if (!diff.value.managed && !version.value) {
			version.value = '1.0.0'
		}
		if (diff.value.managed) {
			void loadPublishedVersions()
		} else {
			publishedVersions.value = []
		}
	} catch (error) {
		addNotification({
			title: formatMessage(messages.checkFailed),
			text: ymclErrorMessage(error),
			type: 'error',
		})
	} finally {
		checking.value = false
	}
}

const publishedVersions = ref<{ version: string; channel?: string | null }[]>([])
const withdrawingVersion = ref<string | null>(null)

async function loadPublishedVersions() {
	try {
		publishedVersions.value = await ymcl.listPublishVersions(props.instanceId)
	} catch {
		publishedVersions.value = []
	}
}

async function withdrawVersion(target: string) {
	if (withdrawingVersion.value) return
	const confirmed = window.confirm(
		formatMessage(messages.withdrawConfirm, { version: target }),
	)
	if (!confirmed) return
	withdrawingVersion.value = target
	try {
		await ymcl.withdrawPublishVersion(props.instanceId, target)
		addNotification({
			title: formatMessage(messages.withdrawn),
			text: `${diff.value?.pack_id ?? ''} ${target}`,
			type: 'success',
		})
		await loadPublishedVersions()
		await checkDiff()
	} catch (error) {
		addNotification({
			title: formatMessage(messages.withdrawFailed),
			text: ymclErrorMessage(error),
			type: 'error',
		})
	} finally {
		withdrawingVersion.value = null
	}
}

async function loadServers() {
	servers.value = []
	serverId.value = ''
	serversError.value = null
	try {
		servers.value = await invoke<MipServerOption[]>('plugin:ymcl|ymcl_mip_servers')
		// 单服务器域直接选中：绑定是玩家能下载到包的前提，别让管理员漏选。
		if (servers.value.length === 1) {
			serverId.value = servers.value[0].serverId
		}
	} catch (error) {
		serversError.value = ymclErrorMessage(error)
	}
}

// ==== 初始包内容树（融合「导出整合包」的文件勾选体验）====

/** 初始包永不收录的顶层目录/文件（与后端 PUBLISH_EXCLUDED_TOP_LEVEL 对齐）。 */
const CONTENT_EXCLUDED_TOP_LEVEL = [
	'saves',
	'logs',
	'crash-reports',
	'screenshots',
	'.pack-staging',
	'.pack-backup',
]

const contentFiles = ref<{ path: string; type: string; size?: number; modified?: number; count?: number }[]>([])
const selectedContentPaths = ref<string[]>([])
const contentTreeKey = ref(0)
const contentLoadId = ref(0)
const contentRoot = ref('')
const contentLoadedDirs = ref(new Set<string>())
const contentCandidates = ref<string[]>([])
/** 管理员取消勾选的路径 = push_initial 的 exclude。 */
const excludedContentPaths = computed(() =>
	contentCandidates.value.filter((path) => !selectedContentPaths.value.includes(path)),
)

function contentPathAllowed(path: string): boolean {
	const top = path.split('/')[0]
	const fileName = path.split('/').pop() ?? path
	return (
		!CONTENT_EXCLUDED_TOP_LEVEL.includes(top) &&
		path !== '.pack-state.json' &&
		!fileName.toLowerCase().endsWith('.log')
	)
}

async function initContentTree() {
	const loadId = ++contentLoadId.value
	contentFiles.value = []
	selectedContentPaths.value = []
	contentCandidates.value = []
	try {
		const [candidates, root] = await Promise.all([
			get_pack_export_candidates(props.instanceId),
			get_full_path(props.instanceId),
		])
		if (loadId !== contentLoadId.value) return
		contentRoot.value = root
		const allowed = candidates.map((path) => path.replaceAll('\\', '/')).filter(contentPathAllowed)
		contentCandidates.value = allowed
		const items = await Promise.all(allowed.map((path) => buildContentItem(root, path)))
		if (loadId !== contentLoadId.value) return
		contentFiles.value = items
		selectedContentPaths.value = allowed
		contentTreeKey.value += 1
	} catch {
		// 树加载失败不阻断发布（后端仍按全量候选打包）。
	}
}

async function buildContentItem(root: string, path: string): Promise<{ path: string; type: string; size?: number; modified?: number; count?: number }> {
	const normalized = path.replaceAll('\\', '/')
	try {
		const entries = await readDir(await join(root, ...path.split(/[\\/]/)))
		const metadata = await contentMetadata(root, path)
		return { path: normalized, type: 'directory', modified: metadata.modified, count: entries.length }
	} catch {
		const metadata = await contentMetadata(root, path)
		return { path: normalized, type: 'file', size: metadata.size, modified: metadata.modified }
	}
}

async function contentMetadata(root: string, path: string): Promise<{ size?: number; modified?: number }> {
	try {
		const metadata = await stat(await join(root, ...path.split(/[\\/]/)))
		return {
			size: metadata.size,
			modified: metadata.mtime ? Math.floor(metadata.mtime.getTime() / 1000) : undefined,
		}
	} catch {
		return {}
	}
}

async function loadContentDirectory(path: string) {
	if (!path || !contentRoot.value || contentLoadedDirs.value.has(path)) return
	const loadId = contentLoadId.value
	contentLoadedDirs.value.add(path)
	try {
		const entries = await readDir(await join(contentRoot.value, ...path.split('/')))
		const children = await Promise.all(
			entries
				.map((entry) => `${path}/${entry.name}`)
				.filter(contentPathAllowed)
				.map((child) => buildContentItem(contentRoot.value, child)),
		)
		if (loadId !== contentLoadId.value) return
		const known = new Map(contentFiles.value.map((file) => [file.path, file]))
		for (const child of children) known.set(child.path, child)
		contentFiles.value = [...known.values()]
	} catch {
		contentLoadedDirs.value.delete(path)
	}
}

async function push() {
	if (!version.value || pushing.value) return
	pushing.value = true
	try {
		const bind = serverId.value
			? { serverId: serverId.value, optional: bindOptional.value }
			: null
		const featurePayload = features.value.map((feature) => ({
			id: feature.id.trim(),
			name: feature.name.trim() || null,
			default: feature.defaultChecked,
			conflicts: [...feature.conflicts],
			files: feature.globsText
				.split('\n')
				.map((line) => line.trim())
				.filter(Boolean),
		}))
		const policyPayload = policies.value
			.filter((rule) => rule.glob.trim())
			.map((rule) => ({ glob: rule.glob.trim(), policy: rule.policy }))
		let report: unknown
		if (diff.value?.managed) {
			report = await invoke('plugin:ymcl|ymcl_publish_push', {
				instanceId: props.instanceId,
				version: version.value,
				channel: channel.value || null,
				bind,
				features: featurePayload,
				policies: policyPayload,
			})
		} else {
			// packId 不传：由启动器生成 `slug-<uuid>`，避免重名冲突。
			report = await invoke('plugin:ymcl|ymcl_publish_initial', {
				instanceId: props.instanceId,
				version: version.value,
				channel: channel.value || null,
				bind,
				features: featurePayload,
				policies: policyPayload,
				exclude: diff.value?.managed ? [] : excludedContentPaths.value,
			})
		}
		const publishedPackId =
			(report as { packId?: string } | null)?.packId ??
			diff.value?.pack_id ??
			''
		// 通知按实际 bind 结果出文案：「已绑定」是玩家能看到包的承诺，
		// 未绑定时必须明说，否则管理员会误以为玩家侧已经可以下载。
		const boundServer = bind
			? servers.value.find((server) => server.serverId === serverId.value)
			: null
		const titleMessage = !bind
			? messages.publishedNoBind
			: bindOptional.value
				? messages.publishedOptional
				: diff.value?.managed
					? messages.published
					: messages.publishedInitial
		addNotification({
			title: formatMessage(titleMessage),
			text: boundServer
				? `${boundServer.name || boundServer.serverId} · ${publishedPackId} → ${version.value}`
				: `${publishedPackId} → ${version.value}`,
			type: bind ? 'success' : 'warn',
		})
		console.debug('publish report', report)
		modal.value?.hide()
	} catch (error) {
		addNotification({
			title: formatMessage(messages.pushFailed),
			text: ymclErrorMessage(error),
			type: 'error',
		})
	} finally {
		pushing.value = false
	}
}

defineExpose({
	show: () => {
		diff.value = null
		version.value = ''
		channel.value = 'stable'
		bindOptional.value = false
		features.value = []
		policies.value = []
		publishedVersions.value = []
		withdrawingVersion.value = null
		modal.value?.show()
		void checkDiff()
		void loadServers()
		void initContentTree()
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
		<div class="flex min-h-40 flex-col gap-4">
			<p class="m-0 text-sm leading-relaxed text-secondary">
				{{ instanceName }} · {{ formatMessage(messages.description) }}
			</p>

			<div
				v-if="gate"
				class="flex items-center gap-2 rounded-xl bg-bg-raised p-4 text-sm text-secondary"
			>
				<TriangleAlertIcon class="h-5 w-5 shrink-0" />
				{{ formatMessage(gate === 'domain' ? messages.needsDomain : messages.needsLogin) }}
			</div>

			<template v-else>
				<div v-if="checking" class="flex items-center gap-2 text-sm text-secondary">
					<LoaderIcon class="h-5 w-5 animate-spin" />
					{{ formatMessage(messages.loading) }}
				</div>

				<template v-else-if="diff">
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

					<div class="flex items-center justify-between gap-2">
						<p class="m-0 text-sm font-semibold text-contrast">
							{{ formatMessage(messages.detailsTitle) }}
						</p>
						<ButtonStyled size="small" type="outlined">
							<button :disabled="diffRows.length === 0" @click="showChanges">
								{{ formatMessage(messages.viewChanges) }}（{{ diffRows.length }}）
							</button>
						</ButtonStyled>
					</div>

					<div
						v-if="diff.managed"
						class="flex flex-col gap-2 rounded-xl bg-bg-raised p-3"
					>
						<div class="flex items-center justify-between">
							<p class="m-0 text-sm font-semibold text-contrast">
								{{ formatMessage(messages.withdrawTitle) }}
							</p>
						</div>
						<p class="m-0 text-xs text-secondary">
							{{ formatMessage(messages.withdrawHint) }}
						</p>
						<p
							v-if="publishedVersions.length === 0"
							class="m-0 text-xs text-secondary"
						>
							{{ formatMessage(messages.versionsEmpty) }}
						</p>
						<div
							v-for="entry in publishedVersions"
							:key="entry.version"
							class="flex items-center justify-between gap-2 rounded-xl bg-bg px-3 py-2 text-sm"
						>
							<div class="flex min-w-0 flex-col">
								<span class="truncate font-medium text-contrast">
									{{ entry.version }}
								</span>
								<span v-if="entry.channel" class="text-xs text-secondary">
									{{ entry.channel }}
								</span>
							</div>
							<ButtonStyled size="small" color="red">
								<button
									:disabled="
										withdrawingVersion !== null || pushing || checking
									"
									@click="withdrawVersion(entry.version)"
								>
									{{
										withdrawingVersion === entry.version
											? formatMessage(messages.withdrawing)
											: formatMessage(messages.withdraw)
									}}
								</button>
							</ButtonStyled>
						</div>
					</div>

					<p
						v-if="!diff.managed && diff.added.length > 0"
						class="m-0 rounded-xl bg-bg-raised p-3 text-sm text-secondary"
					>
						{{ formatMessage(messages.initialMode) }}
					</p>

					<div class="flex flex-col gap-2 rounded-xl bg-bg-raised p-3">
						<div class="flex items-center justify-between">
							<p class="m-0 text-sm font-semibold text-contrast">
								{{ formatMessage(messages.bindLabel) }}
							</p>
							<ButtonStyled v-if="serversError" size="small" type="outlined">
								<button :disabled="checking || pushing" @click="loadServers">
									{{ formatMessage(messages.retry) }}
								</button>
							</ButtonStyled>
						</div>
						<p v-if="serversError" class="m-0 text-xs text-red">
							{{ formatMessage(messages.serversErrorLabel) }}：{{ serversError }}
						</p>
						<template v-else-if="servers.length > 0">
							<Combobox
								:model-value="serverId"
								:options="serverOptions"
								:disabled="pushing"
								class="w-full"
								@update:model-value="(value: string) => (serverId = value)"
							/>
							<div v-if="serverId" class="flex flex-col gap-2">
								<p class="m-0 text-xs font-semibold text-secondary">
									{{ formatMessage(messages.bindModeLabel) }}
								</p>
								<label
									class="flex cursor-pointer items-start gap-2 rounded-xl bg-bg p-3 text-sm"
									:class="!bindOptional ? 'ring-1 ring-brand' : ''"
								>
									<input
										v-model="bindOptional"
										type="radio"
										:value="false"
										:disabled="pushing"
										class="mt-1"
									/>
									<span class="flex flex-col gap-1">
										<span>{{ formatMessage(messages.bindModeRequired) }}</span>
										<span class="text-xs text-secondary">
											{{ formatMessage(messages.bindHintRequired) }}
										</span>
									</span>
								</label>
								<label
									class="flex cursor-pointer items-start gap-2 rounded-xl bg-bg p-3 text-sm"
									:class="bindOptional ? 'ring-1 ring-brand' : ''"
								>
									<input
										v-model="bindOptional"
										type="radio"
										:value="true"
										:disabled="pushing"
										class="mt-1"
									/>
									<span class="flex flex-col gap-1">
										<span>{{ formatMessage(messages.bindModeOptional) }}</span>
										<span class="text-xs text-secondary">
											{{ formatMessage(messages.bindHintOptional) }}
										</span>
									</span>
								</label>
							</div>
						</template>
						<p v-else class="m-0 text-xs text-secondary">
							{{ formatMessage(messages.serversEmpty) }}
						</p>
					</div>

					<div
						v-if="totalChanges > 0"
						class="flex flex-col gap-2 rounded-xl bg-bg-raised p-3"
					>
						<div class="flex items-center justify-between">
							<p class="m-0 text-sm font-semibold text-contrast">
								{{ formatMessage(messages.featuresTitle) }}
							</p>
							<ButtonStyled size="small" type="outlined">
								<button @click="addFeature">
									<PlusIcon />
									{{ formatMessage(messages.addFeature) }}
								</button>
							</ButtonStyled>
						</div>
						<p class="m-0 text-xs text-secondary">
							{{
								formatMessage(
									diff.managed
										? messages.featuresHintManaged
										: messages.featuresHint,
								)
							}}
						</p>
						<div
							v-for="(feature, index) in features"
							:key="index"
							class="flex flex-col gap-2 rounded-lg bg-bg p-3"
						>
							<div class="flex items-center gap-2">
								<StyledInput
									v-model="feature.id"
									:disabled="pushing"
									:placeholder="formatMessage(messages.featureIdRequired)"
									wrapper-class="w-36"
								/>
								<StyledInput
									v-model="feature.name"
									:disabled="pushing"
									:placeholder="formatMessage(messages.featureNameLabel)"
									wrapper-class="min-w-0 flex-1"
								/>
								<ButtonStyled type="transparent" size="small">
									<button
										v-tooltip="formatMessage(messages.deleted)"
										:disabled="pushing"
										@click="removeFeature(index)"
									>
										<TrashIcon />
									</button>
								</ButtonStyled>
							</div>
							<div class="labeled_input w-full">
								<div class="flex items-center justify-between">
									<p class="text-xs font-semibold text-secondary">
										{{ formatMessage(messages.featureGlobsLabel) }}
									</p>
									<ButtonStyled type="transparent" size="small">
										<button :disabled="pushing" @click="openFilePicker(index)">
											<FolderOpenIcon />
											{{ formatMessage(messages.pickFiles) }}
										</button>
									</ButtonStyled>
								</div>
								<textarea
									v-model="feature.globsText"
									:disabled="pushing"
									rows="2"
									class="w-full rounded-lg bg-bg-raised px-3 py-2 font-mono text-xs text-contrast"
									:placeholder="'mods/iris-*.jar\nshaderpacks/**'"
								/>
								<p
									class="m-0 text-xs"
									:class="
										featureMatchCount(feature) > 0 ? 'text-green' : 'text-red'
									"
								>
									{{
										formatMessage(messages.featureMatchCount, {
											count: featureMatchCount(feature),
										})
									}}
								</p>
							</div>
							<div class="flex flex-wrap items-center gap-3">
								<label class="flex items-center gap-1 text-xs text-secondary">
									<Checkbox
										v-model="feature.defaultChecked"
										:disabled="pushing"
									/>
									{{ formatMessage(messages.featureDefaultLabel) }}
								</label>
								<template
									v-if="features.filter((other) => other !== feature).length > 0"
								>
									<span class="text-xs text-secondary">
										{{ formatMessage(messages.featureConflictsLabel) }}:
									</span>
									<label
										v-for="other in features.filter((item) => item !== feature)"
										:key="other.id + features.indexOf(other)"
										class="flex items-center gap-1 text-xs text-secondary"
									>
										<input
											type="checkbox"
											:checked="feature.conflicts.includes(other.id.trim())"
											:disabled="pushing || !other.id.trim()"
											@change="
												(event) => {
													const id = other.id.trim()
													const checked = (
														event.target as HTMLInputElement
													).checked
													if (checked) feature.conflicts.push(id)
													else
														feature.conflicts = feature.conflicts.filter(
															(item) => item !== id,
														)
												}
											"
										/>
										{{ other.id.trim() || '…' }}
									</label>
								</template>
							</div>
						</div>
						<p v-if="featureIssue" class="m-0 text-xs text-red">
							{{ featureIssue }}
						</p>
						<p
							v-else-if="featureNoMatchWarning"
							class="m-0 text-xs text-orange"
						>
							{{ featureNoMatchWarning }}
						</p>
					</div>

					<div
						v-if="totalChanges > 0"
						class="flex flex-col gap-2 rounded-xl bg-bg-raised p-3"
					>
						<div class="flex items-center justify-between">
							<p class="m-0 text-sm font-semibold text-contrast">
								{{ formatMessage(messages.policiesTitle) }}
							</p>
							<ButtonStyled size="small" type="outlined">
								<button @click="addPolicy">
									<PlusIcon />
									{{ formatMessage(messages.addPolicy) }}
								</button>
							</ButtonStyled>
						</div>
						<p class="m-0 text-xs text-secondary">
							{{ formatMessage(messages.policiesHint) }}
						</p>
						<div
							v-for="(rule, index) in policies"
							:key="index"
							class="flex flex-col gap-1"
						>
							<div class="flex items-center gap-2">
								<input
									v-model="rule.glob"
									:disabled="pushing"
									class="min-w-0 flex-1 rounded-lg bg-bg-raised px-3 py-2 font-mono text-xs text-contrast"
									placeholder="config/**（支持 * / ** / ?，含文件夹）"
								/>
								<Combobox
									:model-value="rule.policy"
									:options="policyOptions"
									:disabled="pushing"
									class="w-32 shrink-0"
									@update:model-value="(value: string) => (rule.policy = value)"
								/>
								<ButtonStyled type="transparent" size="small">
									<button
										v-tooltip="formatMessage(messages.deleted)"
										:disabled="pushing"
										@click="removePolicy(index)"
									>
										<TrashIcon />
									</button>
								</ButtonStyled>
							</div>
							<p
								class="m-0 text-xs"
								:class="
									policyMatchCount(rule) > 0 ? 'text-green' : 'text-orange'
								"
							>
								{{
									formatMessage(messages.featureMatchCount, {
										count: policyMatchCount(rule),
									})
								}}
							</p>
						</div>
					</div>

					<template v-if="totalChanges > 0">
						<div
							v-if="!diff.managed && contentFiles.length > 0"
							class="flex flex-col gap-2 rounded-xl bg-bg-raised p-3"
						>
							<p class="m-0 text-sm font-semibold text-contrast">
								{{ formatMessage(messages.contentTitle) }}
							</p>
							<p class="m-0 text-xs text-secondary">
								{{ formatMessage(messages.contentHint) }}
							</p>
							<FileTreeSelect
								:key="contentTreeKey"
								v-model="selectedContentPaths"
								class="min-w-0"
								:items="contentFiles"
								@navigate="loadContentDirectory"
							/>
						</div>
						<div class="grid grid-cols-2 gap-4">
							<div class="labeled_input w-full">
								<p class="text-contrast font-semibold">
									{{ formatMessage(messages.versionLabel) }}
								</p>
								<StyledInput v-model="version" :disabled="pushing" wrapper-class="w-full" />
							</div>
							<div class="labeled_input w-full">
								<p class="text-contrast font-semibold">
									{{ formatMessage(messages.channelLabel) }}
								</p>
								<StyledInput v-model="channel" :disabled="pushing" wrapper-class="w-full" />
							</div>
						</div>
					</template>
					<p v-else class="m-0 text-sm text-secondary">
						{{ formatMessage(messages.nothingToDo) }}
					</p>
				</template>
			</template>
		</div>
		<template #actions>
			<div class="flex items-center justify-end gap-2">
				<ButtonStyled type="outlined">
					<button :disabled="checking || pushing" @click="checkDiff">
						<LoaderIcon v-if="checking" class="animate-spin" />
						{{ formatMessage(diff ? messages.recheck : messages.checkDiff) }}
					</button>
				</ButtonStyled>
				<ButtonStyled
					v-if="diff && (diff.managed ? totalChanges > 0 : diff.added.length > 0)"
					color="brand"
				>
					<button
						:disabled="
							pushing ||
							!version ||
							!!featureIssue ||
							(!diff.managed && contentFiles.length > 0 && selectedContentPaths.length === 0)
						"
						@click="push"
					>
						<LoaderIcon v-if="pushing" class="animate-spin" />
						<UploadIcon v-else />
						{{ formatMessage(diff.managed ? messages.push : messages.pushInitial) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
	<PublishChangeListModal
		ref="changeListModal"
		:instance-id="instanceId"
		@pick="onPickFiles"
	/>
</template>
