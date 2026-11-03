import { createHash } from 'node:crypto'
import { createWriteStream, openAsBlob } from 'node:fs'
import { mkdtemp, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { basename, join } from 'node:path'
import { Readable, Transform } from 'node:stream'
import { pipeline } from 'node:stream/promises'
import { pathToFileURL } from 'node:url'

export const updatePlatforms = [
	{ origin: 'https://www.yudream.online', secret: 'YUDREAM_API_KEY' },
	{ origin: 'https://www.swustmc.cn', secret: 'SWUSTMC_API_KEY' },
]

export function selectedUpdatePlatforms(environment = process.env) {
	const selection = environment.YMCL_UPDATE_PLATFORMS ?? 'both'
	if (selection === 'both') return updatePlatforms
	if (selection === 'yudream') return [updatePlatforms[0]]
	throw new Error('Invalid update platform selection: expected both or yudream')
}

const pluginPath = '/api/plugins/ymcl-content'
const expectedTargets = [
	'darwin-aarch64',
	'darwin-x86_64',
	'linux-aarch64',
	'linux-x86_64',
	'windows-x86_64',
]

function failureDetails(error, secrets = []) {
	const seen = new Set()
	const details = []
	function visit(value) {
		if (!value || seen.has(value) || seen.size >= 8) return
		seen.add(value)
		let message = `${value.name ?? 'Error'}${value.code ? ` [${value.code}]` : ''}: ${value.message ?? String(value)}`
		for (const secret of secrets.filter(Boolean)) message = message.replaceAll(secret, '[redacted]')
		details.push(message)
		visit(value.cause)
		for (const nested of value.errors ?? []) visit(nested)
	}
	visit(error)
	return details.join(' <- ')
}

export async function prepareRelease(catalog, release, tag, signaturesDirectory) {
	const version = tag.replace(/^v/, '')
	if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version))
		throw new Error('Invalid release version')
	if (
		catalog.version !== version ||
		release.tag_name !== tag ||
		release.draft ||
		!release.published_at
	) {
		throw new Error('A published GitHub release and matching artifact catalog are required')
	}
	if (!Array.isArray(catalog.artifacts) || !catalog.artifacts.length)
		throw new Error('No release artifacts')
	const repository = process.env.GITHUB_REPOSITORY ?? 'mcyudream/YMCL-Axolotl'
	const assetsPrefix = `https://github.com/${repository}/releases/download/${tag}/`
	const targets = new Set()
	const filenames = new Set()
	const artifacts = await Promise.all(
		catalog.artifacts.map(async (artifact) => {
			if (
				!artifact.filename ||
				basename(artifact.filename) !== artifact.filename ||
				filenames.has(artifact.filename)
			) {
				throw new Error('Unsafe or duplicate release filename')
			}
			filenames.add(artifact.filename)
			if (
				!/^[a-f0-9]{64}$/i.test(artifact.sha256) ||
				!Number.isSafeInteger(artifact.size) ||
				artifact.size <= 0
			) {
				throw new Error(`Invalid digest or size for ${artifact.filename}`)
			}
			if (artifact.downloadUrl !== `${assetsPrefix}${artifact.filename}`) {
				throw new Error(`Unexpected release download origin for ${artifact.filename}`)
			}
			let signature = null
			if (artifact.signatureFilename) {
				if (artifact.signatureFilename !== `${artifact.filename}.sig`)
					throw new Error('Invalid signature filename')
				signature = (
					await readFile(join(signaturesDirectory, artifact.signatureFilename), 'utf8')
				).trim()
				if (!/^[A-Za-z0-9+/=]+$/.test(signature))
					throw new Error(`Invalid signature for ${artifact.filename}`)
			}
			if (artifact.kind === 'updater') {
				if (!signature) throw new Error(`Missing updater signature for ${artifact.filename}`)
				for (const target of artifact.targetPlatforms ?? []) {
					if (targets.has(target)) throw new Error(`Duplicate updater target ${target}`)
					targets.add(target)
				}
			}
			return { ...artifact, signature }
		}),
	)
	for (const target of expectedTargets) {
		if (!targets.has(target)) throw new Error(`Missing updater target ${target}`)
	}
	const publishedAt = Date.parse(release.published_at)
	if (!Number.isFinite(publishedAt)) throw new Error('Invalid release publication date')
	return {
		version,
		channel: version.includes('-') ? 'beta' : 'release',
		title: `YMCL ${version}`,
		notes: release.body ?? '',
		publishedAt: String(publishedAt),
		forceUpdate: false,
		externalUrl: release.html_url,
		artifacts,
	}
}

function matchesArtifact(actual, expected) {
	return (
		actual?.filename === expected.filename &&
		actual.sha256 === expected.sha256 &&
		(actual.size === expected.size || actual.size === String(expected.size)) &&
		actual.kind === expected.kind &&
		actual.platform === expected.platform &&
		actual.architecture === expected.architecture &&
		(actual.variant ?? '') === (expected.variant ?? '') &&
		(actual.signature ?? null) === expected.signature &&
		JSON.stringify(actual.targetPlatforms ?? []) === JSON.stringify(expected.targetPlatforms ?? [])
	)
}

export async function downloadArtifact(artifact, directory, fetcher = fetch) {
	const started = Date.now()
	console.log(`[GitHub] Downloading ${artifact.filename} (${artifact.size} bytes)`)
	try {
		const response = await fetcher(artifact.downloadUrl, {
			signal: AbortSignal.timeout(10 * 60_000),
		})
		if (!response.ok || !response.body)
			throw new Error(`Download failed for ${artifact.filename}: HTTP ${response.status}`)
		const destination = join(directory, artifact.filename)
		const hash = createHash('sha256')
		let bytes = 0
		await pipeline(
			Readable.fromWeb(response.body),
			new Transform({
				transform(chunk, _encoding, callback) {
					bytes += chunk.length
					if (bytes > artifact.size)
						return callback(new Error(`Oversized artifact ${artifact.filename}`))
					hash.update(chunk)
					callback(null, chunk)
				},
			}),
			createWriteStream(destination),
		)
		if (bytes !== artifact.size || hash.digest('hex') !== artifact.sha256) {
			throw new Error(`Digest or size mismatch for ${artifact.filename}`)
		}
		console.log(
			`[GitHub] Verified SHA-256 and size of ${artifact.filename} in ${Date.now() - started}ms`,
		)
		return destination
	} catch (error) {
		throw new Error(
			`GitHub download ${artifact.filename} failed after ${Date.now() - started}ms: ${failureDetails(error)}`,
		)
	}
}

export async function publishPlatform(platform, apiKey, release, artifactFile, fetcher = fetch) {
	const { origin, secret } = platform
	if (!updatePlatforms.some((site) => site.origin === origin))
		throw new Error('Unknown publish origin')
	if (!apiKey?.trim()) throw new Error(`${origin}: missing ${secret}`)
	const admin = `${pluginPath}/admin/update/releases`
	const releasePath = `${admin}/${encodeURIComponent(release.version)}`
	async function request(
		path,
		{ method = 'GET', body, publicRequest = false, optional = false } = {},
	) {
		const multipart = body instanceof FormData
		const started = Date.now()
		const context = `${origin}: ${method} ${path}`
		console.log(`[${new URL(origin).hostname}] ${method} ${path}`)
		try {
			const response = await fetcher(`${origin}${path}`, {
				method,
				headers: {
					Accept: 'application/json',
					...(!publicRequest ? { 'X-API-Key': apiKey } : {}),
					...(body && !multipart ? { 'Content-Type': 'application/json' } : {}),
				},
				...(body ? { body: multipart ? body : JSON.stringify(body) } : {}),
				signal: AbortSignal.timeout(multipart ? 10 * 60_000 : 30_000),
				redirect: 'error',
			})
			console.log(
				`[${new URL(origin).hostname}] HTTP ${response.status} in ${Date.now() - started}ms`,
			)
			if (optional && response.status === 404) return null
			if (!response.ok || response.status === 204) throw new Error(`HTTP ${response.status}`)
			const data = await response.json()
			if (data.code !== undefined && data.code !== 200 && data.code !== 0) {
				throw new Error('API rejected request')
			}
			return data.data ?? data
		} catch (error) {
			throw new Error(
				`${context} failed after ${Date.now() - started}ms: ${failureDetails(error, [apiKey])}`,
			)
		}
	}
	const existing = await request(releasePath, { optional: true })
	if (existing?.enabled) {
		if (
			existing.yanked ||
			existing.notes !== release.notes ||
			existing.artifacts?.length !== release.artifacts.length ||
			!release.artifacts.every((expected) =>
				existing.artifacts.some((actual) => matchesArtifact(actual, expected)),
			)
		) {
			throw new Error(`${origin}: refusing to overwrite an already published version`)
		}
	} else {
		const { artifacts: expectedArtifacts, ...metadata } = release
		let staged = await request(admin, { method: 'POST', body: { ...metadata, enabled: false } })
		for (const artifact of expectedArtifacts) {
			const previous = staged.artifacts?.find(
				(actual) => matchesArtifact(actual, artifact) && actual.hostedLocally,
			)
			if (previous) continue
			const file = await artifactFile(artifact)
			const body = new FormData()
			body.append('file', await openAsBlob(file), artifact.filename)
			for (const name of ['kind', 'variant', 'platform', 'architecture', 'filename', 'signature']) {
				if (artifact[name]) body.append(name, artifact[name])
			}
			body.append('id', artifact.filename)
			body.append('targetPlatforms', (artifact.targetPlatforms ?? []).join(','))
			staged = await request(`${releasePath}/artifacts/upload`, { method: 'POST', body })
			if (
				!staged.artifacts?.some(
					(actual) => matchesArtifact(actual, artifact) && actual.hostedLocally,
				)
			) {
				throw new Error(
					`${origin}: uploaded artifact integrity check failed for ${artifact.filename}`,
				)
			}
			console.log(`[${new URL(origin).hostname}] Uploaded ${artifact.filename}`)
		}
		const artifacts = expectedArtifacts.map((expected) => {
			const actual = staged.artifacts.find((candidate) => matchesArtifact(candidate, expected))
			if (!actual?.fileKey) throw new Error(`${origin}: artifact is not hosted locally`)
			return {
				...actual,
				downloadUrl: `${origin}${pluginPath}/v1/update/files/${encodeURIComponent(release.version)}/${encodeURIComponent(expected.filename)}`,
			}
		})
		await request(admin, { method: 'POST', body: { ...metadata, artifacts, enabled: true } })
	}
	const manifest = await request(`${pluginPath}/v1/update/manifest?channel=${release.channel}`, {
		publicRequest: true,
	})
	if (manifest.version !== release.version)
		throw new Error(`${origin}: public manifest version mismatch`)
	for (const artifact of release.artifacts.filter((artifact) => artifact.kind === 'updater')) {
		for (const target of artifact.targetPlatforms) {
			const actual = manifest.platforms?.[target]
			const expectedUrl = `${origin}${pluginPath}/v1/update/files/${encodeURIComponent(release.version)}/${encodeURIComponent(artifact.filename)}`
			if (actual?.signature !== artifact.signature || actual.url !== expectedUrl) {
				throw new Error(
					`${origin}: public updater signature or download URL mismatch for ${target}`,
				)
			}
		}
	}
	console.log(`[${new URL(origin).hostname}] Published and verified ${release.version}`)
}

export async function publishPlatforms(release, environment = process.env, fetcher = fetch) {
	const platforms = selectedUpdatePlatforms(environment)
	const directory = await mkdtemp(join(tmpdir(), 'ymcl-release-upload-'))
	const downloads = new Map()
	const artifactFile = (artifact) => {
		if (!downloads.has(artifact.filename))
			downloads.set(artifact.filename, downloadArtifact(artifact, directory, fetcher))
		return downloads.get(artifact.filename)
	}
	try {
		const results = await Promise.allSettled(
			platforms.map((platform) =>
				publishPlatform(platform, environment[platform.secret], release, artifactFile, fetcher),
			),
		)
		const errors = results.flatMap((result, index) =>
			result.status === 'rejected'
				? [
						`${platforms[index].origin}: ${failureDetails(
							result.reason,
							updatePlatforms.map((platform) => environment[platform.secret]),
						)}`,
					]
				: [],
		)
		if (errors.length) throw new Error(errors.join('\n'))
	} finally {
		await rm(directory, { recursive: true, force: true })
	}
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	const [catalogPath, releasePath, signaturesDirectory, tag] = process.argv.slice(2)
	if (!catalogPath || !releasePath || !signaturesDirectory || !tag) {
		throw new Error(
			'Usage: publish-ymcl-platforms.mjs <catalog.json> <release.json> <signatures-directory> <version-tag>',
		)
	}
	const release = await prepareRelease(
		JSON.parse(await readFile(catalogPath, 'utf8')),
		JSON.parse(await readFile(releasePath, 'utf8')),
		tag,
		signaturesDirectory,
	)
	await publishPlatforms(release)
}
