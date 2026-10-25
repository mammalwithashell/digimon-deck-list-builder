//! The coordinated migration must explain page rows to the policy and preserve
//! observer permissions. These are contract tests, not card readiness evidence.
use digimon_engine::{
    card_registry::CardRegistry,
    debug_runner::{make_test_card, DebugRunner},
    effect_context::EffectContext,
    observation::{build_observation_tensor, parse_observation_profile},
    tensor_profiles::standard::{v2_full, v2_lite, v2_lite_deck},
};

#[test]
fn card_page_layout_hashes_match_exported_positions_and_schemas() {
    let mut calculated = Vec::new();
    for id in ["standard_lite_v2", "standard_lite_deck_v2", "standard_full_v2"] {
        let profile = digimon_engine::tensor_profiles::profile_by_id(id).unwrap();
        let computed = profile.layout_hash_with_schema_version_for_test(profile.feature_schema_version);
        println!("{id}: {computed}");
        calculated.push((profile, computed));
    }
    for (profile, computed) in calculated {
        assert_eq!(profile.layout_hash, computed);
    }
}

#[test]
fn physical_page_contract_expands_rows_and_changes_action_semantics_together() {
    assert_eq!(v2_lite::PENDING_CHOICE_ROW_SIZE, 128);
    assert_eq!(v2_lite::TENSOR_SIZE, 9434);
    assert_eq!(v2_lite_deck::TENSOR_SIZE, 9874);
    assert_eq!(v2_full::TENSOR_SIZE, 44506);
    assert_eq!(digimon_engine::action::space::SCHEMA_VERSION, 2);
}

#[test]
fn full_security_inspection_rows_are_visible_but_not_selectable_and_never_leak_to_other_seat() {
    use digimon_engine::action::{build_action_mask, space::PASS};
    let mut r = DebugRunner::builder().add_card(make_test_card("INSPECT-CARD", "Inspected card"))
        .deck(0, &["INSPECT-CARD"; 20]).deck(1, &["INSPECT-CARD"; 20])
        .security(0, &["INSPECT-CARD"; 40]).start();
    r.skip_mulligan();
    let host = r.place_stack(0, &["INSPECT-CARD"]);
    let top = r.game.player(0).battle_area[0].top_card().handle();
    EffectContext::new(&mut r.game, top, Some(host), 0).search_own_security_stack(
        "Inspect all security", true, |_, _| false, |_, _| panic!("no card is eligible"));
    let cards = r.game.card_data.iter().map(|c| (c.card_id.clone(), c.clone())).collect();
    let registry = CardRegistry::from_cards(&cards);
    let id = registry.get_index("INSPECT-CARD");
    assert!(id > 0);
    let profile = parse_observation_profile("standard_lite_v2").unwrap();
    for (page, position) in [(0, 0), (1, 28)] {
        if page > 0 { r.execute_action(0, digimon_engine::card_choice_page::CARD_CHOICE_NEXT).unwrap(); }
        let tensor = build_observation_tensor(&r.game, 0, &registry, profile);
        let base = v2_lite::OFF_PENDING_CHOICE_FEATURES;
        assert_eq!(tensor[base + 86], 1.0);
        assert_eq!(tensor[base + 87], 0.0);
        assert_eq!(tensor[base + 121], 1.0);
        assert_eq!(tensor[base + 93], f32::from(id));
        assert_eq!(tensor[base + 102], page as f32 / 65535.0);
        assert_eq!(tensor[base + 110], position as f32 / 65535.0);
        assert_eq!(build_action_mask(&r.game, 0)[30], 0.0);
        assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
        let other = build_observation_tensor(&r.game, 1, &registry, profile);
        assert!(other[base..base + v2_lite::PENDING_CHOICE_SIZE].iter().all(|&v| v == 0.0));
    }
    assert_eq!(digimon_engine::policies::greedy_action(&r.game,
        &build_action_mask(&r.game, 0)), PASS,
        "greedy must finish a no-match inspection instead of looping back through pages");
    r.execute_action(0, PASS).unwrap();
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn deep_hidden_security_row_explains_position_and_permission_without_card_identity() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("PAGE-CARD", "Page card"))
        .deck(0, &["PAGE-CARD"; 20])
        .deck(1, &["PAGE-CARD"; 20])
        .security(0, &["PAGE-CARD"; 40])
        .memory(3)
        .start();
    r.skip_mulligan();
    let host = r.place_stack(0, &["PAGE-CARD"]);
    let top = r.game.player(0).battle_area[0].top_card().handle();
    EffectContext::new(&mut r.game, top, Some(host), 0).select_security(
        0,
        "Security position",
        true,
        |_, index| index == 39,
        |_, _| {},
    );
    let cards = r
        .game
        .card_data
        .iter()
        .map(|c| (c.card_id.clone(), c.clone()))
        .collect();
    let registry = CardRegistry::from_cards(&cards);
    let profile = parse_observation_profile("standard_lite_v2").unwrap();
    let tensor = build_observation_tensor(&r.game, 0, &registry, profile);
    let base = v2_lite::OFF_PENDING_CHOICE_FEATURES;
    assert_eq!(tensor[base + 86], 1.0, "physical page row must be present");
    assert_eq!(tensor[base + 87], 1.0, "row commits a card");
    assert_eq!(
        tensor[base + 93],
        0.0,
        "face-down security has no printed registry identity"
    );
    assert_eq!(
        tensor[base + 110],
        39.0 / 65535.0,
        "position uses exact limb encoding"
    );
    assert_eq!(
        tensor[base + 119],
        0.0,
        "ordinary security pick has no inspection permission"
    );
    let explanation = digimon_engine::action::explain::explain_action(&r.game, 0, 30);
    assert_eq!(explanation.target_zone, Some(digimon_engine::action::explain::ActionZone::Security));
    assert_eq!(explanation.target_index, Some(39));
    assert!(explanation.card_id.is_none());
}
