//! LM-068 HeavyMetaldramon (dual) — card-text tests.
//!
//! Digimon face: <Security A. +1>. [When Digivolving] [End of Attack] If your
//! hand has 4 or fewer cards, you may play 1 level 4 or lower Digimon card with
//! the [Evil], [Dark Dragon] or [Evil Dragon] trait from your trash without
//! paying the cost. [When Digivolving] [On Deletion] Trash 2 cards in your hand.
//! Then, you may activate 1 [Main] effect on this card's Option side.
//! Option face (use 6): [Trash] [Your Turn] When one of your [Evil], [Dark
//! Dragon] or [Evil Dragon] trait Digimon attacks, if your hand has 4 or fewer
//! cards, you may use this card with the cost reduced by 3. [Main] Delete 1 of
//! your opponent's Digimon with a level as high as the number of cards in your
//! hand or higher.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::action::space::PASS;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "LM-068";

fn security_option_filler() -> CardData {
    let mut c = make_test_card("SEC-OPT", "Security Option Filler");
    c.card_kind = CardKind::Option;
    c
}

fn digi(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(level);
    // Using a purple/red Option needs a purple and a red Digimon or Tamer.
    c.colors = vec![CardColor::Purple, CardColor::Red];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn runner_with(hand: &[&str]) -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("LM-068 YAML parses and compiles")
        .add_card(security_option_filler())
        .add_card(digi("EVIL-ATK", 5, &["Evil Dragon"]))
        .add_card(digi("PLAIN-ATK", 5, &["Dragon"]))
        .add_card(digi("OPP-L3", 3, &[]))
        .add_card(digi("OPP-L6", 6, &[]))
        .deck(0, &["SEC-OPT"; 10])
        .deck(1, &["SEC-OPT"; 10])
        .security(0, &["SEC-OPT"; 3])
        .security(1, &["SEC-OPT"; 3])
        .hand(0, hand)
        .memory(5)
        .start();
    runner.inject_trash(0, CARD_ID);
    runner
}

#[test]
fn lm_068_metadata() {
    let runner = runner_with(&[]);
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "HeavyMetaldramon");
    assert_eq!(card.kind, CompiledCardKind::Dual);
    assert_eq!(card.dp, Some(13000));
    let dual = card.dual.as_ref().expect("dual");
    assert_eq!(dual.option.use_cost, 6);
    assert!(dual.option.keywords.iter().any(|k| k == "ArtsDigivolve"));
}

#[test]
fn lm_068_option_face_has_trash_scoped_ally_attack_clause() {
    let runner = runner_with(&[]);
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let dual = card.dual.as_ref().expect("dual");
    assert!(dual.option.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Trash && t.when.contains(&CompiledTiming::OnAllyAttack)
    )));
}

#[test]
fn lm_068_trash_offers_use_when_evil_dragon_attacks_with_small_hand() {
    let mut runner = runner_with(&["SEC-OPT"; 4]);
    let atk = runner.place_stack(0, &["EVIL-ATK"]);
    runner.place_stack(1, &["OPP-L6"]);
    runner.attack_player(atk, 1, false);
    let view = runner
        .pending_selection_view()
        .expect("Black Sabbath offers its use from the trash");
    assert!(view.is_optional, "\"you may use\" can be declined");
    assert_eq!(view.valid_action_ids.len(), 1, "only Black Sabbath itself");
}

#[test]
fn lm_068_trash_use_deletes_digimon_with_level_at_least_hand_size() {
    let mut runner = runner_with(&["SEC-OPT"; 4]);
    let atk = runner.place_stack(0, &["EVIL-ATK"]);
    runner.place_stack(1, &["OPP-L3"]);
    runner.place_stack(1, &["OPP-L6"]);
    runner.attack_player(atk, 1, false);
    let view = runner.pending_selection_view().expect("use prompt");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("use Black Sabbath from the trash");
    // [Main]: hand has 4 cards, so only the level-6 Digimon (6 >= 4) is a
    // legal target; the level-3 one is not.
    let view = runner.pending_selection_view().expect("[Main] delete target prompt");
    assert_eq!(view.selecting_player, 0);
    assert_eq!(view.valid_action_ids.len(), 1, "only the level-6 Digimon qualifies");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("delete the level-6 Digimon");
    assert_eq!(runner.battle_area_size(1), 1, "level-6 Digimon deleted, level-3 stays");
}

#[test]
fn lm_068_trash_use_declined_leaves_card_in_trash() {
    let mut runner = runner_with(&["SEC-OPT"; 4]);
    let atk = runner.place_stack(0, &["EVIL-ATK"]);
    runner.place_stack(1, &["OPP-L6"]);
    let trash_before = runner.trash_size(0);
    runner.attack_player(atk, 1, false);
    let view = runner.pending_selection_view().expect("use prompt");
    runner.execute_action(0, PASS).expect("decline");
    assert_eq!(runner.trash_size(0), trash_before, "Black Sabbath stays in the trash");
    assert_eq!(runner.battle_area_size(1), 1, "nothing deleted");
}

#[test]
fn lm_068_trash_silent_when_hand_has_five_cards() {
    let mut runner = runner_with(&["SEC-OPT"; 5]);
    let atk = runner.place_stack(0, &["EVIL-ATK"]);
    runner.place_stack(1, &["OPP-L6"]);
    runner.attack_player(atk, 1, false);
    assert!(
        runner.pending_selection_view().is_none(),
        "hand of 5 does not meet the 4-or-fewer condition"
    );
}

#[test]
fn lm_068_trash_silent_when_attacker_lacks_trait() {
    let mut runner = runner_with(&["SEC-OPT"; 2]);
    let atk = runner.place_stack(0, &["PLAIN-ATK"]);
    runner.place_stack(1, &["OPP-L6"]);
    runner.attack_player(atk, 1, false);
    assert!(
        runner.pending_selection_view().is_none(),
        "non-[Evil]/[Dark Dragon]/[Evil Dragon] attacker does not trigger"
    );
}


#[test]
fn lm_068_option_side_main_activation_is_an_option_effect() {
    let runner = runner_with(&[]);
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let dual = card.dual.as_ref().expect("dual");
    assert!(dual.digimon.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::OnDeletion) && t.as_option_effect
    )));
}
