/** Desktop on-device recognition is supplied by the macOS speech helper. */
export function voiceEngine(value: string, platform: string): string {
  return platform !== 'darwin' && value === 'device' ? 'openai' : value
}

/** Every provider a voice settings menu can offer; the call and dictation menus each take their half. */
export function voiceEngineIds(platform: string): string[] {
  return [...(platform === 'darwin' ? ['device'] : []), 'openai', 'gemini', 'elevenlabs', 'xai', 'groq']
}
