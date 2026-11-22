//! Pool-wide authoring guards over every production card spec (`cards/`).
//!
//! Each guard is a class that reached logged games before it was caught (bug
//! scan over 1,224 games, 2026-11). They live in this binary because it is the
//! one CI runs; the same checks run engine-free in
//! `code/digimon-dsl/tests/card_pool_authoring_guards.rs` and (the lints) in
//! `dsl-lint`.
//!
//! 1. `authoring_lint` -- no spec has an unfiltered `card_count_in_zone` over
//!    the battle area (it counts Option cards placed there and Tamers:
//!    ShineGreymon (AD1-016), Medusamon (BT24-017)), and no `when:` list mixes
//!    a card's own timing with the board-wide observer timing of the same
//!    event (Snatchmon (BT21-058): `[on_play, on_digivolve]`).
//! 2. Every `select_trash` pick that feeds a play-from-trash verb carries a
//!    play-legality leaf after compilation, so a card whose play by effect is
//!    prohibited is never offered (Asuna Shiroki (BT24-088), Yellow Scramble
//!    (LM-029), Kaguyamon (EX12-065) under Crimson Blaze (BT8-097) / Pomumon
//!    (BT9-047)).

use std::path::{Path, PathBuf};

fn cards_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("cards")
}

#[test]
fn pool_guard_no_card_spec_trips_an_authoring_lint() {
    let files = digimon_dsl::authoring_lint::pool_card_files(&cards_root());
    assert!(
        files.len() > 1000,
        "the production pool should be found (got {})",
        files.len()
    );
    let findings = digimon_dsl::authoring_lint::check_pool(&cards_root());
    assert!(
        findings.is_empty(),
        "authoring lints tripped:\n{}",
        findings.join("\n")
    );
}

#[test]
fn pool_guard_every_play_from_trash_pick_checks_play_legality() {
    let mut offenders = Vec::new();
    let mut playing_cards = 0usize;
    for file in digimon_dsl::authoring_lint::pool_card_files(&cards_root()) {
        let Ok(spec) = digimon_dsl::loader::load_file(&file) else {
            continue; // a spec that does not load fails the build script, not this guard
        };
        let Ok(card) = digimon_dsl::compile::compile(&spec) else {
            continue;
        };
        if digimon_dsl::play_candidates::plays_a_pick_from_trash(&card) {
            playing_cards += 1;
        }
        for binding in digimon_dsl::play_candidates::unmarked_play_from_trash_picks(&card) {
            offenders.push(format!("{} (binding `{binding}`)", card.card));
        }
    }
    assert!(
        playing_cards > 40,
        "the pool has dozens of cards that play a pick from the trash; found {playing_cards}"
    );
    assert!(
        offenders.is_empty(),
        "select_trash picks that feed a play-from-trash verb without a play-legality leaf \
         (the pick would offer cards the play refuses):\n{}",
        offenders.join("\n")
    );
}
