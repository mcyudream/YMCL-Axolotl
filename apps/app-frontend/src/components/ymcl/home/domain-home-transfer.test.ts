import assert from 'node:assert/strict'
import test from 'node:test'

import type { InstallJobSnapshot } from '../../../helpers/install.ts'
import { domainHomeTransferProgress } from './domain-home-transfer.ts'

test('an unrelated download cannot become the home operation progress', () => {
	const operation = { stage: 'downloading' as const, packId: 'current' }
	const bars = [
		{ bar_type: { type: 'pack_file_download', pack_name: 'other' }, current: 90, total: 100 },
		{
			bar_type: { type: 'pack_file_download', pack_name: 'current', instance_id: 'other-instance' },
			current: 80,
			total: 100,
		},
	]
	assert.equal(domainHomeTransferProgress(operation, [], bars).fraction, null)
	bars.push({
		bar_type: { type: 'pack_file_download', pack_name: 'current' },
		current: 25,
		total: 100,
	})
	assert.equal(domainHomeTransferProgress(operation, [], bars).fraction, 0.25)
	bars.push({
		bar_type: { type: 'pack_file_download', pack_name: 'current' },
		current: 75,
		total: 100,
	})
	assert.equal(domainHomeTransferProgress(operation, [], bars).fraction, null)
})

test('updates match the exact local instance and ignore completed installation progress', () => {
	const operation = {
		stage: 'updating' as const,
		packId: 'current',
		instanceId: 'chosen',
		jobId: 'old-job',
	}
	const bars = [
		{
			bar_type: { type: 'pack_file_download', pack_name: 'current', instance_id: 'other' },
			current: 90,
			total: 100,
		},
		{
			bar_type: { type: 'pack_file_download', pack_name: 'current', instance_id: 'chosen' },
			current: 20,
			total: 100,
		},
	]
	assert.equal(domainHomeTransferProgress(operation, [], bars).fraction, 0.2)
	assert.equal(
		domainHomeTransferProgress({ ...operation, stage: 'finalizing' }, [], bars).fraction,
		null,
	)
})

test('installation progress uses the exact job and its byte progress when present', () => {
	const job = {
		job_id: 'chosen',
		phase: 'downloading_content',
		progress: { current: 1, total: 2, secondary: { current: 20, total: 100 } },
	} as InstallJobSnapshot
	const result = domainHomeTransferProgress(
		{ stage: 'installing', packId: 'current', jobId: 'chosen' },
		[{ ...job, job_id: 'other', progress: { current: 1, total: 1 } }, job],
		[],
	)
	assert.equal(result.job, job)
	assert.equal(result.fraction, 0.2)
	assert.equal(
		domainHomeTransferProgress(
			{ stage: 'installing', packId: 'current', jobId: 'missing' },
			[job],
			[],
		).fraction,
		null,
	)
})

test('unknown totals remain indeterminate and invalid progress never produces NaN', () => {
	const operation = { stage: 'downloading' as const, packId: 'current' }
	for (const [current, total] of [
		[1, 0],
		[NaN, 100],
		[1, Infinity],
		[-1, 100],
	]) {
		assert.equal(
			domainHomeTransferProgress(
				operation,
				[],
				[{ bar_type: { type: 'pack_file_download', pack_name: 'current' }, current, total }],
			).fraction,
			null,
		)
	}
	assert.equal(
		domainHomeTransferProgress(
			operation,
			[],
			[
				{
					bar_type: { type: 'pack_file_download', pack_name: 'current' },
					current: 110,
					total: 100,
				},
			],
		).fraction,
		1,
	)
})
