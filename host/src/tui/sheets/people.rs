//! The People sheet: everyone who uses this computer besides its owner, the devices each person
//! uses, and giving a device to a person or back to the owner.

use super::{Buffer, Rect, centered, footer, frame_box, hline, put, restyle, rput, theme, truncate, window};
use crate::tui::l10n::tr;
use crate::tui::manage::{PeopleSection, PeopleSheet, PeopleStep, PersonAssign};

pub fn draw(buf: &mut Buffer, area: Rect, sheet: &PeopleSheet) {
    let t = theme();
    let title = tr("People");
    let r = centered(area, 88, area.height.saturating_sub(4).clamp(14, 26));
    let inner = frame_box(buf, r, t.text, t.panel, Some((title.as_ref(), t.text)));
    if inner.height < 8 {
        return;
    }
    hline(buf, inner.x, inner.bottom() - 3, inner.width, t.dim.patch(t.panel));
    let body = Rect::new(inner.x, inner.y, inner.width, inner.height - 3);
    let keys = match &sheet.step {
        PeopleStep::List => list(buf, body, sheet),
        PeopleStep::Assign(assign) => picker(buf, body, assign),
    };
    footer(buf, inner, sheet.error.as_deref(), keys);
    if sheet.error.is_none() && sheet.busy {
        put(buf, inner.x, inner.bottom() - 2, inner.width, &tr("Saving…"), t.dim.patch(t.panel));
    }
}

/// The two lists. Returns the footer's key hints.
fn list(buf: &mut Buffer, body: Rect, sheet: &PeopleSheet) -> &'static str {
    let tall = body.height >= 18;
    let head: u16 = if tall { 2 } else { 1 };
    let reserved = head * 2 + 1;
    let room = usize::from(body.height.saturating_sub(reserved) / 2).max(1);
    let mut y = body.y;
    y = heading(
        buf,
        body,
        y,
        "People on this computer",
        "Someone who uses this computer with their own phone or their own bots.",
        tall,
    );
    y = people_rows(buf, body, y, sheet, room);
    y = (y + 1).min(body.bottom());
    y = heading(buf, body, y, "Assign a device", "A member's phone only sees and changes what is theirs.", tall);
    let _ = devices_rows(buf, body, y, sheet, room);
    match sheet.section {
        PeopleSection::People => "n add · ↵ rename · x remove · tab devices · esc",
        PeopleSection::Devices => "↵ assign · tab people · esc",
    }
}

fn heading(buf: &mut Buffer, body: Rect, y: u16, title: &str, note: &str, tall: bool) -> u16 {
    let t = theme();
    if y >= body.bottom() {
        return y;
    }
    put(buf, body.x, y, body.width, &tr(title), t.bold.patch(t.panel));
    if !tall || y + 1 >= body.bottom() {
        return y + 1;
    }
    put(buf, body.x, y + 1, body.width, &tr(note), t.secondary.patch(t.panel));
    y + 2
}

fn people_rows(buf: &mut Buffer, body: Rect, mut y: u16, sheet: &PeopleSheet, room: usize) -> u16 {
    let t = theme();
    let Some(people) = &sheet.people else {
        return line(buf, body, y, &tr("Loading…"), t.dim);
    };
    if people.is_empty() {
        return line(buf, body, y, &tr("No one else yet"), t.secondary);
    }
    let at = sheet.cursor.min(people.len().saturating_sub(1));
    for (i, person) in people.iter().enumerate().skip(window(at, room)).take(room) {
        if y >= body.bottom() {
            break;
        }
        let selected = i == at && sheet.section == PeopleSection::People;
        let id = person["id"].as_str().unwrap_or_default();
        let name = person["name"].as_str().unwrap_or_default();
        let email = person["email"].as_str().unwrap_or_default();
        let left = if email.is_empty() { name.to_owned() } else { format!("{name} · {email}") };
        let devices = sheet.devices_of(id);
        let right = if devices.is_empty() { tr("No devices").into_owned() } else { devices.join(", ") };
        person_row(buf, body, y, &left, &right, selected);
        y += 1;
    }
    y
}

fn devices_rows(buf: &mut Buffer, body: Rect, mut y: u16, sheet: &PeopleSheet, room: usize) -> u16 {
    let t = theme();
    let Some(devices) = &sheet.devices else {
        return line(buf, body, y, &tr("Loading…"), t.dim);
    };
    if devices.is_empty() {
        return line(buf, body, y, &tr("None yet"), t.secondary);
    }
    let at = sheet.cursor.min(devices.len().saturating_sub(1));
    for (i, device) in devices.iter().enumerate().skip(window(at, room)).take(room) {
        if y >= body.bottom() {
            break;
        }
        let selected = i == at && sheet.section == PeopleSection::Devices;
        let name = device["name"].as_str().unwrap_or_default();
        let person = device["person"].as_str().map_or_else(|| tr("This computer's owner").into_owned(), str::to_owned);
        device_row(buf, body, y, name, &person, selected);
        y += 1;
    }
    y
}

fn person_row(buf: &mut Buffer, body: Rect, y: u16, left: &str, right: &str, selected: bool) {
    let t = theme();
    let base = row_base(buf, body, y, selected);
    let half = body.width / 2;
    put(buf, body.x, y, 2, if selected { "▸" } else { " " }, t.secondary.patch(base));
    put(
        buf,
        body.x + 2,
        y,
        half.saturating_sub(2),
        &truncate(left, usize::from(half.saturating_sub(2))),
        t.bold.patch(base),
    );
    rput(buf, body.right(), y, &truncate(right, usize::from(half)), t.secondary.patch(base));
}

fn device_row(buf: &mut Buffer, body: Rect, y: u16, name: &str, person: &str, selected: bool) {
    let t = theme();
    let base = row_base(buf, body, y, selected);
    let half = body.width / 2;
    put(buf, body.x, y, 2, if selected { "▸" } else { " " }, t.secondary.patch(base));
    put(
        buf,
        body.x + 2,
        y,
        half.saturating_sub(2),
        &truncate(name, usize::from(half.saturating_sub(2))),
        t.bold.patch(base),
    );
    rput(buf, body.right(), y, &truncate(person, usize::from(half)), t.secondary.patch(base));
}

fn row_base(buf: &mut Buffer, body: Rect, y: u16, selected: bool) -> ratatui::style::Style {
    let t = theme();
    if selected {
        restyle(buf, Rect::new(body.x - 1, y, body.width + 2, 1), t.sel);
        t.sel
    } else {
        t.panel
    }
}

/// The person picker for one device: the owner first, then every person, the current one marked.
fn picker(buf: &mut Buffer, body: Rect, assign: &PersonAssign) -> &'static str {
    let t = theme();
    let mut y = body.y;
    put(buf, body.x, y, body.width, &tr("Assign a device"), t.bold.patch(t.panel));
    y += 1;
    if y < body.bottom() {
        put(buf, body.x, y, body.width, &assign.name, t.text.patch(t.panel));
        y += 2;
    }
    for (i, (id, name)) in assign.choices.iter().enumerate() {
        if y >= body.bottom() {
            break;
        }
        let selected = i == assign.cursor;
        let base = row_base(buf, body, y, selected);
        put(buf, body.x, y, 2, if selected { "▸" } else { " " }, t.secondary.patch(base));
        put(buf, body.x + 2, y, 2, if id == &assign.current { "●" } else { "○" }, t.secondary.patch(base));
        put(
            buf,
            body.x + 4,
            y,
            body.width.saturating_sub(4),
            &truncate(name, usize::from(body.width.saturating_sub(4))),
            t.text.patch(base),
        );
        y += 1;
    }
    "↑↓ choose · ↵ assign · esc back"
}

fn line(buf: &mut Buffer, body: Rect, y: u16, text: &str, style: ratatui::style::Style) -> u16 {
    if y < body.bottom() {
        put(buf, body.x, y, body.width, text, style.patch(theme().panel));
    }
    y + 1
}
