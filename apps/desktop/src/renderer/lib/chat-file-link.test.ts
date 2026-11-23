import assert from 'node:assert/strict'
import { test } from 'node:test'
import { chatFileHref, chatFileLink, fileReferenceName, isAbsoluteDocumentPath, isKnownFileReference, linkTarget } from './chat-file-link.ts'

test('a full path to any document opens as a file link, including docx, pages and rtf', () => {
  for (const path of ['/Users/e/Documents/Informe.docx', 'C:\\Users\\e\\Notes.pages', '/home/e/notes.rtf', '/Users/e/report.pdf']) {
    assert.equal(isAbsoluteDocumentPath(path), true, path)
    assert.deepEqual(chatFileLink(path), { path, href: chatFileHref(path) }, path)
  }
})

test('a name with spaces and dashes is a file, not a command with a flag', () => {
  assert.equal(isAbsoluteDocumentPath('/Users/e/Informe - final.docx'), true)
  assert.equal(isAbsoluteDocumentPath('/Users/e/My Notes.pages'), true)
})

test('a command line, a flag and a name without an extension stay plain code', () => {
  for (const value of ['/usr/bin/python3 -m venv .venv', '/bin/sh -c "echo hi"', 'C:\\bin\\tool.exe -v', '/Users/e/folder/', 'notes', 'C:\\Users\\e\\.hidden']) {
    assert.equal(chatFileLink(value), null, value)
  }
})

test('a bare or network path still needs a known file type', () => {
  assert.equal(isKnownFileReference('report.pdf'), true)
  assert.equal(isKnownFileReference('Notes.pages'), false)
  assert.equal(chatFileLink('report.pdf'), null, 'a bare name has no workspace to resolve')
  assert.deepEqual(chatFileLink('//server/share/report.pdf'), { path: '//server/share/report.pdf', href: chatFileHref('//server/share/report.pdf') })
  assert.equal(chatFileLink('//server/share/Notes.pages'), null)
})

test('a target with spaces is written between angle brackets', () => {
  assert.equal(linkTarget('<C:/My Docs/Informe.docx>'), 'C:/My Docs/Informe.docx')
  assert.equal(linkTarget('<https://example.com/a b>'), 'https://example.com/a b')
  assert.equal(linkTarget('https://example.com/a'), 'https://example.com/a')
  assert.equal(linkTarget('https://example.com/a b'), 'https://example.com/a')
})

test('a file URL escapes spaces and Windows separators', () => {
  assert.equal(chatFileHref('C:\\My Docs\\Informe.docx'), 'file:///C:/My%20Docs/Informe.docx')
  assert.equal(chatFileHref('/home/me/My Notes.rtf'), 'file:///home/me/My%20Notes.rtf')
  assert.equal(chatFileHref('/home/me/a#b.pdf'), 'file:///home/me/a%23b.pdf')
})

test('the file name decodes percent escapes and keeps the last component', () => {
  assert.equal(fileReferenceName('C:\\a\\b\\My%20Report.pdf'), 'My Report.pdf')
  assert.equal(fileReferenceName('/a/b/'), '')
})
