<template>
	<transition name="fade">
		<Teleport to="body">
			<div
				v-show="shown"
				ref="contextMenu"
				class="context-menu"
				:style="{
					left: left,
					top: top,
				}"
			>
				<template v-if="modernMode">
					<div
						v-for="(option, index) in modernOptions"
						:key="option.id ?? index"
						@click.stop="modernOptionClicked(option)"
					>
						<hr v-if="option.type === 'divider'" class="divider" />
						<div v-else class="item clickable" :class="option.tone === 'red' ? 'red' : 'base'">
							<slot :name="option.id" :option="option">
								<component :is="option.icon" v-if="option.icon" aria-hidden="true" />
								{{ option.label }}
							</slot>
						</div>
					</div>
				</template>
				<template v-else>
					<div
						v-for="(option, index) in options"
						:key="index"
						@click.stop="optionClicked(option.name)"
					>
						<hr v-if="option.type === 'divider'" class="divider" />
						<div
							v-else-if="!(isInstanceLink(item) && option.name === `add_content`)"
							class="item clickable"
							:class="[option.color ?? 'base']"
						>
							<slot :name="option.name" />
						</div>
					</div>
				</template>
			</div>
		</Teleport>
	</transition>
</template>

<script setup>
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue'

const emit = defineEmits(['menu-closed', 'option-clicked'])

const item = ref(null)
const contextMenu = ref(null)
const options = ref([])
const left = ref('0px')
const top = ref('0px')
const shown = ref(false)
const modernOptions = ref([])
const modernMode = ref(false)

const positionMenu = (event) => {
	const menuWidth = contextMenu.value?.clientWidth || 200
	const menuHeight = contextMenu.value?.clientHeight || 100
	const minFromEdge = 10
	const x = event.clientX ?? event.pageX
	const y = event.clientY ?? event.pageY

	// Context menus are fixed to the viewport. Flip to the opposite side when
	// there is not enough room on the right/bottom instead of allowing them to
	// be clipped or compressed by a parent layout.
	left.value =
		x + menuWidth + minFromEdge >= window.innerWidth
			? Math.max(minFromEdge, x - menuWidth - minFromEdge) + 'px'
			: Math.max(minFromEdge, x + minFromEdge) + 'px'
	top.value =
		y + menuHeight + minFromEdge >= window.innerHeight
			? Math.max(minFromEdge, y - menuHeight - minFromEdge) + 'px'
			: Math.max(minFromEdge, y + minFromEdge) + 'px'
}

defineExpose({
	open: (event, passedOptions) => {
		modernMode.value = true
		modernOptions.value = passedOptions
		shown.value = true
		nextTick(() => {
			positionMenu(event)
		})
	},
	close: () => hideContextMenu(),
	showMenu: (event, passedItem, passedOptions) => {
		modernMode.value = false
		modernOptions.value = []
		item.value = passedItem
		options.value = passedOptions

		// show to get dimensions
		shown.value = true

		// then, adjust position if overflowing
		nextTick(() => positionMenu(event))
	},
})

const isInstanceLink = (item) => {
	if (item.instance != undefined && item.instance.link) {
		return true
	} else if (item != undefined && item.link) {
		return true
	}
	return false
}

const hideContextMenu = () => {
	modernMode.value = false
	modernOptions.value = []
	shown.value = false
	emit('menu-closed')
}

const modernOptionClicked = (option) => {
	if (typeof option.action === 'function') option.action()
	if (!option.remainOpen) hideContextMenu()
}

const optionClicked = (option) => {
	emit('option-clicked', {
		item: item.value,
		option: option,
	})
	hideContextMenu()
}

const onEscKeyRelease = (event) => {
	if (event.keyCode === 27) {
		hideContextMenu()
	}
}

const handleClickOutside = (event) => {
	const elements = document.elementsFromPoint(event.clientX, event.clientY)
	if (
		contextMenu.value &&
		contextMenu.value !== event.target &&
		!elements.includes(contextMenu.value)
	) {
		hideContextMenu()
	}
}

onMounted(() => {
	window.addEventListener('click', handleClickOutside)
	document.body.addEventListener('keyup', onEscKeyRelease)
})

onBeforeUnmount(() => {
	window.removeEventListener('click', handleClickOutside)
	document.body.removeEventListener('keyup', onEscKeyRelease)
})
</script>

<style lang="scss" scoped>
.context-menu {
	background-color: var(--color-raised-bg);
	border-radius: var(--radius-md);
	box-shadow: var(--shadow-floating);
	border: 1px solid var(--color-divider);
	margin: 0;
	position: fixed;
	z-index: 1000000;
	overflow: hidden;
	padding: var(--gap-sm);

	.item {
		align-items: center;
		color: var(--color-base);
		cursor: pointer;
		display: flex;
		gap: var(--gap-sm);
		padding: var(--gap-sm);
		border-radius: var(--radius-sm);

		&:hover,
		&:active {
			&.base {
				background-color: var(--color-button-bg);
				color: var(--color-contrast);
			}

			&.primary {
				background-color: var(--color-brand);
				color: var(--color-accent-contrast);
				font-weight: bold;
			}

			&.danger {
				background-color: var(--color-red);
				color: var(--color-accent-contrast);
				font-weight: bold;
			}

			&.contrast {
				background-color: var(--color-orange);
				color: var(--color-accent-contrast);
				font-weight: bold;
			}
		}
	}

	.divider {
		border: 1px solid var(--color-divider);
		margin: var(--gap-sm);
		pointer-events: none;
	}
}

.fade-enter-active,
.fade-leave-active {
	transition: opacity 0.2s ease-in-out;
}

.fade-enter-from,
.fade-leave-to {
	opacity: 0;
}
</style>
