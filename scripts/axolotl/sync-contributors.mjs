import fs from 'node:fs/promises'
import { existsSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const REPOSITORY = 'mcyudream/YMCL-Axolotl'
const OUTPUT_PATH = fileURLToPath(
	new URL('../../apps/app-frontend/src/data/about/contributors.json', import.meta.url),
)
const TEAM_OUTPUT_PATH = fileURLToPath(
	new URL('../../apps/app-frontend/src/data/about/team.json', import.meta.url),
)

/**
 * Current-repo (non-upstream) contributors for YMCL-Axolotl.
 * Everyone else in fork history is Axolotl / Modrinth upstream and is omitted.
 */
const CURRENT_REPO_CONTRIBUTORS = [
	{
		login: 'YDHusky',
		name: 'SiberianHusky',
		url: 'https://github.com/YDHusky',
		avatarUrl: 'https://github.com/YDHusky.png?size=96',
	},
]

function requestHeaders() {
	const headers = {
		Accept: 'application/vnd.github+json',
		'User-Agent': 'YMCL-Axolotl-Contributors-Sync',
	}
	const token = process.env.YMCL_GITHUB_TOKEN || process.env.GITHUB_TOKEN
	if (token) {
		headers.Authorization = `Bearer ${token}`
	}
	return headers
}

async function fetchContributorCounts() {
	const url = new URL(`https://api.github.com/repos/${REPOSITORY}/contributors`)
	url.searchParams.set('per_page', '100')

	const response = await fetch(url, {
		headers: requestHeaders(),
		signal: AbortSignal.timeout(30_000),
	})
	if (!response.ok) throw new Error(`HTTP ${response.status}`)
	const payload = await response.json()
	if (!Array.isArray(payload)) throw new Error('Contributors response was not an array')

	const counts = new Map()
	for (const entry of payload) {
		if (!entry || typeof entry.login !== 'string') continue
		if (entry.type === 'Bot' || /\[bot\]$/i.test(entry.login)) continue
		if (!Number.isInteger(entry.contributions) || entry.contributions < 1) continue
		counts.set(entry.login.toLowerCase(), entry.contributions)
	}
	return counts
}

function resolveContributors(counts) {
	return CURRENT_REPO_CONTRIBUTORS.map((person) => {
		const loginKey = person.login.toLowerCase()
		const nameKey = person.name.toLowerCase()
		const contributions = counts.get(loginKey) ?? counts.get(nameKey) ?? 1
		return {
			name: person.name,
			avatarUrl: person.avatarUrl,
			url: person.url,
			contributions,
		}
	}).sort((left, right) => right.contributions - left.contributions || left.name.localeCompare(right.name))
}

function buildTeam(contributors) {
	return contributors.map((entry) => ({
		name: entry.name,
		avatarUrl: entry.avatarUrl,
		url: entry.url,
	}))
}

async function writeIfChanged(path, data) {
	const nextText = `${JSON.stringify(data, null, '\t')}\n`
	const currentText = existsSync(path) ? await fs.readFile(path, 'utf8') : ''
	if (currentText === nextText) return false
	await fs.writeFile(path, nextText)
	return true
}

async function main() {
	let counts = new Map()
	try {
		counts = await fetchContributorCounts()
	} catch (error) {
		console.warn(`Unable to refresh contribution counts, using defaults: ${error.message}`)
	}

	const contributors = resolveContributors(counts)
	const team = buildTeam(contributors)

	const contributorsChanged = await writeIfChanged(OUTPUT_PATH, contributors)
	const teamChanged = await writeIfChanged(TEAM_OUTPUT_PATH, team)

	if (!contributorsChanged && !teamChanged) {
		console.log(`Current-repo contributors are up to date (${contributors.length} people).`)
		return
	}

	console.log(
		`Synchronized ${contributors.length} current-repo contributor(s) from ${REPOSITORY} (upstream omitted).`,
	)
}
await main()
