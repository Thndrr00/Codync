import { bytesText } from './health.ts'

/** Bundled tools: programs the host runs for the user (see host/src/tools). Only ComfyUI so far; n8n has its own types. */

export type ToolId = 'comfy'

export interface ToolSpec {
  id: ToolId
  /** The renderer page the tool's window shows while the host brings it up (`?window=<page>`). */
  page: string
  title: string
  /** SF Symbol name. */
  icon: string
}

export const TOOLS: Record<ToolId, ToolSpec> = {
  comfy: { id: 'comfy', page: 'comfy', title: 'ComfyUI', icon: 'comfyui.logo' },
}

export type ToolRunState = 'stopped' | 'starting' | 'running' | 'error'

export interface ToolJob {
  id: string
  /** The template that built the graph, when a named template did. */
  template?: string | null
  phase: 'queued' | 'running' | 'done' | 'cancelled' | 'error'
  error: string | null
  files: string[]
  /** The same files with their real type: a video is not an oddly named PNG. */
  outputs?: ToolOutput[]
  progress?: ToolProgress | null
  createdAt?: number
}

export interface ToolOutput {
  path: string
  kind: 'image' | 'video' | 'audio' | 'mesh' | 'file'
  mime: string
  /** A JPEG poster for a video; null when ffmpeg is not installed. */
  thumbnail: string | null
}

export interface ToolProgress {
  node: string
  class: string
  step: number
  total: number
  percent: number | null
}

/** One form field of a template (`host/src/tools/comfy/templates.rs`). */
export interface TemplateInput {
  key: string
  label: string
  type: 'text' | 'number' | 'integer' | 'select' | 'image' | 'video' | 'audio' | 'seed' | 'boolean'
  default?: unknown
  min?: number | null
  max?: number | null
  step?: number | null
  options?: string[]
  required?: boolean
  help?: string | null
}

export interface TemplateModel {
  file: string
  folder: string
  size: number
  sha256?: string | null
  url?: string | null
  /** The repository requires signing in before the file can be downloaded. */
  gated?: boolean
  license?: string | null
}

export interface TemplatePreflightProblem {
  code: string
  message: string
  action: { kind: 'start' | 'download' | 'openUrl' | 'installNode' | 'lowerResolution' | 'freeSpace' | 'useCpu'; [key: string]: unknown }
}

/** A gallery row (`comfyTemplates`). */
export interface ComfyTemplateSummary {
  id: string
  name: string
  version: string
  description: string
  output: 'image' | 'video' | 'audio' | 'mesh'
  tags: string[]
  estimateSeconds: number | null
  vramGb: number | null
  inputs: number
  models: number
}

/** One template's full form (`comfyTemplate`). */
export interface ComfyTemplate extends Omit<ComfyTemplateSummary, 'output' | 'estimateSeconds' | 'vramGb' | 'inputs' | 'models'> {
  inputs: TemplateInput[]
  models: TemplateModel[]
  output: { type: 'image' | 'video' | 'audio' | 'mesh'; mime?: string | null }
  estimate: { seconds?: number | null; vram_gb?: number | null; secondsForThisGpu?: number | null; vramForThisGpu?: number | null }
  license?: string | null
  nodeCount: number
  preflight?: {
    ok: boolean
    problems: TemplatePreflightProblem[]
    warnings: TemplatePreflightProblem[]
    estimate: { seconds: number | null; vramGb: number | null; gpu: string | null; outputGb: number }
  }
}

export interface ModelFile {
  file: string
  size: number
  present: boolean
  declared: boolean
  usedBy: string[]
  url?: string | null
  gated?: boolean
  license?: string | null
}

export interface ModelFolder {
  folder: string
  path: string
  bytes: number
  models: ModelFile[]
}

export interface ModelDownload {
  folder: string
  file: string
  done: number
  total: number
  phase: 'downloading' | 'paused' | 'done' | 'failed'
  error: string | null
  percent: number | null
}

export interface ModelLibrary {
  folders: ModelFolder[]
  totalBytes: number
  diskBytes: number
  freeBytes: number
  downloads: ModelDownload[]
}

/** One Open Source Higgsfield entry: its author's repository, not an installed template. */
export interface CuratedTemplate {
  id: string
  name: string
  repo: string
  license: string
  output: string[]
  summary: string
  author: string
  /** True once its graph is a manifest of its own; the gallery hides it. */
  imported?: boolean
}


/** What the host shows on ComfyUI's card. */
export interface ComfyCard {
  version: string
  /** False on a computer ComfyUI doesn't run on yet (it is built for Windows x64). */
  bundled: boolean
  /** Whether its program is on this computer; the first start downloads it. */
  installed: boolean
  /** The download's size in bytes. */
  downloadBytes: number
  dataDir: string
  modelsDir: string
  modelsUrl: string
  usedBytes: number
  /** Set when the models run on the processor. */
  notice: string | null
}

/** The host's `toolStatus` for one tool. */
export interface ToolState {
  id: ToolId
  enabled: boolean
  state: ToolRunState
  error: string | null
  port: number
  url: string | null
  log: string
  logTail: string
  comfy?: ComfyCard
  job?: ToolJob | null
  /** Finished jobs, newest first, so the card keeps what was made. */
  jobs?: ToolJob[]
  downloads?: ModelDownload[]
  install?: ToolInstall
}

/** The program's first download (`host/src/tools/comfy_install.rs`). */
export interface ToolInstall {
  phase: 'idle' | 'downloading' | 'unpacking' | 'failed'
  done: number
  total: number
  error: string | null
}

/** Whether the program is arriving right now. */
export function installing(install: ToolInstall | undefined): boolean {
  return install?.phase === 'downloading' || install?.phase === 'unpacking'
}

/** How the download goes, in the caller's language; null when nothing is arriving. */
export function installLine(title: string, install: ToolInstall | undefined, tr: (text: string) => string): string | null {
  if (install?.phase === 'downloading') {
    return tr('Downloading {name}… {done} of {total}')
      .replace('{name}', title)
      .replace('{done}', bytesText(install.done))
      .replace('{total}', bytesText(install.total))
  }
  if (install?.phase === 'unpacking') return tr('Unpacking {name}…').replace('{name}', title)
  return null
}
