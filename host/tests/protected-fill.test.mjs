import {test} from 'node:test'
import assert from 'node:assert/strict'
import {readFileSync} from 'node:fs'
import {createRequire} from 'node:module'
const {JSDOM}=createRequire(new URL('../../apps/desktop/package.json',import.meta.url))('jsdom')
const source=readFileSync(new URL('../src/screen/protected-fill.js',import.meta.url),'utf8')
function fixture(html='<input type="password">',url='https://approved.example/login') {
  const dom=new JSDOM(html,{url,runScripts:'outside-only'})
  const field=dom.window.document.querySelector('input')
  field.getClientRects=()=>[{}]
  field.focus()
  return {dom,field,fill:dom.window.eval(source)}
}
test('the exact approved focused password node can be probed without receiving a secret',()=>{
  const {dom,field,fill}=fixture()
  try {
    const probe=fill.call(field,'https://approved.example','password')
    assert.equal(probe.ready,true);assert.equal(field.value,'')
    let changes=0;field.addEventListener('input',()=>changes++)
    assert.equal(fill.call(field,'https://approved.example','password','fixture-secret'),'filled')
    assert.equal(field.value,'fixture-secret');assert.equal(changes,1)
  } finally {dom.window.close()}
})
test('HTTP, lookalike hosts and a different port cannot receive a password',()=>{
  for(const url of ['http://approved.example/login','https://approved.example.evil/login','https://approved.example:8443/login']) {
    const {dom,field,fill}=fixture(undefined,url)
    try {assert.equal(fill.call(field,'https://approved.example','password').ready,false);assert.equal(fill.call(field,'https://approved.example','password','fixture'),'refused');assert.equal(field.value,'')}
    finally {dom.window.close()}
  }
})
test('a stale focus, read-only, disabled or hidden input cannot be filled',()=>{
  for(const mutate of [field=>field.blur(),field=>field.readOnly=true,field=>field.disabled=true,field=>field.getClientRects=()=>[]]) {
    const {dom,field,fill}=fixture()
    try {mutate(field);assert.equal(fill.call(field,'https://approved.example','password','fixture'),'refused');assert.equal(field.value,'')}
    finally {dom.window.close()}
  }
})
test('a code requires an explicit one-time-code field; ordinary text and foreign frames are refused',()=>{
  for(const [html,expected] of [['<input autocomplete="one-time-code">','filled'],['<input type="text">','refused'],['<input type="email" autocomplete="one-time-code">','refused']]) {
    const {dom,field,fill}=fixture(html)
    try {assert.equal(fill.call(field,'https://approved.example','code','123456'),expected)}finally{dom.window.close()}
  }
  const {dom,field,fill}=fixture('<input type="password"><iframe src="https://other.example"></iframe>')
  try {dom.window.document.querySelector('iframe').focus();assert.equal(fill.call(field,'https://approved.example','password').ready,false)} finally{dom.window.close()}
})
test('numeric code inputs never falsely report successful delivery of an alphanumeric code',()=>{
  const {dom,field,fill}=fixture('<input type="number" autocomplete="one-time-code">')
  try {
    assert.equal(fill.call(field,'https://approved.example','code','AB12'),'refused')
    assert.equal(field.value,'')
    assert.equal(fill.call(field,'https://approved.example','code','1234'),'filled')
    assert.equal(field.value,'1234')
  }finally{dom.window.close()}
})
