//! Production-data DNA contracts: YAML-generated fixtures cannot catch ingest drift.
use digimon_engine::{
    action::build_action_mask,
    card_data::CardData,
    debug_runner::{make_test_card_with_level, DebugRunner},
    enums::{CardColor, GamePhase},
    search::decision_player,
};

fn setup(id: &str, first: (u8, CardColor), second: (u8, CardColor)) -> DebugRunner {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/cards.json"
    ))
    .unwrap();
    let mut data = CardData::load_from_str(&raw).unwrap();
    let mut a = make_test_card_with_level("MATERIAL-A", "First material", first.0);
    a.colors = vec![first.1];
    let mut b = make_test_card_with_level("MATERIAL-B", "Second material", second.0);
    b.colors = vec![second.1];
    // Register real effects, then replace generated fixture metadata with the
    // same parsed production data used by self-play and the Python runner.
    let mut r = DebugRunner::builder()
        .dsl_card(id)
        .unwrap()
        .add_card(data.remove(id).unwrap())
        .add_card(a)
        .add_card(b)
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(3)
        .start();
    r.set_first_player(0);
    r.set_phase(GamePhase::Main);
    r.place_on_field(0, "MATERIAL-A", None);
    r.place_on_field(0, "MATERIAL-B", None);
    r.add_to_hand(0, id);
    r
}

#[test]
fn weregarurumon_production_mask_accepts_all_printed_level4_color_pairs() {
    for a in [CardColor::Blue, CardColor::Yellow] {
        for b in [CardColor::Purple, CardColor::Red] {
            for pair in [[a, b], [b, a]] {
                let r = setup("EX12-032", (4, pair[0]), (4, pair[1]));
                assert_eq!(
                    build_action_mask(&r.game, 0)[63],
                    1.0,
                    "printed level-4 pair {pair:?} must offer DNA"
                );
            }
        }
    }
}

#[test]
fn weregarurumon_production_mask_rejects_level5_and_wrong_color_pairs() {
    for (first, second) in [
        ((5, CardColor::Blue), (5, CardColor::Purple)),
        ((4, CardColor::Blue), (5, CardColor::Purple)),
        ((4, CardColor::Blue), (4, CardColor::Yellow)),
    ] {
        let r = setup("EX12-032", first, second);
        assert_eq!(
            build_action_mask(&r.game, 0)[63],
            0.0,
            "unprinted DNA pair {first:?} {second:?} must be rejected"
        );
    }
}

#[test]
fn weregarurumon_production_dna_resolves_and_survives_search_clone() {
    let mut r = setup("EX12-032", (4, CardColor::Blue), (4, CardColor::Purple));
    assert_eq!(build_action_mask(&r.game, 0)[63], 1.0);
    r.game.decode_action(63, 0);
    assert!(
        r.game.pending_selection.is_some(),
        "material choice must be visible"
    );
    let mut clone = r.game.clone();
    for game in [&mut r.game, &mut clone] {
        for _ in 0..8 {
            if game.pending_selection.is_none() {
                break;
            }
            let player = decision_player(game);
            let action = build_action_mask(game, player)
                .iter()
                .enumerate()
                .find(|(a, legal)| **legal > 0.5 && *a != 62 && *a != 93)
                .map(|(a, _)| a as u16)
                .expect("legal non-pass choice");
            game.decode_action(action, player);
        }
        assert!(game.pending_selection.is_none());
        assert_eq!(game.players[0].battle_area.len(), 1);
        let stack = &game.players[0].battle_area[0];
        assert_eq!(stack.top_card().card_id(&game.card_data), "EX12-032");
        assert_eq!(game.memory, 3, "printed DNA cost is zero");
        assert_eq!(game.players[0].hand.len(), 1, "DNA draws one card");
    }
    assert_eq!(r.game.verification_digest(), clone.verification_digest());
}

#[test]
fn other_virus_busters_dna_targets_keep_their_printed_levels() {
    for (id, level, a, b) in [
        ("EX12-035", 5, CardColor::Blue, CardColor::Purple),
        ("EX12-037", 6, CardColor::Blue, CardColor::Red),
        ("EX12-044", 4, CardColor::Yellow, CardColor::Green),
    ] {
        let r = setup(id, (level, a), (level, b));
        assert_eq!(build_action_mask(&r.game, 0)[63], 1.0, "{id}");
        let r = setup(id, (level + 1, a), (level + 1, b));
        assert_eq!(build_action_mask(&r.game, 0)[63], 0.0, "{id}");
    }
}
