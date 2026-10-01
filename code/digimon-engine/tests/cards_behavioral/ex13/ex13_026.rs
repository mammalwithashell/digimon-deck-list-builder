//! EX13-026 Kudamon — Digimon, Lv.3, Yellow, DP 1000, Cost 3.
//! Traits: Holy Beast / DATA SQUAD. Form: Rookie. Attribute: Vaccine.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-026.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-026.md` agrees)
//!
//! ```text
//! Digivolve: Yellow Lv.2 / cost 0;  [Digivolve] Lv.2 w/[DATA SQUAD] trait: Cost 0
//!
//! [When Moving] [On Play] Reveal the top 3 cards of your deck. Among them, add
//! 1 [Holy Beast], [Royal Knight] or [DATA SQUAD] trait card to the hand and
//! place 1 such card face down under any of your [DATA SQUAD] trait Tamers.
//! Return the rest to the bottom of the deck.
//!
//! Inherited Effect:
//! [When Attacking] [Once Per Turn] Give 1 of your opponent's Digimon
//! <Security A. -1> (This Digimon checks 1 fewer security card.) until their
//! turn ends.
//! ```
//! Official Q&A: the card is placed on the BOTTOM of the cards under the Tamer.
//!
//! # DCGO C# reference
//! None — DCGO has no `EX13_026.cs` at `b9a0638cd`. The reveal clause is
//! textually identical (modulo traits) to ST23-06 Gekkomon
//! (`DCGO/Assets/Scripts/CardEffect/ST23/Green/ST23_06.cs`), whose YAML idiom
//! this card reuses.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - A2/A3 reveal-3 two-bucket pick (add to hand + place face down under a
//!   chosen [DATA SQUAD] Tamer), remainder to deck bottom.
//! - Shared [When Moving] + [On Play] trigger.
//! - G4 inherited [When Attacking] [Once Per Turn] opponent debuff
//!   (SecurityAttackChange -1, expiry end of opponent's turn).
//! - Trait-gated alt digivolve (Lv.2 w/[DATA SQUAD] / cost 0).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-026";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = 3;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn tamer(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-026 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("HB", 4, &["Holy Beast"]))
        .add_card(digimon("RK", 6, &["Royal Knight"]))
        .add_card(digimon("DS", 4, &["DATA SQUAD"]))
        .add_card(digimon("PLAIN", 4, &["Beast"]))
        .add_card(digimon("FILL", 3, &[]))
        .add_card(digimon("EGG-DS", 2, &["DATA SQUAD"]))
        .add_card(digimon("CARRIER", 4, &[]))
        .add_card(digimon("OPP", 4, &[]))
        .add_card(tamer("DS-TAMER", &["DATA SQUAD"]))
        .add_card(tamer("PLAIN-TAMER", &["Beast"]))
}

/// Runner with Kudamon in hand and `deck_top` stacked on the deck (index 0 =
/// first revealed card).
fn setup(deck_top: &[&str]) -> DebugRunner {
    let mut deck: Vec<&str> = vec!["FILL"; 4];
    for c in deck_top.iter().rev() {
        deck.push(c);
    }
    builder()
        .hand(0, &[CARD_ID])
        .deck(0, &deck)
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(5)
        .start()
}

fn find_perm(r: &DebugRunner, id: &str) -> PermanentHandle {
    let i = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} on field"));
    PermanentHandle {
        player: 0,
        index: i as u8,
    }
}

fn hand_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn sources_of(r: &DebugRunner, h: PermanentHandle) -> Vec<(String, bool)> {
    let perm = &r.game.players[0].battle_area[h.index as usize];
    perm.card_sources[..perm.card_sources.len() - 1]
        .iter()
        .map(|c| (c.card_id(&r.game.card_data).to_string(), c.face_down))
        .collect()
}

fn revealed_ids(r: &DebugRunner) -> Vec<String> {
    r.game
        .revealed_cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn play_kudamon(r: &mut DebugRunner) {
    r.play(0, 0).expect("Kudamon played");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_026_is_yellow_lv3_holy_beast_data_squad() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(1000));
    assert!(card.traits.iter().any(|t| t == "Holy Beast"));
    assert!(card.traits.iter().any(|t| t == "DATA SQUAD"));
}

#[test]
fn ex13_026_has_two_digivolve_alt_paths() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let digi = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .count();
    assert_eq!(digi, 2, "yellow Lv.2 circle + Lv.2 w/[DATA SQUAD] condition");
}

#[test]
fn ex13_026_clause_shape() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 2);
    let reveal = triggered
        .iter()
        .find(|t| t.scope == CompiledScope::FaceUp)
        .expect("own reveal clause");
    assert!(reveal.when.contains(&CompiledTiming::OnPlay));
    assert!(reveal.when.contains(&CompiledTiming::OnMove));
    assert!(!reveal.once_per_turn);
    let inh = triggered
        .iter()
        .find(|t| t.scope == CompiledScope::Inherited)
        .expect("inherited clause");
    assert_eq!(inh.when, vec![CompiledTiming::WhenAttacking]);
    assert!(inh.once_per_turn);
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2/3 — [On Play] reveal: add 1 + place 1 under a [DATA SQUAD] Tamer
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_026_on_play_adds_one_and_places_one_face_down_under_ds_tamer() {
    let mut r = setup(&["HB", "RK", "PLAIN"]);
    r.place_on_field(0, "DS-TAMER", Some(0));
    play_kudamon(&mut r);
    assert!(r.pending_selection().is_some(), "reveal pick prompt");
    r.auto_resolve().expect("resolve picks + tamer + remainder");

    let tamer = find_perm(&r, "DS-TAMER");
    let srcs = sources_of(&r, tamer);
    assert_eq!(srcs.len(), 1, "one card placed under the Tamer");
    assert!(srcs[0].1, "placed face down");
    let hand = hand_ids(&r);
    assert_eq!(hand.len(), 1, "one card added to hand");
    let mut moved = vec![hand[0].clone(), srcs[0].0.clone()];
    moved.sort();
    assert_eq!(moved, vec!["HB".to_string(), "RK".to_string()]);
    let deck = &r.game.players[0].deck;
    assert_eq!(
        deck[0].card_id(&r.game.card_data),
        "PLAIN",
        "the rest returns to the bottom of the deck"
    );
    assert!(r.pending_selection().is_none());
}

#[test]
fn ex13_026_placed_card_goes_to_bottom_of_existing_tamer_stack() {
    let mut r = setup(&["DS", "HB", "PLAIN"]);
    let t = r.place_stack(0, &["FILL", "DS-TAMER"]);
    r.game.players[0].battle_area[t.index as usize].card_sources[0].face_down = true;
    play_kudamon(&mut r);
    r.auto_resolve().expect("resolve");
    let tamer = find_perm(&r, "DS-TAMER");
    let srcs = sources_of(&r, tamer);
    assert_eq!(srcs.len(), 2);
    assert_ne!(srcs[0].0, "FILL", "new card sits at the BOTTOM (Official Q&A)");
    assert!(srcs[0].1);
}

#[test]
fn ex13_026_data_squad_trait_card_is_eligible() {
    let mut r = setup(&["DS", "PLAIN", "PLAIN"]);
    play_kudamon(&mut r);
    r.auto_resolve().expect("resolve");
    assert_eq!(hand_ids(&r), vec!["DS".to_string()], "[DATA SQUAD] card added");
}

#[test]
fn ex13_026_no_data_squad_tamer_only_adds_to_hand() {
    let mut r = setup(&["HB", "RK", "PLAIN"]);
    r.place_on_field(0, "PLAIN-TAMER", Some(0));
    play_kudamon(&mut r);
    r.auto_resolve().expect("resolve");
    assert_eq!(hand_ids(&r).len(), 1, "one card still added");
    let t = find_perm(&r, "PLAIN-TAMER");
    assert!(
        sources_of(&r, t).is_empty(),
        "nothing goes under a non-[DATA SQUAD] Tamer"
    );
    assert!(r.pending_selection().is_none());
}

#[test]
fn ex13_026_no_eligible_reveal_adds_nothing() {
    let mut r = setup(&["PLAIN", "FILL", "PLAIN"]);
    r.place_on_field(0, "DS-TAMER", Some(0));
    play_kudamon(&mut r);
    r.auto_resolve().expect("resolve");
    assert!(hand_ids(&r).is_empty());
    let t = find_perm(&r, "DS-TAMER");
    assert!(sources_of(&r, t).is_empty());
    assert_eq!(r.deck_size(0), 7, "all 3 revealed cards return to the deck");
}

#[test]
fn ex13_026_when_moving_from_breeding_fires_reveal() {
    let mut r = setup(&["HB", "PLAIN", "PLAIN"]);
    // Move a breeding Kudamon to the battle area.
    r.game.players[0].hand.clear();
    r.place_in_breeding(0, CARD_ID);
    assert!(r.move_from_breeding(0), "move succeeds");
    assert!(
        r.pending_selection().is_some(),
        "[When Moving] fires the reveal clause"
    );
    r.auto_resolve().expect("resolve");
    assert_eq!(hand_ids(&r), vec!["HB".to_string()]);
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3/5 — Inherited [When Attacking] [OPT] Security A. -1
// ════════════════════════════════════════════════════════════════════════════

fn inherited_setup() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = builder()
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 5])
        .memory(5)
        .start();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    (r, carrier, opp)
}

#[test]
fn ex13_026_inherited_when_attacking_gives_security_attack_minus_one() {
    let (mut r, carrier, opp) = inherited_setup();
    r.attack_player(carrier, 1, false);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
    r.auto_resolve().expect("pick the only opponent Digimon");
    assert_eq!(
        r.modifiers().sum(opp, ModifierType::SecurityAttackChange),
        -1,
        "opponent Digimon gets <Security A. -1>"
    );
}

#[test]
fn ex13_026_inherited_debuff_lasts_until_end_of_opponents_turn() {
    let (mut r, carrier, opp) = inherited_setup();
    r.attack_player(carrier, 1, false);
    r.auto_resolve().expect("resolve");
    r.end_turn();
    r.auto_resolve().ok();
    assert_eq!(
        r.modifiers().sum(opp, ModifierType::SecurityAttackChange),
        -1,
        "still active during the opponent's turn"
    );
    r.end_turn();
    r.auto_resolve().ok();
    assert_eq!(
        r.modifiers().sum(opp, ModifierType::SecurityAttackChange),
        0,
        "expired when the opponent's turn ended"
    );
}

#[test]
fn ex13_026_inherited_not_active_on_kudamon_itself_without_carrier() {
    let mut r = builder()
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 5])
        .memory(5)
        .start();
    let kuda = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.attack_player(kuda, 1, false);
    r.auto_resolve().ok();
    assert_eq!(
        r.modifiers().sum(opp, ModifierType::SecurityAttackChange),
        0,
        "inherited text does not apply to the face-up Kudamon"
    );
}

#[test]
fn ex13_026_inherited_once_per_turn() {
    let (mut r, carrier, opp) = inherited_setup();
    r.attack_player(carrier, 1, false);
    r.auto_resolve().expect("resolve");
    // Re-ready the carrier and attack again the same turn.
    r.game.players[0].battle_area[carrier.index as usize].is_suspended = false;
    r.attack_player(carrier, 1, false);
    assert!(
        r.pending_selection().is_none(),
        "[Once Per Turn]: no second debuff prompt"
    );
    assert_eq!(r.modifiers().sum(opp, ModifierType::SecurityAttackChange), -1);
}
