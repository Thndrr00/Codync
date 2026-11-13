#!/usr/bin/env node
// Imports a Freefield app (neofield's "Open Source Higgsfield" studio) as a Codync ComfyUI
// template: its app.json already names the inputs, their bindings and the models, and its
// workflow.api.json is the API-format graph. Model sizes and SHA-256 come from the Hugging Face
// tree API, so the preflight and the downloader can verify them.
//
//   node tools/import-freefield-app.mjs --app app.json --workflow workflow.api.json \
//     --id freefield-angles --name "Angles" --source https://github.com/neuregex/freefield \
//     --out host/assets/comfy-templates/freefield-angles.json
import { readFile, writeFile } from 'node:fs/promises'

const argv = process.argv.slice(2)
const option = (name, fallback = null) => {
  const at = argv.indexOf(name)
  return at >= 0 && argv[at + 1] ? argv[at + 1] : fallback
}

const appPath = option('--app')
const workflowPath = option('--workflow')
const outPath = option('--out')
if (!appPath || !workflowPath || !outPath) {
  console.error('usage: import-freefield-app.mjs --app app.json --workflow workflow.api.json --out template.json [--id id] [--name name] [--source url]')
  process.exit(2)
}

const app = JSON.parse(await readFile(appPath, 'utf8'))
const workflow = JSON.parse(await readFile(workflowPath, 'utf8'))
const id = option('--id', app.id)
const name = option('--name', app.name)
const source = option('--source', '')

/** app.json's types to Codync's template inputs. */
function kindOf(type) {
  switch (type) {
    case 'image': return 'image'
    case 'select': return 'select'
    case 'seed': return 'seed'
    case 'number': case 'int': case 'integer': return 'number'
    case 'boolean': case 'bool': return 'boolean'
    case 'textarea': case 'text': case 'string': return 'text'
    default: return 'text'
  }
}

const inputs = []
const referenced = new Set()
for (const input of app.inputs ?? []) {
  for (const bind of input.bind ?? []) {
    for (const match of (bind.template ?? '').matchAll(/\{([a-zA-Z0-9_]+)\}/g)) referenced.add(match[1])
  }
}
for (const input of app.inputs ?? []) {
  const binds = (input.bind ?? []).filter((bind) => workflow[bind.node])
  // A select can have no binding of its own and still feed another input's template (`{move}`).
  if (binds.length === 0 && !referenced.has(input.id)) continue
  const kind = kindOf(input.type)
  for (const bind of binds) {
    const node = workflow[bind.node]
    if (bind.template) {
      // "{move} {details}" becomes "{{move}} {{details}}": Codync's renderer joins them.
      node.inputs[bind.field] = bind.template.replace(/\{([a-zA-Z0-9_]+)\}/g, '{{$1}}')
    } else {
      node.inputs[bind.field] = `{{${input.id}}}`
    }
  }
  const entry = {
    key: input.id,
    label: input.label ?? input.id,
    type: kind,
    required: input.required === true,
  }
  if (kind === 'select') entry.options = (input.options ?? []).map((option) => (typeof option === 'string' ? option : option.value))
  if (input.default !== undefined && !(kind === 'seed' && input.default === -1)) entry.default = input.default
  if (input.help) entry.help = input.help
  if (input.limits?.max_mb && kind === 'image') entry.help = `${input.help ?? ''} Up to ${input.limits.max_mb} MB.`.trim()
  inputs.push(entry)
}

/** Exact size and LFS SHA-256, from the repository's own tree. */
async function modelInfo(repo, file) {
  const response = await fetch(`https://huggingface.co/api/models/${repo}/tree/main?recursive=1`, {
    headers: { 'user-agent': 'codync-template-import' },
  })
  if (!response.ok) return {}
  const rows = await response.json()
  const hit = (rows ?? []).find((row) => row.path === file)
  return hit ? { size: hit.lfs?.size ?? hit.size ?? 0, sha256: hit.lfs?.oid ?? undefined } : {}
}

const models = []
for (const model of app.models ?? []) {
  const info = await modelInfo(model.repo, model.file)
  models.push({
    file: model.file.split('/').pop(),
    folder: model.dir,
    size: info.size ?? (model.gb ? Math.round(model.gb * 1e9) : 0),
    ...(info.sha256 ? { sha256: info.sha256 } : {}),
    url: `https://huggingface.co/${model.repo}/resolve/main/${model.file}`,
    license: model.license ?? null,
  })
}

const output = app.outputs?.[0]?.type ?? app.section ?? 'image'
const kind = ['video', 'audio', 'mesh'].includes(output) ? output : 'image'
const mime = { image: 'image/png', video: 'video/mp4', audio: 'audio/wav', mesh: 'model/gltf-binary' }[kind]
const totalGb = models.reduce((sum, model) => sum + (model.size > 0 ? model.size : 0), 0) / 1e9
const vram = Math.max(2, Math.ceil(totalGb * 0.75))
const tags = ['higgsfield', 'freefield', app.section].filter(Boolean)
if (vram >= 16) tags.push('high-vram')

const manifest = {
  id,
  name,
  version: '1.0.0',
  description: `${app.summary ?? ''} Imported from Freefield${source ? ` (${source})` : ''}, MIT; its models keep their own licences.`.trim(),
  tags,
  license: 'Freefield is MIT. Models: ' + models.map((model) => model.license).filter(Boolean).join(', '),
  inputs,
  models,
  output: { type: kind, mime },
  estimate: { seconds: 60, vram_gb: vram, output_gb: 0.2, byGpu: { 'rtx-4090': { seconds: 15, vram_gb: vram }, default: { seconds: 60, vram_gb: vram } } },
  workflow,
}

await writeFile(outPath, `${JSON.stringify(manifest, null, 2)}\n`)
console.log(`${outPath}: ${inputs.length} inputs, ${models.length} models, ${totalGb.toFixed(1)} GB, ~${vram} GB VRAM`)
