//! DCGO replay: "From which area?" int menus that DCGO asks whenever EITHER the
//! hand or the trash holds a candidate, and the placement-area menu of
//! Beowolfmon (AD1-015).
//!
//! Three field gaps (raw-20261116T045756Z games 4, 6 and 8) shared one cause:
//! the replay folds DCGO's numbered menus (`generic_int`) onto the engine's
//! one-prompt reading, but only for the shapes it had verified.
//!
//! * Beowolfmon (AD1-015) [End of Attack] / [On Deletion]
//!   (`AD1/Yellow/AD1_015.cs:150-189`): asks "Play Tamer from hand" = 1 /
//!   "Play Tamer from trash" = 2 / "Don't play" = 3 whenever the hand OR the
//!   trash holds a playable Tamer, listing only the zones that do. A hand-only
//!   board therefore still records `generic_int 1` before the hand pick, while
//!   the engine parks ONE `select_zone_cards` pick over both zones. (Strabimon
//!   (P-249), whose menu is asked only when BOTH zones hold a candidate -
//!   `P_249.cs:51-71` sets the zone silently otherwise - keeps that rule.)
//! * Beowolfmon's second menu (`AD1_015.cs:191-210`) is "Place under this
//!   Digimon" = 1 / "Place under a Tamer" = 2 / "Don't place" = 3, identical to
//!   Strabimon's area menu: 2 is followed by a `canNoSelect` Tamer pick.
//! * LadyDevimon (BT25-083) [On Play] / [When Digivolving]
//!   (`BT25/Purple/BT25_083.cs:34-52`) asks Hand = 1 / Trash = 2 / Don't place
//!   = 3 (only the zones that hold a card), but had no entry in the zone-menu
//!   tables, so the int could not be read at all.
//!
//! Staged with `DebugRunner` (real YAML, synthetic cards). No recording content.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::dcgo_recording::{GameEnd, GameStart, RecordingV1, Row, SelectionRow};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
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
            game_id: "zone-menu-either-zone".into(),
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

fn menu(step: u32, v: i64) -> Row {
    sel(step, serde_json::json!({"prompt": "generic_int", "int_value": v}))
}
fn hand_pick(step: u32, id: &str) -> Row {
    sel(step, serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": [id]}))
}
fn trash_pick(step: u32, id: &str) -> Row {
    sel(step, serde_json::json!({"prompt": "SelectCardEffect", "zone": "Trash", "card_ids": [id]}))
}
fn tamer_pick(step: u32, frame: u32) -> Row {
    sel(step, serde_json::json!({"prompt": "SelectPermanentEffect", "targets": [{"player": 0, "frame": frame}]}))
}

fn replay_all(s: &mut ReplaySession) {
    while s.current_step() < s.total_steps() {
        let before = s.current_step();
        s.step();
        assert!(s.divergences().is_empty() && !s.is_paused(), "row {before} diverged: {:?}", s.divergences());
        assert!(s.current_step() > before, "row {before} did not advance");
    }
}

fn halted_reason(s: &ReplaySession, at: u32) -> String {
    match s.divergences() {
        [d] => {
            assert_eq!(d.step, at, "{d:?}");
            match &d.kind {
                DivergenceKind::SelectionResolution { reason } => reason.clone(),
                other => panic!("{other:?}"),
            }
        }
        other => panic!("expected one divergence at row {at}, got {other:?}"),
    }
}

fn zone_ids(game: &Game, trash: bool) -> Vec<String> {
    let z = if trash { &game.players[0].trash } else { &game.players[0].hand };
    z.iter().map(|c| c.card_id(&game.card_data).to_string()).collect()
}

// ── Beowolfmon (AD1-015) ─────────────────────────────────────────────────────

fn beowolfmon_end_of_attack(hand: &[&str], trash: &[&str]) -> Game {
    let mut b = DebugRunner::builder().dsl_card("AD1-015").expect("Beowolfmon YAML");
    for (id, inherited) in [("TEST-Y", true), ("TEST-Y2", true)] {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Tamer;
        c.dp = None;
        c.level = None;
        c.colors = vec![CardColor::Yellow];
        c.play_cost = 9;
        if inherited {
            c.inherited_text = "[Your Turn] This Digimon gets +1000 DP.".into();
        }
        b = b.add_card(c);
    }
    let mut h = make_test_card("TEST-H", "Hybrid");
    h.level = Some(4);
    h.traits = vec!["Hybrid".into()];
    b = b.add_card(h).add_card(make_test_card("FILL", "FILL"));
    let mut r = b.hand(0, hand).deck(0, &["FILL"; 10]).deck(1, &["FILL"; 10]).memory(10).start();
    for id in trash {
        r.inject_trash(0, id);
    }
    let h = r.place_on_field(0, "AD1-015", Some(0));
    r.game.enqueue_triggered(EffectTiming::EndOfAttack, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    assert!(r.game.pending_selection.is_some(), "the Tamer pick is parked");
    r.game
}

/// The g6 shape: the Tamer is in the hand only. DCGO still asks "Play Tamer
/// from hand" = 1 / "Don't play" = 3, so a hand pick follows `generic_int 1`;
/// the second menu "Place under a Tamer" = 2 then names the Tamer just played
/// (board slot 1) and the Hybrid card goes under it, drawing 2.
#[test]
fn beowolfmon_hand_only_tamer_menu_then_placement_under_the_tamer() {
    let mut s = session_over(
        beowolfmon_end_of_attack(&["TEST-Y", "TEST-H"], &[]),
        vec![menu(0, 1), hand_pick(1, "TEST-Y"), menu(2, 2), tamer_pick(3, 1), hand_pick(4, "TEST-H")],
    );
    replay_all(&mut s);
    let bottom: Vec<String> = s.game.players[0].battle_area[1]
        .card_sources
        .iter()
        .map(|c| c.card_id(&s.game.card_data).to_string())
        .collect();
    assert_eq!(bottom, vec!["TEST-H", "TEST-Y"], "the Hybrid card sits under the played Tamer");
    assert_eq!(s.game.players[0].hand.len(), 2, "TEST-Y and TEST-H left the hand, Draw 2");
}

/// "Place under this Digimon" = 1 names the carrier itself.
#[test]
fn beowolfmon_place_under_this_digimon() {
    let mut s = session_over(
        beowolfmon_end_of_attack(&["TEST-Y", "TEST-H"], &[]),
        vec![menu(0, 1), hand_pick(1, "TEST-Y"), menu(2, 1), hand_pick(3, "TEST-H")],
    );
    replay_all(&mut s);
    let under: Vec<String> = s.game.players[0].battle_area[0]
        .card_sources
        .iter()
        .map(|c| c.card_id(&s.game.card_data).to_string())
        .collect();
    assert_eq!(under, vec!["TEST-H", "AD1-015"]);
}

/// A trash-only Tamer: "Play Tamer from trash" = 2 and a trash pick.
#[test]
fn beowolfmon_trash_only_tamer_menu_two_then_trash_pick() {
    let mut s = session_over(
        beowolfmon_end_of_attack(&[], &["TEST-Y"]),
        vec![menu(0, 2), trash_pick(1, "TEST-Y")],
    );
    replay_all(&mut s);
    assert_eq!(s.game.players[0].battle_area.len(), 2, "the Tamer was played from the trash");
    assert!(zone_ids(&s.game, true).is_empty());
}

/// "Don't play" = 3 declines the free play; the hand is untouched.
#[test]
fn beowolfmon_dont_play_declines_the_tamer() {
    let mut s = session_over(beowolfmon_end_of_attack(&["TEST-Y"], &[]), vec![menu(0, 3)]);
    s.step();
    assert!(s.divergences().is_empty() && !s.is_paused(), "{:?}", s.divergences());
    assert_eq!(s.game.players[0].battle_area.len(), 1);
    assert_eq!(zone_ids(&s.game, false), vec!["TEST-Y"]);
}

/// A trash pick row after "From hand" contradicts the menu: the fold halts.
#[test]
fn beowolfmon_menu_one_followed_by_a_trash_pick_is_a_divergence() {
    let mut s = session_over(
        beowolfmon_end_of_attack(&["TEST-Y"], &["TEST-Y2"]),
        vec![menu(0, 1), trash_pick(1, "TEST-Y2")],
    );
    s.step();
    let reason = halted_reason(&s, 0);
    assert!(reason.contains("AD1-015"), "{reason}");
}

// ── Strabimon (P-249): a hand-only candidate asks NO zone menu ───────────────

/// P-249 sets the zone itself when only one zone holds a [Hybrid] card
/// (`P_249.cs:51-71`: `SetInt(canSelectHand ? 1 : 2)`, no `SetIntSelection`),
/// so DCGO records no menu row; an int meeting the hand-only pick is something
/// else and must not be folded as the zone menu.
#[test]
fn strabimon_hand_only_pick_does_not_fold_an_int_as_the_zone_menu() {
    let mut b = DebugRunner::builder().dsl_card("P-249").expect("P-249 YAML");
    let mut h = make_test_card("TEST-H", "Hybrid");
    h.level = Some(4);
    h.traits = vec!["Hybrid".into()];
    b = b.add_card(h).add_card(make_test_card("FILL", "FILL"));
    let mut r = b.hand(0, &["TEST-H"]).deck(0, &["FILL"; 10]).deck(1, &["FILL"; 10]).memory(10).start();
    let carrier = r.place_on_field(0, "P-249", Some(0));
    r.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(carrier));
    r.game.drain_effect_queue();
    assert!(r.game.pending_selection.is_some(), "P-249's pick is parked");
    let mut s = session_over(r.game, vec![menu(0, 1), hand_pick(1, "TEST-H")]);
    s.step();
    assert!(!s.divergences().is_empty(), "an int with no menu in DCGO is not folded");
}

// ── LadyDevimon (BT25-083) ───────────────────────────────────────────────────

fn ladydevimon_on_play(hand: &[&str], trash: &[&str]) -> Game {
    let mut b = DebugRunner::builder().dsl_card("BT25-083").expect("BT25-083 YAML");
    let mut tm = make_test_card("TM", "TM");
    tm.traits = vec!["Three Musketeers".into()];
    b = b.add_card(tm).add_card(make_test_card("FILL", "FILL"));
    let mut r = b.hand(0, hand).deck(0, &["FILL"; 10]).deck(1, &["FILL"; 10]).memory(10).start();
    for id in trash {
        r.inject_trash(0, id);
    }
    let h = r.place_on_field(0, "BT25-083", Some(0));
    // a second Digimon, so "which Digimon gets the card" is a real choice
    r.place_on_field(0, "FILL", Some(1));
    r.game.enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    assert!(r.game.pending_selection.is_some(), "the hand-or-trash pick is parked");
    r.game
}

fn carrier_sources(game: &Game) -> Vec<String> {
    game.players[0].battle_area[0]
        .card_sources
        .iter()
        .map(|c| c.card_id(&game.card_data).to_string())
        .collect()
}

/// The g4 shape: the [Three Musketeers] card is in the trash; DCGO asks
/// Hand = 1 / Trash = 2 / Don't place = 3 (only the zones that hold a card), 2
/// is followed by the trash pick and then the Digimon pick.
#[test]
fn ladydevimon_trash_menu_two_places_the_trash_card_and_draws() {
    let mut s = session_over(
        ladydevimon_on_play(&[], &["TM"]),
        vec![menu(0, 2), trash_pick(1, "TM"), tamer_pick(2, 0)],
    );
    replay_all(&mut s);
    assert_eq!(carrier_sources(&s.game), vec!["TM", "BT25-083"]);
    assert!(zone_ids(&s.game, true).is_empty());
    assert_eq!(s.game.players[0].hand.len(), 1, "Draw 1");
}

#[test]
fn ladydevimon_hand_menu_one_places_the_hand_card() {
    let mut s = session_over(
        ladydevimon_on_play(&["TM"], &[]),
        vec![menu(0, 1), hand_pick(1, "TM"), tamer_pick(2, 0)],
    );
    replay_all(&mut s);
    assert_eq!(carrier_sources(&s.game), vec!["TM", "BT25-083"]);
}

#[test]
fn ladydevimon_dont_place_declines() {
    let mut s = session_over(ladydevimon_on_play(&["TM"], &["TM"]), vec![menu(0, 3)]);
    s.step();
    assert!(s.divergences().is_empty() && !s.is_paused(), "{:?}", s.divergences());
    assert_eq!(carrier_sources(&s.game), vec!["BT25-083"]);
}
