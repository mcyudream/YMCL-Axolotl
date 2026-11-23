import type { ContentItem } from '../types'

export function isPresentContentItem(item: ContentItem): boolean {
	return (
		item.instanceMaterializationState == null || item.instanceMaterializationState === 'present'
	)
}

export function isEnabledContentItem(item: ContentItem): boolean {
	return isPresentContentItem(item) && item.enabled === true
}

export function isDisabledContentItem(item: ContentItem): boolean {
	return isPresentContentItem(item) && item.enabled === false
}

export function canMarkContentModSide(item: ContentItem): boolean {
	return (
		item.project_type === 'mod' &&
		!!item.file_path &&
		!item.installing &&
		isPresentContentItem(item)
	)
}

export function canToggleContentItem(item: ContentItem): boolean {
	return (
		isPresentContentItem(item) &&
		item.enabled !== undefined &&
		item.instanceCapabilities?.canToggle !== false
	)
}

export function canUpdateContentItem(item: ContentItem): boolean {
	return (
		isPresentContentItem(item) &&
		item.update != null &&
		item.instanceCapabilities?.canUpdate !== false
	)
}

export function getUpdatableContentItems(
	items: ContentItem[],
	getItemId: (item: ContentItem) => string,
): ContentItem[] {
	const updates = new Map<string, ContentItem>()
	for (const item of items) {
		if (canUpdateContentItem(item)) updates.set(getItemId(item), item)
	}
	return [...updates.values()]
}

export interface ContentFilterOption {
	id: string
	label: string
}

export function getContentEnvironmentFilterValue(
	item: Pick<ContentItem, 'mod_side' | 'environment'>,
): 'client' | 'server' | 'client_and_server' | 'singleplayer' | undefined {
	if (item.mod_side) {
		if (item.mod_side.source === 'modrinth' && item.environment === 'singleplayer_only')
			return 'singleplayer'
		switch (item.mod_side.side) {
			case 'client':
				return 'client'
			case 'server':
				return 'server'
			case 'both':
				return 'client_and_server'
			default:
				return undefined
		}
	}
	switch (item.environment) {
		case 'client_only':
			return 'client'
		case 'server_only':
		case 'dedicated_server_only':
			return 'server'
		case 'client_and_server':
		case 'client_only_server_optional':
		case 'server_only_client_optional':
		case 'client_or_server':
		case 'client_or_server_prefers_both':
			return 'client_and_server'
		case 'singleplayer_only':
			return 'singleplayer'
		default:
			return undefined
	}
}
