//! EX13-038 Salamon — Digimon, Lv.3, Green, DP 2000, Cost 3.
//! Traits: Mammal (+ (Rule) Beast). Form: Rookie. Attribute: Vaccine.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-038.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-038.md` agrees)
//!
//! ```text
//! Digivolve: Green Lv.2 / cost 0
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 card with [Leopardmon]
//! in its text and 1 Digimon card with [Beast], [Animal] or [Sovereign], other
//! than [Sea Animal], in any of its traits among them to the hand. Return the
//! rest to the bottom of the deck. (Rule) Trait: Has [Beast] Type.
//!
//! Inherited Effect:
//! [All Turns] All of your suspended Digimon get +1000 DP.
//! ```
//! Official Q&A: "[X] in its text" covers name, traits, effects, inherited
//! effects, (Rule), and all requirement text.
//!
//! # DCGO C# reference
//! None — DCGO has no `EX13_038.cs` at `b9a0638cd`. Trait reading follows
//! DCGO `CardSource.HasAvianBeastAnimalTraits` minus Avian/Bird:
//! `ContainsTraits("Beast") || ContainsTraits("Sovereign") ||
//! (ContainsTraits("Animal") && !ContainsTraits("Sea Animal"))` — substring
//! (`trait_contains`) matching.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - A2 reveal-3 two-bucket pick, both buckets to hand, remainder bottom.
//! - `in_text_contains` + `trait_contains` filters.
//! - Inherited static aura gated on the target's suspended state.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "EX13-038";

fn digimon(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-038 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("LEO", "Leopardmon", &["Royal Knight"]))
        .add_card({
            let mut c = digimon("LEO-TEXT", "Texter", &["Machine"]);
            c.effect_text = "[On Play] If you have [Leopardmon], draw 1.".to_string();
            c
        })
        .add_card({
            let mut c = make_test_card("LEO-OPT", "Leopardmon Option");
            c.card_kind = CardKind::Option;
            c.level = None;
            c.dp = None;
            c.play_cost = 2;
            c
        })
        .add_card(digimon("BEAST", "Beastly", &["Beast"]))
        .add_card(digimon("HOLY-BEAST", "Holy", &["Holy Beast"]))
        .add_card(digimon("ANIMAL", "Animalish", &["Animal"]))
        .add_card(digimon("SOV", "Sovereignish", &["Sovereign"]))
        .add_card(digimon("SEA", "Seaish", &["Sea Animal"]))
        .add_card({
            let mut c = make_test_card("BEAST-TAMER", "Beast Tamer");
            c.card_kind = CardKind::Tamer;
            c.level = None;
            c.dp = None;
            c.traits = vec!["Beast".to_string()];
            c
        })
        .add_card(digimon("PLAIN", "Plain", &["Machine"]))
        .add_card(digimon("FILL", "Filler", &[]))
        .add_card(digimon("CARRIER", "Carrier", &[]))
        .add_card(digimon("ALLY", "Ally", &[]))
        .deck(1, &["FILL"; 6])
}

fn setup(deck_top: &[&str]) -> DebugRunner {
    let mut deck: Vec<&str> = vec!["FILL"; 4];
    for c in deck_top.iter().rev() {
        deck.push(c);
    }
    let mut r = builder().hand(0, &[CARD_ID]).deck(0, &deck).memory(5).start();
    r.skip_mulligan();
    r
}

fn hand_ids(r: &DebugRunner) -> Vec<String> {
    let mut v: Vec<String> = r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    v.sort();
    v
}

fn offered_reveal_ids(r: &DebugRunner) -> Vec<String> {
    let view = r.pending_selection_view().expect("reveal prompt pending");
    let mut ids: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| a.checked_sub(SEL_REVEAL_START))
        .filter_map(|i| r.game.revealed_cards.get(i as usize))
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    ids.sort();
    ids
}

fn pick_revealed(r: &mut DebugRunner, card_id: &str) {
    let view = r.pending_selection_view().expect("reveal prompt pending");
    let want = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .find(|&a| {
            a.checked_sub(SEL_REVEAL_START)
                .and_then(|i| r.game.revealed_cards.get(i as usize))
                .is_some_and(|c| c.card_id(&r.game.card_data) == card_id)
        })
        .unwrap_or_else(|| panic!("{card_id} must be a legal pick: {view:?}"));
    r.execute_action(view.selecting_player, want)
        .expect("pick revealed card");
}

/// Suspend through the engine (fires on-suspend timing + re-ticks the
/// declarative auras); unsuspend is a raw flip followed by a re-tick.
fn set_suspended(r: &mut DebugRunner, h: PermanentHandle, v: bool) {
    if v {
        r.game.suspend(h);
    } else {
        r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = false;
    }
    r.game.tick_declarative_effects();
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_038_printed_metadata_rule_trait_and_digivolve_path() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(3), Some(2000), Some(3)));
    assert_eq!(c.color, vec![CompiledColor::Green]);
    assert!(c.traits.contains(&"Mammal".to_string()));
    assert!(
        c.traits.contains(&"Beast".to_string()),
        "(Rule) Trait: Has [Beast] Type"
    );
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 1);
    assert_eq!(digi[0].cost, Some(CompiledCost::Literal(0)));
}

#[test]
fn ex13_038_clause_shape() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1);
    assert_eq!(triggered[0].when, vec![CompiledTiming::OnPlay]);
    assert!(!triggered[0].optional);
    let auras: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, .. }) => {
                Some(*scope)
            }
            _ => None,
        })
        .collect();
    assert_eq!(auras, vec![CompiledScope::Inherited]);
}

// ─── Section 2/3 — [On Play] reveal: Leopardmon-text card + Beast-ish Digimon

#[test]
fn ex13_038_first_pick_offers_leopardmon_text_cards() {
    let mut r = setup(&["LEO-TEXT", "LEO-OPT", "BEAST"]);
    r.play(0, 0).expect("Salamon played");
    assert_eq!(
        offered_reveal_ids(&r),
        vec!["LEO-OPT".to_string(), "LEO-TEXT".to_string()],
        "any card kind with [Leopardmon] in its name or text"
    );
}

#[test]
fn ex13_038_adds_one_of_each_and_bottoms_the_rest() {
    let mut r = setup(&["LEO", "BEAST", "PLAIN"]);
    r.play(0, 0).expect("Salamon played");
    pick_revealed(&mut r, "LEO");
    assert_eq!(offered_reveal_ids(&r), vec!["BEAST".to_string()]);
    pick_revealed(&mut r, "BEAST");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r), vec!["BEAST".to_string(), "LEO".to_string()]);
    assert_eq!(
        r.game.players[0].deck[0].card_id(&r.game.card_data),
        "PLAIN",
        "rest to the bottom"
    );
    assert!(r.pending_selection().is_none());
}

#[test]
fn ex13_038_trait_bucket_uses_substring_and_excludes_sea_animal() {
    let mut r = setup(&["HOLY-BEAST", "SEA", "PLAIN"]);
    r.play(0, 0).expect("Salamon played");
    // No Leopardmon-text card → first bucket empty, second prompt.
    assert_eq!(
        offered_reveal_ids(&r),
        vec!["HOLY-BEAST".to_string()],
        "[Beast] in any trait (Holy Beast) qualifies; [Sea Animal] does not"
    );
}

#[test]
fn ex13_038_trait_bucket_accepts_animal_and_sovereign() {
    let mut r = setup(&["ANIMAL", "SOV", "PLAIN"]);
    r.play(0, 0).expect("Salamon played");
    assert_eq!(
        offered_reveal_ids(&r),
        vec!["ANIMAL".to_string(), "SOV".to_string()]
    );
}

#[test]
fn ex13_038_trait_bucket_requires_a_digimon_card() {
    let mut r = setup(&["BEAST-TAMER", "PLAIN", "FILL"]);
    r.play(0, 0).expect("Salamon played");
    let _ = r.auto_resolve();
    assert!(hand_ids(&r).is_empty(), "a [Beast] Tamer is not a Digimon card");
    assert_eq!(r.deck_size(0), 7);
}

#[test]
fn ex13_038_no_matches_returns_all_three_to_bottom() {
    let mut r = setup(&["PLAIN", "FILL", "SEA"]);
    r.play(0, 0).expect("Salamon played");
    let _ = r.auto_resolve();
    assert!(hand_ids(&r).is_empty());
    assert_eq!(r.deck_size(0), 7);
}

#[test]
fn ex13_038_one_card_cannot_fill_both_slots() {
    // A Leopardmon-named [Beast] Digimon matches both: it is added once.
    let mut r = builder()
        .add_card(digimon("LEO-BEAST", "Leopardmon X", &["Holy Beast"]))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "PLAIN", "FILL", "LEO-BEAST"])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.play(0, 0).expect("Salamon played");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r), vec!["LEO-BEAST".to_string()]);
}

// ─── Section 3 — Inherited [All Turns] suspended Digimon +1000 DP ────────────

#[test]
fn ex13_038_inherited_suspended_digimon_get_plus_1000() {
    let mut r = builder().start();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let ally = r.place_on_field(0, "ALLY", Some(0));
    assert_eq!(r.effective_dp(carrier), Some(4000));
    set_suspended(&mut r, carrier, true);
    set_suspended(&mut r, ally, true);
    assert_eq!(r.effective_dp(carrier), Some(5000));
    assert_eq!(r.effective_dp(ally), Some(5000));
}

#[test]
fn ex13_038_inherited_does_not_buff_unsuspended_digimon() {
    let mut r = builder().start();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let ally = r.place_on_field(0, "ALLY", Some(0));
    assert_eq!(r.effective_dp(carrier), Some(4000));
    assert_eq!(r.effective_dp(ally), Some(4000));
}

#[test]
fn ex13_038_inherited_not_active_face_up() {
    let mut r = builder().start();
    let s = r.place_on_field(0, CARD_ID, Some(0));
    set_suspended(&mut r, s, true);
    assert_eq!(r.effective_dp(s), Some(2000));
}
