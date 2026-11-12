#!/usr/bin/env node
// Builds a CycloneDX 1.5 JSON inventory of the host's Rust crates and the desktop app's npm packages.
// Run from the repository root: node tools/sbom.mjs [out.json]
// It shells out to `cargo metadata`, so run it where the Rust toolchain is installed (CI does).
import { execFileSync } from 'node:child_process'
import { readFileSync, writeFileSync } from 'node:fs'
import { createHash } from 'node:crypto'

const out = process.argv[2] ?? 'sbom.cdx.json'

const components = []

const cargo = JSON.parse(
  execFileSync('cargo', ['metadata', '--format-version', '1', '--manifest-path', 'host/Cargo.toml'], {
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  }),
)
for (const pkg of cargo.packages) {
  if (!pkg.source) continue // our own workspace crates are not third-party components
  const url = pkg.source.replace(/^registry\+/, '').replace(/#.*$/, '')
  components.push({
    type: 'library',
    name: pkg.name,
    version: pkg.version,
    purl: `pkg:cargo/${pkg.name}@${pkg.version}`,
    externalReferences: [{ type: 'distribution', url }],
  })
}

const lock = JSON.parse(readFileSync('apps/desktop/package-lock.json', 'utf8'))
for (const [path, pkg] of Object.entries(lock.packages ?? {})) {
  if (!path.startsWith('node_modules/') || pkg.dev === true) continue
  const name = pkg.name ?? path.slice('node_modules/'.length)
  if (!pkg.version) continue
  components.push({
    type: 'library',
    name,
    version: pkg.version,
    purl: `pkg:npm/${name.replace('@', '%40')}@${pkg.version}`,
    properties: [{ name: 'npm:resolved', value: pkg.resolved ?? '' }],
  })
}

const unique = [...new Map(components.map((c) => [c.purl, c])).values()].sort((a, b) => a.purl.localeCompare(b.purl))

const document = {
  bomFormat: 'CycloneDX',
  specVersion: '1.5',
  version: 1,
  metadata: {
    timestamp: new Date().toISOString(),
    tools: [{ vendor: 'Codync', name: 'sbom.mjs' }],
  },
  components: unique,
}

const text = JSON.stringify(document, null, 2) + '\n'
writeFileSync(out, text)
console.log(`wrote ${unique.length} components to ${out} (${createHash('sha256').update(text).digest('hex').slice(0, 12)})`)
