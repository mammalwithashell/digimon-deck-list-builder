//! BT23-022 Oujamon — Digimon, Lv.5, Blue/Red, DP 9000, Play Cost 9.
//! Trait line (card image): Ult./Appmon | Game | Battle.
//!
//! # Card text (card image + official Bandai DB bundle — authoritative)
//!
//! Digivolve: Lv.4 / Cost 4 (blue/red circle) · Sup. / Cost 4 (rainbow circle)
//! [App Fusion] [Dosukomon] & [Coachmon]: Cost 0
//!   If 2 such cards are linked together, stack the link card on top and digivolve.
//! <Raid>
//! [When Digivolving] [When Attacking] [Once Per Turn] You may link 1 level 4
//!   or lower Digimon card from your hand or this Digimon's digivolution cards
//!   to this Digimon without paying the cost.
//! [All Turns] [Once Per Turn] When this Digimon gets linked, it may unsuspend.
//!
//! Link box:
//!   <Link> [Appmon] trait: Cost 3
//!   +4000 DP
//!   <Security A. +1>
//!
//! Official Q&A Q5243: the [When Digivolving]/[When Attacking] effect can't
//! link a card that doesn't have <Link>.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT23/Blue/BT23_022.cs
//! - AddSelfDigivolutionRequirementStaticEffect(HasSuperAppTraits, cost 4)
//! - AddAppFusionConditionClass (Dosukomon top + Coachmon linked, or reverse), cost 0
//! - AddSelfLinkConditionStaticEffect(IsDigimon && HasAppmonTraits, linkCost 3)
//! - RaidSelfEffect
//! - WD + WA ActivateClass (shared hash "BT23-022_WD/WA", maxCount 1, optional):
//!   level <= 4 Digimon card with CanLinkToTargetPermanent(this) from hand or
//!   this Digimon's digivolution cards (bool zone pick when both qualify),
//!   canNoSelect: true, AddLinkCard (free)
//! - WhenLinked ActivateClass ("BT23-022_WL", maxCount 1, optional): this
//!   Digimon gets linked → IUnsuspendPermanents(self)
//! - ChangeSelfSAttackStaticEffect(+1, isLinkedEffect: true)
//!
//! # Patterns this test covers
//! - C-link link_cards to self from hand / own digivolution cards (free)
//! - E2 OPT shared across two timings + outer optional confirm
//! - when_card_linked_to_this optional self-unsuspend (OPT)
//! - Link box: link_condition, +DP aura, Security A. +1 aura
//! - Alt paths: dual-colour circle, Sup. trait circle, App Fusion

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT23-022";
const PERORIMON: &str = "BT23-039";

fn make_digimon(id: &str, name: &str, level: u8, color: CardColor, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(1000 * level as i32);
    card.play_cost = level as u16;
    card.colors = vec![color];
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT23-022 YAML parses and compiles")
        .dsl_card(PERORIMON)
        .expect("BT23-039 YAML parses and compiles")
        // Level 4 / level 5 Digimon carrying a [Link] (any host, cost 0).
        .link_requirement("LINK-LV4", 0)
        .link_requirement("LINK-LV5", 0)
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("LINK-LV4", "Link Lv4", 4, CardColor::Green, &["Beast"]))
        .add_card(make_digimon("LINK-LV5", "Link Lv5", 5, CardColor::Green, &["Beast"]))
        // Level 4 Digimon WITHOUT <Link> — Q5243: can't be linked.
        .add_card(make_digimon("NOLINK-LV4", "No Link Lv4", 4, CardColor::Green, &["Beast"]))
        .add_card(make_digimon("APP-HOST", "App Host", 4, CardColor::Green, &["Appmon"]))
        .add_card(make_digimon("PLAIN-HOST", "Plain Host", 4, CardColor::Green, &["Beast"]))
        .add_card(make_digimon("OPP-DIGI", "Opp Digi", 4, CardColor::Green, &["Beast"]))
        .add_card(make_digimon("DOSU", "Dosukomon", 4, CardColor::Red, &["Appmon"]))
        .add_card(make_digimon("COACH", "Coachmon", 4, CardColor::Blue, &["Appmon"]))
        .deck(1, &["DECK-PAD"; 12])
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, at: PermanentHandle) {
    r.game.enqueue_triggered(timing, TriggerSource::Permanent(at));
    r.game.drain_effect_queue();
}

fn linked_count(r: &DebugRunner, h: PermanentHandle) -> usize {
    r.game.player(h.player).battle_area[h.index as usize]
        .linked_cards
        .len()
}

fn linked_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    r.game.player(h.player).battle_area[h.index as usize]
        .linked_cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn is_suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.player(h.player).battle_area[h.index as usize].is_suspended
}

fn hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

/// Oujamon (suspended, as if attacking) on the field, main phase.
fn oujamon_on_field(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = base()
        .hand(0, hand)
        .deck(0, &["DECK-PAD"; 12])
        .memory(3)
        .start();
    let ouja = r.place_on_field(0, CARD_ID, Some(0));
    r.game.enter_main_phase();
    (r, ouja)
}

/// Accept the outer "you may" confirm of the WD/WA link clause.
fn accept_outer(r: &mut DebugRunner) {
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Replacement),
        "the optional link effect opens with an accept/decline confirm"
    );
    r.accept_optional_trigger().expect("accept the optional effect");
}

/// Pick the hand card with `card_id` in the current link card-select prompt.
fn pick_card(r: &mut DebugRunner) {
    let view = r.pending_selection_view().expect("card pick prompt");
    assert!(!view.valid_action_ids.is_empty(), "a linkable card is offered");
    r.execute_action(0, view.valid_action_ids[0]).expect("pick");
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt23_022_printed_metadata() {
    let r = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = r.compiled_card(CARD_ID).unwrap();
    assert_eq!(card.name, "Oujamon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(9000));
    assert_eq!(card.cost, Some(9));
}

#[test]
fn bt23_022_link_box_structure() {
    let r = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = r.compiled_card(CARD_ID).unwrap();
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, filter, .. })
            if *cost == 3 && filter.trait_has.as_deref() == Some("Appmon")
    )), "<Link> [Appmon] trait: Cost 3");
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope: CompiledScope::Linked,
            dp_modifier: Some(4000),
            ..
        })
    )), "link box +4000 DP");
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope: CompiledScope::Linked,
            security_attack: Some(1),
            ..
        })
    )), "link box <Security A. +1>");
}

#[test]
fn bt23_022_triggered_clause_shapes() {
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
    assert_eq!(triggered.len(), 2);
    let wdwa = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::WhenAttacking))
        .expect("WD/WA clause");
    assert_eq!(
        wdwa.when,
        vec![CompiledTiming::WhenDigivolving, CompiledTiming::WhenAttacking]
    );
    assert!(wdwa.once_per_turn && wdwa.optional);
    let linked = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::WhenCardLinkedToThis])
        .expect("[All Turns] when this Digimon gets linked clause");
    assert!(linked.once_per_turn && linked.optional);
    assert_eq!(linked.scope, CompiledScope::FaceUp);
}

#[test]
fn bt23_022_has_raid() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).start();
    let ouja = r.place_on_field(0, CARD_ID, Some(0));
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(ouja, Keyword::Raid), "<Raid>");
}

// ─── Section 2 — Digivolution requirements / App Fusion ──────────────────────

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
    // Clear the [When Digivolving] confirm, if any.
    if r.pending_selection().is_some() {
        r.execute_action(0, PASS).ok();
    }
    r.game.drain_effect_queue();
    (proceeded, r.game.memory - mem_before)
}

#[test]
fn bt23_022_digivolves_from_blue_lv4_for_4() {
    let (ok, d) = try_digivolve_over(make_digimon("B4", "B4", 4, CardColor::Blue, &[]));
    assert!(ok, "Lv.4 blue circle");
    assert_eq!(d, -4);
}

#[test]
fn bt23_022_digivolves_from_red_lv4_for_4() {
    let (ok, d) = try_digivolve_over(make_digimon("R4", "R4", 4, CardColor::Red, &[]));
    assert!(ok, "Lv.4 red circle");
    assert_eq!(d, -4);
}

#[test]
fn bt23_022_digivolves_from_any_colour_sup_for_4() {
    let (ok, d) = try_digivolve_over(make_digimon("G-SUP", "G Sup", 4, CardColor::Green, &["Sup.", "Appmon"]));
    assert!(ok, "Sup. circle (rainbow: any colour)");
    assert_eq!(d, -4);
}

#[test]
fn bt23_022_no_digivolve_from_green_lv4_without_sup() {
    let (ok, d) = try_digivolve_over(make_digimon("G4", "G4", 4, CardColor::Green, &["Appmon"]));
    assert!(!ok, "a green non-Sup. Lv.4 matches no printed requirement");
    assert_eq!(d, 0);
}

#[test]
fn bt23_022_app_fusion_dosukomon_with_linked_coachmon() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let host = r.place_on_field(0, "DOSU", Some(0));
    let linked = r.push_linked_owned(host, "COACH", 0);
    let action = encode_digivolve(0, host.index as u16);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[action as usize], 1.0, "App Fusion offered onto Dosukomon+Coachmon");
    r.game.decode_action(action, 0);
    // The red Lv.4 test Dosukomon also meets the paid Lv.4 circle, so the
    // engine asks which cost to pay; choose the App Fusion one.
    assert_eq!(r.pending_kind(), Some(SelectionKind::EffectChoice));
    let view = r.pending_selection_view().unwrap();
    let af = view
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .position(|c| c.label.contains("App Fusion"))
        .expect("App Fusion route offered");
    r.execute_branch(af);
    let perm = &r.game.players[0].battle_area[host.index as usize];
    assert_eq!(perm.top_card().card_id(&r.game.card_data), CARD_ID);
    assert!(perm.card_sources.iter().any(|c| c.handle() == linked));
    assert!(perm.linked_cards.is_empty());
    assert_eq!(r.game.memory, 5, "App Fusion cost 0");
}

#[test]
fn bt23_022_no_app_fusion_with_one_named_card() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    // Dosukomon with a non-Coachmon link card: no App Fusion route. The test
    // Dosukomon is a red Lv.4, so only the paid Lv.4 circle is available.
    let host = r.place_on_field(0, "DOSU", Some(0));
    r.push_linked_owned(host, "LINK-LV4", 0);
    let action = encode_digivolve(0, host.index as u16);
    r.game.decode_action(action, 0);
    assert_ne!(
        r.pending_kind(),
        Some(SelectionKind::EffectChoice),
        "no cost choice: App Fusion is not available"
    );
    assert_eq!(
        r.game.memory, 1,
        "without the Dosukomon + Coachmon pair only the cost-4 circle applies"
    );
}

// ─── Section 3 — Link box ───────────────────────────────────────────────────

#[test]
fn bt23_022_linked_host_gains_4000_dp() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).start();
    let host = r.place_on_field(0, "APP-HOST", Some(0));
    let base_dp = r.game.effective_dp(host).unwrap();
    r.push_linked_owned(host, CARD_ID, 0);
    r.game.tick_declarative_effects();
    assert_eq!(r.game.effective_dp(host), Some(base_dp + 4000));
}

#[test]
fn bt23_022_linked_host_checks_one_additional_security() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .security(1, &["DECK-PAD"; 4])
        .start();
    let host = r.place_on_field(0, "APP-HOST", Some(0));
    r.push_linked_owned(host, CARD_ID, 0);
    r.game.tick_declarative_effects();
    r.game.enter_main_phase();
    r.attack_player(host, 1, false);
    r.auto_resolve().ok();
    assert_eq!(r.security_count(1), 2, "<Security A. +1> on the host: 2 checks");
}

#[test]
fn bt23_022_unlinked_host_checks_one_security() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .security(1, &["DECK-PAD"; 4])
        .start();
    let host = r.place_on_field(0, "APP-HOST", Some(0));
    r.game.enter_main_phase();
    r.attack_player(host, 1, false);
    r.auto_resolve().ok();
    assert_eq!(r.security_count(1), 3, "control: one check without the link");
}

// ─── Section 4 — [WD][WA][OPT] link a level 4 or lower Digimon card ─────────

#[test]
fn bt23_022_wd_links_level4_from_hand_free() {
    let (mut r, ouja) = oujamon_on_field(&["LINK-LV4"]);
    fire(&mut r, EffectTiming::WhenDigivolving, ouja);
    accept_outer(&mut r);
    pick_card(&mut r);
    r.auto_resolve().ok();
    assert_eq!(linked_ids(&r, ouja), vec!["LINK-LV4".to_string()]);
    assert!(hand_ids(&r, 0).is_empty());
    assert_eq!(r.game.memory, 3, "linked without paying the cost");
}

#[test]
fn bt23_022_wa_links_level4_from_own_digivolution_cards() {
    let (mut r, ouja) = oujamon_on_field(&[]);
    r.push_source(ouja, "LINK-LV4");
    fire(&mut r, EffectTiming::WhenAttacking, ouja);
    accept_outer(&mut r);
    pick_card(&mut r);
    r.auto_resolve().ok();
    assert_eq!(linked_ids(&r, ouja), vec!["LINK-LV4".to_string()]);
    assert_eq!(
        r.game.player(0).battle_area[ouja.index as usize].card_sources.len(),
        1,
        "the digivolution card moved to the link slot"
    );
}

#[test]
fn bt23_022_both_zones_offer_a_zone_choice() {
    let (mut r, ouja) = oujamon_on_field(&["LINK-LV4"]);
    r.push_source(ouja, "LINK-LV4");
    fire(&mut r, EffectTiming::WhenDigivolving, ouja);
    accept_outer(&mut r);
    assert_eq!(r.pending_kind(), Some(SelectionKind::EffectChoice));
    let view = r.pending_selection_view().unwrap();
    let labels: Vec<String> = view
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .map(|c| c.label.clone())
        .collect();
    assert_eq!(labels.len(), 2, "hand vs this Digimon's digivolution cards: {labels:?}");
    // Choose the digivolution cards: the hand card stays in hand.
    r.execute_branch(1);
    pick_card(&mut r);
    r.auto_resolve().ok();
    assert_eq!(linked_count(&r, ouja), 1);
    assert_eq!(hand_ids(&r, 0), vec!["LINK-LV4".to_string()]);
}

#[test]
fn bt23_022_level5_card_is_not_offered() {
    let (mut r, ouja) = oujamon_on_field(&["LINK-LV5"]);
    fire(&mut r, EffectTiming::WhenDigivolving, ouja);
    if r.pending_kind() == Some(SelectionKind::Replacement) {
        r.accept_optional_trigger().unwrap();
    }
    assert!(
        r.pending_selection().is_none(),
        "no level 4 or lower candidate → nothing to pick"
    );
    assert_eq!(linked_count(&r, ouja), 0);
}

/// Q5243: a card without <Link> can't be linked by this effect.
#[test]
fn bt23_022_card_without_link_is_not_offered() {
    let (mut r, ouja) = oujamon_on_field(&["NOLINK-LV4"]);
    fire(&mut r, EffectTiming::WhenDigivolving, ouja);
    if r.pending_kind() == Some(SelectionKind::Replacement) {
        r.accept_optional_trigger().unwrap();
    }
    assert!(r.pending_selection().is_none(), "Q5243: no <Link>, not linkable");
    assert_eq!(linked_count(&r, ouja), 0);
}

#[test]
fn bt23_022_link_pick_is_declinable() {
    let (mut r, ouja) = oujamon_on_field(&["LINK-LV4"]);
    fire(&mut r, EffectTiming::WhenDigivolving, ouja);
    accept_outer(&mut r);
    assert!(r.pending_is_optional(), "'You may link' — PASS legal at the pick");
    r.execute_action(0, PASS).unwrap();
    r.auto_resolve().ok();
    assert_eq!(linked_count(&r, ouja), 0);
    assert_eq!(hand_ids(&r, 0), vec!["LINK-LV4".to_string()]);
}

#[test]
fn bt23_022_wd_wa_share_once_per_turn() {
    let (mut r, ouja) = oujamon_on_field(&["LINK-LV4", "LINK-LV4"]);
    fire(&mut r, EffectTiming::WhenDigivolving, ouja);
    accept_outer(&mut r);
    pick_card(&mut r);
    r.auto_resolve().ok();
    assert_eq!(linked_count(&r, ouja), 1);

    fire(&mut r, EffectTiming::WhenAttacking, ouja);
    r.auto_resolve().ok();
    assert_eq!(linked_count(&r, ouja), 1, "[WA] locked out by the shared [Once Per Turn]");

    // Lockout clears on the controller's next turn.
    r.end_turn();
    r.end_turn();
    r.game.enter_main_phase();
    fire(&mut r, EffectTiming::WhenAttacking, ouja);
    accept_outer(&mut r);
    pick_card(&mut r);
    r.auto_resolve().ok();
    // A Digimon holds at most 1 link card (§4-9-5): the new link replaces the
    // old one, which is trashed.
    assert_eq!(linked_count(&r, ouja), 1);
    assert!(
        !hand_ids(&r, 0).contains(&"LINK-LV4".to_string()),
        "the clause fired again next turn and linked the second card"
    );
    assert_eq!(r.trash_size(0), 1, "the replaced link card is trashed (§4-9-5)");
}

/// Declining the optional effect at its confirm does not use it (DCGO
/// optional ActivateClass): [When Attacking] can still link the same turn.
#[test]
fn bt23_022_declining_wd_confirm_keeps_wa_available() {
    let (mut r, ouja) = oujamon_on_field(&["LINK-LV4"]);
    fire(&mut r, EffectTiming::WhenDigivolving, ouja);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
    r.execute_action(0, PASS).unwrap();
    r.auto_resolve().ok();
    assert_eq!(linked_count(&r, ouja), 0);

    fire(&mut r, EffectTiming::WhenAttacking, ouja);
    accept_outer(&mut r);
    pick_card(&mut r);
    r.auto_resolve().ok();
    assert_eq!(linked_count(&r, ouja), 1);
}

// ─── Section 5 — [All Turns][OPT] when this Digimon gets linked, may unsuspend

#[test]
fn bt23_022_effect_link_while_suspended_may_unsuspend() {
    let (mut r, ouja) = oujamon_on_field(&["LINK-LV4"]);
    r.game.suspend(ouja);
    fire(&mut r, EffectTiming::WhenAttacking, ouja);
    accept_outer(&mut r);
    pick_card(&mut r);
    r.game.drain_effect_queue();
    // The link triggers "when this Digimon gets linked" → optional confirm.
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement), "'it may unsuspend'");
    r.accept_optional_trigger().unwrap();
    r.auto_resolve().ok();
    assert!(!is_suspended(&r, ouja), "Oujamon unsuspended after getting linked");
}

#[test]
fn bt23_022_unsuspend_is_optional() {
    let (mut r, ouja) = oujamon_on_field(&["LINK-LV4"]);
    r.game.suspend(ouja);
    fire(&mut r, EffectTiming::WhenAttacking, ouja);
    accept_outer(&mut r);
    pick_card(&mut r);
    r.game.drain_effect_queue();
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
    r.execute_action(0, PASS).unwrap();
    r.auto_resolve().ok();
    assert!(is_suspended(&r, ouja), "declined: stays suspended");
}

#[test]
fn bt23_022_declared_link_onto_oujamon_unsuspends_it() {
    // A link from any source counts: Perorimon declared-linked from the field.
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let ouja = r.place_on_field(0, CARD_ID, Some(0));
    let pero = r.place_on_field(0, PERORIMON, Some(0));
    r.game.enter_main_phase();
    r.game.suspend(ouja);
    let link_action = digimon_engine::action::space::FIELD_EFFECT_START
        + pero.index as u16 * digimon_engine::action::space::EFFECTS_PER_PERMANENT
        + digimon_engine::action::space::FIELD_EFFECT_SLOT_FOR_LINK;
    r.game.decode_action(link_action, 0);
    let host_pick = r.pending_selection().unwrap().valid_action_ids[0];
    r.execute_action(0, host_pick).unwrap();
    r.game.drain_effect_queue();
    // Resolve every pending decision accepting each optional effect: Oujamon's
    // unsuspend confirm (Perorimon's When Linking has no opponent target).
    let mut guard = 0;
    while r.pending_selection().is_some() && guard < 10 {
        let sel = r.pending_selection().unwrap();
        let act = sel.valid_action_ids.first().copied().unwrap_or(PASS);
        r.execute_action(0, act).unwrap();
        guard += 1;
    }
    let ouja = r.perm_handle(0, 0);
    assert_eq!(linked_ids(&r, ouja), vec![PERORIMON.to_string()]);
    assert!(!is_suspended(&r, ouja), "any link onto Oujamon triggers the unsuspend");
}

#[test]
fn bt23_022_unsuspend_once_per_turn() {
    let (mut r, ouja) = oujamon_on_field(&[]);
    r.game.suspend(ouja);
    // First link (fixture-driven OnLink through the engine's link entry).
    r.push_source(ouja, "LINK-LV4");
    fire(&mut r, EffectTiming::WhenAttacking, ouja);
    accept_outer(&mut r);
    pick_card(&mut r);
    r.game.drain_effect_queue();
    r.accept_optional_trigger().unwrap();
    r.auto_resolve().ok();
    assert!(!is_suspended(&r, ouja));

    // Suspend again and link a second card by declaring Perorimon's link.
    r.game.suspend(ouja);
    let pero = r.place_on_field(0, PERORIMON, Some(0));
    let link_action = digimon_engine::action::space::FIELD_EFFECT_START
        + pero.index as u16 * digimon_engine::action::space::EFFECTS_PER_PERMANENT
        + digimon_engine::action::space::FIELD_EFFECT_SLOT_FOR_LINK;
    r.game.decode_action(link_action, 0);
    let host_pick = r.pending_selection().unwrap().valid_action_ids[0];
    r.execute_action(0, host_pick).unwrap();
    r.game.drain_effect_queue();
    assert!(
        r.pending_kind() != Some(SelectionKind::Replacement),
        "[Once Per Turn]: no second unsuspend offer this turn"
    );
    r.auto_resolve().ok();
    let ouja = r.perm_handle(0, 0);
    assert_eq!(linked_ids(&r, ouja), vec![PERORIMON.to_string()], "Perorimon replaced the link card");
    assert!(is_suspended(&r, ouja), "second link this turn does not unsuspend");
}

#[test]
fn bt23_022_link_to_another_digimon_does_not_unsuspend_oujamon() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let ouja = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "APP-HOST", Some(0));
    let pero = r.place_on_field(0, PERORIMON, Some(0));
    r.game.enter_main_phase();
    r.game.suspend(ouja);
    let link_action = digimon_engine::action::space::FIELD_EFFECT_START
        + pero.index as u16 * digimon_engine::action::space::EFFECTS_PER_PERMANENT
        + digimon_engine::action::space::FIELD_EFFECT_SLOT_FOR_LINK;
    r.game.decode_action(link_action, 0);
    // Choose APP-HOST (not Oujamon) as the host.
    let view = r.pending_selection_view().unwrap();
    assert_eq!(view.valid_action_ids.len(), 2, "Oujamon and APP-HOST are [Appmon] hosts");
    r.execute_action(0, view.valid_action_ids[1]).unwrap();
    r.game.drain_effect_queue();
    assert!(
        r.pending_kind() != Some(SelectionKind::Replacement),
        "no unsuspend offer when a different Digimon gets linked"
    );
    r.auto_resolve().ok();
    assert!(is_suspended(&r, ouja));
    assert_eq!(linked_count(&r, ouja), 0);
}

/// The archetype line: Oujamon attacks, links Perorimon from hand, and the link
/// both unsuspends Oujamon and fires Perorimon's [When Linking] suspend.
#[test]
fn bt23_022_attack_links_perorimon_unsuspends_and_suspends_opponent() {
    let mut r = base()
        .hand(0, &[PERORIMON])
        .deck(0, &["DECK-PAD"; 12])
        .security(1, &["DECK-PAD"; 4])
        .memory(3)
        .start();
    let ouja = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-DIGI", Some(0));
    r.game.enter_main_phase();
    r.attack_player(ouja, 1, false);
    // Two [When Attacking] triggers (the link effect and <Raid>): resolve the
    // link effect first.
    assert_eq!(r.pending_kind(), Some(SelectionKind::TriggerOrder));
    let view = r.pending_selection_view().unwrap();
    let link_first = view
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .find(|c| c.keyword.is_none())
        .expect("the link effect is one of the pending triggers")
        .action_id;
    r.execute_action(0, link_first).unwrap();
    // [When Attacking]: accept, pick Perorimon from hand.
    accept_outer(&mut r);
    pick_card(&mut r);
    r.game.drain_effect_queue();
    // Resolve the two link triggers: accept the unsuspend and suspend the
    // opponent's Digimon with Perorimon's [When Linking].
    let mut guard = 0;
    while r.pending_selection().is_some() && guard < 10 {
        let sel = r.pending_selection().unwrap();
        let act = sel.valid_action_ids.first().copied().unwrap_or(PASS);
        r.execute_action(sel.selecting_player, act).unwrap();
        guard += 1;
    }
    let ouja = r.perm_handle(0, 0);
    assert_eq!(linked_ids(&r, ouja), vec![PERORIMON.to_string()]);
    assert!(!is_suspended(&r, ouja), "Oujamon unsuspended by its [All Turns] effect");
    assert!(is_suspended(&r, opp), "Perorimon's [When Linking] suspended the opponent");
    assert_eq!(r.game.memory, 3, "the link was free");
}
