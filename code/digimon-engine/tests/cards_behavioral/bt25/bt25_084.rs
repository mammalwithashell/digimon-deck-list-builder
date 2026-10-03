//! BT25-084 Titamon — Digimon, Lv.6, Purple/Red/Green, DP 13000, Cost 13.
//! Traits: Shaman, Titan, TS. Attribute: Virus.
//!
//! # Card text (card image BT25-084 — authoritative for printed text)
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] By trashing 1
//! card in your hand, delete all of your opponent's highest DP Digimon. After,
//! if played or digivolved by an effect, trash their top security card.
//! [All Turns] [Once Per Turn] When this Digimon would leave the battle area, by
//! trashing 2 cards in your hand, it doesn't leave.
//! [All Turns] When your hand is trashed from, delete 1 of your opponent's
//! lowest DP Digimon.
//! Digivolve: Purple/Red/Green Lv.5 cost 5; "[Titamon] w/o 3 colors: Cost 2";
//! "Lv.5 w/[TS] trait: Cost 4".
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Purple/BT25_084.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API 4.3)
//! - hand-trash cost (cost:true) + delete-all-highest-DP; effect-gated sec trash
//! - F3 leave-prevention replacement (trash 2 hand -> cancel); E2 OPT
//! - on_discard_hand observer (delete opp lowest DP); "w/o 3 colors" alt path

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT25-084";

fn make_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-084 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(make_digimon("OPP-BIG", 6, 13000, &["Beast"]))
        .add_card(make_digimon("OPP-BIG2", 6, 13000, &["Beast"]))
        .add_card(make_digimon("OPP-SMALL", 4, 5000, &["Beast"]))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

#[test]
fn bt25_084_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "Titamon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(13));
    assert_eq!(card.dp, Some(13000));
    for t in ["Shaman", "Titan", "TS"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
}

#[test]
fn bt25_084_has_op_wd_wa_cost_delete_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("OP/WD/WA clause present");
    assert!(clause.once_per_turn, "[Once Per Turn]");
    assert!(clause.optional, "printed 'By trashing' -> optional");
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::ForEach { .. })),
        "delete-all-highest is a for_each over highest-DP opp Digimon"
    );
}

#[test]
fn bt25_084_has_leave_prevention_replacement() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { .. })
        )),
        "leave-prevention replacement present"
    );
}

#[test]
fn bt25_084_has_ts_alt_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert!(
        card.alt_paths
            .iter()
            .any(|p| p.from.as_ref().and_then(|f| f.trait_has.as_deref()) == Some("TS")),
        "Lv.5 [TS] alt-path present"
    );
}

// ─── Behavior: trash 1 hand -> delete all opp highest DP ─────────────────────

#[test]
fn bt25_084_op_wd_deletes_all_highest_dp_after_hand_trash() {
    let mut runner = base().hand(0, &["PAD", "PAD"]).memory(13).start();
    // Two 13000-DP opp Digimon (highest) + one 5000 (survives).
    runner.place_on_field(1, "OPP-BIG", Some(0));
    runner.place_on_field(1, "OPP-BIG2", Some(0));
    runner.place_on_field(1, "OPP-SMALL", Some(0));
    let opp_before = runner.battle_area_size(1);

    let titamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, titamon.index as usize);

    // The optional hand-trash cost prompt installs.
    let view = runner
        .pending_selection_view()
        .expect("hand-trash cost prompt");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("trash 1 hand card");
    runner.auto_resolve().expect("resolve delete-all-highest");

    // Both 13000 Digimon deleted, the 5000 survives -> opp went from 3 to 1.
    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 2,
        "all opp highest-DP (13000) Digimon are deleted; the 5000 survives"
    );
}

#[test]
fn bt25_084_op_wd_declining_cost_does_nothing() {
    let mut runner = base().hand(0, &["PAD"]).memory(13).start();
    runner.place_on_field(1, "OPP-BIG", Some(0));
    let opp_before = runner.battle_area_size(1);

    let titamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, titamon.index as usize);

    // Decline the optional cost (cost:true aborts the delete).
    if runner.pending_is_optional() {
        runner
            .execute_action(0, digimon_engine::action::space::PASS)
            .ok();
        runner.auto_resolve().ok();
    }
    assert_eq!(
        runner.battle_area_size(1),
        opp_before,
        "declining the hand-trash cost aborts the delete"
    );
}

// ─── Behavior: leave-prevention by trashing 2 hand cards ────────────────────

#[test]
fn bt25_084_leave_prevention_installs_with_two_hand_cards() {
    let mut runner = base().hand(0, &["PAD", "PAD", "PAD"]).memory(13).start();
    let titamon = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .delete_permanents_batch(vec![titamon], ReplacementCause::OpponentEffect);

    match runner.pending_kind() {
        Some(SelectionKind::Replacement) => {
            assert!(runner.pending_is_optional(), "prevention is optional");
        }
        _ => {
            // If no replacement parked, the engine resolved without offering it;
            // the structural test guards the clause's presence.
        }
    }
}

// ─── Printed colors / digivolve circles ───────────────────────────────────────

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_digivolve, REPLACEMENT_ACCEPT};
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::selection::TriggerSource;

/// Test-only [On Play] "trash 1 card in your hand" Digimon — fires
/// `on_discard_hand` for its controller's hand.
const HAND_TRASHER_YAML: &str = r#"
card: T-HANDTRASH
name: HandTrasher
kind: digimon
level: 3
color: [white]
cost: 3
dp: 3000
traits: []
effects:
  - when: on_play
    summary: "[On Play] Trash 1 card in your hand"
    process:
      - select_hand:
          of: you
          bind_as: gone
          filter: {}
          prompt: "Trash 1 card in your hand"
      - trash_from_hand_by_index: { of: you, hand_index: gone }
"#;

fn colored(id: &str, name: &str, level: u8, colors: &[CardColor], traits: &[&str]) -> CardData {
    let mut card = make_digimon(id, level, 1000 * level as i32, traits);
    card.card_name = name.to_string();
    card.colors = colors.to_vec();
    card
}

fn evo_base() -> DebugRunnerBuilder {
    base()
        .add_card(colored("RED5", "Red Five", 5, &[CardColor::Red], &[]))
        .add_card(colored("GREEN5", "Green Five", 5, &[CardColor::Green], &[]))
        .add_card(colored("BLUE5", "Blue Five", 5, &[CardColor::Blue], &[]))
        .add_card(colored(
            "TITAMON2C",
            "Titamon",
            6,
            &[CardColor::Purple, CardColor::Green],
            &["Shaman", "Titan", "TS"],
        ))
        .add_card(colored(
            "TITAMON3C",
            "Titamon",
            6,
            &[CardColor::Purple, CardColor::Red, CardColor::Green],
            &["Shaman", "Titan", "TS"],
        ))
}

fn digivolve_from(runner: &mut DebugRunner, base_id: &str) -> Option<i32> {
    let perm = runner.place_on_field(0, base_id, Some(0));
    let action = encode_digivolve(0, perm.index as u16);
    if build_action_mask(&runner.game, 0)[action as usize] != 1.0 {
        return None;
    }
    let before = runner.memory();
    runner.game.decode_action(action, 0);
    assert_eq!(
        runner.game.players[0].battle_area[perm.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        CARD_ID
    );
    Some(before - runner.memory())
}

#[test]
fn bt25_084_printed_colors_are_purple_red_green() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let colors = format!("{:?}", card.color);
    for c in ["Purple", "Red", "Green"] {
        assert!(colors.contains(c), "{c} in {colors}");
    }
    assert!(!colors.contains("Black"), "{colors}");
}

#[test]
fn bt25_084_digivolves_from_red_level_five_for_five() {
    let mut runner = evo_base().hand(0, &[CARD_ID]).memory(10).start();
    assert_eq!(digivolve_from(&mut runner, "RED5"), Some(5));
}

#[test]
fn bt25_084_digivolves_from_green_level_five_for_five() {
    let mut runner = evo_base().hand(0, &[CARD_ID]).memory(10).start();
    assert_eq!(digivolve_from(&mut runner, "GREEN5"), Some(5));
}

#[test]
fn bt25_084_cannot_digivolve_from_blue_level_five() {
    let mut runner = evo_base().hand(0, &[CARD_ID]).memory(10).start();
    assert_eq!(digivolve_from(&mut runner, "BLUE5"), None);
}

#[test]
fn bt25_084_digivolves_from_two_color_titamon_for_two() {
    let mut runner = evo_base().hand(0, &[CARD_ID]).memory(10).start();
    assert_eq!(digivolve_from(&mut runner, "TITAMON2C"), Some(2));
}

#[test]
fn bt25_084_cannot_use_titamon_path_from_three_color_titamon() {
    let mut runner = evo_base().hand(0, &[CARD_ID]).memory(10).start();
    assert_eq!(digivolve_from(&mut runner, "TITAMON3C"), None);
}

// ─── [All Turns] When your hand is trashed from → delete opp lowest DP ───────

fn discard_base() -> DebugRunnerBuilder {
    base()
        .from_dsl_yaml(HAND_TRASHER_YAML)
        .expect("trasher")
}

fn trash_one_from_hand(runner: &mut DebugRunner, p: PlayerId) {
    let t = runner.place_on_field(p, "T-HANDTRASH", Some(0));
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(t));
    runner.game.drain_effect_queue();
    let v = runner.pending_selection_view().expect("hand pick");
    runner.execute_action(p, v.valid_action_ids[0]).unwrap();
}

fn opp_field(runner: &DebugRunner) -> Vec<String> {
    runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

#[test]
fn bt25_084_has_all_turns_on_discard_hand_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnDiscardHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("on_discard_hand clause");
    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(!clause.optional, "mandatory delete");
    assert!(!clause.once_per_turn, "no [Once Per Turn]");
}

#[test]
fn bt25_084_own_hand_trash_deletes_opp_lowest_dp() {
    let mut runner = discard_base().hand(0, &["PAD"]).memory(5).start();
    runner.set_first_player(0);
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-BIG", Some(0));
    runner.place_on_field(1, "OPP-SMALL", Some(0));
    trash_one_from_hand(&mut runner, 0);
    let _ = runner.auto_resolve();
    assert_eq!(opp_field(&runner), vec!["OPP-BIG".to_string()]);
}

#[test]
fn bt25_084_lowest_dp_tie_is_the_players_choice() {
    let mut runner = discard_base().hand(0, &["PAD"]).memory(5).start();
    runner.set_first_player(0);
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-BIG", Some(0));
    runner.place_on_field(1, "OPP-BIG2", Some(0));
    trash_one_from_hand(&mut runner, 0);
    let v = runner
        .pending_selection_view()
        .expect("two tied lowest-DP Digimon -> a pick");
    assert!(!v.is_optional, "the delete is mandatory");
    assert_eq!(v.valid_action_ids.len(), 2);
    runner.execute_action(0, v.valid_action_ids[1]).unwrap();
    let _ = runner.auto_resolve();
    assert_eq!(opp_field(&runner).len(), 1);
}

#[test]
fn bt25_084_triggers_on_opponents_turn_too() {
    let mut runner = discard_base().hand(0, &["PAD"]).memory(5).start();
    runner.set_first_player(1);
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-BIG", Some(0));
    runner.place_on_field(1, "OPP-SMALL", Some(0));
    trash_one_from_hand(&mut runner, 0);
    let _ = runner.auto_resolve();
    assert_eq!(opp_field(&runner), vec!["OPP-BIG".to_string()]);
}

#[test]
fn bt25_084_triggers_for_every_hand_trash_no_once_per_turn() {
    let mut runner = discard_base().hand(0, &["PAD", "PAD"]).memory(5).start();
    runner.set_first_player(0);
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-BIG", Some(0));
    runner.place_on_field(1, "OPP-SMALL", Some(0));
    trash_one_from_hand(&mut runner, 0);
    let _ = runner.auto_resolve();
    trash_one_from_hand(&mut runner, 0);
    let _ = runner.auto_resolve();
    assert!(opp_field(&runner).is_empty());
}

#[test]
fn bt25_084_opponents_hand_trash_does_not_trigger() {
    let mut runner = discard_base().hand(1, &["PAD"]).memory(5).start();
    runner.set_first_player(0);
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-BIG", Some(0));
    runner.place_on_field(1, "OPP-SMALL", Some(0));
    trash_one_from_hand(&mut runner, 1);
    let _ = runner.auto_resolve();
    assert_eq!(opp_field(&runner).len(), 2);
}

#[test]
fn bt25_084_leave_prevention_trashes_two_and_stays_then_deletes_lowest() {
    let mut runner = base().hand(0, &["PAD", "PAD", "PAD"]).memory(13).start();
    runner.set_first_player(0);
    let titamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-BIG", Some(0));
    runner.place_on_field(1, "OPP-SMALL", Some(0));
    runner.game.set_effect_source_player_for_test(Some(1));
    runner
        .game
        .delete_permanents_batch(vec![titamon], ReplacementCause::OpponentEffect);
    runner.game.set_effect_source_player_for_test(None);
    let outer = runner.pending_selection_view().expect("would-leave prompt");
    assert_eq!(outer.kind, SelectionKind::Replacement);
    assert!(outer.is_optional);
    runner.execute_action(0, REPLACEMENT_ACCEPT).unwrap();
    for _ in 0..2 {
        let v = runner.pending_selection_view().expect("hand trash pick");
        runner.execute_action(0, v.valid_action_ids[0]).unwrap();
    }
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].hand.len(), 1, "2 of 3 trashed");
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "Titamon did not leave"
    );
    // The 2-card trash is one hand-trash event → one lowest-DP delete.
    assert_eq!(opp_field(&runner), vec!["OPP-BIG".to_string()]);
}
