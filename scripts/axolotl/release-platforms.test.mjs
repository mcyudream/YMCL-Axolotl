import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { createServer } from 'node:http'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'
import { checkReleaseSecrets, releaseSecrets } from './check-release-secrets.mjs'
import { verifyEmbeddedCurseForgeKey } from './verify-embedded-curseforge-key.mjs'
import {
	downloadArtifact,
	prepareRelease,
	publishPlatforms,
	publishPlatform,
	updatePlatforms,
	uploadMultipart,
} from './publish-ymcl-platforms.mjs'

const version = '1.9.7-beta.10'
const tag = `v${version}`
const signature = Buffer.from('test signature').toString('base64')
const bytes = Buffer.from('test release binary')
const sha256 = createHash('sha256').update(bytes).digest('hex')
const artifacts = [[`YMCL_${version}_x64-setup.nsis.zip`, ['windows-x86_64']]].map(
	([filename, targetPlatforms]) => ({
		filename,
		targetPlatforms,
		kind: 'updater',
		variant: 'tauri',
		platform: targetPlatforms[0],
		architecture: filename.includes('universal') ? 'universal' : 'x86_64',
		signatureFilename: `${filename}.sig`,
		signature,
		size: bytes.length,
		sha256,
		downloadUrl: `https://github.com/mcyudream/YMCL-Axolotl/releases/download/${tag}/${filename}`,
	}),
)
const release = {
	version,
	channel: 'beta',
	title: `YMCL ${version}`,
	notes: 'Release notes.',
	publishedAt: '1793577600000',
	forceUpdate: false,
	externalUrl: `https://github.com/mcyudream/YMCL-Axolotl/releases/tag/${tag}`,
	artifacts,
}

async function withUploadServer(handler, run) {
	const errors = []
	const server = createServer(async (request, response) => {
		try {
			const chunks = []
			for await (const chunk of request) chunks.push(chunk)
			const body = await new Response(Buffer.concat(chunks), {
				headers: { 'Content-Type': request.headers['content-type'] },
			}).formData()
			await handler(request, response, body)
		} catch (error) {
			errors.push(error)
			response.writeHead(500).end('{}')
		}
	})
	await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
	try {
		await run(`http://127.0.0.1:${server.address().port}/upload`)
		assert.deepEqual(errors, [])
	} finally {
		server.closeAllConnections()
		await new Promise((resolve) => server.close(resolve))
	}
}

test('multipart uploads retry resets and transient HTTP failures without losing file data', async () => {
	const directory = await mkdtemp(join(tmpdir(), 'ymcl-upload-transport-test-'))
	try {
		const file = join(directory, 'binary; "quoted".zip')
		await writeFile(file, bytes)
		const body = new FormData()
		body.append('file', new Blob([bytes]), 'binary; "quoted".zip')
		body.append('id', 'test-artifact')
		body.append('signature', signature)
		let requests = 0
		await withUploadServer(
			async (request, response, uploaded) => {
				requests++
				assert.equal(request.headers['x-api-key'], 'test-upload-key')
				assert.equal(uploaded.get('id'), 'test-artifact')
				assert.equal(uploaded.get('signature'), signature)
				assert.equal(uploaded.get('file').name, 'binary; "quoted".zip')
				assert.deepEqual(Buffer.from(await uploaded.get('file').arrayBuffer()), bytes)
				if (requests === 1) request.socket.destroy()
				else response.writeHead(requests === 2 ? 503 : 200).end('{"uploaded":true}')
			},
			async (url) => {
				const response = await uploadMultipart(url, 'test-upload-key', body, file)
				assert.equal(response.status, 200)
				assert.deepEqual(await response.json(), { uploaded: true })
			},
		)
		assert.equal(requests, 3)
	} finally {
		await rm(directory, { recursive: true, force: true })
	}
})

test('multipart uploads reject invalid credentials and do not retry auth errors or follow redirects', async () => {
	const directory = await mkdtemp(join(tmpdir(), 'ymcl-upload-rejection-test-'))
	try {
		const file = join(directory, 'binary.zip')
		await writeFile(file, bytes)
		const body = new FormData()
		body.append('file', new Blob([bytes]), 'binary.zip')
		let requests = 0
		await withUploadServer(
			(request, response) => {
				requests++
				assert.equal(request.url, '/upload')
				response.writeHead(requests === 1 ? 403 : 302, { Location: '/redirect-target' }).end('{}')
			},
			async (url) => {
				await assert.rejects(uploadMultipart(url, 'invalid\ncredential', body, file), /credential/)
				assert.equal(requests, 0)
				assert.equal((await uploadMultipart(url, 'test-upload-key', body, file)).status, 403)
				assert.equal((await uploadMultipart(url, 'test-upload-key', body, file)).status, 302)
			},
		)
		assert.equal(requests, 2)
	} finally {
		await rm(directory, { recursive: true, force: true })
	}
})

test('production secrets fail closed without printing secret values', () => {
	const environment = Object.fromEntries(releaseSecrets.map((name) => [name, 'test-private-value']))
	assert.doesNotThrow(() => checkReleaseSecrets(environment))
	for (const name of releaseSecrets) {
		assert.throws(() => checkReleaseSecrets({ ...environment, [name]: ' \t' }), new RegExp(name))
	}
	assert.throws(
		() => checkReleaseSecrets({ ...environment, CURSEFORGE_API_KEY: 'key\nsecond line' }),
		/multiline/,
	)
})

test('release publication requires signing and YuDream credentials only', () => {
	const environment = {
		CURSEFORGE_API_KEY: 'test-curseforge',
		TAURI_SIGNING_PRIVATE_KEY: 'test-signing-key',
		YUDREAM_API_KEY: 'test-yudream',
	}
	assert.doesNotThrow(() => checkReleaseSecrets(environment))
	for (const name of ['CURSEFORGE_API_KEY', 'TAURI_SIGNING_PRIVATE_KEY', 'YUDREAM_API_KEY']) {
		assert.throws(() => checkReleaseSecrets({ ...environment, [name]: '' }), new RegExp(name))
	}
	assert.throws(
		() => checkReleaseSecrets({ ...environment, YUDREAM_API_KEY: 'test\nkey' }),
		/multiline/,
	)
})

test('checks the actual binary for the literal embedded key', async () => {
	const directory = await mkdtemp(join(tmpdir(), 'ymcl-embedded-key-test-'))
	try {
		const binary = join(directory, 'YMCL')
		await writeFile(binary, 'binary\0$2a$10$fake-key\0suffix')
		await verifyEmbeddedCurseForgeKey(binary, '$2a$10$fake-key')
		await assert.rejects(verifyEmbeddedCurseForgeKey(binary, 'absent'), /does not contain/)
		await assert.rejects(verifyEmbeddedCurseForgeKey(binary, '  '), /missing/)
	} finally {
		await rm(directory, { recursive: true, force: true })
	}
})

test('release preparation requires all signed update targets and trusted download paths', async () => {
	const directory = await mkdtemp(join(tmpdir(), 'ymcl-release-metadata-test-'))
	try {
		for (const artifact of artifacts)
			await writeFile(join(directory, artifact.signatureFilename), signature)
		const metadata = {
			tag_name: tag,
			published_at: '2026-11-02T00:00:00Z',
			body: release.notes,
			html_url: release.externalUrl,
		}
		const catalog = { version, artifacts }
		const prepared = await prepareRelease(catalog, metadata, tag, directory)
		assert.equal(prepared.artifacts.length, 1)
		assert.equal(prepared.channel, 'beta')
		await assert.rejects(
			prepareRelease(catalog, { ...metadata, draft: true }, tag, directory),
			/published/,
		)
		await assert.rejects(
			prepareRelease(
				{ ...catalog, artifacts: [{ ...artifacts[0], targetPlatforms: [] }] },
				metadata,
				tag,
				directory,
			),
			/Missing updater target/,
		)
		await assert.rejects(
			prepareRelease(
				{
					...catalog,
					artifacts: [{ ...artifacts[0], downloadUrl: 'https://untrusted.example/file' }],
				},
				metadata,
				tag,
				directory,
			),
			/download origin/,
		)
		await assert.rejects(
			prepareRelease(
				{ ...catalog, artifacts: [{ ...artifacts[0], signatureFilename: '../private.sig' }] },
				metadata,
				tag,
				directory,
			),
			/signature filename/,
		)
	} finally {
		await rm(directory, { recursive: true, force: true })
	}
})

function platformMock({
	corrupt = false,
	stringSizes = false,
	portableUrl,
	resetUpload = false,
	uploadStatus,
} = {}) {
	const state = new Map()
	const requests = []
	let downloads = 0
	const fetcher = async (url, options = {}) => {
		if (url.startsWith('https://github.com/')) {
			downloads++
			return new Response(bytes)
		}
		const { origin, pathname } = new URL(url)
		requests.push({ origin, pathname, method: options.method, headers: options.headers })
		let record = state.get(origin)
		if (pathname.endsWith('/manifest')) {
			assert.equal(options.headers['X-API-Key'], undefined)
			assert.equal(record?.enabled, true)
			const platforms = {}
			for (const artifact of record.artifacts)
				for (const target of artifact.targetPlatforms) {
					platforms[target] = { signature: artifact.signature, url: artifact.downloadUrl }
				}
			return Response.json({ version, platforms })
		}
		assert.ok(options.headers['X-API-Key'])
		if (options.method === 'GET') {
			return record
				? Response.json({ code: 200, data: record })
				: Response.json({}, { status: 404 })
		}
		if (options.method === 'POST' && pathname.endsWith('/artifacts')) {
			const artifact = JSON.parse(options.body)
			record.artifacts = [
				...record.artifacts.filter((item) => item.filename !== artifact.filename),
				{
					...artifact,
					hostedLocally: false,
					fileKey: null,
					downloadUrl: portableUrl ?? artifact.downloadUrl,
				},
			]
		} else if (options.body instanceof FormData) {
			assert.equal(record.enabled, false)
			if (resetUpload)
				throw new TypeError('fetch failed', {
					cause: Object.assign(new Error('connection reset'), { code: 'ECONNRESET' }),
				})
			if (uploadStatus) return Response.json({ message: 'upload failed' }, { status: uploadStatus })
			const body = options.body
			assert.deepEqual(Buffer.from(await body.get('file').arrayBuffer()), bytes)
			const uploaded = Object.fromEntries(
				['id', 'kind', 'variant', 'platform', 'architecture', 'filename', 'signature'].map(
					(name) => [name, body.get(name)],
				),
			)
			uploaded.targetPlatforms = body.get('targetPlatforms').split(',')
			Object.assign(uploaded, {
				size: stringSizes ? String(bytes.length) : bytes.length,
				sha256: corrupt ? '0'.repeat(64) : sha256,
				hostedLocally: true,
				fileKey: `files/${uploaded.filename}`,
			})
			record.artifacts = [
				...record.artifacts.filter((artifact) => artifact.filename !== uploaded.filename),
				uploaded,
			]
		} else {
			const body = JSON.parse(options.body)
			record = { ...record, ...body, artifacts: body.artifacts ?? record?.artifacts ?? [] }
			state.set(origin, record)
		}
		return Response.json({ code: 200, data: record })
	}
	return { fetcher, state, requests, downloads: () => downloads }
}

test('portable downloads register the versioned GitHub URL without a second large upload', async () => {
	const portable = {
		filename: `YMCL_${version}_x64_portable.zip`,
		kind: 'portable',
		variant: 'portable',
		platform: 'windows',
		architecture: 'x86_64',
		targetPlatforms: [],
		size: 390_923_099,
		sha256: 'b'.repeat(64),
		downloadUrl: `https://github.com/mcyudream/YMCL-Axolotl/releases/download/${tag}/YMCL_${version}_x64_portable.zip`,
	}
	const value = { ...release, artifacts: [...artifacts, portable] }
	const mock = platformMock()
	await publishPlatforms(value, { YUDREAM_API_KEY: 'test-key' }, mock.fetcher)
	assert.equal(mock.downloads(), artifacts.length)
	const published = mock.state.get(updatePlatforms[0].origin)
	const stored = published.artifacts.find((artifact) => artifact.kind === 'portable')
	assert.equal(stored.downloadUrl, portable.downloadUrl)
	assert.equal(stored.sha256, portable.sha256)
	assert.equal(stored.size, portable.size)
	assert.equal(stored.hostedLocally, false)
	assert.ok(published.artifacts.find((artifact) => artifact.kind === 'updater').hostedLocally)
	await publishPlatforms(value, { YUDREAM_API_KEY: 'test-key' }, mock.fetcher)
	assert.equal(mock.downloads(), artifacts.length)
	const invalid = platformMock({ portableUrl: 'https://untrusted.example/portable.zip' })
	await assert.rejects(
		publishPlatforms(value, { YUDREAM_API_KEY: 'test-key' }, invalid.fetcher),
		/portable download URL or metadata mismatch/,
	)
	assert.equal(invalid.state.get(updatePlatforms[0].origin).enabled, false)
	const rejected = platformMock({ uploadStatus: 554 })
	await publishPlatforms(release, { YUDREAM_API_KEY: 'test-yudream' }, rejected.fetcher)
	assert.equal(rejected.state.get(updatePlatforms[0].origin).enabled, true)
})

test('uploads to YuDream and safely reuses verified published artifacts', async () => {
	const mock = platformMock()
	const environment = { YUDREAM_API_KEY: 'test-yudream' }
	await publishPlatforms(release, environment, mock.fetcher)
	assert.equal(mock.downloads(), artifacts.length)
	assert.deepEqual([...mock.state.keys()], ['https://www.yudream.online'])
	for (const platform of updatePlatforms) {
		const stored = mock.state.get(platform.origin)
		assert.equal(stored.enabled, true)
		assert.equal(stored.artifacts.length, artifacts.length)
		assert.ok(
			stored.artifacts.every(
				(artifact) => artifact.fileKey && artifact.downloadUrl.startsWith(`${platform.origin}/`),
			),
		)
	}
	const writes = mock.requests.filter((request) => request.method === 'POST').length
	await publishPlatforms(release, environment, mock.fetcher)
	assert.equal(mock.downloads(), artifacts.length)
	assert.equal(mock.requests.filter((request) => request.method === 'POST').length, writes)
	await assert.rejects(
		publishPlatforms({ ...release, notes: 'Changed published notes.' }, environment, mock.fetcher),
		/refusing to overwrite/,
	)
})

test('failed integrity verification leaves the staged release disabled', async () => {
	const directory = await mkdtemp(join(tmpdir(), 'ymcl-upload-failure-test-'))
	try {
		const binary = join(directory, 'test-binary')
		await writeFile(binary, bytes)
		const mock = platformMock({ corrupt: true })
		await assert.rejects(
			publishPlatform(updatePlatforms[0], 'test-key', release, async () => binary, mock.fetcher),
			/integrity/,
		)
		assert.equal(mock.state.get(updatePlatforms[0].origin).enabled, false)
	} finally {
		await rm(directory, { recursive: true, force: true })
	}
})

test('accepts canonical string sizes from the servers and reuses staged uploads', async () => {
	const mock = platformMock({ stringSizes: true })
	const environment = { YUDREAM_API_KEY: 'test-yudream' }
	await publishPlatforms(release, environment, mock.fetcher)
	assert.equal(mock.downloads(), artifacts.length)
	for (const platform of updatePlatforms) mock.state.get(platform.origin).enabled = false
	await publishPlatforms(release, environment, mock.fetcher)
	assert.equal(mock.downloads(), artifacts.length)
	await publishPlatforms(release, environment, mock.fetcher)
	assert.equal(mock.downloads(), artifacts.length)
	for (const platform of updatePlatforms) {
		mock.state.get(platform.origin).artifacts[0].size = String(bytes.length + 1)
	}
	await assert.rejects(
		publishPlatforms(release, environment, mock.fetcher),
		/refusing to overwrite/,
	)
})

test('registers the signed GitHub updater URL after the platform resets a large upload', async () => {
	const mock = platformMock({ resetUpload: true })
	await publishPlatforms(release, { YUDREAM_API_KEY: 'test-yudream' }, mock.fetcher)
	const published = mock.state.get(updatePlatforms[0].origin)
	const updater = published.artifacts.find((artifact) => artifact.kind === 'updater')
	assert.equal(updater.hostedLocally, false)
	assert.equal(updater.downloadUrl, artifacts[0].downloadUrl)
	assert.equal(updater.signature, signature)
	assert.equal(updater.sha256, sha256)
	assert.equal(published.enabled, true)
	assert.equal(
		mock.requests.filter((request) => request.pathname.endsWith('/artifacts/upload')).length,
		1,
	)
	const invalid = platformMock({
		resetUpload: true,
		portableUrl: 'https://untrusted.example/update.zip',
	})
	await assert.rejects(
		publishPlatforms(release, { YUDREAM_API_KEY: 'test-yudream' }, invalid.fetcher),
		/updater download URL or metadata mismatch/,
	)
	assert.equal(invalid.state.get(updatePlatforms[0].origin).enabled, false)
})

test('missing YuDream credentials fail before sending requests or downloading artifacts', async () => {
	const mock = platformMock()
	await assert.rejects(publishPlatforms(release, {}, mock.fetcher), /missing YUDREAM_API_KEY/)
	assert.equal(mock.requests.length, 0)
	assert.equal(mock.downloads(), 0)
})

test('retired update platform rejects publication before sending credentials', async () => {
	const mock = platformMock()
	await assert.rejects(
		publishPlatform(
			{ origin: 'https://www.swustmc.cn', secret: 'SWUSTMC_API_KEY' },
			'test-private-key',
			release,
			async () => {
				throw new Error('Unexpected artifact download')
			},
			mock.fetcher,
		),
		/Unknown publish origin/,
	)
	assert.equal(mock.requests.length, 0)
	assert.equal(mock.downloads(), 0)
})

test('downloads are rejected when the digest or expected size is wrong', async () => {
	const directory = await mkdtemp(join(tmpdir(), 'ymcl-download-integrity-test-'))
	try {
		const file = await downloadArtifact(artifacts[0], directory, async () => new Response(bytes))
		assert.deepEqual(await readFile(file), bytes)
		await assert.rejects(
			downloadArtifact(
				{ ...artifacts[0], sha256: '0'.repeat(64) },
				directory,
				async () => new Response(bytes),
			),
			/mismatch/,
		)
		await assert.rejects(
			downloadArtifact({ ...artifacts[0], size: 1 }, directory, async () => new Response(bytes)),
			/Oversized/,
		)
	} finally {
		await rm(directory, { recursive: true, force: true })
	}
})

test('network failures report the site, request, nested cause and elapsed time without keys', async () => {
	const mock = platformMock()
	const environment = { YUDREAM_API_KEY: 'test-private-key' }
	const fetcher = async (url, options) => {
		if (url.startsWith(updatePlatforms[0].origin)) {
			throw new TypeError('fetch failed', {
				cause: Object.assign(new Error('connection timed out test-private-key'), {
					code: 'UND_ERR_CONNECT_TIMEOUT',
				}),
			})
		}
		return mock.fetcher(url, options)
	}
	await assert.rejects(publishPlatforms(release, environment, fetcher), (error) => {
		assert.match(
			error.message,
			/www\.yudream\.online: GET \/api\/plugins\/ymcl-content\/admin\/update\/releases\//,
		)
		assert.match(error.message, /failed after \d+ms/)
		assert.match(error.message, /UND_ERR_CONNECT_TIMEOUT/)
		assert.doesNotMatch(error.message, /test-private-key/)
		return true
	})
	assert.equal(mock.state.has(updatePlatforms[0].origin), false)
})

test('download failures identify the GitHub filename and preserve the transport error code', async () => {
	const directory = await mkdtemp(join(tmpdir(), 'ymcl-download-network-test-'))
	try {
		await assert.rejects(
			downloadArtifact(artifacts[0], directory, async () => {
				throw new TypeError('fetch failed', {
					cause: Object.assign(new Error('socket closed'), { code: 'ECONNRESET' }),
				})
			}),
			(error) => {
				assert.ok(error.message.includes(artifacts[0].filename))
				assert.match(error.message, /GitHub download.*failed after \d+ms/)
				assert.match(error.message, /ECONNRESET/)
				return true
			},
		)
	} finally {
		await rm(directory, { recursive: true, force: true })
	}
})
