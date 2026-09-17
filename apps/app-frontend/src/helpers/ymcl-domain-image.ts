/**
 * Domain-supplied image URLs (avatar, logo, theme background, activity cover)
 * may be absolute https, absolute http, or a site-relative path reported by
 * the adapter. Relative paths must be rebased onto the joined domain origin —
 * the WebView origin is the launcher itself, not the domain.
 */

const BROWSER_SCHEMES = /^(?:data:|blob:|asset:|tauri:)/i

export function resolveDomainImageUrl(
	raw: string | null | undefined,
	origin: string | null | undefined,
): string | null {
	if (typeof raw !== 'string') return null
	const trimmed = raw.trim()
	if (!trimmed) return null
	if (BROWSER_SCHEMES.test(trimmed) || /^https?:\/\//i.test(trimmed)) return trimmed
	// Protocol-relative URLs resolve against the WebView origin (the launcher),
	// never the domain — pin them to https.
	if (trimmed.startsWith('//')) return `https:${trimmed}`
	if (!origin) return trimmed
	const base = origin.replace(/\/+$/, '')
	const path = trimmed.replace(/^\/+/, '')
	return `${base}/${path}`
}
