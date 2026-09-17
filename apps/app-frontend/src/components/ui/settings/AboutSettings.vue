<script setup lang="ts">
import {
	ChevronDownIcon,
	ExternalIcon,
	GithubIcon,
	HeartHandshakeIcon,
	IssuesIcon,
	ScaleIcon,
	UsersIcon,
} from '@modrinth/assets'
import { Avatar, defineMessages, NewButton as Button, useVIntl } from '@modrinth/ui'
import { getVersion } from '@tauri-apps/api/app'
import { defineAsyncComponent, inject, nextTick, onScopeDispose, ref, shallowRef } from 'vue'

import aboutBanner from '@/assets/about/yudream-launcher-banner.jpg'
import { AxolotlBrandConfig } from '@/config'
import { contributors, type TeamMember, teamMembers } from '@/data/about'

import { type AboutMemberExperience, getAboutMemberExperience } from './about-member-experiences'

const EasterEggGameModal = defineAsyncComponent(
	() => import('@/components/ui/easteregg/EasterEggGameModal.vue'),
)
const EasterEggContributorsModal = defineAsyncComponent(
	() => import('@/components/ui/easteregg/EasterEggContributorsModal.vue'),
)

const { formatMessage } = useVIntl()
// Top-level await + dynamic imports previously broke HMR after ymcl edits;
// keep setup synchronous-friendly so Settings > About stays enterable.
const version = ref('')
const experienceHost = ref<HTMLElement>()
const activeMemberExperience = shallowRef<AboutMemberExperience>()
const pressingMemberName = ref<string>()
let longPressTimer: ReturnType<typeof window.setTimeout> | undefined
let pressStart = { x: 0, y: 0 }
let suppressNextMemberClick = false
const replayOnboarding = inject<(mode: 'main' | 'instance') => Promise<void>>('replayOnboarding')

void getVersion()
	.then((value) => {
		version.value = value
	})
	.catch(() => {
		version.value = ''
	})

const licenseUrl = `${AxolotlBrandConfig.repositoryUrl}/blob/main/LICENSE`
const copyingUrl = `${AxolotlBrandConfig.repositoryUrl}/blob/main/COPYING.md`
const thirdPartyLicensesUrl = `${AxolotlBrandConfig.repositoryUrl}/tree/main/third-party/licenses`

function cancelMemberLongPress() {
	if (longPressTimer) window.clearTimeout(longPressTimer)
	longPressTimer = undefined
	pressingMemberName.value = undefined
}

function startMemberLongPress(member: TeamMember, event: PointerEvent) {
	const experience = getAboutMemberExperience(member.experience)
	if (!experience || event.button !== 0) return

	cancelMemberLongPress()
	pressStart = { x: event.clientX, y: event.clientY }
	pressingMemberName.value = member.name
	longPressTimer = window.setTimeout(async () => {
		activeMemberExperience.value = experience
		suppressNextMemberClick = true
		cancelMemberLongPress()
		await nextTick()
		experienceHost.value?.scrollIntoView({ behavior: 'smooth', block: 'center' })
	}, experience.longPressDuration)
}

function moveMemberLongPress(event: PointerEvent) {
	if (!longPressTimer) return
	if (Math.hypot(event.clientX - pressStart.x, event.clientY - pressStart.y) > 8) {
		cancelMemberLongPress()
	}
}

function handleMemberClick(event: MouseEvent) {
	if (!suppressNextMemberClick) return
	suppressNextMemberClick = false
	event.preventDefault()
	event.stopPropagation()
}

function handleMemberContextMenu(member: TeamMember, event: MouseEvent) {
	if (getAboutMemberExperience(member.experience)) event.preventDefault()
}

function closeMemberExperience() {
	activeMemberExperience.value = undefined
}

const gameModal = ref<{ show: () => void } | null>(null)
const contributorsModal = ref<{ show: () => void } | null>(null)

let typedBuffer = ''
const secretCodes = ['cyf112233', 'cxkcxkckx']

const konamiSequence = [
	'ArrowUp',
	'ArrowUp',
	'ArrowDown',
	'ArrowDown',
	'ArrowLeft',
	'ArrowRight',
	'ArrowLeft',
	'ArrowRight',
	'KeyB',
	'KeyA',
]
let konamiIndex = 0

function handleEasterEggKeydown(event: KeyboardEvent) {
	typedBuffer = (typedBuffer + event.key).toLowerCase()
	const maxCodeLen = Math.max(...secretCodes.map((c) => c.length))
	if (typedBuffer.length > maxCodeLen) {
		typedBuffer = typedBuffer.slice(-maxCodeLen)
	}
	if (secretCodes.some((code) => typedBuffer.endsWith(code))) {
		typedBuffer = ''
		gameModal.value?.show()
		return
	}

	const expected = konamiSequence[konamiIndex]
	if (event.code === expected) {
		konamiIndex++
		if (konamiIndex === konamiSequence.length) {
			konamiIndex = 0
			contributorsModal.value?.show()
		}
	} else {
		konamiIndex = event.code === konamiSequence[0] ? 1 : 0
	}
}

function onEasterEggOpenGame() {
	gameModal.value?.show()
}

document.addEventListener('keydown', handleEasterEggKeydown)
onScopeDispose(() => document.removeEventListener('keydown', handleEasterEggKeydown))

onScopeDispose(cancelMemberLongPress)

const messages = defineMessages({
	productTitle: {
		id: 'app.settings.about.product-title',
		defaultMessage: 'About {productName}',
	},
	productDescription: {
		id: 'app.settings.about.description',
		defaultMessage: 'A launcher designed for B2B, powered by YuDream Admin Skin.',
	},
	version: {
		id: 'app.settings.about.version',
		defaultMessage: 'Version {version}',
	},
	replayOnboarding: {
		id: 'app.settings.about.replay-onboarding',
		defaultMessage: 'Replay tour',
	},
	developmentTeam: {
		id: 'app.settings.about.development-team',
		defaultMessage: 'Development team',
	},
	communitySupport: {
		id: 'app.settings.about.community-support',
		defaultMessage: 'Project & community',
	},
	repository: {
		id: 'app.settings.about.repository',
		defaultMessage: 'Source code',
	},
	reportIssue: {
		id: 'app.settings.about.report-issue',
		defaultMessage: 'Issues & feedback',
	},
	licenseAttribution: {
		id: 'app.settings.about.license-attribution',
		defaultMessage: 'License & attribution',
	},
	attribution: {
		id: 'app.settings.about.attribution',
		defaultMessage:
			'YMCL (YuDream Launcher) is an open-source Minecraft launcher secondary-developed from Axolotl Launcher.',
	},
	basedOnAxolotl: {
		id: 'app.settings.about.based-on-axolotl',
		defaultMessage: 'Based on secondary development of Axolotl',
	},
	notAffiliated: {
		id: 'app.settings.about.not-affiliated',
		defaultMessage:
			'Modrinth is a trademark of Rinth, Inc. YMCL (YuDream Launcher) is not affiliated with or endorsed by Rinth, Inc.',
	},
	originalSource: {
		id: 'app.settings.about.original-source',
		defaultMessage: 'Original Modrinth source',
	},
	projectLicense: {
		id: 'app.settings.about.project-license',
		defaultMessage: 'Project license (GPL-3.0)',
	},
	copyingGuidelines: {
		id: 'app.settings.about.copying-guidelines',
		defaultMessage: 'Copying guidelines',
	},
	thirdPartyLicenses: {
		id: 'app.settings.about.third-party-licenses',
		defaultMessage: 'Third-party licenses',
	},
	contributors: {
		id: 'app.settings.about.contributors',
		defaultMessage: 'Contributors',
	},
	contributorsCount: {
		id: 'app.settings.about.contributors-count',
		defaultMessage: '{count, plural, one {# contributor} other {# contributors}}',
	},
})

const projectLinks = [
	{
		href: AxolotlBrandConfig.repositoryUrl,
		label: messages.repository,
		icon: GithubIcon,
	},
	{
		href: AxolotlBrandConfig.supportUrl,
		label: messages.reportIssue,
		icon: IssuesIcon,
	},
]
</script>

<template>
	<div class="about-page flex flex-col gap-6">
		<section id="settings-target-about-product" tabindex="-1" class="about-panel">
			<div class="flex flex-col items-center gap-4">
				<div
					ref="experienceHost"
					class="relative m-0 w-full overflow-hidden h-64 rounded-xl"
					style="
						mask-image: linear-gradient(to bottom, black 97%, transparent 100%);
						-webkit-mask-image: linear-gradient(to bottom, black 97%, transparent 100%);
					"
				>
					<img
						v-if="!activeMemberExperience"
						class="size-full object-cover object-center"
						:src="aboutBanner"
						:alt="
							formatMessage(messages.productTitle, {
								productName: AxolotlBrandConfig.productName,
							})
						"
					/>
					<component
						:is="activeMemberExperience?.component"
						v-if="activeMemberExperience"
						@exit="closeMemberExperience"
					/>
				</div>
				<div class="min-w-0 text-center">
					<h2 class="m-0 text-xl font-semibold text-contrast">
						{{
							formatMessage(messages.productTitle, {
								productName: AxolotlBrandConfig.productName,
							})
						}}
					</h2>
					<p class="m-0 mt-1 text-secondary">
						{{ formatMessage(messages.version, { version }) }}
					</p>
				</div>
			</div>
			<p class="m-0 mt-3 text-center text-primary">
				{{ formatMessage(messages.productDescription) }}
			</p>
		</section>

		<section>
			<h3 class="m-0 mb-3 flex items-center gap-2 text-base font-semibold text-contrast">
				<UsersIcon class="size-5 text-secondary" />
				{{ formatMessage(messages.developmentTeam) }}
			</h3>
			<ul class="m-0 grid list-none grid-cols-2 gap-3 p-0 sm:grid-cols-3">
				<li v-for="member in teamMembers" :key="member.name" class="min-w-0">
					<component
						:is="member.url ? 'a' : 'div'"
						:href="member.url"
						:target="member.url ? '_blank' : undefined"
						:rel="member.url ? 'noopener noreferrer' : undefined"
						class="flex min-w-0 select-none flex-col items-center gap-3 rounded-xl bg-surface-4 p-4"
						:class="[
							member.url ? 'transition-colors hover:bg-surface-5' : 'cursor-default',
							pressingMemberName === member.name ? 'ring-4 ring-brand-shadow' : '',
						]"
						@pointerdown="startMemberLongPress(member, $event)"
						@pointermove="moveMemberLongPress"
						@pointerup="cancelMemberLongPress"
						@pointercancel="cancelMemberLongPress"
						@dragstart="cancelMemberLongPress"
						@click="handleMemberClick"
						@contextmenu="handleMemberContextMenu(member, $event)"
					>
					<Avatar
						v-if="member.avatarUrl"
						:src="member.avatarUrl"
						:alt="member.name"
						size="4rem"
						circle
						no-shadow
					/>
					<div
						v-else
						class="flex size-16 items-center justify-center rounded-full bg-surface-3 text-lg font-bold text-secondary"
						aria-hidden="true"
					>
						{{ member.name.slice(0, 1) }}
					</div>
						<span class="block truncate text-center font-semibold text-contrast">{{
							member.name
						}}</span>
					</component>
				</li>
			</ul>
		</section>

		<section>
			<h3 class="m-0 mb-3 flex items-center gap-2 text-base font-semibold text-contrast">
				<HeartHandshakeIcon class="size-5 text-secondary" />
				{{ formatMessage(messages.communitySupport) }}
			</h3>
			<div class="grid gap-3 sm:grid-cols-2">
				<a
					v-for="link in projectLinks"
					:key="link.label?.id ?? link.href"
					:href="link.href"
					target="_blank"
					rel="noopener noreferrer"
					class="flex min-w-0 items-center gap-3 rounded-xl bg-surface-4 p-4 transition-colors hover:bg-surface-5"
				>
					<span
						class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-surface-2 text-contrast"
					>
						<component :is="link.icon" class="size-6" />
					</span>
					<span class="min-w-0 flex-1 font-semibold text-contrast">
						{{ formatMessage(link.label) }}
					</span>
					<ExternalIcon class="size-5 shrink-0 text-secondary" />
				</a>
			</div>
		</section>

		<section>
			<h3 class="m-0 mb-3 flex items-center gap-2 text-base font-semibold text-contrast">
				<ScaleIcon class="size-5 text-secondary" />
				{{ formatMessage(messages.licenseAttribution) }}
			</h3>
			<div class="about-panel about-panel-compact">
				<p class="m-0 text-primary">
					{{ formatMessage(messages.attribution) }}
				</p>
				<p class="m-0 mt-2 text-sm text-secondary">
					{{ formatMessage(messages.basedOnAxolotl) }}
				</p>
				<a
					:href="AxolotlBrandConfig.repositoryUrl"
					target="_blank"
					rel="noopener noreferrer"
					class="mt-2 inline-flex min-w-0 items-center gap-2 text-sm font-semibold text-brand hover:underline"
				>
					<GithubIcon class="size-4 shrink-0" />
					<span class="truncate">{{ AxolotlBrandConfig.repositoryUrl }}</span>
				</a>
				<p class="m-0 mt-2 text-sm text-secondary">
					{{ formatMessage(messages.notAffiliated) }}
				</p>
			</div>
			<div class="mt-3 flex flex-wrap gap-2">
				<a
					:href="AxolotlBrandConfig.repositoryUrl"
					target="_blank"
					rel="noopener noreferrer"
					class="inline-flex items-center gap-2 rounded-lg bg-surface-4 px-3 py-2 text-sm font-semibold text-contrast transition-colors hover:bg-surface-5"
				>
					<GithubIcon class="size-4 text-secondary" />
					{{ formatMessage(messages.basedOnAxolotl) }}
					<ExternalIcon class="size-4 text-secondary" />
				</a>
				<a
					:href="licenseUrl"
					target="_blank"
					rel="noopener noreferrer"
					class="inline-flex items-center gap-2 rounded-lg bg-surface-4 px-3 py-2 text-sm font-semibold text-contrast transition-colors hover:bg-surface-5"
				>
					{{ formatMessage(messages.projectLicense) }}
					<ExternalIcon class="size-4 text-secondary" />
				</a>
				<a
					:href="copyingUrl"
					target="_blank"
					rel="noopener noreferrer"
					class="inline-flex items-center gap-2 rounded-lg bg-surface-4 px-3 py-2 text-sm font-semibold text-contrast transition-colors hover:bg-surface-5"
				>
					{{ formatMessage(messages.copyingGuidelines) }}
					<ExternalIcon class="size-4 text-secondary" />
				</a>
				<a
					:href="thirdPartyLicensesUrl"
					target="_blank"
					rel="noopener noreferrer"
					class="inline-flex items-center gap-2 rounded-lg bg-surface-4 px-3 py-2 text-sm font-semibold text-contrast transition-colors hover:bg-surface-5"
				>
					{{ formatMessage(messages.thirdPartyLicenses) }}
					<ExternalIcon class="size-4 text-secondary" />
				</a>
				<a
					href="https://github.com/modrinth/code"
					target="_blank"
					rel="noopener noreferrer"
					class="inline-flex items-center gap-2 rounded-lg bg-surface-4 px-3 py-2 text-sm font-semibold text-contrast transition-colors hover:bg-surface-5"
				>
					{{ formatMessage(messages.originalSource) }}
					<ExternalIcon class="size-4 text-secondary" />
				</a>
			</div>
		</section>

		<details class="group pt-4 about-settings-details">
			<summary
				class="flex cursor-pointer list-none items-center gap-2 text-base font-semibold text-contrast [&::-webkit-details-marker]:hidden"
			>
				<UsersIcon class="size-5 text-secondary" />
				<span>{{ formatMessage(messages.contributors) }}</span>
				<span class="rounded-full bg-surface-4 px-2 py-0.5 text-xs text-secondary">
					{{ formatMessage(messages.contributorsCount, { count: contributors.length }) }}
				</span>
				<ChevronDownIcon
					class="ml-auto size-5 text-secondary transition-transform group-open:rotate-180"
				/>
			</summary>
			<div class="mt-3 flex flex-wrap gap-2">
				<a
					v-for="contributor in contributors"
					:key="contributor.name"
					:href="contributor.url"
					target="_blank"
					rel="noopener noreferrer"
					class="flex min-w-0 items-center gap-1.5 rounded-full bg-surface-4 py-1 pl-1 pr-2.5 transition-colors hover:bg-surface-5"
				>
					<Avatar
						:src="contributor.avatarUrl"
						:alt="contributor.name"
						size="1.5rem"
						circle
						no-shadow
						loading="lazy"
					/>
					<span class="truncate text-sm text-primary">{{ contributor.name }}</span>
				</a>
			</div>
		</details>

		<div id="settings-target-about-replay-tour" tabindex="-1" class="flex flex-wrap gap-2">
			<Button type="base" @click="replayOnboarding?.('main')">
				{{ formatMessage(messages.replayOnboarding) }}
			</Button>
		</div>
	</div>

	<EasterEggGameModal ref="gameModal" />
	<EasterEggContributorsModal ref="contributorsModal" @open-game="onEasterEggOpenGame" />
</template>

<style scoped>
.about-settings-details {
	border-top: 1px solid
		var(--settings-divider, color-mix(in srgb, var(--surface-4) 55%, transparent));
}

.about-panel {
	padding: 1.25rem;
	border: 1px solid
		var(--settings-card-border, color-mix(in srgb, var(--surface-4) 72%, transparent));
	border-radius: var(--radius-md);
	background: var(--surface-2);
}

.about-panel-compact {
	padding: var(--gap-lg);
}

.about-page :deep(.rounded-xl.bg-surface-4) {
	border: 1px solid
		var(--settings-card-border, color-mix(in srgb, var(--surface-4) 72%, transparent));
	border-radius: var(--radius-md);
	background: var(--surface-2);
}

.about-page :deep(.rounded-xl.bg-surface-2) {
	border-radius: var(--radius-sm);
}
</style>
