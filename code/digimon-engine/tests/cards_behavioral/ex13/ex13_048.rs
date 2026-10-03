//! EX13-048 Kotemon — Digimon, Lv.3, Black, DP 2000, Cost 3.
//! Traits: Reptile. Form: Rookie. Attribute: Data.
//!
//! # Card text (`data/cards.json` — no per-card JSON was ingested for this
//! card; official Bandai DB bundle `data/card_bundles/EX13-048.md` agrees)
//!
//! ```text
//! Digivolve: Black Lv.2 / cost 0
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 card with [Knightmon]
//! in its text and 1 card with [Knightmon] in its name among them to the hand.
//! Return the rest to the bottom of the deck.
//!
//! Inherited Effect:
//! [All Turns] [Once Per Turn] When this Digimon would leave the battle area
//! other than by your effects, by deleting 1 of your other Digimon with
//! [Knightmon] in its text, it doesn't leave.
//! ```
//! Official Q&A: "a card with [Knightmon] in its text" covers name, traits,
//! effects, inherited effects, (Rule) and requirement text — e.g. a card
//! named [DarkKnightmon] qualifies (`in_text_contains`).
//!
//! # DCGO C# reference
//! None — DCGO has no `EX13_048.cs` at `b9a0638cd`. The reveal clause is the
//! EX13-038 Salamon two-bucket `reveal_search` shape; the inherited clause is
//! the EX13-027 Chuumon / EX11-022 Karakurumon inherited leave-prevention
//! with a [Knightmon]-text cost.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - A2/A3 reveal-3 two-bucket pick (both to hand, cross-bucket de-dup),
//!   remainder bottom.
//! - F3 inherited leave-prevention replacement, OPT, select-and-delete cost,
//!   "other than by your effects" cause gate.

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
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-048";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, name: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Black];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = 3;
    c.traits.clear();
    c
}

fn knightmon_text(id: &str, name: &str, level: u8) -> CardData {
    let mut c = digimon(id, name, level);
    c.effect_text = "[On Play] If you have [Knightmon], draw 1.".to_string();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-048 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("KNIGHT", "Knightmon", 5))
        .add_card(digimon("DARK", "DarkKnightmon", 6))
        .add_card(knightmon_text("TEXT", "Texter", 4))
        .add_card({
            let mut c = make_test_card("TEXT-OPT", "Knight Charge");
            c.card_kind = CardKind::Option;
            c.level = None;
            c.dp = None;
            c.play_cost = 2;
            c.effect_text = "[Main] If you have [Knightmon], delete 1 Digimon.".to_string();
            c
        })
        .add_card(digimon("PLAIN", "Plainmon", 4))
        .add_card(digimon("FILL", "Filler", 3))
        .add_card(digimon("CARRIER", "Carrier", 4))
        .add_card(knightmon_text("ALLY-TEXT", "Ally Texter", 4))
        .add_card(digimon("ALLY-PLAIN", "Plain Ally", 4))
        .add_card(digimon("OPP-KNIGHT", "Knightmon", 5))
}

/// Kotemon in hand, `deck_top` stacked on the deck (index 0 = first revealed).
fn setup(deck_top: &[&str]) -> DebugRunner {
    let mut deck: Vec<&str> = vec!["FILL"; 4];
    for c in deck_top.iter().rev() {
        deck.push(c);
    }
    let mut r = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &deck)
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
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

fn field_ids(r: &DebugRunner, player: usize) -> Vec<String> {
    r.game.players[player]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn handle_of(r: &DebugRunner, id: &str) -> PermanentHandle {
    let i = field_ids(r, 0)
        .iter()
        .position(|c| c == id)
        .unwrap_or_else(|| panic!("{id} on field"));
    PermanentHandle {
        player: 0,
        index: i as u8,
    }
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

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_048_printed_metadata_and_digivolve_path() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(3), Some(2000), Some(3)));
    assert_eq!(c.color, vec![CompiledColor::Black]);
    assert_eq!(c.traits, vec!["Reptile".to_string()]);
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 1, "Black Lv.2 circle");
    assert_eq!(digi[0].cost, Some(CompiledCost::Literal(0)));
}

#[test]
fn ex13_048_clause_shape() {
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
    assert!(!triggered[0].optional, "reveal is mandatory");
    assert_eq!(triggered[0].scope, CompiledScope::FaceUp);

    let replacements: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                scope,
                optional,
                once_per_turn,
                ..
            }) => Some((*scope, *optional, *once_per_turn)),
            _ => None,
        })
        .collect();
    assert_eq!(
        replacements,
        vec![(CompiledScope::Inherited, true, true)],
        "one inherited, optional (by deleting…), once-per-turn replacement"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2/3 — [On Play] reveal: 1 [Knightmon]-text card + 1 [Knightmon]-name
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_048_text_bucket_offers_any_card_with_knightmon_in_text() {
    let mut r = setup(&["TEXT", "TEXT-OPT", "KNIGHT"]);
    r.play(0, 0).expect("Kotemon played");
    assert_eq!(
        offered_reveal_ids(&r),
        vec!["KNIGHT".to_string(), "TEXT".to_string(), "TEXT-OPT".to_string()],
        "name, effect text, any card kind all count as 'in its text'"
    );
}

#[test]
fn ex13_048_text_bucket_excludes_cards_without_knightmon() {
    let mut r = setup(&["PLAIN", "TEXT", "FILL"]);
    r.play(0, 0).expect("Kotemon played");
    assert_eq!(offered_reveal_ids(&r), vec!["TEXT".to_string()]);
}

#[test]
fn ex13_048_name_bucket_requires_knightmon_in_name() {
    // The text bucket takes TEXT-OPT; the name bucket must not offer TEXT
    // (Knightmon only in its effect text).
    let mut r = setup(&["TEXT-OPT", "TEXT", "DARK"]);
    r.play(0, 0).expect("Kotemon played");
    pick_revealed(&mut r, "TEXT-OPT");
    assert_eq!(
        offered_reveal_ids(&r),
        vec!["DARK".to_string()],
        "[Knightmon] in its name includes DarkKnightmon, excludes text-only cards"
    );
}

#[test]
fn ex13_048_adds_one_of_each_and_bottoms_the_rest() {
    let mut r = setup(&["TEXT", "KNIGHT", "PLAIN"]);
    r.play(0, 0).expect("Kotemon played");
    pick_revealed(&mut r, "TEXT");
    pick_revealed(&mut r, "KNIGHT");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r), vec!["KNIGHT".to_string(), "TEXT".to_string()]);
    assert_eq!(
        r.game.players[0].deck[0].card_id(&r.game.card_data),
        "PLAIN",
        "rest to the bottom"
    );
    assert!(r.pending_selection().is_none());
}

#[test]
fn ex13_048_one_card_cannot_fill_both_slots() {
    // A lone Knightmon matches both buckets: it is added once.
    let mut r = setup(&["KNIGHT", "PLAIN", "FILL"]);
    r.play(0, 0).expect("Kotemon played");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r), vec!["KNIGHT".to_string()]);
    assert_eq!(r.deck_size(0), 6, "the other 2 go to the bottom");
}

#[test]
fn ex13_048_no_match_returns_all_three_to_bottom() {
    let mut r = setup(&["PLAIN", "FILL", "PLAIN"]);
    r.play(0, 0).expect("Kotemon played");
    let _ = r.auto_resolve();
    assert!(hand_ids(&r).is_empty());
    assert_eq!(r.deck_size(0), 7);
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3/5 — Inherited [All Turns][OPT] leave prevention
// ════════════════════════════════════════════════════════════════════════════

fn inherited_setup() -> (DebugRunner, PermanentHandle) {
    let mut r = builder().deck(0, &["FILL"; 6]).deck(1, &["FILL"; 6]).start();
    r.skip_mulligan();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    (r, carrier)
}

fn accept_and_pay(r: &mut DebugRunner) {
    let accept = r.pending_selection_view().expect("replacement accept prompt");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    r.execute_action(0, accept.valid_action_ids[0])
        .expect("accept replacement");
    let cost = r.pending_selection_view().expect("cost prompt");
    let pick = *cost
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("a cost Digimon");
    r.execute_action(0, pick).expect("pay");
    let _ = r.auto_resolve();
}

#[test]
fn ex13_048_inherited_deletes_other_knightmon_text_digimon_and_carrier_stays() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "ALLY-TEXT", Some(0));
    r.place_on_field(0, "ALLY-PLAIN", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let accept = r.pending_selection_view().expect("replacement accept prompt");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    assert!(accept.is_optional);
    r.execute_action(0, accept.valid_action_ids[0])
        .expect("accept replacement");

    let cost = r.pending_selection_view().expect("select cost Digimon");
    assert_eq!(cost.kind, SelectionKind::OwnField);
    let offered: Vec<_> = cost
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .collect();
    assert_eq!(offered.len(), 1, "only the other [Knightmon]-text Digimon");
    r.execute_action(0, *offered[0]).expect("pay cost");
    let _ = r.auto_resolve();

    let field = field_ids(&r, 0);
    assert!(field.contains(&"CARRIER".to_string()), "carrier doesn't leave");
    assert!(!field.contains(&"ALLY-TEXT".to_string()), "cost Digimon deleted");
    assert!(field.contains(&"ALLY-PLAIN".to_string()));
}

#[test]
fn ex13_048_inherited_knightmon_named_digimon_also_qualifies() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "DARK", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    accept_and_pay(&mut r);
    let field = field_ids(&r, 0);
    assert!(field.contains(&"CARRIER".to_string()));
    assert!(!field.contains(&"DARK".to_string()));
}

#[test]
fn ex13_048_inherited_not_offered_without_other_knightmon_text_digimon() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "ALLY-PLAIN", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(
        r.pending_selection_view().is_none(),
        "no payable cost → no prevention offer"
    );
    assert!(!field_ids(&r, 0).contains(&"CARRIER".to_string()));
}

#[test]
fn ex13_048_inherited_opponent_knightmon_is_not_a_legal_cost() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(1, "OPP-KNIGHT", Some(1));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(
        r.pending_selection_view().is_none(),
        "\"1 of YOUR other Digimon\" — the opponent's Knightmon can't pay"
    );
    assert!(!field_ids(&r, 0).contains(&"CARRIER".to_string()));
    assert!(field_ids(&r, 1).contains(&"OPP-KNIGHT".to_string()));
}

#[test]
fn ex13_048_inherited_carrier_itself_with_knightmon_text_is_not_other() {
    let mut r = builder().deck(0, &["FILL"; 6]).start();
    r.skip_mulligan();
    let carrier = r.place_stack(0, &[CARD_ID, "KNIGHT"]);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none());
    assert!(!field_ids(&r, 0).contains(&"KNIGHT".to_string()));
}

#[test]
fn ex13_048_inherited_not_offered_for_own_effect_removal() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "ALLY-TEXT", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OwnEffect);
    assert!(
        r.pending_selection_view().is_none(),
        "\"other than by your effects\""
    );
    assert!(!field_ids(&r, 0).contains(&"CARRIER".to_string()));
}

#[test]
fn ex13_048_inherited_offered_for_battle_deletion_cause() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "ALLY-TEXT", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::Battle);
    assert!(
        r.pending_selection_view().is_some(),
        "battle deletion is not 'your effects'"
    );
}

#[test]
fn ex13_048_inherited_decline_lets_carrier_leave() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "ALLY-TEXT", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    let accept = r.pending_selection_view().expect("accept prompt");
    assert!(accept.is_optional, "\"by deleting…\" is a may-pay");
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    let field = field_ids(&r, 0);
    assert!(!field.contains(&"CARRIER".to_string()));
    assert!(field.contains(&"ALLY-TEXT".to_string()));
}

#[test]
fn ex13_048_inherited_protects_only_this_digimon() {
    let (mut r, _carrier) = inherited_setup();
    r.place_on_field(0, "ALLY-TEXT", Some(0));
    let plain = r.place_on_field(0, "ALLY-PLAIN", Some(0));
    r.game
        .delete_permanent_with_cause(plain, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none());
    assert!(!field_ids(&r, 0).contains(&"ALLY-PLAIN".to_string()));
}

#[test]
fn ex13_048_inherited_once_per_turn() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "ALLY-TEXT", Some(0));
    r.place_on_field(0, "DARK", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    accept_and_pay(&mut r);
    assert!(field_ids(&r, 0).contains(&"CARRIER".to_string()));

    let carrier = handle_of(&r, "CARRIER");
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(
        r.pending_selection_view().is_none(),
        "once per turn: not offered again this turn"
    );
    assert!(!field_ids(&r, 0).contains(&"CARRIER".to_string()));
}

#[test]
fn ex13_048_inherited_lockout_clears_next_turn() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "ALLY-TEXT", Some(0));
    r.place_on_field(0, "DARK", Some(0));
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    accept_and_pay(&mut r);

    r.end_turn();
    let _ = r.auto_resolve();
    let carrier = handle_of(&r, "CARRIER");
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(
        r.pending_selection_view().is_some(),
        "OPT resets on the next turn"
    );
}
