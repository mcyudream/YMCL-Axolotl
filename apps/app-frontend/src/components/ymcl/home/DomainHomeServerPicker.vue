<script setup lang="ts">
import { CheckIcon, ChevronDownIcon, SearchIcon } from '@modrinth/assets'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed, nextTick, ref, useId, watch } from 'vue'

import type { DomainServerRow } from '@/composables/useDomainServers'

import DomainHomeServerIdentity from './DomainHomeServerIdentity.vue'

const props = defineProps<{
	servers: DomainServerRow[]
	selectedId: string
	disabled?: boolean
}>()
const emit = defineEmits<{ select: [serverId: string] }>()
const { formatMessage } = useVIntl()
const expanded = ref(false)
const search = ref('')
const trigger = ref<HTMLButtonElement>()
const choicesId = useId()
const selected = computed(() => props.servers.find((row) => row.serverId === props.selectedId))
const matches = computed(() => {
	if (props.servers.length <= 6) return props.servers
	const query = search.value.trim().toLocaleLowerCase()
	return props.servers.filter((row) =>
		[row.name, row.seasonName, row.requiredVersion, row.primaryAddress].some((value) =>
			value?.toLocaleLowerCase().includes(query),
		),
	)
})
const messages = defineMessages({
	choose: { id: 'app.ymcl.landing.select-server', defaultMessage: 'Choose a server' },
	switch: { id: 'app.ymcl.landing.switch-server', defaultMessage: 'Switch server' },
	count: { id: 'app.ymcl.landing.server-count', defaultMessage: '{count} servers' },
	search: { id: 'app.ymcl.landing.search-servers', defaultMessage: 'Search servers' },
	noMatches: {
		id: 'app.ymcl.landing.no-matching-servers',
		defaultMessage: 'No servers match your search.',
	},
})

async function select(serverId: string) {
	if (props.disabled) return
	emit('select', serverId)
	expanded.value = false
	search.value = ''
	await nextTick()
	trigger.value?.focus({ preventScroll: true })
}

watch(
	() => props.disabled,
	(disabled) => {
		if (disabled) expanded.value = false
	},
)
</script>

<template>
	<div v-if="selected" class="min-w-0">
		<div class="flex min-w-0 items-center gap-3">
			<DomainHomeServerIdentity :server="selected" />
			<button
				v-if="servers.length > 1"
				ref="trigger"
				type="button"
				class="flex shrink-0 cursor-pointer items-center gap-2 rounded-lg border border-solid border-white/20 bg-white/5 px-3 py-2 text-xs font-medium text-white transition-colors hover:border-white/40 hover:bg-white/15 disabled:cursor-not-allowed disabled:opacity-50 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
				:aria-expanded="expanded"
				:aria-controls="choicesId"
				:disabled="disabled"
				@click="expanded = !expanded"
			>
				{{ formatMessage(messages.switch) }}
				<ChevronDownIcon
					class="size-4 transition-transform motion-reduce:transition-none"
					:class="{ 'rotate-180': expanded }"
				/>
			</button>
		</div>
		<div
			v-if="expanded && servers.length > 1"
			:id="choicesId"
			class="mt-3 rounded-2xl border border-solid border-white/15 bg-black/35 p-3 backdrop-blur-xl"
		>
			<div class="mb-3 flex items-center justify-between gap-3 px-1 text-xs text-white/60">
				<span>{{ formatMessage(messages.choose) }}</span>
				<span>{{ formatMessage(messages.count, { count: servers.length }) }}</span>
			</div>
			<label
				v-if="servers.length > 6"
				class="mb-3 flex items-center gap-2 rounded-lg border border-solid border-white/15 bg-black/15 px-3 text-white/60 focus-within:border-white/50"
			>
				<SearchIcon class="size-4 shrink-0" />
				<input
					v-model="search"
					type="search"
					class="min-w-0 flex-1 border-0 bg-transparent py-2 text-sm text-white outline-none"
					:aria-label="formatMessage(messages.search)"
					:placeholder="formatMessage(messages.search)"
				/>
			</label>
			<div
				class="grid max-h-64 grid-cols-[repeat(auto-fit,minmax(min(100%,11rem),1fr))] gap-2 overflow-y-auto overscroll-contain"
				:aria-label="formatMessage(messages.choose)"
			>
				<button
					v-for="row in matches"
					:key="row.serverId"
					type="button"
					:aria-pressed="row.serverId === selectedId"
					:disabled="disabled"
					class="flex min-w-0 cursor-pointer items-center gap-2 rounded-xl border border-solid p-3 text-left text-white transition-colors disabled:cursor-not-allowed disabled:opacity-60 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-white"
					:class="
						row.serverId === selectedId
							? 'border-white/40 bg-white/15'
							: 'border-transparent bg-white/5 hover:border-white/20 hover:bg-white/10'
					"
					@click="select(row.serverId)"
				>
					<DomainHomeServerIdentity :server="row" />
					<CheckIcon v-if="row.serverId === selectedId" class="size-4 shrink-0 text-white/80" />
				</button>
			</div>
			<p v-if="!matches.length" class="m-0 px-1 py-3 text-sm text-white/60" role="status">
				{{ formatMessage(messages.noMatches) }}
			</p>
		</div>
	</div>
</template>
