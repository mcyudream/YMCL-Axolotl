import { defineStore } from 'pinia'

import type { HomeDashboardConfig } from '@/components/home/home-dashboard'
import {
	PERSONAL_DOMAIN_ID,
	ymcl,
	type YmclDomainSummary,
	type YmclExternalPoll,
	type YmclManifest,
	type YmclStoredSession,
} from '@/helpers/ymcl'
import { mapHomeProfileToDashboard, type YmclHomeProfile } from '@/helpers/ymcl-home'

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
	}),
	getters: {
		isPersonal: (state) => state.activeDomainId === PERSONAL_DOMAIN_ID,
		activeDomain: (state) =>
			state.domains.find((domain) => domain.id === state.activeDomainId) ?? null,
		/**
		 * Manifest navigation sorted for rendering; the personal domain has no
		 * manifest and falls back to the built-in launcher navigation.
		 */
		navigation: (state) =>
			[...(state.manifest?.navigation ?? [])].sort((a, b) => (a.sort ?? 0) - (b.sort ?? 0)),
		pageById: (state) => (pageId: string) =>
			state.manifest?.pages.find((page) => page.id === pageId) ?? null,
		/**
		 * Domain-hosted home layout (YAP §6.5 home capability) mapped onto the
		 * launcher's existing dashboard config via the shared mapping in
		 * helpers/ymcl-home. Null when the domain does not host the layout.
		 */
		domainHomeDashboard: (state): HomeDashboardConfig | null =>
			mapHomeProfileToDashboard(state.manifest?.home as YmclHomeProfile | undefined),
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
		async init() {
			if (this.initialized) return
			const domainState = await ymcl.domainsState().catch(() => null)
			if (domainState) {
				this.domains = domainState.domains
				this.activeDomainId = domainState.active_domain_id
			}
			this.manifest = await ymcl.manifest().catch(() => null)
			if (!this.isPersonal && this.activeDomainId) {
				this.session = await ymcl.session(this.activeDomainId).catch(() => null)
			}
			this.initialized = true
		},
		applyDomainsState(domainState: YmclDomainsState) {
			this.domains = domainState.domains
			this.activeDomainId = domainState.active_domain_id
		},
		async addDomain(origin: string) {
			this.adding = true
			try {
				this.applyDomainsState(await ymcl.addDomain(origin))
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
			this.loadingManifest = true
			try {
				this.applyDomainsState(await ymcl.activateDomain(id))
				this.manifest = await ymcl.manifest().catch(() => null)
				this.session = id === PERSONAL_DOMAIN_ID ? null : await ymcl.session(id).catch(() => null)
			} finally {
				this.loadingManifest = false
			}
		},
		/**
		 * Reacts to a server-pushed event (YAP §6.10). Events only locate
		 * the change; the actual data is always pulled fresh.
		 */
		async handleServerEvent(event: { type?: string }) {
			switch (event.type) {
				case 'manifest.updated':
				case 'session.context_changed':
					await this.refreshManifest()
					if (!this.isPersonal && this.activeDomainId) {
						this.session = await ymcl.session(this.activeDomainId).catch(() => null)
					}
					break
				case 'session.revoked':
					this.session = null
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
			this.loadingManifest = true
			try {
				this.manifest = await ymcl.refreshManifest().catch(() => this.manifest)
			} finally {
				this.loadingManifest = false
			}
		},
		setSession(session: YmclStoredSession | null) {
			this.session = session
		},
		async passwordLogin(username: string, password: string) {
			this.loggingIn = true
			try {
				this.session = await ymcl.passwordLogin(this.activeDomainId, username, password)
			} finally {
				this.loggingIn = false
			}
		},
		async oauthLogin() {
			this.loggingIn = true
			try {
				this.session = await ymcl.oauthLogin(this.activeDomainId)
			} finally {
				this.loggingIn = false
			}
		},
		/**
		 * Runs an external third-party login to completion: begin (opens the
		 * browser on the Rust side), then polls until completed/expired or
		 * the overall deadline passes. Returns the final poll status.
		 */
		async externalLogin(provider: string): Promise<YmclExternalPoll | null> {
			this.loggingIn = true
			try {
				const flow = await ymcl.externalBegin(this.activeDomainId, provider)
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
