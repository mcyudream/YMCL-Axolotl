import { ref, watch, type Ref } from 'vue'

/** Wait for a persistent copy before playback; a failed cache must not prevent streaming. */
export function useDomainHomeVideo(
	source: Ref<string | null>,
	cache: (url: string) => Promise<string>,
): Ref<string | null> {
	const playback = ref<string | null>(null)
	watch(
		source,
		async (url, _, onCleanup) => {
			let cancelled = false
			onCleanup(() => (cancelled = true))
			playback.value = null
			if (!url) return
			if (!/^https?:\/\//i.test(url)) {
				playback.value = url
				return
			}
			try {
				const local = await cache(url)
				if (!cancelled) playback.value = local
			} catch {
				if (!cancelled) playback.value = url
			}
		},
		{ immediate: true, flush: 'sync' },
	)
	return playback
}
