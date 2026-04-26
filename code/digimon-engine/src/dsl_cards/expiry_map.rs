//! Translate DSL expiry strings into engine `Expiry` enum values.
//!
//! Keys are snake_case to match the rest of the YAML DSL surface (verb names,
//! field names, predicate kinds). The set here is the single source of truth
//! for what the engine recognizes; the validator's `KNOWN_EXPIRY_KEYS` mirrors
//! it, and `tests/dsl/expiry_parity.rs` enforces the equality bidirectionally.

use crate::enums::Expiry;

pub fn lookup_expiry(s: &str) -> Option<Expiry> {
    Some(match s {
        "permanent" => Expiry::Permanent,
        "end_of_turn" => Expiry::EndOfTurn,
        "end_of_opponents_turn" => Expiry::EndOfOpponentsTurn,
        "end_of_attack" => Expiry::EndOfAttack,
        "end_of_battle" => Expiry::EndOfBattle,
        "until_leave_field" => Expiry::UntilLeaveField,
        _ => return None,
    })
}

/// Every key recognized by `lookup_expiry`. Used by the parity test to
/// catch drift against `digimon_dsl::validator::KNOWN_EXPIRY_KEYS`.
pub fn all_engine_expiry_keys() -> &'static [&'static str] {
    &[
        "permanent",
        "end_of_turn",
        "end_of_opponents_turn",
        "end_of_attack",
        "end_of_battle",
        "until_leave_field",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::Expiry;

    #[test]
    fn all_variants_round_trip() {
        assert_eq!(lookup_expiry("permanent"), Some(Expiry::Permanent));
        assert_eq!(lookup_expiry("end_of_turn"), Some(Expiry::EndOfTurn));
        assert_eq!(lookup_expiry("end_of_opponents_turn"), Some(Expiry::EndOfOpponentsTurn));
        assert_eq!(lookup_expiry("end_of_attack"), Some(Expiry::EndOfAttack));
        assert_eq!(lookup_expiry("end_of_battle"), Some(Expiry::EndOfBattle));
        assert_eq!(lookup_expiry("until_leave_field"), Some(Expiry::UntilLeaveField));
    }

    #[test]
    fn pascal_case_no_longer_recognized() {
        // Catches accidental regressions back to the pre-snake_case keys.
        assert_eq!(lookup_expiry("EndOfTurn"), None);
        assert_eq!(lookup_expiry("Permanent"), None);
        assert_eq!(lookup_expiry("UntilLeaveField"), None);
    }

    #[test]
    fn unknown_returns_none() {
        assert_eq!(lookup_expiry("bogus"), None);
        assert_eq!(lookup_expiry(""), None);
    }

    #[test]
    fn all_engine_keys_lookup_succeeds() {
        for &k in all_engine_expiry_keys() {
            assert!(lookup_expiry(k).is_some(), "key {k:?} listed in all_engine_expiry_keys but not matched in lookup_expiry");
        }
    }
}
