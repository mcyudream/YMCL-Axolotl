import { computed, ref, toValue, watch, type MaybeRefOrGetter, type Ref } from 'vue'

import { resolveDomainImageUrl } from '@/helpers/ymcl-domain-image'
import { useYmclStore } from '@/store/ymcl'

/**
 * Displayable URL for a domain image. Relative paths resolve against the
 * provided origin (or the active domain when omitted).
 */
export function useDomainImage(
	raw: MaybeRefOrGetter<string | null | undefined>,
	origin?: MaybeRefOrGetter<string | null | undefined>,
): Ref<string | null> {
	const ymclStore = useYmclStore()
	const displayUrl = ref<string | null>(null)

	watch(
		() => {
			const domainOrigin =
				origin === undefined ? (ymclStore.activeDomain?.origin ?? null) : toValue(origin)
			return [toValue(raw), domainOrigin] as const
		},
		([value, domainOrigin]) => {
			displayUrl.value = resolveDomainImageUrl(value, domainOrigin)
		},
		{ immediate: true },
	)

	return displayUrl
}

/** Active-domain origin helper for call sites that only need a computed. */
export function useActiveDomainOrigin() {
	const ymclStore = useYmclStore()
	return computed(() => ymclStore.activeDomain?.origin ?? null)
}
