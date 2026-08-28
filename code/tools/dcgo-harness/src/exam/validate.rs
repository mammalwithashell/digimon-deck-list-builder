//! Lint a draft scenario before it is run.
//!
//! Not a general schema validator — a **memory of what went wrong**. Every rule
//! here corresponds to a failure family the first campaign actually hit:
//!
//! - `unknown-clause-id` — the orphan class: a scenario naming a clause the
//!   extractor does not produce passes its own assertions while covering
//!   nothing in the denominator, an invisible sixth verdict class.
//! - `unstacked-card` — sim-only does not shuffle and DCGO does, so a line
//!   naming a card its `stack:` does not can lower in one mode and fail in the
//!   other.
//! - `security-contents-assert` — security is a count in the projection
//!   precisely because its contents are hidden information.
//!
//! Findings carry the guide topic that explains them, so an agent can fix the
//! draft without re-reading the whole contract.
//!
//! Deliberately does NOT parse into `exam::scenario::Scenario` / `StepAction`:
//! that parser is strict by design (rule 27 of CLAUDE.md — an unknown verb or
//! a malformed clause id must fail loudly rather than desync a line), so it
//! stops at the FIRST problem. A lint that also stopped at the first problem
//! would send an author back through Unity-adjacent tooling once per mistake.
//! Instead this module walks the YAML as loosely-typed `serde_yml::Value` and
//! collects every finding in one pass — but it reuses the parser's own
//! **vocabulary** (`scenario::STEP_VERBS`) rather than redeclaring it, so the
//! two never disagree about what a legal verb is. The prompt-kind vocabulary
//! has no equivalent shared constant anywhere in this crate today (DCGO's
//! class names are matched dynamically in `adapter.rs`, and the sim-side
//! labels like `main_phase` / `mulligan` / `breeding_action` are free-form
//! strings threaded through `main.rs`) — that is the fallback case the task
//! brief anticipates, so the 13-kind list below is declared locally.

use crate::exam::scenario::STEP_VERBS;

/// One lint finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Stable kebab-case rule id, e.g. `unknown-verb`.
    pub rule: String,
    pub message: String,
    /// `exam_authoring_guide` topic that explains this rule.
    pub guide_topic: String,
}

impl Finding {
    fn new(rule: &str, message: String, topic: &str) -> Finding {
        Finding {
            rule: rule.to_string(),
            message,
            guide_topic: topic.to_string(),
        }
    }
}

/// The 13 prompt kinds a `expect:` may name.
///
/// **Canonical source: `docs/DCGO_EXAM.md`, "DCGO has 13 decision kinds".**
/// This is a hand-kept copy, because the crate has no single reusable Rust
/// definition -- `adapter.rs` computes DCGO names through scattered match
/// arms and several kinds (`SelectCountEffect`, `SelectDigiXrosClass`,
/// `generic_int`) never appear there as literals at all.
///
/// Nothing guards the two against drift. If a 14th decision kind is ever
/// added to that table, add it HERE too -- otherwise this linter rejects a
/// legitimately new prompt kind, which is the false-positive class it exists
/// to avoid.
const PROMPT_KINDS: &[&str] = &[
    "SelectCardEffect",
    "SelectHandEffect",
    "SelectPermanentEffect",
    "SelectAttackEffect",
    "SelectCountEffect",
    "SelectDigiXrosClass",
    "MultipleSkills",
    "OptionalSkill",
    "generic_int",
    "generic_bool",
    "mulligan",
    "breeding_action",
    "main_phase",
];

#[derive(Debug, serde::Deserialize)]
struct RawScenario {
    card: Option<String>,
    clause: Option<String>,
    #[serde(default)]
    decks: serde_yml::Value,
    #[serde(default)]
    steps: Vec<serde_yml::Value>,
    #[serde(default)]
    assert: Vec<serde_yml::Value>,
}

/// Lint `text`. An empty result is clean.
///
/// `known_clause_ids` is the clause-coverage denominator when the caller has
/// it; without it the clause-id check degrades to the card-prefix check only,
/// and says so rather than silently skipping.
pub fn validate_yaml(text: &str, known_clause_ids: Option<&[String]>) -> Vec<Finding> {
    let mut out = Vec::new();

    let parsed: RawScenario = match serde_yml::from_str(text) {
        Ok(p) => p,
        Err(e) => {
            out.push(Finding::new(
                "unparseable",
                format!("scenario YAML does not parse: {e}"),
                "format",
            ));
            return out;
        }
    };

    let card = parsed.card.clone().unwrap_or_default();
    if card.is_empty() {
        out.push(Finding::new("missing-card", "`card:` is required".into(), "format"));
    }

    match parsed.clause.as_deref() {
        None | Some("") => out.push(Finding::new(
            "missing-clause",
            "`clause:` is required and must be a clause_coverage id".into(),
            "format",
        )),
        Some(clause) => {
            let prefix = clause.split('#').next().unwrap_or_default();
            if !card.is_empty() && prefix != card {
                out.push(Finding::new(
                    "clause-card-mismatch",
                    format!("clause {clause:?} does not belong to card {card:?}"),
                    "format",
                ));
            }
            if let Some(known) = known_clause_ids {
                if !known.iter().any(|k| k == clause) {
                    out.push(Finding::new(
                        "unknown-clause-id",
                        format!(
                            "clause {clause:?} is not produced by the extractor for this card; \
                             a scenario naming an unknown clause covers nothing in the denominator"
                        ),
                        "verdicts",
                    ));
                }
            }
        }
    }

    // Verbs and prompt kinds.
    let mut named_cards: Vec<String> = Vec::new();
    for (i, step) in parsed.steps.iter().enumerate() {
        if let Some(do_map) = step.get("do").and_then(|v| v.as_mapping()) {
            for (k, v) in do_map {
                let verb = k.as_str().unwrap_or_default();
                if !STEP_VERBS.contains(&verb) {
                    out.push(Finding::new(
                        "unknown-verb",
                        format!("step {i}: verb {verb:?} is not in the vocabulary ({STEP_VERBS:?})"),
                        "steps",
                    ));
                }
                if let Some(c) = v.get("card").and_then(|c| c.as_str()) {
                    named_cards.push(c.to_string());
                }
            }
        }
        if let Some(prompt) = step
            .get("expect")
            .and_then(|e| e.get("prompt"))
            .and_then(|p| p.as_str())
        {
            if !PROMPT_KINDS.contains(&prompt) {
                out.push(Finding::new(
                    "unknown-prompt-kind",
                    format!("step {i}: prompt {prompt:?} is not one of the 13 kinds"),
                    "prompts",
                ));
            }
        }
    }

    // Every card the line names must be stacked: sim-only does not shuffle.
    let stacked: Vec<String> = parsed
        .decks
        .as_mapping()
        .map(|seats| {
            seats
                .values()
                .filter_map(|seat| seat.get("stack").and_then(|s| s.as_sequence()))
                .flatten()
                .filter_map(|c| c.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    for c in &named_cards {
        if !stacked.contains(c) {
            out.push(Finding::new(
                "unstacked-card",
                format!(
                    "the line names {c:?} but no seat's `stack:` does; sim-only deals from \
                     `rest:` in file order while DCGO shuffles, so this lowers in one mode \
                     and fails in the other"
                ),
                "decks",
            ));
        }
    }

    // Security contents are hidden information.
    for (i, a) in parsed.assert.iter().enumerate() {
        if let Some(that) = a.get("that").and_then(|t| t.as_mapping()) {
            for key in that.keys() {
                let k = key.as_str().unwrap_or_default();
                if k.contains("security.") && !k.ends_with("security.count") {
                    out.push(Finding::new(
                        "security-contents-assert",
                        format!(
                            "assert {i}: {k:?} reaches into security CONTENTS; security is a \
                             count in the projection because its contents are hidden"
                        ),
                        "assert",
                    ));
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
card: EX12-004
clause: EX12-004#effect#0
seed: 424242
decks:
  p0: { stack: [ST1-02, EX12-004], rest: toho-braves }
  p1: { stack: [], rest: toho-braves }
steps:
  - actor: 0
    do:     { play: {card: EX12-004, from: hand} }
    expect: { prompt: main_phase }
"#;

    #[test]
    fn a_well_formed_scenario_has_no_findings() {
        assert!(validate_yaml(GOOD, None).is_empty());
    }

    #[test]
    fn rejects_a_clause_id_the_extractor_does_not_produce() {
        // The orphan class: a scenario naming an unknown clause passes its own
        // assertions while covering nothing in the denominator.
        let known = vec!["EX12-004#effect#0".to_string()];
        let bad = GOOD.replace("EX12-004#effect#0", "EX12-004#effect#9");
        let f = validate_yaml(&bad, Some(&known));
        assert!(f.iter().any(|f| f.rule == "unknown-clause-id"), "{f:?}");
    }

    #[test]
    fn rejects_a_clause_id_that_does_not_belong_to_the_card() {
        let bad = GOOD.replace("clause: EX12-004#effect#0", "clause: BT8-084#effect#0");
        let f = validate_yaml(&bad, None);
        assert!(f.iter().any(|f| f.rule == "clause-card-mismatch"), "{f:?}");
    }

    #[test]
    fn rejects_a_verb_outside_the_vocabulary() {
        let bad = GOOD.replace("play:", "teleport:");
        let f = validate_yaml(&bad, None);
        assert!(f.iter().any(|f| f.rule == "unknown-verb"), "{f:?}");
    }

    #[test]
    fn rejects_a_prompt_kind_outside_the_thirteen() {
        let bad = GOOD.replace("prompt: main_phase", "prompt: SelectSomething");
        let f = validate_yaml(&bad, None);
        assert!(f.iter().any(|f| f.rule == "unknown-prompt-kind"), "{f:?}");
    }

    #[test]
    fn flags_a_card_the_line_names_but_the_stack_does_not() {
        // The sim-only trap: sim-only does not shuffle, DCGO does, so an
        // unstacked card can lower in one mode and fail in the other.
        let bad = GOOD.replace("stack: [ST1-02, EX12-004]", "stack: [ST1-02]");
        let f = validate_yaml(&bad, None);
        assert!(f.iter().any(|f| f.rule == "unstacked-card"), "{f:?}");
    }

    #[test]
    fn flags_an_assert_over_security_contents() {
        let bad = format!("{GOOD}assert:\n  - at: 1\n    that: {{ p0.security.0: EX12-004 }}\n");
        let f = validate_yaml(&bad, None);
        assert!(f.iter().any(|f| f.rule == "security-contents-assert"), "{f:?}");
    }

    #[test]
    fn every_finding_points_at_a_guide_topic() {
        let bad = GOOD.replace("play:", "teleport:");
        for f in validate_yaml(&bad, None) {
            assert!(!f.guide_topic.is_empty(), "{f:?} must route to a guide topic");
        }
    }
}
