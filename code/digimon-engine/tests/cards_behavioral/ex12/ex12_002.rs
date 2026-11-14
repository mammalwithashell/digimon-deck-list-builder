use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX12-002";

fn red_sw_digimon(id: &str, level: u8, play_cost: u16) -> CardData {
    let mut card = make_test_card_with_level(id, id, level);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.traits = vec!["SW".to_string()];
    card.play_cost = play_cost;
    card.dp = Some(if level <= 3 { 3000 } else { 5000 });
    card
}

fn red_sw_evo(id: &str) -> CardData {
    let mut card = red_sw_digimon(id, 4, 4);
    card.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 3,
    }];
    card
}

fn plain_red_digimon(id: &str) -> CardData {
    let mut card = red_sw_digimon(id, 3, 3);
    card.traits.clear();
    card
}

fn hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|card| card.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

fn select_hand_card(runner: &mut DebugRunner, player: u8, card_id: &str) {
    for _ in 0..8 {
        let view = runner
            .pending_selection_view()
            .expect("EX12-002 should install a hand selection");
        if view.kind == SelectionKind::Hand {
            let slot = hand_index(runner, player, card_id) as u16;
            let action = [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
                .into_iter()
                .find(|action| view.valid_action_ids.contains(action))
                .unwrap_or_else(|| panic!("{card_id} was not selectable from hand; view={view:?}"));
            runner
                .execute_action(player, action)
                .expect("select EX12-002 hand digivolve target");
            return;
        }
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|action| *action != PASS)
            .unwrap_or_else(|| panic!("unexpected EX12-002 prompt with only PASS: {view:?}"));
        runner
            .execute_action(view.selecting_player, action)
            .expect("advance EX12-002 prompt");
    }
    panic!("EX12-002 never reached the expected hand selection");
}

fn top_card_id(runner: &DebugRunner, player: u8, field_index: usize) -> String {
    runner.game.players[player as usize].battle_area[field_index]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

#[test]
fn ex12_002_inherited_when_other_sw_played_may_digivolve_source_with_cost_reduced_by_2() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-002 YAML loads")
        .add_card(red_sw_digimon("CARRIER", 3, 3))
        .add_card(red_sw_digimon("SW-ALLY", 3, 3))
        .add_card(red_sw_evo("SW-EVO"))
        .add_card(plain_red_digimon("DRAW"))
        .hand(0, &["SW-ALLY", "SW-EVO"])
        .deck(0, &["DRAW"])
        .memory(10)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let memory_before = runner.memory();
    let ally_slot = hand_index(&runner, 0, "SW-ALLY");

    assert!(
        runner.play(0, ally_slot).is_some(),
        "SW ally should be playable"
    );
    select_hand_card(&mut runner, 0, "SW-EVO");
    runner.auto_resolve().expect("finish EX12-002 effect");

    assert_eq!(
        top_card_id(&runner, 0, carrier.index as usize),
        "SW-EVO",
        "EX12-002's inherited effect should digivolve the source carrier"
    );
    assert_eq!(
        memory_before - runner.memory(),
        4,
        "play cost 3 plus digivolve cost 3 reduced by 2 should spend 4 memory"
    );
}

#[test]
fn ex12_002_inherited_does_not_trigger_for_non_sw_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-002 YAML loads")
        .add_card(red_sw_digimon("CARRIER", 3, 3))
        .add_card(plain_red_digimon("PLAIN-ALLY"))
        .add_card(red_sw_evo("SW-EVO"))
        .hand(0, &["PLAIN-ALLY", "SW-EVO"])
        .memory(10)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let ally_slot = hand_index(&runner, 0, "PLAIN-ALLY");
    assert!(
        runner.play(0, ally_slot).is_some(),
        "plain ally should be playable"
    );
    let _ = runner.auto_resolve();

    assert_eq!(
        top_card_id(&runner, 0, carrier.index as usize),
        "CARRIER",
        "non-SW plays must not trigger EX12-002"
    );
    assert!(
        runner.pending_selection().is_none(),
        "non-SW play should not leave an EX12-002 selection pending"
    );
}

// --- Hand-pick candidate legality (select-hand playability audit) ---
// DCGO EX12_002.cs:53-64 calls `CardEffectCommons.DigivolveIntoHandOrTrashCard
// (targetPermanent: this Digimon, cardCondition: EqualsTraits("SW"), payCost:
// true, reduceCostTuple: (2, null), isHand: true)`, whose candidate filter
// (CardEffectCommons.cs:858-882) is `IsDigimon && cardCondition &&
// !CanNotEvolve(target) && CanPlayCardTargetFrame(target, PayCost: true, ...)`
// with the -2 already installed (CardEffectCommons.cs:887-929): only an [SW]
// Digimon this Digimon can digivolve into (requirements met, CR 4.3 8-1) and
// whose reduced cost can be paid (CR 4.3 1-3-11-1).

/// Mococomon (EX12-002) under a red level 3 [SW] carrier, with `hand` in hand.
/// SW-ALLY is placed and its play event fired with memory `memory`; returns the
/// hand cards the digivolve pick offers (sorted; empty if no pick installs).
fn ex12_002_offered(extra: Vec<CardData>, hand: &[&str], memory: i16) -> Vec<String> {
    let mut builder = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-002 YAML loads")
        .add_card(red_sw_digimon("CARRIER", 3, 3))
        .add_card(red_sw_digimon("SW-ALLY", 3, 3))
        .add_card(red_sw_evo("SW-EVO"));
    for card in extra {
        builder = builder.add_card(card);
    }
    let mut runner = builder.hand(0, hand).memory(10).start();
    runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let ally = runner.place_on_field(0, "SW-ALLY", Some(0));
    runner.game.memory = memory;
    runner.fire_play_event_triggers(ally.player, ally.index as usize, false, false);

    for _ in 0..8 {
        let Some(view) = runner.pending_selection_view() else {
            return Vec::new();
        };
        if view.kind == SelectionKind::Hand {
            let hand_ids: Vec<String> = runner.game.players[0]
                .hand
                .iter()
                .map(|c| c.card_id(&runner.game.card_data).to_string())
                .collect();
            let mut offered: Vec<String> = view
                .valid_action_ids
                .iter()
                .copied()
                .filter(|&id| id != PASS)
                .map(|id| {
                    let slot = if id >= HAND_EFFECT_START {
                        id - HAND_EFFECT_START
                    } else {
                        id - PLAY_HAND_START
                    };
                    hand_ids[slot as usize].clone()
                })
                .collect();
            offered.sort();
            return offered;
        }
        let Some(action) = view.valid_action_ids.iter().copied().find(|a| *a != PASS) else {
            return Vec::new();
        };
        runner
            .execute_action(view.selecting_player, action)
            .expect("advance EX12-002 prompt");
    }
    panic!("EX12-002 never reached the hand selection");
}

/// A level 6 [SW] Digimon can't digivolve onto the level 3 carrier (no
/// digivolution requirement met), so it isn't offered. Regression guard: the
/// compile pass adds the requirement check to requirement-honouring digivolves.
#[test]
fn ex12_002_pick_excludes_sw_card_whose_requirement_is_unmet() {
    let mut lv6 = red_sw_digimon("SW-LV6", 6, 12);
    lv6.evo_costs = vec![EvoCost { card_color: 0, level: 5, memory_cost: 3 }];
    let offered = ex12_002_offered(vec![lv6], &["SW-EVO", "SW-LV6"], 10);
    assert_eq!(offered, vec!["SW-EVO".to_string()]);
}

/// At memory -10 (10 on the opponent's side) a digivolution cost of 3 reduced
/// to 1 can't be paid, but one of 2 reduced to 0 can: only the latter is
/// offered (DCGO `CanPlayCardTargetFrame(PayCost: true)`; CR 4.3 1-3-11-1).
#[test]
fn ex12_002_pick_excludes_unaffordable_digivolution() {
    let mut cheap = red_sw_digimon("SW-EVO-CHEAP", 4, 4);
    cheap.evo_costs = vec![EvoCost { card_color: 0, level: 3, memory_cost: 2 }];
    let offered = ex12_002_offered(vec![cheap], &["SW-EVO", "SW-EVO-CHEAP"], -10);
    assert_eq!(offered, vec!["SW-EVO-CHEAP".to_string()]);
}
