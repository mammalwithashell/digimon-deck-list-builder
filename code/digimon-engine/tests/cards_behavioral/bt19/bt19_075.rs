//! BT19-075 MoonMillenniummon — Track E forced hand-reduction + Tamer-deletion
//! rider (result-count binding), plus the inline-fixture proof of the
//! `trash_opponent_hand_to_count { bind_count_as }` → `floor_div` mechanism.
//!
//! Printed [On Play] / [When Digivolving] (shared):
//!   Your opponent trashes cards in their hand until they have 5 left. For
//!   every 2 cards trashed by this effect, delete 1 of your opponent's Tamers.
//!
//! DCGO oracle (BT19_075.cs, OnEnterFieldAnyone blocks):
//!   - Opponent trashes (hand - 5) cards (mandatory, canNoSelect:false).
//!   - `maxDeletes = min(floor(trashed/2), opponentTamerCount)`.
//!   - The CARD'S CONTROLLER picks `maxDeletes` opponent Tamers to delete
//!     (mandatory, canNoSelect:false), only when `trashed >= 2`.
//!
//! Still BLOCKED (out of scope for this slice): the [All Turns] Composite
//! delete-cost leave replacement, and the [All Turns][OPT] OnAnyDeletion
//! security-trash observer.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT19-075";

fn digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card_with_level(id, name, level);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card
}

fn tamer(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-075 YAML loads")
        // MoonMillenniummon carrier goes on P0's field.
        // Opponent hand filler + opponent Tamers.
        .add_card(digimon("HAND-A", "Hand A", 3, 1000))
        .add_card(digimon("HAND-B", "Hand B", 3, 1000))
        .add_card(digimon("HAND-C", "Hand C", 3, 1000))
        .add_card(digimon("HAND-D", "Hand D", 3, 1000))
        .add_card(digimon("HAND-E", "Hand E", 3, 1000))
        .add_card(digimon("HAND-F", "Hand F", 3, 1000))
        .add_card(digimon("HAND-G", "Hand G", 3, 1000))
        .add_card(digimon("HAND-H", "Hand H", 3, 1000))
        .add_card(digimon("HAND-I", "Hand I", 3, 1000))
        .add_card(tamer("OPP-TAMER-1", "Opp Tamer 1"))
        .add_card(tamer("OPP-TAMER-2", "Opp Tamer 2"))
        .add_card(tamer("OPP-TAMER-3", "Opp Tamer 3"))
        .add_card(digimon("OWN-TAMER-DECOY", "own decoy", 3, 1000))
        .memory(10)
        .start()
}

/// Push `n` filler cards into the opponent's (P1) hand.
fn fill_opp_hand(runner: &mut DebugRunner, ids: &[&str]) {
    for id in ids {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| &c.card_id == id)
            .unwrap_or_else(|| panic!("fill_opp_hand: unknown card {id}"));
        let next = runner.game.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, 1, next);
        runner.game.players[1].hand.push(card);
    }
}

// ───────────────────── Structural ─────────────────────────────────────────

#[test]
fn bt19_075_now_authors_the_trash_and_tamer_deletion_clause() {
    let r = runner();
    let compiled = r.compiled_card(CARD_ID).expect("compiled BT19-075");
    assert!(
        !compiled.effects.is_empty(),
        "BT19-075 now authors the Track E trash + Tamer-deletion rider"
    );
}

// ───────────────────── Real card behavior ─────────────────────────────────

/// Opponent has 9 hand cards → trashes 4 to reach 5 → floor(4/2)=2 Tamers
/// deleted. Opponent has 3 Tamers, so 2 of them go.
#[test]
fn bt19_075_trash_four_deletes_two_tamers() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    // Opponent field: 3 Tamers (deletion candidates).
    runner.place_on_field(1, "OPP-TAMER-1", Some(0));
    runner.place_on_field(1, "OPP-TAMER-2", Some(0));
    runner.place_on_field(1, "OPP-TAMER-3", Some(0));
    // Opponent hand: 9 cards → must trash 4 to reach 5.
    fill_opp_hand(
        &mut runner,
        &[
            "HAND-A", "HAND-B", "HAND-C", "HAND-D", "HAND-E", "HAND-F", "HAND-G", "HAND-H", "HAND-I",
        ],
    );
    assert_eq!(runner.game.players[1].hand.len(), 9);
    assert_eq!(runner.battle_area_size(1), 3, "3 opponent Tamers to start");

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(moon));
    runner.game.drain_effect_queue();

    // First: the opponent trashes 4 hand cards (mandatory count-capped).
    runner.auto_resolve().expect("resolve trash + Tamer deletion");

    assert_eq!(
        runner.game.players[1].hand.len(),
        5,
        "opponent trashes down to 5 cards"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "floor(4/2)=2 opponent Tamers deleted (3 → 1)"
    );
}

/// Opponent has 8 hand cards → trashes 3 → floor(3/2)=1 Tamer deleted.
#[test]
fn bt19_075_trash_three_deletes_one_tamer() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-TAMER-1", Some(0));
    runner.place_on_field(1, "OPP-TAMER-2", Some(0));
    fill_opp_hand(
        &mut runner,
        &[
            "HAND-A", "HAND-B", "HAND-C", "HAND-D", "HAND-E", "HAND-F", "HAND-G", "HAND-H",
        ],
    );
    assert_eq!(runner.game.players[1].hand.len(), 8);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(moon));
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("resolve");

    assert_eq!(runner.game.players[1].hand.len(), 5, "trashed 3 to reach 5");
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "floor(3/2)=1 Tamer deleted (2 → 1)"
    );
}

/// Opponent has 6 hand cards → trashes 1 → floor(1/2)=0 → no Tamer deleted.
#[test]
fn bt19_075_trash_one_deletes_no_tamer() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-TAMER-1", Some(0));
    runner.place_on_field(1, "OPP-TAMER-2", Some(0));
    fill_opp_hand(
        &mut runner,
        &["HAND-A", "HAND-B", "HAND-C", "HAND-D", "HAND-E", "HAND-F"],
    );
    assert_eq!(runner.game.players[1].hand.len(), 6);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(moon));
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("resolve");

    assert_eq!(runner.game.players[1].hand.len(), 5, "trashed 1 to reach 5");
    assert_eq!(
        runner.battle_area_size(1),
        2,
        "floor(1/2)=0 → no Tamers deleted"
    );
}

/// Deletion clamps to the number of opponent Tamers actually present:
/// trash 4 → floor(4/2)=2 wanted, but only 1 Tamer exists → delete 1.
#[test]
fn bt19_075_tamer_deletion_clamps_to_available() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-TAMER-1", Some(0)); // only 1 Tamer
    fill_opp_hand(
        &mut runner,
        &[
            "HAND-A", "HAND-B", "HAND-C", "HAND-D", "HAND-E", "HAND-F", "HAND-G", "HAND-H", "HAND-I",
        ],
    );

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(moon));
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("resolve");

    assert_eq!(runner.game.players[1].hand.len(), 5, "trashed 4 to reach 5");
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "wanted 2 deletes but only 1 Tamer present → delete the 1"
    );
}

/// Opponent Digimon are NOT deletion candidates (only Tamers). trash 4 → 2
/// wanted, but the only opponent permanent is a Digimon → nothing deleted.
#[test]
fn bt19_075_only_deletes_tamers_not_digimon() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "HAND-A", Some(0)); // an opponent Digimon on field
    fill_opp_hand(
        &mut runner,
        &[
            "HAND-A", "HAND-B", "HAND-C", "HAND-D", "HAND-E", "HAND-F", "HAND-G", "HAND-H", "HAND-I",
        ],
    );

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(moon));
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("resolve");

    assert_eq!(runner.game.players[1].hand.len(), 5);
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "the opponent Digimon is not a Tamer → not deleted"
    );
}

/// No-op path: opponent hand already ≤ 5 → no trashing, no Tamer deletion,
/// no lingering selection.
#[test]
fn bt19_075_noop_when_hand_at_or_below_five() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-TAMER-1", Some(0));
    runner.place_on_field(1, "OPP-TAMER-2", Some(0));
    fill_opp_hand(&mut runner, &["HAND-A", "HAND-B", "HAND-C"]); // only 3 in hand

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(moon));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "hand already ≤ 5 → no trash selection installs"
    );
    assert_eq!(runner.game.players[1].hand.len(), 3, "hand untouched");
    assert_eq!(runner.battle_area_size(1), 2, "no Tamers deleted");
}

/// [When Digivolving] fires the same clause.
#[test]
fn bt19_075_when_digivolving_also_trashes_and_deletes() {
    let mut runner = runner();
    let moon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-TAMER-1", Some(0));
    runner.place_on_field(1, "OPP-TAMER-2", Some(0));
    fill_opp_hand(
        &mut runner,
        &[
            "HAND-A", "HAND-B", "HAND-C", "HAND-D", "HAND-E", "HAND-F", "HAND-G",
        ],
    ); // 7 → trash 2 → floor(2/2)=1

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(moon),
    );
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("resolve");

    assert_eq!(runner.game.players[1].hand.len(), 5, "trashed 2 to reach 5");
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "floor(2/2)=1 Tamer deleted (2 → 1)"
    );
}

// ───────────────────── Composite / observer still blocked ──────────────────

#[test]
#[ignore = "blocked on Composite delete-cost replacement authoring for MoonMillenniummon leave prevention"]
fn bt19_075_composite_cost_prevents_leaving_battle_area() {}
