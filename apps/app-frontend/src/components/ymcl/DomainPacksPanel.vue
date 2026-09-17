<script setup lang="ts">
import {
	DownloadIcon,
	LoaderIcon,
	PackageIcon,
	TriangleAlertIcon,
	UpdatedIcon,
} from '@modrinth/assets'
import { ButtonStyled, defineMessages, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'
import { useRouter } from 'vue-router'

import type { DomainPackEntry } from '@/composables/useDomainPacks'
import { useDomainPacks } from '@/composables/useDomainPacks'

/**
 * Library-side view of the domain's published modpacks (MIP appendix B).
 * Data + actions come from the shared useDomainPacks composable, which also
 * backs the home widget and the create-page section.
 */

const { formatMessage } = useVIntl()
const router = useRouter()
const {
	loading,
	packs,
	downloadingServer,
	updatingInstance,
	hasDomainFace,
	emptyReason,
	loadError,
	installFor,
	hasUpdate,
	refresh,
	download,
	update,
} = useDomainPacks()

const messages = defineMessages({
	title: {
		id: 'app.ymcl.packs.title',
		defaultMessage: '域整合包',
	},
	description: {
		id: 'app.ymcl.packs.description',
		defaultMessage: '域内发布的服务器整合包；下载会创建一个新的本地实例，可选内容按默认勾选。',
	},
	download: {
		id: 'app.ymcl.packs.download',
		defaultMessage: '下载整合包',
	},
	downloading: {
		id: 'app.ymcl.packs.downloading',
		defaultMessage: '正在下载…',
	},
	mcVersion: {
		id: 'app.ymcl.packs.mc-version',
		defaultMessage: 'MC {version}',
	},
	installedOn: {
		id: 'app.ymcl.packs.installed-on',
		defaultMessage: '已安装到「{name}」',
	},
	updateAvailable: {
		id: 'app.ymcl.packs.update-available',
		defaultMessage: '可更新到 {version}',
	},
	updateNow: {
		id: 'app.ymcl.packs.update-now',
		defaultMessage: '更新整合包',
	},
	openInstance: {
		id: 'app.ymcl.packs.open-instance',
		defaultMessage: '打开实例',
	},
	season: { id: 'app.ymcl.install.season', defaultMessage: '当前周目：{name}' },
	versionLabel: { id: 'app.ymcl.install.version', defaultMessage: '版本' },
	loading: { id: 'app.ymcl.publish.loading', defaultMessage: '加载中…' },
	checkUpdates: {
		id: 'app.ymcl.packs.check-updates',
		defaultMessage: '检查更新',
	},
	emptyLoadFailed: {
		id: 'app.ymcl.packs.empty-load-failed',
		defaultMessage: '暂时无法连接域的服务器列表',
	},
	emptyNoBound: {
		id: 'app.ymcl.packs.empty-no-bound',
		defaultMessage: '管理员还没有发布可加入的服务器整合包，请稍后再试。',
	},
	emptyNoPack: {
		id: 'app.ymcl.packs.empty-no-pack',
		defaultMessage: '暂时没有可用的服务器整合包（可能正在更新或重新发布），请稍后点击「检查更新」重试。',
	},
	retry: { id: 'app.ymcl.manage.retry', defaultMessage: '重试' },
})

const visible = computed(
	() =>
		hasDomainFace.value &&
		(loading.value || packs.value.length > 0 || emptyReason.value !== 'none'),
)

function openInstance(pack: DomainPackEntry) {
	const install = installFor(pack)
	if (install) router.push(`/instance/${encodeURIComponent(install.instanceId)}`)
}
</script>

<template>
	<section v-if="visible" class="flex flex-col gap-2">
		<div class="flex flex-col gap-0.5">
			<h2 class="m-0 flex items-center gap-2 text-lg font-bold text-contrast">
				<PackageIcon class="h-5 w-5" />
				{{ formatMessage(messages.title) }}
			</h2>
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.description) }}
			</p>
		</div>
		<div class="flex items-center justify-end">
			<ButtonStyled size="small" type="outlined">
				<button :disabled="loading" class="flex items-center gap-2" @click="refresh()">
					<LoaderIcon v-if="loading" class="h-4 w-4 animate-spin" />
					<UpdatedIcon v-else class="h-4 w-4" />
					{{ formatMessage(messages.checkUpdates) }}
				</button>
			</ButtonStyled>
		</div>

		<div v-if="loading" class="flex items-center gap-2 p-3 text-sm text-secondary">
			<LoaderIcon class="h-5 w-5 animate-spin" />
			{{ formatMessage(messages.loading) }}
		</div>

		<div
			v-else-if="packs.length === 0 && emptyReason !== 'none'"
			class="flex flex-col gap-2 rounded-xl bg-bg-raised p-3 text-sm"
		>
			<div class="flex flex-wrap items-center gap-2">
				<TriangleAlertIcon class="h-5 w-5 shrink-0 text-orange" />
				<span class="min-w-0 flex-1 text-secondary">
					<template v-if="emptyReason === 'load-failed'">
						{{ formatMessage(messages.emptyLoadFailed) }}：{{ loadError }}
					</template>
					<template v-else-if="emptyReason === 'no-bound'">
						{{ formatMessage(messages.emptyNoBound) }}
					</template>
					<template v-else>{{ formatMessage(messages.emptyNoPack) }}</template>
				</span>
				<ButtonStyled size="small" type="outlined">
					<button :disabled="loading" @click="refresh()">
						{{ formatMessage(messages.retry) }}
					</button>
				</ButtonStyled>
			</div>
			<div
				v-for="diagnostic in serverDiagnostics"
				:key="diagnostic.serverName"
				class="flex flex-wrap items-center gap-x-2 text-xs"
				:class="diagnostic.ok ? 'text-secondary' : 'text-orange'"
			>
				<span class="font-semibold">{{ diagnostic.serverName }}</span>
				<span>·</span>
				<span class="min-w-0 break-all">{{ diagnostic.detail }}</span>
			</div>
		</div>

		<div v-else class="flex flex-col gap-2">
			<div
				v-for="pack in packs"
				:key="pack.serverId"
				class="flex flex-wrap items-center justify-between gap-3 rounded-xl bg-bg-raised p-3"
			>
				<div class="flex min-w-0 flex-col gap-1">
					<span class="font-semibold text-contrast">{{ pack.serverName }}</span>
					<span class="flex flex-wrap items-center gap-x-2 gap-y-0.5 text-xs text-secondary">
						<span>{{ pack.packId }}</span>
						<span>·</span>
						<span>{{ formatMessage(messages.versionLabel) }} {{ pack.targetVersion }}</span>
						<template v-if="pack.requiredVersion">
							<span>·</span>
							<span>{{
								formatMessage(messages.mcVersion, { version: pack.requiredVersion })
							}}</span>
						</template>
						<template v-if="pack.seasonName">
							<span>·</span>
							<span>{{ formatMessage(messages.season, { name: pack.seasonName }) }}</span>
						</template>
					</span>
					<span
						v-if="installFor(pack)"
						class="flex flex-wrap items-center gap-x-2 text-xs"
						:class="hasUpdate(pack) ? 'text-orange' : 'text-secondary'"
					>
						<template v-if="hasUpdate(pack)">
							<UpdatedIcon class="h-4 w-4" />
							{{ formatMessage(messages.updateAvailable, { version: pack.targetVersion }) }}
						</template>
						{{
							formatMessage(messages.installedOn, {
								name: installFor(pack)?.instanceName,
							})
						}}
					</span>
				</div>

				<div class="flex items-center gap-2">
					<ButtonStyled color="brand" size="small">
						<button
							v-if="!installFor(pack)"
							:disabled="!!downloadingServer"
							class="flex items-center gap-2"
							@click="download(pack)"
						>
							<LoaderIcon v-if="downloadingServer === pack.serverId" class="animate-spin" />
							<DownloadIcon v-else />
							{{
								formatMessage(
									downloadingServer === pack.serverId ? messages.downloading : messages.download,
								)
							}}
						</button>
						<button
							v-else-if="hasUpdate(pack)"
							:disabled="!!updatingInstance"
							class="flex items-center gap-2"
							@click="update(pack)"
						>
							<LoaderIcon
								v-if="updatingInstance === installFor(pack)?.instanceId"
								class="animate-spin"
							/>
							<UpdatedIcon v-else />
							{{ formatMessage(messages.updateNow) }}
						</button>
						<button v-else class="flex items-center gap-2" @click="openInstance(pack)">
							{{ formatMessage(messages.openInstance) }}
						</button>
					</ButtonStyled>
				</div>
			</div>
		</div>
	</section>
</template>
