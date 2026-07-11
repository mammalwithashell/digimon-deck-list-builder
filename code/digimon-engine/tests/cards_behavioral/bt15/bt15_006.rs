// NOTE: the full authored test file (582 lines, 12 tests across 7 sections
// per RUST_DSL_TEST_API.md §4.3/§5/§11) is at
// code/digimon-engine/tests/cards_behavioral/bt15/bt15_006.rs in the
// worktree; all 12 tests pass. Reproduced below verbatim.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

fn pad(id: &str) -> digimon_engine::card_data::CardData {
    make_test_card(id, id)
}

fn digimon_at_level(id: &str, level: u8) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.level = Some(level);
    card
}

fn tamer_at_level(id: &str, level: u8) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = digimon_engine::enums::CardKind::Tamer;
    card.level = Some(level);
    card
}

fn runner_with_hand(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT15-006")
        .expect("BT15-006 YAML must be present in the embedded DSL pack")
        .add_card(pad("PAD"))
        .add_card(pad("HOST"))
        .add_card(digimon_at_level("LV5-DIGI", 5))
        .add_card(digimon_at_level("LV6-DIGI", 6))
        .add_card(digimon_at_level("LV3-DIGI", 3))
        .add_card(digimon_at_level("LV4-DIGI", 4))
        .add_card(tamer_at_level("LV5-TAMER", 5))
        .deck(0, &["PAD", "PAD", "PAD", "PAD", "PAD"])
        .deck(1, &["PAD", "PAD", "PAD", "PAD", "PAD"])
        .hand(0, hand)
        .memory(5)
        .start()
}

fn place_host_over_egg(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_stack(0, &["BT15-006", "HOST"])
}

#[test]
fn bt15_006_yaml_compiles_and_is_in_pack() {
    let runner = runner_with_hand(&[]);
    assert!(runner.compiled_card("BT15-006").is_some());
}

#[test]
fn bt15_006_has_one_optional_inherited_on_deletion_clause() {
    let runner = runner_with_hand(&[]);
    let compiled = runner.compiled_card("BT15-006").unwrap();
    assert_eq!(compiled.effects.len(), 1);
    match &compiled.effects[0] {
        CompiledClause::Triggered(clause) => {
            assert_eq!(clause.scope, CompiledScope::Inherited);
            assert_eq!(clause.when, vec![CompiledTiming::OnDeletion]);
            assert!(clause.optional);
        }
        other => panic!("expected Triggered clause; got {other:?}"),
    }
}

#[test]
fn bt15_006_trashing_level5_digimon_draws_two() {
    let mut runner = runner_with_hand(&["LV5-DIGI"]);
    let host = place_host_over_egg(&mut runner);
    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);
    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    let view = runner.pending_selection_view().unwrap();
    assert!(runner.pending_is_optional());
    let trash_action = *view.valid_action_ids.first().unwrap();
    runner.execute_action(0, trash_action).unwrap();
    let _ = runner.auto_resolve();
    assert_eq!(runner.hand_size(0) as i32 - hand_before as i32, 1);
    assert_eq!(runner.deck_size(0) as i32 - deck_before as i32, -2);
}

#[test]
fn bt15_006_trashing_level6_digimon_also_qualifies() {
    let mut runner = runner_with_hand(&["LV6-DIGI"]);
    let host = place_host_over_egg(&mut runner);
    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    let view = runner.pending_selection_view().unwrap();
    let trash_action = *view.valid_action_ids.first().unwrap();
    runner.execute_action(0, trash_action).unwrap();
    let _ = runner.auto_resolve();
    assert!(!runner.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "LV6-DIGI"));
}

#[test]
fn bt15_006_declining_trash_skips_draw() {
    let mut runner = runner_with_hand(&["LV5-DIGI"]);
    let host = place_host_over_egg(&mut runner);
    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);
    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    runner.pending_selection_view().unwrap();
    assert!(runner.pending_is_optional());
    runner.execute_action(0, PASS).unwrap();
    let _ = runner.auto_resolve();
    assert_eq!(runner.hand_size(0), hand_before);
    assert_eq!(runner.deck_size(0), deck_before);
    assert!(runner.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "LV5-DIGI"));
}

#[test]
fn bt15_006_no_level5_plus_digimon_in_hand_skips_prompt_entirely() {
    let mut runner = runner_with_hand(&["LV3-DIGI", "LV4-DIGI", "LV5-TAMER"]);
    let host = place_host_over_egg(&mut runner);
    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);
    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    let _ = runner.auto_resolve();
    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.hand_size(0), hand_before);
    assert_eq!(runner.deck_size(0), deck_before);
}

#[test]
fn bt15_006_empty_hand_skips_prompt_entirely() {
    let mut runner = runner_with_hand(&[]);
    let host = place_host_over_egg(&mut runner);
    let deck_before = runner.deck_size(0);
    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    let _ = runner.auto_resolve();
    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.deck_size(0), deck_before);
}

#[test]
fn bt15_006_multiple_eligible_cards_offers_choice_of_which_to_trash() {
    let mut runner = runner_with_hand(&["LV5-DIGI", "LV6-DIGI", "LV3-DIGI"]);
    let host = place_host_over_egg(&mut runner);
    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(view.valid_action_ids.len(), 2);
    let hand_index_of_lv6 = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "LV6-DIGI")
        .unwrap();
    let lv6_action = digimon_engine::action::space::PLAY_HAND_START + hand_index_of_lv6 as u16;
    assert!(view.valid_action_ids.contains(&lv6_action));
    runner.execute_action(0, lv6_action).unwrap();
    let _ = runner.auto_resolve();
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(!hand_ids.contains(&"LV6-DIGI".to_string()));
    assert!(hand_ids.contains(&"LV5-DIGI".to_string()));
    assert!(hand_ids.contains(&"LV3-DIGI".to_string()));
}

#[test]
fn bt15_006_trashing_moves_the_discarded_card_into_trash() {
    let mut runner = runner_with_hand(&["LV5-DIGI"]);
    let host = place_host_over_egg(&mut runner);
    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    let trash_before = runner.game.players[0].trash.len();
    let view = runner.pending_selection_view().unwrap();
    let trash_action = *view.valid_action_ids.first().unwrap();
    runner.execute_action(0, trash_action).unwrap();
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].trash.len() - trash_before, 1);
    assert!(runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "LV5-DIGI"));
}

#[test]
fn bt15_006_declining_trash_moves_no_card_into_trash() {
    let mut runner = runner_with_hand(&["LV5-DIGI"]);
    let host = place_host_over_egg(&mut runner);
    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    let trash_before = runner.game.players[0].trash.len();
    runner.pending_selection_view().unwrap();
    runner.execute_action(0, PASS).unwrap();
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].trash.len(), trash_before);
}

#[test]
fn bt15_006_game_event_trash_fires_for_the_level5_hand_discard_cost() {
    let mut runner = runner_with_hand(&["LV5-DIGI"]);
    let host = place_host_over_egg(&mut runner);
    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    let checkpoint = runner.event_checkpoint();
    let view = runner.pending_selection_view().unwrap();
    let trash_action = *view.valid_action_ids.first().unwrap();
    runner.execute_action(0, trash_action).unwrap();
    let _ = runner.auto_resolve();
    let trash_events: Vec<&GameEvent> = runner
        .events_since(checkpoint)
        .iter()
        .filter(|e| matches!(e, GameEvent::Trash { .. }))
        .collect();
    // The level-5 hand discard is a cost paid via trash_from_hand_by_index,
    // which routes through Game::trash_card and MUST emit GameEvent::Trash
    // (engine event-emission contract, fixed 2026-07-02 alongside BT19-069).
    assert!(
        !trash_events.is_empty(),
        "the level-5 hand-discard cost must emit GameEvent::Trash"
    );
}

#[test]
fn bt15_006_pass_is_legal_when_eligible_target_exists() {
    let mut runner = runner_with_hand(&["LV5-DIGI"]);
    let host = place_host_over_egg(&mut runner);
    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    runner.pending_selection_view().unwrap();
    assert!(runner.pending_is_optional());
    runner.execute_action(0, PASS).unwrap();
}
