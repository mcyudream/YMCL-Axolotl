import assert from 'node:assert/strict'
import test from 'node:test'
import { createPinia } from 'pinia'

import { useDomainHomeSession } from './domain-home-session.ts'

test('returning to home within a launcher session restores the selected page', () => {
	const pinia = createPinia()
	const firstVisit = useDomainHomeSession(pinia)
	assert.equal(firstVisit.pageFor('domain', 'player'), 0)
	firstVisit.rememberPage('domain', 'player', 1)
	const returnVisit = useDomainHomeSession(pinia)
	assert.equal(returnVisit.pageFor('domain', 'player'), 1)
	returnVisit.rememberPage('domain', 'player', 0)
	assert.equal(firstVisit.pageFor('domain', 'player'), 0)
})

test('page selections are isolated between domains, accounts and anonymous visits', () => {
	const session = useDomainHomeSession(createPinia())
	session.rememberPage('domain-a', 'player-a', 1)
	assert.equal(session.pageFor('domain-b', 'player-a'), 0)
	assert.equal(session.pageFor('domain-a', 'player-b'), 0)
	assert.equal(session.pageFor('domain-a', null), 0)
	session.rememberPage('domain-b', 'player-b', 1)
	assert.equal(session.pageFor('domain-a', 'player-a'), 1)
})

test('starting a new launcher session begins on the first page', () => {
	const previousRun = useDomainHomeSession(createPinia())
	previousRun.rememberPage('domain', 'player', 1)
	const nextRun = useDomainHomeSession(createPinia())
	assert.equal(nextRun.pageFor('domain', 'player'), 0)
})

test('returning home restores the selected server in its domain and account', () => {
	const pinia = createPinia()
	const firstVisit = useDomainHomeSession(pinia)
	firstVisit.rememberServer('domain', 'player', 'downloading-server')
	const returnVisit = useDomainHomeSession(pinia)
	assert.equal(returnVisit.serverFor('domain', 'player'), 'downloading-server')
	assert.equal(returnVisit.serverFor('other-domain', 'player'), '')
	assert.equal(returnVisit.serverFor('domain', 'other-player'), '')
	assert.equal(returnVisit.serverFor('domain', null), '')
	assert.equal(useDomainHomeSession(createPinia()).serverFor('domain', 'player'), '')
})
