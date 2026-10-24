import { execFileSync } from 'node:child_process'
import { prepareBlockbench, prepareJava, run } from './prepare-build-resources.mjs'

const targets = process.argv.slice(2)
if (targets.some((target) => !/^[a-zA-Z0-9_-]+$/.test(target))) {
	throw new Error('Expected Rust target triples as arguments')
}
if (!targets.length) {
	targets.push(execFileSync('rustc', ['-vV'], { encoding: 'utf8' }).match(/^host: (.+)$/m)[1])
}

const start = performance.now()
const tasks = await Promise.allSettled([
	run('cargo', ['fetch', '--locked', ...targets.flatMap((target) => ['--target', target])]),
	prepareJava(),
	prepareBlockbench(),
])
for (const result of tasks) {
	if (result.status === 'rejected') {
		console.error(result.reason)
		process.exitCode = 1
	}
}
console.log(`[prepare] finished in ${((performance.now() - start) / 1000).toFixed(2)}s`)
