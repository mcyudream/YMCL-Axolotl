import { defineStore } from 'pinia'
import { watch } from 'vue'

import { useTheming } from '@/store/state'
import { useYmclStore } from '@/store/ymcl'

/**
 * Domain theme overlay (YAP §6.9): the active domain's ThemeProfile is
 * composed with the user's personal preferences. `locked` fields override
 * the user; suggested fields only apply while the user has not chosen a
 * personal value. cssVars are gated by the launcher whitelist.
 */

export interface YmclThemeProfile {
	schemaVersion?: number
	mode?: { default?: 'dark' | 'light' | 'oled' | 'system'; allowUserOverride?: boolean }
	accentColor?: { value?: string; locked?: boolean }
	background?: { url?: string; blur?: number; opacity?: number; locked?: boolean }
	window?: { transparent?: boolean; locked?: boolean }
	logoUrl?: string
	cssVars?: Record<string, string>
}

interface YmclThemeStoreState {
	profile: YmclThemeProfile | null
	backgroundPath: string | null
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
		backgroundPath: null,
	}),
	getters: {
		isLocked: (state) => (field: 'accentColor' | 'background' | 'window') =>
			state.profile?.[field]?.locked === true,
	},
	actions: {
		/**
		 * Applies the active domain's theme profile; a null profile restores
		 * the user's personal theme. Called on domain activation.
		 */
		applyProfile(profile: YmclThemeProfile | null) {
			this.profile = profile
			applyCssVars(profile)
			const themeStore = useTheming()
			if (!profile) {
				// Restore personal preferences from persisted settings mirror
				themeStore.setThemeClass()
				return
			}
			const mode = profile.mode
			if (mode?.default && mode.allowUserOverride === false) {
				themeStore.setThemeState(mode.default)
			}
			const accent = profile.accentColor
			if (accent?.value && (accent.locked || themeStore.selectedAccentColor === 'system')) {
				themeStore.setAccentColor(accent.value)
			}
			const background = profile.background
			if (background?.url && (background.locked || !themeStore.customBackgroundPath)) {
				themeStore.customBackgroundPath = this.backgroundPath ?? background.url
				themeStore.customBackgroundBlur = background.blur ?? 0
				themeStore.customBackgroundOpacity = background.opacity ?? 1
			}
		},
	},
})

/** Wires domain activation to theme application; call once from App.vue. */
export function initYmclTheme() {
	const ymclStore = useYmclStore()
	const themeStore = useYmclThemeStore()
	watch(
		() => [ymclStore.activeDomainId, ymclStore.manifest] as const,
		() => {
			const manifestTheme = ymclStore.manifest?.theme as YmclThemeProfile | undefined
			themeStore.applyProfile(ymclStore.isPersonal ? null : (manifestTheme ?? null))
		},
		{ immediate: true },
	)
}
