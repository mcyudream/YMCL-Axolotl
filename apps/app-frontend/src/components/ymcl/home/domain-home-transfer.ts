import type { DomainPackOperationProgress } from '../../../composables/useDomainPacks.ts'
import type { InstallJobSnapshot } from '../../../helpers/install.ts'
import { installProgressFraction } from '../../../helpers/install-progress.ts'
import type { LoadingBar } from '../../../helpers/state.ts'

/** Only show progress belonging to the operation started by this panel. */
export function domainHomeTransferProgress(
	operation: DomainPackOperationProgress | null,
	jobs: InstallJobSnapshot[],
	bars: LoadingBar[],
) {
	if (!operation) return { fraction: null, job: undefined }
	if (operation.stage === 'installing') {
		const job = jobs.find((candidate) => candidate.job_id === operation.jobId)
		return { fraction: job ? installProgressFraction(job) : null, job }
	}
	if (operation.stage !== 'downloading' && operation.stage !== 'updating') {
		return { fraction: null, job: undefined }
	}
	const candidates = bars.filter(
		(bar) =>
			bar.bar_type?.type === 'pack_file_download' &&
			bar.bar_type.pack_name === operation.packId &&
			(bar.bar_type.instance_id ?? '') ===
				(operation.stage === 'downloading' ? '' : (operation.instanceId ?? '')),
	)
	const bar = candidates.length === 1 ? candidates[0] : undefined
	const fraction =
		bar &&
		Number.isFinite(bar.current) &&
		Number.isFinite(bar.total) &&
		bar.current! >= 0 &&
		bar.total! > 0
			? Math.max(0, Math.min(1, bar.current! / bar.total!))
			: null
	return { fraction, job: undefined }
}
