import { invoke } from '@tauri-apps/api/core'

export const SCHEMATIC_SOURCES = [
	{ id: 'reden', requiresLogin: false, sorts: ['default'] },
	{ id: 'buildschematics', requiresLogin: false, sorts: ['default', 'name'] },
] as const
export type SchematicSource = (typeof SCHEMATIC_SOURCES)[number]['id']
export type SchematicBrowseSource = SchematicSource | 'all'
export type SchematicSort = (typeof SCHEMATIC_SOURCES)[number]['sorts'][number]
export function isSchematicSource(value: unknown): value is SchematicSource {
	return SCHEMATIC_SOURCES.some((source) => source.id === value)
}
export interface SchematicProject {
	source: SchematicSource
	id: string
	title: string
	summary: string
	author: string
	cover: string | null
	gameVersions: string[]
	files: { name: string; size: number | null }[]
	filesResolved: boolean
	sourceUrl: string
}
export interface SchematicDetail extends SchematicProject {
	description: string
	descriptionFormat: 'html' | 'markdown' | 'plain'
}
export interface SchematicPage {
	items: SchematicProject[]
	pageCount: number
	total: number
	failedSources: SchematicSource[]
}
export function getSchematic(source: SchematicSource, id: string): Promise<SchematicDetail> {
	return invoke('plugin:schematics|schematics_get', { source, id })
}
export function listSchematics(
	sources: SchematicSource[],
	page: number,
	query: string,
	pageSize: number,
	sort: SchematicSort,
): Promise<SchematicPage> {
	return invoke('plugin:schematics|schematics_list', { sources, page, query, pageSize, sort })
}
export function installSchematic(request: {
	instanceId: string
	source: SchematicSource
	id: string
	fileName: string
}): Promise<string> {
	return invoke('plugin:schematics|schematics_install', request)
}

export const installedSchematicsKey = (instanceId: string | null) =>
	['schematics', 'installed', instanceId] as const

/** Admits files to the preview flow; the native parser validates their NBT format. */
export function isSchematicFileName(name: string): boolean {
	return /\.(litematic|schem)$/i.test(name)
}

export type InstalledSchematic = { source: string; projectId: string; fileName: string }

export function getInstalledSchematics(instanceId: string): Promise<InstalledSchematic[]> {
	return invoke('plugin:schematics|schematics_installed', { instanceId })
}

export function hasInstalledSchematic(
	installed: InstalledSchematic[] | undefined,
	source: string,
	projectId: string,
	fileName: string,
): boolean {
	return (installed ?? []).some(
		(item) =>
			item.source === source &&
			item.projectId === projectId &&
			(!fileName || item.fileName === fileName),
	)
}
