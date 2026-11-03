import fs from 'node:fs'

const [manifestPath, tag, source = 'github'] = process.argv.slice(2)
const expectedVersion = tag?.replace(/^v/, '')

if (!manifestPath || !expectedVersion || !['github', 'cnb'].includes(source)) {
	throw new Error('Usage: node verify-update-manifest.mjs <latest.json> <version-tag> [github|cnb]')
}

const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'))

const repository = (process.env.GITHUB_REPOSITORY ?? 'mcyudream/YMCL-Axolotl').toLowerCase()

if (manifest.version !== expectedVersion) {
	throw new Error(`Manifest version ${manifest.version} does not match ${expectedVersion}`)
}

const requiredPlatforms = ['windows-x86_64']

if (Object.keys(manifest.platforms ?? {}).some((platform) => !requiredPlatforms.includes(platform))) {
	throw new Error('Only Windows updates are currently supported')
}

for (const platform of requiredPlatforms) {
	const update = manifest.platforms?.[platform]

	if (!update || typeof update.signature !== 'string' || update.signature.trim().length < 32) {
		throw new Error(`Missing signed update for ${platform}`)
	}

	const url = new URL(update.url)
	const pathname = decodeURIComponent(url.pathname).toLowerCase()
	const isExpectedUrl =
		url.protocol === 'https:' &&
		(source === 'github'
			? url.hostname === 'github.com' &&
				pathname.includes(`/${repository}/releases/download/`)
			: url.hostname === 'cnb.cool' &&
				pathname.includes(`/axlmc/axolotl/-/releases/download/${tag.toLowerCase()}/`))
	if (!isExpectedUrl) {
		throw new Error(`Unexpected ${source} update URL for ${platform}: ${update.url}`)
	}
}

console.log(`Verified signed ${source} updater manifest for ${expectedVersion}`)
