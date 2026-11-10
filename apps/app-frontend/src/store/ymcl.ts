import { defineStore } from 'pinia'

import type { HomeDashboardConfig } from '@/components/home/home-dashboard'
import { normalizeDomainHomeLanding } from '@/components/ymcl/home/domain-home'
import {
	PERSONAL_DOMAIN_ID,
	ymcl,
	type YmclDomainsState,
	type YmclDomainSummary,
	type YmclExternalPoll,
	type YmclManifest,
	type YmclNavigationItem,
	type YmclStoredSession,
} from '@/helpers/ymcl'
import { ymclNavTo } from '@/helpers/ymcl-domain'
import { mapHomeProfileToDashboard, type YmclHomeProfile } from '@/helpers/ymcl-home'
import { normalizeYmclManifest } from '@/helpers/ymcl-nav'
import {
	filterSimpleModeNavigation,
	isSimpleModeRouteHidden,
	normalizeSimpleMode,
	type SimpleModeEntrance,
	simpleModeHiddenRoutes,
} from '@/helpers/ymcl-simple-mode'

interface YmclStoreState {
	domains: YmclDomainSummary[]
	activeDomainId: string
	manifest: YmclManifest | null
	session: YmclStoredSession | null
	initialized: boolean
	adding: boolean
	loadingManifest: boolean
	loggingIn: boolean
	/** Bumped on data-affecting push events; page hosts watch and re-pull. */
	dataEpoch: number
	/**
	 * Bumped whenever the active domain needs a sign-in and has none (fresh
	 * activation, added domain, session loss). App.vue watches it to pop the
	 * domain login dialog.
	 */
	loginPromptAt: number
	/** Cached process-owned unlocks, restored when the WebView is recreated. */
	unlockedSimpleModeDomains: string[]
	domainEpoch: number
	activationRequestId: number
}

const EXTERNAL_POLL_INTERVAL_MS = 2000
const EXTERNAL_POLL_MAX_MS = 5 * 60 * 1000

export const useYmclStore = defineStore('ymclStore', {
	state: (): YmclStoreState => ({
		domains: [],
		activeDomainId: PERSONAL_DOMAIN_ID,
		manifest: null,
		session: null,
		initialized: false,
		adding: false,
		loadingManifest: false,
		loggingIn: false,
		dataEpoch: 0,
		loginPromptAt: 0,
		unlockedSimpleModeDomains: [],
		domainEpoch: 0,
		activationRequestId: 0,
	}),
	getters: {
		isPersonal: (state) => state.activeDomainId === PERSONAL_DOMAIN_ID,
		activeDomain: (state) =>
			state.domains.find((domain) => domain.id === state.activeDomainId) ?? null,
		/**
		 * True when the active domain demands authentication (YAP `auth.required`)
		 * but no session is established: domain content is off-limits until the
		 * sign-in dialog completes.
		 */
		loginRequired(): boolean {
			if (this.isPersonal || this.session) return false
			return this.activeDomain?.capabilities?.auth?.required === true
		},
		/** Domain-controlled presentation policy. */
		simpleMode: (state) => normalizeSimpleMode(state.manifest?.simple_mode),
		simpleModeActive(): boolean {
			return (
				!this.isPersonal &&
				this.simpleMode.enabled &&
				!this.unlockedSimpleModeDomains.includes(this.activeDomainId)
			)
		},
		simpleModeHiddenRoutes(): string[] {
			return this.simpleModeActive
				? simpleModeHiddenRoutes(
						this.simpleMode,
						this.manifest?.navigation ?? [],
						ymclNavTo,
						this.manifest?.pages ?? [],
					)
				: []
		},
		isRouteHidden(): (route: string) => boolean {
			return (route: string): boolean => isSimpleModeRouteHidden(route, this.simpleModeHiddenRoutes)
		},
		isEntranceHidden(): (entrance: SimpleModeEntrance) => boolean {
			return (entrance: SimpleModeEntrance): boolean =>
				this.simpleModeActive && this.simpleMode.hiddenEntrances.includes(entrance)
		},
		/** Manifest navigation filtered for presentation and sorted for rendering. */
		navigation(state): YmclNavigationItem[] {
			const navigation = state.manifest?.navigation ?? []
			return [
				...(this.simpleModeActive
					? filterSimpleModeNavigation(
							navigation,
							this.simpleMode,
							this.simpleModeHiddenRoutes,
							ymclNavTo,
						)
					: navigation),
			].sort((a, b) => (a.sort ?? 0) - (b.sort ?? 0))
		},
		pageById: (state) => (pageId: string) => {
			const pages = state.manifest?.pages ?? []
			return (
				pages.find((page) => page.id === pageId) ??
				pages.find((page) => page.data_source === pageId) ??
				null
			)
		},
		domainHomeLanding: (state) =>
			normalizeDomainHomeLanding((state.manifest?.home as YmclHomeProfile | undefined)?.landing),
		showDomainLanding(): boolean {
			return !this.isPersonal && !this.loginRequired && this.domainHomeLanding.enabled
		},
		/**
		 * Domain-hosted home layout (YAP §6.5 home capability) mapped onto the
		 * launcher's existing dashboard config via the shared mapping in
		 * helpers/ymcl-home. Null when the domain does not host the layout.
		 * dataSources back the adapter's data-card contributions.
		 */
		domainHomeDashboard: (state): HomeDashboardConfig | null =>
			mapHomeProfileToDashboard(
				state.manifest?.home as YmclHomeProfile | undefined,
				state.manifest?.data_sources,
			),
		domainHomeLocked: (state) => {
			const home = state.manifest?.home as { locked?: boolean } | undefined
			return home?.locked === true
		},
		/**
		 * Session context with switching options; only meaningful for
		 * multi-context users (YAP §6.4 context capability).
		 */
		sessionContext: (state) => state.session?.session.context ?? null,
	},
	actions: {
		async unlockSimpleMode() {
			if (!this.simpleModeActive) return
			const unlocked = await ymcl.unlockSimpleMode(this.activeDomainId)
			this.unlockedSimpleModeDomains = [
				...new Set([...this.unlockedSimpleModeDomains, ...unlocked]),
			]
		},
		setManifest(manifest: YmclManifest | null) {
			this.manifest = normalizeYmclManifest(manifest)
		},
		async init() {
			if (this.initialized) return
			const requestId = this.activationRequestId
			const [domainState, unlocked] = await Promise.all([
				ymcl.domainsState().catch(() => null),
				ymcl.simpleModeUnlocked(),
			])
			this.unlockedSimpleModeDomains = [
				...new Set([...this.unlockedSimpleModeDomains, ...unlocked]),
			]
			if (requestId !== this.activationRequestId) return
			if (domainState) this.applyDomainsState(domainState)
			const epoch = this.domainEpoch
			const isCurrent = () => epoch === this.domainEpoch && requestId === this.activationRequestId
			if (this.activeDomainId && this.activeDomainId !== PERSONAL_DOMAIN_ID) {
				const fresh = await ymcl.refreshManifest().catch(() => null)
				if (!isCurrent()) return
				const manifest = fresh ?? (await ymcl.manifest().catch(() => null))
				if (!isCurrent()) return
				this.setManifest(manifest)
			} else {
				const manifest = await ymcl.manifest().catch(() => null)
				if (!isCurrent()) return
				this.setManifest(manifest)
			}
			if (!this.isPersonal && this.activeDomainId) {
				const session = await ymcl.session(this.activeDomainId).catch(() => null)
				if (!isCurrent()) return
				this.session = session
			}
			this.initialized = true
			this.requireLogin(true)
		},
		applyDomainsState(domainState: YmclDomainsState) {
			if (this.activeDomainId !== domainState.active_domain_id) {
				this.domainEpoch += 1
				this.setManifest(null)
				this.session = null
			}
			this.domains = domainState.domains
			this.activeDomainId = domainState.active_domain_id
		},
		/**
		 * Marks that the sign-in dialog should pop for the active domain. Force
		 * for explicit transitions (activation, add, startup); the deduped path
		 * absorbs bursts of session-expiry pushes.
		 */
		requireLogin(force = false) {
			if (!this.loginRequired) return
			const now = Date.now()
			if (!force && now - this.loginPromptAt < 3000) return
			this.loginPromptAt = now
		},
		/** Kicks back to the personal domain when a domain was dismissed while
		 * still unauthenticated — no domain access without a session. */
		async leaveDomainIfLoginRequired() {
			if (this.loginRequired) await this.activateDomain(PERSONAL_DOMAIN_ID)
		},
		async addDomain(origin: string) {
			this.adding = true
			try {
				const known = new Set(this.domains.map((domain) => domain.id))
				this.applyDomainsState(await ymcl.addDomain(origin))
				const added = this.domains.find((domain) => !known.has(domain.id) && !domain.is_personal)
				// Switch straight into the new domain; activateDomain then pops
				// the sign-in dialog because the fresh domain has no session.
				if (added) await this.activateDomain(added.id)
			} finally {
				this.adding = false
			}
		},
		async removeDomain(id: string) {
			await ymcl.logout(id).catch(() => null)
			this.applyDomainsState(await ymcl.removeDomain(id))
			if (this.activeDomainId === id) this.session = null
		},
		async activateDomain(id: string) {
			const requestId = ++this.activationRequestId
			this.loadingManifest = true
			try {
				const domainState = await ymcl.activateDomain(id)
				if (requestId !== this.activationRequestId) return
				this.applyDomainsState(domainState)
				const epoch = this.domainEpoch
				const isCurrent = () =>
					requestId === this.activationRequestId &&
					epoch === this.domainEpoch &&
					id === this.activeDomainId
				if (!isCurrent()) return
				// Session first: authed manifest includes RBAC-filtered nav/pages.
				const session = id === PERSONAL_DOMAIN_ID ? null : await ymcl.session(id).catch(() => null)
				if (!isCurrent()) return
				this.session = session
				if (id === PERSONAL_DOMAIN_ID) {
					this.setManifest(null)
				} else {
					const fresh = await ymcl.refreshManifest().catch(() => null)
					if (!isCurrent()) return
					const manifest = fresh ?? (await ymcl.manifest().catch(() => null))
					if (!isCurrent()) return
					this.setManifest(manifest)
				}
				this.requireLogin(true)
			} finally {
				if (requestId === this.activationRequestId) this.loadingManifest = false
			}
		},
		/**
		 * Reacts to a server-pushed event (YAP §6.10). Events only locate
		 * the change; the actual data is always pulled fresh.
		 */
		async handleServerEvent(event: { type?: string }) {
			switch (event.type) {
				case 'manifest.updated':
				case 'session.context_changed': {
					const epoch = this.domainEpoch
					const domainId = this.activeDomainId
					await this.refreshManifest()
					if (domainId !== PERSONAL_DOMAIN_ID && epoch === this.domainEpoch) {
						const session = await ymcl.session(domainId).catch(() => null)
						if (epoch === this.domainEpoch) this.session = session
					}
					break
				}
				case 'session.revoked':
					this.session = null
					this.requireLogin()
					break
				case 'session.expired':
					await this.handleSessionExpired(
						(event as { domain_id?: string }).domain_id,
					)
					break
				case 'pack.published':
				case 'binding.updated':
					// Envelope data may have changed; DomainPageHost watches
					// this counter and re-pulls its data source (YAP §6.10:
					// events locate, GET pulls).
					this.dataEpoch += 1
					break
				default:
					break
			}
		},
		async refreshManifest() {
			const epoch = this.domainEpoch
			const domainId = this.activeDomainId
			this.loadingManifest = true
			try {
				const next = await ymcl.refreshManifest().catch(() => null)
				if (epoch === this.domainEpoch && domainId === this.activeDomainId)
					this.setManifest(next ?? this.manifest)
			} finally {
				if (epoch === this.domainEpoch) this.loadingManifest = false
			}
		},
		/**
		 * The domain session died and could not be renewed silently (no refresh
		 * token, or the refresh token itself expired). Clears the stale session
		 * and stamps the loss so App.vue pops the domain login dialog.
		 */
		async handleSessionExpired(domainId?: string) {
			if (domainId && this.activeDomainId !== domainId) return
			this.session = null
			// 并发请求可能连续失败，3 秒内的重复上报只视为一次。
			this.requireLogin()
		},
		setSession(session: YmclStoredSession | null) {
			this.session = session
		},
		async passwordLogin(username: string, password: string) {
			this.loggingIn = true
			try {
				this.session = await ymcl.passwordLogin(this.activeDomainId, username, password)
				// Re-pull manifest with the new session so nav/pages match RBAC.
				await this.refreshManifest()
			} finally {
				this.loggingIn = false
			}
		},
		async register(username: string, email: string, password: string, nickname?: string) {
			this.loggingIn = true
			try {
				await ymcl.register(this.activeDomainId, username, email, password, nickname)
			} finally {
				this.loggingIn = false
			}
		},
		async oauthLogin() {
			this.loggingIn = true
			try {
				this.session = await ymcl.oauthLogin(this.activeDomainId)
				await this.refreshManifest()
			} finally {
				this.loggingIn = false
			}
		},
		/**
		 * Runs an external third-party login to completion: begin (opens the
		 * browser on the Rust side), then polls until completed/expired or
		 * the overall deadline passes. Returns the final poll status.
		 */
		async externalLogin(
			provider: string,
			loginType?: string,
		): Promise<YmclExternalPoll | null> {
			this.loggingIn = true
			try {
				const flow = await ymcl.externalBegin(
					this.activeDomainId,
					provider,
					loginType,
				)
				const deadline = Date.now() + EXTERNAL_POLL_MAX_MS
				while (Date.now() < deadline) {
					await new Promise((resolve) => setTimeout(resolve, EXTERNAL_POLL_INTERVAL_MS))
					const poll = await ymcl.externalPoll(this.activeDomainId, provider, flow.flow_id)
					if (poll.status === 'completed' && poll.access_token) {
						this.session = await ymcl.externalFinish(
							this.activeDomainId,
							poll.access_token,
							poll.refresh_token ?? null,
							poll.expires_in ?? null,
						)
						return poll
					}
					if (poll.status === 'expired') return poll
				}
				return { status: 'expired' }
			} finally {
				this.loggingIn = false
			}
		},
		async logout() {
			await ymcl.logout(this.activeDomainId)
			this.session = null
			this.requireLogin(true)
		},
		async switchDepartment(deptId: string) {
			if (!this.session) return
			this.session.session.context = undefined
			const info = await ymcl.contextSwitch(this.activeDomainId, deptId, null)
			this.session = { ...this.session, session: info }
		},
		async switchRole(roleId: string) {
			if (!this.session) return
			const info = await ymcl.contextSwitch(this.activeDomainId, null, roleId)
			this.session = { ...this.session, session: info }
		},
	},
})
