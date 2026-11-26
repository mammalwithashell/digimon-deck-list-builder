//! When 2+ interactive `BeforePayCost` Option-use cost reducers qualify
//! simultaneously (e.g. two copies of BT25-090 Tomoro Tenma when using a
//! [Glowing Dawn] Option), `try_prompt_interactive_option_use_cost_reducer`
//! must surface a `SelectionKind::TriggerOrder` menu so the player can choose
//! which reducer activates first (CR 4.3 §15-4-3; DCGO `MultipleSkills`),
//! decline all when none is needed to pay the cost, or disallow declining all
//! when reduction is required to pay the cost (CR 4.3 §1-3-11-3).

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{HAND_EFFECT_START, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, CostDelta};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

fn filler(id: &str) -> CardData {
    let mut card = make_test_card_with_level(id, id, 3);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Green];
    card.play_cost = 3;
    card.dp = Some(3000);
    card
}

fn hand_slot(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

fn setup_two_tomoros(memory: i16) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-090")
        .expect("BT25-090 YAML loads")
        .dsl_card("BT25-057")
        .expect("BT25-057 YAML loads")
        .add_card(filler("FD-CARD"))
        .add_card(filler("ALLY-MON"))
        .add_card(filler("OPP-SEC"))
        .hand(0, &["BT25-057"])
        .deck(0, &["FD-CARD"; 4])
        .security(1, &["OPP-SEC"])
        .memory(memory)
        .start();

    // Tamer 0: BT25-090 with 0 face-down sources (still eligible because BT25-090
    // trashes from under ANY of your Tamers).
    let tamer0 = runner.place_on_field(0, "BT25-090", Some(0));
    // Tamer 1: BT25-090 with 1 face-down source under it.
    let tamer1 = runner.place_stack(0, &["FD-CARD", "BT25-090"]);
    runner.game.players[0].battle_area[tamer1.index as usize].card_sources[0].face_down = true;
    // Own Digimon for BT25-057's [Main] target.
    runner.place_on_field(0, "ALLY-MON", Some(0));

    (runner, tamer0, tamer1)
}

#[test]
fn two_interactive_option_use_reducers_present_trigger_order_and_allow_picking_second_copy() {
    let (mut runner, tamer0, tamer1) = setup_two_tomoros(1);
    let slot = hand_slot(&runner, 0, "BT25-057");

    runner
        .game
        .use_option_from_hand_with_cost(0, slot, CostDelta::Reduce(0));

    let pending = runner
        .pending_selection()
        .expect("two BT25-090 reducers must park on TriggerOrder");
    assert_eq!(pending.kind, SelectionKind::TriggerOrder);
    assert!(pending.is_optional, "affordable without reduction -> PASS legal");
    assert_eq!(
        pending.valid_action_ids,
        vec![HAND_EFFECT_START, HAND_EFFECT_START + 1],
        "both BT25-090 copies must be selectable"
    );
    let mask = build_action_mask(&runner.game, 0);
    assert!(mask[HAND_EFFECT_START as usize] > 0.5);
    assert!(mask[(HAND_EFFECT_START + 1) as usize] > 0.5);
    assert!(mask[PASS as usize] > 0.5);

    // Choose the SECOND BT25-090 copy (action 31), not the first.
    runner
        .execute_action(0, HAND_EFFECT_START + 1)
        .expect("pick second BT25-090");

    // Its pay_cost immediately parks on picking the Tamer with a face-down card (tamer1).
    let tamer_pick = runner
        .pending_selection()
        .expect("BT25-090 pay_cost parks on Tamer selection");
    assert_eq!(tamer_pick.kind, SelectionKind::OwnField);
    let pick_action = tamer_pick.valid_action_ids[0];
    runner
        .execute_action(0, pick_action)
        .expect("trash face-down card under tamer1");

    // Memory paid is 4 - 1 = 3 -> 1 - 3 = -2; second BT25-090 spent its OPT, first did not.
    assert_eq!(
        runner.game.players[0].battle_area[tamer1.index as usize]
            .card_sources
            .len(),
        1,
        "face-down card under tamer1 was trashed"
    );
    assert_eq!(runner.game.opt_used(tamer0), 0, "first BT25-090 OPT stays unspent");
    assert_eq!(runner.game.opt_used(tamer1), 1, "chosen second BT25-090 OPT is spent");
    assert_eq!(runner.memory(), -2, "Option cost reduced from 4 to 3 (1 - 3 = -2)");
}

#[test]
fn two_interactive_option_use_reducers_pass_declines_all_optional() {
    let (mut runner, tamer0, tamer1) = setup_two_tomoros(1);
    let slot = hand_slot(&runner, 0, "BT25-057");

    runner
        .game
        .use_option_from_hand_with_cost(0, slot, CostDelta::Reduce(0));
    assert_eq!(runner.pending_kind(), Some(SelectionKind::TriggerOrder));

    // Decline all optional Option-use reducers via PASS.
    runner.execute_action(0, PASS).expect("decline all reducers");

    assert_eq!(
        runner.game.players[0].battle_area[tamer1.index as usize]
            .card_sources
            .len(),
        2,
        "face-down card under tamer1 stays untouched"
    );
    assert_eq!(runner.game.opt_used(tamer0), 0, "tamer0 OPT unspent");
    assert_eq!(runner.game.opt_used(tamer1), 0, "tamer1 OPT unspent");
    assert_eq!(runner.memory(), -3, "full Option use cost 4 paid (1 - 4 = -3)");
}

#[test]
fn two_interactive_option_use_reducers_disallow_pass_when_reduction_is_needed() {
    // Use cost 4 at memory -7 -> -11 < -10 without reduction, so at least one
    // BT25-090 (-1 -> -10) is needed (CR 4.3 §1-3-11-3).
    let (mut runner, _tamer0, _tamer1) = setup_two_tomoros(-7);
    let slot = hand_slot(&runner, 0, "BT25-057");

    runner
        .game
        .use_option_from_hand_with_cost(0, slot, CostDelta::Reduce(0));

    let pending = runner
        .pending_selection()
        .expect("two BT25-090 reducers must park on TriggerOrder");
    assert_eq!(pending.kind, SelectionKind::TriggerOrder);
    assert!(
        !pending.is_optional,
        "declining all reducers leaves use cost 4 unpayable at memory -7"
    );
    let mask = build_action_mask(&runner.game, 0);
    assert!(mask[HAND_EFFECT_START as usize] > 0.5);
    assert!(mask[(HAND_EFFECT_START + 1) as usize] > 0.5);
    assert_eq!(
        mask[PASS as usize], 0.0,
        "PASS must be masked out when reduction is required to pay the Option cost"
    );
}
