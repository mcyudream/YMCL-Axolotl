import { loadVersionResources, TEXTURE_ATLAS } from './resources.ts'

const images = new Map<string, Promise<HTMLImageElement>>()
let atlasImage: Promise<HTMLImageElement> | undefined

function loadImage(src: string): Promise<HTMLImageElement> {
	return new Promise((resolve, reject) => {
		const image = new Image()
		image.onload = () => resolve(image)
		image.onerror = () => reject(new Error('Unable to load Minecraft item image'))
		image.src = src
	})
}

export function loadMinecraftItemImage(item: string): Promise<HTMLImageElement> {
	const id = item.includes(':') ? item : `minecraft:${item}`
	const cached = images.get(id)
	if (cached) return cached
	const pending = (async () => {
		const resources = await loadVersionResources('26.2')
		const texture = resources.itemsById[id]?.texture
		const rect = texture ? TEXTURE_ATLAS.layout[texture] : undefined
		if (!rect) throw new Error(`Missing Minecraft item texture: ${id}`)
		atlasImage ??= loadImage(TEXTURE_ATLAS.url).catch((error) => {
			atlasImage = undefined
			throw error
		})
		const atlas = await atlasImage
		const canvas = document.createElement('canvas')
		canvas.width = rect[2]
		canvas.height = rect[3]
		const context = canvas.getContext('2d')
		if (!context) throw new Error('Unable to create item image')
		context.drawImage(atlas, ...rect, 0, 0, rect[2], rect[3])
		return loadImage(canvas.toDataURL('image/png'))
	})().catch((error) => {
		images.delete(id)
		throw error
	})
	images.set(id, pending)
	return pending
}
