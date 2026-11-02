import assert from 'node:assert/strict'
import { describe, it } from 'node:test'
import { describeBannerMaterials } from './materials.ts'
import {
	BannerParseError,
	MAX_IMPORT_BYTES,
	parseBannerText,
	toDesignJson,
	toGiveCommand,
	toItemData,
	toPatternsComponent,
	unsupportedPatterns,
} from './serialization.ts'
import { type BannerDesign, createBannerLayer } from './types.ts'
const design: BannerDesign = {
	baseColor: 'white',
	layers: [createBannerLayer('stripe_bottom', 'red'), createBannerLayer('creeper', 'black')],
}
function codes(run: () => unknown): string {
	try {
		run()
	} catch (error) {
		if (error instanceof BannerParseError) return error.code
		throw error
	}
	return 'none'
}
describe('banner export', () => {
	it('writes the modern component syntax', () => {
		assert.strictEqual(
			toPatternsComponent(design),
			'[{pattern:"minecraft:stripe_bottom",color:"red"},{pattern:"minecraft:creeper",color:"black"}]',
		)
		assert.strictEqual(
			toGiveCommand(design),
			'/give @p minecraft:white_banner[minecraft:banner_patterns=[{pattern:"minecraft:stripe_bottom",color:"red"},{pattern:"minecraft:creeper",color:"black"}]] 1',
		)
	})
	it('leaves a single space where a plain banner has no item data', () => {
		const plain: BannerDesign = { baseColor: 'blue', layers: [] }
		assert.strictEqual(toItemData(plain), '')
		assert.strictEqual(toGiveCommand(plain), '/give @p minecraft:blue_banner 1')
		// Only the modern dialect can come out without item data; the older ones
		// always carry the block entity tag, and 1.12 keeps it after the damage value.
		assert.strictEqual(
			toGiveCommand(plain, { version: 'legacy' }),
			'/give @p minecraft:blue_banner{BlockEntityTag:{Base:11b,Patterns:[]}} 1',
		)
		assert.strictEqual(
			toGiveCommand(plain, { version: 'classic' }),
			'/give @p minecraft:banner 1 0 {BlockEntityTag:{Base:11b,Patterns:[]}}',
		)
	})
	it('writes the pre-1.20.5 item NBT with short pattern codes', () => {
		assert.strictEqual(
			toItemData(design, { version: 'legacy' }),
			'{BlockEntityTag:{Base:0b,Patterns:[{Pattern:"bs",Color:14b},{Pattern:"cre",Color:15b}]}}',
		)
		assert.strictEqual(
			toGiveCommand(design, { version: 'legacy', target: '@s', count: 2 }),
			'/give @s minecraft:white_banner{BlockEntityTag:{Base:0b,Patterns:[{Pattern:"bs",Color:14b},{Pattern:"cre",Color:15b}]}} 2',
		)
	})
	it('writes a design file that round trips', () => {
		const parsed = parseBannerText(toDesignJson(design))
		assert.strictEqual(parsed.baseColor, 'white')
		assert.deepStrictEqual(
			parsed.layers.map((layer) => [layer.pattern, layer.color]),
			[
				['stripe_bottom', 'red'],
				['creeper', 'black'],
			],
		)
	})
})
describe('banner export version support', () => {
	it('flags patterns the legacy item data cannot carry', () => {
		const newer: BannerDesign = {
			baseColor: 'white',
			layers: [createBannerLayer('flow', 'red'), createBannerLayer('stripe_top', 'blue')],
		}
		assert.deepStrictEqual(unsupportedPatterns(newer, 'legacy'), ['flow'])
		assert.deepStrictEqual(unsupportedPatterns(newer, 'modern'), [])
	})
	it('keeps the 1.13 pattern set and the 1.14 to 1.20 additions in legacy', () => {
		const older: BannerDesign = {
			baseColor: 'white',
			layers: [
				createBannerLayer('bricks', 'red'),
				createBannerLayer('piglin', 'black'),
				createBannerLayer('globe', 'blue'),
			],
		}
		assert.deepStrictEqual(unsupportedPatterns(older, 'legacy'), [])
	})
	it('reports each unsupported pattern once', () => {
		const repeated: BannerDesign = {
			baseColor: 'white',
			layers: [
				createBannerLayer('guster', 'red'),
				createBannerLayer('guster', 'blue'),
				createBannerLayer('flow', 'black'),
			],
		}
		assert.deepStrictEqual(unsupportedPatterns(repeated, 'legacy'), ['guster', 'flow'])
	})
	it('allows 1.14 to 1.20 patterns in legacy but flags the 1.20.5 ones', () => {
		const mixed: BannerDesign = {
			baseColor: 'white',
			layers: [
				createBannerLayer('stripe_top', 'red'),
				createBannerLayer('piglin', 'black'),
				createBannerLayer('globe', 'blue'),
				createBannerLayer('flow', 'white'),
			],
		}
		assert.deepStrictEqual(unsupportedPatterns(mixed, 'modern'), [])
		assert.deepStrictEqual(unsupportedPatterns(mixed, 'legacy'), ['flow'])
	})
	it('writes one short code per legacy layer', () => {
		const design: BannerDesign = {
			baseColor: 'light_gray',
			layers: [
				createBannerLayer('stripe_top', 'red'),
				createBannerLayer('diagonal_up_right', 'blue'),
			],
		}
		assert.strictEqual(
			toItemData(design, { version: 'legacy' }),
			'{BlockEntityTag:{Base:8b,Patterns:[{Pattern:"ts",Color:14b},{Pattern:"rd",Color:11b}]}}',
		)
	})
	it('writes the 1.12 command with one banner item and a damage value', () => {
		const classic: BannerDesign = {
			baseColor: 'red',
			layers: [createBannerLayer('stripe_top', 'white'), createBannerLayer('border', 'white')],
		}
		assert.strictEqual(
			toGiveCommand(classic, { version: 'classic' }),
			'/give @p minecraft:banner 1 0 {BlockEntityTag:{Base:14b,Patterns:[{Pattern:"ts",Color:0b},{Pattern:"bo",Color:0b}]}}',
		)
		// The tag itself is the same one 1.13 through 1.20.4 read.
		assert.strictEqual(
			toItemData(classic, { version: 'classic' }),
			toItemData(classic, { version: 'legacy' }),
		)
	})
	it('keeps the count before the damage value in the 1.12 command', () => {
		assert.strictEqual(
			toGiveCommand(design, { version: 'classic', target: '@s', count: 3 }),
			'/give @s minecraft:banner 3 0 {BlockEntityTag:{Base:0b,Patterns:[{Pattern:"bs",Color:14b},{Pattern:"cre",Color:15b}]}}',
		)
	})
	it('flags 1.14 and later patterns for the 1.12 target', () => {
		const newer: BannerDesign = {
			baseColor: 'white',
			layers: [
				createBannerLayer('bricks', 'red'),
				createBannerLayer('piglin', 'black'),
				createBannerLayer('globe', 'blue'),
			],
		}
		assert.deepStrictEqual(unsupportedPatterns(newer, 'classic'), ['piglin', 'globe'])
		assert.deepStrictEqual(unsupportedPatterns(newer, 'legacy'), [])
	})
})
describe('banner import', () => {
	it('reads the modern give command', () => {
		const parsed = parseBannerText(
			'/give @p minecraft:red_banner[minecraft:banner_patterns=[{pattern:"minecraft:stripe_top",color:"blue"}]] 1',
		)
		assert.strictEqual(parsed.baseColor, 'red')
		assert.deepStrictEqual(
			parsed.layers.map((layer) => [layer.pattern, layer.color]),
			[['stripe_top', 'blue']],
		)
	})
	it('reads component data without a command', () => {
		const parsed = parseBannerText(
			'[minecraft:banner_patterns=[{pattern:"minecraft:skull",color:"white"}]]',
		)
		assert.strictEqual(parsed.baseColor, 'white')
		assert.strictEqual(parsed.layers[0].pattern, 'skull')
	})
	it('reads legacy item NBT', () => {
		const parsed = parseBannerText(
			'{BlockEntityTag:{Base:14b,Patterns:[{Pattern:"curly_border",Color:0b}]}}',
		)
		assert.strictEqual(parsed.baseColor, 'red')
		assert.strictEqual(parsed.layers[0].pattern, 'curly_border')
		assert.strictEqual(parsed.layers[0].color, 'white')
	})
	it('reads a numeric base colour in a design file the way it reads layer colours', () => {
		const parsed = parseBannerText('{"baseColor":14,"layers":[{"pattern":"cross","color":0}]}')
		assert.strictEqual(parsed.baseColor, 'red')
		assert.strictEqual(parsed.layers[0].color, 'white')
		assert.strictEqual(
			codes(() => parseBannerText('{"baseColor":99,"layers":[]}')),
			'invalid-color',
		)
	})
	it('falls back to the item id when the legacy NBT has no base color', () => {
		const parsed = parseBannerText(
			'/give @p minecraft:lime_banner{BlockEntityTag:{Patterns:[{Pattern:"bricks",Color:4b}]}}',
		)
		assert.strictEqual(parsed.baseColor, 'lime')
		assert.strictEqual(parsed.layers[0].color, 'yellow')
	})
	it('reads the 1.12 give command, which keeps the data tag last', () => {
		const parsed = parseBannerText(
			'/give @p minecraft:banner 1 8 {BlockEntityTag:{Base:8,Patterns:[{Pattern:"ts",Color:14}]}}',
		)
		assert.strictEqual(parsed.baseColor, 'light_gray')
		assert.strictEqual(parsed.layers[0].pattern, 'stripe_top')
		assert.strictEqual(parsed.layers[0].color, 'red')
	})
	it('takes the base color from the damage value of a plain banner', () => {
		const parsed = parseBannerText(
			'/give @p minecraft:banner 1 14 {BlockEntityTag:{Patterns:[{Pattern:"bs",Color:0}]}}',
		)
		assert.strictEqual(parsed.baseColor, 'red')
		assert.strictEqual(parsed.layers[0].pattern, 'stripe_bottom')
	})
	it('reads the 1.12 command this exporter writes', () => {
		const parsed = parseBannerText(
			'/give @p minecraft:banner 1 0 {BlockEntityTag:{Base:14b,Patterns:[{Pattern:"ts",Color:0b},{Pattern:"bo",Color:0b}]}}',
		)
		assert.strictEqual(parsed.baseColor, 'red')
		assert.deepStrictEqual(
			parsed.layers.map((layer) => [layer.pattern, layer.color]),
			[
				['stripe_top', 'white'],
				['border', 'white'],
			],
		)
	})
	it('keeps a selector that carries a quoted space together with the item', () => {
		const parsed = parseBannerText(
			'/give @a[name="a b"] minecraft:red_banner[minecraft:banner_patterns=[{pattern:"cross",color:"white"}]] 1',
		)
		assert.strictEqual(parsed.baseColor, 'red')
		assert.deepStrictEqual(
			parsed.layers.map((layer) => [layer.pattern, layer.color]),
			[['cross', 'white']],
		)
	})
	it('rejects give commands for items that are not banners', () => {
		assert.strictEqual(
			codes(() => parseBannerText('/give @p minecraft:diamond')),
			'unsupported',
		)
		assert.strictEqual(
			codes(() => parseBannerText('/give @p minecraft:red_dye 1')),
			'unsupported',
		)
	})
	it('rejects unknown patterns and colors', () => {
		assert.strictEqual(
			codes(() => parseBannerText('')),
			'empty',
		)
		assert.strictEqual(
			codes(() => parseBannerText('hello')),
			'unsupported',
		)
		assert.strictEqual(
			codes(() => parseBannerText('[minecraft:banner_patterns=[{pattern:"nope",color:"red"}]]')),
			'invalid-pattern',
		)
		assert.strictEqual(
			codes(() => parseBannerText('[minecraft:banner_patterns=[{pattern:"cross",color:"pink2"}]]')),
			'invalid-color',
		)
		assert.strictEqual(
			codes(() => parseBannerText('{broken')),
			'invalid-data',
		)
	})
	it('rejects text far larger than a design', () => {
		const huge = `{BlockEntityTag:{Base:0b,Patterns:[${'{Pattern:"cross",Color:0b},'.repeat(20000)}]}}`
		assert.strictEqual(
			codes(() => parseBannerText(huge)),
			'too-large',
		)
	})
	it('measures the import cap in bytes rather than UTF-16 code units', () => {
		// Two bytes each once encoded, so this is under the cap as a string
		// length and over it as a payload.
		const wide = 'ä'.repeat(MAX_IMPORT_BYTES / 2 + 1)
		assert.ok(wide.length < MAX_IMPORT_BYTES)
		assert.strictEqual(
			codes(() => parseBannerText(wide)),
			'too-large',
		)
	})
	it('caps imported layers at the survival limit', () => {
		const layers = Array.from(
			{ length: 8 },
			(_, index) => `{Pattern:"cross",Color:${index}b}`,
		).join(',')
		const parsed = parseBannerText(`{BlockEntityTag:{Base:0b,Patterns:[${layers}]}}`)
		assert.strictEqual(parsed.layers.length, 6)
	})
})
describe('banner materials', () => {
	it('counts dyes, pattern items and the banner itself', () => {
		const materials = describeBannerMaterials(design)
		assert.strictEqual(materials.bannerItem, 'minecraft:white_banner')
		assert.deepStrictEqual(materials.dyes, [
			{ color: 'red', count: 1, item: 'minecraft:red_dye' },
			{ color: 'black', count: 1, item: 'minecraft:black_dye' },
		])
		assert.deepStrictEqual(materials.patternItems, ['minecraft:creeper_banner_pattern'])
	})
	it('merges repeated layer colors without counting the base color', () => {
		const materials = describeBannerMaterials({
			baseColor: 'red',
			layers: [createBannerLayer('cross', 'red'), createBannerLayer('border', 'red')],
		})
		assert.strictEqual(materials.bannerItem, 'minecraft:red_banner')
		assert.deepStrictEqual(materials.dyes, [{ color: 'red', count: 2, item: 'minecraft:red_dye' }])
		assert.deepStrictEqual(materials.patternItems, [])
	})
	it('lists each pattern item once', () => {
		const materials = describeBannerMaterials({
			baseColor: 'white',
			layers: [
				createBannerLayer('creeper', 'black'),
				createBannerLayer('creeper', 'red'),
				createBannerLayer('globe', 'blue'),
			],
		})
		assert.deepStrictEqual(materials.patternItems, [
			'minecraft:creeper_banner_pattern',
			'minecraft:globe_banner_pattern',
		])
	})
})
