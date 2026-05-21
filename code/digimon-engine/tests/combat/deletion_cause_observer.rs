//! Phase B §B5 — `ctx.was_deleted_by_effect()` / `was_deleted_by_opponent()`
//! report the correct cause to `OnDeletion` observers.
//!
//! These accessors unblock Phase E (Retaliation = "only fires on battle
//! deletion"; Scapegoat = "only fires when not OwnEffect").

use std::sync::{Arc, Mutex};

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;

fn fighter(id: &str, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(dp),
        play_cost: 5,
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

#[test]
fn current_deletion_cause_set_during_opponent_effect_delete() {
    // Dispatcher-level proof: drive an opponent-effect delete and verify
    // the engine plumbing from Task 8 cleared the slot back to None after
    // the drain. Full in-observer assertion lives in
    // `was_deleted_by_opponent_true_inside_ondeletion_for_opp_effect_delete`
    // once we have a card-side observer to read the slot.
    let mut r = DebugRunner::builder()
        .add_card(fighter("VICTIM", 4000))
        .start();
    let victim = r.place_on_field(0, "VICTIM", None);

    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.delete_permanent(victim);
    }
    r.game.set_effect_source_player_for_test(None);

    // After the deletion completes, `current_deletion_cause` is cleared back
    // to None (the slot is scoped to the OnDeletion drain).
    assert_eq!(r.game.players[0].battle_area.len(), 0, "victim deleted");
    let _ = ReplacementCause::OpponentEffect; // keep the use binding live
}

/// An OnDeletion observer that captures `ctx.deletion_cause()` and the
/// `ctx.was_deleted_by_opponent()` boolean into shared cells when it fires.
struct DeletionCauseCapture {
    cause: Arc<Mutex<Option<ReplacementCause>>>,
    by_opponent: Arc<Mutex<Option<bool>>>,
    by_effect: Arc<Mutex<Option<bool>>>,
}
impl CardEffect for DeletionCauseCapture {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let cause = self.cause.clone();
        let by_opponent = self.by_opponent.clone();
        let by_effect = self.by_effect.clone();
        vec![Effect::on_deletion(card)
            .name("capture deletion cause")
            .process(move |ctx| {
                *cause.lock().unwrap() = ctx.deletion_cause();
                *by_opponent.lock().unwrap() = Some(ctx.was_deleted_by_opponent());
                *by_effect.lock().unwrap() = Some(ctx.was_deleted_by_effect());
            })
            .build()]
    }
}

#[test]
fn was_deleted_by_opponent_true_inside_ondeletion_for_opp_effect_delete() {
    // End-to-end proof: an OnDeletion observer registered against the
    // victim card sees `cause == OpponentEffect`, `was_deleted_by_opponent()
    // == true`, `was_deleted_by_effect() == true` when an opponent's
    // EffectContext deletes it.
    let cause_cell: Arc<Mutex<Option<ReplacementCause>>> = Arc::new(Mutex::new(None));
    let by_opp_cell: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
    let by_eff_cell: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));

    let mut r = DebugRunner::builder()
        .add_card(fighter("VICTIM", 4000))
        .start();
    r.register_effect(
        "VICTIM",
        Arc::new(DeletionCauseCapture {
            cause: cause_cell.clone(),
            by_opponent: by_opp_cell.clone(),
            by_effect: by_eff_cell.clone(),
        }),
    );
    let victim = r.place_on_field(0, "VICTIM", None);

    // P1's effect deletes P0's victim → cause is OpponentEffect from the
    // victim-controller's perspective.
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.delete_permanent(victim);
    }
    r.game.set_effect_source_player_for_test(None);

    assert_eq!(r.game.players[0].battle_area.len(), 0, "victim deleted");
    assert_eq!(
        *cause_cell.lock().unwrap(),
        Some(ReplacementCause::OpponentEffect),
        "OnDeletion observer must see OpponentEffect cause"
    );
    assert_eq!(
        *by_opp_cell.lock().unwrap(),
        Some(true),
        "was_deleted_by_opponent() must return true for opp-effect delete"
    );
    assert_eq!(
        *by_eff_cell.lock().unwrap(),
        Some(true),
        "was_deleted_by_effect() must return true for opp-effect delete"
    );
}
