/**
 * YMCL 通用内容分发 + 自有更新平台客户端。
 *
 * 内容与更新由 yda 插件 `ymcl-content` 提供，与 YAP 域 / ymcl-adapter 无关，
 * 也独立于 Axolotl 的 update.axlmc.org。
 *
 * URL base 约定（均可放 env，便于修改）：
 * - VITE_YMCL_CONTENT_URL：站点 origin 或完整插件 v1 前缀
 * - VITE_YMCL_UPDATE_URL：可选，更新 API 独立 base（origin 或 .../v1/update）
 */
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'

export type YmclContentKind = 'guides' | 'updates' | 'online'

export type YmclContentRecord = {
	id: string
	kind: string
	title: string
	summary?: string | null
	body?: string | null
	coverUrl?: string | null
	url?: string | null
	meta?: string | null
	sort?: number
	enabled?: boolean
	updatedAt?: string | null
}

export type YmclContentListPayload = {
	apiVersion?: number
	kind?: string
	records: YmclContentRecord[]
	total?: number
}

export type YmclContentCatalogPayload = {
	apiVersion?: number
	guides: YmclContentRecord[]
	updates: YmclContentRecord[]
	online: YmclContentRecord[]
}

export type YmclUpdateChannel = 'release' | 'beta'

export type YmclUpdateLatestPayload = {
	apiVersion?: number
	channel?: string
	version?: string | null
	publishedAt?: string | null
	forceUpdate?: boolean
	notes?: string
}

export type YmclUpdateManifestPlatform = {
	signature: string
	url: string
}

export type YmclUpdateManifest = {
	version: string
	notes?: string
	pub_date?: string
	platforms: Record<string, YmclUpdateManifestPlatform>
	published_at?: string
	force_update?: boolean
	channel?: string
}

export type YmclUpdateVersionsCatalog = {
	apiVersion?: number
	versions: Array<{
		version: string
		channel?: string
		artifacts: Array<{
			kind: string
			variant?: string | null
			platform?: string | null
			architecture?: string | null
			relative_path?: string | null
			url?: string | null
			sha256?: string | null
			size?: number
			filename?: string | null
		}>
	}>
}

export type YmclUpdateDownloadsLatest = {
	apiVersion?: number
	version: string
	channel?: string
	publishedAt?: string | null
	downloads: Array<{
		filename: string
		url: string
		kind?: string
		platform?: string
		architecture?: string
	}>
}

export type YmclUpdateHistoryRelease = {
	id: string
	version: string
	channel?: string
	title?: string
	publishedAt?: string | null
	forceUpdate?: boolean
	notes?: string | null
	changes?: Record<string, string[]>
	externalUrl?: string | null
}

export type YmclUpdateHistoryPayload = {
	apiVersion?: number
	releases: YmclUpdateHistoryRelease[]
	total?: number
}

const API_PREFIX = '/api/plugins/ymcl-content/v1'
const PLUGIN_PATH = '/plugins/ymcl-content/v1'
const UPDATE_SUFFIX = '/update'

function trimTrailingSlash(value: string) {
	return value.replace(/\/+$/, '')
}

function readEnv(raw: unknown): string | null {
	if (typeof raw !== 'string' || !raw.trim()) return null
	try {
		const url = new URL(trimTrailingSlash(raw.trim()))
		if (!['https:', 'http:'].includes(url.protocol)) return null
		return trimTrailingSlash(url.origin + url.pathname.replace(/\/+$/, ''))
	} catch {
		return null
	}
}

function isPluginV1Base(base: string) {
	return /\/api\/plugins\/ymcl-content\/v1$/i.test(base) || /\/api\/plugins\/ymcl-content\/v1\//i.test(`${base}/`)
}

function isUpdateApiBase(base: string) {
	return /\/api\/plugins\/ymcl-content\/v1\/update$/i.test(base) || /\/api\/plugins\/ymcl-content\/v1\/update\//i.test(`${base}/`)
}

/** 站点 origin 可能已带 /api（yda 网关前缀），拼接时避免 /api/api/... */
function pluginPrefixForContentBase(base: string) {
	if (/\/api$/i.test(base) || /\/api\//i.test(`${base}/`)) {
		return PLUGIN_PATH
	}
	return API_PREFIX
}

/** Configured content distribution origin, or null when content distribution is off. */
export function getYmclContentBaseUrl(): string | null {
	return readEnv(import.meta.env.VITE_YMCL_CONTENT_URL)
}

/**
 * YMCL 更新平台 API base（…/api/plugins/ymcl-content/v1/update）。
 * 优先级：VITE_YMCL_UPDATE_URL > VITE_YMCL_CONTENT_URL 推导 > null
 */
export function getYmclUpdateApiBase(): string | null {
	const explicit = readEnv(import.meta.env.VITE_YMCL_UPDATE_URL)
	if (explicit) {
		if (isUpdateApiBase(explicit)) return trimTrailingSlash(explicit)
		if (isPluginV1Base(explicit)) return `${trimTrailingSlash(explicit)}${UPDATE_SUFFIX}`
		return `${trimTrailingSlash(explicit)}${pluginPrefixForContentBase(explicit)}${UPDATE_SUFFIX}`
	}
	const content = getYmclContentBaseUrl()
	if (!content) return null
	if (isUpdateApiBase(content)) return trimTrailingSlash(content)
	if (isPluginV1Base(content)) return `${trimTrailingSlash(content)}${UPDATE_SUFFIX}`
	return `${trimTrailingSlash(content)}${pluginPrefixForContentBase(content)}${UPDATE_SUFFIX}`
}

export function isYmclContentConfigured() {
	return getYmclContentBaseUrl() !== null
}

export function isYmclUpdateConfigured() {
	return getYmclUpdateApiBase() !== null
}

/**
 * Resolve request path against the configured distribution origin.
 * Accepts either a bare origin (`https://host`) or a full plugin base
 * (`https://host/api/plugins/ymcl-content/v1`).
 */
function resolveApiUrl(path: string): string | null {
	const base = getYmclContentBaseUrl()
	if (!base) return null
	const normalizedPath = path.startsWith('/') ? path : `/${path}`
	if (isPluginV1Base(base) || isUpdateApiBase(base)) {
		// content base may already be plugin v1; strip update suffix if present
		const contentBase = isUpdateApiBase(base)
			? trimTrailingSlash(base).replace(/\/update$/i, '')
			: trimTrailingSlash(base)
		return `${contentBase}${normalizedPath}`
	}
	return `${trimTrailingSlash(base)}${pluginPrefixForContentBase(base)}${normalizedPath}`
}

/** Resolve path against YMCL update API base. path starts with `/` and is relative to update root. */
function resolveUpdateUrl(path: string): string | null {
	const base = getYmclUpdateApiBase()
	if (!base) return null
	const normalizedPath = path.startsWith('/') ? path : `/${path}`
	return `${trimTrailingSlash(base)}${normalizedPath}`
}

async function fetchJsonFromUrl<T>(url: string | null, timeoutMs = 10000): Promise<T | null> {
	if (!url) return null
	const controller = new AbortController()
	const timer = setTimeout(() => controller.abort(), timeoutMs)
	try {
		const response = await tauriFetch(url, {
			signal: controller.signal,
			headers: { Accept: 'application/json' },
		})
		if (!response.ok) return null
		const text = await response.text()
		if (!text || text.length > 4_500_000) return null
		return JSON.parse(text) as T
	} catch {
		return null
	} finally {
		clearTimeout(timer)
	}
}

async function fetchJson<T>(path: string, timeoutMs = 10000): Promise<T | null> {
	return fetchJsonFromUrl<T>(resolveApiUrl(path), timeoutMs)
}

function normalizeRecords(value: unknown): YmclContentRecord[] {
	if (!Array.isArray(value)) return []
	return value.filter((entry): entry is YmclContentRecord => {
		if (!entry || typeof entry !== 'object') return false
		const item = entry as YmclContentRecord
		return typeof item.id === 'string' && typeof item.title === 'string' && item.title.length > 0
	})
}

export async function fetchYmclContentList(kind: YmclContentKind): Promise<YmclContentRecord[]> {
	const payload = await fetchJson<YmclContentListPayload>(`/content/${kind}`)
	if (!payload) return []
	return normalizeRecords(payload.records).filter((item) => item.enabled !== false)
}

export async function fetchYmclContentCatalog(): Promise<YmclContentCatalogPayload | null> {
	const payload = await fetchJson<YmclContentCatalogPayload>('/catalog')
	if (!payload) return null
	return {
		apiVersion: payload.apiVersion,
		guides: normalizeRecords(payload.guides).filter((item) => item.enabled !== false),
		updates: normalizeRecords(payload.updates).filter((item) => item.enabled !== false),
		online: normalizeRecords(payload.online).filter((item) => item.enabled !== false),
	}
}

export async function fetchYmclGuides() {
	return fetchYmclContentList('guides')
}

export async function fetchYmclUpdates() {
	return fetchYmclContentList('updates')
}

export async function fetchYmclOnlineContent() {
	return fetchYmclContentList('online')
}

export async function fetchYmclUpdateLatest(
	channel: YmclUpdateChannel = 'release',
): Promise<YmclUpdateLatestPayload | null> {
	return fetchJsonFromUrl<YmclUpdateLatestPayload>(
		resolveUpdateUrl(`/latest?channel=${encodeURIComponent(channel)}`),
	)
}

export async function fetchYmclUpdateManifest(params: {
	channel?: YmclUpdateChannel
	platform?: string
	version?: string
}): Promise<YmclUpdateManifest | null> {
	const query = new URLSearchParams()
	if (params.channel) query.set('channel', params.channel)
	if (params.platform) query.set('platform', params.platform)
	if (params.version) query.set('version', params.version)
	const suffix = query.toString() ? `?${query.toString()}` : ''
	return fetchJsonFromUrl<YmclUpdateManifest>(resolveUpdateUrl(`/manifest${suffix}`))
}

export async function fetchYmclUpdateVersionsCatalog(): Promise<YmclUpdateVersionsCatalog | null> {
	return fetchJsonFromUrl<YmclUpdateVersionsCatalog>(resolveUpdateUrl('/versions'))
}

export async function fetchYmclUpdateDownloadsLatest(
	channel: YmclUpdateChannel = 'release',
): Promise<YmclUpdateDownloadsLatest | null> {
	return fetchJsonFromUrl<YmclUpdateDownloadsLatest>(
		resolveUpdateUrl(`/downloads/latest?channel=${encodeURIComponent(channel)}`),
	)
}

/** 更新页「版本历史」：ymcl-content 发布列表（含分类变更）。 */
export async function fetchYmclUpdateHistory(options?: {
	channel?: YmclUpdateChannel | 'all'
	limit?: number
}): Promise<YmclUpdateHistoryPayload | null> {
	const query = new URLSearchParams()
	if (options?.channel && options.channel !== 'all') {
		query.set('channel', options.channel)
	}
	if (options?.limit) {
		query.set('limit', String(options.limit))
	}
	const suffix = query.toString() ? `?${query.toString()}` : ''
	return fetchJsonFromUrl<YmclUpdateHistoryPayload>(resolveUpdateUrl(`/history${suffix}`))
}

/** yda 站点 origin（去掉插件路径与尾部 /api）。 */
export function getYmclSiteOrigin(): string | null {
	const base = getYmclContentBaseUrl()
	if (!base) return null
	return trimTrailingSlash(
		base
			.replace(/\/api\/plugins\/ymcl-content\/v1(?:\/update)?$/i, '')
			.replace(/\/plugins\/ymcl-content\/v1(?:\/update)?$/i, '')
			.replace(/\/api$/i, ''),
	)
}

/**
 * 「查看完整更新日志」外链。
 * 优先级：VITE_YMCL_CHANGELOG_URL > 发布上的 externalUrl > yda 站点 origin > null
 */
export function getYmclChangelogUrl(releaseExternalUrl?: string | null): string | null {
	const explicit = import.meta.env.VITE_YMCL_CHANGELOG_URL
	if (typeof explicit === 'string' && explicit.trim()) {
		return explicit.trim()
	}
	if (releaseExternalUrl && typeof releaseExternalUrl === 'string' && releaseExternalUrl.trim()) {
		return releaseExternalUrl.trim()
	}
	return getYmclSiteOrigin()
}
