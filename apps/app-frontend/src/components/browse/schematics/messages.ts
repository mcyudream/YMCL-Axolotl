import { defineMessages } from '@modrinth/ui'

export const schematicMessages = defineMessages({
	installationStatusError: {
		id: 'app.schematics.installation-status-error',
		defaultMessage: 'Could not check which schematics are installed.',
	},
	instancesError: {
		id: 'app.schematics.instances-error',
		defaultMessage: 'Could not load your instances.',
	},
	schematics: { id: 'app.browse.schematics-project-type', defaultMessage: 'Schematics' },
	loadError: {
		id: 'app.schematics.load-error',
		defaultMessage: 'Could not load schematics.',
	},
	emptyTitle: { id: 'app.schematics.empty-title', defaultMessage: 'No schematics found' },
	emptyDescription: {
		id: 'app.schematics.empty-description',
		defaultMessage: 'Try another page or a different search.',
	},
	loading: { id: 'app.schematics.loading', defaultMessage: 'Loading schematics…' },
	allSources: { id: 'app.browse.source.all', defaultMessage: 'All sources' },
	failedSources: {
		id: 'app.schematics.failed-sources',
		defaultMessage: '{sources} could not be loaded.',
	},
	retry: { id: 'app.schematics.retry', defaultMessage: 'Retry' },
	search: { id: 'app.schematics.search', defaultMessage: 'Search schematics' },
	view: { id: 'app.browse.display-mode.switch', defaultMessage: 'Switch view' },
	sortHot: { id: 'app.schematics.sort.hot', defaultMessage: 'Popular' },
	sortNewest: { id: 'browse.sort.project-published', defaultMessage: 'Project first published' },
	sortFeatured: { id: 'app.schematics.sort.featured', defaultMessage: 'Featured' },
	sortDefault: { id: 'app.schematics.sort.default', defaultMessage: 'Source order' },
	sortName: { id: 'app.schematics.sort.name', defaultMessage: 'Name' },
	openSource: { id: 'app.schematics.open-source', defaultMessage: 'View on source website' },
	backToInstance: { id: 'app.browse.back-to-instance', defaultMessage: 'Back to instance' },
	listView: { id: 'app.browse.display-mode.list', defaultMessage: 'List' },
	compactView: { id: 'app.browse.display-mode.compact-list', defaultMessage: 'Compact list' },
	gridView: { id: 'app.browse.display-mode.grid', defaultMessage: 'Grid' },
	authorSummary: { id: 'app.schematics.author-summary', defaultMessage: '{author} · {summary}' },
	selectInstance: { id: 'app.schematics.select-instance', defaultMessage: 'Choose an instance' },
	install: { id: 'app.schematics.install', defaultMessage: 'Install' },
	installing: { id: 'app.schematics.installing', defaultMessage: 'Installing…' },
	preview: { id: 'app.schematics.preview', defaultMessage: 'Preview' },
	files: { id: 'app.schematics.files', defaultMessage: 'Files' },
	description: { id: 'project.description.title', defaultMessage: 'Description' },
	creator: { id: 'app.schematics.creator', defaultMessage: 'Creator' },
	sourceHeading: { id: 'app.schematics.source-heading', defaultMessage: 'Source' },
	previewSource: {
		id: 'app.schematics.preview-source',
		defaultMessage: '{source} · {fileName}',
	},
	installed: { id: 'app.schematics.installed', defaultMessage: 'Installed' },
	installSuccess: { id: 'app.schematics.install-success', defaultMessage: 'Schematic installed' },
	installFailed: {
		id: 'app.schematics.install-failed',
		defaultMessage: 'Could not install this schematic. Please try again.',
	},
	fileExists: {
		id: 'app.schematics.file-exists',
		defaultMessage:
			'A file with this name already exists in the instance. Choose another instance or rename the existing file.',
	},
	cancel: { id: 'app.third-party.cancel', defaultMessage: 'Cancel' },
	modpacks: { id: 'app.browse.project-type.modpacks', defaultMessage: 'Modpacks' },
	mods: { id: 'app.browse.project-type.mods', defaultMessage: 'Mods' },
	resourcepacks: {
		id: 'app.browse.project-type.resourcepacks',
		defaultMessage: 'Resource Packs',
	},
	datapacks: { id: 'app.browse.project-type.datapacks', defaultMessage: 'Data Packs' },
	maps: { id: 'app.browse.project-type.maps', defaultMessage: 'Maps' },
	shaders: { id: 'app.browse.project-type.shaders', defaultMessage: 'Shaders' },
	servers: { id: 'app.browse.project-type.servers', defaultMessage: 'Servers' },
	favorites: { id: 'app.browse.project-type.favorites', defaultMessage: 'Favorites' },
})

export const schematicSourceMessages = defineMessages({
	reden: { id: 'app.schematics.source.reden', defaultMessage: 'Reden' },
	buildschematics: {
		id: 'app.schematics.source.buildschematics',
		defaultMessage: 'BuildSchematics',
	},
})
