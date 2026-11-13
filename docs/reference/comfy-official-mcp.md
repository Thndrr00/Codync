# The official Comfy MCP, evaluated against Codync's bridge

Comfy ships a local MCP server, [`Comfy-Org/comfy-mcp`](https://github.com/Comfy-Org/comfy-mcp) (PyPI `comfy-mcp`, AGPL-3.0-or-later OR commercial). This note records what it covers, what Codync's `comfyui` bridge still owns, and why the bridge was not replaced in this change. Its tool surface was checked by `tools/comfy-mcp-smoke.mjs` against 0.10.0; no workflow ran, because this machine has no ComfyUI and no GPU.

## What it is

- A **stdio** MCP server that wraps **`comfy-cli` ≥ 1.14.0**, which is the engine behind every tool. It needs Python ≥ 3.10 and a ComfyUI workspace that `comfy-cli` manages (`comfy install` or `comfy set-default`).
- Version 0.10.0 at the time of writing; its own README calls it **beta** (41 tools in the README, **39** in the installed 0.10.0).
- Default target `127.0.0.1:8188`; `COMFY_LOCAL_URL` moves a local server on another port, `COMFYUI_URL`/`COMFYUI_HOST` submit runs to another machine.
- Every tool shells out to `comfy --where local --json` and unwraps `envelope/1`. It is a thin wrapper, not an HTTP client of its own.
- Licensing: **AGPL-3.0-or-later OR commercial**. Using it as a separate, user-installed connector is one thing; bundling or shipping it inside Codync would pull AGPL obligations into the distribution.

## The smoke run

On 13 November 2026, `tools/comfy-mcp-smoke.mjs` ran against `comfy-mcp 0.10.0` + `comfy-cli 1.26.0` in a throwaway Python 3.14 venv on this Windows machine, with no ComfyUI workspace and no GPU:

```
Official Comfy MCP: 39 tools
list templates     search_templates    ok
describe template  get_template        ok
run by name        run_template        ok
run a graph        run_workflow        ok
job status         job                 ok
fetch outputs      fetch_outputs       ok
upload input       upload_file         ok
```

`server_info` answered with `workspace.exists: false`, `server.running: false`, `gpu: null` and a `workspace_not_found` warning; `search_templates` returned 761 gallery templates. That is the tool surface verified, not a generation: with no ComfyUI running, no workflow was executed.

## Coverage of what T1–T9 needed

| Operation | Official tool | Covered? |
| --- | --- | --- |
| List templates | `search_templates(query, tag, type, model, provider, exclude_api)` | Yes, but only ComfyUI's **gallery** (`Comfy-Org/workflow_templates`), not Codync's manifests |
| Describe a template | `get_template(name, check_local)` + `local_check` against live `object_info` | Yes; its “template” is a gallery name plus workflow slots, not a form schema |
| Run by name | `run_template(name, params, wait, confirm_spend)` | Yes, with slot addresses (`6.text`) or slot names |
| Run a graph | `run_workflow(workflow_path, wait, confirm_spend)` | Yes, API-format or UI export, by file path |
| Status | `job(action="status"｜"queue"｜"watch"｜"wait")` | Yes, including progress notifications |
| Cancel | `job(action="cancel", prompt_id)` | Yes |
| Upload an input | `upload_file(paths, overwrite)` | Yes, to the target's `input/` |
| Fetch outputs | `fetch_outputs(prompt_id, out_dir, url_only, inline_images)`, `preview_media` | Yes; copies files to a directory and can inline images |
| Nodes | `nodes(action="search"｜"get"｜"list"｜…)` | Yes, richer than Codync's `list_nodes` |
| Models | `search_models`, `download_model`, `download(action=…)` | Yes, by URL into the comfy-cli workspace |
| Preflight | `validate_workflow`, `workflow_deps`, `node_dependencies` | Mostly; it validates a graph but has no Codync manifest, VRAM estimate or disk action |
| Server lifecycle | `server_info`, `launch_comfyui`, `stop_comfyui`, `restart_comfyui`, `update_comfyui`, `get_logs` | Yes, but only for a `comfy-cli` workspace, not Codync's pinned portable |

So the official server **does cover the seven operations the bridge implements generically**. What it does not cover is the part that makes ComfyUI a Codync feature:

- **Codync's named templates** with a manifest (`template.json`), a validated input schema, per-GPU estimates and a generated form. The official “template” is a gallery workflow with slots; it has no `estimate`, no `models` contract with sizes/hashes, and no Codync preflight.
- **Codync's preflight**: models on disk at the declared size, VRAM free against the estimate, disk for the output, one action per problem, and a refusal before queueing.
- **Codync's job delivery**: the typed output (image/video/audio/mesh) attached to the bot's chat with `send_file`, the twenty-entry host history and the card's progress/poster. The official `fetch_outputs` writes to a directory and returns paths; it has no chat.
- **Codync's runtime**: the pinned Windows portable with its own embedded Python, data folder, port and model folders. The official wraps a `comfy-cli` workspace and a Python the user installs.

## Decision for this change

Do not replace the bridge now. Three concrete reasons:

1. **Licence**: the official is AGPL-3.0-or-later or commercial. Bundling it in the installer would change Codync's distribution obligations. A user-installed connector is not the same thing, but automatic registration would still make Codync set it up.
2. **Runtime**: it needs Python + `comfy-cli` + a workspace it manages. Codync ships a self-contained portable and starts it with its own flags and data folder; adding a second Python toolchain to drive the same engine would duplicate the runner and its port handling.
3. **Beta**: 41 tools, 0.10.0, changing quickly. The bridge's behaviour is covered by Codync's own tests (unit tests and the ignored real-ComfyUI suite), and the official has no Codync-specific parts to replace.

What we do instead:

- Keep the bridge's **names** stable (`list_models`, `list_nodes`, `run_workflow`, `job_status`, `cancel_job`, `upload_image`), so a bot written against them keeps working if the official server is registered beside them.
- Keep Codync's `comfy_*` tools as the extension with their own names: templates, preflight, run-by-name, typed result, upload.
- Offer a **smoke harness** (`tools/comfy-mcp-smoke.mjs`) that checks the official server's seven operations over stdio whenever someone wants to adopt it. It does not run in CI (no Python or package in the build image).
- If adoption is chosen later, the removal order is: `list_models`/`list_nodes` → `run_workflow`/`wait_for_job`/`job_status`/`cancel_job` → `upload_image` → `example_workflow`. The `comfy_*` template tools stay either way, because the official has no Codync manifest, preflight or chat attachment.

## The harness

```
node tools/comfy-mcp-smoke.mjs --bin "C:\path\to\comfy-mcp.exe" --comfy-bin "C:\path\to\comfy.exe"
```

It starts the server with `COMFY_BIN` set, performs the MCP handshake (`initialize`, `notifications/initialized`), lists tools, and prints a table of the operations above with `ok`/`missing`. With `--call` it also calls `server_info` and `search_templates` and prints their JSON, which is the part that needs a running ComfyUI.

Run it only where the official server and `comfy-cli` are installed; Codync's own bridge is unaffected.
