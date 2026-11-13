# ComfyUI (bundled tool)

How ComfyUI ships and runs inside Codync, what was taken from its repository, and where Codync differs from the plan (`docs/plans/bundled-tools-production.md`).

## One pin

`packaging/comfy/version.json` is the only place the version is named: repo, tag (`v0.44.0`), commit (`2c75dc1b…`), release asset, size and SHA-256. The host downloads exactly that release (`host/src/tools/comfy_install.rs`); `packaging/comfy/fetch.mjs` reads it too, for development. To update ComfyUI: change that file from the release page, nothing else.

## What is taken from the official repository (Comfy-Org/ComfyUI @ v0.44.0)

| Piece | Source | Used for |
| --- | --- | --- |
| `ComfyUI_windows_portable_nvidia.7z` | the release asset, checked against the pinned size and SHA-256 | the program: downloaded by the host on ComfyUI's first start, checked as it arrives, unpacked with the `7za.exe` shipped beside the host into `<data>/comfy/program/<version>` |
| Backend, unchanged | `main.py` and everything it loads | started as `python_embeded/python.exe -s ComfyUI/main.py …`; no file of it is copied into Codync |
| CLI flags | `comfy/cli_args.py`: `--listen`, `--port`, `--disable-auto-launch`, `--user-directory`, `--input-directory`, `--output-directory`, `--temp-directory`, `--extra-model-paths-config`, `--cpu` | `tools/comfy.rs` `launch` |
| Folder keys | `extra_model_paths.yaml.example` | the generated `~/.codync/comfy/extra_model_paths.yaml` (`MODEL_FOLDERS`) |
| HTTP API | `server.py`: `/system_stats`, `/object_info`, `/prompt`, `/queue`, `/history/{id}`, `/interrupt`, `/view`, `/upload/image` | `tools/comfy_api.rs` |
| Example workflow | `script_examples/basic_api_example.py`, the graph in `prompt_text`, copied verbatim | `tools/comfy_example.json` (the `example_workflow` tool) |

The only copied content is the example graph: its JSON is identical to the original's (`diff` of the two JSON values is empty; Codync only reflows it onto one line per node).

## Where this differs from the plan

- **No multi-user mode, no seeded user.** In v0.44.0 `app/user_manager.py` has no tokens: with `--multi-user` a user is an id sent in the `comfy-user` header and the frontend shows a user picker when none is stored; without it there is the single `default` user, no sign-in screen at all, and the editor's settings are stored on the server under `--user-directory`. Seeding `local@codync` would add the very picker the plan forbids and no security (the server listens on loopback only), so Codync runs single-user. There is nothing for the vault to hold and no session to retry.
- **Node names are checked against `/object_info`, not `nodes.py`.** The running server is the truth for built-in and custom nodes alike (`comfy_job::validate`); a copy of `nodes.py` would go stale.
- **The repo has no `user/default/workflows`** in this version; the example is the repo's API example above.
- **Windows x64 only** (the NVIDIA portable also runs on the CPU). Other targets get an empty `comfy/` folder and the card says the build does not include ComfyUI.

## Runtime

`host/src/tools/` is the shared runner (see its `mod.rs`): `runner.rs` starts the process inside a Job Object (`agent::process::Process`, so stopping it ends Python and everything it started), picks the port (8188, else a free one, noted in the log), keeps the state `stopped/starting/running/error`, and watches for a crash; `health.rs` probes `GET /system_stats`; `log.rs` writes `~/.codync/logs/comfy.log` (rotated at 5 MB); `lifecycle.rs` starts what was switched on with the host and stops everything on shutdown. n8n still runs through `host/src/n8n.rs`.

Data: `~/.codync/comfy/{user,models,custom_nodes,input,output,temp}` (`~/.codync-dev` for Codync Dev). The program is not in the installer: NSIS (32-bit `makensis`) can't map the 2.5 GB package, so a Windows installer that holds it fails to build. The first start downloads it into `<data>/comfy/program/<version>`; a new pin downloads the new version and removes the old one, and models and settings are never touched. A download needs 8 GB free, stalls after a minute without data, retries network and server failures, and never retries a wrong archive or a missing release. Both `electron-builder.yml` and `electron-builder.dev.yml` list `build/native/7za.exe` because the Dev config replaces the base resource list.

## Bots

The host registers a `comfyui` connector while ComfyUI runs (`lifecycle::offer_to_bots`); a bot gets the `codync-host mcp comfy` server only when its owner turned that connector on (`agent/bot/session.rs`). Tools: `comfy_list_templates`, `comfy_describe_template`, `comfy_run_template`, `comfy_job_status`, `comfy_cancel_job`, `comfy_upload_input`, `comfy_result`, plus the original `list_models`, `list_nodes`, `example_workflow`, `run_workflow`, `wait_for_job`, `upload_image`, `job_status`, `cancel_job`. The host follows each job (`/history`, `/queue`, `/ws`, 30 minute limit) for the card. The files reach the bot's chat with `send_file`, which only works while the bot's turn runs, so the bot's own call delivers them: `comfy_run_template` queues and waits up to 5 minutes; a longer job returns still running and `comfy_result` (or a `comfy_job_status` look) delivers it later, once. A wait abandoned because the asking bot's turn stopped cancels the job (`comfy_job::CancelOnDrop`); another bot's wait never does. One job at a time. Host API: `toolStatus|toolStart|toolStop` (`{id}`), `comfyCancel`, `comfyJobImage`, `comfyJobOutput`, `comfyJobs`, `comfyTemplates`, `comfyTemplate`, `comfyRunTemplate`, `comfyModels`, `comfyModelDownload`, `comfyModelPause`, `comfyModelDelete`, `comfyUploadImage`; `comfyCall` (the MCP bridge) is loopback-only, like the other bridges.

## Templates

`host/src/tools/comfy/templates.rs` loads one manifest per folder under `<data>/comfy/templates/<id>/template.json`:

| Field | Meaning |
| --- | --- |
| `id`, `name`, `version`, `description`, `tags`, `license` | what the gallery and the bot show |
| `workflow` | the graph in ComfyUI's API format, with `{{input}}` placeholders in its strings |
| `inputs` | the form: `key`, `label`, `type` (`text`, `number`, `integer`, `select`, `image`, `video`, `audio`, `file`, `seed`, `boolean`), `default`, `min`, `max`, `step`, `options`, `required`, `help` |
| `models` | `file`, `folder` (a key of `extra_model_paths.yaml`), `size`, `sha256`, `url`, `license` |
| `output` | `image`, `video`, `audio` or `mesh`, and an optional MIME override |
| `estimate` | seconds, VRAM and output GB, overall and per GPU family under `byGpu` (`rtx-4090`, `default`…) |

Validation happens before anything is listed or run: ids, ranges, select options, model folders, sizes, hashes and URLs, and that the placeholder set equals the input set (a form field the graph never uses is refused, and a placeholder with no field is refused). The graph is data, never code: only placeholder substitution is applied, and every node class is checked against the running server's `/object_info` (`comfy_job::validate`). `{{key}}` as a whole string keeps its JSON type (a number stays a number); inside a longer string it is joined as text.

The twenty base manifests are embedded from `host/assets/comfy-templates/` and copied on startup. An untouched base manifest is updated in place; one the user edited is left alone and the new version is written beside it as `template.json.new`. Their model URLs were checked against the publishers and their sizes and SHA-256 come from the repositories: Stable Diffusion 1.5 fp16 (`2,132,696,762` bytes, CreativeML OpenRAIL-M), ControlNet v1.1 canny (`1,445,234,681` bytes, Apache-2.0), Real-ESRGAN x4plus (`67,040,989` bytes, BSD-3-Clause) and the five Qwen-Image-Edit files of Freefield's Angles (Apache-2.0, 31.2 GB in total, exact LFS sizes and hashes). The fifteen imported families each carry the licence of their model and of their author's pack; the non-commercial ones say so (Qwen-Image 2.1, Fibo 2.0, Reshoot's capture path).

`tools/import-freefield-app.mjs` turns a Freefield `app.json` and `workflow.api.json` into a manifest: it binds each declared input to its graph slot, merges template bindings (`{move} {details}` becomes `{{move}} {{details}}`), keeps the models the app declares, and fills their size and SHA-256 from the Hugging Face tree. `freefield-angles` is imported from it.

**Paint a Plate** is imported from AInVFX's Apache-2.0 `web/workflows/edit.json` and `web/models.json`: its 30-node edit graph exposes five images, a caption, the sampler values and the relight mask controls. Its three FLUX 3 files are **gated** on Hugging Face, so the manifest marks them `gated`: the preflight and the Models tab open their page instead of pretending a download can work without a signed-in token.

`tools/check-comfy-manifests.mjs` checks every embedded manifest (ids, placeholders, input types, model folders, gated pages, output kind) without compiling the host; the loader enforces the same rules.

**A family can need an input clip, a song or a scene**, so a template input can be `video`, `audio` or `file` as well as `image`: the value is a local path the host uploads (up to 300 MB) or a name the server already has, and the extension must be one the loader reads. Each family keeps its author's graph:

- **H3 Motion: change what happens in a mask** (`h3-motion-mask`), from PxTicks' AGPL-3.0 `workflows/api/h3m_inpaint.api.json`: a clip, a white-on-black mask clip of the same length, the prompt for what appears inside the mask, length and size. It needs the author's ComfyUI-vlo pack and VideoHelperSuite; the preflight names each missing node.
- **Swap the person in a clip** (`openjutsu-swap`), from Genobi's MIT copy of Comfy Openjutsu and Akatz Labs' MIT H3 Character Swap: a five-to-fifteen-second clip and one character picture. It needs Akatz Labs' `h3_character_swap` pack, GPL-3.0, with Genobi's GPL patch; that is a licence the user accepts by installing it, not one Codync ships.
- **OneShot MV: a song into a music video** (`oneshot-mv`), from VisualFrisson's MIT `api/OneShotMV_director_api.json`: a song, one line about the world, length, shape, quality. `FVDirector` is one node that plans, casts, renders and edits inside its own pack, so the manifest is three nodes and the pack is required.

### The other ten families

All fifteen entries of `curated.json` are imported; each card still opens its author's repository and states its licence. A licence never blocks an import: the user accepts the author's pack licence by installing the pack.

- **Blocksfield: a pitch into a game screenshot** (`blocksfield-art`, MIT), 13 core nodes on Qwen-Image 2.1 Turbo. **ComfyArt: restyle a picture** (`comfyart-edit`, MIT), 9 core nodes on the same model.
- **Henge: move a picture like a clip** (`henge-motion`, MIT), 17 nodes on H3 with the author's motion LoRA and a SAM 3.1 / SAM 3D picker.
- **Splat Edit Engine: replace an object in a splat** (`spede-replace`, AGPL-3.0), the author's local replace graph, 35 nodes.
- **Max Editor: retouch a passage of a clip** (`maxeditor-retouch`, Apache-2.0) and **H3 Audio Retake: redo a speaking face** (`h3-audio-retake`, MIT), both on H3: the first tracks and masks a subject, the second crops the face and rebuilds only the mouth.
- **Creativefield: reimagine a picture** (`creativefield-edit`, GPL-3.0), Fibo 2.0 and a Qwen3-VL prompt writer.
- **Panorama Stickers: a picture into a 360 cut-out** (`panorama-cutout`, MIT), on Qwen-Image 2.1.
- **Sonder: render the timeline** (`sonder-render`, GPL-3.0), the reference-to-video render of the author's editor, 140 nodes.
- **Reshoot 4D: reshoot from a new camera** (`reshoot-4d`, AGPL-3.0), a 4D capture and a photoreal reshoot through LTX 2.3.

Three of them only publish a graph the editor saves, so `tools/comfy-ui-to-api.mjs` converts it: it inlines subgraphs, follows reroutes, folds primitive nodes, drops notes and muted or bypassed nodes, and takes each widget's own name from the newer `widgets_values_named` (falling back to `tools/comfy-widget-names.json` for the older positional lists). Its tests are `node --test tools/comfy-ui-to-api.test.mjs`. The ten manifests are rebuilt from those raw graphs with `node tools/comfy-families/build.mjs` (plus `build-audioretake.mjs`), over the resolved inventory in `tools/comfy-families/models.json`.

Splat Edit Engine takes files that are not pictures, clips or sounds — a Gaussian-splat scene, a camera pack, the cut-out object — so those fields are of type `file` (`ply`, `npz`, `spz`, `splat`, `obj`, `glb`, `gltf`, `zip`). It resolves its paths against the process working directory rather than the input folder, so its manifest prefixes them with `input/`.

Manual steps that a manifest cannot declare, all stated on the card:

- **Reshoot 4D**: SMPL-X registration and `body_models/smplx/SMPLX_NEUTRAL.npz`, a separate ReDirector environment and the LTX 2.3 guides. The capture path is research-only (GVHMR, SMPL-X, and a CC BY-NC-SA view LoRA).
- **Sonder**: the author's sample dataset or a project of your own; without one the editor node has nothing to render.
- **H3 Audio Retake**: its face detector (`face_yolov8m.pt`) downloads itself on first use.
- **Splat Edit Engine**: a scene, its camera pack and the cut-out object, all exported by its own studio.

Non-commercial models are marked as such in the manifest licence text: Qwen-Image 2.1 and Fibo 2.0, plus Reshoot's Turbo LoRA, GVHMR and SMPL-X. Scene files count against the same 300 MB upload limit.

## Preflight

`host/src/tools/comfy/preflight.rs` checks, before queueing: the server is running; each class the graph uses exists in `/object_info`; each declared model is on disk at its declared size; the GPU has the estimated free VRAM; and the output disk has room. Each problem has an action the card and the bot can act on: `start`, `installNode`, `download` (folder, file, URL, size, licence), `lowerResolution`, `freeSpace`. A run with any unresolved problem is refused; no job enters the queue. A missing NVIDIA card is a warning (`useCpu`), not a blocker.

## Jobs, progress and typed outputs

A job is `queued → running → done | cancelled | error`, with the node and step `/ws` reports (`{"progress": {"node", "class", "step", "total", "percent"}}`), its template id and when it started. Finished jobs stay in a twenty-entry history in the host, so the phone and the card read the same state after the desktop closes. Cancelling a running job interrupts it, a queued one is dequeued; a bot's stopped turn cancels the job it was waiting for.

`outputs` is read from the whole `outputs` record (not only `images`): image, video, audio and mesh keep their own extension and MIME (`comfy/results.rs`), and a video gets a JPEG poster made with `ffmpeg` when it is on `PATH`. A missing `ffmpeg` costs the poster, not the output. A graph that reports no file of its own (`oneshot-mv`'s `FVDirector` writes the film itself, `spede-replace` writes the splat) falls back to the ingestible outputs created since the job was queued, newest five at most. The chat attachment path is unchanged: `send_file` copies any type, and the apps already show images and video.

## Models

`host/src/tools/comfy/models.rs` lists every models folder with present and declared files, the templates that need each one, bytes per folder and the disk's free space. Downloads are HTTP range requests into `<file>.part`, only for a `(folder, file)` a manifest declares and only from that manifest's URL, so a bot or a client can't point the downloader at arbitrary text. Pause stops after the chunk in flight; start resumes at the `.part` size. A finished file is checked against the manifest's size and SHA-256 before it is renamed. Deleting a model a template needs asks for confirmation. A **gated** model (Paint a Plate's FLUX files) is not downloaded by the host: the card opens its Hugging Face page and the user signs in and places the file in the folder, because the repository never allows an anonymous fetch.

## Not covered here yet

- **The official ComfyUI MCP** was evaluated and its tool surface checked: [comfy-official-mcp.md](comfy-official-mcp.md) has the coverage table, the actual 39-tool smoke run and why Codync keeps its bridge (AGPL/commercial licence, a Python + `comfy-cli` runtime, beta). `tools/comfy-mcp-smoke.mjs` is the harness.
- **The Higgsfield challenge pack** is fully imported: `host/assets/comfy-templates/curated.json` lists the fifteen entries with their authors' repositories, licences and output types, and each has a manifest. See [the ten families above](#the-other-ten-families) for their prerequisites and the non-commercial ones.
- **One job at a time**, no parallel queue, and the twenty-entry history lives in the host process (not SQLite).
- **The gallery, form and model manager are on the desktop**; Android shows the job, its percentage and cancel, and iOS has no ComfyUI section yet. The host owns the state, so those screens can follow without host changes.
- **Audio has a player in the chat and a `.glb` a WebGL viewer** (`thread/MeshViewer.tsx`); other mesh and splat formats keep their download card, so a `.ply` or `.spz` arrives as a file.
- **The version bump** was left out of this change: `tools/bump-version.py` needs `xcodegen`, which needs macOS.

## Known limits

- The first start needs about 10 GB free; models are extra.
- The portable is about 2 GB to download and 4.5 GB unpacked, on the first start rather than in the installer.
- Windows may ask for a firewall rule if something outside loopback is attempted; ComfyUI is started with `--listen 127.0.0.1`, so none is expected.
- Uninstalling keeps `~/.codync/comfy` unless the user answers Yes to "Also delete your ComfyUI models…" (`resources/installer.nsh` runs `codync-host uninstall --remove-comfy`; the host knows which data folder is this app's, Dev or production; a silent uninstall keeps it). Updating never touches it.
- Not covered here: macOS and Linux, several jobs in parallel.
