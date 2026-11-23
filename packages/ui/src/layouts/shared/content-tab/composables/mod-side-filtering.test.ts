import assert from 'node:assert/strict'
import test from 'node:test'

import type { ContentItem } from '../types.ts'
import { canMarkContentModSide, getContentEnvironmentFilterValue } from './content-filtering.ts'

test('side editing is available only for installed mod files', () => {
	const item = { project_type: 'mod', file_path: 'mods/example.jar' } as ContentItem
	assert.equal(canMarkContentModSide(item), true)
	assert.equal(canMarkContentModSide({ ...item, instanceMaterializationState: 'missing' }), false)
	assert.equal(
		canMarkContentModSide({
			...item,
			pendingManualDownload: true,
			instanceMaterializationState: 'pending_manual',
		}),
		false,
	)
	assert.equal(canMarkContentModSide({ ...item, installing: true }), false)
	assert.equal(canMarkContentModSide({ ...item, project_type: 'datapack' }), false)
})

test('manual side overrides provider environment in the existing environment filter', () => {
	assert.equal(
		getContentEnvironmentFilterValue({
			environment: 'client_only',
			mod_side: { side: 'server', server_support: 'required', source: 'instance' },
		}),
		'server',
	)
	assert.equal(
		getContentEnvironmentFilterValue({
			environment: 'client_and_server',
			mod_side: { side: 'unknown', server_support: 'unknown', source: 'local' },
		}),
		undefined,
	)
})

test('supports offline JAR detection and keeps server-optional mods in both environments', () => {
	assert.equal(
		getContentEnvironmentFilterValue({
			mod_side: { side: 'both', server_support: 'optional', source: 'modrinth' },
		}),
		'client_and_server',
	)
	assert.equal(
		getContentEnvironmentFilterValue({
			mod_side: { side: 'client', server_support: 'unsupported', source: 'embedded' },
		}),
		'client',
	)
	assert.equal(
		getContentEnvironmentFilterValue({ environment: 'singleplayer_only' }),
		'singleplayer',
	)
	assert.equal(getContentEnvironmentFilterValue({}), undefined)
})
