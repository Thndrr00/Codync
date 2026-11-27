import test from 'node:test'
import assert from 'node:assert/strict'
import { pdf_save_name } from './pdf-save-name.ts'

test('a PDF saves under its chat name, not a random id', () => {
  assert.equal(pdf_save_name({ name: 'Quarterly report.pdf', path: '/tmp/b6ce5234.pdf' }, 'b6ce5234'), 'Quarterly report.pdf')
  assert.equal(pdf_save_name({ path: '/tmp/report.pdf' }, 'b6ce5234'), 'report.pdf')
  assert.equal(pdf_save_name({}, 'x'), 'x')
  assert.equal(pdf_save_name({}, ''), 'document.pdf')
})
