<script setup lang="ts">
import { ref, watch } from 'vue'

import { useDomainImage } from '@/composables/use-domain-image'
import { useYmclStore } from '@/store/ymcl'

/**
 * Domain image with origin rebasing and a silent failure state. When the
 * resolved URL fails to load the slot (or nothing) is shown instead of the
 * browser's broken-image chrome.
 */
defineOptions({ inheritAttrs: false })

const props = defineProps<{
	src?: string | null
	origin?: string | null
	alt?: string
}>()

const ymclStore = useYmclStore()
const failed = ref(false)
// Omitted origin falls back to the active domain; an explicit origin (even
// null) is used as-is so listing another domain never rebases onto the wrong host.
const displayUrl = useDomainImage(
	() => props.src,
	() => (props.origin === undefined ? (ymclStore.activeDomain?.origin ?? null) : props.origin),
)

watch(displayUrl, () => {
	failed.value = false
})
</script>

<template>
	<img
		v-if="displayUrl && !failed"
		:src="displayUrl"
		:alt="alt"
		v-bind="$attrs"
		@error="failed = true"
	/>
	<slot v-else name="fallback" />
</template>
