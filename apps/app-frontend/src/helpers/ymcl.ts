// YMCL domain adapter protocol (YAP) helpers. One yda deployment is one
// domain; the personal domain is the launcher's built-in fallback.
import { invoke } from '@tauri-apps/api/core'

export const PERSONAL_DOMAIN_ID = 'personal'

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
}

export interface YmclAuthConfig {
	required: boolean
	methods: YmclAuthMethod[]
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

export interface YmclManifest {
	protocol_version: number
	domain?: YmclDomainIdentity | null
	navigation: YmclNavigationItem[]
	pages: YmclPageDescriptor[]
	data_sources: unknown[]
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
}

export interface YmclUpdateResult {
	applied: boolean
	staged_files: number
	deleted_files: number
	new_version: string
}

export const ymcl = {
	preLaunchCheck: (instanceId: string) =>
		invoke<YmclUpdateCheck>('plugin:ymcl|ymcl_pre_launch_check', { instanceId }),
	applyPackUpdate: (instanceId: string) =>
		invoke<YmclUpdateResult>('plugin:ymcl|ymcl_pack_apply_update', { instanceId }),
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
	oauthLogin: (domainId: string) =>
		invoke<YmclStoredSession>('plugin:ymcl|ymcl_auth_oauth_login', { domainId }),
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
}
