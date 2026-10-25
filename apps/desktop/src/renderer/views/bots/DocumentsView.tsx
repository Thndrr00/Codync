import { lazy, Suspense, useEffect, useMemo, useState } from 'react'
import type { Bot } from '@shared/models'
import { Button, CardSection, DropdownMenu, IconButton, TabChoice } from '../../components/Controls'
import { Dialog, ModalHeader, Sheet } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import type { BotStore } from '../../store/bot-store'
import type { DocumentsStore } from '../../store/documents-store'
import { documentTree, documentDescendants } from '../../lib/document-tree'
import { DocumentMarkdownEditor } from './DocumentMarkdownEditor'
import { MarkdownText } from '../thread/Markdown'
import { RepliesView } from '../thread/RepliesView'
import { relativeTime } from '../thread/chat-items'

type Section = 'documents' | 'reviews' | 'activity'
type EditorMode = 'visual' | 'markdown' | 'preview'
type DraftPrompt = { store: DocumentsStore; session: number }
const DocumentVisualEditor = lazy(() => import('./DocumentVisualEditor'))
const SECTIONS: { id: Section; label: string }[] = [
  { id: 'documents', label: 'Documents' }, { id: 'reviews', label: 'Review' }, { id: 'activity', label: 'Activity' },
]

/** One bot's document library, immutable proposed changes, and computer activity. */
export function DocumentsCard({ bot, store }: { bot: Bot; store: BotStore }) {
  const documents = useModel(useMemo(() => store.documentsFor(bot.id), [bot.id, store]))
  const [section, setSection] = useState<Section>('documents')
  const [mode, setMode] = useState<EditorMode>('visual')
  const [confirmDelete, setConfirmDelete] = useState<DraftPrompt | null>(null)
  const [confirmDiscard, setConfirmDiscard] = useState<DraftPrompt | null>(null)
  const [conversation, setConversation] = useState<{ botId: string; documentId: string } | null>(null)
  useEffect(() => {
    documents.resume()
    void documents.load()
    void documents.refreshExtras()
    const refresh = setInterval(() => {
      void documents.load(false, true)
      void documents.refreshExtras()
    }, 5_000)
    return () => { clearInterval(refresh); documents.dispose() }
  }, [documents])
  const draft = documents.draft
  const current = documents.items.find((item) => item.id === draft?.id)
  const descendants = useMemo(() => documentDescendants(documents.items, draft?.id ?? ''), [documents.items, draft?.id])
  return <div className="documents-panel">
    <TabChoice label="Document library" selection={section} options={SECTIONS} onChange={setSection} />
    {documents.error && <p role="alert" className="memory-error">{documents.error}</p>}
    {documents.draftWarning && <p role="alert" className="memory-error">{documents.draftWarning}</p>}
    <Text as="div" size="caption" tone="secondary" className="documents-status" aria-live="polite">{documents.notice}</Text>
    {section === 'documents' ? <>
      <HStack gap={8}>
        <input className="documents-title" aria-label="Search documents" placeholder="Search documents" value={documents.query} onChange={(event) => documents.search(event.target.value)} />
        <IconButton icon="plus" title="New document" disabled={documents.busy} onClick={() => documents.create()} />
        <IconButton icon="arrow.clockwise" title="Refresh documents" disabled={documents.loading} onClick={() => void documents.load()} />
      </HStack>
      <CardSection>
        {documents.pendingDrafts.length > 0 && <VStack gap={8}>
          <Text weight="semibold">Drafts saved on this device</Text>
          {documents.pendingDrafts.map((saved) => <Button key={saved.key} kind="secondary" disabled={documents.busy} onClick={() => void documents.recoverDraft(saved.key)}>Continue {saved.title}</Button>)}
        </VStack>}
        {documents.loading && <Text as="div" tone="secondary">Loading documents…</Text>}
        {!documents.loading && !documents.items.length && <Text as="div" tone="secondary">{documents.query ? 'No matching documents.' : 'No documents yet. Create a document to keep notes and plans with this bot.'}</Text>}
        {documentTree(documents.items, !!documents.query).map(({ page: item, depth }) => <button key={item.id} className="documents-row press" style={{ paddingLeft: Math.min(depth, 8) * 16 }} disabled={documents.busy} onClick={() => void documents.open(item.id)}>
          <VStack gap={2}>
            <Text weight="semibold">{item.title}</Text>
            <Text size="caption" tone="secondary">{item.parentId ? `Inside ${documents.items.find((parent) => parent.id === item.parentId)?.title ?? 'another document'} · ` : ''}Revision {item.revision} · {relativeTime.day(item.updatedAt)}</Text>
          </VStack>
        </button>)}
        {documents.next && <Button kind="secondary" disabled={documents.loading} onClick={() => void documents.load(true)}>Load more</Button>}
      </CardSection>
      {documents.opening ? <Text as="div" tone="secondary">Opening document…</Text> : draft && <section className="documents-editor fade-in" aria-label="Document editor">
        <HStack gap={8} wrap>
          <Text size="headline" grow>{draft.id ? 'Edit document' : 'New document'}</Text>
          {draft.id && <>
            <IconButton icon="bubble.left.and.bubble.right" title="Discuss saved document" disabled={documents.busy || documents.dirty || !!documents.conflict}
              onClick={() => { if (draft.id) setConversation({ botId: bot.id, documentId: draft.id }) }} />
            <IconButton icon="doc.badge.plus" title="Create child document" disabled={documents.busy} onClick={() => documents.create(draft.id)} />
            {!documents.busy && !documents.dirty && <DropdownMenu title="Move document" className="styled-button secondary" items={() => [
              { title: 'Move to library root', action: () => void documents.move(null) },
              ...documents.items.filter((item) => !descendants.has(item.id) && item.id !== current?.parentId).map((item) => ({ title: `Move inside ${item.title}`, action: () => void documents.move(item.id) })),
            ]}>Move</DropdownMenu>}
            <IconButton icon="trash" title="Delete document" disabled={documents.busy || !!documents.conflict} onClick={() => setConfirmDelete({ store: documents, session: documents.editorSession })} />
          </>}
        </HStack>
        {documents.conflict && <VStack gap={8} className="documents-status" role="alert">
          <Text>This document changed elsewhere. Your draft is preserved. Compare the latest version before choosing what to keep.</Text>
          <Text weight="semibold">Latest: {documents.conflict.title}</Text>
          <div className="documents-preview"><MarkdownText text={documents.conflict.body} /></div>
          <HStack gap={8} wrap>
            <Button kind="secondary" disabled={documents.busy} onClick={() => documents.reloadConflict()}>Use latest version</Button>
            <Button disabled={documents.busy || !documents.dirty} onClick={() => void documents.save(true)}>Replace latest with my draft</Button>
          </HStack>
        </VStack>}
        <input className="documents-title" aria-label="Document title" placeholder="Document title" disabled={documents.mutating} value={draft.title} onChange={(event) => documents.edit({ title: event.target.value })} />
        <TabChoice label="Document editing mode" selection={mode} options={[{ id: 'visual', label: 'Visual' }, { id: 'markdown', label: 'Markdown' }, { id: 'preview', label: 'Preview' }]} onChange={setMode} />
        {mode === 'preview' ? <div className="documents-preview"><MarkdownText text={draft.body} /></div>
          : mode === 'visual' ? <Suspense fallback={<p role="status">Loading visual editor…</p>}>
            <DocumentVisualEditor key={`${bot.id}:${documents.editorSession}`} value={draft.body} disabled={documents.mutating} onChange={(body) => documents.edit({ body })} />
          </Suspense> : <DocumentMarkdownEditor key={`${bot.id}:${documents.editorSession}`} value={draft.body} disabled={documents.mutating} onChange={(body) => documents.edit({ body })} />}
        <HStack gap={8}>
          <Text size="caption" tone="secondary" grow aria-live="polite">{documents.busy ? 'Saving…' : documents.conflict ? 'Resolve the conflict to save.' : documents.dirty ? 'Unsaved draft · saves automatically after typing.' : 'All changes saved.'}</Text>
          <Button disabled={documents.busy || !documents.dirty || !draft.title.trim() || !!documents.conflict} onClick={() => void documents.save()}>Save</Button>
          <Button kind="secondary" disabled={documents.busy} onClick={() => documents.saveAsCopy()}>Save as a new document</Button>
          {documents.dirty && <Button kind="secondary" disabled={documents.busy} onClick={() => setConfirmDiscard({ store: documents, session: documents.editorSession })}>Discard draft</Button>}
        </HStack>
      </section>}
    </> : section === 'reviews' ? <CardSection>
      <HStack><Text size="headline" grow>Proposed document changes</Text><IconButton title="Refresh reviews" icon="arrow.clockwise" onClick={() => void documents.refreshExtras()} /></HStack>
      {documents.extrasLoading && <Text as="div" tone="secondary">Loading reviews…</Text>}
      {!documents.extrasLoading && !documents.reviews.length && <Text as="div" tone="secondary">No document changes to review.</Text>}
      {documents.reviews.map((review) => <button key={review.id} className="documents-row press" disabled={documents.busy} onClick={() => void documents.inspectReview(review.id)}>
        <VStack gap={2}><Text weight="semibold">{review.title}</Text><Text size="caption" tone="secondary">{review.status} · {relativeTime.day(review.createdAt)}</Text></VStack>
      </button>)}
    </CardSection> : <CardSection>
      <HStack><Text size="headline" grow>Computer activity</Text><IconButton title="Copy activity CSV" icon="doc.on.doc" onClick={() => void documents.copyActivity()} /><IconButton title="Refresh activity" icon="arrow.clockwise" onClick={() => void documents.refreshExtras()} /></HStack>
      {documents.extrasLoading && <Text as="div" tone="secondary">Loading activity…</Text>}
      {!documents.extrasLoading && !documents.activity.length && <Text as="div" tone="secondary">No computer activity recorded yet.</Text>}
      {documents.activity.map((event) => <VStack key={event.id} gap={2} className="documents-row">
        <Text weight="semibold">{event.action}</Text>
        <Text size="caption" tone={event.result === 'failed' ? 'danger' : 'secondary'}>{event.result} · {event.actorKind}{event.actorId ? ` (${event.actorId})` : ''} · {relativeTime.day(event.createdAt)}</Text>
      </VStack>)}
    </CardSection>}
    <Dialog open={confirmDelete?.store === documents && confirmDelete.session === documents.editorSession} title="Delete this document and its children?" onClose={() => setConfirmDelete(null)} actions={[{ title: 'Delete', destructive: true, action: () => { if (confirmDelete?.store === documents && confirmDelete.session === documents.editorSession) void documents.remove() } }]} />
    <Dialog open={confirmDiscard?.store === documents && confirmDiscard.session === documents.editorSession} title="Discard changes that have not yet been saved?" onClose={() => setConfirmDiscard(null)} actions={[{ title: 'Discard draft', destructive: true, action: () => { if (confirmDiscard?.store === documents && confirmDiscard.session === documents.editorSession) documents.discardDraft() } }]} />
    <Sheet open={conversation?.botId === bot.id} onClose={() => setConversation(null)} width={720} height={680}>
      {conversation?.botId === bot.id && <RepliesView botId={bot.id} rootId={`doc:${conversation.documentId}`} documentTitle={documents.items.find((page) => page.id === conversation.documentId)?.title ?? (draft?.id === conversation.documentId ? draft.title : undefined)} close={() => setConversation(null)} />}
    </Sheet>
    <Sheet open={documents.review !== null} onClose={() => documents.closeReview()} width={640}>
      {documents.review && <VStack gap={12} className="documents-editor">
        <ModalHeader title="Review document change" />
        <Text size="headline">{documents.review.draft.title}</Text>
        <Text size="caption" tone="secondary">{documents.review.status} · Proposed revision {documents.review.draft.ifRevision}</Text>
        <Text size="caption" tone="secondary">{documents.review.draft.id ? `Update document ${documents.review.draft.id}` : `Create document${documents.review.draft.parentId ? ` inside ${documents.review.draft.parentId}` : ' at library root'}`}</Text>
        <div className="documents-preview"><MarkdownText text={documents.review.draft.body} /></div>
        <details><summary>Exact proposed Markdown</summary><pre className="documents-preview">{documents.review.draft.body}</pre></details>
        {documents.reviewCurrent && <><Text weight="semibold">Saved version at inspection · revision {documents.reviewCurrent.revision}</Text><details><summary>Compare saved Markdown</summary><pre className="documents-preview">{documents.reviewCurrent.body}</pre></details></>}
        {documents.review.document && <><Text weight="semibold">Recorded proposal result · revision {documents.review.document.revision}</Text><div className="documents-preview"><MarkdownText text={documents.review.document.body} /></div></>}
        {documents.error && <p role="alert" className="memory-error">{documents.error}</p>}
        {documents.review.status === 'conflict' && <Text tone="danger">The document changed. This proposal was not applied.</Text>}
        {documents.review.status === 'pending' && <HStack gap={8}>
          <Button kind="secondary" disabled={documents.busy} onClick={() => void documents.resolve('decline')}>Decline</Button>
          <Button disabled={documents.busy} onClick={() => void documents.resolve('approve')}>Approve exact proposal</Button>
        </HStack>}
      </VStack>}
    </Sheet>
  </div>
}
