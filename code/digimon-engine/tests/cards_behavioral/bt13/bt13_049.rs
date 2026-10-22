//! BT13-049 Lalamon — Digimon, Lv.3, Green, DP 1000, Cost 3.
//! Traits: Vegetation. Form: Rookie. Attribute: Data.
//! Digivolve: green Lv.2, cost 0.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT13-049.md; card image agrees)
//!
//! ```text
//! [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with
//! [Vegetation], [Plant], or [Fairy] in one of its traits and 1 [Yoshino
//! Fujieda] among them to the hand. Place the rest at the bottom of the deck
//! in any order.
//! Inherited: [Your Turn][Once Per Turn] When this Digimon would digivolve, if
//! you have a green Tamer, reduce the digivolution cost by 1.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT13/Green/BT13_049.cs
//!   - On Play: SimplifiedRevealDeckTopCardsAndSelect(3, [IsDigimon &&
//!     (HasPlantTraits || HasFairyTraits) → AddHand max 1, CardNames contains
//!     "Yoshino Fujieda" → AddHand max 1], remainder → DeckBottom).
//!   - Inherited: ChangeCostClass -1 (OPT), IsOwnerTurn, owner has a green
//!     Tamer, applies to THIS permanent's digivolution.
//!
//! # Patterns this test covers
//! - A-row reveal top 3, two mandatory buckets + `no_duplicate_cards`
//!   (BT16-029 shape), "in one of its traits" = `trait_contains`
//! - remainder to the deck bottom in a player-chosen order
//! - `name_is` honors a card's printed name alias (ST24-14 "(Rule) Name: Also
//!   treated as [Yoshino Fujieda]/[Keenan Crier]")
//! - inherited OPT digivolve -1 with a green Tamer (BT14-046 shape)

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT13-049";

fn digimon_with_traits(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn tamer_named(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = vec![CardColor::Green];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn reveal_runner(deck: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-049 in pack")
        .dsl_card("ST24-14")
        .expect("ST24-14 in pack")
        .add_card(digimon_with_traits("VEG", &["Vegetation"]))
        .add_card(digimon_with_traits("VEG-2", &["Vegetation"]))
        .add_card(digimon_with_traits("PLANT", &["Plant"]))
        .add_card(digimon_with_traits("FAIRY", &["Fairy Spirit"]))
        .add_card(digimon_with_traits("DRAGON", &["Dragon"]))
        .add_card(tamer_named("YOSHI", "Yoshino Fujieda", &[]))
        .add_card(tamer_named("VEG-TAMER", "VEG-TAMER", &["Vegetation"]))
        .add_card(make_test_card("F1", "F1"))
        .add_card(make_test_card("F2", "F2"))
        .add_card(make_test_card("F3", "F3"))
        .deck(0, deck)
        .deck(1, &["F1"; 6])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start()
}

fn revealed_action(r: &DebugRunner, id: &str) -> u16 {
    r.game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(i, c)| (c.card_id(&r.game.card_data) == id).then_some(SEL_REVEAL_START + i as u16))
        .unwrap_or_else(|| panic!("{id} is revealed"))
}

fn pick(r: &mut DebugRunner, id: &str) {
    let a = revealed_action(r, id);
    let v = r.pending_selection_view().expect("pending pick");
    assert!(v.valid_action_ids.contains(&a), "{id} pickable; {:?}", v.valid_action_ids);
    r.execute_action(v.selecting_player, a).expect("pick");
}

fn hand_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn deck_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

/// Finish the effect: any remainder ordering prompt is answered in offer order.
fn finish(r: &mut DebugRunner) {
    r.auto_resolve().expect("finish");
}

// ─── SECTION 1 — Structural ─────────────────────────────────────────────────

#[test]
fn bt13_049_metadata_and_green_lv2_cost0_circle() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.name, "Lalamon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(1000));
    assert_eq!(card.traits, vec!["Vegetation".to_string()]);
    assert_eq!(card.alt_paths.len(), 1);
    assert_eq!(card.alt_paths[0].cost, Some(CompiledCost::Literal(0)));
}

#[test]
fn bt13_049_on_play_is_mandatory_and_inherited_reducer_is_opt() {
    let card = dsl_card_data::compiled(CARD_ID);
    let on_play = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when == vec![CompiledTiming::OnPlay] && t.scope == CompiledScope::FaceUp =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("[On Play]");
    assert!(!on_play.optional);
    let inherited: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                scope: CompiledScope::Inherited,
                once_per_turn,
                optional,
                amount,
                ..
            }) => Some((*once_per_turn, *optional, *amount)),
            _ => None,
        })
        .collect();
    assert_eq!(inherited, vec![(true, false, Some(1))]);
}

// ─── SECTION 2 — [On Play] reveal 3 ─────────────────────────────────────────

/// Both buckets are filled; the leftover goes to the bottom of the deck.
#[test]
fn bt13_049_adds_a_vegetation_digimon_and_yoshino_rest_to_bottom() {
    let mut r = reveal_runner(&["F3", "F1", "YOSHI", "VEG"]);
    r.play(0, 0).expect("play Lalamon");
    let v = r.pending_selection_view().expect("bucket 1");
    assert!(!v.is_optional && !v.valid_action_ids.contains(&PASS), "mandatory");
    pick(&mut r, "VEG");
    let v = r.pending_selection_view().expect("bucket 2");
    assert!(!v.is_optional && !v.valid_action_ids.contains(&PASS), "mandatory");
    pick(&mut r, "YOSHI");
    finish(&mut r);
    let hand = hand_ids(&r);
    assert!(hand.contains(&"VEG".into()) && hand.contains(&"YOSHI".into()), "{hand:?}");
    assert_eq!(r.game.players[0].deck.len(), 2, "F1 bottomed under F3");
    assert_eq!(deck_ids(&r).iter().filter(|i| *i == "F1").count(), 1);
}

/// "[Plant]" and a trait merely CONTAINING "Fairy" both qualify; a plain
/// Digimon does not.
#[test]
fn bt13_049_plant_and_fairy_substring_traits_qualify() {
    let mut r = reveal_runner(&["F3", "DRAGON", "FAIRY", "PLANT"]);
    r.play(0, 0).expect("play");
    let v = r.pending_selection_view().expect("bucket 1");
    assert!(v.valid_action_ids.contains(&revealed_action(&r, "PLANT")));
    assert!(v.valid_action_ids.contains(&revealed_action(&r, "FAIRY")));
    assert!(!v.valid_action_ids.contains(&revealed_action(&r, "DRAGON")));
}

/// Bucket 1 needs a Digimon CARD: a [Vegetation] Tamer is not offered, so the
/// only pick is Yoshino.
#[test]
fn bt13_049_vegetation_tamer_and_wrong_trait_digimon_not_offered() {
    let mut r = reveal_runner(&["F3", "DRAGON", "VEG-TAMER", "YOSHI"]);
    r.play(0, 0).expect("play");
    let v = r.pending_selection_view().expect("Yoshino bucket");
    assert_eq!(v.valid_action_ids, vec![revealed_action(&r, "YOSHI")]);
    pick(&mut r, "YOSHI");
    finish(&mut r);
    let hand = hand_ids(&r);
    assert!(!hand.contains(&"VEG-TAMER".into()) && !hand.contains(&"DRAGON".into()));
    assert!(hand.contains(&"YOSHI".into()));
}

/// No match at all: nothing is added, all 3 go to the bottom.
#[test]
fn bt13_049_no_match_adds_nothing() {
    let mut r = reveal_runner(&["VEG", "F3", "F2", "F1"]);
    let hand_before = r.game.players[0].hand.len();
    r.play(0, 0).expect("play");
    finish(&mut r);
    assert_eq!(r.game.players[0].hand.len(), hand_before - 1, "only Lalamon left the hand");
    assert_eq!(r.game.players[0].deck.len(), 4);
    assert_eq!(
        deck_ids(&r).last().map(String::as_str),
        Some("VEG"),
        "the unrevealed card is still on top (last = top)"
    );
}

/// One card per bucket: with two qualifying Digimon only one is added; the
/// other goes to the bottom.
#[test]
fn bt13_049_adds_only_one_qualifying_digimon() {
    let mut r = reveal_runner(&["F3", "F1", "VEG-2", "VEG"]);
    r.play(0, 0).expect("play");
    pick(&mut r, "VEG");
    finish(&mut r);
    let hand = hand_ids(&r);
    assert!(hand.contains(&"VEG".into()));
    assert!(!hand.contains(&"VEG-2".into()), "max 1");
    assert!(deck_ids(&r).contains(&"VEG-2".to_string()));
}

/// "in any order": the remainder order is a player choice.
#[test]
fn bt13_049_remainder_order_is_player_chosen() {
    for first in ["F1", "F2"] {
        let mut r = reveal_runner(&["F3", "F2", "F1", "VEG"]);
        r.play(0, 0).expect("play");
        pick(&mut r, "VEG");
        let v = r.pending_selection_view().expect("ordering prompt");
        assert!(
            matches!(v.kind, SelectionKind::OrderedPermutation { remaining: 2 }),
            "{:?}",
            v.kind
        );
        let a = revealed_action(&r, first);
        r.execute_action(v.selecting_player, a).expect("order 1");
        finish(&mut r);
        let other = if first == "F1" { "F2" } else { "F1" };
        assert_eq!(
            deck_ids(&r),
            vec![other.to_string(), first.to_string(), "F3".to_string()],
            "bottomed under F3 in the chosen order (index 0 = bottom)"
        );
    }
}

/// ST24-14 "Yoshino Fujieda & Keenan Crier" is also treated as [Yoshino
/// Fujieda] (its printed ＜Rule＞), so it fills the Yoshino bucket.
#[test]
fn bt13_049_name_alias_yoshino_qualifies() {
    let mut r = reveal_runner(&["F3", "F1", "ST24-14", "VEG"]);
    r.play(0, 0).expect("play");
    pick(&mut r, "VEG");
    let v = r.pending_selection_view().expect("Yoshino bucket");
    assert!(
        v.valid_action_ids.contains(&revealed_action(&r, "ST24-14")),
        "ST24-14 is also [Yoshino Fujieda]: {:?}",
        v.valid_action_ids
    );
    pick(&mut r, "ST24-14");
    finish(&mut r);
    assert!(hand_ids(&r).contains(&"ST24-14".into()));
}

// ─── SECTION 3 — Inherited digivolve -1 with a green Tamer ─────────────────

fn inh_runner() -> DebugRunner {
    let tamer = |id: &str, color: CardColor| {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Tamer;
        c.colors = vec![color];
        c.level = None;
        c.dp = None;
        c
    };
    let green = |id: &str, level: u8| {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Digimon;
        c.colors = vec![CardColor::Green];
        c.level = Some(level);
        c.dp = Some(3000);
        c.evo_costs = vec![EvoCost {
            card_color: 3,
            level: level - 1,
            memory_cost: 3,
        }];
        c
    };
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-049 in pack")
        .add_card(tamer("G-TAMER", CardColor::Green))
        .add_card(tamer("R-TAMER", CardColor::Red))
        .add_card(green("TOP-G4", 4))
        .add_card(green("G-LV5", 5))
        .add_card(green("G-LV6", 6))
        .add_card(make_test_card("F1", "F1"))
        .deck(0, &["F1"; 10])
        .deck(1, &["F1"; 10])
        .memory(10)
        .start();
    r.set_first_player(0);
    r.game.enter_main_phase();
    r.game.set_memory(10);
    r
}

fn digivolve(r: &mut DebugRunner, carrier: PermanentHandle, id: &str) -> i16 {
    let idx = r.add_to_hand(0, id);
    let before = r.memory();
    let ok = r
        .game
        .digivolve_from_hand(0, idx, carrier.index as usize, PlaySource::ByHand);
    assert!(ok, "digivolve resolves");
    let _ = r.auto_resolve();
    (before - r.memory()).abs()
}

#[test]
fn bt13_049_inherited_with_green_tamer_reduces_digivolve_cost_by_1() {
    let mut r = inh_runner();
    r.place_on_field(0, "G-TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "TOP-G4"]);
    assert_eq!(digivolve(&mut r, carrier, "G-LV5"), 2);
}

#[test]
fn bt13_049_inherited_without_green_tamer_pays_full() {
    let mut r = inh_runner();
    r.place_on_field(0, "R-TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "TOP-G4"]);
    assert_eq!(digivolve(&mut r, carrier, "G-LV5"), 3);
}

#[test]
fn bt13_049_inherited_inactive_while_lalamon_is_the_top_card() {
    let mut r = inh_runner();
    r.place_on_field(0, "G-TAMER", Some(0));
    let me = r.place_on_field(0, CARD_ID, Some(0));
    assert_eq!(digivolve(&mut r, me, "TOP-G4"), 3);
}

#[test]
fn bt13_049_inherited_once_per_turn() {
    let mut r = inh_runner();
    r.place_on_field(0, "G-TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "TOP-G4"]);
    assert_eq!(digivolve(&mut r, carrier, "G-LV5"), 2);
    assert_eq!(digivolve(&mut r, carrier, "G-LV6"), 3, "[Once Per Turn]");
}

#[test]
fn bt13_049_inherited_not_on_the_opponents_turn() {
    let mut r = inh_runner();
    r.place_on_field(0, "G-TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "TOP-G4"]);
    r.game.turn_player_idx = 1;
    assert_eq!(digivolve(&mut r, carrier, "G-LV5"), 3, "[Your Turn]");
}
