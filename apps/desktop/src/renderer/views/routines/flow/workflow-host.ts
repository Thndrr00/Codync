// The UI half of the portable workflow API (OpenBot `ipc-endpoints.ts` → `workflows:*`). The host
// owns every decision; these are typed, additive calls from this canvas. An older host answers
// "unknown method", so each call tries its names in order and returns null when none answers —
// the canvas then keeps working with the nodes and runs it already has (the store's `firstAnswer`
// pattern, kept here because the store's file is outside this view's scope).

import type {
  WorkflowFields, WorkflowGitHubHistory, WorkflowGraph, WorkflowRunDetail, WorkflowRunPage,
  WorkflowVersion, WorkflowNode, WorkflowDelivery,
} from '../../../lib/routine-flow.ts'

export type WorkflowCall = <T>(method: string, body: object) => Promise<T>

/** The first method that answers with a usable value, or null on an older host. */
async function first<T>(
  call: WorkflowCall,
  methods: readonly string[],
  body: object,
  accept: (answer: unknown) => T | null,
): Promise<T | null> {
  for (const method of methods) {
    try {
      const value = accept(await call<unknown>(method, body))
      if (value !== null) return value
    } catch (error) {
      const text = error instanceof Error ? error.message : String(error)
      if (!/unknown method|404|not found/i.test(text)) throw error
    }
  }
  return null
}

const object = (value: unknown): Record<string, unknown> | null =>
  value && typeof value === 'object' ? value as Record<string, unknown> : null

const array = (value: unknown): unknown[] | null => Array.isArray(value) ? value : null

/** Saves a drawn graph with the revision it was built from; a conflict returns the host's graph. */
export function saveGraph(call: WorkflowCall, workflowId: string, baseRevision: number, graph: WorkflowGraph) {
  return first(call, ['saveWorkflowGraph', 'saveRoutineWorkflowGraph'], { workflowId, baseRevision, graph }, (answer) => {
    const value = object(answer)
    return value && typeof value.ok === 'boolean'
      ? { ok: value.ok as boolean, revision: Number(value.revision ?? 0), conflict: value.conflict ?? null }
      : null
  })
}

/** The up-to-50 autosave, restore and run versions of a workflow. */
export function versions(call: WorkflowCall, workflowId: string) {
  return first(call, ['workflowVersions', 'workflowVersionList'], { workflowId }, (answer) =>
    array(answer)?.filter((item): item is WorkflowVersion => {
      const version = object(item)
      return !!version && typeof version.id === 'string' && typeof version.at === 'string'
    }) ?? null)
}

/** The graph a version held, shown beside the current one before a restore. */
export function versionGraph(call: WorkflowCall, workflowId: string, versionId: string) {
  return first(call, ['workflowVersionGraph', 'workflowGraphVersion'], { workflowId, versionId }, (answer) => {
    const value = object(answer)
    return value && Array.isArray(value.nodes) && Array.isArray(value.edges) ? value as unknown as WorkflowGraph : null
  })
}

export function restoreVersion(call: WorkflowCall, workflowId: string, versionId: string) {
  return first(call, ['restoreWorkflowVersion', 'workflowRestoreVersion'], { workflowId, versionId }, (answer) => object(answer))
}

/** One numbered page of runs, oldest row last; `before` is the previous page's cursor. */
export function runs(call: WorkflowCall, workflowId: string, before: string | null, limit: number) {
  return first(call, ['workflowRuns', 'workflowRunHistory'], { workflowId, before: before ?? undefined, limit }, (answer) => {
    const value = object(answer)
    if (!value || !Array.isArray(value.runs)) return null
    return value as unknown as WorkflowRunPage
  })
}

export function run(call: WorkflowCall, runId: string) {
  return first(call, ['workflowRun', 'workflowRunDetail'], { runId }, (answer) => {
    const value = object(answer)
    return value && object(value.run) ? value as unknown as WorkflowRunDetail : null
  })
}

/** Retries one failed attempt with a fresh request id on the host. */
export function retryStep(call: WorkflowCall, stepId: string, requestId: string) {
  return first(call, ['retryWorkflowStep', 'workflowRetryStep'], { stepId, requestId }, (answer) => object(answer))
}

/** The human's Approve / Reject on a waiting approval step. */
export function decide(call: WorkflowCall, stepId: string, approved: boolean) {
  return first(call, ['decideWorkflowStep', 'workflowDecideStep'], { stepId, approved }, (answer) => answer !== undefined ? true : null)
}

/** Stops one step, or every step of one run, by id. */
export function stopStep(call: WorkflowCall, stepId: string) {
  return first(call, ['stopWorkflowStep', 'workflowStopStep'], { stepId }, () => true)
}

export function stopRun(call: WorkflowCall, runId: string) {
  return first(call, ['stopWorkflowRun', 'workflowStopRun'], { runId }, () => true)
}

/** A workflow webhook's URL and a freshly rotated secret. */
export function rotateWebhookSecret(call: WorkflowCall, workflowId: string, nodeId: string) {
  return first(call, ['rotateWorkflowWebhookSecret', 'workflowRotateWebhookSecret'], { workflowId, nodeId }, (answer) => {
    const value = object(answer)
    return typeof value?.secret === 'string' ? value.secret as string : null
  })
}

export function githubHistory(call: WorkflowCall, nodeId: string) {
  return first(call, ['workflowGitHubHistory', 'workflowGithubHistory'], { nodeId }, (answer) => {
    const value = object(answer)
    return value && Array.isArray(value.deliveries) ? value as unknown as WorkflowGitHubHistory : null
  })
}

/** Starts a run from the newest delivery that was skipped, so a filter change can be tested. */
export function runLastEvent(call: WorkflowCall, workflowId: string, nodeId: string, requestId: string) {
  return first(call, ['runWorkflowWithLastEvent', 'workflowGitHubRunLastEvent'], { workflowId, nodeId, requestId }, (answer) => {
    const value = object(answer)
    return value && object(value.run) ? value as unknown as WorkflowRunDetail : null
  })
}

/** The catalog and per-repository labels/people a GitHub trigger node offers. */
export function githubCatalog(call: WorkflowCall) {
  return first(call, ['workflowGitHubCatalog', 'workflowGithubCatalog'], {}, (answer) => object(answer))
}

export function githubOptions(call: WorkflowCall, repository: string) {
  return first(call, ['workflowGitHubOptions', 'workflowGithubOptions'], { repository }, (answer) => object(answer))
}

/** The fields the host saved with one step, for the history sheet's "Settings used" row. */
export function stepSettings(call: WorkflowCall, stepId: string) {
  return first(call, ['workflowStepSettings', 'workflowStepSettingsAnswer'], { stepId }, (answer) => {
    const value = object(answer)
    return value && 'settings' in value ? value : null
  })
}

/** The reason a delivery was skipped, as one short line under it. */
export const skipReasonOf = (delivery: WorkflowDelivery): string => delivery.reason ?? delivery.outcome

/** A node's saved fields, with one changed value (an immutable copy for the graph being saved). */
export function withField(node: WorkflowNode, name: string, value: WorkflowFields[string]): WorkflowNode {
  return { ...node, fields: { ...node.fields, [name]: value } }
}
