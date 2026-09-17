import assert from 'node:assert/strict'
import test from 'node:test'

import { normalizeHomeDashboard } from '../components/home/home-dashboard.ts'
import {
	HOME_CARD_TYPE_OPTIONS,
	mapDashboardToHomeProfile,
	mapHomeProfileToDashboard,
	MAPPABLE_HOME_CARD_TYPES,
	resolveDataCardSource,
	resolveDataSourceTitle,
	splitDataSourceId,
} from './ymcl-home.ts'

const dataSources = [
	{ id: 'mc.servers.list', providerCode: 'mc', sourceCode: 'servers.list' },
	{ id: 'mc.stats.online', providerCode: 'mc', sourceCode: 'stats.online' },
]

test('maps adapter-contributed cards onto dashboard widgets', () => {
	const config = mapHomeProfileToDashboard(
		{
			layout: 'grid',
			cards: [
				{ id: 'c1', type: 'page-shortcut', title: '服务器', params: { target: 'page:mc.servers' } },
				{
					id: 'c2',
					type: 'data-card',
					title: '在线统计',
					params: { dataSource: 'mc.stats.online', variant: 'stats' },
				},
				{ id: 'c3', type: 'announcement', title: '公告' },
			],
		},
		dataSources,
	)

	assert.ok(config)
	assert.equal(config.widgets.length, 2)
	const [shortcut, dataCard] = config.widgets
	assert.equal(shortcut.kind, 'page-shortcut')
	assert.deepEqual(shortcut.shortcut, { target: 'page:mc.servers', title: '服务器' })
	assert.equal(dataCard.kind, 'data-card')
	assert.deepEqual(dataCard.dataSource, {
		id: 'mc.stats.online',
		providerCode: 'mc',
		sourceCode: 'stats.online',
		variant: 'stats',
		title: '在线统计',
	})
})

test('unresolvable data-card bindings are dropped, ids split as fallback', () => {
	// Adapter ids are "<providerCode>.<sourceCode>"; a dot-less id cannot be
	// split and no manifest declaration exists, so the card is dropped.
	const dropped = mapHomeProfileToDashboard({
		cards: [{ id: 'x', type: 'data-card', params: { dataSource: 'nosource' } }],
	})
	assert.equal(dropped, null)

	const fallback = mapHomeProfileToDashboard({
		cards: [{ id: 'y', type: 'data-card', params: { dataSource: 'mc.servers.list' } }],
	})
	assert.ok(fallback)
	assert.deepEqual(fallback.widgets[0].dataSource, {
		id: 'mc.servers.list',
		providerCode: 'mc',
		sourceCode: 'servers.list',
		variant: 'list',
	})
})

test('new domain widgets publish back with protocol params', () => {
	const config = mapHomeProfileToDashboard(
		{ cards: [{ id: 'c1', type: 'page-shortcut', params: { target: 'page:mc.servers' } }] },
		dataSources,
	)
	assert.ok(config)
	const edited = {
		...config,
		widgets: [
			...config.widgets,
			{
				id: 'new-1',
				kind: 'data-card' as const,
				size: '2x1' as const,
				dataSource: {
					id: 'mc.stats.online',
					providerCode: 'mc',
					sourceCode: 'stats.online',
					variant: 'hero' as const,
				},
			},
		],
	}
	const { profile, skippedWidgets } = mapDashboardToHomeProfile(edited, {
		cards: [
			{ id: 'c1', type: 'page-shortcut', title: '服务器', params: { target: 'page:mc.servers' } },
			{ id: 'gone', type: 'page-shortcut', params: { target: 'page:removed' } },
		],
	})

	assert.equal(skippedWidgets, 0)
	assert.equal(profile.cards?.length, 2)
	// The edited existing card keeps its original payload; sort/size sync from the editor.
	assert.deepEqual(profile.cards?.[0], {
		id: 'c1',
		type: 'page-shortcut',
		title: '服务器',
		params: { target: 'page:mc.servers' },
		sort: 0,
		size: '1x1',
	})
	assert.deepEqual(profile.cards?.[1].params, {
		dataSource: 'mc.stats.online',
		variant: 'hero',
	})
	assert.equal(profile.cards?.[1].size, '2x1')
	// Removing the card in the editor stays removed.
	assert.ok(!profile.cards?.some((card) => card.id === 'gone'))
})

test('normalize keeps valid domain widgets and drops broken ones', () => {
	const normalized = normalizeHomeDashboard({
		version: 1,
		layout: 'grid',
		widgets: [
			{
				id: 'a',
				kind: 'page-shortcut',
				size: '1x1',
				shortcut: { target: 'native:/skin_wardrobe', title: '更衣柜' },
			},
			{ id: 'b', kind: 'page-shortcut', size: '1x1', shortcut: { target: '' } },
			{
				id: 'c',
				kind: 'data-card',
				size: '3x3',
				dataSource: { id: 'x.y', providerCode: 'x', sourceCode: 'y', variant: 'nope' },
			},
			{ id: 'd', kind: 'data-card', size: '2x1', dataSource: { id: 'x' } },
		],
	})

	assert.ok(normalized)
	assert.equal(normalized.widgets.length, 2)
	assert.deepEqual(normalized.widgets[0], {
		id: 'a',
		kind: 'page-shortcut',
		size: '1x1',
		shortcut: { target: 'native:/skin_wardrobe', title: '更衣柜' },
	})
	// Invalid variants fall back to omitted, oversize falls back to the default.
	assert.deepEqual(normalized.widgets[1].dataSource, {
		id: 'x.y',
		providerCode: 'x',
		sourceCode: 'y',
	})
	assert.equal(normalized.widgets[1].size, '2x1')
})

test('mappable card types cover built-ins and adapter contributions', () => {
	assert.deepEqual(
		[...MAPPABLE_HOME_CARD_TYPES],
		[
			...HOME_CARD_TYPE_OPTIONS.map((option) => option.value),
			'page-shortcut',
			'data-card',
		],
	)
})

test('greeting options round-trip through card params', () => {
	const config = mapHomeProfileToDashboard(
		{
			cards: [
				{
					id: 'g1',
					type: 'greeting',
					params: { greetingMode: 'text', greetingText: '欢迎来到余梦', greetingFont: 'mono' },
				},
			],
		},
		[],
	)
	assert.ok(config)
	assert.equal(config.widgets[0].kind, 'greeting')
	assert.equal(config.widgets[0].options?.greetingText, '欢迎来到余梦')

	const { profile, skippedWidgets } = mapDashboardToHomeProfile(config)
	assert.equal(skippedWidgets, 0)
	assert.equal(profile.cards?.[0].type, 'greeting')
	assert.equal(profile.cards?.[0].params?.greetingText, '欢迎来到余梦')
})

test('resolveDataCardSource prefers the manifest declaration', () => {
	assert.deepEqual(resolveDataCardSource('mc.servers.list', dataSources), {
		providerCode: 'mc',
		sourceCode: 'servers.list',
		title: null,
	})
	assert.deepEqual(resolveDataCardSource('mc.servers.list'), {
		providerCode: 'mc',
		sourceCode: 'servers.list',
		title: null,
	})
	assert.equal(resolveDataCardSource('no-separator'), null)
})

test('resolveDataCardSource migrates legacy bare-sourceCode bindings', () => {
	// 适配器路由对补齐后 id 由 "servers" 变为 "<provider>.servers"；
	// 旧卡片绑定的裸 sourceCode 按声明匹配自动迁移。
	assert.deepEqual(
		resolveDataCardSource('servers', [
			{ id: 'minecraft-server.servers', providerCode: 'minecraft-server', sourceCode: 'servers' },
		]),
		{ providerCode: 'minecraft-server', sourceCode: 'servers', title: null },
	)
	assert.equal(resolveDataCardSource('servers'), null)
})

test('resolveDataCardSource surfaces adapter-declared titles', () => {
	assert.deepEqual(
		resolveDataCardSource('minecraft-activity-proof.activities', [
			{
				id: 'minecraft-activity-proof.activities',
				provider_code: 'minecraft-activity-proof',
				source_code: 'activities',
				title: '活动',
			},
		]),
		{
			providerCode: 'minecraft-activity-proof',
			sourceCode: 'activities',
			title: '活动',
		},
	)
})

test('mapHomeProfileToDashboard prefers declaration title over raw-id card title', () => {
	const config = mapHomeProfileToDashboard(
		{
			cards: [
				{
					id: 'c1',
					type: 'data-card',
					title: 'minecraft-server.servers',
					params: { dataSource: 'minecraft-server.servers' },
				},
			],
		},
		[
			{
				id: 'minecraft-server.servers',
				provider_code: 'minecraft-server',
				source_code: 'servers',
				title: '服务器',
			},
		],
	)
	assert.ok(config)
	assert.equal(config.widgets[0].dataSource?.title, '服务器')
})

test('resolveDataSourceTitle only uses adapter-provided labels', () => {
	assert.equal(
		resolveDataSourceTitle({
			id: 'minecraft-server.servers',
			cardTitle: null,
			declarationTitle: '服务器',
			pageTitle: '备用',
			navigationTitle: null,
		}),
		'服务器',
	)
	assert.equal(
		resolveDataSourceTitle({
			id: 'minecraft-activity-proof.activities',
			pageTitle: '活动中心',
		}),
		'活动中心',
	)
	// A stale card title equal to the raw id must not mask page/declaration titles.
	assert.equal(
		resolveDataSourceTitle({
			id: 'minecraft-server.servers',
			cardTitle: 'minecraft-server.servers',
			pageTitle: '服务器',
		}),
		'服务器',
	)
})

test('resolveDataSourceTitle falls back to well-known sourceCode labels', () => {
	assert.equal(
		resolveDataSourceTitle({ id: 'minecraft-activity-proof.activities' }),
		'活动',
	)
	assert.equal(resolveDataSourceTitle({ id: 'project-progress.my-tasks' }), '我的任务')
	assert.equal(
		resolveDataSourceTitle({ id: 'project-progress.claimable-tasks' }),
		'可认领任务',
	)
})

test('resolveDataSourceTitle humanizes unknown source codes', () => {
	assert.equal(resolveDataSourceTitle({ id: 'example-plugin.season-leaderboard' }), 'Season Leaderboard')
	assert.equal(resolveDataSourceTitle({ id: 'mc.stats.online' }), 'Stats Online')
})

test('splitDataSourceId keeps dotted source codes intact', () => {
	assert.deepEqual(splitDataSourceId('mc.servers.list'), {
		providerCode: 'mc',
		sourceCode: 'servers.list',
	})
	assert.deepEqual(splitDataSourceId('minecraft-server.servers'), {
		providerCode: 'minecraft-server',
		sourceCode: 'servers',
	})
	assert.equal(splitDataSourceId('nodot'), null)
})
