//! P-156 Future Potential! - Option, Cost 2, Black.
//!
//! cards.json evidence:
//! - "While you have a Tamer, you can ignore this card's color requirements."
//! - "[Main] Choose 1 Tamer. You may play 1 Digimon card with the same color as
//!   the chosen Tamer with a play cost of 3 or less from your hand or trash
//!   without paying the cost."
//! - "Security Effect [Security] You may play 1 Tamer card from your hand
//!   without paying the cost. Then, add this card to your hand."
//!
//! Supported faithfully here:
//! - Conditional Option color bypass via top-level `use_requirement`.
//! - Main chosen-Tamer color binding with visible hand/trash choice and free
//!   play of an eligible Digimon.
//! - Mandatory Security tail that adds the currently resolving Option to hand.
//!
//! Omitted rather than approximated:
//! - Security optional Tamer play before the mandatory add-to-hand tail.

use std::path::Path;

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use digimon_engine::effect_context::EffectReadContext;
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::selection::OptionPlayResult;

fn yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/p/P-156.yaml"))
        .expect("P-156 YAML must exist at cards/p/P-156.yaml")
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML must parse and compile")
        .memory(10)
        .start()
}

fn filler(id: &str) -> digimon_engine::CardData {
    make_test_card(id, id)
}

fn tamer(id: &str, color: CardColor) -> digimon_engine::CardData {
    let mut card = make_test_card(id, "Color Tamer");
    card.card_kind = CardKind::Tamer;
    card.colors = vec![color];
    card.play_cost = 3;
    card
}

fn digimon(id: &str, color: CardColor, cost: u16) -> digimon_engine::CardData {
    let mut card = make_test_card(id, "Color Digimon");
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = cost;
    card
}

fn zone_contains_card(runner: &DebugRunner, player: PlayerId, zone: Zone, card_id: &str) -> bool {
    let player = runner.game.player(player);
    let cards = match zone {
        Zone::Hand => &player.hand,
        Zone::Trash => &player.trash,
    };
    cards.iter().any(|card| {
        runner
            .game
            .card_data_for_handle(card.handle())
            .is_some_and(|data| data.card_id == card_id)
    })
}

enum Zone {
    Hand,
    Trash,
}

fn battle_area_contains_card(runner: &DebugRunner, player: PlayerId, card_id: &str) -> bool {
    runner.game.player(player).battle_area.iter().any(|perm| {
        runner
            .game
            .card_data_for_handle(perm.top_card().handle())
            .is_some_and(|data| data.card_id == card_id)
    })
}

#[test]
fn p_156_yaml_parses_and_compiles() {
    let _runner = runner();
}

#[test]
fn p_156_is_black_option_cost_2_with_tamer_use_requirement() {
    let runner = runner();
    let compiled = runner.compiled_card("P-156").expect("compiled card");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(2));
    assert!(
        compiled.use_requirement.is_some(),
        "P-156 must encode the printed Tamer-based color-bypass condition as an Option use requirement"
    );
}

#[test]
fn p_156_tamer_use_requirement_matches_tamer_on_field() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML parses")
        .add_card(tamer("YELLOW-TAMER", CardColor::Yellow))
        .add_card(filler("FILL"))
        .hand(0, &["P-156"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "YELLOW-TAMER", Some(0));
    let source = runner.game.players[0].hand[0].handle();
    let ctx = EffectReadContext::new(&runner.game, source, None, 0);
    let use_req = runner
        .compiled_card("P-156")
        .expect("compiled card")
        .use_requirement
        .as_ref()
        .expect("P-156 use requirement");

    assert!(
        eval_predicate(use_req, &ctx, PredicateSubject::None),
        "having any Tamer should satisfy the printed color-bypass condition"
    );
}

#[test]
fn p_156_use_requirement_does_not_match_without_tamer() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-156"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    let source = runner.game.players[0].hand[0].handle();
    let ctx = EffectReadContext::new(&runner.game, source, None, 0);
    let use_req = runner
        .compiled_card("P-156")
        .expect("compiled card")
        .use_requirement
        .as_ref()
        .expect("P-156 use requirement");

    assert!(
        !eval_predicate(use_req, &ctx, PredicateSubject::None),
        "without a Tamer, the color-bypass use requirement must not be satisfied"
    );
}

#[test]
fn p_156_security_tail_adds_this_option_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML parses")
        .add_card(digimon("ATTACKER", CardColor::Red, 3))
        .security(1, &["P-156"])
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let result = runner.attack_player(attacker, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(runner.security_count(1), 0);
    assert!(zone_contains_card(&runner, 1, Zone::Hand, "P-156"));
    assert!(!zone_contains_card(&runner, 1, Zone::Trash, "P-156"));
}

#[test]
fn p_156_has_main_and_security_clauses() {
    let runner = runner();
    let compiled = runner.compiled_card("P-156").expect("compiled card");

    let has_main = compiled.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(triggered) => triggered.when.contains(&CompiledTiming::Main),
        _ => false,
    });

    assert!(has_main, "P-156 must expose its printed Main effect");
    assert!(
        compiled.effects.iter().any(|clause| match clause {
            CompiledClause::Triggered(triggered) => {
                triggered.when.contains(&CompiledTiming::OnSecurity)
            }
            _ => false,
        }),
        "P-156 must expose its Security effect"
    );
}

#[test]
fn p_156_main_chooses_tamer_then_filters_hand_or_trash_digimon_by_bound_tamer_color_and_cost() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML parses")
        .add_card(tamer("YELLOW-TAMER", CardColor::Yellow))
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(digimon("YELLOW-HAND", CardColor::Yellow, 3))
        .add_card(digimon("RED-HAND", CardColor::Red, 3))
        .add_card(digimon("YELLOW-HIGH", CardColor::Yellow, 4))
        .add_card(filler("FILL"))
        .hand(0, &["P-156", "YELLOW-HAND", "RED-HAND", "YELLOW-HIGH"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "YELLOW-TAMER", Some(0));
    runner.place_on_field(1, "RED-TAMER", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let memory_after_option_cost = runner.memory();

    let tamer_choice = runner
        .pending_selection_view()
        .expect("chosen Tamer selection");
    assert_eq!(
        tamer_choice.valid_action_ids.len(),
        2,
        "own and opponent Tamers should both be visible choices"
    );
    runner
        .execute_action(
            tamer_choice.selecting_player,
            tamer_choice.valid_action_ids[0],
        )
        .expect("choose the yellow Tamer");

    runner.execute_branch(0).expect("choose hand branch");
    let hand_choice = runner
        .pending_selection_view()
        .expect("hand Digimon selection");
    assert_eq!(
        hand_choice.valid_action_ids,
        vec![PLAY_HAND_START],
        "only the yellow play-cost-3 Digimon should be selectable from hand"
    );
    runner
        .execute_action(hand_choice.selecting_player, PLAY_HAND_START)
        .expect("play yellow hand Digimon");
    runner.auto_resolve().expect("finish P-156 Main");

    assert!(battle_area_contains_card(&runner, 0, "YELLOW-HAND"));
    assert!(!battle_area_contains_card(&runner, 0, "RED-HAND"));
    assert!(!battle_area_contains_card(&runner, 0, "YELLOW-HIGH"));
    assert_eq!(
        runner.memory(),
        memory_after_option_cost,
        "selected Digimon must be played without paying its cost"
    );
}

#[test]
fn p_156_main_requires_at_least_one_tamer_choice_before_option_is_legal() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML parses")
        .add_card(digimon("BLACK-SOURCE", CardColor::Black, 3))
        .add_card(filler("FILL"))
        .hand(0, &["P-156"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "BLACK-SOURCE", Some(0));
    runner.game.enter_main_phase();

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 0.0,
        "P-156 should not be legal when its mandatory Tamer choice has no candidates"
    );
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid,
        "direct Option use must share the action-mask preflight"
    );
}

#[test]
fn p_156_main_can_choose_opponent_tamer_when_only_opponent_has_tamer() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML parses")
        .add_card(digimon("BLACK-SOURCE", CardColor::Black, 3))
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(digimon("RED-HAND", CardColor::Red, 3))
        .add_card(digimon("YELLOW-HAND", CardColor::Yellow, 3))
        .add_card(filler("FILL"))
        .hand(0, &["P-156", "RED-HAND", "YELLOW-HAND"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "BLACK-SOURCE", Some(0));
    runner.place_on_field(1, "RED-TAMER", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let tamer_choice = runner
        .pending_selection_view()
        .expect("opponent Tamer selection");
    assert_eq!(
        tamer_choice.valid_action_ids.len(),
        1,
        "the opponent Tamer should be the only available Tamer choice"
    );
    runner
        .execute_action(
            tamer_choice.selecting_player,
            tamer_choice.valid_action_ids[0],
        )
        .expect("choose opponent red Tamer");
    runner.execute_branch(0).expect("choose hand branch");

    let hand_choice = runner
        .pending_selection_view()
        .expect("hand Digimon selection");
    assert_eq!(
        hand_choice.valid_action_ids,
        vec![PLAY_HAND_START],
        "the selected opponent Tamer's color should filter hand Digimon"
    );
}

#[test]
fn p_156_main_trash_branch_uses_same_bound_tamer_color_and_cost_filter() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML parses")
        .add_card(tamer("YELLOW-TAMER", CardColor::Yellow))
        .add_card(digimon("YELLOW-TRASH", CardColor::Yellow, 3))
        .add_card(digimon("RED-TRASH", CardColor::Red, 3))
        .add_card(digimon("YELLOW-HIGH", CardColor::Yellow, 4))
        .add_card(filler("FILL"))
        .hand(0, &["P-156"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "YELLOW-TAMER", Some(0));
    for card_id in ["YELLOW-TRASH", "RED-TRASH", "YELLOW-HIGH"] {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|data| data.card_id == card_id)
            .expect("test card registered");
        let next_idx = runner.game.next_card_index();
        runner.game.players[0]
            .trash
            .push(digimon_engine::card_source::CardSource::new(
                data_idx, 0, next_idx,
            ));
    }
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let tamer_choice = runner
        .pending_selection_view()
        .expect("chosen Tamer selection");
    runner
        .execute_action(
            tamer_choice.selecting_player,
            tamer_choice.valid_action_ids[0],
        )
        .expect("choose yellow Tamer");
    runner.execute_branch(1).expect("choose trash branch");

    let trash_choice = runner
        .pending_selection_view()
        .expect("trash Digimon selection");
    assert_eq!(
        trash_choice.valid_action_ids,
        vec![TRASH_EFFECT_START],
        "only the yellow play-cost-3 Digimon should be selectable from trash"
    );
    runner
        .execute_action(trash_choice.selecting_player, TRASH_EFFECT_START)
        .expect("play yellow trash Digimon");
    runner.auto_resolve().expect("finish P-156 Main");

    assert!(battle_area_contains_card(&runner, 0, "YELLOW-TRASH"));
    assert!(!battle_area_contains_card(&runner, 0, "RED-TRASH"));
    assert!(!battle_area_contains_card(&runner, 0, "YELLOW-HIGH"));
}

#[test]
#[ignore = "pending: PUPPETS-G017 - optional Tamer play must be declinable while the mandatory add-this-option-to-hand tail still resolves"]
fn p_156_security_can_decline_tamer_play_and_still_add_this_option_to_hand() {
    todo!("decline the optional Security Tamer play and assert P-156 still moves to hand");
}

#[test]
#[ignore = "pending: PUPPETS-G017 - optional Tamer play before mandatory Security tail needs continuation-on-accept and continuation-on-decline"]
fn p_156_security_can_play_tamer_from_hand_then_add_this_option_to_hand() {
    todo!("select a Tamer from hand during Security, play it for free, then assert P-156 moves to hand");
}
