import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

import { COMMAND_GUIDES } from './command-guides.ts'
import { CATEGORY_KEYS, PERMISSION_KEYS, PERMISSION_DESC_KEYS } from './commands/registry.ts'
import { loadCommandsForVersion, loadManifest } from './loader.ts'
import { commandMessage } from './messages.ts'

test('all versioned command labels, arguments, presets and guides have localized text', () => {
	const keys = new Set<string>()
	function collect(value: unknown): void {
		if (typeof value === 'string' && value.startsWith('app.lab.command-generator.')) keys.add(value)
		else if (value && typeof value === 'object') Object.values(value).forEach(collect)
	}
	for (const version of loadManifest().versions) collect(loadCommandsForVersion(version.id))
	collect([COMMAND_GUIDES, CATEGORY_KEYS, PERMISSION_KEYS, PERMISSION_DESC_KEYS])
	const locales = ['en-US', 'zh-CN'].map((locale) =>
		JSON.parse(
			readFileSync(new URL(`../../locales/${locale}/index.json`, import.meta.url), 'utf8'),
		),
	)
	for (const key of keys) {
		for (const locale of locales) assert.ok(locale[key]?.message, key)
		const descriptor = commandMessage(key)
		assert.equal(descriptor.id, key)
		assert.notEqual(descriptor.defaultMessage, key, key)
	}
})
