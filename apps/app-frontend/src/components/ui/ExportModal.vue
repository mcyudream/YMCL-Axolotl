<script setup>
import { FolderOpenIcon, XIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	commonMessages,
	Combobox,
	defineMessages,
	FileTreeSelect,
	injectNotificationManager,
	injectPopupNotificationManager,
	NewModal,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { join } from '@tauri-apps/api/path'
import { save } from '@tauri-apps/plugin-dialog'
import { readDir, stat } from '@tauri-apps/plugin-fs'
import { nextTick, onBeforeUnmount, ref } from 'vue'

import { PackageIcon } from '@/assets/icons'
import {
	export_instance_mrpack,
	get_full_path,
	get_pack_export_candidates,
} from '@/helpers/instance'
import { highlightInFolder } from '@/helpers/utils'

const { handleError } = injectNotificationManager()
const popupNotificationManager = injectPopupNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	filesError: { id: 'app.export-modal.files-error', defaultMessage: 'Could not read the instance files.' },
	retry: { id: 'app.export-modal.retry', defaultMessage: 'Retry' },
	format: { id: 'app.export-modal.format', defaultMessage: 'Package format' },
	modrinth: { id: 'app.export-modal.format-modrinth', defaultMessage: 'Modrinth (.mrpack)' },
	curseforge: { id: 'app.export-modal.format-curseforge', defaultMessage: 'CurseForge (.zip)' },
	generic: { id: 'app.export-modal.format-generic', defaultMessage: 'Generic ZIP (.zip)' },
	genericDescription: { id: 'app.export-modal.generic-description', defaultMessage: 'Contains the selected files. The recipient configures Minecraft and the mod loader.' },
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
	show: () => {
		resetExportState()
		exportModal.value.show()
		void initFiles()
	},
})

const exportModal = ref(null)
const format = ref('modrinth')
const exporting = ref(false)
const filesLoading = ref(false)
const filesError = ref(false)
const nameInput = ref(props.instance.name)
const exportDescription = ref('')
const versionInput = ref('1.0.0')
const files = ref([])
const selectedFilePaths = ref([])
const fileTreeKey = ref(0)
const filesLoadId = ref(0)
const instanceRoot = ref('')
const loadedDirectories = ref(new Set())

async function initFiles() {
	filesLoading.value = true
	filesError.value = false
	const loadId = ++filesLoadId.value
	const instanceId = props.instance.id
	try {
		const [filePaths, root] = await Promise.all([
			get_pack_export_candidates(instanceId),
			get_full_path(instanceId),
		])
		const exportCandidates = await Promise.all(filePaths.map((path) => buildExportCandidateItem(root, path)))
		if (loadId !== filesLoadId.value || instanceId !== props.instance.id) return
		instanceRoot.value = root
		files.value = exportCandidates
		selectedFilePaths.value = files.value
			.filter((file) => !file.disabled && isDefaultSelectedExportCandidate(file.path))
			.map((file) => file.path)
	} catch (error) {
		if (loadId === filesLoadId.value) filesError.value = true
	} finally {
		if (loadId === filesLoadId.value) filesLoading.value = false
	}
}

const exportPack = async () => {
	if (exporting.value || filesLoading.value || filesError.value) return
	exporting.value = true
	try {
		const extension = format.value === 'modrinth' ? 'mrpack' : 'zip'
		const outputPath = await save({
			defaultPath: `${nameInput.value} ${versionInput.value}.${extension}`,
			filters: [{ name: formatMessage(messages[format.value]), extensions: [extension] }],
		})
		if (!outputPath) return
		await export_instance_mrpack(
			props.instance.id,
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
			buttons: [{
				label: formatMessage(commonMessages.openInFolderButton),
				icon: FolderOpenIcon,
				action: () => highlightInFolder(outputPath).catch(handleError),
			}],
		})
	} catch (error) {
		handleError(error)
	} finally {
		exporting.value = false
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
}

onBeforeUnmount(() => { filesLoadId.value++ })

async function loadExportDirectory(path) {
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
		@hide="filesLoadId++"
		width="46rem"
		max-width="calc(100vw - 2rem)"
	>
		<fieldset :disabled="exporting" :inert="exporting" class="m-0 flex min-w-0 flex-col gap-4 border-0 p-0" data-onboarding-id="pack-export-formats">
			<label class="flex flex-col gap-2 font-semibold text-contrast">
				{{ formatMessage(messages.format) }}
				<Combobox v-model="format" :disabled="exporting" :options="['modrinth', 'curseforge', 'generic'].map(value => ({ value, label: formatMessage(messages[value]) }))" />
			</label>
			<p v-if="format === 'generic'" class="m-0 text-sm text-secondary">{{ formatMessage(messages.genericDescription) }}</p>
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
				<span>{{ formatMessage(messages.filesError) }}</span>
				<ButtonStyled><button type="button" @click="initFiles">{{ formatMessage(messages.retry) }}</button></ButtonStyled>
			</div>
			<FileTreeSelect v-else
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
					<button :disabled="exporting || filesLoading || filesError || !nameInput.trim() || !versionInput.trim()" @click="exportPack">
						<PackageIcon />
						{{ formatMessage(messages.exportButton) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>
