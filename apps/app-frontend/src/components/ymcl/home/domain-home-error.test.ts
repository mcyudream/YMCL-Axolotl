import assert from 'node:assert/strict'
import test from 'node:test'

import { domainHomeErrorKind } from './domain-home-error.ts'

test('interrupted manifest downloads receive a short network explanation', () => {
	assert.equal(
		domainHomeErrorKind(
			'failed to read response body from https://domain.test/manifest (status=200 OK) Caused by: stream closed because of a broken pipe Location: packages/app-lib/src/util/fetch.rs:2420:44 Download failed after 4/4 attempts',
		),
		'errorNetwork',
	)
})

test('authentication and disk failures take precedence over embedded transport diagnostics', () => {
	assert.equal(
		domainHomeErrorKind('403 Forbidden from https://domain.test/manifest: network request failed'),
		'errorAuth',
	)
	assert.equal(domainHomeErrorKind('Domain session expired'), 'errorAuth')
	assert.equal(
		domainHomeErrorKind('No space left on device while reading response body'),
		'errorSpace',
	)
})

test('an arbitrary URL alone does not turn an unknown failure into a network diagnosis', () => {
	assert.equal(domainHomeErrorKind('Invalid JSON in https://domain.test/manifest'), 'errorGeneric')
	assert.equal(domainHomeErrorKind('Unknown loader version'), 'errorGeneric')
})
