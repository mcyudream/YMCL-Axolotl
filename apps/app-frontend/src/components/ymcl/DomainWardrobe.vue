<script setup lang="ts">
import { CheckIcon, PlusIcon, SpinnerIcon, TrashIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	Combobox,
	commonMessages,
	ConfirmModal,
	defineMessages,
	injectNotificationManager,
	NavTabs,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import SkinPreviewRenderer from '@modrinth/ui/src/components/skin/SkinPreviewRenderer.vue'
import { arrayBufferToBase64 } from '@modrinth/utils'
import { computedAsync } from '@vueuse/core'
import { computed, onMounted, ref, useTemplateRef, watch } from 'vue'

import { skinBlobUrlMap } from '@/helpers/rendering/batch-skin-renderer'
import { generateSkinPreviews } from '@/helpers/rendering/skin-preview-renderer'
import {
	determineModelType,
	normalize_skin_texture,
	type Skin,
	type SkinModel,
} from '@/helpers/skins'
import {
	ymcl,
	type YmclCloset,
	type YmclClosetCape,
	type YmclClosetSkin,
	ymclErrorMessage,
	type YmclLibraryEntry,
	type YmclSkinProfile,
} from '@/helpers/ymcl'

const props = defineProps<{
	domainId: string
	domainName: string
}>()

const { formatMessage } = useVIntl()
const notifications = injectNotificationManager()
const { addNotification, handleError } = notifications

const messages = defineMessages({
	title: {
		id: 'app.skins.domain.title',
		defaultMessage: 'Domain wardrobe',
	},
	domainBadge: {
		id: 'app.skins.domain.badge',
		defaultMessage: 'Managed by {domain}',
	},
	profileLabel: {
		id: 'app.skins.domain.profile-label',
		defaultMessage: 'Character',
	},
	skinsTab: {
		id: 'app.skins.domain.tabs.skins',
		defaultMessage: 'Skins',
	},
	capesTab: {
		id: 'app.skins.domain.tabs.capes',
		defaultMessage: 'Capes',
	},
	libraryTab: {
		id: 'app.skins.domain.tabs.library',
		defaultMessage: 'Library',
	},
	applyButton: {
		id: 'app.skins.domain.apply-button',
		defaultMessage: 'Apply',
	},
	resetButton: {
		id: 'app.skins.domain.reset-button',
		defaultMessage: 'Reset',
	},
	uploadButton: {
		id: 'app.skins.domain.upload-button',
		defaultMessage: 'Upload skin',
	},
	uploadCapeButton: {
		id: 'app.skins.domain.upload-cape-button',
		defaultMessage: 'Upload cape',
	},
	collectButton: {
		id: 'app.skins.domain.collect-button',
		defaultMessage: 'Add to wardrobe',
	},
	collectedNotice: {
		id: 'app.skins.domain.collected-notice',
		defaultMessage: 'Added to your wardrobe.',
	},
	loadMoreButton: {
		id: 'app.skins.domain.load-more-button',
		defaultMessage: 'Load more',
	},
	emptyLibrary: {
		id: 'app.skins.domain.empty-library',
		defaultMessage: 'The domain skin library is empty.',
	},
	noCapeCard: {
		id: 'app.skins.domain.no-cape',
		defaultMessage: 'No cape',
	},
	equippedBadge: {
		id: 'app.skins.domain.equipped-badge',
		defaultMessage: 'Equipped',
	},
	emptySkins: {
		id: 'app.skins.domain.empty-skins',
		defaultMessage: 'No skins in this domain wardrobe yet. Upload one or pick from the library.',
	},
	emptyCapes: {
		id: 'app.skins.domain.empty-capes',
		defaultMessage: 'This domain does not offer any capes yet.',
	},
	emptyCapesWithUpload: {
		id: 'app.skins.domain.empty-capes-upload',
		defaultMessage: 'No capes in this wardrobe yet. Upload one or pick from the library.',
	},
	loadFailed: {
		id: 'app.skins.domain.load-failed',
		defaultMessage: 'Failed to load the domain wardrobe: {message}',
	},
	retryButton: {
		id: 'app.skins.domain.retry-button',
		defaultMessage: 'Retry',
	},
	uploadTitle: {
		id: 'app.skins.domain.upload.title',
		defaultMessage: 'Upload skin to domain',
	},
	uploadCapeTitle: {
		id: 'app.skins.domain.upload.cape-title',
		defaultMessage: 'Upload cape to domain',
	},
	uploadDescription: {
		id: 'app.skins.domain.upload.description',
		defaultMessage:
			'The skin is uploaded to {domain} and added to your wardrobe there. It is not equipped automatically.',
	},
	nameLabel: {
		id: 'app.skins.domain.upload.name-label',
		defaultMessage: 'Name',
	},
	namePlaceholder: {
		id: 'app.skins.domain.upload.name-placeholder',
		defaultMessage: 'Optional display name',
	},
	modelLabel: {
		id: 'app.skins.domain.upload.model-label',
		defaultMessage: 'Arm style',
	},
	modelClassic: {
		id: 'app.skins.domain.upload.model-classic',
		defaultMessage: 'Classic',
	},
	modelSlim: {
		id: 'app.skins.domain.upload.model-slim',
		defaultMessage: 'Slim',
	},
	uploadProceed: {
		id: 'app.skins.domain.upload.proceed',
		defaultMessage: 'Upload',
	},
	fileInputHint: {
		id: 'app.skins.domain.upload.file-hint',
		defaultMessage: 'Choose a PNG skin file',
	},
	uploadFailed: {
		id: 'app.skins.domain.upload.failed',
		defaultMessage: 'Failed to upload the skin: {message}',
	},
	uploadCapeFailed: {
		id: 'app.skins.domain.upload.cape-failed',
		defaultMessage: 'Failed to upload the cape: {message}',
	},
	uploadSuccess: {
		id: 'app.skins.domain.upload.success',
		defaultMessage: 'Skin added to your domain wardrobe.',
	},
	uploadCapeSuccess: {
		id: 'app.skins.domain.upload.cape-success',
		defaultMessage: 'Cape added to your domain wardrobe.',
	},
	fileCapeHint: {
		id: 'app.skins.domain.upload.cape-file-hint',
		defaultMessage: 'Choose a PNG cape file',
	},
	deleteTitle: {
		id: 'app.skins.domain.delete.title',
		defaultMessage: 'Delete this skin from the domain?',
	},
	deleteDescription: {
		id: 'app.skins.domain.delete.description',
		defaultMessage:
			'The skin is removed from your wardrobe on {domain}. If it is currently equipped, it is taken off as well.',
	},
	deleteFailed: {
		id: 'app.skins.domain.delete.failed',
		defaultMessage: 'Failed to delete the skin: {message}',
	},
	equipFailed: {
		id: 'app.skins.domain.equip-failed',
		defaultMessage: 'Failed to apply: {message}',
	},
})

type TabKey = 'skins' | 'capes' | 'library'

const closet = ref<YmclCloset | null>(null)
const profiles = ref<YmclSkinProfile[]>([])
const loading = ref(true)
const loadError = ref<string | null>(null)
const applying = ref(false)

const activeProfileId = ref<string | null>(null)
const selectedSkinId = ref<string | null>(null)
const selectedCapeId = ref<string | null>(null)
const activeTab = ref<TabKey>('skins')

/** WebGL-baked front renders (blob URLs) keyed by closet/library item id. */
const skinPreviews = ref(new Map<string, string>())
/** Cape textures as data URLs (raw fetch — capes are not skins), keyed by cape id. */
const capeDataUrls = ref(new Map<string, string>())

const libraryItems = ref<YmclLibraryEntry[]>([])
const libraryPage = ref(0)
const libraryHasMore = ref(false)
const libraryLoading = ref(false)
const libraryLoaded = ref(false)
const collectingId = ref<string | null>(null)

const uploadModal = useTemplateRef<InstanceType<typeof NewModal>>('uploadModal')
const deleteModal = useTemplateRef<InstanceType<typeof ConfirmModal>>('deleteModal')
const uploadFileInput = useTemplateRef<HTMLInputElement>('uploadFileInput')

const uploadName = ref('')
const uploadModel = ref<'classic' | 'slim'>('classic')
const uploadKind = ref<'skin' | 'cape'>('skin')
const uploadDataUrl = ref<string | null>(null)
const uploadFilename = ref('skin.png')
const uploading = ref(false)
const skinToDelete = ref<YmclClosetSkin | null>(null)

const activeProfile = computed(
	() => profiles.value.find((profile) => profile.id === activeProfileId.value) ?? null,
)

const profileOptions = computed(() =>
	profiles.value.map((profile) => ({ value: profile.id, label: profile.name })),
)

const equipped = computed(() => closet.value?.equipped ?? null)

const selectedSkin = computed(() => {
	const listed = closet.value?.skins.find((skin) => skin.id === selectedSkinId.value)
	if (listed) return listed
	// 已装备的皮肤可能不在衣柜清单（历史外置纹理），仍可预览。
	if (selectedSkinId.value === null && equipped.value?.skin) return equipped.value.skin
	return null
})

const selectedCape = computed(
	() => closet.value?.capes.find((cape) => cape.id === selectedCapeId.value) ?? null,
)

const hasPendingChange = computed(
	() =>
		(selectedSkin.value?.id ?? null) !== (equipped.value?.skin?.id ?? null) ||
		(selectedCape.value?.id ?? null) !== (equipped.value?.cape_id ?? null),
)

const previewTexture = computedAsync(async () => {
	const skin = selectedSkin.value
	if (!skin) return ''
	try {
		return await toDataUrl(skin.url)
	} catch {
		return ''
	}
})

const previewCape = computedAsync(async () => {
	const cape = selectedCape.value
	if (!cape) return undefined
	return capeDataUrls.value.get(cape.id)
})

const previewVariant = computed(() => {
	const variant = selectedSkin.value?.variant
	if (variant === 'SLIM' || variant === 'CLASSIC') return variant
	return undefined
})

async function toDataUrl(url: string): Promise<string> {
	const bytes = await normalize_skin_texture(url)
	return `data:image/png;base64,${arrayBufferToBase64(bytes)}`
}

/** Bakes proper 3D previews through the same shared WebGL pipeline the local
 * skin page uses (correct models, capes and shading — the hand-rolled 2D
 * composite it replaces rendered legacy textures wrong). */
async function buildPreviews(wardrobe: YmclCloset, library: YmclLibraryEntry[]) {
	const mapped: { key: string; skin: Skin }[] = []

	const mapSkin = (id: string, entry: YmclClosetSkin | YmclLibraryEntry) => {
		const variant: SkinModel =
			entry.variant === 'SLIM' || entry.variant === 'CLASSIC' ? entry.variant : 'UNKNOWN'
		const textureKey = `ymcl:${id}`
		mapped.push({
			key: `${textureKey}+${variant}+no-cape`,
			skin: {
				texture_key: textureKey,
				name: entry.name ?? undefined,
				variant,
				cape_id: undefined,
				texture: entry.url,
				source: 'custom_external',
				is_equipped: false,
			},
		})
	}

	for (const skin of wardrobe.skins) mapSkin(skin.id, skin)
	for (const entry of library) if (entry.entry_type !== 'cape') mapSkin(entry.id, entry)

	if (mapped.length > 0) {
		await generateSkinPreviews(
			mapped.map((entry) => entry.skin),
			[],
		)
	}

	const previews = new Map<string, string>()
	for (const entry of mapped) {
		const baked = skinBlobUrlMap.get(entry.key)
		if (baked) previews.set(entry.key.slice('ymcl:'.length).split('+')[0], baked.forwards)
	}
	skinPreviews.value = previews
}

async function loadCapeTextures(wardrobe: YmclCloset) {
	const capes = new Map(capeDataUrls.value)
	for (const cape of wardrobe.capes) {
		if (capes.has(cape.id)) continue
		try {
			capes.set(cape.id, await ymcl.skinTexture(cape.url))
		} catch {
			// Cape thumbnail missing; the modal preview falls back to nothing.
		}
	}
	capeDataUrls.value = capes
}

async function load() {
	loading.value = true
	loadError.value = null
	try {
		profiles.value = (await ymcl.skinProfiles(props.domainId)) ?? []
		if (!activeProfileId.value || !profiles.value.some((p) => p.id === activeProfileId.value)) {
			activeProfileId.value =
				profiles.value.find((profile) => profile.current)?.id ?? profiles.value[0]?.id ?? null
		}
		if (!activeProfileId.value) {
			throw new Error('The domain did not return any Minecraft profiles')
		}
		const wardrobe = await ymcl.skinCloset(props.domainId, activeProfileId.value)
		closet.value = wardrobe
		selectedSkinId.value = wardrobe.equipped?.skin?.id ?? null
		selectedCapeId.value = wardrobe.equipped?.cape_id ?? null
		void buildPreviews(wardrobe, libraryItems.value)
		void loadCapeTextures(wardrobe)
	} catch (error) {
		loadError.value = ymclErrorMessage(error)
	} finally {
		loading.value = false
	}
}

watch(activeProfileId, () => {
	void load()
})

function switchProfile(profileId: string) {
	if (profileId && profileId !== activeProfileId.value) {
		activeProfileId.value = profileId
	}
}

function selectSkin(skin: YmclClosetSkin) {
	selectedSkinId.value = skin.id
}

function selectCape(cape: YmclClosetCape | null) {
	selectedCapeId.value = cape?.id ?? null
}

function resetSelection() {
	selectedSkinId.value = closet.value?.equipped?.skin?.id ?? null
	selectedCapeId.value = closet.value?.equipped?.cape_id ?? null
}

async function applySelection() {
	if (!activeProfileId.value || applying.value || !hasPendingChange.value) return
	applying.value = true
	try {
		const next = await ymcl.skinEquip(
			props.domainId,
			activeProfileId.value,
			selectedSkinId.value,
			selectedCapeId.value,
		)
		if (closet.value) {
			closet.value.equipped = next
		}
	} catch (error) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.title),
			text: formatMessage(messages.equipFailed, { message: ymclErrorMessage(error) }),
		})
	} finally {
		applying.value = false
	}
}

async function loadLibrary(reset = false) {
	if (libraryLoading.value) return
	if (reset) {
		libraryPage.value = 0
		libraryHasMore.value = false
	}
	libraryLoading.value = true
	try {
		const page = libraryPage.value + 1
		const result = await ymcl.skinLibrary(props.domainId, page, 24)
		libraryItems.value = reset ? result.items : [...libraryItems.value, ...result.items]
		libraryPage.value = page
		libraryHasMore.value = result.has_more
		libraryLoaded.value = true
		// 披风条目直接取原始纹理做缩略图（皮肤条目走 WebGL 烘焙）。
		const capes = new Map(capeDataUrls.value)
		for (const entry of result.items) {
			if (entry.entry_type !== 'cape' || capes.has(entry.id)) continue
			try {
				capes.set(entry.id, await ymcl.skinTexture(entry.url))
			} catch {
				// 缩略图缺失时留空。
			}
		}
		capeDataUrls.value = capes
		void buildPreviews(closet.value ?? ({ skins: [], capes: [] } as YmclCloset), result.items)
	} catch (error) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.title),
			text: ymclErrorMessage(error),
		})
	} finally {
		libraryLoading.value = false
	}
}

async function collectEntry(entry: YmclLibraryEntry) {
	if (!activeProfileId.value || collectingId.value) return
	collectingId.value = entry.id
	try {
		await ymcl.skinCollect(
			props.domainId,
			activeProfileId.value,
			entry.hash ?? entry.id,
			entry.name ?? undefined,
		)
		addNotification({ type: 'success', title: formatMessage(messages.collectedNotice) })
		const isCape = entry.entry_type === 'cape'
		await load()
		activeTab.value = isCape ? 'capes' : 'skins'
	} catch (error) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.title),
			text: ymclErrorMessage(error),
		})
	} finally {
		collectingId.value = null
	}
}

function openUploadModal(kind: 'skin' | 'cape' = 'skin') {
	uploadKind.value = kind
	uploadName.value = ''
	uploadDataUrl.value = null
	uploadFilename.value = kind === 'cape' ? 'cape.png' : 'skin.png'
	uploadModel.value = 'classic'
	uploadModal.value?.show()
}

function openUploadFileBrowser() {
	uploadFileInput.value?.click()
}

async function onUploadFileInputChange(event: Event) {
	const file = (event.target as HTMLInputElement).files?.[0]
	if (file) {
		await prepareUploadFile(file)
	}
	if (uploadFileInput.value) {
		uploadFileInput.value.value = ''
	}
}

async function prepareUploadFile(file: File) {
	try {
		const buffer = await file.arrayBuffer()
		const original = `data:image/png;base64,${arrayBufferToBase64(buffer)}`
		uploadFilename.value = file.name || (uploadKind.value === 'cape' ? 'cape.png' : 'skin.png')
		if (uploadKind.value === 'cape') {
			// 披风纹理不做皮肤归一化，直接使用原图。
			uploadDataUrl.value = original
			return
		}
		const normalized = await normalize_skin_texture(original)
		const normalizedUrl = `data:image/png;base64,${arrayBufferToBase64(normalized)}`
		uploadDataUrl.value = normalizedUrl
		uploadModel.value = (await determineModelType(normalizedUrl)) === 'SLIM' ? 'slim' : 'classic'
	} catch (error) {
		handleError(error as Error)
	}
}

async function confirmUpload() {
	if (!uploadDataUrl.value || !activeProfileId.value || uploading.value) return
	uploading.value = true
	try {
		if (uploadKind.value === 'cape') {
			await ymcl.capeUpload(
				props.domainId,
				activeProfileId.value,
				uploadDataUrl.value,
				uploadFilename.value,
				uploadName.value.trim() || undefined,
			)
			addNotification({ type: 'success', title: formatMessage(messages.uploadCapeSuccess) })
		} else {
			await ymcl.skinUpload(
				props.domainId,
				activeProfileId.value,
				uploadDataUrl.value,
				uploadModel.value,
				uploadFilename.value,
				uploadName.value.trim() || undefined,
			)
			addNotification({ type: 'success', title: formatMessage(messages.uploadSuccess) })
		}
		uploadModal.value?.hide()
		await load()
	} catch (error) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.title),
			text: formatMessage(
				uploadKind.value === 'cape' ? messages.uploadCapeFailed : messages.uploadFailed,
				{ message: ymclErrorMessage(error) },
			),
		})
	} finally {
		uploading.value = false
	}
}

function confirmDeleteSkin(skin: YmclClosetSkin) {
	skinToDelete.value = skin
	deleteModal.value?.show()
}

async function deleteSkin() {
	const skin = skinToDelete.value
	if (!skin || !activeProfileId.value) return
	try {
		await ymcl.skinDelete(props.domainId, activeProfileId.value, skin.id)
		if (selectedSkinId.value === skin.id) {
			selectedSkinId.value = null
		}
		await load()
	} catch (error) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.title),
			text: formatMessage(messages.deleteFailed, { message: ymclErrorMessage(error) }),
		})
	} finally {
		skinToDelete.value = null
	}
}

function switchTab(index: number) {
	activeTab.value = index === 0 ? 'skins' : index === 1 ? 'capes' : 'library'
	if (activeTab.value === 'library' && !libraryLoaded.value) {
		void loadLibrary(true)
	}
}

onMounted(() => {
	void load()
})

const tabLinks = computed(() => [
	{ href: 'skins', label: formatMessage(messages.skinsTab) },
	{ href: 'capes', label: formatMessage(messages.capesTab) },
	{ href: 'library', label: formatMessage(messages.libraryTab) },
])
</script>

<template>
	<div class="skin-layout box-border min-h-full p-4">
		<div v-if="loading && !closet" class="flex items-center justify-center pt-[20%]">
			<SpinnerIcon class="size-8 animate-spin text-secondary" />
		</div>
		<div v-else-if="loadError" class="flex flex-col items-center gap-4 pt-[18%]">
			<p class="m-0 max-w-md text-center text-secondary">
				{{ formatMessage(messages.loadFailed, { message: loadError }) }}
			</p>
			<ButtonStyled>
				<button @click="load">
					{{ formatMessage(messages.retryButton) }}
				</button>
			</ButtonStyled>
		</div>
		<template v-else-if="closet">
			<div class="sticky top-6 self-start p-2 pt-0">
				<h1 class="m-0 flex items-center gap-2 text-2xl font-bold">
					{{ formatMessage(messages.title) }}
				</h1>
				<p class="m-0 mt-1 text-sm text-secondary">
					{{ formatMessage(messages.domainBadge, { domain: props.domainName }) }}
				</p>
				<div v-if="profiles.length > 1" class="ml-5 mt-4">
					<label class="mb-1 block text-xs font-medium text-secondary">
						{{ formatMessage(messages.profileLabel) }}
					</label>
					<Combobox
						:model-value="activeProfile?.name ?? ''"
						:options="profileOptions"
						:display-value="activeProfile?.name"
						class="w-full"
						@update:model-value="switchProfile"
					/>
				</div>
				<div
					class="ml-5 mt-4 flex h-[calc(80vh-8rem)] items-center justify-center max-[700px]:h-[calc(50vh-8rem)]"
				>
					<SkinPreviewRenderer
						:cape-src="previewCape"
						:texture-src="previewTexture || ''"
						:variant="previewVariant"
						:nametag="activeProfile?.name"
						:initial-rotation="Math.PI / 8"
					>
						<template v-if="hasPendingChange" #subtitle>
							<div class="flex flex-wrap items-center justify-center gap-2 px-2">
								<button
									class="flex h-10 min-w-0 cursor-pointer items-center justify-center gap-2 rounded-[14px] border-0 bg-surface-4 px-4 py-2.5 text-base font-semibold leading-5 text-contrast shadow-md transition-[filter,transform] duration-200 enabled:hover:brightness-[--hover-brightness] enabled:active:scale-95 disabled:cursor-not-allowed disabled:opacity-50"
									:disabled="applying"
									@click="resetSelection"
								>
									{{ formatMessage(messages.resetButton) }}
								</button>
								<button
									class="flex h-10 min-w-0 cursor-pointer items-center justify-center gap-2 rounded-[14px] border-0 bg-brand px-4 py-2.5 text-base font-semibold leading-5 text-contrast shadow-md transition-[filter,transform] duration-200 enabled:hover:brightness-[--hover-brightness] enabled:active:scale-95 disabled:cursor-not-allowed disabled:opacity-50"
									:disabled="applying"
									@click="applySelection"
								>
									<SpinnerIcon v-if="applying" class="animate-spin" />
									<CheckIcon v-else />
									{{ formatMessage(messages.applyButton) }}
								</button>
							</div>
						</template>
					</SkinPreviewRenderer>
				</div>
			</div>

			<div class="pt-2">
				<div class="mb-4 flex flex-wrap items-center justify-between gap-3">
					<NavTabs
						:active-index="activeTab === 'skins' ? 0 : activeTab === 'capes' ? 1 : 2"
						:links="tabLinks"
						mode="local"
						@tab-click="switchTab"
					/>
					<ButtonStyled v-if="activeTab === 'skins'" color="brand">
						<button @click="openUploadModal('skin')">
							<PlusIcon />
							{{ formatMessage(messages.uploadButton) }}
						</button>
					</ButtonStyled>
					<ButtonStyled v-else-if="activeTab === 'capes'" color="brand">
						<button @click="openUploadModal('cape')">
							<PlusIcon />
							{{ formatMessage(messages.uploadCapeButton) }}
						</button>
					</ButtonStyled>
				</div>

				<div v-if="activeTab === 'skins'">
					<div
						v-if="closet.skins.length === 0"
						class="rounded-xl bg-bg-raised p-6 text-center text-secondary"
					>
						{{ formatMessage(messages.emptySkins) }}
					</div>
					<div v-else class="grid grid-cols-[repeat(auto-fill,minmax(9rem,1fr))] gap-3">
						<button
							v-for="skin in closet.skins"
							:key="skin.id"
							class="group relative m-0 flex cursor-pointer flex-col gap-2 border-0 bg-transparent p-1 text-left"
							@click="selectSkin(skin)"
						>
							<div
								class="relative aspect-[31/40] w-full min-w-0 overflow-hidden rounded-[20px] bg-bg-raised transition-all"
								:class="
									selectedSkinId === skin.id ? 'ring-2 ring-brand' : 'group-hover:brightness-110'
								"
							>
								<img
									v-if="skinPreviews.get(skin.id)"
									:src="skinPreviews.get(skin.id)"
									:alt="skin.name ?? skin.id"
									class="h-full w-full object-contain p-2"
								/>
								<div
									v-if="equipped?.skin?.id === skin.id"
									class="absolute top-2 left-2 flex items-center gap-1 rounded-full bg-brand px-2 py-0.5 text-xs font-semibold text-contrast"
								>
									<CheckIcon class="size-3" />
									{{ formatMessage(messages.equippedBadge) }}
								</div>
								<span
									class="absolute right-2 bottom-2 flex size-8 cursor-pointer items-center justify-center rounded-full bg-surface-4 opacity-0 transition-opacity group-hover:opacity-100"
									role="button"
									@click.stop="confirmDeleteSkin(skin)"
								>
									<TrashIcon class="size-4 text-contrast" />
								</span>
							</div>
							<span class="truncate px-1 text-sm font-medium text-primary">
								{{ skin.name ?? skin.id }}
							</span>
						</button>
					</div>
				</div>

				<div v-else-if="activeTab === 'capes'">
					<div
						v-if="closet.capes.length === 0"
						class="rounded-xl bg-bg-raised p-6 text-center text-secondary"
					>
						{{ formatMessage(messages.emptyCapesWithUpload) }}
					</div>
					<div v-else class="grid grid-cols-[repeat(auto-fill,minmax(9rem,1fr))] gap-3">
						<button
							class="group relative m-0 flex cursor-pointer flex-col gap-2 border-0 bg-transparent p-1 text-left"
							@click="selectCape(null)"
						>
							<div
								class="flex aspect-[31/40] w-full items-center justify-center rounded-[20px] bg-bg-raised transition-all"
								:class="
									selectedCapeId === null ? 'ring-2 ring-brand' : 'group-hover:brightness-110'
								"
							>
								<span class="text-sm text-secondary">
									{{ formatMessage(messages.noCapeCard) }}
								</span>
							</div>
						</button>
						<button
							v-for="cape in closet.capes"
							:key="cape.id"
							class="group relative m-0 flex cursor-pointer flex-col gap-2 border-0 bg-transparent p-1 text-left"
							@click="selectCape(cape)"
						>
							<div
								class="relative aspect-[31/40] w-full rounded-[20px] bg-bg-raised transition-all"
								:class="
									selectedCapeId === cape.id ? 'ring-2 ring-brand' : 'group-hover:brightness-110'
								"
							>
								<div v-if="capeDataUrls.get(cape.id)" class="cape-thumb">
									<img :src="capeDataUrls.get(cape.id)" :alt="cape.name ?? cape.id" />
								</div>
								<div
									v-if="equipped?.cape_id === cape.id"
									class="absolute top-2 left-2 flex items-center gap-1 rounded-full bg-brand px-2 py-0.5 text-xs font-semibold text-contrast"
								>
									<CheckIcon class="size-3" />
									{{ formatMessage(messages.equippedBadge) }}
								</div>
							</div>
							<span class="truncate px-1 text-sm font-medium text-primary">
								{{ cape.name ?? cape.id }}
							</span>
						</button>
					</div>
				</div>

				<div v-else>
					<div
						v-if="libraryLoaded && libraryItems.length === 0"
						class="rounded-xl bg-bg-raised p-6 text-center text-secondary"
					>
						{{ formatMessage(messages.emptyLibrary) }}
					</div>
					<div v-else class="grid grid-cols-[repeat(auto-fill,minmax(9rem,1fr))] gap-3">
						<div
							v-for="entry in libraryItems"
							:key="entry.id"
							class="group relative m-0 flex flex-col gap-2 p-1 text-left"
						>
							<div
								class="relative aspect-[31/40] w-full overflow-hidden rounded-[20px] bg-bg-raised transition-all group-hover:brightness-110"
							>
								<!-- 披风缩略图走 CSS 裁切，皮肤条目仍是 WebGL 烘焙图 -->
								<div
									v-if="entry.entry_type === 'cape' && capeDataUrls.get(entry.id)"
									class="cape-thumb"
								>
									<img :src="capeDataUrls.get(entry.id)" :alt="entry.name ?? entry.id" />
								</div>
								<img
									v-else-if="skinPreviews.get(entry.id)"
									:src="skinPreviews.get(entry.id)"
									:alt="entry.name ?? entry.id"
									class="h-full w-full object-contain p-3"
								/>
								<ButtonStyled color="brand">
									<!-- 定位类必须落在 button 上：ButtonStyled 根元素是 display:contents，不生成盒子 -->
									<button
										class="absolute inset-x-3 bottom-3"
										:disabled="collectingId === entry.id"
										@click="collectEntry(entry)"
									>
										<SpinnerIcon v-if="collectingId === entry.id" class="animate-spin" />
										<PlusIcon v-else />
										{{ formatMessage(messages.collectButton) }}
									</button>
								</ButtonStyled>
							</div>
							<span class="truncate px-1 text-sm font-medium text-primary">
								{{ entry.name ?? entry.id }}
							</span>
						</div>
					</div>
					<div v-if="libraryHasMore || libraryLoading" class="mt-4 flex justify-center">
						<ButtonStyled>
							<button :disabled="libraryLoading" @click="loadLibrary()">
								<SpinnerIcon v-if="libraryLoading" class="animate-spin" />
								{{ formatMessage(messages.loadMoreButton) }}
							</button>
						</ButtonStyled>
					</div>
				</div>
			</div>

			<NewModal
				ref="uploadModal"
				:header="
					uploadKind === 'cape'
						? formatMessage(messages.uploadCapeTitle)
						: formatMessage(messages.uploadTitle)
				"
			>
				<div class="flex flex-col gap-4">
					<p class="m-0 text-secondary">
						{{ formatMessage(messages.uploadDescription, { domain: props.domainName }) }}
					</p>
					<div
						class="flex cursor-pointer items-center justify-center rounded-xl border-2 border-dashed border-surface-5 p-4"
						@click="openUploadFileBrowser"
					>
						<span v-if="!uploadDataUrl" class="text-secondary">
							{{
								formatMessage(
									uploadKind === 'cape'
										? messages.fileCapeHint
										: messages.fileInputHint,
								)
							}}
						</span>
						<img
							v-else
							:src="uploadDataUrl"
							:alt="uploadFilename"
							class="max-h-40 object-contain"
						/>
					</div>
					<div>
						<label class="mb-1 block text-sm font-semibold text-primary">
							{{ formatMessage(messages.nameLabel) }}
						</label>
						<input
							v-model="uploadName"
							class="w-full rounded-lg border border-surface-5 bg-bg-raised p-2 text-primary"
							:placeholder="formatMessage(messages.namePlaceholder)"
						/>
					</div>
					<div v-if="uploadKind !== 'cape'">
						<span class="mb-1 block text-sm font-semibold text-primary">
							{{ formatMessage(messages.modelLabel) }}
						</span>
						<div class="flex gap-2">
							<ButtonStyled :color="uploadModel === 'classic' ? 'brand' : 'standard'">
								<button @click="uploadModel = 'classic'">
									{{ formatMessage(messages.modelClassic) }}
								</button>
							</ButtonStyled>
							<ButtonStyled :color="uploadModel === 'slim' ? 'brand' : 'standard'">
								<button @click="uploadModel = 'slim'">
									{{ formatMessage(messages.modelSlim) }}
								</button>
							</ButtonStyled>
						</div>
					</div>
					<ButtonStyled color="brand" class="mt-2">
						<button :disabled="!uploadDataUrl || uploading" @click="confirmUpload">
							<SpinnerIcon v-if="uploading" class="animate-spin" />
							<PlusIcon v-else />
							{{ formatMessage(messages.uploadProceed) }}
						</button>
					</ButtonStyled>
				</div>
			</NewModal>
			<input
				ref="uploadFileInput"
				type="file"
				accept="image/png"
				class="hidden"
				@change="onUploadFileInputChange"
			/>
			<ConfirmModal
				ref="deleteModal"
				:title="formatMessage(messages.deleteTitle)"
				:description="formatMessage(messages.deleteDescription, { domain: props.domainName })"
				:proceed-label="formatMessage(commonMessages.deleteLabel)"
				@proceed="deleteSkin"
			/>
		</template>
	</div>
</template>

<style lang="scss" scoped>
.skin-layout {
	display: grid;
	grid-template-columns: minmax(0, 1fr) minmax(0, 2.5fr);
	gap: 2.5rem;

	@media (max-width: 700px) {
		grid-template-columns: 1fr;
	}
}

/* 缩放偏移整张 64×32 披风纹理，只露出 (1,1) 起 10×16 的正面区域 */
.cape-thumb {
	position: absolute;
	inset: 0;
	margin: auto;
	height: 78%;
	aspect-ratio: 10 / 16;
	overflow: hidden;

	img {
		position: absolute;
		width: calc(64 / 10 * 100%);
		height: calc(32 / 16 * 100%);
		left: calc(1 / 10 * -100%);
		top: calc(1 / 16 * -100%);
		image-rendering: pixelated;
		/* 放大 1% 并以正面区域中心为原点，抵消取整导致相邻区域渗色 */
		scale: 1.01;
		transform-origin: calc(5 / 64 * 100%) calc(8 / 32 * 100%);
	}
}
</style>
