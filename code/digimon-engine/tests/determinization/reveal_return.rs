use super::*;
use digimon_engine::enums::{GamePhase, StackPosition};

#[test]
fn public_reveal_bottom_survives_unrelated_draw_and_closed_sampling() {
    with_big_stack(|| {
        for owner in 0..2 {
            for cloned in [false, true] {
                let mut game = new_game_with(912, D1, D1);
                game.enable_belief_knowledge().unwrap();
                game.start_game();
                game.current_phase = GamePhase::Main;
                let handles = game.reveal_top_deck(owner, 1);
                let handle = handles[0];
                let id = game.revealed_cards[0].card_id(&game.card_data).to_string();
                let position = game.players[owner as usize].deck.len();
                assert!(position > 1);
                assert!(game.return_to_deck_from_reveal(owner, handle, StackPosition::Bottom));
                let mut game = if cloned { game.clone() } else { game };
                game.enable_belief_knowledge().unwrap();
                for viewer in 0..2 {
                    assert!(game.belief_knowledge(viewer).unwrap().known_positions(owner)
                        .contains(&(id.clone(), KnowledgeZone::Deck, position)),
                        "public bottom position must be remembered before drawing");
                }
                assert!(game.draw_one_for_player(owner).unwrap());
                game.enable_belief_knowledge().unwrap();
                for viewer in 0..2 {
                    let memory = game.belief_knowledge(viewer).unwrap();
                    assert!(memory.known_positions(owner)
                        .contains(&(id.clone(), KnowledgeZone::Deck, position - 1)));
                    for seed in 0..3 {
                        let world = materialize_closed(&game, viewer, memory, &prior(), seed).unwrap();
                        let bottom = world.players[owner as usize].deck.first().unwrap();
                        assert_eq!(bottom.handle(), handle);
                        assert_eq!(bottom.card_id(&world.card_data), id);
                    }
                }
                game.shuffle_deck(owner);
                for viewer in 0..2 {
                    assert!(game.belief_knowledge(viewer).unwrap().known_positions(owner)
                        .iter().all(|(_, zone, _)| *zone != KnowledgeZone::Deck));
                }
            }
        }
    });
}

#[test]
fn random_reveal_return_does_not_disclose_the_sampled_insertion_position() {
    with_big_stack(|| {
        for owner in 0..2 {
            for cloned in [false, true] {
                let mut game = new_game_with(913, D1, D1);
                game.enable_belief_knowledge().unwrap();
                game.start_game();
                game.current_phase = GamePhase::Main;
                let handle = game.reveal_top_deck(owner, 1)[0];
                assert!(game.return_to_deck_from_reveal(owner, handle, StackPosition::Random));
                let mut game = if cloned { game.clone() } else { game };
                game.enable_belief_knowledge().unwrap();
                for viewer in 0..2 {
                    assert!(game.belief_knowledge(viewer).unwrap().known_positions(owner)
                        .iter().all(|(_, zone, _)| *zone != KnowledgeZone::Deck));
                    materialize_closed(&game, viewer, game.belief_knowledge(viewer).unwrap(),
                        &prior(), 7).unwrap();
                }
            }
        }
    });
}
