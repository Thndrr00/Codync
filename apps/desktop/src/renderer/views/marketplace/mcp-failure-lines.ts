/**
 * A saved MCP failure is shown over as many lines as it needs (OpenBot 0.73.0's wrapped connection
 * error): the kind on its own line, then the message wrapped on words. A long token or URL is broken
 * so it cannot push the connector's buttons off the row.
 */
export interface McpFailure {
  kind: string
  message: string
}

export function mcpFailureLines(failure: McpFailure | null | undefined, width = 56): string[] {
  if (!failure) return []
  const lines = [failure.kind.toUpperCase()]
  const words = failure.message.trim().split(/\s+/).filter(Boolean)
  let line = ''
  for (const word of words) {
    const pieces = word.length > width ? word.match(new RegExp(`.{1,${width}}`, 'g')) ?? [word] : [word]
    for (const piece of pieces) {
      if (!line) line = piece
      else if (`${line} ${piece}`.length <= width) line += ` ${piece}`
      else {
        lines.push(line)
        line = piece
      }
    }
  }
  if (line) lines.push(line)
  return lines
}
