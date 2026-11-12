import type { GameInstance } from '../../../helpers/types.ts'
import type { YmclJoinPreview, YmclUpdateCheck } from '../../../helpers/ymcl.ts'

/** A server change invalidates both the preview and the subsequent local update check. */
export async function inspectDomainHomeServer(
	server: { serverId: string; instanceId?: string },
	api: {
		preview: (serverId: string) => Promise<YmclJoinPreview>
		instances: () => Promise<GameInstance[]>
		check: (instanceId: string) => Promise<YmclUpdateCheck>
	},
	current: () => boolean,
) {
	const [preview, instances] = await Promise.all([api.preview(server.serverId), api.instances()])
	if (!current()) return null
	const instanceId = preview.installed?.instance_id ?? server.instanceId
	const check = instanceId ? await api.check(instanceId) : null
	return current() ? { preview, instances, check } : null
}
