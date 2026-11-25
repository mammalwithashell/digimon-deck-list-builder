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
//! 3. `printed_duration_lint` -- a printed "until your turn ends" / "until your
//!    opponent's turn ends" is authored with the matching player-relative
//!    expiry, not `end_of_turn` (CR 4.3 15-10-1-1-1): Blastmon (EX10-034)
//!    lost its bonus a turn early when used on the opponent's turn.

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

#[test]
fn pool_guard_printed_turn_durations_use_player_relative_expiries() {
    let official = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/card_official.json");
    let report = digimon_dsl::printed_duration_lint::check_pool(&cards_root(), &official)
        .expect("read data/card_official.json");
    assert!(
        report.checked > 1000,
        "only {} specs had printed text -- is card_official.json present?",
        report.checked
    );
    assert!(
        report.findings.is_empty(),
        "printed turn durations authored with the wrong expiry:
{}",
        report.findings.join("
")
    );
    assert!(
        report.stale_exemptions.is_empty(),
        "PRINTED_DURATION_EXEMPT entries no longer needed (remove them): {:?}",
        report.stale_exemptions
    );
}

/// Each `REFUND_AFTER_ACCEPT_REVIEWED` entry (a `refund_opt` behind an
/// explicit accept, reviewed against the [Once Per Turn] table in
/// qa/rules/mechanics-gaps-2026-10.md W14) must still match a refund at its
/// clause; a stale entry (refund gone, clause moved) must be re-reviewed or
/// dropped. A new unreviewed clause fails the authoring-lint guard above.
#[test]
fn pool_guard_refund_after_accept_reviews_are_live() {
    let stale = digimon_dsl::authoring_lint::stale_refund_reviews(&cards_root());
    assert!(
        stale.is_empty(),
        "REFUND_AFTER_ACCEPT_REVIEWED entries no longer match their clause:\n{}",
        stale.join("\n")
    );
}

/// Each `REFUND_AFTER_ACCEPT_REVIEWED` entry names the behavioural tests that
/// prove its row (the table itself is a review record: the lint cannot see
/// which path reaches a `refund_opt`). A named test that no longer exists
/// leaves the entry unproven.
#[test]
fn pool_guard_refund_after_accept_reviews_name_existing_tests() {
    let tests_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut missing = Vec::new();
    for entry in digimon_dsl::authoring_lint::REFUND_AFTER_ACCEPT_REVIEWED {
        for named in entry.tests {
            let (file, test) = named.split_once("::").expect("<file>::<test fn>");
            let source = std::fs::read_to_string(tests_root.join(file)).unwrap_or_default();
            let declared = source
                .match_indices(&format!("fn {test}("))
                .any(|(at, _)| source[..at].trim_end().ends_with("#[test]"));
            if !declared {
                missing.push(format!("{} {}: {named}", entry.card, entry.clause));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "REFUND_AFTER_ACCEPT_REVIEWED names tests that do not exist:\n{}",
        missing.join("\n")
    );
}

/// Declared [Main] refunds (owner ruling 2026-11-24, "Match dcgo on the main
/// use"): the pool's [Main] clauses with a `refund_opt` are exactly
/// `MAIN_REFUND_BY_OWNER_RULING`. A [Main] that refunds without being listed,
/// or a listed clause that no longer refunds, fails here.
#[test]
fn pool_guard_main_refunds_are_exactly_the_owner_ruling_list() {
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

/// Unpayable deletion payers (owner ruling 2026-11-24, Chuumon (EX13-027),
/// Sukamon (EX13-028), Kotemon (EX13-048), Gladimon (EX13-052)): the pool's
/// clauses with `offer_unpayable_deletion_payers: true` are exactly
/// `UNPAYABLE_DELETION_PAYERS_BY_OWNER_RULING`; every other paid-deletion
/// clause keeps the CR 4.3 15-7-4 payer filter.
#[test]
fn pool_guard_unpayable_deletion_payers_are_exactly_the_owner_ruling_list() {
    let found = digimon_dsl::authoring_lint::pool_unpayable_deletion_payer_offers(&cards_root());
    let mut listed: Vec<(String, String)> =
        digimon_dsl::authoring_lint::UNPAYABLE_DELETION_PAYERS_BY_OWNER_RULING
            .iter()
            .map(|(c, k)| (c.to_string(), k.to_string()))
            .collect();
    listed.sort();
    assert_eq!(
        found, listed,
        "clauses offering unpayable deletion payers (left) must equal \
         UNPAYABLE_DELETION_PAYERS_BY_OWNER_RULING (right)"
    );
}
