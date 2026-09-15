<template>
	<Dropdown v-model:shown="shown" placement="bottom-end" :triggers="['click']" :hide-triggers="['click']">
		<ButtonStyled type="transparent">
			<button
				v-tooltip="formatMessage(messages.switchDomain)"
				:aria-label="formatMessage(messages.switchDomain)"
				class="flex items-center gap-1.5"
			>
				<GlobeIcon />
				<span class="hidden max-w-32 truncate text-sm font-medium xl:inline">
					{{ activeLabel }}
				</span>
				<DropdownIcon class="size-3 text-secondary" />
			</button>
		</ButtonStyled>
		<template #popper>
			<div class="w-56 p-2">
				<div class="mb-1 px-2 text-xs font-semibold text-secondary">
					{{ formatMessage(messages.title) }}
				</div>
				<button
					v-for="domain in ymclStore.domains"
					:key="domain.id"
					class="flex w-full items-center gap-2 rounded-lg p-2 text-left hover:bg-button-bg"
					@click="activate(domain)"
				>
					<img
						v-if="domain.logo_url"
						:src="domain.logo_url"
						:alt="domain.display_name"
						class="size-5 shrink-0 rounded object-contain"
					/>
					<GlobeIcon v-else class="size-5 shrink-0 text-secondary" />
					<span class="min-w-0 flex-1 truncate text-sm text-contrast">
						{{ domain.is_personal ? formatMessage(messages.personalDomain) : domain.display_name }}
					</span>
					<CheckIcon v-if="domain.is_active" class="size-4 shrink-0 text-brand" />
				</button>
				<hr class="my-1 border-surface-5" />
				<button
					class="flex w-full items-center gap-2 rounded-lg p-2 text-left text-sm text-secondary hover:bg-button-bg hover:text-contrast"
					@click="goToSettings"
				>
					<SettingsIcon class="size-4 shrink-0" />
					{{ formatMessage(messages.manageDomains) }}
				</button>
			</div>
		</template>
	</Dropdown>
</template>

<script setup lang="ts">
import { CheckIcon, DropdownIcon, GlobeIcon, SettingsIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, Dropdown, useVIntl } from '@modrinth/ui'
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'

import type { YmclDomainSummary } from '@/helpers/ymcl'
import { useYmclStore } from '@/store/ymcl'

const { formatMessage } = useVIntl()
const ymclStore = useYmclStore()
const router = useRouter()

const shown = ref(false)

const messages = defineMessages({
	switchDomain: {
		id: 'app.ymcl.switch-domain',
		defaultMessage: '切换域',
	},
	title: {
		id: 'app.ymcl.switch-domain.title',
		defaultMessage: '当前域',
	},
	personalDomain: {
		id: 'app.ymcl.switch-domain.personal',
		defaultMessage: '个人域',
	},
	manageDomains: {
		id: 'app.ymcl.switch-domain.manage',
		defaultMessage: '管理域…',
	},
})

const activeLabel = computed(() => {
	if (ymclStore.isPersonal) return formatMessage(messages.personalDomain)
	return ymclStore.activeDomain?.display_name ?? ''
})

async function activate(domain: YmclDomainSummary) {
	shown.value = false
	try {
		await ymclStore.activateDomain(domain.id)
	} catch (error) {
		console.error('Failed to activate domain', error)
	}
}

function goToSettings() {
	shown.value = false
	router.push('/settings#ymcl-domains')
}

</script>
