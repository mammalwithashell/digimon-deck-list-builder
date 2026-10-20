//! EventTarget must not apply an effect to a replacement occupying an old slot.
//! Internal event fixtures exercise identity resolution, not card-source approval.
#![cfg(feature = "dsl-yaml-loader")]

use digimon_dsl::compiled::{CompiledBindingRef, CompiledStep};
use digimon_engine::{
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    dsl_cards::{
        binding_ref::{resolve_binding_ref, ResolvedBinding},
        bindings::Bindings,
        step::{run_steps, RunOutcome},
    },
    effect_context::EffectContext,
    enums::Keyword,
    permanent::PermanentHandle,
    trigger_context::TriggerContext,
};

fn fixture(seat: u8) -> (DebugRunner, PermanentHandle, CardHandle, CardHandle) {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("IDENTITY-SOURCE", "Source"))
        .add_card(make_test_card("IDENTITY-OLD", "Old target"))
        .add_card(make_test_card("IDENTITY-NEW", "Replacement"))
        .build();
    let source = r.place_on_field(1 - seat, "IDENTITY-SOURCE", None);
    let source_card = r.game.players[source.player as usize].battle_area[0]
        .top_card()
        .handle();
    r.place_on_field(seat, "IDENTITY-OLD", None);
    let old = r.game.players[seat as usize].battle_area.remove(0);
    let old_card = old.top_card().handle();
    r.game.players[seat as usize].trash.extend(old.card_sources);
    let replacement = r.place_on_field(seat, "IDENTITY-NEW", None);
    (r, replacement, source_card, old_card)
}

fn resolve(r: &mut DebugRunner, source: CardHandle) -> Option<ResolvedBinding> {
    let owner = r
        .game
        .players
        .iter()
        .position(|p| {
            p.battle_area
                .iter()
                .any(|perm| perm.top_card().handle() == source)
        })
        .expect("source is live") as u8;
    let ctx = EffectContext::new(&mut r.game, source, None, owner);
    resolve_binding_ref(&CompiledBindingRef::EventTarget, &ctx, &Bindings::new())
}

#[test]
fn stale_event_card_cannot_grant_keyword_to_reused_slot() {
    for seat in [0, 1] {
        let (mut r, replacement, source, old) = fixture(seat);
        r.game.current_trigger_context = Some({
            let mut context = TriggerContext::default();
            context.event_permanent = Some(replacement);
            context.event_card = Some(old);
            context
        });
        let outcome = {
            let mut ctx = EffectContext::new(&mut r.game, source, None, 1 - seat);
            run_steps(
                &[CompiledStep::GrantKeyword {
                    target: CompiledBindingRef::EventTarget,
                    keyword: "Rush".into(),
                    expiry: "end_of_turn".into(),
                    value: None,
                }],
                &mut ctx,
                &mut Bindings::new(),
            )
        };
        assert!(matches!(outcome, RunOutcome::Synchronous));
        assert!(
            !r.game.modifiers.has_keyword(replacement, Keyword::Rush),
            "seat {seat}: a missing host identity must not defeat the stale event identity"
        );
        assert_eq!(resolve(&mut r, source), Some(ResolvedBinding::Card(old)));
    }
}

#[test]
fn stale_host_only_identity_cannot_resolve_to_reused_slot() {
    for seat in [0, 1] {
        let (mut r, replacement, source, old) = fixture(seat);
        r.game.current_trigger_context = Some({
            let mut context = TriggerContext::default();
            context.event_permanent = Some(replacement);
            context.event_host_card = Some(old);
            context
        });
        assert_eq!(resolve(&mut r, source), None);
    }
}

#[test]
fn known_primary_or_link_host_identity_still_resolves_live_permanent() {
    for seat in [0, 1] {
        let (mut r, replacement, source, old) = fixture(seat);
        let live = r.game.players[seat as usize].battle_area[0]
            .top_card()
            .handle();
        for (primary, host) in [
            (Some(live), None),
            (Some(live), Some(old)),
            (Some(old), Some(live)),
            (None, Some(live)),
        ] {
            r.game.current_trigger_context = Some({
                let mut context = TriggerContext::default();
                context.event_permanent = Some(replacement);
                context.event_card = primary;
                context.event_host_card = host;
                context
            });
            assert_eq!(
                resolve(&mut r, source),
                Some(ResolvedBinding::Permanent(replacement))
            );
        }
    }
}

#[test]
fn two_stale_identities_do_not_resolve_a_replacement() {
    for seat in [0, 1] {
        let (mut r, replacement, source, old) = fixture(seat);
        r.game.current_trigger_context = Some({
            let mut context = TriggerContext::default();
            context.event_permanent = Some(replacement);
            context.event_card = Some(old);
            context.event_host_card = Some(source);
            context
        });
        assert_eq!(resolve(&mut r, source), Some(ResolvedBinding::Card(old)));
    }
}

#[test]
fn unidentified_legacy_and_card_only_contexts_keep_existing_contract() {
    let (mut r, replacement, source, old) = fixture(0);
    r.game.current_trigger_context = Some({
        let mut context = TriggerContext::default();
        context.event_permanent = Some(replacement);
        context
    });
    assert_eq!(
        resolve(&mut r, source),
        Some(ResolvedBinding::Permanent(replacement))
    );
    r.game.current_trigger_context = Some({
        let mut context = TriggerContext::default();
        context.event_card = Some(old);
        context
    });
    assert_eq!(resolve(&mut r, source), Some(ResolvedBinding::Card(old)));
    r.game.current_trigger_context = None;
    assert_eq!(resolve(&mut r, source), None);
}
