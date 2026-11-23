import assert from 'node:assert/strict'
import test from 'node:test'

import { selectServerPackFiles, type ServerPackPreview } from './mod-sides.ts'

function file(path: string, selected: boolean): ServerPackPreview['files'][number] {
	return { path, selected, size: 1, modSide: null, requires: [], downloads: [] }
}

test('optional root files and custom folders stay unselected until chosen', () => {
	const files = [
		file('config/server.toml', true),
		file('server-icon.png', false),
		file('custom/server-rules.txt', false),
	]
	assert.deepEqual(selectServerPackFiles(files), ['config/server.toml'])
	assert.deepEqual(
		selectServerPackFiles(files, {
			files,
			selectedPaths: ['server-icon.png', 'custom/server-rules.txt'],
		}),
		['server-icon.png', 'custom/server-rules.txt'],
	)
})

test('changing full and slim modes keeps selected extras and deselected defaults', () => {
	const files = [file('mods/server.jar', true), file('custom/server-rules.txt', false)]
	const selectedPaths = ['custom/server-rules.txt']
	const slim = files.map((item) => ({
		...item,
		downloads: item.path.endsWith('.jar') ? ['https://example.test/server.jar'] : [],
	}))
	const slimSelection = selectServerPackFiles(slim, { files, selectedPaths })
	assert.deepEqual(slimSelection, selectedPaths)
	assert.deepEqual(
		selectServerPackFiles(files, { files: slim, selectedPaths: slimSelection }),
		selectedPaths,
	)
})

test('refresh removes missing paths and applies defaults only to new candidates', () => {
	const previous = [
		file('mods/old.jar', true),
		file('config/server.toml', true),
		file('custom/server-rules.txt', false),
	]
	const refreshed = [
		file('mods/new.jar', true),
		file('config/server.toml', true),
		file('custom/server-rules.txt', false),
		file('custom/new.txt', false),
	]
	assert.deepEqual(
		selectServerPackFiles(refreshed, {
			files: previous,
			selectedPaths: ['mods/old.jar', 'custom/server-rules.txt'],
		}),
		['mods/new.jar', 'custom/server-rules.txt'],
	)
})
