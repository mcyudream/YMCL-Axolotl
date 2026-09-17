<script setup lang="ts">
import { LoaderIcon, PackageIcon, TriangleAlertIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	Checkbox,
	defineMessages,
	injectNotificationManager,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

import { ymclErrorMessage } from '@/helpers/ymcl'

/**
 * Player-side optional-content selector (YAP §7 / MIP WF-5): lists the
 * binding's declared features and the instance's current selection;
 * applying runs the download/delete pipeline (deselection only removes
 * locally-unmodified files).
 */

const props = defineProps<{
	instanceId: string
}>()

const { addNotification } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: {
		id: 'app.ymcl.features.title',
		defaultMessage: '可选组件',
	},
	description: {
		id: 'app.ymcl.features.description',
		defaultMessage:
			'勾选要启用的可选内容。取消勾选只删除本地未修改的文件；与已选项互斥的内容会被禁用。',
	},
	loading: { id: 'app.ymcl.publish.loading', defaultMessage: '加载中…' },
	unmanaged: {
		id: 'app.ymcl.features.unmanaged',
		defaultMessage: '该实例未绑定域整合包，没有可选组件。',
	},
	noFeatures: {
		id: 'app.ymcl.features.no-features',
		defaultMessage: '当前版本没有声明可选内容。',
	},
	apply: {
		id: 'app.ymcl.features.apply',
		defaultMessage: '应用变更',
	},
	applied: {
		id: 'app.ymcl.features.applied',
		defaultMessage: '可选组件已更新。',
	},
	applyFailed: {
		id: 'app.ymcl.features.apply-failed',
		defaultMessage: '应用可选组件失败',
	},
	loadFailed: {
		id: 'app.ymcl.features.load-failed',
		defaultMessage: '读取可选组件失败',
	},
	conflictsWith: {
		id: 'app.ymcl.features.conflicts-with',
		defaultMessage: '与 {id} 互斥',
	},
})

interface PackFeature {
	id: string
	name?: string | null
	default: boolean
	conflicts: string[]
}

interface PackFeatures {
	managed: boolean
	pack_id?: string | null
	target_version?: string | null
	features: PackFeature[]
	selected: string[]
}

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const loading = ref(false)
const applying = ref(false)
const managed = ref(false)
const features = ref<PackFeature[]>([])
const checked = ref<Set<string>>(new Set())
const initialSelected = ref<string[]>([])

/** A feature cannot be checked when a selected feature declares it as a conflict. */
function disabledReason(feature: PackFeature): string | null {
	if (checked.value.has(feature.id)) return null
	for (const other of features.value) {
		if (other.id === feature.id) continue
		if (checked.value.has(other.id) && other.conflicts.includes(feature.id)) {
			return formatMessage(messages.conflictsWith, {
				id: other.name || other.id,
			})
		}
	}
	return null
}

function toggle(featureId: string, value: boolean) {
	const next = new Set(checked.value)
	if (value) {
		next.add(featureId)
		// 强制互斥：勾选一项时取消与它互斥的项。
		const feature = features.value.find((item) => item.id === featureId)
		for (const conflict of feature?.conflicts ?? []) next.delete(conflict)
	} else {
		next.delete(featureId)
	}
	checked.value = next
}

async function load() {
	loading.value = true
	try {
		const result = await invoke<PackFeatures>('plugin:ymcl|ymcl_pack_features', {
			instanceId: props.instanceId,
		})
		managed.value = result.managed
		features.value = result.features ?? []
		initialSelected.value = [...result.selected]
		checked.value = new Set(result.selected)
	} catch (error) {
		addNotification({
			title: formatMessage(messages.loadFailed),
			text: ymclErrorMessage(error),
			type: 'error',
		})
		managed.value = false
		features.value = []
	} finally {
		loading.value = false
	}
}

async function apply() {
	if (applying.value) return
	applying.value = true
	try {
		await invoke('plugin:ymcl|ymcl_pack_set_features', {
			instanceId: props.instanceId,
			selected: [...checked.value],
		})
		initialSelected.value = [...checked.value]
		addNotification({
			title: formatMessage(messages.applied),
			type: 'success',
		})
	} catch (error) {
		addNotification({
			title: formatMessage(messages.applyFailed),
			text: ymclErrorMessage(error),
			type: 'error',
		})
	} finally {
		applying.value = false
	}
}

defineExpose({
	show: () => {
		loading.value = true
		modal.value?.show()
		void load()
	},
})
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.title)"
		scrollable
		width="30rem"
		max-width="calc(100vw - 2rem)"
	>
		<div class="flex min-h-40 flex-col gap-3">
			<div v-if="loading" class="flex items-center gap-2 text-sm text-secondary">
				<LoaderIcon class="h-5 w-5 animate-spin" />
				{{ formatMessage(messages.loading) }}
			</div>

			<template v-else>
				<div
					v-if="!managed || features.length === 0"
					class="flex items-center gap-2 rounded-xl bg-bg-raised p-4 text-sm text-secondary"
				>
					<TriangleAlertIcon class="h-5 w-5 shrink-0" />
					{{
						formatMessage(!managed ? messages.unmanaged : messages.noFeatures)
					}}
				</div>

				<template v-else>
					<p class="m-0 text-sm leading-relaxed text-secondary">
						{{ formatMessage(messages.description) }}
					</p>
					<div class="flex flex-col gap-2">
						<label
							v-for="feature in features"
							:key="feature.id"
							class="flex cursor-pointer items-start gap-3 rounded-xl bg-bg-raised p-3"
							:class="disabledReason(feature) ? 'opacity-60' : ''"
						>
							<Checkbox
								:model-value="checked.has(feature.id)"
								:disabled="applying || !!disabledReason(feature)"
								class="mt-0.5"
								@update:model-value="
									(value: boolean) => toggle(feature.id, value)
								"
							/>
							<span class="flex min-w-0 flex-col gap-0.5">
								<span class="font-medium text-contrast">
									{{ feature.name || feature.id }}
								</span>
								<span
									v-if="feature.name && feature.name !== feature.id"
									class="font-mono text-xs text-secondary"
								>
									{{ feature.id }}
								</span>
								<span v-if="disabledReason(feature)" class="text-xs text-orange">
									{{ disabledReason(feature) }}
								</span>
							</span>
						</label>
					</div>
					<ButtonStyled color="brand">
						<button
							:disabled="applying"
							class="flex items-center gap-2"
							@click="apply"
						>
							<LoaderIcon v-if="applying" class="animate-spin" />
							<PackageIcon v-else />
							{{ formatMessage(messages.apply) }}
						</button>
					</ButtonStyled>
				</template>
			</template>
		</div>
	</NewModal>
</template>
