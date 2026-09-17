import { defineAsyncComponent, type Component } from 'vue'

export type AboutMemberExperience = {
	component: Component
	longPressDuration: number
}

const memberExperiences: Record<string, AboutMemberExperience> = {
	'axolotl-merge': {
		// Lazy so Settings > About can open without pulling the merge-game graph.
		component: defineAsyncComponent(() => import('../AboutMergeGame.vue')),
		longPressDuration: 800,
	},
}

export function getAboutMemberExperience(experience: unknown): AboutMemberExperience | undefined {
	return typeof experience === 'string' ? memberExperiences[experience] : undefined
}
