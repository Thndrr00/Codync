import { useEffect, useState } from 'react'
import { Button } from '../components/Controls'
import { ModalHeader, Sheet } from '../components/Overlay'
import { HStack, VStack } from '../components/Stack'
import { Text } from '../components/Text'
import { Field } from './marketplace/pieces'
import type { BotStore } from '../store/bot-store'

/**
 * Turns one message into a skill for its bot: the message becomes the instructions, with a name and
 * a line saying when to use it. The skill is installed for that bot only; other bots keep it off
 * until the person turns it on.
 */
export function SaveSkillSheet({ open, botId, text, store, onClose }: {
  open: boolean
  botId: string
  text: string
  store: BotStore | null
  onClose: () => void
}) {
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [instructions, setInstructions] = useState('')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!open) return
    const first = (text.split('\n')[0] ?? '').trim().slice(0, 80)
    setName(first)
    setDescription('')
    setInstructions(text)
    setError(null)
  }, [open, text])

  const save = async () => {
    if (!store || !name.trim() || !instructions.trim()) return
    setSaving(true)
    setError(null)
    try {
      await store.saveMessageAsSkill(botId, name.trim(), description.trim(), instructions.trim())
      onClose()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setSaving(false)
    }
  }

  return (
    <Sheet open={open} onClose={onClose} width={520} maxHeight="82vh">
      <VStack gap={12}>
        <ModalHeader icon="book.pages" title="Save as a skill" subtitle="This bot reads it next time it needs to do this; other bots keep it off." onClose={onClose} />
        <VStack gap={4}>
          <Text size="caption" tone="secondary">Name</Text>
          <Field label="Name" value={name} onChange={setName} placeholder="What the skill does" />
        </VStack>
        <VStack gap={4}>
          <Text size="caption" tone="secondary">When to use it</Text>
          <Field label="Description" value={description} onChange={setDescription} placeholder="One sentence: when the bot should read this skill" />
        </VStack>
        <VStack gap={4}>
          <Text size="caption" tone="secondary">Instructions</Text>
          <Field label="Instructions" value={instructions} onChange={setInstructions} lines={[6, 14]} />
        </VStack>
        {error ? <Text size="caption" tone="danger">{error}</Text> : null}
        <HStack justify="end" gap={8}>
          <Button kind="secondary" onClick={onClose}>Cancel</Button>
          <Button disabled={saving || !store || !name.trim() || !instructions.trim()} onClick={() => void save()}>
            {saving ? 'Saving…' : 'Save skill'}
          </Button>
        </HStack>
      </VStack>
    </Sheet>
  )
}
