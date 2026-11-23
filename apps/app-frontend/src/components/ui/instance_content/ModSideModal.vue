<script setup lang="ts">
import {
	ButtonStyled,
	canMarkContentModSide,
	Combobox,
	commonMessages,
	type ContentItem,
	type ContentModSide,
	defineMessages,
	injectNotificationManager,
	modSideMessages,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

import { type ModSideScope, setModSide } from '@/helpers/mod-sides'

const props = defineProps<{ instanceId: string }>()
const emit = defineEmits<{ saved: [items: ContentItem[]] }>()
const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const messages = defineMessages({
	title: { id: 'app.mod-side.title', defaultMessage: 'Set side for {count, number} mods' },
	scope: { id: 'app.mod-side.scope', defaultMessage: 'Apply to' },
	instance: { id: 'app.mod-side.scope-instance', defaultMessage: 'This modpack' },
	local: {
		id: 'app.mod-side.scope-local',
		defaultMessage: 'All local modpacks using this mod and loader',
	},
	auto: { id: 'app.mod-side.auto', defaultMessage: 'Remove override' },
	autoHint: {
		id: 'app.mod-side.auto-hint',
		defaultMessage:
			'Removes the rule in the selected scope. Inherited local rules and automatic detection still apply.',
	},
	side: { id: 'app.mod-side.side', defaultMessage: 'Mod side' },
})
const modal = ref<InstanceType<typeof NewModal>>()
const items = ref<ContentItem[]>([])
const editingInstanceId = ref('')
const scope = ref<ModSideScope>('instance')
const side = ref<ContentModSide | 'auto'>('unknown')
const saving = ref(false)
watch(
	() => props.instanceId,
	() => {
		if (!saving.value) modal.value?.hide()
	},
)
const options = computed(() => [
	...(['client', 'server', 'both', 'unknown'] as const).map((value) => ({
		value,
		label: formatMessage(modSideMessages[value]),
	})),
	{ value: 'auto', label: formatMessage(messages.auto) },
])

defineExpose({
	show(selected: ContentItem[]) {
		if (saving.value) return
		items.value = selected.filter(canMarkContentModSide)
		if (!items.value.length) return
		scope.value = 'instance'
		editingInstanceId.value = props.instanceId
		const sides = new Set(items.value.map((item) => item.mod_side?.side ?? 'unknown'))
		side.value = sides.size === 1 ? [...sides][0] : 'unknown'
		modal.value?.show()
	},
})

async function save() {
	if (saving.value || editingInstanceId.value !== props.instanceId) return
	saving.value = true
	const instanceId = editingInstanceId.value
	try {
		const updated = await setModSide(
			instanceId,
			items.value.map((item) => item.file_path!),
			scope.value,
			side.value === 'auto' ? null : side.value,
		)
		if (instanceId !== props.instanceId) return
		emit('saved', updated)
		saving.value = false
		modal.value?.hide()
	} catch (error) {
		handleError(error)
	} finally {
		saving.value = false
		if (instanceId !== props.instanceId) modal.value?.hide()
	}
}
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.title, { count: items.length })"
		:disable-close="saving"
	>
		<form class="flex flex-col gap-4" @submit.prevent="save">
			<p v-if="items.length === 1" class="m-0 truncate font-semibold text-contrast">
				{{ items[0].project?.title ?? items[0].file_name }}
			</p>
			<ul v-else class="m-0 max-h-32 overflow-y-auto pl-5 text-sm text-secondary">
				<li v-for="item in items" :key="item.file_path" class="break-words">
					{{ item.project?.title ?? item.file_name }}
				</li>
			</ul>
			<label class="flex flex-col gap-2 font-semibold">
				{{ formatMessage(messages.side) }}
				<Combobox v-model="side" :options="options" :disabled="saving" />
			</label>
			<label class="flex flex-col gap-2 font-semibold">
				{{ formatMessage(messages.scope) }}
				<Combobox
					v-model="scope"
					:disabled="saving"
					:options="[
						{ value: 'instance', label: formatMessage(messages.instance) },
						{ value: 'local', label: formatMessage(messages.local) },
					]"
				/>
			</label>
			<p v-if="side === 'auto'" class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.autoHint) }}
			</p>
			<div class="flex justify-end gap-2">
				<ButtonStyled type="outlined">
					<button type="button" :disabled="saving" @click="modal?.hide()">
						{{ formatMessage(commonMessages.cancelButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button type="submit" :disabled="saving">
						{{ formatMessage(commonMessages.saveButton) }}
					</button>
				</ButtonStyled>
			</div>
		</form>
	</NewModal>
</template>
