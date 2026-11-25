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
        put(buf, inner.x, inner.y, inner.width, &tr("Loading…"), t.dim.patch(t.panel));
        footer(buf, inner, None, "esc · shift+←→ pages");
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

    let liveness = &data["liveness"];
    if liveness.is_object() {
        line(buf, &mut y, tr("Liveness").into_owned(), t.bold.patch(t.panel));
        let headline = liveness["headline"].as_str().unwrap_or("");
        let outcome = liveness["outcome"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {}: {}", tr("Delivery"), outcome), t.text.patch(t.panel));
        y += 1;
    }

    let dictation = &data["dictation"];
    if dictation.is_object() {
        line(buf, &mut y, tr("Voice dictation").into_owned(), t.bold.patch(t.panel));
        let headline = dictation["headline"].as_str().unwrap_or("");
        let engine = dictation["engine"].as_str().unwrap_or("");
        let model = dictation["model"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{headline} · {engine} · {model}"), t.text.patch(t.panel));
        y += 1;
    }

    let access = &data["access"];
    if access.is_object() {
        line(buf, &mut y, tr("Public access").into_owned(), t.bold.patch(t.panel));
        let mode = access["mode"].as_str().unwrap_or("");
        let routing = access["routing"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{mode} · {routing}"), t.text.patch(t.panel));
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
        line(buf, &mut y, format!("{headline} · {origin}"), t.text.patch(t.panel));
        let items: Vec<String> = blocks["blocks"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(8)
                    .map(|b| {
                        format!("  {} {}", if b["says"] == true { "●" } else { "○" }, b["text"].as_str().unwrap_or(""))
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
                    .map(|i| format!("  {} · {}", i["name"].as_str().unwrap_or(""), i["header"].as_str().unwrap_or("")))
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
        line(buf, &mut y, format!("{model} · {price}"), t.text.patch(t.panel));
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
        line(buf, &mut y, format!("{label} · {headline}"), t.text.patch(t.panel));
        let lines: Vec<String> = agent["lines"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|l| {
                        format!(
                            "  {} {}: {}",
                            if l["set"] == true { "●" } else { "○" },
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
        line(buf, &mut y, format!("{headline} · {rhythm}"), t.text.patch(t.panel));
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
                    .map(|i| format!("  {} · {}", i["name"].as_str().unwrap_or(""), i["reason"].as_str().unwrap_or("")))
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
                    .map(|i| format!("  {} · {}", i["title"].as_str().unwrap_or(""), i["age"].as_str().unwrap_or("")))
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
        line(buf, &mut y, format!("{headline} · {}: {}", tr("This computer"), local), t.text.patch(t.panel));
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
        line(buf, &mut y, format!("{label} · {headline}"), t.text.patch(t.panel));
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
        line(buf, &mut y, format!("{label} · {headline}"), t.text.patch(t.panel));
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
        line(buf, &mut y, format!("{bar} {label} · {headline}"), t.text.patch(t.panel));
        let lines: Vec<String> = session["lines"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|l| {
                        format!(
                            "  {} {}: {}",
                            if l["ok"] == true { "●" } else { "○" },
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
        line(buf, &mut y, format!("{label} · {headline}"), t.text.patch(t.panel));
        let lines: Vec<String> = workspace["lines"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|l| {
                        format!(
                            "  {} {}: {}",
                            if l["set"] == true { "●" } else { "○" },
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
        line(buf, &mut y, format!("{label} · {headline}"), t.text.patch(t.panel));
        line(buf, &mut y, format!("  {}: {}", tr("Shared notes"), shared), t.secondary.patch(t.panel));
        let items: Vec<String> = memory["lines"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|l| {
                        format!(
                            "  {} {}: {}",
                            if l["on"] == true { "●" } else { "○" },
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
        line(buf, &mut y, format!("{effort} · {headline}"), t.text.patch(t.panel));
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
            .map(|a| a.iter().map(|n| format!("  • {}", n["name"].as_str().unwrap_or(""))).collect())
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
        line(buf, &mut y, format!("  {band} · {section} · #{}", rank), t.secondary.patch(t.panel));
        let traits: Vec<String> = identity["traits"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|tr| {
                        format!(
                            "  {} {}: {}",
                            if tr["on"] == true { "●" } else { "○" },
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
        line(buf, &mut y, format!("{bar} {label} · {headline}"), t.text.patch(t.panel));
        let switches: Vec<String> = resilience["switches"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|s| {
                        format!("{} {}", if s["on"] == true { "●" } else { "○" }, s["label"].as_str().unwrap_or(""))
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
        line(buf, &mut y, format!("{label} · {headline}"), t.text.patch(t.panel));
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
            .map(|a| a.iter().take(8).map(|s| format!("  • {}", s["name"].as_str().unwrap_or(""))).collect())
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
            format!("  {}: {active} · {}: {paused} · {mix}", tr("Active"), tr("Paused")),
            t.secondary.patch(t.panel),
        );
        let soonest: Vec<String> = schedule["soonest"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(6)
                    .map(|r| {
                        format!(
                            "  {} {} · {}{}",
                            if r["enabled"] == true { "●" } else { "○" },
                            r["name"].as_str().unwrap_or(""),
                            r["trigger"].as_str().unwrap_or(""),
                            {
                                let n = r["nextIn"].as_str().unwrap_or("");
                                if n.is_empty() { String::new() } else { format!(" · {n}") }
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
        line(buf, &mut y, format!("{verdict} · {headline}"), t.text.patch(t.panel));
        let steps = week["totalLabel"].as_str().unwrap_or("0");
        let trend = week["trend"]["label"].as_str().unwrap_or("");
        line(buf, &mut y, format!("  {}: {steps} · {}: {trend}", tr("Steps"), tr("Trend")), t.secondary.patch(t.panel));
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
                            if c["enabled"] == true { "●" } else { "○" },
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
        line(buf, &mut y, format!("{label} · {headline}"), t.text.patch(t.panel));
        let checks: Vec<String> = health["checks"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(10)
                    .map(|c| {
                        format!(
                            "  {} {}: {}",
                            if c["severity"] == "error" { "!" } else { "·" },
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
        line(buf, &mut y, format!("{headline} · {steps} {}", tr("steps")), t.secondary.patch(t.panel));
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
            line(buf, &mut y, format!("  • {group}"), t.secondary.patch(t.panel));
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
                            "▮".repeat(d["width"].as_u64().unwrap_or(0).max(1) as usize),
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
                        if detail.is_empty() { format!("  • {text}") } else { format!("  • {text} — {detail}") }
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
            line(buf, &mut y, format!("• {item}"), t.secondary.patch(t.panel));
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
        line(buf, &mut y, format!("• {name}{flag}"), t.text.patch(t.panel));
        let schedules = routine["schedules"]
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" · "))
            .unwrap_or_default();
        if !schedules.is_empty() {
            line(buf, &mut y, format!("  {schedules}"), t.secondary.patch(t.panel));
        }
        if let Some(next) = routine["nextIn"].as_str() {
            line(buf, &mut y, format!("  {}: {next}", tr("Next in")), t.dim.patch(t.panel));
        }
    }
    footer(buf, inner, None, "esc · shift+←→ pages");
}
