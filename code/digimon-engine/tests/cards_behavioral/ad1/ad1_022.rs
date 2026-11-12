//! AD1-022 Izzy Izumi & Tai Kamiya — Tamer, Green+Red, Cost 3.
//! Traits: ADVENTURE / Hero.
//!
//! # Card text (official bundle `data/card_bundles/AD1-022.md`)
//!
//! ```text
//! [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
//! [Your Turn] When any of your other [ADVENTURE] trait Digimon or Tamers are
//! played, by suspending this Tamer, 1 of your Digimon may digivolve into an
//! [ADVENTURE] trait Digimon card in the hand. For every 2 of your Tamers'
//! colors, reduce this effect's digivolution cost by 1.
//!
//! Security Effect: [Security] Play this card without paying the cost.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/AD1/Green/AD1_022.cs` —
//! `Gain1MemoryTamerOpponentDigimonEffect`; OnEnterFieldAnyone (optional,
//! owner turn, `CanTriggerOnPermanentPlay` for another own ADVENTURE
//! Digimon/Tamer, `CanActivateSuspendCostEffect`): suspend self, then
//! `reduceCost = distinct colours of all own Tamers / 2`, select 1 own Digimon
//! (canNoSelect) → `DigivolveIntoHandOrTrashCard(IsDigimon && ADVENTURE,
//! payCost: true, reduceCost)`; `PlaySelfTamerSecurityEffect`.

#![allow(dead_code)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledColor, CompiledTiming};
use digimon_engine::action::space::{encode_attack, HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "AD1-022";

fn digimon(id: &str, level: u8, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(level);
    c.play_cost = 3;
    if adventure {
        c.traits = vec!["ADVENTURE".to_string()];
    }
    c
}

fn tamer(id: &str, color: CardColor, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = vec![color];
    c.play_cost = 2;
    if adventure {
        c.traits = vec!["ADVENTURE".to_string()];
    }
    c
}

/// Lv.4 that digivolves from a red Lv.3 for 3.
fn evo(id: &str, adventure: bool) -> CardData {
    let mut c = digimon(id, 4, adventure);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.evo_costs = vec![EvoCost { card_color: 0, level: 3, memory_cost: 3 }];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("AD1-022 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("BASE", 3, false))
        .add_card(digimon("ADV-ROOKIE", 3, true))
        .add_card(digimon("PLAIN-ROOKIE", 3, false))
        .add_card(tamer("ADV-TAMER", CardColor::Blue, true))
        .add_card(tamer("T-YELLOW", CardColor::Yellow, false))
        .add_card(tamer("T-BLUE", CardColor::Blue, false))
        .add_card(evo("ADV-EVO", true))
        .add_card(evo("PLAIN-EVO", false))
        .add_card(make_test_card("FILL", "Fill"))
}

fn hand_index(runner: &DebugRunner, id: &str) -> usize {
    runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be in hand"))
}

fn hand_actions(runner: &DebugRunner, id: &str) -> [u16; 2] {
    let slot = hand_index(runner, id) as u16;
    [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
}

fn top_id(runner: &DebugRunner, h: PermanentHandle) -> String {
    runner.game.players[h.player as usize].battle_area[h.index as usize].top_card().card_id(&runner.game.card_data).to_string()
}

fn suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

struct Setup {
    runner: DebugRunner,
    tamer: PermanentHandle,
    base: PermanentHandle,
}

/// AD1-022 + BASE (red Lv.3) + extra Tamers on P0's field; `hand` in hand.
fn setup(extra_tamers: &[&str], hand: &[&str]) -> Setup {
    let mut runner = builder()
        .hand(0, hand)
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    for t in extra_tamers {
        runner.place_on_field(0, t, Some(0));
    }
    Setup { runner, tamer, base }
}

fn play(runner: &mut DebugRunner, id: &str) {
    let slot = hand_index(runner, id);
    runner.play(0, slot).expect("plays");
}

/// Accept the outer optional trigger (pays the suspend cost).
fn accept_trigger(runner: &mut DebugRunner) {
    let view = runner.pending_selection_view().expect("optional trigger prompt");
    assert!(view.is_optional, "the trigger is optional (by suspending ... may)");
    assert!(
        !matches!(view.kind, SelectionKind::OwnField | SelectionKind::Hand),
        "first prompt is the use-the-effect gate; got {view:?}"
    );
    runner.accept_optional_trigger().expect("accept");
}

fn pick_own(runner: &mut DebugRunner, h: PermanentHandle) {
    let view = runner.pending_selection_view().expect("own-Digimon prompt");
    assert_eq!(view.kind, SelectionKind::OwnField);
    let action = encode_attack(0, h.index as u16);
    assert!(view.valid_action_ids.contains(&action), "view={view:?}");
    runner.execute_action(view.selecting_player, action).expect("pick base");
}

fn pick_hand(runner: &mut DebugRunner, id: &str) {
    let view = runner.pending_selection_view().expect("hand prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    let action = hand_actions(runner, id)
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
        .unwrap_or_else(|| panic!("{id} must be selectable; view={view:?}"));
    runner.execute_action(view.selecting_player, action).expect("pick evo");
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ad1_022_identity_and_clauses() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(3));
    assert!(card.color.contains(&CompiledColor::Green) && card.color.contains(&CompiledColor::Red));
    assert!(card.traits.iter().any(|t| t == "ADVENTURE") && card.traits.iter().any(|t| t == "Hero"));
    let timings: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t.when.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        timings,
        vec![
            vec![CompiledTiming::StartOfYourMainPhase],
            vec![CompiledTiming::OnAllyPlayed],
            vec![CompiledTiming::OnSecurity],
        ]
    );
}

// ─── Section 2 — [Start of Your Main Phase] ──────────────────────────────────

#[test]
fn ad1_022_start_of_main_gains_memory_only_if_opponent_has_a_digimon() {
    for (opp_digimon, expected) in [(true, 1), (false, 0)] {
        let mut runner = builder().deck(0, &["FILL"]).deck(1, &["FILL"]).memory(0).start();
        runner.place_on_field(0, CARD_ID, Some(0));
        if opp_digimon {
            runner.place_on_field(1, "BASE", Some(0));
        }
        runner.game.memory = 0;
        runner.game.enter_main_phase();
        let _ = runner.auto_resolve();
        assert_eq!(runner.memory(), expected, "opp has Digimon: {opp_digimon}");
    }
}

// ─── Section 3 — [Your Turn] ally-played → suspend → digivolve ───────────────

#[test]
fn ad1_022_adventure_digimon_played_lets_a_digimon_digivolve_with_reduced_cost() {
    let Setup { mut runner, tamer, base } = setup(&[], &["ADV-ROOKIE", "ADV-EVO"]);
    let before = runner.memory();
    play(&mut runner, "ADV-ROOKIE");
    accept_trigger(&mut runner);
    assert!(suspended(&runner, tamer), "suspend cost paid");
    pick_own(&mut runner, base);
    pick_hand(&mut runner, "ADV-EVO");
    let _ = runner.auto_resolve();
    assert_eq!(top_id(&runner, base), "ADV-EVO");
    // AD1-022 is the only Tamer: Green+Red = 2 colours → reduce 1 (3 → 2).
    assert_eq!(before - runner.memory(), 3 + 2, "play 3 + digivolve 3-1");
}

#[test]
fn ad1_022_four_tamer_colors_reduce_by_two() {
    let Setup { mut runner, base, .. } = setup(&["T-YELLOW", "T-BLUE"], &["ADV-ROOKIE", "ADV-EVO"]);
    let before = runner.memory();
    play(&mut runner, "ADV-ROOKIE");
    accept_trigger(&mut runner);
    pick_own(&mut runner, base);
    pick_hand(&mut runner, "ADV-EVO");
    let _ = runner.auto_resolve();
    assert_eq!(top_id(&runner, base), "ADV-EVO");
    assert_eq!(before - runner.memory(), 3 + 1, "4 colours → reduce 2 (3 → 1)");
}

#[test]
fn ad1_022_three_tamer_colors_still_reduce_by_one() {
    let Setup { mut runner, base, .. } = setup(&["T-BLUE"], &["ADV-ROOKIE", "ADV-EVO"]);
    let before = runner.memory();
    play(&mut runner, "ADV-ROOKIE");
    accept_trigger(&mut runner);
    pick_own(&mut runner, base);
    pick_hand(&mut runner, "ADV-EVO");
    let _ = runner.auto_resolve();
    assert_eq!(before - runner.memory(), 3 + 2, "3 colours → floor(3/2) = 1");
}

#[test]
fn ad1_022_adventure_tamer_played_also_triggers() {
    let Setup { mut runner, tamer, base } = setup(&[], &["ADV-TAMER", "ADV-EVO"]);
    play(&mut runner, "ADV-TAMER");
    accept_trigger(&mut runner);
    assert!(suspended(&runner, tamer));
    pick_own(&mut runner, base);
    pick_hand(&mut runner, "ADV-EVO");
    let _ = runner.auto_resolve();
    assert_eq!(top_id(&runner, base), "ADV-EVO");
}

#[test]
fn ad1_022_non_adventure_play_does_not_trigger() {
    let Setup { mut runner, tamer, base } = setup(&[], &["PLAIN-ROOKIE", "ADV-EVO"]);
    play(&mut runner, "PLAIN-ROOKIE");
    assert!(runner.pending_selection().is_none());
    assert!(!suspended(&runner, tamer));
    assert_eq!(top_id(&runner, base), "BASE");
}

#[test]
fn ad1_022_declining_keeps_the_tamer_unsuspended() {
    let Setup { mut runner, tamer, base } = setup(&[], &["ADV-ROOKIE", "ADV-EVO"]);
    play(&mut runner, "ADV-ROOKIE");
    runner.decline_optional_trigger().expect("decline");
    let _ = runner.auto_resolve();
    assert!(!suspended(&runner, tamer));
    assert_eq!(top_id(&runner, base), "BASE");
}

#[test]
fn ad1_022_digivolve_choice_may_be_passed_after_paying() {
    let Setup { mut runner, tamer, base } = setup(&[], &["ADV-ROOKIE", "ADV-EVO"]);
    play(&mut runner, "ADV-ROOKIE");
    accept_trigger(&mut runner);
    let view = runner.pending_selection_view().expect("own-Digimon prompt");
    assert!(view.is_optional, "'may digivolve'");
    runner.execute_action(view.selecting_player, PASS).expect("pass");
    let _ = runner.auto_resolve();
    assert!(suspended(&runner, tamer));
    assert_eq!(top_id(&runner, base), "BASE");
}

#[test]
fn ad1_022_only_adventure_digimon_cards_are_offered() {
    let Setup { mut runner, base, .. } = setup(&[], &["ADV-ROOKIE", "ADV-EVO", "PLAIN-EVO"]);
    play(&mut runner, "ADV-ROOKIE");
    accept_trigger(&mut runner);
    pick_own(&mut runner, base);
    let view = runner.pending_selection_view().expect("hand prompt");
    assert!(!hand_actions(&runner, "PLAIN-EVO").iter().any(|a| view.valid_action_ids.contains(a)));
    assert!(hand_actions(&runner, "ADV-EVO").iter().any(|a| view.valid_action_ids.contains(a)));
}

#[test]
fn ad1_022_suspended_tamer_cannot_pay() {
    let Setup { mut runner, tamer, base } = setup(&[], &["ADV-ROOKIE", "ADV-EVO"]);
    runner.game.players[0].battle_area[tamer.index as usize].is_suspended = true;
    play(&mut runner, "ADV-ROOKIE");
    assert!(runner.pending_selection().is_none(), "no suspend cost available");
    assert_eq!(top_id(&runner, base), "BASE");
}

#[test]
fn ad1_022_does_not_trigger_on_the_opponents_turn() {
    let Setup { mut runner, tamer, .. } = setup(&[], &["ADV-EVO"]);
    runner.end_turn();
    let _ = runner.auto_resolve();
    let ally = runner.place_on_field(0, "ADV-ROOKIE", None);
    runner.fire_play_event_triggers(0, ally.index as usize, true, false);
    assert!(runner.pending_selection().is_none(), "[Your Turn]");
    assert!(!suspended(&runner, tamer));
}

#[test]
fn ad1_022_playing_itself_does_not_trigger() {
    let mut runner = builder()
        .hand(0, &[CARD_ID, "ADV-EVO"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    runner.place_on_field(0, "BASE", Some(0));
    play(&mut runner, CARD_ID);
    assert!(runner.pending_selection().is_none(), "'other' excludes AD1-022 itself");
}
