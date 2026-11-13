#!/usr/bin/env node
// Checks the ComfyUI manifests the app embeds, without compiling the host: ids, placeholders,
// input types, model folders/sizes/hashes/URLs and the output kind. The Rust loader enforces the
// same rules; this is the fast check while editing the JSON.
import { readdir, readFile } from 'node:fs/promises'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const dir = fileURLToPath(new URL('../host/assets/comfy-templates/', import.meta.url))
const folders = [
  'checkpoints', 'configs', 'loras', 'vae', 'text_encoders', 'diffusion_models',
  'clip_vision', 'embeddings', 'controlnet', 'upscale_models', 'vae_approx',
]
const kinds = ['text', 'number', 'integer', 'select', 'image', 'seed', 'boolean']
const outputs = ['image', 'video', 'audio', 'mesh']

function placeholders(value, found = new Set()) {
  if (typeof value === 'string') {
    for (const match of value.matchAll(/{{([a-z0-9_]+)}}/g)) found.add(match[1])
  } else if (Array.isArray(value)) {
    value.forEach((item) => placeholders(item, found))
  } else if (value && typeof value === 'object') {
    Object.values(value).forEach((item) => placeholders(item, found))
  }
  return found
}

let failures = 0
for (const file of (await readdir(dir)).filter((name) => name.endsWith('.json') && name !== 'curated.json')) {
  const manifest = JSON.parse(await readFile(join(dir, file), 'utf8'))
  const fail = (message) => { failures++; console.error(`${file}: ${message}`) }
  if (!/^[a-z0-9_-]+$/.test(manifest.id)) fail(`invalid id ${manifest.id}`)
  if (!/^\d+\.\d+\.\d+$/.test(manifest.version)) fail(`invalid version ${manifest.version}`)
  if (!manifest.workflow || Object.keys(manifest.workflow).length === 0) fail('empty workflow')
  for (const [node, body] of Object.entries(manifest.workflow ?? {})) {
    if (!body.class_type) fail(`node ${node} has no class_type`)
  }
  const keys = new Set()
  for (const input of manifest.inputs ?? []) {
    if (keys.has(input.key)) fail(`duplicate input ${input.key}`)
    keys.add(input.key)
    if (!kinds.includes(input.type)) fail(`input ${input.key} has type ${input.type}`)
    if (input.type === 'select' && (!input.options?.length || (input.default != null && !input.options.includes(input.default)))) {
      fail(`select ${input.key} has no usable options/default`)
    }
  }
  const used = placeholders(manifest.workflow)
  for (const key of used) if (!keys.has(key)) fail(`placeholder {{${key}}} has no input`)
  for (const key of keys) if (!used.has(key)) fail(`input ${key} is never used`)
  for (const model of manifest.models ?? []) {
    if (!folders.includes(model.folder)) fail(`model ${model.file} has folder ${model.folder}`)
    if (!(model.size > 0)) fail(`model ${model.file} has no size`)
    if (model.sha256 && !/^[a-f0-9]{64}$/i.test(model.sha256)) fail(`model ${model.file} has a bad SHA-256`)
    if (model.url && !/^https?:\/\//.test(model.url)) fail(`model ${model.file} has a non-HTTP URL`)
    if (model.gated && !model.url) fail(`gated model ${model.file} has no page`)
  }
  if (!outputs.includes(manifest.output?.type)) fail(`output ${manifest.output?.type} is invalid`)
}

if (failures > 0) process.exit(1)
console.log('ComfyUI manifests: ok')
