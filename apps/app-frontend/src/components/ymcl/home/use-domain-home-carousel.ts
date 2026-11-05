import { computed, onScopeDispose, ref, toValue, watch, type MaybeRefOrGetter, type Ref } from 'vue'

import type { DomainHomeSlide } from './domain-home.ts'

export function releaseDomainHomeVideo(element: HTMLVideoElement) {
	element.pause()
	element.removeAttribute('src')
	element.load()
}

/** Images use the configured interval; videos advance only after playback completes. */
export function useDomainHomeCarousel(options: {
	slides: Ref<DomainHomeSlide[]>
	interval: MaybeRefOrGetter<number>
	active: MaybeRefOrGetter<boolean>
	visible: MaybeRefOrGetter<boolean>
	interacting: MaybeRefOrGetter<boolean>
	reducedMotion: MaybeRefOrGetter<boolean>
	video: Ref<HTMLVideoElement | undefined>
}) {
	const index = ref(0)
	const currentSlide = computed(() => options.slides.value[index.value])
	const paused = ref(toValue(options.reducedMotion))
	const ended = ref(false)
	const failed = ref(new Set<string>())
	const identity = (slide: DomainHomeSlide) => `${slide.id}:${slide.video || slide.image}`
	const currentFailed = computed(() => {
		const slide = currentSlide.value
		return !!slide && failed.value.has(identity(slide))
	})
	const loopVideo = computed(
		() => options.slides.value.filter((slide) => !failed.value.has(identity(slide))).length === 1,
	)
	const running = computed(
		() => toValue(options.active) && toValue(options.visible) && !paused.value,
	)
	const canAdvance = computed(() => running.value && !toValue(options.interacting))
	let timer: ReturnType<typeof setTimeout> | undefined

	function clearTimer() {
		clearTimeout(timer)
		timer = undefined
	}
	function changeSlide(next: number) {
		const count = options.slides.value.length
		if (!count) return
		index.value = ((next % count) + count) % count
		const slide = currentSlide.value
		if (slide && failed.value.has(identity(slide))) {
			failed.value = new Set([...failed.value].filter((key) => key !== identity(slide)))
		}
	}
	function advance() {
		const slides = options.slides.value
		for (let offset = 1; offset < slides.length; offset++) {
			const next = (index.value + offset) % slides.length
			if (!failed.value.has(identity(slides[next]))) {
				index.value = next
				return
			}
		}
	}
	function onVideoEnded(event: Event) {
		if (event.target === options.video.value) ended.value = true
	}
	function onVideoError(event?: Event) {
		if (event && event.target !== options.video.value) return
		const slide = currentSlide.value
		if (slide?.video) failed.value = new Set([...failed.value, identity(slide)])
	}

	watch(
		() => options.slides.value.map(identity).join('\n'),
		() => {
			index.value = 0
			failed.value = new Set()
		},
	)
	watch(
		() => currentSlide.value && identity(currentSlide.value),
		() => (ended.value = false),
		{ flush: 'sync' },
	)
	watch(
		() => toValue(options.reducedMotion),
		(reduce) => {
			if (reduce) paused.value = true
		},
	)
	watch(
		[options.video, running, currentFailed],
		([element, play, unavailable], _, onCleanup) => {
			if (!element) return
			let cancelled = false
			onCleanup(() => {
				cancelled = true
				element.pause()
			})
			if (!play || unavailable || element.ended) {
				element.pause()
				return
			}
			element.muted = true
			void element.play().then(
				() => {
					if (options.video.value !== element || !running.value) element.pause()
				},
				(error: unknown) => {
					if (cancelled) return
					const name = error instanceof Error ? error.name : ''
					if (name === 'NotAllowedError') paused.value = true
					else if (name !== 'AbortError') onVideoError()
				},
			)
		},
		{ immediate: true, flush: 'post' },
	)
	watch(
		[currentSlide, canAdvance, currentFailed, ended, () => toValue(options.interval)],
		([slide, advanceAllowed, unavailable, complete, interval]) => {
			clearTimer()
			if (!slide || !advanceAllowed) return
			if (unavailable || (slide.video && complete)) advance()
			else if (!slide.video && options.slides.value.length > 1)
				timer = setTimeout(advance, interval * 1000)
		},
		{ immediate: true },
	)
	onScopeDispose(() => {
		clearTimer()
		if (options.video.value) releaseDomainHomeVideo(options.video.value)
	})

	return {
		index,
		currentSlide,
		paused,
		currentFailed,
		loopVideo,
		changeSlide,
		onVideoEnded,
		onVideoError,
	}
}
