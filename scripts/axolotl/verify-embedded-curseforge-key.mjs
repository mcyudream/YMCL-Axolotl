import { readFile } from 'node:fs/promises'
import { pathToFileURL } from 'node:url'

export async function verifyEmbeddedCurseForgeKey(binaryPath, key) {
	if (!key?.trim()) throw new Error('CURSEFORGE_API_KEY is missing')
	const binary = await readFile(binaryPath)
	if (!binary.includes(Buffer.from(key))) {
		throw new Error('The production binary does not contain the configured CurseForge API key')
	}
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	const binary = process.argv[2]
	if (!binary) throw new Error('Usage: verify-embedded-curseforge-key.mjs <production-binary>')
	await verifyEmbeddedCurseForgeKey(binary, process.env.CURSEFORGE_API_KEY)
	console.log('Verified: the production binary contains the configured CurseForge API key.')
}
