<script setup lang="ts">
import { RefreshCwIcon } from '@modrinth/assets'

import Button from '#ui/components/base/buttons/Button.vue'
import { useVIntl } from '#ui/composables/i18n'
import { commonMessages } from '#ui/utils/common-messages'

const props = defineProps<{
	label?: string
	pendingLabel?: string
	loading?: boolean
	disabled?: boolean
	tooltip?: string | null
}>()

const { formatMessage } = useVIntl()
</script>

<template>
	<Button
		v-tooltip="props.tooltip"
		type="outlined"
		size="lg"
		:loading="props.loading"
		:disabled="props.disabled"
	>
		<RefreshCwIcon aria-hidden="true" />
		<span class="grid">
			<span
				class="[grid-area:1/1]"
				:class="{ invisible: props.loading && props.pendingLabel }"
				:aria-hidden="(props.loading && !!props.pendingLabel) || undefined"
			>
				{{ props.label ?? formatMessage(commonMessages.refreshButton) }}
			</span>
			<span
				v-if="props.pendingLabel"
				class="[grid-area:1/1]"
				:class="{ invisible: !props.loading }"
				:aria-hidden="!props.loading || undefined"
			>
				{{ props.pendingLabel }}
			</span>
		</span>
	</Button>
</template>
