//! ST21-01 Tsunomon — Digi-Egg, Lv.2, Purple. Traits: Lesser / ADVENTURE.
//!
//! Printed (official observation, card_official.json):
//!   Inherited Effect: [On Deletion] You may return 1 Digimon card with the
//!   [ADVENTURE] trait from your trash to the hand.
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/ST21/Purple/ST21_01.cs —
//!   OnDestroyedAnyone, SetIsInheritedEffect(true), isOptional true,
//!   SelectCardEffect Root.Trash Mode.AddHand, canNoSelect true,
//!   filter IsDigimon && HasAdventureTraits; CanActivate requires a match in trash.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

const CARD_ID: &str = "ST21-01";

fn digimon(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-01 YAML must be present in the embedded DSL pack")
        .add_card(digimon("HOST", &["Beast"]))
        .add_card(digimon("ADV-DIGI", &["Beast", "ADVENTURE"]))
        .add_card(digimon("NON-ADV", &["Beast"]))
        .add_card({
            let mut c = make_test_card("ADV-TAMER", "AdvTamer");
            c.card_kind = CardKind::Tamer;
            c.level = None;
            c.dp = None;
            c.traits = vec!["ADVENTURE".to_string()];
            c
        })
        .add_card(make_test_card("PAD", "PAD"))
        .deck(0, &["PAD"; 5])
        .deck(1, &["PAD"; 5])
        .memory(5)
        .start()
}

fn push_trash(r: &mut DebugRunner, id: &str) {
    let idx = r.game.card_data.iter().position(|c| c.card_id == id).unwrap();
    let iid = r.game.next_card_index();
    r.game.players[0].trash.push(CardSource::new(idx, 0, iid));
}

fn in_hand(r: &DebugRunner, id: &str) -> bool {
    r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == id)
}

#[test]
fn st21_01_is_purple_lv2_egg_with_one_inherited_optional_on_deletion() {
    let r = runner();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::DigiEgg);
    assert_eq!(card.level, Some(2));
    assert!(card.traits.iter().any(|t| t == "ADVENTURE"));
    let clauses: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(clauses.len(), 1);
    assert_eq!(clauses[0].scope, CompiledScope::Inherited);
    assert_eq!(clauses[0].when, vec![CompiledTiming::OnDeletion]);
}

#[test]
fn st21_01_inherited_on_deletion_offers_optional_adventure_pick_and_returns_it() {
    let mut r = runner();
    let host = r.place_stack(0, &[CARD_ID, "HOST"]);
    push_trash(&mut r, "ADV-DIGI");

    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    // "You may" — the pick must be declinable.
    let view = r.pending_selection_view().expect("trash pick installs");
    assert!(
        r.pending_is_optional() || view.valid_action_ids.contains(&PASS),
        "'You may return' must be declinable"
    );
    let non_pass: Vec<_> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|a| *a != PASS)
        .collect();
    assert_eq!(non_pass.len(), 1, "only the ADVENTURE Digimon is selectable");
    r.execute_action(view.selecting_player, non_pass[0])
        .expect("pick");
    r.auto_resolve().expect("finish");
    assert!(in_hand(&r, "ADV-DIGI"), "the ADVENTURE Digimon returns to hand");
}

#[test]
fn st21_01_decline_leaves_card_in_trash() {
    let mut r = runner();
    let host = r.place_stack(0, &[CARD_ID, "HOST"]);
    push_trash(&mut r, "ADV-DIGI");

    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    // Decline every optional prompt (outer trigger prompt and/or inner pick).
    while let Some(sel) = r.pending_selection() {
        let p = sel.selecting_player;
        r.execute_action(p, PASS).expect("PASS is legal for 'you may'");
    }
    assert!(!in_hand(&r, "ADV-DIGI"));
    assert!(r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "ADV-DIGI"));
}

#[test]
fn st21_01_non_adventure_and_tamer_cards_are_not_offered() {
    let mut r = runner();
    let host = r.place_stack(0, &[CARD_ID, "HOST"]);
    push_trash(&mut r, "NON-ADV");
    push_trash(&mut r, "ADV-TAMER");
    push_trash(&mut r, "ADV-DIGI");

    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    let view = r
        .pending_selection_view()
        .expect("the trash pick must install (ADV-DIGI is eligible)");
    let picks: Vec<_> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|a| *a != PASS)
        .collect();
    assert_eq!(
        picks.len(),
        1,
        "neither a non-ADVENTURE Digimon nor an ADVENTURE Tamer may be returned"
    );
}

#[test]
fn st21_01_face_up_egg_has_no_effect_when_it_is_the_top_card() {
    // The text is inherited-only: a lone Tsunomon permanent being deleted
    // (it is its own top card) must not offer the return.
    let mut r = runner();
    let egg = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, "ADV-DIGI");
    r.game
        .delete_permanent_with_cause(egg, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection().is_none());
    assert!(!in_hand(&r, "ADV-DIGI"));
}
