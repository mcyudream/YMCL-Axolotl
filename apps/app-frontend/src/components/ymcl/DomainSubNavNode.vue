<script setup lang="ts">
import { ChevronDownIcon } from '@modrinth/assets'
import { computed, inject, useCssModule } from 'vue'
import { useRoute } from 'vue-router'

import type { YmclNavigationItem } from '@/helpers/ymcl'
import {
	isYmclNavDirectory,
	isYmclNavVisible,
	ymclNavHasTarget,
	ymclNavTo,
} from '@/helpers/ymcl-domain'
import { resolveYmclNavIcon } from '@/helpers/ymcl-nav-icon'

import { DOMAIN_SUB_NAV_CONTEXT, type DomainSubNavContext } from './domain-sub-nav-context'

const props = defineProps<{
	item: YmclNavigationItem
	depth: number
}>()

const route = useRoute()
const ctx = inject<DomainSubNavContext>(DOMAIN_SUB_NAV_CONTEXT)
const style = useCssModule()

const icon = computed(() => resolveYmclNavIcon(props.item.icon))
const title = computed(() => props.item.title?.trim() || props.item.id)
const liveChildren = computed(() =>
	(props.item.children ?? []).filter(
		(child) => child.type !== 'separator' && isYmclNavVisible(child),
	),
)
const hasTarget = computed(() => ymclNavHasTarget(props.item))
/** Pure grouping node: 目录 without its own route/page. */
const isGroup = computed(
	() => isYmclNavDirectory(props.item) && !hasTarget.value && liveChildren.value.length > 0,
)
const isActive = computed(() => {
	const target =
		props.item.page_id ?? (props.item.type === 'page' ? props.item.id : null)
	if (target) return target === ctx?.activePageId.value
	const type = (props.item.type || '').toLowerCase()
	if (type === 'native' || type === 'link' || type === 'route') {
		return route.path === ymclNavTo(props.item)
	}
	return false
})
const collapsed = computed(() => ctx?.isCollapsed(props.item.id) ?? false)
const indentStyle = computed(() => ({ '--subnav-indent': `${props.depth * 14}px` }))

function toggle() {
	ctx?.toggle(props.item.id)
}
</script>

<template>
	<div v-if="item.type === 'separator'" :class="style.divider" role="separator" />
	<section v-else-if="isGroup" :class="style.group">
		<button
			type="button"
			:class="[style.groupButton, 'hover:bg-surface-3 hover:text-contrast']"
			:style="indentStyle"
			:aria-expanded="!collapsed"
			@click="toggle"
		>
			<component :is="icon" class="size-3.5 shrink-0" />
			<span class="truncate">{{ title }}</span>
			<ChevronDownIcon
				class="ml-auto size-3.5 shrink-0 transition-transform"
				:class="collapsed ? '' : 'rotate-180'"
			/>
		</button>
		<div v-show="!collapsed" :class="style.items">
			<DomainSubNavNode
				v-for="child in liveChildren"
				:key="child.id"
				:item="child"
				:depth="depth + 1"
			/>
		</div>
	</section>
	<div v-else :class="style.row">
		<RouterLink
			:to="ymclNavTo(item)"
			:class="[style.itemButton, 'hover:bg-surface-3 hover:text-contrast']"
			:style="indentStyle"
			:data-active="isActive || undefined"
		>
			<component :is="icon" class="size-4 shrink-0" />
			<span class="truncate">{{ title }}</span>
		</RouterLink>
		<button
			v-if="liveChildren.length"
			type="button"
			:class="[style.chevronButton, 'hover:bg-surface-4 hover:text-contrast']"
			:aria-expanded="!collapsed"
			@click="toggle"
		>
			<ChevronDownIcon
				class="size-3.5 transition-transform"
				:class="collapsed ? '' : 'rotate-180'"
			/>
		</button>
		<div v-if="liveChildren.length" v-show="!collapsed" :class="style.items">
			<DomainSubNavNode
				v-for="child in liveChildren"
				:key="child.id"
				:item="child"
				:depth="depth + 1"
			/>
		</div>
	</div>
</template>

<style module>
.divider {
	height: 1px;
	margin: var(--gap-xs) var(--gap-sm);
	background: color-mix(in srgb, var(--surface-4) 55%, transparent);
}

.group,
.row {
	display: flex;
	position: relative;
	min-width: 0;
	flex-direction: column;
	gap: var(--gap-xs);
}

.items {
	display: flex;
	min-width: 0;
	flex-direction: column;
	gap: var(--gap-xs);
}

.groupButton,
.itemButton {
	display: flex;
	width: 100%;
	align-items: center;
	gap: var(--gap-sm);
	border: 0;
	border-radius: var(--radius-sm);
	background: transparent;
	text-align: left;
	cursor: pointer;
	text-decoration: none;
	padding-left: calc(var(--gap-sm) + var(--subnav-indent, 0px));
	padding-right: var(--gap-sm);
	transition:
		background-color 120ms ease,
		color 120ms ease;
}

.groupButton {
	min-height: 1.75rem;
	color: var(--color-secondary);
	font-size: 0.75rem;
	font-weight: 600;
}

.itemButton {
	min-height: 2.25rem;
	color: var(--color-text-primary);
	font-size: 0.875rem;
	font-weight: 600;
}

.itemButton[data-active] {
	background: var(--color-button-bg-selected);
	color: var(--color-button-text-selected);
}

.chevronButton {
	display: flex;
	position: absolute;
	top: 0.375rem;
	right: var(--gap-xs);
	align-items: center;
	justify-content: center;
	width: 1.5rem;
	min-height: 1.5rem;
	border: 0;
	border-radius: var(--radius-sm);
	background: transparent;
	color: var(--color-secondary);
	cursor: pointer;
	transition:
		background-color 120ms ease,
		color 120ms ease;
}
</style>
