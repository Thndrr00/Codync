import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { createContext, Script } from 'node:vm'
import ts from 'typescript'

test('creating a bot preserves an explicitly chosen unavailable agent and model', async () => {
  const draft = { name: ' Reviewer ', backend: 'codex', model: 'selected-model', modelOptions: { reasoning: 'high' } }
  let saved: typeof draft | undefined
  let selected: string | undefined
  const store = {
    hello: { backends: [{ id: 'claude', available: true, curated: true }, { id: 'codex', available: false, curated: true }] },
    save: async (value: typeof draft) => { saved = value; return { id: 'new-bot' } },
    setSelection: (id: string) => { selected = id },
  }
  const dependencies: Record<string, unknown> = {
    react: { useState: () => [draft, () => {}], useCallback: (callback: unknown) => callback },
    'react/jsx-runtime': { jsx: (type: unknown, props: unknown) => ({ type, props }), jsxs: (type: unknown, props: unknown) => ({ type, props }) },
    '@shared/models': {}, '@shared/catalog': {},
    '../../components/Avatar': {}, '../../components/Controls': {}, '../../components/Overlay': {},
    '../../components/Stack': {}, '../../components/Text': {},
    '../../store/context': { useStore: () => store }, '../../lib/use-language': {},
    './drafts': {
      fillDefaults: () => ({ ...draft, backend: 'claude', model: null, modelOptions: {} }),
      newDraft: () => ({}),
    },
    './parts': {}, './NewBotParts': {}, './use-catalog': { useCatalog: () => ({ catalog: null }) }, './bots.css': {},
  }
  const source = readFileSync(new URL('../views/bots/NewBotView.tsx', import.meta.url), 'utf8')
  const output = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2023, jsx: ts.JsxEmit.ReactJSX },
    fileName: 'NewBotView.tsx',
  }).outputText
  const exports: { NewBotView?: (props: { computer: null }) => { props: { create: () => Promise<void> } } } = {}
  new Script(output).runInContext(createContext({
    exports,
    require: (name: string) => {
      if (!(name in dependencies)) throw new Error(`Unmocked dependency: ${name}`)
      return dependencies[name]
    },
  }))
  assert.ok(exports.NewBotView)
  await exports.NewBotView({ computer: null }).props.create()
  assert.equal(saved?.name, 'Reviewer')
  assert.equal(saved?.backend, 'codex')
  assert.equal(saved?.model, 'selected-model')
  assert.equal(saved?.modelOptions.reasoning, 'high')
  assert.equal(selected, 'new-bot')
})
