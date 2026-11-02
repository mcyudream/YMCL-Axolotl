type CloseGuard = () => boolean | Promise<boolean>

const guards = new Set<CloseGuard>()
let pending: Promise<boolean> | undefined

export function registerAppCloseGuard(guard: CloseGuard): () => void {
	guards.add(guard)
	return () => guards.delete(guard)
}

export async function confirmAppClose(): Promise<boolean> {
	if (pending) return pending
	pending = (async () => {
		try {
			for (const guard of guards) {
				if (!(await guard())) return false
			}
			return true
		} catch (error) {
			console.error('Failed to prepare the app for closing', error)
			return false
		}
	})()
	try {
		return await pending
	} finally {
		pending = undefined
	}
}
