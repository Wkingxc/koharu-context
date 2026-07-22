import os from 'node:os'

export function tauriBuildArgs(platform = os.type()) {
  const args = ['run', 'scripts/dev.ts', 'tauri', 'build']
  if (platform === 'Darwin') {
    args.push('--bundles', 'app')
  } else {
    args.push('--no-bundle')
  }
  return args
}

if (import.meta.main) {
  const build = Bun.spawn([process.execPath, ...tauriBuildArgs()], {
    stdin: 'inherit',
    stdout: 'inherit',
    stderr: 'inherit',
    env: process.env,
  })

  process.exit(await build.exited)
}
