import { invoke } from '@tauri-apps/api/core'

export type ContentRefreshJob = {
	instanceId: string
	id: string
	state:
		| 'queued'
		| 'scanning'
		| 'enriching'
		| 'completed'
		| 'partial'
		| 'failed'
		| 'cancelled'
		| 'interrupted'
	total: number
	processed: number
	failed: number
	error: string | null
	updatedAt: number
	failures: { path: string; error: string }[]
}

export function isContentRefreshActive(job: ContentRefreshJob | null): boolean {
	return job != null && ['queued', 'scanning', 'enriching'].includes(job.state)
}

export function getContentRefresh(instanceId: string): Promise<ContentRefreshJob | null> {
	return invoke('plugin:instance|instance_get_content_refresh', { instanceId })
}

export function startContentRefresh(
	instanceId: string,
	retryFailed = false,
): Promise<ContentRefreshJob> {
	return invoke('plugin:instance|instance_start_content_refresh', { instanceId, retryFailed })
}

export function cancelContentRefresh(instanceId: string): Promise<void> {
	return invoke('plugin:instance|instance_cancel_content_refresh', { instanceId })
}
