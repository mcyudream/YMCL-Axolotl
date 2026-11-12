<script setup lang="ts">
import { ServerIcon } from '@modrinth/assets'
import { defineMessages, useVIntl } from '@modrinth/ui'

import DomainImg from '@/components/ymcl/DomainImg.vue'
import type { DomainServerRow } from '@/composables/useDomainServers'

withDefaults(defineProps<{ server: DomainServerRow; surface?: 'scenery' | 'panel' }>(), {
	surface: 'scenery',
})
const { formatMessage } = useVIntl()
const messages = defineMessages({
	online: { id: 'app.ymcl.landing.online', defaultMessage: 'Online' },
	offline: { id: 'app.ymcl.landing.offline', defaultMessage: 'Offline' },
	players: { id: 'app.ymcl.landing.players', defaultMessage: '{count} online' },
})
</script>

<template>
	<span
		class="flex min-w-0 flex-1 items-center gap-3"
		:class="surface === 'panel' ? 'text-contrast' : 'text-white'"
	>
		<DomainImg
			:src="server.favicon"
			alt=""
			class="size-10 shrink-0 rounded-xl object-cover [image-rendering:pixelated]"
		>
			<template #fallback>
				<span
					class="grid size-10 shrink-0 place-items-center rounded-xl"
					:class="surface === 'panel' ? 'bg-surface-4' : 'bg-white/10'"
				>
					<ServerIcon class="size-5" />
				</span>
			</template>
		</DomainImg>
		<span class="flex min-w-0 flex-1 flex-col gap-1">
			<span class="truncate text-sm font-semibold" :title="server.name">{{ server.name }}</span>
			<span
				class="flex flex-wrap items-center gap-x-1.5 gap-y-1 text-xs"
				:class="surface === 'panel' ? 'text-secondary' : 'text-white/60'"
			>
				<span
					v-if="server.status !== 'unknown'"
					class="size-1.5 shrink-0 rounded-full"
					:class="server.status === 'online' ? 'bg-green' : 'bg-red'"
				/>
				<span v-if="server.status === 'offline'">{{ formatMessage(messages.offline) }}</span>
				<span v-else-if="server.onlinePlayers != null">{{
					formatMessage(messages.players, { count: server.onlinePlayers })
				}}</span>
				<span v-else-if="server.status === 'online'">{{ formatMessage(messages.online) }}</span>
				<span v-if="server.requiredVersion">MC {{ server.requiredVersion }}</span>
				<span v-if="server.seasonName" class="truncate" :title="server.seasonName">
					{{ server.seasonName }}
				</span>
			</span>
		</span>
	</span>
</template>
