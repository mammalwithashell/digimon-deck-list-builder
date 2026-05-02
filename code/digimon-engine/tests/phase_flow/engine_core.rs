use std::collections::HashMap;

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    BREEDING_TARGET, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, HATCH,
    MOVE_FROM_BREEDING, PLAY_HAND_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_registry::CardRegistry;
use digimon_engine::card_source::CardSource;
use digimon_engine::enums::*;
use digimon_engine::game::Game;
use digimon_engine::permanent::Permanent;
use digimon_engine::rules::Rules;

/// Build a minimal card database for testing.
fn test_card_db() -> HashMap<String, CardData> {
    let json = r#"{
        "BT1-001": {
            "card_id": "BT1-001", "card_name_eng": "Koromon",
            "card_effect_class_name": "BT1_001", "play_cost": 0, "dp": -1,
            "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
            "type_eng": ["Lesser"], "form_eng": ["In-Training"], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "BT1-010": {
            "card_id": "BT1-010", "card_name_eng": "Agumon",
            "card_effect_class_name": "BT1_010", "play_cost": 3, "dp": 2000,
            "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Reptile"], "form_eng": ["Rookie"], "attribute_eng": ["Vaccine"],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "BT1-025": {
            "card_id": "BT1-025", "card_name_eng": "Greymon",
            "card_effect_class_name": "BT1_025", "play_cost": 5, "dp": 5000,
            "level": 4, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Dinosaur"], "form_eng": ["Champion"], "attribute_eng": ["Vaccine"],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "",
            "evo_costs": [{"card_color": 0, "level": 3, "memory_cost": 2}]
        },
        "BT1-011": {
            "card_id": "BT1-011", "card_name_eng": "Training Agumon",
            "card_effect_class_name": "BT1_011", "play_cost": 3, "dp": 2000,
            "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Reptile"], "form_eng": ["Rookie"], "attribute_eng": ["Vaccine"],
            "effect_description_eng": "＜Training＞", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "BT1-085": {
            "card_id": "BT1-085", "card_name_eng": "Tai Kamiya",
            "card_effect_class_name": "BT1_085", "play_cost": 2, "dp": -1,
            "level": -1, "card_kind": 1, "rarity": 0, "card_colors": [0],
            "type_eng": ["DigiDestined"], "form_eng": [], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "BT1-093": {
            "card_id": "BT1-093", "card_name_eng": "Gaia Force",
            "card_effect_class_name": "BT1_093", "play_cost": 8, "dp": -1,
            "level": -1, "card_kind": 2, "rarity": 0, "card_colors": [0],
            "type_eng": [], "form_eng": [], "attribute_eng": [],
            "effect_description_eng": "Delete 1 of your opponent's Digimon.",
            "inherited_effect_description_eng": "",
            "security_effect_description_eng": "Delete 1 of your opponent's Digimon.",
            "evo_costs": []
        }
    }"#;
    CardData::load_from_str(json).unwrap()
}

/// Build a test deck (20 main deck + 4 eggs = 24 cards for testing).
/// Large enough for EDH (needs 5 hand + 7 security = 12 from main deck).
fn test_deck() -> Vec<String> {
    let mut deck = Vec::new();
    // 4 eggs
    for _ in 0..4 {
        deck.push("BT1-001".to_string());
    }
    // 6 Agumon
    for _ in 0..6 {
        deck.push("BT1-010".to_string());
    }
    // 6 Greymon
    for _ in 0..6 {
        deck.push("BT1-025".to_string());
    }
    // 4 Tai
    for _ in 0..4 {
        deck.push("BT1-085".to_string());
    }
    // 4 Gaia Force
    for _ in 0..4 {
        deck.push("BT1-093".to_string());
    }
    deck
}

#[test]
fn game_creation_standard() {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::standard();

    let game = Game::new(&[deck.clone(), deck], &db, rules, Some(42)).unwrap();

    assert_eq!(game.players.len(), 2);
    assert_eq!(game.turn_count, 0);
    assert_eq!(game.current_phase, GamePhase::Mulligan);
    assert!(!game.game_over);

    // Opening hands are drawn up-front; security is deferred until mulligan
    // finalizes (see §1.6 parity fix). Hands are 5; security is still 0.
    assert_eq!(game.player(0).hand_size(), 5);
    assert_eq!(game.player(0).security_count(), 0);
    assert_eq!(game.player(1).hand_size(), 5);
    assert_eq!(game.player(1).security_count(), 0);

    // Digitama deck should have 4 eggs
    assert_eq!(game.player(0).digitama_deck.len(), 4);

    // Mulligan state is initialized: both players pending, none have re-drawn.
    assert_eq!(game.mulligan_pending.len(), 2);
    assert_eq!(game.mulligan_used, vec![false, false]);
}

#[test]
fn game_start_and_turn_flow() {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::standard();

    let mut game = Game::new(&[deck.clone(), deck], &db, rules, Some(42)).unwrap();
    game.start_game();

    // After start_game: turn 1; whichever player the coin flip picked is active.
    // Don't assume a specific player — the first-player coin flip randomizes.
    assert_eq!(game.turn_count, 1);
    let first_player = game.turn_player();
    // First player's turn-1 draw is skipped, so their hand is still 5.
    assert_eq!(game.player(first_player).hand_size(), 5);
    // Security was laid during finalize_mulligan (auto-kept by start_game).
    assert_eq!(game.player(0).security_count(), 5);
    assert_eq!(game.player(1).security_count(), 5);
    // Phase should be Breeding (after Unsuspend + Draw).
    assert_eq!(game.current_phase, GamePhase::Breeding);
}

#[test]
fn breeding_hatch_and_move() {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::standard();

    let mut game = Game::new(&[deck.clone(), deck], &db, rules, Some(42)).unwrap();
    game.start_game();

    let tp = game.turn_player();

    // Hatch: move egg from digitama deck to breeding area
    assert!(game.player(tp).breeding_area.is_none());
    let initial_eggs = game.player(tp).digitama_deck.len();
    assert!(game.hatch(tp));
    assert!(game.player(tp).breeding_area.is_some());
    assert_eq!(game.player(tp).digitama_deck.len(), initial_eggs - 1);

    // Can't hatch again (breeding area occupied)
    assert!(!game.hatch(tp));

    // Can't move a level-2 egg directly to the battle area.
    assert!(!game.move_from_breeding(tp));
    assert!(game.player(tp).breeding_area.is_some());
    assert_eq!(game.player(tp).field_count(), 0);

    // Move a level-3 permanent from breeding to battle area.
    let agumon_idx = game
        .card_data
        .iter()
        .position(|card| card.card_id == "BT1-010")
        .expect("Agumon in test card DB");
    game.player_mut(tp).breeding_area = Some(Permanent::new(
        CardSource::new(agumon_idx, tp, 999),
        game.turn_count,
    ));
    assert!(game.move_from_breeding(tp));
    assert!(game.player(tp).breeding_area.is_none());
    assert_eq!(game.player(tp).field_count(), 1);

    // Verify the permanent is the level-3 Digimon.
    let perm = &game.player(tp).battle_area[0];
    assert_eq!(perm.level(&game.card_data), Some(3));
}

#[test]
fn decode_hatch_advances_to_main_phase() {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::standard();

    let mut game = Game::new(&[deck.clone(), deck], &db, rules, Some(42)).unwrap();
    game.start_game();

    assert_eq!(game.current_phase, GamePhase::Breeding);
    game.decode_action(HATCH, game.turn_player());

    assert_eq!(game.current_phase, GamePhase::Main);
}

#[test]
fn decode_move_from_breeding_advances_to_main_phase() {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::standard();

    let mut game = Game::new(&[deck.clone(), deck], &db, rules, Some(42)).unwrap();
    game.start_game();

    let tp = game.turn_player();
    let agumon_idx = game
        .card_data
        .iter()
        .position(|card| card.card_id == "BT1-010")
        .expect("Agumon in test card DB");
    game.player_mut(tp).breeding_area = Some(Permanent::new(
        CardSource::new(agumon_idx, tp, 999),
        game.turn_count,
    ));

    assert_eq!(game.current_phase, GamePhase::Breeding);
    game.decode_action(MOVE_FROM_BREEDING, tp);

    assert_eq!(game.current_phase, GamePhase::Main);
}

#[test]
fn decode_move_from_breeding_rejects_level_2_and_stays_in_breeding_phase() {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::standard();

    let mut game = Game::new(&[deck.clone(), deck], &db, rules, Some(42)).unwrap();
    game.start_game();

    let tp = game.turn_player();
    let egg_idx = game
        .card_data
        .iter()
        .position(|card| card.card_id == "BT1-001")
        .expect("Koromon in test card DB");
    game.player_mut(tp).breeding_area = Some(Permanent::new(
        CardSource::new(egg_idx, tp, 998),
        game.turn_count,
    ));

    assert_eq!(game.current_phase, GamePhase::Breeding);
    game.decode_action(MOVE_FROM_BREEDING, tp);

    assert_eq!(
        game.current_phase,
        GamePhase::Breeding,
        "illegal level-2 move must not advance as though the move resolved",
    );
    assert!(
        game.player(tp).breeding_area.is_some(),
        "level-2 permanent must remain in breeding",
    );
    assert_eq!(
        game.player(tp).field_count(),
        0,
        "level-2 permanent must not enter the battle area",
    );
}

#[test]
fn main_phase_play_mask_respects_full_field_but_keeps_options() {
    let db = test_card_db();
    let deck = test_deck();
    let mut rules = Rules::standard();
    rules.field_slots = 1;

    let mut game = Game::new(&[deck.clone(), deck], &db, rules, Some(42)).unwrap();
    game.start_game();
    game.enter_main_phase();

    let tp = game.turn_player();
    let agumon_idx = game
        .card_data
        .iter()
        .position(|card| card.card_id == "BT1-010")
        .expect("Agumon in test card DB");
    let tai_idx = game
        .card_data
        .iter()
        .position(|card| card.card_id == "BT1-085")
        .expect("Tai in test card DB");
    let option_idx = game
        .card_data
        .iter()
        .position(|card| card.card_id == "BT1-093")
        .expect("Gaia Force in test card DB");

    game.player_mut(tp).battle_area = vec![Permanent::new(
        CardSource::new(agumon_idx, tp, 900),
        game.turn_count,
    )];
    game.player_mut(tp).hand = vec![
        CardSource::new(agumon_idx, tp, 901),
        CardSource::new(tai_idx, tp, 902),
        CardSource::new(option_idx, tp, 903),
    ];
    game.set_memory(10);

    let mask = build_action_mask(&game, tp);

    assert_eq!(
        mask[(PLAY_HAND_START + 0) as usize],
        0.0,
        "Digimon hand play must be masked off when battle area is full",
    );
    assert_eq!(
        mask[(PLAY_HAND_START + 1) as usize],
        0.0,
        "Tamer hand play must be masked off when battle area is full",
    );
    assert_eq!(
        mask[(PLAY_HAND_START + 2) as usize],
        1.0,
        "Option use does not add a battle-area permanent and can remain legal",
    );
}

#[test]
fn full_field_masks_breeding_move_and_skips_breeding_phase() {
    let db = test_card_db();
    let deck = test_deck();
    let mut rules = Rules::standard();
    rules.field_slots = 1;

    let mut game = Game::new(&[deck.clone(), deck], &db, rules, Some(42)).unwrap();
    game.start_game();

    let tp = game.turn_player();
    let next = game.next_clockwise(tp);
    let agumon_idx = game
        .card_data
        .iter()
        .position(|card| card.card_id == "BT1-010")
        .expect("Agumon in test card DB");

    game.player_mut(tp).breeding_area = Some(Permanent::new(
        CardSource::new(agumon_idx, tp, 910),
        game.turn_count,
    ));
    game.player_mut(tp).battle_area = vec![Permanent::new(
        CardSource::new(agumon_idx, tp, 911),
        game.turn_count,
    )];

    let mask = build_action_mask(&game, tp);
    assert_eq!(
        mask[MOVE_FROM_BREEDING as usize], 0.0,
        "MOVE_FROM_BREEDING must be masked off when battle area is full",
    );

    game.player_mut(next).breeding_area = Some(Permanent::new(
        CardSource::new(agumon_idx, next, 912),
        game.turn_count,
    ));
    game.player_mut(next).battle_area = vec![Permanent::new(
        CardSource::new(agumon_idx, next, 913),
        game.turn_count,
    )];

    game.enter_main_phase();
    game.set_memory(0);
    game.pass_turn();

    assert_eq!(game.turn_player(), next);
    assert_eq!(
        game.current_phase,
        GamePhase::Main,
        "full field means the next player's level >= 3 breeding permanent is not actionable",
    );
}

#[test]
fn training_carrier_does_not_keep_turn_in_breeding_when_only_training_is_available() {
    let db = test_card_db();
    let deck = test_deck();
    let mut rules = Rules::standard();
    rules.field_slots = 1;

    let mut game = Game::new(&[deck.clone(), deck], &db, rules, Some(42)).unwrap();
    game.start_game();

    let tp = game.turn_player();
    let next = game.next_clockwise(tp);
    let agumon_idx = game
        .card_data
        .iter()
        .position(|card| card.card_id == "BT1-010")
        .expect("Agumon in test card DB");
    let training_idx = game
        .card_data
        .iter()
        .position(|card| card.card_id == "BT1-011")
        .expect("Training Agumon in test card DB");

    game.player_mut(next).breeding_area = Some(Permanent::new(
        CardSource::new(training_idx, next, 920),
        game.turn_count,
    ));
    game.player_mut(next).battle_area = vec![Permanent::new(
        CardSource::new(agumon_idx, next, 921),
        game.turn_count,
    )];

    game.enter_main_phase();
    game.set_memory(0);
    game.pass_turn();

    assert_eq!(game.turn_player(), next);
    assert_eq!(
        game.current_phase,
        GamePhase::Main,
        "Training from breeding is surfaced through the Main-phase mask, so it must not park the turn in Breeding"
    );

    let mask = build_action_mask(&game, next);
    let breeding_training_bit =
        FIELD_EFFECT_START + BREEDING_TARGET * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN;
    assert_eq!(
        mask[breeding_training_bit as usize], 1.0,
        "Main-phase mask should surface the breeding-area Training activation"
    );
    assert_eq!(
        mask[MOVE_FROM_BREEDING as usize], 0.0,
        "full field keeps move unavailable; Training is the only relevant action"
    );
}

#[test]
fn memory_system() {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::standard();

    let mut game = Game::new(&[deck.clone(), deck], &db, rules, Some(42)).unwrap();
    game.start_game();

    game.enter_main_phase();

    // Set memory to a known value for clean testing
    game.set_memory(5);
    assert_eq!(game.memory, 5);

    // Pay cost that doesn't cross 0
    assert!(game.pay_memory(3));
    assert_eq!(game.memory, 2);
    assert!(!game.game_over);

    // Check memory clamping
    game.set_memory(10);
    game.gain_memory(5);
    assert_eq!(game.memory, 10); // clamped to max
}

#[test]
fn pass_turn_advances_player() {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::standard();

    let mut game = Game::new(&[deck.clone(), deck], &db, rules, Some(42)).unwrap();
    game.start_game();
    game.enter_main_phase();

    // Coin flip randomizes the first player — capture it rather than assuming P0.
    let first_player = game.turn_player();
    let second_player = if first_player == 0 { 1 } else { 0 };
    let initial_turn = game.turn_count;

    game.pass_turn();

    // Turn should advance to the other player.
    assert_eq!(game.turn_player(), second_player);
    assert_eq!(game.turn_count, initial_turn + 1);
    // The second player draws on their first turn (only the first player
    // skips turn-1 draw). Hand should be 6 (5 opening + 1 draw).
    assert_eq!(game.player(second_player).hand_size(), 6);
}

#[test]
fn play_card_from_hand() {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::standard();

    let mut game = Game::new(&[deck.clone(), deck], &db, rules, Some(42)).unwrap();
    game.start_game();
    game.enter_main_phase();

    let tp = game.turn_player();
    let initial_hand = game.player(tp).hand_size();

    // Play the first card from hand
    let result = game.play_from_hand(tp, 0);
    assert!(result.is_some());
    assert_eq!(game.player(tp).hand_size(), initial_hand - 1);
    assert_eq!(game.player(tp).field_count(), 1);
}

#[test]
fn opponents_and_turn_order() {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::standard();

    let game = Game::new(&[deck.clone(), deck], &db, rules, Some(42)).unwrap();

    // Standard: P0's opponent is P1
    let opps = game.opponents(0);
    assert_eq!(opps, vec![1]);

    // Next clockwise from P0 is P1
    assert_eq!(game.next_clockwise(0), 1);
    assert_eq!(game.next_clockwise(1), 0);
}

#[test]
fn card_registry_with_real_db() {
    let db = test_card_db();
    let reg = CardRegistry::from_cards(&db);

    assert_eq!(reg.count(), 6);
    // All 6 test cards should have non-zero indices
    assert!(reg.get_index("BT1-001") > 0);
    assert!(reg.get_index("BT1-010") > 0);
    assert!(reg.get_index("BT1-011") > 0);
    assert!(reg.get_index("BT1-025") > 0);
    assert!(reg.get_index("BT1-085") > 0);
    assert!(reg.get_index("BT1-093") > 0);
}

#[test]
fn card_data_parsing() {
    let db = test_card_db();

    let agumon = &db["BT1-010"];
    assert_eq!(agumon.card_name, "Agumon");
    assert_eq!(agumon.card_kind, CardKind::Digimon);
    assert_eq!(agumon.level, Some(3));
    assert_eq!(agumon.dp, Some(2000));
    assert_eq!(agumon.play_cost, 3);
    assert_eq!(agumon.colors, vec![CardColor::Red]);

    let gaia = &db["BT1-093"];
    assert_eq!(gaia.card_kind, CardKind::Option);
    assert_eq!(gaia.level, None); // -1 -> None
    assert_eq!(gaia.dp, None); // -1 -> None
}

#[test]
fn permanent_digivolution() {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::standard();

    let mut game = Game::new(&[deck.clone(), deck], &db, rules, Some(42)).unwrap();
    game.start_game();
    game.enter_main_phase();

    let tp = game.turn_player();

    // Play a Lv3 from hand
    game.play_from_hand(tp, 0);

    // Manually construct a Greymon CardSource and digivolve
    let greymon_data_idx = game
        .card_data
        .iter()
        .position(|d| d.card_id == "BT1-025")
        .unwrap();
    let card_idx = game.next_card_index();
    let greymon_card = digimon_engine::CardSource::new(greymon_data_idx, tp, card_idx);

    game.digivolve_onto(tp, 0, greymon_card);

    let perm = &game.player(tp).battle_area[0];
    assert_eq!(perm.stack_size(), 2);
    assert_eq!(perm.level(&game.card_data), Some(4));
    assert_eq!(perm.base_dp(&game.card_data), Some(5000));
    assert!(perm.is_digimon(&game.card_data));
}

#[test]
fn player_delete_permanent() {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::standard();

    let mut game = Game::new(&[deck.clone(), deck], &db, rules, Some(42)).unwrap();
    game.start_game();
    game.enter_main_phase();

    let tp = game.turn_player();
    let initial_trash = game.player(tp).trash.len();

    // Play a card
    game.play_from_hand(tp, 0);
    assert_eq!(game.player(tp).field_count(), 1);

    // Delete it
    game.player_mut(tp).delete_permanent(0);
    assert_eq!(game.player(tp).field_count(), 0);
    assert_eq!(game.player(tp).trash.len(), initial_trash + 1);
}

#[test]
fn action_space_constants() {
    use digimon_engine::action::*;

    assert_eq!(ACTION_SPACE_SIZE, 2168);
    assert_eq!(PASS, 62);
    assert_eq!(HATCH, 60);
    assert_eq!(MOVE_FROM_BREEDING, 61);

    // Verify ranges don't overlap
    assert!(PLAY_HAND_END <= HAND_EFFECT_START);
    assert!(HAND_EFFECT_END <= HATCH);
    assert!(DNA_DIGIVOLVE_END <= ATTACK_START);
    assert!(ATTACK_END <= DIGIVOLVE_END); // Attack end = Digivolve start = 400
    assert!(DIGIVOLVE_END <= FIELD_EFFECT_END);
    assert!(TRASH_EFFECT_END <= SOURCE_SELECT_START);
    assert!(SOURCE_SELECT_END as usize <= ACTION_SPACE_SIZE);
}

#[test]
fn edh_game_creation() {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::edh();

    let mut game = Game::new(
        &[deck.clone(), deck.clone(), deck.clone(), deck],
        &db,
        rules,
        Some(42),
    )
    .unwrap();

    assert_eq!(game.players.len(), 4);
    assert_eq!(game.turn_order.len(), 4);
    // All 4 players are pending mulligan decisions.
    assert_eq!(game.mulligan_pending.len(), 4);

    // Security is laid only after mulligan finalizes.
    game.start_game();
    assert_eq!(game.player(0).security_count(), 7);
    assert_eq!(game.player(3).security_count(), 7);

    // P0's opponents should be [1, 2, 3]
    let opps = game.opponents(0);
    assert_eq!(opps.len(), 3);
    assert!(opps.contains(&1));
    assert!(opps.contains(&2));
    assert!(opps.contains(&3));
}

#[test]
fn edh_all_skip_draw_round1() {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::edh();

    let mut game = Game::new(
        &[deck.clone(), deck.clone(), deck.clone(), deck],
        &db,
        rules,
        Some(42),
    )
    .unwrap();
    game.start_game();

    // P0 should NOT have drawn (round 1, all skip)
    assert_eq!(game.player(0).hand_size(), 5);

    // Pass to P1
    game.enter_main_phase();
    game.pass_turn();

    // P1 should also NOT have drawn (still round 1)
    assert_eq!(game.player(1).hand_size(), 5);
}

#[test]
fn elimination_removes_from_turn_order() {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::edh();

    let mut game = Game::new(
        &[deck.clone(), deck.clone(), deck.clone(), deck],
        &db,
        rules,
        Some(42),
    )
    .unwrap();

    game.eliminate_player(2);

    assert!(game.player(2).is_eliminated);
    assert_eq!(game.turn_order.len(), 3);
    assert!(!game.turn_order.contains(&2));
    assert!(!game.game_over); // 3 players remain

    game.eliminate_player(1);
    game.eliminate_player(3);

    // Only P0 remains — game over
    assert!(game.game_over);
    assert_eq!(game.winner, Some(0));
}
