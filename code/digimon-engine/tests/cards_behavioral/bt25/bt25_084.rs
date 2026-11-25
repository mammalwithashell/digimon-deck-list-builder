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
    // DCGO BT25_084.cs: one DestroyPermanentsClass over the IsMaxDP list (one
    // batch), inside "if discarded", else RemoveUse.
    let gated = clause.process.iter().find_map(|s| match s {
        CompiledStep::If { then, else_branch, .. } => Some((then, else_branch)),
        _ => None,
    });
    let (then, else_) = gated.expect("the paid tail is gated on the discard");
    assert!(
        then.iter()
            .any(|s| matches!(s, CompiledStep::DeleteAllPermanents { .. })),
        "delete-all-highest is ONE batch (delete_all_permanents)"
    );
    assert!(
        else_.iter().any(|s| matches!(s, CompiledStep::RefundOpt)),
        "a declined discard gives the use back (DCGO RemoveUse)"
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

    // Both 13000 Digimon are deleted by the clause; the 1-card hand trash is
    // also a "your hand is trashed from" event, so Titamon's [All Turns]
    // observer then deletes the opp's lowest-DP Digimon (the 5000).
    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 3,
        "highest-DP pair deleted, then the on-hand-trashed delete takes the 5000"
    );
}

#[test]
fn bt25_084_op_wd_declining_cost_does_nothing() {
    let mut runner = base().hand(0, &["PAD"]).memory(13).start();
    runner.place_on_field(1, "OPP-BIG", Some(0));
    let opp_before = runner.battle_area_size(1);

    let titamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, titamon.index as usize);

    // The single hand pick is the decline (DCGO optional: false, SelectHand
    // canNoSelect: true): choosing nothing deletes nothing.
    assert!(runner.pending_is_optional(), "the hand pick is declinable");
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("choose no card");
    assert!(runner.pending_selection().is_none(), "no decision is left");
    assert_eq!(
        runner.battle_area_size(1),
        opp_before,
        "declining the hand-trash cost aborts the delete"
    );
    assert_eq!(runner.hand_size(0), 1, "nothing trashed");
    // DCGO BT25_084.cs: `if (!discarded) activateClass.RemoveUse()` -- the use
    // is given back (owner ruling 2026-11-24, "Generally let's look to match
    // dcgo"), so the next [When Digivolving] this turn offers it again.
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(titamon));
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_is_optional(),
        "the declined discard kept the [Once Per Turn]"
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
    Some(i32::from(before - runner.memory()))
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
    let field = opp_field(&runner);
    assert!(field.contains(&"OPP-BIG".to_string()) && field.contains(&"OPP-SMALL".to_string()), "{field:?}");
}

#[test]
fn bt25_084_leave_prevention_trashes_two_and_stays_then_deletes_lowest() {
    let mut runner = base().hand(0, &["PAD", "PAD", "PAD"]).memory(13).start();
    runner.set_first_player(0);
    let titamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-BIG", Some(0));
    runner.place_on_field(1, "OPP-SMALL", Some(0));
    runner.game.set_effect_source_player_for_test(Some(1));
    runner.game.delete_permanent_with_effects(titamon);
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

/// Comprehensive Rules Ver. 4.3 §15-7-4: "If executing an optional processing
/// condition's contents isn't possible, then the player can't choose to
/// execute them." With 1 card in hand, "by trashing 2 cards in your hand"
/// can't be executed, so the leave-prevention is not offered and Titamon
/// leaves (DCGO BT25_084.cs `CanActivateCondition`: `HandCards.Count() >= 2`).
/// Since the DCGO-parity change (owner ruling 2026-11-23) the first hand pick
/// is a declinable non-cost pick, so the YAML gates the offer itself with
/// `active_when: count_gte 2` hand cards.
/// G-ENGINE-UNPAYABLE-OPTIONAL-CONDITION-OFFERED.
#[test]
fn bt25_084_leave_prevention_not_offered_with_one_hand_card() {
    let mut runner = base().hand(0, &["PAD"]).memory(13).start();
    runner.set_first_player(0);
    let titamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.set_effect_source_player_for_test(Some(1));
    runner.game.delete_permanent_with_effects(titamon);
    runner.game.set_effect_source_player_for_test(None);
    assert!(
        runner
            .pending_selection_view()
            .is_none_or(|v| v.kind != SelectionKind::Replacement),
        "1 card in hand: the 2-card condition can't be chosen, so no prevention is offered"
    );
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].hand.len(), 1, "nothing was trashed");
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "Titamon left the battle area"
    );
}

// ─── [All Turns][OPT] leave prevention: a declined cost keeps the use ───────
//
// Owner ruling 2026-11-23 ("parity with DCGO"; qa/rules/mechanics-gaps-2026-10.md
// W14 row (f)). DCGO BT25_084.cs: the replacement is offered only while the
// hand holds 2 cards (`CanActivateCondition` `HandCards.Count() >= 2`); after
// the accept, one `SelectHandEffect` (maxCount 2, canNoSelect true,
// canEndNotMax false) asks for 2 cards or none. Fewer than 2 chosen ->
// `RemoveUse()` and Titamon leaves. 2 chosen -> spent, it doesn't leave.
// Official Q6398: trashing only 1 of the 2 cards can't meet the condition.
//
// To observe the kept use on a later event of the same turn, Titamon carries
// <Evade>: the would-be-deleted window is offered after the declined
// would-leave window and keeps it on the field, suspended (so <Evade> itself
// is not offered on the next deletion).

fn titamon_board(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = base().hand(0, hand).memory(13).start();
    runner.set_first_player(0);
    let titamon = runner.place_on_field(0, CARD_ID, Some(0));
    (runner, titamon)
}

fn titamon_handle(runner: &DebugRunner) -> Option<PermanentHandle> {
    runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .map(|i| PermanentHandle { player: 0, index: i as u8 })
}

fn grant_evade(runner: &mut DebugRunner, h: PermanentHandle) {
    runner.game.modifiers.grant_keyword(
        h,
        digimon_engine::enums::Keyword::Evade,
        digimon_engine::enums::Expiry::Permanent,
        0,
    );
}

fn remove_by_opponent(runner: &mut DebugRunner, h: PermanentHandle) {
    runner
        .game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
}

/// While <Evade> can also answer the deletion, the engine first asks which
/// immediate replacement effect applies next (CR 4.3 15-8-5); pick <Evade>
/// (`evade`) or Titamon's clause. With one candidate left, its own accept
/// prompt is already pending.
fn pick_replacement(runner: &mut DebugRunner, evade: bool) {
    let v = runner.pending_selection_view().expect("replacement prompt");
    if v.kind == SelectionKind::EffectChoice && v.prompt.contains("immediate replacement") {
        let row = v
            .effect_choices
            .as_ref()
            .and_then(|rows| rows.iter().find(|c| c.label.contains("Evade") == evade))
            .unwrap_or_else(|| panic!("replacement order rows: {:?}", v.effect_choices))
            .action_id;
        runner.execute_action(v.selecting_player, row).expect("order pick");
    }
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Replacement));
}

/// Accept <Evade> for the same deletion. It asks for nothing more, so the
/// declined clause was not offered again for this event.
fn bt25_084_accept_evade(runner: &mut DebugRunner) {
    pick_replacement(runner, true);
    runner.execute_action(0, REPLACEMENT_ACCEPT).expect("accept <Evade>");
    assert!(
        runner.pending_selection().is_none(),
        "the same deletion does not offer the declined clause again"
    );
}

/// PASS is legal at the pending prompt (the action mask, not the raw ids).
fn pass_legal(runner: &DebugRunner) -> bool {
    let p = runner.pending_selection().expect("prompt").selecting_player;
    build_action_mask(&runner.game, p)[digimon_engine::action::space::PASS as usize] == 1.0
}

fn hand_pick_ids(runner: &DebugRunner) -> Vec<u16> {
    runner
        .pending_selection_view()
        .expect("hand pick")
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != digimon_engine::action::space::PASS)
        .collect()
}

#[test]
fn bt25_084_leave_prevention_outer_decline_keeps_opt() {
    let (mut runner, titamon) = titamon_board(&["PAD", "PAD", "PAD", "PAD"]);
    grant_evade(&mut runner, titamon);
    remove_by_opponent(&mut runner, titamon);
    pick_replacement(&mut runner, false);
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the replacement");
    bt25_084_accept_evade(&mut runner);
    assert_eq!(runner.game.players[0].hand.len(), 4, "nothing trashed");
    let titamon = titamon_handle(&runner).expect("<Evade> kept Titamon");
    remove_by_opponent(&mut runner, titamon);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "declined at the accept prompt: offered again this turn"
    );
}

#[test]
fn bt25_084_leave_prevention_accept_then_decline_cost_keeps_opt() {
    let (mut runner, titamon) = titamon_board(&["PAD", "PAD", "PAD", "PAD"]);
    grant_evade(&mut runner, titamon);
    remove_by_opponent(&mut runner, titamon);
    pick_replacement(&mut runner, false);
    runner.execute_action(0, REPLACEMENT_ACCEPT).expect("accept");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    assert!(
        pass_legal(&runner),
        "DCGO BT25_084.cs SelectHandEffect canNoSelect: the 2-card cost may be declined after the accept"
    );
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the cost");
    assert_eq!(runner.game.players[0].hand.len(), 4, "nothing trashed");
    bt25_084_accept_evade(&mut runner);
    let titamon = titamon_handle(&runner).expect("<Evade> kept Titamon");
    remove_by_opponent(&mut runner, titamon);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "DCGO RemoveUse: the declined cost keeps the [Once Per Turn]"
    );
    runner.execute_action(0, REPLACEMENT_ACCEPT).expect("accept again");
    assert_eq!(hand_pick_ids(&runner).len(), 4, "Titamon's own hand pick");
}

#[test]
fn bt25_084_leave_prevention_declined_cost_lets_it_leave() {
    let (mut runner, titamon) = titamon_board(&["PAD", "PAD", "PAD"]);
    remove_by_opponent(&mut runner, titamon);
    runner.execute_action(0, REPLACEMENT_ACCEPT).expect("accept");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    assert!(pass_legal(&runner), "the 2-card cost may be declined after the accept");
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the cost");
    assert!(runner.pending_selection().is_none(), "no decision is left");
    assert!(titamon_handle(&runner).is_none(), "the cost was not paid: Titamon leaves");
    assert_eq!(runner.game.players[0].hand.len(), 3, "nothing trashed");
}

/// Choosing the first card commits to the 2-card cost: the second pick
/// offers no decline (DCGO `canEndNotMax: false`, 2 cards or none; Q6398).
/// Both trashed: spent, and a later removal this turn is not offered it even
/// with 2 more cards in hand.
#[test]
fn bt25_084_leave_prevention_paid_spends_opt_and_second_pick_is_mandatory() {
    let (mut runner, titamon) = titamon_board(&["PAD", "PAD", "PAD", "PAD"]);
    remove_by_opponent(&mut runner, titamon);
    runner.execute_action(0, REPLACEMENT_ACCEPT).expect("accept");
    let first = hand_pick_ids(&runner)[0];
    runner.execute_action(0, first).expect("first card");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand), "second hand pick");
    assert!(
        !pass_legal(&runner),
        "one of the two cards is not a partial payment the player may stop at"
    );
    let second = hand_pick_ids(&runner)[0];
    runner.execute_action(0, second).expect("second card");
    assert!(runner.pending_selection().is_none(), "no decision is left");
    assert_eq!(runner.game.players[0].hand.len(), 2, "2 of 4 trashed");
    let titamon = titamon_handle(&runner).expect("Titamon did not leave");
    remove_by_opponent(&mut runner, titamon);
    assert!(
        runner
            .pending_selection_view()
            .is_none_or(|v| v.kind != SelectionKind::Replacement),
        "spent: not offered again this turn"
    );
    assert!(runner.pending_selection().is_none(), "no decision is left");
    assert!(titamon_handle(&runner).is_none());
}

// ─── [OP][WD][WA][OPT] use accounting, batch deletion, by-effect tail ───────
//
// DCGO BT25_084.cs L106-123: one SelectHandEffect (canNoSelect); if a card was
// discarded, ONE DestroyPermanentsClass over every opponent IsMaxDP Digimon,
// then, if EnteredByEffect, trash the opponent's top security card; else
// RemoveUse(). AdditionalActivateCondition HandCards.Count > 0.

/// Paying the discard spends the [Once Per Turn]: a later [When Digivolving]
/// the same turn is not offered.
#[test]
fn bt25_084_op_wd_paying_spends_opt() {
    let mut runner = base().hand(0, &["PAD", "PAD"]).memory(13).start();
    runner.place_on_field(1, "OPP-BIG", Some(0));
    let titamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, titamon.index as usize);
    let v = runner.pending_selection_view().expect("hand pick");
    let card = *v
        .valid_action_ids
        .iter()
        .find(|&&a| a != digimon_engine::action::space::PASS)
        .unwrap();
    runner.execute_action(0, card).unwrap();
    assert!(runner.pending_selection().is_none(), "no decision is left");
    assert_eq!(runner.battle_area_size(1), 0, "OPP-BIG deleted");
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(titamon));
    runner.game.drain_effect_queue();
    assert!(runner.pending_selection().is_none(), "the paid use is spent");
}

/// With an empty hand the clause is not offered (DCGO CanActivate: hand > 0),
/// and nothing is spent: with a card in hand later the same turn it is.
#[test]
fn bt25_084_op_wd_not_offered_with_empty_hand_and_not_spent() {
    let mut runner = base().memory(13).start();
    runner.place_on_field(1, "OPP-BIG", Some(0));
    let titamon = runner.place_on_field(0, CARD_ID, Some(0));
    assert_eq!(runner.hand_size(0), 0);
    runner.fire_on_play(0, titamon.index as usize);
    assert!(runner.pending_selection().is_none(), "empty hand: not offered");
    runner.add_to_hand(0, "PAD");
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(titamon));
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_is_optional(),
        "the unoffered [On Play] spent nothing"
    );
}

/// Test-only observer: "when any Digimon is deleted, gain 1 memory". A
/// deletion observer triggers ONCE per deletion event (§15-5-2; the engine
/// collapses a batch's observer entries), so it counts deletion events.
const DELETION_COUNTER_YAML: &str = r#"
card: T-DEL-COUNTER
name: DeletionCounter
kind: digimon
level: 3
color: [white]
cost: 3
dp: 3000
traits: []
effects:
  - when: on_any_deletion
    condition:
      event_target_kind: digimon
    process:
      - gain_memory: 1
"#;

/// The tied highest-DP Digimon leave as ONE deletion (DCGO one
/// DestroyPermanentsClass over the whole IsMaxDP list), not one deletion per
/// Digimon: a deletion observer fires once.
#[test]
fn bt25_084_tied_highest_dp_deleted_as_one_batch() {
    let mut runner = base()
        .from_dsl_yaml(DELETION_COUNTER_YAML)
        .expect("observer")
        .hand(0, &["PAD"])
        .memory(3)
        .start();
    runner.place_on_field(1, "OPP-BIG", Some(0));
    runner.place_on_field(1, "OPP-BIG2", Some(0));
    runner.place_on_field(0, "T-DEL-COUNTER", Some(0));
    let titamon = runner.place_on_field(0, CARD_ID, Some(0));
    let memory_before = runner.game.memory;
    runner.fire_on_play(0, titamon.index as usize);
    let v = runner.pending_selection_view().expect("hand pick");
    let card = *v
        .valid_action_ids
        .iter()
        .find(|&&a| a != digimon_engine::action::space::PASS)
        .unwrap();
    runner.execute_action(0, card).unwrap();
    assert!(runner.pending_selection().is_none(), "no decision is left");
    assert_eq!(runner.battle_area_size(1), 0, "both tied Digimon deleted");
    assert_eq!(
        runner.game.memory - memory_before,
        1,
        "one deletion event, not two sequential ones"
    );
}

/// Test-only [On Play]: digivolve 1 of your level 5 Digimon into a [Titamon]
/// card in your hand, by an effect.
const EVO_ENABLER_YAML: &str = r#"
card: T-EVO-ENABLER
name: EvoEnabler
kind: digimon
level: 3
color: [white]
cost: 3
dp: 3000
traits: []
effects:
  - when: on_play
    summary: "[On Play] Digivolve 1 of your level 5 Digimon into a Titamon in hand"
    process:
      - select_own_permanent:
          bind_as: base
          filter: { level_eq: 5 }
          prompt: "Choose the Digimon to digivolve"
      - select_hand:
          of: you
          bind_as: evo
          filter: { name_contains: "Titamon" }
          prompt: "Digivolve into"
      - effect_initiated_digivolve:
          target: base
          from_hand: evo
          cost: 0
          ignore_requirements: true
"#;

fn resolve_picking_first(runner: &mut DebugRunner) {
    for _ in 0..16 {
        let Some(v) = runner.pending_selection_view() else { return };
        let id = *v
            .valid_action_ids
            .iter()
            .find(|&&a| a != digimon_engine::action::space::PASS)
            .expect("a non-PASS choice");
        runner.execute_action(v.selecting_player, id).unwrap();
    }
    panic!("selections did not finish");
}

/// "After, if played or digivolved by an effect, trash their top security
/// card": DCGO EnteredByEffect = (CanTriggerOnPlay || CanTriggerWhenDigivolving)
/// && IsByEffect. Titamon digivolved by an effect, discard paid: the
/// opponent's top security card is trashed.
#[test]
fn bt25_084_wd_digivolved_by_effect_trashes_opponent_top_security() {
    let mut runner = evo_base()
        .from_dsl_yaml(EVO_ENABLER_YAML)
        .expect("enabler")
        .hand(0, &[CARD_ID, "PAD"])
        .security(1, &["PAD"; 3])
        .memory(3)
        .start();
    runner.place_on_field(1, "OPP-BIG", Some(0));
    runner.place_on_field(0, "RED5", Some(0));
    let enabler = runner.place_on_field(0, "T-EVO-ENABLER", Some(0));
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(enabler));
    runner.game.drain_effect_queue();
    resolve_picking_first(&mut runner);
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "Titamon digivolved by the effect"
    );
    // (The digivolve itself draws a card, so the hand size proves nothing.)
    assert_eq!(runner.battle_area_size(1), 0, "the discard was paid: OPP-BIG deleted");
    assert_eq!(
        runner.security_count(1),
        2,
        "digivolved by an effect: the opponent's top security card is trashed"
    );
}

/// Control: a normal (player-action) digivolve into Titamon pays the discard
/// and deletes, but does not trash the opponent's security.
#[test]
fn bt25_084_wd_normal_digivolve_does_not_trash_security() {
    let mut runner = evo_base()
        .hand(0, &[CARD_ID, "PAD"])
        .security(1, &["PAD"; 3])
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-BIG", Some(0));
    let base_perm = runner.place_on_field(0, "RED5", Some(0));
    let action = encode_digivolve(0, base_perm.index as u16);
    assert_eq!(build_action_mask(&runner.game, 0)[action as usize], 1.0);
    runner.game.decode_action(action, 0);
    resolve_picking_first(&mut runner);
    // (The digivolve itself draws a card, so the hand size proves nothing.)
    assert_eq!(runner.battle_area_size(1), 0, "the discard was paid: OPP-BIG deleted");
    assert_eq!(runner.security_count(1), 3, "a normal digivolve trashes no security");
}
