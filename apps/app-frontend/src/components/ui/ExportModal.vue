<script setup>
import { FolderOpenIcon, XIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	Combobox,
	commonMessages,
	defineMessages,
	FileTreeSelect,
	injectNotificationManager,
	injectPopupNotificationManager,
	LoadingIndicator,
	modSideMessages,
	NewModal,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { join } from '@tauri-apps/api/path'
import { save } from '@tauri-apps/plugin-dialog'
import { readDir, stat } from '@tauri-apps/plugin-fs'
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'

import { PackageIcon } from '@/assets/icons'
import {
	export_instance_mrpack,
	get_full_path,
	get_pack_export_candidates,
} from '@/helpers/instance'
import { exportServerPack, previewServerPack } from '@/helpers/mod-sides'
import { highlightInFolder } from '@/helpers/utils'

const { handleError } = injectNotificationManager()
const popupNotificationManager = injectPopupNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	filesError: {
		id: 'app.export-modal.files-error',
		defaultMessage: 'Could not read the instance files.',
	},
	retry: { id: 'app.export-modal.retry', defaultMessage: 'Retry' },
	refreshPreview: {
		id: 'app.export-modal.refresh-preview',
		defaultMessage: 'Refresh file preview',
	},
	format: { id: 'app.export-modal.format', defaultMessage: 'Package format' },
	modrinth: { id: 'app.export-modal.format-modrinth', defaultMessage: 'Modrinth (.mrpack)' },
	curseforge: { id: 'app.export-modal.format-curseforge', defaultMessage: 'CurseForge (.zip)' },
	generic: { id: 'app.export-modal.format-generic', defaultMessage: 'Generic ZIP (.zip)' },
	server_full: {
		id: 'app.export-modal.format-server-full',
		defaultMessage: 'Full server pack (.zip)',
	},
	server_slim: {
		id: 'app.export-modal.format-server-slim',
		defaultMessage: 'Slim server pack (.zip)',
	},
	serverDescription: {
		id: 'app.export-modal.server-description',
		defaultMessage:
			'Exports enabled server-compatible and unknown-side mods with server configs and scripts. Client-only mods, client scripts, worlds and player data are excluded. Install the matching server core separately; launch scripts require Python 3.9+ and Java.',
	},
	slimDescription: {
		id: 'app.export-modal.server-slim-description',
		defaultMessage:
			'Downloads mods with verified source URLs on first launch and checks SHA-256. Local mods and mods without a usable download URL are bundled.',
	},
	serverSummary: {
		id: 'app.export-modal.server-summary',
		defaultMessage:
			'{mods, number} mods selected · {unknown, number} unknown side · {downloads, number} downloaded on first launch · {excluded, number} excluded',
	},
	serverMissing: {
		id: 'app.export-modal.server-missing',
		defaultMessage:
			'{count, number} required mod files are missing from this instance. Complete the downloads before exporting a usable server pack.',
	},
	excludedMods: {
		id: 'app.export-modal.server-excluded-mods',
		defaultMessage: 'Excluded mods ({count, number})',
	},
	unknownMods: {
		id: 'app.export-modal.server-unknown-mods',
		defaultMessage: 'Review unknown-side mods ({count, number})',
	},
	disabledMod: {
		id: 'app.export-modal.server-disabled-mod',
		defaultMessage: 'Disabled',
	},
	dependencyMissing: {
		id: 'app.export-modal.server-dependency-missing',
		defaultMessage:
			'Required dependencies are excluded. Select them or correct their mod side before exporting:',
	},
	genericDescription: {
		id: 'app.export-modal.generic-description',
		defaultMessage:
			'Contains the selected files. The recipient configures Minecraft and the mod loader.',
	},
	header: { id: 'app.export-modal.header', defaultMessage: 'Export modpack' },
	modpackNameLabel: { id: 'app.export-modal.modpack-name-label', defaultMessage: 'Modpack name' },
	modpackNamePlaceholder: {
		id: 'app.export-modal.modpack-name-placeholder',
		defaultMessage: 'Modpack name',
	},
	versionNumberLabel: {
		id: 'app.export-modal.version-number-label',
		defaultMessage: 'Version number',
	},
	versionNumberPlaceholder: {
		id: 'app.export-modal.version-number-placeholder',
		defaultMessage: '1.0.0',
	},
	descriptionPlaceholder: {
		id: 'app.export-modal.description-placeholder',
		defaultMessage: 'Enter modpack description...',
	},
	exportButton: { id: 'app.export-modal.export-button', defaultMessage: 'Export' },
	exportComplete: {
		id: 'app.export-modal.export-complete',
		defaultMessage: 'Export complete',
	},
	exportCompleteDescription: {
		id: 'app.export-modal.export-complete-description',
		defaultMessage: '{name} was exported successfully.',
	},
})

const props = defineProps({
	instance: {
		type: Object,
		required: true,
	},
})

defineExpose({
	show: (initialFormat = '') => {
		isOpen.value = false
		if (
			['modrinth', 'curseforge', 'generic', 'server_full', 'server_slim'].includes(initialFormat)
		) {
			format.value = initialFormat
		}
		resetExportState()
		isOpen.value = true
		exportModal.value.show()
		void initFiles()
	},
})

const exportModal = ref(null)
const format = ref('modrinth')
const isOpen = ref(false)
const isServerFormat = computed(
	() => format.value === 'server_full' || format.value === 'server_slim',
)
const serverMode = computed(() => (format.value === 'server_slim' ? 'slim' : 'full'))
const serverPreview = ref(null)
const exporting = ref(false)
const filesLoading = ref(false)
const filesError = ref(false)
const fileErrorMessage = ref('')
const filesInstanceId = ref('')
const nameInput = ref(props.instance.name)
const exportDescription = ref('')
const versionInput = ref('1.0.0')
const files = ref([])
const selectedFilePaths = ref([])
const fileTreeKey = ref(0)
const filesLoadId = ref(0)
const instanceRoot = ref('')
const loadedDirectories = ref(new Set())
const selectedServerFiles = computed(
	() =>
		serverPreview.value?.files.filter((file) => selectedFilePaths.value.includes(file.path)) ?? [],
)
const unknownServerFiles = computed(() =>
	selectedServerFiles.value.filter((file) => file.modSide?.side === 'unknown'),
)
const serverSummary = computed(() => ({
	mods: selectedServerFiles.value.filter((file) => file.modSide).length,
	unknown: selectedServerFiles.value.filter((file) => file.modSide?.side === 'unknown').length,
	downloads: selectedServerFiles.value.filter((file) => file.downloads.length > 0).length,
	excluded: serverPreview.value?.excludedMods.length ?? 0,
}))
const dependencyIssues = computed(() => {
	const selected = new Set(selectedFilePaths.value)
	return selectedServerFiles.value.flatMap((file) =>
		file.requires.filter((path) => !selected.has(path)).map((path) => `${file.path} → ${path}`),
	)
})

watch(
	format,
	() => {
		if (isOpen.value) void initFiles()
	},
	{ flush: 'sync' },
)
watch(
	() => props.instance.id,
	() => {
		if (exporting.value) return
		resetExportState()
		if (isOpen.value) void initFiles()
	},
)

async function initFiles(preserveSelection = false) {
	const previousPaths = new Set(files.value.map((file) => file.path))
	const previousSelected = new Set(selectedFilePaths.value)
	filesLoading.value = true
	filesError.value = false
	fileErrorMessage.value = ''
	filesInstanceId.value = ''
	const loadId = ++filesLoadId.value
	const instanceId = props.instance.id
	files.value = []
	selectedFilePaths.value = []
	serverPreview.value = null
	fileTreeKey.value++
	try {
		if (isServerFormat.value) {
			const preview = await previewServerPack(instanceId, serverMode.value)
			if (loadId !== filesLoadId.value || instanceId !== props.instance.id) return
			serverPreview.value = preview
			filesInstanceId.value = instanceId
			files.value = preview.files.map((file) => ({
				path: file.path,
				type: 'file',
				size: file.size,
			}))
			selectedFilePaths.value = preview.files
				.filter((file) =>
					preserveSelection === true && previousPaths.has(file.path)
						? previousSelected.has(file.path)
						: file.selected,
				)
				.map((file) => file.path)
			return
		}
		const [filePaths, root] = await Promise.all([
			get_pack_export_candidates(instanceId),
			get_full_path(instanceId),
		])
		const exportCandidates = await Promise.all(
			filePaths.map((path) => buildExportCandidateItem(root, path)),
		)
		if (loadId !== filesLoadId.value || instanceId !== props.instance.id) return
		instanceRoot.value = root
		filesInstanceId.value = instanceId
		files.value = exportCandidates
		selectedFilePaths.value = files.value
			.filter((file) => !file.disabled && isDefaultSelectedExportCandidate(file.path))
			.map((file) => file.path)
	} catch (error) {
		if (loadId === filesLoadId.value) {
			filesError.value = true
			fileErrorMessage.value = error instanceof Error ? error.message : String(error)
		}
	} finally {
		if (loadId === filesLoadId.value) filesLoading.value = false
	}
}

const exportPack = async () => {
	if (
		exporting.value ||
		filesLoading.value ||
		filesError.value ||
		filesInstanceId.value !== props.instance.id
	)
		return
	exporting.value = true
	const instanceId = filesInstanceId.value
	try {
		const extension = format.value === 'modrinth' ? 'mrpack' : 'zip'
		const outputPath = await save({
			defaultPath: `${nameInput.value} ${versionInput.value}.${extension}`,
			filters: [{ name: formatMessage(messages[format.value]), extensions: [extension] }],
		})
		if (!outputPath) return
		if (instanceId !== props.instance.id) return
		if (isServerFormat.value) {
			await exportServerPack(instanceId, outputPath, {
				mode: serverMode.value,
				selectedPaths: selectedFilePaths.value,
				revision: serverPreview.value.revision,
				name: nameInput.value,
				version: versionInput.value,
				description: exportDescription.value,
			})
		} else
			await export_instance_mrpack(
				instanceId,
				outputPath,
				selectedFilePaths.value,
				versionInput.value,
				exportDescription.value,
				nameInput.value,
				format.value,
			)
		exporting.value = false
		await nextTick()
		exportModal.value.hide()
		const fileName = outputPath.split(/[\\/]/).pop() ?? outputPath
		popupNotificationManager.addPopupNotification({
			title: formatMessage(messages.exportComplete),
			text: formatMessage(messages.exportCompleteDescription, { name: fileName }),
			type: 'success',
			buttons: [
				{
					label: formatMessage(commonMessages.openInFolderButton),
					icon: FolderOpenIcon,
					action: () => highlightInFolder(outputPath).catch(handleError),
				},
			],
		})
	} catch (error) {
		handleError(error)
	} finally {
		exporting.value = false
		if (instanceId !== props.instance.id && isOpen.value) {
			resetExportState()
			void initFiles()
		}
	}
}

function resetExportState() {
	nameInput.value = props.instance.name
	exportDescription.value = ''
	versionInput.value = '1.0.0'
	files.value = []
	selectedFilePaths.value = []
	fileTreeKey.value += 1
	instanceRoot.value = ''
	loadedDirectories.value = new Set()
	filesError.value = false
	serverPreview.value = null
	filesInstanceId.value = ''
	fileErrorMessage.value = ''
}

function handleHide() {
	filesLoadId.value++
	isOpen.value = false
}

onBeforeUnmount(() => {
	filesLoadId.value++
})

async function loadExportDirectory(path) {
	if (isServerFormat.value) return
	if (!path || !instanceRoot.value || loadedDirectories.value.has(path)) return

	const loadId = filesLoadId.value
	loadedDirectories.value.add(path)

	try {
		const entries = await readDir(await join(instanceRoot.value, ...path.split('/')))
		const childItems = await Promise.all(
			entries.map((entry) => buildExportDirectoryChildItem(instanceRoot.value, path, entry)),
		)
		if (loadId !== filesLoadId.value) return

		appendExportItems(childItems)
	} catch {
		loadedDirectories.value.delete(path)
	}
}

async function buildExportCandidateItem(instanceRoot, path) {
	try {
		const entries = await readDir(await join(instanceRoot, ...path.split('/')))
		const metadata = await getExportCandidateMetadata(instanceRoot, path)
		return {
			path,
			type: 'directory',
			disabled: isExportCandidateDisabled(path),
			modified: metadata.modified,
			count: entries.length,
		}
	} catch {
		return buildExportFileItem(instanceRoot, path)
	}
}

async function buildExportDirectoryChildItem(instanceRoot, parentPath, entry) {
	const path = `${parentPath}/${entry.name}`
	if (entry.isDirectory) {
		const metadata = await getExportCandidateMetadata(instanceRoot, path)
		return {
			path,
			type: 'directory',
			disabled: isExportCandidateDisabled(path),
			modified: metadata.modified,
		}
	}

	return buildExportFileItem(instanceRoot, path)
}

async function buildExportFileItem(instanceRoot, path) {
	const metadata = await getExportCandidateMetadata(instanceRoot, path)
	return {
		path,
		type: 'file',
		disabled: isExportCandidateDisabled(path),
		size: metadata.size,
		modified: metadata.modified,
	}
}

function appendExportItems(items) {
	const nextFiles = new Map(files.value.map((file) => [normalizeExportPath(file.path), file]))
	for (const item of items) {
		nextFiles.set(normalizeExportPath(item.path), item)
	}
	files.value = [...nextFiles.values()]
}

async function getExportCandidateMetadata(instanceRoot, path) {
	try {
		const metadata = await stat(await join(instanceRoot, ...path.split('/')))
		return {
			size: metadata.size,
			modified: metadata.mtime ? Math.floor(metadata.mtime.getTime() / 1000) : undefined,
		}
	} catch {
		return {}
	}
}

function normalizeExportPath(path) {
	return path.replaceAll('\\', '/').split('/').filter(Boolean).join('/')
}

function isDefaultSelectedExportCandidate(path) {
	return (
		path.startsWith('mods') ||
		path.startsWith('datapacks') ||
		path.startsWith('resourcepacks') ||
		path.startsWith('shaderpacks') ||
		path.startsWith('config')
	)
}

function isExportCandidateDisabled(path) {
	return (
		path === 'profile.json' ||
		path.startsWith('modrinth_logs') ||
		path.startsWith('.fabric') ||
		path.startsWith('__MACOSX')
	)
}
</script>

<template>
	<NewModal
		ref="exportModal"
		:header="formatMessage(messages.header)"
		scrollable
		:disable-close="exporting"
		width="46rem"
		max-width="calc(100vw - 2rem)"
		@hide="handleHide"
	>
		<fieldset
			:disabled="exporting"
			:inert="exporting"
			class="m-0 flex min-w-0 flex-col gap-4 border-0 p-0"
			data-onboarding-id="pack-export-formats"
		>
			<label class="flex flex-col gap-2 font-semibold text-contrast">
				{{ formatMessage(messages.format) }}
				<Combobox
					v-model="format"
					:disabled="exporting"
					:options="
						['modrinth', 'curseforge', 'generic', 'server_full', 'server_slim'].map((value) => ({
							value,
							label: formatMessage(messages[value]),
						}))
					"
				/>
			</label>
			<p v-if="format === 'generic'" class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.genericDescription) }}
			</p>
			<template v-if="isServerFormat">
				<p class="m-0 text-sm text-secondary">{{ formatMessage(messages.serverDescription) }}</p>
				<p v-if="format === 'server_slim'" class="m-0 text-sm text-secondary">
					{{ formatMessage(messages.slimDescription) }}
				</p>
				<p v-if="serverPreview" class="m-0 text-sm text-secondary">
					{{ formatMessage(messages.serverSummary, serverSummary) }}
				</p>
				<div class="flex justify-end">
					<ButtonStyled type="outlined" size="small">
						<button type="button" :disabled="exporting || filesLoading" @click="initFiles(true)">
							{{ formatMessage(messages.refreshPreview) }}
						</button>
					</ButtonStyled>
				</div>
				<details v-if="unknownServerFiles.length" class="text-sm text-secondary">
					<summary class="cursor-pointer">
						{{ formatMessage(messages.unknownMods, { count: unknownServerFiles.length }) }}
					</summary>
					<ul class="my-2 max-h-32 overflow-y-auto">
						<li v-for="file in unknownServerFiles" :key="file.path" class="break-all">
							{{ file.path }}
						</li>
					</ul>
				</details>
				<details v-if="serverPreview?.excludedMods.length" class="text-sm text-secondary">
					<summary class="cursor-pointer">
						{{ formatMessage(messages.excludedMods, { count: serverPreview.excludedMods.length }) }}
					</summary>
					<ul class="my-2 max-h-32 overflow-y-auto">
						<li v-for="file in serverPreview.excludedMods" :key="file.path" class="break-all">
							{{ file.path }} ·
							{{
								formatMessage(
									file.reason === 'client_only' ? modSideMessages.client : messages.disabledMod,
								)
							}}
						</li>
					</ul>
				</details>
				<div v-if="serverPreview?.missingMods.length" class="text-sm text-orange" role="alert">
					<p class="m-0">
						{{ formatMessage(messages.serverMissing, { count: serverPreview.missingMods.length }) }}
					</p>
					<ul class="my-2 max-h-32 overflow-y-auto">
						<li v-for="path in serverPreview.missingMods" :key="path" class="break-all">
							{{ path }}
						</li>
					</ul>
				</div>
				<div v-if="dependencyIssues.length" role="alert" class="text-sm text-orange">
					<p class="m-0">{{ formatMessage(messages.dependencyMissing) }}</p>
					<ul class="my-2 max-h-32 overflow-y-auto">
						<li v-for="issue in dependencyIssues" :key="issue" class="break-all">{{ issue }}</li>
					</ul>
				</div>
			</template>
			<div class="grid grid-cols-2 gap-4">
				<div class="labeled_input w-full">
					<p class="text-contrast font-semibold">{{ formatMessage(messages.modpackNameLabel) }}</p>
					<StyledInput
						v-model="nameInput"
						type="text"
						:placeholder="formatMessage(messages.modpackNamePlaceholder)"
						clearable
						wrapper-class="w-full"
					/>
				</div>
				<div class="labeled_input w-full">
					<p class="text-contrast font-semibold">
						{{ formatMessage(messages.versionNumberLabel) }}
					</p>
					<StyledInput
						v-model="versionInput"
						type="text"
						:placeholder="formatMessage(messages.versionNumberPlaceholder)"
						clearable
						wrapper-class="w-full"
					/>
				</div>
			</div>
			<div class="flex flex-col gap-2 min-w-0">
				<p class="m-0 text-contrast font-semibold">
					{{ formatMessage(commonMessages.descriptionLabel) }}
				</p>
				<StyledInput
					v-model="exportDescription"
					multiline
					:placeholder="formatMessage(messages.descriptionPlaceholder)"
					wrapper-class="w-full"
				/>
			</div>
			<div v-if="filesError" role="alert" class="flex items-center gap-3">
				<div class="min-w-0">
					<p class="m-0">{{ formatMessage(messages.filesError) }}</p>
					<p v-if="fileErrorMessage" class="m-0 break-words text-sm text-secondary">
						{{ fileErrorMessage }}
					</p>
				</div>
				<ButtonStyled
					><button type="button" @click="initFiles()">
						{{ formatMessage(messages.retry) }}
					</button></ButtonStyled
				>
			</div>
			<LoadingIndicator v-else-if="filesLoading" />
			<FileTreeSelect
				v-else
				:key="fileTreeKey"
				v-model="selectedFilePaths"
				class="min-w-0"
				:items="files"
				@navigate="loadExportDirectory"
			/>
		</fieldset>
		<template #actions>
			<div class="flex items-center justify-end gap-2">
				<ButtonStyled type="outlined">
					<button :disabled="exporting" @click="exportModal.hide">
						<XIcon />
						{{ formatMessage(commonMessages.cancelButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button
						:disabled="
							exporting ||
							filesLoading ||
							filesError ||
							!nameInput.trim() ||
							!versionInput.trim() ||
							(isServerFormat &&
								(!serverPreview || !selectedFilePaths.length || dependencyIssues.length > 0))
						"
						@click="exportPack"
					>
						<PackageIcon />
						{{ formatMessage(messages.exportButton) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>
