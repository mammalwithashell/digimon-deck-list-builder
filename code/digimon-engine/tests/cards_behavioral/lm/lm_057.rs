//! LM-057 Wall Training - Option, Cost 2, Red/Blue.
//!
//! # Card text (cards.json / card image)
//!
//! While you don't have [Wall Training] in the battle area, you can ignore
//! this card's color requirements.
//!
//! [Main] Reveal the top 2 cards of your deck. Add 1 red or blue card among
//! them to the hand. Return the rest to the bottom of deck. Then, place this
//! card in the battle area.
//!
//! [Main] <Delay> (By trashing this card after the placing turn, activate the
//! effect below.)
//! ・1 of your Digimon may digivolve into a red or blue Digimon card in your
//! hand for its digivolution cost. When it would digivolve by this effect,
//! reduce the cost by 2.
//!
//! Security Effect: [Security] Reveal the top 2 cards of your deck. Add 1 red
//! or blue card among them to the hand. Return the rest to the bottom of deck.
//! Then, place this card in the battle area.
//!
//! # DCGO C# reference
//! `Assets/Scripts/CardEffect/LM/Red/LM_057.cs` — `IgnoreColorConditionClass`
//! (no own battle-area permanent whose top card is NAMED [Wall Training]),
//! `SimplifiedRevealDeckTopCardsAndSelect` (1 red-or-blue card of any kind →
//! hand, rest → deck bottom) + `PlaceDelayOptionCards`, Delay
//! `SelectPermanentEffect(canNoSelect: true)` over Digimon with a legal
//! red/blue hand candidate → `DigivolveIntoHandOrTrashCard(isHand, payCost,
//! reduceCost: 2)`, Security = `AddActivateMainOptionSecurityEffect`.
//!
//! # Patterns this test covers
//! - D3 conditional Option color-requirement bypass (name-based)
//! - A2 reveal 2, add 1 matching card, bottom the rest
//! - Standard player-activated <Delay> body (PUPPETS-G009)
//! - Effect-initiated digivolve from hand with cost reduction, legal-target
//!   gated (`has_digivolve_candidate` / `can_digivolve_onto`)
//! - Inherited security mirror plus battle-area placement

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledPlayerRef,
    CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS, PLAY_HAND_START,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::{OptionPlayResult, SelectionKind, TriggerSource};

const CARD_ID: &str = "LM-057";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("LM-057 YAML must parse and compile")
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

/// Neutral (white) filler — `make_test_card` defaults to red, which would
/// match LM-057's red/blue filter.
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

fn lv4_evo(id: &str, color: CardColor, base_color: CardColor, cost: u16) -> CardData {
    let mut card = digimon(id, color, 4);
    card.evo_costs = vec![EvoCost {
        card_color: color_code(base_color),
        level: 3,
        memory_cost: cost,
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

fn park_as_delay(runner: &mut DebugRunner, index: usize) {
    let placing_turn = runner.game.turn_count;
    runner.game.player_mut(0).battle_area[index].option_state = OptionState::Delayed {
        owner: 0,
        trash_on_turn: u16::MAX,
        trigger: DelayTrigger::MainPhaseActivated,
        placed_on_turn: placing_turn,
    };
}

// ───────────────────────── Section 1: structure ─────────────────────────

#[test]
fn lm_057_is_red_blue_option_cost_2() {
    let runner = builder().start();
    let compiled = runner.compiled_card(CARD_ID).expect("registered");
    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Red, CompiledColor::Blue]
    );
    assert_eq!(compiled.cost, Some(2));
}

#[test]
fn lm_057_color_bypass_checks_name_in_your_battle_area() {
    let runner = builder().start();
    let compiled = runner.compiled_card(CARD_ID).expect("registered");
    let no_perm = compiled
        .use_requirement
        .as_ref()
        .expect("color bypass compiles as use_requirement")
        .no_permanent
        .as_ref()
        .expect("bypass is disabled by your own battle-area [Wall Training]");
    assert_eq!(no_perm.of, CompiledPlayerRef::You);
    assert_eq!(
        no_perm.predicate.name_is.as_deref(),
        Some("Wall Training"),
        "DCGO checks the top card NAME, not the card number"
    );
}

#[test]
fn lm_057_has_main_delay_and_security_clauses() {
    let runner = builder().start();
    let compiled = runner.compiled_card(CARD_ID).expect("registered");
    assert_eq!(compiled.effects.len(), 3);

    match &compiled.effects[0] {
        CompiledClause::Triggered(t) => {
            assert!(t.when.contains(&CompiledTiming::MainFromHand));
            assert!(!t.optional, "the reveal/add [Main] effect is mandatory");
        }
        other => panic!("clause 0 must be main_from_hand; got {other:?}"),
    }
    match &compiled.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
            trigger, process, ..
        }) => {
            assert_eq!(*trigger, CompiledTiming::Delayed);
            assert!(process
                .iter()
                .any(|s| matches!(s, CompiledStep::EffectInitiatedDigivolve { .. })));
        }
        other => panic!("clause 1 must be a Delay declarative; got {other:?}"),
    }
    match &compiled.effects[2] {
        CompiledClause::Triggered(t) => {
            assert_eq!(t.scope, CompiledScope::Inherited);
            assert!(t.when.contains(&CompiledTiming::OnSecurity));
            assert!(t
                .process
                .iter()
                .any(|s| matches!(s, CompiledStep::RevealTopDeck { count: 2, .. })));
            assert!(t
                .process
                .iter()
                .any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption)));
        }
        other => panic!("clause 2 must be inherited on_security; got {other:?}"),
    }
}

// ─────────────── Section 2: color-requirement bypass gating ───────────────

#[test]
fn lm_057_usable_without_red_or_blue_when_no_wall_training_in_battle_area() {
    let mut runner = builder()
        .add_card(tamer("YELLOW-TAMER", CardColor::Yellow))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .memory(10)
        .start();
    runner.place_on_field(0, "YELLOW-TAMER", Some(0));
    runner.game.enter_main_phase();
    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        1.0,
        "bypass: usable with no red/blue permanent while no [Wall Training] is in battle area"
    );
}

#[test]
fn lm_057_bypass_disabled_by_wall_training_in_battle_area() {
    let mut runner = builder()
        .add_card(tamer("YELLOW-TAMER", CardColor::Yellow))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .memory(10)
        .start();
    runner.place_on_field(0, "YELLOW-TAMER", Some(0));
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();
    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        0.0,
        "a battle-area [Wall Training] disables the bypass; no red/blue permanent → unusable"
    );
}

#[test]
fn lm_057_bypass_disabled_by_any_card_named_wall_training() {
    let mut wall_alt = option("WALL-ALT", CardColor::Red);
    wall_alt.card_name = "Wall Training".to_string();
    let mut runner = builder()
        .add_card(tamer("YELLOW-TAMER", CardColor::Yellow))
        .add_card(wall_alt)
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .memory(10)
        .start();
    runner.place_on_field(0, "YELLOW-TAMER", Some(0));
    runner.place_on_field(0, "WALL-ALT", Some(0));
    runner.game.enter_main_phase();
    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        0.0,
        "the bypass keys on the NAME [Wall Training], not on the card number"
    );
}

#[test]
fn lm_057_opponents_wall_training_does_not_disable_bypass() {
    let mut runner = builder()
        .add_card(tamer("YELLOW-TAMER", CardColor::Yellow))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .memory(10)
        .start();
    runner.place_on_field(0, "YELLOW-TAMER", Some(0));
    runner.place_on_field(1, CARD_ID, Some(1));
    runner.game.enter_main_phase();
    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        1.0,
        "only YOUR battle area is checked ('you don't have')"
    );
}

#[test]
fn lm_057_normal_color_requirement_still_works_with_wall_training_in_play() {
    let mut runner = builder()
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(tamer("BLUE-TAMER", CardColor::Blue))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .memory(10)
        .start();
    runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.place_on_field(0, "BLUE-TAMER", Some(0));
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();
    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        1.0,
        "red + blue permanents satisfy the printed (two-color) requirement normally"
    );
}

#[test]
fn lm_057_one_of_two_colors_is_not_enough_without_bypass() {
    let mut runner = builder()
        .add_card(tamer("BLUE-TAMER", CardColor::Blue))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .memory(10)
        .start();
    runner.place_on_field(0, "BLUE-TAMER", Some(0));
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();
    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        0.0,
        "a two-color Option needs both colors once the bypass is off"
    );
}

// ─────────────────────── Section 3: [Main] behavior ───────────────────────

#[test]
fn lm_057_main_offers_only_red_or_blue_and_is_mandatory() {
    let mut runner = builder()
        .add_card(option("BLUE-OPT", CardColor::Blue))
        .add_card(digimon("YELLOW-DIGI", CardColor::Yellow, 3))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "YELLOW-DIGI", "BLUE-OPT"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();

    assert!(runner.game.activate_hand_main(0, 0));
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Reveal));
    let reveal = runner.pending_selection_view().unwrap();
    assert_eq!(
        reveal.valid_action_ids.len(),
        1,
        "only the blue card (an Option — any card kind) is selectable"
    );
    assert!(!reveal.is_optional, "the add-to-hand pick is mandatory");
    assert_eq!(build_action_mask(&runner.game, 0)[PASS as usize], 0.0);

    runner
        .execute_action(0, reveal.valid_action_ids[0])
        .expect("pick resolves");
    let _ = runner.auto_resolve();

    let hand = hand_ids(&runner, 0);
    assert!(hand.iter().any(|id| id == "BLUE-OPT"));
    assert!(!hand.iter().any(|id| id == "YELLOW-DIGI"));
    assert_eq!(
        runner.game.player(0).deck[0].card_id(&runner.game.card_data),
        "YELLOW-DIGI",
        "the unchosen revealed card goes to the bottom of the deck"
    );
}

#[test]
fn lm_057_main_both_red_and_blue_revealed_offers_both() {
    let mut runner = builder()
        .add_card(option("BLUE-OPT", CardColor::Blue))
        .add_card(digimon("RED-DIGI", CardColor::Red, 3))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "RED-DIGI", "BLUE-OPT"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();

    assert!(runner.game.activate_hand_main(0, 0));
    let reveal = runner.pending_selection_view().unwrap();
    assert_eq!(
        reveal.valid_action_ids.len(),
        2,
        "red and blue revealed cards are both eligible — the player chooses"
    );
}

#[test]
fn lm_057_main_no_match_bottoms_both_and_still_places() {
    let mut runner = builder()
        .add_card(digimon("YELLOW-DIGI", CardColor::Yellow, 3))
        .add_card(digimon("GREEN-DIGI", CardColor::Green, 3))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "YELLOW-DIGI", "GREEN-DIGI"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner
        .auto_resolve()
        .expect("resolve with no eligible reveal");

    let hand = hand_ids(&runner, 0);
    assert!(!hand
        .iter()
        .any(|id| id == "YELLOW-DIGI" || id == "GREEN-DIGI"));
    let deck = &runner.game.player(0).deck;
    let bottom_two: Vec<&str> = deck[..2]
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(bottom_two.contains(&"YELLOW-DIGI") && bottom_two.contains(&"GREEN-DIGI"));
    assert!(delayed_self_in_battle_area(&runner, 0));
}

#[test]
fn lm_057_playing_main_places_it_as_delayed_option() {
    let mut runner = builder()
        .add_card(option("BLUE-OPT", CardColor::Blue))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "BLUE-OPT"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner.auto_resolve().expect("resolve placement");
    assert!(delayed_self_in_battle_area(&runner, 0));
    assert_eq!(runner.trash_size(0), 0, "LM-057 is placed, not trashed");
}

// ─────────────────────── Section 3b: [Security] ───────────────────────

#[test]
fn lm_057_security_reveals_adds_and_places_self_in_battle_area() {
    let mut runner = builder()
        .add_card(digimon("ATTACKER", CardColor::Yellow, 3))
        .add_card(digimon("RED-DIGI", CardColor::Red, 3))
        .add_card(digimon("YELLOW-DIGI", CardColor::Yellow, 3))
        .add_card(filler("FILL"))
        .security(1, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL", "YELLOW-DIGI", "RED-DIGI"])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let result = runner.attack_player(attacker, 1, false);
    assert_eq!(result, AttackResult::InProgress);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Reveal));
    let reveal = runner.pending_selection_view().unwrap();
    assert_eq!(
        reveal.valid_action_ids.len(),
        1,
        "only the red card qualifies"
    );
    runner
        .execute_action(reveal.selecting_player, reveal.valid_action_ids[0])
        .expect("security pick resolves");
    runner.auto_resolve().expect("resolve rest");

    assert!(hand_ids(&runner, 1).iter().any(|id| id == "RED-DIGI"));
    assert_eq!(
        runner.game.player(1).deck[0].card_id(&runner.game.card_data),
        "YELLOW-DIGI",
        "unchosen card to deck bottom"
    );
    assert_eq!(runner.security_count(1), 0);
    assert_eq!(
        runner.trash_size(1),
        0,
        "placed in battle area, not trashed"
    );
    assert!(delayed_self_in_battle_area(&runner, 1));
}

// ─────────────────────── Section 3c: <Delay> body ───────────────────────

fn delay_runner(hand: &[&str]) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut runner = builder()
        .add_card(digimon("RED-BASE", CardColor::Red, 3))
        .add_card(digimon("WHITE-BASE", CardColor::White, 3))
        .add_card(lv4_evo("BLUE-EVO", CardColor::Blue, CardColor::Red, 3))
        .add_card(lv4_evo("YELLOW-EVO", CardColor::Yellow, CardColor::Red, 3))
        .add_card(lv4_evo(
            "RED-WHITE-EVO",
            CardColor::Red,
            CardColor::White,
            3,
        ))
        .add_card(option("RED-OPT", CardColor::Red))
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
fn lm_057_delay_target_only_digimon_with_legal_red_or_blue_hand_candidate() {
    // BLUE-EVO can digivolve onto a red Lv3; YELLOW-EVO (wrong color) onto a red
    // Lv3 too; nothing red/blue can go onto WHITE-BASE.
    let (mut runner, delay_perm) = delay_runner(&["BLUE-EVO", "YELLOW-EVO", "RED-OPT"]);
    runner.place_on_field(0, "RED-BASE", Some(0));
    runner.place_on_field(0, "WHITE-BASE", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::DelayEffect,
        TriggerSource::Permanent(delay_perm),
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let target_pick = runner.pending_selection_view().unwrap();
    assert!(
        target_pick.is_optional,
        "'may digivolve' — target pick is declinable"
    );
    assert_eq!(build_action_mask(&runner.game, 0)[PASS as usize], 1.0);
    assert_eq!(
        target_pick.valid_action_ids.len(),
        1,
        "only RED-BASE has a legal red/blue Digimon result card in hand"
    );
}

#[test]
fn lm_057_delay_hand_pick_filters_red_or_blue_digimon_and_reduces_cost_by_2() {
    let (mut runner, delay_perm) = delay_runner(&["BLUE-EVO", "YELLOW-EVO", "RED-OPT"]);
    runner.place_on_field(0, "RED-BASE", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::DelayEffect,
        TriggerSource::Permanent(delay_perm),
    );
    runner.game.drain_effect_queue();
    let target_pick = runner.pending_selection_view().unwrap();
    runner
        .execute_action(0, target_pick.valid_action_ids[0])
        .expect("target pick");

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    let evo_pick = runner.pending_selection_view().unwrap();
    assert_eq!(
        evo_pick.valid_action_ids.len(),
        1,
        "only BLUE-EVO: YELLOW-EVO is the wrong color, RED-OPT is not a Digimon"
    );
    assert!(
        evo_pick.is_optional,
        "DCGO isOptional default — hand pick declinable"
    );

    runner
        .execute_action(0, evo_pick.valid_action_ids[0])
        .expect("evo pick");
    runner.game.drain_effect_queue();

    let base = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.is_digimon(&runner.game.card_data))
        .expect("base remains");
    assert_eq!(base.stack_size(), 2);
    assert_eq!(base.top_card().card_id(&runner.game.card_data), "BLUE-EVO");
    assert_eq!(runner.memory(), 4, "evo cost 3 reduced by 2 → pays 1");
}

#[test]
fn lm_057_delay_red_digimon_result_also_eligible() {
    let (mut runner, delay_perm) = delay_runner(&["RED-WHITE-EVO"]);
    runner.place_on_field(0, "WHITE-BASE", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::DelayEffect,
        TriggerSource::Permanent(delay_perm),
    );
    runner.game.drain_effect_queue();
    let target_pick = runner.pending_selection_view().unwrap();
    assert_eq!(target_pick.valid_action_ids.len(), 1);
    runner
        .execute_action(0, target_pick.valid_action_ids[0])
        .expect("target pick");
    let evo_pick = runner.pending_selection_view().unwrap();
    runner
        .execute_action(0, evo_pick.valid_action_ids[0])
        .expect("evo pick");
    runner.game.drain_effect_queue();

    let base = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.is_digimon(&runner.game.card_data))
        .expect("base remains");
    assert_eq!(
        base.top_card().card_id(&runner.game.card_data),
        "RED-WHITE-EVO"
    );
    assert_eq!(runner.memory(), 4);
}

#[test]
fn lm_057_delay_declining_target_does_not_digivolve() {
    let (mut runner, delay_perm) = delay_runner(&["BLUE-EVO"]);
    runner.place_on_field(0, "RED-BASE", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::DelayEffect,
        TriggerSource::Permanent(delay_perm),
    );
    runner.game.drain_effect_queue();
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    runner.execute_action(0, PASS).expect("decline");
    runner.game.drain_effect_queue();

    assert!(hand_ids(&runner, 0).iter().any(|id| id == "BLUE-EVO"));
    assert_eq!(runner.memory(), 5, "no cost paid when declined");
}

#[test]
fn lm_057_delay_is_main_phase_action_after_placing_turn() {
    let (mut runner, delay_perm) = delay_runner(&["BLUE-EVO"]);
    runner.place_on_field(0, "RED-BASE", Some(0));
    let delay_idx = delay_perm.index as usize;
    park_as_delay(&mut runner, delay_idx);
    let bit = (FIELD_EFFECT_START
        + delay_idx as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN) as usize;

    runner.game.enter_main_phase();
    assert_eq!(
        build_action_mask(&runner.game, 0)[bit],
        0.0,
        "<Delay> is not activatable on the placing turn"
    );

    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
    runner.game.set_memory(5);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[bit], 1.0, "<Delay> activation is a legal action");
    assert_eq!(mask[PASS as usize], 1.0);

    runner.game.decode_action(bit as u16, 0);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let target_pick = runner.pending_selection_view().unwrap();
    runner
        .execute_action(0, target_pick.valid_action_ids[0])
        .expect("target pick");
    let evo_pick = runner.pending_selection_view().unwrap();
    runner
        .execute_action(0, evo_pick.valid_action_ids[0])
        .expect("evo pick");
    runner.game.drain_effect_queue();

    let base = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.is_digimon(&runner.game.card_data))
        .expect("base remains");
    assert_eq!(base.top_card().card_id(&runner.game.card_data), "BLUE-EVO");
    assert_eq!(runner.memory(), 4);
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| matches!(p.option_state, OptionState::Delayed { .. })),
        "LM-057 is trashed as the <Delay> activation cost"
    );
}

/// DCGO `CanSelectPermanentCondition`: a Digimon is offered only if a red or
/// blue Digimon card in hand can digivolve onto it. RED-BASE's only hand
/// candidate here is YELLOW-EVO (off-color), so no target may be offered.
#[test]

fn lm_057_delay_target_excludes_digimon_with_only_off_color_candidate() {
    let (mut runner, delay_perm) = delay_runner(&["YELLOW-EVO"]);
    runner.place_on_field(0, "RED-BASE", Some(0));
    runner.game.enqueue_triggered(
        EffectTiming::DelayEffect,
        TriggerSource::Permanent(delay_perm),
    );
    runner.game.drain_effect_queue();
    assert_eq!(
        runner.pending_kind(),
        None,
        "no Digimon has a legal red/blue result card → no target prompt"
    );
}
