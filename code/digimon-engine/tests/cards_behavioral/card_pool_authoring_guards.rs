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
//! 2. Every pick that feeds an effect play verb (from the trash, hand, union
//!    zones, digivolution cards, materials, reveal buckets, security) carries
//!    a play-legality leaf after compilation, so a card the play would refuse
//!    is never offered (Asuna Shiroki (BT24-088), Yellow Scramble (LM-029),
//!    Kaguyamon (EX12-065) under Crimson Blaze (BT8-097) / Pomumon (BT9-047);
//!    the Digi-Egg Nyaromon (EX12-001) under Proximamon (EX12-077); the union
//!    picks of Gankoomon (BT13-019), Unique Emblem: Poseidia Lagoon (BT22-096),
//!    The Sistermon Sisters Training Gym (BT23-099), Tsunomon (BT24-007); a `<Decode>` under Pomumon).

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
fn pool_guard_every_effect_play_pick_checks_play_legality() {
    let mut offenders = Vec::new();
    let mut playing_cards = 0usize;
    for file in digimon_dsl::authoring_lint::pool_card_files(&cards_root()) {
        let Ok(spec) = digimon_dsl::loader::load_file(&file) else {
            continue; // a spec that does not load fails the build script, not this guard
        };
        let Ok(card) = digimon_dsl::compile::compile(&spec) else {
            continue;
        };
        if digimon_dsl::play_candidates::plays_a_pick(&card) {
            playing_cards += 1;
        }
        for binding in digimon_dsl::play_candidates::unmarked_play_picks(&card) {
            offenders.push(format!("{} (binding `{binding}`)", card.card));
        }
    }
    assert!(
        playing_cards > 200,
        "the pool has hundreds of cards that play a pick; found {playing_cards}"
    );
    assert!(
        offenders.is_empty(),
        "picks that feed an effect play verb without a play-legality leaf \
         (the pick would offer cards the play refuses):\n{}",
        offenders.join("\n")
    );
}
