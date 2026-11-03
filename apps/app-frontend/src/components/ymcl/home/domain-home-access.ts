import {
	DOMAIN_DESIGN_PERMISSION,
	hasDomainPermission,
	PERSONAL_DOMAIN_ID,
	type YmclStoredSession,
} from '../../../helpers/ymcl.ts'

export function domainHomeLayoutAccess(input: {
	domainId: string
	session: YmclStoredSession | null
	hasDashboard: boolean
	locked: boolean
}) {
	const personal = input.domainId === PERSONAL_DOMAIN_ID
	const member = !personal && input.session?.domain_id === input.domainId
	return {
		canCustomize: personal || (member && input.hasDashboard && !input.locked),
		canManageDomain:
			member &&
			input.hasDashboard &&
			hasDomainPermission(input.session?.session.permissions, DOMAIN_DESIGN_PERMISSION),
	}
}
