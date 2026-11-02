import assert from 'node:assert/strict'
import { it } from 'node:test'
import { emptyHistory, MAX_HISTORY, recordHistory, redoHistory, undoHistory } from './history.ts'
import { type BannerDesign, createBannerLayer } from './types.ts'
function design(baseColor: BannerDesign['baseColor']): BannerDesign {
	return { baseColor, layers: [createBannerLayer('stripe_center', 'black')] }
}
/** Vitest's matchers do not narrow types, so a step the test needs is unwrapped here. */
function present<T>(value: T | null): T {
	if (!value) throw new Error('expected a history step')
	return value
}
it('undo walks back through every recorded state', () => {
	let history = emptyHistory()
	let current = design('white')
	history = recordHistory(history, current)
	current = design('red')
	history = recordHistory(history, current)
	current = design('blue')
	const first = present(undoHistory(history, current))
	assert.strictEqual(first.design.baseColor, 'red')
	const second = present(undoHistory(first.history, first.design))
	assert.strictEqual(second.design.baseColor, 'white')
	assert.strictEqual(undoHistory(second.history, second.design), null)
})
it('redo replays the states that were undone', () => {
	let history = emptyHistory()
	history = recordHistory(history, design('white'))
	const undone = present(undoHistory(history, design('red')))
	const redone = present(redoHistory(undone.history, undone.design))
	assert.strictEqual(redone.design.baseColor, 'red')
	assert.strictEqual(redoHistory(redone.history, redone.design), null)
})
it('a new edit drops the redo stack', () => {
	let history = emptyHistory()
	history = recordHistory(history, design('white'))
	const undone = present(undoHistory(history, design('red')))
	const edited = recordHistory(undone.history, design('blue'))
	assert.deepStrictEqual(edited.future, [])
	assert.strictEqual(redoHistory(edited, design('green')), null)
})
it('the history keeps at most MAX_HISTORY snapshots', () => {
	let history = emptyHistory()
	for (let index = 0; index < MAX_HISTORY + 10; index += 1) {
		history = recordHistory(history, design('white'))
	}
	assert.strictEqual(history.past.length, MAX_HISTORY)
	// Undoing everything still stops at the oldest kept snapshot.
	let steps = 0
	let step = undoHistory(history, design('red'))
	while (step) {
		steps += 1
		step = undoHistory(step.history, step.design)
	}
	assert.strictEqual(steps, MAX_HISTORY)
})
it('undo and redo return null when there is nothing to restore', () => {
	const history = emptyHistory()
	assert.strictEqual(undoHistory(history, design('white')), null)
	assert.strictEqual(redoHistory(history, design('white')), null)
})
