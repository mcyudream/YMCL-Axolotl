<script setup lang="ts">
import { DownloadIcon } from '@modrinth/assets'
import { ButtonStyled } from '@modrinth/ui'
import { computed } from 'vue'

import type { YmclAction } from '@/helpers/ymcl-actions'

/**
 * pack-catalog renderer (YAP §6.6): MIP pack entries with version and
 * change summary. Installation itself lands with MIP in P5; the primary
 * action renders but reports unavailable until then.
 */
const props = defineProps<{
	envelope: {
		records?: Record<string, unknown>[]
		itemActions?: YmclAction[]
		allow?: string[]
	}
}>()

const records = computed(() => props.envelope.records ?? [])
const itemActions = computed(() => props.envelope.itemActions ?? [])
</script>

<template>
	<div class="flex flex-col gap-2">
		<div
			v-for="record in records"
			:key="String(record.packId ?? record.id ?? record.title)"
			class="flex items-center gap-3 rounded-xl border border-solid border-surface-5 bg-bg-raised p-3"
		>
			<img
				v-if="record.icon"
				:src="String(record.icon)"
				:alt="String(record.title ?? '')"
				class="h-10 w-10 shrink-0 rounded-lg object-contain"
			/>
			<div class="min-w-0 flex-1">
				<div class="truncate font-semibold text-contrast">{{ record.title }}</div>
				<div class="truncate text-xs text-secondary">
					v{{ record.version ?? '?' }}
					<template v-if="record.channel"> · {{ record.channel }}</template>
					<template v-if="record.summary"> · {{ record.summary }}</template>
				</div>
			</div>
			<ButtonStyled v-if="itemActions.some((action) => action.primary)" type="standard">
				<button disabled>
					<DownloadIcon />
					{{ itemActions.find((action) => action.primary)?.title }}
				</button>
			</ButtonStyled>
		</div>
	</div>
</template>
