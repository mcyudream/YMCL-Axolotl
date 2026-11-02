import assert from 'node:assert/strict'
import { describe, it } from 'node:test'
import { DYE_COLOR_DEFINITIONS, dyeColorDefinition } from './colors.ts'
import { BANNER_FACE_HEIGHT, BANNER_FACE_WIDTH, patternMask } from './masks.ts'
import { BANNER_PATTERNS, type BannerPatternId } from './patterns.ts'
import { renderBannerPixels } from './render.ts'
import { type BannerDesign, createBannerLayer } from './types.ts'
function pixelAt(pixels: Uint8ClampedArray, x: number, y: number) {
	const offset = (y * BANNER_FACE_WIDTH + x) * 4
	return [pixels[offset], pixels[offset + 1], pixels[offset + 2], pixels[offset + 3]]
}
/** Vitest's matchers do not narrow types, so a mask a test needs is unwrapped here. */
function maskOf(id: BannerPatternId): Uint8Array {
	const mask = patternMask(id)
	if (!mask) throw new Error(`missing mask for ${id}`)
	return mask
}
describe('banner rendering', () => {
	it('fills the face with the base color when there are no layers', () => {
		const pixels = renderBannerPixels({ baseColor: 'red', layers: [] })
		assert.strictEqual(pixels.length, BANNER_FACE_WIDTH * BANNER_FACE_HEIGHT * 4)
		assert.deepStrictEqual(pixelAt(pixels, 0, 0), [176, 46, 38, 255])
		assert.deepStrictEqual(
			pixelAt(pixels, BANNER_FACE_WIDTH - 1, BANNER_FACE_HEIGHT - 1),
			[176, 46, 38, 255],
		)
	})
	it('paints a pattern over the base color with the layer color', () => {
		const design: BannerDesign = {
			baseColor: 'white',
			layers: [createBannerLayer('stripe_top', 'black')],
		}
		const pixels = renderBannerPixels(design)
		assert.deepStrictEqual(pixelAt(pixels, 10, 2), [29, 29, 33, 255])
		assert.deepStrictEqual(pixelAt(pixels, 10, 30), [249, 255, 254, 255])
	})
	it('lets a later layer cover an earlier one', () => {
		const design: BannerDesign = {
			baseColor: 'white',
			layers: [createBannerLayer('stripe_top', 'black'), createBannerLayer('base', 'red')],
		}
		const pixels = renderBannerPixels(design)
		assert.deepStrictEqual(pixelAt(pixels, 10, 2), [176, 46, 38, 255])
	})
	it('has a mask for every pattern except the solid field', () => {
		for (const pattern of BANNER_PATTERNS) {
			if (pattern.id === 'base') {
				assert.strictEqual(patternMask(pattern.id), null)
				continue
			}
			const mask = maskOf(pattern.id)
			assert.strictEqual(mask.length, BANNER_FACE_WIDTH * BANNER_FACE_HEIGHT)
			assert.ok(
				mask.some((alpha) => alpha > 0),
				`${pattern.id} mask is empty`,
			)
		}
	})
	it('crops the front face without the banner side columns', () => {
		assert.strictEqual(BANNER_FACE_WIDTH, 20)
		assert.strictEqual(BANNER_FACE_HEIGHT, 40)
		const border = maskOf('border')
		for (let x = 0; x < BANNER_FACE_WIDTH; x++) {
			assert.ok(border[x] > 0, `top edge at ${x} is unpainted`)
			assert.ok(
				border[(BANNER_FACE_HEIGHT - 1) * BANNER_FACE_WIDTH + x] > 0,
				`bottom edge at ${x} is unpainted`,
			)
		}
		for (let y = 0; y < BANNER_FACE_HEIGHT; y++) {
			assert.ok(border[y * BANNER_FACE_WIDTH] > 0, `left edge at ${y} is unpainted`)
			assert.ok(
				border[y * BANNER_FACE_WIDTH + BANNER_FACE_WIDTH - 1] > 0,
				`right edge at ${y} is unpainted`,
			)
		}
	})
	it('centres a symmetric pattern in the face', () => {
		const mask = maskOf('stripe_center')
		const painted: number[] = []
		for (let x = 0; x < BANNER_FACE_WIDTH; x++) {
			if (mask[x] > 0) painted.push(x)
		}
		assert.ok(painted.length > 0)
		assert.strictEqual(painted[0], BANNER_FACE_WIDTH - 1 - painted[painted.length - 1])
	})
	it('defines legacy indexes that match the dye order', () => {
		DYE_COLOR_DEFINITIONS.forEach((definition, index) => {
			assert.strictEqual(definition.legacyIndex, index)
			assert.strictEqual(
				dyeColorDefinition(definition.id).bannerItem,
				`minecraft:${definition.id}_banner`,
			)
		})
	})
})
