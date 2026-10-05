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
//! - `orphan-interaction` / `unknown-covered-clause` — the same orphan class
//!   for interaction exams (card-loop design D8): an `interaction.id` absent
//!   from the committed interaction denominator, or a `covers:` id the
//!   extractor does not produce, covers nothing that gates anything. With no
//!   denominator at all, an interaction scenario is refused
//!   (`interaction-denominator-missing`) rather than waved through; a legacy
//!   clause scenario never needs it.
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

use crate::exam::scenario::{is_clause_id_shaped, parse_probe_id, STEP_VERBS};
use crate::exam::verdict::InteractionBook;

/// The interaction denominator as the lint sees it. Injected by the caller (the
/// MCP's `exam_validate` loads `data/interaction_denominator.json` or an
/// explicit path) so tests and other roots can point it anywhere.
#[derive(Debug, Clone, Copy)]
pub enum InteractionDenominator<'a> {
    Loaded(&'a InteractionBook),
    /// No denominator is available; the text says why, and is quoted in the
    /// finding an interaction scenario gets.
    Missing(&'a str),
}

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
    covers: Option<serde_yml::Value>,
    #[serde(default)]
    interaction: Option<serde_yml::Value>,
    #[serde(default)]
    expect_ruling: Option<serde_yml::Value>,
    #[serde(default)]
    decks: serde_yml::Value,
    #[serde(default)]
    steps: Vec<serde_yml::Value>,
    #[serde(default)]
    assert: Vec<serde_yml::Value>,
}

const NO_DENOMINATOR: &str = "no interaction denominator was supplied to this check";

/// Lint `text` with no interaction denominator. An empty result is clean.
///
/// `known_clause_ids` is the clause-coverage denominator when the caller has
/// it; without it the clause-id check degrades to the card-prefix check only,
/// and says so rather than silently skipping. An interaction scenario is
/// always refused here -- use [`validate_scenario`] with the denominator.
pub fn validate_yaml(text: &str, known_clause_ids: Option<&[String]>) -> Vec<Finding> {
    validate_scenario(
        text,
        known_clause_ids,
        InteractionDenominator::Missing(NO_DENOMINATOR),
    )
}

/// [`validate_yaml`] plus the `deck-budget` rule ([`deck_budget`]) when a
/// deck book is given.
pub fn validate_yaml_with_decks(
    text: &str,
    known_clause_ids: Option<&[String]>,
    book: Option<&crate::exam::deckbook::DeckBook>,
) -> Vec<Finding> {
    let mut out = validate_yaml(text, known_clause_ids);
    if let Some(book) = book {
        out.extend(deck_budget(text, book));
    }
    out
}

/// `deck-budget`: each seat's `stack:` must be suppliable from its `rest:`
/// deck's MAIN deck -- `stack:` orders the main deck (DCGO shuffles the egg
/// deck), and the job builder refuses a stacked card the main deck cannot
/// supply. Caught here, before lowering, it reads as the copy-count error it
/// is rather than as `stacked card X is not in deck`. An unknown `rest:` deck
/// is reported too. An unparseable scenario yields nothing (the `unparseable`
/// rule reports it).
pub fn deck_budget(text: &str, book: &crate::exam::deckbook::DeckBook) -> Vec<Finding> {
    let Ok(parsed) = serde_yml::from_str::<RawScenario>(text) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for seat in ["p0", "p1"] {
        let Some(seat_v) = parsed.decks.get(seat) else { continue };
        let Some(rest) = seat_v.get("rest").and_then(|v| v.as_str()) else { continue };
        let stack: Vec<String> = seat_v
            .get("stack")
            .and_then(|v| v.as_sequence())
            .map(|s| s.iter().filter_map(|c| c.as_str().map(str::to_string)).collect())
            .unwrap_or_default();
        let entry = match book.resolve(rest) {
            Ok(e) => e,
            Err(e) => {
                out.push(Finding::new("deck-budget", format!("{seat}: {e}"), "decks"));
                continue;
            }
        };
        let mut stacked: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
        for id in &stack {
            *stacked.entry(id.as_str()).or_default() += 1;
        }
        for (id, k) in stacked {
            let m = entry.main.iter().filter(|c| c.as_str() == id).count();
            if k > m {
                out.push(Finding::new(
                    "deck-budget",
                    format!("{seat} stacks {k}x {id} but deck `{rest}` holds {m} in its main deck"),
                    "decks",
                ));
            }
        }
    }
    out
}

/// Lint `text` against both denominators. An empty result is clean.
pub fn validate_scenario(
    text: &str,
    known_clause_ids: Option<&[String]>,
    interactions: InteractionDenominator<'_>,
) -> Vec<Finding> {
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

    // Security contents are hidden information -- in `assert:` and in the
    // ruling's assertions alike.
    let ruling_rows: Vec<serde_yml::Value> = parsed
        .expect_ruling
        .as_ref()
        .and_then(|r| r.get("assert"))
        .and_then(|a| a.as_sequence())
        .cloned()
        .unwrap_or_default();
    for (label, rows) in [("assert", &parsed.assert), ("expect_ruling assert", &ruling_rows)] {
        for (i, a) in rows.iter().enumerate() {
            if let Some(that) = a.get("that").and_then(|t| t.as_mapping()) {
                for key in that.keys() {
                    let k = key.as_str().unwrap_or_default();
                    if k.contains("security.") && !k.ends_with("security.count") {
                        out.push(Finding::new(
                            "security-contents-assert",
                            format!(
                                "{label} {i}: {k:?} reaches into security CONTENTS; security \
                                 is a count in the projection because its contents are hidden"
                            ),
                            "assert",
                        ));
                    }
                }
            }
        }
    }

    let covers = lint_covers(&parsed, known_clause_ids, &mut out);
    lint_interaction(&parsed, &card, &covers, interactions, &mut out);
    lint_expect_ruling(&parsed, &mut out);

    out
}

/// `covers:` -- every id clause-shaped, known to the extractor (when the
/// caller has its denominator), and including `clause:`. Returns the effective
/// covered set for the interaction checks.
fn lint_covers(
    parsed: &RawScenario,
    known_clause_ids: Option<&[String]>,
    out: &mut Vec<Finding>,
) -> Vec<String> {
    let clause = parsed.clause.clone().unwrap_or_default();
    let Some(raw) = parsed.covers.as_ref() else {
        return vec![clause];
    };
    let Some(seq) = raw.as_sequence() else {
        out.push(Finding::new(
            "malformed-covers",
            "`covers:` must be a list of clause ids".into(),
            "format",
        ));
        return vec![clause];
    };
    let mut ids = Vec::new();
    for v in seq {
        let Some(id) = v.as_str() else {
            out.push(Finding::new(
                "malformed-covers",
                format!("covers entry {v:?} is not a clause id string"),
                "format",
            ));
            continue;
        };
        if !is_clause_id_shaped(id) {
            out.push(Finding::new(
                "malformed-covers",
                format!("covers entry {id:?} is not a clause_coverage id (card_id#zone#index)"),
                "format",
            ));
        } else if let Some(known) = known_clause_ids {
            if !known.iter().any(|k| k == id) {
                out.push(Finding::new(
                    "unknown-covered-clause",
                    format!(
                        "covers names {id:?}, which the extractor does not produce; a covered \
                         clause outside the denominator covers nothing"
                    ),
                    "verdicts",
                ));
            }
        }
        ids.push(id.to_string());
    }
    if !clause.is_empty() && !ids.contains(&clause) {
        out.push(Finding::new(
            "malformed-covers",
            format!("covers must include the scenario's own clause {clause:?}"),
            "format",
        ));
    }
    ids
}

/// `interaction:` -- shape, source/prefix agreement, and the orphan rule
/// against the interaction denominator.
fn lint_interaction(
    parsed: &RawScenario,
    card: &str,
    covers: &[String],
    denominator: InteractionDenominator<'_>,
    out: &mut Vec<Finding>,
) {
    let Some(block) = parsed.interaction.as_ref() else {
        return;
    };
    let field = |k: &str| block.get(k).and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let (id, source, kind) = (field("id"), field("source"), field("kind"));
    if id.is_empty() || !["qa", "probe", "combo"].contains(&source.as_str())
        || !["positive", "negative"].contains(&kind.as_str())
    {
        out.push(Finding::new(
            "malformed-interaction",
            format!(
                "`interaction:` needs `id`, `source: qa|probe|combo` and \
                 `kind: positive|negative` (got id {id:?}, source {source:?}, kind {kind:?})"
            ),
            "format",
        ));
        return;
    }
    if !id.starts_with(&format!("{source}:")) {
        out.push(Finding::new(
            "interaction-source-mismatch",
            format!("interaction id {id:?} does not carry the `{source}:` prefix its source names"),
            "format",
        ));
        return;
    }
    if source == "probe" {
        match parse_probe_id(&id) {
            None => out.push(Finding::new(
                "malformed-interaction",
                format!("interaction id {id:?} is not `probe:<clause-id>:<family>[:neg]`"),
                "format",
            )),
            Some((clause, _, _)) if !covers.iter().any(|c| c == clause) => out.push(Finding::new(
                "probe-clause-not-covered",
                format!(
                    "interaction {id:?} probes {clause:?}, which this scenario does not cover; \
                     add it to `covers:`"
                ),
                "format",
            )),
            Some(_) => {}
        }
    }
    if source == "combo" {
        // Model-authored combos never gate, so they are not in the
        // denominator by construction (design D6) -- nothing to orphan.
        return;
    }
    let book = match denominator {
        InteractionDenominator::Missing(why) => {
            out.push(Finding::new(
                "interaction-denominator-missing",
                format!(
                    "interaction {id:?} cannot be checked: the interaction denominator was not \
                     generated ({why}). Build it with `python -m tools.card_loop interactions \
                     build`; an interaction scenario is never validated against nothing"
                ),
                "verdicts",
            ));
            return;
        }
        InteractionDenominator::Loaded(book) => book,
    };
    let Some(entry) = book.get(&id) else {
        out.push(Finding::new(
            "orphan-interaction",
            format!(
                "interaction {id:?} is not in the interaction denominator ({}); a scenario \
                 naming an unknown interaction covers nothing that gates any card",
                book.source()
            ),
            "verdicts",
        ));
        return;
    };
    if !card.is_empty() && !entry.card_ids.iter().any(|c| c == card) {
        out.push(Finding::new(
            "interaction-card-mismatch",
            format!(
                "interaction {id:?} counts for {:?}, not for this scenario's card {card:?}",
                entry.card_ids
            ),
            "verdicts",
        ));
    }
    if entry.kind != kind {
        out.push(Finding::new(
            "interaction-kind-mismatch",
            format!("interaction {id:?} is `{}` in the denominator, not `{kind}`", entry.kind),
            "format",
        ));
    }
}

/// `expect_ruling:` -- only on a Q&A interaction, naming the same Q-number, and
/// asserting at least one observable.
fn lint_expect_ruling(parsed: &RawScenario, out: &mut Vec<Finding>) {
    let Some(r) = parsed.expect_ruling.as_ref() else {
        return;
    };
    let interaction_id = parsed
        .interaction
        .as_ref()
        .and_then(|i| i.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let q_id = r.get("q_id").and_then(|v| v.as_str()).unwrap_or_default();
    if !interaction_id.starts_with("qa:") {
        out.push(Finding::new(
            "ruling-without-qa-interaction",
            "`expect_ruling:` encodes an official ruling, so the scenario needs \
             `interaction: { id: qa:<Q-number>, source: qa, ... }`"
                .into(),
            "format",
        ));
    } else if format!("qa:{q_id}") != interaction_id {
        out.push(Finding::new(
            "ruling-q-mismatch",
            format!("expect_ruling q_id {q_id:?} does not match interaction id {interaction_id:?}"),
            "format",
        ));
    }
    let rows = r.get("assert").and_then(|a| a.as_sequence()).map(|s| s.len()).unwrap_or(0);
    if rows == 0 {
        out.push(Finding::new(
            "empty-ruling",
            "`expect_ruling.assert` asserts nothing, so the ruling leg of the three-way \
             comparison would be vacuous"
                .into(),
            "assert",
        ));
    }
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

    // -- interaction exams (card-loop design D8) ---------------------------

    const BOOK: &str = r#"{"version": 1, "interactions": {
        "qa:Q1601": {"source": "qa", "q_id": "Q1601", "card_ids": ["EX12-004", "BT1-001"],
                     "kind": "positive", "gating": true, "text_sha256": "r"},
        "probe:EX12-004#effect#0:scope:neg": {"source": "probe", "card_ids": ["EX12-004"],
                     "clause_id": "EX12-004#effect#0", "family": "scope",
                     "family_version": "families@1", "kind": "negative", "gating": true,
                     "text_sha256": "c"}}}"#;

    fn book() -> InteractionBook {
        InteractionBook::from_json(BOOK, "fixture-denominator").unwrap()
    }

    fn lint(extra: &str) -> Vec<Finding> {
        let b = book();
        validate_scenario(
            &format!("{GOOD}{extra}"),
            Some(&["EX12-004#effect#0".to_string(), "EX12-004#effect#1".to_string()]),
            InteractionDenominator::Loaded(&b),
        )
    }

    fn rules(f: &[Finding]) -> Vec<&str> {
        f.iter().map(|f| f.rule.as_str()).collect()
    }

    #[test]
    fn a_legacy_scenario_is_clean_with_or_without_the_denominator() {
        assert!(lint("").is_empty());
        assert!(validate_yaml(GOOD, Some(&["EX12-004#effect#0".to_string()])).is_empty());
    }

    #[test]
    fn a_known_interaction_is_clean() {
        let f = lint("interaction: { id: \"qa:Q1601\", source: qa, kind: positive }\n");
        assert!(f.is_empty(), "{f:?}");
        let f = lint("interaction: { id: \"probe:EX12-004#effect#0:scope:neg\", source: probe, kind: negative }\n");
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn an_orphan_interaction_is_rejected_naming_the_id() {
        let f = lint("interaction: { id: \"qa:Q9999\", source: qa, kind: positive }\n");
        assert_eq!(rules(&f), vec!["orphan-interaction"]);
        assert!(f[0].message.contains("qa:Q9999"), "{f:?}");
    }

    #[test]
    fn without_a_denominator_an_interaction_scenario_is_refused_but_a_legacy_one_is_not() {
        let text = format!("{GOOD}interaction: {{ id: \"qa:Q1601\", source: qa, kind: positive }}\n");
        let f = validate_scenario(&text, None, InteractionDenominator::Missing("no file at x"));
        assert_eq!(rules(&f), vec!["interaction-denominator-missing"]);
        assert!(f[0].message.contains("not generated"), "{f:?}");
        assert!(validate_yaml(&text, None).iter().any(|f| f.rule == "interaction-denominator-missing"));
        assert!(validate_scenario(GOOD, None, InteractionDenominator::Missing("x")).is_empty());
    }

    #[test]
    fn a_combo_interaction_needs_no_denominator() {
        let text = format!("{GOOD}interaction: {{ id: \"combo:toho:raid\", source: combo, kind: positive }}\n");
        assert!(validate_scenario(&text, None, InteractionDenominator::Missing("x")).is_empty());
    }

    #[test]
    fn unknown_covered_clauses_are_rejected() {
        let f = lint("covers: [EX12-004#effect#0, EX12-004#effect#7]\n");
        assert_eq!(rules(&f), vec!["unknown-covered-clause"]);
        assert!(f[0].message.contains("EX12-004#effect#7"));
        assert!(lint("covers: [EX12-004#effect#0, EX12-004#effect#1]\n").is_empty());
    }

    #[test]
    fn malformed_covers_are_rejected() {
        assert!(rules(&lint("covers: [EX12-004#effect#0, on_play]\n")).contains(&"malformed-covers"));
        assert!(rules(&lint("covers: [EX12-004#effect#1]\n")).contains(&"malformed-covers"));
        assert!(rules(&lint("covers: EX12-004#effect#0\n")).contains(&"malformed-covers"));
    }

    #[test]
    fn interaction_shape_and_denominator_disagreements_are_findings() {
        assert_eq!(
            rules(&lint("interaction: { id: \"qa:Q1601\", source: probe, kind: positive }\n")),
            vec!["interaction-source-mismatch"]
        );
        assert_eq!(
            rules(&lint("interaction: { id: \"qa:Q1601\", source: qa, kind: negative }\n")),
            vec!["interaction-kind-mismatch"]
        );
        assert_eq!(
            rules(&lint("interaction: { id: \"qa:Q1601\", source: qa }\n")),
            vec!["malformed-interaction"]
        );
        let other_card = format!(
            "{}interaction: {{ id: \"qa:Q1601\", source: qa, kind: positive }}\n",
            GOOD.replace("EX12-004", "ST1-02")
        );
        let b = book();
        let f = validate_scenario(&other_card, None, InteractionDenominator::Loaded(&b));
        assert!(rules(&f).contains(&"interaction-card-mismatch"), "{f:?}");
    }

    #[test]
    fn a_probe_on_an_uncovered_clause_is_a_finding() {
        let f = lint("interaction: { id: \"probe:EX12-004#effect#1:scope:neg\", source: probe, kind: negative }\n");
        assert!(rules(&f).contains(&"probe-clause-not-covered"), "{f:?}");
        assert!(rules(&f).contains(&"orphan-interaction"), "{f:?}");
    }

    /// The denominator the Python builder actually writes
    /// (`tools.card_loop.interactions build` over the fixture inputs) is what
    /// this lint reads -- not just the hand-written shape above.
    #[test]
    fn the_python_built_denominator_drives_the_orphan_rule() {
        let root = std::env::var("DIGIMON_REPO_ROOT")
            .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../..").to_string());
        let path = std::path::Path::new(&root)
            .join("code/tests/tools/fixtures/card_loop/interactions/interaction_denominator.json");
        let b = InteractionBook::load(&path).expect("fixture denominator loads");
        assert_eq!(
            b.get("qa:Q2002").unwrap().card_ids,
            vec!["BT7-056".to_string(), "EX4-030".to_string()]
        );
        let scenario = |interaction: &str, covers: &str| {
            format!(
                "card: EX4-030\nclause: EX4-030#effect#2\n{covers}interaction: {interaction}\nseed: 1\n\
                 decks:\n  p0: {{ stack: [EX4-030], rest: x }}\n  p1: {{ stack: [], rest: x }}\n\
                 steps:\n  - actor: 0\n    do: {{ pass: {{}} }}\n"
            )
        };
        let lint = |text: String| validate_scenario(&text, None, InteractionDenominator::Loaded(&b));
        assert!(lint(scenario("{ id: \"qa:Q2002\", source: qa, kind: positive }", "")).is_empty());
        assert!(lint(scenario(
            "{ id: \"probe:EX4-030#effect#2:scope:neg\", source: probe, kind: negative }",
            ""
        ))
        .is_empty());
        let f = lint(scenario("{ id: \"qa:Q2003\", source: qa, kind: positive }", ""));
        assert_eq!(rules(&f), vec!["interaction-card-mismatch"], "Q2003 is BT21-029's");
        let f = lint(scenario(
            "{ id: \"probe:EX4-030#effect#2:would_replacement\", source: probe, kind: positive }",
            "",
        ));
        assert_eq!(rules(&f), vec!["orphan-interaction"]);
    }

    #[test]
    fn expect_ruling_linkage_is_checked() {
        let ok = "interaction: { id: \"qa:Q1601\", source: qa, kind: positive }\nexpect_ruling:\n  q_id: Q1601\n  assert:\n    - at: 1\n      that: { p0.memory: 3 }\n";
        assert!(lint(ok).is_empty(), "{:?}", lint(ok));
        assert_eq!(rules(&lint(&ok.replace("q_id: Q1601", "q_id: Q2"))), vec!["ruling-q-mismatch"]);
        let no_interaction = ok.replace("interaction: { id: \"qa:Q1601\", source: qa, kind: positive }\n", "");
        assert_eq!(rules(&lint(&no_interaction)), vec!["ruling-without-qa-interaction"]);
        let empty = "interaction: { id: \"qa:Q1601\", source: qa, kind: positive }\nexpect_ruling: { q_id: Q1601, assert: [] }\n";
        assert_eq!(rules(&lint(empty)), vec!["empty-ruling"]);
        let secret = ok.replace("p0.memory: 3", "p1.security.0: EX12-004");
        assert_eq!(rules(&lint(&secret)), vec!["security-contents-assert"]);
    }
}

#[cfg(test)]
mod deck_budget_tests {
    use super::*;

    fn rocks_book() -> crate::exam::deckbook::DeckBook {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        crate::exam::deckbook::DeckBook::load(
            Some(root.join("qa/dcgo-exams/EX10/rocks_pool.json").as_path()),
            root.join("data/cards.json").as_path(),
        )
        .unwrap()
    }

    fn line(p0_stack: &str, p0_rest: &str) -> String {
        format!(
            "card: EX10-025\nclause: EX10-025#effect#0\nseed: 1\ndecks:\n  p0: {{ stack: [{p0_stack}], rest: {p0_rest} }}\n  p1: {{ stack: [], rest: rocks-exam }}\nsteps:\n  - actor: 0\n    do: {{ pass: {{}} }}\n"
        )
    }

    #[test]
    fn deck_budget_flags_more_stacked_copies_than_the_deck_holds() {
        let yaml = line("EX10-025, EX10-025, EX10-025, EX10-025, EX10-025", "rocks-exam");
        let findings = validate_yaml_with_decks(&yaml, None, Some(&rocks_book()));
        let f = findings.iter().find(|f| f.rule == "deck-budget").expect("deck-budget finding");
        assert!(f.message.contains("5x EX10-025") && f.message.contains("rocks-exam"), "{}", f.message);
    }

    #[test]
    fn a_stack_the_deck_can_supply_passes_the_budget() {
        let findings = validate_yaml_with_decks(&line("EX10-025", "rocks-exam"), None, Some(&rocks_book()));
        assert!(!findings.iter().any(|f| f.rule == "deck-budget"), "{findings:?}");
    }

    #[test]
    fn an_unknown_rest_deck_is_a_deck_budget_finding() {
        let findings = validate_yaml_with_decks(&line("", "no-such-deck"), None, Some(&rocks_book()));
        let f = findings.iter().find(|f| f.rule == "deck-budget").expect("deck-budget finding");
        assert!(f.message.contains("no-such-deck"), "{}", f.message);
    }

    #[test]
    fn a_card_only_the_egg_deck_holds_cannot_be_stacked() {
        // `stack:` orders the MAIN deck; the job builder refuses an egg there.
        let book = rocks_book();
        let egg = book.resolve("rocks-exam").unwrap().eggs[0].clone();
        let findings = validate_yaml_with_decks(&line(&egg, "rocks-exam"), None, Some(&book));
        assert!(findings.iter().any(|f| f.rule == "deck-budget"), "{findings:?}");
    }

    #[test]
    fn without_a_book_there_is_no_budget_check() {
        let yaml = line("EX10-025, EX10-025, EX10-025, EX10-025, EX10-025", "rocks-exam");
        assert!(!validate_yaml_with_decks(&yaml, None, None).iter().any(|f| f.rule == "deck-budget"));
    }
}
