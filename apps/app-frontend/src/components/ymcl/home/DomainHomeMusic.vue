<script setup lang="ts">
import { LoaderIcon, PauseIcon, PlayIcon } from '@modrinth/assets'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'

import { resolveDomainImageUrl } from '@/helpers/ymcl-domain-image'
import { useYmclStore } from '@/store/ymcl'

import type { DomainHomeLanding } from './domain-home'
import DomainHomeRange from './DomainHomeRange.vue'

const props = defineProps<{ music: DomainHomeLanding['music']; preview?: boolean }>()
const { formatMessage } = useVIntl()
const store = useYmclStore()
const audio = ref<HTMLAudioElement>()
const playing = ref(false)
const pending = ref(false)
const failed = ref(false)
const blocked = ref(false)
const volume = ref(props.music.volume)
const source = computed(() => resolveDomainImageUrl(props.music.url, store.activeDomain?.origin))
const messages = defineMessages({
	title: { id: 'app.ymcl.landing.music', defaultMessage: 'Background music' },
	play: { id: 'app.ymcl.landing.music-play', defaultMessage: 'Play music' },
	pause: { id: 'app.ymcl.landing.music-pause', defaultMessage: 'Pause music' },
	volume: { id: 'app.ymcl.landing.music-volume', defaultMessage: 'Music volume' },
	blocked: { id: 'app.ymcl.landing.music-blocked', defaultMessage: 'Click play to enable music' },
	error: {
		id: 'app.ymcl.landing.music-error',
		defaultMessage: 'Music unavailable. Click play to retry.',
	},
})

async function play() {
	const element = audio.value
	if (!element || pending.value) return
	pending.value = true
	blocked.value = false
	try {
		if (failed.value) element.load()
		failed.value = false
		await element.play()
	} catch (error) {
		if (audio.value !== element) return
		if (error instanceof DOMException && error.name === 'NotAllowedError') blocked.value = true
		else if (!(error instanceof DOMException && error.name === 'AbortError')) failed.value = true
	} finally {
		if (audio.value === element) pending.value = false
	}
}

function toggle() {
	if (playing.value) audio.value?.pause()
	else void play()
}

function onAudioError() {
	failed.value = true
	pending.value = false
}

watch(
	[source, () => props.music.autoplay, () => props.preview],
	async (_, __, onCleanup) => {
		let cancelled = false
		onCleanup(() => {
			cancelled = true
			audio.value?.pause()
		})
		audio.value?.pause()
		failed.value = false
		blocked.value = false
		pending.value = false
		await nextTick()
		if (cancelled) return
		if (audio.value) audio.value.volume = volume.value / 100
		if (source.value && props.music.autoplay && !props.preview) void play()
	},
	{ immediate: true },
)
watch(
	() => props.music.volume,
	(value) => {
		volume.value = value
	},
)
watch(volume, (value) => {
	if (audio.value) audio.value.volume = value / 100
})
onBeforeUnmount(() => audio.value?.pause())
</script>

<template>
	<div v-if="source" class="domain-music" :data-playing="playing">
		<audio
			ref="audio"
			:key="source"
			:src="source"
			loop
			preload="none"
			@play="playing = true"
			@pause="playing = false"
			@error="onAudioError"
		/>
		<button
			type="button"
			class="domain-music-toggle"
			:disabled="pending"
			:aria-label="formatMessage(playing ? messages.pause : messages.play)"
			@click="toggle"
		>
			<LoaderIcon v-if="pending" class="animate-spin" />
			<PauseIcon v-else-if="playing" />
			<PlayIcon v-else />
		</button>
		<div class="domain-music-copy">
			<span class="domain-music-label"
				><span class="domain-music-bars" aria-hidden="true"><i /><i /><i /></span
				>{{ formatMessage(messages.title) }}</span
			>
			<strong>{{ music.title || formatMessage(messages.title) }}</strong>
			<span v-if="failed || blocked" class="domain-music-status" role="status">{{
				formatMessage(failed ? messages.error : messages.blocked)
			}}</span>
		</div>
		<DomainHomeRange v-model="volume" :aria-label="formatMessage(messages.volume)" />
	</div>
</template>

<style scoped>
.domain-music {
	display: flex;
	align-items: center;
	gap: 0.75rem;
	max-width: 24rem;
	padding: 0.75rem 1rem;
	border: 1px solid rgb(255 255 255 / 16%);
	border-radius: var(--radius-xl);
	background: rgb(12 18 24 / 56%);
	backdrop-filter: blur(16px);
	color: #fff;
}
.domain-music-toggle {
	display: grid;
	width: 2.5rem;
	height: 2.5rem;
	flex-shrink: 0;
	place-items: center;
	border: 1px solid rgb(255 255 255 / 18%);
	border-radius: 50%;
	background: rgb(255 255 255 / 8%);
	color: #fff;
	cursor: pointer;
}
.domain-music-toggle:hover {
	background: rgb(255 255 255 / 18%);
}
.domain-music-toggle:disabled {
	cursor: wait;
	opacity: 0.6;
}
.domain-music-toggle svg {
	width: 1rem;
	height: 1rem;
}
.domain-music-copy {
	display: flex;
	min-width: 0;
	flex: 1;
	flex-direction: column;
	gap: 0.25rem;
}
.domain-music-copy strong {
	overflow: hidden;
	font-size: 0.8125rem;
	font-weight: 500;
	text-overflow: ellipsis;
	white-space: nowrap;
}
.domain-music-label {
	display: flex;
	align-items: center;
	gap: 0.375rem;
	font-size: 0.625rem;
	color: rgb(255 255 255 / 60%);
}
.domain-music-status {
	font-size: 0.6875rem;
	line-height: 1.5;
	color: rgb(255 255 255 / 75%);
}
.domain-music input {
	width: 3.5rem;
	--domain-home-range-track: rgb(255 255 255 / 25%);
}
.domain-music-bars {
	display: flex;
	height: 0.625rem;
	align-items: center;
	gap: 2px;
}
.domain-music-bars i {
	width: 2px;
	height: 40%;
	background: currentColor;
}
.domain-music-bars i:nth-child(2) {
	height: 100%;
	animation-delay: 150ms;
}
.domain-music-bars i:nth-child(3) {
	height: 65%;
	animation-delay: 300ms;
}
.domain-music[data-playing='true'] .domain-music-bars i {
	animation: music-level 900ms ease-in-out infinite alternate;
}
@keyframes music-level {
	to {
		height: 20%;
	}
}
@media (prefers-reduced-motion: reduce) {
	.domain-music-bars i {
		animation: none;
	}
}
</style>
