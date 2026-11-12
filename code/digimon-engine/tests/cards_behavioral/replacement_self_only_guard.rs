//! Guard: "when THIS Digimon would leave / be deleted" replacements stay
//! self-only.
//!
//! A replacement clause is SELF-ONLY unless its `active_when` reads the
//! replacement subject, in which case it fires for ANY of the controller's
//! permanents leaving (`lower_replacement.rs`, `predicate_reads_replacement_subject`).
//! `replacement_subject_is_mine: true` looks like a harmless "mine" qualifier
//! but is exactly such a leaf. DCGO's `CanTriggerWhenRemoveField(hashtable, card)`
//! is `permanent.cardSources.Contains(card)` - the carrier's own permanent.
//!
//! Replaying a recorded game showed ST24-06's inherited leave-save firing for a
//! Tamer that a Burst Digivolve returned to hand; an audit of the 40 cards using
//! the leaf found these twelve authored cross-permanent against "this Digimon"
//! text. The other 30 are genuinely cross-permanent ("1 of your Digimon ...",
//! Delay options) and are deliberately not listed.

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledPredicate};

use crate::dsl_card_data::compiled;

const SELF_ONLY_CARDS: &[&str] = &[
    "BT13-075", "BT20-056", "BT22-036", "BT25-027", "BT26-044", "EX11-022", "EX5-015", "EX6-057",
    "EX7-027", "EX9-032", "ST24-06", "ST24-10",
];

fn reads_subject_is_mine(p: &CompiledPredicate) -> bool {
    p.replacement_subject_is_mine.is_some()
        || p.all_of.iter().any(reads_subject_is_mine)
        || p.any_of.iter().any(reads_subject_is_mine)
        || p.none_of.iter().any(reads_subject_is_mine)
}

/// `is_source: true` evaluated against the replacement subject pins it to the
/// carrier (`lower_replacement.rs` evaluates `active_when` on the subject), so a
/// subject-reading leaf alongside it is still self-only. That is main's form for
/// these twelve cards (7d1d1b48d "fix would-leave scope").
fn requires_subject_is_source(p: &CompiledPredicate) -> bool {
    p.is_source == Some(true) || p.all_of.iter().any(requires_subject_is_source)
}

#[test]
fn this_digimon_leave_replacements_do_not_read_the_replacement_subject() {
    let mut examined = 0usize;
    for id in SELF_ONLY_CARDS {
        let card = compiled(id);
        for clause in &card.effects {
            // `kind: replacement` compiles to a Declarative::Replacement clause.
            if let CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                active_when,
                ..
            }) = clause
            {
                examined += 1;
                if let Some(active_when) = active_when {
                    assert!(
                        !reads_subject_is_mine(active_when)
                            || requires_subject_is_source(active_when),
                        "{id}: a self-only replacement must not use replacement_subject_is_mine                          (it makes the clause fire for ANY of your permanents leaving)"
                    );
                }
            }
        }
    }
    // Non-vacuity: each of the twelve carries at least one replacement clause,
    // so a representation change that stops yielding them fails here instead of
    // silently passing.
    assert!(
        examined >= SELF_ONLY_CARDS.len(),
        "only {examined} replacement clauses examined across {} cards",
        SELF_ONLY_CARDS.len()
    );
}
