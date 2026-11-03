<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'
import { nextTick, ref } from 'vue'

import { useYmclStore } from '@/store/ymcl'

import type { DomainHomeLanding } from './domain-home'
import {
	canTurnDomainHomePage,
	createDomainHomeWheelPager,
	DOMAIN_HOME_PAGE_TRANSITION_MS,
	type DomainHomePage,
} from './domain-home-paging'
import DomainHome from './DomainHome.vue'

defineProps<{
	config: DomainHomeLanding
	playerName: string | null
}>()

const store = useYmclStore()
const { formatMessage } = useVIntl()
const page = defineModel<DomainHomePage>('page', { required: true })
const informationVisited = ref(page.value === 1)
const landingPage = ref<HTMLElement>()
const informationPage = ref<HTMLElement>()
const gesture = createDomainHomeWheelPager()
const messages = defineMessages({
	navigation: { id: 'app.ymcl.landing.pages.navigation', defaultMessage: 'Home pages' },
	landing: { id: 'app.ymcl.landing.pages.landing', defaultMessage: 'Home' },
	information: { id: 'app.ymcl.landing.pages.information', defaultMessage: 'Domain information' },
	empty: {
		id: 'app.ymcl.landing.pages.empty',
		defaultMessage: 'There are no information cards here yet.',
	},
})

async function changePage(next: DomainHomePage) {
	if (page.value === next) return
	const leaving = page.value === 0 ? landingPage.value : informationPage.value
	const moveFocus = leaving?.contains(document.activeElement)
	page.value = next
	informationVisited.value ||= next === 1
	gesture.lock(performance.now())
	await nextTick()
	if (moveFocus) {
		const entering = next === 0 ? landingPage.value : informationPage.value
		entering?.focus({ preventScroll: true })
	}
}

function ownsWheel(target: Element, panel: HTMLElement): boolean {
	if (!panel.contains(target)) return false
	if (
		target.closest(
			'input, textarea, select, [contenteditable="true"], [role="dialog"], [role="menu"], [role="listbox"]',
		)
	)
		return true
	for (
		let element: Element | null = target;
		element && element !== panel;
		element = element.parentElement
	) {
		const style = getComputedStyle(element)
		if (/(auto|scroll)/.test(style.overflowY) && element.scrollHeight > element.clientHeight + 2)
			return true
	}
	return false
}

function onWheel(event: WheelEvent) {
	if (event.defaultPrevented || event.ctrlKey || event.metaKey || event.altKey || event.shiftKey)
		return
	const panel = page.value === 0 ? landingPage.value : informationPage.value
	const target = event.target
	if (!panel || !(target instanceof Element)) return
	const result = gesture.consume({
		deltaX: event.deltaX,
		deltaY: event.deltaY,
		deltaMode: event.deltaMode,
		timestamp: performance.now(),
		viewportHeight: panel.clientHeight,
		canTurn: !ownsWheel(target, panel) && canTurnDomainHomePage(page.value, event.deltaY, panel),
	})
	if (result.preventDefault) event.preventDefault()
	if (result.turn) void changePage(result.turn > 0 ? 1 : 0)
}
</script>

<template>
	<div
		class="relative h-full min-h-0 min-w-0 overflow-hidden"
		:style="{
			'--domain-home-page-duration': `${DOMAIN_HOME_PAGE_TRANSITION_MS}ms`,
			'--domain-home-navigation-space': '3.5rem',
		}"
		@wheel="onWheel"
	>
		<section
			ref="landingPage"
			class="absolute inset-0 overflow-y-auto overscroll-contain outline-none"
			:inert="page !== 0"
			:aria-hidden="page !== 0"
			:aria-label="formatMessage(messages.landing)"
			tabindex="-1"
		>
			<DomainHome :config="config" :player-name="playerName" :active="page === 0" />
		</section>
		<div
			class="pointer-events-none absolute inset-0 bg-[color-mix(in_srgb,var(--surface-1)_94%,transparent)] backdrop-blur-xl transition-opacity duration-[var(--domain-home-page-duration)] ease-in-out motion-reduce:transition-none"
			:class="page === 1 ? 'opacity-100' : 'opacity-0'"
			aria-hidden="true"
		/>
		<section
			ref="informationPage"
			class="domain-home-information absolute inset-0 overflow-y-auto overscroll-contain outline-none"
			:class="{ 'is-current': page === 1, 'pointer-events-none': page !== 1 }"
			:inert="page !== 1"
			:aria-hidden="page !== 1"
			:aria-label="formatMessage(messages.information)"
			tabindex="-1"
		>
			<div v-if="informationVisited" class="min-h-full min-w-0 pr-8">
				<header class="mx-auto w-full max-w-[96rem] px-6 pb-0 pt-6">
					<p class="m-0 text-sm text-secondary">
						{{ store.manifest?.domain?.name || store.activeDomain?.display_name }}
					</p>
					<h1 class="mb-0 mt-2 text-2xl font-semibold text-contrast">
						{{ formatMessage(messages.information) }}
					</h1>
				</header>
				<slot name="information">
					<p class="m-0 p-6 text-secondary">{{ formatMessage(messages.empty) }}</p>
				</slot>
			</div>
		</section>
		<nav
			class="absolute right-2 top-1/2 z-10 flex -translate-y-1/2 flex-col gap-1 transition-colors duration-[var(--domain-home-page-duration)] motion-reduce:transition-none"
			:class="page === 0 ? 'text-white' : 'text-secondary'"
			:aria-label="formatMessage(messages.navigation)"
		>
			<button
				v-for="(label, index) in [messages.landing, messages.information]"
				:key="label.id"
				v-tooltip.left="formatMessage(label)"
				type="button"
				:aria-current="page === index ? 'page' : undefined"
				:aria-label="formatMessage(label)"
				class="group grid size-8 cursor-pointer place-items-center rounded-full border-0 bg-transparent p-0 text-inherit transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand"
				:class="
					page === 0
						? 'hover:bg-white/10 active:bg-white/20'
						: 'hover:bg-surface-3 active:bg-surface-4'
				"
				@click="changePage(index as DomainHomePage)"
			>
				<span
					class="w-1.5 rounded-full bg-current transition-[height,opacity] duration-300 group-hover:opacity-100 group-focus-visible:opacity-100 motion-reduce:transition-none"
					:class="page === index ? 'h-5 opacity-90' : 'h-1.5 opacity-40'"
					aria-hidden="true"
				/>
			</button>
		</nav>
	</div>
</template>

<style scoped>
.domain-home-information {
	visibility: hidden;
	opacity: 0;
	transform: translate3d(0, 1.5rem, 0);
	transition:
		opacity 180ms ease,
		transform var(--domain-home-page-duration) cubic-bezier(0.16, 1, 0.3, 1),
		visibility 0s var(--domain-home-page-duration);
}
.domain-home-information.is-current {
	visibility: visible;
	opacity: 1;
	transform: translate3d(0, 0, 0);
	transition:
		opacity 420ms 100ms ease,
		transform var(--domain-home-page-duration) cubic-bezier(0.16, 1, 0.3, 1);
}
@media (prefers-reduced-motion: reduce) {
	.domain-home-information,
	.domain-home-information.is-current {
		transition: none;
	}
}
</style>
