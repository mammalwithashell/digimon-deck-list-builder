//! BT21-075 SkullGreymon — Digimon, Lv.5, Purple/Red, DP 8000, Cost 8.
//! Traits: Undead / ADVENTURE. Attribute: Virus.
//!
//! Printed (official observation / data/card_bundles/BT21-075.md):
//!   Standard digivolve: Purple Lv.4 / 4, Red Lv.4 / 4  (cards.json drops Red)
//!   [Digivolve] Lv.4 w/[Greymon] in name or w/[ADVENTURE] trait: Cost 3
//!   [On Play] [When Digivolving] 1 of your Digimon gains <Raid> and
//!     <Retaliation> until your opponent's turn ends.
//!   [On Deletion] You may play 1 [ADVENTURE] trait card with a play cost of 4
//!     or less from your trash without paying the cost.
//!   Inherited: [On Deletion] (same text).
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/BT21/Purple/BT21_075.cs — OP/WD pick
//! canNoSelect false over own battle-area Digimon; GainRaid + GainRetaliation
//! UntilOpponentTurnEnd. On Deletion (face-up + inherited) isOptional, select
//! from trash canNoSelect true; filter EqualsTraits(ADVENTURE) && HasPlayCost
//! && cost <= 4.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledPredicate,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::TriggerSource;

const CARD_ID: &str = "BT21-075";

fn digimon(id: &str, cost: u16, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = cost;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn tamer(id: &str, cost: u16, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-075 YAML must be present in the embedded DSL pack")
        .add_card(digimon("ALLY", 4, &[]))
        .add_card(digimon("HOST", 6, &[]))
        .add_card(digimon("ADV-4", 4, &["ADVENTURE"]))
        .add_card(digimon("ADV-5", 5, &["ADVENTURE"]))
        .add_card(tamer("ADV-TAMER", 3, &["ADVENTURE"]))
        .add_card(digimon("NON-ADV-3", 3, &["Beast"]))
        .add_card(make_test_card("PAD", "PAD"))
        .deck(0, &["PAD"; 5])
        .deck(1, &["PAD"; 5])
        .memory(10)
}

fn push_trash(r: &mut DebugRunner, id: &str) {
    let idx = r.game.card_data.iter().position(|c| c.card_id == id).unwrap();
    let iid = r.game.next_card_index();
    r.game.players[0].trash.push(CardSource::new(idx, 0, iid));
}

fn on_field(r: &DebugRunner, id: &str) -> bool {
    r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == id)
}

fn flat(p: &CompiledPredicate) -> Vec<&CompiledPredicate> {
    let mut v = vec![p];
    for q in p.all_of.iter().chain(p.any_of.iter()) {
        v.extend(flat(q));
    }
    v
}

/// Accept an outer optional-trigger prompt if one is pending.
fn accept_outer(r: &mut DebugRunner) {
    if r.pending_kind() == Some(digimon_engine::selection::SelectionKind::Replacement) {
        r.accept_optional_trigger().expect("accept outer optional prompt");
    }
}

// ─── Structural ──────────────────────────────────────────────────────────────

#[test]
fn bt21_075_alt_paths_include_red_circle_and_greymon_or_adventure() {
    let r = base().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(8000));
    assert!(card.color.contains(&CompiledColor::Purple) && card.color.contains(&CompiledColor::Red));
    assert!(card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && matches!(p.cost, Some(CompiledCost::Literal(4)))
            && p.from.as_ref().is_some_and(|f| {
                let ps = flat(f);
                ps.iter().any(|q| q.level_eq == Some(4))
                    && ps.iter().any(|q| q.color_is == Some(CompiledColor::Red))
            })
    }), "printed red Lv.4 / 4 circle (dropped by cards.json) must be authored");
    assert!(card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && matches!(p.cost, Some(CompiledCost::Literal(3)))
            && p.from.as_ref().is_some_and(|f| {
                let ps = flat(f);
                ps.iter().any(|q| q.level_eq == Some(4))
                    && ps.iter().any(|q| q.name_contains.as_deref() == Some("Greymon"))
                    && ps.iter().any(|q| q.trait_has.as_deref() == Some("ADVENTURE"))
            })
    }));
}

#[test]
fn bt21_075_declares_face_up_and_inherited_on_deletion() {
    let r = base().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let t: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(t.iter().any(|t| t.when == vec![CompiledTiming::OnDeletion]
        && t.scope == CompiledScope::FaceUp));
    assert!(t.iter().any(|t| t.when == vec![CompiledTiming::OnDeletion]
        && t.scope == CompiledScope::Inherited));
}

// ─── [On Play][When Digivolving] Raid + Retaliation ──────────────────────────

#[test]
fn bt21_075_on_play_grants_raid_and_retaliation_to_chosen_own_digimon() {
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let opp = r.place_on_field(1, "ALLY", Some(0));

    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    let view = r.pending_selection_view().expect("own-Digimon pick");
    assert!(!view.valid_action_ids.contains(&PASS), "mandatory pick");
    assert_eq!(view.valid_action_ids.len(), 2, "both own Digimon, no opponent's");
    let action = digimon_engine::action::space::ATTACK_START + ally.index as u16;
    assert!(view.valid_action_ids.contains(&action));
    r.execute_action(0, action).expect("pick ally");
    let _ = r.auto_resolve();

    assert!(r.game.has_keyword(ally, Keyword::Raid));
    assert!(r.game.has_keyword(ally, Keyword::Retaliation));
    assert!(!r.game.has_keyword(me, Keyword::Raid));
    assert!(!r.game.has_keyword(opp, Keyword::Raid));
}

#[test]
fn bt21_075_keywords_last_through_opponents_turn_then_expire() {
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    assert!(r.game.has_keyword(me, Keyword::Retaliation));

    r.end_turn();
    let _ = r.auto_resolve_declining_end_of_turn_attacks();
    assert!(
        r.game.has_keyword(me, Keyword::Raid) && r.game.has_keyword(me, Keyword::Retaliation),
        "still active during the opponent's turn"
    );
    r.end_turn();
    let _ = r.auto_resolve_declining_end_of_turn_attacks();
    assert!(
        !r.game.has_keyword(me, Keyword::Raid) && !r.game.has_keyword(me, Keyword::Retaliation),
        "expired at the end of the opponent's turn"
    );
}

// ─── [On Deletion] play [ADVENTURE] cost <= 4 from trash ─────────────────────

#[test]
fn bt21_075_on_deletion_offers_only_adventure_cost_4_or_less_digimon_and_tamers() {
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, "ADV-4");
    push_trash(&mut r, "ADV-5");
    push_trash(&mut r, "ADV-TAMER");
    push_trash(&mut r, "NON-ADV-3");

    r.game
        .delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    accept_outer(&mut r);
    let view = r.pending_selection_view().expect("trash pick installs");
    let picks: Vec<_> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|a| *a != PASS)
        .collect();
    assert_eq!(picks.len(), 2, "ADV-4 Digimon and ADV-TAMER only");
    assert!(
        r.pending_is_optional() || view.valid_action_ids.contains(&PASS),
        "'You may play' must be declinable"
    );
}

#[test]
fn bt21_075_on_deletion_plays_chosen_card_for_free() {
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, "ADV-4");
    let mem = r.memory();
    r.game
        .delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    accept_outer(&mut r);
    r.auto_resolve().expect("resolve");
    assert!(on_field(&r, "ADV-4"), "ADV-4 played from trash");
    assert_eq!(r.memory(), mem, "without paying the cost");
}

#[test]
fn bt21_075_on_deletion_decline_plays_nothing() {
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, "ADV-4");
    r.game
        .delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    while let Some(sel) = r.pending_selection() {
        let p = sel.selecting_player;
        r.execute_action(p, PASS).expect("PASS legal");
    }
    assert!(!on_field(&r, "ADV-4"));
}

#[test]
fn bt21_075_inherited_on_deletion_fires_from_under_host() {
    let mut r = base().start();
    let host = r.place_stack(0, &[CARD_ID, "HOST"]);
    push_trash(&mut r, "ADV-TAMER");
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    accept_outer(&mut r);
    r.auto_resolve().expect("resolve");
    assert!(on_field(&r, "ADV-TAMER"), "inherited On Deletion plays the Tamer");
}
