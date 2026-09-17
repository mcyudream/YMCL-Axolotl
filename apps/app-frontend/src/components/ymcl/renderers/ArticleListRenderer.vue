<script setup lang="ts">
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed } from 'vue'

import DomainImg from '@/components/ymcl/DomainImg.vue'

/**
 * article-list renderer (YAP §6.6): image + title + summary rows linking
 * out to the domain's site. URL must be https.
 */
const props = defineProps<{
	envelope: { records?: Record<string, unknown>[] }
}>()

const records = computed(() => props.envelope.records ?? [])

function open(record: Record<string, unknown>) {
	const url = String(record.url ?? '')
	if (/^https:\/\//.test(url)) void openUrl(url)
}
</script>

<template>
	<div class="flex flex-col gap-2">
		<button
			v-for="record in records"
			:key="String(record.id ?? record.title)"
			class="flex cursor-pointer items-center gap-3 rounded-xl border-0 bg-bg-raised p-3 text-left transition-colors hover:bg-button-bg"
			@click="open(record)"
		>
			<DomainImg
				:src="typeof record.cover === 'string' ? record.cover : null"
				:alt="String(record.title ?? '')"
				class="h-16 w-24 shrink-0 rounded-lg object-cover"
			/>
			<div class="min-w-0 flex-1">
				<div class="truncate font-semibold text-contrast">
					{{ record.title }}
				</div>
				<div v-if="record.summary" class="line-clamp-2 text-xs text-secondary">
					{{ record.summary }}
				</div>
				<div v-if="record.date" class="mt-1 text-xs text-secondary">
					{{ record.date }}
				</div>
			</div>
		</button>
	</div>
</template>
