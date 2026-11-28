//! A bot's Insights page: the host-computed summary of its routines, model and daily use.

use serde_json::Value;

use super::{App, Buffer, Rect, centered, footer, frame_box, name_of, page_tabs, put, theme};
use crate::tui::l10n::tr;
use crate::tui::manage::{Insights, Page};

pub fn draw(buf: &mut Buffer, area: Rect, app: &App, i: &Insights) {
    let t = theme();
    let r = centered(area, 76, 24);
    let title = name_of(app, &i.bot);
    let inner = frame_box(buf, r, t.text, t.panel, Some((&title, t.text)));
    let inner = page_tabs(buf, inner, Page::Insights);
    let Some(data) = i.data.as_ref().filter(|v| !v.is_null()) else {
        put(buf, inner.x, inner.y, inner.width, &tr("LoadingÃ¢â‚¬Â¦"), t.dim.patch(t.panel));
        footer(buf, inner, None, "esc Ã‚Â· shift+Ã¢â€ ÂÃ¢â€ â€™ pages");
        return;
    };

    let mut y = inner.y;
    let line = |buf: &mut Buffer, y: &mut u16, text: String, style| {
        put(buf, inner.x, *y, inner.width, &text, style);
        *y += 1;
    };

    let provider = data["provider"].as_str().unwrap_or("");
    let description = data["description"].as_str().unwrap_or("");
    if !description.is_empty() {
        put(
            buf,
            inner.x,
            y,
            inner.width,
            &crate::tui::md::truncate(description, inner.width as usize),
            t.secondary.patch(t.panel),
        );
        y += 2;
    }
    let model = data["model"].as_str().unwrap_or("");
    if !provider.is_empty() || !model.is_empty() {
        line(
            buf,
            &mut y,
            format!("{}: {}", tr("Agent"), if provider.is_empty() { model } else { provider }),
            t.bold.patch(t.panel),
        );
        if !model.is_empty() && !provider.is_empty() {
            line(buf, &mut y, format!("{}: {}", tr("Model"), model), t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let used = data["usage"]["usedText"].as_str().unwrap_or("0");
    let limit = data["usage"]["limit"].as_u64();
    let usage = match limit {
        Some(limit) => format!("{}: {} / {limit}", tr("Tokens"), used),
        None => format!("{}: {}", tr("Tokens"), used),
    };
    line(buf, &mut y, usage, t.text.patch(t.panel));
    line(
        buf,
        &mut y,
        format!("{}: {}", tr("Skills"), data["skills"]["total"].as_u64().unwrap_or(0)),
        t.secondary.patch(t.panel),
    );
    y += 1;

    for field in ["mail", "media", "replypolicy", "notices"] {
        let section = &data[field];
        if section.is_object() {
            let title = match field {
                "mail" => "Mail",
                "media" => "Media",
                "replypolicy" => "Reply policy",
                _ => "Notices",
            };
            line(buf, &mut y, tr(title).into_owned(), t.bold.patch(t.panel));
            let headline = section["headline"].as_str().unwrap_or("");
            line(buf, &mut y, headline.to_owned(), t.text.patch(t.panel));
            y += 1;
        }
    }

    let closed = &data["closed"];
    if closed.is_object() {
        line(buf, &mut y, tr("Closed island").into_owned(), t.bold.patch(t.panel));
        let headline = closed["headline"].as_str().unwrap_or("");
        let music = closed["music"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {music}"), t.text.patch(t.panel));
        y += 1;
    }

    let anim = &data["anim"];
    if anim.is_object() {
        line(buf, &mut y, tr("Animation").into_owned(), t.bold.patch(t.panel));
        let headline = anim["headline"].as_str().unwrap_or("");
        let line_text = anim["line"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {line_text}"), t.text.patch(t.panel));
        y += 1;
    }

    let spotify = &data["spotify"];
    if spotify.is_object() {
        line(buf, &mut y, tr("Spotify").into_owned(), t.bold.patch(t.panel));
        let headline = spotify["headline"].as_str().unwrap_or("");
        let track = spotify["track"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {track}"), t.text.patch(t.panel));
        y += 1;
    }

    let mochidesktop = &data["mochidesktop"];
    if mochidesktop.is_object() {
        line(buf, &mut y, tr("Mochi desktop").into_owned(), t.bold.patch(t.panel));
        let headline = mochidesktop["headline"].as_str().unwrap_or("");
        let line_text = mochidesktop["line"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {line_text}"), t.text.patch(t.panel));
        y += 1;
    }

    let whatsnew = &data["whatsnew"];
    if whatsnew.is_object() {
        line(buf, &mut y, tr("What's new").into_owned(), t.bold.patch(t.panel));
        let headline = whatsnew["headline"].as_str().unwrap_or("");
        let version = whatsnew["version"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {version}"), t.text.patch(t.panel));
        y += 1;
    }

    let recap = &data["recap"];
    if recap.is_object() {
        line(buf, &mut y, tr("Weekly recap").into_owned(), t.bold.patch(t.panel));
        let headline = recap["headline"].as_str().unwrap_or("");
        let duration = recap["duration"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {duration}"), t.text.patch(t.panel));
        y += 1;
    }

    let islandlayout = &data["islandlayout"];
    if islandlayout.is_object() {
        line(buf, &mut y, tr("Island layout").into_owned(), t.bold.patch(t.panel));
        let headline = islandlayout["headline"].as_str().unwrap_or("");
        let panel = islandlayout["panel"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {panel}"), t.text.patch(t.panel));
        y += 1;
    }

    let coucoushortcuts = &data["coucoushortcuts"];
    if coucoushortcuts.is_object() {
        line(buf, &mut y, tr("Shortcuts").into_owned(), t.bold.patch(t.panel));
        let headline = coucoushortcuts["headline"].as_str().unwrap_or("");
        let keys = coucoushortcuts["keys"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {keys}"), t.text.patch(t.panel));
        y += 1;
    }

    let pills = &data["pills"];
    if pills.is_object() {
        line(buf, &mut y, tr("Pills").into_owned(), t.bold.patch(t.panel));
        let headline = pills["headline"].as_str().unwrap_or("");
        let main = pills["main"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {main}"), t.text.patch(t.panel));
        y += 1;
    }

    let coucoumail = &data["coucoumail"];
    if coucoumail.is_object() {
        line(buf, &mut y, tr("Mail").into_owned(), t.bold.patch(t.panel));
        let headline = coucoumail["headline"].as_str().unwrap_or("");
        let rule = coucoumail["rule"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {rule}"), t.text.patch(t.panel));
        y += 1;
    }

    let whatsapp = &data["whatsapp"];
    if whatsapp.is_object() {
        line(buf, &mut y, tr("WhatsApp").into_owned(), t.bold.patch(t.panel));
        let headline = whatsapp["headline"].as_str().unwrap_or("");
        let preview = whatsapp["preview"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {preview}"), t.text.patch(t.panel));
        y += 1;
    }

    let aurora = &data["aurora"];
    if aurora.is_object() {
        line(buf, &mut y, tr("Aurora background").into_owned(), t.bold.patch(t.panel));
        let headline = aurora["headline"].as_str().unwrap_or("");
        let ground = aurora["ground"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {ground}"), t.text.patch(t.panel));
        y += 1;
    }

    let github = &data["github"];
    if github.is_object() {
        line(buf, &mut y, tr("GitHub").into_owned(), t.bold.patch(t.panel));
        let headline = github["headline"].as_str().unwrap_or("");
        let prs = github["prs"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {prs}"), t.text.patch(t.panel));
        y += 1;
    }

    let toollabels = &data["toollabels"];
    if toollabels.is_object() {
        line(buf, &mut y, tr("Tool labels").into_owned(), t.bold.patch(t.panel));
        let headline = toollabels["headline"].as_str().unwrap_or("");
        let key = toollabels["key"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {key}"), t.text.patch(t.panel));
        y += 1;
    }

    let contextblocks = &data["contextblocks"];
    if contextblocks.is_object() {
        line(buf, &mut y, tr("Context blocks").into_owned(), t.bold.patch(t.panel));
        let headline = contextblocks["headline"].as_str().unwrap_or("");
        let turns = contextblocks["turns"].as_u64().unwrap_or(0);
        line(buf, &mut y, format!("{headline} · {turns}"), t.text.patch(t.panel));
        y += 1;
    }

    let work = &data["work"];
    if work.is_object() {
        line(buf, &mut y, tr("Work").into_owned(), t.bold.patch(t.panel));
        let headline = work["headline"].as_str().unwrap_or("");
        let duration = work["duration"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {duration}"), t.text.patch(t.panel));
        y += 1;
    }

    let channelaccess = &data["channelaccess"];
    if channelaccess.is_object() {
        line(buf, &mut y, tr("Channel access").into_owned(), t.bold.patch(t.panel));
        let headline = channelaccess["headline"].as_str().unwrap_or("");
        let summary_line = channelaccess["summaryLine"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {summary_line}"), t.text.patch(t.panel));
        y += 1;
    }

    let layout = &data["layout"];
    if layout.is_object() {
        line(buf, &mut y, tr("Layout").into_owned(), t.bold.patch(t.panel));
        let headline = layout["headline"].as_str().unwrap_or("");
        let active = layout["active"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {active}"), t.text.patch(t.panel));
        y += 1;
    }

    let draftstash = &data["draftstash"];
    if draftstash.is_object() {
        line(buf, &mut y, tr("Draft stash").into_owned(), t.bold.patch(t.panel));
        let headline = draftstash["headline"].as_str().unwrap_or("");
        let title = draftstash["title"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {title}"), t.text.patch(t.panel));
        y += 1;
    }

    let service = &data["service"];
    if service.is_object() {
        line(buf, &mut y, tr("Service").into_owned(), t.bold.patch(t.panel));
        let headline = service["headline"].as_str().unwrap_or("");
        let unit = service["unit"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {unit}"), t.text.patch(t.panel));
        y += 1;
    }

    let plugins = &data["plugins"];
    if plugins.is_object() {
        line(buf, &mut y, tr("Plugins").into_owned(), t.bold.patch(t.panel));
        let headline = plugins["headline"].as_str().unwrap_or("");
        let root = plugins["root"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {root}"), t.text.patch(t.panel));
        y += 1;
    }

    let csiterm = &data["csiterm"];
    if csiterm.is_object() {
        line(buf, &mut y, tr("Terminal sequences").into_owned(), t.bold.patch(t.panel));
        let headline = csiterm["headline"].as_str().unwrap_or("");
        let scroll = csiterm["scroll"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {scroll}"), t.text.patch(t.panel));
        y += 1;
    }

    let promptactions = &data["promptactions"];
    if promptactions.is_object() {
        line(buf, &mut y, tr("Prompt actions").into_owned(), t.bold.patch(t.panel));
        let headline = promptactions["headline"].as_str().unwrap_or("");
        let surface = promptactions["surface"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {surface}"), t.text.patch(t.panel));
        y += 1;
    }

    let botstore = &data["botstore"];
    if botstore.is_object() {
        line(buf, &mut y, tr("Bot store").into_owned(), t.bold.patch(t.panel));
        let headline = botstore["headline"].as_str().unwrap_or("");
        let folder = botstore["folder"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {folder}"), t.text.patch(t.panel));
        y += 1;
    }

    let modelprefs = &data["modelprefs"];
    if modelprefs.is_object() {
        line(buf, &mut y, tr("Model preferences").into_owned(), t.bold.patch(t.panel));
        let headline = modelprefs["headline"].as_str().unwrap_or("");
        let default = modelprefs["modelDefault"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {default}"), t.text.patch(t.panel));
        y += 1;
    }

    let router = &data["router"];
    if router.is_object() {
        line(buf, &mut y, tr("Router").into_owned(), t.bold.patch(t.panel));
        let headline = router["headline"].as_str().unwrap_or("");
        let strategy = router["strategy"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {strategy}"), t.text.patch(t.panel));
        y += 1;
    }

    let cmdpolicy = &data["cmdpolicy"];
    if cmdpolicy.is_object() {
        line(buf, &mut y, tr("Command policy").into_owned(), t.bold.patch(t.panel));
        let headline = cmdpolicy["headline"].as_str().unwrap_or("");
        let verdict = cmdpolicy["verdict"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {verdict}"), t.text.patch(t.panel));
        y += 1;
    }

    let missions = &data["missions"];
    if missions.is_object() {
        line(buf, &mut y, tr("Missions").into_owned(), t.bold.patch(t.panel));
        let headline = missions["headline"].as_str().unwrap_or("");
        let span = missions["span"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {span}"), t.text.patch(t.panel));
        y += 1;
    }

    let noai = &data["noai"];
    if noai.is_object() {
        line(buf, &mut y, tr("No AI").into_owned(), t.bold.patch(t.panel));
        let headline = noai["headline"].as_str().unwrap_or("");
        let state = noai["state"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {state}"), t.text.patch(t.panel));
        y += 1;
    }

    let devices = &data["devices"];
    if devices.is_object() {
        line(buf, &mut y, tr("Devices").into_owned(), t.bold.patch(t.panel));
        let headline = devices["headline"].as_str().unwrap_or("");
        let look = devices["look"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {look}"), t.text.patch(t.panel));
        y += 1;
    }

    let vault = &data["vault"];
    if vault.is_object() {
        line(buf, &mut y, tr("Vault").into_owned(), t.bold.patch(t.panel));
        let headline = vault["headline"].as_str().unwrap_or("");
        let title = vault["title"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {title}"), t.text.patch(t.panel));
        y += 1;
    }

    let toolreplay = &data["toolreplay"];
    if toolreplay.is_object() {
        line(buf, &mut y, tr("Tool replay").into_owned(), t.bold.patch(t.panel));
        let headline = toolreplay["headline"].as_str().unwrap_or("");
        let outcome = toolreplay["outcome"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {outcome}"), t.text.patch(t.panel));
        y += 1;
    }

    let secretcleanup = &data["secretcleanup"];
    if secretcleanup.is_object() {
        line(buf, &mut y, tr("Secret cleanup").into_owned(), t.bold.patch(t.panel));
        let headline = secretcleanup["headline"].as_str().unwrap_or("");
        let kind = secretcleanup["kind"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {kind}"), t.text.patch(t.panel));
        y += 1;
    }

    let avatar = &data["avatar"];
    if avatar.is_object() {
        line(buf, &mut y, tr("Avatar").into_owned(), t.bold.patch(t.panel));
        let headline = avatar["headline"].as_str().unwrap_or("");
        let hex = avatar["hex"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {hex}"), t.text.patch(t.panel));
        y += 1;
    }

    let providers = &data["providers"];
    if providers.is_object() {
        line(buf, &mut y, tr("Providers").into_owned(), t.bold.patch(t.panel));
        let headline = providers["headline"].as_str().unwrap_or("");
        let guess = providers["guess"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {guess}"), t.text.patch(t.panel));
        y += 1;
    }

    let plans = &data["plans"];
    if plans.is_object() {
        line(buf, &mut y, tr("Coding plans").into_owned(), t.bold.patch(t.panel));
        let headline = plans["headline"].as_str().unwrap_or("");
        let choice = plans["choice"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {choice}"), t.text.patch(t.panel));
        y += 1;
    }

    let accounts = &data["accounts"];
    if accounts.is_object() {
        line(buf, &mut y, tr("Accounts").into_owned(), t.bold.patch(t.panel));
        let headline = accounts["headline"].as_str().unwrap_or("");
        let state = accounts["state"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {state}"), t.text.patch(t.panel));
        y += 1;
    }

    let engine = &data["engine"];
    if engine.is_object() {
        line(buf, &mut y, tr("Engine").into_owned(), t.bold.patch(t.panel));
        let headline = engine["headline"].as_str().unwrap_or("");
        let state = engine["state"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {state}"), t.text.patch(t.panel));
        y += 1;
    }

    let brainpicker = &data["brainpicker"];
    if brainpicker.is_object() {
        line(buf, &mut y, tr("Brain picker").into_owned(), t.bold.patch(t.panel));
        let headline = brainpicker["headline"].as_str().unwrap_or("");
        let description = brainpicker["description"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {description}"), t.text.patch(t.panel));
        y += 1;
    }

    let approvalrules = &data["approvalrules"];
    if approvalrules.is_object() {
        line(buf, &mut y, tr("Approval rules").into_owned(), t.bold.patch(t.panel));
        let headline = approvalrules["headline"].as_str().unwrap_or("");
        let tool = approvalrules["tool"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {tool}"), t.text.patch(t.panel));
        y += 1;
    }

    let iteration = &data["iteration"];
    if iteration.is_object() {
        line(buf, &mut y, tr("Iteration").into_owned(), t.bold.patch(t.panel));
        let headline = iteration["headline"].as_str().unwrap_or("");
        let ghosts = iteration["ghosts"].as_u64().unwrap_or(0);
        line(buf, &mut y, format!("{headline} · {}: {ghosts}", tr("Ghosts")), t.text.patch(t.panel));
        y += 1;
    }

    let rrule = &data["rrule"];
    if rrule.is_object() {
        line(buf, &mut y, tr("Recurrence").into_owned(), t.bold.patch(t.panel));
        let headline = rrule["headline"].as_str().unwrap_or("");
        let freq = rrule["freq"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {freq}"), t.text.patch(t.panel));
        y += 1;
    }

    let autofill = &data["autofill"];
    if autofill.is_object() {
        line(buf, &mut y, tr("Autofill").into_owned(), t.bold.patch(t.panel));
        let headline = autofill["headline"].as_str().unwrap_or("");
        let best = autofill["best"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {}: {best}", tr("Best")), t.text.patch(t.panel));
        y += 1;
    }

    let verify = &data["verify"];
    if verify.is_object() {
        line(buf, &mut y, tr("Verification").into_owned(), t.bold.patch(t.panel));
        let headline = verify["headline"].as_str().unwrap_or("");
        let evidence = verify["evidence"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {}: {evidence}", tr("Evidence")), t.text.patch(t.panel));
        y += 1;
    }

    let contextpin = &data["contextpin"];
    if contextpin.is_object() {
        line(buf, &mut y, tr("Window pin").into_owned(), t.bold.patch(t.panel));
        let headline = contextpin["headline"].as_str().unwrap_or("");
        let marker = contextpin["marker"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {}: {marker}", tr("Marker")), t.text.patch(t.panel));
        y += 1;
    }

    let mdtables = &data["mdtables"];
    if mdtables.is_object() {
        line(buf, &mut y, tr("Markdown tables").into_owned(), t.bold.patch(t.panel));
        let headline = mdtables["headline"].as_str().unwrap_or("");
        let layout = mdtables["layout"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {}: {layout}", tr("Layout")), t.text.patch(t.panel));
        y += 1;
    }

    let contextselect = &data["contextselect"];
    if contextselect.is_object() {
        line(buf, &mut y, tr("Context budget").into_owned(), t.bold.patch(t.panel));
        let headline = contextselect["headline"].as_str().unwrap_or("");
        let kept = contextselect["kept"].as_u64().unwrap_or(0);
        line(buf, &mut y, format!("{headline} · {}: {kept}", tr("Kept")), t.text.patch(t.panel));
        y += 1;
    }

    let picker = &data["picker"];
    if picker.is_object() {
        line(buf, &mut y, tr("Usage picker").into_owned(), t.bold.patch(t.panel));
        let headline = picker["headline"].as_str().unwrap_or("");
        let used = picker["used"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {}: {used}", tr("Used")), t.text.patch(t.panel));
        y += 1;
    }

    let palette = &data["palette"];
    if palette.is_object() {
        line(buf, &mut y, tr("Command palette").into_owned(), t.bold.patch(t.panel));
        let headline = palette["headline"].as_str().unwrap_or("");
        let results = palette["results"].as_u64().unwrap_or(0);
        line(buf, &mut y, format!("{headline} · {}: {results}", tr("Results")), t.text.patch(t.panel));
        y += 1;
    }

    let shortcuts = &data["shortcuts"];
    if shortcuts.is_object() {
        line(buf, &mut y, tr("Shortcuts").into_owned(), t.bold.patch(t.panel));
        let headline = shortcuts["headline"].as_str().unwrap_or("");
        let quote = shortcuts["quote"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {}: {quote}", tr("Quote")), t.text.patch(t.panel));
        y += 1;
    }

    let evalextra = &data["evalextra"];
    if evalextra.is_object() {
        line(buf, &mut y, tr("Eval metrics").into_owned(), t.bold.patch(t.panel));
        let headline = evalextra["headline"].as_str().unwrap_or("");
        let median = evalextra["median"].as_u64().unwrap_or(0);
        line(buf, &mut y, format!("{headline} Ã‚Â· {}: {median}", tr("Median")), t.text.patch(t.panel));
        y += 1;
    }

    let connextra = &data["connextra"];
    if connextra.is_object() {
        line(buf, &mut y, tr("Connections extra").into_owned(), t.bold.patch(t.panel));
        let headline = connextra["headline"].as_str().unwrap_or("");
        let channels = connextra["channels"].as_array().map_or(0, Vec::len);
        line(buf, &mut y, format!("{headline} Ã‚Â· {}: {channels}", tr("Channels")), t.text.patch(t.panel));
        y += 1;
    }

    let searchextra = &data["searchextra"];
    if searchextra.is_object() {
        line(buf, &mut y, tr("Ranked search").into_owned(), t.bold.patch(t.panel));
        let headline = searchextra["headline"].as_str().unwrap_or("");
        let matched = searchextra["matched"].as_u64().unwrap_or(0);
        line(buf, &mut y, format!("{headline} Ã‚Â· {}: {matched}", tr("Matched")), t.text.patch(t.panel));
        y += 1;
    }

    let memoryctx = &data["memoryctx"];
    if memoryctx.is_object() {
        line(buf, &mut y, tr("Memory context").into_owned(), t.bold.patch(t.panel));
        let headline = memoryctx["headline"].as_str().unwrap_or("");
        let hidden = memoryctx["hidden"].as_u64().unwrap_or(0);
        line(buf, &mut y, format!("{headline} Ã‚Â· {}: {hidden}", tr("Hidden")), t.text.patch(t.panel));
        y += 1;
    }

    let apierror = &data["apierror"];
    if apierror.is_object() {
        line(buf, &mut y, tr("API errors").into_owned(), t.bold.patch(t.panel));
        let headline = apierror["headline"].as_str().unwrap_or("");
        let status = apierror["status"].as_str().unwrap_or("");
        let samples = apierror["samples"].as_u64().unwrap_or(0);
        line(buf, &mut y, format!("{headline} Ã‚Â· {status} Ã‚Â· {}: {samples}", tr("Samples")), t.text.patch(t.panel));
        y += 1;
    }

    let history = &data["history"];
    if history.is_object() {
        line(buf, &mut y, tr("History").into_owned(), t.bold.patch(t.panel));
        let headline = history["headline"].as_str().unwrap_or("");
        let withheld = history["withheld"].as_u64().unwrap_or(0);
        line(buf, &mut y, format!("{headline} Ã‚Â· {}: {withheld}", tr("Withheld")), t.text.patch(t.panel));
        y += 1;
    }

    let review = &data["review"];
    if review.is_object() {
        line(buf, &mut y, tr("Review queue").into_owned(), t.bold.patch(t.panel));
        let headline = review["headline"].as_str().unwrap_or("");
        let mode = review["mode"].as_str().unwrap_or("");
        let queued = review["queued"].as_u64().unwrap_or(0);
        line(
            buf,
            &mut y,
            format!("{headline} Ã‚Â· {}: {mode} Ã‚Â· {}: {queued}", tr("Mode"), tr("Queued")),
            t.text.patch(t.panel),
        );
        y += 1;
    }

    let blueprint = &data["blueprint"];
    if blueprint.is_object() {
        line(buf, &mut y, tr("Blueprint").into_owned(), t.bold.patch(t.panel));
        let headline = blueprint["headline"].as_str().unwrap_or("");
        let cron = blueprint["cron"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} Ã‚Â· {}: {cron}", tr("Cron")), t.text.patch(t.panel));
        y += 1;
    }

    let stream = &data["stream"];
    if stream.is_object() {
        line(buf, &mut y, tr("Stream").into_owned(), t.bold.patch(t.panel));
        let headline = stream["headline"].as_str().unwrap_or("");
        let held = stream["held"].as_u64().unwrap_or(0);
        let chunks = stream["chunks"].as_u64().unwrap_or(0);
        line(
            buf,
            &mut y,
            format!("{headline} Ã‚Â· {}: {held} Ã‚Â· {}: {chunks}", tr("Held"), tr("Chunks")),
            t.text.patch(t.panel),
        );
        y += 1;
    }

    let i18n = &data["i18n"];
    if i18n.is_object() {
        line(buf, &mut y, tr("Locale").into_owned(), t.bold.patch(t.panel));
        let headline = i18n["headline"].as_str().unwrap_or("");
        let plural = i18n["plural"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} Ã‚Â· {}: {plural}", tr("Plural")), t.text.patch(t.panel));
        y += 1;
    }

    let quota = &data["quota"];
    if quota.is_object() {
        line(buf, &mut y, tr("Quota").into_owned(), t.bold.patch(t.panel));
        let headline = quota["headline"].as_str().unwrap_or("");
        let status = quota["status"].as_str().unwrap_or("");
        let remaining = quota["remaining"].as_i64().unwrap_or(0);
        line(
            buf,
            &mut y,
            format!("{headline} Ã‚Â· {status} Ã‚Â· {}: {remaining}%", tr("Remaining")),
            t.text.patch(t.panel),
        );
        y += 1;
    }

    let appearance = &data["appearance"];
    if appearance.is_object() {
        line(buf, &mut y, tr("Appearance").into_owned(), t.bold.patch(t.panel));
        let headline = appearance["headline"].as_str().unwrap_or("");
        let accent = appearance["accent"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} Ã‚Â· {}: {accent}", tr("Accent")), t.text.patch(t.panel));
        y += 1;
    }

    let connstate = &data["connstate"];
    if connstate.is_object() {
        line(buf, &mut y, tr("Connections").into_owned(), t.bold.patch(t.panel));
        let headline = connstate["headline"].as_str().unwrap_or("");
        let state = connstate["state"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} Ã‚Â· {}: {state}", tr("State")), t.text.patch(t.panel));
        y += 1;
    }

    let eval_result = &data["eval"];
    if eval_result.is_object() {
        line(buf, &mut y, tr("Evaluation").into_owned(), t.bold.patch(t.panel));
        let headline = eval_result["headline"].as_str().unwrap_or("");
        let verdict = eval_result["verdict"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} Ã‚Â· {}: {verdict}", tr("Verdict")), t.text.patch(t.panel));
        y += 1;
    }

    let search = &data["search"];
    if search.is_object() {
        line(buf, &mut y, tr("Search").into_owned(), t.bold.patch(t.panel));
        let headline = search["headline"].as_str().unwrap_or("");
        let hits = search["hits"].as_u64().unwrap_or(0);
        line(buf, &mut y, format!("{headline} Ã‚Â· {}: {hits}", tr("Hits")), t.text.patch(t.panel));
        y += 1;
    }

    let mailtext = &data["mailtext"];
    if mailtext.is_object() {
        line(buf, &mut y, tr("Mail & data").into_owned(), t.bold.patch(t.panel));
        let headline = mailtext["headline"].as_str().unwrap_or("");
        let address = mailtext["address"].as_str().unwrap_or("");
        let rows = mailtext["rows"].as_u64().unwrap_or(0);
        line(buf, &mut y, format!("{headline} Ã‚Â· {address} Ã‚Â· {}: {rows}", tr("Rows")), t.text.patch(t.panel));
        y += 1;
    }

    let notify = &data["notify"];
    if notify.is_object() {
        line(buf, &mut y, tr("Notifications").into_owned(), t.bold.patch(t.panel));
        let headline = notify["headline"].as_str().unwrap_or("");
        let state = notify["state"].as_str().unwrap_or("");
        let failed = notify["failed"].as_u64().unwrap_or(0);
        line(
            buf,
            &mut y,
            format!("{headline} Ã‚Â· {}: {state} Ã‚Â· {}: {failed}", tr("State"), tr("Failed")),
            t.text.patch(t.panel),
        );
        y += 1;
    }

    let markdown = &data["markdown"];
    if markdown.is_object() {
        line(buf, &mut y, tr("Markdown").into_owned(), t.bold.patch(t.panel));
        let headline = markdown["headline"].as_str().unwrap_or("");
        let preview = markdown["preview"].as_str().unwrap_or("");
        let mentions = markdown["mentions"].as_u64().unwrap_or(0);
        line(buf, &mut y, format!("{headline} Ã‚Â· {}: {mentions}", tr("Mentions")), t.text.patch(t.panel));
        if !preview.is_empty() {
            line(buf, &mut y, preview.to_owned(), t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let localfmt = &data["localfmt"];
    if localfmt.is_object() {
        line(buf, &mut y, tr("Local format").into_owned(), t.bold.patch(t.panel));
        let headline = localfmt["headline"].as_str().unwrap_or("");
        let weekday = localfmt["weekday"].as_str().unwrap_or("");
        let tokens = localfmt["tokens"].as_str().unwrap_or("");
        line(
            buf,
            &mut y,
            format!("{headline} Ã‚Â· {weekday} Ã‚Â· {}: {}", tr("Tokens"), tokens),
            t.text.patch(t.panel),
        );
        y += 1;
    }

    let textfmt = &data["textfmt"];
    if textfmt.is_object() {
        line(buf, &mut y, tr("Text hygiene").into_owned(), t.bold.patch(t.panel));
        let headline = textfmt["headline"].as_str().unwrap_or("");
        let handle = textfmt["handle"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} Ã‚Â· {}: {}", tr("Handle"), handle), t.text.patch(t.panel));
        y += 1;
    }

    let agenteschat = &data["agenteschat"];
    if agenteschat.is_object() {
        line(buf, &mut y, tr("Context budget").into_owned(), t.bold.patch(t.panel));
        let headline = agenteschat["headline"].as_str().unwrap_or("");
        let window = agenteschat["window"].as_str().unwrap_or("");
        let mode = agenteschat["auto"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} Ã‚Â· {}: {}", tr("Window"), window), t.text.patch(t.panel));
        line(buf, &mut y, format!("  {}: {}", tr("Mode"), mode), t.secondary.patch(t.panel));
        y += 1;
    }

    let openbot = &data["openbot"];
    if openbot.is_object() {
        line(buf, &mut y, tr("Run model").into_owned(), t.bold.patch(t.panel));
        let headline = openbot["headline"].as_str().unwrap_or("");
        let effort = openbot["effort"].as_str().unwrap_or("");
        let route = openbot["route"].as_str().unwrap_or("");
        line(
            buf,
            &mut y,
            format!("{headline} Ã‚Â· {}: {} Ã‚Â· {}: {}", tr("Effort"), effort, tr("Route"), route),
            t.text.patch(t.panel),
        );
        y += 1;
    }

    let opendots = &data["opendots"];
    if opendots.is_object() {
        line(buf, &mut y, tr("Learning").into_owned(), t.bold.patch(t.panel));
        let headline = opendots["headline"].as_str().unwrap_or("");
        let container = opendots["container"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} Ã‚Â· {}: {}", tr("Container"), container), t.text.patch(t.panel));
        y += 1;
    }

    let context = &data["context"];
    if context.is_object() {
        line(buf, &mut y, tr("Context window").into_owned(), t.bold.patch(t.panel));
        let headline = context["headline"].as_str().unwrap_or("");
        line(buf, &mut y, headline.to_owned(), t.text.patch(t.panel));
        let page = context["page"].as_str().unwrap_or("");
        let shots = context["screenshots"].as_str().unwrap_or("");
        line(
            buf,
            &mut y,
            format!("  {}: {page} Ã‚Â· {}: {shots}", tr("Page states"), tr("Screenshots")),
            t.secondary.patch(t.panel),
        );
        y += 1;
    }

    let liveness = &data["liveness"];
    if liveness.is_object() {
        line(buf, &mut y, tr("Liveness").into_owned(), t.bold.patch(t.panel));
        let headline = liveness["headline"].as_str().unwrap_or("");
        let outcome = liveness["outcome"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} Ã‚Â· {}: {}", tr("Delivery"), outcome), t.text.patch(t.panel));
        y += 1;
    }

    let dictation = &data["dictation"];
    if dictation.is_object() {
        line(buf, &mut y, tr("Voice dictation").into_owned(), t.bold.patch(t.panel));
        let headline = dictation["headline"].as_str().unwrap_or("");
        let engine = dictation["engine"].as_str().unwrap_or("");
        let model = dictation["model"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} Ã‚Â· {engine} Ã‚Â· {model}"), t.text.patch(t.panel));
        y += 1;
    }

    let access = &data["access"];
    if access.is_object() {
        line(buf, &mut y, tr("Public access").into_owned(), t.bold.patch(t.panel));
        let mode = access["mode"].as_str().unwrap_or("");
        let routing = access["routing"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{mode} Ã‚Â· {routing}"), t.text.patch(t.panel));
        let items: Vec<String> = access["collections"]
            .as_array()
            .map(|a| a.iter().take(8).map(|c| format!("  {}", c["name"].as_str().unwrap_or(""))).collect())
            .unwrap_or_default();
        for item in items {
            line(buf, &mut y, item, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let blocks = &data["blocks"];
    if blocks.is_object() {
        line(buf, &mut y, tr("Last message").into_owned(), t.bold.patch(t.panel));
        let headline = blocks["headline"].as_str().unwrap_or("");
        let origin = blocks["origin"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} Ã‚Â· {origin}"), t.text.patch(t.panel));
        let items: Vec<String> = blocks["blocks"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(8)
                    .map(|b| {
                        format!(
                            "  {} {}",
                            if b["says"] == true { "Ã¢â€”Â" } else { "Ã¢â€”â€¹" },
                            b["text"].as_str().unwrap_or("")
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for item in items {
            line(buf, &mut y, item, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let directory = &data["directory"];
    if directory.is_object() {
        line(buf, &mut y, tr("Directory").into_owned(), t.bold.patch(t.panel));
        let headline = directory["headline"].as_str().unwrap_or("");
        let bar = directory["bar"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{bar} {headline}"), t.text.patch(t.panel));
        let items: Vec<String> = directory["integrations"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(8)
                    .map(|i| {
                        format!("  {} Ã‚Â· {}", i["name"].as_str().unwrap_or(""), i["header"].as_str().unwrap_or(""))
                    })
                    .collect()
            })
            .unwrap_or_default();
        for item in items {
            line(buf, &mut y, item, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let prompts = &data["prompts"];
    if prompts.is_object() {
        line(buf, &mut y, tr("Prompt style").into_owned(), t.bold.patch(t.panel));
        let model = prompts["model"].as_str().unwrap_or("");
        let price = prompts["price"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{model} Ã‚Â· {price}"), t.text.patch(t.panel));
        let lines: Vec<String> = prompts["lines"]
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect())
            .unwrap_or_default();
        for prompt_line in lines {
            line(buf, &mut y, format!("  {prompt_line}"), t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let fresh = &data["fresh"];
    if fresh.is_object() {
        line(buf, &mut y, tr("Latest from bdots").into_owned(), t.bold.patch(t.panel));
        let agent = fresh["agent"].as_str().unwrap_or("");
        line(buf, &mut y, agent.to_owned(), t.text.patch(t.panel));
        let rows: Vec<String> = fresh["summary"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|r| format!("  {}: {}", r["key"].as_str().unwrap_or(""), r["value"].as_str().unwrap_or("")))
                    .collect()
            })
            .unwrap_or_default();
        for row in rows {
            line(buf, &mut y, row, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let agent = &data["agent"];
    if agent.is_object() {
        line(buf, &mut y, tr("Agent profile").into_owned(), t.bold.patch(t.panel));
        let label = agent["label"].as_str().unwrap_or("");
        let headline = agent["headline"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{label} Ã‚Â· {headline}"), t.text.patch(t.panel));
        let lines: Vec<String> = agent["lines"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|l| {
                        format!(
                            "  {} {}: {}",
                            if l["set"] == true { "Ã¢â€”Â" } else { "Ã¢â€”â€¹" },
                            l["label"].as_str().unwrap_or(""),
                            l["detail"].as_str().unwrap_or("")
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for item in lines {
            line(buf, &mut y, item, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let conversation = &data["conversation"];
    if conversation.is_object() {
        line(buf, &mut y, tr("Conversation").into_owned(), t.bold.patch(t.panel));
        let headline = conversation["headline"].as_str().unwrap_or("");
        let rhythm = conversation["line"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} Ã‚Â· {rhythm}"), t.text.patch(t.panel));
        let kinds: Vec<String> = conversation["kinds"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|k| {
                        format!(
                            "  {} {} {}",
                            k["label"].as_str().unwrap_or(""),
                            k["bar"].as_str().unwrap_or(""),
                            k["count"].as_u64().unwrap_or(0)
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for kind in kinds {
            line(buf, &mut y, kind, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let failures = &data["failures"];
    if failures.is_object() && failures["total"].as_str() != Some("0") {
        line(buf, &mut y, tr("Recent failures").into_owned(), t.bold.patch(t.panel));
        let headline = failures["headline"].as_str().unwrap_or("");
        line(buf, &mut y, headline.to_owned(), t.text.patch(t.panel));
        let items: Vec<String> = failures["items"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(8)
                    .map(|i| {
                        format!("  {} Ã‚Â· {}", i["name"].as_str().unwrap_or(""), i["reason"].as_str().unwrap_or(""))
                    })
                    .collect()
            })
            .unwrap_or_default();
        for item in items {
            line(buf, &mut y, item, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let approvals = &data["approvals"];
    if approvals.is_object() {
        line(buf, &mut y, tr("Approvals").into_owned(), t.bold.patch(t.panel));
        let headline = approvals["headline"].as_str().unwrap_or("");
        let bar = approvals["bar"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{bar} {headline}"), t.text.patch(t.panel));
        let items: Vec<String> = approvals["items"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(8)
                    .map(|i| {
                        format!("  {} Ã‚Â· {}", i["title"].as_str().unwrap_or(""), i["age"].as_str().unwrap_or(""))
                    })
                    .collect()
            })
            .unwrap_or_default();
        for item in items {
            line(buf, &mut y, item, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let zones = &data["timezone"];
    if zones.is_object() {
        line(buf, &mut y, tr("Time zones").into_owned(), t.bold.patch(t.panel));
        let headline = zones["headline"].as_str().unwrap_or("");
        let local = zones["local"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} Ã‚Â· {}: {}", tr("This computer"), local), t.text.patch(t.panel));
        let all: Vec<String> = zones["all"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|z| {
                        format!(
                            "  {} {} {}",
                            z["label"].as_str().unwrap_or(""),
                            z["bar"].as_str().unwrap_or(""),
                            z["count"].as_u64().unwrap_or(0)
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for zone in all {
            line(buf, &mut y, zone, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let digest = &data["digest"];
    if digest.is_object() {
        line(buf, &mut y, tr("At a glance").into_owned(), t.bold.patch(t.panel));
        let label = digest["label"].as_str().unwrap_or("");
        let headline = digest["headline"].as_str().unwrap_or("");
        let one_line = digest["oneLine"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{label} Ã‚Â· {headline}"), t.text.patch(t.panel));
        line(buf, &mut y, one_line.to_owned(), t.tertiary.patch(t.panel));
        let stats: Vec<String> = digest["stats"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|s| format!("  {}: {}", s["label"].as_str().unwrap_or(""), s["value"].as_str().unwrap_or("")))
                    .collect()
            })
            .unwrap_or_default();
        for stat_line in stats {
            line(buf, &mut y, stat_line, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let advice = &data["advice"];
    if advice.is_object() {
        line(buf, &mut y, tr("Suggestions").into_owned(), t.bold.patch(t.panel));
        let label = advice["label"].as_str().unwrap_or("");
        let headline = advice["headline"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{label} Ã‚Â· {headline}"), t.text.patch(t.panel));
        let items: Vec<String> = advice["items"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(8)
                    .map(|i| {
                        format!(
                            "  {}: {}",
                            i["priorityLabel"].as_str().unwrap_or(""),
                            i["title"].as_str().unwrap_or("")
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for item in items {
            line(buf, &mut y, item, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let session = &data["session"];
    if session.is_object() {
        line(buf, &mut y, tr("Session").into_owned(), t.bold.patch(t.panel));
        let label = session["label"].as_str().unwrap_or("");
        let headline = session["headline"].as_str().unwrap_or("");
        let bar = session["bar"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{bar} {label} Ã‚Â· {headline}"), t.text.patch(t.panel));
        let lines: Vec<String> = session["lines"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|l| {
                        format!(
                            "  {} {}: {}",
                            if l["ok"] == true { "Ã¢â€”Â" } else { "Ã¢â€”â€¹" },
                            l["label"].as_str().unwrap_or(""),
                            l["detail"].as_str().unwrap_or("")
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for item in lines {
            line(buf, &mut y, item, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let workspace = &data["workspace"];
    if workspace.is_object() {
        line(buf, &mut y, tr("Workspace").into_owned(), t.bold.patch(t.panel));
        let label = workspace["label"].as_str().unwrap_or("");
        let headline = workspace["headline"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{label} Ã‚Â· {headline}"), t.text.patch(t.panel));
        let lines: Vec<String> = workspace["lines"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|l| {
                        format!(
                            "  {} {}: {}",
                            if l["set"] == true { "Ã¢â€”Â" } else { "Ã¢â€”â€¹" },
                            l["label"].as_str().unwrap_or(""),
                            l["detail"].as_str().unwrap_or("")
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for item in lines {
            line(buf, &mut y, item, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let memory = &data["memory"];
    if memory.is_object() {
        line(buf, &mut y, tr("Memory").into_owned(), t.bold.patch(t.panel));
        let label = memory["label"].as_str().unwrap_or("");
        let headline = memory["headline"].as_str().unwrap_or("");
        let shared = memory["shared"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{label} Ã‚Â· {headline}"), t.text.patch(t.panel));
        line(buf, &mut y, format!("  {}: {}", tr("Shared notes"), shared), t.secondary.patch(t.panel));
        let items: Vec<String> = memory["lines"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|l| {
                        format!(
                            "  {} {}: {}",
                            if l["on"] == true { "Ã¢â€”Â" } else { "Ã¢â€”â€¹" },
                            l["title"].as_str().unwrap_or(""),
                            l["detail"].as_str().unwrap_or("")
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for item in items {
            line(buf, &mut y, item, t.tertiary.patch(t.panel));
        }
        y += 1;
    }

    let tuning = &data["tuning"];
    if tuning.is_object() {
        line(buf, &mut y, tr("Model tuning").into_owned(), t.bold.patch(t.panel));
        let effort = tuning["effort"].as_str().unwrap_or("");
        let headline = tuning["headline"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{effort} Ã‚Â· {headline}"), t.text.patch(t.panel));
        let options: Vec<String> = tuning["options"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(8)
                    .map(|o| format!("  {}: {}", o["title"].as_str().unwrap_or(""), o["value"].as_str().unwrap_or("")))
                    .collect()
            })
            .unwrap_or_default();
        for option in options {
            line(buf, &mut y, option, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let accounts = &data["connectors"];
    if accounts.is_object() {
        line(buf, &mut y, tr("Connected accounts").into_owned(), t.bold.patch(t.panel));
        let headline = accounts["headline"].as_str().unwrap_or("");
        line(buf, &mut y, headline.to_owned(), t.text.patch(t.panel));
        let cats: Vec<String> = accounts["categories"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|c| {
                        format!(
                            "  {} {} {}",
                            c["kind"].as_str().unwrap_or(""),
                            c["bar"].as_str().unwrap_or(""),
                            c["count"].as_u64().unwrap_or(0)
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for cat in cats {
            line(buf, &mut y, cat, t.secondary.patch(t.panel));
        }
        let names: Vec<String> = accounts["connectors"]
            .as_array()
            .map(|a| a.iter().map(|n| format!("  Ã¢â‚¬Â¢ {}", n["name"].as_str().unwrap_or(""))).collect())
            .unwrap_or_default();
        for name in names {
            line(buf, &mut y, name, t.tertiary.patch(t.panel));
        }
        y += 1;
    }

    let identity = &data["identity"];
    if identity.is_object() {
        line(buf, &mut y, tr("Identity").into_owned(), t.bold.patch(t.panel));
        let headline = identity["headline"].as_str().unwrap_or("");
        line(buf, &mut y, headline.to_owned(), t.text.patch(t.panel));
        let band = identity["band"].as_str().unwrap_or("");
        let section = identity["section"].as_str().unwrap_or("");
        let rank = identity["rank"].as_u64().unwrap_or(0);
        line(buf, &mut y, format!("  {band} Ã‚Â· {section} Ã‚Â· #{}", rank), t.secondary.patch(t.panel));
        let traits: Vec<String> = identity["traits"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|tr| {
                        format!(
                            "  {} {}: {}",
                            if tr["on"] == true { "Ã¢â€”Â" } else { "Ã¢â€”â€¹" },
                            tr["label"].as_str().unwrap_or(""),
                            tr["detail"].as_str().unwrap_or("")
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for trait_line in traits {
            line(buf, &mut y, trait_line, t.tertiary.patch(t.panel));
        }
        y += 1;
    }

    let resilience = &data["resilience"];
    if resilience.is_object() {
        line(buf, &mut y, tr("Resilience").into_owned(), t.bold.patch(t.panel));
        let label = resilience["label"].as_str().unwrap_or("");
        let headline = resilience["headline"].as_str().unwrap_or("");
        let bar = resilience["bar"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{bar} {label} Ã‚Â· {headline}"), t.text.patch(t.panel));
        let switches: Vec<String> = resilience["switches"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|s| {
                        format!(
                            "{} {}",
                            if s["on"] == true { "Ã¢â€”Â" } else { "Ã¢â€”â€¹" },
                            s["label"].as_str().unwrap_or("")
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        if !switches.is_empty() {
            line(buf, &mut y, format!("  {}", switches.join("  ")), t.secondary.patch(t.panel));
        }
        let fallbacks: Vec<String> = resilience["fallbacks"]
            .as_array()
            .map(|a| a.iter().map(|f| format!("  {}", f["label"].as_str().unwrap_or(""))).collect())
            .unwrap_or_default();
        for fb in fallbacks {
            line(buf, &mut y, fb, t.tertiary.patch(t.panel));
        }
        y += 1;
    }

    let trust = &data["trust"];
    if trust.is_object() {
        line(buf, &mut y, tr("Trust").into_owned(), t.bold.patch(t.panel));
        let label = trust["label"].as_str().unwrap_or("");
        let headline = trust["headline"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{label} Ã‚Â· {headline}"), t.text.patch(t.panel));
        let mode = trust["modeLabel"].as_str().unwrap_or("");
        line(buf, &mut y, format!("  {}: {}", tr("Approval"), mode), t.secondary.patch(t.panel));
        let rules: Vec<String> = trust["rules"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(8)
                    .map(|r| {
                        let kind = r["kind"].as_str().unwrap_or("");
                        if kind.is_empty() {
                            format!(
                                "  {} {}",
                                r["decision"].as_str().unwrap_or(""),
                                r["pattern"].as_str().unwrap_or("")
                            )
                        } else {
                            format!(
                                "  {} {} ({kind})",
                                r["decision"].as_str().unwrap_or(""),
                                r["pattern"].as_str().unwrap_or("")
                            )
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        for rule in rules {
            line(buf, &mut y, rule, t.tertiary.patch(t.panel));
        }
        y += 1;
    }

    let library = &data["library"];
    if library.is_object() {
        line(buf, &mut y, tr("Skill library").into_owned(), t.bold.patch(t.panel));
        let headline = library["headline"].as_str().unwrap_or("");
        line(buf, &mut y, headline.to_owned(), t.text.patch(t.panel));
        let kinds: Vec<String> = library["kinds"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|k| {
                        format!(
                            "  {} {} {}",
                            k["kind"].as_str().unwrap_or(""),
                            k["bar"].as_str().unwrap_or(""),
                            k["count"].as_u64().unwrap_or(0)
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for kind in kinds {
            line(buf, &mut y, kind, t.secondary.patch(t.panel));
        }
        let skills: Vec<String> = library["skills"]
            .as_array()
            .map(|a| a.iter().take(8).map(|s| format!("  Ã¢â‚¬Â¢ {}", s["name"].as_str().unwrap_or(""))).collect())
            .unwrap_or_default();
        for skill in skills {
            line(buf, &mut y, skill, t.tertiary.patch(t.panel));
        }
        y += 1;
    }

    let schedule = &data["routineSummary"];
    if schedule.is_object() {
        line(buf, &mut y, tr("Schedule").into_owned(), t.bold.patch(t.panel));
        let headline = schedule["headline"].as_str().unwrap_or("");
        let bar = schedule["bar"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{bar} {headline}"), t.text.patch(t.panel));
        let active = schedule["active"].as_u64().unwrap_or(0);
        let paused = schedule["paused"].as_u64().unwrap_or(0);
        let mix = schedule["mix"].as_str().unwrap_or("");
        line(
            buf,
            &mut y,
            format!("  {}: {active} Ã‚Â· {}: {paused} Ã‚Â· {mix}", tr("Active"), tr("Paused")),
            t.secondary.patch(t.panel),
        );
        let soonest: Vec<String> = schedule["soonest"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(6)
                    .map(|r| {
                        format!(
                            "  {} {} Ã‚Â· {}{}",
                            if r["enabled"] == true { "Ã¢â€”Â" } else { "Ã¢â€”â€¹" },
                            r["name"].as_str().unwrap_or(""),
                            r["trigger"].as_str().unwrap_or(""),
                            {
                                let n = r["nextIn"].as_str().unwrap_or("");
                                if n.is_empty() { String::new() } else { format!(" Ã‚Â· {n}") }
                            }
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for item in soonest {
            line(buf, &mut y, item, t.tertiary.patch(t.panel));
        }
        y += 1;
    }

    let week = &data["week"];
    if week.is_object() {
        line(buf, &mut y, tr("This week").into_owned(), t.bold.patch(t.panel));
        let verdict = week["verdict"].as_str().unwrap_or("");
        let headline = week["headline"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{verdict} Ã‚Â· {headline}"), t.text.patch(t.panel));
        let steps = week["totalLabel"].as_str().unwrap_or("0");
        let trend = week["trend"]["label"].as_str().unwrap_or("");
        line(
            buf,
            &mut y,
            format!("  {}: {steps} Ã‚Â· {}: {trend}", tr("Steps"), tr("Trend")),
            t.secondary.patch(t.panel),
        );
        let top: Vec<String> = week["top"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(5)
                    .map(|d| {
                        format!(
                            "  {} {} {}",
                            d["label"].as_str().unwrap_or(""),
                            d["bar"].as_str().unwrap_or(""),
                            d["count"].as_u64().unwrap_or(0)
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for day in top {
            line(buf, &mut y, day, t.tertiary.patch(t.panel));
        }
        y += 1;
    }

    let profile = &data["profile"];
    if profile.is_object() {
        line(buf, &mut y, tr("Capabilities").into_owned(), t.bold.patch(t.panel));
        let headline = profile["headline"].as_str().unwrap_or("");
        let bar = profile["bar"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{bar} {headline}"), t.accent.patch(t.panel));
        let caps: Vec<String> = profile["caps"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(15)
                    .map(|c| {
                        format!(
                            "  {} {}: {}",
                            if c["enabled"] == true { "Ã¢â€”Â" } else { "Ã¢â€”â€¹" },
                            c["title"].as_str().unwrap_or(""),
                            c["detail"].as_str().unwrap_or("")
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for cap in caps {
            line(buf, &mut y, cap, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let health = &data["health"];
    if health.is_object() {
        line(buf, &mut y, tr("Health").into_owned(), t.bold.patch(t.panel));
        let label = health["label"].as_str().unwrap_or("");
        let headline = health["headline"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{label} Ã‚Â· {headline}"), t.text.patch(t.panel));
        let checks: Vec<String> = health["checks"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(10)
                    .map(|c| {
                        format!(
                            "  {} {}: {}",
                            if c["severity"] == "error" { "!" } else { "Ã‚Â·" },
                            c["title"].as_str().unwrap_or(""),
                            c["detail"].as_str().unwrap_or("")
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for check in checks {
            line(buf, &mut y, check, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let activity = &data["activity"];
    if activity.is_object() {
        line(buf, &mut y, tr("Activity").into_owned(), t.bold.patch(t.panel));
        let headline = activity["headline"].as_str().unwrap_or("");
        let steps = activity["steps"].as_u64().unwrap_or(0);
        let errors = activity["errors"].as_u64().unwrap_or(0);
        line(buf, &mut y, format!("{headline} Ã‚Â· {steps} {}", tr("steps")), t.secondary.patch(t.panel));
        if errors > 0 {
            line(buf, &mut y, format!("{errors} {}", tr("failed")), t.red.patch(t.panel));
        }
        let groups: Vec<String> = activity["groups"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(6)
                    .map(|g| format!("{} {}", g["count"].as_u64().unwrap_or(0), g["label"].as_str().unwrap_or("")))
                    .collect()
            })
            .unwrap_or_default();
        for group in groups {
            line(buf, &mut y, format!("  Ã¢â‚¬Â¢ {group}"), t.secondary.patch(t.panel));
        }
        let days: Vec<String> = activity["days"]
            .as_array()
            .map(|a| {
                a.iter()
                    .rev()
                    .take(5)
                    .map(|d| {
                        format!(
                            "  {} {} {}",
                            d["label"].as_str().unwrap_or(""),
                            "Ã¢â€“Â®".repeat(d["width"].as_u64().unwrap_or(0).max(1) as usize),
                            d["count"].as_u64().unwrap_or(0)
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for day in days {
            line(buf, &mut y, day, t.dim.patch(t.panel));
        }
        let items: Vec<String> = activity["items"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(6)
                    .map(|i| {
                        let text = i["text"].as_str().unwrap_or("");
                        let detail = i["detail"].as_str().unwrap_or("");
                        if detail.is_empty() {
                            format!("  Ã¢â‚¬Â¢ {text}")
                        } else {
                            format!("  Ã¢â‚¬Â¢ {text} Ã¢â‚¬â€ {detail}")
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        for item in items {
            line(buf, &mut y, item, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let recent: Vec<&str> =
        data["recentWork"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    if !recent.is_empty() {
        line(buf, &mut y, tr("Recent work").into_owned(), t.bold.patch(t.panel));
        for item in recent {
            line(buf, &mut y, format!("Ã¢â‚¬Â¢ {item}"), t.secondary.patch(t.panel));
        }
        y += 1;
    }

    line(
        buf,
        &mut y,
        format!("{} ({})", tr("Routines"), data["routines"].as_array().map_or(0, Vec::len)),
        t.bold.patch(t.panel),
    );
    let routines: &[Value] = data["routines"].as_array().map_or(&[], Vec::as_slice);
    if routines.is_empty() {
        line(buf, &mut y, tr("No routines yet.").into_owned(), t.dim.patch(t.panel));
    }
    for routine in routines {
        let name = routine["name"].as_str().unwrap_or("?");
        let flag = if routine["enabled"] == true { "" } else { " (paused)" };
        line(buf, &mut y, format!("Ã¢â‚¬Â¢ {name}{flag}"), t.text.patch(t.panel));
        let schedules = routine["schedules"]
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" Ã‚Â· "))
            .unwrap_or_default();
        if !schedules.is_empty() {
            line(buf, &mut y, format!("  {schedules}"), t.secondary.patch(t.panel));
        }
        if let Some(next) = routine["nextIn"].as_str() {
            line(buf, &mut y, format!("  {}: {next}", tr("Next in")), t.dim.patch(t.panel));
        }
    }
    footer(buf, inner, None, "esc Ã‚Â· shift+Ã¢â€ ÂÃ¢â€ â€™ pages");
}
