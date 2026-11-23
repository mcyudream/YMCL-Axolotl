import type { ContentItem, ContentModSide, ContentModSideInfo } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'

export type ModSideScope = 'instance' | 'local'
export type ServerPackMode = 'full' | 'slim'

export interface ServerPackPreview {
	revision: number
	files: Array<{
		path: string
		size: number
		selected: boolean
		modSide: ContentModSideInfo | null
		requires: string[]
		downloads: string[]
	}>
	excludedMods: Array<{ path: string; reason: string }>
	missingMods: string[]
}

export function selectServerPackFiles(
	files: ServerPackPreview['files'],
	previous?: { files: ReadonlyArray<{ path: string }>; selectedPaths: readonly string[] },
): string[] {
	const previousPaths = new Set(previous?.files.map((file) => file.path))
	const previousSelected = new Set(previous?.selectedPaths)
	return files
		.filter((file) =>
			previousPaths.has(file.path) ? previousSelected.has(file.path) : file.selected,
		)
		.map((file) => file.path)
}

export async function setModSide(
	instanceId: string,
	paths: string[],
	scope: ModSideScope,
	side: ContentModSide | null,
): Promise<ContentItem[]> {
	return await invoke('plugin:instance|instance_set_mod_side', { instanceId, paths, scope, side })
}

export async function previewServerPack(
	instanceId: string,
	mode: ServerPackMode,
): Promise<ServerPackPreview> {
	return await invoke('plugin:instance|instance_preview_server_pack', { instanceId, mode })
}

export async function exportServerPack(
	instanceId: string,
	exportLocation: string,
	request: {
		mode: ServerPackMode
		selectedPaths: string[]
		revision: number
		name: string
		version: string
		description: string
	},
): Promise<void> {
	await invoke('plugin:instance|instance_export_server_pack', {
		instanceId,
		exportLocation,
		request,
	})
}
