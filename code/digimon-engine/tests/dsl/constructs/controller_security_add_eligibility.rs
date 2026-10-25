//! Controller-relative security payment eligibility, including field gates.
use digimon_dsl::compiled::CompiledPredicate;
use digimon_engine::{
    debug_runner::{make_test_card, DebugRunner},
    dsl_cards::predicate::{eval_predicate, PredicateSubject},
    effect_context::EffectReadContext,
    enums::{Expiry, ModifierType},
    modifiers::{ModifierEntry, PlayerModifierEntry},
};

fn allowed(r: &DebugRunner, actor: u8) -> bool {
    let host = r.perm_handle(actor, 0);
    let card = r.game.player(actor).battle_area[0].top_card().handle();
    eval_predicate(&CompiledPredicate { controller_can_add_security: Some(true),
        ..Default::default() }, &EffectReadContext::new(&r.game, card, Some(host), actor), PredicateSubject::None)
}

fn runner(actor: u8) -> DebugRunner {
    let mut r = DebugRunner::builder().add_card(make_test_card("SEC-GATE", "Gate"))
        .deck(0, &["SEC-GATE"; 20]).deck(1, &["SEC-GATE"; 20]).start();
    r.skip_mulligan();
    r.place_stack(actor, &["SEC-GATE"]);
    r.game.players[actor as usize].battle_area[0].card_sources[0].owner = 1 - actor;
    r
}

#[test]
fn controller_security_permission_uses_actor_instead_of_physical_card_owner() {
    for actor in [0, 1] {
        let mut r = runner(actor);
        r.game.modifiers.add_player_modifier(1 - actor, PlayerModifierEntry::simple(
            ModifierType::CannotAddSecurityByEffect, 1, Expiry::Permanent, None, 1 - actor));
        assert!(allowed(&r, actor));
        r.game.modifiers.add_player_modifier(actor, PlayerModifierEntry::simple(
            ModifierType::CannotAddSecurityByEffect, 1, Expiry::Permanent, None, actor));
        assert!(!allowed(&r, actor));
    }
}

#[test]
fn controller_security_permission_includes_field_prohibitions_used_by_actual_payment() {
    for actor in [0, 1] {
        let mut r = runner(actor);
        assert!(allowed(&r, actor));
        let host = r.perm_handle(actor, 0);
        r.game.modifiers.add(host, ModifierEntry::simple(
            ModifierType::CannotAddSecurity, 0, Expiry::Permanent, actor));
        assert!(!allowed(&r, actor), "payability must agree with the security movement gate");
    }
}
