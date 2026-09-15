<script setup lang="ts">
import { computed } from 'vue'

/**
 * rich-text renderer (YAP §6.6): whitelisted Markdown rendering for rules
 * and tutorials. HTML is never rendered — text comes in as plain lines and
 * the whitelist below maps Markdown prefixes to styled elements.
 */
const props = defineProps<{
	envelope: { records?: Record<string, unknown>[] }
}>()

interface RichBlock {
	kind: 'h1' | 'h2' | 'h3' | 'li' | 'quote' | 'p'
	text: string
}

const records = computed(() => props.envelope.records ?? [])

const blocks = computed<RichBlock[]>(() => {
	const record = records.value[0]
	if (!record) return []
	const body = String(record.body ?? record.content ?? '')
	return body.split('\n').map((line) => {
		const trimmed = line.trimEnd()
		if (trimmed.startsWith('### ')) return { kind: 'h3', text: trimmed.slice(4) }
		if (trimmed.startsWith('## ')) return { kind: 'h2', text: trimmed.slice(3) }
		if (trimmed.startsWith('# ')) return { kind: 'h1', text: trimmed.slice(2) }
		if (trimmed.startsWith('- ') || trimmed.startsWith('* '))
			return { kind: 'li', text: trimmed.slice(2) }
		if (trimmed.startsWith('> ')) return { kind: 'quote', text: trimmed.slice(2) }
		return { kind: 'p', text: trimmed }
	})
})
</script>

<template>
	<div class="mx-auto flex max-w-200 flex-col gap-2 p-4">
		<template v-for="(block, index) in blocks" :key="index">
			<h1 v-if="block.kind === 'h1'" class="m-0 mt-4 text-2xl font-bold text-contrast">
				{{ block.text }}
			</h1>
			<h2 v-else-if="block.kind === 'h2'" class="m-0 mt-3 text-xl font-semibold text-contrast">
				{{ block.text }}
			</h2>
			<h3 v-else-if="block.kind === 'h3'" class="m-0 mt-2 text-lg font-semibold text-contrast">
				{{ block.text }}
			</h3>
			<li v-else-if="block.kind === 'li'" class="ml-4 list-disc text-sm text-secondary">
				{{ block.text }}
			</li>
			<blockquote
				v-else-if="block.kind === 'quote'"
				class="m-0 border-l-2 border-brand pl-3 text-sm text-secondary"
			>
				{{ block.text }}
			</blockquote>
			<p v-else class="m-0 text-sm leading-relaxed text-secondary">
				{{ block.text }}
			</p>
		</template>
	</div>
</template>
