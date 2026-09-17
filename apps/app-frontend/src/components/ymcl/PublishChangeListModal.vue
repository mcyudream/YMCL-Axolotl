<script setup lang="ts">
import { LoaderIcon, TriangleAlertIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	defineMessages,
	NewModal,
	useFormatBytes,
	useVIntl,
} from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import { join } from '@tauri-apps/api/path'
import { computed, ref } from 'vue'

import StudioEditor from '@/components/instance/studio/StudioEditor.vue'
import { get_full_path } from '@/helpers/instance'
import { readStudioText } from '@/helpers/studio'

/**
 * Large change-list modal for the publish flow (YAP §7): the full diff list
 * on the left, a read-only preview of the selected file on the right using
 * the built-in studio viewer components. Read-only: nothing here writes.
 */

const props = defineProps<{
	instanceId: string
}>()

const emit = defineEmits<{ pick: [paths: string[]] }>()

const { formatMessage } = useVIntl()
const formatBytes = useFormatBytes()

const messages = defineMessages({
	title: {
		id: 'app.ymcl.change-list.title',
		defaultMessage: '变更明细',
	},
	empty: {
		id: 'app.ymcl.change-list.empty',
		defaultMessage: '没有变更',
	},
	changed: { id: 'app.ymcl.publish.changed', defaultMessage: '修改' },
	added: { id: 'app.ymcl.publish.added', defaultMessage: '新增' },
	deleted: { id: 'app.ymcl.publish.deleted', defaultMessage: '删除' },
	moved: { id: 'app.ymcl.publish.moved', defaultMessage: '移动' },
	previewHint: {
		id: 'app.ymcl.change-list.preview-hint',
		defaultMessage: '在左侧选择一个文件以预览内容。',
	},
	previewDeleted: {
		id: 'app.ymcl.change-list.preview-deleted',
		defaultMessage: '该文件在本地已删除，没有内容可预览。',
	},
	previewUnsupported: {
		id: 'app.ymcl.change-list.preview-unsupported',
		defaultMessage: '该文件类型暂不支持预览。',
	},
	previewFailed: {
		id: 'app.ymcl.change-list.preview-failed',
		defaultMessage: '读取文件失败。',
	},
	previewBinary: {
		id: 'app.ymcl.change-list.preview-binary',
		defaultMessage: '二进制文件暂不支持预览。',
	},
	pickTitle: {
		id: 'app.ymcl.change-list.pick-title',
		defaultMessage: '选择文件',
	},
	pickHint: {
		id: 'app.ymcl.change-list.pick-hint',
		defaultMessage: '勾选要加入该可选内容的文件（精确路径），确定后填入 glob 列表；也可以继续手工写通配符。',
	},
	pickedCount: {
		id: 'app.ymcl.change-list.picked-count',
		defaultMessage: '已选 {count} 个文件',
	},
	confirmPick: {
		id: 'app.ymcl.change-list.pick-confirm',
		defaultMessage: '确定',
	},
})

export interface ChangeListEntry {
	path: string
	sha512?: string | null
	from?: string | null
	size?: number | null
}

export interface ChangeListDiff {
	changed: ChangeListEntry[]
	added: ChangeListEntry[]
	deleted: string[]
	moved: ChangeListEntry[]
}

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const diff = ref<ChangeListDiff | null>(null)

type RowKind = 'changed' | 'added' | 'deleted' | 'moved'

interface DiffRow {
	kind: RowKind
	path: string
	from?: string | null
	size?: number | null
}

const rows = computed<DiffRow[]>(() => {
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

const rowKindMessages = {
	changed: messages.changed,
	added: messages.added,
	deleted: messages.deleted,
	moved: messages.moved,
}

const rowKindStyles: Record<RowKind, string> = {
	changed: 'bg-surface-4 text-secondary',
	added: 'bg-green/15 text-green',
	deleted: 'bg-red/15 text-red',
	moved: 'bg-blue/15 text-blue',
}

const selectedPath = ref<string | null>(null)

/** 挑选模式：勾选文件后由父组件收走精确路径（填充可选内容）。 */
const pickMode = ref(false)
const picked = ref<Set<string>>(new Set())

function togglePick(path: string) {
	const next = new Set(picked.value)
	if (next.has(path)) next.delete(path)
	else next.add(path)
	picked.value = next
}

function confirmPick() {
	emit('pick', [...picked.value])
	modal.value?.hide()
}

type PreviewState =
	| { kind: 'idle' }
	| { kind: 'loading' }
	| { kind: 'deleted' }
	| { kind: 'unsupported' }
	| { kind: 'failed' }
	| { kind: 'media'; media: 'image' | 'video'; url: string }
	| { kind: 'text'; content: string; language: string }

const preview = ref<PreviewState>({ kind: 'idle' })
let previewToken = 0

const BINARY_EXTENSIONS = /\.(jar|zip|7z|rar|gz|xz|zst|lz4|dat|nbt|mca|mclevel|bin|exe|dll|so|dylib)$/i
const IMAGE_EXTENSIONS = /\.(png|jpe?g|gif|webp|svg|bmp|ico|avif)$/i
const VIDEO_EXTENSIONS = /\.(mp4|webm|ogv|mov|m4v)$/i

function previewLanguage(path: string): string {
	const extension = path.split('.').pop()?.toLowerCase()
	if (extension === 'yml') return 'yaml'
	return extension ?? 'plaintext'
}

async function openPreview(row: DiffRow) {
	selectedPath.value = row.path
	const token = ++previewToken
	if (row.kind === 'deleted') {
		preview.value = { kind: 'deleted' }
		return
	}
	if (BINARY_EXTENSIONS.test(row.path)) {
		preview.value = { kind: 'unsupported' }
		return
	}
	if (IMAGE_EXTENSIONS.test(row.path) || VIDEO_EXTENSIONS.test(row.path)) {
		preview.value = { kind: 'loading' }
		try {
			const root = await get_full_path(props.instanceId)
			const url = convertFileSrc(await join(root, row.path))
			// 只有用最终值覆盖才算数：竞态时丢弃过期结果。
			if (token === previewToken) {
				preview.value = {
					kind: 'media',
					media: VIDEO_EXTENSIONS.test(row.path) ? 'video' : 'image',
					url,
				}
			}
		} catch {
			if (token === previewToken) preview.value = { kind: 'failed' }
		}
		return
	}
	preview.value = { kind: 'loading' }
	try {
		const content = await readStudioText(props.instanceId, row.path)
		if (token === previewToken) {
			preview.value = {
				kind: 'text',
				content,
				language: previewLanguage(row.path),
			}
		}
	} catch {
		if (token === previewToken) preview.value = { kind: 'unsupported' }
	}
}

function selectRow(row: DiffRow) {
	if (pickMode.value) togglePick(row.path)
	if (row.path !== selectedPath.value) void openPreview(row)
}

/** 挑选模式下只列出可归属的文件（新增/修改/移动）。 */
const visibleRows = computed(() =>
	pickMode.value ? rows.value.filter((row) => row.kind !== 'deleted') : rows.value,
)

defineExpose({
	show: (
		changeDiff: ChangeListDiff,
		options?: { pickMode?: boolean; selected?: string[] },
	) => {
		diff.value = changeDiff
		selectedPath.value = null
		preview.value = { kind: 'idle' }
		pickMode.value = !!options?.pickMode
		picked.value = new Set(options?.selected ?? [])
		modal.value?.show()
		// 默认选中第一个可预览的文件。
		const first = visibleRows.value.find((row) => row.kind !== 'deleted')
		if (first) void openPreview(first)
	},
})
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(pickMode ? messages.pickTitle : messages.title)"
		scrollable
		width="60rem"
		max-width="calc(100vw - 2rem)"
	>
		<div class="grid min-h-[26rem] grid-cols-1 gap-4 md:grid-cols-2">
			<div class="flex min-w-0 flex-col gap-2">
						<p
							v-if="pickMode"
							class="m-0 text-xs text-secondary"
						>
							{{ formatMessage(messages.pickHint) }}
						</p>
						<div
							v-if="rows.length === 0"
							class="flex h-full items-center justify-center rounded-xl bg-bg-raised p-6 text-sm text-secondary"
						>
							{{ formatMessage(messages.empty) }}
						</div>
						<div v-else class="max-h-[60vh] overflow-y-auto rounded-xl bg-bg-raised">
							<button
								v-for="row in visibleRows"
								:key="row.kind + row.path"
								class="flex w-full cursor-pointer items-center gap-2 border-b border-solid border-bg px-3 py-1.5 text-left text-sm hover:bg-surface-4"
								:class="selectedPath === row.path ? 'bg-surface-4' : ''"
								@click="selectRow(row)"
							>
								<input
									v-if="pickMode"
									type="checkbox"
									class="shrink-0"
									:checked="picked.has(row.path)"
									@click.stop
									@change="togglePick(row.path)"
								/>
								<span
									:class="[
										'shrink-0 rounded-full px-2 py-0.5 text-xs font-medium',
										rowKindStyles[row.kind],
									]"
								>
									{{ formatMessage(rowKindMessages[row.kind]) }}
								</span>
								<span
									class="min-w-0 flex-1 truncate font-mono text-xs text-contrast"
									:title="row.from ? `${row.from} → ${row.path}` : row.path"
								>
									<template v-if="row.from">
										<span class="text-secondary">{{ row.from }} → </span>
									</template>
									{{ row.path }}
								</span>
								<span v-if="row.size != null" class="shrink-0 text-xs text-secondary">
									{{ formatBytes(row.size) }}
								</span>
							</button>
						</div>
					</div>

			<div
				class="flex min-h-[24rem] min-w-0 flex-col overflow-hidden rounded-xl bg-bg-raised"
			>
				<template v-if="selectedPath">
					<p
						class="shrink-0 truncate border-0 border-b border-solid border-bg px-3 py-2 font-mono text-xs text-secondary"
						:title="selectedPath"
					>
						{{ selectedPath }}
					</p>
					<div class="relative min-h-0 flex-1">
						<div
							v-if="preview.kind === 'loading'"
							class="flex h-full items-center justify-center gap-2 text-sm text-secondary"
						>
							<LoaderIcon class="h-5 w-5 animate-spin" />
						</div>
						<div
							v-else-if="
								preview.kind === 'deleted' ||
								preview.kind === 'unsupported' ||
								preview.kind === 'failed'
							"
							class="flex h-full flex-col items-center justify-center gap-2 p-6 text-center text-sm text-secondary"
						>
							<TriangleAlertIcon class="h-6 w-6" />
							{{
								preview.kind === 'deleted'
									? formatMessage(messages.previewDeleted)
									: preview.kind === 'failed'
										? formatMessage(messages.previewFailed)
										: formatMessage(messages.previewBinary)
							}}
						</div>
						<img
							v-else-if="preview.kind === 'media' && preview.media === 'image'"
							:src="preview.url"
							class="h-full w-full object-contain"
							alt=""
						/>
						<video
							v-else-if="preview.kind === 'media' && preview.media === 'video'"
							:src="preview.url"
							class="h-full w-full object-contain"
							controls
						/>
						<StudioEditor
							v-else-if="preview.kind === 'text'"
							:content="preview.content"
							:file-path="selectedPath"
							:language="preview.language"
							read-only
							class="h-full"
						/>
					</div>
				</template>
				<div
					v-else
					class="flex h-full items-center justify-center p-6 text-center text-sm text-secondary"
				>
					{{ formatMessage(messages.previewHint) }}
				</div>
			</div>
		</div>
		<template v-if="pickMode" #actions>
			<div class="flex w-full items-center justify-between gap-2">
				<span class="text-sm text-secondary">
					{{ formatMessage(messages.pickedCount, { count: picked.size }) }}
				</span>
				<ButtonStyled color="brand">
					<button @click="confirmPick">
						{{ formatMessage(messages.confirmPick) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>
