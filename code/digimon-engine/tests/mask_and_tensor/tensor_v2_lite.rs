use digimon_engine::action::space::encode_attack;
use digimon_engine::card_registry::CardRegistry;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectSourceKind, GamePhase};
use digimon_engine::observation::{build_observation_tensor, parse_observation_profile};
use digimon_engine::selection::{PendingSelection, SelectionKind};
use digimon_engine::tensor_profiles::standard::v2_lite;

use crate::tensor_helpers::sample_game_with_known_cards;

fn registry_from_runner(r: &DebugRunner) -> CardRegistry {
    let cards = r
        .game
        .card_data
        .iter()
        .map(|card| (card.card_id.clone(), card.clone()))
        .collect();
    CardRegistry::from_cards(&cards)
}

fn v2_lite_tensor(r: &DebugRunner, observer: u8, registry: &CardRegistry) -> Vec<f32> {
    let profile = parse_observation_profile("standard_lite_v2").unwrap();
    build_observation_tensor(&r.game, observer, registry, profile)
}

#[test]
fn v2_lite_tensor_has_expected_size_and_version_marker() {
    let (game, registry) = sample_game_with_known_cards();
    let profile = parse_observation_profile("standard_lite_v2").unwrap();
    let tensor = build_observation_tensor(&game, 0, &registry, profile);

    assert_eq!(tensor.len(), v2_lite::TENSOR_SIZE);
    assert_eq!(tensor[v2_lite::OFF_GLOBAL_FEATURES], 2.0);
}

#[test]
fn v2_lite_does_not_encode_opponent_hand_identities() {
    let (game, registry) = sample_game_with_known_cards();
    let profile = parse_observation_profile("standard_lite_v2").unwrap();
    let tensor = build_observation_tensor(&game, 0, &registry, profile);
    let opponent_hand_card_index = registry.get_index("ST1-03") as f32;

    let known_positions = &digimon_engine::tensor_profiles::profile_by_id("standard_lite_v2")
        .unwrap()
        .positions()
        .0;
    for position in known_positions {
        if *position < v2_lite::OFF_OWN_HAND
            || *position >= v2_lite::OFF_OWN_HAND + v2_lite::OWN_HAND_SIZE
        {
            assert_ne!(
                tensor[*position], opponent_hand_card_index,
                "opponent hand card identity leaked at tensor position {position}"
            );
        }
    }
}

#[test]
fn v2_lite_uses_breeding_slot_14_with_battle_affordances_off() {
    let (game, registry) = sample_game_with_known_cards();
    let profile = parse_observation_profile("standard_lite_v2").unwrap();
    let tensor = build_observation_tensor(&game, 0, &registry, profile);

    let own_breeding_base = v2_lite::OFF_PERMANENT_SLOTS + 14 * v2_lite::PERMANENT_SLOT_SIZE;
    assert_eq!(tensor[own_breeding_base], 1.0);
    assert_eq!(tensor[own_breeding_base + 3], 0.0);
    assert_eq!(tensor[own_breeding_base + 4], 1.0);
    assert_eq!(tensor[own_breeding_base + 33], 0.0);
    assert_eq!(tensor[own_breeding_base + 34], 0.0);
}

#[test]
fn v2_lite_redacts_face_down_source_card_identity() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SRC-HIDDEN", "HiddenSource"))
        .add_card(make_test_card("SRC-TOP", "TopCard"))
        .start();

    r.place_on_field(0, "SRC-HIDDEN", Some(0));
    let top_data_idx = r
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == "SRC-TOP")
        .unwrap();
    let top = CardSource::new(top_data_idx, 0, r.game.next_card_index());
    r.game.digivolve_onto(0, 0, top);
    r.game.players[0].battle_area[0].card_sources[0].face_down = true;

    let registry = registry_from_runner(&r);
    let hidden_index = registry.get_index("SRC-HIDDEN") as f32;
    let tensor = v2_lite_tensor(&r, 0, &registry);
    let source_card_offset = v2_lite::OFF_PERMANENT_SLOTS
        + v2_lite::PERM_SOURCE_START_OFFSET
        + v2_lite::PERM_SOURCE_CARD_ID_OFFSET;

    assert_eq!(tensor[source_card_offset], 0.0);
    for position in digimon_engine::tensor_profiles::profile_by_id("standard_lite_v2")
        .unwrap()
        .positions()
        .0
    {
        assert_ne!(
            tensor[position], hidden_index,
            "face-down source card identity leaked at tensor position {position}"
        );
    }
}

#[test]
fn v2_lite_pending_choice_details_are_private_to_selecting_player() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("CHOICE-SRC", "ChoiceSource"))
        .start();
    let source = r.place_on_field(0, "CHOICE-SRC", Some(0));
    let source_card = r.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();
    let action_id = encode_attack(0, 14);
    r.game.current_phase = GamePhase::SelectTarget;
    r.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::Target,
        selecting_player: 0,
        previous_phase: GamePhase::Main,
        valid_action_ids: vec![action_id],
        is_optional: true,
        prompt: "choose a target".to_string(),
        effect_choices: None,
        source_card,
        source_permanent: Some(source),
        source_kind: EffectSourceKind::Digimon,
        callback: Box::new(|_, _| {}),
        on_decline: Some(Box::new(|_| {})),
    });

    let registry = registry_from_runner(&r);
    let selecting_tensor = v2_lite_tensor(&r, 0, &registry);
    let observing_tensor = v2_lite_tensor(&r, 1, &registry);
    let row_base = v2_lite::OFF_PENDING_CHOICE_FEATURES;
    let source_index = registry.get_index("CHOICE-SRC") as f32;

    assert_eq!(selecting_tensor[row_base], 1.0);
    assert!(selecting_tensor[row_base + 2] > 0.0);
    assert!(selecting_tensor[row_base + 4] > 0.0);
    assert_eq!(selecting_tensor[row_base + 18], 1.0);
    assert_eq!(
        selecting_tensor[row_base + v2_lite::PENDING_SOURCE_CARD_ID_OFFSET],
        source_index
    );
    assert_eq!(selecting_tensor[v2_lite::OFF_DECISION_CONTEXT + 27], 1.0);
    assert!(selecting_tensor[v2_lite::OFF_DECISION_CONTEXT + 28] > 0.0);

    assert_eq!(observing_tensor[v2_lite::OFF_DECISION_CONTEXT + 25], 1.0);
    assert_eq!(observing_tensor[v2_lite::OFF_DECISION_CONTEXT + 26], -1.0);
    assert_eq!(observing_tensor[v2_lite::OFF_DECISION_CONTEXT + 27], 0.0);
    assert_eq!(observing_tensor[v2_lite::OFF_DECISION_CONTEXT + 28], 0.0);
    assert_eq!(observing_tensor[row_base + 2], 0.0);
    assert_eq!(observing_tensor[row_base + 4], 0.0);
    assert_eq!(observing_tensor[row_base + 18], 0.0);
    assert_eq!(
        observing_tensor[row_base + v2_lite::PENDING_SOURCE_CARD_ID_OFFSET],
        0.0
    );
}

#[test]
fn v2_lite_opponent_permanent_rows_do_not_get_attack_affordances() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("ATK", "Attacker"))
        .add_card(make_test_card("DEF", "Defender"))
        .start();
    r.place_on_field(0, "ATK", Some(0));
    r.place_on_field(1, "ATK", Some(0));
    r.place_on_field(1, "DEF", Some(0));
    r.game.enter_main_phase();

    let registry = registry_from_runner(&r);
    let tensor = v2_lite_tensor(&r, 0, &registry);
    let own_base = v2_lite::OFF_PERMANENT_SLOTS;
    let opponent_base = v2_lite::OFF_PERMANENT_SLOTS
        + v2_lite::PERMANENT_SLOTS_PER_PLAYER * v2_lite::PERMANENT_SLOT_SIZE;

    assert_eq!(tensor[own_base + 33], 1.0);
    assert_eq!(tensor[opponent_base + 33], 0.0);
}
