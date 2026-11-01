#[path = "../examples/search_quality/mulligan_continuation_root.rs"]
mod root;
#[path = "determinization/support.rs"]
mod support;
use digimon_engine::{card_registry::CardRegistry, game::Game, search::decision_player};
use rand::{RngCore, SeedableRng};

fn saved(game: &Game, registry: &CardRegistry, actions: Vec<u16>) -> root::SavedRoot {
    let actor = decision_player(game);
    root::SavedRoot { id: "test-root".into(), deck_indices: vec![0, 1], game_seed: 41,
        actions, mover: actor, hand: game.players[actor as usize].hand.iter()
            .map(|c| c.card_id(&game.card_data).to_owned()).collect(),
        observation_sha256: root::observation_hash(game, registry, actor) }
}

#[test]
fn both_mulligan_seats_reconstruct_and_force_exact_engine_semantics() {
    support::with_big_stack(|| {
        let registry = CardRegistry::from_cards(support::card_data());
        let decks = vec![support::owned(support::D1), support::owned(support::D2)];
        for prior in [None, Some(0), Some(1)] {
            let mut game = support::new_game(41);
            game.turn_count = 1;
            let prefix: Vec<u16> = prior.into_iter().collect();
            for &action in &prefix { let pid = decision_player(&game); game.decode_action(action, pid); }
            let node = saved(&game, &registry, prefix);
            let restored = root::reconstruct(&node, &decks, support::card_data(), &registry).unwrap();
            assert_eq!(game.verification_digest(), restored.verification_digest());
            for branch in [0, 1] {
                let before = game.verification_digest();
                let mut fork = root::intervene(&game, &registry, &node, branch, 712).unwrap();
                let mut direct = game.clone();
                direct.rng = rand::rngs::StdRng::seed_from_u64(712);
                direct.accept_mulligan(node.mover, branch == 0).unwrap();
                assert_eq!(fork.verification_digest(), direct.verification_digest());
                assert_eq!(fork.rng.next_u64(), direct.rng.next_u64());
                assert_eq!(game.verification_digest(), before);
            }
        }
    });
}

#[test]
fn old_future_rng_never_changes_forced_outcome_and_source_rng_is_preserved() {
    support::with_big_stack(|| {
        let registry = CardRegistry::from_cards(support::card_data());
        let mut game = support::new_game(41); game.turn_count = 1;
        let node = saved(&game, &registry, vec![]);
        let mut changed = game.clone(); changed.rng = rand::rngs::StdRng::seed_from_u64(999);
        for branch in [0, 1] {
            let mut a = root::intervene(&game, &registry, &node, branch, 12).unwrap();
            let mut b = root::intervene(&changed, &registry, &node, branch, 12).unwrap();
            assert_eq!(a.verification_digest(), b.verification_digest());
            assert_eq!(a.rng.next_u64(), b.rng.next_u64());
        }
        let mut expected = support::new_game(41);
        assert_eq!(game.rng.next_u64(), expected.rng.next_u64());
        let a = root::intervene(&changed, &registry, &node, 1, 12).unwrap();
        let b = root::intervene(&changed, &registry, &node, 1, 13).unwrap();
        assert_ne!(a.verification_digest(), b.verification_digest());
    });
}

#[test]
fn corrupt_root_contract_and_illegal_prefix_fail_closed() {
    support::with_big_stack(|| {
        let registry = CardRegistry::from_cards(support::card_data());
        let decks = vec![support::owned(support::D1), support::owned(support::D2)];
        let mut game = support::new_game(41); game.turn_count = 1;
        let node = saved(&game, &registry, vec![]);
        for kind in 0..5 {
            let mut bad = node.clone();
            match kind { 0 => bad.mover = 1 - bad.mover, 1 => bad.hand.reverse(),
                2 => bad.observation_sha256 = "wrong".into(), 3 => bad.actions = vec![2191],
                _ => bad.deck_indices = vec![999, 0] }
            assert!(root::reconstruct(&bad, &decks, support::card_data(), &registry).is_err());
        }
        assert!(root::intervene(&game, &registry, &node, 2, 12).is_err());
    });
}
