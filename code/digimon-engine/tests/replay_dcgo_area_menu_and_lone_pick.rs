//! DCGO-recording replay folds for two prompt shapes DCGO asks differently from
//! the engine.
//!
//! 1. P-249 Strabimon's [Start of Your Main Phase] "To which area will you place
//!    a card?" (DCGO `P/Yellow/P_249.cs`): a ONE-BASED int menu (1 = under this
//!    Digimon, 2 = under a Tamer with inherited effects, then a
//!    `SelectPermanentEffect` for which Tamer, 3 = don't place). The engine parks
//!    one optional `OwnField` pick over this Digimon and every eligible Tamer.
//!    The replay folds a 2 when this actor's `SelectPermanentEffect` row (a
//!    target, or the cancel of DCGO's `canNoSelect` Tamer pick) follows
//!    (`placement_area_int` in `runners/replay.rs`); otherwise
//!    `selection_resolve::resolve_next` maps 1 to the source permanent and any
//!    other value to the decline. Only for cards whose DCGO menu is verified to
//!    be P-249's (`selection_resolve::dcgo_placement_area_menu`).
//! 2. A mandatory ONE-candidate field pick in front of a `MultipleSkills` row
//!    (EX13-026 Kudamon placing a revealed card under the only [DATA SQUAD]
//!    Tamer): DCGO skips a single-candidate pick, so there is no row for it; the
//!    replay answers it before the ordering row is processed. With two Tamers
//!    the pick is a real decision and must stay parked.
//!
//! Each scenario is staged with `DebugRunner` on the real DSL card (P-249 /
//! EX13-026), parked at the prompt under test, and swapped into a
//! `ReplaySession` built from a DCGO `RecordingV1` whose rows are the selection
//! rows DCGO writes for that decision. The session's own constructed game only
//! exists to satisfy `from_dcgo`; the swapped-in game is what the rows drive.

use std::collections::HashMap;

use digimon_engine::action::space::{ATTACK_START, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::dcgo_recording::{GameEnd, GameStart, RecordingV1, Row, SelectionRow};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::runners::replay::ReplaySession;
use digimon_engine::selection::SelectionKind;
use digimon_engine::{Game, TriggerSource};

// ── Recording scaffolding ───────────────────────────────────────────────────

/// One-card pool for the throwaway game `from_dcgo` constructs (replaced
/// before any row is applied).
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

/// A selection row as the DCGO recorder writes it (`step` is the row index).
fn sel(step: u32, extra: serde_json::Value) -> Row {
    let mut v = serde_json::json!({ "step": step, "actor": 0, "phase": "Main" });
    for (k, val) in extra.as_object().unwrap() {
        v[k] = val.clone();
    }
    Row::Selection(serde_json::from_value::<SelectionRow>(v).unwrap())
}

/// A bot-vs-bot DCGO recording carrying only `rows`, with `game` swapped in
/// as the live game the rows replay against.
fn session_over(game: Game, rows: Vec<Row>) -> ReplaySession {
    let deck = vec!["ST1-03".to_string(); 50];
    let total = rows.len() as u32;
    let rec = RecordingV1 {
        start: GameStart {
            v: 1,
            game_id: "area-menu-lone-pick".into(),
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
        end: GameEnd {
            winner: -1,
            reason: "truncated: test".into(),
            total_steps: total,
        },
    };
    let mut s = ReplaySession::from_dcgo(rec, &shell_db(), false).expect("recording constructs");
    s.game = game;
    s
}

/// Step every row; each must apply without a divergence or pause.
fn replay_all(s: &mut ReplaySession) {
    while s.current_step() < s.total_steps() {
        let before = s.current_step();
        s.step();
        assert!(
            s.divergences().is_empty() && !s.is_paused(),
            "row {before} diverged: {:?}",
            s.divergences()
        );
        assert_eq!(s.current_step(), before + 1, "row {before} did not advance the cursor");
    }
}

fn source_ids(game: &Game, slot: usize) -> Vec<String> {
    game.players[0].battle_area[slot]
        .card_sources
        .iter()
        .map(|c| c.card_id(&game.card_data).to_string())
        .collect()
}

fn hand_ids(game: &Game) -> Vec<String> {
    game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&game.card_data).to_string())
        .collect()
}

/// The session stopped at row `at` with ONE selection-resolution divergence.
fn assert_selection_divergence(s: &ReplaySession, at: u32) {
    use digimon_engine::runners::replay::DivergenceKind;
    match s.divergences() {
        [d] => {
            assert_eq!(d.step, at, "{d:?}");
            assert!(matches!(d.kind, DivergenceKind::SelectionResolution { .. }), "{d:?}");
        }
        other => panic!("expected one divergence at row {at}, got {other:?}"),
    }
    assert!(s.is_paused(), "a divergence halts the replay");
    assert_eq!(s.current_step(), at, "the row is not consumed");
}

// ── (1) P-249 Strabimon area menu ───────────────────────────────────────────

/// P-249 at slot 0, two Tamers with inherited effects at slots 1 and 2, one
/// [Hybrid] card in hand (none in trash: DCGO then asks no "from which area"
/// menu - it calls the unrecorded `SetInt(1)`). Fires [Start of Your Main
/// Phase] and returns the game parked at the payment pick.
fn strabimon_at_start_of_main() -> Game {
    let mut b = DebugRunner::builder().dsl_card("P-249").expect("P-249 YAML");
    for (id, kind, inherited, hybrid) in [
        ("TAMER", CardKind::Tamer, true, false),
        ("MATERIAL", CardKind::Digimon, false, true),
        ("FILL", CardKind::Digimon, false, false),
    ] {
        let mut c = make_test_card(id, id);
        c.card_kind = kind;
        c.colors = vec![CardColor::Yellow];
        if kind == CardKind::Tamer {
            c.level = None;
            c.dp = None;
        }
        if inherited {
            c.inherited_text = "[Your Turn] This Digimon gets +1000 DP.".into();
        }
        if hybrid {
            c.traits = vec!["Hybrid".into()];
        }
        b = b.add_card(c);
    }
    let mut r = b
        .hand(0, &["MATERIAL"])
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .memory(10)
        .start();
    let h = r.place_stack(0, &["P-249"]);
    assert_eq!(h.index, 0);
    assert_eq!(r.place_stack(0, &["TAMER"]).index, 1);
    assert_eq!(r.place_stack(0, &["TAMER"]).index, 2);
    r.game
        .enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    assert!(
        r.pending_selection().is_some(),
        "Strabimon parks its payment pick"
    );
    r.game
}

/// The engine's host pick after the payment card was chosen: this Digimon
/// (slot 0) and both Tamers, optional.
fn assert_host_pick(game: &Game) {
    let p = game.pending_selection.as_ref().expect("host pick parked");
    assert!(matches!(p.kind, SelectionKind::OwnField), "{:?}", p.kind);
    assert!(p.is_optional);
    for slot in 0..3u16 {
        assert!(
            p.valid_action_ids.contains(&(ATTACK_START + slot)),
            "slot {slot} is a candidate: {:?}",
            p.valid_action_ids
        );
    }
}

#[test]
fn p_249_area_menu_one_places_under_strabimon_itself() {
    let mut s = session_over(
        strabimon_at_start_of_main(),
        vec![
            sel(0, serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": ["MATERIAL"]})),
            sel(1, serde_json::json!({"prompt": "generic_int", "int_value": 1})),
        ],
    );
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert_host_pick(&s.game);
    // A positional reading of 1 would take the SECOND candidate (the Tamer at
    // slot 1); the one-based menu's 1 is "Under this Digimon".
    s.step();
    assert!(s.divergences().is_empty() && !s.is_paused(), "{:?}", s.divergences());
    assert_eq!(s.current_step(), 2);
    assert_eq!(source_ids(&s.game, 0), vec!["MATERIAL", "P-249"], "bottom source of Strabimon");
    assert_eq!(source_ids(&s.game, 1), vec!["TAMER"]);
    assert_eq!(source_ids(&s.game, 2), vec!["TAMER"]);
    assert!(hand_ids(&s.game).is_empty());
}

#[test]
fn p_249_area_menu_two_with_target_row_places_under_the_named_tamer() {
    let mut s = session_over(
        strabimon_at_start_of_main(),
        vec![
            sel(0, serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": ["MATERIAL"]})),
            sel(1, serde_json::json!({"prompt": "generic_int", "int_value": 2})),
            sel(2, serde_json::json!({"prompt": "SelectPermanentEffect", "targets": [{"player": 0, "frame": 2}]})),
        ],
    );
    s.step();
    assert_host_pick(&s.game);
    // The int only chose the area: consumed without touching the engine, the
    // host pick stays parked for the target row.
    s.step();
    assert!(s.divergences().is_empty() && !s.is_paused(), "{:?}", s.divergences());
    assert_eq!(s.current_step(), 2);
    assert_host_pick(&s.game);
    assert_eq!(hand_ids(&s.game), vec!["MATERIAL"], "nothing placed by the area int");
    s.step();
    assert!(s.divergences().is_empty() && !s.is_paused(), "{:?}", s.divergences());
    assert_eq!(s.current_step(), 3);
    assert_eq!(source_ids(&s.game, 2), vec!["MATERIAL", "TAMER"], "under the Tamer the row named");
    assert_eq!(source_ids(&s.game, 1), vec!["TAMER"]);
    assert_eq!(source_ids(&s.game, 0), vec!["P-249"]);
    assert!(hand_ids(&s.game).is_empty());
}

#[test]
fn p_249_area_menu_three_is_the_decline() {
    let mut s = session_over(
        strabimon_at_start_of_main(),
        vec![
            sel(0, serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": ["MATERIAL"]})),
            sel(1, serde_json::json!({"prompt": "generic_int", "int_value": 3})),
        ],
    );
    s.step();
    assert_host_pick(&s.game);
    s.step();
    assert!(s.divergences().is_empty() && !s.is_paused(), "{:?}", s.divergences());
    assert_eq!(s.current_step(), 2);
    assert!(s.game.pending_selection.is_none(), "declined: nothing left to answer");
    assert_eq!(hand_ids(&s.game), vec!["MATERIAL"], "Don't place: the card stays in hand");
    for slot in 0..3 {
        assert_eq!(source_ids(&s.game, slot).len(), 1, "slot {slot} unchanged");
    }
}

/// The same int-2 row with NO Tamer row after it. In `P_249.cs:157-190` a 2
/// ("Under Tamer with inherited effects") is ALWAYS followed by DCGO's
/// `canNoSelect` Tamer `SelectPermanentEffect`, which records a pick or a
/// cancel. A 2 with neither after it is a recording the menu cannot explain, so
/// the replay stops with a divergence instead of guessing the decline (or the
/// candidate at position 2). Nothing is placed and the host pick stays parked.
#[test]
fn p_249_area_menu_two_without_tamer_row_is_a_divergence() {
    let mut s = session_over(
        strabimon_at_start_of_main(),
        vec![
            sel(0, serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": ["MATERIAL"]})),
            sel(1, serde_json::json!({"prompt": "generic_int", "int_value": 2})),
        ],
    );
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert_host_pick(&s.game);
    s.step();
    assert_selection_divergence(&s, 1);
    assert_host_pick(&s.game);
    assert_eq!(hand_ids(&s.game), vec!["MATERIAL"]);
    for slot in 0..3 {
        assert_eq!(source_ids(&s.game, slot).len(), 1, "slot {slot} unchanged");
    }
}

/// A value outside P-249's menu (1 / 2 / 3) cannot be read at all: no
/// positional candidate, no decline.
#[test]
fn p_249_area_menu_value_outside_the_menu_is_a_divergence() {
    let mut s = session_over(
        strabimon_at_start_of_main(),
        vec![
            sel(0, serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": ["MATERIAL"]})),
            sel(1, serde_json::json!({"prompt": "generic_int", "int_value": 4})),
        ],
    );
    s.step();
    assert_host_pick(&s.game);
    s.step();
    assert_selection_divergence(&s, 1);
    assert_host_pick(&s.game);
    assert_eq!(hand_ids(&s.game), vec!["MATERIAL"]);
}

/// P-249 at slot 0 with two Tamers at slots 1 and 2, and a hand-built
/// `OwnField` prompt sourced from P-249 with the given candidates and
/// optionality (shapes the DSL never parks, to pin what the menu may mean).
/// The callback records the id it is resolved with.
fn strabimon_with_host_prompt(
    valid: Vec<u16>,
    optional: bool,
) -> (Game, std::sync::Arc<std::sync::Mutex<Option<u16>>>) {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::enums::{EffectSourceKind, GamePhase};
    use digimon_engine::selection::PendingSelection;
    use std::sync::{Arc, Mutex};

    let mut tamer = make_test_card("TAMER", "TAMER");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    tamer.inherited_text = "[Your Turn] This Digimon gets +1000 DP.".into();
    let mut r = DebugRunner::builder()
        .dsl_card("P-249")
        .expect("P-249 YAML")
        .add_card(tamer)
        .add_card(make_test_card("FILL", "FILL"))
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();
    let src = r.place_stack(0, &["P-249"]);
    r.place_stack(0, &["TAMER"]);
    r.place_stack(0, &["TAMER"]);
    let source_card = CardHandle(r.game.players[0].battle_area[src.index as usize].card_sources[0].card_index);
    let chosen: Arc<Mutex<Option<u16>>> = Arc::new(Mutex::new(None));
    let sink = chosen.clone();
    r.game.pending_selection = Some(PendingSelection {
        card_choice: None,
        zone_owner: None,
        kind: SelectionKind::OwnField,
        selecting_player: 0,
        previous_phase: GamePhase::Main,
        valid_action_ids: valid,
        is_optional: optional,
        prompt: "Choose this Digimon or one of your Tamers with inherited effects".into(),
        effect_choices: None,
        source_card,
        source_permanent: Some(src),
        source_kind: EffectSourceKind::Digimon,
        callback: Box::new(move |_, id| *sink.lock().unwrap() = Some(id)),
        on_decline: None,
    });
    (r.game, chosen)
}

/// "Under this Digimon" (1) names Strabimon itself. When the prompt does not
/// accept Strabimon, the 1 has no answer here: a divergence, not the
/// candidate at position 1.
#[test]
fn p_249_area_menu_one_when_strabimon_is_not_a_candidate_is_a_divergence() {
    let (game, chosen) = strabimon_with_host_prompt(vec![ATTACK_START + 1, ATTACK_START + 2], true);
    let mut s = session_over(game, vec![sel(0, serde_json::json!({"prompt": "generic_int", "int_value": 1}))]);
    s.step();
    assert_selection_divergence(&s, 0);
    assert_eq!(*chosen.lock().unwrap(), None, "nothing chosen for the player");
}

/// 2 and 3 are only answers to an OPTIONAL host pick (the Tamer pick's cancel /
/// "Don't place"). Against a mandatory pick they are a divergence.
#[test]
fn p_249_area_menu_two_or_three_against_a_mandatory_pick_is_a_divergence() {
    for v in [2i64, 3] {
        let (game, chosen) =
            strabimon_with_host_prompt(vec![ATTACK_START, ATTACK_START + 1, ATTACK_START + 2], false);
        let mut s = session_over(game, vec![sel(0, serde_json::json!({"prompt": "generic_int", "int_value": v}))]);
        s.step();
        assert_selection_divergence(&s, 0);
        assert_eq!(*chosen.lock().unwrap(), None, "int {v}: nothing chosen for the player");
    }
}

/// P-249's EARLIER menu, "From which area will you select a card?" (From hand
/// = 1 / From trash = 2 / Don't select = 3, `P_249.cs:53-75`), meets the
/// engine's hand+trash payment pick, not the host pick. A 2 there followed by
/// the trash card row only narrows the zone: the trash card is the payment,
/// and the area menu's 1 then places it under Strabimon.
#[test]
fn p_249_zone_menu_two_is_not_read_as_the_placement_menu() {
    let mut b = DebugRunner::builder().dsl_card("P-249").expect("P-249 YAML");
    for (id, kind, inherited, hybrid) in [
        ("TAMER", CardKind::Tamer, true, false),
        ("MATERIAL", CardKind::Digimon, false, true),
        ("TRASHMAT", CardKind::Digimon, false, true),
        ("FILL", CardKind::Digimon, false, false),
    ] {
        let mut c = make_test_card(id, id);
        c.card_kind = kind;
        c.colors = vec![CardColor::Yellow];
        if kind == CardKind::Tamer {
            c.level = None;
            c.dp = None;
        }
        if inherited {
            c.inherited_text = "[Your Turn] This Digimon gets +1000 DP.".into();
        }
        if hybrid {
            c.traits = vec!["Hybrid".into()];
        }
        b = b.add_card(c);
    }
    let mut r = b
        .hand(0, &["MATERIAL"])
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .memory(10)
        .start();
    r.inject_trash(0, "TRASHMAT");
    let h = r.place_stack(0, &["P-249"]);
    r.place_stack(0, &["TAMER"]);
    r.game
        .enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    // The payment is one `select_zone_cards` pick over hand AND trash.
    let p = r.pending_selection().expect("payment pick parked");
    assert!(matches!(p.kind, SelectionKind::CountCappedMultiSelect { .. }), "{:?}", p.kind);
    let mut s = session_over(
        r.game,
        vec![
            sel(0, serde_json::json!({"prompt": "generic_int", "int_value": 2})),
            sel(1, serde_json::json!({"prompt": "SelectCardEffect", "zone": "Trash", "card_ids": ["TRASHMAT"]})),
            sel(2, serde_json::json!({"prompt": "generic_int", "int_value": 1})),
        ],
    );
    replay_all(&mut s);
    assert_eq!(source_ids(&s.game, 0), vec!["TRASHMAT", "P-249"], "the trash card, under Strabimon");
    assert_eq!(source_ids(&s.game, 1), vec!["TAMER"]);
    assert_eq!(hand_ids(&s.game), vec!["MATERIAL"], "the hand card was not the payment");
}

/// Only a 2 opens DCGO's Tamer pick (`P_249.cs`: `SelectedIntValue == 1` places
/// under this Digimon, 3 stops). A 1 followed by a same-seat
/// `SelectPermanentEffect` row naming the Tamer at slot 2 still places under
/// Strabimon itself: that row belongs to something else and must not be read
/// as the Tamer choice.
#[test]
fn p_249_area_menu_one_ignores_a_following_unrelated_target_row() {
    let mut s = session_over(
        strabimon_at_start_of_main(),
        vec![
            sel(0, serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": ["MATERIAL"]})),
            sel(1, serde_json::json!({"prompt": "generic_int", "int_value": 1})),
            sel(2, serde_json::json!({"prompt": "SelectPermanentEffect", "targets": [{"player": 0, "frame": 2}]})),
        ],
    );
    s.step();
    assert_host_pick(&s.game);
    s.step();
    assert!(s.divergences().is_empty() && !s.is_paused(), "{:?}", s.divergences());
    assert_eq!(s.current_step(), 2);
    assert_eq!(source_ids(&s.game, 0), vec!["MATERIAL", "P-249"], "under Strabimon itself");
    assert_eq!(source_ids(&s.game, 2), vec!["TAMER"], "not under the Tamer the later row names");
    assert_eq!(source_ids(&s.game, 1), vec!["TAMER"]);
}

/// A 3 ("Don't place") is the decline even when a same-seat target row follows.
#[test]
fn p_249_area_menu_three_ignores_a_following_unrelated_target_row() {
    let mut s = session_over(
        strabimon_at_start_of_main(),
        vec![
            sel(0, serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": ["MATERIAL"]})),
            sel(1, serde_json::json!({"prompt": "generic_int", "int_value": 3})),
            sel(2, serde_json::json!({"prompt": "SelectPermanentEffect", "targets": [{"player": 0, "frame": 2}]})),
        ],
    );
    s.step();
    assert_host_pick(&s.game);
    s.step();
    assert!(s.divergences().is_empty() && !s.is_paused(), "{:?}", s.divergences());
    assert_eq!(s.current_step(), 2);
    assert!(s.game.pending_selection.is_none(), "declined");
    assert_eq!(hand_ids(&s.game), vec!["MATERIAL"], "Don't place: the card stays in hand");
    for slot in 0..3 {
        assert_eq!(source_ids(&s.game, slot).len(), 1, "slot {slot} unchanged");
    }
}

/// "Under Tamer" (2), then DCGO's `canNoSelect` Tamer pick declined: DCGO
/// records the decline as a `SelectPermanentEffect` cancel with no targets.
/// The int only chose the area (consumed, the host pick stays parked), and the
/// cancel declines the engine's optional `OwnField` pick: nothing is placed.
#[test]
fn p_249_area_menu_two_then_cancelled_tamer_pick_places_nothing() {
    let mut s = session_over(
        strabimon_at_start_of_main(),
        vec![
            sel(0, serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": ["MATERIAL"]})),
            sel(1, serde_json::json!({"prompt": "generic_int", "int_value": 2})),
            sel(2, serde_json::json!({"prompt": "SelectPermanentEffect", "cancel": true})),
        ],
    );
    s.step();
    assert_host_pick(&s.game);
    s.step();
    assert!(s.divergences().is_empty() && !s.is_paused(), "{:?}", s.divergences());
    assert_eq!(s.current_step(), 2);
    assert_host_pick(&s.game);
    s.step();
    assert!(s.divergences().is_empty() && !s.is_paused(), "{:?}", s.divergences());
    assert_eq!(s.current_step(), 3);
    assert!(s.game.pending_selection.is_none(), "the cancel declined the host pick");
    assert_eq!(hand_ids(&s.game), vec!["MATERIAL"], "the card stays where it is");
    for slot in 0..3 {
        assert_eq!(source_ids(&s.game, slot).len(), 1, "slot {slot} unchanged");
    }
}

/// P-249's menu meaning (1 = this Digimon, 3 on an optional pick = decline)
/// belongs to P-249's menu alone. Another card whose DCGO int menu is numbered
/// from 1 (BT15-102 is in `dcgo_int_menu_base.json`; its "from Battle Area" = 1
/// is a different question) carries a card-authored menu constant, not a
/// position: against an `OwnField` pick (no branch list to match it to) the
/// int has no reading, so the replay stops instead of taking the candidate at
/// that position.
#[test]
fn another_one_based_menu_card_does_not_borrow_p_249_area_meaning() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::enums::{EffectSourceKind, GamePhase};
    use digimon_engine::selection::PendingSelection;
    use std::sync::{Arc, Mutex};

    for int_value in [1i64, 2] {
        let mut r = DebugRunner::builder()
            .add_card(make_test_card("BT15-102", "One-based menu card"))
            .add_card(make_test_card("OTHER", "OTHER"))
            .deck(0, &["OTHER"; 5])
            .deck(1, &["OTHER"; 5])
            .start();
        let src = r.place_stack(0, &["BT15-102"]);
        r.place_stack(0, &["OTHER"]);
        r.place_stack(0, &["OTHER"]);
        let source_card = CardHandle(r.game.players[0].battle_area[src.index as usize].card_sources[0].card_index);
        let chosen: Arc<Mutex<Option<u16>>> = Arc::new(Mutex::new(None));
        let sink = chosen.clone();
        r.game.pending_selection = Some(PendingSelection {
            card_choice: None,
            zone_owner: None,
            kind: SelectionKind::OwnField,
            selecting_player: 0,
            previous_phase: GamePhase::Main,
            valid_action_ids: vec![ATTACK_START, ATTACK_START + 1, ATTACK_START + 2],
            is_optional: true,
            prompt: "Choose 1 of your permanents".into(),
            effect_choices: None,
            source_card,
            source_permanent: Some(src),
            source_kind: EffectSourceKind::Digimon,
            callback: Box::new(move |_, id| *sink.lock().unwrap() = Some(id)),
            on_decline: None,
        });
        let mut s = session_over(
            r.game,
            vec![sel(0, serde_json::json!({"prompt": "generic_int", "int_value": int_value}))],
        );
        s.step();
        assert_selection_divergence(&s, 0);
        assert_eq!(
            *chosen.lock().unwrap(),
            None,
            "int {int_value}: neither P-249's reading nor a positional one may apply to BT15-102"
        );
    }
}

// ── (2) Lone field pick in front of a MultipleSkills row ────────────────────

/// Kudamon (EX13-026) played with `tamers` [DATA SQUAD] Tamers on the field and
/// a reveal of two eligible cards; the reveal picks are answered, leaving the
/// game parked at the mandatory "which [DATA SQUAD] Tamer" pick. Returns the
/// game and the Tamer slots.
fn kudamon_parked_at_tamer_pick(tamers: usize) -> (Game, Vec<PermanentHandle>) {
    let digimon = |id: &str, traits: &[&str]| {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Digimon;
        c.colors = vec![CardColor::Yellow];
        c.level = Some(4);
        c.dp = Some(4000);
        c.play_cost = 3;
        c.traits = traits.iter().map(|t| t.to_string()).collect();
        c
    };
    let mut tamer = make_test_card("DS-TAMER", "DS-TAMER");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    tamer.play_cost = 3;
    tamer.colors = vec![CardColor::Yellow];
    tamer.traits = vec!["DATA SQUAD".into()];
    // Deck index 0 is the bottom; the last three are revealed HB, RK, PLAIN.
    let mut r = DebugRunner::builder()
        .dsl_card("EX13-026")
        .expect("EX13-026 YAML")
        .add_card(digimon("HB", &["Holy Beast"]))
        .add_card(digimon("RK", &["Royal Knight"]))
        .add_card(digimon("PLAIN", &["Beast"]))
        .add_card(digimon("FILL", &[]))
        .add_card(tamer)
        .hand(0, &["EX13-026"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "PLAIN", "RK", "HB"])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(5)
        .start();
    let handles: Vec<_> = (0..tamers).map(|_| r.place_stack(0, &["DS-TAMER"])).collect();
    r.play(0, 0).expect("Kudamon played");
    // Answer the reveal picks (add 1 / place 1) until the Tamer pick parks.
    for _ in 0..8 {
        let p = r.pending_selection().expect("Kudamon still resolving");
        if matches!(p.kind, SelectionKind::OwnField) {
            break;
        }
        let (who, id) = (
            p.selecting_player,
            *p.valid_action_ids.iter().find(|&&a| a != PASS).expect("a pick"),
        );
        r.execute_action(who, id).unwrap();
    }
    let p = r.pending_selection().expect("Tamer pick parked");
    assert!(matches!(p.kind, SelectionKind::OwnField), "{:?}", p.kind);
    assert!(!p.is_optional, "the placement is mandatory");
    assert_eq!(p.valid_action_ids.len(), tamers, "{:?}", p.valid_action_ids);
    (r.game, handles)
}

fn face_down_sources(game: &Game, h: PermanentHandle) -> Vec<(String, bool)> {
    let perm = &game.players[0].battle_area[h.index as usize];
    perm.card_sources[..perm.card_sources.len() - 1]
        .iter()
        .map(|c| (c.card_id(&game.card_data).to_string(), c.face_down))
        .collect()
}

#[test]
fn ex13_026_lone_tamer_pick_is_answered_before_a_multiple_skills_row() {
    let (game, tamers) = kudamon_parked_at_tamer_pick(1);
    let mut s = session_over(
        game,
        vec![sel(0, serde_json::json!({"prompt": "MultipleSkills", "int_value": 0}))],
    );
    replay_all(&mut s);
    let placed = face_down_sources(&s.game, tamers[0]);
    assert_eq!(placed.len(), 1, "the lone [DATA SQUAD] Tamer received the card");
    assert!(placed[0].1, "placed face down");
    // What remains is the deck-bottom order of the ONE leftover revealed card: a
    // single-option `OrderedPermutation` DCGO never records (the replay's
    // `answer_forced_single_ordering_prompt` takes it before the next row). No
    // field pick is left parked.
    if let Some(p) = s.game.pending_selection.as_ref() {
        assert!(
            matches!(p.kind, SelectionKind::OrderedPermutation { .. }) && p.valid_action_ids.len() == 1,
            "only the forced remainder order may remain: {:?}",
            (&p.kind, &p.valid_action_ids)
        );
        let (who, id) = (p.selecting_player, p.valid_action_ids[0]);
        s.game.decode_action(id, who);
    }
    assert!(s.game.pending_selection.is_none(), "the effect finished");
    // Remainder returned to the bottom of the deck (index 0 = bottom).
    assert_eq!(s.game.players[0].deck[0].card_id(&s.game.card_data), "PLAIN");
}

/// Control: with TWO [DATA SQUAD] Tamers the pick is a real decision. The
/// ordering row cannot answer it, and the fold must not choose for the player.
#[test]
fn ex13_026_two_candidate_tamer_pick_is_not_auto_answered_by_a_multiple_skills_row() {
    let (game, tamers) = kudamon_parked_at_tamer_pick(2);
    let mut s = session_over(
        game,
        vec![sel(0, serde_json::json!({"prompt": "MultipleSkills", "int_value": 0}))],
    );
    s.step();
    let p = s.game.pending_selection.as_ref().expect("still parked");
    assert!(matches!(p.kind, SelectionKind::OwnField));
    assert_eq!(p.valid_action_ids.len(), 2);
    for h in tamers {
        assert!(face_down_sources(&s.game, h).is_empty(), "no Tamer chosen for the player");
    }
}

/// The skipped pick belongs to the seat whose ordering panel follows it: an
/// ordering row of the OTHER seat proves nothing about P0's pick, which must
/// stay parked rather than be answered on P0's behalf.
#[test]
fn ex13_026_lone_tamer_pick_is_not_answered_for_another_seats_multiple_skills_row() {
    use digimon_engine::runners::replay::DivergenceKind;
    let (game, tamers) = kudamon_parked_at_tamer_pick(1);
    let mut s = session_over(
        game,
        vec![sel(0, serde_json::json!({"actor": 1, "prompt": "MultipleSkills", "int_value": 0}))],
    );
    s.step();
    assert!(
        face_down_sources(&s.game, tamers[0]).is_empty(),
        "P1's ordering row must not place P0's card: {:?}",
        s.game.pending_selection
    );
    // ... and P1's row has no prompt of P1 to answer: the engine waits on P0, so
    // the replay stops with an actor divergence instead of consuming the row.
    match s.divergences() {
        [d] => assert!(
            matches!(d.kind, DivergenceKind::Actor { expected: 0, recorded: 1 }),
            "{d:?}"
        ),
        other => panic!("expected one actor divergence, got {other:?}"),
    }
    assert!(s.is_paused());
    assert_eq!(s.current_step(), 0, "P1's row is not consumed");
    let p = s.game.pending_selection.as_ref().expect("P0's Tamer pick is still parked");
    assert!(matches!(p.kind, SelectionKind::OwnField) && p.selecting_player == 0);
}
