//! Official Language Standardization Rules page4: Indramon excludes Dramon.
use digimon_dsl::compiled::CompiledPredicate;
use digimon_engine::{
    debug_runner::{make_test_card, DebugRunner},
    dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject},
    effect_context::EffectReadContext,
    trigger_context::TriggerContext,
};

fn fixture(name: &str, alias: Option<&str>) -> DebugRunner {
    let mut c = make_test_card("NAME-SUBJECT", name);
    if let Some(alias) = alias {
        c.also_treated_as.push(alias.into());
    }
    let mut r = DebugRunner::builder().add_card(c).start();
    r.skip_mulligan();
    r.place_on_field(0, "NAME-SUBJECT", Some(0));
    r
}
#[test]
fn physical_card_and_static_alias_name_queries_apply_language_exception() {
    for (name, alias, want) in [
        ("Indramon", None, false),
        ("Wingdramon", None, true),
        ("Plain", Some("Indramon"), false),
        ("Indramon", Some("Wingdramon"), true),
    ] {
        let r = fixture(name, alias);
        let c = r.game.players[0].battle_area[0].top_card();
        assert_eq!(
            c.contains_card_name("Dramon", &r.game.card_data),
            want,
            "{name} {alias:?}"
        );
        assert!(
            c.contains_card_name(name, &r.game.card_data),
            "exact-name search remains possible"
        );
    }
}
#[test]
fn runtime_alias_language_exception_does_not_hide_other_legal_names() {
    let mut r = fixture("Plain", None);
    r.game.players[0].battle_area[0]
        .card_sources
        .last_mut()
        .unwrap()
        .also_treated_as
        .push("Indramon".into());
    assert!(!r.game.players[0].battle_area[0]
        .top_card()
        .contains_card_name("Dramon", &r.game.card_data));
    r.game.players[0].battle_area[0]
        .card_sources
        .last_mut()
        .unwrap()
        .also_treated_as
        .push("Exadramon".into());
    assert!(r.game.players[0].battle_area[0]
        .top_card()
        .contains_card_name("Dramon", &r.game.card_data));
}
#[test]
fn card_permanent_source_and_event_name_predicates_share_language_semantics() {
    for name in ["Indramon", "Wingdramon"] {
        let mut r = fixture(name, None);
        let h = digimon_engine::PermanentHandle {
            player: 0,
            index: 0,
        };
        let card = r.game.players[0].battle_area[0].top_card().handle();
        r.game.current_trigger_context = Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.event_card = Some(card); __fixture_trigger_context.event_permanent = Some(h); __fixture_trigger_context });
        let ctx = EffectReadContext::new(&r.game, card, Some(h), 0);
        let want = name == "Wingdramon";
        let pred = CompiledPredicate {
            name_contains: Some("Dramon".into()),
            ..Default::default()
        };
        for subject in [PredicateSubject::Card(card), PredicateSubject::Permanent(h)] {
            assert_eq!(
                eval_predicate_with_bindings(&pred, &ctx, subject, None),
                want,
                "{name}"
            );
        }
        for pred in [
            CompiledPredicate {
                source_name_contains: Some("Dramon".into()),
                ..Default::default()
            },
            CompiledPredicate {
                event_target_name_contains: Some("Dramon".into()),
                ..Default::default()
            },
            CompiledPredicate {
                event_card_name_contains: Some("Dramon".into()),
                ..Default::default()
            },
        ] {
            assert_eq!(
                eval_predicate_with_bindings(&pred, &ctx, PredicateSubject::None, None),
                want,
                "{name}"
            );
        }
    }
}

#[test]
fn event_and_source_name_queries_keep_static_runtime_and_effect_names() {
    use digimon_engine::{
        enums::{Expiry, ModifierType},
        modifiers::{ModifierEntry, ModifierPayload},
    };
    for mode in 0..3 {
        let mut r = fixture(
            "Indramon",
            if mode == 0 { Some("Wingdramon") } else { None },
        );
        let h = digimon_engine::PermanentHandle {
            player: 0,
            index: 0,
        };
        if mode == 1 {
            r.game.players[0].battle_area[0]
                .top_card_mut()
                .also_treated_as
                .push("Wingdramon".into());
        }
        if mode == 2 {
            r.game.modifiers.add(
                h,
                ModifierEntry::simple(ModifierType::ChangeBaseCardName, 0, Expiry::EndOfTurn, 0)
                    .with_payload(ModifierPayload::Name {
                        value: "Wingdramon".into(),
                        base: true,
                    }),
            );
        }
        let card = r.game.players[0].battle_area[0].top_card().handle();
        r.game.current_trigger_context = Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.event_card = Some(card); __fixture_trigger_context.event_permanent = Some(h); __fixture_trigger_context });
        let ctx = EffectReadContext::new(&r.game, card, Some(h), 0);
        for pred in [
            CompiledPredicate {
                source_name_contains: Some("Dramon".into()),
                ..Default::default()
            },
            CompiledPredicate {
                event_target_name_contains: Some("Dramon".into()),
                ..Default::default()
            },
            CompiledPredicate {
                event_card_name_contains: Some("Dramon".into()),
                ..Default::default()
            },
        ] {
            assert!(
                eval_predicate_with_bindings(&pred, &ctx, PredicateSubject::None, None),
                "alias mode {mode}: {pred:?}"
            );
        }
    }
}

#[test]
fn replaced_permanent_names_do_not_fall_back_to_printed_names() {
    use digimon_engine::{
        enums::{Expiry, ModifierType},
        modifiers::{ModifierEntry, ModifierPayload},
    };
    for replacement in ["Plain", "Indramon"] {
        let mut r = fixture("Wingdramon", None);
        let h = digimon_engine::PermanentHandle {
            player: 0,
            index: 0,
        };
        r.game.modifiers.add(
            h,
            ModifierEntry::simple(ModifierType::ChangeBaseCardName, 0, Expiry::EndOfTurn, 0)
                .with_payload(ModifierPayload::Name {
                    value: replacement.into(),
                    base: true,
                }),
        );
        let card = r.game.players[0].battle_area[0].top_card().handle();
        r.game.current_trigger_context = Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.event_card = Some(card); __fixture_trigger_context.event_permanent = Some(h); __fixture_trigger_context });
        let ctx = EffectReadContext::new(&r.game, card, Some(h), 0);
        for pred in [
            CompiledPredicate {
                name_contains: Some("Dramon".into()),
                ..Default::default()
            },
            CompiledPredicate {
                name_is: Some("Wingdramon".into()),
                ..Default::default()
            },
            CompiledPredicate {
                name_in: Some(vec!["Wingdramon".into()]),
                ..Default::default()
            },
        ] {
            assert!(
                !eval_predicate_with_bindings(&pred, &ctx, PredicateSubject::Permanent(h), None),
                "replacement {replacement}: {pred:?}"
            );
        }
        for pred in [
            CompiledPredicate {
                source_name_contains: Some("Dramon".into()),
                ..Default::default()
            },
            CompiledPredicate {
                event_target_name_contains: Some("Dramon".into()),
                ..Default::default()
            },
            CompiledPredicate {
                event_card_name_contains: Some("Dramon".into()),
                ..Default::default()
            },
        ] {
            assert!(
                !eval_predicate_with_bindings(&pred, &ctx, PredicateSubject::None, None),
                "replacement {replacement}: {pred:?}"
            );
        }
    }
}

#[test]
fn deleted_target_names_use_only_last_known_effective_identity() {
    use digimon_engine::{
        enums::CardKind,
        trigger_context::{DeletedObjectSnapshot, EventCause},
    };
    for name in ["Plain", "Indramon", "Groundramon"] {
        let mut r = fixture("Wingdramon", None);
        let card = r.game.players[0].battle_area[0].top_card().handle();
        r.game.current_trigger_context = Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.deleted_object = Some(DeletedObjectSnapshot {
                former_controller: 0,
                top_card: card,
                card_kind: CardKind::Digimon,
                retained_kind: None,
                traits: vec![],
                level: Some(5),
                dp: Some(7000),
                cause: EventCause::EffectDeletion,
                dp_just_before: Some(7000),
                level_just_before: Some(5),
                cost_just_before: Some(7),
                names_just_before: vec![name.into()],
                traits_just_before: vec![],
                source_count_just_before: 0,
                digisources_just_before: vec![],
                is_token: false,
            }); __fixture_trigger_context });
        let ctx = EffectReadContext::new(&r.game, card, None, 0);
        let pred = CompiledPredicate {
            event_target_name_contains: Some("Dramon".into()),
            ..Default::default()
        };
        assert_eq!(
            eval_predicate_with_bindings(&pred, &ctx, PredicateSubject::None, None),
            name == "Groundramon",
            "snapshot {name}"
        );
    }
}

#[test]
fn name_overlay_is_scoped_to_exact_physical_top_card() {
    use digimon_engine::{
        enums::{Expiry, ModifierType},
        modifiers::{ModifierEntry, ModifierPayload},
    };
    let mut r = fixture("Plain", None);
    let h = digimon_engine::PermanentHandle {
        player: 0,
        index: 0,
    };
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ChangeBaseCardName, 0, Expiry::EndOfTurn, 0)
            .with_payload(ModifierPayload::Name {
                value: "Wingdramon".into(),
                base: true,
            }),
    );
    let renamed = r.game.players[0].battle_area[0].top_card().handle();
    let other = r.place_on_field(0, "NAME-SUBJECT", Some(0));
    let unrenamed = r.game.players[0].battle_area[other.index as usize]
        .top_card()
        .handle();
    for (card, permanent, want) in [(renamed, h, true), (unrenamed, other, false)] {
        r.game.current_trigger_context = Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.event_card = Some(card); __fixture_trigger_context });
        let ctx = EffectReadContext::new(&r.game, card, Some(permanent), 0);
        for pred in [
            CompiledPredicate {
                source_name_contains: Some("Dramon".into()),
                ..Default::default()
            },
            CompiledPredicate {
                event_card_name_contains: Some("Dramon".into()),
                ..Default::default()
            },
        ] {
            assert_eq!(
                eval_predicate_with_bindings(&pred, &ctx, PredicateSubject::None, None),
                want,
                "physical card {card:?}"
            );
        }
    }
}

#[test]
fn event_exact_name_uses_runtime_alias_and_replaced_name() {
    use digimon_engine::{
        enums::{Expiry, ModifierType},
        modifiers::{ModifierEntry, ModifierPayload},
    };
    for renamed in [false, true] {
        let mut r = fixture("Plain", None);
        let h = digimon_engine::PermanentHandle {
            player: 0,
            index: 0,
        };
        if renamed {
            r.game.modifiers.add(
                h,
                ModifierEntry::simple(ModifierType::ChangeBaseCardName, 0, Expiry::EndOfTurn, 0)
                    .with_payload(ModifierPayload::Name {
                        value: "Wingdramon".into(),
                        base: true,
                    }),
            );
        } else {
            r.game.players[0].battle_area[0]
                .top_card_mut()
                .also_treated_as
                .push("Wingdramon".into());
        }
        let card = r.game.players[0].battle_area[0].top_card().handle();
        r.game.current_trigger_context = Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.event_card = Some(card); __fixture_trigger_context });
        let ctx = EffectReadContext::new(&r.game, card, Some(h), 0);
        for (name, want) in [("Wingdramon", true), ("Plain", !renamed)] {
            let pred = CompiledPredicate {
                event_card_name_is: Some(name.into()),
                ..Default::default()
            };
            assert_eq!(
                eval_predicate_with_bindings(&pred, &ctx, PredicateSubject::None, None),
                want,
                "{name}, renamed={renamed}"
            );
        }
    }
}

#[test]
fn deleted_event_card_uses_its_own_snapshot_without_borrowing_another_card() {
    use digimon_engine::{
        enums::CardKind,
        trigger_context::{DeletedObjectSnapshot, EventCause},
    };
    let mut r = fixture("Wingdramon", None);
    let physical = r.game.players[0]
        .battle_area
        .remove(0)
        .card_sources
        .remove(0);
    let card = physical.handle();
    r.game.players[0].trash.push(physical);
    let other = r.place_on_field(0, "NAME-SUBJECT", Some(0));
    let other_card = r.game.players[0].battle_area[other.index as usize]
        .top_card()
        .handle();
    for event_card in [card, other_card] {
        r.game.current_trigger_context = Some({ let mut __fixture_trigger_context = TriggerContext::default(); __fixture_trigger_context.event_card = Some(event_card); __fixture_trigger_context.deleted_object = Some(DeletedObjectSnapshot {
                former_controller: 0,
                top_card: card,
                card_kind: CardKind::Digimon,
                retained_kind: None,
                traits: vec![],
                level: Some(5),
                dp: Some(7000),
                cause: EventCause::EffectDeletion,
                dp_just_before: Some(7000),
                level_just_before: Some(5),
                cost_just_before: Some(7),
                names_just_before: vec!["Plain".into()],
                traits_just_before: vec![],
                source_count_just_before: 0,
                digisources_just_before: vec![],
                is_token: false,
            }); __fixture_trigger_context });
        let ctx = EffectReadContext::new(&r.game, card, None, 0);
        for pred in [
            CompiledPredicate {
                event_card_name_contains: Some("Dramon".into()),
                ..Default::default()
            },
            CompiledPredicate {
                event_card_name_is: Some("Wingdramon".into()),
                ..Default::default()
            },
        ] {
            assert_eq!(
                eval_predicate_with_bindings(&pred, &ctx, PredicateSubject::None, None),
                event_card == other_card,
                "event {event_card:?}"
            );
        }
    }
}

#[test]
fn card_subjects_keep_physical_names_in_top_source_hand_and_trash() {
    use digimon_engine::{
        enums::{Expiry, ModifierType},
        modifiers::{ModifierEntry, ModifierPayload},
    };
    let mut r = fixture("Wingdramon", None);
    let h = r.place_stack(0, &["NAME-SUBJECT", "NAME-SUBJECT"]);
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ChangeBaseCardName, 0, Expiry::EndOfTurn, 0)
            .with_payload(ModifierPayload::Name {
                value: "Plain".into(),
                base: true,
            }),
    );
    let mut cards: Vec<_> = r.game.players[0].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.handle())
        .collect();
    let hand_index = r.add_to_hand(0, "NAME-SUBJECT");
    cards.push(r.game.players[0].hand[hand_index].handle());
    let trash_index = r.add_to_hand(0, "NAME-SUBJECT");
    let trashed = r.game.players[0].hand.remove(trash_index);
    cards.push(trashed.handle());
    r.game.players[0].trash.push(trashed);
    let ctx = EffectReadContext::new(&r.game, cards[0], Some(h), 0);
    for card in cards {
        for pred in [
            CompiledPredicate {
                name_contains: Some("Dramon".into()),
                ..Default::default()
            },
            CompiledPredicate {
                name_is: Some("Wingdramon".into()),
                ..Default::default()
            },
            CompiledPredicate {
                name_in: Some(vec!["Wingdramon".into()]),
                ..Default::default()
            },
        ] {
            assert!(
                eval_predicate_with_bindings(&pred, &ctx, PredicateSubject::Card(card), None),
                "physical {card:?}"
            );
        }
    }
}

#[test]
fn rules_stack_name_query_uses_effective_top_and_physical_lower_sources() {
    use digimon_engine::{
        enums::{Expiry, ModifierType},
        modifiers::{ModifierEntry, ModifierPayload},
    };
    let mut r = fixture("Wingdramon", None);
    let single = digimon_engine::PermanentHandle {
        player: 0,
        index: 0,
    };
    let stack = r.place_stack(0, &["NAME-SUBJECT", "NAME-SUBJECT"]);
    for h in [single, stack] {
        r.game.modifiers.add(
            h,
            ModifierEntry::simple(ModifierType::ChangeBaseCardName, 0, Expiry::EndOfTurn, 0)
                .with_payload(ModifierPayload::Name {
                    value: "Plain".into(),
                    base: true,
                }),
        );
        assert_eq!(
            r.game.players[0].battle_area[h.index as usize].contains_card_name_for_rules(
                "Dramon",
                &r.game.card_data,
                &r.game.modifiers,
                h
            ),
            h == stack
        );
    }
}
