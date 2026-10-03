//! EX13-044 Breakdramon — Digimon, Lv.6, Green/Red, DP 12000, Cost 12.
//! Traits: Machine Dragon. Form: Mega. Attribute: Virus.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-044.json` + `card_overrides.json`;
//! official Bandai DB bundle `data/card_bundles/EX13-044.md` agrees)
//!
//! ```text
//! Digivolve: Green Lv.5 / cost 4; Red Lv.5 / cost 4.
//! [Digivolve] [Groundramon]/[Wingdramon]: Cost 3
//! Assembly -5: Lv.5 × Lv.4 × Lv.3, all w/[Dracomon]/[Examon] in text
//!   (When this would be played, by placing the specified cards from the trash
//!   under it, reduce the play cost.)
//!
//! Effect:
//! <Piercing> <Blocker>
//! [On Play] [When Digivolving] You may suspend up to 2 Digimon or Tamers.
//! Then, 2 of your opponent's Digimon or Tamers can't unsuspend until their
//! turn ends.
//! [All Turns] [Once Per Turn] When any of your Digimon suspend, 1 of your
//! Digimon with [Dracomon] or [Examon] in its text may battle 1 of your
//! opponent's Digimon.
//!
//! Inherited Effect:
//! [All Turns] [Once Per Turn] When any of your Digimon suspend, 1 of your
//! Digimon with [Dracomon] or [Examon] in its text may battle 1 of your
//! opponent's Digimon.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Green/EX13_044.cs
//! - Alt digivolve `EqualsCardName("Groundramon") || ("Wingdramon")`, cost 3.
//! - `PierceSelfEffect`, `BlockerSelfStaticEffect`.
//! - `AddAssemblyConditionClass`: elements Lv.5 ×1, Lv.4 ×1, Lv.3 ×1, each
//!   own card with `HasText("Dracomon") || HasText("Examon")`, reduceCost 5.
//! - Shared [OP]/[WD] (optional: false): `SelectPermanentEffect(Tap, max
//!   min(2, unsuspended Digimon/Tamers on EITHER side), canNoSelect: true,
//!   canEndNotMax: true)`, then `SelectPermanentEffect(max min(2, opponent
//!   Digimon/Tamers), canNoSelect: false)` → `GainCanNotUnsuspend(
//!   UntilOpponentTurnEnd)`.
//! - `OnTappedAnyone` ×2 (face + inherited, separate hash → separate OPT):
//!   `CanTriggerWhenPermanentSuspends(own Digimon)`; activate only if an own
//!   [Dracomon]/[Examon]-text Digimon AND an opponent Digimon exist; pick
//!   battler then defender (both canNoSelect) → `IBattle`; `RemoveUse()` when
//!   no battle happened (declining does not burn the once-per-turn).
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - H3 Piercing / Blocker face keywords.
//! - Assembly (3 per-level trash materials, "[X] in its text").
//! - "up to 2" any-side suspend (optional) + mandatory clamp-to-available
//!   2-target CannotUnsuspend lock (EX12-063 / BT24-051 idioms).
//! - E2 [All Turns][OPT] own-Digimon-suspend observer → chosen battler battles
//!   a chosen opponent Digimon (`battle:` step, EX11-074 idiom), face +
//!   inherited copies.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_engine::action::space::{
    encode_attack, encode_digivolve, PASS, PLAY_HAND_START, TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "EX13-044";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, name: &str, level: u8, dp: i32, text: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = level as u16;
    c.colors = vec![CardColor::Green];
    c.effect_text = text.to_string();
    c
}

fn builder() -> DebugRunnerBuilder {
    let mut tamer = make_test_card("OPP-TAMER", "Opp Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    let mut own_tamer = make_test_card("OWN-TAMER", "Own Tamer");
    own_tamer.card_kind = CardKind::Tamer;
    own_tamer.level = None;
    own_tamer.dp = None;
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-044 YAML parses, compiles and is in the embedded pack")
        // Assembly materials.
        .add_card(digimon("MAT-L5", "Text Five", 5, 7000, "Treated as [Examon] material."))
        .add_card(digimon("MAT-L4", "Dracomon Four", 4, 5000, ""))
        .add_card(digimon("MAT-L3", "Dracomon Three", 3, 3000, ""))
        .add_card(digimon("PLAIN-L4", "Plain Four", 4, 5000, ""))
        // Board.
        .add_card(digimon("OWN-DRACO", "Dracomon Striker", 4, 9000, ""))
        .add_card(digimon("OWN-PLAIN", "Plain Striker", 4, 9000, ""))
        .add_card(digimon("OPP-A", "Opp A", 3, 3000, ""))
        .add_card(digimon("OPP-B", "Opp B", 4, 4000, ""))
        .add_card(tamer)
        .add_card(own_tamer)
        .add_card(digimon("FILL", "Fill", 3, 2000, ""))
        .add_card({
            // Yellow: only the "[Groundramon]/[Wingdramon]" route applies to it.
            let mut c = digimon("GROUNDRAMON", "Groundramon", 5, 7000, "");
            c.colors = vec![CardColor::Yellow];
            c
        })
}

fn start(builder: DebugRunnerBuilder) -> DebugRunner {
    let mut runner = builder
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .start();
    runner.skip_mulligan();
    runner
}

fn hand_index(runner: &DebugRunner, player: u8, id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be in player {player}'s hand"))
}

fn field_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn find_perm(runner: &DebugRunner, player: u8, id: &str) -> PermanentHandle {
    let index = field_ids(runner, player)
        .iter()
        .position(|f| f == id)
        .unwrap_or_else(|| panic!("{id} must be on player {player}'s field"));
    PermanentHandle {
        player,
        index: index as u8,
    }
}

fn is_suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

/// Action id for a pick over BOTH players' fields (`select_any_permanent`).
fn perm_action(h: PermanentHandle) -> u16 {
    encode_attack(h.player as u16, h.index as u16)
}

/// Action id for a single-side field pick (own-only / opponent-only prompts).
fn side_action(h: PermanentHandle) -> u16 {
    encode_attack(0, h.index as u16)
}

/// Pick `target` (either side) at the optional "suspend up to 2" prompt.
/// DCGO: one SelectPermanentEffect with canNoSelect / canEndNotMax, so each
/// pick is a single `select_any_permanent` step and PASS stops early.
fn suspend_pick(runner: &mut DebugRunner, target: PermanentHandle) {
    pick(runner, perm_action(target));
}

fn pick(runner: &mut DebugRunner, action: u16) {
    let view = runner.pending_selection_view().expect("a prompt must be pending");
    assert!(
        view.valid_action_ids.contains(&action),
        "action {action} must be legal; view={view:?}"
    );
    runner
        .execute_action(view.selecting_player, action)
        .expect("pick");
}

fn pass(runner: &mut DebugRunner) {
    let view = runner.pending_selection_view().expect("a prompt must be pending");
    assert!(view.is_optional, "PASS must be legal here; view={view:?}");
    runner.execute_action(view.selecting_player, PASS).expect("pass");
}

fn accept(runner: &mut DebugRunner) {
    let view = runner.pending_selection_view().expect("a prompt must be pending");
    let action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("non-PASS option");
    runner
        .execute_action(view.selecting_player, action)
        .expect("accept");
}

fn triggered(runner: &DebugRunner) -> Vec<CompiledTriggeredClause> {
    runner
        .compiled_card(CARD_ID)
        .expect("compiled")
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_044_has_piercing_and_blocker() {
    let mut runner = start(builder());
    let h = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(runner.game.has_keyword(h, Keyword::Piercing), "<Piercing>");
    assert!(runner.game.has_keyword(h, Keyword::Blocker), "<Blocker>");
}

#[test]
fn ex13_044_alt_paths_match_printed_card() {
    let runner = start(builder());
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let digivolves = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(digivolves, 3, "Green Lv.5/4 + Red Lv.5/4 + [Groundramon]/[Wingdramon]/3");
    let assembly = card
        .alt_paths
        .iter()
        .find(|p| matches!(p.kind, CompiledAltPathKind::Assembly))
        .expect("Assembly path");
    assert_eq!(assembly.cost, Some(CompiledCost::Literal(5)), "Assembly -5");
    assert_eq!(assembly.materials.len(), 3, "Lv.5 × Lv.4 × Lv.3");
}

#[test]
fn ex13_044_clause_shape_matches_printed_text() {
    let runner = start(builder());
    let t = triggered(&runner);
    let shared = t
        .iter()
        .find(|c| c.when.contains(&CompiledTiming::OnPlay))
        .expect("[On Play] clause");
    assert!(shared.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(!shared.optional, "mandatory clause (the suspend half is optional inside)");
    let observers: Vec<_> = t
        .iter()
        .filter(|c| c.when.contains(&CompiledTiming::OnSuspend))
        .collect();
    assert_eq!(observers.len(), 2, "face + inherited suspend observers");
    assert!(observers.iter().all(|c| c.once_per_turn), "[Once Per Turn]");
    assert!(observers.iter().all(|c| c.optional), "\"may battle\"");
    assert!(observers.iter().any(|c| c.scope == CompiledScope::FaceUp));
    assert!(observers.iter().any(|c| c.scope == CompiledScope::Inherited));
}

// ─── Section 3 — Assembly -5 ─────────────────────────────────────────────────

#[test]
fn ex13_044_assembly_places_three_text_materials_and_costs_7() {
    let mut runner = start(builder().hand(0, &[CARD_ID]).memory(10));
    runner.inject_trash(0, "MAT-L5");
    runner.inject_trash(0, "MAT-L4");
    runner.inject_trash(0, "MAT-L3");
    let memory_before = runner.game.memory;
    let slot = hand_index(&runner, 0, CARD_ID) as u16;
    runner.game.decode_action(PLAY_HAND_START + slot, 0);
    for _ in 0..3 {
        let view = runner
            .pending_selection_view()
            .expect("each Assembly element is chosen from trash");
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a >= TRASH_EFFECT_START && a != PASS)
            .expect("a trash material pick");
        runner.execute_action(view.selecting_player, action).expect("pick material");
    }
    // Decline the optional suspend; nothing to lock on an empty opponent board.
    if runner.pending_selection().is_some() {
        pass(&mut runner);
    }
    let _ = runner.auto_resolve();

    let h = find_perm(&runner, 0, CARD_ID);
    assert_eq!(
        runner.game.players[0].battle_area[h.index as usize].card_sources.len(),
        4,
        "three materials under Breakdramon"
    );
    assert!(runner.game.players[0].trash.is_empty());
    assert_eq!(memory_before - runner.game.memory, 7, "12 − 5 = 7");
}

#[test]
fn ex13_044_assembly_needs_the_text_on_every_material() {
    // Lv.4 without [Dracomon]/[Examon] text: the recipe is incomplete.
    let mut runner = start(builder().hand(0, &[CARD_ID]).memory(10));
    runner.inject_trash(0, "MAT-L5");
    runner.inject_trash(0, "PLAIN-L4");
    runner.inject_trash(0, "MAT-L3");
    let slot = hand_index(&runner, 0, CARD_ID) as u16;
    runner.game.decode_action(PLAY_HAND_START + slot, 0);
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].trash.len(), 3, "no material left the trash");
    let h = find_perm(&runner, 0, CARD_ID);
    assert_eq!(
        runner.game.players[0].battle_area[h.index as usize].card_sources.len(),
        1,
        "played at full cost with nothing placed under it"
    );
}

// ─── Section 3 — [On Play] suspend up to 2, lock 2 ───────────────────────────

/// Play Breakdramon from hand (via Assembly -5, so the turn does not pass).
fn play_breakdramon(opp: &[&str], own: &[&str]) -> DebugRunner {
    let mut runner = start(builder().hand(0, &[CARD_ID]).memory(10));
    for id in own {
        runner.place_on_field(0, id, Some(0));
    }
    for id in opp {
        runner.place_on_field(1, id, Some(0));
    }
    runner.inject_trash(0, "MAT-L5");
    runner.inject_trash(0, "MAT-L4");
    runner.inject_trash(0, "MAT-L3");
    let slot = hand_index(&runner, 0, CARD_ID) as u16;
    runner.game.decode_action(PLAY_HAND_START + slot, 0);
    for _ in 0..3 {
        let view = runner.pending_selection_view().expect("Assembly material prompt");
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a >= TRASH_EFFECT_START && a != PASS)
            .expect("a trash material pick");
        runner.execute_action(view.selecting_player, action).expect("pick material");
    }
    assert!(field_ids(&runner, 0).contains(&CARD_ID.to_string()), "Breakdramon is in play");
    runner
}

fn locked(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.modifiers().has(h, ModifierType::CannotUnsuspend)
}

fn lock_first_available(runner: &mut DebugRunner) {
    for _ in 0..3 {
        if runner.pending_selection().is_none() {
            return;
        }
        accept(runner);
    }
}

#[test]
fn ex13_044_on_play_suspends_two_chosen_permanents_then_locks_two_opponents() {
    let mut runner = play_breakdramon(&["OPP-A", "OPP-B", "OPP-TAMER"], &[]);
    let opp_a = find_perm(&runner, 1, "OPP-A");
    let tamer = find_perm(&runner, 1, "OPP-TAMER");
    suspend_pick(&mut runner, opp_a);
    suspend_pick(&mut runner, tamer);
    assert!(is_suspended(&runner, opp_a));
    assert!(is_suspended(&runner, tamer), "Tamers can be suspended too");
    lock_first_available(&mut runner);
    let _ = runner.auto_resolve();

    let locks = ["OPP-A", "OPP-B", "OPP-TAMER"]
        .iter()
        .filter(|id| locked(&runner, find_perm(&runner, 1, id)))
        .count();
    assert_eq!(locks, 2, "exactly 2 opponent Digimon/Tamers can't unsuspend");
}

#[test]
fn ex13_044_on_play_suspend_may_target_your_own_permanents() {
    let mut runner = play_breakdramon(&["OPP-A"], &["OWN-PLAIN"]);
    let own = find_perm(&runner, 0, "OWN-PLAIN");
    let view = runner.pending_selection_view().expect("suspend prompt");
    assert!(view.valid_action_ids.contains(&perm_action(own)), "\"any Digimon or Tamers\"");
    let opp = find_perm(&runner, 1, "OPP-A");
    assert!(view.valid_action_ids.contains(&perm_action(opp)));
    pick(&mut runner, perm_action(own));
    assert!(is_suspended(&runner, own));
}

#[test]
fn ex13_044_on_play_suspend_is_optional_but_the_lock_still_happens() {
    let mut runner = play_breakdramon(&["OPP-A", "OPP-B"], &[]);
    pass(&mut runner); // "Don't suspend"
    let opp_a = find_perm(&runner, 1, "OPP-A");
    let opp_b = find_perm(&runner, 1, "OPP-B");
    assert!(!is_suspended(&runner, opp_a));
    lock_first_available(&mut runner);
    let _ = runner.auto_resolve();
    assert!(locked(&runner, opp_a) && locked(&runner, opp_b), "both locked");
}

#[test]
fn ex13_044_on_play_can_stop_after_one_suspend() {
    let mut runner = play_breakdramon(&["OPP-A", "OPP-B"], &[]);
    let opp_a = find_perm(&runner, 1, "OPP-A");
    let opp_b = find_perm(&runner, 1, "OPP-B");
    suspend_pick(&mut runner, opp_a);
    pass(&mut runner); // "up to 2": stop after one
    assert!(is_suspended(&runner, opp_a));
    assert!(!is_suspended(&runner, opp_b));
}

#[test]
fn ex13_044_on_play_lock_clamps_to_one_opponent_permanent() {
    let mut runner = play_breakdramon(&["OPP-A"], &[]);
    pass(&mut runner);
    lock_first_available(&mut runner);
    let _ = runner.auto_resolve();
    assert!(locked(&runner, find_perm(&runner, 1, "OPP-A")));
    assert!(runner.pending_selection().is_none());
}

#[test]
fn ex13_044_lock_prevents_unsuspending_through_the_opponents_turn() {
    let mut runner = play_breakdramon(&["OPP-A"], &[]);
    let opp_a = find_perm(&runner, 1, "OPP-A");
    suspend_pick(&mut runner, opp_a);
    pass(&mut runner);
    lock_first_available(&mut runner);
    let _ = runner.auto_resolve();
    runner.end_turn(); // opponent's unsuspend phase
    let _ = runner.auto_resolve();
    assert!(is_suspended(&runner, find_perm(&runner, 1, "OPP-A")), "stays suspended");
}

#[test]
fn ex13_044_digivolving_from_groundramon_costs_3_and_fires_when_digivolving() {
    let mut runner = start(builder().hand(0, &[CARD_ID]).memory(10));
    let base = runner.place_on_field(0, "GROUNDRAMON", Some(0));
    let opp = runner.place_on_field(1, "OPP-A", Some(0));
    let memory_before = runner.game.memory;
    let slot = hand_index(&runner, 0, CARD_ID) as u16;
    runner
        .game
        .decode_action(encode_digivolve(slot, base.index as u16), 0);
    assert_eq!(memory_before - runner.game.memory, 3, "[Groundramon]/[Wingdramon]: Cost 3");
    suspend_pick(&mut runner, opp);
    assert!(is_suspended(&runner, opp), "[When Digivolving] runs the same clause");
    // The only unsuspended permanent left is Breakdramon itself: stop.
    if runner
        .pending_selection_view()
        .is_some_and(|v| v.is_optional)
    {
        pass(&mut runner);
    }
    lock_first_available(&mut runner);
    let _ = runner.auto_resolve();
    assert!(locked(&runner, opp));
}

// ─── Section 2/3/5 — [All Turns][OPT] suspend → battle ───────────────────────

fn battle_board(stack_under: bool) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut runner = start(builder().memory(5));
    if stack_under {
        // Inherited copy: Breakdramon as a digivolution card under the battler.
        runner.place_stack(0, &[CARD_ID, "OWN-DRACO"]);
    } else {
        runner.place_on_field(0, CARD_ID, Some(0));
        runner.place_on_field(0, "OWN-DRACO", Some(0));
    }
    runner.place_on_field(0, "OWN-PLAIN", Some(0));
    runner.place_on_field(0, "OWN-TAMER", Some(0));
    runner.place_on_field(1, "OPP-A", Some(0));
    let draco = find_perm(&runner, 0, "OWN-DRACO");
    let opp = find_perm(&runner, 1, "OPP-A");
    (runner, draco, opp)
}

fn do_battle(runner: &mut DebugRunner, battler: PermanentHandle, defender: PermanentHandle) {
    accept(runner); // "may battle" gate
    pick(runner, side_action(battler));
    pick(runner, side_action(defender));
    let _ = runner.auto_resolve();
}

#[test]
fn ex13_044_own_digimon_suspending_lets_a_text_digimon_battle() {
    let (mut runner, draco, opp) = battle_board(false);
    let plain = find_perm(&runner, 0, "OWN-PLAIN");
    runner.game.suspend(plain);
    do_battle(&mut runner, draco, opp);
    assert!(!field_ids(&runner, 1).contains(&"OPP-A".to_string()), "9000 beats 3000");
    assert!(field_ids(&runner, 0).contains(&"OWN-DRACO".to_string()));
}

#[test]
fn ex13_044_battler_must_have_dracomon_or_examon_text() {
    let (mut runner, _draco, _opp) = battle_board(false);
    let plain = find_perm(&runner, 0, "OWN-PLAIN");
    runner.game.suspend(plain);
    accept(&mut runner);
    let view = runner.pending_selection_view().expect("battler prompt");
    assert!(
        !view.valid_action_ids.contains(&side_action(plain)),
        "a Digimon without the text can't be the battler"
    );
}

#[test]
fn ex13_044_opponent_digimon_suspending_does_not_trigger() {
    let (mut runner, _draco, opp) = battle_board(false);
    runner.game.suspend(opp);
    assert!(runner.pending_selection().is_none(), "only YOUR Digimon suspending");
}

#[test]
fn ex13_044_own_tamer_suspending_does_not_trigger() {
    let (mut runner, _draco, _opp) = battle_board(false);
    let tamer = find_perm(&runner, 0, "OWN-TAMER");
    runner.game.suspend(tamer);
    assert!(runner.pending_selection().is_none(), "only Digimon suspending");
}

#[test]
fn ex13_044_no_opponent_digimon_means_no_prompt() {
    let mut runner = start(builder().memory(5));
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "OWN-DRACO", Some(0));
    let plain = runner.place_on_field(0, "OWN-PLAIN", Some(0));
    runner.place_on_field(1, "OPP-TAMER", Some(0));
    runner.game.suspend(plain);
    assert!(runner.pending_selection().is_none(), "nothing to battle");
}

#[test]
fn ex13_044_battle_is_once_per_turn_and_resets_next_turn() {
    let (mut runner, draco, _opp) = battle_board(false);
    runner.place_on_field(1, "OPP-B", Some(0));
    let plain = find_perm(&runner, 0, "OWN-PLAIN");
    runner.game.suspend(plain);
    let opp_a = find_perm(&runner, 1, "OPP-A");
    do_battle(&mut runner, draco, opp_a);

    // Second suspend in the same turn: the face copy is spent.
    let draco = find_perm(&runner, 0, "OWN-DRACO");
    runner.game.suspend(draco);
    assert!(runner.pending_selection().is_none(), "[Once Per Turn]");

    // Lockout clears after the turn passes back.
    runner.end_turn();
    let _ = runner.auto_resolve();
    runner.end_turn();
    let _ = runner.auto_resolve();
    let plain = find_perm(&runner, 0, "OWN-PLAIN");
    if is_suspended(&runner, plain) {
        panic!("unsuspend phase should have readied OWN-PLAIN");
    }
    runner.game.suspend(plain);
    assert!(runner.pending_selection().is_some(), "usable again next turn");
}

#[test]
fn ex13_044_declining_the_battle_does_not_spend_the_once_per_turn() {
    let (mut runner, draco, opp) = battle_board(false);
    let plain = find_perm(&runner, 0, "OWN-PLAIN");
    runner.game.suspend(plain);
    pass(&mut runner);
    let _ = runner.auto_resolve();
    assert!(field_ids(&runner, 1).contains(&"OPP-A".to_string()));
    runner.game.suspend(draco);
    assert!(
        runner.pending_selection().is_some(),
        "no battle happened, so the effect is still available (DCGO RemoveUse)"
    );
    do_battle(&mut runner, draco, opp);
    assert!(!field_ids(&runner, 1).contains(&"OPP-A".to_string()));
}

#[test]
fn ex13_044_inherited_copy_triggers_from_under_a_digimon() {
    let (mut runner, draco, opp) = battle_board(true);
    let plain = find_perm(&runner, 0, "OWN-PLAIN");
    runner.game.suspend(plain);
    do_battle(&mut runner, draco, opp);
    assert!(!field_ids(&runner, 1).contains(&"OPP-A".to_string()));
}
