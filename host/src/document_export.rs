//! Exporting a document to a file a person can open elsewhere: HTML always, Word or PDF through a
//! conversion chain, and never a change to the document itself.
//!
//! The chain is the machine's own tools: `pandoc` writes HTML, Word (`.docx`) or PDF (through one of the
//! PDF engines it knows), and for PDF without either, a Word copy is handed to `LibreOffice` (`soffice`).
//! With no converter installed, HTML still works through a plain, escaped page, and Word or PDF say
//! which converter to install instead of failing vaguely.

use crate::hub::Hub;
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const FORMATS: &[&str] = &["html", "docx", "pdf"];
/// PDF engines pandoc can drive, in the order tried.
const PDF_ENGINES: &[&str] = &["typst", "weasyprint", "wkhtmltopdf", "xelatex", "lualatex", "pdflatex"];

/// `exportDocument {botId, id, format}`: writes a copy into the bot's `exports` folder and names the
/// converter that ran (or `plain` for the HTML fallback).
pub fn export(hub: &Hub, bot: &str, id: &str, format: &str) -> Result<Value> {
    ensure!(FORMATS.contains(&format), "format must be html, docx or pdf");
    let document =
        hub.store.document(id)?.filter(|document| document.bot_id == bot).context("unknown document for this bot")?;
    let target = path(bot, &document.title, id, format);
    let dir = target.parent().context("export has no folder")?;
    std::fs::create_dir_all(dir)?;
    let stem = stem(&document.title, id);
    let markdown = dir.join(format!("{stem}.md"));
    std::fs::write(&markdown, format!("# {}\n\n{}", document.title, document.body))?;
    let converter = match format {
        "html" => {
            if pandoc(&markdown, &target, &[]).is_ok() {
                "pandoc"
            } else {
                std::fs::write(&target, render_html(&document.title, &document.body))?;
                "plain"
            }
        }
        "docx" => {
            pandoc(&markdown, &target, &[])
                .context("Word export needs pandoc: install it from pandoc.org, or export HTML instead")?;
            "pandoc"
        }
        _ => match pdf(&markdown, &target) {
            Ok(converter) => converter,
            Err(_) => bail!(
                "PDF export needs pandoc with a PDF engine (typst, weasyprint, wkhtmltopdf, LaTeX) or \
                 LibreOffice: install one, or export Word or HTML instead"
            ),
        },
    };
    let _ = std::fs::remove_file(&markdown);
    Ok(json!({"path": target, "format": format, "converter": converter}))
}

/// A PDF through pandoc, or a Word copy handed to `LibreOffice` when pandoc has no engine for it.
fn pdf(source: &Path, target: &Path) -> Result<&'static str> {
    for engine in PDF_ENGINES {
        if pandoc(source, target, &["--pdf-engine", engine]).is_ok() {
            return Ok(engine);
        }
    }
    let docx = target.with_extension("docx");
    pandoc(source, &docx, &[])?;
    let converted = soffice(&docx, target);
    let _ = std::fs::remove_file(&docx);
    converted?;
    Ok("LibreOffice")
}

fn pandoc(source: &Path, target: &Path, extra: &[&str]) -> Result<()> {
    let mut command = Command::new("pandoc");
    command.arg(source);
    match target.extension().and_then(|ext| ext.to_str()) {
        Some("html") => command.args(["-t", "html5", "--standalone"]),
        Some("docx") => command.args(["-t", "docx"]),
        Some("pdf") => command.args(["-t", "pdf"]),
        _ => &mut command,
    };
    let status = command.args(extra).arg("-o").arg(target).status().context("pandoc isn't installed")?;
    ensure!(status.success(), "pandoc failed");
    Ok(())
}

fn soffice(source: &Path, target: &Path) -> Result<()> {
    let dir = target.parent().context("target has no folder")?;
    let status = Command::new("soffice")
        .args(["--headless", "--convert-to", "pdf", "--outdir"])
        .arg(dir)
        .arg(source)
        .status()
        .context("LibreOffice isn't installed")?;
    ensure!(status.success(), "LibreOffice failed");
    let produced = dir.join(format!("{}.pdf", source.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default()));
    if produced != target && produced.is_file() {
        std::fs::rename(produced, target)?;
    }
    Ok(())
}

/// The plain page with no converter: the title and the Markdown text escaped, so it always opens.
fn render_html(title: &str, body: &str) -> String {
    format!(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\"><title>{}</title></head>\n\
         <body><h1>{}</h1><pre style=\"white-space:pre-wrap\">{}</pre></body></html>\n",
        escape(title),
        escape(title),
        escape(body)
    )
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A file name from the title that stays inside the exports folder and under a readable length.
fn stem(title: &str, id: &str) -> String {
    let mut name: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '-' })
        .take(80)
        .collect();
    while name.ends_with('-') || name.ends_with('.') {
        name.pop();
    }
    if name.trim_matches('-').is_empty() {
        name = format!("document-{}", &id[..id.len().min(8)]);
    }
    name
}

/// Where an exported copy of `id` in `format` lives (the newest one wins).
pub fn path(bot: &str, title: &str, id: &str, format: &str) -> PathBuf {
    crate::service::data_dir().join("bots").join(bot).join("exports").join(format!("{}.{format}", stem(title, id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::identity::Identity;
    use crate::store::{DocumentDraft, Store};
    use std::sync::Arc;

    fn document(hub: &Arc<Hub>) {
        let config = serde_json::from_value(json!({"id": "b1", "name": "B", "backend": "claude"})).unwrap();
        hub.store.save_bot(&config).unwrap();
        let draft = DocumentDraft {
            id: None,
            parent_id: None,
            title: "Notes <draft> & plans".into(),
            body: "Use <this> & that.".into(),
            if_revision: 0,
        };
        hub.store.save_document("b1", draft).unwrap();
    }

    fn hub() -> (Arc<Hub>, PathBuf) {
        let dir = std::env::temp_dir().join(format!("codync-export-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(Path::new(":memory:")).unwrap();
        let identity = Identity::load_or_create(&dir).unwrap();
        (Hub::new(store, "h".into(), identity, "t".into(), 0), dir)
    }

    #[test]
    fn the_plain_page_escapes_the_title_and_the_text() {
        let page = render_html("A <b>", "x & y");
        assert!(page.contains("<title>A &lt;b&gt;</title>"));
        assert!(page.contains("<pre style=\"white-space:pre-wrap\">x &amp; y</pre>"));
    }

    #[test]
    fn a_file_name_stays_inside_the_folder_and_reads_like_the_title() {
        assert_eq!(stem("Notes <draft> & plans", "abcd1234"), "Notes--draft----plans");
        assert_eq!(stem("   ", "abcdefgh1"), "document-abcdefgh");
        assert_eq!(stem(&"n".repeat(200), "x"), "n".repeat(80));
    }

    #[test]
    fn html_export_writes_the_document_beside_its_name() {
        let (hub, dir) = hub();
        document(&hub);
        let id = hub.store.documents("b1", "", None, 10).unwrap().items[0].id.clone();
        let out = export(&hub, "b1", &id, "html").expect("html always exports");
        assert!(matches!(out["converter"].as_str(), Some("pandoc" | "plain")));
        let target = out["path"].as_str().unwrap();
        let written = std::fs::read_to_string(target).unwrap();
        assert!(written.contains("Notes &lt;draft&gt; &amp; plans"));
        assert!(path("b1", "Notes <draft> & plans", &id, "html").is_file());
        assert!(export(&hub, "b1", &id, "rtf").is_err());
        assert!(export(&hub, "b1", "unknown", "html").is_err());
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(crate::service::data_dir().join("bots/b1/exports"));
    }
}
