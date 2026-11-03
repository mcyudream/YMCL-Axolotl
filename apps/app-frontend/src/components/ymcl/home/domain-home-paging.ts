export type DomainHomePage = 0 | 1

export const DOMAIN_HOME_PAGE_TRANSITION_MS = 640

const WHEEL_TRANSITION_LOCK_MS = DOMAIN_HOME_PAGE_TRANSITION_MS + 80

interface WheelInput {
	deltaX: number
	deltaY: number
	deltaMode: number
	timestamp: number
	viewportHeight: number
	canTurn: boolean
}

/** Keep one wheel gesture with its initial scroller, including touchpad inertia. */
export function createDomainHomeWheelPager() {
	let lastTimestamp = -Infinity
	let lockedUntil = -Infinity
	let distance = 0
	let direction = 0
	let owner: 'content' | 'paging' | null = null

	return {
		lock(timestamp: number) {
			lastTimestamp = timestamp
			lockedUntil = timestamp + WHEEL_TRANSITION_LOCK_MS
			distance = 0
			owner = 'paging'
		},
		consume(input: WheelInput): { turn: -1 | 0 | 1; preventDefault: boolean } {
			if (!input.deltaY || Math.abs(input.deltaX) >= Math.abs(input.deltaY))
				return { turn: 0, preventDefault: false }
			if (input.timestamp - lastTimestamp > 220) {
				distance = 0
				direction = 0
				owner = null
			}
			lastTimestamp = input.timestamp
			if (owner === 'paging' || input.timestamp < lockedUntil) {
				owner = 'paging'
				return { turn: 0, preventDefault: true }
			}
			if (owner === 'content') return { turn: 0, preventDefault: false }
			if (!input.canTurn) {
				owner = 'content'
				return { turn: 0, preventDefault: false }
			}

			const nextDirection = Math.sign(input.deltaY)
			if (nextDirection !== direction) distance = 0
			direction = nextDirection
			const unit = input.deltaMode === 1 ? 16 : input.deltaMode === 2 ? input.viewportHeight : 1
			distance += Math.abs(input.deltaY) * unit
			if (distance < 48) return { turn: 0, preventDefault: true }

			owner = 'paging'
			lockedUntil = input.timestamp + WHEEL_TRANSITION_LOCK_MS
			return { turn: direction > 0 ? 1 : -1, preventDefault: true }
		},
	}
}

export function canTurnDomainHomePage(
	page: DomainHomePage,
	deltaY: number,
	scroll: { scrollTop: number; scrollHeight: number; clientHeight: number },
): boolean {
	return page === 0
		? deltaY > 0 && scroll.scrollTop + scroll.clientHeight >= scroll.scrollHeight - 2
		: deltaY < 0 && scroll.scrollTop <= 2
}
