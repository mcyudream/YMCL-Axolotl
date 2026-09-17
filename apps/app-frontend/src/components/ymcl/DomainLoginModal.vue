<script setup lang="ts">
import { LogInIcon, PlusIcon, SpinnerIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	NewModal,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { computed, nextTick, onMounted, ref } from 'vue'

import { ymcl, ymclErrorMessage, type YmclExternalProvider } from '@/helpers/ymcl'
import { useYmclStore } from '@/store/ymcl'

/**
 * One-shot domain sign-in: the login establishes the domain session and the
 * Minecraft (Yggdrasil) account attaches from the same session via the
 * domain's authlib-injector exchange endpoint — no password replay.
 *
 * When the account has no Minecraft character, login is blocked and the
 * dialog forces character creation before completing the sign-in.
 */

type YggProfile = { id: string; name: string }
type YggCredential = { profile: { id: string; name: string } }

const emit = defineEmits<{
	signedIn: [credential: YggCredential | null]
}>()

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()
const ymclStore = useYmclStore()

const modal = ref<InstanceType<typeof NewModal>>()
const username = ref('')
const password = ref('')
const email = ref('')
const characterName = ref('')
const busy = ref(false)
const error = ref<string | null>(null)
const notice = ref<string | null>(null)
const step = ref<'form' | 'profiles' | 'character'>('form')
const mode = ref<'login' | 'register'>('login')
const pendingProfiles = ref<YggProfile[]>([])
const externalProviders = ref<YmclExternalProvider[]>([])
const oauthInFlight = ref(false)
/**
 * Bumped on cancel/close so an in-flight login that lands later (its HTTP
 * request cannot be aborted) silently discards instead of repopulating a
 * freshly reopened dialog.
 */
let attemptId = 0

const domainName = computed(() => ymclStore.activeDomain?.display_name ?? '')
const auth = computed(() => ymclStore.activeDomain?.capabilities?.auth)
const authMethods = computed(() => auth.value?.methods ?? [])
const hasOauth = computed(() =>
	authMethods.value.some((method) => method.type === 'oauth-web'),
)
const registrationEnabled = computed(
	() => auth.value?.registration?.enabled !== false,
)
const formValid = computed(
	() => username.value.trim().length > 0 && password.value.length > 0,
)
const registerValid = computed(
	() =>
		username.value.trim().length > 0 &&
		email.value.trim().length > 0 &&
		password.value.length > 0,
)
const characterValid = computed(() => characterName.value.trim().length >= 3)

const messages = defineMessages({
	title: {
		id: 'app.ymcl.domain-login.title',
		defaultMessage: '登录「{domain}」',
	},
	description: {
		id: 'app.ymcl.domain-login.description',
		defaultMessage:
			'使用域账号登录。登录后会同时关联该域的 Minecraft（Yggdrasil）角色，无需再单独添加账号。',
	},
	usernameLabel: {
		id: 'app.ymcl.domain-login.username',
		defaultMessage: '账号',
	},
	usernamePlaceholder: {
		id: 'app.ymcl.domain-login.username-placeholder',
		defaultMessage: '域账号或邮箱',
	},
	passwordLabel: {
		id: 'app.ymcl.domain-login.password',
		defaultMessage: '密码',
	},
	signIn: {
		id: 'app.ymcl.domain-login.sign-in',
		defaultMessage: '登录',
	},
	oauthSignIn: {
		id: 'app.ymcl.domain-login.oauth',
		defaultMessage: '使用浏览器登录',
	},
	registerTab: {
		id: 'app.ymcl.domain-login.register-tab',
		defaultMessage: '注册账号',
	},
	loginTab: {
		id: 'app.ymcl.domain-login.login-tab',
		defaultMessage: '已有账号',
	},
	emailLabel: {
		id: 'app.ymcl.domain-login.email',
		defaultMessage: '邮箱',
	},
	emailPlaceholder: {
		id: 'app.ymcl.domain-login.email-placeholder',
		defaultMessage: '用于登录与找回密码',
	},
	registerSubmit: {
		id: 'app.ymcl.domain-login.register-submit',
		defaultMessage: '注册',
	},
	registerSuccess: {
		id: 'app.ymcl.domain-login.register-success',
		defaultMessage: '注册成功。请前往邮箱完成验证后，再回到此处登录。',
	},
	registerDescription: {
		id: 'app.ymcl.domain-login.register-description',
		defaultMessage: '填写信息注册域账号。注册后需完成邮箱验证才能登录。',
	},
	profilePrompt: {
		id: 'app.ymcl.domain-login.profile-prompt',
		defaultMessage: '该账号有多个 Minecraft 角色，请选择要使用的角色：',
	},
	yggFailed: {
		id: 'app.ymcl.domain-login.ygg-failed',
		defaultMessage: '域会话已建立，但关联 Minecraft 角色失败：{error}',
	},
	characterTitle: {
		id: 'app.ymcl.domain-login.character.title',
		defaultMessage: '创建 Minecraft 角色',
	},
	characterDescription: {
		id: 'app.ymcl.domain-login.character.description',
		defaultMessage:
			'该域账号还没有 Minecraft 角色。请先创建角色，才能完成登录并进入游戏。',
	},
	characterNameLabel: {
		id: 'app.ymcl.domain-login.character.name',
		defaultMessage: '角色名',
	},
	characterNamePlaceholder: {
		id: 'app.ymcl.domain-login.character.name-placeholder',
		defaultMessage: '3–16 位字母、数字或下划线',
	},
	characterCreate: {
		id: 'app.ymcl.domain-login.character.create',
		defaultMessage: '创建角色并登录',
	},
	characterRequired: {
		id: 'app.ymcl.domain-login.character.required',
		defaultMessage: '必须先创建 Minecraft 角色才能完成登录。',
	},
	externalPending: {
		id: 'app.ymcl.domain-login.external.pending',
		defaultMessage: '请在浏览器中完成第三方登录…',
	},
})

onMounted(() => {
	void loadExternalProviders()
})

async function loadExternalProviders() {
	try {
		externalProviders.value = await ymcl.externalProviders(ymclStore.activeDomainId)
	} catch {
		externalProviders.value = []
	}
}

function show() {
	attemptId += 1
	username.value = ''
	password.value = ''
	email.value = ''
	characterName.value = ''
	error.value = null
	notice.value = null
	step.value = 'form'
	mode.value = 'login'
	pendingProfiles.value = []
	oauthInFlight.value = false
	busy.value = false
	void loadExternalProviders()
	modal.value?.show()
}

/** Cancels any pending wait and closes the dialog. A pending browser-OAuth
 * flow additionally stops the backend loopback listener so it does not keep
 * waiting out its timeout window. */
async function cancel() {
	attemptId += 1
	if (oauthInFlight.value) {
		oauthInFlight.value = false
		ymcl.oauthCancel().catch(() => {})
	}
	busy.value = false
	await nextTick()
	modal.value?.hide()
}

async function completeSignIn(credential: YggCredential | null) {
	// Clear busy before hiding: NewModal.hide() refuses to run while
	// disable-close (= busy) is active. The prop only flips once this
	// component re-renders, so flush a tick first or hide() still sees the
	// stale `true` and silently stays open.
	busy.value = false
	oauthInFlight.value = false
	emit('signedIn', credential)
	await nextTick()
	modal.value?.hide()
}

async function submit() {
	if (!formValid.value || busy.value) return
	error.value = null
	notice.value = null
	busy.value = true
	const id = ++attemptId
	try {
		await ymclStore.passwordLogin(username.value.trim(), password.value)
	} catch (err) {
		if (id === attemptId) {
			error.value = ymclErrorMessage(err)
			busy.value = false
		}
		return
	}
	if (id !== attemptId) return
	await linkYggProfile(id, undefined)
}

async function submitRegister() {
	if (!registerValid.value || busy.value) return
	error.value = null
	notice.value = null
	busy.value = true
	const id = ++attemptId
	try {
		await ymclStore.register(
			username.value.trim(),
			email.value.trim(),
			password.value,
		)
		if (id !== attemptId) return
		// Host requires email verification before login succeeds — do not
		// auto-login; switch back to the login form with the chosen username.
		const registeredUsername = username.value.trim()
		mode.value = 'login'
		password.value = ''
		email.value = ''
		username.value = registeredUsername
		const message = formatMessage(messages.registerSuccess)
		notice.value = message
		addNotification({
			type: 'success',
			title: formatMessage(messages.registerSubmit),
			text: message,
		})
	} catch (err) {
		if (id === attemptId) error.value = ymclErrorMessage(err)
	} finally {
		if (id === attemptId) busy.value = false
	}
}

/** Attaches the Minecraft account from the domain session (Sa-Token) via the
 * authlib-injector exchange endpoint — passwordless. Forces character
 * creation when the account has no Minecraft profile yet. */
async function linkYggProfile(id: number, profileName?: string) {
	try {
		const result = await ymcl.yggExchange(ymclStore.activeDomainId, profileName)
		if (id !== attemptId) return
		if (!profileName && result.profiles.length > 1) {
			pendingProfiles.value = result.profiles
			step.value = 'profiles'
			busy.value = false
			return
		}
		await completeSignIn(result.credentials)
	} catch (err) {
		if (id !== attemptId) return
		const message = ymclErrorMessage(err)
		// The exchange fails when the account owns no Minecraft profile.
		// Keep the domain session (already stored) but force character creation.
		const needsCharacter =
			message.includes('no available Minecraft profile') ||
			message.includes('profiles') ||
			message.includes('角色')
		if (needsCharacter) {
			step.value = 'character'
			notice.value = formatMessage(messages.characterRequired)
			busy.value = false
			return
		}
		// Session already succeeded; the Minecraft side is degraded only when
		// the exchange endpoint itself is missing.
		notice.value = formatMessage(messages.yggFailed, { error: message })
		busy.value = false
	} finally {
		if (id === attemptId && step.value !== 'character' && step.value !== 'profiles') {
			busy.value = false
		}
	}
}

async function createCharacter() {
	if (!characterValid.value || busy.value) return
	error.value = null
	notice.value = null
	busy.value = true
	const id = ++attemptId
	try {
		await ymcl.skinCreateProfile(
			ymclStore.activeDomainId,
			characterName.value.trim(),
		)
		if (id !== attemptId) return
		await linkYggProfile(id, characterName.value.trim())
	} catch (err) {
		if (id === attemptId) {
			error.value = ymclErrorMessage(err)
			busy.value = false
		}
	}
}

async function chooseProfile(profileName: string) {
	if (busy.value) return
	error.value = null
	busy.value = true
	const id = ++attemptId
	try {
		const result = await ymcl.yggExchange(ymclStore.activeDomainId, profileName)
		if (id !== attemptId) return
		await completeSignIn(result.credentials)
	} catch (err) {
		if (id === attemptId) error.value = ymclErrorMessage(err)
	} finally {
		if (id === attemptId) busy.value = false
	}
}

async function signInWithOauth() {
	if (busy.value) return
	error.value = null
	notice.value = null
	busy.value = true
	oauthInFlight.value = true
	const id = ++attemptId
	try {
		await ymclStore.oauthLogin()
		if (id !== attemptId) return
		await linkYggProfile(id)
	} catch (err) {
		if (id === attemptId) error.value = ymclErrorMessage(err)
	} finally {
		if (id === attemptId) {
			busy.value = false
			oauthInFlight.value = false
		}
	}
}

async function signInWithExternal(provider: string) {
	if (busy.value) return
	error.value = null
	notice.value = formatMessage(messages.externalPending)
	busy.value = true
	const id = ++attemptId
	try {
		const poll = await ymclStore.externalLogin(provider)
		if (id !== attemptId) return
		if (poll?.status === 'completed') {
			await linkYggProfile(id)
			return
		}
		if (poll?.status === 'bind_required') {
			error.value = '该第三方账号尚未绑定域账号，请先在站点完成绑定'
			return
		}
		error.value = '第三方登录未完成或已超时'
	} catch (err) {
		if (id === attemptId) error.value = ymclErrorMessage(err)
	} finally {
		if (id === attemptId) {
			busy.value = false
			notice.value = null
		}
	}
}

defineExpose({ show })
</script>

<template>
	<NewModal
		ref="modal"
		:header="
			step === 'character'
				? formatMessage(messages.characterTitle)
				: formatMessage(messages.title, { domain: domainName })
		"
		:disable-close="busy"
		max-width="420px"
	>
		<div class="flex min-w-[22rem] flex-col gap-4">
			<template v-if="step === 'form'">
				<p class="m-0 text-secondary">
					{{
						mode === 'register'
							? formatMessage(messages.registerDescription)
							: formatMessage(messages.description)
					}}
				</p>

				<div v-if="registrationEnabled" class="flex gap-2">
					<ButtonStyled :color="mode === 'login' ? 'brand' : 'standard'">
						<button :disabled="busy" @click="mode = 'login'">
							{{ formatMessage(messages.loginTab) }}
						</button>
					</ButtonStyled>
					<ButtonStyled :color="mode === 'register' ? 'brand' : 'standard'">
						<button :disabled="busy" @click="mode = 'register'">
							{{ formatMessage(messages.registerTab) }}
						</button>
					</ButtonStyled>
				</div>

				<label class="flex flex-col gap-2 font-semibold">
					{{ formatMessage(messages.usernameLabel) }}
					<StyledInput
						v-model="username"
						:disabled="busy"
						:placeholder="formatMessage(messages.usernamePlaceholder)"
						autocomplete="username"
						wrapper-class="w-full"
						@keyup.enter="mode === 'register' ? submitRegister() : submit()"
					/>
				</label>
				<label v-if="mode === 'register'" class="flex flex-col gap-2 font-semibold">
					{{ formatMessage(messages.emailLabel) }}
					<StyledInput
						v-model="email"
						:disabled="busy"
						:placeholder="formatMessage(messages.emailPlaceholder)"
						autocomplete="email"
						wrapper-class="w-full"
						@keyup.enter="submitRegister"
					/>
				</label>
				<label class="flex flex-col gap-2 font-semibold">
					{{ formatMessage(messages.passwordLabel) }}
					<StyledInput
						v-model="password"
						type="password"
						:disabled="busy"
						:autocomplete="mode === 'register' ? 'new-password' : 'current-password'"
						wrapper-class="w-full"
						@keyup.enter="mode === 'register' ? submitRegister() : submit()"
					/>
				</label>
				<div class="flex flex-wrap items-center gap-2">
					<ButtonStyled color="brand">
						<button
							v-if="mode === 'login'"
							:disabled="busy || !formValid"
							@click="submit"
						>
							<SpinnerIcon v-if="busy" class="animate-spin" />
							<LogInIcon v-else />
							{{ formatMessage(messages.signIn) }}
						</button>
						<button v-else :disabled="busy || !registerValid" @click="submitRegister">
							<SpinnerIcon v-if="busy" class="animate-spin" />
							<PlusIcon v-else />
							{{ formatMessage(messages.registerSubmit) }}
						</button>
					</ButtonStyled>
					<ButtonStyled v-if="mode === 'login' && hasOauth" type="standard">
						<button :disabled="busy" @click="signInWithOauth">
							{{ formatMessage(messages.oauthSignIn) }}
						</button>
					</ButtonStyled>
					<ButtonStyled type="standard" class="ml-auto">
						<button @click="cancel">
							{{ formatMessage(commonMessages.cancelButton) }}
						</button>
					</ButtonStyled>
				</div>

				<div v-if="mode === 'login' && externalProviders.length" class="flex flex-col gap-2">
					<ButtonStyled
						v-for="provider in externalProviders"
						:key="provider.code"
						type="standard"
					>
						<button
							:disabled="busy"
							@click="signInWithExternal(provider.code)"
						>
							使用 {{ provider.name ?? provider.code }} 登录
						</button>
					</ButtonStyled>
				</div>
			</template>

			<template v-else-if="step === 'profiles'">
				<p class="m-0 text-secondary">{{ formatMessage(messages.profilePrompt) }}</p>
				<ButtonStyled
					v-for="profile in pendingProfiles"
					:key="profile.id"
					class="w-full"
				>
					<button :disabled="busy" @click="chooseProfile(profile.name)">
						<SpinnerIcon v-if="busy" class="animate-spin" />
						{{ profile.name }}
					</button>
				</ButtonStyled>
				<div class="flex justify-end">
					<ButtonStyled type="standard">
						<button @click="cancel">
							{{ formatMessage(commonMessages.cancelButton) }}
						</button>
					</ButtonStyled>
				</div>
			</template>

			<template v-else>
				<p class="m-0 text-secondary">
					{{ formatMessage(messages.characterDescription) }}
				</p>
				<label class="flex flex-col gap-2 font-semibold">
					{{ formatMessage(messages.characterNameLabel) }}
					<StyledInput
						v-model="characterName"
						:disabled="busy"
						:placeholder="formatMessage(messages.characterNamePlaceholder)"
						autocomplete="off"
						spellcheck="false"
						wrapper-class="w-full"
						@keyup.enter="createCharacter"
					/>
				</label>
				<div class="flex flex-wrap items-center gap-2">
					<ButtonStyled color="brand">
						<button :disabled="busy || !characterValid" @click="createCharacter">
							<SpinnerIcon v-if="busy" class="animate-spin" />
							<PlusIcon v-else />
							{{ formatMessage(messages.characterCreate) }}
						</button>
					</ButtonStyled>
					<ButtonStyled type="standard" class="ml-auto">
						<button @click="cancel">
							{{ formatMessage(commonMessages.cancelButton) }}
						</button>
					</ButtonStyled>
				</div>
			</template>

			<p v-if="notice" class="m-0 text-sm text-orange">{{ notice }}</p>
			<p v-else-if="error" class="m-0 text-sm text-red">{{ error }}</p>
		</div>
	</NewModal>
</template>
