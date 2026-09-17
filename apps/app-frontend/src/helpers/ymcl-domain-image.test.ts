import assert from 'node:assert/strict'
import test from 'node:test'

import { resolveDomainImageUrl } from './ymcl-domain-image.ts'

test('keeps absolute http(s), data, and blob URLs', () => {
	assert.equal(
		resolveDomainImageUrl('https://cdn.example.com/a.png', 'https://domain.example'),
		'https://cdn.example.com/a.png',
	)
	assert.equal(
		resolveDomainImageUrl('http://domain.example/bg.jpg', 'https://domain.example'),
		'http://domain.example/bg.jpg',
	)
	assert.equal(
		resolveDomainImageUrl('data:image/png;base64,abc', 'https://domain.example'),
		'data:image/png;base64,abc',
	)
	assert.equal(resolveDomainImageUrl('blob:http://x/1', null), 'blob:http://x/1')
})

test('rebases site-relative adapter paths onto the joined origin', () => {
	assert.equal(
		resolveDomainImageUrl('/static/avatar.png', 'https://admin.example.com'),
		'https://admin.example.com/static/avatar.png',
	)
	assert.equal(
		resolveDomainImageUrl('static/avatar.png', 'https://admin.example.com/'),
		'https://admin.example.com/static/avatar.png',
	)
})

test('returns null for empty values', () => {
	assert.equal(resolveDomainImageUrl(null, 'https://domain.example'), null)
	assert.equal(resolveDomainImageUrl(undefined, 'https://domain.example'), null)
	assert.equal(resolveDomainImageUrl('   ', 'https://domain.example'), null)
})

test('pins protocol-relative URLs to https', () => {
	assert.equal(
		resolveDomainImageUrl('//cdn.example.com/a.png', 'https://admin.example.com'),
		'https://cdn.example.com/a.png',
	)
})

test('leaves relative paths unchanged when origin is missing', () => {
	assert.equal(resolveDomainImageUrl('/static/a.png', null), '/static/a.png')
	assert.equal(resolveDomainImageUrl('/static/a.png', undefined), '/static/a.png')
})

