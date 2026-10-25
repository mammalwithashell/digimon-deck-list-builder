use digimon_engine::{
    action::build_action_mask,
    card_data::CardData,
    card_registry::CardRegistry,
    observation::ObservationProfileId,
    runners::HeadlessRunner,
    search::{decision_player, determinized_search_batched, DeterminizedConfig, UniformEvaluator},
};

fn replay(id: u64, count: usize) -> (HeadlessRunner, serde_json::Value) {
    let evidence: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/superpowers/research/2026-10-25-generation3-regressions.json"
    ))
    .unwrap();
    let case = evidence["games"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["game"] == id)
        .unwrap()
        .clone();
    let cards = CardData::load_from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/cards.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let deck = |key: &str| {
        case[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_owned())
            .collect()
    };
    let mut r = HeadlessRunner::new(
        deck("deck1"),
        deck("deck2"),
        &cards,
        false,
        false,
        false,
        Some(case["seed"].as_u64().unwrap()),
    )
    .unwrap();
    for (i, s) in case["prefix"]
        .as_array()
        .unwrap()
        .iter()
        .take(count)
        .enumerate()
    {
        let p = s["player"].as_u64().unwrap() as u8;
        let a = s["action"].as_u64().unwrap() as u16;
        assert_eq!(decision_player(&r.game), p, "actor at {i}");
        assert!(
            build_action_mask(&r.game, p)[a as usize] > 0.0,
            "illegal action {a} at {i}"
        );
        r.game.decode_action(a, p);
    }
    (r, case)
}

#[test]
fn generation3_game521_finishes_delay_choices() {
    let (r, _) = replay(521, 19);
    let nested = r
        .game
        .pending_selection
        .as_ref()
        .expect("Delay preserves nested leave replacement");
    assert_eq!(
        nested.kind,
        digimon_engine::selection::SelectionKind::Replacement
    );
    assert_eq!(
        nested.previous_phase,
        digimon_engine::enums::GamePhase::Main
    );
    let mut game = r.game.clone();
    let mut cloned = game.clone();
    for _ in 0..40 {
        let Some(selection) = game.pending_selection.as_ref() else {
            break;
        };
        let p = selection.selecting_player;
        let mask = build_action_mask(&game, p);
        let a = if mask[62] > 0.0 {
            62
        } else {
            selection.valid_action_ids[0]
        };
        game.decode_action(a, p);
        cloned.decode_action(a, p);
        assert_eq!(
            digimon_engine::serialization::to_ui_json(&game),
            digimon_engine::serialization::to_ui_json(&cloned)
        );
    }
    assert!(game.pending_selection.is_none());
    assert_eq!(game.current_phase, digimon_engine::enums::GamePhase::Main);
    assert!(
        build_action_mask(&game, decision_player(&game))
            .iter()
            .filter(|x| **x > 0.0)
            .count()
            > 1
    );
}

#[test]
fn generation3_game858_returns_egg_to_egg_deck() {
    let (r, _) = replay(858, 45);
    assert!(r.game.players[0]
        .hand
        .iter()
        .all(|c| c.card_id(&r.game.card_data) != "EX12-004"));
    assert_eq!(
        r.game.players[0].digitama_deck[0].card_id(&r.game.card_data),
        "EX12-004"
    );
    assert!(
        r.game.pending_selection.is_some(),
        "Darumamon still offers its independent play choice"
    );
}

#[test]
fn generation3_game340_search_continuation_does_not_panic() {
    let (r, c) = replay(340, 111);
    let registry = CardRegistry::from_cards(
        &CardData::load_from_str(
            &std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../data/cards.json"
            ))
            .unwrap(),
        )
        .unwrap(),
    );
    let mut cfg = DeterminizedConfig::default();
    cfg.worlds = 4;
    cfg.search.simulations = 320;
    cfg.search.seed = c["search_seed"].as_u64().unwrap();
    cfg.search.observation_profile = ObservationProfileId::StandardLiteV2;
    let result = determinized_search_batched(
        &registry,
        &r.game,
        decision_player(&r.game),
        &mut UniformEvaluator,
        &cfg,
        4,
    );
    assert_eq!(result.simulations_run, 320);
}

#[test]
#[ignore = "requires the frozen cloud checkpoint supplied by GEN3_MODEL"]
fn generation3_game340_frozen_checkpoint_search() {
    let (r, c) = replay(340, 111);
    let registry = CardRegistry::from_cards(
        &CardData::load_from_str(
            &std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../data/cards.json"
            ))
            .unwrap(),
        )
        .unwrap(),
    );
    let mut cfg = DeterminizedConfig::default();
    cfg.worlds = 4;
    cfg.search.simulations = 320;
    cfg.search.seed = c["search_seed"].as_u64().unwrap();
    cfg.search.observation_profile = ObservationProfileId::StandardLiteV2;
    let mut eval = digimon_engine::inference::BatchedPolicyValueEvaluator::load_for_profile(
        std::path::Path::new(&std::env::var("GEN3_MODEL").unwrap()),
        cfg.search.observation_profile,
    )
    .unwrap();
    let result = determinized_search_batched(
        &registry,
        &r.game,
        decision_player(&r.game),
        &mut eval,
        &cfg,
        4,
    );
    assert_eq!(result.simulations_run, 320);
}
