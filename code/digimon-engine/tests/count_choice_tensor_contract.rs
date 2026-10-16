//! Contract RED: install after count core but before tensor candidate.
use digimon_engine::count_choice::{
    install_count_choice, CountChoiceHost, CountChoicePage, CountChoiceValue,
};
use digimon_engine::debug_runner::make_test_card;
use digimon_engine::dsl_cards::{bindings::Bindings, step::StepRuntime};
use digimon_engine::observation::{build_observation_tensor, parse_observation_profile};
use digimon_engine::tensor_profiles::standard::{v2_full, v2_lite, v2_lite_deck};
use digimon_engine::{CardRegistry, DebugRunner, EffectContext};

fn fixture() -> (DebugRunner, CardRegistry) {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("COUNT-TENSOR", "Count tensor"))
        .deck(0, &["COUNT-TENSOR"; 12])
        .deck(1, &["COUNT-TENSOR"; 12])
        .memory(3)
        .start();
    r.game.current_phase = digimon_engine::GamePhase::Main;
    r.place_on_field(0, "COUNT-TENSOR", Some(0));
    let source = r.game.player(0).battle_area[0].top_card().handle();
    let values = (7..=300)
        .map(|n| CountChoiceValue {
            declared: n,
            payable: n - 1,
        })
        .collect();
    let page = CountChoicePage::new(
        values,
        Some(CountChoiceHost {
            player: 0,
            field_index: 0,
            battle_identity: u64::MAX,
        }),
    )
    .unwrap();
    let mut ctx = EffectContext::new(&mut r.game, source, None, 0);
    install_count_choice(
        &mut ctx,
        page,
        true,
        "Count".into(),
        None,
        vec![],
        Bindings::new(),
        StepRuntime::default(),
    );
    let registry = CardRegistry::from_cards(
        &r.game
            .card_data
            .iter()
            .map(|c| (c.card_id.clone(), c.clone()))
            .collect(),
    );
    (r, registry)
}
fn decode(t: &[f32], start: usize) -> u64 {
    t[start..start + 4].iter().enumerate().fold(0, |n, (i, v)| {
        n | (((v * 65535.0).round() as u64) << (16 * i))
    })
}
fn tensor(r: &DebugRunner, registry: &CardRegistry, profile: &str, observer: u8) -> Vec<f32> {
    build_observation_tensor(
        &r.game,
        observer,
        registry,
        parse_observation_profile(profile).unwrap(),
    )
}

#[test]
fn schemas_change_without_changing_shapes_and_match_computed_hashes() {
    for (p, schema, size) in [
        (v2_lite::PROFILE, "standard_lite_v2.3", 8410),
        (v2_lite_deck::PROFILE, "standard_lite_deck_v2.2", 8850),
        (v2_full::PROFILE, "standard_full_v2.4", 43482),
    ] {
        assert_eq!(p.feature_schema_version, schema);
        assert_eq!(p.tensor_size, size);
        assert_eq!(
            p.layout_hash,
            p.layout_hash_with_schema_version_for_test(schema)
        );
    }
    assert_eq!(digimon_engine::action::ACTION_SPACE_SIZE, 2192);
}

#[test]
fn pending_rows_expose_exact_counts_pages_and_physical_host_identity() {
    let (mut r, registry) = fixture();
    let t = tensor(&r, &registry, "standard_lite_v2", 0);
    let first = v2_lite::OFF_PENDING_CHOICE_FEATURES;
    assert_eq!(t[first + 60], 1.0);
    assert_eq!(decode(&t, first + 63), 7);
    assert_eq!(decode(&t, first + 67), 6);
    assert_eq!(decode(&t, first + 71), 0);
    assert_eq!(decode(&t, first + 75), 11);
    assert_eq!(decode(&t, first + 79), u64::MAX);
    assert_eq!(t[first + 83], 1.0);
    assert_eq!(t[first + 85], 1.0);
    let next = first + 28 * v2_lite::PENDING_CHOICE_ROW_SIZE;
    assert_eq!(t[next + 62], 1.0);
    assert_eq!(decode(&t, next + 63), 0);
    for _ in 0..10 {
        r.game.resolve_selection(0, 59).unwrap();
    }
    let t = tensor(&r, &registry, "standard_lite_v2", 0);
    assert_eq!(decode(&t, first + 63), 287);
    assert_eq!(decode(&t, first + 71), 10);
    let last = first + 13 * v2_lite::PENDING_CHOICE_ROW_SIZE;
    assert_eq!(decode(&t, last + 63), 300);
    assert_eq!(decode(&t, last + 67), 299);
    let previous = first + 14 * v2_lite::PENDING_CHOICE_ROW_SIZE;
    assert_eq!(t[previous + 61], 1.0);
}

#[test]
fn all_profiles_hide_count_metadata_from_nonchoosing_player() {
    let (r, registry) = fixture();
    for profile in [
        "standard_lite_v2",
        "standard_lite_deck_v2",
        "standard_full_v2",
    ] {
        let t = tensor(&r, &registry, profile, 1);
        for row in 0..32 {
            let start = v2_lite::OFF_PENDING_CHOICE_FEATURES + row * 96;
            assert!(t[start + 60..start + 86].iter().all(|v| *v == 0.0));
        }
    }
}

#[test]
fn full_action_features_distinguish_count_commit_and_navigation() {
    let (r, registry) = fixture();
    let t = tensor(&r, &registry, "standard_full_v2", 0);
    let row = |id| v2_full::OFF_ACTION_ID_FEATURES + id * v2_full::ACTION_ID_ROW_SIZE;
    assert_eq!(t[row(30) + 10], 1.0);
    assert_eq!(t[row(59) + 15], 1.0);
    assert_eq!(t[row(58) + 11], 0.0);
    let hidden = tensor(&r, &registry, "standard_full_v2", 1);
    assert_eq!(hidden[row(30) + 10], 0.0);
    assert_eq!(hidden[row(59) + 15], 0.0);
}

#[test]
fn ordinary_choice_keeps_new_numeric_fields_zero() {
    let (mut r, registry) = fixture();
    r.game.pending_selection = None;
    r.game.pending_selection_resume = None;
    let source = r.game.player(0).battle_area[0].top_card().handle();
    let mut ctx = EffectContext::new(&mut r.game, source, None, 0);
    ctx.select_effect_choice("Ordinary", vec!["A".into(), "B".into()], |_, _| {});
    let t = tensor(&r, &registry, "standard_lite_v2", 0);
    for row in 0..32 {
        let start = v2_lite::OFF_PENDING_CHOICE_FEATURES + row * 96;
        assert!(t[start + 60..start + 86].iter().all(|v| *v == 0.0));
    }
}
