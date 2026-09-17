<script setup lang="ts">
import { PlusIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	commonMessages,
	defineMessages,
	NewModal,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { ref } from 'vue'

import { useYmclStore } from '@/store/ymcl'
import { ymclErrorMessage } from '@/helpers/ymcl'

const emit = defineEmits<{
	added: []
}>()

const { formatMessage } = useVIntl()
const ymclStore = useYmclStore()

const modal = ref<InstanceType<typeof NewModal>>()
const origin = ref('')
const error = ref<string | null>(null)

const messages = defineMessages({
	title: {
		id: 'app.ymcl.add-domain.title',
		defaultMessage: '添加域',
	},
	description: {
		id: 'app.ymcl.add-domain.description',
		defaultMessage: '输入要加入的域地址，连接成功后即可切换到该域。',
	},
	placeholder: {
		id: 'app.ymcl.add-domain.placeholder',
		defaultMessage: '域地址，例如 yda.example.com',
	},
	confirm: {
		id: 'app.ymcl.add-domain.confirm',
		defaultMessage: '添加',
	},
	adding: {
		id: 'app.ymcl.add-domain.adding',
		defaultMessage: '正在连接域，请稍候…',
	},
})

function show(prefill?: string) {
	origin.value = prefill ? prefill.trim() : ''
	error.value = null
	modal.value?.show()
	if (prefill && prefill.trim()) {
		// Deep-link add-site: the origin comes from a trusted domain page, connect directly
		void confirm()
	}
}

async function confirm() {
	const trimmed = origin.value.trim()
	if (!trimmed || ymclStore.adding) return
	error.value = null
	try {
		await ymclStore.addDomain(trimmed)
		emit('added')
		modal.value?.hide()
	} catch (err) {
		error.value = ymclErrorMessage(err)
	}
}

defineExpose({ show })
</script>

<template>
	<NewModal ref="modal" :header="formatMessage(messages.title)" max-width="420px">
		<div class="flex min-w-[22rem] flex-col gap-4">
			<p class="m-0 text-secondary">{{ formatMessage(messages.description) }}</p>
			<StyledInput
				v-model="origin"
				:placeholder="formatMessage(messages.placeholder)"
				:disabled="ymclStore.adding"
				autocomplete="off"
				spellcheck="false"
				inputmode="url"
				wrapper-class="w-full"
				@keyup.enter="confirm"
			/>
			<p v-if="ymclStore.adding" class="m-0 flex items-center gap-2 text-sm text-secondary">
				<SpinnerIcon class="animate-spin" />
				{{ formatMessage(messages.adding) }}
			</p>
			<p v-else-if="error" class="m-0 text-sm text-red">{{ error }}</p>
			<div class="flex justify-end gap-2">
				<ButtonStyled>
					<button :disabled="ymclStore.adding" @click="modal?.hide()">
						<XIcon />
						{{ formatMessage(commonMessages.cancelButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button :disabled="ymclStore.adding || !origin.trim()" @click="confirm">
						<SpinnerIcon v-if="ymclStore.adding" class="animate-spin" />
						<PlusIcon v-else />
						{{ formatMessage(messages.confirm) }}
					</button>
				</ButtonStyled>
			</div>
		</div>
	</NewModal>
</template>
