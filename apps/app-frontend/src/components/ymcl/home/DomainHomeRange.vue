<script setup lang="ts">
const model = defineModel<number>({ required: true })
withDefaults(defineProps<{ max?: number }>(), { max: 100 })
</script>

<template>
	<input
		v-model.number="model"
		class="domain-home-range"
		type="range"
		min="0"
		:max="max"
		:style="{ '--range-progress': `${(model / max) * 100}%` }"
	/>
</template>

<style scoped>
.domain-home-range {
	width: 100%;
	min-width: 0;
	height: 1.5rem;
	min-height: 0;
	margin: 0;
	padding: 0;
	border: 0;
	background: transparent;
	box-shadow: none;
	appearance: none;
	cursor: pointer;
	--range-background: linear-gradient(
		to right,
		var(--color-brand) var(--range-progress),
		var(--domain-home-range-track, var(--color-divider)) var(--range-progress)
	);
}
.domain-home-range::-webkit-slider-runnable-track {
	height: 0.25rem;
	border-radius: 999px;
	background: var(--range-background);
}
.domain-home-range::-moz-range-track {
	height: 0.25rem;
	border-radius: 999px;
	background: var(--range-background);
}
.domain-home-range::-webkit-slider-thumb {
	width: 0.75rem;
	height: 0.75rem;
	margin-top: -0.25rem;
	border: 0;
	border-radius: 50%;
	background: var(--color-brand);
	appearance: none;
}
.domain-home-range::-moz-range-thumb {
	width: 0.75rem;
	height: 0.75rem;
	border: 0;
	border-radius: 50%;
	background: var(--color-brand);
}
</style>
