//! Phase C end-to-end test card — Decoy-like substitute replacement.
//!
//! When an ally would be deleted, the decoy carrier may redirect deletion
//! to itself. Tests `EffectContext::substitute_replacement` end-to-end.
//!
//! Implementation note: a real Decoy keyword's process would just call
//! `substitute_replacement` directly without any inner select (the
//! "confirm" is the OUTER optional-accept). To exercise Phase C's
//! parked-replacement substrate end-to-end (rather than the synchronous
//! `rctx.substitute(...)` path), this test installs a one-option "confirm
//! decoy" select_own_permanent that filters to just the decoy itself.
//! The callback runs `substitute_replacement(decoy_self)`. Behavior is
//! semantically equivalent — the test proves the parked-substitute flow
//! commits via commit_deferred_outcome's Substituted arm.

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
    let _decoy = r.place_on_field(0, "DECOY", None);
    let ally = r.place_on_field(0, "ALLY", None);

    // Delete the ally — decoy's WhenWouldBeDeleted fires (subject = ally).
    r.game.delete_permanent_with_effects(ally);

    // Outer accept dialog up.
    assert!(r.game.pending_selection.is_some());
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("outer accept");

    // Inner confirm prompt — pick the only valid action.
    let pending = r.game.pending_selection.as_ref().expect("confirm");
    let action = pending.valid_action_ids[0];
    let player = pending.selecting_player;
    r.game
        .resolve_selection(player, action)
        .expect("confirm pick");

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
