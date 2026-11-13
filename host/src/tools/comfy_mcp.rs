//! The `comfyui` MCP tools a bot gets when its owner turns the ComfyUI connector on for it.
//! Calls reach the host as `comfyCall`; the host talks to the local ComfyUI. Nothing is generated
//! unless the bot calls `run_workflow` or `comfy_run_template`.
//!
//! Templates are the intended way in: a bot asks for a named template and fills its form, instead of
//! writing a graph of twenty nodes. `run_workflow` stays for graphs the bot already has.

use super::comfy::templates::{InputKind, Template};
use super::comfy::{self, CONNECTOR_ID, preflight, results, templates};
use super::comfy_api::Api;
use crate::hub::Hub;
use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub const INSTRUCTIONS: &str = "Make images, video, audio and 3D with the ComfyUI on this computer. \
Start with comfy_list_templates: each template has a form and knows which models it needs. comfy_describe_template \
shows its inputs and its preflight; comfy_run_template runs it and sends the finished files to this chat. \
When a model is missing, the reply names the download action. A slow job comes back still running: call \
comfy_result to keep waiting and get its files, comfy_job_status to look without waiting, or comfy_cancel_job \
to stop it. One job runs at a time. list_models shows what is installed, list_nodes finds node names, and \
run_workflow takes a whole graph in ComfyUI's API format for anything the templates don't cover. Use \
comfy_upload_input to give a workflow an input file (its returned name goes in a LoadImage node).";

/// How long one call waits for a job: under the agents' own tool timeouts. A longer job is waited for again.
const WAIT_IN_CALL: Duration = Duration::from_mins(5);
/// The MCP bridge's timeout for a `comfyCall`: the wait plus the time to queue and attach.
pub const CALL_TIMEOUT: Duration = Duration::from_mins(6);

/// The example graph: the default workflow of the repo's `script_examples/basic_api_example.py`.
const EXAMPLE: &str = include_str!("comfy_example.json");
const MAX_UPLOAD: u64 = 100 * 1024 * 1024;

pub fn tools() -> Value {
    json!([
        tool(
            "comfy_list_templates",
            "The named templates this computer can run, with what each makes and how long it takes. Start here.",
            &json!({"query": {"type": "string", "description": "Only names, tags and descriptions containing this."}}),
            &[]
        ),
        tool(
            "comfy_describe_template",
            "One template's form (inputs with types, ranges and defaults), its models and its preflight.",
            &json!({"id": {"type": "string"}}),
            &["id"]
        ),
        tool(
            "comfy_run_template",
            "Run a template by id with values for its form. Waits for the result and sends its files to this chat; a slow job returns still running.",
            &json!({
                "id": {"type": "string"},
                "values": {"type": "object", "description": "Input key to value; omitted inputs take their default."},
                "wait": {"type": "boolean", "description": "False queues and returns at once."}
            }),
            &["id"]
        ),
        tool(
            "comfy_job_status",
            "The state of the current or last job, with its progress and outputs.",
            &json!({}),
            &[]
        ),
        tool("comfy_cancel_job", "Stop the job that is running or waiting.", &json!({}), &[]),
        tool(
            "comfy_result",
            "Keep waiting for the running job and send its files to this chat when it ends.",
            &json!({}),
            &[]
        ),
        tool(
            "comfy_upload_input",
            "Copy a local image, audio or video into ComfyUI's inputs; returns the name a loader node uses.",
            &json!({"path": {"type": "string"}, "name": {"type": "string"}}),
            &["path"]
        ),
        tool("list_models", "List the installed checkpoints, LoRAs and VAEs.", &json!({}), &[]),
        tool(
            "list_nodes",
            "List ComfyUI node names, optionally only those containing `query`.",
            &json!({"query": {"type": "string"}}),
            &[]
        ),
        tool("example_workflow", "A working text-to-image workflow in API format to start from.", &json!({}), &[]),
        tool(
            "run_workflow",
            "Run a workflow (ComfyUI API format): waits for it and sends its files to this chat. A slow one returns still running.",
            &json!({"workflow": {"type": "object", "description": "Numbered nodes with class_type and inputs."}}),
            &["workflow"],
        ),
        tool(
            "upload_image",
            "Copy a local image into ComfyUI's inputs; returns the name for a LoadImage node.",
            &json!({"path": {"type": "string"}, "name": {"type": "string"}}),
            &["path"],
        ),
        tool(
            "wait_for_job",
            "Keep waiting for the running job; its files are sent to this chat when it ends.",
            &json!({}),
            &[]
        ),
        tool(
            "job_status",
            "The state of the current or last job, without waiting: queued, running, done, cancelled or error.",
            &json!({}),
            &[]
        ),
        tool("cancel_job", "Stop the job that is running or waiting.", &json!({}), &[]),
    ])
}

fn tool(name: &str, description: &str, properties: &Value, required: &[&str]) -> Value {
    let read_only = matches!(
        name,
        "list_models"
            | "list_nodes"
            | "example_workflow"
            | "comfy_list_templates"
            | "comfy_describe_template"
            | "comfy_job_status"
            | "job_status"
    );
    json!({
        "name": name,
        "description": description,
        "inputSchema": {"type": "object", "properties": properties, "required": required},
        "annotations": {"readOnlyHint": read_only, "idempotentHint": read_only},
    })
}

/// One tool call from `bot`.
pub async fn call(hub: &Arc<Hub>, bot: &str, name: &str, args: &Value) -> Result<Value> {
    if name == "example_workflow" {
        return serde_json::from_str(EXAMPLE).context("the bundled example workflow");
    }
    let jobs = hub.tools.comfy_jobs();
    match name {
        "comfy_list_templates" => Ok(list_templates(args)),
        "comfy_describe_template" => describe_template(hub, args).await,
        "comfy_run_template" => run_template(hub, bot, args).await,
        "comfy_job_status" | "job_status" => jobs.wait(hub, bot, Duration::ZERO).await,
        "comfy_cancel_job" | "cancel_job" => jobs.cancel(hub).await,
        "comfy_result" | "wait_for_job" => jobs.wait(hub, bot, WAIT_IN_CALL).await,
        "comfy_upload_input" | "upload_image" => upload(hub, args).await,
        "run_workflow" => {
            jobs.submit(hub, bot, &args["workflow"]).await?;
            jobs.wait(hub, bot, WAIT_IN_CALL).await
        }
        "list_models" => {
            let info = api(hub)?.object_info().await?;
            Ok(json!({
                "checkpoints": options(&info, "CheckpointLoaderSimple", "ckpt_name"),
                "loras": options(&info, "LoraLoader", "lora_name"),
                "vae": options(&info, "VAELoader", "vae_name"),
                "modelsFolder": comfy::checkpoints_dir(),
            }))
        }
        "list_nodes" => {
            let info = api(hub)?.object_info().await?;
            let query = args["query"].as_str().unwrap_or_default().to_lowercase();
            let names: Vec<&String> = info
                .as_object()
                .into_iter()
                .flat_map(|all| all.keys())
                .filter(|k| k.to_lowercase().contains(&query))
                .take(200)
                .collect();
            Ok(json!({ "nodes": names }))
        }
        other => bail!("unknown {CONNECTOR_ID} tool: {other}"),
    }
}

/// The templates a bot can choose from, filtered by `query` over name, tags and description.
fn list_templates(args: &Value) -> Value {
    let query = args["query"].as_str().unwrap_or_default().to_lowercase();
    let templates: Vec<Value> = templates::list()
        .into_iter()
        .filter(|template| {
            query.is_empty()
                || template.name.to_lowercase().contains(&query)
                || template.description.to_lowercase().contains(&query)
                || template.tags.iter().any(|tag| tag.to_lowercase().contains(&query))
        })
        .map(|template| template.summary())
        .collect();
    json!({"templates": templates, "problems": templates::problems()})
}

/// One template's form and its preflight. The preflight needs the server, so a stopped ComfyUI is a
/// problem in the report rather than an error here.
pub async fn describe_template(hub: &Arc<Hub>, args: &Value) -> Result<Value> {
    let template = templates::load(args["id"].as_str().context("`id` is required")?)?;
    let gpu = comfy::gpu();
    let running = hub.tools.comfy_running();
    let info = match running {
        true => api(hub)?.object_info().await.ok(),
        false => None,
    };
    let report = preflight::check(&template, running, info.as_ref());
    let mut described = template.describe(gpu.as_ref().map(|g| g.name.as_str()));
    described["preflight"] = report;
    Ok(described)
}

/// Runs a named template: its form values, uploaded inputs, preflight, graph and job.
pub async fn run_template(hub: &Arc<Hub>, bot: &str, args: &Value) -> Result<Value> {
    let id = args["id"].as_str().context("`id` is required")?;
    let template = templates::load(id)?;
    let empty = Map::new();
    let mut values = template.values(args.get("values").and_then(Value::as_object).unwrap_or(&empty))?;

    let running = hub.tools.comfy_running();
    let api = if running { Some(api(hub)?) } else { None };
    let info = match &api {
        Some(api) => api.object_info().await.ok(),
        None => None,
    };
    let report = preflight::check(&template, running, info.as_ref());

    // A picture the form points at is uploaded before the check: its name must be a file the server
    // already has, and the upload is the only step that needs the local path.
    if let Some(api) = &api {
        upload_inputs(api, &template, &mut values).await?;
    }
    if report["ok"] != true {
        return Ok(json!({
            "ok": false,
            "problems": report["problems"],
            "warnings": report["warnings"],
            "estimate": report["estimate"],
            "note": "nothing was queued: resolve the problems, then run it again",
        }));
    }

    let graph = template.graph(&values)?;
    let jobs = hub.tools.comfy_jobs();
    let job = jobs.submit_for(hub, bot, Some(id), &graph).await?;
    if args["wait"].as_bool() == Some(false) {
        return Ok(json!({"ok": true, "job": job, "estimate": report["estimate"]}));
    }
    let mut status = jobs.wait(hub, bot, WAIT_IN_CALL).await?;
    status["template"] = json!(id);
    status["estimate"] = report["estimate"].clone();
    Ok(status)
}

/// Replaces each image input's local path with the name ComfyUI stored it under.
async fn upload_inputs(
    api: &Api,
    template: &Template,
    values: &mut std::collections::BTreeMap<String, Value>,
) -> Result<()> {
    for input in template.inputs.iter().filter(|i| i.kind == InputKind::Image) {
        let Some(value) = values.get(&input.key).and_then(Value::as_str) else { continue };
        let path = PathBuf::from(value);
        if !path.is_file() {
            // Already a name in ComfyUI's input folder.
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| crate::chat::files::name(n).ok())
            .context("the image needs a file name")?;
        let meta = tokio::fs::metadata(&path).await?;
        anyhow::ensure!(meta.len() <= MAX_UPLOAD, "the input must be at most 100 MB");
        let stored = api.upload_image(&name, tokio::fs::read(&path).await?).await?;
        values.insert(input.key.clone(), json!(stored));
    }
    Ok(())
}

/// The first output of the last finished job, ready for the card: its type, its MIME and a picture
/// (the file itself for an image, its ffmpeg poster for a video).
pub async fn job_preview(hub: &Arc<Hub>) -> Result<Value> {
    let Some(file) = hub.tools.comfy_jobs().first_file() else {
        return Ok(json!({ "image": null, "kind": null, "mime": null }));
    };
    let output = results::Output::describe(&file);
    let picture = match output.kind {
        "image" => data_url(&file, output.mime).await,
        _ => match &output.thumbnail {
            Some(poster) => data_url(poster, "image/jpeg").await,
            None => None,
        },
    };
    Ok(json!({
        "image": picture,
        "kind": output.kind,
        "mime": output.mime,
        "path": output.path.to_string_lossy(),
    }))
}

/// The first image of the last finished job as a data URL; kept for clients that only know images.
pub async fn job_image(hub: &Arc<Hub>) -> Result<Value> {
    let preview = job_preview(hub).await?;
    Ok(json!({ "image": preview["image"] }))
}

async fn data_url(path: &std::path::Path, mime: &str) -> Option<String> {
    use base64::Engine as _;
    let bytes = tokio::fs::read(path).await.ok()?;
    Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

fn api(hub: &Hub) -> Result<Api> {
    anyhow::ensure!(hub.tools.comfy_running(), "ComfyUI is not running: start it from Extensions › Apps");
    Ok(Api::new(hub.tools.comfy_port()))
}

/// The choices of one input of one node class.
fn options(info: &Value, class: &str, input: &str) -> Value {
    info[class]["input"]["required"][input][0].clone()
}

async fn upload(hub: &Arc<Hub>, args: &Value) -> Result<Value> {
    let path = PathBuf::from(args["path"].as_str().context("`path` is required")?);
    anyhow::ensure!(results::ingestible(&path), "upload takes a picture, an audio file or a video");
    let meta = tokio::fs::metadata(&path).await.with_context(|| format!("reading {}", path.display()))?;
    anyhow::ensure!(meta.is_file() && meta.len() <= MAX_UPLOAD, "the input must be a file of at most 100 MB");
    let name = match args["name"].as_str() {
        Some(name) => crate::chat::files::name(name)?,
        None => path.file_name().and_then(|n| n.to_str()).context("the path has no file name")?.to_owned(),
    };
    upload_bytes(hub, &name, tokio::fs::read(&path).await?).await
}

/// An input a client picked with its own file dialog: the bytes go straight to ComfyUI, with no
/// temporary file of the host's to clean up.
pub async fn upload_bytes(hub: &Arc<Hub>, name: &str, bytes: Vec<u8>) -> Result<Value> {
    anyhow::ensure!(
        !bytes.is_empty() && bytes.len() as u64 <= MAX_UPLOAD,
        "the input must be a file of at most 100 MB"
    );
    let name = crate::chat::files::name(name)?;
    let stored = api(hub)?.upload_image(&name, bytes).await?;
    Ok(json!({ "name": stored }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_inputs_a_loader_reads_are_uploaded() {
        use std::path::Path;
        assert!(results::ingestible(Path::new("C:/in/cat.PNG")) && results::ingestible(Path::new("take.wav")));
        assert!(
            !results::ingestible(Path::new("C:/Users/me/.ssh/id_ed25519"))
                && !results::ingestible(Path::new("notes.txt"))
        );
    }

    #[test]
    fn the_example_is_a_graph_of_known_nodes() {
        let graph: Value = serde_json::from_str(EXAMPLE).expect("valid JSON");
        assert_eq!(graph.as_object().expect("an object").len(), 7);
        assert_eq!(graph["9"]["class_type"], "SaveImage");
    }

    #[test]
    fn every_tool_has_a_name_and_a_schema() {
        for tool in tools().as_array().expect("a list") {
            assert!(tool["name"].is_string() && tool["inputSchema"]["type"] == "object");
        }
    }
}
