import { createCipheriv, createDecipheriv, createHash, createPublicKey, diffieHellman, generateKeyPairSync, hkdfSync, randomBytes, sign, type KeyObject } from 'node:crypto'

// A phone with no account on the encrypted channel (docs/reference/remote-relay.md §6), for what only a phone can make
// the host do: asking to pair, and then calling the host as the paired device.

const b64 = (bytes: Uint8Array) => Buffer.from(bytes).toString('base64url')
const unb64 = (text: string) => Buffer.from(text, 'base64url')
const rawKey = (key: KeyObject) => unb64(key.export({ format: 'jwk' }).x as string)
const sha256 = (...parts: (string | Uint8Array)[]) => parts.reduce((hash, part) => hash.update(part), createHash('sha256')).digest()

const FINAL = Buffer.from([0x00])
const TAG_LENGTH = 16

function frameParams(counter: number) {
  const be = Buffer.alloc(8)
  be.writeBigUInt64BE(BigInt(counter))
  return { nonce: Buffer.concat([Buffer.alloc(4), be]), aad: Buffer.concat([Buffer.from('codync/frame/v1'), be]) }
}

function sealFrame(key: Uint8Array, counter: number, message: string) {
  const { nonce, aad } = frameParams(counter)
  const plain = Buffer.concat([FINAL, Buffer.from(message)])
  const cipher = createCipheriv('chacha20-poly1305', key, nonce, { authTagLength: TAG_LENGTH })
  cipher.setAAD(aad, { plaintextLength: plain.length })
  return Buffer.concat([cipher.update(plain), cipher.final(), cipher.getAuthTag()])
}

function openFrame(key: Uint8Array, counter: number, sealed: Buffer) {
  const { nonce, aad } = frameParams(counter)
  const text = sealed.subarray(0, sealed.length - TAG_LENGTH)
  const decipher = createDecipheriv('chacha20-poly1305', key, nonce, { authTagLength: TAG_LENGTH })
  decipher.setAAD(aad, { plaintextLength: text.length })
  decipher.setAuthTag(sealed.subarray(sealed.length - TAG_LENGTH))
  return Buffer.concat([decipher.update(text), decipher.final()]).subarray(1).toString()
}

/** The host's answer to a call: `ok` with its result, or `err` with its status and message. */
export type Answer = { ok?: unknown; err?: { status: number; message: string } }

/** A phone the host let in on the channel: it stays open for calls until `close`. */
export type Phone = {
  /** Asks to pair for `code`, under `name`; resolves with the host's reply. */
  pair(code: string, name: string): Promise<Answer>
  /** Calls a host method as this phone; rejects with the host's message when it refuses. */
  call<T>(method: string, body: unknown): Promise<T>
  close(): void
}

/** A device's key pair, fresh: its key signs the handshake and names it to the computer. */
type Device = { publicKey: KeyObject; privateKey: KeyObject }

const newDevice = (): Device => {
  const keys = generateKeyPairSync('ed25519')
  return { publicKey: keys.publicKey, privateKey: keys.privateKey }
}

/** Opens the encrypted channel, handshake done: `pairing` to ask the computer to let a new phone in,
 *  or a phone it already knows, by its key. */
async function openPhone(port: number, computerId: string, pairing: boolean, device = newDevice()): Promise<Phone> {
  const ephemeral = generateKeyPairSync('x25519')
  const cid = unb64(computerId)
  const [dk, ek, n] = [rawKey(device.publicKey), rawKey(ephemeral.publicKey), randomBytes(32)]
  const socket = new WebSocket(`ws://127.0.0.1:${port}/channel?v=1`)
  const pending = new Map<number, (answer: Answer) => void>()
  let sendKey: Buffer
  let receiveKey: Buffer
  let calls = 0
  let frames = 0
  await new Promise<void>((resolve, reject) => {
    socket.onerror = () => reject(new Error('The sandbox host refused the channel.'))
    // A channel that closes under a call leaves nothing waiting forever.
    socket.onclose = () => {
      for (const settle of pending.values()) settle({ err: { status: 0, message: 'The channel closed.' } })
      pending.clear()
    }
    socket.onopen = () => socket.send(JSON.stringify({
      t: 'hello', v: 1, dk: b64(dk), ek: b64(ek), n: b64(n), pair: pairing,
      sig: b64(sign(null, Buffer.concat([Buffer.from('codync/hs1/v1'), cid, dk, ek, n]), device.privateKey)),
    }))
    socket.onmessage = ({ data }) => {
      const message = JSON.parse(String(data)) as { t: string; ek?: string; c?: number; d?: string }
      if (message.t === 'welcome') {
        const hostEk = unb64(message.ek as string)
        const th = sha256('codync/hs2/v1', cid, dk, ek, n, hostEk)
        const shared = diffieHellman({ privateKey: ephemeral.privateKey, publicKey: x25519PublicKey(hostEk) })
        const key = (info: string) => Buffer.from(hkdfSync('sha256', shared, th, info, 32))
        receiveKey = key('codync/h2d/v1')
        sendKey = key('codync/d2h/v1')
        resolve()
      } else if (message.t === 'f') {
        const answer = JSON.parse(openFrame(receiveKey, message.c as number, unb64(message.d as string))) as {
          id?: number; ok?: unknown; err?: Answer['err']
        }
        const settle = answer.id === undefined ? undefined : pending.get(answer.id)
        if (settle && answer.id !== undefined) {
          pending.delete(answer.id)
          settle(answer)
        }
      } else if (message.t === 'reject') {
        reject(new Error(`The sandbox host rejected the phone: ${String(data)}`))
      }
    }
  })
  const ask = (method: string, body: unknown) => new Promise<Answer>((resolve) => {
    const id = ++calls
    pending.set(id, resolve)
    // Frames are numbered from zero, one after the other: the host refuses a skipped counter.
    const frame = frames++
    socket.send(JSON.stringify({ t: 'f', c: frame, d: b64(sealFrame(sendKey, frame, JSON.stringify({ id, m: method, b: body }))) }))
  })
  return {
    async pair(code, name) {
      const answer = await ask('pair', { code, name, platform: 'android' })
      return answer
    },
    async call<T>(method: string, body: unknown) {
      const answer = await ask(method, body)
      if (answer.err) throw new Error(`${answer.err.status}: ${answer.err.message}`)
      return answer.ok as T
    },
    close() {
      socket.close()
    },
  }
}

/** Sends `pair` for a pairing code and resolves with the host's reply: `ok`, or `err` with its status and message. */
export async function pairAsSignedOutPhone(port: number, computerId: string, code: string, name: string) {
  const phone = await openPhone(port, computerId, true)
  try {
    return await phone.pair(code, name)
  } finally {
    phone.close()
  }
}

/** Pairs, and once the host let it in, reopens the channel as that device and keeps it for calls. */
export async function connectPhone(port: number, computerId: string, code: string, name: string) {
  // The pairing channel is closed by the host as soon as it answers (`4100`), so the phone that
  // makes the calls is a second channel on the same key, now a device the computer knows.
  const device = newDevice()
  const asking = await openPhone(port, computerId, true, device)
  try {
    const answer = await asking.pair(code, name)
    // `PHONE_DEBUG=1` prints what the computer answered, instead of the bare failure.
    if (process.env.PHONE_DEBUG) console.log('pair answer', JSON.stringify(answer))
    if (answer.err) throw new Error(`The computer refused ${name}: ${answer.err.message}`)
  } catch (error) {
    asking.close()
    throw error
  }
  asking.close()
  return await openPhone(port, computerId, false, device)
}

function x25519PublicKey(raw: Uint8Array) {
  return createPublicKey({ key: { kty: 'OKP', crv: 'X25519', x: b64(raw) }, format: 'jwk' })
}

