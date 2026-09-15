<script setup lang="ts">
import { computed } from 'vue'

/**
 * stats renderer (YAP §6.6): simple label/value stat cards (online count,
 * registration stats). Pairs with data-card variant "stats".
 */
const props = defineProps<{
	envelope: { records?: Record<string, unknown>[] }
}>()

const records = computed(() => props.envelope.records ?? [])
</script>

<template>
	<div class="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-3">
		<div
			v-for="record in records"
			:key="String(record.label ?? record.id)"
			class="flex flex-col gap-1 rounded-xl bg-bg-raised p-4"
		>
			<span class="text-2xl font-bold text-contrast">{{ record.value ?? '—' }}</span>
			<span class="text-xs font-medium text-secondary">{{ record.label }}</span>
		</div>
	</div>
</template>
