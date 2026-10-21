//! LM-058 Parkour Training - Option, Cost 2, Blue/Green.
//!
//! # Card text (cards.json / card image)
//!
//! While you don't have [Parkour Training] in the battle area, you can ignore
//! this card's color requirements.
//!
//! [Main] Reveal the top 2 cards of your deck. Add 1 blue or green card among
//! them to the hand. Return the rest to the bottom of deck. Then, place this
//! card in the battle area.
//!
//! [Main] <Delay> (By trashing this card after the placing turn, activate the
//! effect below.)
//! ・1 of your Digimon may digivolve into a blue or green Digimon card in your
//! hand for its digivolution cost. When it would digivolve by this effect,
//! reduce the cost by 2.
//!
//! Security Effect: [Security] Reveal the top 2 cards of your deck. Add 1 blue
//! or green card among them to the hand. Return the rest to the bottom of deck.
//! Then, place this card in the battle area.
//!
//! # DCGO C# reference
//! `Assets/Scripts/CardEffect/LM/Blue/LM_058.cs` — structurally identical to
//! LM_057 (Wall Training) with only the name and the two colors changed.
//! Full clause coverage of the shared "Training" mechanic lives in
//! `lm_057.rs`; this file covers the per-card deltas (colors, name-based
//! bypass, color filter positive/negative, Delay digivolve) behaviorally.
//!
//! # Patterns this test covers
//! - D3 conditional Option color-requirement bypass (name-based)
//! - A2 reveal 2, add 1 matching card, bottom the rest
//! - Standard player-activated <Delay> body with legal-target-gated
//!   effect-initiated digivolve from hand, cost -2
//! - Inherited security mirror plus battle-area placement

use digimon_dsl::compiled::{CompiledCardKind, CompiledColor};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS, PLAY_HAND_START,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::selection::{OptionPlayResult, SelectionKind, TriggerSource};

const CARD_ID: &str = "LM-058";
const NAME: &str = "Parkour Training";
const COLOR_A: CardColor = CardColor::Blue;
const COLOR_B: CardColor = CardColor::Green;
/// A color that is neither of this card's colors.
const OFF: CardColor = CardColor::Red;

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("LM-058 YAML must parse and compile")
}

fn card(id: &str, kind: CardKind, colors: Vec<CardColor>) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = kind;
    card.colors = colors;
    card
}

fn digimon(id: &str, color: CardColor, level: u8) -> CardData {
    let mut card = card(id, CardKind::Digimon, vec![color]);
    card.level = Some(level);
    card.dp = Some(3000);
    card.play_cost = 4;
    card
}

fn tamer(id: &str, color: CardColor) -> CardData {
    card(id, CardKind::Tamer, vec![color])
}

fn option(id: &str, color: CardColor) -> CardData {
    card(id, CardKind::Option, vec![color])
}

fn filler(id: &str) -> CardData {
    card(id, CardKind::Digimon, vec![CardColor::White])
}

fn color_code(color: CardColor) -> u8 {
    match color {
        CardColor::Red => 0,
        CardColor::Blue => 1,
        CardColor::Yellow => 2,
        CardColor::Green => 3,
        CardColor::White => 4,
        CardColor::Black => 5,
        CardColor::Purple => 6,
    }
}

fn lv4_evo(id: &str, color: CardColor, base_color: CardColor) -> CardData {
    let mut card = digimon(id, color, 4);
    card.evo_costs = vec![EvoCost {
        card_color: color_code(base_color),
        level: 3,
        memory_cost: 3,
    }];
    card
}

fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner
        .game
        .player(player)
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn delayed_self_in_battle_area(runner: &DebugRunner, player: u8) -> bool {
    runner
        .game
        .player(player)
        .battle_area
        .iter()
        .any(|permanent| {
            permanent.top_card().card_id(&runner.game.card_data) == CARD_ID
                && matches!(
                    permanent.option_state,
                    OptionState::Delayed {
                        trigger: DelayTrigger::MainPhaseActivated,
                        ..
                    }
                )
        })
}

// Body-only fixtures model an Option already placed by its printed Main effect.
// Keep the real placing turn: this is not permission to activate Delay early.
fn place_delayed_fixture(
    runner: &mut DebugRunner,
    owner: u8,
    card: &str,
) -> digimon_engine::permanent::PermanentHandle {
    let handle = runner.place_on_field(owner, card, Some(owner as u16));
    let placed_on_turn = runner.game.turn_count;
    runner.game.player_mut(owner).battle_area[handle.index as usize].option_state =
        digimon_engine::permanent::OptionState::Delayed {
            owner,
            trash_on_turn: u16::MAX,
            trigger: digimon_engine::enums::DelayTrigger::MainPhaseActivated,
            placed_on_turn,
        };
    handle
}

#[test]
fn lm_058_is_blue_green_option_cost_2_with_name_bypass() {
    let runner = builder().start();
    let compiled = runner.compiled_card(CARD_ID).expect("registered");
    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Blue, CompiledColor::Green]
    );
    assert_eq!(compiled.cost, Some(2));
    let no_perm = compiled
        .use_requirement
        .as_ref()
        .and_then(|u| u.no_permanent.as_ref())
        .expect("name-gated color bypass");
    assert_eq!(no_perm.predicate.name_is.as_deref(), Some(NAME));
}

#[test]
fn lm_058_bypass_usable_without_matching_colors_when_none_in_battle_area() {
    let mut runner = builder()
        .add_card(tamer("OFF-TAMER", OFF))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .memory(10)
        .start();
    runner.place_on_field(0, "OFF-TAMER", Some(0));
    runner.game.enter_main_phase();
    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        1.0
    );
}

#[test]
fn lm_058_bypass_disabled_by_same_name_in_battle_area() {
    let mut runner = builder()
        .add_card(tamer("OFF-TAMER", OFF))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .memory(10)
        .start();
    runner.place_on_field(0, "OFF-TAMER", Some(0));
    place_delayed_fixture(&mut runner, 0, CARD_ID);
    runner.game.enter_main_phase();
    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        0.0
    );
}

#[test]
fn lm_058_main_offers_both_matching_colors_mandatorily() {
    let mut runner = builder()
        .add_card(option("A-OPT", COLOR_A))
        .add_card(digimon("B-DIGI", COLOR_B, 3))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "B-DIGI", "A-OPT"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();
    assert!(runner.game.activate_hand_main(0, 0));
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Reveal));
    let reveal = runner.pending_selection_view().unwrap();
    assert_eq!(
        reveal.valid_action_ids.len(),
        2,
        "blue and green both qualify"
    );
    assert!(!reveal.is_optional);
}

#[test]
fn lm_058_main_off_color_not_offered_and_bottomed() {
    let mut runner = builder()
        .add_card(option("A-OPT", COLOR_A))
        .add_card(digimon("OFF-DIGI", OFF, 3))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "OFF-DIGI", "A-OPT"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Reveal));
    let reveal = runner.pending_selection_view().unwrap();
    assert_eq!(
        reveal.valid_action_ids.len(),
        1,
        "off-color card is not eligible"
    );
    runner
        .execute_action(0, reveal.valid_action_ids[0])
        .expect("pick");
    let _ = runner.auto_resolve();
    let hand = hand_ids(&runner, 0);
    assert!(hand.iter().any(|id| id == "A-OPT"));
    assert!(!hand.iter().any(|id| id == "OFF-DIGI"));
    assert_eq!(
        runner.game.player(0).deck[0].card_id(&runner.game.card_data),
        "OFF-DIGI"
    );
    assert!(delayed_self_in_battle_area(&runner, 0));
}

#[test]
fn lm_058_playing_main_places_it_as_delayed_option() {
    let mut runner = builder()
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner.auto_resolve().expect("resolve");
    assert!(delayed_self_in_battle_area(&runner, 0));
    assert_eq!(runner.trash_size(0), 0);
}

#[test]
fn lm_058_security_adds_matching_card_and_places_self() {
    let mut runner = builder()
        .add_card(digimon("ATTACKER", OFF, 3))
        .add_card(digimon("B-DIGI", COLOR_B, 3))
        .add_card(digimon("OFF-DIGI", OFF, 3))
        .add_card(filler("FILL"))
        .security(1, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL", "OFF-DIGI", "B-DIGI"])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    assert_eq!(
        runner.attack_player(attacker, 1, false),
        AttackResult::InProgress
    );
    let reveal = runner.pending_selection_view().unwrap();
    assert_eq!(reveal.valid_action_ids.len(), 1);
    runner
        .execute_action(reveal.selecting_player, reveal.valid_action_ids[0])
        .expect("pick");
    runner.auto_resolve().expect("resolve");
    assert!(hand_ids(&runner, 1).iter().any(|id| id == "B-DIGI"));
    assert_eq!(runner.trash_size(1), 0);
    assert!(delayed_self_in_battle_area(&runner, 1));
}

fn delay_runner(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = builder()
        .add_card(digimon("WHITE-BASE", CardColor::White, 3))
        .add_card(digimon("OFF-BASE", OFF, 3))
        .add_card(lv4_evo("A-EVO", COLOR_A, CardColor::White))
        .add_card(lv4_evo("B-EVO", COLOR_B, CardColor::White))
        .add_card(lv4_evo("OFF-EVO", OFF, OFF))
        .add_card(option("A-OPT", COLOR_A))
        .add_card(filler("FILL"))
        .hand(0, hand)
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    let delay_perm = place_delayed_fixture(&mut runner, 0, CARD_ID);
    (runner, delay_perm)
}

#[test]
fn lm_058_delay_targets_and_hand_pick_filtered_cost_reduced_by_2() {
    // OFF-EVO (off-color, white-incompatible) and A-OPT (Option) are in hand
    // but can never be picked; only B-EVO can digivolve onto WHITE-BASE.
    let (mut runner, delay_perm) = delay_runner(&["B-EVO", "OFF-EVO", "A-OPT"]);
    runner.place_on_field(0, "WHITE-BASE", Some(0));
    runner.game.enqueue_triggered(
        EffectTiming::DelayEffect,
        TriggerSource::Permanent(delay_perm),
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let target_pick = runner.pending_selection_view().unwrap();
    assert!(target_pick.is_optional);
    assert_eq!(target_pick.valid_action_ids.len(), 1);
    runner
        .execute_action(0, target_pick.valid_action_ids[0])
        .expect("target");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    let evo_pick = runner.pending_selection_view().unwrap();
    assert_eq!(
        evo_pick.valid_action_ids.len(),
        1,
        "only the matching-color Digimon card (not the Option, not off-color)"
    );
    runner
        .execute_action(0, evo_pick.valid_action_ids[0])
        .expect("evo");
    runner.game.drain_effect_queue();

    let base = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| {
            p.is_digimon(&runner.game.card_data)
                && p.top_card().card_id(&runner.game.card_data) == "B-EVO"
        })
        .expect("WHITE-BASE digivolved into B-EVO");
    assert_eq!(base.stack_size(), 2);
    assert_eq!(runner.memory(), 4, "evo cost 3 reduced by 2 → pays 1");
}

#[test]
fn lm_058_delay_color_a_result_eligible_and_decline_is_legal() {
    let (mut runner, delay_perm) = delay_runner(&["A-EVO"]);
    runner.place_on_field(0, "WHITE-BASE", Some(0));
    runner.game.enqueue_triggered(
        EffectTiming::DelayEffect,
        TriggerSource::Permanent(delay_perm),
    );
    runner.game.drain_effect_queue();
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let target_pick = runner.pending_selection_view().unwrap();
    assert_eq!(target_pick.valid_action_ids.len(), 1);
    assert_eq!(build_action_mask(&runner.game, 0)[PASS as usize], 1.0);
    runner.execute_action(0, PASS).expect("decline");
    runner.game.drain_effect_queue();
    assert!(hand_ids(&runner, 0).iter().any(|id| id == "A-EVO"));
    assert_eq!(runner.memory(), 5);
}

/// DCGO `CanSelectPermanentCondition`: a Digimon is offered only if a
/// blue/green Digimon card in hand can digivolve onto it. OFF-BASE's only hand
/// candidate is off-color, so it must not be offered.
#[test]

fn lm_058_delay_target_excludes_digimon_with_only_off_color_candidate() {
    let (mut runner, delay_perm) = delay_runner(&["OFF-EVO"]);
    runner.place_on_field(0, "OFF-BASE", Some(0));
    runner.game.enqueue_triggered(
        EffectTiming::DelayEffect,
        TriggerSource::Permanent(delay_perm),
    );
    runner.game.drain_effect_queue();
    assert_eq!(
        runner.pending_kind(),
        None,
        "no Digimon has a legal blue/green result card → no target prompt"
    );
}

/// Actual paid Option placement, then a later Main activation (not synthetic Delay timing).
fn audited_delay(two_routes: bool) -> DebugRunner {
    let mut base = digimon("AUDIT-BASE", CardColor::Green, 3);
    base.colors.push(CardColor::Blue);
    let mut evo = lv4_evo("AUDIT-EVO", CardColor::Blue, CardColor::Green);
    if two_routes {
        evo.evo_costs.push(EvoCost {
            card_color: color_code(CardColor::Blue),
            level: 3,
            memory_cost: 4,
        });
    }
    let mut r = builder()
        .add_card(base)
        .add_card(evo)
        .add_card(filler("AUDIT-FILL"))
        .hand(0, &[CARD_ID, "AUDIT-EVO"])
        .deck(0, &["AUDIT-FILL"; 12])
        .deck(1, &["AUDIT-FILL"; 12])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 8, "the printed Option cost is paid");
    assert!(delayed_self_in_battle_area(&r, 0));
    r.place_on_field(0, "AUDIT-BASE", Some(0));
    let index = r
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == CARD_ID)
        .unwrap();
    let action =
        FIELD_EFFECT_START + index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN;
    assert_eq!(
        build_action_mask(&r.game, 0)[action as usize],
        0.0,
        "placing-turn activation is forbidden"
    );
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    r.game.enter_main_phase();
    r.game.set_memory(5);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    assert_eq!(
        r.trash_size(0),
        1,
        "Delay trashes the Option before selection"
    );
    r
}

#[test]
fn lm_058_real_delay_target_decline_ends_without_hand_prompt() {
    let mut r = audited_delay(false);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OwnField));
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
    r.execute_action(0, PASS).unwrap();
    assert!(
        r.game.pending_selection.is_none(),
        "declining the host must end the effect"
    );
    assert_eq!(r.memory(), 5);
    assert!(hand_ids(&r, 0).iter().any(|id| id == "AUDIT-EVO"));
    assert_eq!(r.game.player(0).battle_area[0].stack_size(), 1);
}

#[test]
fn lm_058_real_delay_hand_decline_ends_without_payment() {
    let mut r = audited_delay(false);
    let pick = r.pending_selection_view().unwrap();
    r.execute_action(0, pick.valid_action_ids[0]).unwrap();
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
    r.execute_action(0, PASS).unwrap();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), 5);
    assert!(hand_ids(&r, 0).iter().any(|id| id == "AUDIT-EVO"));
}

#[test]
fn lm_058_real_delay_cloned_route_choice_keeps_both_reduced_costs() {
    let mut r = audited_delay(true);
    for _ in 0..2 {
        let pick = r.pending_selection_view().unwrap();
        r.execute_action(0, pick.valid_action_ids[0]).unwrap();
    }
    let routes = r
        .pending_selection_view()
        .expect("two printed evolution routes remain choices");
    assert_eq!(routes.valid_action_ids.len(), 2);
    assert!(!routes.is_optional);
    let actions = routes.valid_action_ids.clone();
    let saved = r.game.clone();
    let mut memories = Vec::new();
    for action in actions {
        r.game = saved.clone();
        assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
        r.execute_action(0, action).unwrap();
        assert!(r.game.pending_selection.is_none());
        memories.push(r.memory());
        assert_eq!(
            r.game.player(0).battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            "AUDIT-EVO"
        );
        assert_eq!(r.game.player(0).battle_area[0].stack_size(), 2);
    }
    memories.sort();
    assert_eq!(
        memories,
        vec![3, 4],
        "printed costs 3/4 reduced by 2 must pay 1/2"
    );
}

#[test]
fn lm_058_exact_name_bypass_checks_owner_battle_area_only() {
    for (name, owner, expected) in [
        (NAME, 0, 0.0),
        ("Parkour Training Plus", 0, 1.0),
        (NAME, 1, 1.0),
    ] {
        let mut same = option("OTHER-PRINTING", OFF);
        same.card_name = name.to_string();
        let mut r = builder()
            .add_card(same)
            .add_card(filler("FILL"))
            .hand(0, &[CARD_ID])
            .deck(0, &["FILL"; 6])
            .memory(10)
            .start();
        r.place_on_field(owner, "OTHER-PRINTING", Some(0));
        r.game.enter_main_phase();
        assert_eq!(
            build_action_mask(&r.game, 0)[PLAY_HAND_START as usize],
            expected,
            "{name}, owner {owner}"
        );
    }
}

#[test]
fn lm_058_delay_excludes_incompatible_matching_color_and_wrong_host() {
    let (mut r, delay) = delay_runner(&["A-EVO", "B-EVO", "A-OPT"]);
    r.place_on_field(0, "WHITE-BASE", Some(0));
    r.place_on_field(0, "OFF-BASE", Some(0));
    r.game
        .enqueue_triggered(EffectTiming::DelayEffect, TriggerSource::Permanent(delay));
    r.game.drain_effect_queue();
    let target = r.pending_selection_view().unwrap();
    assert_eq!(
        target.valid_action_ids.len(),
        1,
        "only white host has a legal result"
    );
    r.execute_action(0, target.valid_action_ids[0]).unwrap();
    let hand = r.pending_selection_view().unwrap();
    assert_eq!(
        hand.valid_action_ids.len(),
        2,
        "both blue and green Digimon are eligible; Option is not"
    );
    assert!(hand.is_optional);
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
}

#[test]
fn lm_058_main_zero_one_two_card_decks_resolve_without_fabricating_cards() {
    for deck in [vec![], vec!["GREEN"], vec!["OFF"], vec!["OFF", "GREEN"]] {
        let mut r = builder()
            .add_card(digimon("GREEN", COLOR_B, 3))
            .add_card(digimon("OFF", OFF, 3))
            .hand(0, &[CARD_ID])
            .deck(0, &deck)
            .memory(10)
            .start();
        r.game.enter_main_phase();
        assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
        r.game.decode_action(PLAY_HAND_START, 0);
        if deck.contains(&"GREEN") {
            let pick = r
                .pending_selection_view()
                .expect("eligible reveal must be selected");
            assert_eq!(r.pending_kind(), Some(SelectionKind::Reveal));
            assert!(!pick.is_optional);
            assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
            r.execute_action(0, pick.valid_action_ids[0]).unwrap();
        }
        r.auto_resolve().unwrap();
        assert!(delayed_self_in_battle_area(&r, 0));
        assert_eq!(
            r.game.player(0).hand.len(),
            usize::from(deck.contains(&"GREEN"))
        );
        assert_eq!(
            r.game.player(0).deck.len(),
            usize::from(deck.contains(&"OFF"))
        );
    }
}

#[test]
fn lm_058_no_match_bottom_order_is_a_real_choice_and_clone_safe() {
    let mut r = builder()
        .add_card(option("OFF-A", OFF))
        .add_card(option("OFF-B", OFF))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "OFF-A", "OFF-B"])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    let pick = r.pending_selection_view().unwrap();
    assert_eq!(pick.valid_action_ids.len(), 2);
    assert!(!pick.is_optional);
    let actions = pick.valid_action_ids.clone();
    let saved = r.game.clone();
    let mut outcomes = Vec::new();
    for action in actions {
        r.game = saved.clone();
        assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
        r.execute_action(0, action).unwrap();
        r.auto_resolve().unwrap();
        outcomes.push(
            r.game
                .player(0)
                .deck
                .iter()
                .map(|c| c.card_id(&r.game.card_data).to_string())
                .collect::<Vec<_>>(),
        );
        assert!(delayed_self_in_battle_area(&r, 0));
        assert_eq!(r.game.player(0).hand.len(), 0);
    }
    outcomes.sort();
    assert_eq!(
        outcomes,
        vec![
            vec!["OFF-A", "OFF-B", "FILL"],
            vec!["OFF-B", "OFF-A", "FILL"]
        ]
    );
}

#[test]
fn lm_058_delay_hand_rejects_matching_color_with_incompatible_evolution_requirement() {
    let mut r = builder()
        .add_card(digimon("BASE", CardColor::White, 3))
        .add_card(lv4_evo("LEGAL", COLOR_A, CardColor::White))
        .add_card(lv4_evo("ILLEGAL", COLOR_B, OFF))
        .add_card(filler("FILL"))
        .hand(0, &["LEGAL", "ILLEGAL"])
        .deck(0, &["FILL"; 6])
        .memory(5)
        .start();
    let delay = place_delayed_fixture(&mut r, 0, CARD_ID);
    r.place_on_field(0, "BASE", Some(0));
    r.game
        .enqueue_triggered(EffectTiming::DelayEffect, TriggerSource::Permanent(delay));
    r.game.drain_effect_queue();
    let host = r.pending_selection_view().unwrap();
    assert_eq!(host.valid_action_ids.len(), 1);
    r.execute_action(0, host.valid_action_ids[0]).unwrap();
    let hand = r.pending_selection_view().unwrap();
    assert_eq!(
        hand.valid_action_ids.len(),
        1,
        "green color alone cannot override required red base"
    );
    r.execute_action(0, hand.valid_action_ids[0]).unwrap();
    assert!(hand_ids(&r, 0).iter().any(|id| id == "ILLEGAL"));
    assert_eq!(r.memory(), 4);
}

// ─────── Task 7 re-audit (2026-10-21): printed BT20 route choice ───────
//
// G-ENGINE-EFFECT-DIGIVOLVE-COST-CHOICE (RESOLVED 2026-10-10) reclassified this
// card on the Examon pair Groundramon (BT20-042, green/red — green
// qualifies) on Coredramon (BT20-040): named-Coredramon route 3 and level-4
// colour routes 4, reduced by 2 → the player chooses paying 1 or 2.
#[test]
fn lm_058_real_delay_groundramon_on_coredramon_offers_reduced_1_and_2() {
    let mut r = builder()
        .dsl_card("BT20-040")
        .expect("Coredramon YAML")
        .dsl_card("BT20-042")
        .expect("Groundramon YAML")
        .add_card(filler("AUDIT-FILL"))
        .hand(0, &[CARD_ID, "BT20-042"])
        .deck(0, &["AUDIT-FILL"; 12])
        .deck(1, &["AUDIT-FILL"; 12])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    r.auto_resolve().unwrap();
    assert!(delayed_self_in_battle_area(&r, 0));
    r.place_on_field(0, "BT20-040", Some(0));
    let index = r
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == CARD_ID)
        .unwrap();
    let action =
        FIELD_EFFECT_START + index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN;
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    r.game.enter_main_phase();
    r.game.set_memory(5);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    for kind in [SelectionKind::OwnField, SelectionKind::Hand] {
        assert_eq!(r.pending_kind(), Some(kind));
        let v = r.pending_selection_view().unwrap();
        let pick = *v.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
        assert_eq!(build_action_mask(&r.game, 0)[pick as usize], 1.0);
        r.execute_action(0, pick).unwrap();
    }
    let routes = r
        .pending_selection_view()
        .expect("named 3 and colour 4 routes stay a player choice");
    assert!(!routes.is_optional);
    assert_eq!(routes.valid_action_ids.len(), 2);
    let saved = r.game.clone();
    let mut memories = Vec::new();
    for a in routes.valid_action_ids.clone() {
        r.game = saved.clone();
        assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
        r.execute_action(0, a).unwrap();
        r.auto_resolve().unwrap();
        assert!(r
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "BT20-042"
                && p.stack_size() == 2));
        memories.push(r.memory());
    }
    memories.sort();
    assert_eq!(memories, vec![3, 4], "pay 4-2=2 or 3-2=1 from 5 memory");
}
