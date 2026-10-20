//! A cached store must describe the exact original metadata, not its lengths.
use digimon_engine::card_data::{
    CardData, DnaCost, DnaRequirement, DualCardData, DualDigimonFace, DualOptionFace, EvoCost,
};
use digimon_engine::card_store::{build_shared_card_store, shared_card_store, SharedCardStore};
use digimon_engine::debug_runner::make_test_card;
use digimon_engine::enums::CardColor;
use std::collections::HashMap;
use std::sync::Arc;

fn db(card: CardData) -> HashMap<String, CardData> {
    HashMap::from([(card.card_id.clone(), card)])
}

fn row(store: &SharedCardStore, id: &str) -> serde_json::Value {
    serde_json::to_value(&store.data[store.index[id]]).unwrap()
}

fn distinguishes(original: CardData, changed: CardData) {
    let id = original.card_id.clone();
    let original_db = db(original);
    let original_store = shared_card_store(&original_db);
    let before = row(&original_store, &id);
    let changed_db = db(changed);
    let fresh = build_shared_card_store(&changed_db);
    assert_ne!(
        before,
        row(&fresh, &id),
        "control must change actual stored metadata"
    );
    let changed_store = shared_card_store(&changed_db);
    assert_eq!(
        row(&changed_store, &id),
        row(&fresh, &id),
        "cached {id} must equal a fresh build"
    );
    assert!(!Arc::ptr_eq(&original_store, &changed_store));
    assert_eq!(
        row(&original_store, &id),
        before,
        "existing games retain their original metadata"
    );
    assert!(Arc::ptr_eq(
        &original_store,
        &shared_card_store(&original_db)
    ));
    assert!(Arc::ptr_eq(&changed_store, &shared_card_store(&changed_db)));
}

#[test]
fn same_length_text_and_alias_changes_do_not_reuse_old_cards() {
    let fields = [
        "card_name",
        "effect_text",
        "inherited_text",
        "security_text",
        "link_text",
        "effect_class_name",
        "digixros_aliases",
        "also_treated_as",
    ];
    for field in fields {
        let id = format!("CACHE-TEXT-{field}");
        let mut value = serde_json::to_value(make_test_card(&id, "Alpha")).unwrap();
        let list = matches!(field, "digixros_aliases" | "also_treated_as");
        value[field] = if list {
            serde_json::json!(["Alpha"])
        } else {
            serde_json::json!("Alpha")
        };
        let original = serde_json::from_value(value.clone()).unwrap();
        value[field] = if list {
            serde_json::json!(["Bravo"])
        } else {
            serde_json::json!("Bravo")
        };
        distinguishes(original, serde_json::from_value(value).unwrap());
    }
}

#[test]
fn same_length_ordinary_and_dna_cost_changes_do_not_reuse_old_cards() {
    let mut original = make_test_card("CACHE-EVO", "Cost");
    original.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 2,
    }];
    let mut changed = original.clone();
    changed.evo_costs[0].memory_cost = 3;
    distinguishes(original.clone(), changed);
    let mut changed = original.clone();
    changed.evo_costs[0].card_color = 1;
    distinguishes(original.clone(), changed);
    let mut changed = original.clone();
    changed.evo_costs[0].level = 4;
    distinguishes(original, changed);
    for field in ["name", "text"] {
        let mut original = make_test_card(&format!("CACHE-DNA-{field}"), "DNA");
        let requirement = DnaRequirement {
            level: 4,
            card_colors: vec![CardColor::Red],
            name_contains: "Alpha".into(),
            text_contains: "Alpha".into(),
        };
        original.dna_costs = vec![DnaCost {
            requirement1: requirement.clone(),
            requirement2: requirement,
            memory_cost: 0,
        }];
        let mut changed = original.clone();
        if field == "name" {
            changed.dna_costs[0].requirement1.name_contains = "Bravo".into();
        } else {
            changed.dna_costs[0].requirement2.text_contains = "Bravo".into();
        }
        distinguishes(original, changed);
    }
}

#[test]
fn dual_and_tensor_metadata_changes_do_not_reuse_old_cards() {
    let mut original = make_test_card("CACHE-DUAL", "Dual");
    original.dual = Some(DualCardData {
        digimon: DualDigimonFace {
            level: 3,
            dp: 2000,
            colors: vec![CardColor::Red],
            traits: vec!["Alpha".into()],
            evo_costs: vec![],
            effect_text: "Alpha".into(),
            inherited_text: String::new(),
            keywords: vec![],
        },
        option: DualOptionFace {
            use_cost: 4,
            colors: vec![CardColor::Blue],
            effect_text: "Alpha".into(),
            security_text: String::new(),
            keywords: vec![],
        },
    });
    let mut changed = original.clone();
    changed.dual.as_mut().unwrap().option.use_cost = 5;
    distinguishes(original.clone(), changed);
    let mut changed = original.clone();
    changed.dual.as_mut().unwrap().digimon.dp = 3000;
    distinguishes(original.clone(), changed);
    let mut changed = original.clone();
    changed.dual.as_mut().unwrap().option.effect_text = "Bravo".into();
    distinguishes(original, changed);
    let mut original = make_test_card("CACHE-NORM", "Tensor");
    original.norm_id = 0.125;
    let mut changed = original.clone();
    changed.norm_id = 0.25;
    distinguishes(original, changed);
}

#[test]
fn identical_maps_share_a_store_regardless_of_insertion_order() {
    let a = make_test_card("CACHE-ORDER-A", "Alpha");
    let b = make_test_card("CACHE-ORDER-B", "Bravo");
    let first = HashMap::from([
        (a.card_id.clone(), a.clone()),
        (b.card_id.clone(), b.clone()),
    ]);
    let second = HashMap::from([(b.card_id.clone(), b), (a.card_id.clone(), a)]);
    let held = shared_card_store(&first);
    assert!(Arc::ptr_eq(&held, &shared_card_store(&second)));
}

#[test]
fn tensor_zero_sign_is_preserved_in_distinct_stores() {
    let mut a = make_test_card("CACHE-ZERO-SIGN", "Zero");
    a.norm_id = 0.0;
    let mut b = a.clone();
    b.norm_id = -0.0;
    let sa = shared_card_store(&db(a));
    let sb = shared_card_store(&db(b));
    assert_eq!(
        sa.data[sa.index["CACHE-ZERO-SIGN"]].norm_id.to_bits(),
        0.0_f32.to_bits()
    );
    assert_eq!(
        sb.data[sb.index["CACHE-ZERO-SIGN"]].norm_id.to_bits(),
        (-0.0_f32).to_bits()
    );
    assert!(!Arc::ptr_eq(&sa, &sb));
}

#[test]
fn simultaneous_identical_inputs_share_the_winning_store() {
    let cards: HashMap<_, _> = (0..200)
        .map(|n| {
            let id = format!("CACHE-CONCURRENT-{n}");
            (id.clone(), make_test_card(&id, "Concurrent"))
        })
        .collect();
    let cards = Arc::new(cards);
    let barrier = Arc::new(std::sync::Barrier::new(8));
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let cards = Arc::clone(&cards);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                shared_card_store(&cards)
            })
        })
        .collect();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert!(results.iter().all(|store| Arc::ptr_eq(&results[0], store)));
}

#[test]
#[ignore = "explicit same-profile cache timing observation, not a training benchmark"]
fn measure_repeated_full_metadata_cache_lookup() {
    let path = std::env::var("CACHE_BENCH_CARDS").expect("explicit metadata input required");
    let cards = CardData::load_from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let held = shared_card_store(&cards);
    let start = std::time::Instant::now();
    for _ in 0..100 {
        let hit = std::hint::black_box(shared_card_store(std::hint::black_box(&cards)));
        assert!(Arc::ptr_eq(&held, &hit));
    }
    eprintln!(
        "cache_metadata_records={} lookups=100 elapsed_ms={}",
        cards.len(),
        start.elapsed().as_secs_f64() * 1000.0
    );
}
