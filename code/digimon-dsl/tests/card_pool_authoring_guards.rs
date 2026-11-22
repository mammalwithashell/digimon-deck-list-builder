//! Pool-wide guards over the production card specs (`code/digimon-engine/cards/`).
//!
//! These run in the light `digimon-dsl` crate (no engine build), so a card that
//! reintroduces one of the classes below fails fast. The same two checks also
//! run in the engine's `cards_behavioral` binary (`card_pool_authoring_guards`),
//! which is the lane CI executes.
//!
//! 1. the raw-YAML authoring lints (`digimon_dsl::authoring_lint`): an
//!    unfiltered `card_count_in_zone` over the battle area, and a card's own
//!    timing mixed with a board-wide observer timing in one `when:` list;
//! 2. every `select_trash` whose binding feeds a `play_from_trash_free` /
//!    `play_from_trash` carries a play-legality leaf after compilation
//!    (`digimon_dsl::play_candidates`), so a card the play verb would refuse
//!    (effect-play lock, trash-play lock, full battle area) is never offered.

use std::path::{Path, PathBuf};

fn cards_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../digimon-engine/cards")
}

#[test]
fn no_card_spec_trips_an_authoring_lint() {
    let files = digimon_dsl::authoring_lint::pool_card_files(&cards_root());
    assert!(files.len() > 1000, "the production pool should be found (got {})", files.len());
    let findings = digimon_dsl::authoring_lint::check_pool(&cards_root());
    assert!(findings.is_empty(), "authoring lints tripped:\n{}", findings.join("\n"));
}

#[test]
fn every_play_from_trash_pick_checks_play_legality_after_compilation() {
    let mut offenders = Vec::new();
    let mut playing_cards = 0usize;
    for file in digimon_dsl::authoring_lint::pool_card_files(&cards_root()) {
        let Ok(spec) = digimon_dsl::loader::load_file(&file) else {
            continue; // a spec that does not load is the loader's finding
        };
        let Ok(card) = digimon_dsl::compile::compile(&spec) else {
            continue; // likewise for the compiler
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
