<script setup lang="ts">
import {
	ButtonStyled,
	commonMessages,
	defineMessages,
	injectPopupNotificationManager,
	NewModal,
	type PopupNotification,
	useVIntl,
} from '@modrinth/ui'
import { useModalStack } from '@modrinth/ui/src/composables/modal-stack'
import { renderString } from '@modrinth/utils'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'

import {
	announcementKey,
	isAnnouncementActive,
	parseAnnouncements,
	type RemoteAnnouncement,
	safeAnnouncementUrl,
} from '@/helpers/remote-announcements'
import { fetchYmclUpdates, isYmclContentConfigured, type YmclContentRecord } from '@/helpers/ymcl-content'

const props = defineProps<{ ready: boolean; previewOnly?: boolean }>()
const manager = injectPopupNotificationManager()
const { formatMessage } = useVIntl()
const { hasModal } = useModalStack()
const messages = defineMessages({
	view: { id: 'app.remote-announcements.view', defaultMessage: 'View announcement' },
	unread: { id: 'app.remote-announcements.unread', defaultMessage: 'Unread' },
	readAll: {
		id: 'app.remote-announcements.read-all',
		defaultMessage: 'Mark all announcements as read',
	},
	previewTitle: {
		id: 'app.remote-announcements.preview-title',
		defaultMessage: 'Announcement style preview',
	},
	previewSummary: {
		id: 'app.remote-announcements.preview-summary',
		defaultMessage:
			'This is a local preview. Select View announcement to preview the full Markdown content.',
	},
	previewContent: {
		id: 'app.remote-announcements.preview-content',
		defaultMessage:
			'## Announcement preview\n\nThis is **sample content**, not a published announcement.\n\n- Supports headings, lists, and links\n- Close buttons are always available\n\n> Previewing does not change real announcement read status.\n\n| Type | Display |\n| --- | --- |\n| Modal | Full Markdown content |\n| Popup | Summary, then full content |\n\n[Visit the website](https://www.ghs.red)',
	},
	previewAction: { id: 'app.remote-announcements.preview-action', defaultMessage: 'Visit website' },
})
const modal = ref<InstanceType<typeof NewModal>>()
const selected = ref<RemoteAnnouncement | null>(null)
const active = ref(false)
const html = computed(() => renderString(selected.value?.content ?? ''))
const stateKey = 'ymcl-content-announcements-v1'
const notices = new Map<string, PopupNotification>()
const reminded = new Set<string>()
const read = new Set<string>()
const queuedThisSession = new Set<string>()
let items: RemoteAnnouncement[] = []
let pending: RemoteAnnouncement[] = []
let cacheKey = ''
let cacheLoaded = false
let inFlight = false
let disposed = false
let lastAttempt = 0
let controller: AbortController | undefined
let interval: ReturnType<typeof setInterval> | undefined
let advanceTimer: ReturnType<typeof setTimeout> | undefined

function mapUpdateRecord(record: YmclContentRecord, now: string): RemoteAnnouncement {
	const published = record.updatedAt && Number.isFinite(Date.parse(record.updatedAt))
		? record.updatedAt
		: now
	return {
		id: record.id,
		title: record.title,
		summary: record.summary?.slice(0, 300) ?? null,
		content: record.body || record.summary || record.title,
		type: 'modal',
		priority: 'normal',
		starts_at: published,
		ends_at: null,
		published_at: published,
		action_label: record.url ? 'Visit' : null,
		action_url: record.url ? safeAnnouncementUrl(record.url) : null,
	}
}

function persist() {
	if (props.previewOnly) return
	try {
		localStorage.setItem(
			stateKey,
			JSON.stringify({
				reminded: [...reminded].slice(-1000),
				read: [...read].slice(-1000),
			}),
		)
	} catch {
		// localStorage may be unavailable (private mode / quota)
	}
}
function updateNotice(item: RemoteAnnouncement, popup: PopupNotification) {
	const unread = !read.has(announcementKey(item))
	popup.title = unread ? formatMessage(messages.unread) + ' · ' + item.title : item.title
	popup.text = item.summary || item.content.slice(0, 300)
	popup.onClick = () => show(item)
	popup.buttons = [
		{ label: formatMessage(messages.view), action: () => show(item), keepOpen: true },
	]
}
async function show(item: RemoteAnnouncement) {
	if (disposed || !props.ready || !isAnnouncementActive(item) || (hasModal.value && !active.value))
		return
	selected.value = item
	active.value = true
	const key = announcementKey(item)
	read.add(key)
	reminded.add(key)
	persist()
	const popup = notices.get(key)
	if (popup) {
		manager.collapseNotification(popup.id)
		updateNotice(item, popup)
	}
	pending = pending.filter((entry) => announcementKey(entry) !== key)
	await nextTick()
	if (!disposed) modal.value?.show()
}
function markAllRead() {
	for (const item of items) {
		read.add(announcementKey(item))
		reminded.add(announcementKey(item))
		const popup = notices.get(announcementKey(item))
		if (popup) {
			updateNotice(item, popup)
			manager.collapseNotification(popup.id)
		}
	}
	pending = []
	persist()
}
function advance() {
	if (disposed || !props.ready || hasModal.value || active.value) return
	const next = pending.shift()
	if (next) {
		void show(next)
		return
	}
	for (const item of items) {
		const key = announcementKey(item)
		const popup = notices.get(key)
		if (item.type === 'notification' && popup && !reminded.has(key) && isAnnouncementActive(item)) {
			manager.expandNotification(popup.id)
			reminded.add(key)
			persist()
			break
		}
	}
}
function closed() {
	active.value = false
	if (advanceTimer) clearTimeout(advanceTimer)
	advanceTimer = setTimeout(advance, 350)
}
function sync(next: RemoteAnnouncement[], fresh: boolean) {
	items = next.filter((item) => isAnnouncementActive(item))
	const keys = new Set(items.map(announcementKey))
	for (const [key, popup] of notices) {
		if (!keys.has(key)) {
			manager.removeNotification(popup.id)
			notices.delete(key)
		}
	}
	pending = pending.filter((item) => keys.has(announcementKey(item)))
	if (selected.value && !keys.has(announcementKey(selected.value))) modal.value?.hide()
	for (const item of [...items].reverse()) {
		const key = announcementKey(item)
		let popup = notices.get(key)
		if (!popup) {
			popup = manager.addPopupNotification({
				title: item.title,
				type: 'info',
				collapsed: true,
				autoCloseMs: 15000,
			})
			notices.set(key, popup)
		}
		updateNotice(item, popup)
	}
	if (fresh) {
		for (const item of items) {
			const key = announcementKey(item)
			if (
				item.type === 'modal' &&
				!queuedThisSession.has(key) &&
				(!read.has(key) || item.priority === 'critical')
			) {
				queuedThisSession.add(key)
				pending.push(item)
			}
		}
		advance()
	}
}
function loadCache() {
	if (cacheLoaded || !cacheKey) return
	cacheLoaded = true
	try {
		const cached = JSON.parse(localStorage.getItem(cacheKey) ?? 'null')
		if (cached && typeof cached.savedAt === 'number' && Date.now() - cached.savedAt < 86400000) {
			const parsed = parseAnnouncements(cached.items)
			if (parsed) sync(parsed, false)
		}
	} catch {
		// Ignore malformed or expired cache payloads
	}
}
async function refresh() {
	if (inFlight || disposed) return
	if (!isYmclContentConfigured()) return
	inFlight = true
	lastAttempt = Date.now()
	const abort = new AbortController()
	controller = abort
	if (!cacheKey) {
		cacheKey = stateKey + ':cache'
	}
	try {
		if (disposed || abort.signal.aborted) return
		loadCache()
		const records = await fetchYmclUpdates()
		if (disposed || abort.signal.aborted) return
		if (!records.length && !items.length) return
		const now = new Date().toISOString()
		const mapped = records.map((record) => mapUpdateRecord(record, now))
		const parsed = parseAnnouncements(mapped)
		if (!parsed || disposed) return
		sync(parsed, true)
		try {
			localStorage.setItem(cacheKey, JSON.stringify({ savedAt: Date.now(), items: parsed }))
		} catch {
			// Ignore cache write failures
		}
	} catch {
		// Network/parse failures are non-fatal; next reconnect retries
	} finally {
		inFlight = false
		controller = undefined
	}
}
async function openLink(value: unknown) {
	const url = safeAnnouncementUrl(value)
	if (!url) return
	try {
		await openUrl(url)
	} catch {
		// Opener may reject unknown schemes
	}
}
function contentClick(event: MouseEvent) {
	const link = event.target instanceof Element ? event.target.closest('a') : null
	if (!link) return
	event.preventDefault()
	event.stopPropagation()
	void openLink(link.getAttribute('href'))
}
function reconnect() {
	if (Date.now() - lastAttempt > 30000) void refresh()
}
function preview(type: RemoteAnnouncement['type'], withAction = false) {
	if (!props.previewOnly || !props.ready || hasModal.value || disposed) return
	const now = new Date().toISOString()
	const item: RemoteAnnouncement = {
		id: 'local-preview-' + type,
		title: formatMessage(messages.previewTitle),
		summary: formatMessage(messages.previewSummary),
		content: formatMessage(messages.previewContent),
		type,
		priority: 'normal',
		starts_at: now,
		ends_at: null,
		published_at: now,
		action_label: withAction ? formatMessage(messages.previewAction) : null,
		action_url: withAction ? 'https://www.ghs.red' : null,
	}
	read.clear()
	reminded.clear()
	queuedThisSession.clear()
	sync([item], true)
}
defineExpose({ preview })
onMounted(() => {
	if (props.previewOnly) return
	try {
		const saved = JSON.parse(localStorage.getItem(stateKey) ?? 'null')
		if (saved && Array.isArray(saved.reminded))
			for (const key of saved.reminded) if (typeof key === 'string') reminded.add(key)
		if (saved && Array.isArray(saved.read))
			for (const key of saved.read) if (typeof key === 'string') read.add(key)
	} catch {
		// Ignore malformed local read-state
	}
	const start = () => {
		void refresh()
		interval = setInterval(() => {
			sync(items, false)
			advance()
			if (Date.now() - lastAttempt >= 300000) void refresh()
		}, 15000)
		window.addEventListener('online', reconnect)
	}
	// Defer network polling until after first paint.
	if (typeof requestIdleCallback === 'function') {
		requestIdleCallback(start, { timeout: 2000 })
	} else {
		setTimeout(start, 500)
	}
})
watch([() => props.ready, hasModal], () => {
	if (advanceTimer) clearTimeout(advanceTimer)
	advanceTimer = setTimeout(advance, 350)
})
onUnmounted(() => {
	disposed = true
	controller?.abort()
	if (interval) clearInterval(interval)
	if (advanceTimer) clearTimeout(advanceTimer)
	window.removeEventListener('online', reconnect)
	for (const popup of notices.values()) manager.removeNotification(popup.id)
})
</script>

<template>
	<NewModal ref="modal" :header="selected?.title" :on-hide="closed" max-width="640px" scrollable>
		<div
			class="markdown-body break-words"
			@click="contentClick"
			@auxclick="contentClick"
			v-html="html"
		/>
		<template #actions>
			<div class="flex flex-wrap justify-end gap-2">
				<ButtonStyled
					><button @click="markAllRead">{{ formatMessage(messages.readAll) }}</button></ButtonStyled
				>
				<ButtonStyled v-if="selected?.action_url && selected.action_label" color="brand">
					<button @click="openLink(selected.action_url)">{{ selected.action_label }}</button>
				</ButtonStyled>
				<ButtonStyled
					><button @click="modal?.hide()">
						{{ formatMessage(commonMessages.closeButton) }}
					</button></ButtonStyled
				>
			</div>
		</template>
	</NewModal>
</template>
