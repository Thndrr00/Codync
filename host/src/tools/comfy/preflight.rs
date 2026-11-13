//! The checks before a template is queued: the server is up, every node class is installed, every
//! model is on disk at the size the manifest declares, the graphics card has the memory the estimate
//! wants, and the disk has room for the output. Each problem comes with the action that fixes it, and
//! a job never enters the queue while one is unresolved.

use super::templates::{Model, Template};
use crate::tools::comfy;
use serde_json::{Value, json};

/// What a run needs and what is missing, for the card and the bot.
#[allow(
    clippy::cast_precision_loss,
    reason = "a megabyte count never needs the 52 bits a f64 keeps, and the GB figure is only shown"
)]
pub fn check(template: &Template, running: bool, info: Option<&Value>) -> Value {
    let gpu = comfy::gpu();
    let gpu_name = gpu.as_ref().map(|g| g.name.as_str());
    let mut problems = Vec::new();
    let mut warnings = Vec::new();

    if !running {
        problems.push(problem(
            "serverStopped",
            "ComfyUI is not running. Start it from Extensions › Apps.",
            &json!({"kind": "start"}),
        ));
    }
    if let Some(info) = info {
        for class in template.classes() {
            if info.get(&class).is_none() {
                problems.push(problem(
                    "nodeMissing",
                    format!("This template needs the node `{class}`, which is not installed. Install it in ComfyUI, then try again."),
                    &json!({"kind": "installNode", "node": class}),
                ));
            }
        }
    }

    for model in &template.models {
        if let Some(issue) = model_issue(model) {
            problems.push(issue);
        }
    }

    let seconds = template.estimate.seconds_for(gpu_name);
    let vram = template.estimate.vram_for(gpu_name);
    if let (Some(need), Some(gpu)) = (vram, gpu.as_ref())
        && need > 0.0
        && (gpu.free_mb as f64 / 1024.0) < need
    {
        problems.push(problem(
            "vram",
            format!(
                "This template wants about {need:.1} GB of graphics memory and {} has {:.1} GB free. Lower the size, or close what is using the card.",
                gpu.name,
                gpu.free_mb as f64 / 1024.0
            ),
            &json!({"kind": "lowerResolution"}),
        ));
    }
    if gpu.is_none() && vram.is_some_and(|need| need > 0.0) {
        warnings.push(problem(
            "cpu",
            "No NVIDIA graphics card was found: this runs on the processor, which takes minutes instead of seconds.",
            &json!({"kind": "useCpu"}),
        ));
    }

    let output_gb = template.estimate.output_gb.unwrap_or(1.0);
    if let Some(free) = comfy::free_space(&comfy::output_dir())
        && (free as f64) < output_gb * 1e9
    {
        problems.push(problem(
            "disk",
            format!("This output needs about {output_gb:.1} GB and the disk has {:.1} GB free.", free as f64 / 1e9),
            &json!({"kind": "freeSpace"}),
        ));
    }

    json!({
        "ok": problems.is_empty(),
        "problems": problems,
        "warnings": warnings,
        "estimate": {
            "seconds": seconds,
            "vramGb": vram,
            "vramTotalGb": gpu.as_ref().map(|g| g.total_mb as f64 / 1024.0),
            "gpu": gpu_name,
            "outputGb": output_gb,
        },
    })
}

/// The model's own problem: absent, or a different size than the manifest promised.
fn model_issue(model: &Model) -> Option<Value> {
    let path = model.path().ok()?;
    let meta = std::fs::metadata(&path);
    let present = meta.as_ref().is_ok_and(std::fs::Metadata::is_file);
    if present && (model.size == 0 || meta.is_ok_and(|m| m.len() == model.size)) {
        return None;
    }
    let code = if present { "modelIncomplete" } else { "modelMissing" };
    let message = if present {
        format!("`{}` is on disk but is not the size this template expects. Download it again.", model.file)
    } else if model.gated {
        format!(
            "`{}` comes from a gated Hugging Face repository: open its page, sign in and accept its licence, then put the file in the {} folder.",
            model.file, model.folder
        )
    } else {
        format!("`{}` is not downloaded. It goes in the {} folder.", model.file, model.folder)
    };
    let action = if model.gated {
        json!({"kind": "openUrl", "url": model.url, "license": model.license})
    } else {
        download_action(model)
    };
    Some(problem(code, message, &action))
}

fn download_action(model: &Model) -> Value {
    json!({
        "kind": "download",
        "folder": model.folder,
        "file": model.file,
        "url": model.url,
        "size": model.size,
        "license": model.license,
        "gated": model.gated,
    })
}

fn problem(code: &str, message: impl Into<String>, action: &Value) -> Value {
    json!({"code": code, "message": message.into(), "action": action})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Template {
        serde_json::from_value(json!({
            "id": "test",
            "name": "Test",
            "version": "1.0.0",
            "description": "A test",
            "workflow": {"1": {"class_type": "CheckpointLoaderSimple", "inputs": {"ckpt_name": "not-installed.safetensors"}}},
            "inputs": [],
            "models": [{
                "file": "not-installed.safetensors",
                "folder": "checkpoints",
                "size": 1000,
                "url": "https://example.test/model.safetensors"
            }],
            "output": {"type": "image"},
            "estimate": {"seconds": 5, "vram_gb": 0.0, "output_gb": 0.001}
        }))
        .expect("a valid manifest")
    }

    #[test]
    fn a_stopped_server_and_a_missing_model_are_problems_with_actions() {
        let report = check(&manifest(), false, None);
        assert_eq!(report["ok"], false);
        let codes: Vec<&str> =
            report["problems"].as_array().expect("a list").iter().filter_map(|p| p["code"].as_str()).collect();
        assert!(codes.contains(&"serverStopped") && codes.contains(&"modelMissing"), "{report}");
        let download = report["problems"]
            .as_array()
            .expect("a list")
            .iter()
            .find(|p| p["code"] == "modelMissing")
            .expect("the model problem");
        assert_eq!(download["action"]["kind"], "download");
        assert_eq!(download["action"]["url"], "https://example.test/model.safetensors");
    }

    #[test]
    fn a_gated_model_opens_its_page_instead_of_downloading() {
        let mut manifest = manifest();
        manifest.models[0].gated = true;
        manifest.models[0].url = Some("https://huggingface.co/owner/repo".into());
        manifest.models[0].sha256 = None;
        let report = check(&manifest, false, None);
        let problem = report["problems"]
            .as_array()
            .expect("a list")
            .iter()
            .find(|p| p["code"] == "modelMissing")
            .expect("the gated model");
        assert_eq!(problem["action"]["kind"], "openUrl");
        assert_eq!(problem["action"]["url"], "https://huggingface.co/owner/repo");
        assert!(problem["message"].as_str().is_some_and(|m| m.contains("sign in")), "{problem}");
    }

    #[test]
    fn an_unknown_node_is_named_for_installing() {
        let info = json!({});
        let report = check(&manifest(), true, Some(&info));
        let missing = report["problems"]
            .as_array()
            .expect("a list")
            .iter()
            .find(|p| p["code"] == "nodeMissing")
            .expect("the node problem");
        assert_eq!(missing["action"]["node"], "CheckpointLoaderSimple");
    }
}
