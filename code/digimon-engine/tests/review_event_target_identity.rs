#![cfg(feature = "dsl-yaml-loader")]
//! Independent queue/attachment regression review. Synthetic setup is not card or action approval.
use digimon_dsl::compiled::CompiledBindingRef;
use digimon_engine::{
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    dsl_cards::{
        binding_ref::{resolve_binding_ref, ResolvedBinding},
        bindings::Bindings,
    },
    effect_context::EffectContext,
    enums::{CardKind, EffectTiming, GamePhase, Keyword, LinkCardSource},
    permanent::PermanentHandle,
    selection::TriggerSource,
    trigger_context::{DeletedObjectSnapshot, EffectAttribution, EventCause, TriggerContext},
};
fn runner(timing: &str) -> DebugRunner {
    let yaml=format!("card: REV-OBSERVER\nname: Review observer\nkind: digimon\ncolor: [red]\nlevel: 3\ndp: 2000\neffects:\n  - when: {timing}\n    process:\n      - grant_keyword: {{target: event_target, keyword: Rush, expiry: end_of_turn}}\n");
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .unwrap()
        .add_card(make_test_card("REV-OLD", "Review old"))
        .add_card(make_test_card("REV-NEW", "Review new"))
        .add_card(make_test_card("REV-LINK", "Review link"))
        .add_card(make_test_card("REV-DISTRACTOR", "Review distractor"))
        .hand(0, &["REV-LINK"])
        .hand(1, &["REV-LINK"])
        .memory(10)
        .build();
    r.skip_mulligan();
    r.set_phase(GamePhase::Main);
    r
}
fn card(r: &DebugRunner, h: PermanentHandle) -> CardHandle {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .handle()
}
fn remove(r: &mut DebugRunner, h: PermanentHandle) {
    let p = r.game.players[h.player as usize]
        .battle_area
        .remove(h.index as usize);
    r.game.players[h.player as usize]
        .trash
        .extend(p.card_sources);
}
fn has(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.modifiers.has_keyword(h, Keyword::Rush)
}
#[test]
fn queued_native_entered_field_context_rejects_reused_slot_both_seats() {
    for seat in [0, 1] {
        for replace in [false, true] {
            let mut r = runner("on_any_digimon_played");
            let observer = r.place_on_field(1 - seat, "REV-OBSERVER", None);
            let old = r.place_on_field(seat, "REV-OLD", None);
            let old_card = card(&r, old);
            r.game.enqueue_triggered(
                EffectTiming::OnEnterFieldAnyone,
                TriggerSource::EnteredField {
                    player: seat,
                    permanent: old,
                    card: old_card,
                    effect_initiated: true,
                },
            );
            assert!(!r.game.effect_queue.is_empty());
            let target = if replace {
                remove(&mut r, old);
                r.place_on_field(seat, "REV-NEW", None)
            } else {
                old
            };
            r.game.drain_effect_queue();
            assert!(r.game.pending_selection.is_none());
            assert_eq!(has(&r, target), !replace);
            assert!(!has(&r, observer));
        }
    }
}
#[test]
fn queued_host_only_source_added_context_rejects_reused_slot() {
    for seat in [0, 1] {
        for replace in [false, true] {
            let mut r = runner("on_add_digivolution_cards");
            let observer = r.place_on_field(1 - seat, "REV-OBSERVER", None);
            let old = r.place_on_field(seat, "REV-OLD", None);
            let old_card = card(&r, old);
            r.game.enqueue_triggered(
                EffectTiming::OnAddDigivolutionCards,
                TriggerSource::SourcesAddedToStack {
                    host: old,
                    host_card: old_card,
                    cards: vec![old_card],
                    cause: EffectAttribution {
                        controller: seat,
                        source_card: None,
                        source_permanent: None,
                    },
                },
            );
            assert!(!r.game.effect_queue.is_empty());
            let target = if replace {
                remove(&mut r, old);
                r.place_on_field(seat, "REV-NEW", None)
            } else {
                old
            };
            r.game.drain_effect_queue();
            assert!(r.game.pending_selection.is_none());
            assert_eq!(has(&r, target), !replace);
            assert!(!has(&r, observer));
        }
    }
}
#[test]
fn actual_native_link_attachment_preserves_host_fallback() {
    for seat in [0, 1] {
        let mut r = runner("on_any_link");
        let observer = r.place_on_field(seat, "REV-OBSERVER", None);
        let host = r.place_on_field(seat, "REV-OLD", None);
        let distractor = r.place_on_field(1 - seat, "REV-DISTRACTOR", None);
        let linked = r.game.players[seat as usize].hand[0].handle();
        assert!(r
            .game
            .link_chosen_card_into_host(host, linked, LinkCardSource::Hand(seat)));
        assert!(r.game.pending_selection.is_none());
        assert_eq!(
            r.game.players[seat as usize].battle_area[host.index as usize]
                .linked_cards
                .len(),
            1
        );
        assert!(has(&r, host));
        assert!(!has(&r, observer));
        assert!(!has(&r, distractor));
    }
}
#[test]
fn queued_shifted_slot_never_targets_new_occupant_or_silently_relocates() {
    for seat in [0, 1] {
        let mut r = runner("on_any_digimon_played");
        r.place_on_field(1 - seat, "REV-OBSERVER", None);
        let preceding = r.place_on_field(seat, "REV-DISTRACTOR", None);
        let target = r.place_on_field(seat, "REV-OLD", None);
        r.place_on_field(seat, "REV-NEW", None);
        let target_card = card(&r, target);
        r.game.enqueue_triggered(
            EffectTiming::OnEnterFieldAnyone,
            TriggerSource::EnteredField {
                player: seat,
                permanent: target,
                card: target_card,
                effect_initiated: true,
            },
        );
        assert!(!r.game.effect_queue.is_empty());
        remove(&mut r, preceding);
        r.game.drain_effect_queue();
        assert!(!has(
            &r,
            PermanentHandle {
                player: seat,
                index: 0
            }
        ));
        assert!(!has(
            &r,
            PermanentHandle {
                player: seat,
                index: 1
            }
        ));
    }
}
#[test]
fn deleted_snapshot_precedence_and_absent_event_slot_are_unchanged() {
    let mut r = runner("on_any_digimon_played");
    let observer = r.place_on_field(0, "REV-OBSERVER", None);
    let live = r.place_on_field(1, "REV-OLD", None);
    let source = card(&r, observer);
    let deleted = r.game.players[1].hand[0].handle();
    r.game.current_trigger_context = Some(TriggerContext {
        event_permanent: Some(live),
        event_card: Some(card(&r, live)),
        deleted_object: Some(DeletedObjectSnapshot {
            former_controller: 1,
            top_card: deleted,
            card_kind: CardKind::Digimon,
            traits: vec![],
            level: Some(3),
            dp: Some(2000),
            cause: EventCause::EffectDeletion,
            dp_just_before: Some(2000),
            level_just_before: Some(3),
            cost_just_before: Some(3),
            names_just_before: vec!["Review deleted".into()],
            traits_just_before: vec![],
            source_count_just_before: 0,
            digisources_just_before: vec![],
            is_token: false,
        }),
        ..Default::default()
    });
    {
        let ctx = EffectContext::new(&mut r.game, source, Some(observer), 0);
        assert_eq!(
            resolve_binding_ref(&CompiledBindingRef::EventTarget, &ctx, &Bindings::new()),
            Some(ResolvedBinding::Card(deleted))
        );
    }
    r.game.current_trigger_context = Some(TriggerContext {
        event_permanent: Some(PermanentHandle {
            player: 1,
            index: 254,
        }),
        event_card: Some(deleted),
        ..Default::default()
    });
    let ctx = EffectContext::new(&mut r.game, source, Some(observer), 0);
    assert_eq!(
        resolve_binding_ref(&CompiledBindingRef::EventTarget, &ctx, &Bindings::new()),
        Some(ResolvedBinding::Card(deleted))
    );
}
