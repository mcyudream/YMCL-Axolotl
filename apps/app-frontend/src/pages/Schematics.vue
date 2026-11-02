<script setup lang="ts">
import { BoxIcon } from '@modrinth/assets'
import {
	Combobox,
	EmptyState,
	NavTabs,
	NewButton,
	Pagination,
	ProjectCard,
	ProjectCardList,
	ProjectCardSkeleton,
	TextInput,
	useVIntl,
} from '@modrinth/ui'
import { useQuery } from '@tanstack/vue-query'
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import {
	schematicMessages as messages,
	schematicSourceMessages,
} from '@/components/browse/schematics/messages'
import { createBrowseProjectTabs } from '@/helpers/browse-project-tabs'
import { isSchematicSource, listSchematics, type SchematicSource } from '@/helpers/schematics'

const { formatMessage } = useVIntl()
const route = useRoute()
const router = useRouter()
const search = ref(typeof route.query.q === 'string' ? route.query.q : '')
const source = computed(() => (isSchematicSource(route.query.source) ? route.query.source : 'all'))
const query = computed(() => (typeof route.query.q === 'string' ? route.query.q : ''))
const page = computed(() => Math.max(1, Math.min(10000, Number(route.query.page) || 1)))
const sourceOptions = computed(() => [
	{ value: 'all', label: formatMessage(messages.allSources) },
	...(['reden', 'buildschematics'] as const).map((id) => ({
		value: id,
		label: formatMessage(schematicSourceMessages[id]),
	})),
])
const tabs = computed(() =>
	createBrowseProjectTabs(
		{
			modpacks: formatMessage(messages.modpacks),
			mods: formatMessage(messages.mods),
			resourcepacks: formatMessage(messages.resourcepacks),
			datapacks: formatMessage(messages.datapacks),
			maps: formatMessage(messages.maps),
			shaders: formatMessage(messages.shaders),
			servers: formatMessage(messages.servers),
			favorites: formatMessage(messages.favorites),
			schematics: formatMessage(messages.schematics),
		},
		typeof route.query.i === 'string' ? `?i=${encodeURIComponent(route.query.i)}` : '',
	),
)
const results = useQuery({
	queryKey: computed(() => ['schematics', 'browse', source.value, query.value, page.value]),
	queryFn: async () => {
		const request = { source: source.value, query: query.value, page: page.value }
		const sources: SchematicSource[] =
			request.source === 'all' ? ['reden', 'buildschematics'] : [request.source]
		return {
			...(await listSchematics(sources, request.page, request.query, 20, 'default')),
			request,
		}
	},
	placeholderData: (previous) => previous,
	retry: false,
})
const snapshot = results.data
function setSource(value: unknown) {
	void router.replace({ query: { ...route.query, source: String(value), page: undefined } })
}
function submitSearch() {
	void router.replace({
		query: { ...route.query, q: search.value.trim() || undefined, page: undefined },
	})
}
function setPage(value: number) {
	void router.replace({ query: { ...route.query, page: value > 1 ? value : undefined } })
}
function projectLink(source: string, id: string) {
	return {
		path: `/project/schematic/${source}/${encodeURIComponent(id)}`,
		query: { i: route.query.i, b: route.fullPath },
	}
}
watch(query, (value) => {
	search.value = value
})
</script>

<template>
	<div data-onboarding-id="browse-schematics-content" class="flex flex-col gap-4 p-6">
		<NavTabs :links="tabs" />
		<form class="flex flex-wrap items-center gap-3" @submit.prevent="submitSearch">
			<TextInput
				v-model="search"
				class="min-w-48 flex-1"
				:placeholder="formatMessage(messages.search)"
			/>
			<NewButton native-type="submit" :loading="results.isFetching.value">{{
				formatMessage(messages.search)
			}}</NewButton>
			<Combobox
				:model-value="source"
				:options="sourceOptions"
				:searchable="false"
				@update:model-value="setSource"
			/>
		</form>
		<div
			v-if="results.isError.value"
			role="alert"
			class="flex flex-wrap items-center gap-3 text-red"
		>
			<span>{{ formatMessage(messages.loadError) }}</span>
			<NewButton @click="results.refetch()">{{ formatMessage(messages.retry) }}</NewButton>
		</div>
		<div
			v-if="snapshot?.failedSources.length"
			role="status"
			class="flex items-center gap-3 text-secondary"
		>
			{{
				formatMessage(messages.failedSources, {
					sources: snapshot.failedSources
						.map((id) => formatMessage(schematicSourceMessages[id]))
						.join(', '),
				})
			}}
			<NewButton @click="results.refetch()">{{ formatMessage(messages.retry) }}</NewButton>
		</div>
		<ProjectCardList
			v-if="results.isPending.value"
			layout="grid"
			:aria-label="formatMessage(messages.loading)"
		>
			<ProjectCardSkeleton v-for="index in 6" :key="index" layout="grid" />
		</ProjectCardList>
		<EmptyState
			v-else-if="snapshot && !snapshot.items.length"
			:icon="BoxIcon"
			:heading="formatMessage(messages.emptyTitle)"
			:description="formatMessage(messages.emptyDescription)"
		/>
		<ProjectCardList v-else-if="snapshot" layout="grid" :aria-busy="results.isFetching.value">
			<ProjectCard
				v-for="project in snapshot.items"
				:key="`${project.source}:${project.id}`"
				layout="grid"
				:link="projectLink(project.source, project.id)"
				:title="project.title"
				:summary="
					[project.author, project.summary, formatMessage(schematicSourceMessages[project.source])]
						.filter(Boolean)
						.join(' · ')
				"
				:icon-url="project.cover ?? undefined"
				:banner="project.cover ?? undefined"
				:tags="project.gameVersions"
			/>
		</ProjectCardList>
		<Pagination
			v-if="snapshot"
			:page="snapshot.request.page"
			:count="snapshot.pageCount"
			:loading="results.isFetching.value"
			@switch-page="setPage"
		/>
	</div>
</template>
