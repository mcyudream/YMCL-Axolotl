<script setup lang="ts">
import { PlayIcon } from '@modrinth/assets'
import { computed } from 'vue'

import { executeAction, renderActionParams, type YmclAction } from '@/helpers/ymcl-actions'

/**
 * card-grid renderer (YAP §6.6): a generic wall of cover cards with a
 * primary action per card (activities, entries, recommendations).
 */
const props = defineProps<{
	envelope: {
		records?: Record<string, unknown>[]
		itemActions?: YmclAction[]
		allow?: string[]
	}
}>()

const emit = defineEmits<{ reload: [] }>()

const records = computed(() => props.envelope.records ?? [])
const itemActions = computed(() => props.envelope.itemActions ?? [])
const allow = computed(() => props.envelope.allow ?? [])

function primaryAction(): YmclAction | null {
	return itemActions.value.find((action) => action.primary) ?? null
}

function isExecutable(action: YmclAction, record: Record<string, unknown>): boolean {
	const params = renderActionParams(action, record)
	if (action.kind === 'client:open-url') return /^https:\/\//.test(String(params.url ?? ''))
	if (action.kind === 'client:launch-instance') return !!params.instanceId
	if (action.kind === 'client:launch-server') return !!params.address && !!params.instanceId
	return false
}

async function run(action: YmclAction, record: Record<string, unknown>) {
	if (action.kind === 'client:reload') {
		emit('reload')
		return
	}
	await executeAction(action, { allow: allow.value, record })
}
</script>

<template>
	<div v-if="records.length > 0" class="grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-3">
		<button
			v-for="record in records"
			:key="String(record.id ?? record.title)"
			class="group relative cursor-pointer overflow-hidden rounded-2xl border-0 bg-transparent p-0 text-left"
			:disabled="!primaryAction() || !isExecutable(primaryAction()!, record)"
			@click="primaryAction() && run(primaryAction()!, record)"
		>
			<div class="absolute inset-0 transition-transform duration-200 group-hover:scale-[1.02]">
				<img
					v-if="record.cover"
					:src="String(record.cover)"
					:alt="String(record.title ?? '')"
					class="h-full w-full object-cover"
				/>
				<div v-else class="h-full w-full bg-bg-raised"></div>
			</div>
			<div
				class="relative flex min-h-36 flex-col justify-end gap-1 bg-gradient-to-t from-[rgba(0,0,0,0.85)] to-transparent p-4"
			>
				<div class="font-semibold text-white">
					{{ record.title }}
				</div>
				<div v-if="record.summary" class="line-clamp-2 text-xs text-white/70">
					{{ record.summary }}
				</div>
				<div
					v-if="primaryAction() && isExecutable(primaryAction()!, record)"
					class="mt-1 flex items-center gap-1 text-xs font-medium text-white/90"
				>
					<PlayIcon class="h-3 w-3" />
					{{ primaryAction()!.title }}
				</div>
			</div>
		</button>
	</div>
</template>
