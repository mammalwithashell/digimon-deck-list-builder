//! DCGO replay: SIRIUSMON (EX12-018)'s "place up to 2 ... from your hand or
//! trash" zone loop.
//!
//! DCGO (`CardEffect/EX12/Red/EX12_018.cs:84-160`) asks in a LOOP while fewer
//! than 2 cards are chosen: the int menu "Where will you select cards from ...?"
//! (From Hand = 1, From Trash = 2, Don't place any = 3), then a pick of up to
//! `2 - chosen` cards in that zone (`SelectHandEffect` / `SelectCardEffect`,
//! `canNoSelect`, `canEndNotMax`). After the loop, if any card was chosen, the
//! bool "Top of digivolution sources" (true) / "Bottom ..." (false). One card
//! from the hand is therefore recorded as: menu 1, hand pick, menu again.
//!
//! The engine asks the zone ONCE (an `EffectChoice`), then ONE up-to-2 pick in
//! that zone, then top/bottom. The replay closed that pick after DCGO's first
//! card (a trailing PASS) and read DCGO's second menu as the top/bottom
//! answer. These tests pin the fold: further picks in the same zone feed the
//! same engine pick, menu 3 closes it, and a pick from the OTHER zone - legal
//! in DCGO and by the printed text ("from your hand or trash"), not expressible
//! in the engine's one-zone pick - is an honest divergence.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::dcgo_recording::{GameEnd, GameStart, RecordingV1, Row, SelectionRow};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::runners::replay::{DivergenceKind, ReplaySession};
use digimon_engine::{CardColor, Game, TriggerSource};

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

fn sel(step: u32, extra: serde_json::Value) -> Row {
    let mut v = serde_json::json!({ "step": step, "actor": 0, "phase": "Main" });
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
            game_id: "zone-menu-loop".into(),
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

fn vb(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Red];
    c.level = Some(4);
    c.traits = vec!["VB".into()];
    c
}

/// Siriusmon (EX12-018) on top of a Lv.5 VB Digimon, [When Digivolving]
/// fired; VB-A and VB-B in hand, VB-T in trash; the opponent has no Digimon
/// (no -DP target). Parked at the engine's zone menu.
fn sirius_when_digivolving() -> Game {
    let mut base = vb("BASE");
    base.level = Some(5);
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-018")
        .expect("EX12-018 YAML")
        .add_card(base)
        .add_card(vb("VB-A"))
        .add_card(vb("VB-B"))
        .add_card(vb("VB-T"))
        .add_card(make_test_card("FILL", "FILL"))
        .hand(0, &["VB-A", "VB-B"])
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .memory(10)
        .start();
    r.inject_trash(0, "VB-T");
    let h = r.place_stack(0, &["BASE", "EX12-018"]);
    r.game.enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    assert!(r.game.pending_selection.is_some(), "the zone menu is parked");
    r.game
}

fn menu(step: u32, v: i64) -> Row {
    sel(step, serde_json::json!({"prompt": "generic_int", "int_value": v}))
}
fn hand_pick(step: u32, id: &str) -> Row {
    sel(step, serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": [id]}))
}
fn top_bottom(step: u32, top: bool) -> Row {
    sel(step, serde_json::json!({"prompt": "generic_bool", "bool_value": top}))
}

fn sources(game: &Game) -> Vec<String> {
    game.players[0].battle_area[0]
        .card_sources
        .iter()
        .map(|c| c.card_id(&game.card_data).to_string())
        .collect()
}

fn replay_all(s: &mut ReplaySession) {
    while s.current_step() < s.total_steps() {
        let before = s.current_step();
        s.step();
        assert!(
            s.divergences().is_empty() && !s.is_paused(),
            "row {before} diverged: {:?}",
            s.divergences()
        );
        assert!(s.current_step() > before, "row {before} did not advance");
    }
}

fn halted_at(s: &ReplaySession, at: u32) -> String {
    let reason = match s.divergences() {
        [d] => {
            assert_eq!(d.step, at, "{d:?}");
            match &d.kind {
                DivergenceKind::SelectionResolution { reason } => reason.clone(),
                other => panic!("{other:?}"),
            }
        }
        other => panic!("expected one divergence at row {at}, got {other:?}"),
    };
    assert!(s.is_paused());
    assert_eq!(s.current_step(), at);
    reason
}

/// Menu 1, VB-A, menu 1, VB-B, Bottom: both hand cards go under Siriusmon as
/// bottom sources (the g4 Virus Busters vs Toho Braves shape, rows 28-32).
#[test]
fn ex12_018_two_hand_picks_across_the_menu_loop_place_both_at_the_bottom() {
    let mut s = session_over(
        sirius_when_digivolving(),
        vec![menu(0, 1), hand_pick(1, "VB-A"), menu(2, 1), hand_pick(3, "VB-B"), top_bottom(4, false)],
    );
    replay_all(&mut s);
    let src = sources(&s.game);
    assert_eq!(src.last().map(String::as_str), Some("EX12-018"), "{src:?}");
    assert_eq!(src.len(), 4, "{src:?}");
    let mut placed: Vec<_> = src[..2].to_vec();
    placed.sort();
    assert_eq!(placed, vec!["VB-A", "VB-B"], "both placed at the BOTTOM: {src:?}");
    assert_eq!(src[2], "BASE", "{src:?}");
    assert!(s.game.players[0].hand.is_empty());
}

/// Menu 1, VB-A, then "Don't place any" (3) ends DCGO's loop with one card;
/// the top/bottom bool follows.
#[test]
fn ex12_018_one_hand_pick_then_dont_place_any_closes_the_pick() {
    let mut s = session_over(
        sirius_when_digivolving(),
        vec![menu(0, 1), hand_pick(1, "VB-A"), menu(2, 3), top_bottom(3, true)],
    );
    replay_all(&mut s);
    let src = sources(&s.game);
    // card_sources run bottom -> top card; a TOP digivolution card sits right
    // under the top card.
    assert_eq!(src, vec!["BASE", "VB-A", "EX12-018"], "{src:?}");
    assert_eq!(s.game.players[0].hand.len(), 1, "VB-B stays in hand");
}

/// Menu 1, VB-A, menu 2 (From Trash), VB-T: DCGO mixed the zones, which the
/// engine's one-zone pick cannot express. Halt at the first pick.
#[test]
fn ex12_018_second_pick_from_the_other_zone_is_a_divergence() {
    let mut s = session_over(
        sirius_when_digivolving(),
        vec![
            menu(0, 1),
            hand_pick(1, "VB-A"),
            menu(2, 2),
            sel(3, serde_json::json!({"prompt": "SelectCardEffect", "card_ids": ["VB-T"]})),
            top_bottom(4, false),
        ],
    );
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    s.step();
    let reason = halted_at(&s, 1);
    assert!(reason.contains("EX12-018"), "{reason}");
}

/// One card picked and no menu after it: DCGO's loop always asks the menu
/// again while fewer than 2 cards are chosen, so the recording contradicts.
#[test]
fn ex12_018_one_pick_without_the_next_menu_is_a_divergence() {
    let mut s = session_over(
        sirius_when_digivolving(),
        vec![menu(0, 1), hand_pick(1, "VB-A"), top_bottom(2, false)],
    );
    s.step();
    s.step();
    let reason = halted_at(&s, 1);
    assert!(reason.contains("EX12-018"), "{reason}");
}

// ── RYUGUMON (EX12-036): DCGO's no-op unsuspend pick ─────────────────────────
//
// EX12_036.cs:136-160: after placing the hand card as the bottom source, DCGO
// asks "1 of your Digimon may unsuspend" over EVERY one of your Digimon
// (`IsYourDigimonCondition`, `Mode.UnTap`, `canNoSelect`), and unsuspending
// one that is not suspended does nothing (`IUnsuspendPermanents.Unsuspend`
// keeps only `IsSuspended` ones, CardController.cs:6146-6152). The engine offers
// only suspended Digimon (EX12-036.yaml `is_suspended: true`) and asks nothing
// when none is. DCGO's pick of an unsuspended Digimon is that same nothing.

fn ryugumon_on_play() -> Game {
    let mut fish = make_test_card("FISH", "FISH");
    fish.traits = vec!["Aquatic".into(), "Sea Animal".into()];
    fish.level = Some(4);
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-036")
        .expect("EX12-036 YAML")
        .add_card(fish)
        .add_card(make_test_card("FILL", "FILL"))
        .hand(0, &["FISH"])
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .memory(10)
        .start();
    let h = r.place_on_field(0, "EX12-036", None);
    r.game.enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    assert!(r.game.pending_selection.is_some(), "the bottom-source pick is parked");
    r.game
}

#[test]
fn ex12_036_dcgo_unsuspend_pick_of_an_unsuspended_digimon_is_a_no_op() {
    let mut s = session_over(
        ryugumon_on_play(),
        vec![
            hand_pick(0, "FISH"),
            sel(1, serde_json::json!({"prompt": "SelectPermanentEffect", "board_p0": ["EX12-036"], "board_p1": [],
                "targets": [{"player": 0, "frame": 0}]})),
        ],
    );
    replay_all(&mut s);
    assert_eq!(s.current_step(), 2);
    let src: Vec<String> = s.game.players[0].battle_area[0]
        .card_sources
        .iter()
        .map(|c| c.card_id(&s.game.card_data).to_string())
        .collect();
    assert_eq!(src, vec!["FISH", "EX12-036"]);
    assert!(
        s.replay_notes().iter().any(|n| n.contains("EX12-036") && n.contains("unsuspend")),
        "{:?}",
        s.replay_notes()
    );
}

/// Regression guard: an unsuspend target on the OPPONENT's side is no pick
/// DCGO could make there; the row stays unanswered and halts.
#[test]
fn ex12_036_unsuspend_row_naming_an_opponent_digimon_still_halts() {
    let mut s = session_over(
        ryugumon_on_play(),
        vec![
            hand_pick(0, "FISH"),
            sel(1, serde_json::json!({"prompt": "SelectPermanentEffect", "board_p0": ["EX12-036"], "board_p1": ["FILL"],
                "targets": [{"player": 1, "frame": 0}]})),
        ],
    );
    s.step();
    s.step();
    halted_at(&s, 1);
}
