//! `Game::has_keyword_type` — keyword lookup by TYPE (Comprehensive Rules 4.3
//! §16-2: "A keyword effect is considered to be the same type even if it has a
//! different numerical value or specified card included in the icon enclosed
//! in brackets").
//!
//! The rules_conformance witness covers the DSL `has_keyword` predicate end to
//! end; these tests pin the query itself across origins (printed face,
//! modifier grant) and its negative cases.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{Expiry, Keyword};
use digimon_engine::permanent::PermanentHandle;

use super::helpers::{digimon_with_keywords, plain_digimon, plain_tamer};

/// A Digimon with a printed `<Fragment (1)>` and a granted `<Fragment (2)>`
/// answers "has a <Fragment>" for every value, and not for a different keyword.
#[test]
fn has_keyword_type_matches_every_value_of_the_keyword() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_with_keywords(
            "FRAG-DIGI",
            3,
            3000,
            vec![Keyword::Fragment(1)],
        ))
        .add_card(plain_digimon("PLAIN"))
        .start();
    let carrier = r.place_on_field(0, "FRAG-DIGI", None);
    let plain = r.place_on_field(0, "PLAIN", None);

    // Printed face <Fragment (1)>: the type is found whatever value is asked
    // for, while exact-value `has_keyword` still distinguishes the values.
    assert!(r.game.has_keyword_type(carrier, &Keyword::Fragment(1)));
    assert!(r.game.has_keyword_type(carrier, &Keyword::Fragment(2)));
    assert!(r.game.has_keyword(carrier, Keyword::Fragment(1)));
    assert!(!r.game.has_keyword(carrier, Keyword::Fragment(2)));

    // A granted <Fragment (2)> on a Digimon that has no printed Fragment is
    // found by type too (the modifier-grant origin).
    r.game
        .modifiers
        .grant_keyword(plain, Keyword::Fragment(2), Expiry::EndOfTurn, 0);
    assert!(r.game.has_keyword_type(plain, &Keyword::Fragment(1)));
    assert!(r.game.has_keyword_type(plain, &Keyword::Fragment(2)));

    // Both origins on one Digimon.
    r.game
        .modifiers
        .grant_keyword(carrier, Keyword::Fragment(2), Expiry::EndOfTurn, 0);
    assert!(r.game.has_keyword_type(carrier, &Keyword::Fragment(3)));

    // A different keyword type is not matched.
    assert!(!r.game.has_keyword_type(carrier, &Keyword::Blocker));
    assert!(!r.game.has_keyword_type(carrier, &Keyword::Decoy(0)));
    assert!(!r.game.has_keyword_type(plain, &Keyword::Rush));
}

/// A Tamer and a token that lack a keyword do not match it by type; an
/// out-of-range handle is false rather than a panic.
#[test]
fn has_keyword_type_is_false_for_tamer_token_and_missing_handle() {
    let mut r = DebugRunner::builder()
        .add_card(plain_tamer("TAMER"))
        .add_card(plain_digimon("TOKEN-HOST"))
        .start();
    let tamer = r.place_on_field(0, "TAMER", None);
    let token = r.place_on_field(0, "TOKEN-HOST", None);
    r.game.players[0].battle_area[token.index as usize]
        .top_card_mut()
        .is_token = true;

    for kind in [
        Keyword::Fragment(1),
        Keyword::Blocker,
        Keyword::Decoy(0),
        Keyword::MaterialSave(1),
    ] {
        assert!(!r.game.has_keyword_type(tamer, &kind), "Tamer: {kind:?}");
        assert!(!r.game.has_keyword_type(token, &kind), "token: {kind:?}");
    }

    let missing = PermanentHandle {
        player: 0,
        index: 40,
    };
    assert!(!r.game.has_keyword_type(missing, &Keyword::Fragment(1)));
}
