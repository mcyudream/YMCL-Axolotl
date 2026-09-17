/**
 * YMCL 更新平台（ymcl-content）→ 启动器更新公告模型。
 * 未配置 YMCL 更新源时，更新页回退本地 Axolotl catalog。
 */
import type {
	AnnouncementChangeType,
	LauncherAnnouncement,
} from '@/announcements/catalog'
import { ANNOUNCEMENT_CHANGE_TYPES } from '@/announcements/catalog'
import {
	fetchYmclUpdateHistory,
	getYmclChangelogUrl,
	isYmclUpdateConfigured,
	type YmclUpdateHistoryRelease,
} from '@/helpers/ymcl-content'

function bilingual(text: string) {
	return { 'en-US': text, 'zh-CN': text } as const
}

function normalizePublishedAt(raw?: string | null): string {
	if (!raw) return ''
	const value = String(raw).trim()
	if (!value) return ''
	if (/^\d{4}-\d{2}-\d{2}/.test(value)) {
		return value.slice(0, 10)
	}
	const millis = Number(value)
	if (Number.isFinite(millis) && millis > 0) {
		try {
			return new Date(millis).toISOString().slice(0, 10)
		} catch {
			return value
		}
	}
	const parsed = Date.parse(value)
	if (Number.isFinite(parsed)) {
		return new Date(parsed).toISOString().slice(0, 10)
	}
	return value
}

export function mapYmclReleaseToAnnouncement(
	release: YmclUpdateHistoryRelease,
): LauncherAnnouncement {
	const titleText = release.title?.trim() || `YMCL ${release.version}`
	const changes: Partial<Record<AnnouncementChangeType, readonly { 'en-US': string; 'zh-CN': string }[]>> =
		{}
	for (const type of ANNOUNCEMENT_CHANGE_TYPES) {
		const items = release.changes?.[type]
		if (Array.isArray(items) && items.length) {
			changes[type] = items
				.filter((item) => typeof item === 'string' && item.trim())
				.map((item) => bilingual(item.trim()))
		}
	}
	const notes = release.notes?.trim()
	const externalUrl = getYmclChangelogUrl(release.externalUrl)
	return {
		id: release.id || `ymcl-${release.version}`,
		version: release.version,
		publishedAt: normalizePublishedAt(release.publishedAt),
		title: bilingual(titleText),
		changes,
		notes: notes ? bilingual(notes) : undefined,
		externalUrl: externalUrl ?? undefined,
	}
}

/** 拉取 YMCL 更新平台历史；未配置或失败返回 null，由调用方回退本地 catalog。 */
export async function loadYmclUpdateAnnouncements(options?: {
	channel?: 'release' | 'beta' | 'all'
	limit?: number
}): Promise<LauncherAnnouncement[] | null> {
	if (!isYmclUpdateConfigured()) return null
	const payload = await fetchYmclUpdateHistory({
		channel: options?.channel ?? 'all',
		limit: options?.limit ?? 50,
	})
	if (!payload || !Array.isArray(payload.releases)) return null
	return payload.releases
		.filter((item) => item && typeof item.version === 'string' && item.version)
		.map(mapYmclReleaseToAnnouncement)
}

export function findYmclAnnouncementByVersion(
	announcements: readonly LauncherAnnouncement[],
	version: string | null | undefined,
) {
	if (!version) return undefined
	return announcements.find((item) => item.version === version)
}

export function getUpdatePlatformLabel(): string | null {
	if (!isYmclUpdateConfigured()) return null
	return 'YMCL 更新平台（ymcl-content）'
}

/** 更新日志按钮默认外链：已配置 YMCL 时优先 YMCL，否则 Axolotl 官网。 */
export function resolveUpdateChangelogUrl(announcementExternalUrl?: string): string {
	if (announcementExternalUrl && announcementExternalUrl.trim()) {
		return announcementExternalUrl.trim()
	}
	if (isYmclUpdateConfigured()) {
		return getYmclChangelogUrl() || 'https://www.axlmc.org/'
	}
	return 'https://www.axlmc.org/'
}
