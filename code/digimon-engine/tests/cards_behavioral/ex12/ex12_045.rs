use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_END, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::selection::SelectionKind;

use super::support::{
    digimon, fire_own_security_removed, hand_contains, hand_index, plain_digimon,
    select_first_non_pass, select_hand_card, tamer, yellow_sw, DebugRunner,
};

const CARD_ID: &str = "EX12-045";

/// Walk the prompts (accepting each non-hand prompt with its first non-PASS
/// action) to the hand pick and return the card ids it offers, sorted. Empty
/// when no hand pick installs.
fn offered_hand_ids(runner: &mut DebugRunner, player: u8) -> Vec<String> {
    for _ in 0..8 {
        let Some(view) = runner.pending_selection_view() else {
            return Vec::new();
        };
        if view.kind == SelectionKind::Hand {
            let hand: Vec<String> = runner.game.players[player as usize]
                .hand
                .iter()
                .map(|c| c.card_id(&runner.game.card_data).to_string())
                .collect();
            let mut ids: Vec<String> = view
                .valid_action_ids
                .iter()
                .copied()
                .filter(|&a| a != PASS)
                .map(|a| {
                    let slot = if a < PLAY_HAND_END { a - PLAY_HAND_START } else { a - HAND_EFFECT_START };
                    hand[slot as usize].clone()
                })
                .collect();
            ids.sort();
            return ids;
        }
        let Some(action) = view.valid_action_ids.iter().copied().find(|&a| a != PASS) else {
            return Vec::new();
        };
        runner.execute_action(view.selecting_player, action).expect("advance prompt");
    }
    panic!("never reached the hand pick");
}

/// An [SW] Tamer (cost 3, so 1 after the -2).
fn sw_tamer(id: &str) -> CardData {
    let mut card = tamer(id, CardColor::Yellow);
    card.traits = vec!["SW".to_string()];
    card
}

/// Sanzomon (EX12-045) on the field, `memory` memory, `hand` in hand; returns
/// the runner ready to fire "When your security stack is removed from".
fn sanzomon_runner(extra: Vec<CardData>, hand: &[&str], memory: i16) -> DebugRunner {
    let mut builder = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-045 YAML loads")
        .add_card(plain_digimon("SEC", CardColor::Yellow, 3, 3000));
    for card in extra {
        builder = builder.add_card(card);
    }
    let mut runner = builder
        .hand(0, hand)
        .security(0, &["SEC"])
        .memory(memory)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner
}

// ── [Your Turn][OPT] security-removed play: candidate legality ───────────────
// DCGO EX12_045.cs:106-111 `CanSelectCardCondition`: text/trait AND
// `CardEffectCommons.CanPlayAsNewPermanent(cardSource, true, activateClass,
// fixedCost: GetCostItself - 2)` (GameContextDeterminarion.cs:419-432): not an
// Option (so never a DUAL, whose kinds include Option — CR 4.3 4-6-2: a DUAL is
// digivolved or used, never played), an empty battle-area frame, and
// `CanPlayCardTargetFrame(payCost: true)` — no play prohibition, a play cost
// (never a Digi-Egg, CR 4.3 7-1-1) and the reduced cost payable (7-1-3-2).
// A card the play would refuse is not a legal choice (CR 4.3 1-3-2).

/// An [SW] Digimon costing 13 (11 after the -2) can't be paid from 0 memory
/// (memory would fall below -10): not offered; the payable one is.
#[test]
fn ex12_045_security_removed_pick_excludes_unaffordable_card() {
    let pricey = digimon("SW-PRICEY", CardColor::Yellow, 6, 13, 12000, &["SW"]);
    let mut runner = sanzomon_runner(
        vec![yellow_sw("SW-PLAY", 3), pricey],
        &["SW-PLAY", "SW-PRICEY"],
        0,
    );
    fire_own_security_removed(&mut runner, 0);
    assert_eq!(offered_hand_ids(&mut runner, 0), vec!["SW-PLAY".to_string()]);
}

/// "You can't play Digimon by effects": the [SW] Digimon is not offered, the
/// [SW] Tamer still is.
#[test]
fn ex12_045_security_removed_pick_excludes_digimon_under_effect_play_prohibition() {
    let mut runner = sanzomon_runner(
        vec![yellow_sw("SW-PLAY", 3), sw_tamer("SW-TAMER")],
        &["SW-PLAY", "SW-TAMER"],
        10,
    );
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
    );
    fire_own_security_removed(&mut runner, 0);
    assert_eq!(offered_hand_ids(&mut runner, 0), vec!["SW-TAMER".to_string()]);
}

/// A full battle area: nothing can be played, so nothing is offered.
#[test]
fn ex12_045_security_removed_pick_offers_nothing_when_battle_area_full() {
    let mut runner = sanzomon_runner(
        vec![yellow_sw("SW-PLAY", 3), sw_tamer("SW-TAMER")],
        &["SW-PLAY", "SW-TAMER"],
        10,
    );
    runner.game.rules.field_slots = runner.game.players[0].battle_area.len() as u8;
    fire_own_security_removed(&mut runner, 0);
    assert!(offered_hand_ids(&mut runner, 0).is_empty());
    assert!(hand_contains(&runner, 0, "SW-PLAY") && hand_contains(&runner, 0, "SW-TAMER"));
}

/// A [SW] Digi-Egg has no play cost (CR 4.3 7-1-1): never offered. Regression
/// guard (the pick's `kind: digimon | tamer` already excludes it).
#[test]
fn ex12_045_security_removed_pick_excludes_sw_digi_egg() {
    let mut egg = digimon("SW-EGG", CardColor::Yellow, 2, 0, 0, &["SW"]);
    egg.card_kind = CardKind::DigiEgg;
    egg.dp = None;
    let mut runner = sanzomon_runner(vec![yellow_sw("SW-PLAY", 3), egg], &["SW-PLAY", "SW-EGG"], 10);
    fire_own_security_removed(&mut runner, 0);
    assert_eq!(offered_hand_ids(&mut runner, 0), vec!["SW-PLAY".to_string()]);
}

#[test]
fn ex12_045_inherited_when_attacking_is_not_optional_after_trigger() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-045 YAML loads")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-045");
    let inherited = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.scope == CompiledScope::Inherited
                    && triggered.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("printed inherited [When Attacking] clause");

    assert!(
        !inherited.optional,
        "printed inherited body has no 'you may' after the trigger resolves"
    );
}

#[test]
fn ex12_045_on_play_adds_top_security_then_recovers_when_at_two_or_less() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-045 YAML loads")
        .add_card(plain_digimon("SEC-A", CardColor::Yellow, 3, 3000))
        .add_card(plain_digimon("SEC-B", CardColor::Yellow, 3, 3000))
        .add_card(plain_digimon("SEC-C", CardColor::Yellow, 3, 3000))
        .add_card(plain_digimon("RECOVER", CardColor::Yellow, 3, 3000))
        .hand(0, &[CARD_ID])
        .security(0, &["SEC-A", "SEC-B", "SEC-C"])
        .deck(0, &["RECOVER"])
        .memory(10)
        .start();

    let hand_before = runner.game.players[0].hand.len();
    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-045");
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish EX12-045 on-play");

    assert_eq!(
        runner.security_count(0),
        3,
        "starting at 3 security: add 1 to hand leaves 2, then Recovery +1 returns to 3"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "played EX12-045 from hand and added one security card to hand"
    );
    assert_eq!(runner.deck_size(0), 0, "Recovery +1 uses the deck card");
}

#[test]
fn ex12_045_security_removed_plays_sw_from_hand_with_cost_reduced_by_2() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-045 YAML loads")
        .add_card(yellow_sw("SW-PLAY", 3))
        .add_card(plain_digimon("SEC", CardColor::Yellow, 3, 3000))
        .hand(0, &["SW-PLAY"])
        .security(0, &["SEC"])
        .memory(10)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let field_before = runner.battle_area_size(0);
    let memory_before = runner.memory();

    fire_own_security_removed(&mut runner, 0);
    select_hand_card(&mut runner, 0, "SW-PLAY");
    runner
        .auto_resolve()
        .expect("finish EX12-045 security-removed play");

    assert_eq!(runner.battle_area_size(0), field_before + 1);
    assert!(!hand_contains(&runner, 0, "SW-PLAY"));
    assert_eq!(memory_before - runner.memory(), 1);
}

#[test]
fn ex12_045_inherited_when_attacking_reduces_one_opponent_digimon_by_4000() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-045 YAML loads")
        .add_card(plain_digimon("CARRIER", CardColor::Yellow, 4, 7000))
        .add_card(plain_digimon("OPP", CardColor::Blue, 4, 6000))
        .add_card(plain_digimon("SEC", CardColor::Blue, 3, 3000))
        .security(1, &["SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.attack_player(carrier, 1, false);
    select_first_non_pass(&mut runner);
    runner
        .auto_resolve()
        .expect("finish inherited DP reduction");

    assert_eq!(runner.effective_dp(opp), Some(2000));
}

/// EX12-048 SeitenGokuumon (cost 13, Assembly -6) at memory -3:
/// 13 - 2 = 11 is unpayable from -3, but with 3 distinct Assembly materials
/// in trash the payable cost is 13 - 2 - 6 = 5 (-3 - 5 = -8 >= -10), so
/// `affordable_with_cost: { reduce: 2 }` MUST offer EX12-048 (DCGO
/// `CardSource.GetPayingCostWithBaseCost` `checkAvailability: true`).
/// Without 3 distinct materials (or under `CannotReducePlayCost`), EX12-048
/// must remain excluded.
#[test]
fn ex12_045_security_removed_pick_credits_assembly_cost_reduction() {
    let build = |trash_ids: &[&str], lock_reduction: bool| {
        let mut runner = DebugRunner::builder()
            .dsl_card(CARD_ID)
            .expect("EX12-045 YAML loads")
            .dsl_card("EX12-048")
            .expect("EX12-048 YAML loads")
            .dsl_card("EX12-029")
            .expect("EX12-029 YAML loads")
            .dsl_card("EX12-056")
            .expect("EX12-056 YAML loads")
            .add_card(plain_digimon("SEC", CardColor::Yellow, 3, 3000))
            .hand(0, &["EX12-048"])
            .security(0, &["SEC"])
            .memory(-3)
            .start();
        runner.place_on_field(0, CARD_ID, Some(0));
        for &cid in trash_ids {
            runner.inject_trash(0, cid);
        }
        if lock_reduction {
            runner.game.modifiers.add_player_modifier(
                0,
                PlayerModifierEntry::simple(
                    ModifierType::CannotReducePlayCost,
                    0,
                    Expiry::Permanent,
                    None,
                    1,
                ),
            );
        }
        fire_own_security_removed(&mut runner, 0);
        offered_hand_ids(&mut runner, 0)
    };

    assert_eq!(
        build(&["EX12-029", "EX12-045", "EX12-056"], false),
        vec!["EX12-048".to_string()],
        "fulfillable Assembly -6 makes EX12-048 payable (cost 5) from -3 memory"
    );
    assert!(
        build(&["EX12-029", "EX12-045"], false).is_empty(),
        "only 2 distinct Assembly materials cannot fulfill Assembly -6, so EX12-048 stays unaffordable"
    );
    assert!(
        build(&["EX12-029", "EX12-045", "EX12-056"], true).is_empty(),
        "CannotReducePlayCost blocks both Sanzomon's -2 and Assembly's -6"
    );
}

/// EX12-029 Sagomon (cost 7, DigiXros -2) at memory -6:
/// 7 - 2 = 5 is unpayable from -6 (-6 - 5 = -11 < -10), but with a legal
/// DigiXros material (EX12-045 on field is Lv.5 [SW] Digimon) the payable
/// cost is 7 - 2 - 2 = 3 (-6 - 3 = -9 >= -10), so EX12-029 must be offered
/// unless play-cost reduction is locked.
#[test]
fn ex12_045_security_removed_pick_credits_digixros_cost_reduction() {
    let build = |lock_reduction: bool| {
        let mut runner = DebugRunner::builder()
            .dsl_card(CARD_ID)
            .expect("EX12-045 YAML loads")
            .dsl_card("EX12-029")
            .expect("EX12-029 YAML loads")
            .add_card(plain_digimon("SEC", CardColor::Yellow, 3, 3000))
            .hand(0, &["EX12-029"])
            .security(0, &["SEC"])
            .memory(-6)
            .start();
        runner.place_on_field(0, CARD_ID, Some(0));
        if lock_reduction {
            runner.game.modifiers.add_player_modifier(
                0,
                PlayerModifierEntry::simple(
                    ModifierType::CannotReducePlayCost,
                    0,
                    Expiry::Permanent,
                    None,
                    1,
                ),
            );
        }
        fire_own_security_removed(&mut runner, 0);
        offered_hand_ids(&mut runner, 0)
    };

    assert_eq!(
        build(false),
        vec!["EX12-029".to_string()],
        "fulfillable DigiXros -2 makes EX12-029 payable (cost 3) from -6 memory"
    );
    assert!(
        build(true).is_empty(),
        "CannotReducePlayCost blocks DigiXros reduction too"
    );
}

/// EX12-045 playing a DigiXros Digimon (`EX12-015` Gokuumon, cost 7,
/// `<DigiXros -2>`) with `CostDelta::Reduce(2)` must stack BOTH reductions:
/// - With 1 DigiXros material: `7 - 2 - 2 = 3` memory paid (not 5).
/// - Declining DigiXros (`PASS`): `7 - 2 = 5` memory paid (not 7).
/// - At memory `-6`: stopping DigiXros is illegal (`-6 - 5 = -11 < -10`), and
///   picking 1 DigiXros material pays 3 (`-6 -> -9`) so EX12-015 enters field.
#[test]
fn ex12_045_security_removed_play_stacks_reduce_2_with_digixros_reduction() {
    use super::support::{decline_all_selections, red_sw};

    // 1. With 1 DigiXros material tucked from hand: 7 - 2 - 2 = 3 memory paid.
    {
        let mut runner = DebugRunner::builder()
            .dsl_card(CARD_ID)
            .expect("EX12-045 YAML loads")
            .dsl_card("EX12-015")
            .expect("EX12-015 YAML loads")
            .add_card(red_sw("SW-MAT", 4))
            .add_card(plain_digimon("SEC", CardColor::Yellow, 3, 3000))
            .hand(0, &["EX12-015", "SW-MAT"])
            .security(0, &["SEC"])
            .memory(10)
            .start();
        runner.place_on_field(0, CARD_ID, Some(0));
        let memory_before = runner.memory();

        fire_own_security_removed(&mut runner, 0);
        select_hand_card(&mut runner, 0, "EX12-015");
        let mat_view = runner
            .pending_selection_view()
            .expect("EX12-015 DigiXros material prompt");
        assert_eq!(mat_view.kind, SelectionKind::Material);
        assert!(mat_view.is_optional, "cost 5 is affordable from memory 10");
        select_first_non_pass(&mut runner);
        decline_all_selections(&mut runner);

        assert_eq!(
            memory_before - runner.memory(),
            3,
            "EX12-015 (cost 7) - EX12-045 (2) - DigiXros (2) must pay 3 memory"
        );
        let gokuu = runner.game.players[0]
            .battle_area
            .iter()
            .find(|p| p.top_card().card_id(&runner.game.card_data) == "EX12-015")
            .expect("EX12-015 entered field");
        assert_eq!(gokuu.card_sources.len(), 2, "1 DigiXros material stacked under EX12-015");
    }

    // 2. Declining DigiXros via PASS: 7 - 2 = 5 memory paid (not printed 7).
    {
        let mut runner = DebugRunner::builder()
            .dsl_card(CARD_ID)
            .expect("EX12-045 YAML loads")
            .dsl_card("EX12-015")
            .expect("EX12-015 YAML loads")
            .add_card(red_sw("SW-MAT", 4))
            .add_card(plain_digimon("SEC", CardColor::Yellow, 3, 3000))
            .hand(0, &["EX12-015", "SW-MAT"])
            .security(0, &["SEC"])
            .memory(10)
            .start();
        runner.place_on_field(0, CARD_ID, Some(0));
        let memory_before = runner.memory();

        fire_own_security_removed(&mut runner, 0);
        select_hand_card(&mut runner, 0, "EX12-015");
        let mat_view = runner
            .pending_selection_view()
            .expect("EX12-015 DigiXros material prompt");
        assert_eq!(mat_view.kind, SelectionKind::Material);
        assert!(mat_view.is_optional);
        runner.execute_action(0, PASS).expect("decline DigiXros");
        decline_all_selections(&mut runner);

        assert_eq!(
            memory_before - runner.memory(),
            5,
            "EX12-015 (cost 7) - EX12-045 (2) with 0 DigiXros materials must pay 5 memory"
        );
    }

    // 3. At memory -6: stop_cost = 5 is unpayable (-11 < -10) so DigiXros is
    //    mandatory, and picking 1 material pays 3 (-6 -> -9).
    {
        let mut runner = DebugRunner::builder()
            .dsl_card(CARD_ID)
            .expect("EX12-045 YAML loads")
            .dsl_card("EX12-015")
            .expect("EX12-015 YAML loads")
            .add_card(red_sw("SW-MAT", 4))
            .add_card(plain_digimon("SEC", CardColor::Yellow, 3, 3000))
            .hand(0, &["EX12-015", "SW-MAT"])
            .security(0, &["SEC"])
            .memory(-6)
            .start();
        runner.place_on_field(0, CARD_ID, Some(0));

        fire_own_security_removed(&mut runner, 0);
        select_hand_card(&mut runner, 0, "EX12-015");
        let mat_view = runner
            .pending_selection_view()
            .expect("EX12-015 DigiXros material prompt");
        assert_eq!(mat_view.kind, SelectionKind::Material);
        assert!(
            !mat_view.is_optional,
            "cost 5 is unpayable from memory -6 so player cannot decline DigiXros"
        );
        select_first_non_pass(&mut runner);
        // Check memory while EX12-015's [On Play] holds turn end (or if turn
        // already passed to P1, memory flipped from -9 to +9).
        assert_eq!(runner.memory().abs(), 9, "-6 - 3 = -9 memory");
        assert!(
            runner.game.players[0]
                .battle_area
                .iter()
                .any(|p| p.top_card().card_id(&runner.game.card_data) == "EX12-015"),
            "EX12-015 entered field from -6 memory"
        );
        decline_all_selections(&mut runner);
    }
}
