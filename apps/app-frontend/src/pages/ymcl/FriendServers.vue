<script setup lang="ts">
import { GlobeIcon, RefreshCwIcon } from '@modrinth/assets'
import { ButtonStyled, injectNotificationManager } from '@modrinth/ui'
import { computed, onMounted, reactive, ref, watch } from 'vue'

import AddDomainModal from '@/components/ymcl/AddDomainModal.vue'
import { install_create_modpack_instance, wait_for_install_job } from '@/helpers/install'
import type { InteropPackEntry } from '@/helpers/ymcl'
import { ymcl, ymclErrorMessage } from '@/helpers/ymcl'
import { useYmclStore } from '@/store/ymcl'

const { handleError, addNotification } = injectNotificationManager()
const ymclStore = useYmclStore()
const addDomainModal = ref<InstanceType<typeof AddDomainModal>>()

const state = reactive({
  loading: false,
  packs: [] as InteropPackEntry[],
  updatedAt: '',
  copied: '',
  installingId: '',
  error: '',
})

/** 数据源：第一个非个人域的 origin（yda 宿主 well-known 合并目录，含互采友站条目） */
const sourceDomain = computed(() =>
  ymclStore.domains.find((domain) => !domain.is_personal && domain.origin) ?? null,
)

async function load() {
  const domain = sourceDomain.value
  if (!domain) {
    state.packs = []
    return
  }
  state.loading = true
  state.error = ''
  try {
    const body = await ymcl.fetchOriginCatalog(domain.origin!)
    state.packs = body.packs ?? []
    state.updatedAt = body.updated_at ?? ''
  } catch (error) {
    state.error = ymclErrorMessage(error)
  } finally {
    state.loading = false
  }
}

function serverAddress(pack: InteropPackEntry): string {
  const server = pack.server
  if (!server) return '-'
  return server.port ? `${server.address}:${server.port}` : server.address
}

async function copyServer(pack: InteropPackEntry) {
  try {
    await navigator.clipboard.writeText(serverAddress(pack))
    state.copied = pack.id
    setTimeout(() => {
      if (state.copied === pack.id) state.copied = ''
    }, 1500)
  } catch {
    handleError(new Error('clipboard unavailable'))
  }
}

const installingId = ref('')

/** 一键进入：下载友站整合包（哈希校验）→ 导入为本地实例 → 提示启动后加入服务器 */
async function quickEnter(pack: InteropPackEntry) {
  const domain = sourceDomain.value
  if (!domain || !pack.artifact || installingId.value) return
  installingId.value = pack.id
  try {
    const archive = await ymcl.interopPackDownloadByOrigin(domain.origin!, pack.id)
    const job = await install_create_modpack_instance({ type: 'fromFile', path: archive.path }, null)
    await wait_for_install_job(job.job_id)
    addNotification({
      title: pack.name || pack.id,
      text: `实例已创建，启动后连接 ${serverAddress(pack)}`,
      type: 'success',
    })
  } catch (error) {
    handleError(error)
  } finally {
    installingId.value = ''
  }
}

// 域列表异步初始化：sourceDomain 出现/切换时自动加载，避免挂载竞态空白
watch(
	sourceDomain,
	(domain) => {
		if (domain) void load()
		else state.packs = []
	},
	{ immediate: true },
)
</script>

<template>
  <div class="flex flex-col gap-4 p-6">
    <AddDomainModal ref="addDomainModal" />
    <div class="flex items-center justify-between gap-3">
      <h1 class="m-0 text-lg font-semibold text-contrast">友站服务器</h1>
      <ButtonStyled>
        <button :disabled="state.loading" @click="load">
          <RefreshCwIcon :class="{ 'animate-spin': state.loading }" />
          刷新
        </button>
      </ButtonStyled>
    </div>
    <p class="m-0 text-sm text-secondary">
      来自已加入域点的 mc-interop v1 合并目录（本站已发布条目 + peers 互采友站条目，5 分钟缓存）。
    </p>

    <div
      v-if="state.error"
      class="rounded-xl border border-solid border-red-500/40 bg-red-500/10 p-3 text-sm text-red-500"
    >
      加载失败：{{ state.error }}
    </div>

    <div v-if="!sourceDomain" class="rounded-xl border border-solid border-surface-5 bg-bg-raised p-6 text-center">
      <p class="m-0 text-sm text-secondary">
        尚未加入任何域点。添加 yda 或友站站点后，这里会展示可一键进入的服务器与整合包。
      </p>
      <ButtonStyled class="mt-3">
        <button @click="addDomainModal?.show()">添加域点</button>
      </ButtonStyled>
    </div>

    <template v-else>
      <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
        <div
          v-for="pack in state.packs"
          :key="pack.id + ':' + (pack.source?.site_id ?? 'own')"
          class="flex flex-col gap-2 rounded-xl border border-solid border-surface-5 bg-bg-raised p-4"
        >
          <div class="flex flex-wrap items-center gap-2">
            <span class="font-semibold text-contrast">{{ pack.name || pack.id }}</span>
            <span
              v-if="pack.source"
              class="rounded-full bg-surface-5 px-2 py-0.5 text-xs font-medium text-secondary"
            >友站 {{ pack.source.site_id }}</span>
            <span
              v-if="pack.artifact"
              class="rounded-full bg-brand px-2 py-0.5 text-xs font-semibold text-brand-inverted-on/bg-brand"
            >r{{ pack.revision ?? '?' }}</span>
          </div>
          <p class="m-0 line-clamp-2 text-sm text-secondary">{{ pack.summary || '暂无简介' }}</p>
          <div class="flex flex-wrap gap-2 text-xs text-secondary">
            <span v-if="pack.game_version">MC {{ pack.game_version }}</span>
            <span v-if="pack.loader">{{ pack.loader }}{{ pack.loader_version ? ` ${pack.loader_version}` : '' }}</span>
            <span v-if="pack.artifact?.format">{{ pack.artifact.format }}</span>
          </div>
          <div class="mt-auto flex flex-wrap items-center gap-2 pt-1">
            <ButtonStyled v-if="pack.artifact" type="brand">
              <button :disabled="installingId === pack.id" @click="quickEnter(pack)">
                {{ installingId === pack.id ? '下载导入中…' : '一键进入' }}
              </button>
            </ButtonStyled>
            <ButtonStyled v-if="pack.server" type="outlined">
              <button @click="copyServer(pack)">
                {{ state.copied === pack.id ? '已复制' : `复制 ${serverAddress(pack)}` }}
              </button>
            </ButtonStyled>
          </div>
        </div>
      </div>
      <p v-if="!state.packs.length" class="m-0 text-sm text-secondary">
        目录为空：该域点尚未发布任何条目。
      </p>
    </template>
  </div>
</template>
