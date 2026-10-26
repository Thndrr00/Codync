import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import vm from 'node:vm'
import ts from 'typescript'

type Node = { type: string; props: Record<string, any>; children: unknown[] }
function panel(create: (...args: unknown[]) => Promise<unknown>) {
  const state: unknown[] = []
  const copied: string[] = []
  let cursor = 0
  const React = {
    createElement: (type: string, props: Record<string, any>, ...children: unknown[]): Node => ({ type, props: props ?? {}, children }),
    useState: (initial: unknown) => {
      const index = cursor++
      if (!(index in state)) state[index] = initial
      return [state[index], (next: unknown) => { state[index] = typeof next === 'function' ? next(state[index]) : next }]
    },
    useEffect: () => {},
  }
  const source = readFileSync(new URL('../components/BotAppPanel.tsx', import.meta.url), 'utf8')
  const output = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.React, target: ts.ScriptTarget.ES2022 } }).outputText
  const exports: Record<string, any> = {}
  vm.runInNewContext(output, {
    exports, React, window: { codync: { app: { copy: (url: string) => copied.push(url) } } },
    require: (name: string) => {
      if (name === 'react') return React
      if (name === './SlideShell') return { SlideShell: 'shell', useLatchedWhileClosing: (app: unknown) => app }
      return { Button: 'button', SegmentedChoice: 'choice', Icon: 'icon', HStack: 'row', VStack: 'column', Spacer: 'spacer', Text: 'text' }
    },
  })
  const render = () => {
    cursor = 0
    return exports.BotAppPanel({ app: { slug: 'tracker', title: 'Tracker', url: 'http://127.0.0.1/apps/bot/tracker/OWNER/', version: 1 }, onClose: () => {}, onCreateShareLink: create }) as Node
  }
  const nodes = (value: unknown): Node[] => Array.isArray(value) ? value.flatMap(nodes) : value && typeof value === 'object' && 'children' in value
    ? [value as Node, ...(value as Node).children.flatMap(nodes)] : []
  const text = (value: unknown): string => Array.isArray(value) ? value.map(text).join('') : value && typeof value === 'object' && 'children' in value
    ? (value as Node).children.map(text).join('') : typeof value === 'string' ? value : ''
  const button = (root: Node) => nodes(root).find(node => node.type === 'button' && text(node) === 'Create & Copy Link')!
  return { render, button, text, nodes, copied }
}

test('the real share action copies the new capability through the desktop bridge without browser clipboard access', async () => {
  const url = 'http://127.0.0.1:19223/apps/bot/tracker/NEW_VIEW/'
  const ui = panel(async (...args) => {
    assert.deepEqual(args, ['tracker', 'Team link', 'view'])
    return { link: { token: 'NEW_VIEW', mode: 'view' }, url }
  })
  await ui.button(ui.render()).props.onClick()
  assert.deepEqual(ui.copied, [url])
  assert.ok(ui.text(ui.render()).includes(`Copied capability URL: ${url}`))
})

test('a failed second share shows its error and clears the previously copied capability', async () => {
  let attempts = 0
  const url = 'http://127.0.0.1:19223/apps/bot/tracker/PREVIOUS/'
  const ui = panel(async () => {
    if (attempts++ === 0) return { link: { token: 'PREVIOUS', mode: 'view' }, url }
    throw new Error('The computer is offline.')
  })
  await ui.button(ui.render()).props.onClick()
  assert.ok(ui.text(ui.render()).includes(url))
  await ui.button(ui.render()).props.onClick()
  assert.equal(ui.copied.length, 1)
  assert.ok(ui.text(ui.render()).includes('Could not create the share link.'))
  assert.ok(!ui.text(ui.render()).includes('Copied capability URL:'))
})
