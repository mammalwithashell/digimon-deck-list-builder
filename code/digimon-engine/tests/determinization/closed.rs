use super::support::*;
use digimon_engine::determinize::{beliefs::*, closed::materialize_closed};
use digimon_engine::search::{
    determinized_search_sampled, DeterminizationMode, DeterminizedConfig, UniformEvaluator,
};

fn prior() -> WeightedDeckPrior {
    let cards: Vec<String> = D1.iter().map(|c| c.to_string()).collect();
    WeightedDeckPrior::new(
        PriorSnapshot::new(
            "standard",
            "starter-counting-fixture",
            cards.iter().cloned().collect(),
            vec![CandidateDeck { cards, weight: 1.0 }],
            0.1,
            128,
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn different_true_compositions_produce_identical_closed_worlds_and_search() {
    with_big_stack(|| {
        let mut a = new_game_with(11, D1, D1);
        a.mulligan_pending = vec![0, 1];
        let mut b = new_game_with(11, D1, D3);
        b.mulligan_pending = vec![0, 1];
        let memory = KnowledgeLedger::new(0).unwrap();
        let p = prior();
        let wa = materialize_closed(&a, 0, &memory, &p, 77).unwrap();
        let wb = materialize_closed(&b, 0, &memory, &p, 77).unwrap();
        assert_eq!(wa.to_scenario(), wb.to_scenario());
        assert!(wb.players[1]
            .hand
            .iter()
            .chain(&wb.players[1].deck)
            .any(|c| c.card_id(&wb.card_data).starts_with("ST1-")));
        assert_eq!(full_zone_dump(&wa), full_zone_dump(&wb));
        let registry = digimon_engine::card_registry::CardRegistry::from_cards(card_data());
        for mode in [DeterminizationMode::Pimc, DeterminizationMode::IsMcts] {
            let mut config = DeterminizedConfig::default();
            config.mode = mode;
            config.worlds = 2;
            config.search.simulations = 4;
            let ra = determinized_search_sampled(
                &registry,
                &a,
                0,
                &mut UniformEvaluator,
                &config,
                1,
                &mut |seed| materialize_closed(&a, 0, &memory, &p, seed),
            )
            .unwrap();
            let rb = determinized_search_sampled(
                &registry,
                &b,
                0,
                &mut UniformEvaluator,
                &config,
                1,
                &mut |seed| materialize_closed(&b, 0, &memory, &p, seed),
            )
            .unwrap();
            assert_eq!(ra, rb);
        }
    });
}

#[test]
fn public_recovery_is_remembered_by_both_seats_and_survives_clone() {
    with_big_stack(|| {
        let mut g = new_game_with(22, D1, D1);
        g.enable_belief_knowledge().unwrap();
        let card = g.players[1].hand.pop().unwrap();
        let handle = card.handle();
        let id = card.card_id(&g.card_data).to_string();
        g.players[1].trash.push(card);
        assert!(g.add_to_hand_from_trash(1, handle));
        for viewer in 0..2 {
            assert!(g
                .belief_knowledge(viewer)
                .unwrap()
                .known_hidden(1)
                .contains(&(id.clone(), KnowledgeZone::Hand)));
        }
        let clone = g.clone();
        assert_eq!(
            g.belief_knowledge(0).unwrap(),
            clone.belief_knowledge(0).unwrap()
        );
        let p = prior();
        let world = materialize_closed(&g, 0, g.belief_knowledge(0).unwrap(), &p, 25).unwrap();
        assert!(world.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&world.card_data) == id));
    });
}

#[test]
fn closed_selfplay_generates_provenanced_shards_on_both_seats() {
    with_big_stack(|| {
        let mut config = digimon_engine::selfplay::SelfPlayConfig::default();
        config.games = 2;
        config.seed = 71;
        config.search.worlds = 2;
        config.search.search.simulations = 4;
        config.belief_prior = Some(prior().snapshot().clone());
        let dir = std::env::temp_dir().join(format!("closed-selfplay-{}", std::process::id()));
        let result = digimon_engine::selfplay::run_self_play(
            card_data(),
            &[D1.iter().map(|c| c.to_string()).collect()],
            &[],
            &mut UniformEvaluator,
            &config,
            &dir,
            "uniform",
        )
        .unwrap();
        assert_eq!(result.games, 2);
        let m: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(m["information"]["regime"], "closed_decklist");
        assert!(m["information"]["prior_hash"].as_str().unwrap().len() == 64);
    });
}

#[test]
fn remembered_own_topdeck_is_pinned_without_revealing_it_to_opponent() {
    with_big_stack(|| {
        let g = new_game_with(55, D1, D1);
        let c = g.players[0].deck.last().unwrap();
        let mut memory = KnowledgeLedger::new(0).unwrap();
        memory
            .observe(
                0,
                c.card_index,
                c.card_id(&g.card_data),
                KnowledgeZone::Deck,
                Some(0),
            )
            .unwrap();
        for seed in 0..8 {
            let w = materialize_closed(&g, 0, &memory, &prior(), seed).unwrap();
            assert_eq!(
                w.players[0].deck.last().unwrap().card_id(&w.card_data),
                c.card_id(&g.card_data)
            );
            digimon_engine::determinize::closed::check_hypothesis(&w).unwrap();
        }
    });
}

#[test]
fn full_public_trash_counts_exceed_the_current_policy_identity_window() {
    with_big_stack(|| {
        let mut a = new_game_with(90, D1, D1);
        a.start_game();
        a.turn_order = vec![0, 1];
        a.turn_player_idx = 0;
        a.current_phase = digimon_engine::enums::GamePhase::Main;
        let p = &mut a.players[1];
        let mut main = std::mem::take(&mut p.hand);
        main.append(&mut p.deck);
        main.append(&mut p.security);
        p.trash = main.drain(..46).collect();
        p.deck = main;
        let mut b = a.clone();
        let different = b.players[1]
            .deck
            .iter()
            .position(|c| c.card_id(&b.card_data) != b.players[1].trash[45].card_id(&b.card_data))
            .unwrap();
        let p = &mut b.players[1];
        std::mem::swap(&mut p.deck[different], &mut p.trash[45]);
        let registry = digimon_engine::card_registry::CardRegistry::from_cards(card_data());
        let profile = digimon_engine::observation::default_observation_profile();
        let oa = digimon_engine::observation::build_observation_tensor(&a, 0, &registry, profile);
        let ob = digimon_engine::observation::build_observation_tensor(&b, 0, &registry, profile);
        assert_eq!(
            oa, ob,
            "this probe documents the current 45-identity policy limitation"
        );
        let memory = KnowledgeLedger::new(0).unwrap();
        assert_ne!(
            BeliefEvidence::extract(&a, 0, &memory).unwrap().accounted,
            BeliefEvidence::extract(&b, 0, &memory).unwrap().accounted
        );
    });
}

#[test]
fn inspected_security_transfer_retains_entitled_identity_and_remaining_positions() {
    with_big_stack(|| {
        use digimon_engine::{effect_context::EffectContext, enums::GamePhase};
        let mut g = new_game_with(29, D1, D1);
        g.enable_belief_knowledge().unwrap();
        g.start_game();
        g.current_phase = GamePhase::Main;
        let source = g.players[0].hand[0].handle();
        EffectContext::new(&mut g, source, None, 0).select_security(
            1,
            "Inspect",
            false,
            |_, _| true,
            |_, _| {},
        );
        let n = g.players[1].security.len();
        g.pending_selection
            .as_mut()
            .unwrap()
            .card_choice
            .as_mut()
            .unwrap()
            .grant_security_inspection(0, &(0..n).collect::<Vec<_>>());
        let card = g.players[1].security[0].clone();
        let id = card.card_id(&g.card_data).to_string();
        // The transfer facade refreshes the live inspection before moving it.
        assert!(g.add_to_hand_from_security(1, card.handle()));
        let memory = g.belief_knowledge(0).unwrap();
        assert!(memory.known_hidden(1).contains(&(id, KnowledgeZone::Hand)));
        assert_eq!(
            memory
                .known_positions(1)
                .iter()
                .filter(|(_, z, _)| *z == KnowledgeZone::Security)
                .count(),
            n - 1
        );
        let restored: KnowledgeLedger =
            serde_json::from_str(&serde_json::to_string(memory).unwrap()).unwrap();
        assert_eq!(restored, *memory);
    });
}

#[test]
fn a_card_absent_from_the_true_list_can_be_legally_played_in_the_hypothesis() {
    with_big_stack(|| {
        let mut g = new_game_with(11, D1, D3);
        g.start_game();
        let mut w =
            materialize_closed(&g, 0, &KnowledgeLedger::new(0).unwrap(), &prior(), 88).unwrap();
        w.mulligan_pending.clear();
        w.current_phase = digimon_engine::enums::GamePhase::Main;
        w.turn_order = vec![1, 0];
        w.turn_player_idx = 0;
        w.memory = 10;
        let mask = digimon_engine::build_action_mask(&w, 1);
        let action = (digimon_engine::action::PLAY_HAND_START
            ..digimon_engine::action::PLAY_HAND_END)
            .find(|a| mask[*a as usize] > 0.5)
            .expect("sampled list has a playable card");
        let card =
            w.players[1].hand[(action - digimon_engine::action::PLAY_HAND_START) as usize].clone();
        assert!(card.card_id(&w.card_data).starts_with("ST1-"));
        assert!(!g.players[1]
            .original_deck
            .iter()
            .any(|r| r.card_id == card.card_id(&w.card_data)));
        w.decode_action(action, 1);
        assert!(w.players[1]
            .hand
            .iter()
            .all(|c| c.handle() != card.handle()));
        digimon_engine::determinize::closed::check_hypothesis(&w).unwrap();
    });
}

#[test]
fn concealed_transforms_fail_explicitly_and_replay_reset_drops_old_knowledge() {
    with_big_stack(|| {
        let mut g = new_game_with(11, D1, D1);
        g.enable_belief_knowledge().unwrap();
        g.players[1].hand[0]
            .also_treated_as
            .push("concealed-transform".into());
        assert!(
            materialize_closed(&g, 0, g.belief_knowledge(0).unwrap(), &prior(), 0)
                .err().unwrap()
                .contains("transformed concealed")
        );
        g.reset_for_replay();
        assert!(g.belief_knowledge(0).is_err());
    });
}
