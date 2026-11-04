import { pathToFileURL } from 'node:url'
import { updatePlatforms } from './publish-ymcl-platforms.mjs'

export const releaseSecrets = [
	'CURSEFORGE_API_KEY',
	'TAURI_SIGNING_PRIVATE_KEY',
	...updatePlatforms.map((platform) => platform.secret),
]

export function checkReleaseSecrets(environment = process.env) {
	for (const name of releaseSecrets) {
		if (!environment[name]?.trim()) throw new Error(`Missing required release secret: ${name}`)
	}
	for (const name of releaseSecrets.filter((name) => name !== 'TAURI_SIGNING_PRIVATE_KEY')) {
		if (/[\r\n]/.test(environment[name]))
			throw new Error(`Invalid multiline release secret: ${name}`)
	}
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	checkReleaseSecrets()
	console.log('Required build, signing and YuDream publish secrets are configured.')
}
