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
