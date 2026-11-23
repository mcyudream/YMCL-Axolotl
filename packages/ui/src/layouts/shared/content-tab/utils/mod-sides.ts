import { defineMessages } from '#ui/composables/i18n'

export const modSideMessages = defineMessages({
	client: { id: 'content.mod-side.client', defaultMessage: 'Client only' },
	server: { id: 'content.mod-side.server', defaultMessage: 'Server only' },
	both: { id: 'content.mod-side.both', defaultMessage: 'Client and server' },
	unknown: { id: 'content.mod-side.unknown', defaultMessage: 'Unknown side' },
	optional: { id: 'content.mod-side.optional', defaultMessage: 'Server optional' },
	mark: { id: 'content.mod-side.mark', defaultMessage: 'Set mod side' },
	exportServer: { id: 'content.mod-side.export-server', defaultMessage: 'Export server pack' },
})

export const modSideSourceMessages = defineMessages({
	instance: { id: 'content.mod-side.source.instance', defaultMessage: 'Override for this modpack' },
	local: {
		id: 'content.mod-side.source.local',
		defaultMessage: 'Local rule for this mod and loader',
	},
	modrinth: {
		id: 'content.mod-side.source.modrinth',
		defaultMessage: 'Detected from Modrinth metadata',
	},
	curseforge: {
		id: 'content.mod-side.source.curseforge',
		defaultMessage: 'Detected from CurseForge metadata',
	},
	embedded: { id: 'content.mod-side.source.embedded', defaultMessage: 'Declared in the mod JAR' },
	unknown: {
		id: 'content.mod-side.source.unknown',
		defaultMessage: 'No explicit environment information',
	},
})
