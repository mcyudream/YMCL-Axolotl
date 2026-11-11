import type { ContentItem } from '@modrinth/ui'

export function contentItemReference(
	item: Pick<ContentItem, 'origin_provider' | 'provider_refs'> | null | undefined,
) {
	const references = item?.provider_refs ?? []
	const source = item?.origin_provider
		? references.find((reference) => reference.provider === item.origin_provider)
		: undefined
	if (item?.origin_provider && !source) return null
	const candidates = source
		? references.filter(
				(reference) =>
					reference.provider === source.provider && reference.project_id === source.project_id,
			)
		: references
	return (
		candidates.find((reference) =>
			reference.provider === 'curseforge'
				? reference.file_id != null
				: reference.version_id != null || ('file_id' in reference && reference.file_id != null),
		) ??
		source ??
		references[0] ??
		null
	)
}

export function contentItemProvider(
	item: Pick<ContentItem, 'origin_provider' | 'provider_refs'> | null | undefined,
) {
	return item?.origin_provider ?? contentItemReference(item)?.provider ?? null
}

export function matchesContentItem(
	item: ContentItem,
	target: ContentItem,
	originalFileName: string,
	originalFilePath?: string,
) {
	if (
		item.file_name === originalFileName ||
		item.file_path === originalFilePath ||
		item.file_path === target.file_path
	)
		return true

	const projectId = target.project?.id
	if (!projectId || item.project?.id !== projectId) return false

	const versionId = target.version?.id
	return !versionId || item.version?.id === versionId
}

export function applyContentItemUpdates(
	items: ContentItem[],
	target: ContentItem,
	originalFileName: string,
	originalFilePath: string | undefined,
	updates: Partial<ContentItem>,
) {
	for (const item of items) {
		if (matchesContentItem(item, target, originalFileName, originalFilePath)) {
			Object.assign(item, updates)
		}
	}
	Object.assign(target, updates)
}
