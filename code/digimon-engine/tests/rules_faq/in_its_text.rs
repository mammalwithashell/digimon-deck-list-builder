//! In its Text / In its Name — surface M. Ledger rows IT-1..IT-3.
//!
//! "For effects that refer to a 'card with [X] in its name', if a card name
//! includes [X] as part of its name, does it count even if it's not a perfect
//! match? Yes. ... an effect that reads '1 Digimon card with [Agumon] in its
//! name' applies to 'ToyAgumon', 'BushiAgumon' alike." (General Rules/FAQ.)
//!
//! IT-3 pinned via the engine's own name-match predicate
//! `Permanent::contains_card_name` (case-insensitive substring) on a real
//! placed card. IT-1/IT-2 ("in its TEXT" span + icon-exact `<Save>` vs
//! `<Material Save>`) are TBD (tasks.md §7.4).

#![allow(unused_imports)]

use digimon_engine::debug_runner::DebugRunner;

/// IT-3 — "X in its name" is a case-insensitive SUBSTRING match: AD1-004
/// WarGreymon counts for "with Greymon in its name" (and "greymon"), but not for
/// an unrelated name.
#[test]
fn it3_name_match_is_case_insensitive_substring() {
    let mut r = DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 (WarGreymon) in embedded DSL pack")
        .start();
    let p = r.turn_player();
    let h = r.place_on_field(p, "AD1-004", Some(0));

    let perm = r
        .game
        .player(p)
        .battle_area
        .get(h.index as usize)
        .expect("placed permanent present");

    assert!(
        perm.contains_card_name("Greymon", &r.game.card_data),
        "FAQ: 'WarGreymon' counts for 'with Greymon in its name' (substring)"
    );
    assert!(
        perm.contains_card_name("greymon", &r.game.card_data),
        "FAQ: name match is case-insensitive"
    );
    assert!(
        !perm.contains_card_name("Garurumon", &r.game.card_data),
        "an unrelated name does not match"
    );
}
