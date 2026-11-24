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
//! 2. every pick whose binding an effect play verb consumes (trash, hand,
//!    union zones, digivolution cards, materials, reveal buckets, security)
//!    carries a play-legality leaf after compilation
//!    (`digimon_dsl::play_candidates`), so a card the play verb would refuse
//!    (effect-play lock, trash-play lock, full battle area, no play cost) is
//!    never offered;
//! 3. a printed "until your turn ends" / "until your opponent's turn ends" is
//!    authored with the matching player-relative expiry
//!    (`digimon_dsl::printed_duration_lint`; Blastmon (EX10-034)).

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
fn every_effect_play_pick_checks_play_legality_after_compilation() {
    let mut offenders = Vec::new();
    let mut playing_cards = 0usize;
    for file in digimon_dsl::authoring_lint::pool_card_files(&cards_root()) {
        let Ok(spec) = digimon_dsl::loader::load_file(&file) else {
            continue; // a spec that does not load is the loader's finding
        };
        let Ok(card) = digimon_dsl::compile::compile(&spec) else {
            continue; // likewise for the compiler
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

#[test]
fn printed_turn_durations_use_player_relative_expiries() {
    let official = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/card_official.json");
    let report = digimon_dsl::printed_duration_lint::check_pool(&cards_root(), &official)
        .expect("read data/card_official.json");
    assert!(report.checked > 1000, "only {} specs had printed text", report.checked);
    assert!(
        report.findings.is_empty(),
        "printed turn durations authored with the wrong expiry:
{}",
        report.findings.join("
")
    );
    assert!(
        report.stale_exemptions.is_empty(),
        "PRINTED_DURATION_EXEMPT entries no longer needed: {:?}",
        report.stale_exemptions
    );
}

/// `may_attack_now` has no `windowed` deferral any more: an effect's "may
/// attack" is declared inside the effect (CR 4.3 15-1-2). Monarchlizamon
/// (BT25-057) authored `windowed: true` and attacked in the end-of-turn
/// window, after its own Arts Digivolve (scan finding F02).
#[test]
fn may_attack_now_rejects_the_removed_windowed_flag() {
    let err = serde_yml::from_str::<digimon_dsl::step::StepSpec>(
        "may_attack_now: { attacker: source, optional: true, windowed: true }",
    )
    .expect_err("`windowed` must not parse");
    assert!(err.to_string().contains("windowed"), "{err}");
}

/// Each `REFUND_AFTER_ACCEPT_REVIEWED` entry (a `refund_opt` behind an
/// explicit accept, reviewed against the [Once Per Turn] table in
/// qa/rules/mechanics-gaps-2026-10.md W14) must still match a refund at its
/// clause; a stale entry (refund gone, clause moved) must be re-reviewed or
/// dropped. A new unreviewed clause fails the authoring-lint guard above.
#[test]
fn refund_after_accept_reviews_are_live() {
    let stale = digimon_dsl::authoring_lint::stale_refund_reviews(&cards_root());
    assert!(
        stale.is_empty(),
        "REFUND_AFTER_ACCEPT_REVIEWED entries no longer match their clause:\n{}",
        stale.join("\n")
    );
}

/// Declared [Main] refunds (owner ruling 2026-11-24, "Match dcgo on the main
/// use"): the pool's [Main] clauses with a `refund_opt` are exactly
/// `MAIN_REFUND_BY_OWNER_RULING`. A [Main] that refunds without being listed,
/// or a listed clause that no longer refunds, fails here.
#[test]
fn main_refunds_are_exactly_the_owner_ruling_list() {
    let found = digimon_dsl::authoring_lint::pool_main_refunds(&cards_root());
    let mut listed: Vec<(String, String)> = digimon_dsl::authoring_lint::MAIN_REFUND_BY_OWNER_RULING
        .iter()
        .map(|(c, k)| (c.to_string(), k.to_string()))
        .collect();
    listed.sort();
    assert_eq!(
        found, listed,
        "[Main] refunds in the pool (left) must equal MAIN_REFUND_BY_OWNER_RULING (right)"
    );
}
