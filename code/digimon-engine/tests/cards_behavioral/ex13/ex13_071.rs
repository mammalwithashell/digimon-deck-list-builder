//! EX13-071 Richard Sampson — Tamer, Yellow, Cost 4. Trait: DATA SQUAD.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-071.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-071.md` agrees)
//!
//! ```text
//! [Start of Your Main Phase] [On Play] You may place your deck's top card face
//! down under this Tamer. Then, if your opponent has a Digimon, gain 1 memory.
//! [Main] [Once Per Turn] By trashing 3 bottom face-down cards from under any
//! of your Tamers and placing 1 each of level 4 and level 5 [Holy Beast] trait
//! yellow Digimon cards from your trash as 1 of your [Kudamon]'s bottom
//! digivolution cards, it may digivolve into [Kentaurosmon] in the hand or
//! trash, ignoring level and with the cost reduced by 1.
//!
//! Security Effect:
//! [Security] Play this card without paying the cost.
//! ```
//! Official Q&A: the card is placed on the BOTTOM of the cards under the Tamer.
//!
//! # DCGO C# reference
//! None — DCGO has no `EX13_071.cs` at `b9a0638cd`. ST24-09 (deck-top face
//! down under a [DATA SQUAD] Tamer), BT25-087 (`[Security]` play self) and
//! BT25-035 (`trash_bottom_face_down_sources_under_tamers`) are the idioms.
//! "Ignoring level" is DCGO `CardEffectCommons.IgnoreRequirement.Level`
//! (BT24_025.cs / BT12_089.cs): the printed digivolution cost of the
//! colour-matching circle is still paid (here −1); only the level gate drops.
//!
//! # Verdict — IMPLEMENTED (2026-10-01)
//! All three clauses are authored and tested live. The [Main] clause landed
//! with `G-DIGIVOLVE-IGNORE-LEVEL-PRINTED-COST`: `effect_initiated_digivolve
//! { ignore_level: true }` waives only the level gate (colour-matching
//! printed circle kept, its printed cost paid, then `cost: { reduce: 1 }`).
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - B Tamer [Start of Your Main Phase] + [On Play] shared clause; optional
//!   ("you may") deck-top face-down placement; conditional memory gain.
//! - B Tamer [Security] play self free.
//! - C multi-cost material placement + effect digivolve ignoring level.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-071";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-071 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("FILL", 3, 1000))
        .add_card(digimon("TOPCARD", 3, 1000))
        .add_card(digimon("OPP", 4, 4000))
        .add_card(digimon("ATTACKER", 5, 9000))
}

fn runner(hand: &[&str]) -> DebugRunner {
    builder()
        .hand(0, hand)
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "TOPCARD"])
        .deck(1, &["FILL"; 8])
        .security(0, &["FILL"; 3])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start()
}

fn find_richard(r: &DebugRunner) -> PermanentHandle {
    let i = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == CARD_ID)
        .expect("Richard Sampson on field");
    PermanentHandle {
        player: 0,
        index: i as u8,
    }
}

fn stash(r: &DebugRunner, h: PermanentHandle) -> Vec<(String, bool)> {
    let perm = &r.game.players[0].battle_area[h.index as usize];
    perm.card_sources[..perm.card_sources.len() - 1]
        .iter()
        .map(|c| (c.card_id(&r.game.card_data).to_string(), c.face_down))
        .collect()
}

fn play_richard(r: &mut DebugRunner) {
    r.play(0, 0).expect("Richard Sampson played");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_071_is_yellow_data_squad_tamer() {
    let r = runner(&[]);
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(4));
    assert!(card.traits.iter().any(|t| t == "DATA SQUAD"));
}

#[test]
fn ex13_071_clause_shape() {
    let r = runner(&[]);
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    let shared = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("[Start of Your Main Phase][On Play] clause");
    assert!(shared.when.contains(&CompiledTiming::StartOfYourMainPhase));
    assert!(!shared.once_per_turn);
    assert!(
        triggered
            .iter()
            .any(|t| t.when.contains(&CompiledTiming::OnSecurity)),
        "[Security] play-self clause"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2/3 — [On Play]: optional face-down placement + conditional memory
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_071_on_play_place_deck_top_face_down_under_this_tamer() {
    let mut r = runner(&[CARD_ID]);
    play_richard(&mut r);
    assert_eq!(r.pending_kind(), Some(SelectionKind::EffectChoice));
    r.execute_branch(0).expect("place");
    r.auto_resolve().expect("drain");
    let rich = find_richard(&r);
    assert_eq!(
        stash(&r, rich),
        vec![("TOPCARD".to_string(), true)],
        "deck-top card placed face down under Richard"
    );
    assert_eq!(r.deck_size(0), 4);
}

#[test]
fn ex13_071_on_play_may_decline_the_placement() {
    let mut r = runner(&[CARD_ID]);
    play_richard(&mut r);
    r.execute_branch(1).expect("don't place");
    r.auto_resolve().expect("drain");
    let rich = find_richard(&r);
    assert!(stash(&r, rich).is_empty());
    assert_eq!(r.deck_size(0), 5);
}

#[test]
fn ex13_071_gains_memory_when_opponent_has_a_digimon() {
    let mut r = runner(&[CARD_ID]);
    r.place_on_field(1, "OPP", Some(0));
    play_richard(&mut r);
    r.execute_branch(1).expect("don't place");
    r.auto_resolve().expect("drain");
    assert_eq!(r.memory(), 5 - 4 + 1, "paid 4, gained 1");
}

#[test]
fn ex13_071_no_memory_when_opponent_has_no_digimon() {
    let mut r = runner(&[CARD_ID]);
    play_richard(&mut r);
    r.execute_branch(0).expect("place");
    r.auto_resolve().expect("drain");
    assert_eq!(r.memory(), 5 - 4, "paid 4, no gain");
}

#[test]
fn ex13_071_memory_gain_does_not_depend_on_placing() {
    let mut r = runner(&[CARD_ID]);
    r.place_on_field(1, "OPP", Some(0));
    play_richard(&mut r);
    r.execute_branch(0).expect("place");
    r.auto_resolve().expect("drain");
    assert_eq!(r.memory(), 5 - 4 + 1);
}

#[test]
fn ex13_071_start_of_your_main_phase_fires() {
    let mut r = runner(&[]);
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP", Some(0));
    r.end_turn();
    r.auto_resolve().ok();
    r.end_turn();
    assert_eq!(r.turn_player(), 0, "back to player 0");
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::EffectChoice),
        "[Start of Your Main Phase] offers the placement"
    );
    let mem = r.memory();
    r.execute_branch(0).expect("place");
    r.auto_resolve().expect("drain");
    let rich = find_richard(&r);
    assert_eq!(stash(&r, rich).len(), 1);
    assert!(stash(&r, rich)[0].1, "face down");
    assert_eq!(r.memory(), mem + 1, "opponent has a Digimon → +1 memory");
}

#[test]
fn ex13_071_start_of_opponents_main_phase_does_not_fire() {
    let mut r = runner(&[]);
    r.place_on_field(0, CARD_ID, Some(0));
    r.end_turn();
    assert_eq!(r.turn_player(), 1);
    assert!(
        r.pending_selection().is_none(),
        "not your main phase → nothing"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — [Security] play this card free
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_071_security_plays_itself_free() {
    let mut r = builder()
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(0, &[CARD_ID])
        .memory(5)
        .start();
    r.end_turn();
    assert_eq!(r.turn_player(), 1);
    let att = r.place_on_field(1, "ATTACKER", Some(0));
    let mem = r.memory();
    r.attack_player(att, 0, false);
    // Richard lands; its [On Play] offers the placement (to player 0).
    for _ in 0..6 {
        let Some(view) = r.pending_selection_view() else { break };
        if view.effect_choices.is_some() {
            r.execute_branch(1).expect("decline placement");
        } else {
            r.execute_action(view.selecting_player, view.valid_action_ids[0])
                .expect("first");
        }
    }
    let on_field = r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == CARD_ID);
    assert!(on_field, "[Security] played Richard Sampson");
    let _ = mem;
}


// ════════════════════════════════════════════════════════════════════════════
// Section 4 — [Main] [Once Per Turn]: 3 face-down trashed + Lv.4/Lv.5 under a
// [Kudamon] → it may digivolve into [Kentaurosmon] (hand/trash) ignoring
// level, cost −1. G-DIGIVOLVE-IGNORE-LEVEL-PRINTED-COST (resolved 2026-10-01).
// ════════════════════════════════════════════════════════════════════════════

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{EFFECTS_PER_PERMANENT, FIELD_EFFECT_START};
use digimon_engine::card_data::EvoCost;
use digimon_engine::enums::GamePhase;
use digimon_engine::selection::PendingSelectionView;

fn holy_beast(id: &str, name: &str, level: u8) -> CardData {
    let mut c = digimon(id, level, 1000 * level as i32);
    c.card_name = name.to_string();
    c.traits = vec!["Holy Beast".to_string()];
    c
}

/// Kentaurosmon-named Lv.6 yellow; printed circle Yellow Lv.5 cost 3.
fn kentaurosmon(id: &str) -> CardData {
    let mut c = digimon(id, 6, 12000);
    c.card_name = "Kentaurosmon".to_string();
    c.play_cost = 12;
    c.evo_costs = vec![EvoCost {
        card_color: 2,
        level: 5,
        memory_cost: 3,
    }];
    c
}

struct MainScene {
    r: DebugRunner,
    richard: PermanentHandle,
    kudamon: PermanentHandle,
}

/// Richard on field with `face_down` face-down cards under him, a [Kudamon],
/// Lv.4 / Lv.5 yellow [Holy Beast] cards in trash as requested, Kentaurosmon
/// in hand (or trash), memory 5, Main phase.
fn main_scene(face_down: usize, lv4: bool, lv5: bool, ken_in_trash: bool) -> MainScene {
    let hand: Vec<&str> = if ken_in_trash { vec![] } else { vec!["KEN"] };
    let mut r = builder()
        .add_card(holy_beast("KUDA", "Kudamon", 3))
        .add_card(holy_beast("HB4", "Reppamon", 4))
        .add_card(holy_beast("HB5", "Chirinmon", 5))
        .add_card(kentaurosmon("KEN"))
        .hand(0, &hand)
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(0, &["FILL"; 3])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();
    let richard = r.place_on_field(0, CARD_ID, Some(0));
    for _ in 0..face_down {
        r.push_source(richard, "FILL");
    }
    {
        let perm = &mut r.game.players[0].battle_area[richard.index as usize];
        let n = perm.card_sources.len();
        for c in perm.card_sources[..n - 1].iter_mut() {
            c.face_down = true;
        }
    }
    let kudamon = r.place_on_field(0, "KUDA", Some(0));
    if lv4 {
        r.inject_trash(0, "HB4");
    }
    if lv5 {
        r.inject_trash(0, "HB5");
    }
    if ken_in_trash {
        r.inject_trash(0, "KEN");
    }
    r.game.current_phase = GamePhase::Main;
    MainScene { r, richard, kudamon }
}

fn main_offered(r: &DebugRunner, richard: PermanentHandle) -> bool {
    let mask = build_action_mask(&r.game, 0);
    let base = FIELD_EFFECT_START + richard.index as u16 * EFFECTS_PER_PERMANENT;
    (base..base + EFFECTS_PER_PERMANENT).any(|a| mask[a as usize] == 1.0)
}

fn first_non_pass(v: &PendingSelectionView) -> u16 {
    *v.valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("a non-PASS option")
}

/// Drive the [Main] body: `order` answers the bottom-order EffectChoice;
/// `digivolve` accepts (first candidate) or PASSes the Kentaurosmon pick.
/// Every other prompt (face-down trash picks, Kudamon, Lv.4 / Lv.5 picks)
/// takes its first candidate. Returns the prompts seen.
fn drive_main(r: &mut DebugRunner, order: usize, digivolve: bool) -> Vec<String> {
    let mut seen = Vec::new();
    for _ in 0..30 {
        let Some(v) = r.pending_selection_view() else { break };
        seen.push(v.prompt.clone());
        if v.effect_choices.is_some() {
            r.execute_branch(order).expect("order choice");
        } else if v.prompt.contains("Kentaurosmon") {
            assert!(v.is_optional, "the digivolve is optional (\"it may digivolve\")");
            let a = if digivolve { first_non_pass(&v) } else { PASS };
            r.execute_action(v.selecting_player, a).expect("kentaurosmon pick");
        } else {
            r.execute_action(v.selecting_player, first_non_pass(&v)).expect("pick");
        }
    }
    seen
}

fn stack(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    r.game.players[0].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn face_down_count(r: &DebugRunner, h: PermanentHandle) -> usize {
    r.game.players[0].battle_area[h.index as usize]
        .card_sources
        .iter()
        .filter(|c| c.face_down)
        .count()
}

#[test]
fn ex13_071_main_places_materials_and_digivolves_kudamon_ignoring_level_cost_minus_one() {
    let MainScene {
        mut r,
        richard,
        kudamon,
    } = main_scene(3, true, true, false);
    assert!(main_offered(&r, richard), "[Main] offered when the whole cost is payable");
    let trash0 = r.trash_size(0);
    assert!(r.game.activate_field_main(0, richard.index as usize));
    let seen = drive_main(&mut r, 0, true);
    assert!(seen.iter().any(|p| p.contains("Kentaurosmon")), "digivolve pick offered");
    assert_eq!(face_down_count(&r, richard), 0, "3 bottom face-down cards trashed");
    assert_eq!(
        stack(&r, kudamon),
        vec!["HB5", "HB4", "KUDA", "KEN"],
        "Lv.4 placed first, Lv.5 at the very bottom; then digivolved into Kentaurosmon"
    );
    assert_eq!(
        r.memory(),
        5 - (3 - 1),
        "level ignored, colour circle kept: printed cost 3 reduced by 1"
    );
    assert_eq!(r.trash_size(0), trash0 + 3 - 2, "+3 face-down, −Lv.4, −Lv.5");
}

#[test]
fn ex13_071_main_bottom_order_is_the_players_choice() {
    let MainScene {
        mut r,
        richard,
        kudamon,
    } = main_scene(3, true, true, false);
    assert!(r.game.activate_field_main(0, richard.index as usize));
    drive_main(&mut r, 1, true);
    assert_eq!(stack(&r, kudamon), vec!["HB4", "HB5", "KUDA", "KEN"]);
}

#[test]
fn ex13_071_main_digivolves_from_trash() {
    let MainScene {
        mut r,
        richard,
        kudamon,
    } = main_scene(3, true, true, true);
    assert!(r.game.activate_field_main(0, richard.index as usize));
    drive_main(&mut r, 0, true);
    assert_eq!(stack(&r, kudamon).last().map(String::as_str), Some("KEN"));
    assert_eq!(r.memory(), 3);
}

#[test]
fn ex13_071_main_digivolve_may_be_declined_after_the_cost_is_paid() {
    let MainScene {
        mut r,
        richard,
        kudamon,
    } = main_scene(3, true, true, false);
    assert!(r.game.activate_field_main(0, richard.index as usize));
    drive_main(&mut r, 0, false);
    assert_eq!(stack(&r, kudamon), vec!["HB5", "HB4", "KUDA"], "cost paid, no digivolve");
    assert_eq!(face_down_count(&r, richard), 0);
    assert_eq!(r.memory(), 5);
    assert_eq!(r.hand_size(0), 1, "Kentaurosmon stays in hand");
}

#[test]
fn ex13_071_main_needs_three_face_down_cards() {
    let MainScene { r, richard, .. } = main_scene(2, true, true, false);
    assert!(!main_offered(&r, richard), "2 face-down cards → cost unpayable");
}

#[test]
fn ex13_071_main_needs_both_a_level_4_and_a_level_5_in_trash() {
    let MainScene { r, richard, .. } = main_scene(3, true, false, false);
    assert!(!main_offered(&r, richard), "no Lv.5 in trash");
    let MainScene { r, richard, .. } = main_scene(3, false, true, false);
    assert!(!main_offered(&r, richard), "no Lv.4 in trash");
}

#[test]
fn ex13_071_main_needs_a_kudamon() {
    let MainScene {
        mut r,
        richard,
        kudamon,
    } = main_scene(3, true, true, false);
    // Replace the Kudamon with a non-Kudamon yellow [Holy Beast] Lv.3.
    r.game.players[0].battle_area.remove(kudamon.index as usize);
    assert!(!main_offered(&r, richard), "no [Kudamon] → no target for the placement");
}

#[test]
fn ex13_071_main_is_once_per_turn() {
    let MainScene {
        mut r,
        richard,
        ..
    } = main_scene(6, true, true, false);
    r.inject_trash(0, "HB4");
    r.inject_trash(0, "HB5");
    assert!(r.game.activate_field_main(0, richard.index as usize));
    drive_main(&mut r, 0, false);
    assert!(
        face_down_count(&r, richard) >= 3,
        "a second activation's cost would still be payable"
    );
    assert!(!main_offered(&r, richard), "[Once Per Turn]");
}
