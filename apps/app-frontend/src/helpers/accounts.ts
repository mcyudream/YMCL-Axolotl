export type MinecraftAccountSortable = {
	account_type?: string
	profile?: {
		id?: string
		name?: string
	}
}

/** A domain sees only its own service's profiles; personal mode excludes joined domains. */
export function accountsForDomain<
	T extends {
		account_type?: string
		yggdrasil?: { api_root: string } | null
	},
>(
	accounts: readonly T[],
	domainRoots: Readonly<Record<string, readonly string[]>>,
	domainId: string | null,
): T[] {
	return accounts.filter((account) => {
		const apiRoot =
			account.account_type === 'yggdrasil'
				? account.yggdrasil?.api_root.replace(/\/+$/, '')
				: undefined
		if (domainId !== null) return !!apiRoot && !!domainRoots[domainId]?.includes(apiRoot)
		return !apiRoot || !Object.values(domainRoots).some((roots) => roots.includes(apiRoot))
	})
}

const ACCOUNT_TYPE_ORDER: Record<string, number> = {
	microsoft: 0,
	yggdrasil: 1,
	offline: 2,
}

export function compareMinecraftAccounts(
	left: MinecraftAccountSortable,
	right: MinecraftAccountSortable,
): number {
	const nameCmp = (left.profile?.name ?? '').localeCompare(right.profile?.name ?? '')
	if (nameCmp !== 0) return nameCmp

	const typeCmp =
		(ACCOUNT_TYPE_ORDER[left.account_type ?? ''] ?? 3) -
		(ACCOUNT_TYPE_ORDER[right.account_type ?? ''] ?? 3)
	if (typeCmp !== 0) return typeCmp

	const leftId = left.profile?.id ?? ''
	const rightId = right.profile?.id ?? ''
	return leftId < rightId ? -1 : leftId > rightId ? 1 : 0
}

export function sortMinecraftAccounts<T extends MinecraftAccountSortable>(
	accounts: readonly T[],
): T[] {
	return [...accounts].sort(compareMinecraftAccounts)
}
