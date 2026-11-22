import { useRef, useState } from 'react'
import { Spinner } from '../../../components/Controls'
import { Icon } from '../../../components/Icon'
import { AnchoredMenu, type MenuItem } from '../../../components/Menu'
import { Dialog, DialogField } from '../../../components/Overlay'
import { HStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import { folderOf, workflowsInFolder, type WorkflowFolderAction, type WorkflowFoldersSnapshot } from '../../../lib/routine-flow.ts'
import { useTr } from '../../../lib/use-language'

interface NameDialog {
  mode: 'create' | 'rename'
  folderId?: string
}

/**
 * The title bar's "All workflows" menu: every workflow of this computer, grouped by its folder,
 * with OpenBot's folder actions. Folders are shared by everyone on the computer (the host owns
 * them); the open tab list stays on this device.
 */
interface WorkflowEntry {
  id: string
  name: string
  botId: string
}

export function AllWorkflowsMenu({ workflows, current, folders, loading, error, onOpen, onNew, onRename, onFolderAction }: {
  workflows: readonly WorkflowEntry[]
  /** The routine id of the open canvas's workflow, which is what the host files in folders. */
  current: string
  folders: WorkflowFoldersSnapshot | null
  loading: boolean
  error: string | null
  onOpen: (workflow: WorkflowEntry) => void
  onNew: () => void
  onRename: (workflow: WorkflowEntry) => void
  onFolderAction: (action: WorkflowFolderAction) => void
}) {
  const tr = useTr()
  const button = useRef<HTMLButtonElement>(null)
  const [open, setOpen] = useState(false)
  const [nameDialog, setNameDialog] = useState<NameDialog | null>(null)
  const [draftName, setDraftName] = useState('')
  const ids = workflows.map((workflow) => workflow.id)
  const entryOf = (id: string) => workflows.find((workflow) => workflow.id === id)
  const nameOf = (id: string) => entryOf(id)?.name ?? id
  const folderName = (id: string) => {
    const folderId = folderOf(folders, id)
    return folderId ? folders?.folders.find((folder) => folder.id === folderId)?.name ?? null : null
  }

  const openNameDialog = (mode: NameDialog['mode'], folderId?: string) => {
    setDraftName(mode === 'rename' ? folders?.folders.find((folder) => folder.id === folderId)?.name ?? '' : '')
    setNameDialog({ mode, folderId })
  }

  const saveName = () => {
    const name = draftName.trim()
    if (name && nameDialog) {
      if (nameDialog.mode === 'create') {
        onFolderAction({ action: 'create', id: crypto.randomUUID(), name, workflowIds: current ? [current] : [] })
      } else if (nameDialog.folderId) {
        onFolderAction({ action: 'rename', folderId: nameDialog.folderId, name })
      }
    }
    setNameDialog(null)
  }

  const workflowItems = (folderId: string | null): MenuItem[] =>
    workflowsInFolder(folders, ids, folderId).map((id) => ({
      title: nameOf(id),
      verbatim: true,
      selected: id === current,
      detail: folderName(id) ?? tr('No folder'),
      action: () => { const entry = entryOf(id); if (entry) onOpen(entry) },
    }))

  const placeItems = (id: string): MenuItem[] => [
    ...(folders?.folders ?? []).map((folder) => ({
      title: folder.name,
      verbatim: true,
      selected: folderOf(folders, id) === folder.id,
      action: () => onFolderAction({ action: 'place', workflowIds: [id], folderId: folder.id }),
    })),
    {
      title: 'No folder',
      selected: folderOf(folders, id) === null,
      action: () => onFolderAction({ action: 'place', workflowIds: [id], folderId: null }),
    },
    { title: 'New folder', icon: 'folder', divider: true, keepsOpen: false, action: () => openNameDialog('create') },
  ]

  const items = (): MenuItem[] => {
    const rows: MenuItem[] = []
    if (error) rows.push({ title: error, verbatim: true, disabled: true, action: () => {} })
    for (const folder of folders?.folders ?? []) {
      const members = workflowItems(folder.id)
      rows.push({
        title: folder.name,
        verbatim: true,
        icon: folderOf(folders, current) === folder.id ? 'folder' : undefined,
        detail: `${members.length}`,
        submenu: [
          ...(members.length ? members : [{ title: 'This folder is empty', disabled: true, action: () => {} }]),
          { title: 'Move workflow here', icon: 'folder', divider: true, disabled: !current, action: () => onFolderAction({ action: 'place', workflowIds: [current], folderId: folder.id }) },
          { title: 'Rename folder', icon: 'pencil', action: () => openNameDialog('rename', folder.id) },
          { title: 'Delete folder', icon: 'trash', destructive: true, action: () => onFolderAction({ action: 'delete', folderId: folder.id }) },
        ],
      })
    }
    rows.push(...workflowItems(null))
    rows.push(
      { title: 'New workflow', icon: 'plus', divider: true, action: onNew },
      { title: 'New folder', icon: 'folder', action: () => openNameDialog('create') },
      { title: 'Move workflow', icon: 'folder', disabled: !current, submenu: placeItems(current) },
      { title: 'Rename workflow', icon: 'pencil', action: () => { const entry = entryOf(current); if (entry) onRename(entry) } },
    )
    return rows
  }

  return (
    <>
      <button
        ref={button}
        type="button"
        className="flow-all-workflows"
        aria-haspopup="menu"
        aria-expanded={open}
        title={tr('All workflows')}
        onClick={() => setOpen(!open)}
      >
        <HStack gap={4}>
          <Text weight="semibold">{tr('All workflows')}</Text>
          {loading ? <Spinner size={12} /> : <Icon name="chevron.down" size={10} weight="semibold" color="var(--secondary)" />}
        </HStack>
      </button>
      <AnchoredMenu open={open} onClose={() => setOpen(false)} anchor={button} items={items} live />
      <Dialog
        open={nameDialog !== null}
        title={tr(nameDialog?.mode === 'rename' ? 'Rename folder' : 'New folder')}
        actions={[{ title: nameDialog?.mode === 'rename' ? 'Rename' : 'Create', disabled: !draftName.trim(), action: saveName }]}
        onClose={() => setNameDialog(null)}
      >
        <DialogField
          value={draftName}
          onChange={setDraftName}
          onSubmit={saveName}
          label={tr('Folder name')}
          placeholder={tr('Folder name')}
        />
      </Dialog>
    </>
  )
}
