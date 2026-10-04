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
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
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
    runner.place_on_field(0, CARD_ID, Some(0));
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
    let delay_perm = runner.place_on_field(0, CARD_ID, Some(0));
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
