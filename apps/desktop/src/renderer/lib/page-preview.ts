/**
 * The chat's visual page preview is deliberately inert: the iframe has no `allow-scripts`, and this
 * helper also takes scripts, inline handlers, embedded frames and outside resources out of the HTML.
 * It is a pure string transform so the preview can be tested without a browser.
 */

const SCRIPT = /<script\b[^>]*>[\s\S]*?<\/script\s*>/gi
const EMBED = /<\/?\s*(iframe|object|embed|applet|link|base|form)\b[^>]*>/gi
const HANDLER = /\son[a-z]+\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]+)/gi
const RESOURCE = /\s(?:src|srcset|href|poster|data|xlink:href)\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]+)/gi
const REFRESH = /<meta\b[^>]*http-equiv\s*=\s*["']?refresh["']?[^>]*>/gi
const IMPORT = /@import[^;]*;/gi
const NETWORK_URL = /url\(\s*(["']?)(?:https?:)?\/\/[^)]*?\1\s*\)/gi

/** HTML safe to put in a script-free `srcDoc`: no scripts, no handlers, no network fetches. */
export function safePageHtml(html: string): string {
  return html
    .replace(SCRIPT, '')
    .replace(EMBED, '')
    .replace(REFRESH, '')
    .replace(HANDLER, '')
    // Fragment links still work inside the page; anything that loads bytes from outside does not.
    .replace(RESOURCE, (attribute) => (/=\s*["']?#/.test(attribute) ? attribute : ''))
    .replace(IMPORT, '')
    .replace(NETWORK_URL, 'url()')
}
