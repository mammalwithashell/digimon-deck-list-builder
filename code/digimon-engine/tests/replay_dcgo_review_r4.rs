//! DCGO replay: round-4 review findings 1-7 and 10 (the replay's folds of
//! DCGO's zone menus, forced counts, unrecorded orders and lone picks).
//!
//! Every game is staged with `DebugRunner` (synthetic cards, hand-built
//! prompts where the engine path is not the subject) and served through a
//! DCGO recording (`ReplaySession::from_dcgo`) whose rows are the rows under test.
//! No recording content is used.
//!
//! 1. A `generic_bool` in front of a pick over the hand AND another zone is
//!    DCGO's "From hand" / "From trash" menu (`SetBoolSelection`; e.g. Asuna
//!    Shiroki (BT25-092): "From hand" = true / "From trash" = false,
//!    BT25_092.cs:220-227, and "From hand" / "From digivolution cards",
//!    BT25_092.cs:127-131). Its value must name the zone of the actor's next
//!    pick under THAT card's DCGO menu, and the pick resolves inside that zone.
//! 2. A `SelectCountEffect` whose candidate set is one value is no decision
//!    (DCGO's `IDegeneration.Degeneration` opens it even with one trashable
//!    card, CardController.cs:5281-5297), so it never halts as "met no prompt".
//! 3. The engine's "Choose the bottommost remaining card", which DCGO never
//!    asks, is answered only by a playback `engine_action`, never by an
//!    unrelated row read positionally.
//! 4. The forced lone-pick loop never answers ANOTHER seat's optional pick.
//! 5. A "From which area?" int menu (P-249: From hand = 1 / From trash = 2 /
//!    Don't select = 3, P_249.cs:53-76) must agree with the zone of the pick
//!    that follows; the pick resolves in that zone (the same card id in hand
//!    and trash takes the copy DCGO named); "Don't" never opens a pick.
//! 6. One branch of a card-less trigger-order choice that the replay cannot
//!    follow does not cut the evaluation of the others short.
//! 7. A playback row (`engine_action`) is never re-read by a card fold.
//! 10. Siriusmon (EX12-018)'s DCGO loop re-offers a card it already chose
//!    (EX12_018.cs:115-131: `canTargetCondition_ByPreSelecetedList: null`), so
//!    the same id in two passes cannot be told from two copies: it halts.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{self, ATTACK_START, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::dcgo_recording::{GameEnd, GameStart, RecordingV1, Row, SelectionRow};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectSourceKind, EffectTiming, GamePhase};
use digimon_engine::game::Game;
use digimon_engine::runners::replay::{DivergenceKind, ReplaySession};
use digimon_engine::selection::{PendingSelection, SelectionKind, TriggerSource, UnionZoneSet};

// ── Scaffolding ──────────────────────────────────────────────────────────────

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

/// A P0 selection row in the main phase with `json`'s fields.
fn row(json: serde_json::Value) -> serde_json::Value {
    let mut v = serde_json::json!({ "actor": 0, "phase": "Main" });
    for (k, val) in json.as_object().unwrap() {
        v[k] = val.clone();
    }
    v
}

/// A DCGO recording of `rows` (numbered in order) replayed over the staged
/// `game`. The game is moved in, not cloned: a hand-built prompt's closure
/// callback does not survive `Game::clone`.
fn session(game: Game, rows: Vec<serde_json::Value>) -> ReplaySession {
    let rows: Vec<Row> = rows
        .into_iter()
        .enumerate()
        .map(|(i, mut v)| {
            v["step"] = serde_json::json!(i);
            Row::Selection(serde_json::from_value::<SelectionRow>(v).expect("selection row"))
        })
        .collect();
    let deck = vec!["ST1-03".to_string(); 50];
    let total = rows.len() as u32;
    let rec = RecordingV1 {
        start: GameStart {
            v: 1,
            game_id: "review-r4".into(),
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

/// Every row applies without a divergence and advances the cursor.
fn replay_all(s: &mut ReplaySession) {
    while s.current_step() < s.total_steps() {
        let before = s.current_step();
        s.step();
        assert!(s.divergences().is_empty() && !s.is_paused(), "row {before} diverged: {:?}", s.divergences());
        assert!(s.current_step() > before, "row {before} did not advance the cursor");
    }
}

/// Exactly one selection-resolution divergence, at row `at`, which halted the
/// replay there; returns its reason.
fn halted_at(s: &ReplaySession, at: u32) -> String {
    let reason = match s.divergences() {
        [d] => {
            assert_eq!(d.step, at, "{d:?}");
            match &d.kind {
                DivergenceKind::SelectionResolution { reason } => reason.clone(),
                other => panic!("expected a selection-resolution divergence, got {other:?}"),
            }
        }
        other => panic!("expected one divergence at row {at}, got {other:?}"),
    };
    assert!(s.is_paused(), "the divergence halts the replay");
    assert_eq!(s.current_step(), at, "the row is not consumed");
    reason
}

type Chosen = Arc<Mutex<Option<u16>>>;

/// P0 to act in the main phase; `source` on P0's field; EVO in P0's hand
/// (index 0) and trash (index 0); a hand-built pick of `kind` parked for
/// `selecting` with `ids`. The callback records the id it resolves with.
fn parked_pick(
    source: &str,
    kind: SelectionKind,
    selecting: u8,
    optional: bool,
    ids: Vec<u16>,
    prompt: &str,
) -> (Game, Chosen) {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card(source, source))
        .add_card(make_test_card("EVO", "EVO"))
        .add_card(make_test_card("MON", "MON"))
        .hand(0, &["EVO"])
        .deck(0, &["MON"; 5])
        .deck(1, &["MON"; 5])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    r.inject_trash(0, "EVO");
    let h = r.place_on_field(0, source, Some(0));
    r.place_on_field(1, "MON", Some(0));
    let source_card = r.top_card(h);
    let chosen: Chosen = Arc::new(Mutex::new(None));
    let sink = chosen.clone();
    r.game.pending_selection = Some(PendingSelection {
        card_choice: None,
        zone_owner: None,
        kind,
        selecting_player: selecting,
        previous_phase: GamePhase::Main,
        valid_action_ids: ids,
        is_optional: optional,
        prompt: prompt.into(),
        effect_choices: None,
        source_card,
        source_permanent: Some(h),
        source_kind: EffectSourceKind::Digimon,
        callback: Box::new(move |_, id| *sink.lock().unwrap() = Some(id)),
        on_decline: None,
    });
    (r.game, chosen)
}

const HAND_EVO: u16 = PLAY_HAND_START;
const TRASH_EVO: u16 = TRASH_EFFECT_START;

/// An optional pick over P0's hand AND trash (EVO in both) of `source`.
fn hand_or_trash_pick(source: &str) -> (Game, Chosen) {
    parked_pick(
        source,
        SelectionKind::UnionZone { zones: UnionZoneSet::HAND | UnionZoneSet::TRASH },
        0,
        true,
        vec![HAND_EVO, TRASH_EVO],
        "Choose 1 card from your hand or trash",
    )
}

fn hand_pick_row() -> serde_json::Value {
    row(serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": ["EVO"]}))
}
fn trash_pick_row() -> serde_json::Value {
    row(serde_json::json!({"prompt": "SelectCardEffect", "zone": "Trash", "card_ids": ["EVO"]}))
}
fn bool_row(value: bool) -> serde_json::Value {
    row(serde_json::json!({"prompt": "generic_bool", "bool_value": value}))
}
fn int_row(value: i64) -> serde_json::Value {
    row(serde_json::json!({"prompt": "generic_int", "int_value": value}))
}

// ── (1) DCGO's "From hand" / "From trash" bool ───────────────────────────────

/// Asuna Shiroki (BT25-092)'s "From hand" = true (BT25_092.cs:220-227),
/// followed by the hand pick: the bool named the zone; the hand copy is used.
/// The replay halted here: only the false / trash half was recognised.
#[test]
fn bool_true_from_hand_before_a_hand_pick_is_the_zone_menu() {
    let (game, chosen) = hand_or_trash_pick("BT25-092");
    let mut s = session(game, vec![bool_row(true), hand_pick_row()]);
    replay_all(&mut s);
    assert_eq!(*chosen.lock().unwrap(), Some(HAND_EVO), "the hand copy was picked");
}

/// "From trash" = false, then the trash pick of a card also in hand: DCGO
/// picked the TRASH copy (`SelectCardEffect` root Trash), so the engine must
/// not take the hand copy that its hand-first identity search finds first.
#[test]
fn bool_false_from_trash_takes_the_trash_copy_when_the_hand_holds_the_same_card() {
    let (game, chosen) = hand_or_trash_pick("BT25-092");
    let mut s = session(game, vec![bool_row(false), trash_pick_row()]);
    replay_all(&mut s);
    assert_eq!(*chosen.lock().unwrap(), Some(TRASH_EVO), "the trash copy was picked");
}

/// The bool contradicts the pick that follows ("From hand" then a trash
/// pick): no reading is grounded, so the replay stops at the bool.
/// Regression guard: halted before the fix too.
#[test]
fn bool_true_before_a_trash_pick_is_a_divergence() {
    let (game, chosen) = hand_or_trash_pick("BT25-092");
    let mut s = session(game, vec![bool_row(true), trash_pick_row()]);
    s.step();
    halted_at(&s, 0);
    assert_eq!(*chosen.lock().unwrap(), None);
}

/// A card whose DCGO menu is the other way round (Lucemon (BT17-025):
/// "From trash" = true, BT17_025.cs:89-90) never has its bool read with
/// BT25-092's meaning: a true before a hand pick halts.
#[test]
fn bool_zone_meaning_comes_from_the_cards_own_dcgo_menu() {
    let (game, chosen) = hand_or_trash_pick("BT17-025");
    let mut s = session(game, vec![bool_row(true), hand_pick_row()]);
    s.step();
    halted_at(&s, 0);
    assert_eq!(*chosen.lock().unwrap(), None);
}

/// Asuna's [Main] cost: "From hand" = true / "From digivolution cards" =
/// false (BT25_092.cs:127-131); the engine asks ONE mandatory pick over the
/// hand and the digivolution cards (BT25-092.yaml `select_union_zone` over
/// `[hand, material]`). A true followed by the hand pick names the zone.
#[test]
fn bool_true_from_hand_before_a_hand_pick_against_a_hand_and_material_pick() {
    let (game, chosen) = parked_pick(
        "BT25-092",
        SelectionKind::UnionZone { zones: UnionZoneSet::HAND | UnionZoneSet::MATERIAL },
        0,
        false,
        vec![HAND_EVO],
        "Trash 1 Option card from your hand or your Digimon's digivolution cards",
    );
    let mut s = session(game, vec![bool_row(true), hand_pick_row()]);
    replay_all(&mut s);
    assert_eq!(*chosen.lock().unwrap(), Some(HAND_EVO));
}

// ── (2) a forced single-value count ──────────────────────────────────────────

/// <De-Digivolve 1> with one card to trash: the engine trashes it without a
/// prompt (CR 4.3 16-12-1, the number is forced), DCGO still records its
/// `SelectCountEffect` with candidates [1] (CardController.cs:5281-5297). A
/// forced count is consumed with a note only when the engine resolved that
/// effect since the last decision (round-5 review, finding 3:
/// `replay_dcgo_review_r5.rs`); here nothing resolved at all, so the count
/// is an effect DCGO ran and the engine did not - it halts.
#[test]
fn forced_single_value_count_with_nothing_parked_and_nothing_resolved_halts() {
    let r = DebugRunner::builder()
        .add_card(make_test_card("MON", "MON"))
        .deck(0, &["MON"; 5])
        .deck(1, &["MON"; 5])
        .start();
    assert!(r.game.pending_selection.is_none());
    let mut s = session(
        r.game,
        vec![row(serde_json::json!({"prompt": "SelectCountEffect", "count": 1, "candidates": [1]}))],
    );
    s.step();
    let reason = halted_at(&s, 0);
    assert!(reason.contains("SelectCountEffect") && reason.contains("forced"), "{reason}");
}

/// Regression guard: a count with a real choice (two candidates) that meets
/// no prompt is still a halting divergence.
#[test]
fn count_with_a_real_choice_and_nothing_parked_still_halts() {
    let r = DebugRunner::builder()
        .add_card(make_test_card("MON", "MON"))
        .deck(0, &["MON"; 5])
        .deck(1, &["MON"; 5])
        .start();
    let mut s = session(
        r.game,
        vec![row(serde_json::json!({"prompt": "SelectCountEffect", "count": 1, "candidates": [1, 2]}))],
    );
    s.step();
    let reason = halted_at(&s, 0);
    assert!(reason.contains("met no prompt"), "{reason}");
}

// ── (3) the unrecorded deck-bottom order ─────────────────────────────────────

fn bottommost_order() -> (Game, Chosen) {
    parked_pick(
        "MON",
        SelectionKind::AnyField,
        0,
        false,
        vec![ATTACK_START, ATTACK_START + 1],
        "Choose the bottommost remaining card",
    )
}

/// An unrelated int row after the order DCGO never asks resolved it by
/// position (`valid_action_ids[0]`). It must halt instead.
#[test]
fn unrelated_int_row_never_answers_the_unrecorded_bottom_order() {
    let (game, chosen) = bottommost_order();
    let mut s = session(game, vec![int_row(0)]);
    s.step();
    let reason = halted_at(&s, 0);
    assert!(reason.contains("bottommost"), "{reason}");
    assert_eq!(*chosen.lock().unwrap(), None, "no slot was taken as the bottommost");
}

/// Regression guard: a playback row stamped with the engine action that
/// answered the order is the real answer.
#[test]
fn playback_engine_action_still_answers_the_bottom_order() {
    let (game, chosen) = bottommost_order();
    let mut s = session(
        game,
        vec![row(serde_json::json!({"prompt": "generic_int", "int_value": 0, "engine_action": ATTACK_START + 1}))],
    );
    replay_all(&mut s);
    assert_eq!(*chosen.lock().unwrap(), Some(ATTACK_START + 1));
}

// ── (4) the forced lone-pick loop and another seat's optional pick ──────────

/// P1 has an OPTIONAL one-candidate pick parked; P0's row cannot resolve
/// against it. The loop accepted P1's "you may" on P0's evidence. It must
/// leave P1's decision alone: the actor mismatch is reported instead.
#[test]
fn forced_lone_pick_loop_never_accepts_another_seats_optional_pick() {
    let (game, chosen) = parked_pick(
        "MON",
        SelectionKind::OwnField,
        1,
        true,
        vec![ATTACK_START],
        "Choose 1 of your Digimon",
    );
    let mut s = session(
        game,
        vec![row(serde_json::json!({"prompt": "SelectCardEffect", "zone": "Trash", "card_ids": ["NOPE"]}))],
    );
    s.step();
    assert_eq!(*chosen.lock().unwrap(), None, "P1's optional pick was not accepted for it");
    assert!(s.is_paused(), "{:?}", s.divergences());
    assert!(
        s.divergences().iter().any(|d| matches!(d.kind, DivergenceKind::Actor { expected: 1, recorded: 0 })),
        "{:?}",
        s.divergences()
    );
}

/// Regression guard: P0's OWN optional one-candidate pick in front of P0's
/// row for a later prompt is still DCGO's unasked lone pick.
#[test]
fn forced_lone_pick_loop_still_answers_the_actors_own_optional_pick() {
    let (game, chosen) = parked_pick(
        "MON",
        SelectionKind::OwnField,
        0,
        true,
        vec![ATTACK_START],
        "Choose 1 of your Digimon",
    );
    let mut s = session(
        game,
        vec![row(serde_json::json!({"prompt": "SelectCardEffect", "zone": "Trash", "card_ids": ["NOPE"]}))],
    );
    s.step();
    assert_eq!(*chosen.lock().unwrap(), Some(ATTACK_START), "the lone candidate was taken");
}

// ── (5) DCGO's "From which area?" int menu ───────────────────────────────────

/// EX12-074's menu is P-249's numbering (From hand = 1, From trash = 2,
/// Don't = 3; EX12_074.cs:146-151). "From trash" then the trash pick of a
/// card also in hand: the TRASH copy is the one DCGO picked.
#[test]
fn int_from_trash_takes_the_trash_copy_when_the_hand_holds_the_same_card() {
    let (game, chosen) = hand_or_trash_pick("EX12-074");
    let mut s = session(game, vec![int_row(2), trash_pick_row()]);
    replay_all(&mut s);
    assert_eq!(*chosen.lock().unwrap(), Some(TRASH_EVO));
}

/// Regression guard: "From hand" then the hand pick takes the hand copy.
#[test]
fn int_from_hand_takes_the_hand_copy() {
    let (game, chosen) = hand_or_trash_pick("EX12-074");
    let mut s = session(game, vec![int_row(1), hand_pick_row()]);
    replay_all(&mut s);
    assert_eq!(*chosen.lock().unwrap(), Some(HAND_EVO));
}

/// The 2026-11-13 recordings carry no `zone` on a `SelectCardEffect`. After
/// "From trash" such a row is the trash pick (every verified hand-or-trash
/// menu asks the hand as a `SelectHandEffect`), and it still takes the TRASH
/// copy of a card that is also in hand.
#[test]
fn int_from_trash_then_a_zoneless_card_pick_takes_the_trash_copy() {
    let (game, chosen) = hand_or_trash_pick("EX12-074");
    let mut s = session(
        game,
        vec![int_row(2), row(serde_json::json!({"prompt": "SelectCardEffect", "card_ids": ["EVO"]}))],
    );
    replay_all(&mut s);
    assert_eq!(*chosen.lock().unwrap(), Some(TRASH_EVO));
}

/// The same for Asuna Shiroki (BT25-092)'s "From trash" bool.
#[test]
fn bool_from_trash_then_a_zoneless_card_pick_takes_the_trash_copy() {
    let (game, chosen) = hand_or_trash_pick("BT25-092");
    let mut s = session(
        game,
        vec![bool_row(false), row(serde_json::json!({"prompt": "SelectCardEffect", "card_ids": ["EVO"]}))],
    );
    replay_all(&mut s);
    assert_eq!(*chosen.lock().unwrap(), Some(TRASH_EVO));
}

/// "From hand" followed by a TRASH pick: the menu and the pick disagree.
#[test]
fn int_menu_contradicted_by_the_pick_zone_is_a_divergence() {
    let (game, chosen) = hand_or_trash_pick("EX12-074");
    let mut s = session(game, vec![int_row(1), trash_pick_row()]);
    s.step();
    halted_at(&s, 0);
    assert_eq!(*chosen.lock().unwrap(), None);
}

/// "Don't play" (3) opens no pick in DCGO (EX12_074.cs:160-170), so a card
/// row after it belongs to something else: halt instead of consuming the 3.
#[test]
fn int_dont_followed_by_a_pick_is_a_divergence() {
    let (game, chosen) = hand_or_trash_pick("EX12-074");
    let mut s = session(game, vec![int_row(3), hand_pick_row()]);
    s.step();
    halted_at(&s, 0);
    assert_eq!(*chosen.lock().unwrap(), None);
}

/// Regression guard: "Don't play" with no pick after it declines the
/// optional pick (PASS).
#[test]
fn int_dont_alone_declines_the_optional_pick() {
    let (game, chosen) = hand_or_trash_pick("EX12-074");
    let mut s = session(game, vec![int_row(3)]);
    replay_all(&mut s);
    assert_eq!(*chosen.lock().unwrap(), None, "nothing picked");
    assert!(s.game.pending_selection.is_none(), "the pick was declined");
}

/// EX11-022's menu counts from 0 (From hand = 0, From trash = 1,
/// EX11_022.cs:72-76): its 1 is the TRASH, so a 1 before a hand pick halts.
#[test]
fn int_menu_values_come_from_the_cards_own_dcgo_menu() {
    let (game, chosen) = hand_or_trash_pick("EX11-022");
    let mut s = session(game, vec![int_row(1), hand_pick_row()]);
    s.step();
    halted_at(&s, 0);
    assert_eq!(*chosen.lock().unwrap(), None);
}

// ── (6) a trigger-order branch the replay cannot follow ─────────────────────

const SHISHIMAMON: &str = "EX12-046";

/// An inherited [End of Attack] that returns every opponent Digimon to the
/// bottom of the deck: with two of them the engine asks their order, which
/// DCGO never records.
const SWEEP_YAML: &str = r#"
card: TEST-EOA-SWEEP
name: Test End of Attack Sweep
kind: digimon
level: 4
color: [yellow]
cost: 5
dp: 5000
traits: [Test]
effects:
  - scope: inherited
    when: end_of_attack
    summary: "[Inherited][End of Attack] Return all opponent Digimon to the bottom of the deck"
    process:
      - return_all_to_deck:
          over:
            all_of:
              - of: opponent
              - zone: [battle_area]
              - kind: digimon
          position: bottom
"#;

fn digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card_with_level(id, id, level);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.play_cost = 5;
    card.dp = Some(dp);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

/// One branch of the card-less order (the sweep) parks the deck-bottom order
/// DCGO decides without a row; the other (Shishimamon's play) fits the next
/// row. The replay cannot tell which DCGO took, so it stops - but only after
/// trying every branch, and it says what each one did. It used to return on
/// the first branch it could not follow.
#[test]
fn unfollowable_order_branch_does_not_cut_the_other_branches_short() {
    let mut runner = DebugRunner::builder()
        .dsl_card(SHISHIMAMON)
        .expect("EX12-046 YAML loads")
        .from_dsl_yaml(SWEEP_YAML)
        .expect("sweep fixture compiles")
        .add_card(digimon("CARRIER", 6, 9000, &[]))
        .add_card(digimon("TB-LOW", 4, 5000, &["Shambala", "TB"]))
        .add_card(digimon("OPP", 4, 6000, &[]))
        .hand(0, &["TB-LOW"])
        .memory(3)
        .start();
    runner.game.turn_count = 1;
    runner.place_stack(1, &["OPP"]);
    runner.place_stack(1, &["OPP"]);
    let carrier = runner.place_stack(0, &["TEST-EOA-SWEEP", SHISHIMAMON, "CARRIER"]);
    runner.game.enqueue_triggered(EffectTiming::EndOfAttack, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();
    let p = runner.game.pending_selection.as_ref().expect("the order prompt is parked");
    assert!(matches!(p.kind, SelectionKind::TriggerOrder), "{:?}", p.kind);
    let mut s = session(
        runner.game,
        vec![
            row(serde_json::json!({"prompt": "MultipleSkills", "int_value": 0})),
            row(serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": ["TB-LOW"]})),
        ],
    );
    s.step();
    let reason = halted_at(&s, 0);
    assert!(reason.contains("is ambiguous"), "{reason}");
    assert!(reason.contains("bottommost"), "the unfollowable branch is named: {reason}");
    assert!(reason.contains("1 entry fits"), "the other branch was evaluated: {reason}");
}

// ── (7) a playback row is never re-read by a card fold ──────────────────────

const BT20_095: &str = "BT20-095";

fn black(id: &str, chronicle: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Black];
    c.dp = Some(4000);
    if chronicle {
        c.traits = vec!["Chronicle".into()];
    }
    c
}

/// Lv.4 [Chronicle] Digimon that digivolves from a black Lv.3.
fn evolution(id: &str) -> CardData {
    let mut c = black(id, true);
    c.level = Some(4);
    c.play_cost = 6;
    c.evo_costs = vec![digimon_engine::card_data::EvoCost { card_color: CardColor::Black as u8, level: 3, memory_cost: 3 }];
    c
}

/// BT20-095 placed as a <Delay> by P0, P1's main phase, a Lv.3 [Chronicle]
/// Digimon in P0's breeding area, EVOLUTION in P0's trash; P0's [Chronicle]
/// Digimon is deleted and the Delay's "use it?" accept is parked (the same
/// staging as `replay_dcgo_bool_meaning.rs`).
fn delay_offered() -> Game {
    let mut r = DebugRunner::builder()
        .dsl_card(BT20_095)
        .expect("BT20-095 YAML")
        .add_card(black("FILL", false))
        .add_card(black("CHRON", true))
        .add_card(evolution("EVOLUTION"))
        .hand(0, &[BT20_095])
        .deck(0, &["FILL"; 30])
        .deck(1, &["FILL"; 30])
        .security(0, &["FILL"; 5])
        .security(1, &["FILL"; 5])
        .memory(10)
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    r.place_on_field(0, "FILL", Some(0));
    let idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == BT20_095)
        .unwrap();
    r.game.decode_action(space::PLAY_HAND_START + idx as u16, 0);
    r.auto_resolve().unwrap();
    r.pass_turn();
    r.auto_resolve().unwrap();
    r.game.enter_main_phase();
    assert_eq!(r.turn_player(), 1);
    r.place_in_breeding(0, "CHRON");
    r.inject_trash(0, "EVOLUTION");
    let victim = r.place_on_field(0, "CHRON", Some(0));
    r.game.delete_permanent_with_effects(victim);
    r.game.drain_effect_queue();
    let p = r.game.pending_selection.as_ref().expect("the Delay is offered");
    assert_eq!(p.valid_action_ids, vec![space::REPLACEMENT_ACCEPT], "{}", p.prompt);
    r.game
}

/// FELLOWSHIP OF HOPE'S KEEPERS (BT20-095)'s <Delay> fold reads DCGO's
/// "Digivolve?" bool and then demands DCGO's zone and card rows. A playback
/// row carries the engine action it realised (here the breeding move); it is
/// answered by that action, not re-read by the fold - which halted here,
/// demanding a card row the playback recording has no reason to carry.
#[test]
fn bt20_095_fold_leaves_a_playback_row_to_its_engine_action() {
    let move_id = space::encode_breeding_select(0).expect("breeding move id");
    let mut s = session(
        delay_offered(),
        vec![
            row(serde_json::json!({"prompt": "generic_bool", "bool_value": true, "engine_action": space::REPLACEMENT_ACCEPT})),
            row(serde_json::json!({"prompt": "generic_bool", "bool_value": true, "engine_action": move_id})),
        ],
    );
    replay_all(&mut s);
    assert!(s.game.players[0].breeding_area.is_none(), "the stamped move was applied");
    let p = s.game.pending_selection.as_ref().expect("the digivolution pick is parked next");
    assert_eq!(p.selecting_player, 0);
}

// ── (10) Siriusmon (EX12-018) picking one id twice ──────────────────────────

fn vb(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Red];
    c.level = Some(4);
    c.traits = vec!["VB".into()];
    c
}

/// Siriusmon (EX12-018) on a Lv.5 VB Digimon, [When Digivolving] fired, two
/// copies of VB-A in hand.
fn sirius_two_copies() -> Game {
    let mut base = vb("BASE");
    base.level = Some(5);
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-018")
        .expect("EX12-018 YAML")
        .add_card(base)
        .add_card(vb("VB-A"))
        .add_card(make_test_card("FILL", "FILL"))
        .hand(0, &["VB-A", "VB-A"])
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .memory(10)
        .start();
    let h = r.place_stack(0, &["BASE", "EX12-018"]);
    r.game.enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    assert!(r.game.pending_selection.is_some());
    r.game
}

/// DCGO's second pass re-offers the card the first pass chose
/// (EX12_018.cs:115-131, no `canTargetCondition_ByPreSelecetedList`), so two
/// passes naming VB-A may be ONE physical card picked twice. The fold fed the
/// engine two different copies; it must halt instead.
#[test]
fn ex12_018_same_id_in_two_passes_is_a_divergence() {
    let hand = |id: &str| row(serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": [id]}));
    let mut s = session(
        sirius_two_copies(),
        vec![int_row(1), hand("VB-A"), int_row(1), hand("VB-A"), bool_row(false)],
    );
    let mut halted = None;
    for _ in 0..5 {
        s.step();
        if s.is_paused() {
            halted = Some(s.current_step());
            break;
        }
    }
    let at = halted.unwrap_or_else(|| panic!("the replay must halt: {:?}", s.divergences()));
    let reason = halted_at(&s, at);
    assert!(reason.contains("EX12-018") && reason.contains("twice"), "{reason}");
    assert_eq!(s.game.players[0].hand.len(), 2, "no copy was placed");
}
