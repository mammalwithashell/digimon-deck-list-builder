//! Authoring lints over the RAW card YAML: shapes that parse, compile and run
//! but that no printed card means. Each rule is a class that reached logged
//! games before it was caught (bug scan over 1,224 games), so the rule is an
//! error, not a warning.
//!
//! The walk is over the untyped YAML value on purpose: both shapes can sit at
//! any depth (a formula inside a nested `if`, a granted effect's `when:`), and
//! a typed walk would have to be kept in step with every new step variant.
//!
//! Rules
//!
//! * `unfiltered-battle-area-count` -- `card_count_in_zone` over `battle_area`
//!   with no `filter`. An unfiltered count is `battle_area.len()`: every
//!   permanent, including an Option card an effect placed in the battle area
//!   (CR 4.3 4-28-1, 17-1-3-2-2: it stays there as an Option card) and Tamers
//!   (4-4-1). No printed card counts "permanents"; they count "Digimon",
//!   "Digimon and Tamers", "Tamers", ... so the filter must be spelled out.
//!   ShineGreymon (AD1-016) "-3000 DP for each of your Digimon and Tamers" and
//!   Medusamon (BT24-017) "+2000 DP for each of your opponent's Digimon" both
//!   counted Digital Accident Tactics Squad (P-235) / a Tamer this way.
//!
//! * `self-and-observer-timing` -- one `when:` list mixing a card's OWN timing
//!   (`on_play`, `when_digivolving`, `when_attacking`, `on_deletion`) with the
//!   board-wide OBSERVER timing of the same event family (`on_digivolve`,
//!   `on_enter_field_anyone`, `on_any_digimon_played`, `on_ally_played`,
//!   `on_ally_attack`, `on_opponent_attack`, `on_any_deletion`). The printed
//!   tag pair "[On Play] [When Digivolving]" is `[on_play, when_digivolving]`;
//!   `on_digivolve` fires whenever ANY Digimon on either side digivolves.
//!   Snatchmon (BT21-058) was authored `[on_play, on_digivolve]` and re-ran its
//!   whole effect on every digivolution in the game. A printed observer ("When
//!   any of your Digimon are played or digivolve") uses observer timings only;
//!   "When Digimon attack" is `[on_ally_attack, on_opponent_attack]` (the
//!   observer's own attack is inside `on_ally_attack`), never `when_attacking`.
//!
//! * `refund-after-explicit-accept` -- a `refund_opt` inside a clause whose
//!   activation passes an explicit accept: an `optional: true` replacement
//!   (the "May accept replacement" prompt), an `outer_prompt: true` clause, or
//!   a [Main] / [Counter] clause (activation-type, CR 4.3 15-8-4 /
//!   15-16-14-1: activated by declaring it). Whether such a refund is right
//!   depends on which path reaches it, and the YAML shape cannot tell. The
//!   [Once Per Turn] table (qa/rules/mechanics-gaps-2026-10.md W14) decides;
//!   the accept prompt is DCGO's skippable Yes/No, not printed, and for a
//!   printed "you may X" the choice to activate is the choice to do X:
//!   - a. declined at the accept prompt: kept (no refund step needed);
//!   - b. accepted, X was possible, the player chose not to do X: KEPT
//!     (official Q1818, Q4086, Q6946) -- the refund is right;
//!   - W13. "you may A. Then, you may B." with both halves declined: KEPT
//!     (owner decision W13, 2026-10-21) -- the refund is right;
//!   - c. the player chose X, whatever then happened to it: SPENT (CR 4.3
//!     15-14-1-4, official Q5426) -- a refund is wrong;
//!   - d. nothing was possible but the effect was offered: SPENT (CR 4.3
//!     15-14-1-4); where DCGO does not offer the effect in that state the
//!     clause mirrors that with a `condition:` instead -- a refund is wrong;
//!   - e. mandatory content ran, or an opponent's choice was processed:
//!     SPENT (official Q4739, Q5056, Q5171, Q5330) -- a refund is wrong.
//!   So every such clause is listed in [`REFUND_AFTER_ACCEPT_REVIEWED`] with
//!   the row its refund path is reached on (b or W13 only; [`RefundRow`] has
//!   no variant for c, d or e), why it is reached on no other, and the
//!   behavioural tests that prove it. An unlisted clause is an error, and the
//!   pool guards fail on an entry whose clause no longer refunds behind an
//!   accept ([`stale_refund_reviews`]) or whose named test is gone. The table
//!   is a review record, not a check of the row: the lint cannot see which
//!   path reaches a `refund_opt`, so the named tests carry the proof.
//!   Erlangmon (EX12-034) once refunded on its nothing-playable path (row d)
//!   and was used twice in one turn in logged games.

use serde_yml::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoringFinding {
    pub rule: &'static str,
    pub path: String,
    pub message: String,
}

const SELF_TIMINGS: &[&str] = &["on_play", "when_digivolving", "when_attacking", "on_deletion"];
/// Activation-type timings: the player activates the effect by declaring it.
const DECLARED_TIMINGS: &[&str] = &["main", "main_on_field", "main_from_hand", "main_from_trash", "counter"];

/// Why a clause mapping's activation is an explicit choice, if it is.
fn explicit_accept(clause: &serde_yml::Mapping) -> Option<String> {
    let is_true = |k: &str| clause.get(k).and_then(Value::as_bool) == Some(true);
    if clause.get("kind").and_then(Value::as_str) == Some("replacement") && is_true("optional") {
        return Some("an optional replacement (accepted at its \"May accept replacement\" prompt)".into());
    }
    if is_true("outer_prompt") {
        return Some("an `outer_prompt: true` clause (accepted at its outer prompt)".into());
    }
    let timings: Vec<&str> = match clause.get("when") {
        Some(Value::String(s)) => vec![s.as_str()],
        Some(Value::Sequence(items)) => items.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    timings
        .into_iter()
        .find(|t| DECLARED_TIMINGS.contains(t))
        .map(|t| format!("a `{t}` clause (activation-type: activated by declaring it)"))
}

fn contains_key(value: &Value, wanted: &str) -> bool {
    match value {
        Value::Mapping(map) => map
            .iter()
            .any(|(k, v)| k.as_str() == Some(wanted) || contains_key(v, wanted)),
        Value::Sequence(items) => items.iter().any(|v| contains_key(v, wanted)),
        _ => false,
    }
}
const OBSERVER_TIMINGS: &[&str] = &[
    "on_digivolve",
    "on_enter_field_anyone",
    "on_any_digimon_played",
    "on_ally_played",
    "on_ally_attack",
    "on_opponent_attack",
    "on_any_deletion",
];

/// The rows of the [Once Per Turn] table (qa/rules/mechanics-gaps-2026-10.md
/// W14) on which a refund behind an explicit accept is right. Rows c, d and e
/// spend the use, so they have no variant: a clause that refunds on them
/// cannot be listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefundRow {
    /// b: accepted, the printed "you may X" was possible, and the player chose
    /// not to do X (official Q1818, Q4086, Q6946).
    DeclinedWhilePossible,
    /// W13: "you may A. Then, you may B." with both halves declined
    /// (owner decision 2026-10-21).
    BothHalvesDeclined,
}

/// One reviewed `refund_opt` behind an explicit accept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewedRefund {
    pub card: &'static str,
    /// The clause's path in the spec, as the lint reports it without the
    /// trailing `.process` (`effects[5]`).
    pub clause: &'static str,
    pub row: RefundRow,
    /// Why the refund path is reached on `row` and on no spending row.
    pub reason: &'static str,
    /// The behavioural tests that carry the proof, each as
    /// `<file under code/digimon-engine/tests>::<test fn>`: the listed row
    /// keeps the use, choosing X spends it (row c), and the nothing-possible
    /// state spends it or is not offered (row d). Where one half is always
    /// possible, so that no row-d state exists, `reason` says which.
    pub tests: &'static [&'static str],
}

/// Every production clause with a `refund_opt` behind an explicit accept,
/// reviewed against the [Once Per Turn] table.
///
/// This is a review record, not a proof. The lint sees that a clause refunds
/// behind an accept; it cannot see which path reaches the refund, so a clause
/// that also refunds on a spending row (an unconditional `else: refund_opt`
/// that runs when nothing was possible, row d) could still be listed as row
/// b. What proves an entry is its `tests`: behavioural tests for the kept
/// row, for row c and for the row-d state. The pool guards fail on a new
/// unlisted clause (the lint), on a stale entry ([`stale_refund_reviews`])
/// and, in `cards_behavioral`, on a named test that no longer exists.
pub const REFUND_AFTER_ACCEPT_REVIEWED: &[ReviewedRefund] = &[
    ReviewedRefund {
        card: "AD1-024",
        clause: "effects[3]",
        row: RefundRow::BothHalvesDeclined,
        reason: concat!(
            "Imperialdramon: Fighter Mode prints \"you may suspend 1 of your opponent's Digimon and ",
            "unsuspend this Digimon. Then, if played or digivolved by effects, you may return ...\": ",
            "W13. Its outer_prompt is DCGO's unprinted skippable Yes/No (AD1_024.cs SetIsSkippable). ",
            "The refund needs every pick absent, so no half executed."
        ),
        tests: &[
            "cards_behavioral/ad1/ad1_024.rs::ad1_024_all_turns_accept_then_decline_everything_keeps_opt",
            "cards_behavioral/ad1/ad1_024.rs::ad1_024_all_turns_is_once_per_turn",
        ],
    },
];

/// Whether `finding` is the reviewed refund `entry` of `card`.
fn reviewed_by(card: &str, finding: &AuthoringFinding, entry: &ReviewedRefund) -> bool {
    finding.rule == "refund-after-explicit-accept"
        && entry.card == card
        && finding.path.strip_suffix(".process").unwrap_or(&finding.path) == entry.clause
}

fn card_id(value: &Value) -> Option<&str> {
    value.as_mapping()?.get("card")?.as_str()
}

/// Lint one card's raw YAML text, ignoring [`REFUND_AFTER_ACCEPT_REVIEWED`].
pub fn check_authoring_yaml_unexempted(raw: &str) -> Vec<AuthoringFinding> {
    let mut out = Vec::new();
    if let Ok(value) = serde_yml::from_str::<Value>(raw) {
        walk(&value, "", &mut out);
    }
    out
}

/// Lint one card's raw YAML text against a reviewed-refund table.
pub fn check_authoring_yaml_with(raw: &str, reviewed: &[ReviewedRefund]) -> Vec<AuthoringFinding> {
    let mut out = check_authoring_yaml_unexempted(raw);
    let card = serde_yml::from_str::<Value>(raw)
        .ok()
        .and_then(|v| card_id(&v).map(str::to_string));
    if let Some(card) = card {
        out.retain(|f| !reviewed.iter().any(|entry| reviewed_by(&card, f, entry)));
    }
    out
}

/// Lint one card's raw YAML text. A file that does not parse yields no
/// findings here (the loader reports the parse error).
pub fn check_authoring_yaml(raw: &str) -> Vec<AuthoringFinding> {
    check_authoring_yaml_with(raw, REFUND_AFTER_ACCEPT_REVIEWED)
}

/// Whether the spec text `raw` still has the refund `entry` reviews.
pub fn refund_review_is_live(raw: &str, entry: &ReviewedRefund) -> bool {
    check_authoring_yaml_unexempted(raw)
        .iter()
        .any(|f| reviewed_by(entry.card, f, entry))
}

/// Entries of `reviewed` whose card spec under `cards_root` no longer has a
/// `refund_opt` behind an explicit accept at the listed clause; one line each.
pub fn stale_refund_reviews_with(cards_root: &std::path::Path, reviewed: &[ReviewedRefund]) -> Vec<String> {
    let files = pool_card_files(cards_root);
    reviewed
        .iter()
        .filter(|entry| {
            !files
                .iter()
                .filter(|f| f.file_stem().and_then(|s| s.to_str()) == Some(entry.card))
                .filter_map(|f| std::fs::read_to_string(f).ok())
                .any(|raw| refund_review_is_live(&raw, entry))
        })
        .map(|entry| format!("{} {} ({:?})", entry.card, entry.clause, entry.row))
        .collect()
}

/// [`stale_refund_reviews_with`] over [`REFUND_AFTER_ACCEPT_REVIEWED`].
pub fn stale_refund_reviews(cards_root: &std::path::Path) -> Vec<String> {
    stale_refund_reviews_with(cards_root, REFUND_AFTER_ACCEPT_REVIEWED)
}

/// Every production card spec under `cards_root` (`code/digimon-engine/cards`), sorted. `_examples/` holds DSL
/// documentation samples and `test/` synthetic engine fixtures; neither is a printed card. For the pool guards.
pub fn pool_card_files(cards_root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let Ok(sets) = std::fs::read_dir(cards_root) else {
        return files;
    };
    for set in sets.flatten() {
        let set = set.path();
        let name = set.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        if !set.is_dir() || name.starts_with('_') || name == "test" {
            continue;
        }
        let Ok(cards) = std::fs::read_dir(&set) else {
            continue;
        };
        for card in cards.flatten() {
            let card = card.path();
            if card.extension().and_then(|e| e.to_str()) == Some("yaml") {
                files.push(card);
            }
        }
    }
    files.sort();
    files
}

/// Run [`check_authoring_yaml`] over every production card spec; one line per finding.
pub fn check_pool(cards_root: &std::path::Path) -> Vec<String> {
    let mut findings = Vec::new();
    for file in pool_card_files(cards_root) {
        let Ok(raw) = std::fs::read_to_string(&file) else {
            continue;
        };
        for finding in check_authoring_yaml(&raw) {
            findings.push(format!(
                "{} [{}] {}: {}",
                file.file_name().and_then(|n| n.to_str()).unwrap_or_default(),
                finding.rule,
                finding.path,
                finding.message
            ));
        }
    }
    findings
}

fn zone_includes_battle_area(zone: Option<&Value>) -> bool {
    match zone {
        Some(Value::String(s)) => s == "battle_area",
        Some(Value::Sequence(items)) => items
            .iter()
            .any(|item| matches!(item, Value::String(s) if s == "battle_area")),
        _ => false,
    }
}

fn walk(value: &Value, path: &str, out: &mut Vec<AuthoringFinding>) {
    match value {
        Value::Mapping(map) => {
            if let (Some(process), Some(why)) = (map.get("process"), explicit_accept(map)) {
                if contains_key(process, "refund_opt") {
                    out.push(AuthoringFinding {
                        rule: "refund-after-explicit-accept",
                        path: if path.is_empty() { "process".into() } else { format!("{path}.process") },
                        message: format!(
                            "unreviewed `refund_opt` in {why}. Check which path reaches it against the [Once Per Turn] table (qa/rules/mechanics-gaps-2026-10.md W14): a refund is right only when the player chose not to do a printed \"you may X\" that was possible (row b: Q1818, Q4086, Q6946) or declined both halves of \"you may A. Then, you may B.\" (W13); it is wrong once the player chose X (row c: CR 4.3 15-14-1-4, Q5426), when nothing was possible but the effect was offered (row d), or after mandatory content or an opponent's choice (row e: Q4739, Q5056). List the clause in REFUND_AFTER_ACCEPT_REVIEWED with its row, reason and tests, or gate or drop the refund"
                        ),
                    });
                }
            }
            for (key, child) in map {
                let Some(key) = key.as_str() else {
                    continue;
                };
                let child_path = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };
                if key == "card_count_in_zone" {
                    if let Value::Mapping(spec) = child {
                        // Absent, null, or an empty mapping (`filter: {}`): a
                        // present key that constrains nothing still counts
                        // every permanent.
                        let filter_missing = spec.get("filter").is_none_or(|filter| match filter {
                            Value::Null => true,
                            Value::Mapping(map) => map.is_empty(),
                            _ => false,
                        });
                        if zone_includes_battle_area(spec.get("zone")) && filter_missing {
                            out.push(AuthoringFinding {
                                rule: "unfiltered-battle-area-count",
                                path: child_path.clone(),
                                message: "card_count_in_zone over battle_area has no filter: it counts EVERY permanent, Option cards placed in the battle area and Tamers included (CR 4.3 4-28-1, 4-4-1). Spell out what the card counts, e.g. `filter: { kind: digimon }` or `filter: { any_of: [{ kind: digimon }, { kind: tamer }] }`".into(),
                            });
                        }
                    }
                }
                if key == "when" {
                    if let Value::Sequence(items) = child {
                        let timings: Vec<&str> = items.iter().filter_map(Value::as_str).collect();
                        let own = timings.iter().copied().find(|t| SELF_TIMINGS.contains(t));
                        let observer = timings.iter().copied().find(|t| OBSERVER_TIMINGS.contains(t));
                        if let (Some(own), Some(observer)) = (own, observer) {
                            out.push(AuthoringFinding {
                                rule: "self-and-observer-timing",
                                path: child_path.clone(),
                                message: format!(
                                    "`when` mixes the card's own timing `{own}` with the board-wide observer timing `{observer}`. A printed \"[On Play] [When Digivolving]\" is `[on_play, when_digivolving]` (`on_digivolve` fires on EVERY Digimon's digivolution); a printed observer (\"When any ... are played or digivolve\", \"When Digimon attack\") uses observer timings only"
                                ),
                            });
                        }
                    }
                }
                walk(child, &child_path, out);
            }
        }
        Value::Sequence(items) => {
            for (index, child) in items.iter().enumerate() {
                walk(child, &format!("{path}[{index}]"), out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(raw: &str) -> Vec<&'static str> {
        check_authoring_yaml(raw).into_iter().map(|f| f.rule).collect()
    }

    #[test]
    fn unfiltered_battle_area_count_is_rejected() {
        let raw = "effects:\n  - when: on_play\n    process:\n      - add_dp_modifier:\n          value:\n            formula:\n              base: 0\n              per:\n                card_count_in_zone:\n                  of: you\n                  zone: battle_area\n              delta: -3000\n";
        let findings = check_authoring_yaml(raw);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, "unfiltered-battle-area-count");
        assert!(findings[0].path.ends_with("per.card_count_in_zone"));
    }

    #[test]
    fn filtered_battle_area_count_and_other_zones_pass() {
        let filtered = "effects:\n  - when: on_play\n    process:\n      - x:\n          per:\n            card_count_in_zone: { of: opponent, zone: battle_area, filter: { kind: digimon } }\n";
        assert!(rules(filtered).is_empty());
        let trash = "effects:\n  - when: on_play\n    process:\n      - x:\n          per:\n            card_count_in_zone: { of: you, zone: trash }\n";
        assert!(rules(trash).is_empty());
    }

    /// `filter: {}` is a present key with no constraint: the count is still
    /// every permanent, so it is the same finding as a missing filter.
    #[test]
    fn empty_filter_mapping_is_an_unfiltered_battle_area_count() {
        for filter in ["{}", "{ }", "null", "~"] {
            let raw = format!(
                "effects:\n  - when: on_play\n    process:\n      - x:\n          per:\n            card_count_in_zone: {{ of: you, zone: battle_area, filter: {filter} }}\n"
            );
            assert_eq!(rules(&raw), vec!["unfiltered-battle-area-count"], "filter: {filter}");
        }
        let list_zone = "effects:\n  - when: on_play\n    process:\n      - x:\n          per:\n            card_count_in_zone: { of: you, zone: [battle_area], filter: {} }\n";
        assert_eq!(rules(list_zone), vec!["unfiltered-battle-area-count"]);
    }

    #[test]
    fn on_play_with_board_wide_on_digivolve_is_rejected() {
        // Snatchmon (BT21-058): printed "[On Play] [When Digivolving]".
        let raw = "effects:\n  - when: [on_play, on_digivolve]\n    process: []\n";
        assert_eq!(rules(raw), vec!["self-and-observer-timing"]);
    }

    #[test]
    fn when_attacking_with_opponent_attack_observer_is_rejected() {
        let raw = "effects:\n  - when: [when_attacking, on_opponent_attack]\n    process: []\n";
        assert_eq!(rules(raw), vec!["self-and-observer-timing"]);
    }

    #[test]
    fn self_only_and_observer_only_lists_pass() {
        for when in [
            "[on_play, when_digivolving]",
            "[on_play, when_digivolving, when_attacking]",
            "[on_enter_field_anyone, on_digivolve]",
            "[on_ally_played, on_digivolve]",
            "[on_ally_attack, on_opponent_attack]",
            "on_digivolve",
        ] {
            let raw = format!("effects:\n  - when: {when}\n    process: []\n");
            assert!(rules(&raw).is_empty(), "{when}");
        }
    }

    const REFUND_TAIL: &str = "    process:\n      - select_hand: { of: you, bind_as: pick, optional: true, filter: {} }\n      - if:\n          condition: { binding_absent: pick }\n          then:\n            - refund_opt: {}\n";

    #[test]
    fn refund_after_an_explicit_accept_is_rejected() {
        for head in [
            // Erlangmon (EX12-034) / Slayerdramon (EX13-024) shape.
            "  - kind: replacement\n    trigger: when_would_leave_battle_area\n    optional: true\n    once_per_turn: true\n",
            "  - when: on_add_to_deck\n    once_per_turn: true\n    optional: true\n    outer_prompt: true\n",
            "  - when: main_on_field\n    once_per_turn: true\n    optional: true\n",
            "  - when: counter\n    once_per_turn: true\n",
            "  - when: [when_attacking, counter]\n    once_per_turn: true\n",
        ] {
            let raw = format!("effects:\n{head}{REFUND_TAIL}");
            assert_eq!(rules(&raw), vec!["refund-after-explicit-accept"], "{head}");
        }
    }

    #[test]
    fn refund_without_an_explicit_accept_passes() {
        for head in [
            // Folded "may" (Q1818) and the W13 then-may shape: no outer accept.
            "  - when: when_attacking\n    once_per_turn: true\n",
            "  - when: on_play\n    once_per_turn: true\n    optional: true\n",
            // A mandatory (non-optional) replacement has no accept prompt.
            "  - kind: replacement\n    trigger: when_would_leave_battle_area\n    once_per_turn: true\n",
        ] {
            let raw = format!("effects:\n{head}{REFUND_TAIL}");
            assert!(rules(&raw).is_empty(), "{head}");
        }
        // An explicit-accept clause without a refund is fine.
        let raw = "effects:\n  - when: main_on_field\n    once_per_turn: true\n    process:\n      - draw: { of: you, count: 1 }\n";
        assert!(rules(raw).is_empty());
    }

    const TEST_REVIEW: &[ReviewedRefund] = &[ReviewedRefund {
        card: "TEST-1",
        clause: "effects[0]",
        row: RefundRow::DeclinedWhilePossible,
        reason: "test",
        tests: &[],
    }];

    fn rules_with(raw: &str) -> Vec<&'static str> {
        check_authoring_yaml_with(raw, TEST_REVIEW).into_iter().map(|f| f.rule).collect()
    }

    /// A reviewed entry clears exactly its card's clause: the same body on
    /// another card, or at another clause of the same card, is still flagged.
    #[test]
    fn reviewed_refund_applies_to_its_clause_only() {
        let clause = format!(
            "  - when: on_add_to_deck\n    once_per_turn: true\n    optional: true\n    outer_prompt: true\n{REFUND_TAIL}"
        );
        let reviewed = format!("card: TEST-1\neffects:\n{clause}");
        assert!(rules_with(&reviewed).is_empty());
        assert_eq!(
            check_authoring_yaml_unexempted(&reviewed)
                .into_iter()
                .map(|f| f.rule)
                .collect::<Vec<_>>(),
            vec!["refund-after-explicit-accept"]
        );
        let other_card = format!("card: TEST-2\neffects:\n{clause}");
        assert_eq!(rules_with(&other_card), vec!["refund-after-explicit-accept"]);
        let other_clause =
            format!("card: TEST-1\neffects:\n  - when: on_play\n    process: []\n{clause}");
        assert_eq!(rules_with(&other_clause), vec!["refund-after-explicit-accept"]);
    }

    /// Liveness: an entry is live while its clause still refunds behind an
    /// accept, and stale once the refund (or the accept) is gone or the
    /// clause moved.
    #[test]
    fn reviewed_refund_liveness() {
        let entry = &TEST_REVIEW[0];
        let live = format!("card: TEST-1\neffects:\n  - when: counter\n    once_per_turn: true\n{REFUND_TAIL}");
        assert!(refund_review_is_live(&live, entry));
        let no_refund = "card: TEST-1\neffects:\n  - when: counter\n    once_per_turn: true\n    process:\n      - draw: { of: you, count: 1 }\n";
        assert!(!refund_review_is_live(no_refund, entry));
        let no_accept = format!("card: TEST-1\neffects:\n  - when: on_play\n    once_per_turn: true\n{REFUND_TAIL}");
        assert!(!refund_review_is_live(&no_accept, entry));
        let moved = format!(
            "card: TEST-1\neffects:\n  - when: on_play\n    process: []\n  - when: counter\n    once_per_turn: true\n{REFUND_TAIL}"
        );
        assert!(!refund_review_is_live(&moved, entry), "the clause moved: re-review it");
    }

    /// The production table: one entry per clause, each with a reason and
    /// the tests that prove it.
    #[test]
    fn reviewed_refund_table_is_well_formed() {
        for (i, a) in REFUND_AFTER_ACCEPT_REVIEWED.iter().enumerate() {
            assert!(!a.reason.trim().is_empty(), "{} {}", a.card, a.clause);
            assert!(a.clause.starts_with("effects["), "{} {}", a.card, a.clause);
            assert!(!a.tests.is_empty(), "{} {}: name the tests that prove the row", a.card, a.clause);
            for t in a.tests {
                let (file, name) = t.split_once("::").unwrap_or(("", ""));
                assert!(
                    file.ends_with(".rs") && !name.is_empty(),
                    "{} {}: `{t}` is not <file>::<test fn>",
                    a.card,
                    a.clause
                );
            }
            for b in &REFUND_AFTER_ACCEPT_REVIEWED[i + 1..] {
                assert!(
                    (a.card, a.clause) != (b.card, b.clause),
                    "duplicate entry {} {}",
                    a.card,
                    a.clause
                );
            }
        }
    }

    #[test]
    fn nested_granted_effect_is_walked() {
        let raw = "effects:\n  - when: on_play\n    process:\n      - grant_triggered_effect:\n          effect:\n            when: [on_deletion, on_any_deletion]\n            process: []\n";
        assert_eq!(rules(raw), vec!["self-and-observer-timing"]);
    }
}
