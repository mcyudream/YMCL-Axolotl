<script setup lang="ts">
import { IssuesIcon, LinkIcon } from '@modrinth/assets'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'

import type { HomeWidgetPlacement, HomeWidgetSize } from '@/components/home/home-dashboard'
import { domainPageRoute, domainRendererIcon, parseShortcutTarget } from '@/helpers/ymcl-domain'
import { useYmclStore } from '@/store/ymcl'

/**
 * page-shortcut home widget (YAP §6.5 adapter card contribution): opens a
 * domain page or native route from the home dashboard.
 */

const props = defineProps<{
	placement: HomeWidgetPlacement
	dashboardSize: HomeWidgetSize
}>()

const { formatMessage } = useVIntl()
const ymclStore = useYmclStore()

const messages = defineMessages({
	kind: { id: 'app.home.domain-shortcut.kind', defaultMessage: '域入口' },
	unavailable: {
		id: 'app.home.domain-shortcut.unavailable',
		defaultMessage: '此入口已被域移除',
	},
})

const parsed = computed(() =>
	props.placement.shortcut ? parseShortcutTarget(props.placement.shortcut.target) : null,
)
const page = computed(() =>
	parsed.value?.kind === 'page' ? ymclStore.pageById(parsed.value.pageId) : null,
)
/** Resolved when the target exists: domain page (present in manifest) or native route. */
const available = computed(
	() => parsed.value?.kind === 'native' || (parsed.value?.kind === 'page' && !!page.value),
)
const route = computed(() => {
	if (parsed.value?.kind === 'native') return parsed.value.route
	if (parsed.value?.kind === 'page') return domainPageRoute(parsed.value.pageId)
	return '/'
})
const title = computed(
	() =>
		props.placement.shortcut?.title ??
		page.value?.title ??
		page.value?.id ??
		(parsed.value?.kind === 'native' ? parsed.value.route : '') ??
		'',
)
const icon = computed(() =>
	parsed.value?.kind === 'native' ? LinkIcon : domainRendererIcon(page.value?.renderer),
)
</script>

<template>
	<router-link
		v-if="available"
		class="home-domain-shortcut flex min-w-0 min-h-0 h-full cursor-pointer items-center gap-3 rounded-xl bg-button-bg p-3 no-underline transition-all hover:brightness-90"
		:data-size="dashboardSize"
		:to="route"
	>
		<span
			class="flex size-11 shrink-0 items-center justify-center rounded-lg bg-bg-raised text-secondary"
		>
			<component :is="icon" class="size-6" aria-hidden="true" />
		</span>
		<span class="flex min-w-0 flex-1 flex-col gap-0.5">
			<span
				class="flex min-w-0 items-center gap-1 text-[0.6875rem] font-bold uppercase leading-none text-secondary"
			>
				<component :is="icon" class="size-3" aria-hidden="true" />
				{{ formatMessage(messages.kind) }}
			</span>
			<strong class="truncate text-base font-bold text-contrast">{{ title }}</strong>
			<span v-if="parsed?.kind === 'page'" class="truncate text-xs text-secondary">
				{{ page?.renderer ?? '' }}
			</span>
		</span>
	</router-link>
	<div
		v-else
		class="flex min-w-0 min-h-0 h-full flex-col items-center justify-center gap-2 p-4 text-center"
		:data-size="dashboardSize"
	>
		<IssuesIcon class="size-6 text-secondary" aria-hidden="true" />
		<strong class="max-w-full truncate text-sm text-contrast">{{
			placement.shortcut?.title ?? ''
		}}</strong>
		<span class="text-xs text-secondary">{{ formatMessage(messages.unavailable) }}</span>
	</div>
</template>
