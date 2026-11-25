//! Authoring checks against the official printed text; no runtime semantics.

use crate::clause::{ClauseSpec, DeclarativeKind, Timing, TimingSet};
use crate::security_icon_lint::PrintedSection;
use crate::spec::CardSpec;
use serde_json::Value;

#[path = "printed_text_lint_picks.rs"]
mod picks;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintedTextFinding {
    pub kind: &'static str,
    /// Stable flattened index: effects, then DUAL Digimon, then DUAL Option.
    pub clause_index: usize,
}

fn normalize(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '‘' | '’' | 'ʼ' => '\'',
            '“' | '”' => '"',
            '–' | '—' | '‑' | '−' => '-',
            _ => c,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn has_may(text: &str) -> bool {
    let words: Vec<_> = text
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty())
        .collect();
    words.iter().enumerate().any(|(i, w)| {
        *w == "may"
                && words.get(i + 1) != Some(&"not")
                // Use-requirement flood gates ("you may ignore this card's color
                // requirements") describe a play permission, not a prompt.
                && !(words.get(i + 1) == Some(&"ignore")
                    && words[i + 1..].iter().take(6).any(|w| *w == "color"))
    })
}

/// Printed wording without parenthesized reminder text, which explains keyword
/// machinery rather than the clause's own choices.
fn without_reminders(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth = 0usize;
    for c in text.chars() {
        match c {
            '(' | '（' => depth += 1,
            ')' | '）' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => (),
        }
    }
    out
}

/// "Activate this card's [Main] effects" style clauses delegate their wording,
/// and so their optionality, to the referenced effect.
fn references_other_effect(text: &str) -> bool {
    text.contains("activate this card's [main] effect")
        || text.contains("activate 1 [main] effect on this card's option side")
}

fn has_up_to(text: &str) -> bool {
    text.contains("up to")
}

fn has_optional_payment(text: &str) -> bool {
    let words: Vec<_> = text
        .split(|c: char| !c.is_alphabetic())
        .filter(|s| !s.is_empty())
        .collect();
    words.windows(2).any(|pair| {
        pair[0] == "by"
            && matches!(
                pair[1],
                "trashing"
                    | "suspending"
                    | "deleting"
                    | "returning"
                    | "placing"
                    | "discarding"
                    | "paying"
                    | "attacking"
                    | "digivolving"
                    | "revealing"
                    | "adding"
                    | "choosing"
                    | "unsuspending"
            )
    })
}

fn has_optional(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            map.get("kind").and_then(Value::as_str) == Some("delay")
                || map.iter().any(|(key, v)| {
                    ((key == "optional" && (v == &Value::Bool(true) || v.is_array()))
                        || (matches!(key.as_str(), "outer_prompt" | "optional_zero")
                            && v == &Value::Bool(true)))
                        || (key.starts_with("may_") && v.is_object())
                        || key == "arm_digivolve_cost_reducer"
                        || (key == "link_cards" && v["count"].get("up_to").is_some())
                        || (key == "select_zone_cards" && v["min"].as_u64().unwrap_or(0) == 0)
                        || (matches!(
                            key.as_str(),
                            "select_own_sources" | "select_under_tamer_sources"
                        ) && v["min"].as_u64() == Some(0)
                            && v["max"].as_u64().map_or(false, |max| max > 0))
                        || (key == "select_reveal_buckets"
                            && v["buckets"].as_array().map_or(false, |buckets| {
                                buckets.iter().any(|bucket| {
                                    bucket["min"].as_u64().unwrap_or(0) == 0
                                        && bucket["max"].as_u64().unwrap_or(1) > 0
                                })
                            }))
                        || (key == "select_effect_choice"
                            && v["labels"].as_array().map_or(false, |labels| {
                                labels.iter().filter_map(Value::as_str).any(|label| {
                                    let label = normalize(label);
                                    label == "decline" || label.starts_with("don't ")
                                })
                            }))
                        || (!matches!(key.as_str(), "text" | "summary" | "summary_key")
                            && has_optional(v))
                })
        }
        Value::Array(items) => items.iter().any(has_optional),
        _ => false,
    }
}

fn leading_tags(mut text: &str) -> (Vec<&str>, Vec<&str>) {
    let mut tags = Vec::new();
    let mut zones = Vec::new();
    loop {
        text = text.trim_start();
        if let Some(rest) = text.strip_prefix('{') {
            let Some((zone, tail)) = rest.split_once('}') else {
                break;
            };
            zones.push(zone);
            text = tail;
            continue;
        }
        let (rest, close) = if let Some(rest) = text.strip_prefix('[') {
            (rest, ']')
        } else if let Some(rest) = text.strip_prefix('＜') {
            (rest, '＞')
        } else if let Some(rest) = text.strip_prefix('<') {
            (rest, '>')
        } else {
            break;
        };
        let Some((tag, tail)) = rest.split_once(close) else {
            break;
        };
        if matches!(tag, "hand" | "trash" | "breeding" | "battle area" | "field") {
            zones.push(tag);
        } else {
            tags.push(tag);
        }
        text = tail;
    }
    (tags, zones)
}

fn main_zone_timing(zone: &str) -> Option<Timing> {
    match zone {
        "hand" => Some(Timing::MainFromHand),
        "trash" => Some(Timing::MainFromTrash),
        "battle area" | "field" => Some(Timing::MainOnField),
        _ => None,
    }
}

fn timing(tag: &str) -> Option<Timing> {
    Some(match tag {
        "on play" => Timing::OnPlay,
        "when digivolving" => Timing::WhenDigivolving,
        "when attacking" => Timing::WhenAttacking,
        "on deletion" => Timing::OnDeletion,
        "start of your main phase" => Timing::StartOfYourMainPhase,
        "start of opponent's main phase" => Timing::StartOfOpponentsMainPhase,
        "start of your turn" => Timing::StartOfYourTurn,
        "start of opponent's turn" => Timing::StartOfOpponentsTurn,
        "end of your turn" => Timing::EndOfYourTurn,
        "end of opponent's turn" => Timing::EndOfOpponentsTurn,
        "end of all turns" => Timing::EndOfAllTurns,
        "end of attack" => Timing::EndOfAttack,
        "main" => Timing::Main,
        "security" => Timing::OnSecurity,
        "counter" => Timing::Counter,
        _ => return None,
    })
}

fn turn_gate(value: &Value, key: &str) -> bool {
    value.get(key) == Some(&Value::Bool(true))
        || value
            .get("all_of")
            .and_then(Value::as_array)
            .map_or(false, |items| items.iter().any(|v| turn_gate(v, key)))
}

pub fn check_printed_text(spec: &CardSpec, printed: &[PrintedSection]) -> Vec<PrintedTextFinding> {
    let mut clauses: Vec<_> = spec.effects.iter().collect();
    if let Some(dual) = &spec.dual {
        clauses.extend(&dual.digimon.effects);
        clauses.extend(&dual.option.effects);
    }
    let printed: Vec<_> = printed.iter().map(|s| normalize(&s.text)).collect();
    let mut findings = Vec::new();
    for (clause_index, clause) in clauses.into_iter().enumerate() {
        let mut report = |kind| findings.push(PrintedTextFinding { kind, clause_index });
        let text = match clause {
            ClauseSpec::Triggered(c) => &c.text,
            ClauseSpec::Declarative(c) => &c.text,
        };
        let Some(text) = text else {
            report("missing_text");
            continue;
        };
        let text = normalize(text);
        if text.is_empty() || !printed.iter().any(|p| p.contains(&text)) {
            report("text_not_printed");
        }
        let value = serde_json::to_value(clause).expect("clause serializes");
        for kind in picks::check(&value, &text) {
            report(kind);
        }
        let keyword = matches!(clause, ClauseSpec::Declarative(c) if c.kind == DeclarativeKind::GrantKeyword)
            && (text.starts_with('<') || text.starts_with('＜'));
        // Keyword machinery owns the choices described by keyword reminder text.
        if !keyword && !references_other_effect(&text) {
            let text = without_reminders(&text);
            let detach = matches!(clause, ClauseSpec::Declarative(c) if c.kind == DeclarativeKind::Replacement)
                && text.strip_prefix(['<', '＜']).is_some_and(|keyword| {
                    keyword.split([' ', '>', '＞']).next() == Some("detach")
                });
            let activated_payment = has_optional_payment(&text)
                && !has_may(&text)
                && matches!(clause, ClauseSpec::Triggered(c) if match &c.when {
                    TimingSet::Single(t) => matches!(t, Timing::MainFromHand | Timing::MainOnField),
                    TimingSet::Multi(ts) => !ts.is_empty() && ts.iter().all(|t| matches!(t, Timing::MainFromHand | Timing::MainOnField)),
                });
            let text_optional = has_may(&text)
                || has_optional_payment(&text)
                || detach
                || matches!(clause, ClauseSpec::Declarative(c) if c.kind == DeclarativeKind::Delay);
            match (text_optional, has_optional(&value) || activated_payment) {
                (true, false) => report("may_without_optional"),
                (false, true) if !has_up_to(&text) => report("optional_without_may"),
                _ => (),
            }
        }
        let (tags, zones) = leading_tags(&text);
        let main_zones: Vec<_> = zones
            .iter()
            .filter_map(|zone| main_zone_timing(zone))
            .collect();
        let mapped: Vec<_> = tags
            .iter()
            .filter_map(|tag| timing(tag))
            .flat_map(|t| {
                if t == Timing::Main && !main_zones.is_empty() {
                    main_zones.clone()
                } else {
                    vec![t]
                }
            })
            .collect();
        let timing_matches = match clause {
            ClauseSpec::Triggered(c) => mapped.iter().any(|t| match &c.when {
                TimingSet::Single(when) => {
                    when == t
                        || (zones.is_empty()
                            && *t == Timing::Main
                            && matches!(when, Timing::MainFromHand | Timing::MainOnField))
                }
                TimingSet::Multi(when) => {
                    when.contains(t)
                        || (zones.is_empty()
                            && *t == Timing::Main
                            && (when.contains(&Timing::MainFromHand)
                                || when.contains(&Timing::MainOnField)))
                }
            }),
            ClauseSpec::Declarative(c) => {
                c.kind == DeclarativeKind::Delay
                    && value["trigger"] == "delayed"
                    && mapped == [Timing::Main]
                    && (text.contains("＜delay＞") || text.contains("<delay>"))
            }
        };
        let turn_mismatch = tags.iter().any(|tag| {
            let key = match *tag {
                "your turn" => "your_turn",
                "opponent's turn" => "opponents_turn",
                "all turns" => "all_turns",
                _ => return false,
            };
            // All-turn effects are also represented by the absence of a turn gate.
            let active = &value["active_when"];
            if key == "all_turns" {
                turn_gate(active, "your_turn") || turn_gate(active, "opponents_turn")
            } else {
                !turn_gate(active, key)
            }
        });
        if (!mapped.is_empty() && !timing_matches) || turn_mismatch {
            report("timing_tag_mismatch");
        }
        let capped = value["once_per_turn"] == Value::Bool(true)
            || value["max_per_turn"].as_u64().map_or(false, |n| n > 0)
            || value["opt_group"].as_str().is_some();
        if tags.contains(&"once per turn") != capped {
            report("once_per_turn_mismatch");
        }
    }
    findings
}
