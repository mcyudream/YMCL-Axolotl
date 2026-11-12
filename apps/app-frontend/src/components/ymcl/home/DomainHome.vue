<script setup lang="ts">
import {
	ChevronLeftIcon,
	ChevronRightIcon,
	CompassIcon,
	PauseIcon,
	PlayIcon,
} from '@modrinth/assets'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import { useDocumentVisibility, usePreferredReducedMotion } from '@vueuse/core'
import { computed, ref } from 'vue'

import DomainImg from '@/components/ymcl/DomainImg.vue'
import { ymcl } from '@/helpers/ymcl'
import { resolveDomainImageUrl } from '@/helpers/ymcl-domain-image'
import { useYmclStore } from '@/store/ymcl'

import {
	type DomainHomeLanding,
	isDomainHomeMediaUrl,
	normalizeDomainHomeLanding,
} from './domain-home'
import DomainHomeFocus from './DomainHomeFocus.vue'
import DomainHomeLaunch from './DomainHomeLaunch.vue'
import DomainHomeMusic from './DomainHomeMusic.vue'
import DomainHomeProfile from './DomainHomeProfile.vue'
import { releaseDomainHomeVideo, useDomainHomeCarousel } from './use-domain-home-carousel'
import { useDomainHomeVideo } from './use-domain-home-video'

const props = withDefaults(
	defineProps<{
		config: DomainHomeLanding
		preview?: boolean
		playerName?: string | null
		active?: boolean
	}>(),
	{ active: true },
)
const store = useYmclStore()
const { formatMessage } = useVIntl()
const config = computed(() => normalizeDomainHomeLanding(props.config))

const domainName = computed(
	() => store.manifest?.domain?.name || store.activeDomain?.display_name || '',
)
const domainLogo = computed(() => store.manifest?.domain?.logo_url || store.activeDomain?.logo_url)
const slides = computed(() => {
	if (config.value.slides.length) return config.value.slides
	const theme = store.manifest?.theme as { background?: { url?: string } } | undefined
	const image = theme?.background?.url
	return image && isDomainHomeMediaUrl(image)
		? [{ id: 'domain-theme', image, video: '', title: '', description: '' }]
		: []
})
const hovering = ref(false)
const focusWithin = ref(false)
const visibility = useDocumentVisibility()
const reducedMotion = usePreferredReducedMotion()
const video = ref<HTMLVideoElement>()
const {
	index: slideIndex,
	currentSlide,
	paused,
	currentFailed,
	loopVideo,
	changeSlide,
	onVideoEnded,
	onVideoError,
} = useDomainHomeCarousel({
	slides,
	interval: () => config.value.interval,
	active: () => props.active,
	visible: () => visibility.value === 'visible',
	interacting: () => hovering.value || focusWithin.value,
	reducedMotion: () => reducedMotion.value === 'reduce',
	video,
})
const remoteVideoSource = computed(() =>
	currentFailed.value
		? null
		: resolveDomainImageUrl(currentSlide.value?.video, store.activeDomain?.origin),
)
const videoSource = useDomainHomeVideo(remoteVideoSource, async (url) =>
	convertFileSrc(await ymcl.cacheHomeVideo(url)),
)
const copy = computed(() => ({
	title:
		(config.value.copyMode === 'individual' && currentSlide.value?.title) ||
		config.value.title ||
		domainName.value,
	description:
		(config.value.copyMode === 'individual' && currentSlide.value?.description) ||
		config.value.description ||
		store.manifest?.domain?.description,
}))

const messages = defineMessages({
	home: { id: 'app.ymcl.landing.home', defaultMessage: 'Domain home' },
	welcome: { id: 'app.ymcl.landing.welcome', defaultMessage: 'Welcome to' },
	welcomePlayer: { id: 'app.ymcl.landing.welcome-player', defaultMessage: 'Welcome back, {name}' },
	previous: { id: 'app.ymcl.landing.previous', defaultMessage: 'Previous background' },
	next: { id: 'app.ymcl.landing.next', defaultMessage: 'Next background' },
	slide: { id: 'app.ymcl.landing.slide', defaultMessage: 'Background {index} of {total}' },
	pause: { id: 'app.ymcl.landing.pause', defaultMessage: 'Pause slideshow' },
	resume: { id: 'app.ymcl.landing.resume', defaultMessage: 'Resume slideshow' },
})

function onFocusOut(event: FocusEvent) {
	if (!(event.currentTarget as HTMLElement).contains(event.relatedTarget as Node | null))
		focusWithin.value = false
}
</script>

<template>
	<section
		class="domain-home box-border h-full min-h-0 gap-[clamp(0.75rem,2vh,1.5rem)] overflow-hidden"
		:class="{ 'domain-home-preview': preview, 'domain-home-inactive': !active }"
		:style="{ '--domain-home-overlay': config.overlay / 100 }"
	>
		<div class="domain-home-scenery" aria-hidden="true">
			<DomainImg
				v-for="(slide, index) in slides"
				:key="`${slide.id}:${slide.image}`"
				:src="slide.image"
				alt=""
				class="domain-home-background"
				:class="{ 'is-current': index === slideIndex }"
			/>
			<Transition
				name="domain-home-video"
				@before-leave="(element) => (element as HTMLVideoElement).pause()"
				@after-leave="(element) => releaseDomainHomeVideo(element as HTMLVideoElement)"
			>
				<video
					v-if="videoSource"
					ref="video"
					:key="`${currentSlide?.id}:${videoSource}`"
					:src="videoSource"
					:loop="loopVideo"
					muted
					playsinline
					preload="metadata"
					class="domain-home-background is-current"
					@ended="onVideoEnded"
					@error="onVideoError"
				/>
			</Transition>
		</div>
		<div class="domain-home-shade" aria-hidden="true" />
		<header class="domain-home-header shrink-0">
			<div class="domain-home-identity">
				<DomainImg :src="domainLogo" alt="" class="domain-home-logo"
					><template #fallback><CompassIcon class="domain-home-logo" /></template
				></DomainImg>
				<div>
					<strong>{{ domainName }}</strong
					><span>{{ formatMessage(messages.home) }}</span>
				</div>
			</div>
		</header>

		<div
			class="domain-home-main grid min-h-0 min-w-0 flex-1 items-stretch gap-8 overflow-y-auto overscroll-contain py-[clamp(0.5rem,2vh,2rem)]"
			:class="{ 'has-focus': config.focusSources.length > 0 }"
		>
			<div class="domain-home-intro min-w-0 self-center">
				<p class="domain-home-eyebrow">
					{{
						playerName
							? formatMessage(messages.welcomePlayer, { name: playerName })
							: formatMessage(messages.welcome)
					}}
				</p>
				<Transition name="domain-home-copy" mode="out-in">
					<div :key="`${copy.title}:${copy.description}`">
						<h1>{{ copy.title }}</h1>
						<p v-if="copy.description" class="domain-home-description">{{ copy.description }}</p>
					</div>
				</Transition>
			</div>
			<div v-if="config.focusSources.length" class="relative min-h-0 min-w-0">
				<DomainHomeFocus
					class="min-h-0 max-h-full min-[901px]:absolute min-[901px]:inset-x-0 min-[901px]:top-1/2 min-[901px]:-translate-y-1/2"
					:sources="config.focusSources"
					:preview="preview"
				/>
			</div>
		</div>

		<nav
			v-if="slides.length > 1 || currentSlide?.video"
			class="domain-home-carousel shrink-0"
			:aria-label="formatMessage(messages.home)"
			@mouseenter="hovering = true"
			@mouseleave="hovering = false"
			@focusin="focusWithin = true"
			@focusout="onFocusOut"
		>
			<button
				v-if="slides.length > 1"
				type="button"
				class="domain-home-circle"
				:aria-label="formatMessage(messages.previous)"
				@click="changeSlide(slideIndex - 1)"
			>
				<ChevronLeftIcon />
			</button>
			<div v-if="slides.length > 1" class="domain-home-dots">
				<button
					v-for="(slide, index) in slides"
					:key="slide.id"
					type="button"
					:aria-label="formatMessage(messages.slide, { index: index + 1, total: slides.length })"
					:aria-current="index === slideIndex ? 'true' : undefined"
					@click="changeSlide(index)"
				>
					<span />
				</button>
			</div>
			<button
				v-if="slides.length > 1"
				type="button"
				class="domain-home-circle"
				:aria-label="formatMessage(messages.next)"
				@click="changeSlide(slideIndex + 1)"
			>
				<ChevronRightIcon />
			</button>
			<button
				type="button"
				class="domain-home-circle"
				:aria-label="formatMessage(paused ? messages.resume : messages.pause)"
				:aria-pressed="paused"
				@click="paused = !paused"
			>
				<PlayIcon v-if="paused" /><PauseIcon v-else />
			</button>
		</nav>

		<footer
			class="grid min-h-0 max-h-[60%] shrink-0 grid-cols-[minmax(0,1fr)_minmax(22rem,28rem)] grid-rows-[minmax(0,1fr)] items-end gap-6 max-[900px]:flex max-[900px]:flex-col max-[900px]:items-stretch"
		>
			<div class="domain-home-ambience flex min-w-0 shrink-0 flex-col items-start gap-3">
				<DomainHomeProfile v-if="!preview" />
				<DomainHomeMusic v-if="config.music.url" :music="config.music" :preview="preview" />
			</div>
			<DomainHomeLaunch :featured-server-id="config.featuredServerId" :preview="preview" />
		</footer>
	</section>
</template>

<style scoped>
.domain-home {
	position: relative;
	display: flex;
	flex-direction: column;
	padding: clamp(1.5rem, 4vw, 3rem);
	padding-block: clamp(1rem, 3vh, 2rem);
	padding-inline-end: max(var(--domain-home-navigation-space, 0px), clamp(1.5rem, 4vw, 3rem));
	isolation: isolate;
	background: #111d22;
	color: #fff;
}
.domain-home-preview {
	height: 38rem;
	min-height: 38rem;
	border-radius: var(--radius-xl);
}
.domain-home-scenery,
.domain-home-background,
.domain-home-shade {
	position: absolute;
	z-index: -1;
	inset: 0;
	width: 100%;
	height: 100%;
	pointer-events: none;
}
.domain-home-scenery {
	background:
		radial-gradient(
			ellipse at 75% 20%,
			color-mix(in srgb, var(--color-brand) 24%, #263d48),
			transparent 65%
		),
		linear-gradient(155deg, #24363e, #10191d 80%);
}
.domain-home-background {
	object-fit: cover;
	opacity: 0;
	transition: opacity 1000ms ease;
}
.domain-home-background.is-current {
	opacity: 1;
}
.domain-home-background.domain-home-video-enter-from,
.domain-home-background.domain-home-video-leave-to {
	opacity: 0;
}
.domain-home-shade {
	background:
		linear-gradient(0deg, rgb(5 11 14 / 85%), transparent 65%),
		linear-gradient(90deg, rgb(5 11 14 / 40%), transparent 80%),
		rgb(0 0 0 / var(--domain-home-overlay));
}
.domain-home-header {
	display: flex;
	min-width: 0;
	align-items: center;
	justify-content: space-between;
	gap: 1rem;
}
.domain-home-identity {
	display: flex;
	min-width: 0;
	align-items: center;
	gap: 0.75rem;
}
.domain-home-logo {
	width: 2.5rem;
	height: 2.5rem;
	flex-shrink: 0;
	border-radius: var(--radius-md);
	object-fit: cover;
}
.domain-home-identity > div {
	display: flex;
	min-width: 0;
	flex-direction: column;
	gap: 0.25rem;
}
.domain-home-identity strong {
	overflow: hidden;
	font-size: 0.9375rem;
	text-overflow: ellipsis;
	white-space: nowrap;
}
.domain-home-identity span {
	font-size: 0.6875rem;
	color: rgb(255 255 255 / 60%);
}
.domain-home-main.has-focus {
	grid-template-columns: minmax(0, 1fr) minmax(16rem, 21rem);
}
.domain-home-eyebrow {
	margin: 0 0 1rem;
	font-size: 0.875rem;
	font-weight: 500;
	color: rgb(255 255 255 / 75%);
}
.domain-home-intro h1 {
	margin: 0;
	max-width: 14em;
	font-size: clamp(2.5rem, 5vw, 4.5rem);
	font-weight: 700;
	letter-spacing: -0.04em;
	line-height: 1.15;
	text-wrap: balance;
	overflow-wrap: anywhere;
	text-shadow: 0 2px 24px rgb(0 0 0 / 20%);
}
.domain-home-description {
	max-width: 32em;
	margin: 1.5rem 0 0;
	font-size: 0.9375rem;
	line-height: 1.9;
	color: rgb(255 255 255 / 78%);
	white-space: pre-line;
	overflow-wrap: anywhere;
}
.domain-home-carousel {
	display: flex;
	align-items: center;
	gap: 0.5rem;
}
.domain-home-circle {
	display: grid;
	width: 2rem;
	height: 2rem;
	flex-shrink: 0;
	place-items: center;
	padding: 0;
	border: 1px solid rgb(255 255 255 / 16%);
	border-radius: 50%;
	background: rgb(0 0 0 / 16%);
	color: #fff;
	cursor: pointer;
}
.domain-home-circle:hover {
	background: rgb(255 255 255 / 18%);
}
.domain-home-circle svg {
	width: 0.875rem;
	height: 0.875rem;
}
.domain-home-dots {
	display: flex;
	flex-wrap: wrap;
	align-items: center;
	gap: 0.125rem;
}
.domain-home-dots button {
	display: grid;
	min-width: 1.25rem;
	height: 2rem;
	place-items: center;
	padding: 0.25rem;
	border: 0;
	background: transparent;
	cursor: pointer;
}
.domain-home-dots span {
	width: 0.375rem;
	height: 0.375rem;
	border-radius: 999px;
	background: rgb(255 255 255 / 40%);
	transition:
		width 200ms ease,
		background 200ms ease;
}
.domain-home-dots [aria-current='true'] span {
	width: 1.5rem;
	background: #fff;
}
.domain-home > :not(.domain-home-scenery, .domain-home-shade) {
	opacity: 1;
	transform: translate3d(0, 0, 0);
	transition:
		opacity 420ms 100ms ease,
		transform var(--domain-home-page-duration, 640ms) cubic-bezier(0.16, 1, 0.3, 1);
}
.domain-home-inactive > :not(.domain-home-scenery, .domain-home-shade) {
	opacity: 0;
	transform: translate3d(0, -1.25rem, 0);
	transition:
		opacity 180ms ease,
		transform var(--domain-home-page-duration, 640ms) cubic-bezier(0.16, 1, 0.3, 1);
}
.domain-home-copy-enter-active,
.domain-home-copy-leave-active {
	transition:
		opacity 200ms ease,
		transform 200ms ease;
}
.domain-home-copy-enter-from {
	opacity: 0;
	transform: translateY(0.5rem);
}
.domain-home-copy-leave-to {
	opacity: 0;
	transform: translateY(-0.5rem);
}
@media (max-width: 900px) {
	.domain-home-main.has-focus {
		grid-template-columns: minmax(0, 1fr);
	}
	.domain-home-ambience:empty {
		display: none;
	}
}
@media (prefers-reduced-motion: reduce) {
	.domain-home > :not(.domain-home-scenery, .domain-home-shade),
	.domain-home-background,
	.domain-home-copy-enter-active,
	.domain-home-copy-leave-active {
		transition: none;
	}
}
</style>
