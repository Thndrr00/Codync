use super::{Buffer, Rect, centered, footer, frame_box, input_line, put, row, s, theme, window};
use crate::tui::manage::mcp_tools::{McpMode, McpToolsSheet};
use ratatui::widgets::{Paragraph, Widget, Wrap};

pub fn draw(buf: &mut Buffer, area: Rect, sheet: &McpToolsSheet) {
    let palette = theme();
    let title = format!("{} tools", sheet.name);
    let inner = frame_box(buf, centered(area, 100, 32), palette.text, palette.panel, Some((&title, palette.text)));
    if inner.height < 7 {
        return;
    }
    let body = Rect::new(inner.x, inner.y + 3, inner.width, inner.height.saturating_sub(6));
    put(
        buf,
        inner.x,
        inner.y,
        inner.width,
        if sheet.tab == 0 {
            "Tool policies · unknown: Ask first · observed read-only: Allow"
        } else {
            "Current requests · refresh for current status"
        },
        palette.text,
    );
    put(
        buf,
        inner.x,
        inner.y + 1,
        inner.width,
        "HTTP, local MCP and Composio · all bots on this connection",
        palette.secondary,
    );
    let keys = if sheet.mode == McpMode::Receipt {
        input_line(buf, body, body.y, &sheet.tool, "Saved request ID (retrieval never executes)", true, true);
        "Enter retrieve saved result · Esc cancel"
    } else if sheet.mode == McpMode::Revoke || sheet.mode == McpMode::Reset {
        let text = if sheet.mode == McpMode::Reset {
            "Restore server default and revoke this tool's earlier authorizations?"
        } else {
            "Revoke current requests for ALL bots? Future requests retain their policies."
        };
        Paragraph::new(format!("{text}\n\nEffects already sent cannot be undone. Credentials remain installed."))
            .wrap(Wrap { trim: false })
            .style(palette.panel)
            .render(body, buf);
        "Enter confirm · Esc cancel"
    } else if let McpMode::Edit(policy_focus) = sheet.mode {
        input_line(buf, body, body.y, &sheet.tool, "Exact tool name", !policy_focus, true);
        let modes = ["Ask first", "Allow", "Deny"];
        put(
            buf,
            body.x,
            body.y + 2,
            body.width,
            &format!("Policy: {}{}", modes[sheet.policy], if policy_focus { " ←→" } else { "" }),
            palette.text,
        );
        "Tab field · ←→ policy · Enter save and revoke earlier authorizations · Esc cancel"
    } else if let Some(detail) = &sheet.detail {
        let arguments = serde_json::to_string_pretty(&detail["arguments"]).unwrap_or_default();
        let text = format!(
            "{} · bot {} · {}\nRequest ID: {}\n\n{arguments}",
            s(detail, "tool"),
            s(detail, "botId"),
            s(detail, "status"),
            s(detail, "id")
        );
        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .scroll((sheet.scroll, 0))
            .style(palette.panel)
            .render(body, buf);
        if s(detail, "status") == "pending" {
            "↑↓ scroll exact arguments · a approve & run · d decline · Esc back"
        } else {
            "↑↓ scroll saved result · Esc back"
        }
    } else {
        if sheet.rows.is_empty() {
            put(
                buf,
                body.x,
                body.y,
                body.width,
                if sheet.busy {
                    "Loading…"
                } else if sheet.tab == 0 {
                    "No observed tools or overrides. Unknown tools ask first. Press n to add a policy."
                } else {
                    "No current requests."
                },
                palette.secondary,
            );
        }
        let offset = window(sheet.cursor, usize::from(body.height) / 2);
        for (line, index) in (offset..sheet.rows.len()).take(usize::from(body.height) / 2).enumerate() {
            let value = &sheet.rows[index];
            let y = body.y + u16::try_from(line * 2).unwrap_or(0);
            let style = row(buf, body, y, index == sheet.cursor);
            put(buf, body.x, y, body.width, s(value, "tool"), palette.text.patch(style));
            let subtitle = if sheet.tab == 0 {
                s(value, "mode").to_owned()
            } else {
                format!("{} · bot {}", s(value, "status"), s(value, "botId"))
            };
            put(buf, body.x, y + 1, body.width, &subtitle, palette.secondary.patch(style));
        }
        "Tab policies/requests · ↑↓ select · Enter inspect/edit · n new policy · Del reset · l more · g saved result · r refresh · Ctrl+X revoke all · Esc close"
    };
    footer(buf, inner, sheet.error.as_deref().or(sheet.busy.then_some("Waiting for host…")), keys);
}
