//! DCGO replay: the order of cards returned to the deck bottom.
//!
//! AMATERASUMON (EX12-047)'s [On Play][When Digivolving] returns 2 cards from
//! the opponent's trash to the bottom of their deck. DCGO writes TWO
//! `SelectCardEffect` rows for it: the pick (EX12_047.cs:104-130, "Select 2
//! cards to bottom deck"), then - because 2 cards are placed - the order prompt
//! of `CardEffectCommons.ReturnRevealedCardsToLibraryBottom`
//! (RevealLibrary.cs:469-521: "cards with lower numbers are on top", i.e. the
//! LAST card of that row is the very bottom; `CardObjectController.cs:954-1020`
//! appends each to `LibraryCards`, whose index 0 is the top). CR 4.3 3-1-3-4:
//! the player whose card moves them chooses that order.
//!
//! The engine asks no separate order: `return_trash_list_to_deck_bottom` places
//! the picks in PICK order, the first pick at the very bottom
//! (`EffectContext::return_trash_cards_to_deck_bottom`). The replay must take
//! the order from DCGO's second row - feeding the picks so the engine's bottom
//! order equals DCGO's - and consume both rows; it must never guess it.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::dcgo_recording::{GameEnd, GameStart, RecordingV1, Row, SelectionRow};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::runners::replay::{DivergenceKind, ReplaySession};
use digimon_engine::{Game, TriggerSource};

fn shell_db() -> HashMap<String, CardData> {
    CardData::load_from_str(
        r#"{
        "ST1-03": {
            "card_id": "ST1-03", "card_name_eng": "Koromon",
            "card_effect_class_name": "ST1_03", "play_cost": 3, "dp": 2000,
            "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Lesser"], "form_eng": ["Rookie"], "attribute_eng": ["Free"],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        }
    }"#,
    )
    .unwrap()
}

fn sel(step: u32, actor: u8, extra: serde_json::Value) -> Row {
    let mut v = serde_json::json!({ "step": step, "actor": actor, "phase": "Main" });
    for (k, val) in extra.as_object().unwrap() {
        v[k] = val.clone();
    }
    Row::Selection(serde_json::from_value::<SelectionRow>(v).unwrap())
}

fn session_over(game: Game, rows: Vec<Row>) -> ReplaySession {
    let deck = vec!["ST1-03".to_string(); 50];
    let total = rows.len() as u32;
    let rec = RecordingV1 {
        start: GameStart {
            v: 1,
            game_id: "bottom-order".into(),
            timestamp: "t".into(),
            my_player_id: 0,
            first_player: Some(0),
            is_ai: true,
            my_deck_post_shuffle: deck.clone(),
            opp_deck_post_shuffle: Some(deck),
            opp_decklist_composition: None,
            my_egg_deck: None,
            opp_egg_deck: None,
        },
        rows,
        end: GameEnd { winner: -1, reason: "truncated: test".into(), total_steps: total },
    };
    let mut s = ReplaySession::from_dcgo(rec, &shell_db(), false).expect("recording constructs");
    s.game = game;
    s
}

/// P0's Amaterasumon (EX12-047) has just been played; P1 has two Digimon
/// (WEAK is the lowest DP) and RED / BLUE / FILL in the trash. The On Play is
/// fired and parked at its first prompt.
fn amaterasumon_on_play() -> Game {
    let mut red = make_test_card("RED", "RED");
    red.colors = vec![digimon_engine::CardColor::Red];
    let mut blue = make_test_card("BLUE", "BLUE");
    blue.colors = vec![digimon_engine::CardColor::Blue];
    let mut weak = make_test_card("WEAK", "WEAK");
    weak.dp = Some(1000);
    let mut strong = make_test_card("STRONG", "STRONG");
    strong.dp = Some(9000);
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-047")
        .expect("EX12-047 YAML")
        .add_card(make_test_card("FILL", "FILL"))
        .add_card(red)
        .add_card(blue)
        .add_card(weak)
        .add_card(strong)
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    r.place_on_field(1, "WEAK", Some(0));
    r.place_on_field(1, "STRONG", Some(0));
    for id in ["RED", "FILL", "BLUE"] {
        r.inject_trash(1, id);
    }
    let h = r.place_on_field(0, "EX12-047", None);
    r.game.enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    r.game
}

fn deck_bottom(game: &Game, n: usize) -> Vec<String> {
    game.players[1].deck.iter().take(n).map(|c| c.card_id(&game.card_data).to_string()).collect()
}

fn rows(pick: [&str; 2], order: Option<[&str; 2]>) -> Vec<Row> {
    let mut v = vec![
        // DCGO's "Select 1 Digimon to delete" (canNoSelect false): WEAK at frame 0.
        sel(0, 0, serde_json::json!({"prompt": "SelectPermanentEffect", "targets": [{"player": 1, "frame": 0}]})),
        sel(1, 0, serde_json::json!({"prompt": "SelectCardEffect", "card_ids": pick})),
    ];
    if let Some(order) = order {
        v.push(sel(2, 0, serde_json::json!({"prompt": "SelectCardEffect", "card_ids": order})));
    }
    v
}

/// Step until the row after the order row would run (or a divergence).
fn step_through(s: &mut ReplaySession, until: u32) {
    while s.current_step() < until && !s.is_paused() {
        let before = s.current_step();
        s.step();
        assert!(s.divergences().is_empty(), "row {before}: {:?}", s.divergences());
    }
}

#[test]
fn ex12_047_bottom_order_comes_from_dcgos_second_row() {
    // DCGO's order row [RED, BLUE]: RED on top of BLUE, BLUE the very bottom.
    let mut s = session_over(amaterasumon_on_play(), rows(["RED", "BLUE"], Some(["RED", "BLUE"])));
    step_through(&mut s, 3);
    assert_eq!(s.current_step(), 3, "pick and order rows consumed together");
    assert_eq!(deck_bottom(&s.game, 2), vec!["BLUE", "RED"], "deck index 0 is the bottom");
}

#[test]
fn ex12_047_bottom_order_reversed_by_the_order_row_not_the_pick() {
    // Same pick, opposite order: the pick row's order says nothing.
    let mut s = session_over(amaterasumon_on_play(), rows(["RED", "BLUE"], Some(["BLUE", "RED"])));
    step_through(&mut s, 3);
    assert_eq!(s.current_step(), 3);
    assert_eq!(deck_bottom(&s.game, 2), vec!["RED", "BLUE"]);
}

/// Two cards returned but no order row: DCGO always asks the order of two
/// (RevealLibrary.cs:486), so the recording is missing a decision. The replay
/// stops at the pick instead of guessing the order.
#[test]
fn ex12_047_two_picks_without_an_order_row_is_a_divergence() {
    let mut recorded = rows(["RED", "BLUE"], None);
    recorded.push(sel(2, 1, serde_json::json!({"prompt": "SelectCardEffect", "cancel": true})));
    let mut s = session_over(amaterasumon_on_play(), recorded);
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    s.step();
    match s.divergences() {
        [d] => {
            assert_eq!(d.step, 1, "{d:?}");
            assert!(
                matches!(&d.kind, DivergenceKind::SelectionResolution { reason } if reason.contains("EX12-047")),
                "{d:?}"
            );
        }
        other => panic!("expected one divergence at row 1, got {other:?}"),
    }
    assert!(s.is_paused());
    assert_eq!(deck_bottom(&s.game, 1), vec!["FILL"], "nothing returned");
}
