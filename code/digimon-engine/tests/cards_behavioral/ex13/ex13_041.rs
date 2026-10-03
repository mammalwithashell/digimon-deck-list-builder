//! EX13-041 Groundramon — Digimon, Lv.5, Green/Red, DP 7000, Cost 7.
//! Traits: Earth Dragon. Form: Ultimate. Attribute: Virus.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-041.md`)
//!
//! Digivolve: Green Lv.4 / 4; Red Lv.4 / 4.  [Digivolve] [Coredramon]: Cost 3
//! ＜Fortitude＞
//! [On Play] [When Digivolving] Suspend 1 of your opponent's Digimon or
//! Tamers. Then, 1 of their Digimon or Tamers can't unsuspend in their next
//! unsuspend phase.
//! [All Turns] This Digimon is also treated as Lv.6 [Breakdramon] for
//! [Examon]'s DNA digivolution.
//! Inherited: [All Turns] [Once Per Turn] When any of your Digimon with
//! [Dracomon] or [Examon] in their texts delete your opponent's Digimon in
//! battle, trash their top security card.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Green/EX13_041.cs — shared [OP]/[WD]
//! (Tap pick, then `GainCanNotUnsuspendPlayerEffect(isOnlyActivePhase: true,
//! UntilOwnerActivePhase)`), `AddJogressLevelsClass` + `ChangeCardNamesClass`
//! (+[Breakdramon]), inherited `OnEndBattle` OPT
//! `CanTriggerWhenDeleteOpponentDigimonByBattle` → `IDestroySecurity`.
//!
//! # Patterns
//! - `CannotUnsuspendInUnsuspendPhase` (unsuspend-phase-only lock).
//! - G-DNA-MATERIAL-TREATED-AS-FOR-TARGET (`dna_material_identity` aura).
//! - G-DSL-BATTLE-DELETER (`event_battle_deleter` on `on_ally_won_battle`).

#![allow(dead_code)]

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_digivolve, ATTACK_START, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Expiry, GamePhase, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-041";
const EXAMON: &str = "EX13-045";

fn digimon(id: &str, name: &str, level: u8, dp: i32, color: CardColor, text: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = level as u16 + 2;
    c.colors = vec![color];
    c.effect_text = text.to_string();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-041 YAML loads")
        .dsl_card(EXAMON)
        .expect("EX13-045 YAML loads")
        .add_card(digimon("GREEN4", "Green Four", 4, 4000, CardColor::Green, ""))
        .add_card(digimon("CORE", "Coredramon", 4, 5000, CardColor::Blue, ""))
        .add_card(digimon("BLUE6", "Blue Six", 6, 11000, CardColor::Blue, ""))
        .add_card(digimon("RED6", "Red Six", 6, 11000, CardColor::Red, ""))
        .add_card(digimon("DRACO-TOP", "Draco Top", 6, 11000, CardColor::Green, "[Examon] support."))
        .add_card(digimon("DRACO-ALLY", "Draco Ally", 6, 11000, CardColor::Red, "Search [Dracomon]."))
        .add_card(digimon("PLAIN-TOP", "Plain Top", 6, 11000, CardColor::Green, ""))
        .add_card(digimon("OPP-WEAK", "Opp Weak", 3, 3000, CardColor::Purple, ""))
        .add_card(digimon("OPP-WEAK2", "Opp Weak Two", 3, 3000, CardColor::Purple, ""))
        .add_card({
            let mut t = make_test_card("OPP-TAMER", "Opp Tamer");
            t.card_kind = CardKind::Tamer;
            t.level = None;
            t.dp = None;
            t
        })
        .add_card(make_test_card("FILL", "Fill"))
}

fn base(hand: &[&str], memory: i16) -> DebugRunner {
    let mut runner = builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .security(1, &["FILL"; 4])
        .memory(memory)
        .start();
    runner.skip_mulligan();
    runner.game.current_phase = GamePhase::Main;
    runner
}

fn field_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn find(runner: &DebugRunner, player: u8, id: &str) -> PermanentHandle {
    let index = runner.game.players[player as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} on player {player}'s field"));
    PermanentHandle { player, index: index as u8 }
}

fn pick_opp(runner: &mut DebugRunner, id: &str) {
    let view = runner.pending_selection_view().expect("opponent-permanent prompt");
    assert_eq!(view.kind, SelectionKind::OppField, "{view:?}");
    assert!(!view.is_optional, "both picks are mandatory");
    let h = find(runner, 1, id);
    runner
        .execute_action(view.selecting_player, ATTACK_START + h.index as u16)
        .expect("pick");
}

fn suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn phase_locked(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner
        .game
        .modifiers
        .has(h, ModifierType::CannotUnsuspendInUnsuspendPhase)
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_041_metadata_and_digivolve_routes() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(5), Some(7000), Some(7)));
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 3, "green Lv.4, red Lv.4, [Coredramon]");
}

#[test]
fn ex13_041_has_fortitude() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(runner.game.has_keyword(me, Keyword::Fortitude));
}

#[test]
fn ex13_041_inherited_clause_shape() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    let ess = c
        .effects
        .iter()
        .find_map(|cl| match cl {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited => Some(t),
            _ => None,
        })
        .expect("inherited clause");
    assert_eq!(ess.when, vec![CompiledTiming::OnAllyWonBattle]);
    assert!(ess.once_per_turn && !ess.optional, "mandatory, [Once Per Turn]");
}

#[test]
fn ex13_041_digivolves_from_coredramon_for_3() {
    let mut runner = base(&[CARD_ID], 5);
    let core = runner.place_on_field(0, "CORE", Some(0));
    runner
        .game
        .decode_action(encode_digivolve(0, core.index as u16), 0);
    assert_eq!(field_ids(&runner, 0), vec![CARD_ID]);
    assert_eq!(runner.memory(), 2);
}

// ─── Section 2 — [On Play] / [When Digivolving] ──────────────────────────────

#[test]
fn ex13_041_on_play_suspends_then_locks_the_next_unsuspend_phase() {
    let mut runner = base(&[CARD_ID], 10);
    let opp = runner.place_on_field(1, "OPP-WEAK", Some(0));
    runner.play(0, 0).expect("Groundramon plays");
    pick_opp(&mut runner, "OPP-WEAK");
    assert!(suspended(&runner, opp), "suspended");
    pick_opp(&mut runner, "OPP-WEAK");
    let _ = runner.auto_resolve();
    assert!(phase_locked(&runner, opp));
    assert!(
        !runner.game.modifiers.has(opp, ModifierType::CannotUnsuspend),
        "not the broader effect-unsuspend lock"
    );

    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1);
    assert!(suspended(&runner, opp), "skipped their next unsuspend phase");

    // Effects may still unsuspend it ("in their next unsuspend phase" only).
    runner.game.unsuspend_with_cause(opp, true);
    assert!(!suspended(&runner, opp));
}

#[test]
fn ex13_041_lock_lasts_only_one_unsuspend_phase() {
    let mut runner = base(&[CARD_ID], 10);
    let opp = runner.place_on_field(1, "OPP-WEAK", Some(0));
    runner.play(0, 0).expect("plays");
    pick_opp(&mut runner, "OPP-WEAK");
    pick_opp(&mut runner, "OPP-WEAK");
    let _ = runner.auto_resolve();
    runner.end_turn(); // → their turn: stays suspended
    let _ = runner.auto_resolve();
    assert!(suspended(&runner, opp));
    runner.end_turn(); // → our turn
    let _ = runner.auto_resolve();
    assert!(!phase_locked(&runner, opp), "expired with their turn");
    runner.end_turn(); // → their following turn: unsuspends normally
    let _ = runner.auto_resolve();
    assert!(!suspended(&runner, opp));
}

#[test]
fn ex13_041_lock_installed_on_their_turn_survives_to_their_next_unsuspend_phase() {
    // e.g. replayed by <Fortitude> during the opponent's turn: "their NEXT
    // unsuspend phase" is the one at the start of their following turn.
    let mut runner = base(&[], 3);
    let opp = runner.place_on_field(1, "OPP-WEAK", Some(0));
    runner.game.memory = -3; // we overspent: the opponent starts with 3
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1);
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, me.index as usize);
    pick_opp(&mut runner, "OPP-WEAK");
    pick_opp(&mut runner, "OPP-WEAK");
    let _ = runner.auto_resolve();
    assert!(
        runner.pending_selection_view().is_none(),
        "On Play fully resolved: {:?}",
        runner.pending_selection_view()
    );
    // Installed during THEIR turn with `end_of_opponents_next_turn`: the entry
    // skips the current (their) turn-end and expires at the end of their
    // FOLLOWING turn — i.e. it is still present at that turn's unsuspend phase.
    let entries = runner
        .game
        .modifiers
        .get(opp, ModifierType::CannotUnsuspendInUnsuspendPhase);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].expiry, Expiry::EndOfOpponentsNextTurn);
    assert_eq!(entries[0].pending_skips, 1, "skip the turn-end it was installed in");
    runner.game.modifiers.expire_end_of_turn(1); // their current turn ends
    assert!(phase_locked(&runner, opp), "survives their current turn-end");
    runner.game.modifiers.expire_end_of_turn(0); // our turn ends
    assert!(phase_locked(&runner, opp), "present at their next unsuspend phase");
    runner.game.modifiers.expire_end_of_turn(1); // their next turn ends
    assert!(!phase_locked(&runner, opp), "gone after that one unsuspend phase");
}

#[test]
fn ex13_041_suspend_and_lock_may_pick_different_targets_and_tamers() {
    let mut runner = base(&[CARD_ID], 10);
    let weak = runner.place_on_field(1, "OPP-WEAK", Some(0));
    let tamer = runner.place_on_field(1, "OPP-TAMER", Some(0));
    runner.play(0, 0).expect("plays");
    pick_opp(&mut runner, "OPP-TAMER");
    pick_opp(&mut runner, "OPP-WEAK");
    let _ = runner.auto_resolve();
    assert!(suspended(&runner, tamer));
    assert!(phase_locked(&runner, weak));
    assert!(!phase_locked(&runner, tamer));
}

#[test]
fn ex13_041_when_digivolving_fires_the_shared_clause() {
    let mut runner = base(&[CARD_ID], 10);
    let opp = runner.place_on_field(1, "OPP-WEAK", Some(0));
    let lv4 = runner.place_on_field(0, "GREEN4", Some(0));
    runner
        .game
        .decode_action(encode_digivolve(0, lv4.index as u16), 0);
    pick_opp(&mut runner, "OPP-WEAK");
    pick_opp(&mut runner, "OPP-WEAK");
    let _ = runner.auto_resolve();
    assert!(suspended(&runner, opp) && phase_locked(&runner, opp));
}

// ─── Section 3 — Lv.6 [Breakdramon] for [Examon]'s DNA digivolution ──────────

#[test]
fn ex13_041_is_the_green_lv6_half_of_examons_dna() {
    let mut runner = base(&[EXAMON], 10);
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "BLUE6", Some(0));
    runner.game.tick_declarative_effects();
    assert!(runner.game.initiate_dna_digivolve(0, 0), "DNA legal");
    runner.game.resolve_selection(0, 0).expect("first material");
    runner.game.resolve_selection(0, 1).expect("second material");
    let examon = find(&runner, 0, EXAMON);
    let stack: Vec<String> = runner.game.players[0].battle_area[examon.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(stack.contains(&CARD_ID.to_string()) && stack.contains(&"BLUE6".to_string()));
}

#[test]
fn ex13_041_needs_a_blue_lv6_partner() {
    let mut runner = base(&[EXAMON], 10);
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "RED6", Some(0));
    runner.game.tick_declarative_effects();
    assert!(!runner.game.has_valid_dna_route_for_hand_card(0, 0));
}

#[test]
fn ex13_041_extras_name_breakdramon_only_for_examon() {
    let mut runner = base(&[EXAMON], 0);
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.tick_declarative_effects();
    let exa = runner.game.players[0].hand[0].clone();
    let extras = runner.game.dna_material_extras(me, &exa);
    assert_eq!(extras.levels, vec![6]);
    assert_eq!(extras.names, vec!["Breakdramon".to_string()]);
}

// ─── Section 4 — inherited: text Digimon deletes in battle → trash security ──

fn battle_board(top: &str) -> (DebugRunner, PermanentHandle) {
    let mut runner = base(&[], 0);
    let h = runner.place_stack(0, &[CARD_ID, top]);
    (runner, h)
}

#[test]
fn ex13_041_text_carrier_deleting_in_battle_trashes_their_top_security() {
    let (mut runner, me) = battle_board("DRACO-TOP");
    let opp = runner.place_on_field(1, "OPP-WEAK", Some(0));
    runner.battle_digimon(me, opp);
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(1), 0);
    assert_eq!(runner.security_count(1), 3);
}

#[test]
fn ex13_041_any_other_text_digimon_of_yours_also_counts() {
    let (mut runner, _me) = battle_board("PLAIN-TOP");
    let ally = runner.place_on_field(0, "DRACO-ALLY", Some(0));
    let opp = runner.place_on_field(1, "OPP-WEAK", Some(0));
    runner.battle_digimon(ally, opp);
    let _ = runner.auto_resolve();
    assert_eq!(runner.security_count(1), 3, "\"any of your Digimon\"");
}

#[test]
fn ex13_041_deleter_without_the_text_does_not_trigger() {
    let (mut runner, me) = battle_board("PLAIN-TOP");
    let opp = runner.place_on_field(1, "OPP-WEAK", Some(0));
    runner.battle_digimon(me, opp);
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(1), 0);
    assert_eq!(runner.security_count(1), 4);
}

#[test]
fn ex13_041_prevented_deletion_does_not_trigger() {
    let (mut runner, me) = battle_board("DRACO-TOP");
    let opp = runner.place_on_field(1, "OPP-WEAK", Some(0));
    runner.game.modifiers.add(
        opp,
        ModifierEntry::simple(ModifierType::CannotBeDestroyedByBattle, 0, Expiry::Permanent, 1),
    );
    runner.battle_digimon(me, opp);
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(1), 1);
    assert_eq!(runner.security_count(1), 4);
}

#[test]
fn ex13_041_battle_deletion_trigger_is_once_per_turn() {
    let (mut runner, me) = battle_board("DRACO-TOP");
    let a = runner.place_on_field(1, "OPP-WEAK", Some(0));
    runner.place_on_field(1, "OPP-WEAK2", Some(0));
    runner.battle_digimon(me, a);
    let _ = runner.auto_resolve();
    let b = find(&runner, 1, "OPP-WEAK2");
    runner.battle_digimon(me, b);
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(1), 0, "both deleted");
    assert_eq!(runner.security_count(1), 3, "[Once Per Turn]");
}

#[test]
fn ex13_041_face_up_groundramon_has_no_inherited_trigger_for_itself() {
    // The clause is inherited-only: a top-card Groundramon does not carry it.
    let mut runner = base(&[], 0);
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-WEAK", Some(0));
    runner.battle_digimon(me, opp);
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(1), 0);
    assert_eq!(runner.security_count(1), 4);
}
