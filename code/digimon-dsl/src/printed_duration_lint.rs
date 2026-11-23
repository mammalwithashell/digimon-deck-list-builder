//! Printed-duration lint: a card that prints a duration tied to ONE player's
//! turn must author the matching player-relative expiry.
//!
//! "until your turn ends" / "until the end of your turn" lasts until the end
//! of the turn of the card's player ("you" is the player of that card, CR 4.3
//! 15-10-1-1-1); "until your opponent's turn ends" / "until the end of their
//! turn" lasts until the end of the opponent's turn. When the effect resolves
//! on the OTHER player's turn, the plain `end_of_turn` expiry ends it a turn
//! early. Blastmon (EX10-034) "[All Turns] ... gains <Security A. +1> and
//! +3000 DP until your turn ends" was authored `end_of_turn` and lost both at
//! the end of the opponent's turn (scan finding F18); DCGO uses
//! `EffectDuration.UntilOwnerTurnEnd` / `UntilOpponentTurnEnd` for these.
//!
//! The check is a count over the whole card, not a per-clause match: for each
//! of the two printed phrases, the spec must carry at least as many expiries of
//! the matching family (`end_of_your_turn` / `end_of_your_next_turn`, or
//! `end_of_opponents_turn` / `end_of_opponents_next_turn`) as the printed text
//! has phrases. A clause that prints the phrase but authors `end_of_turn`
//! drops the count below the printed one. "next turn" phrases are not counted
//! (they have their own expiries and never match the patterns below).
//!
//! The printed text is the official Bandai DB mirror (`data/card_official.json`
//! `text_sections`). Where the mirror's English differs from the card face, the
//! face wins (see [`PRINTED_DURATION_EXEMPT`]).

use crate::security_icon_lint::PrintedSection;
use serde_yml::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintedDurationFinding {
    pub path: String,
    pub message: String,
}

/// Cards exempt from the check, each with the reason. An entry must still be
/// needed: the CI gate fails on an exemption the card no longer trips.
pub const PRINTED_DURATION_EXEMPT: &[(&str, &str)] = &[
    (
        "BT25-057",
        "Monarchlizamon: the mirror's Option text reads \"+5000 DP until your opponent's turn \
         ends\"; the card face prints \"for the turn\" and DCGO BT25_057.cs uses \
         UntilEachTurnEnd. The face wins (end_of_turn).",
    ),
    (
        "EX8-022",
        "Frigimon: the mirror repeats the inherited \"[When Attacking] ... until the end of \
         their turn\" as a \"Security Effect\" section; the card face has no security effect, \
         so the phrase is printed once (one end_of_opponents_turn is authored).",
    ),
    (
        "EX11-074",
        "Vortexdramon: readiness slice; the [When Digivolving]/[When Attacking] clause that \
         prints \"until your opponent's turn ends\" is not implemented yet (YAML header).",
    ),
    (
        "ST17-07",
        "Rapidmon: the green-Tamer protection rider (\"until the end of your opponent's turn\") \
         is omitted as BLOCKED on G-OPPONENT-SCOPED-EFFECT-PROTECTION-DSL (YAML header).",
    ),
];

const YOUR_TURN_PHRASES: &[&str] = &["until the end of your turn", "until your turn ends"];
const OPPONENT_TURN_PHRASES: &[&str] = &[
    "until the end of your opponent's turn",
    "until the end of their turn",
    "until your opponent's turn ends",
    "until their turn ends",
];
const YOUR_TURN_EXPIRIES: &[&str] = &["end_of_your_turn", "end_of_your_next_turn"];
const OPPONENT_TURN_EXPIRIES: &[&str] = &["end_of_opponents_turn", "end_of_opponents_next_turn"];

fn count_phrases(text: &str, phrases: &[&str]) -> usize {
    let text = text.replace(['\u{2019}', '\u{2018}'], "'").to_lowercase();
    phrases.iter().map(|p| text.matches(p).count()).sum()
}

/// Printed "until your turn ends" and "until your opponent's turn ends"
/// counts (both spellings of each) over all printed sections.
pub fn printed_duration_counts(sections: &[PrintedSection]) -> (usize, usize) {
    sections.iter().fold((0, 0), |(y, o), s| {
        (
            y + count_phrases(&s.text, YOUR_TURN_PHRASES),
            o + count_phrases(&s.text, OPPONENT_TURN_PHRASES),
        )
    })
}

fn walk_expiries(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Mapping(map) => {
            for (key, child) in map {
                if key.as_str() == Some("expiry") {
                    if let Some(s) = child.as_str() {
                        out.push(s.to_string());
                    }
                }
                walk_expiries(child, out);
            }
        }
        Value::Sequence(items) => items.iter().for_each(|i| walk_expiries(i, out)),
        Value::Tagged(t) => walk_expiries(&t.value, out),
        _ => {}
    }
}

/// Authored (your-turn family, opponent-turn family) expiry counts in a card's
/// raw YAML, at any depth.
pub fn authored_expiry_counts(raw_yaml: &str) -> (usize, usize) {
    let Ok(value) = serde_yml::from_str::<Value>(raw_yaml) else {
        return (0, 0);
    };
    let mut expiries = Vec::new();
    walk_expiries(&value, &mut expiries);
    let count = |family: &[&str]| expiries.iter().filter(|e| family.contains(&e.as_str())).count();
    (count(YOUR_TURN_EXPIRIES), count(OPPONENT_TURN_EXPIRIES))
}

/// Check one card, ignoring [`PRINTED_DURATION_EXEMPT`].
pub fn check_printed_durations_unexempted(
    raw_yaml: &str,
    sections: &[PrintedSection],
) -> Vec<PrintedDurationFinding> {
    let (printed_yours, printed_opps) = printed_duration_counts(sections);
    let (authored_yours, authored_opps) = authored_expiry_counts(raw_yaml);
    let mut out = Vec::new();
    if authored_yours < printed_yours {
        out.push(PrintedDurationFinding {
            path: "effects".into(),
            message: format!(
                "the card prints \"until your turn ends\" {printed_yours} time(s) but the spec has \
                 {authored_yours} `end_of_your_turn`/`end_of_your_next_turn` expiry(ies); a clause \
                 authored `end_of_turn` ends a turn early when it resolves on the opponent's turn \
                 (CR 4.3 15-10-1-1-1)"
            ),
        });
    }
    if authored_opps < printed_opps {
        out.push(PrintedDurationFinding {
            path: "effects".into(),
            message: format!(
                "the card prints \"until your opponent's turn ends\" {printed_opps} time(s) but the \
                 spec has {authored_opps} `end_of_opponents_turn`/`end_of_opponents_next_turn` \
                 expiry(ies); `end_of_turn` ends it at the end of the current turn"
            ),
        });
    }
    out
}

/// Check one card (`card_id` from the spec), honouring the exemptions.
pub fn check_printed_durations(
    card_id: &str,
    raw_yaml: &str,
    sections: &[PrintedSection],
) -> Vec<PrintedDurationFinding> {
    if PRINTED_DURATION_EXEMPT.iter().any(|(id, _)| *id == card_id) {
        return Vec::new();
    }
    check_printed_durations_unexempted(raw_yaml, sections)
}

/// `data/card_official.json` text sections by card id.
pub fn load_printed_sections(
    card_official: &std::path::Path,
) -> Result<std::collections::HashMap<String, Vec<PrintedSection>>, String> {
    let raw = std::fs::read_to_string(card_official).map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let cards = v["cards"].as_object().ok_or("no `cards` object")?;
    Ok(cards
        .iter()
        .map(|(id, c)| {
            let sections = c["text_sections"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|s| PrintedSection {
                            label: s["label"].as_str().unwrap_or_default().to_string(),
                            text: s["text"].as_str().unwrap_or_default().to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            (id.clone(), sections)
        })
        .collect())
}

/// Result of [`check_pool`].
#[derive(Debug, Default)]
pub struct PoolReport {
    /// Specs that had printed text to compare against.
    pub checked: usize,
    /// One line per finding, `<file>: <message>`.
    pub findings: Vec<String>,
    /// Exempt cards that no longer trip the check (the exemption should go).
    pub stale_exemptions: Vec<String>,
}

/// Run the check over every production spec (`authoring_lint::pool_card_files`).
/// The card id is the file stem (`EX10-034.yaml`).
pub fn check_pool(
    cards_root: &std::path::Path,
    card_official: &std::path::Path,
) -> Result<PoolReport, String> {
    let printed = load_printed_sections(card_official)?;
    let mut report = PoolReport::default();
    for file in crate::authoring_lint::pool_card_files(cards_root) {
        let Some(id) = file.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(sections) = printed.get(id) else {
            continue;
        };
        let Ok(raw) = std::fs::read_to_string(&file) else {
            continue;
        };
        report.checked += 1;
        let unexempted = check_printed_durations_unexempted(&raw, sections);
        if PRINTED_DURATION_EXEMPT.iter().any(|(e, _)| *e == id) {
            if unexempted.is_empty() {
                report.stale_exemptions.push(id.to_string());
            }
            continue;
        }
        for f in unexempted {
            report.findings.push(format!("{id}: {}", f.message));
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effect(text: &str) -> Vec<PrintedSection> {
        vec![PrintedSection {
            label: "Effect".into(),
            text: text.into(),
        }]
    }

    #[test]
    fn your_turn_phrase_with_end_of_turn_is_flagged() {
        let yaml = "card: X\neffects:\n  - process:\n      - add_dp_modifier: { target: source, value: 3000, expiry: end_of_turn }\n";
        let f = check_printed_durations("X", yaml, &effect("gets +3000 DP until your turn ends."));
        assert_eq!(f.len(), 1, "{f:?}");
    }

    #[test]
    fn your_turn_phrase_with_end_of_your_turn_passes() {
        let yaml = "card: X\neffects:\n  - process:\n      - add_dp_modifier:\n          target: source\n          value: 3000\n          expiry: end_of_your_turn\n";
        assert!(check_printed_durations("X", yaml, &effect("+3000 DP until the end of your turn.")).is_empty());
    }

    #[test]
    fn opponent_phrase_needs_an_opponent_expiry_and_next_turn_is_ignored() {
        let bad = "card: X\neffects:\n  - process:\n      - add_dp_modifier: { target: t, value: -3000, expiry: end_of_turn }\n";
        let text = "1 of their Digimon gets -3000 DP until their turn ends.";
        assert_eq!(check_printed_durations("X", bad, &effect(text)).len(), 1);
        let good = bad.replace("end_of_turn", "end_of_opponents_turn");
        assert!(check_printed_durations("X", &good, &effect(text)).is_empty());
        // "until the end of your opponent's next turn" is not this phrase.
        let next = "gets -3000 DP until the end of your opponent\u{2019}s next turn.";
        assert!(check_printed_durations("X", bad, &effect(next)).is_empty());
    }
}
