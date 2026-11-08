//! A public hand-to-source move must not erase unrelated entitled hand memory.
//! These fixtures exercise zone knowledge only; no model/scored trajectory runs.
#[path = "determinization/support.rs"]
mod support;
use support::{new_game_with, with_big_stack, D1};
use digimon_engine::{
    card_source::CardHandle,
    determinize::beliefs::KnowledgeZone,
    enums::CardSourceRef,
    game::Game,
    permanent::{Permanent, PermanentHandle},
};

fn fixture(owner: u8) -> (Game, CardHandle, CardHandle, usize, PermanentHandle) {
    let mut game = new_game_with(902, D1, D1);
    let take = |game: &mut Game, id: &str| {
        let i = game.players[owner as usize].deck.iter()
            .position(|c| c.card_id(&game.card_data) == id).unwrap();
        game.players[owner as usize].deck.remove(i)
    };
    let host = take(&mut game, "ST1-03");
    game.players[owner as usize].battle_area.push(Permanent::new(host, 0));
    let known = take(&mut game, "ST1-02");
    let known_handle = known.handle();
    game.players[owner as usize].trash.push(known);
    let placed = take(&mut game, "ST1-04");
    let placed_handle = placed.handle();
    game.players[owner as usize].hand.push(placed);
    game.enable_belief_knowledge().unwrap();
    assert!(game.add_to_hand_from_trash(owner, known_handle));
    game.enable_belief_knowledge().unwrap();
    assert!(game.belief_knowledge(1-owner).unwrap().known_hidden(owner)
        .contains(&("ST1-02".into(), KnowledgeZone::Hand)));
    let index = game.players[owner as usize].hand.iter()
        .position(|c| c.handle() == placed_handle).unwrap();
    (game, known_handle, placed_handle, index, PermanentHandle { player: owner, index: 0 })
}

fn public_source_move(top: bool) {
    for owner in 0..2 {
        let (mut game, known, placed, index, host) = fixture(owner);
        let source = CardSourceRef::Hand(owner, index);
        assert!(if top { game.place_as_top_source(source, host, false) }
            else { game.place_as_bottom_source(source, host, false) });
        game.enable_belief_knowledge().unwrap();
        let memory = game.belief_knowledge(1-owner).unwrap();
        assert!(memory.known_hidden(owner).contains(&("ST1-02".into(), KnowledgeZone::Hand)),
            "public departure of a different identity must preserve the remembered recovery");
        assert!(serde_json::to_value(memory).unwrap()["cards"].as_array().unwrap().iter()
            .any(|c| c["instance"].as_u64() == Some(known.0 as u64)));
        assert!(game.players[owner as usize].battle_area[0].card_sources.iter()
            .any(|c| c.handle() == placed && !c.face_down));
        assert_eq!(memory, game.clone().belief_knowledge(1-owner).unwrap());
    }
}

#[test]
fn public_bottom_source_preserves_other_recovered_hand_identity() {
    with_big_stack(|| public_source_move(false));
}
#[test]
fn public_top_source_preserves_other_recovered_hand_identity() {
    with_big_stack(|| public_source_move(true));
}
#[test]
fn face_down_source_does_not_disclose_departing_hand_identity() {
    with_big_stack(|| {
        for owner in 0..2 {
            let (mut game, _, placed, index, host) = fixture(owner);
            assert!(game.place_as_bottom_source(CardSourceRef::Hand(owner, index), host, true));
            game.enable_belief_knowledge().unwrap();
            assert!(!serde_json::to_value(game.belief_knowledge(1-owner).unwrap()).unwrap()["cards"]
                .as_array().unwrap().iter().any(|c| c["instance"].as_u64() == Some(placed.0 as u64)));
            assert!(game.players[owner as usize].battle_area[0].card_sources.iter()
                .any(|c| c.handle() == placed && c.face_down));
        }
    });
}
#[test]
fn invalid_source_destination_preserves_hand_and_knowledge() {
    with_big_stack(|| {
        let (mut game, _, _, index, mut host) = fixture(0);
        let memory = game.belief_knowledge(1).unwrap().clone();
        let hand = game.players[0].hand.clone();
        host.index = 99;
        assert!(!game.place_as_bottom_source(CardSourceRef::Hand(0, index), host, false));
        game.enable_belief_knowledge().unwrap();
        assert_eq!(game.players[0].hand.iter().map(|c| c.handle()).collect::<Vec<_>>(),
            hand.iter().map(|c| c.handle()).collect::<Vec<_>>());
        assert_eq!(game.belief_knowledge(1).unwrap(), &memory);
    });
}
