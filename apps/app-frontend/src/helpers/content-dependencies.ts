import type { ContentItem } from '@modrinth/ui'

type DependencyReference = Pick<
	NonNullable<ContentItem['dependency']>['requires'][number],
	'provider' | 'projectId' | 'entryId' | 'fileId'
>

export function dependencyRefMatchesContentItem(
	reference: DependencyReference,
	item: {
		project?: Pick<ContentItem['project'], 'id'> | null
		provider_refs: ContentItem['provider_refs']
		instanceEntryId?: string
		instanceFileId?: string
	},
): boolean {
	if (reference.entryId && item.instanceEntryId) return reference.entryId === item.instanceEntryId
	if (reference.fileId) return reference.fileId === item.instanceFileId
	if (reference.entryId) return false
	const projectId =
		reference.provider === 'curseforge'
			? reference.projectId.replace(/^curseforge:/, '')
			: reference.projectId
	if (reference.provider === 'local' && projectId.startsWith('local:')) {
		return item.project?.id === projectId
	}
	if (item.provider_refs.length > 0) {
		return item.provider_refs.some(
			(ref) => ref.provider === reference.provider && String(ref.project_id) === projectId,
		)
	}

	const itemProjectId = item.project?.id ?? ''
	return reference.provider === 'curseforge'
		? itemProjectId.replace(/^curseforge:/, '') === projectId
		: itemProjectId === projectId
}
