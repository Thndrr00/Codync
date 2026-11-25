/**
 * Why a saved MCP server is tied to one bot's working directory: the host starts a local command in
 * the bot's own folder, so a relative path resolves there and nowhere else (OpenBot plan 003).
 * Shared by the MCP form and the tool-policy panel so they say the same thing. Pure and tested.
 */
export function limit_note(server: string): string {
  const name = server.trim()
  return name
    ? `${name} runs in each bot's working directory, so a relative path works only where that file exists.`
    : 'A saved server runs in the working directory of each bot that uses it, so a relative path works only where that file exists.'
}
