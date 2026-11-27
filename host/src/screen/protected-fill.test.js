const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

// Run with `node --test host/src/screen/protected-fill.test.js`. The fill script is one CDP
// function expression, so the pure selector is extracted between its markers and evaluated alone.
const source = fs.readFileSync(path.join(__dirname, 'protected-fill.js'), 'utf8');
const begin = '// begin password_field';
const end = '// end password_field';
const from = source.indexOf(begin);
const to = source.indexOf(end);
assert.ok(from >= 0 && to > from, 'the pure selector is marked in the fill script');
const password_field = new Function(`return (${source.slice(from + begin.length, to).trim()});`)();
const cb = '// begin code_field';
const ce = '// end code_field';
const cfrom = source.indexOf(cb);
const cto = source.indexOf(ce);
assert.ok(cfrom >= 0 && cto > cfrom, 'the pure code selector is marked in the fill script');
const code_field = new Function(`return (${source.slice(cfrom + cb.length, cto).trim()});`)();

const field = (type, autocomplete, masked) => ({
  type, autocomplete, masked, visible: true, disabled: false, readOnly: false, hidden: false, inert: false,
});

test('a real password input wins over a masked decoy', () => {
  assert.equal(password_field([field('text', 'off', true), field('password', '', false)]), 1);
  assert.equal(password_field([field('password', '', false), field('text', 'off', true)]), 0);
});

test('an explicit current-password field wins, and ties keep the first', () => {
  assert.equal(
    password_field([field('password', '', false), field('password', 'current-password', false)]),
    1,
  );
  assert.equal(
    password_field([field('password', 'current-password', false), field('text', 'off', true)]),
    0,
  );
  assert.equal(
    password_field([field('text', 'off', true), field('password', 'current-password', false)]),
    1,
  );
});

test('an authenticator code field is found by its label or placeholder', () => {
  const otp = { ...field('text', 'off', false), label: 'Verification code', placeholder: '' };
  assert.equal(code_field([field('text', 'off', false), otp]), 1);
  assert.equal(code_field([{ ...field('text', 'one-time-code', false) }]), 0);
  assert.equal(code_field([field('text', 'off', false)]), -1, 'an ordinary text field is not a code field');
  assert.equal(code_field([{ ...otp, hidden: true }]), -1);
});

test('plain text, hidden, disabled and read-only inputs are never chosen', () => {
  assert.equal(password_field([field('text', 'off', false)]), -1);
  assert.equal(password_field([]), -1);
  assert.equal(password_field([{ ...field('password', 'current-password', false), visible: false }]), -1);
  assert.equal(password_field([{ ...field('text', 'off', true), disabled: true }]), -1);
  assert.equal(password_field([{ ...field('text', 'off', true), readOnly: true }]), -1);
  assert.equal(password_field([{ ...field('text', 'off', true), hidden: true }]), -1);
});
