import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { test } from 'node:test'
import { JSDOM } from 'jsdom'

test('the island entry loads the styles that keep its usage meter, pager and cards in bounds', async () => {
  const entry = new URL('../views/island/IslandWindow.tsx', import.meta.url)
  const source = await readFile(entry, 'utf8')
  const sheets = [...source.matchAll(/import ['"]([^'"]+\.css)['"]/g)]
  const styles = await Promise.all(sheets.map((match) => readFile(new URL(match[1], entry), 'utf8')))
  const dom = new JSDOM(`<style>${styles.join('\n')}</style>
    <div class="island"><div class="island-meter"></div>
      <div class="island-usage-pages"><div class="island-usage-navigation"></div>
        <div class="island-row-cards"><div class="island-card island-usage">
          <div class="island-usage-windows"><div class="island-usage-row"></div></div>
        </div></div>
      </div>
    </div>`)
  try {
    const css = (selector: string) => dom.window.getComputedStyle(dom.window.document.querySelector(selector)!)
    assert.equal(css('.island-meter').display, 'flex', 'header meter stays on one line')
    assert.equal(css('.island-usage-navigation').display, 'flex', 'pager stays on one line')
    assert.equal(css('.island-usage-pages').flexDirection, 'column')
    assert.equal(css('.island-usage-pages > .island-row-cards').height, 'auto', 'cards use remaining height below pager')
    assert.equal(css('.island-usage-windows').overflowY, 'auto', 'extra windows remain reachable')
    assert.equal(css('.island-usage-row').display, 'grid', 'labels, numbers and resets have separate tracks')
  } finally {
    dom.window.close()
  }
})
