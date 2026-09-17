import contributorsData from './contributors.json'
import teamData from './team.json'

export interface TeamMember {
	name: string
	avatar?: string
	avatarUrl?: string
	url?: string
	experience?: string
}

export interface Contributor {
	name: string
	avatarUrl: string
	url: string
	contributions: number
}

const teamAvatarModules = import.meta.glob('./avatars/*', {
	eager: true,
	import: 'default',
	query: '?url',
}) as Record<string, string>

export const teamMembers: (TeamMember & { avatarUrl: string })[] = teamData.map((member) => ({
	...member,
	avatarUrl:
		member.avatarUrl ||
		(member.avatar ? teamAvatarModules[`./avatars/${member.avatar}`] : undefined) ||
		'',
}))

export const contributors = contributorsData as Contributor[]
