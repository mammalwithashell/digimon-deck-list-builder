//! `trash_opponent_hand_to_count { bind_count_as }` result-count binding —
//! primitive coverage (G-DSL-TRASH-COUNT-RESULT-BINDING).
//!
//! Proves the mechanism in isolation with an inline `from_dsl_yaml` fixture:
//! an On Play clause forces the opponent to trash cards down to a target
//! count, records the number ACTUALLY trashed into a named literal binding,
//! then consumes that count through the existing `floor_div` /
//! `binding_value` formulas to cap a downstream opponent-Tamer multi-select
//! at `floor(trashed / 2)`. This is the shape BT19-075 MoonMillenniummon
//! needs ("For every 2 cards trashed by this effect, delete 1 of your
//! opponent's Tamers").

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

/// Inline fixture: [On Play] trash opponent's hand to 5; for every 2 trashed
/// by THIS effect, delete 1 of the opponent's Tamers.
const FIXTURE_YAML: &str = r#"
card: TEST-TRASHCOUNT
name: Test Trash Count Binder
kind: digimon
level: 6
color: [purple]
cost: 12
dp: 12000
effects:
  - when: [on_play]
    process:
      - trash_opponent_hand_to_count:
          opponent: opponent
          target_count: 5
          bind_count_as: trashed
      - select_count_capped_multi:
          of: opponent
          zone: battle_area
          max:
            formula:
              floor_div:
                - { binding_value: trashed }
                - 2
          clamp_to_available: true
          filter: { kind: tamer }
          bind_as: doomed_tamers
          prompt: "Choose opponent Tamers to delete"
          optional_zero: false
      - per_selected:
          selection: doomed_tamers
          bind_as: t
          body:
            - delete_permanent: { target: t }
"#;

const CARD_ID: &str = "TEST-TRASHCOUNT";

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
        .from_dsl_yaml(FIXTURE_YAML)
        .expect("inline fixture compiles")
        .add_card(digimon("H1", "Hand 1", 3, 1000))
        .add_card(digimon("H2", "Hand 2", 3, 1000))
        .add_card(digimon("H3", "Hand 3", 3, 1000))
        .add_card(digimon("H4", "Hand 4", 3, 1000))
        .add_card(digimon("H5", "Hand 5", 3, 1000))
        .add_card(digimon("H6", "Hand 6", 3, 1000))
        .add_card(digimon("H7", "Hand 7", 3, 1000))
        .add_card(digimon("H8", "Hand 8", 3, 1000))
        .add_card(digimon("H9", "Hand 9", 3, 1000))
        .add_card(tamer("T1", "Tamer 1"))
        .add_card(tamer("T2", "Tamer 2"))
        .add_card(tamer("T3", "Tamer 3"))
        .memory(10)
        .start()
}

fn fill_opp_hand(runner: &mut DebugRunner, ids: &[&str]) {
    for id in ids {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| &c.card_id == id)
            .unwrap_or_else(|| panic!("fill_opp_hand: unknown card {id}"));
        let next = runner.game.next_card_index();
        runner.game.players[1]
            .hand
            .push(CardSource::new(data_idx, 1, next));
    }
}

/// trashed = 4 → floor(4/2) = 2 Tamers deleted.
#[test]
fn bind_count_as_caps_tamer_delete_at_floor_half() {
    let mut runner = runner();
    let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "T1", Some(0));
    runner.place_on_field(1, "T2", Some(0));
    runner.place_on_field(1, "T3", Some(0));
    fill_opp_hand(
        &mut runner,
        &["H1", "H2", "H3", "H4", "H5", "H6", "H7", "H8", "H9"],
    ); // 9 → trash 4

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("resolve trash + delete");

    assert_eq!(runner.game.players[1].hand.len(), 5, "trashed 4 to reach 5");
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "floor(4/2)=2 Tamers deleted (3 → 1)"
    );
}

/// trashed = 3 → floor(3/2) = 1 Tamer deleted (proves the FLOOR, not round-up).
#[test]
fn bind_count_as_floors_the_division() {
    let mut runner = runner();
    let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "T1", Some(0));
    runner.place_on_field(1, "T2", Some(0));
    fill_opp_hand(
        &mut runner,
        &["H1", "H2", "H3", "H4", "H5", "H6", "H7", "H8"],
    ); // 8 → trash 3

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("resolve");

    assert_eq!(runner.game.players[1].hand.len(), 5, "trashed 3 to reach 5");
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "floor(3/2)=1 Tamer deleted (not 2)"
    );
}

/// trashed = 1 → floor(1/2) = 0 → the downstream select no-ops entirely.
#[test]
fn bind_count_as_zero_when_below_two_trashed() {
    let mut runner = runner();
    let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "T1", Some(0));
    runner.place_on_field(1, "T2", Some(0));
    fill_opp_hand(&mut runner, &["H1", "H2", "H3", "H4", "H5", "H6"]); // 6 → trash 1

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("resolve");

    assert_eq!(runner.game.players[1].hand.len(), 5, "trashed 1 to reach 5");
    assert_eq!(
        runner.battle_area_size(1),
        2,
        "floor(1/2)=0 → no Tamer selection, none deleted"
    );
}

/// No-op path (hand already ≤ target): the binding defaults to 0 and the
/// downstream select cleanly no-ops (no lingering pending selection).
#[test]
fn bind_count_as_defaults_zero_on_noop_trash() {
    let mut runner = runner();
    let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "T1", Some(0));
    runner.place_on_field(1, "T2", Some(0));
    fill_opp_hand(&mut runner, &["H1", "H2", "H3"]); // 3 ≤ 5 → no trash

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no trash + count=0 → no Tamer selection installs"
    );
    assert_eq!(runner.game.players[1].hand.len(), 3, "hand untouched");
    assert_eq!(runner.battle_area_size(1), 2, "no Tamers deleted");
}
