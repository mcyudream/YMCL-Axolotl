import assert from 'node:assert/strict'
import test from 'node:test'

import {
	canTurnDomainHomePage,
	createDomainHomeWheelPager,
	DOMAIN_HOME_PAGE_TRANSITION_MS,
} from './domain-home-paging.ts'

function wheel(timestamp: number, overrides = {}) {
	return {
		deltaX: 0,
		deltaY: 60,
		deltaMode: 0,
		timestamp,
		viewportHeight: 700,
		canTurn: true,
		...overrides,
	}
}

test('pages turn only at their matching content boundary', () => {
	const scroll = { scrollTop: 0, scrollHeight: 1400, clientHeight: 700 }
	assert.equal(canTurnDomainHomePage(0, 60, scroll), false)
	assert.equal(canTurnDomainHomePage(0, 60, { ...scroll, scrollTop: 700 }), true)
	assert.equal(canTurnDomainHomePage(0, -60, { ...scroll, scrollTop: 700 }), false)
	assert.equal(canTurnDomainHomePage(1, -60, scroll), true)
	assert.equal(canTurnDomainHomePage(1, -60, { ...scroll, scrollTop: 300 }), false)
	assert.equal(canTurnDomainHomePage(1, 60, scroll), false)
})

test('one touchpad gesture turns once, including long inertia and direction noise', () => {
	const pager = createDomainHomeWheelPager()
	assert.deepEqual(pager.consume(wheel(0)), { turn: 1, preventDefault: true })
	for (let time = 100; time <= 1400; time += 100) {
		assert.deepEqual(pager.consume(wheel(time, { deltaY: time % 200 ? 10 : -10 })), {
			turn: 0,
			preventDefault: true,
		})
	}
	assert.deepEqual(pager.consume(wheel(1800, { deltaY: -60 })), {
		turn: -1,
		preventDefault: true,
	})
})

test('small touchpad deltas accumulate but horizontal movement never turns pages', () => {
	const pager = createDomainHomeWheelPager()
	assert.deepEqual(pager.consume(wheel(0, { deltaX: 100 })), {
		turn: 0,
		preventDefault: false,
	})
	for (let index = 0; index < 3; index++) {
		assert.equal(pager.consume(wheel(index * 20, { deltaY: 12 })).turn, 0)
	}
	assert.equal(pager.consume(wheel(60, { deltaY: 12 })).turn, 1)
})

test('a gesture that scrolls content cannot flip a page when it reaches the edge', () => {
	const pager = createDomainHomeWheelPager()
	assert.deepEqual(pager.consume(wheel(0, { canTurn: false })), {
		turn: 0,
		preventDefault: false,
	})
	assert.deepEqual(pager.consume(wheel(100)), { turn: 0, preventDefault: false })
	assert.equal(pager.consume(wheel(500)).turn, 1)
})

test('line and page wheel units behave like pixel input', () => {
	for (const input of [
		{ deltaMode: 1, deltaY: 3 },
		{ deltaMode: 2, deltaY: 1 },
	]) {
		assert.equal(createDomainHomeWheelPager().consume(wheel(0, input)).turn, 1)
	}
})

test('click navigation locks wheel inertia until the transition has ended', () => {
	const pager = createDomainHomeWheelPager()
	pager.lock(0)
	assert.deepEqual(pager.consume(wheel(300, { deltaY: -60 })), {
		turn: 0,
		preventDefault: true,
	})
	assert.deepEqual(pager.consume(wheel(500, { deltaY: -60 })), {
		turn: 0,
		preventDefault: true,
	})
	assert.equal(pager.consume(wheel(1000, { deltaY: -60 })).turn, -1)
})

test('the wheel lock covers the full page animation and its continuing gesture', () => {
	const pager = createDomainHomeWheelPager()
	pager.lock(0)
	for (const timestamp of [
		DOMAIN_HOME_PAGE_TRANSITION_MS - 20,
		DOMAIN_HOME_PAGE_TRANSITION_MS + 100,
	]) {
		assert.deepEqual(pager.consume(wheel(timestamp, { deltaY: -60 })), {
			turn: 0,
			preventDefault: true,
		})
	}
	assert.equal(pager.consume(wheel(DOMAIN_HOME_PAGE_TRANSITION_MS + 500, { deltaY: -60 })).turn, -1)
})
