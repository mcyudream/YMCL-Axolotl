import assert from 'node:assert/strict'
import { describe, it } from 'node:test'
import {
	DEFAULT_SKIN_EDITOR_PREFERENCES,
	loadSkinEditorPreferences,
	normalizeExportFilename,
	normalizeSkinEditorPreferences,
	parseSkinEditorMessage,
	saveSkinEditorPreferences,
	shouldKeepRecoveryDraftAfterExport,
	shouldPromptBeforeLeave,
} from './session.ts'
function memoryStorage(initial?: string) {
	let value = initial
	return {
		getItem: () => value ?? null,
		setItem: (_key: string, next: string) => {
			value = next
		},
	}
}
describe('skin editor preferences', () => {
	it('uses defaults for missing or malformed storage', () => {
		assert.deepStrictEqual(
			loadSkinEditorPreferences(memoryStorage('{')),
			DEFAULT_SKIN_EDITOR_PREFERENCES,
		)
		assert.deepStrictEqual(
			normalizeSkinEditorPreferences({ autoSaveIntervalSeconds: Number.NaN }),
			DEFAULT_SKIN_EDITOR_PREFERENCES,
		)
	})
	it('clamps the interval and forces leave warnings when auto-save is off', () => {
		assert.deepStrictEqual(
			saveSkinEditorPreferences(
				{
					autoSaveEnabled: false,
					autoSaveIntervalSeconds: 1,
					warnBeforeLeave: false,
				},
				memoryStorage(),
			),
			{
				autoSaveEnabled: false,
				autoSaveIntervalSeconds: 5,
				warnBeforeLeave: true,
			},
		)
	})
	it('only skips the leave prompt when auto-save is on and warnings are off', () => {
		assert.strictEqual(shouldPromptBeforeLeave(DEFAULT_SKIN_EDITOR_PREFERENCES, false), false)
		assert.strictEqual(shouldPromptBeforeLeave(DEFAULT_SKIN_EDITOR_PREFERENCES, true), true)
		assert.strictEqual(
			shouldPromptBeforeLeave(
				{
					autoSaveEnabled: true,
					autoSaveIntervalSeconds: 30,
					warnBeforeLeave: false,
				},
				true,
			),
			false,
		)
		assert.strictEqual(
			shouldPromptBeforeLeave(
				{
					autoSaveEnabled: false,
					autoSaveIntervalSeconds: 30,
					warnBeforeLeave: false,
				},
				true,
			),
			true,
		)
	})
	it('keeps the recovery draft when edits happened after export started', () => {
		assert.strictEqual(shouldKeepRecoveryDraftAfterExport(2, 2), false)
		assert.strictEqual(shouldKeepRecoveryDraftAfterExport(2, 3), true)
	})
	it('always suggests a local PNG filename for export', () => {
		assert.strictEqual(normalizeExportFilename('Axolotl'), 'Axolotl.png')
		assert.strictEqual(normalizeExportFilename('Axolotl.PNG'), 'Axolotl.PNG')
		assert.strictEqual(normalizeExportFilename('   '), 'skin.png')
	})
})
describe('skin editor messages', () => {
	it('accepts recovery metadata and rejects malformed timestamps', () => {
		assert.deepStrictEqual(
			parseSkinEditorMessage({
				type: 'axolotl-skin-theme-ready',
				recovery: { updatedAt: 42, name: 'Axolotl.png' },
			}),
			{
				type: 'axolotl-skin-theme-ready',
				recovery: { updatedAt: 42, name: 'Axolotl.png' },
			},
		)
		assert.strictEqual(
			parseSkinEditorMessage({
				type: 'axolotl-skin-theme-ready',
				recovery: { updatedAt: '42' },
			}),
			null,
		)
	})
	it('validates export requests and editor state', () => {
		assert.partialDeepStrictEqual(
			parseSkinEditorMessage({
				type: 'axolotl-skin-export-request',
				requestId: 'request',
				name: 'skin.png',
				dataUrl: 'data:image/png;base64,iVBORw0KGgo=',
				reason: 'manual',
			}),
			{ type: 'axolotl-skin-export-request', requestId: 'request' },
		)
		assert.deepStrictEqual(
			parseSkinEditorMessage({
				type: 'axolotl-skin-state',
				dirty: true,
				autoSave: 'saved',
				lastSavedAt: 42,
			}),
			{
				type: 'axolotl-skin-state',
				dirty: true,
				autoSave: 'saved',
				lastSavedAt: 42,
			},
		)
		assert.strictEqual(
			parseSkinEditorMessage({
				type: 'axolotl-skin-state',
				dirty: true,
				autoSave: 'unknown',
			}),
			null,
		)
		assert.strictEqual(
			parseSkinEditorMessage({
				type: 'axolotl-skin-export-request',
				requestId: 'request',
				name: 'skin.png',
				dataUrl: 'https://example.com/skin.png',
				reason: 'manual',
			}),
			null,
		)
		assert.strictEqual(
			parseSkinEditorMessage({
				type: 'axolotl-skin-export-request',
				requestId: 'request',
				name: 'skin.png',
				dataUrl: 'data:image/png;base64,AA==',
				reason: 'manual',
			}),
			null,
		)
	})
})
