<script setup lang="ts">
import { EditIcon } from '@modrinth/assets'
import { nextTick, ref, watch } from 'vue'

const model = defineModel<string>({ default: '' })
const props = withDefaults(
	defineProps<{
		placeholder?: string
		defaultValue?: string
		maxWidth?: string
		maxLength?: number
		editLabel?: string
		activationMode?: 'text' | 'icon' | 'manual'
		iconTextClass?: string
		validate?: (value: string) => boolean
		onChange?: (value: string) => boolean | void | Promise<boolean | void>
	}>(),
	{
		placeholder: '',
		defaultValue: '',
		maxWidth: '100%',
		editLabel: 'Edit',
		activationMode: 'text',
	},
)

const editing = ref(false)
const saving = ref(false)
const invalid = ref(false)
const draft = ref(model.value)
const original = ref(model.value)
const input = ref<HTMLInputElement>()

watch(model, (value) => {
	if (!editing.value) draft.value = value
})

async function startEditing() {
	if (editing.value) return
	original.value = model.value
	draft.value = model.value
	invalid.value = false
	editing.value = true
	await nextTick()
	input.value?.focus()
	input.value?.select()
}

async function apply() {
	if (!editing.value || saving.value) return
	const value = (draft.value || props.defaultValue).trim()
	if (props.validate && !props.validate(value)) {
		invalid.value = true
		return
	}
	saving.value = true
	try {
		if ((await props.onChange?.(value)) === false) {
			invalid.value = true
			return
		}
		model.value = value
		editing.value = false
	} finally {
		saving.value = false
	}
}

function cancel() {
	if (saving.value) return
	draft.value = original.value
	invalid.value = false
	editing.value = false
}

function keydown(event: KeyboardEvent) {
	if (event.key === 'Enter') {
		event.preventDefault()
		void apply()
	} else if (event.key === 'Escape') {
		event.preventDefault()
		cancel()
	}
}

defineExpose({ isEditing: editing, startEditing })
</script>

<template>
	<div class="relative flex h-6 min-w-3 max-w-full items-center border-b" :style="{ maxWidth }">
		<input
			v-if="editing"
			ref="input"
			v-model="draft"
			type="text"
			:maxlength="maxLength"
			:placeholder="placeholder"
			:aria-label="editLabel"
			:aria-invalid="invalid"
			:disabled="saving"
			class="absolute inset-0 w-full min-w-0 border-0 border-b-2 border-brand bg-transparent p-0 text-inherit outline-none"
			@blur="apply"
			@keydown="keydown"
		/>
		<button
			v-else-if="activationMode !== 'manual'"
			type="button"
			class="flex w-full min-w-0 items-center gap-1 truncate border-0 bg-transparent p-0 text-left text-inherit"
			:aria-label="`${editLabel}: ${model || defaultValue || placeholder}`"
			@click="startEditing"
		>
			<span class="truncate" :class="iconTextClass">{{
				model || defaultValue || placeholder
			}}</span>
			<EditIcon v-if="activationMode === 'icon'" class="size-4 shrink-0 text-secondary" />
		</button>
		<span v-else class="truncate">{{ model || defaultValue || placeholder }}</span>
	</div>
</template>
