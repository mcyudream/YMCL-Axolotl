import { defineStore } from 'pinia'

import type { DomainPackOperationProgress, DomainPackOperationResult } from './useDomainPacks.ts'

export interface DomainPackOperationScope {
	domainId: string
	userId: string | null
	serverId: string
}

interface OperationHandle {
	key: string
	id: number
}

interface OperationSnapshot {
	id: number
	running: boolean
	progress: DomainPackOperationProgress
	result: DomainPackOperationResult | null
}

function scopeKey(scope: DomainPackOperationScope): string {
	return JSON.stringify([scope.domainId, scope.userId, scope.serverId])
}

/** Operations outlive their views; install jobs remain the authority for byte progress. */
export const useDomainPackOperations = defineStore('domainPackOperations', {
	state: () => ({
		nextId: 0,
		operations: {} as Record<string, OperationSnapshot>,
	}),
	getters: {
		operationFor: (state) => (scope: DomainPackOperationScope) =>
			state.operations[scopeKey(scope)] ?? null,
	},
	actions: {
		begin(
			scope: DomainPackOperationScope,
			progress: DomainPackOperationProgress,
		): OperationHandle | null {
			const key = scopeKey(scope)
			if (this.operations[key]?.running) return null
			const id = ++this.nextId
			this.operations[key] = { id, running: true, progress, result: null }
			return { key, id }
		},
		advance(handle: OperationHandle, progress: DomainPackOperationProgress) {
			const operation = this.operations[handle.key]
			if (operation?.id !== handle.id || !operation.running) return
			operation.progress = progress
		},
		finish(handle: OperationHandle, result: DomainPackOperationResult) {
			const operation = this.operations[handle.key]
			if (operation?.id !== handle.id || !operation.running) return
			operation.result = result
			operation.running = false
		},
	},
})
