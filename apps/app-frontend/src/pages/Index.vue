<script setup lang="ts">
import {
	CheckIcon,
	GridIcon,
	LayoutTemplateIcon,
	LockIcon,
	LockOpenIcon,
	MinimizeIcon,
	MoveIcon,
	PaintbrushIcon,
	PencilIcon,
	PlusIcon,
	RotateCounterClockwiseIcon,
	SaveIcon,
	SpinnerIcon,
	XIcon,
} from '@modrinth/assets'
import {
	ConfirmModal,
	defineMessages,
	injectNotificationManager,
	injectPageContext,
	useVIntl,
} from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import {
	computed,
	inject,
	onUnmounted,
	type Ref,
	ref,
	shallowRef,
	useTemplateRef,
	watch,
} from 'vue'
import { useRoute, useRouter } from 'vue-router'

import {
	createDefaultHomeDashboard,
	createHomeDashboardSaveQueue,
	HOME_DASHBOARD_VERSION,
	type HomeDashboardConfig,
	normalizeHomeDashboard,
} from '@/components/home/home-dashboard'
import HomeDailyChallenge from '@/components/home/HomeDailyChallenge.vue'
import HomeDashboard from '@/components/home/HomeDashboard.vue'
import HomeInstancePickerModal from '@/components/home/HomeInstancePickerModal.vue'
import HomeMinecraftNews from '@/components/home/HomeMinecraftNews.vue'
import HomeMinimal from '@/components/home/HomeMinimal.vue'
import HomePlayInsights from '@/components/home/HomePlayInsights.vue'
import type AccountsCard from '@/components/ui/AccountsCard.vue'
import { domainHomeLayoutAccess } from '@/components/ymcl/home/domain-home-access'
import type { DomainHomePage } from '@/components/ymcl/home/domain-home-paging'
import { useDomainHomeSession } from '@/components/ymcl/home/domain-home-session'
import DomainHomePages from '@/components/ymcl/home/DomainHomePages.vue'
import { DIRECT_LINKS_SYNCED_EVENT } from '@/helpers/direct-link-sync'
import { instance_listener } from '@/helpers/events'
import { list } from '@/helpers/instance'
import { get as getSettings, set as setSettings } from '@/helpers/settings'
import type { GameInstance } from '@/helpers/types'
import { ymcl } from '@/helpers/ymcl'
import { domainPageRoute, parseShortcutTarget } from '@/helpers/ymcl-domain'
import {
	mapDashboardToHomeProfile,
	mapHomeProfileToDashboard,
	type YmclHomeProfile,
} from '@/helpers/ymcl-home'
import {
	applyHomeOverride,
	clearYmclHomeOverride,
	loadYmclHomeOverride,
	saveYmclHomeOverride,
	type YmclHomeOverride,
} from '@/helpers/ymcl-home-override'
import { useBreadcrumbs } from '@/store/breadcrumbs'
import { useTheming } from '@/store/state'
import type { FeatureFlag, HomeLayout } from '@/store/theme'
import { useYmclStore } from '@/store/ymcl'

const { handleError, addNotification } = injectNotificationManager()
const route = useRoute()
const router = useRouter()
const breadcrumbs = useBreadcrumbs()
const { formatMessage } = useVIntl()
const themeStore = useTheming()
const pageContext = injectPageContext()

const messages = defineMessages({
	home: { id: 'app.home.breadcrumb', defaultMessage: 'Home' },
	editOwnHome: {
		id: 'app.home.layout.edit-own-home',
		defaultMessage: 'Customize your home',
	},
	editDomainLayout: {
		id: 'app.home.layout.edit-domain-layout',
		defaultMessage: 'Edit domain layout',
	},
	publishDomainLayout: {
		id: 'app.home.layout.publish-domain-layout',
		defaultMessage: 'Publish domain layout',
	},
	cancelDomainEditing: {
		id: 'app.home.layout.cancel-domain-editing',
		defaultMessage: 'Cancel domain layout editing',
	},
	lockDomainLayout: {
		id: 'app.home.layout.lock-domain-layout',
		defaultMessage: 'Lock domain layout to prevent member customization',
	},
	unlockDomainLayout: {
		id: 'app.home.layout.unlock-domain-layout',
		defaultMessage: 'Unlock domain layout to allow member customization',
	},
	domainLayoutPublished: {
		id: 'app.home.layout.domain-layout-published',
		defaultMessage: 'Domain layout published for all members',
	},
	domainLayoutLocked: {
		id: 'app.home.layout.domain-layout-locked',
		defaultMessage: 'Domain layout locked',
	},
	domainLayoutUnlocked: {
		id: 'app.home.layout.domain-layout-unlocked',
		defaultMessage: 'Domain layout unlocked',
	},
	unpublishedWidgets: {
		id: 'app.home.layout.unpublished-widgets',
		defaultMessage:
			'{count, plural, one {# device-specific widget was not published.} other {# device-specific widgets were not published.}}',
	},
	switchToMinimal: {
		id: 'app.home.layout.switch-to-minimal',
		defaultMessage: 'Switch to Minimal Home',
	},
	switchToInformation: {
		id: 'app.home.layout.switch-to-information',
		defaultMessage: 'Switch to Information Home',
	},
	homeLayoutToggle: {
		id: 'app.home.layout.toggle',
		defaultMessage: 'Minimal Home',
	},
	switchToGridWidgetLayout: {
		id: 'app.home.widgets.layout.switch-to-grid',
		defaultMessage: 'Switch to grid widget layout',
	},
	switchToFreeWidgetLayout: {
		id: 'app.home.widgets.layout.switch-to-free',
		defaultMessage: 'Switch to free widget layout',
	},
	widgetLayoutToggle: {
		id: 'app.home.widgets.layout.toggle',
		defaultMessage: 'Widget layout mode',
	},
	resetWidgets: {
		id: 'app.home.widgets.reset-confirm',
		defaultMessage: 'Restore the default widget layout?',
	},
	resetWidgetsConfirm: {
		id: 'app.home.widgets.reset-confirm-description',
		defaultMessage: 'Your current layout adjustments will be cleared.',
	},
	customizeWidgets: {
		id: 'app.home.widgets.customize',
		defaultMessage: 'Customize widgets',
	},
	doneEditing: { id: 'app.home.widgets.done', defaultMessage: 'Finish editing' },
	addWidget: { id: 'app.home.widgets.add', defaultMessage: 'Add widget' },
	resetWidgetLayout: {
		id: 'app.home.widgets.reset',
		defaultMessage: 'Restore default widgets',
	},
	personalHomeSaved: {
		id: 'app.home.widgets.personal-saved',
		defaultMessage: 'Your home layout was saved locally',
	},
})

const recentProjectsInHomeFlag: FeatureFlag = 'worlds_in_home'

breadcrumbs.setRootContext({ name: formatMessage(messages.home), link: route.path })

const instances = ref<GameInstance[]>([])
const accountsCard = inject<Ref<InstanceType<typeof AccountsCard> | null>>(
	'accountsCard',
	ref(null),
)
const playerName = computed(() => accountsCard.value?.selectedProfile?.name ?? null)
const ymclStore = useYmclStore()
const homeSession = useDomainHomeSession()
const homeSessionUserId = computed(() =>
	ymclStore.session?.domain_id === ymclStore.activeDomainId
		? ymclStore.session.session.user_id
		: null,
)
const domainHomePage = computed({
	get: () => homeSession.pageFor(ymclStore.activeDomainId, homeSessionUserId.value),
	set: (page: DomainHomePage) =>
		homeSession.rememberPage(ymclStore.activeDomainId, homeSessionUserId.value, page),
})
const domainLanding = computed(() => ymclStore.domainHomeLanding)
const showDomainLanding = computed(() => ymclStore.showDomainLanding)
const domainDashboard = computed<HomeDashboardConfig | null>(() => {
	if (ymclStore.isPersonal) return null
	return (
		ymclStore.domainHomeDashboard ??
		(showDomainLanding.value
			? { version: HOME_DASHBOARD_VERSION, layout: 'grid', widgets: [] }
			: null)
	)
})
const domainDashboardLocked = computed(() => !ymclStore.isPersonal && ymclStore.domainHomeLocked)
const homeAccess = computed(() =>
	domainHomeLayoutAccess({
		domainId: ymclStore.activeDomainId,
		session: ymclStore.session,
		hasDashboard: domainDashboard.value !== null,
		locked: domainDashboardLocked.value,
	}),
)
const canManageDomainLayout = computed(
	() => !ymclStore.isEntranceHidden('home-customization') && homeAccess.value.canManageDomain,
)
/** Unlocked domain homes allow members to customize their own view (YAP §6.5 用户覆盖). */
const canOverrideDomainHome = computed(() => !ymclStore.isPersonal && homeAccess.value.canCustomize)
const canEditOwnHome = computed(
	() => !ymclStore.isEntranceHidden('home-customization') && homeAccess.value.canCustomize,
)
const editingScope = ref<'personal' | 'member' | 'domain' | null>(null)
const editingActive = computed(() => editingScope.value !== null)
const domainEditingConfig = ref<HomeDashboardConfig | null>(null)
type DomainHomeOperation = {
	domainId: string
	userId: string | null
	kind: 'load' | 'publish' | 'lock'
}
const domainOperation = shallowRef<DomainHomeOperation | null>(null)
const savingDomainHome = computed(() => domainOperation.value !== null)
/** Member's local customization of the domain home, keyed per domain (YAP §6.5). */
const domainOverride = ref<YmclHomeOverride | null>(null)
const overrideDomainId = computed(() => (ymclStore.isPersonal ? null : ymclStore.activeDomainId))
watch(
	overrideDomainId,
	(id) => {
		domainOverride.value = id ? loadYmclHomeOverride(id) : null
	},
	{ immediate: true },
)
const memberDashboard = computed(() => {
	const domain = domainDashboard.value
	if (!domain) return null
	return canOverrideDomainHome.value ? applyHomeOverride(domain, domainOverride.value) : domain
})
const effectiveDashboard = computed(() => memberDashboard.value ?? dashboardConfig.value)
const renderedDashboard = computed(() => {
	const config =
		(editingScope.value === 'member' || editingScope.value === 'domain') &&
		domainEditingConfig.value
			? domainEditingConfig.value
			: effectiveDashboard.value
	if (!config || !ymclStore.simpleModeActive || editingActive.value) return config
	return {
		...config,
		widgets: config.widgets.filter((widget) => {
			if (widget.kind === 'page-shortcut' && widget.shortcut) {
				const target = parseShortcutTarget(widget.shortcut.target)
				if (target)
					return !ymclStore.isRouteHidden(
						target.kind === 'page' ? domainPageRoute(target.pageId) : target.route,
					)
			}
			if (widget.kind === 'data-card' && widget.dataSource) {
				const sourceId = widget.dataSource.id
				const pageId =
					ymclStore.manifest?.pages.find((page) => page.data_source === sourceId)?.id ?? sourceId
				return !ymclStore.isRouteHidden(domainPageRoute(pageId))
			}
			return true
		}),
	}
})
const dashboardConfig = ref<HomeDashboardConfig | null>(null)
const dashboard = ref<InstanceType<typeof HomeDashboard>>()
watch(
	[
		() => ymclStore.activeDomainId,
		() => ymclStore.session?.domain_id,
		() => ymclStore.session?.session.user_id,
	],
	() => {
		cancelHomeEditing()
		domainOperation.value = null
	},
	{ flush: 'sync' },
)
watch(
	[canEditOwnHome, canManageDomainLayout],
	() => {
		if (editingScope.value === 'domain' ? !canManageDomainLayout.value : !canEditOwnHome.value)
			cancelHomeEditing()
		if (!canManageDomainLayout.value) domainOperation.value = null
	},
	{ flush: 'sync' },
)
const instancePicker = ref<InstanceType<typeof HomeInstancePickerModal>>()
const isMinimal = computed(() => themeStore.homeLayout === 'minimal')
const showHomeControls = computed(
	() =>
		!showDomainLanding.value ||
		(domainHomePage.value === 1 && (canEditOwnHome.value || canManageDomainLayout.value)),
)
const isFreeWidgetLayout = computed(() => renderedDashboard.value?.layout === 'free')
const switchingLayout = ref(false)
const dashboardSaveQueue = createHomeDashboardSaveQueue(
	async (config) => {
		const settings = await getSettings()
		settings.home_widgets = config
		await setSettings(settings)
	},
	(config) => {
		dashboardConfig.value = config
	},
	handleError,
)
const floatingControlsStyle = computed(() => ({
	bottom: themeStore.getFeatureFlag('page_path') ? '3.5rem' : '1rem',
	right: `calc(${pageContext.floatingActionBarOffsets?.right.value ?? '0px'} + 1rem)`,
}))

const animateSidebarShow = ref(false)
setTimeout(() => {
	animateSidebarShow.value = true
}, 200)

async function clearMissingMinimalInstance() {
	const selectedId = themeStore.minimalHomeInstanceId
	if (!selectedId || instances.value.some((instance) => instance.id === selectedId)) return

	themeStore.minimalHomeInstanceId = null
	try {
		const settings = await getSettings()
		if (settings.minimal_home_instance_id === null) return
		settings.minimal_home_instance_id = null
		await setSettings(settings)
	} catch (error) {
		handleError(error)
	}
}

async function fetchInstances() {
	try {
		instances.value = await list()
		await clearMissingMinimalInstance()
		return true
	} catch (error) {
		handleError(error)
		return false
	}
}

async function loadDashboardConfig() {
	try {
		const settings = await getSettings()
		const normalized = normalizeHomeDashboard(settings.home_widgets)
		if (normalized) {
			dashboardConfig.value = normalized
			return
		}

		const config = createDefaultHomeDashboard(themeStore.getFeatureFlag(recentProjectsInHomeFlag))
		dashboardConfig.value = config
		settings.home_widgets = config
		await setSettings(settings)
	} catch (error) {
		dashboardConfig.value = createDefaultHomeDashboard(
			themeStore.getFeatureFlag(recentProjectsInHomeFlag),
		)
		handleError(error)
	}
}

function updateDashboardConfig(config: HomeDashboardConfig) {
	const previous = dashboardConfig.value ?? config
	dashboardConfig.value = config
	void dashboardSaveQueue.enqueue(config, previous)
}

/**
 * `window.confirm` is dead in Tauri (its dialog command is unavailable), so
 * the reset gate uses the in-app ConfirmModal instead.
 */
const resetConfirmModal = useTemplateRef<InstanceType<typeof ConfirmModal>>('resetConfirmModal')
const resetScope = shallowRef<{
	domainId: string
	userId: string | null
	scope: typeof editingScope.value
} | null>(null)

function requestResetDashboard() {
	if (!editingActive.value || savingDomainHome.value) return
	resetScope.value = {
		domainId: ymclStore.activeDomainId,
		userId: homeSessionUserId.value,
		scope: editingScope.value,
	}
	resetConfirmModal.value?.show()
}

function resetDashboardConfig() {
	const requested = resetScope.value
	resetScope.value = null
	if (
		!requested ||
		!editingActive.value ||
		savingDomainHome.value ||
		requested.domainId !== ymclStore.activeDomainId ||
		requested.userId !== homeSessionUserId.value ||
		requested.scope !== editingScope.value
	)
		return
	if (editingScope.value === 'domain') {
		if (canManageDomainLayout.value)
			domainEditingConfig.value = cloneDashboard(createDefaultHomeDashboard())
		return
	}
	if (!canEditOwnHome.value) return
	if (editingScope.value === 'member') {
		domainEditingConfig.value = cloneDashboard(
			domainDashboard.value ?? createDefaultHomeDashboard(),
		)
		return
	}
	updateDashboardConfig(createDefaultHomeDashboard())
}

function onDashboardChange(config: HomeDashboardConfig) {
	if (savingDomainHome.value) return
	if (editingScope.value === 'domain') {
		if (canManageDomainLayout.value) domainEditingConfig.value = config
		return
	}
	if (!canEditOwnHome.value) return
	if (editingScope.value === 'member') {
		domainEditingConfig.value = config
		return
	}
	// 域布局的只读渲染不得回写个人配置：问候语设置等控件在非编辑态也会发
	// change，若放行会把域卡片写进 settings.home_widgets（个人主页被污染）。
	if (!ymclStore.isPersonal) return
	updateDashboardConfig(config)
}

function cloneDashboard(config: HomeDashboardConfig): HomeDashboardConfig {
	return JSON.parse(JSON.stringify(config)) as HomeDashboardConfig
}

function finishMemberHomeEditing() {
	const config = domainEditingConfig.value
	const domain = domainDashboard.value
	const domainId = overrideDomainId.value
	if (!canOverrideDomainHome.value || !config || !domain || !domainId) return
	const removed = domain.widgets
		.map((widget) => widget.id)
		.filter((id) => !config.widgets.some((widget) => widget.id === id))
	if (JSON.stringify(config) === JSON.stringify(domain)) {
		clearYmclHomeOverride(domainId)
	} else {
		saveYmclHomeOverride(domainId, {
			version: HOME_DASHBOARD_VERSION,
			layout: config.layout,
			widgets: config.widgets,
			removed,
		})
	}
	domainOverride.value = loadYmclHomeOverride(domainId)
	cancelHomeEditing()
	addNotification({
		title: formatMessage(messages.personalHomeSaved),
		type: 'success',
	})
}

function cancelHomeEditing() {
	editingScope.value = null
	domainEditingConfig.value = null
}

function beginDomainOperation(kind: DomainHomeOperation['kind']): DomainHomeOperation | null {
	if (!canManageDomainLayout.value || savingDomainHome.value) return null
	const operation = {
		domainId: ymclStore.activeDomainId,
		userId: homeSessionUserId.value,
		kind,
	}
	domainOperation.value = operation
	return operation
}

function isCurrentDomainOperation(operation: DomainHomeOperation): boolean {
	return (
		domainOperation.value === operation &&
		ymclStore.activeDomainId === operation.domainId &&
		homeSessionUserId.value === operation.userId &&
		canManageDomainLayout.value
	)
}

async function publishDomainHomeProfile(profile: YmclHomeProfile, operation: DomainHomeOperation) {
	await invoke('plugin:ymcl|ymcl_chrome_home_put', {
		config: profile,
		domainId: operation.domainId,
	})
	if (!isCurrentDomainOperation(operation)) return false
	if (ymclStore.manifest) ymclStore.setManifest({ ...ymclStore.manifest, home: profile })
	const fresh = await ymcl.refreshManifest().catch(() => null)
	if (!isCurrentDomainOperation(operation)) return false
	if (fresh) ymclStore.setManifest(fresh)
	return true
}

async function startDomainLayoutEditing() {
	if (editingActive.value) return
	const operation = beginDomainOperation('load')
	if (!operation) return
	try {
		const profile = await invoke<YmclHomeProfile>('plugin:ymcl|ymcl_chrome_home_get')
		if (!isCurrentDomainOperation(operation)) return
		domainEditingConfig.value = cloneDashboard(
			mapHomeProfileToDashboard(profile, ymclStore.manifest?.data_sources) ?? {
				version: HOME_DASHBOARD_VERSION,
				layout: profile.layout === 'free' ? 'free' : 'grid',
				widgets: [],
			},
		)
		editingScope.value = 'domain'
	} catch (error) {
		if (isCurrentDomainOperation(operation)) handleError(error)
	} finally {
		if (domainOperation.value === operation) domainOperation.value = null
	}
}

async function finishDomainLayoutEditing() {
	const config = domainEditingConfig.value
	if (editingScope.value !== 'domain' || !config) return
	const operation = beginDomainOperation('publish')
	if (!operation) return
	try {
		const original = await invoke<YmclHomeProfile>('plugin:ymcl|ymcl_chrome_home_get')
		if (!isCurrentDomainOperation(operation)) return
		const { profile, skippedWidgets } = mapDashboardToHomeProfile(config, original)
		if (!(await publishDomainHomeProfile(profile, operation))) return
		cancelHomeEditing()
		addNotification({
			title: formatMessage(messages.domainLayoutPublished),
			text: skippedWidgets
				? formatMessage(messages.unpublishedWidgets, { count: skippedWidgets })
				: undefined,
			type: 'success',
		})
	} catch (error) {
		if (isCurrentDomainOperation(operation)) handleError(error)
	} finally {
		if (domainOperation.value === operation) domainOperation.value = null
	}
}

async function toggleDomainLayoutLock() {
	if (editingActive.value) return
	const operation = beginDomainOperation('lock')
	if (!operation) return
	try {
		const original = await invoke<YmclHomeProfile>('plugin:ymcl|ymcl_chrome_home_get')
		if (!isCurrentDomainOperation(operation)) return
		const locked = original.locked !== true
		if (!(await publishDomainHomeProfile({ ...original, locked }, operation))) return
		addNotification({
			title: formatMessage(locked ? messages.domainLayoutLocked : messages.domainLayoutUnlocked),
			type: 'success',
		})
	} catch (error) {
		if (isCurrentDomainOperation(operation)) handleError(error)
	} finally {
		if (domainOperation.value === operation) domainOperation.value = null
	}
}

async function selectMinimalInstance(instance: GameInstance) {
	try {
		const settings = await getSettings()
		settings.minimal_home_instance_id = instance.id
		await setSettings(settings)
		themeStore.minimalHomeInstanceId = instance.id
	} catch (error) {
		handleError(error)
	}
}

function createInstance() {
	void router.push('/create')
}

async function toggleHomeLayout() {
	if (switchingLayout.value) return

	const previousLayout = themeStore.homeLayout
	const nextLayout: HomeLayout = previousLayout === 'minimal' ? 'standard' : 'minimal'
	switchingLayout.value = true
	themeStore.homeLayout = nextLayout

	try {
		const settings = await getSettings()
		settings.home_layout = nextLayout
		await setSettings(settings)
		if (nextLayout === 'minimal') cancelHomeEditing()
	} catch (error) {
		themeStore.homeLayout = previousLayout
		handleError(error)
	} finally {
		switchingLayout.value = false
	}
}

function toggleDashboardEditing() {
	if (!canEditOwnHome.value || savingDomainHome.value || editingScope.value === 'domain') return
	if (editingScope.value === 'member') {
		finishMemberHomeEditing()
		return
	}
	if (editingScope.value === 'personal') {
		cancelHomeEditing()
		return
	}
	if (canOverrideDomainHome.value) {
		startDomainHomeEditing()
		return
	}
	editingScope.value = 'personal'
}

function startDomainHomeEditing() {
	if (editingActive.value || !canOverrideDomainHome.value || !memberDashboard.value) return
	domainEditingConfig.value = cloneDashboard(memberDashboard.value)
	editingScope.value = 'member'
}

function toggleWidgetLayout() {
	dashboard.value?.setLayout(isFreeWidgetLayout.value ? 'grid' : 'free')
}

function openWidgetPicker() {
	dashboard.value?.openWidgetPicker()
}

await fetchInstances()
await loadDashboardConfig()

window.addEventListener(DIRECT_LINKS_SYNCED_EVENT, fetchInstances)

const unlistenInstance = await instance_listener(async () => {
	await fetchInstances()
})

onUnmounted(() => {
	domainOperation.value = null
	unlistenInstance()
	window.removeEventListener(DIRECT_LINKS_SYNCED_EVENT, fetchInstances)
})
</script>

<template>
	<HomeInstancePickerModal
		ref="instancePicker"
		:instances="instances"
		:selected-instance-id="themeStore.minimalHomeInstanceId"
		@select="selectMinimalInstance"
	/>
	<div :class="showDomainLanding ? 'h-full min-h-0' : 'min-h-full'">
		<DomainHomePages
			v-if="showDomainLanding"
			:key="`${ymclStore.activeDomainId}:${ymclStore.session?.session.user_id ?? ''}`"
			v-model:page="domainHomePage"
			:config="domainLanding"
			:player-name="playerName"
		>
			<template
				v-if="renderedDashboard && (renderedDashboard.widgets.length || editingActive)"
				#information
			>
				<HomeDashboard
					ref="dashboard"
					:config="renderedDashboard"
					:instances="instances"
					:player-name="playerName"
					:editing="editingActive && !savingDomainHome"
					@change="onDashboardChange"
				/>
			</template>
		</DomainHomePages>
		<HomeDashboard
			v-else-if="!isMinimal && renderedDashboard"
			ref="dashboard"
			:config="renderedDashboard"
			:instances="instances"
			:player-name="playerName"
			:editing="editingActive && !savingDomainHome"
			@change="onDashboardChange"
		/>

		<HomeMinimal
			v-else
			:instances="instances"
			:player-name="playerName"
			:selected-instance-id="themeStore.minimalHomeInstanceId"
			@choose="instancePicker?.show()"
			@create="createInstance"
		/>
	</div>
	<div v-if="showHomeControls" class="home-floating-controls" :style="floatingControlsStyle">
		<template v-if="(!isMinimal || showDomainLanding) && (canEditOwnHome || canManageDomainLayout)">
			<button
				v-if="editingActive"
				v-tooltip="formatMessage(messages.addWidget)"
				type="button"
				class="home-floating-action"
				:aria-label="formatMessage(messages.addWidget)"
				:disabled="savingDomainHome"
				@click="openWidgetPicker"
			>
				<PlusIcon />
			</button>
			<button
				v-if="editingActive"
				v-tooltip="formatMessage(messages.resetWidgetLayout)"
				type="button"
				class="home-floating-action"
				:aria-label="formatMessage(messages.resetWidgetLayout)"
				:disabled="savingDomainHome"
				@click="requestResetDashboard"
			>
				<RotateCounterClockwiseIcon />
			</button>
			<button
				v-if="editingScope !== 'domain' && canEditOwnHome"
				v-tooltip="
					formatMessage(
						editingActive
							? messages.doneEditing
							: !ymclStore.isPersonal
								? messages.editOwnHome
								: messages.customizeWidgets,
					)
				"
				data-onboarding-id="home-widget-customize"
				type="button"
				class="home-floating-action"
				:class="{ 'is-active': editingActive }"
				:aria-label="
					formatMessage(
						editingActive
							? messages.doneEditing
							: !ymclStore.isPersonal
								? messages.editOwnHome
								: messages.customizeWidgets,
					)
				"
				:aria-pressed="editingActive"
				:disabled="savingDomainHome"
				@click="toggleDashboardEditing"
			>
				<CheckIcon v-if="editingActive" />
				<PencilIcon v-else />
			</button>
			<button
				v-if="canManageDomainLayout && (!editingActive || editingScope === 'domain')"
				type="button"
				class="flex h-8 items-center gap-2 rounded-full border-0 px-3 text-sm font-medium transition-colors disabled:cursor-wait disabled:opacity-60"
				:class="
					editingScope === 'domain'
						? 'bg-brand text-accent-contrast hover:brightness-[var(--hover-brightness)]'
						: 'bg-transparent text-secondary hover:bg-surface-3 hover:text-contrast'
				"
				:disabled="savingDomainHome"
				:aria-busy="savingDomainHome"
				@click="
					editingScope === 'domain' ? finishDomainLayoutEditing() : startDomainLayoutEditing()
				"
			>
				<SpinnerIcon
					v-if="savingDomainHome && domainOperation?.kind !== 'lock'"
					class="size-4 animate-spin"
				/>
				<SaveIcon v-else-if="editingScope === 'domain'" class="size-4" />
				<PaintbrushIcon v-else class="size-4" />
				{{
					formatMessage(
						editingScope === 'domain' ? messages.publishDomainLayout : messages.editDomainLayout,
					)
				}}
			</button>
			<button
				v-if="editingScope === 'domain'"
				v-tooltip="formatMessage(messages.cancelDomainEditing)"
				type="button"
				class="home-floating-action"
				:aria-label="formatMessage(messages.cancelDomainEditing)"
				:disabled="savingDomainHome"
				@click="cancelHomeEditing"
			>
				<XIcon />
			</button>
			<button
				v-if="canManageDomainLayout && !editingActive"
				v-tooltip="
					formatMessage(
						domainDashboardLocked ? messages.unlockDomainLayout : messages.lockDomainLayout,
					)
				"
				type="button"
				class="home-floating-action"
				:class="{ 'is-active': domainDashboardLocked }"
				:aria-label="
					formatMessage(
						domainDashboardLocked ? messages.unlockDomainLayout : messages.lockDomainLayout,
					)
				"
				:aria-pressed="domainDashboardLocked"
				:disabled="savingDomainHome"
				@click="toggleDomainLayoutLock"
			>
				<SpinnerIcon v-if="domainOperation?.kind === 'lock'" class="animate-spin" />
				<LockIcon v-else-if="domainDashboardLocked" />
				<LockOpenIcon v-else />
			</button>
			<button
				v-if="editingActive"
				v-tooltip="
					formatMessage(
						isFreeWidgetLayout
							? messages.switchToGridWidgetLayout
							: messages.switchToFreeWidgetLayout,
					)
				"
				type="button"
				role="switch"
				class="home-layout-switch home-widget-layout-switch"
				:class="{ 'is-free': isFreeWidgetLayout }"
				:aria-checked="isFreeWidgetLayout"
				:aria-label="formatMessage(messages.widgetLayoutToggle)"
				:disabled="savingDomainHome"
				@click="toggleWidgetLayout"
			>
				<span class="home-layout-switch-option home-widget-layout-grid" aria-hidden="true">
					<GridIcon />
				</span>
				<span class="home-layout-switch-thumb" aria-hidden="true" />
				<span class="home-layout-switch-option home-widget-layout-free" aria-hidden="true">
					<MoveIcon />
				</span>
			</button>
			<span v-if="!showDomainLanding" class="home-floating-divider" aria-hidden="true" />
		</template>
		<button
			v-if="!showDomainLanding && !ymclStore.isEntranceHidden('home-customization')"
			v-tooltip="formatMessage(isMinimal ? messages.switchToInformation : messages.switchToMinimal)"
			data-onboarding-id="home-layout-switch"
			type="button"
			role="switch"
			class="home-layout-switch"
			:class="{ 'is-minimal': isMinimal }"
			:disabled="switchingLayout || savingDomainHome"
			:aria-checked="isMinimal"
			:aria-label="formatMessage(messages.homeLayoutToggle)"
			@click="toggleHomeLayout"
		>
			<span class="home-layout-switch-option home-layout-switch-information" aria-hidden="true">
				<LayoutTemplateIcon />
			</span>
			<span class="home-layout-switch-thumb" aria-hidden="true" />
			<span class="home-layout-switch-option home-layout-switch-minimal" aria-hidden="true">
				<MinimizeIcon />
			</span>
		</button>
	</div>
	<Teleport v-if="!isMinimal && !showDomainLanding" to="#sidebar-default-teleport-target">
		<div
			class="flex min-w-0 flex-col slide-enter-active"
			:class="{ 'slide-enter-from': !animateSidebarShow }"
		>
			<HomePlayInsights />
			<HomeDailyChallenge />
			<HomeMinecraftNews />
		</div>
	</Teleport>

	<ConfirmModal
		ref="resetConfirmModal"
		:title="formatMessage(messages.resetWidgets)"
		:description="formatMessage(messages.resetWidgetsConfirm)"
		:proceed-icon="RotateCounterClockwiseIcon"
		:proceed-label="formatMessage(messages.resetWidgetLayout)"
		:danger="false"
		@proceed="resetDashboardConfig"
	/>
</template>

<style scoped>
.home-floating-controls {
	position: fixed;
	z-index: 40;
	display: flex;
	height: 2.5rem;
	align-items: center;
	gap: 0.125rem;
	padding: 0.25rem;
	box-sizing: border-box;
	border: 1px solid var(--color-divider);
	border-radius: 9999px;
	background: var(--color-raised-bg);
	box-shadow:
		var(--shadow-button),
		0 0.25rem 0.75rem rgb(0 0 0 / 20%);
	isolation: isolate;
}

.home-floating-action {
	display: flex;
	width: 2rem;
	height: 2rem;
	align-items: center;
	justify-content: center;
	padding: 0;
	border: 0;
	border-radius: 9999px;
	background: transparent;
	color: var(--color-secondary);
	cursor: pointer;
	transition:
		background-color 120ms ease,
		color 120ms ease,
		filter 150ms ease,
		transform 150ms ease;
}

.home-floating-action:hover:not(:disabled) {
	background: var(--color-button-bg);
	color: var(--color-contrast);
}

.home-floating-action:active {
	transform: scale(0.96);
}

.home-floating-action:disabled {
	cursor: wait;
	opacity: 0.6;
}

.home-floating-action:focus-visible,
.home-layout-switch:focus-visible {
	outline: none;
	box-shadow: 0 0 0 4px var(--color-brand-shadow);
}

.home-floating-action.is-active {
	background: var(--color-brand);
	color: var(--color-accent-contrast);
}

.home-floating-action :deep(svg) {
	width: 1rem;
	height: 1rem;
}

.home-floating-divider {
	width: 1px;
	height: 1.25rem;
	margin: 0 0.125rem;
	background: var(--color-divider);
}

.home-layout-switch {
	position: relative;
	display: grid;
	grid-template-columns: repeat(2, 2rem);
	align-items: center;
	/* width: 4.25rem; */ /* closes #210 */
	height: 2rem;
	margin: 0;
	padding: 0;
	border: 0;
	border-radius: 9999px;
	background: var(--color-button-bg);
	cursor: pointer;
	isolation: isolate;
	transition:
		filter 150ms ease,
		transform 150ms ease;
}

.home-layout-switch:hover:not(:disabled) {
	filter: brightness(var(--hover-brightness));
}

.home-layout-switch:active:not(:disabled) {
	transform: scale(0.97);
}

.home-layout-switch:disabled {
	cursor: not-allowed;
	opacity: 0.6;
}

.home-layout-switch-thumb {
	position: absolute;
	top: 0.125rem;
	left: 0.125rem;
	z-index: 0;
	width: 1.75rem;
	height: 1.75rem;
	border-radius: 9999px;
	background: var(--color-brand);
	transition: transform 180ms ease;
}

.home-layout-switch.is-minimal .home-layout-switch-thumb {
	transform: translateX(2rem);
}

.home-widget-layout-switch.is-free .home-layout-switch-thumb {
	transform: translateX(2rem);
}

.home-layout-switch-option {
	position: relative;
	z-index: 1;
	display: flex;
	width: 2rem;
	height: 1.75rem;
	align-items: center;
	justify-content: center;
	color: var(--color-secondary);
	transition: color 180ms ease;
}

.home-layout-switch-option :deep(svg) {
	width: 1rem;
	height: 1rem;
}

.home-layout-switch:not(.is-minimal) .home-layout-switch-information,
.home-layout-switch.is-minimal .home-layout-switch-minimal,
.home-widget-layout-switch:not(.is-free) .home-widget-layout-grid,
.home-widget-layout-switch.is-free .home-widget-layout-free {
	color: var(--color-accent-contrast);
}
</style>
