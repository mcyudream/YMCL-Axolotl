import assert from 'node:assert/strict'
import { describe, it } from 'node:test'
import { isDyeColor } from './colors.ts'
import { presetNameMessages, presetSectionMessages } from './messages.ts'
import { isBannerPatternId, MAX_BANNER_LAYERS } from './patterns.ts'
import { BANNER_PRESETS, type BannerPresetSection, presetsInSection } from './presets.ts'
const GLYPH_SECTIONS: BannerPresetSection[] = ['uppercase', 'lowercase', 'digits', 'suits']
describe('banner presets', () => {
	it('uses unique ids', () => {
		const ids = BANNER_PRESETS.map((preset) => preset.id)
		assert.strictEqual(new Set(ids).size, ids.length)
	})
	it('stays inside the layer limit and uses known patterns and dyes', () => {
		for (const preset of BANNER_PRESETS) {
			const design = preset.design()
			assert.ok(isDyeColor(design.baseColor), `${preset.id} has an unknown base color`)
			assert.ok(
				design.layers.length <= MAX_BANNER_LAYERS,
				`${preset.id} has ${design.layers.length} layers`,
			)
			for (const layer of design.layers) {
				assert.ok(
					isBannerPatternId(layer.pattern),
					`${preset.id} has an unknown pattern ${layer.pattern}`,
				)
				assert.ok(isDyeColor(layer.color), `${preset.id} has an unknown dye ${layer.color}`)
			}
		}
	})
	it('covers the whole alphabet, the digits and the four suits', () => {
		assert.strictEqual(presetsInSection('uppercase').length, 26)
		assert.strictEqual(presetsInSection('lowercase').length, 26)
		assert.strictEqual(presetsInSection('digits').length, 10)
		assert.strictEqual(presetsInSection('suits').length, 4)
		const labels = (section: BannerPresetSection) =>
			presetsInSection(section)
				.map((preset) => preset.label)
				.join('')
		assert.strictEqual(labels('uppercase'), 'ABCDEFGHIJKLMNOPQRSTUVWXYZ')
		assert.strictEqual(labels('lowercase'), 'abcdefghijklmnopqrstuvwxyz')
		assert.strictEqual(labels('digits'), '0123456789')
	})
	it('names every preset, either with a literal glyph or a translated label', () => {
		for (const preset of BANNER_PRESETS) {
			if (preset.label !== undefined) {
				assert.strictEqual(preset.label.length, 1)
				continue
			}
			assert.ok(presetNameMessages[preset.id], `${preset.id} has no name message`)
		}
		for (const section of GLYPH_SECTIONS) {
			for (const preset of presetsInSection(section)) {
				assert.strictEqual(preset.label === undefined, preset.section === 'suits')
			}
		}
	})
	it('has a section title for every section in use', () => {
		for (const preset of BANNER_PRESETS) {
			assert.ok(presetSectionMessages[preset.section], `${preset.section} has no section title`)
		}
	})
})
