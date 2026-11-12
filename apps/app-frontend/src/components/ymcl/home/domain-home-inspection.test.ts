import assert from 'node:assert/strict'
import test from 'node:test'

import type { YmclJoinPreview, YmclUpdateCheck } from '../../../helpers/ymcl.ts'
import { inspectDomainHomeServer } from './domain-home-inspection.ts'

function deferred<T>() {
	let resolve!: (value: T) => void
	const promise = new Promise<T>((yes) => {
		resolve = yes
	})
	return { promise, resolve }
}

const preview = { available: true, installed: { instance_id: 'instance-a' } } as YmclJoinPreview
const check = { managed: true } as YmclUpdateCheck

test('a late preview after switching servers cannot start an update check for the old server', async () => {
	const pending = deferred<YmclJoinPreview>()
	let selected = 'a'
	const checked: string[] = []
	const result = inspectDomainHomeServer(
		{ serverId: 'a' },
		{
			preview: () => pending.promise,
			instances: async () => [],
			check: async (id) => {
				checked.push(id)
				return check
			},
		},
		() => selected === 'a',
	)
	selected = 'b'
	pending.resolve(preview)
	assert.equal(await result, null)
	assert.deepEqual(checked, [])
})

test('a late local update check cannot overwrite the newly selected server', async () => {
	const pending = deferred<YmclUpdateCheck>()
	let current = true
	const started = deferred<boolean>()
	const result = inspectDomainHomeServer(
		{ serverId: 'a' },
		{
			preview: async () => preview,
			instances: async () => [],
			check: () => {
				started.resolve(true)
				return pending.promise
			},
		},
		() => current,
	)
	await started.promise
	current = false
	pending.resolve(check)
	assert.equal(await result, null)
})

test('each server inspection checks only its own installed instance', async () => {
	const checked: string[] = []
	const result = await inspectDomainHomeServer(
		{ serverId: 'b', instanceId: 'instance-b' },
		{
			preview: async () => ({ available: true, installed: null }) as YmclJoinPreview,
			instances: async () => [],
			check: async (id) => {
				checked.push(id)
				return check
			},
		},
		() => true,
	)
	assert.deepEqual(checked, ['instance-b'])
	assert.equal(result?.check, check)
})
