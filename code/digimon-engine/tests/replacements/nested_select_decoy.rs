//! Phase C end-to-end test card — Decoy-like substitute replacement.
//!
//! When an ally would be deleted, the decoy carrier may redirect deletion
//! to itself. Tests `EffectContext::substitute_replacement` end-to-end.
//!
//! This synthetic fixture tests deferred `Substituted` commits, not the native
//! Decoy keyword. Native Decoy pays deletion and then prevents the original
//! deletion (rule 16-18); substitution is a distinct engine primitive.
//! A single-candidate inner selection parks this synthetic replacement before
//! committing substitution. Candidate eligibility excludes its own carrier,
//! including when the substituted deletion starts a fresh replacement window.

use std::sync::Arc;

use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementSubject;

fn decoy_card(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        link_text: String::new(),
        requirement_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: "DECOY_LIKE".to_string(),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

fn ally_card(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        link_text: String::new(),
        requirement_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

/// DecoyLike effect — when an ally would be deleted, the decoy may redirect
/// deletion to itself via substitute_replacement. Each test installs this
/// fresh on its own DebugRunner via `r.register_effect`.
struct DecoyLike;
impl CardEffect for DecoyLike {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("<Decoy>")
            .optional()
            .replacement_condition(|ctx, subject| {
                matches!((ctx.source_permanent, subject),
                    (Some(me), ReplacementSubject::Permanent(ally))
                        if me != *ally && me.player == ally.player)
            })
            .replacement_process(|rctx| {
                // Only fire when the subject is a DIFFERENT permanent
                // (the decoy redirects deletion of its allies, not
                // self-deletion).
                let ally = match rctx.subject {
                    ReplacementSubject::Permanent(h) => h,
                    _ => return,
                };
                let me = match rctx.effect.source_permanent {
                    Some(h) if h != ally => h,
                    _ => return, // self-deletion: skip
                };
                let _ = ally;
                // Install a "confirm" prompt (single own-permanent select with
                // one valid candidate — the decoy itself) so the parked-
                // replacement path is exercised. On accept, the callback
                // calls substitute_replacement(decoy_self).
                rctx.effect.select_own_permanent(
                    "confirm decoy",
                    false,
                    move |_g, h| h == me,
                    move |ctx, _picked| {
                        ctx.substitute_replacement(ReplacementSubject::Permanent(me));
                    },
                );
            })
            .build()]
    }
}

#[test]
fn decoy_substitutes_self_for_ally_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(decoy_card("DECOY"))
        .add_card(ally_card("ALLY"))
        .start();
    r.register_effect("DECOY", Arc::new(DecoyLike));
    let decoy = r.place_on_field(0, "DECOY", None);
    let decoy_card = r.game.players[0].battle_area[decoy.index as usize].top_card().clone();
    let ally = r.place_on_field(0, "ALLY", None);

    // Delete the ally — decoy's WhenWouldBeDeleted fires (subject = ally).
    r.game.delete_permanent_with_effects(ally);

    // Outer accept dialog up.
    assert!(r.game.pending_selection.is_some());
    assert_eq!(digimon_engine::action::build_action_mask(&r.game, 0)[REPLACEMENT_ACCEPT as usize], 1.0);
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("outer accept");

    // Inner confirm prompt — pick the only valid action.
    let pending = r.game.pending_selection.as_ref().expect("confirm");
    assert_eq!(pending.valid_action_ids.len(), 1, "only the synthetic carrier is selectable");
    let action = pending.valid_action_ids[0];
    let player = pending.selecting_player;
    assert_eq!(player, 0);
    assert_eq!(digimon_engine::action::build_action_mask(&r.game, player)[action as usize], 1.0);
    r.game
        .resolve_selection(player, action)
        .expect("confirm pick");

    assert!(r.game.pending_selection.is_none(), "substituted deletion has fully settled");
    assert_eq!(r.game.players[0].trash.len(), 1);
    assert_eq!(r.game.players[0].trash[0].card_id(&r.game.card_data), "DECOY");
    assert_eq!(r.game.players[0].trash[0].card_index, decoy_card.card_index);

    // Substitute outcome: ally survives, decoy is deleted in its place.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "exactly one permanent left after substitute"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "ALLY",
        "the ally is the survivor; decoy was deleted in its place"
    );
}

#[test]
fn synthetic_substitute_does_not_offer_for_its_own_carrier() {
    let mut r = DebugRunner::builder()
        .add_card(decoy_card("DECOY"))
        .add_card(ally_card("ALLY"))
        .start();
    r.register_effect("DECOY", Arc::new(DecoyLike));
    let decoy = r.place_on_field(0, "DECOY", None);
    r.place_on_field(0, "ALLY", None);
    r.game.delete_permanent_with_effects(decoy);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data), "ALLY");
    assert_eq!(r.game.players[0].trash.len(), 1);
    assert_eq!(r.game.players[0].trash[0].card_id(&r.game.card_data), "DECOY");
}
