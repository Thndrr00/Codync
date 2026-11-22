/**
 * Which copy of a skill the details sheet reads. An installed skill is the one the agent uses:
 * its own edited `SKILL.md`, not the newest marketplace text. Only a skill that isn't installed
 * falls back to the catalog copy.
 */

export interface SkillChoice {
  source?: string
  id?: string
  installed?: boolean
}

export interface InstalledSkillRef {
  id: string
  source: string
}

/** `skillDetails` target: `{ id }` reads the installed file, `{ source }` reads the catalog. */
export function skillDetailsTarget(skill: SkillChoice, installed: readonly InstalledSkillRef[]): { id: string } | { source: string } {
  if (skill.source === undefined) return { id: skill.id ?? '' }
  const installedId = installed.find((item) => item.source === skill.source)?.id
    ?? (skill.installed ? skill.source.toLowerCase() : null)
  return installedId ? { id: installedId } : { source: skill.source }
}
