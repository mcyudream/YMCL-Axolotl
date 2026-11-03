import { pathToFileURL } from 'node:url'
import { selectedUpdatePlatforms } from './publish-ymcl-platforms.mjs'

export const releaseSecrets = [
	'CURSEFORGE_API_KEY',
	'TAURI_SIGNING_PRIVATE_KEY',
	'YUDREAM_API_KEY',
	'SWUSTMC_API_KEY',
]

export function checkReleaseSecrets(environment = process.env) {
	const required = [
		'CURSEFORGE_API_KEY',
		'TAURI_SIGNING_PRIVATE_KEY',
		...selectedUpdatePlatforms(environment).map((platform) => platform.secret),
	]
	for (const name of required) {
		if (!environment[name]?.trim()) throw new Error(`Missing required release secret: ${name}`)
	}
	for (const name of required.filter((name) => name !== 'TAURI_SIGNING_PRIVATE_KEY')) {
		if (/[\r\n]/.test(environment[name]))
			throw new Error(`Invalid multiline release secret: ${name}`)
	}
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	checkReleaseSecrets()
	console.log('Required build, signing and selected-platform publish secrets are configured.')
}
