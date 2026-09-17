<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed, provide, ref, watch } from 'vue'

import type { YmclNavigationItem } from '@/helpers/ymcl'
import {
	isYmclNavVisible,
	ymclNavHasTarget,
	ymclNavTo,
} from '@/helpers/ymcl-domain'
import { resolveYmclNavIcon } from '@/helpers/ymcl-nav-icon'
import { useYmclStore } from '@/store/ymcl'

import { DOMAIN_SUB_NAV_CONTEXT } from './domain-sub-nav-context'
import DomainSubNavNode from './DomainSubNavNode.vue'

const props = defineProps<{
	pageId: string
}>()

const { formatMessage } = useVIntl()
const ymclStore = useYmclStore()

const messages = defineMessages({
	navLabel: {
		id: 'app.ymcl.domain-subnav.label',
		defaultMessage: '域页面导航',
	},
})

/**
 * Path from the navigation root to the item targeting `pageId`.
 * A nav item targets a page via `page_id`, or (type "page") via its own id.
 */
function findPagePath(
	items: readonly YmclNavigationItem[],
	pageId: string,
): YmclNavigationItem[] | null {
	for (const item of items) {
		const target = item.page_id ?? (item.type === 'page' ? item.id : null)
		if (target === pageId) return [item]
		const children = item.children ?? []
		if (children.length) {
			const sub = findPagePath(children, pageId)
			if (sub) return [item, ...sub]
		}
	}
	return null
}

const navPath = computed(() => findPagePath(ymclStore.navigation, props.pageId))
const root = computed(() => navPath.value?.[0] ?? null)
const rootChildren = computed(() =>
	(root.value?.children ?? []).filter(
		(item) => item.type !== 'separator' && isYmclNavVisible(item),
	),
)
/** Only render when the active page actually lives under a nested subtree. */
const visible = computed(() => root.value !== null && rootChildren.value.length > 0)
const rootIcon = computed(() => resolveYmclNavIcon(root.value?.icon))
const rootTitle = computed(() => root.value?.title?.trim() || root.value?.id || '')
const rootTo = computed(() =>
	root.value && ymclNavHasTarget(root.value) ? ymclNavTo(root.value) : null,
)

const collapsedKeys = ref<Set<string>>(new Set())

// Keep directories on the active path expanded whenever the route changes.
watch(
	navPath,
	(path) => {
		if (!path?.length) return
		const next = new Set(collapsedKeys.value)
		for (const item of path) next.delete(item.id)
		collapsedKeys.value = next
	},
	{ immediate: true },
)

provide(DOMAIN_SUB_NAV_CONTEXT, {
	activePageId: computed(() => props.pageId),
	isCollapsed: (key) => collapsedKeys.value.has(key),
	toggle: (key) => {
		const next = new Set(collapsedKeys.value)
		if (next.has(key)) next.delete(key)
		else next.add(key)
		collapsedKeys.value = next
	},
})
</script>

<template>
	<aside
		v-if="visible"
		class="domain-subnav"
		:aria-label="formatMessage(messages.navLabel)"
	>
		<RouterLink
			v-if="rootTo"
			:to="rootTo"
			class="domain-subnav-header hover:bg-surface-3 hover:text-contrast"
		>
			<component :is="rootIcon" class="size-4 shrink-0" />
			<span class="truncate">{{ rootTitle }}</span>
		</RouterLink>
		<div v-else class="domain-subnav-header is-static">
			<component :is="rootIcon" class="size-4 shrink-0" />
			<span class="truncate">{{ rootTitle }}</span>
		</div>
		<nav class="domain-subnav-list">
			<DomainSubNavNode
				v-for="item in rootChildren"
				:key="item.id"
				:item="item"
				:depth="0"
			/>
		</nav>
	</aside>
</template>

<style scoped>
.domain-subnav {
	display: flex;
	width: 15rem;
	flex-shrink: 0;
	flex-direction: column;
	gap: var(--gap-sm);
	align-self: flex-start;
	position: sticky;
	top: var(--gap-lg);
	max-height: calc(100vh - 8rem);
	overflow-y: auto;
	border-radius: var(--radius-md);
	background: var(--surface-2);
	padding: var(--gap-md);
}

.domain-subnav-header {
	display: flex;
	align-items: center;
	gap: var(--gap-sm);
	min-height: 2rem;
	padding: 0 var(--gap-sm);
	border-radius: var(--radius-sm);
	color: var(--color-contrast);
	font-size: 0.875rem;
	font-weight: 700;
	text-decoration: none;
	transition:
		background-color 120ms ease,
		color 120ms ease;
}

.domain-subnav-header.is-static {
	cursor: default;
}

.domain-subnav-list {
	display: flex;
	min-width: 0;
	flex-direction: column;
	gap: var(--gap-xs);
}

@media (max-width: 800px) {
	.domain-subnav {
		width: 100%;
		position: static;
		max-height: 18rem;
	}
}
</style>
