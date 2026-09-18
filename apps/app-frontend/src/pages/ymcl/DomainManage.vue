<script setup lang="ts">
import {
	GlobeIcon,
	LoaderIcon,
	LockIcon,
	LogOutIcon,
	PlusIcon,
	RefreshCwIcon,
	TrashIcon,
	UserIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	Combobox,
	defineMessages,
	injectNotificationManager,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'

import AddDomainModal from '@/components/ymcl/AddDomainModal.vue'
import DomainImg from '@/components/ymcl/DomainImg.vue'
import {
	PERSONAL_DOMAIN_ID,
	type YmclDomainSummary,
	type YmclExternalProvider,
} from '@/helpers/ymcl'
import { useYmclStore } from '@/store/ymcl'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const ymclStore = useYmclStore()

const addDomainModal = ref<InstanceType<typeof AddDomainModal>>()
const username = ref('')
const password = ref('')
const externalProviders = ref<YmclExternalProvider[] | null>(null)
const showLogin = ref(false)

const route = useRoute()
// ymcl://add-site?url={origin} lands here via /settings?add_site={origin}#ymcl-domains
// Watching the modal ref too: on a cold start the query is already present
// when this component's setup runs, so the immediate callback fires before
// the template ref binds and `show` would otherwise be skipped silently.
watch(
	[() => route.query.add_site, addDomainModal],
	([url, modal]) => {
		if (modal && typeof url === 'string' && url.trim()) {
			modal.show(url.trim())
		}
	},
	{ immediate: true },
)

const messages = defineMessages({
	title: {
		id: 'app.settings.ymcl-domains.title',
		defaultMessage: '域',
	},
	description: {
		id: 'app.settings.ymcl-domains.description',
		defaultMessage:
			'Join launcher domains (a YuDream Admin deployment). The active domain controls navigation, pages and theming; the personal domain is the launcher without any domain features.',
	},
	addLabel: {
		id: 'app.settings.ymcl-domains.add',
		defaultMessage: '添加域',
	},
	personalDomain: {
		id: 'app.settings.ymcl-domains.personal',
		defaultMessage: '个人域',
	},
	activate: {
		id: 'app.settings.ymcl-domains.activate',
		defaultMessage: '使用此域',
	},
	activeLabel: {
		id: 'app.settings.ymcl-domains.active',
		defaultMessage: '使用中',
	},
	remove: {
		id: 'app.settings.ymcl-domains.remove',
		defaultMessage: 'Remove',
	},
	refreshManifest: {
		id: 'app.settings.ymcl-domains.refresh-manifest',
		defaultMessage: '刷新清单',
	},
	noDomains: {
		id: 'app.settings.ymcl-domains.none',
		defaultMessage: '还没有加入任何域。点击“添加域”输入域地址即可开始。',
	},
	adapterVersion: {
		id: 'app.settings.ymcl-domains.adapter-version',
		defaultMessage: '适配器版本',
	},
	protocolVersion: {
		id: 'app.settings.ymcl-domains.protocol-version',
		defaultMessage: '协议版本',
	},
	account: {
		id: 'app.settings.ymcl-domains.account',
		defaultMessage: '账号',
	},
	signInHeading: {
		id: 'app.settings.ymcl-domains.sign-in',
		defaultMessage: '登录此域',
	},
	usernameLabel: {
		id: 'app.settings.ymcl-domains.username',
		defaultMessage: '用户名或邮箱',
	},
	passwordLabel: {
		id: 'app.settings.ymcl-domains.password',
		defaultMessage: '密码',
	},
	signIn: {
		id: 'app.settings.ymcl-domains.sign-in-button',
		defaultMessage: '登录',
	},
	oauthSignIn: {
		id: 'app.settings.ymcl-domains.oauth-sign-in',
		defaultMessage: '使用浏览器登录',
	},
	signOut: {
		id: 'app.settings.ymcl-domains.sign-out',
		defaultMessage: '退出登录',
	},
	department: {
		id: 'app.settings.ymcl-domains.department',
		defaultMessage: '部门',
	},
	role: {
		id: 'app.settings.ymcl-domains.role',
		defaultMessage: '角色',
	},
	noLoginMethods: {
		id: 'app.settings.ymcl-domains.no-login-methods',
		defaultMessage: '此域无需登录账号。',
	},
	loginPending: {
		id: 'app.settings.ymcl-domains.login-pending',
		defaultMessage: '请在浏览器中完成登录…',
	},
})

const authMethods = computed(() => ymclStore.activeDomain?.capabilities?.auth?.methods ?? [])
const hasPassword = computed(() => authMethods.value.some((method) => method.type === 'password'))
const hasOauth = computed(() => authMethods.value.some((method) => method.type === 'oauth-web'))
const showAccountSection = computed(() => !ymclStore.isPersonal && ymclStore.activeDomain != null)
const sessionContext = computed(() => ymclStore.sessionContext)

const departmentOptions = computed(() =>
	(sessionContext.value?.available_depts ?? []).map((dept) => ({
		value: dept.id,
		label: dept.name,
	})),
)
const roleOptions = computed(() =>
	(sessionContext.value?.available_roles ?? []).map((role) => ({
		value: role.id,
		label: role.name,
	})),
)

async function loadExternalProviders() {
	if (externalProviders.value) return
	try {
		externalProviders.value = await ymcl.externalProviders(ymclStore.activeDomainId)
	} catch {
		externalProviders.value = []
	}
}

watch(showLogin, (visible) => {
	if (visible) void loadExternalProviders()
})

async function activate(domain: YmclDomainSummary) {
	try {
		await ymclStore.activateDomain(domain.id)
	} catch (error) {
		handleError(error)
	}
}

async function remove(domain: YmclDomainSummary) {
	try {
		await ymclStore.removeDomain(domain.id)
	} catch (error) {
		handleError(error)
	}
}

async function refreshManifest() {
	try {
		await ymclStore.refreshManifest()
	} catch (error) {
		handleError(error)
	}
}

async function signInWithPassword() {
	if (!username.value || !password.value) return
	try {
		await ymclStore.passwordLogin(username.value, password.value)
		password.value = ''
	} catch (error) {
		handleError(error)
	}
}

async function signInWithOauth() {
	try {
		await ymclStore.oauthLogin()
	} catch (error) {
		handleError(error)
	}
}

async function signInWithExternal(provider: string) {
	try {
		await ymclStore.externalLogin(provider)
	} catch (error) {
		handleError(error)
	}
}

async function signOut() {
	try {
		await ymclStore.logout()
	} catch (error) {
		handleError(error)
	}
}

async function switchDepartment(deptId: string) {
	try {
		await ymclStore.switchDepartment(deptId)
	} catch (error) {
		handleError(error)
	}
}

async function switchRole(roleId: string) {
	try {
		await ymclStore.switchRole(roleId)
	} catch (error) {
		handleError(error)
	}
}
</script>

<template>
	<div class="flex flex-col gap-4">
		<h2 id="settings-target-ymcl-domains" class="m-0 text-lg font-semibold text-contrast">
			{{ formatMessage(messages.title) }}
		</h2>
		<p class="m-0 text-sm leading-relaxed text-secondary">
			{{ formatMessage(messages.description) }}
		</p>

		<div class="flex">
			<ButtonStyled>
				<button :disabled="ymclStore.adding" @click="addDomainModal?.show()">
					<PlusIcon />
					{{ formatMessage(messages.addLabel) }}
				</button>
			</ButtonStyled>
		</div>

		<div class="flex flex-col gap-2">
			<div
				class="flex items-center gap-3 rounded-xl border border-solid border-surface-5 bg-bg-raised p-3"
			>
				<GlobeIcon class="h-8 w-8 shrink-0 text-secondary" />
				<div class="min-w-0 flex-1">
					<div class="font-semibold text-contrast">
						{{ formatMessage(messages.personalDomain) }}
					</div>
				</div>
				<ButtonStyled v-if="!ymclStore.isPersonal" type="standard">
					<button @click="activate({ id: PERSONAL_DOMAIN_ID } as YmclDomainSummary)">
						{{ formatMessage(messages.activate) }}
					</button>
				</ButtonStyled>
				<span
					v-else
					class="rounded-full bg-brand px-2 py-0.5 text-xs font-semibold text-brand-inverted-on/bg-brand"
				>
					{{ formatMessage(messages.activeLabel) }}
				</span>
			</div>

			<div
				v-for="domain in ymclStore.domains.filter((d) => !d.is_personal)"
				:key="domain.id"
				class="flex items-center gap-3 rounded-xl border border-solid border-surface-5 bg-bg-raised p-3"
			>
				<DomainImg
					:src="domain.logo_url"
					:origin="domain.origin"
					:alt="domain.display_name"
					class="h-8 w-8 shrink-0 rounded-lg object-contain"
				>
					<template #fallback>
						<GlobeIcon class="h-8 w-8 shrink-0 text-secondary" />
					</template>
				</DomainImg>
				<div class="min-w-0 flex-1">
					<div class="truncate font-semibold text-contrast">{{ domain.display_name }}</div>
					<div class="truncate text-xs text-secondary">{{ domain.origin }}</div>
				</div>
				<div class="flex shrink-0 items-center gap-2">
					<span
						v-if="domain.is_active"
						class="rounded-full bg-brand px-2 py-0.5 text-xs font-semibold text-brand-inverted-on/bg-brand"
					>
						{{ formatMessage(messages.activeLabel) }}
					</span>
					<template v-else>
						<ButtonStyled type="standard">
							<button @click="activate(domain)">
								{{ formatMessage(messages.activate) }}
							</button>
						</ButtonStyled>
						<ButtonStyled type="danger" circular>
							<button
								v-tooltip="formatMessage(messages.remove)"
								:disabled="ymclStore.loadingManifest"
								@click="remove(domain)"
							>
								<TrashIcon />
							</button>
						</ButtonStyled>
					</template>
					<ButtonStyled v-if="domain.is_active" type="standard" circular>
						<button
							v-tooltip="formatMessage(messages.refreshManifest)"
							:disabled="ymclStore.loadingManifest"
							@click="refreshManifest"
						>
							<RefreshCwIcon :class="{ 'animate-spin': ymclStore.loadingManifest }" />
						</button>
					</ButtonStyled>
				</div>
			</div>

			<p
				v-if="ymclStore.domains.filter((d) => !d.is_personal).length === 0"
				class="m-0 text-sm text-secondary"
			>
				{{ formatMessage(messages.noDomains) }}
			</p>
		</div>

		<div
			v-if="!ymclStore.isPersonal && ymclStore.activeDomain?.capabilities"
			class="text-sm text-secondary"
		>
			<span v-if="ymclStore.activeDomain.capabilities.adapter_version">
				{{ formatMessage(messages.adapterVersion) }}:
				{{ ymclStore.activeDomain.capabilities.adapter_version }}
			</span>
			<span class="ml-3">
				{{ formatMessage(messages.protocolVersion) }}:
				{{ ymclStore.activeDomain.capabilities.protocol_version }}
			</span>
		</div>

		<template v-if="showAccountSection">
			<h3 class="m-0 mt-2 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.account) }}
			</h3>

			<div v-if="ymclStore.session" class="flex flex-col gap-3">
				<div class="flex items-center gap-3">
					<DomainImg
						:src="ymclStore.session.session.avatar"
						:alt="ymclStore.session.session.username"
						class="h-10 w-10 shrink-0 rounded-full object-cover"
					>
						<template #fallback>
							<UserIcon class="h-10 w-10 shrink-0 text-secondary" />
						</template>
					</DomainImg>
					<div class="min-w-0 flex-1">
						<div class="truncate font-semibold text-contrast">
							{{ ymclStore.session.session.nickname ?? ymclStore.session.session.username }}
						</div>
						<div class="truncate text-xs text-secondary">
							{{ ymclStore.session.session.permissions.join(' · ') }}
						</div>
					</div>
					<ButtonStyled type="danger">
						<button @click="signOut">
							<LogOutIcon />
							{{ formatMessage(messages.signOut) }}
						</button>
					</ButtonStyled>
				</div>

				<div v-if="sessionContext" class="flex gap-3">
					<div v-if="departmentOptions.length > 0" class="flex-1">
						<label class="mb-1 block text-xs font-medium text-secondary">
							{{ formatMessage(messages.department) }}
						</label>
						<Combobox
							:model-value="sessionContext.dept?.name ?? ''"
							:options="departmentOptions"
							:display-value="sessionContext.dept?.name"
							class="w-full"
							@update:model-value="switchDepartment"
						/>
					</div>
					<div v-if="roleOptions.length > 0" class="flex-1">
						<label class="mb-1 block text-xs font-medium text-secondary">
							{{ formatMessage(messages.role) }}
						</label>
						<Combobox
							:model-value="sessionContext.role?.name ?? ''"
							:options="roleOptions"
							:display-value="sessionContext.role?.name"
							class="w-full"
							@update:model-value="switchRole"
						/>
					</div>
				</div>
			</div>

			<div
				v-else-if="hasPassword || hasOauth || authMethods.length > 0"
				class="flex flex-col gap-3"
			>
				<button
					class="flex cursor-pointer items-center gap-2 border-0 bg-transparent p-0 text-sm text-secondary"
					@click="showLogin = !showLogin"
				>
					<LockIcon class="h-4 w-4" />
					{{ formatMessage(messages.signInHeading) }}
				</button>

				<template v-if="showLogin">
					<div v-if="hasPassword" class="flex gap-2">
						<StyledInput
							v-model="username"
							:placeholder="formatMessage(messages.usernameLabel)"
							class="w-full"
						/>
						<StyledInput
							v-model="password"
							type="password"
							:placeholder="formatMessage(messages.passwordLabel)"
							class="w-full"
							@keyup.enter="signInWithPassword"
						/>
						<ButtonStyled>
							<button
								:disabled="ymclStore.loggingIn || !username || !password"
								@click="signInWithPassword"
							>
								{{ formatMessage(messages.signIn) }}
							</button>
						</ButtonStyled>
					</div>

					<div class="flex flex-wrap items-center gap-2">
						<ButtonStyled v-if="hasOauth" type="standard">
							<button :disabled="ymclStore.loggingIn" @click="signInWithOauth">
								<LoaderIcon :class="{ 'animate-spin': ymclStore.loggingIn }" />
								{{ formatMessage(messages.oauthSignIn) }}
							</button>
						</ButtonStyled>
						<template v-if="externalProviders">
							<ButtonStyled
								v-for="provider in externalProviders"
								:key="provider.code"
								type="standard"
							>
								<button :disabled="ymclStore.loggingIn" @click="signInWithExternal(provider.code)">
									{{ provider.name ?? provider.code }}
								</button>
							</ButtonStyled>
						</template>
					</div>

					<p v-if="ymclStore.loggingIn" class="m-0 text-sm text-secondary">
						{{ formatMessage(messages.loginPending) }}
					</p>
				</template>
			</div>

			<p v-else class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.noLoginMethods) }}
			</p>
		</template>
	</div>
	<AddDomainModal ref="addDomainModal" />
</template>
