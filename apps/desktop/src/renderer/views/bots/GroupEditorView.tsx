import { useEffect, useMemo, useRef, useState } from 'react'
import { isGroup, type Bot } from '@shared/models'
import { BotAvatar, GroupAvatar } from '../../components/Avatar'
import { Button, ChoicePicker, IconButton, Spinner, Switch, ValueRow } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { Dialog, ModalHeader, SheetPage, useDismiss } from '../../components/Overlay'
import { VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useStore } from '../../store/context'
import { useModel } from '../../lib/observable'
import { MemoryStore } from '../../store/memory-store'
import { EntryRow } from './MemoryView'
import { errorText } from './drafts'
import { HistoryActions } from './HistoryActions'
import { AutoTextArea } from './parts'
import './bots.css'
import { useTr } from '../../lib/use-language'


/**
 * Change a group chat's name, what it's for and who's in it. Bots are added from a
 * search of the ones not in it yet. New groups start in `NewGroupView`.
 */
export function GroupEditorView({ group }: { group: Bot }) {
  const tr = useTr()
  const store = useStore()
  const dismiss = useDismiss()
  const [name, setName] = useState(group.name)
  const [title, setTitle] = useState(group.title ?? '')
  const [about, setAbout] = useState(group.description)
  const [members, setMembers] = useState<string[]>(group.members)
  const [chiefBotId, setChiefBotId] = useState<string | null>(group.chiefBotId ?? null)
  const [archived, setArchived] = useState(group.archived)
  const [keepMemories, setKeepMemories] = useState(group.keepMemories)
  const [photo, setPhoto] = useState<string | null>(group.avatarPhoto ?? null)
  const [photoError, setPhotoError] = useState<string | null>(null)
  const photoInput = useRef<HTMLInputElement>(null)
  const [query, setQuery] = useState('')
  const [adding, setAdding] = useState(false)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [confirmDelete, setConfirmDelete] = useState<Bot | null>(null)

  const picked = members.map((id) => store.bots.get(id)).filter((b): b is Bot => !!b)
  // Bots that can still join, filtered by the search.
  const q = query.trim().toLowerCase()
  const candidates = store.roster.filter((b) => !isGroup(b) && !members.includes(b.id) && (!q || b.name.toLowerCase().includes(q)))
  const canSave = members.length > 0 && !saving
  // "Alice, Bob" when no name is typed.
  const defaultName = picked.length ? picked.map((b) => b.name).join(', ') : 'Name'
  const current = store.bots.get(group.id)
  const memory = useModel(useMemo(() => new MemoryStore(group.id, () => store.ready()), [group.id, store]))
  // A memory changes anywhere moves the bot's `memoryRev`: read the channel's again.
  useEffect(() => { void memory.load() }, [memory, current?.memoryRev])
  const channel = memory.view?.channel

  const toggle = (id: string) => {
    setMembers((list) => (list.includes(id) ? list.filter((m) => m !== id) : [...list, id]))
    setQuery('')
  }

  const save = () => {
    setSaving(true)
    setError(null)
    store
      .updateGroup({ id: group.id, kind: 'group', name: name.trim() || defaultName, description: about.trim(), members, pinned: store.bots.get(group.id)?.pinned, chiefBotId,
        title: title.trim() || null, archived, keepMemories, avatarPhoto: photo })
      .then(dismiss)
      .catch((e: unknown) => setError(errorText(e)))
      .finally(() => setSaving(false))
  }

  /** The picture is a small data URL, as a bot's photo is: the same 300 KB. */
  const choosePhoto = (file: File | undefined) => {
    if (!file) return
    if (file.size > 300 * 1024) {
      setPhotoError(tr('The picture must be 300 KB or smaller.'))
      return
    }
    const reader = new FileReader()
    reader.onload = () => {
      setPhotoError(null)
      setPhoto(typeof reader.result === 'string' ? reader.result : null)
    }
    reader.onerror = () => setPhotoError(tr('The picture could not be read.'))
    reader.readAsDataURL(file)
  }

  return (
    <SheetPage>
      <ModalHeader icon="person.2" title={tr(`Group chat`)} trailing={saving ? <Spinner /> : <IconButton title="Save" icon="checkmark" disabled={!canSave} onClick={save} />} />
      <div className="group-scroll">
        <div className="group-form">
          <div className="group-identity">
            <GroupAvatar members={picked} size={84} photo={photo} />
            <input className="group-name" value={name} placeholder={defaultName} aria-label="Name" onChange={(e) => setName(e.target.value)} />
            <input className="group-about" value={title} placeholder={tr('A short line under the name')} aria-label="Title" onChange={(e) => setTitle(e.target.value)} />
            <AutoTextArea className="group-about" value={about} placeholder="What this group works on" minRows={1} maxRows={4} onChange={setAbout} />
          </div>
          <VStack>
            <ValueRow label={tr('Picture')} detail={tr("Shown in the roster instead of the members' faces.")}>
              <Button kind="secondary" onClick={() => photoInput.current?.click()}>{tr('Choose Picture')}</Button>
              {photo ? <Button kind="secondary" onClick={() => setPhoto(null)}>{tr('Remove Picture')}</Button> : null}
              <input ref={photoInput} type="file" accept="image/png,image/jpeg,image/webp" hidden
                onChange={(e) => { choosePhoto(e.target.files?.[0]); e.target.value = '' }} />
              {photoError ? <Text size="caption" tone="danger">{photoError}</Text> : null}
            </ValueRow>
            <ValueRow label={tr('Archived')} detail={tr('Archived groups leave the open roster; their conversation stays.')}>
              <Switch on={archived} onChange={setArchived} label={tr('Archived')} />
            </ValueRow>
            <ValueRow label={tr('Keep memories in this channel')} detail={tr("What its members learn here stays in this channel; their other chats don't get it.")}>
              <Switch on={keepMemories} onChange={setKeepMemories} label={tr('Keep memories in this channel')} />
            </ValueRow>
            <div className="group-section-title">{tr('Memory')}</div>
            <Text size="caption" tone="tertiary">{tr('Every member reads these here. To change one, ask a member to remember or forget it.')}</Text>
            {channel?.entries.length
              ? channel.entries.map((entry) => <EntryRow key={entry.id} entry={entry} />)
              : <Text size="caption" tone="tertiary">{tr('Nothing yet. What the channel learns shows up here.')}</Text>}
            <div className="group-section-title">Members</div>
            {picked.map((bot) => (
              <div key={bot.id} className="member-row fade-in">
                <BotAvatar bot={bot} size={32} animated={false} />
                <span className="member-name">{bot.name}</span>
                <IconButton title={tr('Remove {name}').replaceAll('{name}', () => bot.name)} icon="xmark" onClick={() => toggle(bot.id)} />
              </div>
            ))}
            {adding ? (
              <>
                <input
                  className="field-box fade-in group-search"
                  autoFocus
                  value={query}
                  placeholder="Search bots"
                  onChange={(e) => setQuery(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && !e.nativeEvent.isComposing && candidates[0]) toggle(candidates[0].id)
                    if (e.key === 'Escape') setAdding(false)
                  }}
                />
                {candidates.map((bot) => (
                  <button key={bot.id} className="member-row press fade-in" aria-label={tr('Add {name}').replaceAll('{name}', () => bot.name)} onClick={() => toggle(bot.id)}>
                    <BotAvatar bot={bot} size={32} animated={false} />
                    <Text tone="secondary" className="member-name">{bot.name}</Text>
                    <Icon name="plus.circle" size={18} color="var(--secondary)" />
                  </button>
                ))}
              </>
            ) : candidates.length ? (
              <button className="member-row press" onClick={() => setAdding(true)}>
                <span className="member-add-icon">
                  <Icon name="plus" size={16} color="var(--secondary)" />
                </span>
                <Text tone="secondary" className="member-name">{tr(`Add Member`)}</Text>
              </button>
            ) : null}
            <Text as="div" size="caption" tone="tertiary" className="group-note">{tr(`Everyone answers in turn unless you @mention someone.`)}</Text>
            {picked.length > 0 ? (
              <ValueRow
                label="Chief of Staff"
                detail="Coordinates delegated assignments and automatically intervenes if a member stalls or fails."
              >
                <ChoicePicker
                  selection={chiefBotId ?? ''}
                  options={[
                    { id: '', label: 'None (auto-detect Chief)' },
                    ...picked.map((b) => ({ id: b.id, label: b.isChief ? `${b.name} ★` : b.name })),
                  ]}
                  onChange={(id) => setChiefBotId(id || null)}
                />
              </ValueRow>
            ) : null}
          </VStack>
          {error ? <Text size="footnote" tone="danger" className="fade-in">{error}</Text> : null}
          {current ? (
            <HistoryActions bot={current} />
          ) : null}
          {current ? (
            <button className="press group-delete" onClick={() => setConfirmDelete(current)}>
              Delete group chat
            </button>
          ) : null}
        </div>
      </div>
      <DeleteBotDialog bot={confirmDelete} onClose={() => setConfirmDelete(null)} onDeleted={dismiss} />
    </SheetPage>
  )
}

/** The one "Delete <bot>?" confirmation (kit `deleteBotConfirmation`). */
function DeleteBotDialog({ bot, onClose, onDeleted }: { bot: Bot | null; onClose: () => void; onDeleted: () => void }) {
  const store = useStore()
  const tr = useTr()
  return (
    <Dialog
      open={bot !== null}
      title={`Delete ${bot?.name ?? 'bot'}?`}
      message={tr(bot && isGroup(bot) ? 'Its bots and their own chats stay.' : 'Files it changed on your computer stay as they are.')}
      actions={[
        {
          title: 'Delete bot and its conversation',
          destructive: true,
          action: () => {
            if (bot) store.delete(bot)
            onDeleted()
          },
        },
      ]}
      onClose={onClose}
    />
  )
}
