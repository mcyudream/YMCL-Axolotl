import assert from 'node:assert/strict'
import test from 'node:test'
import { effectScope, nextTick, ref, shallowRef } from 'vue'

import { normalizeDomainHomeLanding } from './domain-home.ts'
import { useDomainHomeCarousel } from './use-domain-home-carousel.ts'

function fakeVideo() {
	return {
		ended: false,
		muted: false,
		currentTime: 0,
		plays: 0,
		pauses: 0,
		released: false,
		loads: 0,
		async play() {
			this.plays++
		},
		pause() {
			this.pauses++
		},
		removeAttribute(name: string) {
			this.released = name === 'src'
		},
		load() {
			this.loads++
		},
	}
}

function setup(paths: string[], reduce = false) {
	const scope = effectScope()
	const slides = ref(
		normalizeDomainHomeLanding({
			slides: paths.map((path, index) => ({
				id: String(index),
				[path.endsWith('.mp4') ? 'video' : 'image']: path,
			})),
		}).slides,
	)
	const active = ref(true)
	const visible = ref(true)
	const interacting = ref(false)
	const video = shallowRef<HTMLVideoElement>()
	const carousel = scope.run(() =>
		useDomainHomeCarousel({
			slides,
			interval: 3,
			active,
			visible,
			interacting,
			reducedMotion: reduce,
			video,
		}),
	)!
	const attachVideo = async () => {
		const element = fakeVideo()
		video.value = element as unknown as HTMLVideoElement
		await nextTick()
		return element
	}
	return {
		...carousel,
		slides,
		active,
		visible,
		interacting,
		video,
		attachVideo,
		stop: () => scope.stop(),
	}
}

function mediaEvent(element: ReturnType<typeof fakeVideo>): Event {
	return { target: element } as unknown as Event
}

test('image timers never cut a video short and restart for the following image', async (t) => {
	t.mock.timers.enable({ apis: ['setTimeout'] })
	const carousel = setup(['/first.png', '/movie.mp4', '/last.png'])
	t.after(carousel.stop)
	t.mock.timers.tick(3000)
	await nextTick()
	assert.equal(carousel.index.value, 1)
	const video = await carousel.attachVideo()
	assert.equal(video.plays, 1)
	assert.equal(video.muted, true)
	t.mock.timers.tick(120000)
	await nextTick()
	assert.equal(carousel.index.value, 1)
	video.ended = true
	carousel.onVideoEnded(mediaEvent(video))
	await nextTick()
	assert.equal(carousel.index.value, 2)
	t.mock.timers.tick(3000)
	await nextTick()
	assert.equal(carousel.index.value, 0)
})

test('leaving the page, hiding the window and pausing retain video playback position', async (t) => {
	const carousel = setup(['/movie.mp4', '/image.png'])
	t.after(carousel.stop)
	const video = await carousel.attachVideo()
	video.currentTime = 17
	carousel.active.value = false
	await nextTick()
	assert.ok(video.pauses > 0)
	carousel.active.value = true
	carousel.visible.value = false
	await nextTick()
	assert.equal(video.plays, 1)
	carousel.visible.value = true
	await nextTick()
	assert.equal(video.plays, 2)
	assert.equal(video.currentTime, 17)
	carousel.paused.value = true
	await nextTick()
	video.ended = true
	carousel.onVideoEnded(mediaEvent(video))
	await nextTick()
	assert.equal(carousel.index.value, 0)
	carousel.paused.value = false
	await nextTick()
	assert.equal(carousel.index.value, 1)
})

test('carousel interaction defers a completed video until the controls are released', async (t) => {
	const carousel = setup(['/movie.mp4', '/image.png'])
	t.after(carousel.stop)
	const video = await carousel.attachVideo()
	carousel.interacting.value = true
	video.ended = true
	carousel.onVideoEnded(mediaEvent(video))
	await nextTick()
	assert.equal(carousel.index.value, 0)
	carousel.interacting.value = false
	await nextTick()
	assert.equal(carousel.index.value, 1)
})

test('broken videos are skipped without looping over failures and can be retried manually', async (t) => {
	t.mock.timers.enable({ apis: ['setTimeout'] })
	const carousel = setup(['/broken-a.mp4', '/broken-b.mp4', '/image.png'])
	t.after(carousel.stop)
	const first = await carousel.attachVideo()
	carousel.onVideoError(mediaEvent(first))
	await nextTick()
	assert.equal(carousel.index.value, 1)
	const second = await carousel.attachVideo()
	carousel.onVideoEnded(mediaEvent(first))
	await nextTick()
	assert.equal(carousel.index.value, 1)
	carousel.onVideoError(mediaEvent(second))
	await nextTick()
	assert.equal(carousel.index.value, 2)
	t.mock.timers.tick(3000)
	await nextTick()
	assert.equal(carousel.index.value, 2)
	carousel.changeSlide(0)
	await nextTick()
	assert.equal(carousel.currentFailed.value, false)
	assert.equal((await carousel.attachVideo()).plays, 1)
})

test('reduced motion waits for explicit playback and disposing releases video and timers', async (t) => {
	t.mock.timers.enable({ apis: ['setTimeout'] })
	const carousel = setup(['/movie.mp4', '/image.png'], true)
	const video = await carousel.attachVideo()
	assert.equal(video.plays, 0)
	carousel.paused.value = false
	await nextTick()
	assert.equal(video.plays, 1)
	carousel.stop()
	assert.ok(video.released)
	assert.equal(video.loads, 1)
	t.mock.timers.tick(120000)
	assert.equal(carousel.index.value, 0)
})

test('a remaining playable video loops and an entirely broken playlist stops without cycling', async (t) => {
	const carousel = setup(['/broken.mp4', '/remaining.mp4'])
	t.after(carousel.stop)
	carousel.onVideoError(mediaEvent(await carousel.attachVideo()))
	await nextTick()
	assert.equal(carousel.index.value, 1)
	assert.equal(carousel.loopVideo.value, true)
	carousel.onVideoError(mediaEvent(await carousel.attachVideo()))
	await nextTick()
	assert.equal(carousel.index.value, 1)
	assert.equal(carousel.currentFailed.value, true)
	carousel.changeSlide(0)
	await nextTick()
	assert.equal(carousel.currentFailed.value, false)
	assert.equal(carousel.loopVideo.value, true)
})

test('autoplay denial keeps the video available for an explicit retry', async (t) => {
	const carousel = setup(['/movie.mp4', '/image.png'])
	t.after(carousel.stop)
	const video = fakeVideo()
	video.play = async () => {
		throw new DOMException('Playback requires a click', 'NotAllowedError')
	}
	carousel.video.value = video as unknown as HTMLVideoElement
	await nextTick()
	await nextTick()
	assert.equal(carousel.paused.value, true)
	assert.equal(carousel.currentFailed.value, false)
	assert.equal(carousel.index.value, 0)
	video.play = async () => {
		video.plays++
	}
	carousel.paused.value = false
	await nextTick()
	assert.equal(video.plays, 1)
})
