#!/usr/bin/env node
// Turns a ComfyUI workflow saved by the editor (UI format: `nodes`, `links`, `widgets_values`) into
// the API format a queue accepts (`{ "<id>": { class_type, inputs } }`), so a graph an author only
// publishes for the editor can still become a template. The decisions it makes, in order:
//
// - A subgraph instance is inlined first: its definition is expanded with prefixed ids, its
//   promoted inputs are wired to the outer links, and the wires that used its promoted outputs are
//   pointed at the node inside it that fed them.
// - Reroutes are followed and Primitive nodes are folded into the literal they feed.
// - Muted nodes (mode 2) are dropped; a bypassed node (mode 4) is replaced by a same-typed wire, and
//   dropped when it has none.
// - Notes and other node-less widgets are dropped.
// - The remaining node's `inputs` become links. Its widget inputs keep the next value of
//   `widgets_values` in order (or the value under their name when that is an object); a saved graph
//   only names the widgets it wired, so the rest come from `comfy-widget-names.json`, the field
//   order each node type declares in its pack. A type absent from both is an error.
//
// It never invents values. Usage: node tools/comfy-ui-to-api.mjs <workflow.json> [out.json]
import { readFile, writeFile } from 'node:fs/promises'
import { pathToFileURL } from 'node:url'

const { default: WIDGET_NAMES } = await import('./comfy-widget-names.json', { with: { type: 'json' } })

const PASS_THROUGH = new Set(['Reroute'])
const SILENT = new Set(['MarkdownNote', 'Note', 'PrimitiveNode', 'Reroute'])
const PROMOTED_IN = '-10'
const PROMOTED_OUT = '-20'

/** The graph with every subgraph instance replaced by its definition, ids prefixed and unique. */
export function flatten(graph, depth = 0) {
  if (depth > 4) throw new Error('subgraphs nested too deep')
  const defs = new Map((graph.definitions?.subgraphs ?? []).map((sub) => [sub.id, sub]))
  if (defs.size === 0 && depth === 0) return graph
  const nodes = []
  const links = []
  const instances = new Map()
  const promoted = new Map()
  const outerById = new Map((graph.links ?? []).map((link) => [link[0], link]))

  for (const node of graph.nodes ?? []) {
    const def = defs.get(node.type)
    if (!def) {
      nodes.push(node)
      continue
    }
    const prefix = `${node.id}:`
    // Content the recursion already namespaced through a nested instance keeps its ids; only ids of
    // this level's own definition get the prefix.
    const rename = (id) => (String(id).includes(':') ? String(id) : `${prefix}${id}`)
    const inside = flatten(def, depth + 1)
    for (const marker of inside.nodes.filter((inner) => String(inner.type) === PROMOTED_IN)) {
      // The name sits on the node in some frontends and on its single output in others.
      const name = marker.name ?? marker.outputs?.[0]?.name ?? marker.widgets_values?.[0]
      const outer = node.inputs?.find((input) => input.name === name)
      if (outer?.link != null) promoted.set(rename(marker.id), outer.link)
    }
    const exported = new Map()
    for (const link of inside.links) {
      if (String(link[3]) === PROMOTED_OUT || String(link[3]) === rename(PROMOTED_OUT)) exported.set(link[4], rename(link[1]))
    }
    instances.set(node.id, exported)
    for (const inner of inside.nodes.filter((candidate) => String(candidate.type) !== PROMOTED_IN && String(candidate.type) !== PROMOTED_OUT)) {
      nodes.push({ ...inner, id: rename(inner.id) })
    }
    for (const link of inside.links) links.push([link[0], rename(link[1]), link[2], rename(link[3]), link[4], link[5]])
  }
  for (const link of graph.links ?? []) links.push([link[0], link[1], link[2], link[3], link[4], link[5]])

  // A wire that leaves an instance is re-pointed at the node inside it that filled that output, and
  // one that leaves a promoted input continues at whatever fed the instance's own input.
  for (const link of links) {
    const outer = promoted.get(link[1])
    if (outer !== undefined) {
      const source = outerById.get(outer)
      if (source) {
        link[1] = source[1]
        link[2] = source[2]
      }
    }
    const filled = instances.get(link[1])?.get(link[2])
    if (filled) link[1] = filled
  }
  return { nodes, links, definitions: graph.definitions }
}

export function uiToApi(input) {
  const graph = flatten(input)
  const nodes = new Map((graph.nodes ?? []).map((node) => [node.id, node]))
  const links = new Map((graph.links ?? []).map((link) => [link[0], link[1] == null ? null : { from: link[1], slot: link[2] }]))

  /** The wire that really feeds a link: through reroutes and bypassed nodes. */
  const source = (linkId) => {
    let link = links.get(linkId)
    while (link) {
      const origin = nodes.get(link.from)
      if (!origin) return null
      if (PASS_THROUGH.has(origin.type)) {
        link = links.get(origin.inputs?.[0]?.link)
        continue
      }
      if (origin.mode === 4) {
        const type = origin.outputs?.[link.slot]?.type
        const feed = origin.inputs?.find((wire) => wire.type === type && wire.link != null)
        link = feed ? links.get(feed.link) : null
        continue
      }
      if (origin.mode === 2) return null
      return link
    }
    return null
  }

  /** A Primitive node's single value, when the wire comes straight from one. */
  const primitive = (linkId) => {
    let link = links.get(linkId)
    while (link) {
      const origin = nodes.get(link.from)
      if (!origin) return undefined
      if (PASS_THROUGH.has(origin.type)) {
        link = links.get(origin.inputs?.[0]?.link)
        continue
      }
      if (origin.type !== 'PrimitiveNode') return undefined
      return origin.widgets_values?.[0]
    }
    return undefined
  }

  const api = {}
  for (const node of graph.nodes ?? []) {
    if (node.mode === 2 || node.mode === 4 || SILENT.has(node.type)) continue
    const values = Array.isArray(node.widgets_values) ? node.widgets_values : []
    const named = node.widgets_values_named && typeof node.widgets_values_named === 'object'
      ? node.widgets_values_named
      : (!Array.isArray(node.widgets_values) && node.widgets_values ? node.widgets_values : null)
    const inputs = {}
    const sockets = new Set((node.inputs ?? []).map((input) => input.name))
    const wired = new Set((node.inputs ?? []).filter((input) => input.link != null).map((input) => input.name))
    const converted = new Set((node.inputs ?? []).filter((input) => input.widget).map((input) => input.name))
    // Newer frontends save `widgets_values_named`, the widget names with their values; only the
    // older ones save a bare list, which the name table turns back into names by position.
    const widgets = named ? Object.keys(named) : (WIDGET_NAMES[node.type] ?? [])
    if (widgets.length === 0 && (node.inputs ?? []).some((input) => input.widget)) {
      throw new Error(`node ${node.id} (${node.type}) has widgets the name table doesn't know`)
    }
    let next = 0
    for (const input of node.inputs ?? []) {
      if (input.link == null) continue
      const literal = primitive(input.link)
      if (literal !== undefined) inputs[input.name] = literal
      else {
        const link = source(input.link)
        if (link) inputs[input.name] = [link.from, link.slot]
      }
    }
    // A widget turned into a socket keeps its value in `widgets_values`; a socket that was never a
    // widget does not. So only the former consumes a place.
    for (const name of widgets) {
      if (sockets.has(name) && !converted.has(name)) continue
      const value = named ? named[name] : values[next++]
      if (value !== undefined && !wired.has(name)) inputs[name] = value
    }
    api[node.id] = { class_type: node.type, inputs }
  }
  return api
}

const main = process.argv[1] && pathToFileURL(process.argv[1]).href === import.meta.url
if (main) {
  const [, , input, output] = process.argv
  if (!input) {
    console.error('usage: comfy-ui-to-api.mjs <workflow.json> [out.json]')
    process.exit(2)
  }
  const api = uiToApi(JSON.parse(await readFile(input, 'utf8')))
  const text = `${JSON.stringify(api, null, 2)}\n`
  if (output) await writeFile(output, text)
  else process.stdout.write(text)
}
