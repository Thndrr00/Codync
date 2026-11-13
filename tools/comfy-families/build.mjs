// Rebuilds the ten Higgsfield manifests of this pass from the authors' own graphs, the way they
// published them: nine are already in the API format and are copied with their placeholders set,
// the three editor-only ones go through `tools/comfy-ui-to-api.mjs`. `models.json` is the resolved
// inventory (repository, path, size, hash, licence) and `raw/` the graphs as published. Run with
// `node tools/comfy-families/build.mjs`; it writes into `host/assets/comfy-templates/`, then
// `node tools/check-comfy-manifests.mjs` checks the result.
import { readFile, writeFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import { uiToApi } from '../comfy-ui-to-api.mjs'

const raw = fileURLToPath(new URL('./raw/', import.meta.url))
const out = fileURLToPath(new URL('../../host/assets/comfy-templates/', import.meta.url))
const models = JSON.parse(await readFile(new URL('./models.json', import.meta.url), 'utf8'))

const QWEN = 'Qwen-Image 2.1 Research License (non-commercial)'
const H3 = 'MiniMax H3 Community License'
const SAM = 'SAM License (Meta)'

function model(key, folder, license, { gated = false, file } = {}) {
  const entry = models[key]
  if (!entry || entry.missing) throw new Error(`model ${key} unresolved`)
  return {
    file: file ?? entry.path.split('/').pop(),
    folder,
    size: entry.size,
    ...(entry.sha256 ? { sha256: entry.sha256 } : {}),
    url: gated ? `https://huggingface.co/${entry.repo}` : `https://huggingface.co/${entry.repo}/resolve/main/${entry.path}`,
    ...(gated ? { gated: true } : {}),
    license,
  }
}

const base = (id, name, description, tags, license, inputs, list, output, estimate) => ({
  id,
  name,
  version: '1.0.0',
  description,
  tags: [...tags, 'higgsfield'],
  license,
  inputs,
  models: list,
  output,
  estimate,
  workflow: {},
})

const image = (key, label, required = true, help) => ({ key, label, type: 'image', required, ...(help ? { help } : {}) })
const video = (key, label, help) => ({ key, label, type: 'video', required: true, ...(help ? { help } : {}) })
const text = (key, label, required = true, help) => ({ key, label, type: 'text', required, ...(help ? { help } : {}) })
const number = (key, label, def, min, max, step = 1) => ({ key, label, type: 'integer', default: def, min, max, step })
const seed = (key = 'seed') => ({ key, label: 'Seed', type: 'seed' })

const manifests = []

/** An API graph the author published, without the notes some packs keep beside the nodes. */
async function loadApi(file) {
  const graph = JSON.parse(await readFile(`${raw}/${file}`, 'utf8'))
  for (const [id, value] of Object.entries(graph)) if (!value.class_type) delete graph[id]
  return graph
}

const set = (graph, id, field, value) => {
  const node = graph[id]
  if (!node) throw new Error(`no node ${id}`)
  node.inputs[field] = value
}
const drop = (graph, id) => delete graph[id]
const pointAt = (graph, id, field, from) => set(graph, id, field, from)

// 1. Blocksfield: pitch to concept art, on Qwen-Image 2.1 turbo.
{
  const workflow = await loadApi('g.blocksfield.json')
  set(workflow, '1', 'unet_name', 'qwen_image_2.1_turbo_int8_convrot.safetensors')
  const m = base(
    'blocksfield-art',
    'Blocksfield: game concept art',
    'The concept-art step of Hcoder10’s Blocksfield: a pitch becomes painted game art, props and icons. Imported from Blocksfield (MIT) with its own Qwen-Image 2.1 graph.',
    ['image', 'blocksfield', 'concept-art'],
    `Blocksfield is MIT (Hcoder10). Models: ${QWEN}, which the cards mark as non-commercial.`,
    [
      text('prompt', 'The game pitch'),
      text('negative', 'What to avoid', false),
      number('width', 'Width', 1024, 256, 2048, 16),
      number('height', 'Height', 1024, 256, 2048, 16),
      seed(),
      { key: 'sigmas', label: 'Steps (sigma list)', type: 'text', default: '1.0, 0.909091, 0.727273, 0.0' },
    ],
    [model('qwen-turbo', 'diffusion_models', QWEN), model('qwen-te', 'text_encoders', QWEN), model('qwen-vae', 'vae', QWEN)],
    { type: 'image', mime: 'image/png' },
    { seconds: 12, vram_gb: 12, output_gb: 0.01 },
  )
  m.workflow = workflow
  manifests.push(m)
}

// 2. ComfyArt: an edit of one picture, on Qwen-Image 2.1.
{
  const workflow = await loadApi('g.comfyart.json')
  set(workflow, '11', 'image', '{{image}}')
  set(workflow, '6', 'prompt', '{{prompt}}')
  set(workflow, '6', 'negative_prompt', '{{negative}}')
  set(workflow, '7', 'seed', '{{seed}}')
  const m = base(
    'comfyart-edit',
    'ComfyArt: edit a picture',
    'ComfyArt’s quick edit: describe the change and the picture is rebuilt with it. Imported from ComfyArt (MIT) with its own Qwen-Image 2.1 graph.',
    ['image', 'comfyart', 'edit'],
    `ComfyArt is MIT (jtydhr88). Models: ${QWEN}, which the cards mark as non-commercial.`,
    [image('image', 'The picture to edit'), text('prompt', 'The change'), text('negative', 'What to avoid', false), seed()],
    [model('qwen-int8', 'diffusion_models', QWEN), model('qwen-te', 'text_encoders', QWEN), model('qwen-vae', 'vae', QWEN)],
    { type: 'image', mime: 'image/png' },
    { seconds: 18, vram_gb: 14, output_gb: 0.01 },
  )
  m.workflow = workflow
  manifests.push(m)
}

// 3. Henge: motion transfer from a clip onto a picture, on MiniMax H3.
{
  const workflow = await loadApi('g.henge.json')
  set(workflow, '3', 'lora_name', 'minimax_h3_motion_transfer_v1.0_ref2va.safetensors')
  set(workflow, '7', 'video', '{{source}}')
  set(workflow, '7', 'pictures', '["{{reference}}"]')
  set(workflow, '7', 'width', '{{width}}')
  set(workflow, '7', 'height', '{{height}}')
  set(workflow, '7', 'length', '{{length}}')
  set(workflow, '9', 'prompt', '{{prompt}}')
  set(workflow, '9', 'width', '{{width}}')
  set(workflow, '9', 'height', '{{height}}')
  set(workflow, '9', 'length', '{{length}}')
  set(workflow, '11', 'noise_seed', '{{seed}}')
  set(workflow, '17', 'trim_frames', '{{length}}')
  set(workflow, '18', 'trim_frames', '{{length}}')
  const m = base(
    'henge-motion',
    'Henge: move a picture like a clip',
    'The person in the clip moves, the character in the picture plays it. Imported from Henge (MIT) with its own H3 guide graph, its LoRA and its SAM 3.1 / SAM 3D picker.',
    ['video', 'henge', 'motion-transfer', 'needs-nodes'],
    `Henge is MIT (alisson-anjos) and needs its ComfyUI-Henge pack. Models: ${H3}, ${SAM}, and a Character Swap / Motion Transfer LoRA on mixed asset licences.`,
    [
      video('source', 'The clip whose motion is copied'),
      image('reference', 'The character to animate'),
      text('prompt', 'What happens', false, 'Defaults to “Animate <Picture 1> with <Video 1>.”'),
      number('length', 'Frames', 107, 32, 480),
      number('width', 'Width', 512, 256, 1920, 16),
      number('height', 'Height', 288, 256, 1088, 16),
      seed(),
    ],
    [
      model('h3-unet', 'diffusion_models', H3),
      model('h3-te', 'text_encoders', H3),
      model('h3-vae', 'vae', H3),
      model('h3-motion-lora', 'loras', 'Mixed asset licences (Alissonerdx)'),
      model('sam31', 'checkpoints', 'SAM License (Meta)'),
      model('sam3d', 'detection', 'SAM License (Meta)'),
    ],
    { type: 'video', mime: 'video/mp4' },
    { seconds: 300, vram_gb: 24, output_gb: 1.2 },
  )
  m.workflow = workflow
  manifests.push(m)
}

// 4. Splat Edit Engine: replace an object inside a Gaussian-splat scene.
{
  const workflow = await loadApi('g.spede.json')
  // SpEdE resolves its paths against the process working directory (ComfyUI's own folder), not
  // against the input folder a name usually means, so the values point into `input/` explicitly.
  set(workflow, '1', 'source_ply', 'input/{{scene}}')
  set(workflow, '2', 'camera_targets_npz', 'input/{{cameras}}')
  set(workflow, '3', 'image', '{{reference}}')
  set(workflow, '10', 'object_ply', 'input/{{object}}')
  set(workflow, '100', 'object_ply', 'input/{{object}}')
  set(workflow, '34', 'prompt', '{{prompt}}')
  set(workflow, '84', 'text', '{{object_name}}')
  set(workflow, '86', 'name', '{{object_name}}')
  set(workflow, '35a', 'noise_seed', '{{seed}}')
  const m = base(
    'spede-replace',
    'Splat Edit Engine: replace an object in a splat',
    'Click an object in a Gaussian splat, say what should stand there, and the scene is rebuilt with a new one. Imported from Splat Edit Engine (AGPL-3.0) with its own local replace graph.',
    ['mesh', 'spede', 'splat', 'replace', 'needs-nodes'],
    `Splat Edit Engine is AGPL-3.0 (Burgstall-labs) and needs its ComfyUI-SpEdE pack. Models: ${QWEN}, SAM License, TripoSplat MIT, DINOv3 License, FLUX.2 VAE Apache-2.0.`,
    [
      { key: 'scene', label: 'The scene (3DGS .ply)', type: 'file', required: true },
      { key: 'cameras', label: 'Camera pack (.npz)', type: 'file', required: true },
      { key: 'object', label: 'The object only (.ply)', type: 'file', required: true, help: 'The cut-out object already in the scene, as the pack exports it.' },
      image('reference', 'Hero frame of the view'),
      text('prompt', 'What takes its place'),
      text('object_name', 'Name of the object', true, 'Used for the cut-out and for SAM 3.1.'),
      seed(),
    ],
    [
      model('sam31', 'checkpoints', SAM),
      model('qwen-turbo', 'diffusion_models', QWEN),
      model('qwen-te', 'text_encoders', QWEN),
      model('qwen-vae', 'vae', QWEN),
      model('triposplat', 'diffusion_models', 'MIT (VAST-AI)'),
      model('triposplat-vae', 'vae', 'MIT (VAST-AI)'),
      model('dino-vith', 'clip_vision', 'DINOv3 License (Meta)'),
      model('flux2-vae', 'vae', 'Apache-2.0 (FLUX.2 VAE)'),
    ],
    { type: 'mesh', mime: 'application/octet-stream' },
    { seconds: 240, vram_gb: 20, output_gb: 0.4 },
  )
  m.workflow = workflow
  manifests.push(m)
}

// 5. Max Editor: retouch a passage of a clip, on MiniMax H3.
{
  const workflow = await loadApi('g.maxretouch.json')
  set(workflow, '11', 'file', '{{video}}')
  set(workflow, '112', 'image', '{{face}}')
  set(workflow, '89', 'prompt', '{{prompt}}')
  set(workflow, '28', 'noise_seed', '{{seed}}')
  set(workflow, '108', 'width', '{{width}}')
  set(workflow, '108', 'height', '{{height}}')
  const m = base(
    'maxeditor-retouch',
    'Max Editor: retouch a passage',
    'Tracks a subject in a clip, masks it and regenerates only that area with MiniMax H3, keeping the rest of the shot. Imported from Max Editor (Apache-2.0) with its own retouch graph.',
    ['video', 'max-editor', 'retouch', 'needs-nodes'],
    `Max Editor is Apache-2.0 (xentome) and needs its app’s node pack. Models: ${H3}, ${SAM}; the Pika Flex LoRA is a MiniMax H3 derivative.`,
    [
      video('video', 'The clip to retouch'),
      image('face', 'Reference frame of the subject'),
      text('prompt', 'What should change in the masked area'),
      number('width', 'Width', 1024, 256, 1920, 16),
      number('height', 'Height', 1024, 256, 1920, 16),
      seed(),
    ],
    [
      model('h3-unet', 'diffusion_models', H3, { file: 'Minimax-h3_Singularity_ref2va_v2_int8.safetensors' }),
      model('h3-te', 'text_encoders', H3),
      model('h3-vae-int8', 'vae', H3),
      model('h3-avae', 'vae', H3),
      model('sam31', 'checkpoints', SAM),
      model('h3-motion-lora', 'loras', 'MiniMax H3 Community License'),
      model('taeh3', 'vae_approx', 'MIT or Apache-2.0, per file'),
    ],
    { type: 'video', mime: 'video/mp4' },
    { seconds: 300, vram_gb: 24, output_gb: 1.2 },
  )
  // `Minimax-h3_Singularity` is a third-party checkpoint, not the Comfy-Org file.
  m.models[0] = {
    ...m.models[0],
    url: 'https://huggingface.co/WarmBloodAban/Minimax-h3_Singularity/resolve/main/Minimax-h3_Singularity_ref2va_v2_int8.safetensors',
    sha256: 'db3ec568863db6f5ec1840210d41653afef73d4df7b0cb489c52ad458b6f8a7f',
    size: 34004507622,
  }
  m.models[5] = model('pika-lora', 'loras', H3)
  m.workflow = workflow
  manifests.push(m)
}

// 6. Creativefield: one picture into a new scene, with Fibo 2.0.
{
  const workflow = await loadApi('g.creativefield.json')
  set(workflow, '10', 'image', '{{image}}')
  set(workflow, '13', 'value', '{{prompt}}')
  set(workflow, '5', 'seed', '{{seed}}')
  for (const id of ['30', '31', '32']) drop(workflow, id)
  for (const field of ['images.image_2', 'images.image_3', 'images.image_4']) delete workflow['4'].inputs[field]
  const m = base(
    'creativefield-edit',
    'Creativefield: reimagine a picture',
    'Describe a new scene; the subject, pose and framing of your picture survive. Imported from Creativefield (GPL-3.0) with its own Fibo 2.0 graph and Qwen3-VL prompt writer.',
    ['image', 'creativefield', 'edit', 'needs-nodes'],
    `Creativefield is GPL-3.0 (rhymeas) and needs its Fibo 2.0 node pack. Model: Fibo 2.0, CC BY-NC 4.0 (non-commercial); its prompt writer runs on Qwen3-VL 8B.`,
    [image('image', 'The picture to reimagine'), text('prompt', 'The new scene'), seed()],
    [
      model('fibo2', 'diffusion_models', 'CC BY-NC 4.0'),
      model('qwen3vl-4b', 'text_encoders', 'CC BY-NC 4.0'),
      model('qwen-te', 'text_encoders', 'Qwen-Image 2.1 Research License (non-commercial)', { file: 'qwen3vl_8b_int8_convrot.safetensors' }),
      model('fibo2-vae', 'vae', 'CC BY-NC 4.0'),
    ],
    { type: 'image', mime: 'image/png' },
    { seconds: 20, vram_gb: 16, output_gb: 0.01 },
  )
  m.workflow = workflow
  manifests.push(m)
}

// 7. Panorama Stickers (UI format converted): a picture becomes a 360 panorama.
{
  const workflow = uiToApi(JSON.parse(await readFile(`${raw}/g.panorama.json`, 'utf8')))
  set(workflow, '80', 'image', '{{image}}')
  set(workflow, '64', 'prompt', '{{prompt}}')
  set(workflow, '3', 'seed', '{{seed}}')
  const m = base(
    'panorama-cutout',
    'Panorama Stickers: picture to 360 cut-out',
    'Turns a picture into an equirectangular 360 scene with a cut-out foreground you can orbit. Imported from Panorama Stickers (MIT); its published workflow was editor-only and is converted to API format.',
    ['image', 'panorama', '360', 'needs-nodes'],
    `Panorama Stickers is MIT (nomadoor) and needs its ComfyUI-Panorama-Stickers pack. Models: ${QWEN}, non-commercial.`,
    [image('image', 'The picture'), text('prompt', 'What the panorama should show'), seed()],
    [model('qwen-int8', 'diffusion_models', QWEN), model('qwen-te', 'text_encoders', QWEN), model('qwen-vae', 'vae', QWEN)],
    { type: 'image', mime: 'image/png' },
    { seconds: 30, vram_gb: 16, output_gb: 0.02 },
  )
  m.workflow = workflow
  manifests.push(m)
}

// 8. Sonder: the timeline editor's reference-to-video render, on MiniMax H3.
{
  const workflow = uiToApi(JSON.parse(await readFile(`${raw}/s.sonder_minimax_h3_references.json`, 'utf8')))
  // The author's `H3\` subfolder doesn't resolve on Linux, where `model_path` treats the backslash
  // as a literal, so the manifest keeps the flat name and the download lands it flat too.
  for (const node of Object.values(workflow)) {
    for (const [field, value] of Object.entries(node.inputs)) {
      if (typeof value === 'string') node.inputs[field] = value.replace(/^H3\\/, '')
    }
  }
  // The style loader's two LoRAs are both off, so the graph keeps only its model passthrough.
  // Everything downstream of it reads the model straight from the branch it was fed.
  const feed = workflow['276'].inputs.model
  for (const node of Object.values(workflow)) {
    for (const [field, value] of Object.entries(node.inputs)) {
      if (Array.isArray(value) && String(value[0]) === '276') node.inputs[field] = [...feed]
    }
  }
  drop(workflow, '276')
  set(workflow, '136', 'project', '{{project}}')
  set(workflow, '557', 'noise_seed', '{{seed}}')
  const m = base(
    'sonder-render',
    'Sonder: render the timeline',
    'The reference-to-video render of Sonder’s timeline editor: references, guides and sound on one timeline, rendered with MiniMax H3. Imported from Sonder (GPL-3.0); its workflow was editor-only and is converted to API format.',
    ['video', 'audio', 'sonder', 'needs-nodes'],
    `Sonder is GPL-3.0 (SonderSaid) and needs its Sonder Editor pack plus RES4LYF, rgthree-comfy and KJNodes. Models: ${H3} and two turbo LoRAs on the same licence.`,
    [
      text('project', 'Project name', true, 'A project from the author’s sample dataset, or your own in ComfyUI’s input folder.'),
      seed(),
    ],
    [
      model('h3-unet', 'diffusion_models', H3),
      model('h3-unet-fl2va', 'diffusion_models', H3, { file: 'minimax_h3_fl2va_pruned_int8_convrot.safetensors' }),
      model('h3-te', 'text_encoders', H3),
      model('h3-vae', 'vae', H3),
      model('h3-avae', 'vae', H3),
      model('h3-ref2v-turbo-lora', 'loras', H3, { file: 'minimax_h3_ref2v_lightx2v_turbo_4step_v0.1_resized_avg_rank_20_bf16.safetensors' }),
      model('h3-fl2v-turbo-lora', 'loras', H3, { file: 'minimax_h3_fl2v_lightx2v_turbo_8step_v1.0_resized_avg_rank_24_bf16.safetensors' }),
    ],
    { type: 'video', mime: 'video/mp4' },
    { seconds: 420, vram_gb: 24, output_gb: 1.5 },
  )
  m.workflow = workflow
  manifests.push(m)
}

// 9. Reshoot 4D (UI format converted): reshoot a clip from a new camera.
{
  const workflow = uiToApi(JSON.parse(await readFile(`${raw}/g.reshoot.json`, 'utf8')))
  set(workflow, '1', 'video', '{{source}}')
  set(workflow, '20', 'prompt', '{{prompt}}')
  set(workflow, '20', 'seed', '{{seed}}')
  const m = base(
    'reshoot-4d',
    'Reshoot 4D: reshoot from a new camera',
    'Turns one clip into a 4D capture and reshoots it from a camera that never existed, then renders it photoreal. Imported from Reshoot 4D (AGPL-3.0); its published workflow was editor-only and is converted to API format.',
    ['video', 'reshoot-4d', 'camera', 'needs-nodes'],
    `Reshoot 4D is AGPL-3.0 (itsmetamike) and needs its pack, LTX 2.3 guides and a separate ReDirector environment. Models: LTX-2 Community License, Gemma terms, Apache-2.0 and MIT helpers; its RIFE weights ship with the Frame-Interpolation pack. The capture path is research-only: GVHMR terms, SMPL-X under non-commercial research terms with manual registration (body_models/smplx/SMPLX_NEUTRAL.npz), and the Turbo view LoRA under CC BY-NC-SA 4.0.`,
    [video('source', 'The clip to reshoot', 'One person, 2 to 8 seconds.'), text('prompt', 'What the shot should be', false, 'Empty lets Qwen3-VL write it from the frames.'), seed()],
    [
      model('reshoot-da3', 'geometry_estimation', 'Apache-2.0'),
      model('reshoot-birefnet', 'background_removal', 'MIT (BiRefNet)'),
      model('reshoot-moge', 'geometry_estimation', 'MIT (MoGe)'),
      model('reshoot-ltx', 'diffusion_models', 'LTX-2 Community License'),
      model('reshoot-gemma', 'text_encoders', 'Gemma terms'),
      model('reshoot-proj', 'text_encoders', 'LTX-2 Community License'),
      model('reshoot-vae', 'vae', 'LTX-2 Community License'),
      model('reshoot-avae', 'vae', 'LTX-2 Community License'),
      model('reshoot-up', 'upscale_models', 'LTX-2 Community License'),
      model('reshoot-crossview', 'loras', 'Apache-2.0'),
      model('reshoot-union', 'loras', 'LTX-2 Community License'),
      model('reshoot-detail', 'loras', 'LTX-2 Community License'),
      model('reshoot-qwen', 'text_encoders', 'Apache-2.0 base; conversion terms unknown'),
    ],
    { type: 'video', mime: 'video/mp4' },
    { seconds: 3600, vram_gb: 24, output_gb: 0.2 },
  )
  m.workflow = workflow
  manifests.push(m)
}

for (const manifest of manifests) {
  const text = `${JSON.stringify(manifest, null, 2)}\n`
  await writeFile(`${out}/${manifest.id}.json`, text)
  console.log(manifest.id, manifest.models.length, 'models', Object.keys(manifest.workflow).length, 'nodes')
}

