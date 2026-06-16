use digimon_engine::action::space::encode_attack;
use digimon_engine::card_registry::CardRegistry;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect, EffectCategoryFlags, EffectObservationMetadata};
use digimon_engine::enums::{EffectSourceKind, EffectTiming, GamePhase};
use digimon_engine::observation::{build_observation_tensor, parse_observation_profile};
use digimon_engine::selection::{EffectChoiceEntry, PendingSelection, SelectionKind};
use digimon_engine::tensor_profiles::standard::v2_lite;
use std::sync::Arc;

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

struct OncePerTurnInheritedEffect;

impl CardEffect for OncePerTurnInheritedEffect {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::inherited(card)
            .name("Hidden source OPT")
            .once_per_turn()
            .build()]
    }
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
fn v2_lite_redacts_face_down_source_aggregate_opt_counts() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SRC-HIDDEN-OPT", "HiddenOptSource"))
        .add_card(make_test_card("SRC-TOP", "TopCard"))
        .start();
    r.register_effect("SRC-HIDDEN-OPT", Arc::new(OncePerTurnInheritedEffect));

    r.place_on_field(0, "SRC-HIDDEN-OPT", Some(0));
    let top_data_idx = r
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == "SRC-TOP")
        .unwrap();
    let top = CardSource::new(top_data_idx, 0, r.game.next_card_index());
    r.game.digivolve_onto(0, 0, top);
    let hidden_handle = r.game.players[0].battle_area[0].card_sources[0].handle();
    r.game.players[0].battle_area[0].card_sources[0].face_down = true;
    r.game.players[0].battle_area[0].record_activation(hidden_handle, 0);

    let registry = registry_from_runner(&r);
    assert_eq!(
        r.game
            .opt_total(digimon_engine::permanent::PermanentHandle {
                player: 0,
                index: 0
            }),
        1
    );
    assert_eq!(
        r.game.opt_used(digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: 0
        }),
        1
    );

    let tensor = v2_lite_tensor(&r, 0, &registry);
    assert_eq!(
        tensor[v2_lite::OFF_PERMANENT_SLOTS + v2_lite::PERM_OPT_TOTAL_OFFSET],
        0.0
    );
    assert_eq!(
        tensor[v2_lite::OFF_PERMANENT_SLOTS + v2_lite::PERM_OPT_USED_OFFSET],
        0.0
    );
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
        zone_owner: None,
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
fn v2_lite_pending_choice_rows_follow_valid_action_order() {
    let (mut game, registry) = sample_game_with_known_cards();
    let source_card = game.players[0].battle_area[0].top_card().handle();
    game.current_phase = GamePhase::EffectChoice;
    game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::EffectChoice,
        selecting_player: 0,
        previous_phase: GamePhase::Main,
        valid_action_ids: vec![1003, 62, 1001],
        is_optional: false,
        prompt: "choose in authored order".to_string(),
        effect_choices: None,
        source_card,
        source_permanent: None,
        source_kind: EffectSourceKind::Digimon,
        zone_owner: None,
        callback: Box::new(|_, _| {}),
        on_decline: None,
    });

    let profile = parse_observation_profile("standard_lite_v2").unwrap();
    let tensor = build_observation_tensor(&game, 0, &registry, profile);
    let base = v2_lite::OFF_PENDING_CHOICE_FEATURES;

    assert_eq!(tensor[base], 1.0);
    assert_eq!(
        tensor[base + 2],
        1003.0 / digimon_engine::action::space::ACTION_SPACE_SIZE as f32
    );
    assert_eq!(
        tensor[base + v2_lite::PENDING_CHOICE_ROW_SIZE + 2],
        62.0 / digimon_engine::action::space::ACTION_SPACE_SIZE as f32
    );
    assert_eq!(
        tensor[base + 2 * v2_lite::PENDING_CHOICE_ROW_SIZE + 2],
        1001.0 / digimon_engine::action::space::ACTION_SPACE_SIZE as f32
    );
}

#[test]
fn v2_lite_pending_choice_rows_include_choice_source_kind_timing_and_source_card() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SEL-SRC", "SelectionSource"))
        .add_card(make_test_card("CHOICE-SRC", "ChoiceSource"))
        .start();
    let selection_source = r.place_on_field(0, "SEL-SRC", Some(0));
    let choice_source = r.place_on_field(0, "CHOICE-SRC", Some(1));
    let selection_source_card = r.game.players[0].battle_area[selection_source.index as usize]
        .top_card()
        .handle();
    let choice_source_card = r.game.players[0].battle_area[choice_source.index as usize]
        .top_card()
        .handle();

    r.game.current_phase = GamePhase::EffectChoice;
    r.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::TriggerOrder,
        selecting_player: 0,
        previous_phase: GamePhase::Main,
        valid_action_ids: vec![1003],
        is_optional: false,
        prompt: "choose trigger".to_string(),
        effect_choices: Some(vec![EffectChoiceEntry {
            label: "choice trigger".to_string(),
            action_id: 1003,
            source_card: Some(choice_source_card),
            source_kind: Some(EffectSourceKind::Tamer),
            timing: Some(EffectTiming::OnDeletion),
            is_optional: false,
            observation_metadata: EffectObservationMetadata::default(),
        }]),
        source_card: selection_source_card,
        source_permanent: Some(selection_source),
        source_kind: EffectSourceKind::Digimon,
        zone_owner: None,
        callback: Box::new(|_, _| {}),
        on_decline: None,
    });

    let registry = registry_from_runner(&r);
    let tensor = v2_lite_tensor(&r, 0, &registry);
    let base = v2_lite::OFF_PENDING_CHOICE_FEATURES;

    assert_eq!(tensor[base + 22 + 6], 1.0);
    assert_eq!(tensor[base + 34 + 1], 1.0);
    assert_eq!(
        tensor[base + v2_lite::PENDING_SOURCE_CARD_ID_OFFSET],
        registry.get_index("CHOICE-SRC") as f32
    );
}

#[test]
fn v2_lite_pending_choice_rows_encode_effect_category_metadata() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("CATEGORY-SRC", "CategorySource"))
        .start();
    let source = r.place_on_field(0, "CATEGORY-SRC", Some(0));
    let source_card = r.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();
    let effect = Effect::on_deletion(source_card)
        .observation_metadata(EffectObservationMetadata {
            categories: EffectCategoryFlags {
                delete: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .build();

    r.game.current_phase = GamePhase::EffectChoice;
    r.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::TriggerOrder,
        selecting_player: 0,
        previous_phase: GamePhase::Main,
        valid_action_ids: vec![1003],
        is_optional: false,
        prompt: "choose trigger".to_string(),
        effect_choices: Some(vec![EffectChoiceEntry {
            label: "delete trigger".to_string(),
            action_id: 1003,
            source_card: Some(source_card),
            source_kind: Some(EffectSourceKind::Digimon),
            timing: Some(EffectTiming::OnDeletion),
            is_optional: false,
            observation_metadata: effect.observation_metadata,
        }]),
        source_card,
        source_permanent: Some(source),
        source_kind: EffectSourceKind::Digimon,
        zone_owner: None,
        callback: Box::new(|_, _| {}),
        on_decline: None,
    });

    let registry = registry_from_runner(&r);
    let tensor = v2_lite_tensor(&r, 0, &registry);
    let base = v2_lite::OFF_PENDING_CHOICE_FEATURES;

    assert_eq!(tensor[base + 45], 1.0);
}

/// Build a stack of `count` distinctly-named test cards on `player`'s field
/// and relocate it into the breeding area, returning the runner. The breeding
/// carrier's `card_sources` holds `count` entries (sources 0..count-1).
fn runner_with_breeding_stack(player: u8, count: usize) -> DebugRunner {
    let mut builder = DebugRunner::builder();
    let ids: Vec<String> = (0..count).map(|i| format!("BSRC-{i:02}")).collect();
    for (i, id) in ids.iter().enumerate() {
        builder = builder.add_card(make_test_card(id, &format!("BreedSource{i}")));
    }
    let mut r = builder.start();
    let id_refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let handle = r.place_stack(player, &id_refs);
    // Relocate the freshly-built stack into the breeding area (mirrors
    // DebugRunner::place_in_breeding, which only handles a single card).
    let perm = r.game.players[player as usize]
        .battle_area
        .remove(handle.index as usize);
    r.game.players[player as usize].breeding_area = Some(perm);
    r
}

/// Task S1.4: a breeding-area carrier (e.g. King Drasil) with a 12-card
/// digivolution stack must surface every source slot in the tensor —
/// including source index 11 — because `BREEDING_SOURCE_SELECT` makes all
/// 12 slots (`SOURCES_PER_FIELD == 12`) selectable in the action space.
#[test]
fn v2_lite_breeding_carrier_encodes_twelfth_source_slot() {
    let count = digimon_engine::action::space::SOURCES_PER_FIELD as usize; // 12
    let r = runner_with_breeding_stack(0, count);
    let registry = registry_from_runner(&r);
    let tensor = v2_lite_tensor(&r, 0, &registry);

    let breeding_base = v2_lite::OFF_PERMANENT_SLOTS + 14 * v2_lite::PERMANENT_SLOT_SIZE;
    assert_eq!(tensor[breeding_base], 1.0, "breeding slot 14 occupied");

    // Assert every card-source SLOT index 0..count of the permanent's
    // encoded `card_sources` is observable (non-zero card identity) in the
    // tensor.
    for source_idx in 0..count {
        let source_base = breeding_base
            + v2_lite::PERM_SOURCE_START_OFFSET
            + source_idx * v2_lite::PERM_SOURCE_ENTRY_SIZE
            + v2_lite::PERM_SOURCE_CARD_ID_OFFSET;
        assert!(
            source_base < v2_lite::OFF_PERMANENT_SLOTS + v2_lite::PERMANENT_SLOTS_SIZE,
            "source slot {source_idx} falls outside permanent_slots section"
        );
        assert_ne!(
            tensor[source_base], 0.0,
            "breeding source slot {source_idx} card identity not encoded"
        );
    }
}

/// The same gap exists for the battle-area twin: a battle permanent with a
/// 12-card digivolution stack must surface source index 11.
#[test]
fn v2_lite_battle_permanent_encodes_twelfth_source_slot() {
    let count = digimon_engine::action::space::SOURCES_PER_FIELD as usize; // 12
    let mut builder = DebugRunner::builder();
    let ids: Vec<String> = (0..count).map(|i| format!("BATSRC-{i:02}")).collect();
    for (i, id) in ids.iter().enumerate() {
        builder = builder.add_card(make_test_card(id, &format!("BattleSource{i}")));
    }
    let mut r = builder.start();
    let id_refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    r.place_stack(0, &id_refs);
    let registry = registry_from_runner(&r);
    let tensor = v2_lite_tensor(&r, 0, &registry);

    let perm_base = v2_lite::OFF_PERMANENT_SLOTS;
    assert_eq!(tensor[perm_base], 1.0, "battle slot 0 occupied");
    for source_idx in 0..count {
        let source_base = perm_base
            + v2_lite::PERM_SOURCE_START_OFFSET
            + source_idx * v2_lite::PERM_SOURCE_ENTRY_SIZE
            + v2_lite::PERM_SOURCE_CARD_ID_OFFSET;
        assert!(
            source_base < v2_lite::OFF_PERMANENT_SLOTS + v2_lite::PERMANENT_SLOTS_SIZE,
            "source slot {source_idx} falls outside permanent_slots section"
        );
        assert_ne!(
            tensor[source_base], 0.0,
            "battle source slot {source_idx} card identity not encoded"
        );
    }
}

/// Action ↔ observation consistency: for a breeding carrier, every source
/// index for which `encode_breeding_source_select(owner, s)` is a legal
/// action must have an observable source slot in the tensor.
#[test]
fn v2_lite_breeding_source_select_actions_all_observable() {
    use digimon_engine::action::space::{encode_breeding_source_select, SOURCES_PER_FIELD};

    let count = SOURCES_PER_FIELD as usize;
    let r = runner_with_breeding_stack(0, count);
    let registry = registry_from_runner(&r);
    let tensor = v2_lite_tensor(&r, 0, &registry);
    let breeding_base = v2_lite::OFF_PERMANENT_SLOTS + 14 * v2_lite::PERMANENT_SLOT_SIZE;

    for source in 0..SOURCES_PER_FIELD {
        // owner carrier index 0 is the observer-relative "own" breeding carrier.
        assert!(
            encode_breeding_source_select(0, source).is_some(),
            "encode_breeding_source_select(0, {source}) should be legal"
        );
        let source_base = breeding_base
            + v2_lite::PERM_SOURCE_START_OFFSET
            + source as usize * v2_lite::PERM_SOURCE_ENTRY_SIZE
            + v2_lite::PERM_SOURCE_CARD_ID_OFFSET;
        assert_ne!(
            tensor[source_base], 0.0,
            "BREEDING_SOURCE_SELECT slot {source} selectable but not observable"
        );
    }
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
