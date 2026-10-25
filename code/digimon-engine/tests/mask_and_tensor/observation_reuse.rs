//! Optimized observations must retain the old feature semantics and mask.
use digimon_engine::{
    action::build_action_mask,
    card_registry::CardRegistry,
    card_source::{CardHandle, CardSource},
    debug_runner::{make_test_card, DebugRunner},
    effect::{CardEffect, Effect},
    enums::GamePhase,
    observation::{
        build_observation_tensor, build_observation_tensor_with_mask, ObservationProfileId,
    },
    tensor_profiles::standard::v2_lite as layout,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

struct MixedEffects(Arc<AtomicUsize>);
impl CardEffect for MixedEffects {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        self.0.fetch_add(1, Ordering::Relaxed);
        vec![
            Effect::inherited(card)
                .once_per_turn()
                .shared_opt_group(128)
                .dp_modifier(-3000)
                .build(),
            Effect::inherited(card)
                .once_per_turn()
                .shared_opt_group(128)
                .build(),
            Effect::declarative(card)
                .once_per_turn()
                .dp_modifier(1000)
                .condition(|ctx| ctx.memory() >= 0)
                .dp_modifier_fn(|ctx, _| Some(ctx.memory() as i32 * 100))
                .build(),
            Effect::declarative(card).succession(|_, _| true).build(),
        ]
    }
}

fn fixture(depth: usize) -> (DebugRunner, CardRegistry, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("MIX", "Mixed"))
        .add_card(make_test_card("NONE", "Unregistered"))
        .hand(0, &["MIX"])
        .hand(1, &["NONE"])
        .start();
    r.register_effect("MIX", Arc::new(MixedEffects(calls.clone())));
    r.place_on_field(0, "MIX", Some(0));
    let data_index = r.game.players[0].battle_area[0].top_card().data_index;
    for _ in 1..depth {
        let index = r.game.next_card_index();
        r.game.players[0].battle_area[0]
            .card_sources
            .push(CardSource::new(data_index, 0, index));
    }
    r.place_on_field(1, "NONE", Some(0));
    let source = r.game.players[0].battle_area[0].card_sources[0].handle();
    r.game.players[0].battle_area[0].record_activation(source, 128);
    let registry = CardRegistry::from_cards(
        &r.game
            .card_data
            .iter()
            .map(|c| (c.card_id.clone(), c.clone()))
            .collect(),
    );
    (r, registry, calls)
}

#[test]
fn supplied_mask_matches_public_builder_across_profiles_players_and_phases() {
    let (mut r, registry, _) = fixture(4);
    for phase in [GamePhase::Mulligan, GamePhase::Breeding, GamePhase::Main] {
        r.game.current_phase = phase;
        for player in [0, 1] {
            let mask = build_action_mask(&r.game, player);
            for profile in [
                ObservationProfileId::StandardCompactV1,
                ObservationProfileId::StandardLiteV2,
                ObservationProfileId::StandardLiteDeckV2,
                ObservationProfileId::StandardFullV2,
            ] {
                let expected = build_observation_tensor(&r.game, player, &registry, profile);
                let actual =
                    build_observation_tensor_with_mask(&r.game, player, &registry, profile, &mask);
                assert_eq!(
                    actual.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                    expected.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
                );
            }
            assert_eq!(mask, build_action_mask(&r.game, player));
        }
    }
}

#[test]
fn fused_features_match_independent_queries_including_hidden_and_deep_sources() {
    for depth in [1, 4, 14] {
        let (mut r, registry, _) = fixture(depth);
        let handle = digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: 0,
        };
        for memory in [-1, 0, 3] {
            r.game.set_memory(memory);
            for hidden in [false, true] {
                r.game.players[0].battle_area[0].card_sources[0].face_down = hidden;
                let tensor = build_observation_tensor(
                    &r.game,
                    0,
                    &registry,
                    ObservationProfileId::StandardLiteV2,
                );
                let base = layout::OFF_PERMANENT_SLOTS;
                assert_eq!(
                    tensor[base + layout::PERM_OPT_TOTAL_OFFSET],
                    if hidden {
                        0.
                    } else {
                        r.game.opt_total(handle) as f32
                    }
                );
                assert_eq!(
                    tensor[base + layout::PERM_OPT_USED_OFFSET],
                    if hidden {
                        0.
                    } else {
                        r.game.opt_used(handle) as f32
                    }
                );
                for i in 0..depth.min(layout::PERM_MAX_SOURCES) {
                    let source_base = base
                        + layout::PERM_SOURCE_START_OFFSET
                        + i * layout::PERM_SOURCE_ENTRY_SIZE;
                    let redacted = hidden && i == 0;
                    let opt = if redacted {
                        0.
                    } else {
                        r.game.source_opt_state(handle, i)
                    };
                    let dp = if redacted {
                        0.
                    } else {
                        r.game.source_dp_contribution(handle, i) as f32
                            / digimon_engine::tensor::DP_NORM
                    };
                    assert_eq!(
                        tensor[source_base + layout::PERM_SOURCE_OPT_STATE_OFFSET].to_bits(),
                        opt.to_bits()
                    );
                    assert_eq!(
                        tensor[source_base + layout::PERM_SOURCE_DP_CONTRIBUTION_OFFSET].to_bits(),
                        dp.to_bits()
                    );
                }
            }
        }
    }
}

#[test]
fn observation_constructs_each_sources_effects_once_plus_one_succession_scan() {
    let (mut r, registry, calls) = fixture(4);
    // Mulligan masks require no field-effect inspection, isolating the observation.
    r.game.current_phase = GamePhase::Mulligan;
    let mask = build_action_mask(&r.game, 0);
    calls.store(0, Ordering::Relaxed);
    build_observation_tensor_with_mask(
        &r.game,
        0,
        &registry,
        ObservationProfileId::StandardLiteV2,
        &mask,
    );
    assert_eq!(calls.load(Ordering::Relaxed), 5);
}

#[test]
fn pending_selection_mask_reuse_preserves_both_observers() {
    use digimon_engine::{
        enums::EffectSourceKind,
        selection::{PendingSelection, SelectionKind},
    };
    let (mut r, registry, _) = fixture(2);
    let handle = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    r.game.current_phase = GamePhase::SelectTarget;
    r.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::Target,
        selecting_player: 1,
        previous_phase: GamePhase::Main,
        valid_action_ids: vec![100, 101],
        is_optional: true,
        prompt: "choose".into(),
        effect_choices: None,
        card_choice: None,
        source_card: r.game.players[0].battle_area[0].top_card().handle(),
        source_permanent: Some(handle),
        source_kind: EffectSourceKind::Digimon,
        zone_owner: None,
        callback: Box::new(|_, _| {}),
        on_decline: Some(Box::new(|_| {})),
    });
    for player in [0, 1] {
        let mask = build_action_mask(&r.game, player);
        let expected = build_observation_tensor(
            &r.game,
            player,
            &registry,
            ObservationProfileId::StandardLiteV2,
        );
        let actual = build_observation_tensor_with_mask(
            &r.game,
            player,
            &registry,
            ObservationProfileId::StandardLiteV2,
            &mask,
        );
        assert_eq!(actual, expected);
        assert_eq!(actual[layout::OFF_PENDING_CHOICE_FEATURES], player as f32);
    }
}

struct CountLegality(Arc<AtomicUsize>);
impl CardEffect for CountLegality {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let calls = self.0.clone();
        vec![Effect::on_play(card)
            .timing(digimon_engine::enums::EffectTiming::MainFromHand)
            .condition(move |_| {
                calls.fetch_add(1, Ordering::Relaxed);
                true
            })
            .process(|_| {})
            .build()]
    }
}

#[test]
fn supplied_mask_does_not_repeat_legality_conditions() {
    let (mut r, registry, _) = fixture(1);
    let calls = Arc::new(AtomicUsize::new(0));
    r.register_effect("MIX", Arc::new(CountLegality(calls.clone())));
    r.game.current_phase = GamePhase::Main;
    let mask = build_action_mask(&r.game, 0);
    assert!(calls.load(Ordering::Relaxed) > 0);
    assert_eq!(
        mask[digimon_engine::action::HAND_EFFECT_START as usize],
        1.0
    );
    calls.store(0, Ordering::Relaxed);
    let tensor = build_observation_tensor_with_mask(
        &r.game,
        0,
        &registry,
        ObservationProfileId::StandardLiteV2,
        &mask,
    );
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    assert_eq!(tensor[layout::OFF_OWN_HAND + 20], 1.0);
}
