<script setup lang="ts">
import { BoxIcon, EyeIcon, ExternalIcon, PlusIcon } from '@modrinth/assets'
import {
	EmptyState,
	NewButton,
	ProjectCardSkeleton,
	useVIntl,
	injectNotificationManager,
} from '@modrinth/ui'
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import BrowseInstanceSelector from '@/components/browse/BrowseInstanceSelector.vue'
import {
	schematicMessages as messages,
	schematicSourceMessages,
} from '@/components/browse/schematics/messages'
import InstanceIcon from '@/components/ui/InstanceIcon.vue'
import TranslatedProjectDescription from '@/components/ui/TranslatedProjectDescription.vue'
import { list as listInstances } from '@/helpers/instance'
import { isServerProjectLink } from '@/helpers/instance-project'
import {
	getSchematic,
	getInstalledSchematics,
	hasInstalledSchematic,
	installedSchematicsKey,
	installSchematic,
	isSchematicSource,
	isSchematicFileName,
} from '@/helpers/schematics'
import { getBrowseDefaultInstanceId, setBrowseDefaultInstanceId } from '@/helpers/settings'
import type { GameInstance } from '@/helpers/types'
import { useBreadcrumbs } from '@/store/breadcrumbs'

const { formatMessage } = useVIntl()
const { addNotification, handleError } = injectNotificationManager()
const route = useRoute()
const router = useRouter()
const queryClient = useQueryClient()
const breadcrumbs = useBreadcrumbs()
const source = computed(() =>
	isSchematicSource(route.params.source) ? route.params.source : 'reden',
)
const projectId = computed(() => String(route.params.id ?? ''))
const picker = ref<InstanceType<typeof BrowseInstanceSelector>>()
const selectedInstanceId = ref(
	typeof route.query.i === 'string' ? route.query.i : getBrowseDefaultInstanceId(),
)
const instancesQuery = useQuery({ queryKey: ['schematic-install-targets'], queryFn: listInstances })
const instances = computed(() =>
	(instancesQuery.data.value ?? []).filter(
		(instance) => !isServerProjectLink(instance.link) && instance.install_stage === 'installed',
	),
)
const selectedInstance = computed(
	() => instances.value.find((instance) => instance.id === selectedInstanceId.value) ?? null,
)
const detail = useQuery({
	queryKey: computed(() => ['schematics', 'detail', source.value, projectId.value]),
	queryFn: () => getSchematic(source.value, projectId.value),
	retry: false,
})
const project = detail.data
const installed = useQuery({
	queryKey: computed(() => installedSchematicsKey(selectedInstance.value?.id ?? null)),
	queryFn: () => getInstalledSchematics(selectedInstance.value!.id),
	enabled: computed(() => !!selectedInstance.value),
	retry: false,
})
const installation = useMutation({
	mutationFn: installSchematic,
	onSuccess: (path, request) => {
		void queryClient.invalidateQueries({ queryKey: installedSchematicsKey(request.instanceId) })
		addNotification({ title: formatMessage(messages.installSuccess), text: path, type: 'success' })
	},
	onError: (error) => {
		handleError(
			String(error).includes('schematic_target_exists')
				? new Error(formatMessage(messages.fileExists))
				: error,
		)
	},
})
function chooseInstance(instance: GameInstance) {
	selectedInstanceId.value = instance.id
	setBrowseDefaultInstanceId(instance.id)
	void router.replace({ query: { ...route.query, i: instance.id } })
}
function install(fileName: string) {
	if (!selectedInstance.value) {
		picker.value?.show()
		return
	}
	if (installation.isPending.value || !installed.isSuccess.value) return
	installation.mutate({
		instanceId: selectedInstance.value.id,
		source: source.value,
		id: projectId.value,
		fileName,
	})
}
function preview(fileName: string) {
	void router.push({
		name: 'Schematic workshop',
		query: { source: source.value, project: projectId.value, file: fileName },
	})
}
const browseLink = computed(() =>
	typeof route.query.b === 'string' && /^\/browse\/schematic(?:[?#]|$)/.test(route.query.b)
		? route.query.b
		: '/browse/schematic',
)
watch(project, (value) => {
	if (value) breadcrumbs.setName('Project', value.title)
})
</script>

<template>
	<div class="mx-auto flex w-full max-w-5xl flex-col gap-5 p-6">
		<RouterLink :to="browseLink" class="text-secondary hover:underline">{{
			formatMessage(messages.schematics)
		}}</RouterLink>
		<ProjectCardSkeleton v-if="detail.isPending.value" layout="list" />
		<div v-else-if="detail.isError.value" role="alert" class="flex items-center gap-3">
			<span>{{ formatMessage(messages.loadError) }}</span
			><NewButton @click="detail.refetch()">{{ formatMessage(messages.retry) }}</NewButton>
		</div>
		<template v-else-if="project">
			<header class="flex flex-wrap items-start gap-4">
				<img
					v-if="project.cover"
					:src="project.cover"
					alt=""
					class="size-24 rounded-xl object-cover"
				/>
				<div class="min-w-0 flex-1">
					<h1 class="m-0 break-words text-2xl font-bold text-contrast">{{ project.title }}</h1>
					<p class="mb-0 text-secondary">
						{{
							[
								project.author,
								formatMessage(schematicSourceMessages[project.source]),
								...project.gameVersions,
							]
								.filter(Boolean)
								.join(' · ')
						}}
					</p>
				</div>
				<NewButton @click="openUrl(project.sourceUrl).catch(handleError)"
					><ExternalIcon />{{ formatMessage(messages.openSource) }}</NewButton
				>
			</header>
			<TranslatedProjectDescription
				:description="project.description"
				:active="false"
				:translations="{}"
				mode="translation-only"
				style="weakened"
				:format="project.descriptionFormat === 'html' ? 'html' : 'markdown'"
			/>
			<section data-onboarding-id="schematic-files" class="flex flex-col gap-3">
				<div class="flex flex-wrap items-center justify-between gap-3">
					<h2 class="m-0 text-lg font-semibold">{{ formatMessage(messages.files) }}</h2>
					<NewButton
						:disabled="instancesQuery.isPending.value || installation.isPending.value"
						@click="picker?.show()"
						><InstanceIcon
							v-if="selectedInstance"
							size="1.25rem"
							:icon-path="selectedInstance.icon_path"
							:instance-id="selectedInstance.id"
							:loader="selectedInstance.loader"
						/><PlusIcon v-else />{{
							selectedInstance?.name ?? formatMessage(messages.selectInstance)
						}}</NewButton
					>
				</div>
				<div v-if="instancesQuery.isError.value" role="alert">
					<span>{{ formatMessage(messages.instancesError) }}</span
					><NewButton @click="instancesQuery.refetch()">{{
						formatMessage(messages.retry)
					}}</NewButton>
				</div>
				<div v-if="selectedInstance && installed.isError.value" role="alert">
					<span>{{ formatMessage(messages.installationStatusError) }}</span
					><NewButton @click="installed.refetch()">{{ formatMessage(messages.retry) }}</NewButton>
				</div>
				<div
					v-for="file in project.files"
					:key="file.name"
					class="flex flex-wrap items-center justify-between gap-3 rounded-xl bg-surface-2 p-4"
				>
					<span class="min-w-0 flex-1 break-all font-semibold">{{ file.name }}</span>
					<NewButton v-if="isSchematicFileName(file.name)" @click="preview(file.name)"
						><EyeIcon />{{ formatMessage(messages.preview) }}</NewButton
					>
					<NewButton
						color="brand"
						:loading="
							installation.isPending.value && installation.variables.value?.fileName === file.name
						"
						:disabled="
							installation.isPending.value ||
							(!!selectedInstance && !installed.isSuccess.value) ||
							hasInstalledSchematic(installed.data.value, source, projectId, file.name)
						"
						@click="install(file.name)"
						>{{
							formatMessage(
								hasInstalledSchematic(installed.data.value, source, projectId, file.name)
									? messages.installed
									: messages.install,
							)
						}}</NewButton
					>
				</div>
				<EmptyState
					v-if="!project.files.length"
					:icon="BoxIcon"
					:heading="formatMessage(messages.emptyTitle)"
					><NewButton @click="openUrl(project.sourceUrl).catch(handleError)">{{
						formatMessage(messages.openSource)
					}}</NewButton></EmptyState
				>
			</section>
		</template>
		<BrowseInstanceSelector
			ref="picker"
			:instances="instances"
			:selected-instance="selectedInstance"
			:selected-count="0"
			:install-current="async () => true"
			:clear-current="() => {}"
			@select="chooseInstance"
		/>
	</div>
</template>
