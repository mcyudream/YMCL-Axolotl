<template>
	<div aria-label="YMCL" class="flex h-full items-center gap-2 font-extrabold text-contrast">
		<img aria-hidden="true" class="aspect-square h-full object-contain" :src="axolotlVisual" />
		<span v-if="!iconOnly" class="hidden text-sm tracking-wide xl:inline">YMCL</span>
	</div>
</template>

<script setup lang="ts">
import axolotlVisual from '@modrinth/assets/branding/axolotl.png'

defineProps<{
	iconOnly?: boolean
}>()
</script>
