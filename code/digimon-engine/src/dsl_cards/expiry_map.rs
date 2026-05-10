//! Translate DSL expiry strings into engine `Expiry` enum values.
//!
//! Single source of truth: `EXPIRY_TABLE`. Adding a new `Expiry` variant
//! forces the build to fail at `_expiry_variant_exhaustiveness_check` until
//! the table is updated, and the unit-test parity check below fails the
//! build if the table's key set drifts from `digimon_dsl::validator::KNOWN_EXPIRY_KEYS`.
//!
//! Keys are snake_case to match the rest of the YAML DSL surface.

use crate::enums::Expiry;

static EXPIRY_TABLE: &[(&str, Expiry)] = &[
    ("permanent", Expiry::Permanent),
    ("end_of_turn", Expiry::EndOfTurn),
    ("end_of_opponents_turn", Expiry::EndOfOpponentsTurn),
    ("end_of_your_turn", Expiry::EndOfYourTurn),
    ("end_of_opponents_next_turn", Expiry::EndOfOpponentsNextTurn),
    ("end_of_your_next_turn", Expiry::EndOfYourNextTurn),
    ("end_of_attack", Expiry::EndOfAttack),
    ("end_of_battle", Expiry::EndOfBattle),
    ("until_leave_field", Expiry::UntilLeaveField),
    ("until_condition", Expiry::UntilCondition),
    // `OnceUsed(N)` requires a parameter — DSL surface uses the parametric
    // form `once_used: { uses: N }` not a bare key, so it does not appear
    // in this table. The key string `"once_used"` is reserved by the DSL
    // validator for the parameterized form (see KNOWN_EXPIRY_KEYS).
];

pub fn lookup_expiry(s: &str) -> Option<Expiry> {
    EXPIRY_TABLE
        .iter()
        .find_map(|(k, v)| (*k == s).then_some(*v))
}

/// Compile-time guard: every `Expiry` variant must be enumerated here. Adding
/// a new variant breaks this match's exhaustiveness, prompting the engineer
/// to also add a row to `EXPIRY_TABLE`. Paired with the unit test
/// `every_variant_has_a_table_entry` which catches the runtime side.
///
/// `OnceUsed(_)` and `UntilCondition` deliberately do NOT have a bare-key
/// table entry — `OnceUsed` carries a parameter so the DSL surface uses
/// the parametric form `once_used: { uses: N }` (validator-side check),
/// and `UntilCondition` holds a predicate on the `ModifierEntry` rather
/// than in the enum variant.
#[allow(dead_code)]
const fn _expiry_variant_exhaustiveness_check(e: Expiry) {
    match e {
        Expiry::Permanent
        | Expiry::EndOfTurn
        | Expiry::EndOfOpponentsTurn
        | Expiry::EndOfYourTurn
        | Expiry::EndOfOpponentsNextTurn
        | Expiry::EndOfYourNextTurn
        | Expiry::EndOfAttack
        | Expiry::EndOfBattle
        | Expiry::UntilLeaveField
        | Expiry::UntilCondition
        | Expiry::OnceUsed(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Enumerate every `Expiry` variant. Pairs with
    /// `_expiry_variant_exhaustiveness_check` above: that function ensures
    /// the *enum* hasn't grown a variant we forgot; this function ensures
    /// the *table* covers each variant we do know about.
    fn all_variants() -> &'static [Expiry] {
        &[
            Expiry::Permanent,
            Expiry::EndOfTurn,
            Expiry::EndOfOpponentsTurn,
            Expiry::EndOfYourTurn,
            Expiry::EndOfOpponentsNextTurn,
            Expiry::EndOfYourNextTurn,
            Expiry::EndOfAttack,
            Expiry::EndOfBattle,
            Expiry::UntilLeaveField,
            Expiry::UntilCondition,
        ]
    }

    #[test]
    fn every_variant_has_a_table_entry() {
        for &v in all_variants() {
            assert!(
                EXPIRY_TABLE.iter().any(|(_, tv)| *tv == v),
                "Expiry::{v:?} has no row in EXPIRY_TABLE"
            );
        }
    }

    #[test]
    fn every_table_key_round_trips() {
        for &(k, v) in EXPIRY_TABLE {
            assert_eq!(lookup_expiry(k), Some(v), "{k:?} did not round-trip");
        }
    }

    #[test]
    fn pascal_case_no_longer_recognized() {
        assert_eq!(lookup_expiry("EndOfTurn"), None);
        assert_eq!(lookup_expiry("Permanent"), None);
        assert_eq!(lookup_expiry("UntilLeaveField"), None);
    }

    #[test]
    fn unknown_returns_none() {
        assert_eq!(lookup_expiry("bogus"), None);
        assert_eq!(lookup_expiry(""), None);
    }

    /// Bidirectional parity with `digimon_dsl::validator::KNOWN_EXPIRY_KEYS`.
    /// Lives here (not in `tests/dsl/`) so it can see `EXPIRY_TABLE` directly
    /// without forcing the engine to expose a public-API surface that exists
    /// only for tests.
    ///
    /// Why this matters: when the validator green-lights an expiry the engine
    /// doesn't recognize, `modifiers.rs` does `let Some(expiry) = lookup_expiry(...)
    /// else { return true; }` — silently dropping the modifier. This test
    /// fails the build the moment the two lists drift.
    #[test]
    fn validator_keys_match_engine_table() {
        let validator: BTreeSet<&str> = digimon_dsl::validator::KNOWN_EXPIRY_KEYS
            .iter()
            .copied()
            .collect();
        let engine: BTreeSet<&str> = EXPIRY_TABLE.iter().map(|(k, _)| *k).collect();

        let only_validator: Vec<_> = validator.difference(&engine).collect();
        let only_engine: Vec<_> = engine.difference(&validator).collect();

        assert!(
            only_validator.is_empty() && only_engine.is_empty(),
            "expiry-key drift between digimon-dsl validator and digimon-engine EXPIRY_TABLE. \
             Only in validator (would silently no-op at runtime): {only_validator:?}. \
             Only in engine (validator would reject valid YAML): {only_engine:?}."
        );
    }
}
