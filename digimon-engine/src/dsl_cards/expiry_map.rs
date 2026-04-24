//! Translate DSL expiry strings into engine `Expiry` enum values.

use crate::enums::Expiry;

pub fn lookup_expiry(s: &str) -> Option<Expiry> {
    Some(match s {
        "Permanent" => Expiry::Permanent,
        "EndOfTurn" => Expiry::EndOfTurn,
        "EndOfOpponentsTurn" => Expiry::EndOfOpponentsTurn,
        "EndOfAttack" => Expiry::EndOfAttack,
        "EndOfBattle" => Expiry::EndOfBattle,
        "UntilLeaveField" => Expiry::UntilLeaveField,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::Expiry;

    #[test]
    fn all_variants_round_trip() {
        assert_eq!(lookup_expiry("Permanent"), Some(Expiry::Permanent));
        assert_eq!(lookup_expiry("EndOfTurn"), Some(Expiry::EndOfTurn));
        assert_eq!(lookup_expiry("EndOfOpponentsTurn"), Some(Expiry::EndOfOpponentsTurn));
        assert_eq!(lookup_expiry("EndOfAttack"), Some(Expiry::EndOfAttack));
        assert_eq!(lookup_expiry("EndOfBattle"), Some(Expiry::EndOfBattle));
        assert_eq!(lookup_expiry("UntilLeaveField"), Some(Expiry::UntilLeaveField));
    }

    #[test]
    fn unknown_returns_none() {
        assert_eq!(lookup_expiry("bogus"), None);
        assert_eq!(lookup_expiry(""), None);
    }
}
