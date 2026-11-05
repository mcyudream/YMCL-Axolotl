import assert from 'node:assert/strict'
import test from 'node:test'

import {
	DOMAIN_HOME_MAX_SLIDES,
	isDomainHomeMediaUrl,
	normalizeDomainHomeLanding,
} from './domain-home.ts'

test('carousel copy modes retain both shared copy and individual slide introductions', () => {
	const value = {
		title: '统一标题',
		description: '统一介绍',
		slides: [{ image: '/one.png', title: '图片标题', description: '图片介绍' }],
	}
	assert.equal(normalizeDomainHomeLanding(value).copyMode, 'individual')
	const shared = normalizeDomainHomeLanding({ ...value, copyMode: 'shared' })
	assert.equal(shared.copyMode, 'shared')
	assert.equal(shared.slides[0].title, '图片标题')
	assert.equal(shared.slides[0].description, '图片介绍')
	assert.deepEqual(
		normalizeDomainHomeLanding({ ...shared, copyMode: 'individual' }).slides,
		shared.slides,
	)
})

test('domain homes are opt-in and tolerate missing or malformed manifests', () => {
	for (const value of [null, undefined, [], 1, 'home', { enabled: 'true', slides: {} }]) {
		const home = normalizeDomainHomeLanding(value)
		assert.equal(home.enabled, false)
		assert.deepEqual(home.slides, [])
		assert.equal(home.music.autoplay, false)
		assert.equal(home.interval, 8)
	}
})

test('mixed media preserves old image configurations and video copy through normalization', () => {
	const home = normalizeDomainHomeLanding({
		slides: [
			{ id: 'image', image: '/one.png' },
			{ id: 'video', video: '/movie.mp4', title: ' 视频 ', description: ' 介绍 ' },
			{ id: 'last', image: '/two.png' },
		],
	})
	assert.deepEqual(
		home.slides.map((slide) => [slide.id, slide.image, slide.video]),
		[
			['image', '/one.png', ''],
			['video', '', '/movie.mp4'],
			['last', '/two.png', ''],
		],
	)
	assert.equal(home.slides[1].title, '视频')
	assert.equal(home.slides[1].description, '介绍')
	assert.deepEqual(normalizeDomainHomeLanding(home), home)
	assert.deepEqual(
		normalizeDomainHomeLanding({
			slides: [{ video: 'file:///tmp/movie.mp4' }, { video: 'javascript:alert(1)' }],
		}).slides,
		[],
	)
})

test('shareable media supports remote and relative domain URLs', () => {
	for (const url of [
		'https://cdn.example/scene.webp',
		'http://localhost:9002/music.ogg',
		'/media/scene.png',
		'media/scene.png',
		'//cdn.example/audio.mp3',
		'/media/a%20b.png',
	]) {
		assert.equal(isDomainHomeMediaUrl(url), true, url)
	}
	for (const url of [
		'',
		'javascript:alert(1)',
		'data:image/png;base64,AAAA',
		'file:///tmp/a.png',
		'blob:https://example/id',
		'C:\\music.mp3',
		'https://user:password@example/audio.ogg',
		'https://example/a\nb',
		'https:example.com/a.png',
		'/media/a b.png',
		'https://',
	]) {
		assert.equal(isDomainHomeMediaUrl(url), false, url)
	}
})

test('malformed timing and audio values cannot break a home', () => {
	const home = normalizeDomainHomeLanding({
		enabled: true,
		interval: -100,
		overlay: 900,
		music: { url: 'javascript:alert(1)', volume: 500, autoplay: 'true' },
		focusSources: [' news.feed ', 'news.feed', null, 42, ''],
	})
	assert.equal(home.enabled, true)
	assert.equal(home.interval, 3)
	assert.equal(home.overlay, 85)
	assert.equal(home.music.volume, 100)
	assert.equal(home.music.url, '')
	assert.equal(home.music.autoplay, false)
	assert.deepEqual(home.focusSources, ['news.feed'])
	assert.equal(normalizeDomainHomeLanding({ interval: Infinity }).interval, 8)
	assert.equal(normalizeDomainHomeLanding({ music: { volume: NaN } }).music.volume, 35)
})

test('slides preserve order, use unique identities and cap eager image loading', () => {
	const slides = [
		{ id: 'same', image: '/a.png', title: ' A ' },
		{ id: 'same', image: '/b.png', description: ' B ' },
		{ image: 'javascript:alert(1)' },
		...Array.from({ length: 20 }, (_, index) => ({ image: `/scene-${index}.png` })),
	]
	const home = normalizeDomainHomeLanding({ slides })
	assert.equal(home.slides.length, DOMAIN_HOME_MAX_SLIDES)
	assert.equal(new Set(home.slides.map((slide) => slide.id)).size, DOMAIN_HOME_MAX_SLIDES)
	assert.equal(home.slides[0].title, 'A')
	assert.equal(home.slides[1].description, 'B')
	assert.equal(home.slides[2].image, '/scene-0.png')
	assert.equal(slides[0].title, ' A ')
})
