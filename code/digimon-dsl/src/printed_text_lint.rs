//! Authoring checks against the official printed text; no runtime semantics.

use crate::clause::{ClauseSpec, DeclarativeKind, Timing, TimingSet};
use crate::security_icon_lint::PrintedSection;
use crate::spec::CardSpec;
use serde_json::Value;

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
    words
        .iter()
        .enumerate()
        .any(|(i, w)| *w == "may" && words.get(i + 1) != Some(&"not"))
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
            )
    })
}

fn has_optional(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, v)| {
            ((key == "optional" && (v == &Value::Bool(true) || v.is_array()))
                || (matches!(key.as_str(), "outer_prompt" | "optional_zero")
                    && v == &Value::Bool(true)))
                || (key == "select_zone_cards" && v["min"].as_u64().unwrap_or(0) == 0)
                || (key == "select_effect_choice"
                    && v["labels"].as_array().map_or(false, |labels| {
                        labels.iter().filter_map(Value::as_str).any(|label| {
                            let label = normalize(label);
                            label == "decline" || label.starts_with("don't ")
                        })
                    }))
                || (!matches!(key.as_str(), "text" | "summary" | "summary_key") && has_optional(v))
        }),
        Value::Array(items) => items.iter().any(has_optional),
        _ => false,
    }
}

fn leading_tags(mut text: &str) -> Vec<&str> {
    let mut tags = Vec::new();
    loop {
        text = text.trim_start();
        let Some(rest) = text.strip_prefix('[') else {
            break;
        };
        let Some((tag, tail)) = rest.split_once(']') else {
            break;
        };
        tags.push(tag);
        text = tail;
    }
    tags
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
        let keyword = matches!(clause, ClauseSpec::Declarative(c) if c.kind == DeclarativeKind::GrantKeyword)
            && (text.starts_with('<') || text.starts_with('＜'));
        // Keyword machinery owns the choices described by keyword reminder text.
        if !keyword {
            match (has_may(&text), has_optional(&value)) {
                (true, false) => report("may_without_optional"),
                (false, true) if !has_optional_payment(&text) => report("optional_without_may"),
                _ => (),
            }
        }
        let tags = leading_tags(&text);
        let mapped: Vec<_> = tags.iter().filter_map(|tag| timing(tag)).collect();
        let timing_matches = match clause {
            ClauseSpec::Triggered(c) => mapped.iter().any(|t| match &c.when {
                TimingSet::Single(when) => when == t,
                TimingSet::Multi(when) => when.contains(t),
            }),
            ClauseSpec::Declarative(_) => false,
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
