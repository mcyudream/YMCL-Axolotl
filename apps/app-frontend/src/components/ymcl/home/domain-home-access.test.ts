import assert from 'node:assert/strict'
import test from 'node:test'

import type { YmclStoredSession } from '../../../helpers/ymcl.ts'
import { domainHomeLayoutAccess } from './domain-home-access.ts'

function session(permissions: string[], domainId = 'domain'): YmclStoredSession {
	return {
		domain_id: domainId,
		access_token: 'test',
		session: { user_id: 'user', username: 'user', permissions },
		updated_at: 0,
	}
}

function access(overrides: Partial<Parameters<typeof domainHomeLayoutAccess>[0]> = {}) {
	return domainHomeLayoutAccess({
		domainId: 'domain',
		session: session([]),
		hasDashboard: true,
		locked: false,
		...overrides,
	})
}

test('members can customize unlocked layouts but cannot publish or lock the domain', () => {
	assert.deepEqual(access(), { canCustomize: true, canManageDomain: false })
	assert.deepEqual(access({ locked: true }), { canCustomize: false, canManageDomain: false })
})

test('design grants allow administrators to manage locked layouts', () => {
	for (const grant of ['plugin:ymcl-adapter:design', 'plugin:ymcl-adapter:*', '*']) {
		assert.deepEqual(access({ session: session([grant]), locked: true }), {
			canCustomize: false,
			canManageDomain: true,
		})
	}
	assert.equal(access({ session: session(['plugin:ymcl-adapter:publish']) }).canManageDomain, false)
})

test('anonymous and other-domain sessions cannot edit the current domain', () => {
	for (const value of [null, session(['*'], 'other-domain')]) {
		assert.deepEqual(access({ session: value }), { canCustomize: false, canManageDomain: false })
	}
})

test('personal layout editing does not grant domain management', () => {
	assert.deepEqual(access({ domainId: 'personal', session: session(['*'], 'personal') }), {
		canCustomize: true,
		canManageDomain: false,
	})
})

test('unhosted card layouts do not expose domain editing', () => {
	assert.deepEqual(access({ hasDashboard: false, session: session(['*']) }), {
		canCustomize: false,
		canManageDomain: false,
	})
})
