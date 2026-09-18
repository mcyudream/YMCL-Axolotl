// YMCL domain adapter protocol (YAP) helpers. One yda deployment is one
// domain; the personal domain is the launcher's built-in fallback.
import { invoke } from '@tauri-apps/api/core'

export const PERSONAL_DOMAIN_ID = 'personal'

/** Adapter RBAC permission guarding chrome/home writes (YAP §6.5, admin). */
export const DOMAIN_DESIGN_PERMISSION = 'plugin:ymcl-adapter:design'

/** Adapter RBAC permission guarding pack publishing/ingest (YAP §7, admin). */
export const DOMAIN_PUBLISH_PERMISSION = 'plugin:ymcl-adapter:publish'

/** Session permission check mirroring yda RBAC: exact id, the global
 * wildcard, or a `prefix:*` subtree grant all satisfy a requirement. */
export function hasDomainPermission(
	permissions: string[] | undefined | null,
	required: string,
): boolean {
	if (!permissions || permissions.length === 0) return false
	return permissions.some((permission) => {
		if (permission === '*' || permission === required) return true
		return (
			permission.endsWith(':*') && required.startsWith(permission.slice(0, -1))
		)
	})
}

/**
 * Path yda serves its authlib-injector (Yggdrasil) API under, relative to the
 * domain origin. Domain Minecraft accounts authenticate against this root so
 * the in-game identity and the domain session share one account system.
 */
export function domainYggdrasilRoot(origin: string): string {
	return `${origin.replace(/\/+$/, '')}/api/plugins/authlib-injector`
}

/**
 * Extracts a human-readable message from a rejected Tauri command. The
 * backend serializes Theseus errors as `{ field_name, message }` objects —
 * stringifying them raw dumps JSON into the UI.
 */
/**
 * Backend sentinel (auth.rs SESSION_EXPIRED_MESSAGE) for a domain session
 * that died unrecoverably — surfaced as a readable localized hint instead of
 * raw HTTP noise, while the app simultaneously pops the domain login dialog.
 */
const SESSION_EXPIRED_SENTINEL = 'Domain session expired'

export function ymclErrorMessage(err: unknown): string {
	const message = errorMessageText(err)
	if (message.includes(SESSION_EXPIRED_SENTINEL)) {
		return '域会话已过期，请重新登录'
	}
	return message
}

function errorMessageText(err: unknown): string {
	if (err instanceof Error) return err.message
	if (typeof err === 'string') return err
	if (err && typeof err === 'object') {
		const obj = err as Record<string, unknown>
		// Tauri / Labrinth / yda envelopes: prefer human fields over String(object).
		for (const key of ['message', 'description', 'msg', 'error', 'detail', 'reason'] as const) {
			const value = obj[key]
			if (typeof value === 'string' && value.trim()) {
				return value.replace(/^Error:\s*/, '')
			}
			if (value && typeof value === 'object') {
				const nested = errorMessageText(value)
				if (nested && nested !== '[object Object]') return nested
			}
		}
		const fieldName = typeof obj.field_name === 'string' ? obj.field_name : null
		if (fieldName && typeof obj.message === 'string') {
			return `${fieldName}: ${obj.message}`
		}
		try {
			const json = JSON.stringify(err)
			if (json && json !== '{}' && json !== '[]') return json
		} catch {
			// fall through
		}
	}
	return String(err)
}

/** Safe display text for adapter-provided labels that may be i18n objects. */
export function ymclDisplayText(value: unknown): string {
	if (value == null) return ''
	if (typeof value === 'string') return value
	if (typeof value === 'number' || typeof value === 'boolean') return String(value)
	if (typeof value === 'object' && !Array.isArray(value)) {
		const obj = value as Record<string, unknown>
		for (const key of [
			'zh-CN',
			'zh',
			'cn',
			'zh-TW',
			'en-US',
			'en',
			'default',
			'text',
			'value',
			'name',
			'title',
			'label',
			'message',
		] as const) {
			const nested = obj[key]
			if (typeof nested === 'string' && nested.trim()) return nested.trim()
		}
	}
	const text = errorMessageText(value)
	return text === '[object Object]' ? '' : text
}

/** Normalize adapter/server action envelopes into a success+feedback shape. */
export function interpretYmclServerActionResponse(response: unknown): {
	ok: boolean
	toast?: string
	refresh?: boolean
} {
	if (response === true || response == null) return { ok: true }
	if (response === false) return { ok: false }
	if (typeof response === 'string') {
		const text = response.trim()
		if (!text) return { ok: true }
		const looksFailed = /失败|错误|不可|找不到|无权限|已满|已报名|不存在/.test(text)
		return { ok: !looksFailed, toast: text }
	}
	if (typeof response !== 'object') return { ok: true }
	const obj = response as Record<string, unknown>
	const code = typeof obj.code === 'number' ? obj.code : null
	const status = typeof obj.status === 'number' ? obj.status : null
	const success = obj.success
	const toast =
		ymclDisplayText(obj.toast) ||
		ymclDisplayText(obj.message) ||
		ymclDisplayText(obj.msg) ||
		ymclDisplayText(obj.error) ||
		ymclDisplayText(obj.description) ||
		undefined
	const ok =
		success === false
			? false
			: code != null && code !== 0 && code !== 200
				? false
				: status != null && status !== 0 && status !== 200
					? false
					: true
	return {
		ok,
		...(toast ? { toast } : {}),
		...(typeof obj.refresh === 'boolean' ? { refresh: obj.refresh } : {}),
	}
}

export interface YmclYggProfile {
	id: string
	name: string
}

export interface YmclYggExchange {
	credentials: MinecraftCredential
	profiles: YmclYggProfile[]
}

interface MinecraftCredential {
	account_type: 'microsoft' | 'offline' | 'yggdrasil'
	profile: { id: string; name: string }
	access_token: string
	refresh_token: string
	expires: number
	active: boolean
	yggdrasil?: { api_root: string; server_name: string; login: string }
}

export interface YmclDomainIdentity {
	name: string
	description?: string | null
	logo_url?: string | null
}

export interface YmclAuthMethod {
	type: string
	endpoint?: string | null
	authorize_url?: string | null
	token_url?: string | null
	client_id?: string | null
	scopes: string[]
	providers_url?: string | null
	/** Adapter alias of providers_url (snake_case). */
	providers_endpoint?: string | null
	title?: string | null
}

export interface YmclRegistrationConfig {
	enabled?: boolean
	endpoint?: string | null
	verification_methods_endpoint?: string | null
}

export interface YmclAuthConfig {
	required: boolean
	methods: YmclAuthMethod[]
	registration?: YmclRegistrationConfig | null
}

export interface YmclCapabilities {
	protocol_version: number
	adapter_version?: string | null
	domain?: YmclDomainIdentity | null
	capabilities: string[]
	auth?: YmclAuthConfig | null
}

export interface YmclNavigationItem {
	id: string
	type: string
	page_id?: string | null
	route?: string | null
	title?: string | null
	icon?: string | null
	sort?: number
	required_permission?: string | null
	/** Admin console toggle; false/undefined-disabled items must not render. */
	enabled?: boolean
	disabled?: boolean
	/** Nested entries (目录 → 子菜单 → 子菜单, YAP §6.5 extension). */
	children?: YmclNavigationItem[]
}

export interface YmclPageDescriptor {
	id: string
	renderer?: string | null
	title?: string | null
	data_source?: string | null
	permissions: string[]
	params?: unknown
	bundle?: {
		id: string
		version: string
		entry: string
		url: string
		sha256: string
		permissions?: string[]
	} | null
}

/**
 * Manifest-declared data source (YAP §6.6): the id is adapter-generated
 * (`<providerCode>.<sourceCode>`); the provider/source pair is what the
 * launcher passes to `ymcl_data_fetch`.
 */
/**
 * Manifest-declared data source (YAP §6.6): the id is adapter-generated
 * (`<providerCode>.<sourceCode>`); the provider/source pair is what the
 * launcher passes to `ymcl_data_fetch`. Older adapters omit the pair —
 * entries without it are not fetchable and must not be offered as widgets.
 */
export interface YmclDataSource {
	id: string
	provider_code?: string
	source_code?: string
	/** Adapter-provided human-readable labels (snake/camel both accepted). */
	title?: string
	name?: string
	label?: string
	schema_version?: number
	cache_ttl?: number
}

export interface YmclManifest {
	protocol_version: number
	domain?: YmclDomainIdentity | null
	navigation: YmclNavigationItem[]
	pages: YmclPageDescriptor[]
	data_sources: YmclDataSource[]
	actions?: unknown
	theme?: unknown
	home?: unknown
}

export interface YmclDomainSummary {
	id: string
	origin?: string | null
	display_name: string
	logo_url?: string | null
	is_personal: boolean
	is_active: boolean
	capabilities?: YmclCapabilities | null
}

export interface YmclDomainsState {
	active_domain_id: string
	domains: YmclDomainSummary[]
}

export interface YmclContextOption {
	id: string
	name: string
}

export interface YmclSessionContext {
	dept?: YmclContextOption | null
	role?: YmclContextOption | null
	available_depts: YmclContextOption[]
	available_roles: YmclContextOption[]
}

export interface YmclSessionInfo {
	user_id: string
	username: string
	nickname?: string | null
	avatar?: string | null
	permissions: string[]
	issued_via?: string | null
	context?: YmclSessionContext | null
}

export interface YmclStoredSession {
	domain_id: string
	access_token: string
	refresh_token?: string | null
	expires_at?: number | null
	session: YmclSessionInfo
	updated_at: number
}

export interface YmclExternalProvider {
	code: string
	name?: string | null
}

export interface YmclExternalFlow {
	flow_id: string
	authorize_url: string
}

export interface YmclExternalPoll {
	status: 'pending' | 'completed' | 'expired' | string
	access_token?: string | null
	refresh_token?: string | null
	expires_in?: number | null
}

export interface YmclUpdateCheck {
	managed: boolean
	pack_id?: string | null
	current_version?: string | null
	target_version?: string | null
	server_id?: string | null
	season_id?: string | null
	pending_changes?: number | null
	pending_deletions?: number | null
	/** Paths skipped during apply because every remote source failed. */
	skipped_files?: string[] | null
	/** 发布者写的目标版本更新说明（可选），更新前展示给玩家。 */
	notes?: string | null
}

export interface YmclUpdateResult {
	applied: boolean
	staged_files: number
	deleted_files: number
	new_version: string
	/** Paths whose remote object could not be fetched; local content was kept. */
	skipped_files: string[]
}

/** WF-4 first install step 1 outcome: downloaded mrpack + resolved pack identity. */
export interface YmclMrpackDownload {
	path: string
	pack_id: string
	/**
	 * Version the archive materializes: the target version itself, or the
	 * baseline found along the parent chain when the target has no mrpack
	 * (delta-only release).
	 */
	version: string
	channel?: string | null
	/**
	 * Set when the archive is a baseline: after the import and adopt, the
	 * instance still needs the incremental update to this version.
	 */
	target_version?: string | null
}

export interface YmclSkinProfile {
	id: string
	name: string
	current?: boolean
}

export interface YmclClosetSkin {
	id: string
	name?: string | null
	/** CLASSIC | SLIM | UNKNOWN (YAP §6.11). */
	variant?: string | null
	url: string
	hash?: string | null
}

export interface YmclClosetCape {
	id: string
	name?: string | null
	url: string
}

export interface YmclEquippedState {
	skin?: YmclClosetSkin | null
	cape_id?: string | null
}

export interface YmclCloset {
	profile: YmclSkinProfile
	equipped?: YmclEquippedState | null
	skins: YmclClosetSkin[]
	capes: YmclClosetCape[]
}

export interface YmclSkinUploadResult {
	skin: YmclClosetSkin
}

export interface YmclCapeUploadResult {
	cape: YmclClosetSkin
}

export interface YmclLibraryEntry {
	id: string
	hash?: string | null
	name?: string | null
	/** skin | cape */
	entry_type?: string | null
	variant?: string | null
	url: string
}

export interface YmclLibraryPage {
	page: number
	has_more: boolean
	items: YmclLibraryEntry[]
}

export interface YmclSkinDomainMatch {
	domain_id: string
	domain_name: string
	skins_enabled: boolean
}

export interface YmclPackVersionInfo {
	version: string
	channel?: string | null
	notes?: string | null
}

export const ymcl = {
	preLaunchCheck: (instanceId: string) =>
		invoke<YmclUpdateCheck>('plugin:ymcl|ymcl_pre_launch_check', { instanceId }),
	applyPackUpdate: (instanceId: string) =>
		invoke<YmclUpdateResult>('plugin:ymcl|ymcl_pack_apply_update', { instanceId }),
	listPublishVersions: (instanceId: string) =>
		invoke<YmclPackVersionInfo[]>('plugin:ymcl|ymcl_publish_list_versions', {
			instanceId,
		}),
	withdrawPublishVersion: (instanceId: string, version: string) =>
		invoke<Record<string, unknown>>('plugin:ymcl|ymcl_publish_withdraw', {
			instanceId,
			version,
		}),
	// WF-4 首装走 mrpack：下载归档 → 内置导入器建实例 → adopt MIP 状态
	downloadPackMrpack: (serverId: string, packId: string | null, selected: string[]) =>
		invoke<YmclMrpackDownload>('plugin:ymcl|ymcl_pack_mrpack_download', {
			serverId,
			packId,
			selected,
		}),
	adoptPackState: (
		instanceId: string,
		serverId: string,
		packId: string,
		version: string,
		channel: string | null,
		selected: string[],
		force = false,
	) =>
		invoke<void>('plugin:ymcl|ymcl_pack_adopt_state', {
			instanceId,
			serverId,
			packId,
			version,
			channel,
			selected,
			force,
		}),
	domainsState: () => invoke<YmclDomainsState>('plugin:ymcl|ymcl_domains_state'),
	addDomain: (origin: string) =>
		invoke<YmclDomainsState>('plugin:ymcl|ymcl_domain_add', { origin }),
	removeDomain: (id: string) => invoke<YmclDomainsState>('plugin:ymcl|ymcl_domain_remove', { id }),
	activateDomain: (id: string) =>
		invoke<YmclDomainsState>('plugin:ymcl|ymcl_domain_activate', { id }),
	manifest: () => invoke<YmclManifest | null>('plugin:ymcl|ymcl_manifest_get'),
	refreshManifest: () => invoke<YmclManifest | null>('plugin:ymcl|ymcl_manifest_refresh'),
	session: (domainId: string) =>
		invoke<YmclStoredSession | null>('plugin:ymcl|ymcl_session_get', { domainId }),
	passwordLogin: (domainId: string, username: string, password: string) =>
		invoke<YmclStoredSession>('plugin:ymcl|ymcl_auth_password_login', {
			domainId,
			username,
			password,
		}),
	register: (domainId: string, username: string, email: string, password: string, nickname?: string) =>
		invoke<void>('plugin:ymcl|ymcl_auth_register', {
			domainId,
			username,
			email,
			password,
			nickname: nickname ?? null,
		}),
	oauthLogin: (domainId: string) =>
		invoke<YmclStoredSession>('plugin:ymcl|ymcl_auth_oauth_login', { domainId }),
	oauthCancel: () => invoke<void>('plugin:ymcl|ymcl_auth_oauth_cancel'),
	yggExchange: (domainId: string, profileName?: string) =>
		invoke<YmclYggExchange>('plugin:ymcl|ymcl_ygg_exchange', {
			domainId,
			profileName: profileName ?? null,
		}),
	yggProfiles: (domainId: string) =>
		invoke<YmclYggProfile[]>('plugin:ymcl|ymcl_ygg_profiles', { domainId }),
	externalProviders: (domainId: string) =>
		invoke<YmclExternalProvider[]>('plugin:ymcl|ymcl_auth_external_providers', { domainId }),
	externalBegin: (domainId: string, provider: string) =>
		invoke<YmclExternalFlow>('plugin:ymcl|ymcl_auth_external_begin', { domainId, provider }),
	externalPoll: (domainId: string, provider: string, flowId: string) =>
		invoke<YmclExternalPoll>('plugin:ymcl|ymcl_auth_external_poll', {
			domainId,
			provider,
			flowId,
		}),
	externalFinish: (
		domainId: string,
		accessToken: string,
		refreshToken: string | null,
		expiresIn: number | null,
	) =>
		invoke<YmclStoredSession>('plugin:ymcl|ymcl_auth_external_finish', {
			domainId,
			accessToken,
			refreshToken,
			expiresIn,
		}),
	logout: (domainId: string) => invoke<void>('plugin:ymcl|ymcl_auth_logout', { domainId }),
	contextSwitch: (domainId: string, deptId: string | null, roleId: string | null) =>
		invoke<YmclSessionInfo>('plugin:ymcl|ymcl_context_switch', { domainId, deptId, roleId }),
	skinProfiles: (domainId: string) =>
		invoke<YmclSkinProfile[]>('plugin:ymcl|ymcl_skin_profiles', { domainId }),
	skinCreateProfile: (domainId: string, name: string) =>
		invoke<YmclSkinProfile>('plugin:ymcl|ymcl_skin_create_profile', { domainId, name }),
	skinCloset: (domainId: string, profileId: string) =>
		invoke<YmclCloset>('plugin:ymcl|ymcl_skin_closet', { domainId, profileId }),
	skinEquip: (domainId: string, profileId: string, skinId: string | null, capeId: string | null) =>
		invoke<YmclEquippedState>('plugin:ymcl|ymcl_skin_equip', {
			domainId,
			profileId,
			skinId,
			capeId,
		}),
	skinUpload: (
		domainId: string,
		profileId: string,
		dataUrl: string,
		model: 'classic' | 'slim',
		filename: string,
		name?: string,
	) =>
		invoke<YmclSkinUploadResult>('plugin:ymcl|ymcl_skin_upload', {
			domainId,
			profileId,
			data: dataUrl,
			model,
			filename,
			name: name ?? null,
		}),
	capeUpload: (domainId: string, profileId: string, dataUrl: string, filename: string, name?: string) =>
		invoke<YmclCapeUploadResult>('plugin:ymcl|ymcl_cape_upload', {
			domainId,
			profileId,
			data: dataUrl,
			filename,
			name: name ?? null,
		}),
	skinDelete: (domainId: string, profileId: string, skinId: string) =>
		invoke<void>('plugin:ymcl|ymcl_skin_delete', { domainId, profileId, skinId }),
	skinLibrary: (domainId: string, page: number, limit = 24) =>
		invoke<YmclLibraryPage>('plugin:ymcl|ymcl_skin_library', { domainId, page, limit }),
	skinCollect: (domainId: string, profileId: string, hash: string, name?: string) =>
		invoke<YmclSkinUploadResult>('plugin:ymcl|ymcl_skin_collect', {
			domainId,
			profileId,
			hash,
			name: name ?? null,
		}),
	skinTexture: (url: string) => invoke<string>('plugin:ymcl|ymcl_skin_texture', { url }),
	skinDomainForAccount: (apiRoot: string) =>
		invoke<YmclSkinDomainMatch | null>('plugin:ymcl|ymcl_skin_domain_for_account', {
			apiRoot,
		}),
}

/**
 * 基线 mrpack 安装后的增量追平（WF-5）：目标版本没有完整包时装的是 parent
 * 链上的基线版本，导入并 adopt 之后用既有更新通道把实例拉到目标版本。
 * 失败不抛出——基线实例已可用，启动前的权威更新（pre_launch_update）会
 * 自动重试；返回实例当前所处的版本（追平失败时为基线版本）。
 */
export async function catchUpPackUpdate(
	instanceId: string,
	archive: YmclMrpackDownload,
	onError?: (error: unknown) => void,
): Promise<string> {
	if (!archive.target_version || archive.target_version === archive.version) {
		return archive.version
	}
	try {
		const result = await ymcl.applyPackUpdate(instanceId)
		return result.new_version || archive.target_version
	} catch (error) {
		onError?.(error)
		return archive.version
	}
}
