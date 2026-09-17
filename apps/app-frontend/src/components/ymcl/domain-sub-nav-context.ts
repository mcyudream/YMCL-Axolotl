import type { ComputedRef, InjectionKey } from 'vue'

/** Shared state between DomainSubNav and its recursive DomainSubNavNode rows. */
export interface DomainSubNavContext {
	/** Page id of the domain page currently routed to. */
	activePageId: ComputedRef<string>
	isCollapsed: (key: string) => boolean
	toggle: (key: string) => void
}

export const DOMAIN_SUB_NAV_CONTEXT: InjectionKey<DomainSubNavContext> = Symbol(
	'ymcl-domain-sub-nav',
)
