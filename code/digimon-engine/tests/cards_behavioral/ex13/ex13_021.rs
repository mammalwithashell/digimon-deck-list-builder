//! EX13-021 Wingdramon — Digimon, Lv.5, Blue/Red, DP 7000, Cost 7.
//! Traits: Sky Dragon. Form: Ultimate. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-021.md`)
//!
//! Digivolve: Blue Lv.4 / 4; Red Lv.4 / 4.  [Digivolve] [Coredramon]: Cost 3
//! ＜Jamming＞
//! [On Play] [When Digivolving] Trash the bottom 2 digivolution cards of 1 of
//! your opponent's Digimon. Then, 1 of their Digimon or Tamers can't suspend
//! until their turn ends.
//! [All Turns] This Digimon is also treated as Lv.6 [Slayerdramon] for
//! [Examon]'s DNA digivolution.
//! Inherited: [All Turns] [Once Per Turn] When this Digimon with [Dracomon] or
//! [Examon] in its text suspends, it may unsuspend.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Blue/EX13_021.cs — `AddJogressLevelsClass`
//! (+Lv.6 when the DNA result is [Examon]) + `ChangeCardNamesClass`
//! (+[Slayerdramon]); shared [OP]/[WD] with two independent mandatory picks;
//! inherited `OnTappedAnyone` OPT optional unsuspend gated on HasText.
//!
//! # Patterns
//! - G-DNA-MATERIAL-TREATED-AS-FOR-TARGET (`dna_material_identity` aura).
//! - Trash-bottom-N sources (EX12-029 idiom) + CannotSuspend lock.
//! - Inherited self-suspend → may unsuspend (EX13-022 idiom, text-gated).

#![allow(dead_code)]

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_digivolve, ATTACK_START, DNA_DIGIVOLVE_START, PASS};
use digimon_engine::card_data::{CardData, DnaCost, DnaRequirement};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-021";
const EXAMON: &str = "EX13-045";

fn digimon(id: &str, name: &str, level: u8, color: CardColor, text: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = level as u16 + 2;
    c.colors = vec![color];
    c.effect_text = text.to_string();
    c
}

fn req(level: u8, color: CardColor) -> DnaRequirement {
    DnaRequirement {
        level,
        card_colors: vec![color],
        name_contains: String::new(),
        text_contains: String::new(),
    }
}

fn builder() -> DebugRunnerBuilder {
    // A non-[Examon] card with Examon's exact printed DNA recipe.
    let mut not_examon = digimon("NOT-EXAMON", "Not Examon", 7, CardColor::Green, "");
    not_examon.dna_costs = vec![DnaCost {
        requirement1: req(6, CardColor::Green),
        requirement2: req(6, CardColor::Blue),
        memory_cost: 0,
    }];
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-021 YAML loads")
        .dsl_card(EXAMON)
        .expect("EX13-045 YAML loads")
        .add_card(digimon("BLUE4", "Blue Four", 4, CardColor::Blue, ""))
        .add_card(digimon("CORE", "Coredramon", 4, CardColor::Green, ""))
        .add_card(digimon("GREEN6", "Green Six", 6, CardColor::Green, ""))
        .add_card(digimon("RED6", "Red Six", 6, CardColor::Red, ""))
        .add_card(digimon("DRACO-TOP", "Draco Top", 6, CardColor::Blue, "Search [Dracomon]."))
        .add_card(digimon("PLAIN-TOP", "Plain Top", 6, CardColor::Blue, ""))
        .add_card(digimon("OPP-A", "Opp A", 3, CardColor::Purple, ""))
        .add_card(digimon("OPP-B", "Opp B", 3, CardColor::Purple, ""))
        .add_card(digimon("OPP-C", "Opp C", 4, CardColor::Purple, ""))
        .add_card(digimon("OPP-TOP", "Opp Top", 5, CardColor::Purple, ""))
        .add_card({
            let mut t = make_test_card("OPP-TAMER", "Opp Tamer");
            t.card_kind = CardKind::Tamer;
            t.level = None;
            t.dp = None;
            t
        })
        .add_card(not_examon)
        .add_card(make_test_card("FILL", "Fill"))
}

fn base(hand: &[&str], memory: i16) -> DebugRunner {
    let mut runner = builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(1, &["FILL"; 3])
        .memory(memory)
        .start();
    runner.skip_mulligan();
    runner.game.current_phase = GamePhase::Main;
    runner
}

fn ids(runner: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn field_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn hand_index(runner: &DebugRunner, id: &str) -> usize {
    runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} in hand"))
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

fn cannot_suspend(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.modifiers.has(h, ModifierType::CannotSuspend)
}

fn suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_021_metadata_and_digivolve_routes() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(5), Some(7000), Some(7)));
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 3, "blue Lv.4, red Lv.4, [Coredramon]");
    assert!(digi
        .iter()
        .any(|p| p.from.as_ref().and_then(|f| f.name_is.as_deref()) == Some("Coredramon")));
}

#[test]
fn ex13_021_has_jamming() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(runner.game.has_keyword(me, Keyword::Jamming));
}

#[test]
fn ex13_021_clause_shapes() {
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
    assert_eq!(ess.when, vec![CompiledTiming::OnSuspend]);
    assert!(ess.once_per_turn && ess.optional);
}

#[test]
fn ex13_021_digivolves_from_coredramon_for_3() {
    let mut runner = base(&[CARD_ID], 5);
    let core = runner.place_on_field(0, "CORE", Some(0));
    runner
        .game
        .decode_action(encode_digivolve(0, core.index as u16), 0);
    assert_eq!(field_ids(&runner, 0), vec![CARD_ID]);
    assert_eq!(runner.memory(), 2, "[Digivolve] [Coredramon]: Cost 3");
}

// ─── Section 2 — [On Play] / [When Digivolving] ──────────────────────────────

#[test]
fn ex13_021_on_play_trashes_the_bottom_two_sources_then_locks_suspend() {
    let mut runner = base(&[CARD_ID], 10);
    let opp = runner.place_stack(1, &["OPP-A", "OPP-B", "OPP-C", "OPP-TOP"]);
    runner.play(0, 0).expect("Wingdramon plays");
    pick_opp(&mut runner, "OPP-TOP");
    assert_eq!(ids(&runner, opp), vec!["OPP-C", "OPP-TOP"], "bottom 2 trashed");
    pick_opp(&mut runner, "OPP-TOP");
    let _ = runner.auto_resolve();
    assert!(cannot_suspend(&runner, opp), "can't suspend until their turn ends");
}

#[test]
fn ex13_021_lock_may_target_a_tamer() {
    let mut runner = base(&[CARD_ID], 10);
    runner.place_on_field(1, "OPP-TOP", Some(0));
    let tamer = runner.place_on_field(1, "OPP-TAMER", Some(0));
    runner.play(0, 0).expect("plays");
    pick_opp(&mut runner, "OPP-TOP");
    pick_opp(&mut runner, "OPP-TAMER");
    let _ = runner.auto_resolve();
    assert!(cannot_suspend(&runner, tamer));
}

#[test]
fn ex13_021_lock_still_applies_with_no_opponent_digimon() {
    // Two independent steps: no Digimon to trash from, but a Tamer to lock.
    let mut runner = base(&[CARD_ID], 10);
    let tamer = runner.place_on_field(1, "OPP-TAMER", Some(0));
    runner.play(0, 0).expect("plays");
    pick_opp(&mut runner, "OPP-TAMER");
    let _ = runner.auto_resolve();
    assert!(cannot_suspend(&runner, tamer));
}

#[test]
fn ex13_021_when_digivolving_fires_the_shared_clause() {
    let mut runner = base(&[CARD_ID], 10);
    let opp = runner.place_stack(1, &["OPP-A", "OPP-TOP"]);
    let lv4 = runner.place_on_field(0, "BLUE4", Some(0));
    runner
        .game
        .decode_action(encode_digivolve(0, lv4.index as u16), 0);
    pick_opp(&mut runner, "OPP-TOP");
    pick_opp(&mut runner, "OPP-TOP");
    let _ = runner.auto_resolve();
    assert_eq!(ids(&runner, opp), vec!["OPP-TOP"], "its only source trashed");
    assert!(cannot_suspend(&runner, opp));
}

// ─── Section 3 — Lv.6 [Slayerdramon] for [Examon]'s DNA digivolution ─────────

fn dna_board(partner: &str, result: &str) -> DebugRunner {
    let mut runner = base(&[result], 10);
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, partner, Some(0));
    runner.game.tick_declarative_effects();
    runner
}

#[test]
fn ex13_021_is_the_blue_lv6_half_of_examons_dna() {
    let mut runner = dna_board("GREEN6", EXAMON);
    assert!(runner.game.has_valid_dna_route_for_hand_card(0, 0));
    let mask = digimon_engine::action::mask::build_action_mask(&runner.game, 0);
    assert_eq!(mask[DNA_DIGIVOLVE_START as usize], 1.0, "DNA action exposed");
    assert!(runner.game.initiate_dna_digivolve(0, 0));
    runner.game.resolve_selection(0, 0).expect("first material");
    runner.game.resolve_selection(0, 1).expect("second material");
    let examon = find(&runner, 0, EXAMON);
    let stack = ids(&runner, examon);
    assert!(stack.contains(&CARD_ID.to_string()) && stack.contains(&"GREEN6".to_string()));
    assert_eq!(runner.effective_dp(examon), Some(25000), "Examon's DNA [When Digivolving] fired");
}

#[test]
fn ex13_021_needs_a_green_lv6_partner() {
    let runner = dna_board("RED6", EXAMON);
    assert!(!runner.game.has_valid_dna_route_for_hand_card(0, 0));
}

#[test]
fn ex13_021_is_not_lv6_for_another_cards_dna() {
    let runner = dna_board("GREEN6", "NOT-EXAMON");
    assert!(
        !runner.game.has_valid_dna_route_for_hand_card(0, 0),
        "the treatment is scoped to [Examon]'s DNA digivolution"
    );
}

#[test]
fn ex13_021_keeps_its_printed_level_and_name() {
    let mut runner = base(&[], 0);
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.tick_declarative_effects();
    let perm = &runner.game.players[0].battle_area[me.index as usize];
    assert_eq!(perm.level(&runner.game.card_data), Some(5));
    assert!(!perm
        .top_card()
        .card_names(&runner.game.card_data)
        .contains(&"Slayerdramon"));
}

#[test]
fn ex13_021_treatment_is_a_face_effect_only() {
    // Under another card it is not "this Digimon".
    let mut runner = base(&[EXAMON], 10);
    runner.place_stack(0, &[CARD_ID, "PLAIN-TOP"]);
    runner.place_on_field(0, "GREEN6", Some(0));
    runner.game.tick_declarative_effects();
    // PLAIN-TOP is a printed blue Lv.6, so the pair IS legal on its own merits
    // — the point is the carrier contributes no extras from under it.
    let carrier = find(&runner, 0, "PLAIN-TOP");
    let exa = runner.game.players[0].hand[0].clone();
    assert!(runner.game.dna_material_extras(carrier, &exa).is_empty());
}

// ─── Section 4 — inherited: this [Dracomon]/[Examon]-text Digimon suspends ───

fn carrier(top: &str) -> (DebugRunner, PermanentHandle) {
    let mut runner = base(&[], 3);
    let h = runner.place_stack(0, &[CARD_ID, top]);
    (runner, h)
}

#[test]
fn ex13_021_dracomon_text_carrier_may_unsuspend_after_suspending() {
    let (mut runner, h) = carrier("DRACO-TOP");
    runner.game.suspend(h);
    let view = runner.pending_selection_view().expect("may-unsuspend prompt");
    assert!(view.is_optional);
    let accept = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
    runner.execute_action(view.selecting_player, accept).expect("accept");
    let _ = runner.auto_resolve();
    assert!(!suspended(&runner, h));
}

#[test]
fn ex13_021_unsuspend_is_optional() {
    let (mut runner, h) = carrier("DRACO-TOP");
    runner.game.suspend(h);
    let view = runner.pending_selection_view().expect("prompt");
    runner.execute_action(view.selecting_player, PASS).expect("decline");
    let _ = runner.auto_resolve();
    assert!(suspended(&runner, h));
}

#[test]
fn ex13_021_carrier_without_the_text_does_not_unsuspend() {
    let (mut runner, h) = carrier("PLAIN-TOP");
    runner.game.suspend(h);
    assert!(runner.pending_selection().is_none());
    assert!(suspended(&runner, h));
}

#[test]
fn ex13_021_another_permanent_suspending_does_not_fire_it() {
    let (mut runner, h) = carrier("DRACO-TOP");
    let other = runner.place_on_field(0, "PLAIN-TOP", Some(0));
    runner.game.suspend(other);
    assert!(runner.pending_selection().is_none(), "only THIS Digimon");
    assert!(!suspended(&runner, h));
}

#[test]
fn ex13_021_unsuspend_is_once_per_turn() {
    let (mut runner, h) = carrier("DRACO-TOP");
    runner.game.suspend(h);
    let view = runner.pending_selection_view().expect("prompt");
    let accept = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
    runner.execute_action(view.selecting_player, accept).expect("accept");
    let _ = runner.auto_resolve();
    runner.game.suspend(h);
    assert!(runner.pending_selection().is_none(), "[Once Per Turn] used");
    assert!(suspended(&runner, h));
}
