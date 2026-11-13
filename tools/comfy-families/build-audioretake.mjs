// Rebuilds `h3-audio-retake` from the lip preset its studio serves. Run with
// `node tools/comfy-families/build-audioretake.mjs`.
import { readFile, writeFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'

const raw = fileURLToPath(new URL('./raw/', import.meta.url))
const out = fileURLToPath(new URL('../../host/assets/comfy-templates/', import.meta.url))
const models = JSON.parse(await readFile(new URL('./models.json', import.meta.url), 'utf8'))
const graph = JSON.parse(await readFile(`${raw}/h3ar.lip_preset.json`, 'utf8'))

graph['80'].inputs.video = '{{video}}'
graph['83'].inputs.audio = '{{audio}}'
graph['85'].inputs.image = '{{face}}'
graph['11'].inputs.prompt = '{{prompt}}'
graph['13'].inputs.noise_seed = '{{seed}}'

const H3 = 'MiniMax H3 Community License'

/** An entry the graph names, borrowed from the resolved inventory (flat name, its own folder). */
function model(key, folder, license, file) {
  const entry = models[key]
  return {
    file: file ?? entry.path.split('/').pop(),
    folder,
    size: entry.size,
    sha256: entry.sha256,
    url: `https://huggingface.co/${entry.repo}/resolve/main/${entry.path}`,
    license,
  }
}

const manifest = {
  id: 'h3-audio-retake',
  name: 'H3 Audio Retake: redo a speaking face from new audio',
  version: '1.0.0',
  description:
    'Keeps a shot as it is and regenerates only the mouth: the face is tracked, cropped and rebuilt to follow the new audio, then stitched back into the frame. Imported from H3 Audio Retake Studio (MIT); its lip repair ships inside the studio’s web UI.',
  tags: ['video', 'audio', 'audio-retake', 'lip-sync', 'higgsfield', 'needs-nodes'],
  license: `H3 Audio Retake Studio is MIT (Nebuluss) and needs its pack. Models: ${H3} and the Pika Flex LoRA, an H3 derivative on the same licence. Its face detector downloads itself on first use.`,
  inputs: [
    { key: 'video', label: 'The shot to retake', type: 'video', required: true, help: 'One speaking face, 24 fps.' },
    { key: 'audio', label: 'The new sound', type: 'audio', required: true, help: 'The voice the mouth has to follow.' },
    { key: 'face', label: 'Reference frame of the face', type: 'image', required: true },
    {
      key: 'prompt',
      label: 'What the shot shows',
      type: 'text',
      default: 'A speaker talks naturally, synchronized to the provided audio.',
    },
    { key: 'seed', label: 'Seed', type: 'seed' },
  ],
  models: [
    model('h3-unet', 'diffusion_models', H3, 'Minimax-h3_Singularity_ref2va_v2_int8.safetensors'),
    model('h3-te', 'text_encoders', H3),
    model('h3-vae', 'vae', H3),
    model('h3-avae', 'vae', H3),
    model('pika-lora', 'loras', H3),
  ],
  output: { type: 'video', mime: 'video/mp4' },
  estimate: { seconds: 300, vram_gb: 24, output_gb: 0.3 },
  workflow: graph,
}

await writeFile(`${out}/h3-audio-retake.json`, `${JSON.stringify(manifest, null, 2)}\n`)
console.log('ok')

