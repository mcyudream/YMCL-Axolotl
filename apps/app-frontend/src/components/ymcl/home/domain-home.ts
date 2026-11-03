/** Domain-wide landing page settings carried by the YAP chrome/home document. */
export interface DomainHomeSlide {
	id: string
	image: string
	title: string
	description: string
}

export interface DomainHomeLanding {
	enabled: boolean
	title: string
	description: string
	copyMode: 'shared' | 'individual'
	slides: DomainHomeSlide[]
	interval: number
	overlay: number
	focusSources: string[]
	featuredServerId: string
	music: {
		url: string
		title: string
		autoplay: boolean
		volume: number
	}
}

export const DOMAIN_HOME_MAX_SLIDES = 12

function record(value: unknown): Record<string, unknown> {
	return value && typeof value === 'object' && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: {}
}

function text(value: unknown): string {
	return typeof value === 'string' ? value.trim() : ''
}

function number(value: unknown, fallback: number, min: number, max: number): number {
	return typeof value === 'number' && Number.isFinite(value)
		? Math.min(max, Math.max(min, Math.round(value)))
		: fallback
}

/** Media must be shareable with every member; device-local paths are not publishable. */
export function isDomainHomeMediaUrl(value: string): boolean {
	const source = value.trim()
	if (
		!source ||
		[...source].some((character) => character.charCodeAt(0) <= 32 || character === '\\')
	)
		return false
	if (/^[a-z][a-z\d+.-]*:/i.test(source) && !/^https?:\/\//i.test(source)) return false
	try {
		const url = new URL(source, 'https://domain.invalid/')
		return ['http:', 'https:'].includes(url.protocol) && !url.username && !url.password
	} catch {
		return false
	}
}

/** Normalize untrusted manifest data, including bounds used by timers and audio.volume. */
export function normalizeDomainHomeLanding(value: unknown): DomainHomeLanding {
	const source = record(value)
	const music = record(source.music)
	const ids = new Set<string>()
	const slides: DomainHomeSlide[] = []
	for (const [index, value] of (Array.isArray(source.slides) ? source.slides : []).entries()) {
		const slide = record(value)
		const image = text(slide.image)
		if (!isDomainHomeMediaUrl(image) || slides.length >= DOMAIN_HOME_MAX_SLIDES) continue
		let id = text(slide.id) || `slide-${index}`
		while (ids.has(id)) id += '-duplicate'
		ids.add(id)
		slides.push({ id, image, title: text(slide.title), description: text(slide.description) })
	}
	return {
		enabled: source.enabled === true,
		title: text(source.title),
		description: text(source.description),
		copyMode: source.copyMode === 'shared' ? 'shared' : 'individual',
		slides,
		interval: number(source.interval, 8, 3, 60),
		overlay: number(source.overlay, 35, 0, 85),
		focusSources: Array.isArray(source.focusSources)
			? [...new Set(source.focusSources.map(text).filter(Boolean))]
			: [],
		featuredServerId: text(source.featuredServerId),
		music: {
			url: isDomainHomeMediaUrl(text(music.url)) ? text(music.url) : '',
			title: text(music.title),
			autoplay: music.autoplay === true,
			volume: number(music.volume, 35, 0, 100),
		},
	}
}
