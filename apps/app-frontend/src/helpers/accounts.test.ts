import assert from 'node:assert/strict'
import test from 'node:test'

import { accountsForDomain } from './accounts.ts'

const roots = {
	first: ['https://first.example/authlib-injector', 'https://first.example/yggdrasil'],
	second: ['https://second.example/yggdrasil'],
}
const accounts = [
	{ id: 'microsoft', account_type: 'microsoft' },
	{ id: 'offline', account_type: 'offline' },
	{
		id: 'first-a',
		account_type: 'yggdrasil',
		yggdrasil: { api_root: 'https://first.example/authlib-injector/' },
	},
	{
		id: 'first-b',
		account_type: 'yggdrasil',
		yggdrasil: { api_root: 'https://first.example/yggdrasil' },
	},
	{
		id: 'second',
		account_type: 'yggdrasil',
		yggdrasil: { api_root: 'https://second.example/yggdrasil' },
	},
	{
		id: 'personal-ygg',
		account_type: 'yggdrasil',
		yggdrasil: { api_root: 'https://skins.example/yggdrasil' },
	},
]

test('switching domains scopes Minecraft profiles to that domain service', () => {
	assert.deepEqual(
		accountsForDomain(accounts, roots, 'first').map((account) => account.id),
		['first-a', 'first-b'],
	)
	assert.deepEqual(
		accountsForDomain(accounts, roots, 'second').map((account) => account.id),
		['second'],
	)
	assert.deepEqual(accountsForDomain(accounts, roots, 'unknown'), [])
	assert.equal(accounts.length, 6)
})

test('personal accounts exclude joined domain profiles and keep other account types', () => {
	assert.deepEqual(
		accountsForDomain(accounts, roots, null).map((account) => account.id),
		['microsoft', 'offline', 'personal-ygg'],
	)
})
