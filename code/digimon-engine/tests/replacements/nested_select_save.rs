//! Phase C end-to-end test card — Save-like single-pick replacement.
//!
//! A Digimon with `<Save>` on `WhenWouldBeDeleted` may pick one of the
//! controller's own Tamers; if they do, the Digimon "slides under" the
//! Tamer (Phase D primitive — for this Phase C test we substitute an
//! inline source-push) and the deletion is cancelled. Three cases:
//!   1. Accept + pick → carrier survives, Tamer gains a stack source.
//!   2. Decline outer accept → carrier dies, no stack mutation.
//!   3. No Tamers on field → no Tamer candidate; outcome stays None;
//!      original deletion proceeds.

use std::sync::Arc;

use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementSubject;

fn save_card(id: &str) -> CardData {
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
        effect_class_name: "SAVE_LIKE".to_string(),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    }
}

fn tamer(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Tamer,
        level: None,
        dp: None,
        play_cost: 3,
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
    }
}

/// SAVE_LIKE effect — selects a Tamer, pushes self's top card under it as a
/// new bottom source (Phase D primitive substitute), and cancels deletion.
/// Each test installs this fresh on its own DebugRunner via `r.register_effect`.
struct SaveLike;
impl CardEffect for SaveLike {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("<Save>")
            .optional()
            .replacement_process(|rctx| {
                let me = match rctx.subject {
                    ReplacementSubject::Permanent(h) => h,
                    _ => return,
                };
                rctx.effect.select_own_permanent(
                    "pick a Tamer to slide under",
                    false,
                    |g, h| {
                        let p = &g.players[h.player as usize];
                        if let Some(perm) = p.battle_area.get(h.index as usize) {
                            perm.is_tamer(&g.card_data)
                        } else {
                            false
                        }
                    },
                    move |ctx, tamer| {
                        // Phase D primitive substitute: manually push self's
                        // top card under the tamer as a new bottom source.
                        // For a real Save card this would be
                        // ctx.move_self_under(tamer).
                        let me_player = me.player;
                        let me_idx = me.index as usize;
                        let tamer_player = tamer.player;
                        let tamer_idx = tamer.index as usize;
                        if let Some(top) = ctx.game.players[me_player as usize].battle_area[me_idx]
                            .card_sources
                            .last()
                            .cloned()
                        {
                            ctx.game.players[tamer_player as usize].battle_area[tamer_idx]
                                .card_sources
                                .insert(0, top);
                        }
                        ctx.cancel_leave();
                    },
                );
            })
            .build()]
    }
}

#[test]
fn save_picks_tamer_and_cancels_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(save_card("SAVE-D"))
        .add_card(tamer("TAMER"))
        .start();
    r.register_effect("SAVE-D", Arc::new(SaveLike));
    let saved = r.place_on_field(0, "SAVE-D", None);
    let _t = r.place_on_field(0, "TAMER", None);

    let _stack_before = r.game.players[0].battle_area[saved.index as usize]
        .card_sources
        .len();

    r.game.delete_permanent_with_effects(saved);

    // Outer accept dialog is up.
    assert!(r.game.pending_selection.is_some());
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept");

    // Inner Tamer pick is up; pick the only Tamer.
    let pending = r.game.pending_selection.as_ref().expect("inner select");
    let action = pending.valid_action_ids[0];
    let player = pending.selecting_player;
    r.game.resolve_selection(player, action).expect("pick");

    // Saved digimon survived (cancel_leave fired).
    assert_eq!(
        r.game.players[0].battle_area.len(),
        2,
        "Saved digimon survived because cancel_leave was called"
    );
    // Tamer gained a stack source from our move-under primitive.
    let tamer_perm = &r.game.players[0].battle_area[1];
    assert!(
        tamer_perm.card_sources.len() > 1,
        "Tamer should have gained a stack source from move-self-under"
    );
}

#[test]
fn save_outer_decline_proceeds_with_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(save_card("SAVE-D"))
        .add_card(tamer("TAMER"))
        .start();
    r.register_effect("SAVE-D", Arc::new(SaveLike));
    let saved = r.place_on_field(0, "SAVE-D", None);
    let _t = r.place_on_field(0, "TAMER", None);

    r.game.delete_permanent_with_effects(saved);

    assert!(r.game.pending_selection.is_some());
    use digimon_engine::action::space::PASS;
    r.game.resolve_selection(0, PASS).expect("decline");

    // Decline → original deletion proceeds → Saved digimon is gone.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Decline → original deletion → only the Tamer remains"
    );
    let _ = saved;
}

#[test]
fn save_with_no_tamers_does_not_offer() {
    // No Tamer on field → the candidate filter for the inner select is empty,
    // BUT the outer optional-accept still fires (because Phase C does not
    // pre-filter candidates on inner-filter emptiness — that's a Phase D
    // auto-install authoring concern). On accept, the inner select_own_permanent
    // sees zero candidates and silently no-ops; the user's callback never
    // runs; outcome stays None; original deletion proceeds.
    let mut r = DebugRunner::builder().add_card(save_card("SAVE-D")).start();
    r.register_effect("SAVE-D", Arc::new(SaveLike));
    let saved = r.place_on_field(0, "SAVE-D", None);

    r.game.delete_permanent_with_effects(saved);
    assert!(
        r.game.pending_selection.is_some(),
        "outer accept still installed"
    );

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept");

    // Inner select_own_permanent had no Tamer candidates → no PendingSelection
    // installed; user callback never ran; outcome stayed None.
    // Either parked_replacement is None (process closure returned without
    // installing pending_selection) OR the post-callback drain already fired
    // with outcome=None (which would commit the original delete).
    // Either way, the saved digimon should be gone.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "Empty Tamer filter → outcome=None → original deletion proceeds"
    );
    let _ = saved;
}
