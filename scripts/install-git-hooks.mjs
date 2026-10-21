// Point this clone at .githooks so the unit gate runs before every push.
// CI checkouts skip it: GitHub already runs the same checks, and a runner
// must not rewrite its git config.
import { execFileSync } from 'node:child_process'
import { existsSync } from 'node:fs'

if (process.env.CI || process.env.GITHUB_ACTIONS) {
  process.exit(0)
}

if (!existsSync('.githooks/pre-push')) {
  process.exit(0)
}

try {
  execFileSync('git', ['rev-parse', '--is-inside-work-tree'], { stdio: 'ignore' })
} catch {
  process.exit(0)
}

execFileSync('git', ['config', 'core.hooksPath', '.githooks'])
console.log('Git hooks: .githooks (pre-push runs the unit gate)')
