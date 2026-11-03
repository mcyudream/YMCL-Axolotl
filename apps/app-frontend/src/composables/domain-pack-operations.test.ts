import assert from 'node:assert/strict'
import test from 'node:test'
import { createPinia } from 'pinia'
import { computed, effectScope } from 'vue'

import { useDomainPackOperations } from './domain-pack-operations.ts'

const scope = { domainId: 'domain', userId: 'player', serverId: 'server' }

test('a remounted view observes the original task and its live job progress', () => {
	const pinia = createPinia()
	const owner = useDomainPackOperations(pinia)
	const firstView = effectScope()
	firstView.run(() => computed(() => owner.operationFor(scope)))
	const handle = owner.begin(scope, { stage: 'downloading', packId: 'pack' })!
	firstView.stop()
	owner.advance(handle, { stage: 'installing', packId: 'pack', jobId: 'existing-job' })
	const returnedView = computed(() => useDomainPackOperations(pinia).operationFor(scope))
	assert.equal(returnedView.value?.running, true)
	assert.equal(returnedView.value?.progress.jobId, 'existing-job')
	assert.equal(owner.begin(scope, { stage: 'downloading', packId: 'pack' }), null)
	owner.advance(handle, { stage: 'finalizing', packId: 'pack', instanceId: 'instance' })
	assert.equal(returnedView.value?.progress.stage, 'finalizing')
	owner.finish(handle, {
		ok: true,
		instanceId: 'instance',
		version: '1.0',
		warning: 'Some files need attention',
	})
	assert.equal(returnedView.value?.running, false)
	assert.deepEqual(returnedView.value?.result, {
		ok: true,
		instanceId: 'instance',
		version: '1.0',
		warning: 'Some files need attention',
	})
})

test('completion and failure while away remain available on return', () => {
	const pinia = createPinia()
	const operations = useDomainPackOperations(pinia)
	const handle = operations.begin(scope, { stage: 'installing', packId: '', jobId: 'vanilla-job' })!
	operations.finish(handle, { ok: false, error: 'Client download failed' })
	const restored = useDomainPackOperations(pinia).operationFor(scope)
	assert.equal(restored?.running, false)
	assert.equal(restored?.progress.jobId, 'vanilla-job')
	assert.deepEqual(restored?.result, { ok: false, error: 'Client download failed' })
	const retry = operations.begin(scope, { stage: 'installing', packId: '', jobId: 'vanilla-job' })!
	operations.finish(retry, { ok: true, instanceId: 'vanilla-instance', version: '1.12.2' })
	assert.deepEqual(useDomainPackOperations(pinia).operationFor(scope)?.result, {
		ok: true,
		instanceId: 'vanilla-instance',
		version: '1.12.2',
	})
})

test('operation identity isolates domains, accounts, anonymous visits and servers', () => {
	const operations = useDomainPackOperations(createPinia())
	operations.begin(scope, { stage: 'installing', packId: 'shared-pack', jobId: 'job' })
	for (const other of [
		{ ...scope, domainId: 'another-domain' },
		{ ...scope, userId: 'another-player' },
		{ ...scope, userId: null },
		{ ...scope, serverId: 'another-server' },
	]) {
		assert.equal(operations.operationFor(other), null)
		assert.ok(operations.begin(other, { stage: 'downloading', packId: 'shared-pack' }))
	}
	assert.equal(operations.operationFor(scope)?.progress.jobId, 'job')
})

test('late progress and completion cannot overwrite a retried task', () => {
	const operations = useDomainPackOperations(createPinia())
	const previous = operations.begin(scope, { stage: 'downloading', packId: 'old-pack' })!
	operations.finish(previous, { ok: false, error: 'Failed' })
	const next = operations.begin(scope, {
		stage: 'installing',
		packId: 'new-pack',
		jobId: 'new-job',
	})!
	operations.advance(previous, { stage: 'finalizing', packId: 'old-pack' })
	operations.finish(previous, { ok: true, instanceId: 'wrong-instance', version: 'old' })
	assert.equal(operations.operationFor(scope)?.id, next.id)
	assert.equal(operations.operationFor(scope)?.running, true)
	assert.equal(operations.operationFor(scope)?.progress.jobId, 'new-job')
	assert.equal(operations.operationFor(scope)?.result, null)
})
