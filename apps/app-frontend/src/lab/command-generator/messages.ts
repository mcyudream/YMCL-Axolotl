import { defineMessages, type MessageDescriptor } from '@modrinth/ui'

const messages = defineMessages({
	'app.lab.command-generator.arg.advancement.action.desc': {
		id: 'app.lab.command-generator.arg.advancement.action.desc',
		defaultMessage: 'grant / revoke',
	},
	'app.lab.command-generator.arg.advancement.action.display': {
		id: 'app.lab.command-generator.arg.advancement.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.advancement.advancement.desc': {
		id: 'app.lab.command-generator.arg.advancement.advancement.desc',
		defaultMessage: 'Advancement identifier',
	},
	'app.lab.command-generator.arg.advancement.advancement.display': {
		id: 'app.lab.command-generator.arg.advancement.advancement.display',
		defaultMessage: 'Advancement',
	},
	'app.lab.command-generator.arg.advancement.criterion.desc': {
		id: 'app.lab.command-generator.arg.advancement.criterion.desc',
		defaultMessage: 'Advancement criterion; leave empty for all criteria',
	},
	'app.lab.command-generator.arg.advancement.criterion.display': {
		id: 'app.lab.command-generator.arg.advancement.criterion.display',
		defaultMessage: 'Criterion',
	},
	'app.lab.command-generator.arg.advancement.mode.desc': {
		id: 'app.lab.command-generator.arg.advancement.mode.desc',
		defaultMessage: 'Choose one advancement, an advancement tree, or all advancements.',
	},
	'app.lab.command-generator.arg.advancement.mode.display': {
		id: 'app.lab.command-generator.arg.advancement.mode.display',
		defaultMessage: 'Selection mode',
	},
	'app.lab.command-generator.arg.advancement.target.desc': {
		id: 'app.lab.command-generator.arg.advancement.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.advancement.target.display': {
		id: 'app.lab.command-generator.arg.advancement.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.attribute.action.desc': {
		id: 'app.lab.command-generator.arg.attribute.action.desc',
		defaultMessage: 'get / base / value',
	},
	'app.lab.command-generator.arg.attribute.action.display': {
		id: 'app.lab.command-generator.arg.attribute.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.attribute.attribute.desc': {
		id: 'app.lab.command-generator.arg.attribute.attribute.desc',
		defaultMessage: 'Attribute ID',
	},
	'app.lab.command-generator.arg.attribute.attribute.display': {
		id: 'app.lab.command-generator.arg.attribute.attribute.display',
		defaultMessage: 'Attribute',
	},
	'app.lab.command-generator.arg.attribute.target.desc': {
		id: 'app.lab.command-generator.arg.attribute.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.attribute.target.display': {
		id: 'app.lab.command-generator.arg.attribute.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.attribute.value.desc': {
		id: 'app.lab.command-generator.arg.attribute.value.desc',
		defaultMessage: 'Attribute value',
	},
	'app.lab.command-generator.arg.attribute.value.display': {
		id: 'app.lab.command-generator.arg.attribute.value.display',
		defaultMessage: 'Value',
	},
	'app.lab.command-generator.arg.ban-ip.reason.desc': {
		id: 'app.lab.command-generator.arg.ban-ip.reason.desc',
		defaultMessage: 'Reason for the ban',
	},
	'app.lab.command-generator.arg.ban-ip.reason.display': {
		id: 'app.lab.command-generator.arg.ban-ip.reason.display',
		defaultMessage: 'Reason',
	},
	'app.lab.command-generator.arg.ban-ip.target.desc': {
		id: 'app.lab.command-generator.arg.ban-ip.target.desc',
		defaultMessage: 'IP address to ban',
	},
	'app.lab.command-generator.arg.ban-ip.target.display': {
		id: 'app.lab.command-generator.arg.ban-ip.target.display',
		defaultMessage: 'IP address',
	},
	'app.lab.command-generator.arg.ban.reason.desc': {
		id: 'app.lab.command-generator.arg.ban.reason.desc',
		defaultMessage: 'Reason for the ban',
	},
	'app.lab.command-generator.arg.ban.reason.display': {
		id: 'app.lab.command-generator.arg.ban.reason.display',
		defaultMessage: 'Reason',
	},
	'app.lab.command-generator.arg.ban.target.desc': {
		id: 'app.lab.command-generator.arg.ban.target.desc',
		defaultMessage: 'Player name to ban',
	},
	'app.lab.command-generator.arg.ban.target.display': {
		id: 'app.lab.command-generator.arg.ban.target.display',
		defaultMessage: 'Player name',
	},
	'app.lab.command-generator.arg.banlist.type.desc': {
		id: 'app.lab.command-generator.arg.banlist.type.desc',
		defaultMessage: 'players / ips',
	},
	'app.lab.command-generator.arg.banlist.type.display': {
		id: 'app.lab.command-generator.arg.banlist.type.display',
		defaultMessage: 'Type',
	},
	'app.lab.command-generator.arg.bossbar.action.desc': {
		id: 'app.lab.command-generator.arg.bossbar.action.desc',
		defaultMessage: 'add / remove / get / set / list',
	},
	'app.lab.command-generator.arg.bossbar.action.display': {
		id: 'app.lab.command-generator.arg.bossbar.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.bossbar.id.desc': {
		id: 'app.lab.command-generator.arg.bossbar.id.desc',
		defaultMessage: 'Boss bar identifier',
	},
	'app.lab.command-generator.arg.bossbar.id.display': {
		id: 'app.lab.command-generator.arg.bossbar.id.display',
		defaultMessage: 'ID',
	},
	'app.lab.command-generator.arg.bossbar.property.desc': {
		id: 'app.lab.command-generator.arg.bossbar.property.desc',
		defaultMessage: 'name / color / style / value / max / visible / players',
	},
	'app.lab.command-generator.arg.bossbar.property.display': {
		id: 'app.lab.command-generator.arg.bossbar.property.display',
		defaultMessage: 'Attribute',
	},
	'app.lab.command-generator.arg.bossbar.value.desc': {
		id: 'app.lab.command-generator.arg.bossbar.value.desc',
		defaultMessage: 'Attribute value',
	},
	'app.lab.command-generator.arg.bossbar.value.display': {
		id: 'app.lab.command-generator.arg.bossbar.value.display',
		defaultMessage: 'Value',
	},
	'app.lab.command-generator.arg.clear.item.desc': {
		id: 'app.lab.command-generator.arg.clear.item.desc',
		defaultMessage: 'Item ID to clear',
	},
	'app.lab.command-generator.arg.clear.item.display': {
		id: 'app.lab.command-generator.arg.clear.item.display',
		defaultMessage: 'Items',
	},
	'app.lab.command-generator.arg.clear.maxCount.desc': {
		id: 'app.lab.command-generator.arg.clear.maxCount.desc',
		defaultMessage: 'Maximum number of items to clear',
	},
	'app.lab.command-generator.arg.clear.maxCount.display': {
		id: 'app.lab.command-generator.arg.clear.maxCount.display',
		defaultMessage: 'Maximum count',
	},
	'app.lab.command-generator.arg.clear.target.desc': {
		id: 'app.lab.command-generator.arg.clear.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.clear.target.display': {
		id: 'app.lab.command-generator.arg.clear.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.clone.begin.desc': {
		id: 'app.lab.command-generator.arg.clone.begin.desc',
		defaultMessage: 'Start position',
	},
	'app.lab.command-generator.arg.clone.begin.display': {
		id: 'app.lab.command-generator.arg.clone.begin.display',
		defaultMessage: 'Start position',
	},
	'app.lab.command-generator.arg.clone.destination.desc': {
		id: 'app.lab.command-generator.arg.clone.destination.desc',
		defaultMessage: 'Destination position',
	},
	'app.lab.command-generator.arg.clone.destination.display': {
		id: 'app.lab.command-generator.arg.clone.destination.display',
		defaultMessage: 'Destination position',
	},
	'app.lab.command-generator.arg.clone.end.desc': {
		id: 'app.lab.command-generator.arg.clone.end.desc',
		defaultMessage: 'End position',
	},
	'app.lab.command-generator.arg.clone.end.display': {
		id: 'app.lab.command-generator.arg.clone.end.display',
		defaultMessage: 'End position',
	},
	'app.lab.command-generator.arg.clone.maskMode.desc': {
		id: 'app.lab.command-generator.arg.clone.maskMode.desc',
		defaultMessage: 'filtered / replace / masked',
	},
	'app.lab.command-generator.arg.clone.maskMode.display': {
		id: 'app.lab.command-generator.arg.clone.maskMode.display',
		defaultMessage: 'Mask mode',
	},
	'app.lab.command-generator.arg.compute.context.desc': {
		id: 'app.lab.command-generator.arg.compute.context.desc',
		defaultMessage: 'Choose the context used to evaluate the value.',
	},
	'app.lab.command-generator.arg.compute.context.display': {
		id: 'app.lab.command-generator.arg.compute.context.display',
		defaultMessage: 'Context',
	},
	'app.lab.command-generator.arg.compute.pos.desc': {
		id: 'app.lab.command-generator.arg.compute.pos.desc',
		defaultMessage: 'Block position',
	},
	'app.lab.command-generator.arg.compute.pos.display': {
		id: 'app.lab.command-generator.arg.compute.pos.display',
		defaultMessage: 'Block position',
	},
	'app.lab.command-generator.arg.compute.provider.desc': {
		id: 'app.lab.command-generator.arg.compute.provider.desc',
		defaultMessage: 'Enter an inline number provider or a namespaced registry ID.',
	},
	'app.lab.command-generator.arg.compute.provider.display': {
		id: 'app.lab.command-generator.arg.compute.provider.display',
		defaultMessage: 'Number provider',
	},
	'app.lab.command-generator.arg.compute.scale.desc': {
		id: 'app.lab.command-generator.arg.compute.scale.desc',
		defaultMessage: 'Scale a floating-point result before returning it as an integer.',
	},
	'app.lab.command-generator.arg.compute.scale.display': {
		id: 'app.lab.command-generator.arg.compute.scale.display',
		defaultMessage: 'Scale factor',
	},
	'app.lab.command-generator.arg.compute.target.desc': {
		id: 'app.lab.command-generator.arg.compute.target.desc',
		defaultMessage: 'Select one entity.',
	},
	'app.lab.command-generator.arg.compute.target.display': {
		id: 'app.lab.command-generator.arg.compute.target.display',
		defaultMessage: 'Entities',
	},
	'app.lab.command-generator.arg.compute.type.desc': {
		id: 'app.lab.command-generator.arg.compute.type.desc',
		defaultMessage: 'Result type',
	},
	'app.lab.command-generator.arg.compute.type.display': {
		id: 'app.lab.command-generator.arg.compute.type.display',
		defaultMessage: 'Result type',
	},
	'app.lab.command-generator.arg.damage.amount.desc': {
		id: 'app.lab.command-generator.arg.damage.amount.desc',
		defaultMessage: 'Amount of damage to deal',
	},
	'app.lab.command-generator.arg.damage.amount.display': {
		id: 'app.lab.command-generator.arg.damage.amount.display',
		defaultMessage: 'Damage amount',
	},
	'app.lab.command-generator.arg.damage.damageType.desc': {
		id: 'app.lab.command-generator.arg.damage.damageType.desc',
		defaultMessage: 'Damage type ID',
	},
	'app.lab.command-generator.arg.damage.damageType.display': {
		id: 'app.lab.command-generator.arg.damage.damageType.display',
		defaultMessage: 'Damage type',
	},
	'app.lab.command-generator.arg.damage.source.desc': {
		id: 'app.lab.command-generator.arg.damage.source.desc',
		defaultMessage: 'Source entity selector',
	},
	'app.lab.command-generator.arg.damage.source.display': {
		id: 'app.lab.command-generator.arg.damage.source.display',
		defaultMessage: 'Damage source',
	},
	'app.lab.command-generator.arg.damage.target.desc': {
		id: 'app.lab.command-generator.arg.damage.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.damage.target.display': {
		id: 'app.lab.command-generator.arg.damage.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.data.action.desc': {
		id: 'app.lab.command-generator.arg.data.action.desc',
		defaultMessage: 'get / merge / modify / remove',
	},
	'app.lab.command-generator.arg.data.action.display': {
		id: 'app.lab.command-generator.arg.data.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.data.index.desc': {
		id: 'app.lab.command-generator.arg.data.index.desc',
		defaultMessage: 'Insertion index',
	},
	'app.lab.command-generator.arg.data.index.display': {
		id: 'app.lab.command-generator.arg.data.index.display',
		defaultMessage: 'Insertion index',
	},
	'app.lab.command-generator.arg.data.literalValue.desc': {
		id: 'app.lab.command-generator.arg.data.literalValue.desc',
		defaultMessage: 'SNBT value',
	},
	'app.lab.command-generator.arg.data.literalValue.display': {
		id: 'app.lab.command-generator.arg.data.literalValue.display',
		defaultMessage: 'SNBT value',
	},
	'app.lab.command-generator.arg.data.method.desc': {
		id: 'app.lab.command-generator.arg.data.method.desc',
		defaultMessage: 'Modification method',
	},
	'app.lab.command-generator.arg.data.method.display': {
		id: 'app.lab.command-generator.arg.data.method.display',
		defaultMessage: 'Modification method',
	},
	'app.lab.command-generator.arg.data.path.desc': {
		id: 'app.lab.command-generator.arg.data.path.desc',
		defaultMessage: 'NBT path to modify',
	},
	'app.lab.command-generator.arg.data.path.display': {
		id: 'app.lab.command-generator.arg.data.path.display',
		defaultMessage: 'NBT path',
	},
	'app.lab.command-generator.arg.data.source.desc': {
		id: 'app.lab.command-generator.arg.data.source.desc',
		defaultMessage: 'Value source',
	},
	'app.lab.command-generator.arg.data.source.display': {
		id: 'app.lab.command-generator.arg.data.source.display',
		defaultMessage: 'Value source',
	},
	'app.lab.command-generator.arg.data.target.desc': {
		id: 'app.lab.command-generator.arg.data.target.desc',
		defaultMessage: 'Block position, entity selector, or storage ID',
	},
	'app.lab.command-generator.arg.data.target.display': {
		id: 'app.lab.command-generator.arg.data.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.data.targetType.desc': {
		id: 'app.lab.command-generator.arg.data.targetType.desc',
		defaultMessage: 'block / entity / storage',
	},
	'app.lab.command-generator.arg.data.targetType.display': {
		id: 'app.lab.command-generator.arg.data.targetType.display',
		defaultMessage: 'Target type',
	},
	'app.lab.command-generator.arg.data.value.desc': {
		id: 'app.lab.command-generator.arg.data.value.desc',
		defaultMessage: 'NBT value to write',
	},
	'app.lab.command-generator.arg.data.value.display': {
		id: 'app.lab.command-generator.arg.data.value.display',
		defaultMessage: 'Value',
	},
	'app.lab.command-generator.arg.datapack.action.desc': {
		id: 'app.lab.command-generator.arg.datapack.action.desc',
		defaultMessage: 'enable / disable / list',
	},
	'app.lab.command-generator.arg.datapack.action.display': {
		id: 'app.lab.command-generator.arg.datapack.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.datapack.name.desc': {
		id: 'app.lab.command-generator.arg.datapack.name.desc',
		defaultMessage: 'Data pack identifier',
	},
	'app.lab.command-generator.arg.datapack.name.display': {
		id: 'app.lab.command-generator.arg.datapack.name.display',
		defaultMessage: 'Data pack name',
	},
	'app.lab.command-generator.arg.debug.action.desc': {
		id: 'app.lab.command-generator.arg.debug.action.desc',
		defaultMessage: 'start / stop / function',
	},
	'app.lab.command-generator.arg.debug.action.display': {
		id: 'app.lab.command-generator.arg.debug.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.defaultgamemode.mode.desc': {
		id: 'app.lab.command-generator.arg.defaultgamemode.mode.desc',
		defaultMessage: 'survival / creative / adventure / spectator',
	},
	'app.lab.command-generator.arg.defaultgamemode.mode.display': {
		id: 'app.lab.command-generator.arg.defaultgamemode.mode.display',
		defaultMessage: 'Mode',
	},
	'app.lab.command-generator.arg.deop.target.desc': {
		id: 'app.lab.command-generator.arg.deop.target.desc',
		defaultMessage: 'Name of the player whose operator permission will be removed',
	},
	'app.lab.command-generator.arg.deop.target.display': {
		id: 'app.lab.command-generator.arg.deop.target.display',
		defaultMessage: 'Player name',
	},
	'app.lab.command-generator.arg.dialog.action.desc': {
		id: 'app.lab.command-generator.arg.dialog.action.desc',
		defaultMessage: 'Show or clear a dialog.',
	},
	'app.lab.command-generator.arg.dialog.action.display': {
		id: 'app.lab.command-generator.arg.dialog.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.dialog.dialog.desc': {
		id: 'app.lab.command-generator.arg.dialog.dialog.desc',
		defaultMessage: 'Dialog identifier',
	},
	'app.lab.command-generator.arg.dialog.dialog.display': {
		id: 'app.lab.command-generator.arg.dialog.dialog.display',
		defaultMessage: 'Dialog ID',
	},
	'app.lab.command-generator.arg.dialog.target.desc': {
		id: 'app.lab.command-generator.arg.dialog.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.dialog.target.display': {
		id: 'app.lab.command-generator.arg.dialog.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.difficulty.difficulty.desc': {
		id: 'app.lab.command-generator.arg.difficulty.difficulty.desc',
		defaultMessage: 'peaceful / easy / normal / hard',
	},
	'app.lab.command-generator.arg.difficulty.difficulty.display': {
		id: 'app.lab.command-generator.arg.difficulty.difficulty.display',
		defaultMessage: 'Difficulty',
	},
	'app.lab.command-generator.arg.effect.action.desc': {
		id: 'app.lab.command-generator.arg.effect.action.desc',
		defaultMessage: 'give / clear',
	},
	'app.lab.command-generator.arg.effect.action.display': {
		id: 'app.lab.command-generator.arg.effect.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.effect.amplifier.desc': {
		id: 'app.lab.command-generator.arg.effect.amplifier.desc',
		defaultMessage: 'Effect amplifier (0 means level I)',
	},
	'app.lab.command-generator.arg.effect.amplifier.display': {
		id: 'app.lab.command-generator.arg.effect.amplifier.display',
		defaultMessage: 'Levels',
	},
	'app.lab.command-generator.arg.effect.duration.desc': {
		id: 'app.lab.command-generator.arg.effect.duration.desc',
		defaultMessage: 'Effect duration in seconds',
	},
	'app.lab.command-generator.arg.effect.duration.display': {
		id: 'app.lab.command-generator.arg.effect.duration.display',
		defaultMessage: 'Duration',
	},
	'app.lab.command-generator.arg.effect.effect.desc': {
		id: 'app.lab.command-generator.arg.effect.effect.desc',
		defaultMessage: 'Status effect ID',
	},
	'app.lab.command-generator.arg.effect.effect.display': {
		id: 'app.lab.command-generator.arg.effect.effect.display',
		defaultMessage: 'Effect',
	},
	'app.lab.command-generator.arg.effect.hideParticles.desc': {
		id: 'app.lab.command-generator.arg.effect.hideParticles.desc',
		defaultMessage: 'Whether to hide effect particles',
	},
	'app.lab.command-generator.arg.effect.hideParticles.display': {
		id: 'app.lab.command-generator.arg.effect.hideParticles.display',
		defaultMessage: 'Hide particles',
	},
	'app.lab.command-generator.arg.effect.target.desc': {
		id: 'app.lab.command-generator.arg.effect.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.effect.target.display': {
		id: 'app.lab.command-generator.arg.effect.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.enchant.enchantment.desc': {
		id: 'app.lab.command-generator.arg.enchant.enchantment.desc',
		defaultMessage: 'Enchantment ID',
	},
	'app.lab.command-generator.arg.enchant.enchantment.display': {
		id: 'app.lab.command-generator.arg.enchant.enchantment.display',
		defaultMessage: 'Enchantment',
	},
	'app.lab.command-generator.arg.enchant.level.desc': {
		id: 'app.lab.command-generator.arg.enchant.level.desc',
		defaultMessage: 'Enchantment level',
	},
	'app.lab.command-generator.arg.enchant.level.display': {
		id: 'app.lab.command-generator.arg.enchant.level.display',
		defaultMessage: 'Levels',
	},
	'app.lab.command-generator.arg.enchant.target.desc': {
		id: 'app.lab.command-generator.arg.enchant.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.enchant.target.display': {
		id: 'app.lab.command-generator.arg.enchant.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.execute.align.axes.desc': {
		id: 'app.lab.command-generator.arg.execute.align.axes.desc',
		defaultMessage: 'Coordinate axes to align',
	},
	'app.lab.command-generator.arg.execute.align.axes.display': {
		id: 'app.lab.command-generator.arg.execute.align.axes.display',
		defaultMessage: 'Axes',
	},
	'app.lab.command-generator.arg.execute.anchored.anchor.desc': {
		id: 'app.lab.command-generator.arg.execute.anchored.anchor.desc',
		defaultMessage: 'eyes / feet',
	},
	'app.lab.command-generator.arg.execute.anchored.anchor.display': {
		id: 'app.lab.command-generator.arg.execute.anchored.anchor.display',
		defaultMessage: 'Anchor',
	},
	'app.lab.command-generator.arg.execute.as.target.desc': {
		id: 'app.lab.command-generator.arg.execute.as.target.desc',
		defaultMessage: 'New executor',
	},
	'app.lab.command-generator.arg.execute.as.target.display': {
		id: 'app.lab.command-generator.arg.execute.as.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.execute.at.target.desc': {
		id: 'app.lab.command-generator.arg.execute.at.target.desc',
		defaultMessage: 'Entity providing the position',
	},
	'app.lab.command-generator.arg.execute.at.target.display': {
		id: 'app.lab.command-generator.arg.execute.at.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.execute.facing-entity.anchor.desc': {
		id: 'app.lab.command-generator.arg.execute.facing-entity.anchor.desc',
		defaultMessage: 'eyes / feet',
	},
	'app.lab.command-generator.arg.execute.facing-entity.anchor.display': {
		id: 'app.lab.command-generator.arg.execute.facing-entity.anchor.display',
		defaultMessage: 'Anchor',
	},
	'app.lab.command-generator.arg.execute.facing-entity.target.desc': {
		id: 'app.lab.command-generator.arg.execute.facing-entity.target.desc',
		defaultMessage: 'Entity to face',
	},
	'app.lab.command-generator.arg.execute.facing-entity.target.display': {
		id: 'app.lab.command-generator.arg.execute.facing-entity.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.execute.facing.pos.desc': {
		id: 'app.lab.command-generator.arg.execute.facing.pos.desc',
		defaultMessage: 'Position to face',
	},
	'app.lab.command-generator.arg.execute.facing.pos.display': {
		id: 'app.lab.command-generator.arg.execute.facing.pos.display',
		defaultMessage: 'Position',
	},
	'app.lab.command-generator.arg.execute.if-biome.biome.desc': {
		id: 'app.lab.command-generator.arg.execute.if-biome.biome.desc',
		defaultMessage: 'Biome ID',
	},
	'app.lab.command-generator.arg.execute.if-biome.biome.display': {
		id: 'app.lab.command-generator.arg.execute.if-biome.biome.display',
		defaultMessage: 'Biome',
	},
	'app.lab.command-generator.arg.execute.if-biome.pos.desc': {
		id: 'app.lab.command-generator.arg.execute.if-biome.pos.desc',
		defaultMessage: 'Position to check',
	},
	'app.lab.command-generator.arg.execute.if-biome.pos.display': {
		id: 'app.lab.command-generator.arg.execute.if-biome.pos.display',
		defaultMessage: 'Position',
	},
	'app.lab.command-generator.arg.execute.if-block.block.desc': {
		id: 'app.lab.command-generator.arg.execute.if-block.block.desc',
		defaultMessage: 'Block ID',
	},
	'app.lab.command-generator.arg.execute.if-block.block.display': {
		id: 'app.lab.command-generator.arg.execute.if-block.block.display',
		defaultMessage: 'Blocks',
	},
	'app.lab.command-generator.arg.execute.if-block.pos.desc': {
		id: 'app.lab.command-generator.arg.execute.if-block.pos.desc',
		defaultMessage: 'Position to check',
	},
	'app.lab.command-generator.arg.execute.if-block.pos.display': {
		id: 'app.lab.command-generator.arg.execute.if-block.pos.display',
		defaultMessage: 'Position',
	},
	'app.lab.command-generator.arg.execute.if-blocks.begin.desc': {
		id: 'app.lab.command-generator.arg.execute.if-blocks.begin.desc',
		defaultMessage: 'Start of the source region',
	},
	'app.lab.command-generator.arg.execute.if-blocks.begin.display': {
		id: 'app.lab.command-generator.arg.execute.if-blocks.begin.display',
		defaultMessage: 'Start position',
	},
	'app.lab.command-generator.arg.execute.if-blocks.destination.desc': {
		id: 'app.lab.command-generator.arg.execute.if-blocks.destination.desc',
		defaultMessage: 'Start of the destination region',
	},
	'app.lab.command-generator.arg.execute.if-blocks.destination.display': {
		id: 'app.lab.command-generator.arg.execute.if-blocks.destination.display',
		defaultMessage: 'Destination position',
	},
	'app.lab.command-generator.arg.execute.if-blocks.end.desc': {
		id: 'app.lab.command-generator.arg.execute.if-blocks.end.desc',
		defaultMessage: 'End of the source region',
	},
	'app.lab.command-generator.arg.execute.if-blocks.end.display': {
		id: 'app.lab.command-generator.arg.execute.if-blocks.end.display',
		defaultMessage: 'End position',
	},
	'app.lab.command-generator.arg.execute.if-blocks.mode.desc': {
		id: 'app.lab.command-generator.arg.execute.if-blocks.mode.desc',
		defaultMessage: 'all / masked (ignore air)',
	},
	'app.lab.command-generator.arg.execute.if-blocks.mode.display': {
		id: 'app.lab.command-generator.arg.execute.if-blocks.mode.display',
		defaultMessage: 'Comparison mode',
	},
	'app.lab.command-generator.arg.execute.if-data.path.desc': {
		id: 'app.lab.command-generator.arg.execute.if-data.path.desc',
		defaultMessage: 'NBT path to check',
	},
	'app.lab.command-generator.arg.execute.if-data.path.display': {
		id: 'app.lab.command-generator.arg.execute.if-data.path.display',
		defaultMessage: 'NBT path',
	},
	'app.lab.command-generator.arg.execute.if-data.target.desc': {
		id: 'app.lab.command-generator.arg.execute.if-data.target.desc',
		defaultMessage: 'Block position, entity selector, or storage ID',
	},
	'app.lab.command-generator.arg.execute.if-data.target.display': {
		id: 'app.lab.command-generator.arg.execute.if-data.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.execute.if-data.targetType.desc': {
		id: 'app.lab.command-generator.arg.execute.if-data.targetType.desc',
		defaultMessage: 'block / entity / storage',
	},
	'app.lab.command-generator.arg.execute.if-data.targetType.display': {
		id: 'app.lab.command-generator.arg.execute.if-data.targetType.display',
		defaultMessage: 'Target type',
	},
	'app.lab.command-generator.arg.execute.if-entity.target.desc': {
		id: 'app.lab.command-generator.arg.execute.if-entity.target.desc',
		defaultMessage: 'Entity whose existence will be checked',
	},
	'app.lab.command-generator.arg.execute.if-entity.target.display': {
		id: 'app.lab.command-generator.arg.execute.if-entity.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.execute.if-function.name.desc': {
		id: 'app.lab.command-generator.arg.execute.if-function.name.desc',
		defaultMessage: 'Function identifier',
	},
	'app.lab.command-generator.arg.execute.if-function.name.display': {
		id: 'app.lab.command-generator.arg.execute.if-function.name.display',
		defaultMessage: 'Function name',
	},
	'app.lab.command-generator.arg.execute.if-items.item.desc': {
		id: 'app.lab.command-generator.arg.execute.if-items.item.desc',
		defaultMessage: 'Item ID',
	},
	'app.lab.command-generator.arg.execute.if-items.item.display': {
		id: 'app.lab.command-generator.arg.execute.if-items.item.display',
		defaultMessage: 'Items',
	},
	'app.lab.command-generator.arg.execute.if-items.slot.desc': {
		id: 'app.lab.command-generator.arg.execute.if-items.slot.desc',
		defaultMessage: 'Slot name',
	},
	'app.lab.command-generator.arg.execute.if-items.slot.display': {
		id: 'app.lab.command-generator.arg.execute.if-items.slot.display',
		defaultMessage: 'Slot',
	},
	'app.lab.command-generator.arg.execute.if-items.source.desc': {
		id: 'app.lab.command-generator.arg.execute.if-items.source.desc',
		defaultMessage: 'block / entity',
	},
	'app.lab.command-generator.arg.execute.if-items.source.display': {
		id: 'app.lab.command-generator.arg.execute.if-items.source.display',
		defaultMessage: 'Source',
	},
	'app.lab.command-generator.arg.execute.if-items.target.desc': {
		id: 'app.lab.command-generator.arg.execute.if-items.target.desc',
		defaultMessage: 'Block position or entity selector',
	},
	'app.lab.command-generator.arg.execute.if-items.target.display': {
		id: 'app.lab.command-generator.arg.execute.if-items.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.execute.if-score-matches.objective.desc': {
		id: 'app.lab.command-generator.arg.execute.if-score-matches.objective.desc',
		defaultMessage: 'Scoreboard objective name',
	},
	'app.lab.command-generator.arg.execute.if-score-matches.objective.display': {
		id: 'app.lab.command-generator.arg.execute.if-score-matches.objective.display',
		defaultMessage: 'Objective',
	},
	'app.lab.command-generator.arg.execute.if-score-matches.range.desc': {
		id: 'app.lab.command-generator.arg.execute.if-score-matches.range.desc',
		defaultMessage: 'Numeric range, such as 1..10',
	},
	'app.lab.command-generator.arg.execute.if-score-matches.range.display': {
		id: 'app.lab.command-generator.arg.execute.if-score-matches.range.display',
		defaultMessage: 'Range',
	},
	'app.lab.command-generator.arg.execute.if-score-matches.target.desc': {
		id: 'app.lab.command-generator.arg.execute.if-score-matches.target.desc',
		defaultMessage: 'Score holder',
	},
	'app.lab.command-generator.arg.execute.if-score-matches.target.display': {
		id: 'app.lab.command-generator.arg.execute.if-score-matches.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.execute.if-score.objective.desc': {
		id: 'app.lab.command-generator.arg.execute.if-score.objective.desc',
		defaultMessage: 'Left-hand objective',
	},
	'app.lab.command-generator.arg.execute.if-score.objective.display': {
		id: 'app.lab.command-generator.arg.execute.if-score.objective.display',
		defaultMessage: 'Objective',
	},
	'app.lab.command-generator.arg.execute.if-score.operator.desc': {
		id: 'app.lab.command-generator.arg.execute.if-score.operator.desc',
		defaultMessage: 'Comparison operator',
	},
	'app.lab.command-generator.arg.execute.if-score.operator.display': {
		id: 'app.lab.command-generator.arg.execute.if-score.operator.display',
		defaultMessage: 'Operator',
	},
	'app.lab.command-generator.arg.execute.if-score.source.desc': {
		id: 'app.lab.command-generator.arg.execute.if-score.source.desc',
		defaultMessage: 'Right-hand score holder',
	},
	'app.lab.command-generator.arg.execute.if-score.source.display': {
		id: 'app.lab.command-generator.arg.execute.if-score.source.display',
		defaultMessage: 'Source target',
	},
	'app.lab.command-generator.arg.execute.if-score.sourceObjective.desc': {
		id: 'app.lab.command-generator.arg.execute.if-score.sourceObjective.desc',
		defaultMessage: 'Right-hand objective',
	},
	'app.lab.command-generator.arg.execute.if-score.sourceObjective.display': {
		id: 'app.lab.command-generator.arg.execute.if-score.sourceObjective.display',
		defaultMessage: 'Source objective',
	},
	'app.lab.command-generator.arg.execute.if-score.target.desc': {
		id: 'app.lab.command-generator.arg.execute.if-score.target.desc',
		defaultMessage: 'Left-hand score holder',
	},
	'app.lab.command-generator.arg.execute.if-score.target.display': {
		id: 'app.lab.command-generator.arg.execute.if-score.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.execute.in.dimension.desc': {
		id: 'app.lab.command-generator.arg.execute.in.dimension.desc',
		defaultMessage: 'Target dimension',
	},
	'app.lab.command-generator.arg.execute.in.dimension.display': {
		id: 'app.lab.command-generator.arg.execute.in.dimension.display',
		defaultMessage: 'Dimension',
	},
	'app.lab.command-generator.arg.execute.positioned-as.target.desc': {
		id: 'app.lab.command-generator.arg.execute.positioned-as.target.desc',
		defaultMessage: 'Entity providing the position',
	},
	'app.lab.command-generator.arg.execute.positioned-as.target.display': {
		id: 'app.lab.command-generator.arg.execute.positioned-as.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.execute.positioned.pos.desc': {
		id: 'app.lab.command-generator.arg.execute.positioned.pos.desc',
		defaultMessage: 'New execution position',
	},
	'app.lab.command-generator.arg.execute.positioned.pos.display': {
		id: 'app.lab.command-generator.arg.execute.positioned.pos.display',
		defaultMessage: 'Position',
	},
	'app.lab.command-generator.arg.execute.rotated-as.target.desc': {
		id: 'app.lab.command-generator.arg.execute.rotated-as.target.desc',
		defaultMessage: 'Entity providing the rotation',
	},
	'app.lab.command-generator.arg.execute.rotated-as.target.display': {
		id: 'app.lab.command-generator.arg.execute.rotated-as.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.execute.rotated.pitch.desc': {
		id: 'app.lab.command-generator.arg.execute.rotated.pitch.desc',
		defaultMessage: 'Vertical rotation angle',
	},
	'app.lab.command-generator.arg.execute.rotated.pitch.display': {
		id: 'app.lab.command-generator.arg.execute.rotated.pitch.display',
		defaultMessage: 'Pitch',
	},
	'app.lab.command-generator.arg.execute.rotated.yaw.desc': {
		id: 'app.lab.command-generator.arg.execute.rotated.yaw.desc',
		defaultMessage: 'Horizontal rotation angle',
	},
	'app.lab.command-generator.arg.execute.rotated.yaw.display': {
		id: 'app.lab.command-generator.arg.execute.rotated.yaw.display',
		defaultMessage: 'Yaw',
	},
	'app.lab.command-generator.arg.execute.slot.desc.26': {
		id: 'app.lab.command-generator.arg.execute.slot.desc.26',
		defaultMessage: 'Slot range or slot source.',
	},
	'app.lab.command-generator.arg.execute.slots.desc': {
		id: 'app.lab.command-generator.arg.execute.slots.desc',
		defaultMessage: 'Slot range, inline slot source, or namespaced ID.',
	},
	'app.lab.command-generator.arg.execute.slots.display': {
		id: 'app.lab.command-generator.arg.execute.slots.display',
		defaultMessage: 'Slot source',
	},
	'app.lab.command-generator.arg.execute.source.desc': {
		id: 'app.lab.command-generator.arg.execute.source.desc',
		defaultMessage: 'Source type',
	},
	'app.lab.command-generator.arg.execute.source.display': {
		id: 'app.lab.command-generator.arg.execute.source.display',
		defaultMessage: 'Source type',
	},
	'app.lab.command-generator.arg.execute.store-result-bossbar.id.desc': {
		id: 'app.lab.command-generator.arg.execute.store-result-bossbar.id.desc',
		defaultMessage: 'Boss bar identifier',
	},
	'app.lab.command-generator.arg.execute.store-result-bossbar.id.display': {
		id: 'app.lab.command-generator.arg.execute.store-result-bossbar.id.display',
		defaultMessage: 'Boss bar ID',
	},
	'app.lab.command-generator.arg.execute.store-result-bossbar.valueOrMax.desc': {
		id: 'app.lab.command-generator.arg.execute.store-result-bossbar.valueOrMax.desc',
		defaultMessage: 'value / max',
	},
	'app.lab.command-generator.arg.execute.store-result-bossbar.valueOrMax.display': {
		id: 'app.lab.command-generator.arg.execute.store-result-bossbar.valueOrMax.display',
		defaultMessage: 'Storage field',
	},
	'app.lab.command-generator.arg.execute.store-result-score.objective.desc': {
		id: 'app.lab.command-generator.arg.execute.store-result-score.objective.desc',
		defaultMessage: 'Scoreboard objective name',
	},
	'app.lab.command-generator.arg.execute.store-result-score.objective.display': {
		id: 'app.lab.command-generator.arg.execute.store-result-score.objective.display',
		defaultMessage: 'Objective',
	},
	'app.lab.command-generator.arg.execute.store-result-score.target.desc': {
		id: 'app.lab.command-generator.arg.execute.store-result-score.target.desc',
		defaultMessage: 'Score holder',
	},
	'app.lab.command-generator.arg.execute.store-result-score.target.display': {
		id: 'app.lab.command-generator.arg.execute.store-result-score.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.execute.store-success-score.objective.desc': {
		id: 'app.lab.command-generator.arg.execute.store-success-score.objective.desc',
		defaultMessage: 'Scoreboard objective name',
	},
	'app.lab.command-generator.arg.execute.store-success-score.objective.display': {
		id: 'app.lab.command-generator.arg.execute.store-success-score.objective.display',
		defaultMessage: 'Objective',
	},
	'app.lab.command-generator.arg.execute.store-success-score.target.desc': {
		id: 'app.lab.command-generator.arg.execute.store-success-score.target.desc',
		defaultMessage: 'Score holder',
	},
	'app.lab.command-generator.arg.execute.store-success-score.target.display': {
		id: 'app.lab.command-generator.arg.execute.store-success-score.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.execute.target.desc': {
		id: 'app.lab.command-generator.arg.execute.target.desc',
		defaultMessage: 'Entity or position',
	},
	'app.lab.command-generator.arg.execute.target.display': {
		id: 'app.lab.command-generator.arg.execute.target.display',
		defaultMessage: 'Entity or position',
	},
	'app.lab.command-generator.arg.execute.unless-block.block.desc': {
		id: 'app.lab.command-generator.arg.execute.unless-block.block.desc',
		defaultMessage: 'Block ID',
	},
	'app.lab.command-generator.arg.execute.unless-block.block.display': {
		id: 'app.lab.command-generator.arg.execute.unless-block.block.display',
		defaultMessage: 'Blocks',
	},
	'app.lab.command-generator.arg.execute.unless-block.pos.desc': {
		id: 'app.lab.command-generator.arg.execute.unless-block.pos.desc',
		defaultMessage: 'Position to check',
	},
	'app.lab.command-generator.arg.execute.unless-block.pos.display': {
		id: 'app.lab.command-generator.arg.execute.unless-block.pos.display',
		defaultMessage: 'Position',
	},
	'app.lab.command-generator.arg.execute.unless-entity.target.desc': {
		id: 'app.lab.command-generator.arg.execute.unless-entity.target.desc',
		defaultMessage: 'Entity whose absence will be checked',
	},
	'app.lab.command-generator.arg.execute.unless-entity.target.display': {
		id: 'app.lab.command-generator.arg.execute.unless-entity.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.execute.unless-score.objective.desc': {
		id: 'app.lab.command-generator.arg.execute.unless-score.objective.desc',
		defaultMessage: 'Left-hand objective',
	},
	'app.lab.command-generator.arg.execute.unless-score.objective.display': {
		id: 'app.lab.command-generator.arg.execute.unless-score.objective.display',
		defaultMessage: 'Objective',
	},
	'app.lab.command-generator.arg.execute.unless-score.operator.desc': {
		id: 'app.lab.command-generator.arg.execute.unless-score.operator.desc',
		defaultMessage: 'Comparison operator',
	},
	'app.lab.command-generator.arg.execute.unless-score.operator.display': {
		id: 'app.lab.command-generator.arg.execute.unless-score.operator.display',
		defaultMessage: 'Operator',
	},
	'app.lab.command-generator.arg.execute.unless-score.source.desc': {
		id: 'app.lab.command-generator.arg.execute.unless-score.source.desc',
		defaultMessage: 'Right-hand score holder',
	},
	'app.lab.command-generator.arg.execute.unless-score.source.display': {
		id: 'app.lab.command-generator.arg.execute.unless-score.source.display',
		defaultMessage: 'Source target',
	},
	'app.lab.command-generator.arg.execute.unless-score.sourceObjective.desc': {
		id: 'app.lab.command-generator.arg.execute.unless-score.sourceObjective.desc',
		defaultMessage: 'Right-hand objective',
	},
	'app.lab.command-generator.arg.execute.unless-score.sourceObjective.display': {
		id: 'app.lab.command-generator.arg.execute.unless-score.sourceObjective.display',
		defaultMessage: 'Source objective',
	},
	'app.lab.command-generator.arg.execute.unless-score.target.desc': {
		id: 'app.lab.command-generator.arg.execute.unless-score.target.desc',
		defaultMessage: 'Left-hand score holder',
	},
	'app.lab.command-generator.arg.execute.unless-score.target.display': {
		id: 'app.lab.command-generator.arg.execute.unless-score.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.experience.action.desc': {
		id: 'app.lab.command-generator.arg.experience.action.desc',
		defaultMessage: 'add / set',
	},
	'app.lab.command-generator.arg.experience.action.display': {
		id: 'app.lab.command-generator.arg.experience.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.experience.amount.desc': {
		id: 'app.lab.command-generator.arg.experience.amount.desc',
		defaultMessage: 'Experience amount',
	},
	'app.lab.command-generator.arg.experience.amount.display': {
		id: 'app.lab.command-generator.arg.experience.amount.display',
		defaultMessage: 'Amount',
	},
	'app.lab.command-generator.arg.experience.target.desc': {
		id: 'app.lab.command-generator.arg.experience.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.experience.target.display': {
		id: 'app.lab.command-generator.arg.experience.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.experience.unit.desc': {
		id: 'app.lab.command-generator.arg.experience.unit.desc',
		defaultMessage: 'points / levels',
	},
	'app.lab.command-generator.arg.experience.unit.display': {
		id: 'app.lab.command-generator.arg.experience.unit.display',
		defaultMessage: 'Unit',
	},
	'app.lab.command-generator.arg.fetchprofile.profile.desc': {
		id: 'app.lab.command-generator.arg.fetchprofile.profile.desc',
		defaultMessage: 'Enter a player name, UUID, or single-entity selector.',
	},
	'app.lab.command-generator.arg.fetchprofile.profile.display': {
		id: 'app.lab.command-generator.arg.fetchprofile.profile.display',
		defaultMessage: 'Name, UUID, or selector',
	},
	'app.lab.command-generator.arg.fetchprofile.source.desc': {
		id: 'app.lab.command-generator.arg.fetchprofile.source.desc',
		defaultMessage: 'Fetch a profile by player name, UUID, or entity.',
	},
	'app.lab.command-generator.arg.fetchprofile.source.display': {
		id: 'app.lab.command-generator.arg.fetchprofile.source.display',
		defaultMessage: 'Profile source',
	},
	'app.lab.command-generator.arg.fill.block.desc': {
		id: 'app.lab.command-generator.arg.fill.block.desc',
		defaultMessage: 'Block ID used to fill the region',
	},
	'app.lab.command-generator.arg.fill.block.display': {
		id: 'app.lab.command-generator.arg.fill.block.display',
		defaultMessage: 'Blocks',
	},
	'app.lab.command-generator.arg.fill.from.desc': {
		id: 'app.lab.command-generator.arg.fill.from.desc',
		defaultMessage: 'Start position',
	},
	'app.lab.command-generator.arg.fill.from.display': {
		id: 'app.lab.command-generator.arg.fill.from.display',
		defaultMessage: 'Start position',
	},
	'app.lab.command-generator.arg.fill.mode.desc': {
		id: 'app.lab.command-generator.arg.fill.mode.desc',
		defaultMessage: 'replace / keep / outline / hollow / destroy',
	},
	'app.lab.command-generator.arg.fill.mode.display': {
		id: 'app.lab.command-generator.arg.fill.mode.display',
		defaultMessage: 'Mode',
	},
	'app.lab.command-generator.arg.fill.to.desc': {
		id: 'app.lab.command-generator.arg.fill.to.desc',
		defaultMessage: 'End position',
	},
	'app.lab.command-generator.arg.fill.to.display': {
		id: 'app.lab.command-generator.arg.fill.to.display',
		defaultMessage: 'End position',
	},
	'app.lab.command-generator.arg.fillbiome.biome.desc': {
		id: 'app.lab.command-generator.arg.fillbiome.biome.desc',
		defaultMessage: 'Biome ID',
	},
	'app.lab.command-generator.arg.fillbiome.biome.display': {
		id: 'app.lab.command-generator.arg.fillbiome.biome.display',
		defaultMessage: 'Biome',
	},
	'app.lab.command-generator.arg.fillbiome.from.desc': {
		id: 'app.lab.command-generator.arg.fillbiome.from.desc',
		defaultMessage: 'Start position',
	},
	'app.lab.command-generator.arg.fillbiome.from.display': {
		id: 'app.lab.command-generator.arg.fillbiome.from.display',
		defaultMessage: 'Start position',
	},
	'app.lab.command-generator.arg.fillbiome.to.desc': {
		id: 'app.lab.command-generator.arg.fillbiome.to.desc',
		defaultMessage: 'End position',
	},
	'app.lab.command-generator.arg.fillbiome.to.display': {
		id: 'app.lab.command-generator.arg.fillbiome.to.display',
		defaultMessage: 'End position',
	},
	'app.lab.command-generator.arg.forceload.action.desc': {
		id: 'app.lab.command-generator.arg.forceload.action.desc',
		defaultMessage: 'add / remove / query',
	},
	'app.lab.command-generator.arg.forceload.action.display': {
		id: 'app.lab.command-generator.arg.forceload.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.forceload.from.desc': {
		id: 'app.lab.command-generator.arg.forceload.from.desc',
		defaultMessage: 'Start position',
	},
	'app.lab.command-generator.arg.forceload.from.display': {
		id: 'app.lab.command-generator.arg.forceload.from.display',
		defaultMessage: 'Start position',
	},
	'app.lab.command-generator.arg.forceload.to.desc': {
		id: 'app.lab.command-generator.arg.forceload.to.desc',
		defaultMessage: 'End position',
	},
	'app.lab.command-generator.arg.forceload.to.display': {
		id: 'app.lab.command-generator.arg.forceload.to.display',
		defaultMessage: 'End position',
	},
	'app.lab.command-generator.arg.function.name.desc': {
		id: 'app.lab.command-generator.arg.function.name.desc',
		defaultMessage: 'Function identifier',
	},
	'app.lab.command-generator.arg.function.name.display': {
		id: 'app.lab.command-generator.arg.function.name.display',
		defaultMessage: 'Function name',
	},
	'app.lab.command-generator.arg.gamemode.mode.desc': {
		id: 'app.lab.command-generator.arg.gamemode.mode.desc',
		defaultMessage: 'survival / creative / adventure / spectator',
	},
	'app.lab.command-generator.arg.gamemode.mode.display': {
		id: 'app.lab.command-generator.arg.gamemode.mode.display',
		defaultMessage: 'Mode',
	},
	'app.lab.command-generator.arg.gamemode.target.desc': {
		id: 'app.lab.command-generator.arg.gamemode.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.gamemode.target.display': {
		id: 'app.lab.command-generator.arg.gamemode.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.gamerule.rule.desc': {
		id: 'app.lab.command-generator.arg.gamerule.rule.desc',
		defaultMessage: 'Game rule name',
	},
	'app.lab.command-generator.arg.gamerule.rule.display': {
		id: 'app.lab.command-generator.arg.gamerule.rule.display',
		defaultMessage: 'Rule name',
	},
	'app.lab.command-generator.arg.gamerule.value.desc': {
		id: 'app.lab.command-generator.arg.gamerule.value.desc',
		defaultMessage: 'Game rule value (true/false or a number)',
	},
	'app.lab.command-generator.arg.gamerule.value.desc.26': {
		id: 'app.lab.command-generator.arg.gamerule.value.desc.26',
		defaultMessage:
			'Enter a boolean or integer for the selected game rule. A fire spread radius of 0 stops fire spread.',
	},
	'app.lab.command-generator.arg.gamerule.value.display': {
		id: 'app.lab.command-generator.arg.gamerule.value.display',
		defaultMessage: 'Value',
	},
	'app.lab.command-generator.arg.give.count.desc': {
		id: 'app.lab.command-generator.arg.give.count.desc',
		defaultMessage: 'Item count',
	},
	'app.lab.command-generator.arg.give.count.display': {
		id: 'app.lab.command-generator.arg.give.count.display',
		defaultMessage: 'Amount',
	},
	'app.lab.command-generator.arg.give.item.desc': {
		id: 'app.lab.command-generator.arg.give.item.desc',
		defaultMessage: 'Item ID',
	},
	'app.lab.command-generator.arg.give.item.display': {
		id: 'app.lab.command-generator.arg.give.item.display',
		defaultMessage: 'Items',
	},
	'app.lab.command-generator.arg.give.nbt.desc': {
		id: 'app.lab.command-generator.arg.give.nbt.desc',
		defaultMessage: 'Item NBT data',
	},
	'app.lab.command-generator.arg.give.nbt.display': {
		id: 'app.lab.command-generator.arg.give.nbt.display',
		defaultMessage: 'NBT',
	},
	'app.lab.command-generator.arg.give.target.desc': {
		id: 'app.lab.command-generator.arg.give.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.give.target.display': {
		id: 'app.lab.command-generator.arg.give.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.help.command.desc': {
		id: 'app.lab.command-generator.arg.help.command.desc',
		defaultMessage: 'Command name to query',
	},
	'app.lab.command-generator.arg.help.command.display': {
		id: 'app.lab.command-generator.arg.help.command.display',
		defaultMessage: 'Command name',
	},
	'app.lab.command-generator.arg.item.action.desc': {
		id: 'app.lab.command-generator.arg.item.action.desc',
		defaultMessage: 'replace / modify / block',
	},
	'app.lab.command-generator.arg.item.action.display': {
		id: 'app.lab.command-generator.arg.item.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.item.count.desc': {
		id: 'app.lab.command-generator.arg.item.count.desc',
		defaultMessage: 'Item count',
	},
	'app.lab.command-generator.arg.item.count.display': {
		id: 'app.lab.command-generator.arg.item.count.display',
		defaultMessage: 'Amount',
	},
	'app.lab.command-generator.arg.item.item.desc': {
		id: 'app.lab.command-generator.arg.item.item.desc',
		defaultMessage: 'Item ID',
	},
	'app.lab.command-generator.arg.item.item.display': {
		id: 'app.lab.command-generator.arg.item.item.display',
		defaultMessage: 'Items',
	},
	'app.lab.command-generator.arg.item.modifier.desc': {
		id: 'app.lab.command-generator.arg.item.modifier.desc',
		defaultMessage: 'Resource ID of the item modifier.',
	},
	'app.lab.command-generator.arg.item.modifier.display': {
		id: 'app.lab.command-generator.arg.item.modifier.display',
		defaultMessage: 'Item modifier',
	},
	'app.lab.command-generator.arg.item.nbt.desc': {
		id: 'app.lab.command-generator.arg.item.nbt.desc',
		defaultMessage: 'Item NBT data',
	},
	'app.lab.command-generator.arg.item.nbt.display': {
		id: 'app.lab.command-generator.arg.item.nbt.display',
		defaultMessage: 'NBT',
	},
	'app.lab.command-generator.arg.item.pos.desc': {
		id: 'app.lab.command-generator.arg.item.pos.desc',
		defaultMessage: 'Container position',
	},
	'app.lab.command-generator.arg.item.pos.display': {
		id: 'app.lab.command-generator.arg.item.pos.display',
		defaultMessage: 'Container position',
	},
	'app.lab.command-generator.arg.item.slot.desc': {
		id: 'app.lab.command-generator.arg.item.slot.desc',
		defaultMessage: 'Slot name, such as container.0',
	},
	'app.lab.command-generator.arg.item.slot.desc.26': {
		id: 'app.lab.command-generator.arg.item.slot.desc.26',
		defaultMessage: 'Slot range, inline slot source, or namespaced slot source ID.',
	},
	'app.lab.command-generator.arg.item.slot.display': {
		id: 'app.lab.command-generator.arg.item.slot.display',
		defaultMessage: 'Slot',
	},
	'app.lab.command-generator.arg.item.source.desc': {
		id: 'app.lab.command-generator.arg.item.source.desc',
		defaultMessage: 'Replace the slots with the selected item.',
	},
	'app.lab.command-generator.arg.item.source.display': {
		id: 'app.lab.command-generator.arg.item.source.display',
		defaultMessage: 'Replacement source',
	},
	'app.lab.command-generator.arg.item.target.desc': {
		id: 'app.lab.command-generator.arg.item.target.desc',
		defaultMessage: 'Entity whose items will be modified.',
	},
	'app.lab.command-generator.arg.item.target.display': {
		id: 'app.lab.command-generator.arg.item.target.display',
		defaultMessage: 'Target entity',
	},
	'app.lab.command-generator.arg.item.targetType.desc': {
		id: 'app.lab.command-generator.arg.item.targetType.desc',
		defaultMessage: 'Choose a block or an entity.',
	},
	'app.lab.command-generator.arg.item.targetType.display': {
		id: 'app.lab.command-generator.arg.item.targetType.display',
		defaultMessage: 'Target type',
	},
	'app.lab.command-generator.arg.jfr.action.desc': {
		id: 'app.lab.command-generator.arg.jfr.action.desc',
		defaultMessage: 'start / stop',
	},
	'app.lab.command-generator.arg.jfr.action.display': {
		id: 'app.lab.command-generator.arg.jfr.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.kick.reason.desc': {
		id: 'app.lab.command-generator.arg.kick.reason.desc',
		defaultMessage: 'Reason for kicking the player',
	},
	'app.lab.command-generator.arg.kick.reason.display': {
		id: 'app.lab.command-generator.arg.kick.reason.display',
		defaultMessage: 'Reason',
	},
	'app.lab.command-generator.arg.kick.target.desc': {
		id: 'app.lab.command-generator.arg.kick.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.kick.target.display': {
		id: 'app.lab.command-generator.arg.kick.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.kill.target.desc': {
		id: 'app.lab.command-generator.arg.kill.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.kill.target.display': {
		id: 'app.lab.command-generator.arg.kill.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.list.uuids.desc': {
		id: 'app.lab.command-generator.arg.list.uuids.desc',
		defaultMessage: 'Whether to show UUIDs',
	},
	'app.lab.command-generator.arg.list.uuids.display': {
		id: 'app.lab.command-generator.arg.list.uuids.display',
		defaultMessage: 'Show UUIDs',
	},
	'app.lab.command-generator.arg.locate.id.desc': {
		id: 'app.lab.command-generator.arg.locate.id.desc',
		defaultMessage: 'Target ID to locate',
	},
	'app.lab.command-generator.arg.locate.id.display': {
		id: 'app.lab.command-generator.arg.locate.id.display',
		defaultMessage: 'Identifier',
	},
	'app.lab.command-generator.arg.locate.type.desc': {
		id: 'app.lab.command-generator.arg.locate.type.desc',
		defaultMessage: 'structure / biome / poi',
	},
	'app.lab.command-generator.arg.locate.type.display': {
		id: 'app.lab.command-generator.arg.locate.type.display',
		defaultMessage: 'Type',
	},
	'app.lab.command-generator.arg.loot.action.desc': {
		id: 'app.lab.command-generator.arg.loot.action.desc',
		defaultMessage: 'give / insert / spawn',
	},
	'app.lab.command-generator.arg.loot.action.display': {
		id: 'app.lab.command-generator.arg.loot.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.loot.source.desc': {
		id: 'app.lab.command-generator.arg.loot.source.desc',
		defaultMessage: 'Loot source',
	},
	'app.lab.command-generator.arg.loot.source.display': {
		id: 'app.lab.command-generator.arg.loot.source.display',
		defaultMessage: 'Source',
	},
	'app.lab.command-generator.arg.loot.target.desc': {
		id: 'app.lab.command-generator.arg.loot.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.loot.target.display': {
		id: 'app.lab.command-generator.arg.loot.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.me.message.desc': {
		id: 'app.lab.command-generator.arg.me.message.desc',
		defaultMessage: 'Message text',
	},
	'app.lab.command-generator.arg.me.message.display': {
		id: 'app.lab.command-generator.arg.me.message.display',
		defaultMessage: 'Message',
	},
	'app.lab.command-generator.arg.msg.message.desc': {
		id: 'app.lab.command-generator.arg.msg.message.desc',
		defaultMessage: 'Private message text',
	},
	'app.lab.command-generator.arg.msg.message.display': {
		id: 'app.lab.command-generator.arg.msg.message.display',
		defaultMessage: 'Message',
	},
	'app.lab.command-generator.arg.msg.target.desc': {
		id: 'app.lab.command-generator.arg.msg.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.msg.target.display': {
		id: 'app.lab.command-generator.arg.msg.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.op.target.desc': {
		id: 'app.lab.command-generator.arg.op.target.desc',
		defaultMessage: 'Name of the player to grant operator permission to',
	},
	'app.lab.command-generator.arg.op.target.display': {
		id: 'app.lab.command-generator.arg.op.target.display',
		defaultMessage: 'Player name',
	},
	'app.lab.command-generator.arg.pardon-ip.target.desc': {
		id: 'app.lab.command-generator.arg.pardon-ip.target.desc',
		defaultMessage: 'IP address to unban',
	},
	'app.lab.command-generator.arg.pardon-ip.target.display': {
		id: 'app.lab.command-generator.arg.pardon-ip.target.display',
		defaultMessage: 'IP address',
	},
	'app.lab.command-generator.arg.pardon.target.desc': {
		id: 'app.lab.command-generator.arg.pardon.target.desc',
		defaultMessage: 'Player name to unban',
	},
	'app.lab.command-generator.arg.pardon.target.display': {
		id: 'app.lab.command-generator.arg.pardon.target.display',
		defaultMessage: 'Player name',
	},
	'app.lab.command-generator.arg.particle.count.desc': {
		id: 'app.lab.command-generator.arg.particle.count.desc',
		defaultMessage: 'Particle count',
	},
	'app.lab.command-generator.arg.particle.count.display': {
		id: 'app.lab.command-generator.arg.particle.count.display',
		defaultMessage: 'Amount',
	},
	'app.lab.command-generator.arg.particle.delta.desc': {
		id: 'app.lab.command-generator.arg.particle.delta.desc',
		defaultMessage: 'Spread range',
	},
	'app.lab.command-generator.arg.particle.delta.display': {
		id: 'app.lab.command-generator.arg.particle.delta.display',
		defaultMessage: 'Spread',
	},
	'app.lab.command-generator.arg.particle.mode.desc': {
		id: 'app.lab.command-generator.arg.particle.mode.desc',
		defaultMessage: 'normal / force',
	},
	'app.lab.command-generator.arg.particle.mode.display': {
		id: 'app.lab.command-generator.arg.particle.mode.display',
		defaultMessage: 'Mode',
	},
	'app.lab.command-generator.arg.particle.name.desc': {
		id: 'app.lab.command-generator.arg.particle.name.desc',
		defaultMessage: 'Particle ID',
	},
	'app.lab.command-generator.arg.particle.name.display': {
		id: 'app.lab.command-generator.arg.particle.name.display',
		defaultMessage: 'Particle',
	},
	'app.lab.command-generator.arg.particle.pos.desc': {
		id: 'app.lab.command-generator.arg.particle.pos.desc',
		defaultMessage: 'Spawn position',
	},
	'app.lab.command-generator.arg.particle.pos.display': {
		id: 'app.lab.command-generator.arg.particle.pos.display',
		defaultMessage: 'Position',
	},
	'app.lab.command-generator.arg.particle.speed.desc': {
		id: 'app.lab.command-generator.arg.particle.speed.desc',
		defaultMessage: 'Particle speed',
	},
	'app.lab.command-generator.arg.particle.speed.display': {
		id: 'app.lab.command-generator.arg.particle.speed.display',
		defaultMessage: 'Speed',
	},
	'app.lab.command-generator.arg.placefeature.id.desc': {
		id: 'app.lab.command-generator.arg.placefeature.id.desc',
		defaultMessage: 'Configured feature identifier',
	},
	'app.lab.command-generator.arg.placefeature.id.display': {
		id: 'app.lab.command-generator.arg.placefeature.id.display',
		defaultMessage: 'Feature ID',
	},
	'app.lab.command-generator.arg.placefeature.pos.desc': {
		id: 'app.lab.command-generator.arg.placefeature.pos.desc',
		defaultMessage: 'Placement position',
	},
	'app.lab.command-generator.arg.placefeature.pos.display': {
		id: 'app.lab.command-generator.arg.placefeature.pos.display',
		defaultMessage: 'Position',
	},
	'app.lab.command-generator.arg.playsound.minVolume.desc': {
		id: 'app.lab.command-generator.arg.playsound.minVolume.desc',
		defaultMessage: 'Minimum volume',
	},
	'app.lab.command-generator.arg.playsound.minVolume.display': {
		id: 'app.lab.command-generator.arg.playsound.minVolume.display',
		defaultMessage: 'Minimum volume',
	},
	'app.lab.command-generator.arg.playsound.pitch.desc': {
		id: 'app.lab.command-generator.arg.playsound.pitch.desc',
		defaultMessage: 'Pitch',
	},
	'app.lab.command-generator.arg.playsound.pitch.display': {
		id: 'app.lab.command-generator.arg.playsound.pitch.display',
		defaultMessage: 'Pitch',
	},
	'app.lab.command-generator.arg.playsound.pos.desc': {
		id: 'app.lab.command-generator.arg.playsound.pos.desc',
		defaultMessage: 'Playback position',
	},
	'app.lab.command-generator.arg.playsound.pos.display': {
		id: 'app.lab.command-generator.arg.playsound.pos.display',
		defaultMessage: 'Position',
	},
	'app.lab.command-generator.arg.playsound.sound.desc': {
		id: 'app.lab.command-generator.arg.playsound.sound.desc',
		defaultMessage: 'Sound ID',
	},
	'app.lab.command-generator.arg.playsound.sound.display': {
		id: 'app.lab.command-generator.arg.playsound.sound.display',
		defaultMessage: 'Sound',
	},
	'app.lab.command-generator.arg.playsound.source.desc': {
		id: 'app.lab.command-generator.arg.playsound.source.desc',
		defaultMessage: 'Sound source category',
	},
	'app.lab.command-generator.arg.playsound.source.display': {
		id: 'app.lab.command-generator.arg.playsound.source.display',
		defaultMessage: 'Source',
	},
	'app.lab.command-generator.arg.playsound.target.desc': {
		id: 'app.lab.command-generator.arg.playsound.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.playsound.target.display': {
		id: 'app.lab.command-generator.arg.playsound.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.playsound.volume.desc': {
		id: 'app.lab.command-generator.arg.playsound.volume.desc',
		defaultMessage: 'Volume',
	},
	'app.lab.command-generator.arg.playsound.volume.display': {
		id: 'app.lab.command-generator.arg.playsound.volume.display',
		defaultMessage: 'Volume',
	},
	'app.lab.command-generator.arg.posteffect.action.desc': {
		id: 'app.lab.command-generator.arg.posteffect.action.desc',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.posteffect.action.display': {
		id: 'app.lab.command-generator.arg.posteffect.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.posteffect.effect.desc': {
		id: 'app.lab.command-generator.arg.posteffect.effect.desc',
		defaultMessage: 'Resource ID of the client post-processing effect.',
	},
	'app.lab.command-generator.arg.posteffect.effect.display': {
		id: 'app.lab.command-generator.arg.posteffect.effect.display',
		defaultMessage: 'Post-processing effect',
	},
	'app.lab.command-generator.arg.posteffect.target.desc': {
		id: 'app.lab.command-generator.arg.posteffect.target.desc',
		defaultMessage: 'Players',
	},
	'app.lab.command-generator.arg.posteffect.target.display': {
		id: 'app.lab.command-generator.arg.posteffect.target.display',
		defaultMessage: 'Players',
	},
	'app.lab.command-generator.arg.publish.allowCheats.desc': {
		id: 'app.lab.command-generator.arg.publish.allowCheats.desc',
		defaultMessage: 'Whether to allow cheats',
	},
	'app.lab.command-generator.arg.publish.allowCheats.display': {
		id: 'app.lab.command-generator.arg.publish.allowCheats.display',
		defaultMessage: 'Allow cheats',
	},
	'app.lab.command-generator.arg.publish.gamemode.desc': {
		id: 'app.lab.command-generator.arg.publish.gamemode.desc',
		defaultMessage: 'Default game mode',
	},
	'app.lab.command-generator.arg.publish.gamemode.display': {
		id: 'app.lab.command-generator.arg.publish.gamemode.display',
		defaultMessage: 'Game mode',
	},
	'app.lab.command-generator.arg.publish.port.desc': {
		id: 'app.lab.command-generator.arg.publish.port.desc',
		defaultMessage: 'LAN port',
	},
	'app.lab.command-generator.arg.publish.port.display': {
		id: 'app.lab.command-generator.arg.publish.port.display',
		defaultMessage: 'Port',
	},
	'app.lab.command-generator.arg.random.action.desc': {
		id: 'app.lab.command-generator.arg.random.action.desc',
		defaultMessage: 'value / roll / reset',
	},
	'app.lab.command-generator.arg.random.action.display': {
		id: 'app.lab.command-generator.arg.random.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.random.range.desc': {
		id: 'app.lab.command-generator.arg.random.range.desc',
		defaultMessage: 'Numeric range, such as 1..100',
	},
	'app.lab.command-generator.arg.random.range.display': {
		id: 'app.lab.command-generator.arg.random.range.display',
		defaultMessage: 'Range',
	},
	'app.lab.command-generator.arg.random.sequence.desc': {
		id: 'app.lab.command-generator.arg.random.sequence.desc',
		defaultMessage: 'Random sequence ID',
	},
	'app.lab.command-generator.arg.random.sequence.display': {
		id: 'app.lab.command-generator.arg.random.sequence.display',
		defaultMessage: 'Sequence',
	},
	'app.lab.command-generator.arg.recipe.action.desc': {
		id: 'app.lab.command-generator.arg.recipe.action.desc',
		defaultMessage: 'give / take',
	},
	'app.lab.command-generator.arg.recipe.action.display': {
		id: 'app.lab.command-generator.arg.recipe.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.recipe.recipe.desc': {
		id: 'app.lab.command-generator.arg.recipe.recipe.desc',
		defaultMessage: 'Recipe ID; leave empty for all recipes',
	},
	'app.lab.command-generator.arg.recipe.recipe.display': {
		id: 'app.lab.command-generator.arg.recipe.recipe.display',
		defaultMessage: 'Recipe',
	},
	'app.lab.command-generator.arg.recipe.target.desc': {
		id: 'app.lab.command-generator.arg.recipe.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.recipe.target.display': {
		id: 'app.lab.command-generator.arg.recipe.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.replaceitem.count.desc': {
		id: 'app.lab.command-generator.arg.replaceitem.count.desc',
		defaultMessage: 'Item count',
	},
	'app.lab.command-generator.arg.replaceitem.count.display': {
		id: 'app.lab.command-generator.arg.replaceitem.count.display',
		defaultMessage: 'Amount',
	},
	'app.lab.command-generator.arg.replaceitem.item.desc': {
		id: 'app.lab.command-generator.arg.replaceitem.item.desc',
		defaultMessage: 'Item ID',
	},
	'app.lab.command-generator.arg.replaceitem.item.display': {
		id: 'app.lab.command-generator.arg.replaceitem.item.display',
		defaultMessage: 'Items',
	},
	'app.lab.command-generator.arg.replaceitem.nbt.desc': {
		id: 'app.lab.command-generator.arg.replaceitem.nbt.desc',
		defaultMessage: 'Item NBT data',
	},
	'app.lab.command-generator.arg.replaceitem.nbt.display': {
		id: 'app.lab.command-generator.arg.replaceitem.nbt.display',
		defaultMessage: 'NBT',
	},
	'app.lab.command-generator.arg.replaceitem.slot.desc': {
		id: 'app.lab.command-generator.arg.replaceitem.slot.desc',
		defaultMessage: 'Slot name, such as container.0',
	},
	'app.lab.command-generator.arg.replaceitem.slot.display': {
		id: 'app.lab.command-generator.arg.replaceitem.slot.display',
		defaultMessage: 'Slot',
	},
	'app.lab.command-generator.arg.replaceitem.target.desc': {
		id: 'app.lab.command-generator.arg.replaceitem.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.replaceitem.target.display': {
		id: 'app.lab.command-generator.arg.replaceitem.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.ride.action.desc': {
		id: 'app.lab.command-generator.arg.ride.action.desc',
		defaultMessage: 'mount / dismount',
	},
	'app.lab.command-generator.arg.ride.action.display': {
		id: 'app.lab.command-generator.arg.ride.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.ride.target.desc': {
		id: 'app.lab.command-generator.arg.ride.target.desc',
		defaultMessage: 'Entity that will ride',
	},
	'app.lab.command-generator.arg.ride.target.display': {
		id: 'app.lab.command-generator.arg.ride.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.ride.vehicle.desc': {
		id: 'app.lab.command-generator.arg.ride.vehicle.desc',
		defaultMessage: 'Entity to ride',
	},
	'app.lab.command-generator.arg.ride.vehicle.display': {
		id: 'app.lab.command-generator.arg.ride.vehicle.display',
		defaultMessage: 'Vehicle',
	},
	'app.lab.command-generator.arg.say.message.desc': {
		id: 'app.lab.command-generator.arg.say.message.desc',
		defaultMessage: 'Message to broadcast',
	},
	'app.lab.command-generator.arg.say.message.display': {
		id: 'app.lab.command-generator.arg.say.message.display',
		defaultMessage: 'Message',
	},
	'app.lab.command-generator.arg.schedule.action.desc': {
		id: 'app.lab.command-generator.arg.schedule.action.desc',
		defaultMessage: 'clear / function',
	},
	'app.lab.command-generator.arg.schedule.action.display': {
		id: 'app.lab.command-generator.arg.schedule.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.schedule.name.desc': {
		id: 'app.lab.command-generator.arg.schedule.name.desc',
		defaultMessage: 'Schedule name or function ID',
	},
	'app.lab.command-generator.arg.schedule.name.display': {
		id: 'app.lab.command-generator.arg.schedule.name.display',
		defaultMessage: 'Name',
	},
	'app.lab.command-generator.arg.schedule.time.desc': {
		id: 'app.lab.command-generator.arg.schedule.time.desc',
		defaultMessage: 'Delay, such as 10s or 1d',
	},
	'app.lab.command-generator.arg.schedule.time.display': {
		id: 'app.lab.command-generator.arg.schedule.time.display',
		defaultMessage: 'Time',
	},
	'app.lab.command-generator.arg.scoreboard.action.desc': {
		id: 'app.lab.command-generator.arg.scoreboard.action.desc',
		defaultMessage: 'add / remove / list / set / get / reset / operation',
	},
	'app.lab.command-generator.arg.scoreboard.action.display': {
		id: 'app.lab.command-generator.arg.scoreboard.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.scoreboard.category.desc': {
		id: 'app.lab.command-generator.arg.scoreboard.category.desc',
		defaultMessage: 'objectives / players',
	},
	'app.lab.command-generator.arg.scoreboard.category.display': {
		id: 'app.lab.command-generator.arg.scoreboard.category.display',
		defaultMessage: 'Category',
	},
	'app.lab.command-generator.arg.scoreboard.objective.desc': {
		id: 'app.lab.command-generator.arg.scoreboard.objective.desc',
		defaultMessage: 'Scoreboard objective name',
	},
	'app.lab.command-generator.arg.scoreboard.objective.display': {
		id: 'app.lab.command-generator.arg.scoreboard.objective.display',
		defaultMessage: 'Objective',
	},
	'app.lab.command-generator.arg.setblock.block.desc': {
		id: 'app.lab.command-generator.arg.setblock.block.desc',
		defaultMessage: 'Block ID',
	},
	'app.lab.command-generator.arg.setblock.block.display': {
		id: 'app.lab.command-generator.arg.setblock.block.display',
		defaultMessage: 'Blocks',
	},
	'app.lab.command-generator.arg.setblock.mode.desc': {
		id: 'app.lab.command-generator.arg.setblock.mode.desc',
		defaultMessage: 'replace / keep / destroy',
	},
	'app.lab.command-generator.arg.setblock.mode.display': {
		id: 'app.lab.command-generator.arg.setblock.mode.display',
		defaultMessage: 'Mode',
	},
	'app.lab.command-generator.arg.setblock.pos.desc': {
		id: 'app.lab.command-generator.arg.setblock.pos.desc',
		defaultMessage: 'Position',
	},
	'app.lab.command-generator.arg.setblock.pos.display': {
		id: 'app.lab.command-generator.arg.setblock.pos.display',
		defaultMessage: 'Position',
	},
	'app.lab.command-generator.arg.setidletimeout.minutes.desc': {
		id: 'app.lab.command-generator.arg.setidletimeout.minutes.desc',
		defaultMessage: 'Idle timeout in minutes',
	},
	'app.lab.command-generator.arg.setidletimeout.minutes.display': {
		id: 'app.lab.command-generator.arg.setidletimeout.minutes.display',
		defaultMessage: 'Minutes',
	},
	'app.lab.command-generator.arg.setworldspawn.angle.desc': {
		id: 'app.lab.command-generator.arg.setworldspawn.angle.desc',
		defaultMessage: 'Spawn viewing angle',
	},
	'app.lab.command-generator.arg.setworldspawn.angle.display': {
		id: 'app.lab.command-generator.arg.setworldspawn.angle.display',
		defaultMessage: 'Angle',
	},
	'app.lab.command-generator.arg.setworldspawn.pos.desc': {
		id: 'app.lab.command-generator.arg.setworldspawn.pos.desc',
		defaultMessage: 'World spawn position',
	},
	'app.lab.command-generator.arg.setworldspawn.pos.display': {
		id: 'app.lab.command-generator.arg.setworldspawn.pos.display',
		defaultMessage: 'Position',
	},
	'app.lab.command-generator.arg.spawnpoint.angle.desc': {
		id: 'app.lab.command-generator.arg.spawnpoint.angle.desc',
		defaultMessage: 'Respawn viewing angle',
	},
	'app.lab.command-generator.arg.spawnpoint.angle.display': {
		id: 'app.lab.command-generator.arg.spawnpoint.angle.display',
		defaultMessage: 'Angle',
	},
	'app.lab.command-generator.arg.spawnpoint.pos.desc': {
		id: 'app.lab.command-generator.arg.spawnpoint.pos.desc',
		defaultMessage: 'Respawn position',
	},
	'app.lab.command-generator.arg.spawnpoint.pos.display': {
		id: 'app.lab.command-generator.arg.spawnpoint.pos.display',
		defaultMessage: 'Position',
	},
	'app.lab.command-generator.arg.spawnpoint.target.desc': {
		id: 'app.lab.command-generator.arg.spawnpoint.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.spawnpoint.target.display': {
		id: 'app.lab.command-generator.arg.spawnpoint.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.spectate.player.desc': {
		id: 'app.lab.command-generator.arg.spectate.player.desc',
		defaultMessage: 'Player in spectator mode',
	},
	'app.lab.command-generator.arg.spectate.player.display': {
		id: 'app.lab.command-generator.arg.spectate.player.display',
		defaultMessage: 'Spectator',
	},
	'app.lab.command-generator.arg.spectate.target.desc': {
		id: 'app.lab.command-generator.arg.spectate.target.desc',
		defaultMessage: 'Target entity to spectate',
	},
	'app.lab.command-generator.arg.spectate.target.display': {
		id: 'app.lab.command-generator.arg.spectate.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.spreadplayers.center.desc': {
		id: 'app.lab.command-generator.arg.spreadplayers.center.desc',
		defaultMessage: 'Center of the spread area',
	},
	'app.lab.command-generator.arg.spreadplayers.center.display': {
		id: 'app.lab.command-generator.arg.spreadplayers.center.display',
		defaultMessage: 'Center position',
	},
	'app.lab.command-generator.arg.spreadplayers.maxRange.desc': {
		id: 'app.lab.command-generator.arg.spreadplayers.maxRange.desc',
		defaultMessage: 'Maximum range',
	},
	'app.lab.command-generator.arg.spreadplayers.maxRange.display': {
		id: 'app.lab.command-generator.arg.spreadplayers.maxRange.display',
		defaultMessage: 'Maximum range',
	},
	'app.lab.command-generator.arg.spreadplayers.respectTeams.desc': {
		id: 'app.lab.command-generator.arg.spreadplayers.respectTeams.desc',
		defaultMessage: 'Whether to spread entities together by team',
	},
	'app.lab.command-generator.arg.spreadplayers.respectTeams.display': {
		id: 'app.lab.command-generator.arg.spreadplayers.respectTeams.display',
		defaultMessage: 'Respect teams',
	},
	'app.lab.command-generator.arg.spreadplayers.spreadDistance.desc': {
		id: 'app.lab.command-generator.arg.spreadplayers.spreadDistance.desc',
		defaultMessage: 'Minimum distance between entities',
	},
	'app.lab.command-generator.arg.spreadplayers.spreadDistance.display': {
		id: 'app.lab.command-generator.arg.spreadplayers.spreadDistance.display',
		defaultMessage: 'Spread distance',
	},
	'app.lab.command-generator.arg.spreadplayers.target.desc': {
		id: 'app.lab.command-generator.arg.spreadplayers.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.spreadplayers.target.display': {
		id: 'app.lab.command-generator.arg.spreadplayers.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.stopsound.sound.desc': {
		id: 'app.lab.command-generator.arg.stopsound.sound.desc',
		defaultMessage: 'Sound ID',
	},
	'app.lab.command-generator.arg.stopsound.sound.display': {
		id: 'app.lab.command-generator.arg.stopsound.sound.display',
		defaultMessage: 'Sound',
	},
	'app.lab.command-generator.arg.stopsound.source.desc': {
		id: 'app.lab.command-generator.arg.stopsound.source.desc',
		defaultMessage: 'Sound source category',
	},
	'app.lab.command-generator.arg.stopsound.source.display': {
		id: 'app.lab.command-generator.arg.stopsound.source.display',
		defaultMessage: 'Source',
	},
	'app.lab.command-generator.arg.stopsound.target.desc': {
		id: 'app.lab.command-generator.arg.stopsound.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.stopsound.target.display': {
		id: 'app.lab.command-generator.arg.stopsound.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.stopwatch.action.desc': {
		id: 'app.lab.command-generator.arg.stopwatch.action.desc',
		defaultMessage: 'create / query / restart / remove',
	},
	'app.lab.command-generator.arg.stopwatch.action.display': {
		id: 'app.lab.command-generator.arg.stopwatch.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.stopwatch.id.desc': {
		id: 'app.lab.command-generator.arg.stopwatch.id.desc',
		defaultMessage: 'Stopwatch identifier',
	},
	'app.lab.command-generator.arg.stopwatch.id.display': {
		id: 'app.lab.command-generator.arg.stopwatch.id.display',
		defaultMessage: 'ID',
	},
	'app.lab.command-generator.arg.stopwatch.scale.desc': {
		id: 'app.lab.command-generator.arg.stopwatch.scale.desc',
		defaultMessage: 'Scale factor, such as 0.05 to convert ticks to seconds',
	},
	'app.lab.command-generator.arg.stopwatch.scale.display': {
		id: 'app.lab.command-generator.arg.stopwatch.scale.display',
		defaultMessage: 'Scale',
	},
	'app.lab.command-generator.arg.summon.entity.desc': {
		id: 'app.lab.command-generator.arg.summon.entity.desc',
		defaultMessage: 'Entity ID',
	},
	'app.lab.command-generator.arg.summon.entity.display': {
		id: 'app.lab.command-generator.arg.summon.entity.display',
		defaultMessage: 'Entity type',
	},
	'app.lab.command-generator.arg.summon.nbt.desc': {
		id: 'app.lab.command-generator.arg.summon.nbt.desc',
		defaultMessage: 'Entity NBT data',
	},
	'app.lab.command-generator.arg.summon.nbt.display': {
		id: 'app.lab.command-generator.arg.summon.nbt.display',
		defaultMessage: 'NBT',
	},
	'app.lab.command-generator.arg.summon.pos.desc': {
		id: 'app.lab.command-generator.arg.summon.pos.desc',
		defaultMessage: 'Spawn position',
	},
	'app.lab.command-generator.arg.summon.pos.display': {
		id: 'app.lab.command-generator.arg.summon.pos.display',
		defaultMessage: 'Position',
	},
	'app.lab.command-generator.arg.swing.animation.desc': {
		id: 'app.lab.command-generator.arg.swing.animation.desc',
		defaultMessage: 'Animation',
	},
	'app.lab.command-generator.arg.swing.animation.display': {
		id: 'app.lab.command-generator.arg.swing.animation.display',
		defaultMessage: 'Animation',
	},
	'app.lab.command-generator.arg.swing.duration.desc': {
		id: 'app.lab.command-generator.arg.swing.duration.desc',
		defaultMessage: 'Positive duration in ticks, seconds (s), or days (d).',
	},
	'app.lab.command-generator.arg.swing.duration.display': {
		id: 'app.lab.command-generator.arg.swing.duration.display',
		defaultMessage: 'Animation duration',
	},
	'app.lab.command-generator.arg.swing.hand.desc': {
		id: 'app.lab.command-generator.arg.swing.hand.desc',
		defaultMessage: 'mainhand / offhand',
	},
	'app.lab.command-generator.arg.swing.hand.display': {
		id: 'app.lab.command-generator.arg.swing.hand.display',
		defaultMessage: 'Hand',
	},
	'app.lab.command-generator.arg.swing.target.desc': {
		id: 'app.lab.command-generator.arg.swing.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.swing.target.display': {
		id: 'app.lab.command-generator.arg.swing.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.tag.action.desc': {
		id: 'app.lab.command-generator.arg.tag.action.desc',
		defaultMessage: 'add / remove / list',
	},
	'app.lab.command-generator.arg.tag.action.display': {
		id: 'app.lab.command-generator.arg.tag.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.tag.name.desc': {
		id: 'app.lab.command-generator.arg.tag.name.desc',
		defaultMessage: 'Tag name',
	},
	'app.lab.command-generator.arg.tag.name.display': {
		id: 'app.lab.command-generator.arg.tag.name.display',
		defaultMessage: 'Tag name',
	},
	'app.lab.command-generator.arg.tag.target.desc': {
		id: 'app.lab.command-generator.arg.tag.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.tag.target.display': {
		id: 'app.lab.command-generator.arg.tag.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.team.action.desc': {
		id: 'app.lab.command-generator.arg.team.action.desc',
		defaultMessage: 'add / remove / empty / join / leave / list / modify',
	},
	'app.lab.command-generator.arg.team.action.display': {
		id: 'app.lab.command-generator.arg.team.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.team.members.desc': {
		id: 'app.lab.command-generator.arg.team.members.desc',
		defaultMessage: 'Team member selector',
	},
	'app.lab.command-generator.arg.team.members.display': {
		id: 'app.lab.command-generator.arg.team.members.display',
		defaultMessage: 'Members',
	},
	'app.lab.command-generator.arg.team.team.desc': {
		id: 'app.lab.command-generator.arg.team.team.desc',
		defaultMessage: 'Team name',
	},
	'app.lab.command-generator.arg.team.team.display': {
		id: 'app.lab.command-generator.arg.team.team.display',
		defaultMessage: 'Team name',
	},
	'app.lab.command-generator.arg.teammsg.message.desc': {
		id: 'app.lab.command-generator.arg.teammsg.message.desc',
		defaultMessage: 'Message to send to the team',
	},
	'app.lab.command-generator.arg.teammsg.message.display': {
		id: 'app.lab.command-generator.arg.teammsg.message.display',
		defaultMessage: 'Message',
	},
	'app.lab.command-generator.arg.teleport.destination.desc': {
		id: 'app.lab.command-generator.arg.teleport.destination.desc',
		defaultMessage: 'Destination position or entity selector',
	},
	'app.lab.command-generator.arg.teleport.destination.display': {
		id: 'app.lab.command-generator.arg.teleport.destination.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.teleport.rotation.desc': {
		id: 'app.lab.command-generator.arg.teleport.rotation.desc',
		defaultMessage: 'Yaw / pitch',
	},
	'app.lab.command-generator.arg.teleport.rotation.display': {
		id: 'app.lab.command-generator.arg.teleport.rotation.display',
		defaultMessage: 'Rotation',
	},
	'app.lab.command-generator.arg.teleport.target.desc': {
		id: 'app.lab.command-generator.arg.teleport.target.desc',
		defaultMessage: 'Entities to teleport',
	},
	'app.lab.command-generator.arg.teleport.target.display': {
		id: 'app.lab.command-generator.arg.teleport.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.tell.message.desc': {
		id: 'app.lab.command-generator.arg.tell.message.desc',
		defaultMessage: 'Private message text',
	},
	'app.lab.command-generator.arg.tell.message.display': {
		id: 'app.lab.command-generator.arg.tell.message.display',
		defaultMessage: 'Message',
	},
	'app.lab.command-generator.arg.tell.target.desc': {
		id: 'app.lab.command-generator.arg.tell.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.tell.target.display': {
		id: 'app.lab.command-generator.arg.tell.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.tellraw.message.desc': {
		id: 'app.lab.command-generator.arg.tellraw.message.desc',
		defaultMessage: 'Message in JSON format',
	},
	'app.lab.command-generator.arg.tellraw.message.display': {
		id: 'app.lab.command-generator.arg.tellraw.message.display',
		defaultMessage: 'JSON message',
	},
	'app.lab.command-generator.arg.tellraw.target.desc': {
		id: 'app.lab.command-generator.arg.tellraw.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.tellraw.target.display': {
		id: 'app.lab.command-generator.arg.tellraw.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.tick.action.desc': {
		id: 'app.lab.command-generator.arg.tick.action.desc',
		defaultMessage: 'freeze / unfreeze / rate / sprint / step / query',
	},
	'app.lab.command-generator.arg.tick.action.display': {
		id: 'app.lab.command-generator.arg.tick.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.tick.value.desc': {
		id: 'app.lab.command-generator.arg.tick.value.desc',
		defaultMessage: 'Parameter value',
	},
	'app.lab.command-generator.arg.tick.value.display': {
		id: 'app.lab.command-generator.arg.tick.value.display',
		defaultMessage: 'Value',
	},
	'app.lab.command-generator.arg.time.action.desc': {
		id: 'app.lab.command-generator.arg.time.action.desc',
		defaultMessage: 'set / add / query',
	},
	'app.lab.command-generator.arg.time.action.display': {
		id: 'app.lab.command-generator.arg.time.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.time.clock.desc': {
		id: 'app.lab.command-generator.arg.time.clock.desc',
		defaultMessage: "Leave empty to use the current dimension's default clock.",
	},
	'app.lab.command-generator.arg.time.clock.display': {
		id: 'app.lab.command-generator.arg.time.clock.display',
		defaultMessage: 'World clock',
	},
	'app.lab.command-generator.arg.time.query.desc': {
		id: 'app.lab.command-generator.arg.time.query.desc',
		defaultMessage:
			'Enter time, gametime, or a timeline ID. Append repetition to query the cycle count.',
	},
	'app.lab.command-generator.arg.time.query.display': {
		id: 'app.lab.command-generator.arg.time.query.display',
		defaultMessage: 'Query',
	},
	'app.lab.command-generator.arg.time.rate.desc': {
		id: 'app.lab.command-generator.arg.time.rate.desc',
		defaultMessage: 'Time rate greater than 0 and no greater than 1000.',
	},
	'app.lab.command-generator.arg.time.rate.display': {
		id: 'app.lab.command-generator.arg.time.rate.display',
		defaultMessage: 'Clock rate',
	},
	'app.lab.command-generator.arg.time.value.desc': {
		id: 'app.lab.command-generator.arg.time.value.desc',
		defaultMessage: 'Time value (day/noon/night/midnight or a number)',
	},
	'app.lab.command-generator.arg.time.value.display': {
		id: 'app.lab.command-generator.arg.time.value.display',
		defaultMessage: 'Value',
	},
	'app.lab.command-generator.arg.title.action.desc': {
		id: 'app.lab.command-generator.arg.title.action.desc',
		defaultMessage: 'title / subtitle / actionbar / times / reset',
	},
	'app.lab.command-generator.arg.title.action.display': {
		id: 'app.lab.command-generator.arg.title.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.title.target.desc': {
		id: 'app.lab.command-generator.arg.title.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.title.target.display': {
		id: 'app.lab.command-generator.arg.title.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.title.value.desc': {
		id: 'app.lab.command-generator.arg.title.value.desc',
		defaultMessage: 'Title content or timing values (JSON text)',
	},
	'app.lab.command-generator.arg.title.value.display': {
		id: 'app.lab.command-generator.arg.title.value.display',
		defaultMessage: 'Content',
	},
	'app.lab.command-generator.arg.trigger.objective.desc': {
		id: 'app.lab.command-generator.arg.trigger.objective.desc',
		defaultMessage: 'Trigger objective name',
	},
	'app.lab.command-generator.arg.trigger.objective.display': {
		id: 'app.lab.command-generator.arg.trigger.objective.display',
		defaultMessage: 'Trigger',
	},
	'app.lab.command-generator.arg.trigger.value.desc': {
		id: 'app.lab.command-generator.arg.trigger.value.desc',
		defaultMessage: 'Value to set',
	},
	'app.lab.command-generator.arg.trigger.value.display': {
		id: 'app.lab.command-generator.arg.trigger.value.display',
		defaultMessage: 'Value',
	},
	'app.lab.command-generator.arg.waypoint.action.desc': {
		id: 'app.lab.command-generator.arg.waypoint.action.desc',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.waypoint.action.display': {
		id: 'app.lab.command-generator.arg.waypoint.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.waypoint.color.desc': {
		id: 'app.lab.command-generator.arg.waypoint.color.desc',
		defaultMessage: 'Color name or #RRGGBB',
	},
	'app.lab.command-generator.arg.waypoint.color.display': {
		id: 'app.lab.command-generator.arg.waypoint.color.display',
		defaultMessage: 'Color',
	},
	'app.lab.command-generator.arg.waypoint.name.desc': {
		id: 'app.lab.command-generator.arg.waypoint.name.desc',
		defaultMessage: 'Waypoint name',
	},
	'app.lab.command-generator.arg.waypoint.name.display': {
		id: 'app.lab.command-generator.arg.waypoint.name.display',
		defaultMessage: 'Name',
	},
	'app.lab.command-generator.arg.waypoint.pos.desc': {
		id: 'app.lab.command-generator.arg.waypoint.pos.desc',
		defaultMessage: 'Waypoint position',
	},
	'app.lab.command-generator.arg.waypoint.pos.display': {
		id: 'app.lab.command-generator.arg.waypoint.pos.display',
		defaultMessage: 'Position',
	},
	'app.lab.command-generator.arg.waypoint.property.desc': {
		id: 'app.lab.command-generator.arg.waypoint.property.desc',
		defaultMessage: 'Attribute',
	},
	'app.lab.command-generator.arg.waypoint.property.display': {
		id: 'app.lab.command-generator.arg.waypoint.property.display',
		defaultMessage: 'Attribute',
	},
	'app.lab.command-generator.arg.waypoint.value.desc': {
		id: 'app.lab.command-generator.arg.waypoint.value.desc',
		defaultMessage: 'Color or style',
	},
	'app.lab.command-generator.arg.waypoint.value.display': {
		id: 'app.lab.command-generator.arg.waypoint.value.display',
		defaultMessage: 'Color or style',
	},
	'app.lab.command-generator.arg.waypoint.waypoint.desc': {
		id: 'app.lab.command-generator.arg.waypoint.waypoint.desc',
		defaultMessage: 'Waypoint entity',
	},
	'app.lab.command-generator.arg.waypoint.waypoint.display': {
		id: 'app.lab.command-generator.arg.waypoint.waypoint.display',
		defaultMessage: 'Waypoint entity',
	},
	'app.lab.command-generator.arg.weather.duration.desc': {
		id: 'app.lab.command-generator.arg.weather.duration.desc',
		defaultMessage: 'Weather duration in seconds',
	},
	'app.lab.command-generator.arg.weather.duration.display': {
		id: 'app.lab.command-generator.arg.weather.duration.display',
		defaultMessage: 'Duration',
	},
	'app.lab.command-generator.arg.weather.type.desc': {
		id: 'app.lab.command-generator.arg.weather.type.desc',
		defaultMessage: 'clear / rain / thunder',
	},
	'app.lab.command-generator.arg.weather.type.display': {
		id: 'app.lab.command-generator.arg.weather.type.display',
		defaultMessage: 'Weather',
	},
	'app.lab.command-generator.arg.whitelist.action.desc': {
		id: 'app.lab.command-generator.arg.whitelist.action.desc',
		defaultMessage: 'add / remove / list / on / off / reload',
	},
	'app.lab.command-generator.arg.whitelist.action.display': {
		id: 'app.lab.command-generator.arg.whitelist.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.whitelist.target.desc': {
		id: 'app.lab.command-generator.arg.whitelist.target.desc',
		defaultMessage: 'Player name to add or remove',
	},
	'app.lab.command-generator.arg.whitelist.target.display': {
		id: 'app.lab.command-generator.arg.whitelist.target.display',
		defaultMessage: 'Player name',
	},
	'app.lab.command-generator.arg.worldborder.action.desc': {
		id: 'app.lab.command-generator.arg.worldborder.action.desc',
		defaultMessage: 'add / set / center / damage / get / warning',
	},
	'app.lab.command-generator.arg.worldborder.action.display': {
		id: 'app.lab.command-generator.arg.worldborder.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.worldborder.value.desc': {
		id: 'app.lab.command-generator.arg.worldborder.value.desc',
		defaultMessage: 'Parameter value',
	},
	'app.lab.command-generator.arg.worldborder.value.display': {
		id: 'app.lab.command-generator.arg.worldborder.value.display',
		defaultMessage: 'Value',
	},
	'app.lab.command-generator.arg.xp.action.desc': {
		id: 'app.lab.command-generator.arg.xp.action.desc',
		defaultMessage: 'add / set',
	},
	'app.lab.command-generator.arg.xp.action.display': {
		id: 'app.lab.command-generator.arg.xp.action.display',
		defaultMessage: 'Action',
	},
	'app.lab.command-generator.arg.xp.amount.desc': {
		id: 'app.lab.command-generator.arg.xp.amount.desc',
		defaultMessage: 'Experience amount',
	},
	'app.lab.command-generator.arg.xp.amount.display': {
		id: 'app.lab.command-generator.arg.xp.amount.display',
		defaultMessage: 'Amount',
	},
	'app.lab.command-generator.arg.xp.target.desc': {
		id: 'app.lab.command-generator.arg.xp.target.desc',
		defaultMessage: 'Target selector',
	},
	'app.lab.command-generator.arg.xp.target.display': {
		id: 'app.lab.command-generator.arg.xp.target.display',
		defaultMessage: 'Target',
	},
	'app.lab.command-generator.arg.xp.unit.desc': {
		id: 'app.lab.command-generator.arg.xp.unit.desc',
		defaultMessage: 'points / levels',
	},
	'app.lab.command-generator.arg.xp.unit.display': {
		id: 'app.lab.command-generator.arg.xp.unit.display',
		defaultMessage: 'Unit',
	},
	'app.lab.command-generator.cat.block': {
		id: 'app.lab.command-generator.cat.block',
		defaultMessage: 'Blocks',
	},
	'app.lab.command-generator.cat.communication': {
		id: 'app.lab.command-generator.cat.communication',
		defaultMessage: 'Communication',
	},
	'app.lab.command-generator.cat.data': {
		id: 'app.lab.command-generator.cat.data',
		defaultMessage: 'Data',
	},
	'app.lab.command-generator.cat.entity': {
		id: 'app.lab.command-generator.cat.entity',
		defaultMessage: 'Entities',
	},
	'app.lab.command-generator.cat.item': {
		id: 'app.lab.command-generator.cat.item',
		defaultMessage: 'Items',
	},
	'app.lab.command-generator.cat.misc': {
		id: 'app.lab.command-generator.cat.misc',
		defaultMessage: 'Other',
	},
	'app.lab.command-generator.cat.player': {
		id: 'app.lab.command-generator.cat.player',
		defaultMessage: 'Players',
	},
	'app.lab.command-generator.cat.server': {
		id: 'app.lab.command-generator.cat.server',
		defaultMessage: 'Server',
	},
	'app.lab.command-generator.cat.world': {
		id: 'app.lab.command-generator.cat.world',
		defaultMessage: 'World',
	},
	'app.lab.command-generator.cmd.advancement.desc': {
		id: 'app.lab.command-generator.cmd.advancement.desc',
		defaultMessage: 'Grant or revoke player advancements',
	},
	'app.lab.command-generator.cmd.attribute.desc': {
		id: 'app.lab.command-generator.cmd.attribute.desc',
		defaultMessage: 'Modify or query entity attributes',
	},
	'app.lab.command-generator.cmd.ban-ip.desc': {
		id: 'app.lab.command-generator.cmd.ban-ip.desc',
		defaultMessage: 'Ban an IP address',
	},
	'app.lab.command-generator.cmd.ban.desc': {
		id: 'app.lab.command-generator.cmd.ban.desc',
		defaultMessage: 'Ban a player',
	},
	'app.lab.command-generator.cmd.banlist.desc': {
		id: 'app.lab.command-generator.cmd.banlist.desc',
		defaultMessage: 'View the ban list',
	},
	'app.lab.command-generator.cmd.bossbar.desc': {
		id: 'app.lab.command-generator.cmd.bossbar.desc',
		defaultMessage: 'Create or modify boss bars',
	},
	'app.lab.command-generator.cmd.clear.desc': {
		id: 'app.lab.command-generator.cmd.clear.desc',
		defaultMessage: 'Clear items from player inventories',
	},
	'app.lab.command-generator.cmd.clone.desc': {
		id: 'app.lab.command-generator.cmd.clone.desc',
		defaultMessage: 'Clone a region of blocks',
	},
	'app.lab.command-generator.cmd.compute.desc': {
		id: 'app.lab.command-generator.cmd.compute.desc',
		defaultMessage: 'Evaluate a number provider.',
	},
	'app.lab.command-generator.cmd.damage.desc': {
		id: 'app.lab.command-generator.cmd.damage.desc',
		defaultMessage: 'Deal damage to entities',
	},
	'app.lab.command-generator.cmd.data.desc': {
		id: 'app.lab.command-generator.cmd.data.desc',
		defaultMessage: 'Read or modify block, entity, or storage NBT data',
	},
	'app.lab.command-generator.cmd.datapack.desc': {
		id: 'app.lab.command-generator.cmd.datapack.desc',
		defaultMessage: 'Control data pack loading',
	},
	'app.lab.command-generator.cmd.debug.desc': {
		id: 'app.lab.command-generator.cmd.debug.desc',
		defaultMessage: 'Run debugging commands',
	},
	'app.lab.command-generator.cmd.defaultgamemode.desc': {
		id: 'app.lab.command-generator.cmd.defaultgamemode.desc',
		defaultMessage: 'Set the default game mode for new players',
	},
	'app.lab.command-generator.cmd.deop.desc': {
		id: 'app.lab.command-generator.cmd.deop.desc',
		defaultMessage: "Remove a player's operator permission",
	},
	'app.lab.command-generator.cmd.dialog.desc': {
		id: 'app.lab.command-generator.cmd.dialog.desc',
		defaultMessage: 'Show a client dialog (1.21.6+)',
	},
	'app.lab.command-generator.cmd.difficulty.desc': {
		id: 'app.lab.command-generator.cmd.difficulty.desc',
		defaultMessage: 'Set the game difficulty',
	},
	'app.lab.command-generator.cmd.effect.desc': {
		id: 'app.lab.command-generator.cmd.effect.desc',
		defaultMessage: 'Apply or remove status effects',
	},
	'app.lab.command-generator.cmd.enchant.desc': {
		id: 'app.lab.command-generator.cmd.enchant.desc',
		defaultMessage: 'Enchant the held item',
	},
	'app.lab.command-generator.cmd.execute.desc': {
		id: 'app.lab.command-generator.cmd.execute.desc',
		defaultMessage: 'Run a command with a modified execution context',
	},
	'app.lab.command-generator.cmd.experience.desc': {
		id: 'app.lab.command-generator.cmd.experience.desc',
		defaultMessage: 'Add or set player experience',
	},
	'app.lab.command-generator.cmd.fetchprofile.desc': {
		id: 'app.lab.command-generator.cmd.fetchprofile.desc',
		defaultMessage: 'Fetch a player profile.',
	},
	'app.lab.command-generator.cmd.fill.desc': {
		id: 'app.lab.command-generator.cmd.fill.desc',
		defaultMessage: 'Fill a region with blocks',
	},
	'app.lab.command-generator.cmd.fillbiome.desc': {
		id: 'app.lab.command-generator.cmd.fillbiome.desc',
		defaultMessage: 'Fill a region with a biome (1.19+)',
	},
	'app.lab.command-generator.cmd.forceload.desc': {
		id: 'app.lab.command-generator.cmd.forceload.desc',
		defaultMessage: 'Force chunks to stay loaded or unload them',
	},
	'app.lab.command-generator.cmd.function.desc': {
		id: 'app.lab.command-generator.cmd.function.desc',
		defaultMessage: 'Run a function',
	},
	'app.lab.command-generator.cmd.gamemode.desc': {
		id: 'app.lab.command-generator.cmd.gamemode.desc',
		defaultMessage: "Set a player's game mode",
	},
	'app.lab.command-generator.cmd.gamerule.desc': {
		id: 'app.lab.command-generator.cmd.gamerule.desc',
		defaultMessage: 'Set game rules',
	},
	'app.lab.command-generator.cmd.give.desc': {
		id: 'app.lab.command-generator.cmd.give.desc',
		defaultMessage: 'Give items to players',
	},
	'app.lab.command-generator.cmd.help.desc': {
		id: 'app.lab.command-generator.cmd.help.desc',
		defaultMessage: 'Show command help',
	},
	'app.lab.command-generator.cmd.item.desc': {
		id: 'app.lab.command-generator.cmd.item.desc',
		defaultMessage: 'Modify items in blocks or entities',
	},
	'app.lab.command-generator.cmd.jfr.desc': {
		id: 'app.lab.command-generator.cmd.jfr.desc',
		defaultMessage: 'Profile performance with Java Flight Recorder',
	},
	'app.lab.command-generator.cmd.kick.desc': {
		id: 'app.lab.command-generator.cmd.kick.desc',
		defaultMessage: 'Kick a player',
	},
	'app.lab.command-generator.cmd.kill.desc': {
		id: 'app.lab.command-generator.cmd.kill.desc',
		defaultMessage: 'Kill entities',
	},
	'app.lab.command-generator.cmd.list.desc': {
		id: 'app.lab.command-generator.cmd.list.desc',
		defaultMessage: 'List online players',
	},
	'app.lab.command-generator.cmd.locate.desc': {
		id: 'app.lab.command-generator.cmd.locate.desc',
		defaultMessage: 'Locate the nearest structure or biome',
	},
	'app.lab.command-generator.cmd.locatebiome.desc': {
		id: 'app.lab.command-generator.cmd.locatebiome.desc',
		defaultMessage: 'Locate the nearest biome.',
	},
	'app.lab.command-generator.cmd.loot.desc': {
		id: 'app.lab.command-generator.cmd.loot.desc',
		defaultMessage: 'Generate loot',
	},
	'app.lab.command-generator.cmd.me.desc': {
		id: 'app.lab.command-generator.cmd.me.desc',
		defaultMessage: 'Send an action message',
	},
	'app.lab.command-generator.cmd.msg.desc': {
		id: 'app.lab.command-generator.cmd.msg.desc',
		defaultMessage: 'Send a private message',
	},
	'app.lab.command-generator.cmd.op.desc': {
		id: 'app.lab.command-generator.cmd.op.desc',
		defaultMessage: 'Grant operator permission to a player',
	},
	'app.lab.command-generator.cmd.pardon-ip.desc': {
		id: 'app.lab.command-generator.cmd.pardon-ip.desc',
		defaultMessage: 'Unban an IP address',
	},
	'app.lab.command-generator.cmd.pardon.desc': {
		id: 'app.lab.command-generator.cmd.pardon.desc',
		defaultMessage: 'Unban a player',
	},
	'app.lab.command-generator.cmd.particle.desc': {
		id: 'app.lab.command-generator.cmd.particle.desc',
		defaultMessage: 'Create particle effects',
	},
	'app.lab.command-generator.cmd.placefeature.desc': {
		id: 'app.lab.command-generator.cmd.placefeature.desc',
		defaultMessage: 'Place a configured feature (1.18.2)',
	},
	'app.lab.command-generator.cmd.playsound.desc': {
		id: 'app.lab.command-generator.cmd.playsound.desc',
		defaultMessage: 'Play a sound',
	},
	'app.lab.command-generator.cmd.posteffect.desc': {
		id: 'app.lab.command-generator.cmd.posteffect.desc',
		defaultMessage: 'Manage screen post-processing effects.',
	},
	'app.lab.command-generator.cmd.publish.desc': {
		id: 'app.lab.command-generator.cmd.publish.desc',
		defaultMessage: 'Open a local world to LAN',
	},
	'app.lab.command-generator.cmd.random.desc': {
		id: 'app.lab.command-generator.cmd.random.desc',
		defaultMessage: 'Draw random values (1.20.2+)',
	},
	'app.lab.command-generator.cmd.recipe.desc': {
		id: 'app.lab.command-generator.cmd.recipe.desc',
		defaultMessage: 'Give or take crafting recipes',
	},
	'app.lab.command-generator.cmd.reload.desc': {
		id: 'app.lab.command-generator.cmd.reload.desc',
		defaultMessage: 'Make an entity mount or dismount another entity (1.19.4+)',
	},
	'app.lab.command-generator.cmd.replaceitem.desc': {
		id: 'app.lab.command-generator.cmd.replaceitem.desc',
		defaultMessage: 'Broadcast a message to all players',
	},
	'app.lab.command-generator.cmd.ride.desc': {
		id: 'app.lab.command-generator.cmd.ride.desc',
		defaultMessage: 'Schedule a function',
	},
	'app.lab.command-generator.cmd.say.desc': {
		id: 'app.lab.command-generator.cmd.say.desc',
		defaultMessage: 'Manage the scoreboard',
	},
	'app.lab.command-generator.cmd.schedule.desc': {
		id: 'app.lab.command-generator.cmd.schedule.desc',
		defaultMessage: 'Show the world seed',
	},
	'app.lab.command-generator.cmd.scoreboard.desc': {
		id: 'app.lab.command-generator.cmd.scoreboard.desc',
		defaultMessage: 'Place a single block',
	},
	'app.lab.command-generator.cmd.seed.desc': {
		id: 'app.lab.command-generator.cmd.seed.desc',
		defaultMessage: 'Set the player idle timeout',
	},
	'app.lab.command-generator.cmd.setblock.desc': {
		id: 'app.lab.command-generator.cmd.setblock.desc',
		defaultMessage: 'Set the world spawn point',
	},
	'app.lab.command-generator.cmd.setidletimeout.desc': {
		id: 'app.lab.command-generator.cmd.setidletimeout.desc',
		defaultMessage: "Set a player's respawn point",
	},
	'app.lab.command-generator.cmd.setworldspawn.desc': {
		id: 'app.lab.command-generator.cmd.setworldspawn.desc',
		defaultMessage: 'Make a spectator watch an entity',
	},
	'app.lab.command-generator.cmd.spawnpoint.desc': {
		id: 'app.lab.command-generator.cmd.spawnpoint.desc',
		defaultMessage: 'Spread entities across an area',
	},
	'app.lab.command-generator.cmd.spectate.desc': {
		id: 'app.lab.command-generator.cmd.spectate.desc',
		defaultMessage: 'Stop the server',
	},
	'app.lab.command-generator.cmd.spreadplayers.desc': {
		id: 'app.lab.command-generator.cmd.spreadplayers.desc',
		defaultMessage: 'Stop sounds',
	},
	'app.lab.command-generator.cmd.stop.desc': {
		id: 'app.lab.command-generator.cmd.stop.desc',
		defaultMessage: 'Create or modify real-time stopwatches (1.21.11+)',
	},
	'app.lab.command-generator.cmd.stopsound.desc': {
		id: 'app.lab.command-generator.cmd.stopsound.desc',
		defaultMessage: 'Summon entities',
	},
	'app.lab.command-generator.cmd.stopwatch.desc': {
		id: 'app.lab.command-generator.cmd.stopwatch.desc',
		defaultMessage: "Swing an entity's hand (1.21.4+)",
	},
	'app.lab.command-generator.cmd.summon.desc': {
		id: 'app.lab.command-generator.cmd.summon.desc',
		defaultMessage: 'Manage entity tags',
	},
	'app.lab.command-generator.cmd.swing.desc': {
		id: 'app.lab.command-generator.cmd.swing.desc',
		defaultMessage: 'Manage scoreboard teams',
	},
	'app.lab.command-generator.cmd.tag.desc': {
		id: 'app.lab.command-generator.cmd.tag.desc',
		defaultMessage: 'Send a team message',
	},
	'app.lab.command-generator.cmd.team.desc': {
		id: 'app.lab.command-generator.cmd.team.desc',
		defaultMessage: 'Teleport entities',
	},
	'app.lab.command-generator.cmd.teammsg.desc': {
		id: 'app.lab.command-generator.cmd.teammsg.desc',
		defaultMessage: 'Send a private message',
	},
	'app.lab.command-generator.cmd.teleport.desc': {
		id: 'app.lab.command-generator.cmd.teleport.desc',
		defaultMessage: 'Send a raw JSON message to players',
	},
	'app.lab.command-generator.cmd.tell.desc': {
		id: 'app.lab.command-generator.cmd.tell.desc',
		defaultMessage: 'Send a private message',
	},
	'app.lab.command-generator.cmd.tellraw.desc': {
		id: 'app.lab.command-generator.cmd.tellraw.desc',
		defaultMessage: 'Control the game tick rate (1.20.3+)',
	},
	'app.lab.command-generator.cmd.tick.desc': {
		id: 'app.lab.command-generator.cmd.tick.desc',
		defaultMessage: 'Set or query the game time',
	},
	'app.lab.command-generator.cmd.time.desc': {
		id: 'app.lab.command-generator.cmd.time.desc',
		defaultMessage: 'Manage on-screen titles',
	},
	'app.lab.command-generator.cmd.title.desc': {
		id: 'app.lab.command-generator.cmd.title.desc',
		defaultMessage: 'Modify a scoreboard trigger',
	},
	'app.lab.command-generator.cmd.trigger.desc': {
		id: 'app.lab.command-generator.cmd.trigger.desc',
		defaultMessage: 'Stop sharing a single-player world.',
	},
	'app.lab.command-generator.cmd.unpublish.desc': {
		id: 'app.lab.command-generator.cmd.unpublish.desc',
		defaultMessage: 'Manage locator bar waypoints (1.21.6+)',
	},
	'app.lab.command-generator.cmd.waypoint.desc': {
		id: 'app.lab.command-generator.cmd.waypoint.desc',
		defaultMessage: 'Set the weather',
	},
	'app.lab.command-generator.cmd.weather.desc': {
		id: 'app.lab.command-generator.cmd.weather.desc',
		defaultMessage: 'Manage the server whitelist',
	},
	'app.lab.command-generator.cmd.whitelist.desc': {
		id: 'app.lab.command-generator.cmd.whitelist.desc',
		defaultMessage: 'Manage the world border',
	},
	'app.lab.command-generator.cmd.worldborder.desc': {
		id: 'app.lab.command-generator.cmd.worldborder.desc',
		defaultMessage: 'Add or set player experience',
	},
	'app.lab.command-generator.cmd.xp.desc': {
		id: 'app.lab.command-generator.cmd.xp.desc',
		defaultMessage: 'Add or set player experience',
	},
	'app.lab.command-generator.execute.align.desc': {
		id: 'app.lab.command-generator.execute.align.desc',
		defaultMessage: 'Align the execution position to the block grid',
	},
	'app.lab.command-generator.execute.align.label': {
		id: 'app.lab.command-generator.execute.align.label',
		defaultMessage: 'align · Align position',
	},
	'app.lab.command-generator.execute.anchored.desc': {
		id: 'app.lab.command-generator.execute.anchored.desc',
		defaultMessage: 'Change the execution anchor (feet or eyes) used for ^ coordinates',
	},
	'app.lab.command-generator.execute.anchored.label': {
		id: 'app.lab.command-generator.execute.anchored.label',
		defaultMessage: 'anchored · Change anchor',
	},
	'app.lab.command-generator.execute.as.desc': {
		id: 'app.lab.command-generator.execute.as.desc',
		defaultMessage: 'Change the executing entity while keeping the position and rotation',
	},
	'app.lab.command-generator.execute.as.label': {
		id: 'app.lab.command-generator.execute.as.label',
		defaultMessage: 'as · Change executor',
	},
	'app.lab.command-generator.execute.at.desc': {
		id: 'app.lab.command-generator.execute.at.desc',
		defaultMessage: "Use the target's position and rotation",
	},
	'app.lab.command-generator.execute.at.label': {
		id: 'app.lab.command-generator.execute.at.label',
		defaultMessage: 'at · Change position and rotation',
	},
	'app.lab.command-generator.execute.category.conditions': {
		id: 'app.lab.command-generator.execute.category.conditions',
		defaultMessage: 'Conditions · Check whether to continue',
	},
	'app.lab.command-generator.execute.category.modifiers': {
		id: 'app.lab.command-generator.execute.category.modifiers',
		defaultMessage: 'Modifiers · Change execution context',
	},
	'app.lab.command-generator.execute.category.store': {
		id: 'app.lab.command-generator.execute.category.store',
		defaultMessage: 'Store · Save results (Java Edition)',
	},
	'app.lab.command-generator.execute.facing-entity.desc': {
		id: 'app.lab.command-generator.execute.facing-entity.desc',
		defaultMessage: 'Face an entity',
	},
	'app.lab.command-generator.execute.facing-entity.label': {
		id: 'app.lab.command-generator.execute.facing-entity.label',
		defaultMessage: 'facing entity · Face entity',
	},
	'app.lab.command-generator.execute.facing.desc': {
		id: 'app.lab.command-generator.execute.facing.desc',
		defaultMessage: 'Face a position',
	},
	'app.lab.command-generator.execute.facing.label': {
		id: 'app.lab.command-generator.execute.facing.label',
		defaultMessage: 'facing · Face position',
	},
	'app.lab.command-generator.execute.if-biome.desc': {
		id: 'app.lab.command-generator.execute.if-biome.desc',
		defaultMessage: 'Continue if the biome at the position matches',
	},
	'app.lab.command-generator.execute.if-biome.label': {
		id: 'app.lab.command-generator.execute.if-biome.label',
		defaultMessage: 'if biome · Check biome',
	},
	'app.lab.command-generator.execute.if-block.desc': {
		id: 'app.lab.command-generator.execute.if-block.desc',
		defaultMessage: 'Continue if the block at the position matches',
	},
	'app.lab.command-generator.execute.if-block.label': {
		id: 'app.lab.command-generator.execute.if-block.label',
		defaultMessage: 'if block · Check block',
	},
	'app.lab.command-generator.execute.if-blocks.desc': {
		id: 'app.lab.command-generator.execute.if-blocks.desc',
		defaultMessage: 'Continue if the blocks in two regions match',
	},
	'app.lab.command-generator.execute.if-blocks.label': {
		id: 'app.lab.command-generator.execute.if-blocks.label',
		defaultMessage: 'if blocks · Compare regions',
	},
	'app.lab.command-generator.execute.if-data.desc': {
		id: 'app.lab.command-generator.execute.if-data.desc',
		defaultMessage: 'Continue if the NBT path exists',
	},
	'app.lab.command-generator.execute.if-data.label': {
		id: 'app.lab.command-generator.execute.if-data.label',
		defaultMessage: 'if data · Check NBT data',
	},
	'app.lab.command-generator.execute.if-entity.desc': {
		id: 'app.lab.command-generator.execute.if-entity.desc',
		defaultMessage: 'Continue if an entity exists',
	},
	'app.lab.command-generator.execute.if-entity.label': {
		id: 'app.lab.command-generator.execute.if-entity.label',
		defaultMessage: 'if entity · Check entity',
	},
	'app.lab.command-generator.execute.if-function.desc': {
		id: 'app.lab.command-generator.execute.if-function.desc',
		defaultMessage: 'Continue if a function succeeds',
	},
	'app.lab.command-generator.execute.if-function.label': {
		id: 'app.lab.command-generator.execute.if-function.label',
		defaultMessage: 'if function · Check function result',
	},
	'app.lab.command-generator.execute.if-items.desc': {
		id: 'app.lab.command-generator.execute.if-items.desc',
		defaultMessage: 'Continue if the specified slots contain matching items',
	},
	'app.lab.command-generator.execute.if-items.label': {
		id: 'app.lab.command-generator.execute.if-items.label',
		defaultMessage: 'if items · Check items (1.21+)',
	},
	'app.lab.command-generator.execute.if-score-matches.desc': {
		id: 'app.lab.command-generator.execute.if-score-matches.desc',
		defaultMessage: 'Continue if the score is within the range',
	},
	'app.lab.command-generator.execute.if-score-matches.label': {
		id: 'app.lab.command-generator.execute.if-score-matches.label',
		defaultMessage: 'if score matches · Check score range',
	},
	'app.lab.command-generator.execute.if-score.desc': {
		id: 'app.lab.command-generator.execute.if-score.desc',
		defaultMessage: 'Continue if the score comparison is true',
	},
	'app.lab.command-generator.execute.if-score.label': {
		id: 'app.lab.command-generator.execute.if-score.label',
		defaultMessage: 'if score · Compare scores',
	},
	'app.lab.command-generator.execute.if-slots.desc': {
		id: 'app.lab.command-generator.execute.if-slots.desc',
		defaultMessage: 'Check block or entity slots using a slot source.',
	},
	'app.lab.command-generator.execute.if-slots.label': {
		id: 'app.lab.command-generator.execute.if-slots.label',
		defaultMessage: 'if slots · Check slots',
	},
	'app.lab.command-generator.execute.in.desc': {
		id: 'app.lab.command-generator.execute.in.desc',
		defaultMessage: 'Change the execution dimension while keeping the position coordinates',
	},
	'app.lab.command-generator.execute.in.label': {
		id: 'app.lab.command-generator.execute.in.label',
		defaultMessage: 'in · Change dimension',
	},
	'app.lab.command-generator.execute.positioned-as.desc': {
		id: 'app.lab.command-generator.execute.positioned-as.desc',
		defaultMessage: "Use the target's current position",
	},
	'app.lab.command-generator.execute.positioned-as.label': {
		id: 'app.lab.command-generator.execute.positioned-as.label',
		defaultMessage: 'positioned as · Use entity position',
	},
	'app.lab.command-generator.execute.positioned.desc': {
		id: 'app.lab.command-generator.execute.positioned.desc',
		defaultMessage: 'Change the execution position while keeping the executor and rotation',
	},
	'app.lab.command-generator.execute.positioned.label': {
		id: 'app.lab.command-generator.execute.positioned.label',
		defaultMessage: 'positioned · Change position',
	},
	'app.lab.command-generator.execute.rotated-as.desc': {
		id: 'app.lab.command-generator.execute.rotated-as.desc',
		defaultMessage: "Use the target's current rotation",
	},
	'app.lab.command-generator.execute.rotated-as.label': {
		id: 'app.lab.command-generator.execute.rotated-as.label',
		defaultMessage: 'rotated as · Use entity rotation',
	},
	'app.lab.command-generator.execute.rotated.desc': {
		id: 'app.lab.command-generator.execute.rotated.desc',
		defaultMessage: 'Change the execution rotation while keeping the executor and position',
	},
	'app.lab.command-generator.execute.rotated.label': {
		id: 'app.lab.command-generator.execute.rotated.label',
		defaultMessage: 'rotated · Change rotation',
	},
	'app.lab.command-generator.execute.store-result-bossbar.desc': {
		id: 'app.lab.command-generator.execute.store-result-bossbar.desc',
		defaultMessage: "Store the result in a boss bar's current or maximum value",
	},
	'app.lab.command-generator.execute.store-result-bossbar.label': {
		id: 'app.lab.command-generator.execute.store-result-bossbar.label',
		defaultMessage: 'store result bossbar · Store in boss bar',
	},
	'app.lab.command-generator.execute.store-result-score.desc': {
		id: 'app.lab.command-generator.execute.store-result-score.desc',
		defaultMessage: "Store the command's result in a scoreboard objective",
	},
	'app.lab.command-generator.execute.store-result-score.label': {
		id: 'app.lab.command-generator.execute.store-result-score.label',
		defaultMessage: 'store result score · Store in scoreboard',
	},
	'app.lab.command-generator.execute.store-success-score.desc': {
		id: 'app.lab.command-generator.execute.store-success-score.desc',
		defaultMessage: 'Store command success (1 or 0) in a scoreboard objective',
	},
	'app.lab.command-generator.execute.store-success-score.label': {
		id: 'app.lab.command-generator.execute.store-success-score.label',
		defaultMessage: 'store success score · Store success',
	},
	'app.lab.command-generator.execute.unless-block.desc': {
		id: 'app.lab.command-generator.execute.unless-block.desc',
		defaultMessage: 'Continue if the block at the position does not match',
	},
	'app.lab.command-generator.execute.unless-block.label': {
		id: 'app.lab.command-generator.execute.unless-block.label',
		defaultMessage: 'unless block · Check different block',
	},
	'app.lab.command-generator.execute.unless-entity.desc': {
		id: 'app.lab.command-generator.execute.unless-entity.desc',
		defaultMessage: 'Continue if no matching entity exists',
	},
	'app.lab.command-generator.execute.unless-entity.label': {
		id: 'app.lab.command-generator.execute.unless-entity.label',
		defaultMessage: 'unless entity · Check entity absence',
	},
	'app.lab.command-generator.execute.unless-score.desc': {
		id: 'app.lab.command-generator.execute.unless-score.desc',
		defaultMessage: 'Continue if the score comparison is false',
	},
	'app.lab.command-generator.execute.unless-score.label': {
		id: 'app.lab.command-generator.execute.unless-score.label',
		defaultMessage: 'unless score · Compare scores inversely',
	},
	'app.lab.command-generator.execute.unless-slots.desc': {
		id: 'app.lab.command-generator.execute.unless-slots.desc',
		defaultMessage: 'Check block or entity slots using a slot source.',
	},
	'app.lab.command-generator.execute.unless-slots.label': {
		id: 'app.lab.command-generator.execute.unless-slots.label',
		defaultMessage: 'unless slots · Check slots',
	},
	'app.lab.command-generator.guide.effect.ex1': {
		id: 'app.lab.command-generator.guide.effect.ex1',
		defaultMessage: 'Give yourself Speed III for 30 seconds',
	},
	'app.lab.command-generator.guide.effect.ex2': {
		id: 'app.lab.command-generator.guide.effect.ex2',
		defaultMessage: 'Remove Poison from all players',
	},
	'app.lab.command-generator.guide.effect.summary': {
		id: 'app.lab.command-generator.guide.effect.summary',
		defaultMessage: 'Apply or remove status effects.',
	},
	'app.lab.command-generator.guide.execute.ex1': {
		id: 'app.lab.command-generator.guide.execute.ex1',
		defaultMessage: 'Have each player say hello at their own position',
	},
	'app.lab.command-generator.guide.execute.ex2': {
		id: 'app.lab.command-generator.guide.execute.ex2',
		defaultMessage: 'Broadcast a message if a zombie exists',
	},
	'app.lab.command-generator.guide.execute.summary': {
		id: 'app.lab.command-generator.guide.execute.summary',
		defaultMessage: 'Run commands with a specific context, conditions, and entity selectors.',
	},
	'app.lab.command-generator.guide.execute.tip1': {
		id: 'app.lab.command-generator.guide.execute.tip1',
		defaultMessage: 'End the subcommand chain with run followed by a command',
	},
	'app.lab.command-generator.guide.execute.tip2': {
		id: 'app.lab.command-generator.guide.execute.tip2',
		defaultMessage: 'Common subcommands: as / at / positioned / if / unless',
	},
	'app.lab.command-generator.guide.fill.ex1': {
		id: 'app.lab.command-generator.guide.fill.ex1',
		defaultMessage: 'Fill an 11×1×11 region with stone',
	},
	'app.lab.command-generator.guide.fill.ex2': {
		id: 'app.lab.command-generator.guide.fill.ex2',
		defaultMessage: 'Create a hollow glass shell',
	},
	'app.lab.command-generator.guide.fill.summary': {
		id: 'app.lab.command-generator.guide.fill.summary',
		defaultMessage: 'Fill a region with a block.',
	},
	'app.lab.command-generator.guide.fill.tip1': {
		id: 'app.lab.command-generator.guide.fill.tip1',
		defaultMessage: 'Modes: replace / keep / outline / hollow / destroy',
	},
	'app.lab.command-generator.guide.gamemode.ex1': {
		id: 'app.lab.command-generator.guide.gamemode.ex1',
		defaultMessage: 'Switch all players to Creative mode',
	},
	'app.lab.command-generator.guide.gamemode.ex2': {
		id: 'app.lab.command-generator.guide.gamemode.ex2',
		defaultMessage: 'Switch yourself to Survival mode',
	},
	'app.lab.command-generator.guide.gamemode.summary': {
		id: 'app.lab.command-generator.guide.gamemode.summary',
		defaultMessage: "Change a player's game mode.",
	},
	'app.lab.command-generator.guide.give.ex1': {
		id: 'app.lab.command-generator.guide.give.ex1',
		defaultMessage: 'Give yourself 64 diamonds',
	},
	'app.lab.command-generator.guide.give.ex2': {
		id: 'app.lab.command-generator.guide.give.ex2',
		defaultMessage: 'Give the nearest player a diamond sword with Sharpness V',
	},
	'app.lab.command-generator.guide.give.summary': {
		id: 'app.lab.command-generator.guide.give.summary',
		defaultMessage: 'Give items to players, with optional custom NBT data.',
	},
	'app.lab.command-generator.guide.give.tip1': {
		id: 'app.lab.command-generator.guide.give.tip1',
		defaultMessage: "Item stacks split according to the item's maximum stack size",
	},
	'app.lab.command-generator.guide.give.tip2': {
		id: 'app.lab.command-generator.guide.give.tip2',
		defaultMessage: 'Wrap NBT data in braces',
	},
	'app.lab.command-generator.guide.setblock.ex1': {
		id: 'app.lab.command-generator.guide.setblock.ex1',
		defaultMessage: 'Place a chest above your head',
	},
	'app.lab.command-generator.guide.setblock.summary': {
		id: 'app.lab.command-generator.guide.setblock.summary',
		defaultMessage: 'Place a single block.',
	},
	'app.lab.command-generator.guide.summon.ex1': {
		id: 'app.lab.command-generator.guide.summon.ex1',
		defaultMessage: 'Summon an axolotl at the current position',
	},
	'app.lab.command-generator.guide.summon.ex2': {
		id: 'app.lab.command-generator.guide.summon.ex2',
		defaultMessage: 'Summon a creeper with an explosion radius of 10',
	},
	'app.lab.command-generator.guide.summon.summary': {
		id: 'app.lab.command-generator.guide.summon.summary',
		defaultMessage: 'Summon an entity at a position.',
	},
	'app.lab.command-generator.guide.summon.tip1': {
		id: 'app.lab.command-generator.guide.summon.tip1',
		defaultMessage: 'Include the minecraft: namespace in entity IDs',
	},
	'app.lab.command-generator.guide.time.ex1': {
		id: 'app.lab.command-generator.guide.time.ex1',
		defaultMessage: 'Set the time to day (1000 ticks)',
	},
	'app.lab.command-generator.guide.time.ex2': {
		id: 'app.lab.command-generator.guide.time.ex2',
		defaultMessage: 'Advance the time by 1000 ticks',
	},
	'app.lab.command-generator.guide.time.summary': {
		id: 'app.lab.command-generator.guide.time.summary',
		defaultMessage: 'Set or query the game time.',
	},
	'app.lab.command-generator.guide.time.tip1': {
		id: 'app.lab.command-generator.guide.time.tip1',
		defaultMessage: '0 = sunrise, 6000 = noon, 12000 = sunset, 18000 = midnight',
	},
	'app.lab.command-generator.guide.tp.ex1': {
		id: 'app.lab.command-generator.guide.tp.ex1',
		defaultMessage: 'Teleport yourself to (100, 64, 200)',
	},
	'app.lab.command-generator.guide.tp.ex2': {
		id: 'app.lab.command-generator.guide.tp.ex2',
		defaultMessage: 'Teleport the nearest player to yourself',
	},
	'app.lab.command-generator.guide.tp.summary': {
		id: 'app.lab.command-generator.guide.tp.summary',
		defaultMessage: 'Teleport entities to a position.',
	},
	'app.lab.command-generator.guide.weather.ex1': {
		id: 'app.lab.command-generator.guide.weather.ex1',
		defaultMessage: 'Set clear weather',
	},
	'app.lab.command-generator.guide.weather.ex2': {
		id: 'app.lab.command-generator.guide.weather.ex2',
		defaultMessage: 'Set rain for 10 minutes',
	},
	'app.lab.command-generator.guide.weather.summary': {
		id: 'app.lab.command-generator.guide.weather.summary',
		defaultMessage: 'Set the weather.',
	},
	'app.lab.command-generator.perm.0': {
		id: 'app.lab.command-generator.perm.0',
		defaultMessage: 'Everyone',
	},
	'app.lab.command-generator.perm.0.desc': {
		id: 'app.lab.command-generator.perm.0.desc',
		defaultMessage: 'No special permission required',
	},
	'app.lab.command-generator.perm.1': {
		id: 'app.lab.command-generator.perm.1',
		defaultMessage: 'Moderator',
	},
	'app.lab.command-generator.perm.1.desc': {
		id: 'app.lab.command-generator.perm.1.desc',
		defaultMessage: 'Can bypass spawn protection',
	},
	'app.lab.command-generator.perm.2': {
		id: 'app.lab.command-generator.perm.2',
		defaultMessage: 'Game master',
	},
	'app.lab.command-generator.perm.2.desc': {
		id: 'app.lab.command-generator.perm.2.desc',
		defaultMessage: 'Most gameplay commands, command blocks, and functions',
	},
	'app.lab.command-generator.perm.3': {
		id: 'app.lab.command-generator.perm.3',
		defaultMessage: 'Administrator',
	},
	'app.lab.command-generator.perm.3.desc': {
		id: 'app.lab.command-generator.perm.3.desc',
		defaultMessage: 'Multiplayer management commands',
	},
	'app.lab.command-generator.perm.4': {
		id: 'app.lab.command-generator.perm.4',
		defaultMessage: 'Owner',
	},
	'app.lab.command-generator.perm.4.desc': {
		id: 'app.lab.command-generator.perm.4.desc',
		defaultMessage: 'Server management commands',
	},
	'app.lab.command-generator.preset.action.add': {
		id: 'app.lab.command-generator.preset.action.add',
		defaultMessage: 'Add',
	},
	'app.lab.command-generator.preset.action.set': {
		id: 'app.lab.command-generator.preset.action.set',
		defaultMessage: 'Set',
	},
	'app.lab.command-generator.preset.advancement.action.0': {
		id: 'app.lab.command-generator.preset.advancement.action.0',
		defaultMessage: 'Grant',
	},
	'app.lab.command-generator.preset.advancement.action.1': {
		id: 'app.lab.command-generator.preset.advancement.action.1',
		defaultMessage: 'Remove',
	},
	'app.lab.command-generator.preset.advancement.breed-animals': {
		id: 'app.lab.command-generator.preset.advancement.breed-animals',
		defaultMessage: 'The Parrots and the Bats',
	},
	'app.lab.command-generator.preset.advancement.diamonds': {
		id: 'app.lab.command-generator.preset.advancement.diamonds',
		defaultMessage: 'Diamonds!',
	},
	'app.lab.command-generator.preset.advancement.elytra': {
		id: 'app.lab.command-generator.preset.advancement.elytra',
		defaultMessage: "Sky's the Limit",
	},
	'app.lab.command-generator.preset.advancement.enter-nether': {
		id: 'app.lab.command-generator.preset.advancement.enter-nether',
		defaultMessage: 'We Need to Go Deeper',
	},
	'app.lab.command-generator.preset.advancement.hero-village': {
		id: 'app.lab.command-generator.preset.advancement.hero-village',
		defaultMessage: 'Hero of the Village',
	},
	'app.lab.command-generator.preset.advancement.kill-dragon': {
		id: 'app.lab.command-generator.preset.advancement.kill-dragon',
		defaultMessage: 'Free the End',
	},
	'app.lab.command-generator.preset.advancement.root': {
		id: 'app.lab.command-generator.preset.advancement.root',
		defaultMessage: 'Minecraft (root advancement)',
	},
	'app.lab.command-generator.preset.advancement.stone-age': {
		id: 'app.lab.command-generator.preset.advancement.stone-age',
		defaultMessage: 'Stone Age',
	},
	'app.lab.command-generator.preset.align.x': {
		id: 'app.lab.command-generator.preset.align.x',
		defaultMessage: 'X only',
	},
	'app.lab.command-generator.preset.align.xyz': {
		id: 'app.lab.command-generator.preset.align.xyz',
		defaultMessage: 'All',
	},
	'app.lab.command-generator.preset.align.y': {
		id: 'app.lab.command-generator.preset.align.y',
		defaultMessage: 'Y only',
	},
	'app.lab.command-generator.preset.align.z': {
		id: 'app.lab.command-generator.preset.align.z',
		defaultMessage: 'Z only',
	},
	'app.lab.command-generator.preset.amount.10': {
		id: 'app.lab.command-generator.preset.amount.10',
		defaultMessage: '10',
	},
	'app.lab.command-generator.preset.amount.100': {
		id: 'app.lab.command-generator.preset.amount.100',
		defaultMessage: '100',
	},
	'app.lab.command-generator.preset.amount.30': {
		id: 'app.lab.command-generator.preset.amount.30',
		defaultMessage: '30',
	},
	'app.lab.command-generator.preset.anchor.eyes': {
		id: 'app.lab.command-generator.preset.anchor.eyes',
		defaultMessage: 'Eyes',
	},
	'app.lab.command-generator.preset.anchor.feet': {
		id: 'app.lab.command-generator.preset.anchor.feet',
		defaultMessage: 'Feet',
	},
	'app.lab.command-generator.preset.attribute.action.base': {
		id: 'app.lab.command-generator.preset.attribute.action.base',
		defaultMessage: 'Base value',
	},
	'app.lab.command-generator.preset.attribute.action.get': {
		id: 'app.lab.command-generator.preset.attribute.action.get',
		defaultMessage: 'Query',
	},
	'app.lab.command-generator.preset.attribute.action.value': {
		id: 'app.lab.command-generator.preset.attribute.action.value',
		defaultMessage: 'Set value',
	},
	'app.lab.command-generator.preset.attribute.attack-damage': {
		id: 'app.lab.command-generator.preset.attribute.attack-damage',
		defaultMessage: 'Attack damage',
	},
	'app.lab.command-generator.preset.attribute.max-health': {
		id: 'app.lab.command-generator.preset.attribute.max-health',
		defaultMessage: 'Max health',
	},
	'app.lab.command-generator.preset.attribute.movement-speed': {
		id: 'app.lab.command-generator.preset.attribute.movement-speed',
		defaultMessage: 'Movement speed',
	},
	'app.lab.command-generator.preset.banlist.type.0': {
		id: 'app.lab.command-generator.preset.banlist.type.0',
		defaultMessage: 'Players',
	},
	'app.lab.command-generator.preset.banlist.type.1': {
		id: 'app.lab.command-generator.preset.banlist.type.1',
		defaultMessage: 'IP addresses',
	},
	'app.lab.command-generator.preset.biome.desert': {
		id: 'app.lab.command-generator.preset.biome.desert',
		defaultMessage: 'Desert',
	},
	'app.lab.command-generator.preset.biome.forest': {
		id: 'app.lab.command-generator.preset.biome.forest',
		defaultMessage: 'Forest',
	},
	'app.lab.command-generator.preset.biome.jungle': {
		id: 'app.lab.command-generator.preset.biome.jungle',
		defaultMessage: 'Jungle',
	},
	'app.lab.command-generator.preset.biome.ocean': {
		id: 'app.lab.command-generator.preset.biome.ocean',
		defaultMessage: 'Ocean',
	},
	'app.lab.command-generator.preset.biome.plains': {
		id: 'app.lab.command-generator.preset.biome.plains',
		defaultMessage: 'Plains',
	},
	'app.lab.command-generator.preset.biome.snowy-plains': {
		id: 'app.lab.command-generator.preset.biome.snowy-plains',
		defaultMessage: 'Snowy plains',
	},
	'app.lab.command-generator.preset.block.air': {
		id: 'app.lab.command-generator.preset.block.air',
		defaultMessage: 'Air',
	},
	'app.lab.command-generator.preset.block.chest': {
		id: 'app.lab.command-generator.preset.block.chest',
		defaultMessage: 'Chest',
	},
	'app.lab.command-generator.preset.block.diamond-block': {
		id: 'app.lab.command-generator.preset.block.diamond-block',
		defaultMessage: 'Block of Diamond',
	},
	'app.lab.command-generator.preset.block.glass': {
		id: 'app.lab.command-generator.preset.block.glass',
		defaultMessage: 'Glass',
	},
	'app.lab.command-generator.preset.block.grass': {
		id: 'app.lab.command-generator.preset.block.grass',
		defaultMessage: 'Grass Block',
	},
	'app.lab.command-generator.preset.block.stone': {
		id: 'app.lab.command-generator.preset.block.stone',
		defaultMessage: 'Stone',
	},
	'app.lab.command-generator.preset.blocks-mode.all': {
		id: 'app.lab.command-generator.preset.blocks-mode.all',
		defaultMessage: 'Match all',
	},
	'app.lab.command-generator.preset.blocks-mode.masked': {
		id: 'app.lab.command-generator.preset.blocks-mode.masked',
		defaultMessage: 'Ignore air',
	},
	'app.lab.command-generator.preset.boolean.false': {
		id: 'app.lab.command-generator.preset.boolean.false',
		defaultMessage: 'false',
	},
	'app.lab.command-generator.preset.boolean.true': {
		id: 'app.lab.command-generator.preset.boolean.true',
		defaultMessage: 'true',
	},
	'app.lab.command-generator.preset.bossbar-value.max': {
		id: 'app.lab.command-generator.preset.bossbar-value.max',
		defaultMessage: 'Maximum',
	},
	'app.lab.command-generator.preset.bossbar-value.value': {
		id: 'app.lab.command-generator.preset.bossbar-value.value',
		defaultMessage: 'Current value',
	},
	'app.lab.command-generator.preset.color.blue': {
		id: 'app.lab.command-generator.preset.color.blue',
		defaultMessage: 'Blue',
	},
	'app.lab.command-generator.preset.color.gold': {
		id: 'app.lab.command-generator.preset.color.gold',
		defaultMessage: 'Gold',
	},
	'app.lab.command-generator.preset.color.green': {
		id: 'app.lab.command-generator.preset.color.green',
		defaultMessage: 'Green',
	},
	'app.lab.command-generator.preset.color.red': {
		id: 'app.lab.command-generator.preset.color.red',
		defaultMessage: 'Red',
	},
	'app.lab.command-generator.preset.color.white': {
		id: 'app.lab.command-generator.preset.color.white',
		defaultMessage: 'White',
	},
	'app.lab.command-generator.preset.common.all-entities': {
		id: 'app.lab.command-generator.preset.common.all-entities',
		defaultMessage: 'All entities',
	},
	'app.lab.command-generator.preset.common.all-players': {
		id: 'app.lab.command-generator.preset.common.all-players',
		defaultMessage: 'All players',
	},
	'app.lab.command-generator.preset.common.nearest-player': {
		id: 'app.lab.command-generator.preset.common.nearest-player',
		defaultMessage: 'Nearest player',
	},
	'app.lab.command-generator.preset.common.self': {
		id: 'app.lab.command-generator.preset.common.self',
		defaultMessage: 'Self',
	},
	'app.lab.command-generator.preset.coord.below': {
		id: 'app.lab.command-generator.preset.coord.below',
		defaultMessage: 'Block below',
	},
	'app.lab.command-generator.preset.coord.current': {
		id: 'app.lab.command-generator.preset.coord.current',
		defaultMessage: 'Current position',
	},
	'app.lab.command-generator.preset.coord.world-center': {
		id: 'app.lab.command-generator.preset.coord.world-center',
		defaultMessage: 'World center',
	},
	'app.lab.command-generator.preset.count.1': {
		id: 'app.lab.command-generator.preset.count.1',
		defaultMessage: '1',
	},
	'app.lab.command-generator.preset.count.10': {
		id: 'app.lab.command-generator.preset.count.10',
		defaultMessage: '10',
	},
	'app.lab.command-generator.preset.count.100': {
		id: 'app.lab.command-generator.preset.count.100',
		defaultMessage: '100',
	},
	'app.lab.command-generator.preset.count.16': {
		id: 'app.lab.command-generator.preset.count.16',
		defaultMessage: '16',
	},
	'app.lab.command-generator.preset.count.64': {
		id: 'app.lab.command-generator.preset.count.64',
		defaultMessage: '64',
	},
	'app.lab.command-generator.preset.damage-amount.1': {
		id: 'app.lab.command-generator.preset.damage-amount.1',
		defaultMessage: '1',
	},
	'app.lab.command-generator.preset.damage-amount.10': {
		id: 'app.lab.command-generator.preset.damage-amount.10',
		defaultMessage: '10',
	},
	'app.lab.command-generator.preset.damage-amount.5': {
		id: 'app.lab.command-generator.preset.damage-amount.5',
		defaultMessage: '5',
	},
	'app.lab.command-generator.preset.damage-amount.fatal': {
		id: 'app.lab.command-generator.preset.damage-amount.fatal',
		defaultMessage: 'Lethal',
	},
	'app.lab.command-generator.preset.damage-type.drowning': {
		id: 'app.lab.command-generator.preset.damage-type.drowning',
		defaultMessage: 'Drowning',
	},
	'app.lab.command-generator.preset.damage-type.fall': {
		id: 'app.lab.command-generator.preset.damage-type.fall',
		defaultMessage: 'Fall',
	},
	'app.lab.command-generator.preset.damage-type.fire': {
		id: 'app.lab.command-generator.preset.damage-type.fire',
		defaultMessage: 'Fire',
	},
	'app.lab.command-generator.preset.damage-type.generic': {
		id: 'app.lab.command-generator.preset.damage-type.generic',
		defaultMessage: 'Generic',
	},
	'app.lab.command-generator.preset.damage-type.void': {
		id: 'app.lab.command-generator.preset.damage-type.void',
		defaultMessage: 'Void',
	},
	'app.lab.command-generator.preset.data-target.block': {
		id: 'app.lab.command-generator.preset.data-target.block',
		defaultMessage: 'Blocks',
	},
	'app.lab.command-generator.preset.data-target.entity': {
		id: 'app.lab.command-generator.preset.data-target.entity',
		defaultMessage: 'Entities',
	},
	'app.lab.command-generator.preset.data-target.storage': {
		id: 'app.lab.command-generator.preset.data-target.storage',
		defaultMessage: 'Storage',
	},
	'app.lab.command-generator.preset.difficulty.easy': {
		id: 'app.lab.command-generator.preset.difficulty.easy',
		defaultMessage: 'Easy',
	},
	'app.lab.command-generator.preset.difficulty.hard': {
		id: 'app.lab.command-generator.preset.difficulty.hard',
		defaultMessage: 'Hard',
	},
	'app.lab.command-generator.preset.difficulty.normal': {
		id: 'app.lab.command-generator.preset.difficulty.normal',
		defaultMessage: 'Normal',
	},
	'app.lab.command-generator.preset.difficulty.peaceful': {
		id: 'app.lab.command-generator.preset.difficulty.peaceful',
		defaultMessage: 'Peaceful',
	},
	'app.lab.command-generator.preset.dimension.end': {
		id: 'app.lab.command-generator.preset.dimension.end',
		defaultMessage: 'The End',
	},
	'app.lab.command-generator.preset.dimension.nether': {
		id: 'app.lab.command-generator.preset.dimension.nether',
		defaultMessage: 'The Nether',
	},
	'app.lab.command-generator.preset.dimension.overworld': {
		id: 'app.lab.command-generator.preset.dimension.overworld',
		defaultMessage: 'Overworld',
	},
	'app.lab.command-generator.preset.distance.10': {
		id: 'app.lab.command-generator.preset.distance.10',
		defaultMessage: '10',
	},
	'app.lab.command-generator.preset.distance.100': {
		id: 'app.lab.command-generator.preset.distance.100',
		defaultMessage: '100',
	},
	'app.lab.command-generator.preset.distance.1000': {
		id: 'app.lab.command-generator.preset.distance.1000',
		defaultMessage: '1000',
	},
	'app.lab.command-generator.preset.distance.50': {
		id: 'app.lab.command-generator.preset.distance.50',
		defaultMessage: '50',
	},
	'app.lab.command-generator.preset.distance.500': {
		id: 'app.lab.command-generator.preset.distance.500',
		defaultMessage: '500',
	},
	'app.lab.command-generator.preset.duration.10m': {
		id: 'app.lab.command-generator.preset.duration.10m',
		defaultMessage: '10 minutes',
	},
	'app.lab.command-generator.preset.duration.10s': {
		id: 'app.lab.command-generator.preset.duration.10s',
		defaultMessage: '10 seconds',
	},
	'app.lab.command-generator.preset.duration.1m': {
		id: 'app.lab.command-generator.preset.duration.1m',
		defaultMessage: '1 minute',
	},
	'app.lab.command-generator.preset.duration.30m': {
		id: 'app.lab.command-generator.preset.duration.30m',
		defaultMessage: '30 minutes',
	},
	'app.lab.command-generator.preset.duration.30s': {
		id: 'app.lab.command-generator.preset.duration.30s',
		defaultMessage: '30 seconds',
	},
	'app.lab.command-generator.preset.duration.5m': {
		id: 'app.lab.command-generator.preset.duration.5m',
		defaultMessage: '5 minutes',
	},
	'app.lab.command-generator.preset.effect.action.0': {
		id: 'app.lab.command-generator.preset.effect.action.0',
		defaultMessage: 'Grant',
	},
	'app.lab.command-generator.preset.effect.action.1': {
		id: 'app.lab.command-generator.preset.effect.action.1',
		defaultMessage: 'Remove',
	},
	'app.lab.command-generator.preset.effect.jump-boost': {
		id: 'app.lab.command-generator.preset.effect.jump-boost',
		defaultMessage: 'Jump Boost',
	},
	'app.lab.command-generator.preset.effect.night-vision': {
		id: 'app.lab.command-generator.preset.effect.night-vision',
		defaultMessage: 'Night Vision',
	},
	'app.lab.command-generator.preset.effect.poison': {
		id: 'app.lab.command-generator.preset.effect.poison',
		defaultMessage: 'Poison',
	},
	'app.lab.command-generator.preset.effect.regeneration': {
		id: 'app.lab.command-generator.preset.effect.regeneration',
		defaultMessage: 'Regeneration',
	},
	'app.lab.command-generator.preset.effect.speed': {
		id: 'app.lab.command-generator.preset.effect.speed',
		defaultMessage: 'Speed',
	},
	'app.lab.command-generator.preset.effect.strength': {
		id: 'app.lab.command-generator.preset.effect.strength',
		defaultMessage: 'Strength',
	},
	'app.lab.command-generator.preset.enchantment.fortune': {
		id: 'app.lab.command-generator.preset.enchantment.fortune',
		defaultMessage: 'Fortune',
	},
	'app.lab.command-generator.preset.enchantment.mending': {
		id: 'app.lab.command-generator.preset.enchantment.mending',
		defaultMessage: 'Mending',
	},
	'app.lab.command-generator.preset.enchantment.protection': {
		id: 'app.lab.command-generator.preset.enchantment.protection',
		defaultMessage: 'Protection',
	},
	'app.lab.command-generator.preset.enchantment.sharpness': {
		id: 'app.lab.command-generator.preset.enchantment.sharpness',
		defaultMessage: 'Sharpness',
	},
	'app.lab.command-generator.preset.enchantment.unbreaking': {
		id: 'app.lab.command-generator.preset.enchantment.unbreaking',
		defaultMessage: 'Unbreaking',
	},
	'app.lab.command-generator.preset.entity-type.axolotl': {
		id: 'app.lab.command-generator.preset.entity-type.axolotl',
		defaultMessage: 'Axolotl',
	},
	'app.lab.command-generator.preset.entity-type.boat': {
		id: 'app.lab.command-generator.preset.entity-type.boat',
		defaultMessage: 'Boat',
	},
	'app.lab.command-generator.preset.entity-type.creeper': {
		id: 'app.lab.command-generator.preset.entity-type.creeper',
		defaultMessage: 'Creeper',
	},
	'app.lab.command-generator.preset.entity-type.ender-dragon': {
		id: 'app.lab.command-generator.preset.entity-type.ender-dragon',
		defaultMessage: 'Ender Dragon',
	},
	'app.lab.command-generator.preset.entity-type.horse': {
		id: 'app.lab.command-generator.preset.entity-type.horse',
		defaultMessage: 'Horse',
	},
	'app.lab.command-generator.preset.entity-type.lightning': {
		id: 'app.lab.command-generator.preset.entity-type.lightning',
		defaultMessage: 'Lightning Bolt',
	},
	'app.lab.command-generator.preset.entity-type.minecart': {
		id: 'app.lab.command-generator.preset.entity-type.minecart',
		defaultMessage: 'Minecart',
	},
	'app.lab.command-generator.preset.entity-type.pig': {
		id: 'app.lab.command-generator.preset.entity-type.pig',
		defaultMessage: 'Pig',
	},
	'app.lab.command-generator.preset.entity-type.wither': {
		id: 'app.lab.command-generator.preset.entity-type.wither',
		defaultMessage: 'Wither',
	},
	'app.lab.command-generator.preset.entity-type.zombie': {
		id: 'app.lab.command-generator.preset.entity-type.zombie',
		defaultMessage: 'Zombie',
	},
	'app.lab.command-generator.preset.entity.all-items': {
		id: 'app.lab.command-generator.preset.entity.all-items',
		defaultMessage: 'All item entities',
	},
	'app.lab.command-generator.preset.entity.all-zombies': {
		id: 'app.lab.command-generator.preset.entity.all-zombies',
		defaultMessage: 'All zombies',
	},
	'app.lab.command-generator.preset.feature.birch': {
		id: 'app.lab.command-generator.preset.feature.birch',
		defaultMessage: 'Birch',
	},
	'app.lab.command-generator.preset.feature.flower': {
		id: 'app.lab.command-generator.preset.feature.flower',
		defaultMessage: 'Flower',
	},
	'app.lab.command-generator.preset.feature.grass': {
		id: 'app.lab.command-generator.preset.feature.grass',
		defaultMessage: 'Grass',
	},
	'app.lab.command-generator.preset.feature.oak': {
		id: 'app.lab.command-generator.preset.feature.oak',
		defaultMessage: 'Oak',
	},
	'app.lab.command-generator.preset.feature.spruce': {
		id: 'app.lab.command-generator.preset.feature.spruce',
		defaultMessage: 'Spruce',
	},
	'app.lab.command-generator.preset.fill-mode.destroy': {
		id: 'app.lab.command-generator.preset.fill-mode.destroy',
		defaultMessage: 'Destroy',
	},
	'app.lab.command-generator.preset.fill-mode.hollow': {
		id: 'app.lab.command-generator.preset.fill-mode.hollow',
		defaultMessage: 'Hollow',
	},
	'app.lab.command-generator.preset.fill-mode.keep': {
		id: 'app.lab.command-generator.preset.fill-mode.keep',
		defaultMessage: 'Keep',
	},
	'app.lab.command-generator.preset.fill-mode.outline': {
		id: 'app.lab.command-generator.preset.fill-mode.outline',
		defaultMessage: 'Outline',
	},
	'app.lab.command-generator.preset.fill-mode.replace': {
		id: 'app.lab.command-generator.preset.fill-mode.replace',
		defaultMessage: 'Replace',
	},
	'app.lab.command-generator.preset.gamemode.adventure': {
		id: 'app.lab.command-generator.preset.gamemode.adventure',
		defaultMessage: 'Adventure',
	},
	'app.lab.command-generator.preset.gamemode.creative': {
		id: 'app.lab.command-generator.preset.gamemode.creative',
		defaultMessage: 'Creative',
	},
	'app.lab.command-generator.preset.gamemode.spectator': {
		id: 'app.lab.command-generator.preset.gamemode.spectator',
		defaultMessage: 'Spectator',
	},
	'app.lab.command-generator.preset.gamemode.survival': {
		id: 'app.lab.command-generator.preset.gamemode.survival',
		defaultMessage: 'Survival',
	},
	'app.lab.command-generator.preset.gamerule.fire-radius': {
		id: 'app.lab.command-generator.preset.gamerule.fire-radius',
		defaultMessage: 'Fire spread radius',
	},
	'app.lab.command-generator.preset.gamerule.keep-inventory': {
		id: 'app.lab.command-generator.preset.gamerule.keep-inventory',
		defaultMessage: 'Keep inventory',
	},
	'app.lab.command-generator.preset.gamerule.no-command-feedback': {
		id: 'app.lab.command-generator.preset.gamerule.no-command-feedback',
		defaultMessage: 'Disable command feedback',
	},
	'app.lab.command-generator.preset.gamerule.no-daylight-cycle': {
		id: 'app.lab.command-generator.preset.gamerule.no-daylight-cycle',
		defaultMessage: 'Freeze daylight cycle',
	},
	'app.lab.command-generator.preset.gamerule.no-fire-tick': {
		id: 'app.lab.command-generator.preset.gamerule.no-fire-tick',
		defaultMessage: 'Disable fire spread',
	},
	'app.lab.command-generator.preset.gamerule.no-insomnia': {
		id: 'app.lab.command-generator.preset.gamerule.no-insomnia',
		defaultMessage: 'Disable insomnia',
	},
	'app.lab.command-generator.preset.gamerule.no-mob-griefing': {
		id: 'app.lab.command-generator.preset.gamerule.no-mob-griefing',
		defaultMessage: 'Disable mob griefing',
	},
	'app.lab.command-generator.preset.gamerule.no-mob-spawning': {
		id: 'app.lab.command-generator.preset.gamerule.no-mob-spawning',
		defaultMessage: 'Disable mob spawning',
	},
	'app.lab.command-generator.preset.gamerule.no-natural-regen': {
		id: 'app.lab.command-generator.preset.gamerule.no-natural-regen',
		defaultMessage: 'Disable natural regeneration',
	},
	'app.lab.command-generator.preset.gamerule.no-weather-cycle': {
		id: 'app.lab.command-generator.preset.gamerule.no-weather-cycle',
		defaultMessage: 'Freeze weather cycle',
	},
	'app.lab.command-generator.preset.hand.mainhand': {
		id: 'app.lab.command-generator.preset.hand.mainhand',
		defaultMessage: 'Main hand',
	},
	'app.lab.command-generator.preset.hand.offhand': {
		id: 'app.lab.command-generator.preset.hand.offhand',
		defaultMessage: 'Off hand',
	},
	'app.lab.command-generator.preset.item.diamond': {
		id: 'app.lab.command-generator.preset.item.diamond',
		defaultMessage: 'Diamond',
	},
	'app.lab.command-generator.preset.item.diamond-sword': {
		id: 'app.lab.command-generator.preset.item.diamond-sword',
		defaultMessage: 'Diamond Sword',
	},
	'app.lab.command-generator.preset.item.elytra': {
		id: 'app.lab.command-generator.preset.item.elytra',
		defaultMessage: 'Elytra',
	},
	'app.lab.command-generator.preset.item.enchanted-golden-apple': {
		id: 'app.lab.command-generator.preset.item.enchanted-golden-apple',
		defaultMessage: 'Enchanted Golden Apple',
	},
	'app.lab.command-generator.preset.item.golden-apple': {
		id: 'app.lab.command-generator.preset.item.golden-apple',
		defaultMessage: 'Golden Apple',
	},
	'app.lab.command-generator.preset.item.netherite-ingot': {
		id: 'app.lab.command-generator.preset.item.netherite-ingot',
		defaultMessage: 'Netherite Ingot',
	},
	'app.lab.command-generator.preset.minutes.15': {
		id: 'app.lab.command-generator.preset.minutes.15',
		defaultMessage: '15 minutes',
	},
	'app.lab.command-generator.preset.minutes.30': {
		id: 'app.lab.command-generator.preset.minutes.30',
		defaultMessage: '30 minutes',
	},
	'app.lab.command-generator.preset.minutes.5': {
		id: 'app.lab.command-generator.preset.minutes.5',
		defaultMessage: '5 minutes',
	},
	'app.lab.command-generator.preset.particle.explosion': {
		id: 'app.lab.command-generator.preset.particle.explosion',
		defaultMessage: 'Explosion',
	},
	'app.lab.command-generator.preset.particle.flame': {
		id: 'app.lab.command-generator.preset.particle.flame',
		defaultMessage: 'Fire',
	},
	'app.lab.command-generator.preset.particle.heart': {
		id: 'app.lab.command-generator.preset.particle.heart',
		defaultMessage: 'Heart',
	},
	'app.lab.command-generator.preset.particle.portal': {
		id: 'app.lab.command-generator.preset.particle.portal',
		defaultMessage: 'Portal',
	},
	'app.lab.command-generator.preset.pitch.down': {
		id: 'app.lab.command-generator.preset.pitch.down',
		defaultMessage: 'Down 90',
	},
	'app.lab.command-generator.preset.pitch.horizontal': {
		id: 'app.lab.command-generator.preset.pitch.horizontal',
		defaultMessage: 'Level 0',
	},
	'app.lab.command-generator.preset.pitch.up': {
		id: 'app.lab.command-generator.preset.pitch.up',
		defaultMessage: 'Up -90',
	},
	'app.lab.command-generator.preset.port.25565': {
		id: 'app.lab.command-generator.preset.port.25565',
		defaultMessage: '25565',
	},
	'app.lab.command-generator.preset.port.25566': {
		id: 'app.lab.command-generator.preset.port.25566',
		defaultMessage: '25566',
	},
	'app.lab.command-generator.preset.random-action.reset': {
		id: 'app.lab.command-generator.preset.random-action.reset',
		defaultMessage: 'Reset',
	},
	'app.lab.command-generator.preset.random-action.roll': {
		id: 'app.lab.command-generator.preset.random-action.roll',
		defaultMessage: 'Roll',
	},
	'app.lab.command-generator.preset.random-action.value': {
		id: 'app.lab.command-generator.preset.random-action.value',
		defaultMessage: 'Value',
	},
	'app.lab.command-generator.preset.range.0-1': {
		id: 'app.lab.command-generator.preset.range.0-1',
		defaultMessage: '0 to 1',
	},
	'app.lab.command-generator.preset.range.1-10': {
		id: 'app.lab.command-generator.preset.range.1-10',
		defaultMessage: '1 to 10',
	},
	'app.lab.command-generator.preset.range.1-100': {
		id: 'app.lab.command-generator.preset.range.1-100',
		defaultMessage: '1 to 100',
	},
	'app.lab.command-generator.preset.range.1-6': {
		id: 'app.lab.command-generator.preset.range.1-6',
		defaultMessage: '1 to 6',
	},
	'app.lab.command-generator.preset.range.exact-5': {
		id: 'app.lab.command-generator.preset.range.exact-5',
		defaultMessage: 'Exactly 5',
	},
	'app.lab.command-generator.preset.range.max-5': {
		id: 'app.lab.command-generator.preset.range.max-5',
		defaultMessage: 'At most 5',
	},
	'app.lab.command-generator.preset.range.min-10': {
		id: 'app.lab.command-generator.preset.range.min-10',
		defaultMessage: 'At least 10',
	},
	'app.lab.command-generator.preset.ride-action.dismount': {
		id: 'app.lab.command-generator.preset.ride-action.dismount',
		defaultMessage: 'Dismount',
	},
	'app.lab.command-generator.preset.ride-action.mount': {
		id: 'app.lab.command-generator.preset.ride-action.mount',
		defaultMessage: 'Mount',
	},
	'app.lab.command-generator.preset.rotation.east': {
		id: 'app.lab.command-generator.preset.rotation.east',
		defaultMessage: 'East 90',
	},
	'app.lab.command-generator.preset.rotation.north': {
		id: 'app.lab.command-generator.preset.rotation.north',
		defaultMessage: 'North 0',
	},
	'app.lab.command-generator.preset.rotation.south': {
		id: 'app.lab.command-generator.preset.rotation.south',
		defaultMessage: 'South 180',
	},
	'app.lab.command-generator.preset.rotation.west': {
		id: 'app.lab.command-generator.preset.rotation.west',
		defaultMessage: 'West -90',
	},
	'app.lab.command-generator.preset.scale.0-05': {
		id: 'app.lab.command-generator.preset.scale.0-05',
		defaultMessage: '0.05',
	},
	'app.lab.command-generator.preset.scale.1': {
		id: 'app.lab.command-generator.preset.scale.1',
		defaultMessage: '1',
	},
	'app.lab.command-generator.preset.scale.20': {
		id: 'app.lab.command-generator.preset.scale.20',
		defaultMessage: '20',
	},
	'app.lab.command-generator.preset.scoreboard.category.objectives': {
		id: 'app.lab.command-generator.preset.scoreboard.category.objectives',
		defaultMessage: 'Objective',
	},
	'app.lab.command-generator.preset.scoreboard.category.players': {
		id: 'app.lab.command-generator.preset.scoreboard.category.players',
		defaultMessage: 'Players',
	},
	'app.lab.command-generator.preset.slot.all': {
		id: 'app.lab.command-generator.preset.slot.all',
		defaultMessage: 'All slots',
	},
	'app.lab.command-generator.preset.slot.container-all': {
		id: 'app.lab.command-generator.preset.slot.container-all',
		defaultMessage: 'All containers',
	},
	'app.lab.command-generator.preset.slot.mainhand': {
		id: 'app.lab.command-generator.preset.slot.mainhand',
		defaultMessage: 'Main hand',
	},
	'app.lab.command-generator.preset.stopwatch-action.create': {
		id: 'app.lab.command-generator.preset.stopwatch-action.create',
		defaultMessage: 'Create',
	},
	'app.lab.command-generator.preset.stopwatch-action.query': {
		id: 'app.lab.command-generator.preset.stopwatch-action.query',
		defaultMessage: 'Query',
	},
	'app.lab.command-generator.preset.stopwatch-action.remove': {
		id: 'app.lab.command-generator.preset.stopwatch-action.remove',
		defaultMessage: 'Remove',
	},
	'app.lab.command-generator.preset.stopwatch-action.restart': {
		id: 'app.lab.command-generator.preset.stopwatch-action.restart',
		defaultMessage: 'Restart',
	},
	'app.lab.command-generator.preset.tag-action.add': {
		id: 'app.lab.command-generator.preset.tag-action.add',
		defaultMessage: 'Add',
	},
	'app.lab.command-generator.preset.tag-action.list': {
		id: 'app.lab.command-generator.preset.tag-action.list',
		defaultMessage: 'List',
	},
	'app.lab.command-generator.preset.tag-action.remove': {
		id: 'app.lab.command-generator.preset.tag-action.remove',
		defaultMessage: 'Remove',
	},
	'app.lab.command-generator.preset.tick-action.freeze': {
		id: 'app.lab.command-generator.preset.tick-action.freeze',
		defaultMessage: 'Freeze',
	},
	'app.lab.command-generator.preset.tick-action.query': {
		id: 'app.lab.command-generator.preset.tick-action.query',
		defaultMessage: 'Query',
	},
	'app.lab.command-generator.preset.tick-action.rate': {
		id: 'app.lab.command-generator.preset.tick-action.rate',
		defaultMessage: 'Set rate',
	},
	'app.lab.command-generator.preset.tick-action.sprint': {
		id: 'app.lab.command-generator.preset.tick-action.sprint',
		defaultMessage: 'Sprint',
	},
	'app.lab.command-generator.preset.tick-action.step': {
		id: 'app.lab.command-generator.preset.tick-action.step',
		defaultMessage: 'Step',
	},
	'app.lab.command-generator.preset.tick-action.unfreeze': {
		id: 'app.lab.command-generator.preset.tick-action.unfreeze',
		defaultMessage: 'Unfreeze',
	},
	'app.lab.command-generator.preset.tick-rate.1': {
		id: 'app.lab.command-generator.preset.tick-rate.1',
		defaultMessage: '1',
	},
	'app.lab.command-generator.preset.tick-rate.100': {
		id: 'app.lab.command-generator.preset.tick-rate.100',
		defaultMessage: '100',
	},
	'app.lab.command-generator.preset.tick-rate.20': {
		id: 'app.lab.command-generator.preset.tick-rate.20',
		defaultMessage: '20',
	},
	'app.lab.command-generator.preset.time-action.add': {
		id: 'app.lab.command-generator.preset.time-action.add',
		defaultMessage: 'Add',
	},
	'app.lab.command-generator.preset.time-action.query': {
		id: 'app.lab.command-generator.preset.time-action.query',
		defaultMessage: 'Query',
	},
	'app.lab.command-generator.preset.time-action.set': {
		id: 'app.lab.command-generator.preset.time-action.set',
		defaultMessage: 'Set',
	},
	'app.lab.command-generator.preset.time-of-day.day': {
		id: 'app.lab.command-generator.preset.time-of-day.day',
		defaultMessage: 'Day',
	},
	'app.lab.command-generator.preset.time-of-day.midnight': {
		id: 'app.lab.command-generator.preset.time-of-day.midnight',
		defaultMessage: 'Midnight',
	},
	'app.lab.command-generator.preset.time-of-day.night': {
		id: 'app.lab.command-generator.preset.time-of-day.night',
		defaultMessage: 'Night',
	},
	'app.lab.command-generator.preset.time-of-day.noon': {
		id: 'app.lab.command-generator.preset.time-of-day.noon',
		defaultMessage: 'Noon',
	},
	'app.lab.command-generator.preset.time.10s': {
		id: 'app.lab.command-generator.preset.time.10s',
		defaultMessage: '10 seconds',
	},
	'app.lab.command-generator.preset.time.1d': {
		id: 'app.lab.command-generator.preset.time.1d',
		defaultMessage: '1 day',
	},
	'app.lab.command-generator.preset.time.1m': {
		id: 'app.lab.command-generator.preset.time.1m',
		defaultMessage: '1 minute',
	},
	'app.lab.command-generator.preset.time.1s': {
		id: 'app.lab.command-generator.preset.time.1s',
		defaultMessage: '1 second',
	},
	'app.lab.command-generator.preset.time.clock-time': {
		id: 'app.lab.command-generator.preset.time.clock-time',
		defaultMessage: 'Clock ticks',
	},
	'app.lab.command-generator.preset.time.day-cycles': {
		id: 'app.lab.command-generator.preset.time.day-cycles',
		defaultMessage: 'Daylight cycle count',
	},
	'app.lab.command-generator.preset.time.game-time': {
		id: 'app.lab.command-generator.preset.time.game-time',
		defaultMessage: 'Game ticks',
	},
	'app.lab.command-generator.preset.unit.levels': {
		id: 'app.lab.command-generator.preset.unit.levels',
		defaultMessage: 'Levels',
	},
	'app.lab.command-generator.preset.unit.points': {
		id: 'app.lab.command-generator.preset.unit.points',
		defaultMessage: 'Points',
	},
	'app.lab.command-generator.preset.waypoint-action.add': {
		id: 'app.lab.command-generator.preset.waypoint-action.add',
		defaultMessage: 'Add',
	},
	'app.lab.command-generator.preset.waypoint-action.list': {
		id: 'app.lab.command-generator.preset.waypoint-action.list',
		defaultMessage: 'List',
	},
	'app.lab.command-generator.preset.waypoint-action.modify': {
		id: 'app.lab.command-generator.preset.waypoint-action.modify',
		defaultMessage: 'Modify',
	},
	'app.lab.command-generator.preset.waypoint-action.remove': {
		id: 'app.lab.command-generator.preset.waypoint-action.remove',
		defaultMessage: 'Remove',
	},
	'app.lab.command-generator.preset.weather.clear': {
		id: 'app.lab.command-generator.preset.weather.clear',
		defaultMessage: 'Clear',
	},
	'app.lab.command-generator.preset.weather.rain': {
		id: 'app.lab.command-generator.preset.weather.rain',
		defaultMessage: 'Rain',
	},
	'app.lab.command-generator.preset.weather.thunder': {
		id: 'app.lab.command-generator.preset.weather.thunder',
		defaultMessage: 'Thunder',
	},
})

export function commandMessage(id: string, fallback?: string): MessageDescriptor {
	return Object.hasOwn(messages, id)
		? (messages as Record<string, MessageDescriptor>)[id]
		: { id, defaultMessage: fallback || id }
}
