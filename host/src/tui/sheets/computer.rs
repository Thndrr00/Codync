use super::{Buffer, Rect, centered, footer, frame_box, input_line, put, row, s, theme, window};
use crate::tui::manage::computer::{ComputerMode, ComputerSheet};
use ratatui::widgets::{Paragraph, Widget, Wrap};
pub fn draw(buf: &mut Buffer, area: Rect, sheet: &ComputerSheet) {
    let palette = theme();
    let inner = frame_box(buf, centered(area, 105, 34), palette.text, palette.panel, Some(("Computer", palette.text)));
    if inner.height < 8 {
        return;
    }
    put(
        buf,
        inner.x,
        inner.y,
        inner.width,
        [
            "[Status] Files Terminal Activity",
            "Status [Files] Terminal Activity",
            "Status Files [Terminal] Activity",
            "Status Files Terminal [Activity]",
        ][sheet.tab],
        palette.text,
    );
    let body = Rect::new(inner.x, inner.y + 2, inner.width, inner.height.saturating_sub(5));
    if sheet.mode == ComputerMode::Path || sheet.mode == ComputerMode::Site {
        input_line(
            buf,
            body,
            body.y,
            if sheet.mode == ComputerMode::Path { &sheet.path } else { &sheet.site },
            if sheet.mode == ComputerMode::Path {
                "Relative workspace folder"
            } else {
                "HTTPS website (never the secret)"
            },
            true,
            true,
        );
    } else if sheet.mode == ComputerMode::ConfirmCommand {
        Paragraph::new(format!(
            "Run on the REAL HOST?\n\n{}\n\nEnter confirms; Esc cancels. Limit: 30 seconds, 64 KiB per stream.",
            sheet.command.text
        ))
        .wrap(Wrap { trim: false })
        .style(palette.panel)
        .render(body, buf);
    } else if sheet.tab == 0 {
        let permissions = &sheet.state["permissions"];
        let text = format!(
            "Workspace: {}\n\n0 Master computer access: {}\n1 Desktop and browser: {}\n2 Workspace files: {}\n3 Shell commands: {}\n\nThese gate Codync tools on the real host, not independent harness tools.\nRevocation cancels pending work and owned commands; completed effects remain.\n\nManual entry: {}\np: pause Codync tools, stop current bot work and type a secret yourself on the host.\nf: finish manual entry; a fresh bot snapshot is then required.\nVerify the website yourself. Never paste secrets in chat.",
            s(&sheet.state, "workspace"),
            sheet.state["enabled"],
            permissions["browser"],
            permissions["files"],
            permissions["shell"],
            sheet.state["screen"]["protectedEntry"]
        );
        let text = format!(
            "{text}\n\nResearch: {} · Slack: {}\nResearch keys: Windows Settings; Slack setup: local host.\nLocal Windows entry: b opens private browser; l fills saved login; c fills code.\nf closes it; h requests keeping sign-in in a blank window after closing all private pages. Unknown targets force whole-browser closure.",
            service_label(sheet.state["services"]["research"].as_str()),
            service_label(sheet.state["services"]["slack"].as_str())
        );
        Paragraph::new(text).wrap(Wrap { trim: false }).style(palette.panel).render(body, buf);
    } else if let Some(text) = &sheet.preview {
        Paragraph::new(text.as_str())
            .wrap(Wrap { trim: false })
            .scroll((sheet.scroll, 0))
            .style(palette.panel)
            .render(body, buf);
    } else if sheet.tab == 1 {
        put(
            buf,
            body.x,
            body.y,
            body.width,
            &format!("Folder: {} (press / to open; UTF-8 previews up to 128 KiB)", sheet.folder),
            palette.secondary,
        );
        let offset = window(sheet.cursor, usize::from(body.height.saturating_sub(1)));
        for (line, index) in (offset..sheet.rows.len()).take(usize::from(body.height.saturating_sub(1))).enumerate() {
            let entry = &sheet.rows[index];
            let y = body.y + 1 + u16::try_from(line).unwrap_or(0);
            let style = row(buf, body, y, index == sheet.cursor);
            put(
                buf,
                body.x,
                y,
                body.width,
                &format!(
                    "{}{}{}",
                    if entry["directory"] == true { "Folder: " } else { "" },
                    s(entry, "name"),
                    if entry["link"] == true { " (link; unavailable)" } else { "" }
                ),
                palette.text.patch(style),
            );
        }
    } else if sheet.tab == 2 {
        input_line(buf, body, body.y, &sheet.command, "Command on real host", true, true);
        if let Some(output) = &sheet.output {
            let text = format!("Exit: {}\n{}\n{}", output["exitCode"], s(output, "stdout"), s(output, "stderr"));
            Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .render(Rect::new(body.x, body.y + 2, body.width, body.height.saturating_sub(2)), buf);
        }
    } else {
        let text = sheet.state["activity"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|entry| format!("{} · {} · {}\n", s(entry, "action"), s(entry, "result"), entry["createdAt"]))
            .collect::<String>();
        Paragraph::new(if text.is_empty() { "No computer activity recorded." } else { &text })
            .wrap(Wrap { trim: false })
            .scroll((sheet.scroll, 0))
            .style(palette.panel)
            .render(body, buf);
    }
    footer(
        buf,
        inner,
        sheet
            .error
            .as_deref()
            .or(sheet.busy.then_some("Waiting for host…"))
            .or(sheet.running.then_some("Command running; Status → 3 revokes shell and stops owned processes.")),
        "Tab page · Ctrl+R refresh · ↑↓ select/scroll · Enter open/run/confirm · Esc back/close",
    );
}
fn service_label(value: Option<&str>) -> &'static str {
    match value {
        Some("configured") => "configured",
        Some("notConfigured") => "not configured",
        _ => "unavailable; unlock host credentials to check",
    }
}
