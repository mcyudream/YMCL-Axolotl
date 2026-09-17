<script setup lang="ts">
import {
	CheckIcon,
	GridIcon,
	LayoutTemplateIcon,
	MinimizeIcon,
	MoveIcon,
	PaintbrushIcon,
	PencilIcon,
	PlusIcon,
	RocketIcon,
	RotateCounterClockwiseIcon,
} from '@modrinth/assets'
import {
	ConfirmModal,
	defineMessages,
	injectNotificationManager,
	injectPageContext,
	useVIntl,
} from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { computed, onUnmounted, ref, useTemplateRef, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import {
	createDefaultHomeDashboard,
	createHomeDashboardSaveQueue,
	HOME_DASHBOARD_VERSION,
	type HomeDashboardConfig,
	normalizeHomeDashboard,
} from '@/components/home/home-dashboard'
import { getActivePlayerName } from '@/components/home/home-utils'
import HomeDailyChallenge from '@/components/home/HomeDailyChallenge.vue'
import HomeDashboard from '@/components/home/HomeDashboard.vue'
import HomeInstancePickerModal from '@/components/home/HomeInstancePickerModal.vue'
import HomeMinecraftNews from '@/components/home/HomeMinecraftNews.vue'
import HomeMinimal from '@/components/home/HomeMinimal.vue'
import HomePlayInsights from '@/components/home/HomePlayInsights.vue'
import { useNetworkStatus } from '@/composables/useNetworkStatus'
import { get_default_user, users } from '@/helpers/auth'
import { DIRECT_LINKS_SYNCED_EVENT } from '@/helpers/direct-link-sync'
import { instance_listener } from '@/helpers/events'
import { list } from '@/helpers/instance'
import { get as getSettings, set as setSettings } from '@/helpers/settings'
import type { GameInstance } from '@/helpers/types'
import { DOMAIN_DESIGN_PERMISSION } from '@/helpers/ymcl'
import { mapDashboardToHomeProfile, type YmclHomeProfile } from '@/helpers/ymcl-home'
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
const { offline } = useNetworkStatus()
const themeStore = useTheming()
const pageContext = injectPageContext()

const messages = defineMessages({
	home: { id: 'app.home.breadcrumb', defaultMessage: 'Home' },
	designDomainHome: {
		id: 'app.home.layout.design-domain',
		defaultMessage: 'Edit domain home',
	},
	publishDomainHome: {
		id: 'app.home.layout.publish-domain',
		defaultMessage: 'Publish domain layout',
	},
	editOwnHome: {
		id: 'app.home.layout.edit-own-home',
		defaultMessage: 'Customize your home',
	},
	configureDomainHome: {
		id: 'app.home.layout.configure-domain',
		defaultMessage: 'Configure domain home',
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
	domainHomePublished: {
		id: 'app.home.widgets.domain-published',
		defaultMessage: 'Domain home layout published',
	},
	personalHomeSaved: {
		id: 'app.home.widgets.personal-saved',
		defaultMessage: 'Your home layout was saved locally',
	},
	unmappedWidgetsSkipped: {
		id: 'app.home.widgets.unmapped-skipped',
		defaultMessage: '这些小组件没有对应的域卡片类型，未包含在发布：{widgets}',
	},
})

/** 不可发布小组件的可读名；文案复用小组件选择器的既有条目。 */
const SKIPPED_KIND_MESSAGES: Record<string, { id: string; defaultMessage: string }> = {
	greeting: { id: 'app.home.widgets.greeting', defaultMessage: '问候' },
	calendar: { id: 'app.home.widgets.calendar', defaultMessage: '日历' },
	'pinned-worlds': { id: 'app.home.widgets.pinned-worlds', defaultMessage: '固定的世界' },
	instance: { id: 'app.home.widgets.instance', defaultMessage: '单个实例入口' },
	world: { id: 'app.home.widgets.world', defaultMessage: '单个世界入口' },
	server: { id: 'app.home.widgets.server', defaultMessage: '单个服务器入口' },
}

function skippedKindLabel(kind: string): string {
	const message = SKIPPED_KIND_MESSAGES[kind]
	return message ? formatMessage(message) : kind
}

const recentProjectsInHomeFlag: FeatureFlag = 'worlds_in_home'

breadcrumbs.setRootContext({ name: formatMessage(messages.home), link: route.path })

const instances = ref<GameInstance[]>([])
const playerName = ref<string | null>(null)
const ymclStore = useYmclStore()
const domainDashboard = computed(() =>
	ymclStore.isPersonal ? null : ymclStore.domainHomeDashboard,
)
const domainDashboardLocked = computed(() => !ymclStore.isPersonal && ymclStore.domainHomeLocked)
/** Domain home designer entry (YAP §6.5): signed-in members of a domain. */
const canDesignDomainHome = computed(() => !ymclStore.isPersonal && !!ymclStore.session)
/** chrome/home writes need the adapter's design permission (RBAC-checked server-side). */
const hasDesignPermission = computed(() => {
	const permissions = ymclStore.session?.session.permissions ?? []
	// Host grants superadmins the "*" wildcard instead of enumerating codes.
	return permissions.includes('*') || permissions.includes(DOMAIN_DESIGN_PERMISSION)
})
/** Publishes layout changes to the whole domain (server-side chrome/home PUT). */
const canPublishDomainHome = computed(
	() => canDesignDomainHome.value && hasDesignPermission.value && domainDashboard.value !== null,
)
/** Unlocked domain homes allow members to customize their own view (YAP §6.5 用户覆盖). */
const canOverrideDomainHome = computed(
	() =>
		canDesignDomainHome.value && domainDashboard.value !== null && !domainDashboardLocked.value,
)
/** The domain hosts a renderable home layout: the pencil edits it in place. */
const canEditDomainHome = computed(
	() => canPublishDomainHome.value || canOverrideDomainHome.value,
)
/** No domain home yet: offer bootstrapping one from the current layout. */
const canBootstrapDomainHome = computed(
	() => canDesignDomainHome.value && hasDesignPermission.value && domainDashboard.value === null,
)
const editingActive = computed(() => dashboardEditing.value || domainEditing.value)
const domainEditing = ref(false)
/** true = publishing the domain layout to the server; false = saving a personal override. */
const domainEditingDesign = ref(true)
const domainEditingConfig = ref<HomeDashboardConfig | null>(null)
const savingDomainHome = ref(false)
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
const memberDashboard = computed(() =>
	domainDashboard.value ? applyHomeOverride(domainDashboard.value, domainOverride.value) : null,
)
const effectiveDashboard = computed(() => memberDashboard.value ?? dashboardConfig.value)
const renderedDashboard = computed(() =>
	domainEditing.value && domainEditingConfig.value
		? domainEditingConfig.value
		: effectiveDashboard.value,
)
const dashboardConfig = ref<HomeDashboardConfig | null>(null)
const dashboard = ref<InstanceType<typeof HomeDashboard>>()
const dashboardEditing = ref(false)
const instancePicker = ref<InstanceType<typeof HomeInstancePickerModal>>()
const isMinimal = computed(() => themeStore.homeLayout === 'minimal')
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

async function fetchPlayerName() {
	const selectedUser = await get_default_user(offline.value).catch(() => undefined)
	if (!selectedUser) return

	const accounts = await users(offline.value).catch(() => [])
	playerName.value = getActivePlayerName(selectedUser, accounts)
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

function requestResetDashboard() {
	resetConfirmModal.value?.show()
}

function resetDashboardConfig() {
	if (domainEditing.value) {
		if (!domainEditingDesign.value) {
			// 个人覆盖模式：清空本地覆盖，回到域发布的布局。
			if (overrideDomainId.value) {
				clearYmclHomeOverride(overrideDomainId.value)
				domainOverride.value = null
			}
			domainEditingConfig.value = cloneDashboard(
				domainDashboard.value ?? createDefaultHomeDashboard(),
			)
			return
		}
		// 域设计模式：丢弃当前草稿，从默认小组件重新开始。
		domainEditingConfig.value = cloneDashboard(createDefaultHomeDashboard())
		return
	}
	updateDashboardConfig(createDefaultHomeDashboard())
}

function onDashboardChange(config: HomeDashboardConfig) {
	if (domainEditing.value) {
		domainEditingConfig.value = config
		return
	}
	// 域布局的只读渲染不得回写个人配置：问候语设置等控件在非编辑态也会发
	// change，若放行会把域卡片写进 settings.home_widgets（个人主页被污染）。
	if (!ymclStore.isPersonal && !dashboardEditing.value) return
	updateDashboardConfig(config)
}

function cloneDashboard(config: HomeDashboardConfig): HomeDashboardConfig {
	return JSON.parse(JSON.stringify(config)) as HomeDashboardConfig
}

async function finishDomainHomeEditing() {
	const config = domainEditingConfig.value
	if (!config || savingDomainHome.value) return
	// 个人覆盖：只保存在本地，不写域（YAP §6.5 locked:false 用户覆盖）。
	if (!domainEditingDesign.value) {
		const domain = domainDashboard.value
		const domainId = overrideDomainId.value
		if (domain && domainId) {
			const removed = domain.widgets
				.map((widget) => widget.id)
				.filter((id) => !config.widgets.some((widget) => widget.id === id))
			saveYmclHomeOverride(domainId, {
				version: HOME_DASHBOARD_VERSION,
				layout: config.layout,
				widgets: config.widgets,
				removed,
			})
			domainOverride.value = loadYmclHomeOverride(domainId)
		}
		domainEditing.value = false
		addNotification({
			title: formatMessage(messages.personalHomeSaved),
			type: 'success',
		})
		return
	}
	savingDomainHome.value = true
	try {
		const original = (ymclStore.manifest?.home ?? null) as YmclHomeProfile | null
		const { profile, skippedKinds } = mapDashboardToHomeProfile(config, original)
		await invoke('plugin:ymcl|ymcl_chrome_home_put', { config: profile })
		domainEditing.value = false
		addNotification({
			title: formatMessage(messages.domainHomePublished),
			text: skippedKinds.length
				? formatMessage(messages.unmappedWidgetsSkipped, {
						widgets: skippedKinds.map((kind) => skippedKindLabel(kind)).join('、'),
					})
				: undefined,
			type: 'success',
		})
		// The adapter rebuilds its manifest on save; re-pull so the rendered
		// home reflects the published layout (YAP §6.10).
		await ymclStore.refreshManifest()
	} catch (error) {
		handleError(error)
	} finally {
		savingDomainHome.value = false
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
	const previousEditing = dashboardEditing.value
	switchingLayout.value = true
	themeStore.homeLayout = nextLayout
	if (nextLayout === 'minimal') {
		dashboardEditing.value = false
		domainEditing.value = false
	}

	try {
		const settings = await getSettings()
		settings.home_layout = nextLayout
		await setSettings(settings)
	} catch (error) {
		themeStore.homeLayout = previousLayout
		dashboardEditing.value = previousEditing
		handleError(error)
	} finally {
		switchingLayout.value = false
	}
}

function toggleDashboardEditing() {
	// The pencil personalizes the member's OWN view (YAP §6.5 用户覆盖); the
	// separate publish button (design permission) edits the domain layout.
	if (domainEditing.value) {
		void finishDomainHomeEditing()
		return
	}
	if (dashboardEditing.value) {
		dashboardEditing.value = false
		return
	}
	if (canOverrideDomainHome.value) {
		startDomainHomeEditing(false)
		return
	}
	dashboardEditing.value = true
}

function startDomainHomeEditing(design: boolean) {
	if (domainEditing.value) return
	domainEditingDesign.value = design
	const start = design
		? domainDashboard.value
		: (memberDashboard.value ?? domainDashboard.value)
	domainEditingConfig.value = cloneDashboard(start ?? createDefaultHomeDashboard())
	domainEditing.value = true
}

function startDomainHomeBootstrap() {
	if (domainEditing.value) return
	domainEditingDesign.value = true
	domainEditingConfig.value = cloneDashboard(createDefaultHomeDashboard())
	domainEditing.value = true
}

function toggleWidgetLayout() {
	dashboard.value?.setLayout(isFreeWidgetLayout.value ? 'grid' : 'free')
}

function openWidgetPicker() {
	dashboard.value?.openWidgetPicker()
}

const instancesLoaded = await fetchInstances()
if (!instancesLoaded || instances.value.length > 0) void fetchPlayerName()
await loadDashboardConfig()

window.addEventListener(DIRECT_LINKS_SYNCED_EVENT, fetchInstances)

const unlistenInstance = await instance_listener(async () => {
	await fetchInstances()
})

onUnmounted(() => {
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
	<div class="min-h-full">
		<HomeDashboard
			v-if="!isMinimal && effectiveDashboard"
			ref="dashboard"
			:config="renderedDashboard"
			:instances="instances"
			:player-name="playerName"
			:editing="editingActive"
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
	<div class="home-floating-controls" :style="floatingControlsStyle">
		<template v-if="!isMinimal && (!domainDashboardLocked || canEditDomainHome)">
			<button
				v-if="editingActive"
				v-tooltip="formatMessage(messages.addWidget)"
				type="button"
				class="home-floating-action"
				:aria-label="formatMessage(messages.addWidget)"
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
				@click="requestResetDashboard"
			>
				<RotateCounterClockwiseIcon />
			</button>
			<button
				v-if="editingActive || !domainDashboardLocked"
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
				@click="toggleDashboardEditing"
			>
				<CheckIcon v-if="editingActive" />
				<PencilIcon v-else />
			</button>
			<button
				v-if="canPublishDomainHome && !editingActive"
				v-tooltip="formatMessage(messages.publishDomainHome)"
				type="button"
				class="home-floating-action"
				:aria-label="formatMessage(messages.publishDomainHome)"
				@click="startDomainHomeEditing(true)"
			>
				<RocketIcon />
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
			<span class="home-floating-divider" aria-hidden="true" />
		</template>
		<button
			v-if="canBootstrapDomainHome && !editingActive"
			v-tooltip="formatMessage(messages.configureDomainHome)"
			type="button"
			class="home-floating-action"
			:aria-label="formatMessage(messages.configureDomainHome)"
			@click="startDomainHomeBootstrap"
		>
			<PaintbrushIcon />
		</button>
		<button
			v-tooltip="formatMessage(isMinimal ? messages.switchToInformation : messages.switchToMinimal)"
			data-onboarding-id="home-layout-switch"
			type="button"
			role="switch"
			class="home-layout-switch"
			:class="{ 'is-minimal': isMinimal }"
			:disabled="switchingLayout"
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
	<Teleport v-if="!isMinimal" to="#sidebar-default-teleport-target">
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

.home-floating-action:hover {
	background: var(--color-button-bg);
	color: var(--color-contrast);
}

.home-floating-action:active {
	transform: scale(0.96);
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
