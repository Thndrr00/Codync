import assert from 'node:assert/strict'
import test from 'node:test'
import { HEADER_NAME_MESSAGE, invalidHeaderKey, validHeaderName } from './header-names.ts'

test('a header name is an HTTP token, not a phrase with spaces or symbols', () => {
  for (const good of ['Authorization', 'X-Api-Key', 'X_API_KEY', 'X.Api.Key', 'X~Key', 'X1']) {
    assert.equal(validHeaderName(good), true, `${good} is usable`)
  }
  for (const bad of ['Authorization Bearer', 'X:Key', 'X/Key', 'X(Key)', 'X@Key', '']) {
    assert.equal(validHeaderName(bad), false, `${bad} is not usable`)
  }
  assert.match(HEADER_NAME_MESSAGE, /Authorization/)
  assert.match(HEADER_NAME_MESSAGE, /Value/)
})

test('the form finds the bad key in a Name: value box and accepts a good one', () => {
  assert.equal(invalidHeaderKey('Authorization: Bearer sk-1\nX-Api-Key: k'), null)
  assert.equal(invalidHeaderKey('\n\n'), null, 'blank rows are not keys')
  assert.equal(invalidHeaderKey('Authorization Bearer: sk-1'), 'Authorization Bearer')
  assert.equal(invalidHeaderKey('Authorization: Bearer sk-1\nX Key: k'), 'X Key')
})
