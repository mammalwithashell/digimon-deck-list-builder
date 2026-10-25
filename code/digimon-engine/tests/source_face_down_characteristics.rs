use digimon_dsl::compiled::{CompiledCardKind, CompiledPredicate};
use digimon_engine::{
    debug_runner::{make_test_card, DebugRunner},
    dsl_cards::predicate::{eval_predicate, PredicateSubject},
    effect_context::{EffectReadContext, SourceSelectionRef},
};

#[test]
fn negative_characteristic_filters_cannot_identify_a_face_down_source() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("UNKNOWN-SOURCE", "Unknown source"))
        .add_card(make_test_card("KNOWN-TOP", "Known top"))
        .deck(0, &["KNOWN-TOP"; 20]).deck(1, &["KNOWN-TOP"; 20]).start();
    r.skip_mulligan();
    let host = r.place_stack(0, &["UNKNOWN-SOURCE", "KNOWN-TOP"]);
    let top = r.game.player(0).battle_area[0].top_card().handle();
    let card = r.game.player(0).battle_area[0].card_sources[0].handle();
    r.game.players[0].battle_area[0].card_sources[0].face_down = true;
    let subject = PredicateSubject::Source(SourceSelectionRef {
        permanent: host, field_index: host.index, source_index: 0, card,
    });
    let read = EffectReadContext::new(&r.game, top, Some(host), 0);
    let tamer = CompiledPredicate { kind: Some(CompiledCardKind::Tamer), ..Default::default() };
    assert!(!eval_predicate(&CompiledPredicate { not: Some(Box::new(tamer.clone())),
        ..Default::default() }, &read, subject), "unknown kind is not proof of non-Tamer");
    assert!(!eval_predicate(&CompiledPredicate { none_of: vec![tamer.clone()],
        ..Default::default() }, &read, subject));
    let physical = CompiledPredicate { is_face_down: Some(true), ..Default::default() };
    assert!(eval_predicate(&CompiledPredicate { any_of: vec![tamer, physical],
        ..Default::default() }, &read, subject), "a known structural alternative remains selectable");
}

#[test]
fn face_down_source_keeps_physical_source_properties_but_has_no_printed_characteristics() {
    let mut card = make_test_card("HIDDEN-SOURCE", "Hidden Hybrid");
    card.level = Some(4);
    card.traits = vec!["Hybrid".into()];
    let mut r = DebugRunner::builder()
        .add_card(card)
        .add_card(make_test_card("VISIBLE-TOP", "Visible top"))
        .deck(0, &["VISIBLE-TOP"; 20])
        .deck(1, &["VISIBLE-TOP"; 20])
        .start();
    r.skip_mulligan();
    let host = r.place_stack(0, &["HIDDEN-SOURCE", "VISIBLE-TOP"]);
    let top = r.game.player(0).battle_area[0].top_card().handle();
    let hidden = r.game.player(0).battle_area[0].card_sources[0].handle();
    let printed = CompiledPredicate {
        kind: Some(CompiledCardKind::Digimon),
        trait_has: Some("Hybrid".into()),
        level_eq: Some(4),
        ..Default::default()
    };
    assert!(eval_predicate(
        &printed,
        &EffectReadContext::new(&r.game, top, Some(host), 0),
        PredicateSubject::Card(hidden)
    ));
    r.game.players[0].battle_area[0].card_sources[0].face_down = true;
    let read = EffectReadContext::new(&r.game, top, Some(host), 0);
    let source = SourceSelectionRef {
        permanent: host,
        field_index: host.index,
        source_index: 0,
        card: hidden,
    };
    for subject in [
        PredicateSubject::Source(source),
        PredicateSubject::Card(hidden),
    ] {
        for predicate in [
            CompiledPredicate {
                kind: Some(CompiledCardKind::Digimon),
                ..Default::default()
            },
            CompiledPredicate {
                level_eq: Some(4),
                ..Default::default()
            },
            CompiledPredicate {
                trait_has: Some("Hybrid".into()),
                ..Default::default()
            },
            CompiledPredicate {
                name_is: Some("Hidden Hybrid".into()),
                ..Default::default()
            },
            CompiledPredicate {
                card_number_is: Some("HIDDEN-SOURCE".into()),
                ..Default::default()
            },
        ] {
            assert!(
                !eval_predicate(&predicate, &read, subject),
                "face-down printed predicate: {predicate:?}"
            );
        }
    }
    assert!(eval_predicate(
        &CompiledPredicate {
            is_face_down: Some(true),
            is_bottom_source: Some(true),
            ..Default::default()
        },
        &read,
        PredicateSubject::Source(source)
    ));
    assert!(eval_predicate(
        &CompiledPredicate::default(),
        &read,
        PredicateSubject::Source(source)
    ));
}
