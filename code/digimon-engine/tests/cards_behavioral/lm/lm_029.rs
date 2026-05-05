//! LM-029 Yellow Scramble - Option, Cost 2, Yellow.
//!
//! # Card text (cards.json)
//!
//! [Main] 1 of your yellow Digimon may digivolve into a yellow Digimon card in
//! the hand with the digivolution cost reduced by 3. Then, place this card in
//! the battle area.
//!
//! [Start of Your Turn] If your opponent has a Digimon, <Delay> (By trashing
//! this card after the placing turn, activate the effect below.)
//! Return 1 yellow Digimon card from your trash to the top of the deck. Then,
//! if you don't have a Digimon, you may play 1 yellow Digimon card with 2000 DP
//! or less from your trash without paying the cost.
//!
//! Inherited: Security Effect [Security] You may play 1 yellow Digimon card
//! with 2000 DP or less from your trash without paying the cost. Then, add this
//! card to the hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/LM/Yellow/LM_029.cs
//!
//! # Patterns this test covers
//! - Main option effect-initiated digivolve with cost reduction.
//! - PendingSelection/action-mask visibility for field and hand choices.
//! - Security mandatory add-this-option-to-hand tail.
//! - Blocked optional security trash play and StartOfYourTurn Scramble Delay body
//!   documented with ignored tests.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

const YAML: &str = include_str!("../../../cards/lm/LM-029.yaml");

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn digimon(id: &str, color: CardColor, level: u8, dp: i32, play_cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = play_cost;
    card
}

fn yellow_digimon(id: &str, level: u8, dp: i32, play_cost: u16) -> CardData {
    digimon(id, CardColor::Yellow, level, dp, play_cost)
}

fn yellow_evo(id: &str) -> CardData {
    let mut card = yellow_digimon(id, 4, 5000, 5);
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Yellow as u8,
        level: 3,
        memory_cost: 4,
    }];
    card
}

fn lm_029_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-029 YAML must parse and compile")
        .memory(10)
        .start()
}

#[test]
fn lm_029_yaml_parses_and_compiles() {
    let _runner = lm_029_runner();
}

#[test]
fn lm_029_is_yellow_option_cost_2() {
    let runner = lm_029_runner();
    let compiled = runner.compiled_card("LM-029").expect("LM-029 compiled");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert_eq!(compiled.cost, Some(2));
}

#[test]
fn lm_029_has_supported_main_and_security_clauses_without_raw_rust() {
    let runner = lm_029_runner();
    let compiled = runner.compiled_card("LM-029").expect("LM-029 compiled");

    assert_eq!(
        compiled.effects.len(),
        2,
        "LM-029 ships only the supported Main and Security clauses; Delay body is blocked"
    );
    assert!(
        compiled.effects.iter().all(|clause| !matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::RawRust { .. })
        )),
        "LM-029 must not use raw_rust placeholders"
    );

    let main = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("Main clause must exist");
    assert_eq!(main.scope, CompiledScope::FaceUp);
    assert!(
        main.optional,
        "printed Main text says the yellow Digimon may digivolve"
    );

    let security = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("Security clause must exist");
    assert_eq!(security.scope, CompiledScope::Inherited);
    assert!(
        !security.optional,
        "the supported Security slice is the mandatory add-this-option-to-hand tail"
    );
}

#[test]
fn lm_029_main_exposes_yellow_field_then_yellow_hand_choices_in_mask() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-029 YAML parses")
        .add_card(yellow_digimon("YELLOW-BASE", 3, 2000, 3))
        .add_card(digimon("BLUE-BASE", CardColor::Blue, 3, 2000, 3))
        .add_card(yellow_evo("YELLOW-EVO"))
        .add_card(digimon("BLUE-HAND", CardColor::Blue, 4, 5000, 5))
        .add_card(filler("FILL"))
        .hand(0, &["LM-029", "YELLOW-EVO", "BLUE-HAND"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "YELLOW-BASE", Some(0));
    runner.place_on_field(0, "BLUE-BASE", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));

    let field_view = runner
        .pending_selection_view()
        .expect("Main effect must ask which yellow Digimon digivolves");
    assert_eq!(field_view.kind, SelectionKind::OwnField);
    assert!(
        runner.pending_is_optional(),
        "field selection must expose PASS for the printed may"
    );
    assert_eq!(
        field_view.valid_action_ids.len(),
        1,
        "only the yellow field Digimon should be selectable"
    );
    let mask = build_action_mask(&runner.game, field_view.selecting_player);
    assert_eq!(mask[field_view.valid_action_ids[0] as usize], 1.0);
    runner
        .execute_action(field_view.selecting_player, field_view.valid_action_ids[0])
        .expect("select yellow field Digimon");

    let hand_view = runner
        .pending_selection_view()
        .expect("Main effect must ask which yellow hand card to digivolve into");
    assert_eq!(hand_view.kind, SelectionKind::Hand);
    assert_eq!(
        hand_view.valid_action_ids.len(),
        1,
        "only the yellow hand Digimon should be selectable"
    );
    let mask = build_action_mask(&runner.game, hand_view.selecting_player);
    assert_eq!(mask[hand_view.valid_action_ids[0] as usize], 1.0);
    runner
        .execute_action(hand_view.selecting_player, hand_view.valid_action_ids[0])
        .expect("select yellow hand Digimon");
    runner.auto_resolve().expect("finish digivolve");

    assert_eq!(
        runner.memory(),
        9,
        "evo cost 4 reduced by 3 should pay 1 memory"
    );
    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .all(|card| card.card_id(&runner.game.card_data) != "YELLOW-EVO"),
        "selected yellow evo card must leave hand"
    );
    let evolved = &runner.game.player(0).battle_area[base.index as usize];
    assert_eq!(
        evolved.top_card().card_id(&runner.game.card_data),
        "YELLOW-EVO"
    );
}

#[test]
fn lm_029_main_decline_leaves_field_and_hand_unchanged() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-029 YAML parses")
        .add_card(yellow_digimon("YELLOW-BASE-DECLINE", 3, 2000, 3))
        .add_card(yellow_evo("YELLOW-EVO-DECLINE"))
        .add_card(filler("FILL"))
        .hand(0, &["LM-029", "YELLOW-EVO-DECLINE"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    runner.place_on_field(0, "YELLOW-BASE-DECLINE", Some(0));
    let hand_before = runner.hand_size(0);
    let stack_before = runner.game.player(0).battle_area[0].card_sources.len();

    assert!(runner.game.activate_hand_main(0, 0));
    assert!(runner.pending_is_optional());
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline optional Main digivolve");

    assert_eq!(runner.hand_size(0), hand_before);
    assert_eq!(
        runner.game.player(0).battle_area[0].card_sources.len(),
        stack_before
    );
}

#[test]
fn lm_029_security_adds_this_option_to_hand_without_active_trash_play() {
    let mut attacker = filler("ATTACKER");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-029 YAML parses")
        .add_card(attacker)
        .add_card(yellow_digimon("YELLOW-SMALL", 3, 2000, 3))
        .add_card(yellow_digimon("YELLOW-LARGE", 5, 8000, 8))
        .add_card(digimon("BLUE-SMALL", CardColor::Blue, 3, 2000, 3))
        .add_card(filler("FILL"))
        .security(1, &["LM-029"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["YELLOW-SMALL", "YELLOW-LARGE", "BLUE-SMALL", "FILL"])
        .memory(0)
        .start();

    let yellow = runner.game.players[1].deck.remove(0);
    let large = runner.game.players[1].deck.remove(0);
    let blue = runner.game.players[1].deck.remove(0);
    runner.game.players[1].trash.push(yellow);
    runner.game.players[1].trash.push(large);
    runner.game.players[1].trash.push(blue);
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let result = runner.attack_player(attacker, 1, false);

    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "supported Security tail resolves without exposing the omitted optional trash play"
    );
    assert_eq!(
        runner.pending_kind(),
        None,
        "LM-029 must not expose an approximated trash-selection prompt while DP filtering is unsupported"
    );

    assert_eq!(runner.security_count(1), 0, "LM-029 left security");
    assert!(
        runner
            .game
            .player(1)
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "LM-029"),
        "LM-029 must be added to hand after Security effect"
    );
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "optional trash play is omitted until the DP filter and mandatory tail can be represented faithfully"
    );
    assert!(
        runner
            .game
            .player(1)
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BLUE-SMALL"),
        "non-yellow trash card must remain in trash"
    );
    assert!(runner
        .game
        .player(1)
        .trash
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "YELLOW-LARGE"));
}

#[test]
#[ignore = "pending: G-CARD-PRED-DP-LTE + G-OPTIONAL-SELECTION-CONTINUE-TAIL — security optional trash play needs enforced DP filtering and a declined-selection mandatory tail"]
fn lm_029_security_exposes_optional_trash_choice_and_plays_only_yellow_digimon_dp_2000_or_less() {
    let mut attacker = filler("ATTACKER-DECLINE");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-029 YAML parses")
        .add_card(attacker)
        .add_card(yellow_digimon("YELLOW-SMALL-DECLINE", 3, 2000, 3))
        .add_card(filler("FILL"))
        .security(1, &["LM-029"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["YELLOW-SMALL-DECLINE", "FILL"])
        .memory(0)
        .start();

    let yellow = runner.game.players[1].deck.remove(0);
    runner.game.players[1].trash.push(yellow);
    let attacker = runner.place_on_field(0, "ATTACKER-DECLINE", Some(0));

    let _ = runner.attack_player(attacker, 1, false);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Trash));
    runner
        .execute_action(1, digimon_engine::action::space::PASS)
        .expect("decline optional Security play");
    runner.auto_resolve().expect("finish security effect");

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "declining the may play must not play a Digimon"
    );
    assert_eq!(
        runner.trash_size(1),
        1,
        "declined yellow Digimon remains in trash"
    );
    assert!(
        runner
            .game
            .player(1)
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "LM-029"),
        "LM-029 is still added to hand after declining the optional play"
    );
}

#[test]
#[ignore = "pending: G-ZONE-TRASH-TO-DECK-TOP + G-CARD-PRED-DP-LTE — Scramble Delay body cannot be expressed faithfully in YAML yet"]
fn lm_029_delay_fires_at_start_of_your_turn_and_returns_yellow_trash_to_deck_top() {
    unimplemented!("blocked until native trash-to-deck-top and card DP predicates are available");
}

#[test]
#[ignore = "pending: G-ZONE-TRASH-TO-DECK-TOP + G-CARD-PRED-DP-LTE — Scramble Delay body cannot be expressed faithfully in YAML yet"]
fn lm_029_delay_then_optionally_plays_small_yellow_digimon_if_you_have_no_digimon() {
    unimplemented!("blocked until the complete Scramble Delay body can be authored natively");
}

#[test]
#[ignore = "pending: G-SCRAMBLE-DELAY-BODY — omitting kind: delay also blocks tested Main placement as a Delay option"]
fn lm_029_main_places_this_card_in_battle_area_after_resolution() {
    unimplemented!("blocked until LM-029 can include a faithful native Delay clause");
}
