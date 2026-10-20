//! Exact user-provided UlforceVeedramon export: 19 distinct cards, 4 eggs + 50 main.
//! This proves embedded YAML/runtime loadability, not complete behavioral fidelity.
use digimon_engine::{cards::build_registry, debug_runner::DebugRunner, dsl_registry};

const CARDS: &[(&str, usize)] = &[
    ("EX13-002", 4),
    ("EX13-017", 4),
    ("BT11-023", 4),
    ("BT22-019", 4),
    ("P-138", 4),
    ("EX13-019", 4),
    ("BT22-022", 1),
    ("EX13-022", 4),
    ("BT22-023", 3),
    ("EX13-023", 4),
    ("BT12-029", 2),
    ("EX13-069", 4),
    ("BT11-112", 4),
    ("BT22-085", 1),
    ("P-104", 2),
    ("LM-034", 2),
    ("LM-028", 1),
    ("LM-057", 1),
    ("LM-058", 1),
];

#[test]
fn ulforce_exact_export_all_19_yaml_load_and_register() {
    let pack = dsl_registry::from_embedded_cached().expect("embedded pack loads");
    let registered = build_registry().registered_card_ids();
    let missing: Vec<_> = CARDS
        .iter()
        .filter(|(id, _)| pack.lookup(id).is_none() || !registered.iter().any(|r| r == id))
        .map(|(id, _)| *id)
        .collect();
    assert!(
        missing.is_empty(),
        "Missing production YAML/runtime cards: {missing:?}"
    );
    let mut builder = DebugRunner::builder();
    for (id, _) in CARDS {
        builder = builder.dsl_card(id).expect("each listed YAML lowers");
    }
    let main: Vec<_> = CARDS
        .iter()
        .filter(|(id, _)| *id != "EX13-002")
        .flat_map(|(id, count)| std::iter::repeat_n(*id, *count))
        .collect();
    assert_eq!(main.len(), 50);
    let eggs = ["EX13-002"; 4];
    let runner = builder
        .deck(0, &main)
        .deck(1, &main)
        .digitama(0, &eggs)
        .digitama(1, &eggs)
        .build();
    assert_eq!(runner.deck_size(0), 50);
    assert_eq!(runner.deck_size(1), 50);
    assert_eq!(runner.game.players[0].digitama_deck.len(), 4);
    assert_eq!(runner.game.players[1].digitama_deck.len(), 4);
}
