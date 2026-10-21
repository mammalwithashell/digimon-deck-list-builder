//! BT23-039 Perorimon — Digimon, Lv.3, Green, DP 1000, Play Cost 3.
//! Trait line (card image): Stnd./Appmon | Entertainment | Gourmet.
//!
//! # Card text (card image + official Bandai DB bundle — authoritative)
//!
//! Digivolve: Green Lv.2 / Cost 0 (standard circle)
//! [Digivolve] Lv.2 w/[Appmon] trait: Cost 0
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Appmon]
//! trait and 1 card with the [Game] or [Invincible] trait among them to the
//! hand. Return the rest to the bottom of the deck.
//!
//! Link box:
//!   <Link> [Appmon] trait: Cost 1
//!   +2000 DP
//!   [When Linking] You may suspend 1 of your opponent's Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT23/Green/BT23_039.cs
//! - AddSelfDigivolutionRequirementStaticEffect(HasAppmonTraits && IsLevel2, cost 0)
//! - AddSelfLinkConditionStaticEffect(HasAppmonTraits, linkCost 1)
//! - OnEnterFieldAnyone: SimplifiedRevealDeckTopCardsAndSelect(3) with two
//!   buckets (HasAppmonTraits max 1; EqualsTraits Game || Invincible max 1),
//!   remainder DeckBottom
//! - WhenLinked (SetIsLinkedEffect): SelectPermanentEffect Mode.Tap, opponent
//!   Digimon, canNoSelect: true (optional)
//!
//! # Patterns this test covers
//! - A2 reveal-N two-bucket add-to-hand, remainder to deck bottom
//! - C-link self link-condition, linked DP aura, linked [When Linking]
//! - E1 optional "you may" target pick (PASS legal)
//! - Alt-digivolve: printed circle + trait-gated special condition

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START, PASS,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT23-039";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.colors = vec![CardColor::Green];
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn link_bit(perm: PermanentHandle) -> u16 {
    FIELD_EFFECT_START + perm.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT23-039 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("APPMON-CARD", 3, 2000, 3, &["Appmon", "Search"]))
        .add_card(make_digimon("GAME-CARD", 3, 2000, 3, &["Game"]))
        .add_card(make_digimon("INVINCIBLE-CARD", 6, 12000, 12, &["Invincible"]))
        .add_card(make_digimon("APPMON-GAME", 3, 2000, 3, &["Appmon", "Game"]))
        .add_card(make_digimon("PLAIN-CARD", 3, 2000, 3, &["Beast"]))
        .add_card(make_digimon("HOST-APP", 4, 4000, 4, &["Appmon"]))
        .add_card(make_digimon("HOST-PLAIN", 4, 4000, 4, &["Beast"]))
        .add_card(make_digimon("OPP-DIGI", 4, 4000, 4, &["Beast"]))
        .deck(1, &["DECK-PAD"; 12])
}

fn hand_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn deck_bottom_ids(r: &DebugRunner, n: usize) -> Vec<String> {
    // deck[0] is the bottom card (DebugRunnerBuilder.deck: last listed = top).
    r.game.players[0]
        .deck
        .iter()
        .take(n)
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt23_039_printed_metadata() {
    let r = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = r.compiled_card(CARD_ID).expect("BT23-039 in compiled pack");
    assert_eq!(card.name, "Perorimon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(1000));
    assert_eq!(card.cost, Some(3));
}

#[test]
fn bt23_039_has_appmon_link_condition_cost_1() {
    let r = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = r.compiled_card(CARD_ID).unwrap();
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, filter, .. })
                if *cost == 1 && filter.trait_has.as_deref() == Some("Appmon")
        )
    });
    assert!(has, "printed '<Link> [Appmon] trait: Cost 1'");
}

#[test]
fn bt23_039_has_linked_dp_aura_2000() {
    let r = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = r.compiled_card(CARD_ID).unwrap();
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Linked,
                dp_modifier: Some(2000),
                ..
            })
        )
    });
    assert!(has, "printed link box '+2000 DP'");
}

#[test]
fn bt23_039_has_on_play_and_linked_when_linking_clauses() {
    let r = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = r.compiled_card(CARD_ID).unwrap();
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 2, "On Play + When Linking");
    assert!(triggered.iter().any(|t| t.scope == CompiledScope::FaceUp
        && t.when == vec![CompiledTiming::OnPlay]
        && !t.once_per_turn));
    assert!(triggered.iter().any(|t| t.scope == CompiledScope::Linked
        && t.when == vec![CompiledTiming::WhenLinked]
        && !t.once_per_turn));
}

// ─── Section 2 — Digivolution requirements ───────────────────────────────────

fn make_lv2_base(id: &str, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(2);
    c.dp = Some(1000);
    c.play_cost = 1;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn try_digivolve_over(base_card: CardData) -> (bool, i16) {
    let base_id = base_card.card_id.clone();
    let mut r = base()
        .add_card(base_card)
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, &base_id, Some(0));
    let mem_before = r.game.memory;
    let proceeded = r.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand);
    r.game.drain_effect_queue();
    (proceeded, r.game.memory - mem_before)
}

#[test]
fn bt23_039_digivolves_from_plain_green_lv2_for_0() {
    let (ok, delta) = try_digivolve_over(make_lv2_base("GREEN-LV2", CardColor::Green, &[]));
    assert!(ok, "printed circle Green Lv.2 / Cost 0");
    assert_eq!(delta, 0);
}

#[test]
fn bt23_039_digivolves_from_offcolor_appmon_lv2_for_0() {
    let (ok, delta) =
        try_digivolve_over(make_lv2_base("BLUE-LV2-APP", CardColor::Blue, &["Appmon"]));
    assert!(ok, "special condition Lv.2 w/[Appmon] trait: Cost 0");
    assert_eq!(delta, 0);
}

#[test]
fn bt23_039_no_digivolve_from_offcolor_traitless_lv2() {
    let (ok, delta) = try_digivolve_over(make_lv2_base("BLUE-LV2", CardColor::Blue, &[]));
    assert!(!ok, "neither the green circle nor the [Appmon] condition matches");
    assert_eq!(delta, 0);
}

// ─── Section 3 — [On Play] reveal 3, two buckets ────────────────────────────

#[test]
fn bt23_039_on_play_adds_appmon_and_game_cards() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD", "DECK-PAD", "PLAIN-CARD", "GAME-CARD", "APPMON-CARD"])
        .memory(10)
        .start();
    r.play(0, 0).expect("Perorimon played");
    r.auto_resolve().ok();
    let hand = hand_ids(&r);
    assert!(hand.contains(&"APPMON-CARD".to_string()), "{hand:?}");
    assert!(hand.contains(&"GAME-CARD".to_string()), "{hand:?}");
    assert_eq!(hand.len(), 2, "exactly the two bucket picks are added: {hand:?}");
    assert_eq!(r.trash_size(0), 0, "the rest is not trashed");
    assert_eq!(
        deck_bottom_ids(&r, 1),
        vec!["PLAIN-CARD".to_string()],
        "the unpicked revealed card goes to the bottom of the deck"
    );
}

#[test]
fn bt23_039_on_play_invincible_card_fills_second_bucket() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD", "PLAIN-CARD", "INVINCIBLE-CARD", "APPMON-CARD"])
        .memory(10)
        .start();
    r.play(0, 0).expect("Perorimon played");
    r.auto_resolve().ok();
    let hand = hand_ids(&r);
    assert!(hand.contains(&"INVINCIBLE-CARD".to_string()), "[Invincible] qualifies: {hand:?}");
    assert!(hand.contains(&"APPMON-CARD".to_string()), "{hand:?}");
}

#[test]
fn bt23_039_on_play_plain_cards_are_not_added() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD", "PLAIN-CARD", "PLAIN-CARD", "PLAIN-CARD"])
        .memory(10)
        .start();
    r.play(0, 0).expect("Perorimon played");
    r.auto_resolve().ok();
    assert!(hand_ids(&r).is_empty(), "no revealed card qualifies for either bucket");
    assert_eq!(r.trash_size(0), 0);
    assert_eq!(
        deck_bottom_ids(&r, 3),
        vec!["PLAIN-CARD".to_string(); 3],
        "all three revealed cards return to the bottom"
    );
}

#[test]
fn bt23_039_on_play_game_only_reveal_adds_one() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD", "PLAIN-CARD", "PLAIN-CARD", "GAME-CARD"])
        .memory(10)
        .start();
    r.play(0, 0).expect("Perorimon played");
    r.auto_resolve().ok();
    assert_eq!(hand_ids(&r), vec!["GAME-CARD".to_string()], "the Appmon bucket stays empty");
}

/// One card can't fill both buckets: with two [Appmon]+[Game] cards revealed,
/// both are added (one per bucket).
#[test]
fn bt23_039_on_play_one_card_cannot_fill_both_buckets() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD", "PLAIN-CARD", "APPMON-GAME", "APPMON-GAME"])
        .memory(10)
        .start();
    r.play(0, 0).expect("Perorimon played");
    r.auto_resolve().ok();
    assert_eq!(
        hand_ids(&r),
        vec!["APPMON-GAME".to_string(), "APPMON-GAME".to_string()],
        "two distinct cards, one per bucket"
    );

    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD", "PLAIN-CARD", "PLAIN-CARD", "APPMON-GAME"])
        .memory(10)
        .start();
    r.play(0, 0).expect("Perorimon played");
    r.auto_resolve().ok();
    assert_eq!(
        hand_ids(&r),
        vec!["APPMON-GAME".to_string()],
        "a single qualifying card is added once, not twice"
    );
}

// ─── Section 4 — Link box ───────────────────────────────────────────────────

#[test]
fn bt23_039_linked_host_gains_2000_dp() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let base_dp = r.game.effective_dp(host).unwrap();
    r.push_linked_owned(host, CARD_ID, 0);
    r.game.tick_declarative_effects();
    assert_eq!(r.game.effective_dp(host), Some(base_dp + 2000));
}

fn link_from_field_onto_host(
    host_id: &str,
    with_opp: bool,
) -> (DebugRunner, PermanentHandle, Option<PermanentHandle>) {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let host = r.place_on_field(0, host_id, Some(0));
    let pero = r.place_on_field(0, CARD_ID, Some(0));
    let opp = with_opp.then(|| r.place_on_field(1, "OPP-DIGI", Some(0)));
    r.game.enter_main_phase();
    r.game.decode_action(link_bit(pero), 0);
    (r, host, opp)
}

fn pick_host(r: &mut DebugRunner) {
    let action = r
        .game
        .pending_selection
        .as_ref()
        .expect("link host selection must install")
        .valid_action_ids[0];
    r.game.resolve_selection(0, action).expect("host pick");
}

#[test]
fn bt23_039_link_requires_appmon_host() {
    let (r, _host, _) = link_from_field_onto_host("HOST-PLAIN", false);
    let offered = r
        .game
        .pending_selection
        .as_ref()
        .map(|s| s.valid_action_ids.len())
        .unwrap_or(0);
    assert_eq!(offered, 0, "a non-[Appmon] Digimon is not a legal link host");
}

#[test]
fn bt23_039_when_linking_may_suspend_opponent_digimon() {
    let (mut r, _host, opp) = link_from_field_onto_host("HOST-APP", true);
    let opp = opp.unwrap();
    pick_host(&mut r);
    r.game.drain_effect_queue();
    assert!(
        r.pending_is_optional(),
        "'You may suspend' — the target pick must be declinable"
    );
    let view = r.pending_selection_view().expect("suspend target prompt");
    assert_eq!(view.valid_action_ids.len(), 1, "one opponent Digimon in scope");
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    r.auto_resolve().ok();
    assert!(r.game.player(1).battle_area[opp.index as usize].is_suspended);
}

#[test]
fn bt23_039_when_linking_decline_leaves_opponent_unsuspended() {
    let (mut r, _host, opp) = link_from_field_onto_host("HOST-APP", true);
    let opp = opp.unwrap();
    pick_host(&mut r);
    r.game.drain_effect_queue();
    assert!(r.pending_is_optional());
    r.execute_action(0, PASS).unwrap();
    r.auto_resolve().ok();
    assert!(
        !r.game.player(1).battle_area[opp.index as usize].is_suspended,
        "declining the optional suspend leaves the Digimon active"
    );
}

#[test]
fn bt23_039_when_linking_does_not_suspend_own_digimon() {
    let (mut r, host, _) = link_from_field_onto_host("HOST-APP", false);
    pick_host(&mut r);
    r.auto_resolve().ok();
    assert!(
        !r.game.player(0).battle_area[host.index as usize].is_suspended,
        "only the opponent's Digimon are eligible"
    );
    assert!(r.pending_selection().is_none());
}
