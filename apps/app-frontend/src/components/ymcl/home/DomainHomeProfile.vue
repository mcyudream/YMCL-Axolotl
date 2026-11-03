<script setup lang="ts">
import { ChevronDownIcon, LogInIcon, SpinnerIcon, UsersIcon } from '@modrinth/assets'
import { Avatar, defineMessages, useVIntl } from '@modrinth/ui'
import { computed, inject, type Ref, ref } from 'vue'

import type AccountsCard from '@/components/ui/AccountsCard.vue'
import { useYmclStore } from '@/store/ymcl'

const accountsCard = inject<Ref<InstanceType<typeof AccountsCard> | null>>(
	'accountsCard',
	ref(null),
)
const store = useYmclStore()
const { formatMessage } = useVIntl()
const profile = computed(() => accountsCard.value?.selectedProfile)
const signedIn = computed(() => store.session?.domain_id === store.activeDomainId)
const messages = defineMessages({
	switchProfile: { id: 'minecraft-account.switch-profile', defaultMessage: 'Switch profile' },
	notSignedIn: { id: 'minecraft-account.not-signed-in', defaultMessage: 'Not signed in' },
	signIn: { id: 'minecraft-account.domain-sign-in', defaultMessage: 'Sign in to {domain}' },
})

function switchProfile() {
	if (signedIn.value) void accountsCard.value?.showProfileSwitch()
	else accountsCard.value?.showDomainLogin()
}
</script>

<template>
	<button
		v-if="accountsCard"
		type="button"
		class="flex max-w-full items-center gap-3 rounded-xl border border-solid border-white/15 bg-black/20 px-3 py-2.5 text-left text-white transition-colors hover:bg-white/15 disabled:cursor-not-allowed disabled:opacity-50"
		:disabled="accountsCard.profileSwitchDisabled"
		@click="switchProfile"
	>
		<Avatar
			v-if="profile"
			:src="profile.avatarUrl"
			size="36px"
			pixelated
			:unframed-natural-width="72"
		/>
		<UsersIcon v-else class="size-8 shrink-0 text-white/70" />
		<span class="flex min-w-0 flex-1 flex-col gap-1">
			<strong class="truncate text-sm font-semibold">
				{{ profile?.name || formatMessage(messages.notSignedIn) }}
			</strong>
			<span class="truncate text-xs text-white/65">
				{{
					signedIn
						? formatMessage(messages.switchProfile)
						: formatMessage(messages.signIn, { domain: store.activeDomain?.display_name ?? '' })
				}}
			</span>
		</span>
		<SpinnerIcon v-if="accountsCard.profileSwitchPending" class="size-4 shrink-0 animate-spin" />
		<ChevronDownIcon v-else-if="signedIn" class="size-4 shrink-0 text-white/65" />
		<LogInIcon v-else class="size-4 shrink-0 text-white/65" />
	</button>
</template>
