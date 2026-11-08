//! Knowledge regressions for the actual parked simultaneous-departure route.
//! No new card effect: stage real starter cards, then use the shared typed window.
use crate::{
    card_data::CardData,
    card_source::{CardHandle, CardSource},
    determinize::{beliefs::*, closed::materialize_closed},
    enums::{GamePhase, StackPosition},
    permanent::{Permanent, PermanentHandle},
    rules::Rules,
    trigger_context::TriggerContext,
    Game,
};
use std::sync::Arc;

const DECK: &[&str] = &[
    "ST1-01", "ST1-01", "ST1-01", "ST1-01", "ST1-02", "ST1-02", "ST1-02", "ST1-02",
    "ST1-03", "ST1-03", "ST1-03", "ST1-03", "ST1-04", "ST1-04", "ST1-04", "ST1-04",
    "ST1-05", "ST1-05", "ST1-05", "ST1-05", "ST1-06", "ST1-06", "ST1-06", "ST1-06",
    "ST1-07", "ST1-07", "ST1-07", "ST1-07", "ST1-08", "ST1-08", "ST1-09", "ST1-09",
    "ST1-09", "ST1-09", "ST1-10", "ST1-10", "ST1-10", "ST1-10", "ST1-11", "ST1-11",
    "ST1-12", "ST1-12", "ST1-13", "ST1-13", "ST1-14", "ST1-14", "ST1-14", "ST1-14",
    "ST1-15", "ST1-15", "ST1-16", "ST1-16", "ST1-16", "ST1-16",
];

fn take(game: &mut Game, owner: u8, id: &str) -> CardSource {
    let data = &game.card_data;
    let p = &mut game.players[owner as usize];
    for zone in [&mut p.hand, &mut p.deck, &mut p.security] {
        if let Some(i) = zone.iter().position(|c| c.card_id(data) == id) {
            return zone.remove(i);
        }
    }
    panic!("missing fixture card {id}");
}

struct Fixture {
    game: Game,
    prior: WeightedDeckPrior,
    handles: Vec<PermanentHandle>,
    cards: Vec<(u8, CardHandle, String)>,
    pins: Vec<(u8, CardHandle, String, usize)>,
}

fn fixture(seat: u8, mixed_owners: bool, context: bool) -> Fixture {
    let data = CardData::load_from_str(&std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"), "/../../data/cards.json"
    )).unwrap()).unwrap();
    let deck: Vec<String> = DECK.iter().map(|x| x.to_string()).collect();
    let prior = WeightedDeckPrior::new(PriorSnapshot::new(
        "standard", "grouped-return-knowledge-fixture", deck.iter().cloned().collect(),
        vec![CandidateDeck { cards: deck.clone(), weight: 1.0 }], 0.1, 128,
    ).unwrap()).unwrap();
    let mut game = Game::new(&[deck.clone(), deck], &data, Rules::standard(), Some(92501)).unwrap();
    game.enable_belief_knowledge().unwrap();
    game.start_game();
    game.mulligan_pending.clear();
    game.current_phase = GamePhase::Main;
    game.turn_order = vec![seat, 1 - seat];
    game.turn_player_idx = 0;
    game.memory = 4;
    let mut handles = Vec::new();
    let mut cards = Vec::new();
    for (index, id) in ["ST1-04", "ST1-05", "ST1-06"].into_iter().enumerate() {
        let owner = if mixed_owners && index != 1 { 1 - seat } else { seat };
        let card = take(&mut game, owner, id);
        cards.push((owner, card.handle(), id.to_string()));
        game.players[seat as usize].battle_area.push(Permanent::new(card, 0));
        let handle = PermanentHandle { player: seat, index: index as u8 };
        game.capture_digivolve_host_identity(handle);
        handles.push(handle);
    }
    game.enable_belief_knowledge().unwrap();
    // Unrelated public bottom pins must survive deterministic grouped inserts.
    let mut pins = Vec::new();
    for owner in 0..2 {
        let handle = game.reveal_top_deck(owner, 1)[0];
        let id = game.revealed_cards[0].card_id(&game.card_data).to_string();
        let position = game.players[owner as usize].deck.len();
        assert!(game.return_to_deck_from_reveal(owner, handle, StackPosition::Bottom));
        pins.push((owner, handle, id, position));
    }
    if context {
        let mut captured = TriggerContext::default();
        captured.event_card = Some(cards[0].1).into();
        captured.event_snapshot = Some(Arc::new(game.build_snapshot_for_handle(handles[0]).unwrap()));
        game.current_trigger_context = Some(captured);
    }
    Fixture { game, prior, handles, cards, pins }
}

fn park(f: &mut Fixture, position: StackPosition, include_sources: bool) {
    f.game.return_receipts.push((900, Vec::new()));
    crate::replacement::start_departure_batch(
        &mut f.game, f.handles.clone(), 900, Some(position), include_sources,
    );
    assert!(f.game.pending_selection.is_some(), "actual order decision must park");
    assert!(matches!(f.game.pending_selection_resume.as_ref().unwrap().frames.last(),
        Some(crate::resume::ResumeFrame::OptionalReplacement(_))));
}

fn finish(game: &mut Game, handles: &[PermanentHandle], order: [usize; 3]) {
    for index in order.into_iter().take(2) {
        let h = handles[index];
        let action = crate::action::space::encode_attack(h.player as u16, h.index as u16);
        let selection = game.pending_selection.as_ref().expect("retained ordering prompt");
        let actor = selection.selecting_player;
        assert!(selection.valid_action_ids.contains(&action));
        assert!(crate::build_action_mask(game, actor)[action as usize] > 0.5);
        game.resolve_selection(actor, action).unwrap();
    }
    assert!(game.pending_selection.is_none());
    assert!(game.players.iter().all(|p| p.battle_area.is_empty()));
    assert_eq!(game.return_receipts.iter().find(|(r, _)| *r == 900).unwrap().1.len(), 3);
}

fn check_bottom(context: bool) {
    for seat in 0..2 { for mixed in [false, true] { for cloned in [false, true] {
        for order in [[2, 0, 1], [0, 1, 2]] {
            let mut f = fixture(seat, mixed, context);
            park(&mut f, StackPosition::Bottom, false);
            if cloned { f.game = f.game.clone(); }
            finish(&mut f.game, &f.handles, order);
            f.game.enable_belief_knowledge().unwrap();
            for owner in 0..2 {
                let expected: Vec<_> = order.iter().filter_map(|i| {
                    let (o, handle, _) = &f.cards[*i]; (*o == owner).then_some(*handle)
                }).collect();
                let actual: Vec<_> = f.game.players[owner as usize].deck.iter()
                    .take(expected.len()).map(|c| c.handle()).collect();
                assert_eq!(actual, expected, "explicit bottommost-first order and physical owner routing");
            }
            for drawn in [false, true] {
                if drawn {
                    for owner in 0..2 { assert!(f.game.draw_one_for_player(owner).unwrap()); }
                    f.game.enable_belief_knowledge().unwrap();
                }
                for viewer in 0..2 {
                    let memory = f.game.belief_knowledge(viewer).unwrap();
                    for (owner, handle, id) in &f.cards {
                        let deck = &f.game.players[*owner as usize].deck;
                        let at = deck.iter().position(|c| c.handle() == *handle).unwrap();
                        assert_eq!(memory.known_identity(*owner, handle.0), Some(id.clone()));
                        assert!(memory.known_positions(*owner).contains(&(
                            id.clone(), KnowledgeZone::Deck, deck.len() - at - 1,
                        )), "public grouped bottom position must survive unrelated draws");
                    }
                    for (owner, _, id, position) in &f.pins {
                        assert!(memory.known_positions(*owner).contains(&(
                            id.clone(), KnowledgeZone::Deck, position - usize::from(drawn),
                        )), "deterministic bottom insertion must retain existing pins");
                    }
                    for seed in 0..3 {
                        let world = materialize_closed(&f.game, viewer, memory, &f.prior, seed)
                            .expect("public return history and retained snapshot must audit");
                        for (owner, handle, id) in &f.cards {
                            let actual = f.game.players[*owner as usize].deck.iter()
                                .position(|c| c.handle() == *handle).unwrap();
                            let sampled = &world.players[*owner as usize].deck[actual];
                            assert_eq!(sampled.handle(), *handle);
                            assert_eq!(sampled.card_id(&world.card_data), id);
                        }
                    }
                }
            }
            for owner in 0..2 { f.game.shuffle_deck(owner); }
            for viewer in 0..2 { for owner in 0..2 {
                assert!(f.game.belief_knowledge(viewer).unwrap().known_positions(owner)
                    .iter().all(|(_, zone, _)| *zone != KnowledgeZone::Deck));
            } }
        }
    } } }
}

#[test]
fn grouped_public_bottom_positions_survive_draws_and_closed_worlds() { check_bottom(false); }

#[test]
fn grouped_public_bottom_retains_active_departure_snapshot_after_draw() { check_bottom(true); }

#[test]
fn grouped_top_random_and_whole_stack_keep_conservative_position_contract() {
    for seat in 0..2 { for cloned in [false, true] {
        for (position, stack) in [(StackPosition::Top, false), (StackPosition::Random, false),
            (StackPosition::Bottom, true)] {
            let mut f = fixture(seat, false, false);
            park(&mut f, position, stack);
            if cloned { f.game = f.game.clone(); }
            finish(&mut f.game, &f.handles, [2, 0, 1]);
            f.game.enable_belief_knowledge().unwrap();
            for viewer in 0..2 {
                let memory = f.game.belief_knowledge(viewer).unwrap();
                assert!(memory.known_positions(seat).iter()
                    .all(|(_, zone, _)| *zone != KnowledgeZone::Deck));
                for (owner, handle, id) in &f.cards {
                    assert_eq!(memory.known_identity(*owner, handle.0), Some(id.clone()));
                }
            }
        }
    } }
}
