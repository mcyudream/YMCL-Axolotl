import assert from 'node:assert/strict'
import test from 'node:test'

import {
	normalizeYmclAction,
	normalizeYmclEnvelope,
	ymclRecordBody,
	ymclRecordCover,
	ymclRecordSummary,
	ymclRecordTitle,
} from './ymcl-envelope.ts'

test('normalizeYmclEnvelope reads camelCase envelopes', () => {
	const envelope = normalizeYmclEnvelope({
		records: [{ id: 'a1', title: '活动一', summary: '测试' }],
		itemActions: [
			{
				code: 'join',
				title: '参加',
				kind: 'client:launch-server',
				primary: true,
				params: { address: '{{item.address}}' },
			},
		],
		actions: [{ code: 'reload', title: '刷新', kind: 'client:reload' }],
		allow: ['client:*'],
	})
	assert.equal(envelope.records.length, 1)
	assert.equal(envelope.itemActions.length, 1)
	assert.equal(envelope.itemActions[0]?.primary, true)
	assert.equal(envelope.actions.length, 1)
	assert.deepEqual(envelope.allow, ['client:*'])
})

test('normalizeYmclEnvelope accepts snake_case and yda data envelopes', () => {
	const envelope = normalizeYmclEnvelope(
		{
			code: 0,
			data: {
				records: [{ id: 'p1', name: '项目包' }],
				item_actions: [
					{
						code: 'install',
						name: '安装',
						type: 'client:install-pack',
						is_primary: true,
					},
				],
				page_actions: [{ code: 'refresh', title: '刷新', kind: 'client:reload' }],
			},
		},
		['server:ymcl:*'],
	)
	assert.equal(envelope.records[0]?.name, '项目包')
	assert.equal(envelope.itemActions[0]?.kind, 'client:install-pack')
	assert.equal(envelope.itemActions[0]?.primary, true)
	assert.equal(envelope.actions[0]?.kind, 'client:reload')
	assert.ok(envelope.allow.includes('server:ymcl:*'))
})

test('normalizeYmclEnvelope merges envelope allow with manifest fallback', () => {
	const envelope = normalizeYmclEnvelope(
		{ records: [], item_actions: [], allow: ['client:open-url'] },
		['client:*', 'server:activity:*'],
	)
	assert.deepEqual(
		[...envelope.allow].sort(),
		['client:*', 'client:open-url', 'server:activity:*'].sort(),
	)
})

test('record helpers prefer activity/project-shaped fields', () => {
	const record = {
		name: '春节活动',
		icon: '/covers/a.png',
		summary: '限时活动',
		body: '正文第一行\n正文第二行',
		version: '1.2',
	}
	assert.equal(ymclRecordTitle(record), '春节活动')
	assert.equal(ymclRecordCover(record), '/covers/a.png')
	assert.equal(ymclRecordSummary(record), '限时活动')
	assert.equal(ymclRecordBody(record), '正文第一行\n正文第二行')
})

test('normalizeYmclAction maps dialect aliases and i18n title objects', () => {
	const action = normalizeYmclAction({
		id: 'open',
		name: { 'zh-CN': '查看详情', 'en-US': 'Open detail' },
		action_kind: 'client:open-page',
		is_primary: true,
		parameters: { pageId: 'activity.detail' },
	})
	assert.ok(action)
	assert.equal(action?.kind, 'client:open-page')
	assert.equal(action?.title, '查看详情')
	assert.equal(action?.primary, true)
	assert.deepEqual(action?.params, { pageId: 'activity.detail' })

	const signup = normalizeYmclAction({
		code: 'signup',
		title: { 'zh-CN': '报名' },
		kind: 'server:activity:signup',
		primary: true,
	})
	assert.equal(signup?.title, '报名')
	assert.equal(signup?.primary, true)
	assert.equal(signup?.kind, 'server:activity:signup')
})
