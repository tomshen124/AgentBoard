// Build TaskLoop sidecar binary and copy to resources/bin/
// Called before electron-builder packaging.

import { spawnSync } from 'child_process'
import { copyFileSync, mkdirSync, existsSync } from 'fs'
import { join, dirname } from 'path'
import { fileURLToPath } from 'url'
import { platform } from 'os'

const __dirname = dirname(fileURLToPath(import.meta.url))
const projectRoot = join(__dirname, '..', '..', '..')
const targetDir = join(__dirname, '..', 'resources', 'bin')

const binaryName = platform() === 'win32' ? 'taskloop-sidecar.exe' : 'taskloop-sidecar'

console.log('[build-sidecar] Building TaskLoop sidecar...')

// Compile in release mode
const build = spawnSync('cargo', ['build', '-p', 'taskloop-sidecar', '--release'], {
  cwd: projectRoot,
  stdio: 'inherit',
  shell: true
})

if (build.status !== 0) {
  console.error('[build-sidecar] Cargo build failed')
  process.exit(1)
}

// Copy binary to resources/bin
const sourcePath = join(projectRoot, 'target', 'release', binaryName)

if (!existsSync(sourcePath)) {
  console.error(`[build-sidecar] Binary not found at ${sourcePath}`)
  process.exit(1)
}

mkdirSync(targetDir, { recursive: true })
const destPath = join(targetDir, binaryName)
copyFileSync(sourcePath, destPath)
console.log(`[build-sidecar] Copied ${sourcePath} -> ${destPath}`)

// Make executable on non-Windows
if (platform() !== 'win32') {
  spawnSync('chmod', ['+x', destPath])
}
