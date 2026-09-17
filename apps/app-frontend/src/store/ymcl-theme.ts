import { defineStore } from 'pinia'
import { watch } from 'vue'

import { get } from '@/helpers/settings'
import { useTheming } from '@/store/state'
import {
	ACCENT_COLOR_OPTIONS,
	type AccentColorSetting,
	type ColorTheme,
	parseCustomAccentColor,
} from '@/store/theme'
import { useYmclStore } from '@/store/ymcl'

/**
 * Domain theme overlay (YAP §6.9): the active domain's ThemeProfile is composed
 * over the user's personal preferences — fields the domain configures become
 * the defaults in that domain, unset fields fall back to the personal theme.
 * While a domain profile is active the managed appearance set (color theme,
 * accent color, background, window transparency, advanced rendering, page
 * transitions) is rendered read-only in the settings; the personal domain
 * restores the user's own preferences. cssVars are gated by the launcher
 * whitelist.
 */

export interface YmclThemeProfile {
	schemaVersion?: number
	mode?: { default?: ColorTheme; allowUserOverride?: boolean }
	accentColor?: { value?: string; locked?: boolean }
	background?: { url?: string; blur?: number; opacity?: number; locked?: boolean }
	window?: { transparent?: boolean; opacity?: number; blur?: boolean; locked?: boolean }
	advancedRendering?: { value?: boolean; locked?: boolean }
	pageTransitions?: { value?: boolean; locked?: boolean }
	logoUrl?: string
	cssVars?: Record<string, string>
}

interface YmclThemeStoreState {
	profile: YmclThemeProfile | null
	/** Guards against overlapping async applications (e.g. rapid domain switches). */
	applySeq: number
}

/** Keys a domain may set; anything else is dropped (YAP §6.9). */
const CSS_VAR_WHITELIST = [
	'--color-brand',
	'--color-brand-2',
	'--color-button-bg',
	'--color-raised-bg',
	'--radius-sm',
	'--radius-md',
	'--radius-lg',
	'--radius-xl',
]

const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max)

/** Background blur is specified in pixels (0-40), matching the personal setting. */
function normalizeBlur(value: number | undefined, fallback: number): number {
	if (typeof value !== 'number' || Number.isNaN(value)) return fallback
	return Math.round(clamp(value, 0, 40))
}

/** Opacity accepts both the protocol's 0-1 fraction and a plain 0-100 percent. */
function normalizeOpacity(value: number | undefined, fallback: number): number {
	if (typeof value !== 'number' || Number.isNaN(value)) return fallback
	return Math.round(clamp(value > 0 && value <= 1 ? value * 100 : value, 0, 100))
}

/** Domain accents may arrive as preset names or raw hex; map hex to the custom form. */
function normalizeAccent(value: string | undefined): AccentColorSetting | null {
	if (!value) return null
	if (value === 'system') return 'system'
	if ((ACCENT_COLOR_OPTIONS as readonly string[]).includes(value)) {
		return value as AccentColorSetting
	}
	if (/^#[0-9a-fA-F]{6}$/.test(value)) return `custom:${value.toLowerCase()}`
	return parseCustomAccentColor(value)
}

/** Tolerates bare booleans where the protocol expects a `{ value }` node. */
function boolNode(node: boolean | { value?: boolean } | undefined): { value?: boolean } | null {
	if (typeof node === 'boolean') return { value: node }
	return node ?? null
}

function applyCssVars(profile: YmclThemeProfile | null) {
	const root = document.documentElement
	for (const key of CSS_VAR_WHITELIST) {
		root.style.removeProperty(key)
	}
	if (!profile?.cssVars) return
	for (const [key, value] of Object.entries(profile.cssVars)) {
		if (!CSS_VAR_WHITELIST.includes(key)) continue
		if (!/^#[0-9a-fA-F]{3,8}$/.test(value) && !/^\d+(?:\.\d+)?(?:px|rem|%)$/.test(value)) continue
		root.style.setProperty(key, value)
	}
}

export const useYmclThemeStore = defineStore('ymclThemeStore', {
	state: (): YmclThemeStoreState => ({
		profile: null,
		applySeq: 0,
	}),
	getters: {
		/**
		 * A non-personal domain that declares a theme profile manages the whole
		 * appearance set: the settings page renders it read-only until the user
		 * switches back to the personal domain (YAP §6.9).
		 */
		isAppearanceManaged: (state) => state.profile !== null,
		isLocked: (state) => (field: 'accentColor' | 'background' | 'window') =>
			state.profile?.[field]?.locked === true,
	},
	actions: {
		/**
		 * Applies the active domain's theme profile; a null profile (the personal
		 * domain, or a domain without a theme) restores the user's personal
		 * theme. Called on domain activation and on manifest updates.
		 */
		async applyProfile(profile: YmclThemeProfile | null) {
			const seq = ++this.applySeq
			this.profile = profile
			applyCssVars(profile)
			const themeStore = useTheming()
			// Reset to the persisted personal theme first so fields the profile
			// leaves unset fall back to the user's own preferences — this also
			// clears the previous domain's overlay when switching domains.
			const personal = await get()
			if (seq !== this.applySeq) return
			themeStore.setThemeState(personal.theme)
			const personalAccent = normalizeAccent(personal.accent_color)
			if (personalAccent) themeStore.setAccentColor(personalAccent)
			themeStore.customBackgroundPath = personal.custom_background_path
			themeStore.customBackgroundBlur = personal.custom_background_blur
			themeStore.customBackgroundOpacity = personal.custom_background_opacity
			themeStore.transparentBackground = personal.transparent_background
			themeStore.transparentBackgroundOpacity = personal.transparent_background_opacity
			themeStore.transparentBackgroundBlur = personal.transparent_background_blur
			themeStore.advancedRendering = personal.advanced_rendering
			themeStore.featureFlags.page_transitions = personal.feature_flags.page_transitions
			themeStore.setTransparentBackgroundClass()
			if (!profile) return

			if (profile.mode?.default) themeStore.setThemeState(profile.mode.default)
			const accent = normalizeAccent(profile.accentColor?.value)
			if (accent) themeStore.setAccentColor(accent)
			if (profile.background?.url) {
				themeStore.customBackgroundPath = profile.background.url
				themeStore.customBackgroundBlur = normalizeBlur(
					profile.background.blur,
					themeStore.customBackgroundBlur,
				)
				themeStore.customBackgroundOpacity = normalizeOpacity(
					profile.background.opacity,
					themeStore.customBackgroundOpacity,
				)
			}
			const windowNode = profile.window
			if (windowNode?.transparent !== undefined) {
				themeStore.transparentBackground = windowNode.transparent
				themeStore.transparentBackgroundOpacity = normalizeOpacity(
					windowNode.opacity,
					themeStore.transparentBackgroundOpacity,
				)
				if (typeof windowNode.blur === 'boolean') {
					themeStore.transparentBackgroundBlur = windowNode.blur
				}
				themeStore.setTransparentBackgroundClass()
			}
			const advancedRendering = boolNode(profile.advancedRendering)
			if (typeof advancedRendering?.value === 'boolean') {
				themeStore.advancedRendering = advancedRendering.value
			}
			const pageTransitions = boolNode(profile.pageTransitions)
			if (typeof pageTransitions?.value === 'boolean') {
				themeStore.featureFlags.page_transitions = pageTransitions.value
			}
		},
	},
})

/**
 * Applies the theme profile of the currently active domain. The initYmclTheme
 * watcher covers domain switches and manifest updates; App.vue also calls this
 * once after startup hydration, which would otherwise overwrite a profile
 * applied while hydration was still awaiting.
 */
export function reapplyYmclTheme() {
	const ymclStore = useYmclStore()
	const themeStore = useYmclThemeStore()
	const manifestTheme = ymclStore.manifest?.theme as YmclThemeProfile | undefined
	void themeStore.applyProfile(ymclStore.isPersonal ? null : (manifestTheme ?? null))
}

/** Wires domain activation to theme application; call once from App.vue. */
export function initYmclTheme() {
	const ymclStore = useYmclStore()
	watch(
		() => [ymclStore.activeDomainId, ymclStore.manifest] as const,
		() => reapplyYmclTheme(),
		{ immediate: true },
	)
}
