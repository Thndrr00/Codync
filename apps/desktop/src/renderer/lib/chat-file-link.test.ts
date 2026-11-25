import assert from 'node:assert/strict'
import { test } from 'node:test'
import { chatFileHref, chatFileLink, file_link_target, fileReferenceName, hash_target, isAbsoluteDocumentPath, isKnownFileReference, isMissingFileCandidate, linkTarget, match_file_link, previewKind } from './chat-file-link.ts'

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

test('only local HTML and PDF get an in-app preview, with a safe fallback for the rest', () => {
  assert.equal(previewKind('/Users/e/report.PDF'), 'pdf')
  assert.equal(previewKind('C:\\Docs\\page.htm'), 'html')
  assert.equal(previewKind('//server/share/My%20Doc.html'), 'html')
  for (const path of ['/Users/e/notes.txt', '/Users/e/picture.png', 'https://example.com/page.html', '/a/b/\nfile.pdf', '']) {
    assert.equal(previewKind(path), null, path)
  }
})

test('a hash link resolves inside the preview document and replaces an old fragment', () => {
  assert.equal(
    hash_target('#/project/scope', 'file:///Users/e/page.html'),
    'file:///Users/e/page.html#/project/scope',
  )
  assert.equal(
    hash_target('#line=26', 'file:///Users/e/page.html#old'),
    'file:///Users/e/page.html#line=26',
  )
  assert.equal(hash_target('#top', 'about:srcdoc'), 'about:srcdoc#top')
})

test('a hash link cannot pull the preview to another origin or a broken target', () => {
  assert.equal(hash_target('https://example.com/#x', 'file:///a/page.html'), null)
  assert.equal(hash_target('#x', ''), null)
  assert.equal(hash_target('#x', 'https://example.com/page.html'), null)
  assert.equal(hash_target('#', 'file:///a/page.html'), null)
  assert.equal(hash_target('#x\nmore', 'file:///a/page.html'), null)
})

test('a written folder chain only matches a known file whose folders all agree', () => {
  const known = ['/work/other-repo/skills/y/SKILL.md', '/work/SKILL.md', 'C:/work/notes/report.md']
  assert.equal(match_file_link('other-repo/skills/y/SKILL.md', known)?.path, '/work/other-repo/skills/y/SKILL.md')
  assert.equal(match_file_link('other-repo\\skills\\y\\SKILL.md', known)?.path, '/work/other-repo/skills/y/SKILL.md')
  assert.equal(match_file_link('/work/other-repo/skills/x/SKILL.md', known), null, 'a different folder is not the same file')
  assert.equal(match_file_link('other-repo/SKILL.md', known), null, 'a shortened chain is not the same file')
  assert.equal(match_file_link('SKILL.md', known)?.path, '/work/SKILL.md', 'a bare name still matches by name')
  assert.equal(chatFileLink('other-repo/skills/x/SKILL.md', known), null, 'the link handler follows the same decision')
  assert.equal(chatFileLink('SKILL.md', known)?.path, '/work/SKILL.md')
  assert.equal(isMissingFileCandidate('other-repo/skills/x/SKILL.md'), true)
  assert.equal(isMissingFileCandidate('https://example.com/a/SKILL.md'), false)
})

test('a file target carries a line and column without losing folders or a drive', () => {
  assert.deepEqual(file_link_target('report.md:26'), { path: 'report.md', line: 26 })
  assert.deepEqual(file_link_target('report.md:26:5'), { path: 'report.md', line: 26, column: 5 })
  assert.deepEqual(file_link_target('other-repo/skills/x/SKILL.md:42'), { path: 'other-repo/skills/x/SKILL.md', line: 42 })
  assert.deepEqual(file_link_target('C:\\My Docs\\report.md:12:3'), { path: 'C:\\My Docs\\report.md', line: 12, column: 3 })
  assert.deepEqual(file_link_target('/Users/e/report.md'), { path: '/Users/e/report.md' })
  assert.deepEqual(file_link_target('  report.md:7  '), { path: 'report.md', line: 7 })
  assert.equal(file_link_target('report.md:not-a-line'), null, 'a malformed position is no target')
  assert.equal(file_link_target('report.md:0'), null, 'lines start at one')
  assert.equal(file_link_target('report.md:99999999'), null, 'an absurd line is refused')
  assert.equal(file_link_target(''), null)
})
