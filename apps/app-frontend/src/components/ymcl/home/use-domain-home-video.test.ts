import assert from 'node:assert/strict'
import test from 'node:test'
import { effectScope, nextTick, ref } from 'vue'

import { useDomainHomeVideo } from './use-domain-home-video.ts'

function deferred() {
	let resolve!: (value: string) => void
	let reject!: (error: Error) => void
	const promise = new Promise<string>((yes, no) => {
		resolve = yes
		reject = no
	})
	return { promise, resolve, reject }
}

test('a remote video waits for the persistent copy and plays the local URL', async (t) => {
	const scope = effectScope()
	t.after(() => scope.stop())
	const pending = deferred()
	const source = ref<string | null>('https://domain.test/background.mp4')
	const requests: string[] = []
	const playback = scope.run(() =>
		useDomainHomeVideo(source, (url) => {
			requests.push(url)
			return pending.promise
		}),
	)!
	assert.equal(playback.value, null)
	assert.deepEqual(requests, [source.value])
	pending.resolve('asset://cached.mp4')
	await nextTick()
	assert.equal(playback.value, 'asset://cached.mp4')
})

test('switching backgrounds or domains ignores late cache successes and failures', async (t) => {
	const scope = effectScope()
	t.after(() => scope.stop())
	const old = deferred()
	const next = deferred()
	const source = ref<string | null>('https://first.test/video.mp4')
	const playback = scope.run(() =>
		useDomainHomeVideo(source, (url) => (url.includes('first') ? old.promise : next.promise)),
	)!
	source.value = 'https://second.test/video.mp4'
	next.resolve('asset://second.mp4')
	await nextTick()
	old.reject(new Error('The old domain is offline'))
	await nextTick()
	assert.equal(playback.value, 'asset://second.mp4')
	source.value = null
	assert.equal(playback.value, null)
})

test('cache failures fall back to streaming without failing the carousel', async (t) => {
	const scope = effectScope()
	t.after(() => scope.stop())
	const source = ref<string | null>('https://domain.test/video.mp4')
	const playback = scope.run(() =>
		useDomainHomeVideo(source, async () => {
			throw new Error('Disk unavailable')
		}),
	)!
	await nextTick()
	assert.equal(playback.value, source.value)
})

test('local sources bypass downloads and disposal prevents late playback', async () => {
	const scope = effectScope()
	const source = ref<string | null>('asset://existing.mp4')
	const pending = deferred()
	let calls = 0
	const playback = scope.run(() =>
		useDomainHomeVideo(source, () => {
			calls++
			return pending.promise
		}),
	)!
	assert.equal(playback.value, source.value)
	assert.equal(calls, 0)
	source.value = 'https://domain.test/video.mp4'
	scope.stop()
	pending.resolve('asset://late.mp4')
	await nextTick()
	assert.equal(playback.value, null)
})
