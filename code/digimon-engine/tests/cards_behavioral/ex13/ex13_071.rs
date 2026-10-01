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
//! # Verdict — BLOCKED (hybrid), partial YAML
//! Clauses 1 and 3 are authored and tested live. Clause 2 ([Main]) is BLOCKED
//! on `G-DIGIVOLVE-IGNORE-LEVEL-PRINTED-COST`: `effect_initiated_digivolve`
//! only has `ignore_requirements: true`, which drops colour too AND zeroes the
//! base cost (`matching_memory_cost = 0` in
//! `game_actions/digivolve.rs::effect_initiated_digivolve_from_source_inner`),
//! so "ignoring level, cost reduced by 1" cannot be expressed without a wrong
//! cost. Its tests are `#[ignore]`d with the gap id; the clause is NOT
//! authored (no approximation).
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - B Tamer [Start of Your Main Phase] + [On Play] shared clause; optional
//!   ("you may") deck-top face-down placement; conditional memory gain.
//! - B Tamer [Security] play self free.
//! - C (BLOCKED) multi-cost material placement + effect digivolve.

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
// BLOCKED — [Main] [Once Per Turn] (G-DIGIVOLVE-IGNORE-LEVEL-PRINTED-COST)
// ════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "pending: G-DIGIVOLVE-IGNORE-LEVEL-PRINTED-COST from qa/archetype-qa/engine-gaps.md"]
fn ex13_071_main_places_materials_and_digivolves_kudamon_ignoring_level_cost_minus_one() {
    // Expected once the gap closes: trash 3 bottom face-down cards from under
    // Tamers (each pick surfaced), place 1 Lv.4 + 1 Lv.5 yellow [Holy Beast]
    // Digimon from trash as a [Kudamon]'s bottom digivolution cards, then the
    // optional digivolve into a [Kentaurosmon] in hand OR trash pays that
    // Kentaurosmon's printed yellow-circle cost − 1 (level gate dropped,
    // colour gate kept).
    let r = runner(&[]);
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand)
                || t.when.contains(&CompiledTiming::Main)
        )),
        "[Main] clause authored"
    );
}

#[test]
#[ignore = "pending: G-DIGIVOLVE-IGNORE-LEVEL-PRINTED-COST from qa/archetype-qa/engine-gaps.md"]
fn ex13_071_main_is_once_per_turn() {
    panic!("authored when G-DIGIVOLVE-IGNORE-LEVEL-PRINTED-COST closes");
}
