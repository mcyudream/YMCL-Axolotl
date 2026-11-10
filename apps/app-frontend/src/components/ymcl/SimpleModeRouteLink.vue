<script setup lang="ts">
import { computed } from 'vue'
import { type RouteLocationRaw, RouterLink, useRouter } from 'vue-router'

import { useYmclStore } from '@/store/ymcl'

defineOptions({ inheritAttrs: false })
const props = defineProps<{ to: RouteLocationRaw }>()
const router = useRouter()
const store = useYmclStore()
const hidden = computed(() => store.isRouteHidden(router.resolve(props.to).path))
</script>

<template>
	<span v-if="hidden" v-bind="$attrs"><slot /></span>
	<RouterLink v-else :to="to" v-bind="$attrs"><slot /></RouterLink>
</template>
