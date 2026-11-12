<script setup lang="ts">
import { CheckIcon, ChevronDownIcon, SearchIcon } from '@modrinth/assets'
import { defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

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
const modal = ref<InstanceType<typeof NewModal>>()
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

function select(serverId: string) {
	if (props.disabled) return
	emit('select', serverId)
	modal.value?.hide()
}

watch(
	() => props.disabled,
	(disabled) => {
		if (disabled) modal.value?.hide()
	},
)
</script>

<template>
	<div v-if="selected" class="min-w-0 shrink-0">
		<div class="flex min-w-0 items-center gap-3">
			<DomainHomeServerIdentity :server="selected" />
			<button
				v-if="servers.length > 1"
				type="button"
				class="flex shrink-0 cursor-pointer items-center gap-2 rounded-lg border border-solid border-white/20 bg-white/5 px-3 py-2 text-xs font-medium text-white transition-colors hover:border-white/40 hover:bg-white/15 disabled:cursor-not-allowed disabled:opacity-50 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
				:aria-expanded="expanded"
				aria-haspopup="dialog"
				:disabled="disabled"
				@click="modal?.show($event)"
			>
				{{ formatMessage(messages.switch) }}
				<ChevronDownIcon
					class="size-4 transition-transform motion-reduce:transition-none"
					:class="{ 'rotate-180': expanded }"
				/>
			</button>
		</div>
		<NewModal
			ref="modal"
			:header="formatMessage(messages.choose)"
			width="36rem"
			max-width="calc(100vw - 2rem)"
			scrollable
			:on-show="() => (expanded = true)"
			:on-hide="
				() => {
					expanded = false
					search = ''
				}
			"
		>
			<p class="mb-3 mt-0 text-xs text-secondary">
				{{ formatMessage(messages.count, { count: servers.length }) }}
			</p>
			<label
				v-if="servers.length > 6"
				class="mb-3 flex items-center gap-2 rounded-lg border border-solid border-surface-4 bg-surface-3 px-3 text-secondary focus-within:border-brand"
			>
				<SearchIcon class="size-4 shrink-0" />
				<input
					v-model="search"
					type="search"
					class="min-w-0 flex-1 border-0 bg-transparent py-2 text-sm text-contrast outline-none"
					:aria-label="formatMessage(messages.search)"
					:placeholder="formatMessage(messages.search)"
				/>
			</label>
			<div
				class="grid grid-cols-[repeat(auto-fit,minmax(min(100%,15rem),1fr))] gap-2"
				:aria-label="formatMessage(messages.choose)"
			>
				<button
					v-for="row in matches"
					:key="row.serverId"
					type="button"
					:aria-pressed="row.serverId === selectedId"
					:disabled="disabled"
					class="flex min-w-0 cursor-pointer items-center gap-2 rounded-xl border border-solid p-3 text-left text-contrast transition-colors disabled:cursor-not-allowed disabled:opacity-60 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-brand"
					:class="
						row.serverId === selectedId
							? 'border-brand bg-surface-3'
							: 'border-surface-4 bg-surface-2 hover:border-surface-5 hover:bg-surface-3'
					"
					@click="select(row.serverId)"
				>
					<DomainHomeServerIdentity :server="row" surface="panel" />
					<CheckIcon v-if="row.serverId === selectedId" class="size-4 shrink-0 text-brand" />
				</button>
			</div>
			<p v-if="!matches.length" class="m-0 px-1 py-3 text-sm text-secondary" role="status">
				{{ formatMessage(messages.noMatches) }}
			</p>
		</NewModal>
	</div>
</template>
