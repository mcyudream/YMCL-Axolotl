<script setup lang="ts">
import {
	ArrowDownIcon,
	ArrowUpIcon,
	LoaderIcon,
	PlusIcon,
	SaveIcon,
	TrashIcon,
} from '@modrinth/assets'
import { ButtonStyled, defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref } from 'vue'

import type { HomeDashboardConfig } from '@/components/home/home-dashboard'
import HomeDashboard from '@/components/home/HomeDashboard.vue'
import {
	HOME_CARD_TYPE_OPTIONS,
	mapHomeProfileToDashboard,
	type YmclHomeProfile,
} from '@/helpers/ymcl-home'
import { useYmclStore } from '@/store/ymcl'

/**
 * Domain home layout designer (YAP §6.5, admin): WYSIWYG editor for the
 * domain-hosted home card layout. Edits render through the real dashboard
 * mapping; saving PUTs the config so the adapter rebuilds its manifest.
 */

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const ymclStore = useYmclStore()

const HomeDashboardComponent = HomeDashboard

const messages = defineMessages({
	title: {
		id: 'app.ymcl.design.title',
		defaultMessage: 'Home layout designer',
	},
	description: {
		id: 'app.ymcl.design.description',
		defaultMessage:
			'Configure the home page card layout for everyone in this domain. Saving publishes the layout to all members.',
	},
	addCard: { id: 'app.ymcl.design.add-card', defaultMessage: 'Add card' },
	remove: { id: 'app.ymcl.design.remove', defaultMessage: 'Remove' },
	moveUp: { id: 'app.ymcl.design.move-up', defaultMessage: 'Move up' },
	moveDown: { id: 'app.ymcl.design.move-down', defaultMessage: 'Move down' },
	save: { id: 'app.ymcl.design.save', defaultMessage: 'Publish layout' },
	preview: { id: 'app.ymcl.design.preview', defaultMessage: 'Preview' },
	notUnlocked: {
		id: 'app.ymcl.design.not-unlocked',
		defaultMessage:
			'The home designer requires an active domain with a signed-in account that has design permissions.',
	},
	emptyLayout: {
		id: 'app.ymcl.design.empty-layout',
		defaultMessage: 'No cards yet. Add a card to start building the layout.',
	},
	lockedNote: {
		id: 'app.ymcl.design.locked-note',
		defaultMessage: 'This layout is locked: members cannot reorder or hide cards.',
	},
	unmappedNote: {
		id: 'app.ymcl.design.unmapped-note',
		defaultMessage:
			'{count} card(s) use types without a native widget and are not shown in the preview.',
	},
	saved: { id: 'app.ymcl.design.saved', defaultMessage: 'Layout published' },
})

interface DesignerRow {
	id: string
	type: string
	title?: string
}

const rows = ref<DesignerRow[]>([])
const locked = ref(false)
const loading = ref(false)
const saving = ref(false)
const dirty = ref(false)

const instanceList = ref<{ id: string; name: string; icon_path: null; loader: string }[]>([])

const previewConfig = computed<HomeDashboardConfig | null>(() =>
	mapHomeProfileToDashboard({ cards: rows.value.map((row) => ({ ...row })) }),
)

const unmappedCount = computed(
	() =>
		rows.value.filter((row) => !HOME_CARD_TYPE_OPTIONS.some((o) => o.value === row.type)).length,
)

async function load() {
	loading.value = true
	try {
		const config = await invoke<YmclHomeProfile>('plugin:ymcl|ymcl_chrome_home_get')
		rows.value = (config.cards ?? []).map((card) => ({
			id: card.id,
			type: card.type,
			title: card.title,
		}))
		locked.value = config.locked === true
		dirty.value = false
	} catch (error) {
		handleError(error)
	} finally {
		loading.value = false
	}
}

async function save() {
	saving.value = true
	try {
		await invoke('plugin:ymcl|ymcl_chrome_home_put', {
			config: {
				schemaVersion: 1,
				locked: locked.value,
				columns: 3,
				cards: rows.value.map((row, index) => ({ ...row, sort: index * 10 })),
			},
		})
		dirty.value = false
	} catch (error) {
		handleError(error)
	} finally {
		saving.value = false
	}
}

function addCard(type: string) {
	const mapping = HOME_CARD_TYPE_OPTIONS.find((option) => option.value === type)
	rows.value.push({
		id: `${type}-${Date.now().toString(36)}`,
		type,
		title: mapping?.label,
	})
	dirty.value = true
}

function removeRow(index: number) {
	rows.value.splice(index, 1)
	dirty.value = true
}

function moveRow(index: number, delta: number) {
	const target = index + delta
	if (target < 0 || target >= rows.value.length) return
	const [row] = rows.value.splice(index, 1)
	rows.value.splice(target, 0, row)
	dirty.value = true
}

const typeLabel = (type: string) =>
	HOME_CARD_TYPE_OPTIONS.find((option) => option.value === type)?.label ?? type

onMounted(async () => {
	await load()
	const { list } = await import('@/helpers/instance')
	const instances = await list().catch(() => [])
	instanceList.value = instances.map((instance) => ({
		id: instance.instance_id,
		name: instance.name,
		icon_path: null,
		loader: instance.loader,
	}))
})
</script>

<template>
	<div class="flex min-h-full flex-col gap-4 p-6">
		<div class="flex items-center gap-3">
			<h1 class="m-0 text-2xl font-bold text-contrast">
				{{ formatMessage(messages.title) }}
			</h1>
			<div class="flex-1"></div>
			<ButtonStyled>
				<button :disabled="saving || loading || !dirty" @click="save">
					<LoaderIcon v-if="saving" class="animate-spin" />
					<SaveIcon v-else />
					{{ formatMessage(messages.save) }}
				</button>
			</ButtonStyled>
		</div>
		<p class="m-0 text-sm leading-relaxed text-secondary">
			{{ formatMessage(messages.description) }}
		</p>
		<p v-if="locked" class="m-0 text-sm text-secondary">
			{{ formatMessage(messages.lockedNote) }}
		</p>

		<div v-if="loading" class="rounded-xl bg-bg-raised p-6 text-center text-sm text-secondary">
			<LoaderIcon class="inline animate-spin" />
		</div>
		<div
			v-else-if="rows.length === 0 && !ymclStore.isPersonal"
			class="rounded-xl bg-bg-raised p-6 text-center text-sm text-secondary"
		>
			{{ formatMessage(messages.emptyLayout) }}
		</div>

		<div v-if="rows.length > 0 || HOME_CARD_TYPE_OPTIONS.length > 0" class="grid grid-cols-2 gap-4">
			<div class="flex flex-col gap-2">
				<div
					v-for="(row, index) in rows"
					:key="row.id"
					class="flex items-center gap-2 rounded-xl border border-solid border-surface-5 bg-bg-raised p-3"
				>
					<span class="min-w-0 flex-1 truncate font-medium text-contrast">
						{{ row.title ?? typeLabel(row.type) }}
					</span>
					<ButtonStyled type="standard" circular>
						<button
							v-tooltip="formatMessage(messages.moveUp)"
							:disabled="index === 0"
							@click="moveRow(index, -1)"
						>
							<ArrowUpIcon />
						</button>
					</ButtonStyled>
					<ButtonStyled type="standard" circular>
						<button
							v-tooltip="formatMessage(messages.moveDown)"
							:disabled="index === rows.length - 1"
							@click="moveRow(index, 1)"
						>
							<ArrowDownIcon />
						</button>
					</ButtonStyled>
					<ButtonStyled type="danger" circular>
						<button v-tooltip="formatMessage(messages.remove)" @click="removeRow(index)">
							<TrashIcon />
						</button>
					</ButtonStyled>
				</div>
				<div class="flex flex-wrap gap-2">
					<ButtonStyled
						v-for="option in HOME_CARD_TYPE_OPTIONS"
						:key="option.value"
						type="standard"
					>
						<button @click="addCard(option.value)">
							<PlusIcon />
							{{ option.label }}
						</button>
					</ButtonStyled>
				</div>
			</div>

			<div class="flex flex-col gap-2">
				<div class="text-xs font-semibold uppercase text-secondary">
					{{ formatMessage(messages.preview) }}
				</div>
				<HomeDashboardComponent
					v-if="previewConfig"
					:config="previewConfig"
					:instances="instanceList"
					player-name=""
					:editing="false"
				/>
				<p v-if="unmappedCount > 0" class="m-0 text-xs text-secondary">
					{{ formatMessage(messages.unmappedNote, { count: unmappedCount }) }}
				</p>
			</div>
		</div>
	</div>
</template>
