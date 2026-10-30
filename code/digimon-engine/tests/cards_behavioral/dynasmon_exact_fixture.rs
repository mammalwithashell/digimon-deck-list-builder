//! Exact Tesero fixture loadability; card behavior is covered by per-card tests.
use digimon_engine::{cards::build_registry, debug_runner::DebugRunner, dsl_registry};
use std::collections::BTreeMap;

#[derive(serde::Deserialize)]
struct Fixture {
    main: BTreeMap<String, usize>,
    eggs: BTreeMap<String, usize>,
}

#[test]
fn dynasmon_exact_120647_all_22_production_cards_lower_and_build_50_plus_4() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../../qa/archetype-qa/dynasmon-tesero-120647.json"
    )).unwrap();
    assert_eq!(fixture.main.values().sum::<usize>(), 50);
    assert_eq!(fixture.eggs.values().sum::<usize>(), 4);
    assert_eq!(fixture.main.len() + fixture.eggs.len(), 22);
    let pack = dsl_registry::from_embedded_cached().expect("embedded production pack");
    let registered = build_registry().registered_card_ids();
    let missing: Vec<_> = fixture.main.keys().chain(fixture.eggs.keys())
        .filter(|id| pack.lookup(id).is_none() || !registered.iter().any(|r| r == *id))
        .collect();
    assert!(missing.is_empty(), "Missing production YAML/runtime cards: {missing:?}");
    let mut builder = DebugRunner::builder();
    for id in fixture.main.keys().chain(fixture.eggs.keys()) {
        builder = builder.dsl_card(id).expect("exact fixture card lowers");
    }
    let main: Vec<_> = fixture.main.iter().flat_map(|(id, n)| std::iter::repeat_n(id.as_str(), *n)).collect();
    let eggs: Vec<_> = fixture.eggs.iter().flat_map(|(id, n)| std::iter::repeat_n(id.as_str(), *n)).collect();
    let runner = builder.deck(0, &main).deck(1, &main)
        .digitama(0, &eggs).digitama(1, &eggs).build();
    for player in 0..2 {
        assert_eq!(runner.deck_size(player), 50);
        assert_eq!(runner.game.players[player as usize].digitama_deck.len(), 4);
    }
}
