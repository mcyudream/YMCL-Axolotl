import { defineStore } from 'pinia'

import type { DomainHomePage } from './domain-home-paging.ts'

function pageScope(domainId: string, userId: string | null): string {
	return JSON.stringify([domainId, userId])
}

/** Page selections live only in this launcher process, scoped to each domain and account. */
export const useDomainHomeSession = defineStore('domainHomeSession', {
	state: () => ({
		pages: {} as Record<string, DomainHomePage>,
		servers: {} as Record<string, string>,
	}),
	getters: {
		pageFor:
			(state) =>
			(domainId: string, userId: string | null): DomainHomePage =>
				state.pages[pageScope(domainId, userId)] ?? 0,
		serverFor:
			(state) =>
			(domainId: string, userId: string | null): string =>
				state.servers[pageScope(domainId, userId)] ?? '',
	},
	actions: {
		rememberPage(domainId: string, userId: string | null, page: DomainHomePage) {
			this.pages[pageScope(domainId, userId)] = page
		},
		rememberServer(domainId: string, userId: string | null, serverId: string) {
			this.servers[pageScope(domainId, userId)] = serverId
		},
	},
})
