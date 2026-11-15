//! DCGO replay: what a recorded `generic_bool` may answer.
//!
//! A PvP recording's `generic_bool` row (DCGO
//! `UserSelectionManager.SetBoolForPlayer`, UserSelectionManager.cs:82-120)
//! carries the chosen bool and nothing else: no message, no card. It answers a
//! card-authored `SetBoolSelection` menu (UserSelectionManager.cs:227-281)
//! whose buttons mean whatever that card says. These tests pin:
//!
//! 1. FELLOWSHIP OF HOPE'S KEEPERS (BT20-095)'s <Delay>. DCGO
//!    (`BT20/Black/BT20_095.cs:107-180`) moves `GetBreedingAreaPermanents()[0]`
//!    with NO prompt, then asks the bool "Digivolve?" (Digivolve = true), then -
//!    only when both zones hold a candidate - the bool "From which area do you
//!    play a card?" (From hand = true / From trash = false; with one zone it
//!    calls the unrecorded `SetBool(canSelectHand)`), then the card pick
//!    (`CardEffectCommons.DigivolveIntoHandOrTrashCard`: `SelectHandEffect` for
//!    the hand, `SelectCardEffect` for the trash, both `canNoSelect`). The
//!    engine asks an optional "Move ...?" breeding pick first (BT20-095.yaml;
//!    CR 4.3 15-7-1 "by X, Y" makes X optional), then ONE optional pick over
//!    hand and trash. The replay must answer the move "yes" itself (DCGO
//!    always moves) and answer the pick from the Digivolve bool, the zone bool
//!    and the card row together, halting when one is missing or contradicts.
//! 2. A bool never answers a body choice DCGO does not ask as a bool: RizeGreymon
//!    (ST24-06)'s "Trash 2 face-down cards" / "Don't" is a yes/no-shaped
//!    `EffectChoice`, but ST24_06.cs has no `SetBoolSelection` at all (DCGO asks
//!    a `canNoSelect` Tamer `SelectPermanentEffect` there). Only a card whose
//!    own DCGO script has a yes/no bool menu
//!    (`runners/dcgo_bool_yes_no_cards.json`) can have its yes/no prompts
//!    answered by a bool; top/bottom placement (RevealLibrary.cs:627-645) keeps
//!    its mapping.

use std::collections::HashMap;

use digimon_engine::action::space;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::dcgo_recording::{GameEnd, GameStart, RecordingV1, Row, SelectionRow};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectSourceKind, GamePhase};
use digimon_engine::permanent::OptionState;
use digimon_engine::runners::replay::{DivergenceKind, ReplaySession};
use digimon_engine::selection::{EffectChoiceEntry, PendingSelection, SelectionKind};
use digimon_engine::{CardColor, CardKind, Game};

// ── Recording scaffolding (same shape as replay_dcgo_area_menu_and_lone_pick) ──

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
            game_id: "bool-meaning".into(),
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

fn replay_all(s: &mut ReplaySession) {
    while s.current_step() < s.total_steps() {
        let before = s.current_step();
        s.step();
        assert!(
            s.divergences().is_empty() && !s.is_paused(),
            "row {before} diverged: {:?}",
            s.divergences()
        );
        assert!(s.current_step() > before, "row {before} did not advance the cursor");
    }
}

/// Exactly one selection-resolution divergence, at row `at`, nothing consumed
/// from it; returns its reason.
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

// ── (1) BT20-095 Fellowship of Hope's Keepers <Delay> ─────────────────────────

const BT20_095: &str = "BT20-095";

fn card(id: &str, chronicle: bool) -> CardData {
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
    let mut c = card(id, true);
    c.level = Some(4);
    c.play_cost = 6;
    c.evo_costs = vec![EvoCost { card_color: CardColor::Black as u8, level: 3, memory_cost: 3 }];
    c
}

/// BT20-095 placed as a <Delay> by P0 on an earlier turn, P1's main phase,
/// a Lv.3 [Chronicle] Digimon in P0's breeding area, EVOLUTION in the zones
/// asked for. P0's [Chronicle] Digimon is then deleted: the game is parked at
/// the Delay's "use it?" accept.
fn delay_offered(evolution_in_hand: bool, evolution_in_trash: bool) -> Game {
    let mut r = DebugRunner::builder()
        .dsl_card(BT20_095)
        .expect("BT20-095 YAML")
        .add_card(card("FILL", false))
        .add_card(card("CHRON", true))
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
    // [Main]: reveal 3 (no [Chronicle] card among them), return them, then
    // the Option places itself as a <Delay>.
    let idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == BT20_095)
        .unwrap();
    r.game.decode_action(space::PLAY_HAND_START + idx as u16, 0);
    r.auto_resolve().unwrap();
    assert!(
        r.game.players[0].battle_area.iter().any(|p| p.top_card().card_id(&r.game.card_data) == BT20_095
            && matches!(p.option_state, OptionState::Delayed { .. })),
        "BT20-095 is placed as a Delay"
    );
    r.pass_turn();
    r.auto_resolve().unwrap();
    r.game.enter_main_phase();
    assert_eq!(r.turn_player(), 1);
    r.place_in_breeding(0, "CHRON");
    if evolution_in_hand {
        r.add_to_hand(0, "EVOLUTION");
    }
    if evolution_in_trash {
        r.inject_trash(0, "EVOLUTION");
    }
    let victim = r.place_on_field(0, "CHRON", Some(0));
    r.game.delete_permanent_with_effects(victim);
    r.game.drain_effect_queue();
    let p = r.game.pending_selection.as_ref().expect("the Delay is offered");
    assert_eq!(p.selecting_player, 0);
    assert_eq!(p.valid_action_ids, vec![space::REPLACEMENT_ACCEPT], "{}", p.prompt);
    r.game
}

/// The row DCGO writes for P0 (the remote seat) using the Delay:
/// `OptionalSkill.SetUseOptional(true)`, unstamped.
fn use_delay(step: u32) -> Row {
    sel(step, serde_json::json!({"prompt": "generic_bool", "bool_value": true, "unstamped": true}))
}

fn bool_row(step: u32, value: bool) -> Row {
    sel(step, serde_json::json!({"prompt": "generic_bool", "bool_value": value}))
}

fn moved_top(game: &Game) -> Option<String> {
    // The moved Digimon is the only P0 Digimon with a CHRON at the bottom
    // that is not the deleted one; after the move P0's battle area holds the
    // Delay-paid field minus BT20-095 plus the moved stack.
    game.players[0]
        .battle_area
        .iter()
        .find(|p| p.card_sources.first().is_some_and(|c| c.card_id(&game.card_data) == "CHRON"))
        .map(|p| p.top_card().card_id(&game.card_data).to_string())
}

fn count(zone: &[digimon_engine::card_source::CardSource], game: &Game, id: &str) -> usize {
    zone.iter().filter(|c| c.card_id(&game.card_data) == id).count()
}

/// Only one zone holds a candidate: DCGO records no zone bool
/// (`SetBool(canSelectHand)` is not recorded), so "Digivolve? yes" is followed
/// directly by the trash pick.
#[test]
fn bt20_095_delay_digivolve_yes_trash_only_moves_and_digivolves_from_trash() {
    let mut s = session_over(
        delay_offered(false, true),
        vec![
            use_delay(0),
            bool_row(1, true),
            sel(2, serde_json::json!({"prompt": "SelectCardEffect", "card_ids": ["EVOLUTION"]})),
        ],
    );
    replay_all(&mut s);
    assert!(s.game.players[0].breeding_area.is_none(), "DCGO always moves the breeder");
    assert_eq!(moved_top(&s.game).as_deref(), Some("EVOLUTION"));
    assert_eq!(count(&s.game.players[0].trash, &s.game, "EVOLUTION"), 0);
    assert!(s.game.pending_selection.is_none());
}

/// Both zones: "Digivolve? yes", "From trash" (false), the trash pick. The
/// positional reading took the yes for the MOVE prompt and the false as the
/// decline of the digivolution, then met the pick with no prompt.
#[test]
fn bt20_095_delay_digivolve_yes_from_trash_with_both_zones() {
    let mut s = session_over(
        delay_offered(true, true),
        vec![
            use_delay(0),
            bool_row(1, true),
            bool_row(2, false),
            sel(3, serde_json::json!({"prompt": "SelectCardEffect", "card_ids": ["EVOLUTION"]})),
        ],
    );
    replay_all(&mut s);
    assert!(s.game.players[0].breeding_area.is_none());
    assert_eq!(moved_top(&s.game).as_deref(), Some("EVOLUTION"));
    assert_eq!(count(&s.game.players[0].trash, &s.game, "EVOLUTION"), 0, "the trash copy was used");
    assert_eq!(count(&s.game.players[0].hand, &s.game, "EVOLUTION"), 1, "the hand copy stays");
}

/// Both zones: "Digivolve? yes", "From hand" (true), the hand pick.
#[test]
fn bt20_095_delay_digivolve_yes_from_hand_with_both_zones() {
    let mut s = session_over(
        delay_offered(true, true),
        vec![
            use_delay(0),
            bool_row(1, true),
            bool_row(2, true),
            sel(3, serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": ["EVOLUTION"]})),
        ],
    );
    replay_all(&mut s);
    assert_eq!(moved_top(&s.game).as_deref(), Some("EVOLUTION"));
    assert_eq!(count(&s.game.players[0].hand, &s.game, "EVOLUTION"), 0, "the hand copy was used");
    assert_eq!(count(&s.game.players[0].trash, &s.game, "EVOLUTION"), 1, "the trash copy stays");
}

/// "Digivolve? no": DCGO still moved the breeder (no prompt for the move); the
/// positional reading spent the no as the MOVE's decline.
#[test]
fn bt20_095_delay_digivolve_no_still_moves_the_breeder() {
    let mut s = session_over(delay_offered(false, true), vec![use_delay(0), bool_row(1, false)]);
    replay_all(&mut s);
    assert!(s.game.players[0].breeding_area.is_none(), "DCGO always moves the breeder");
    assert_eq!(moved_top(&s.game).as_deref(), Some("CHRON"), "moved, not digivolved");
    assert_eq!(count(&s.game.players[0].trash, &s.game, "EVOLUTION"), 1);
    assert!(s.game.pending_selection.is_none());
}

/// Both zones hold a candidate but no zone bool follows the yes: DCGO would
/// have asked "From which area?", so the recording contradicts the engine.
#[test]
fn bt20_095_delay_missing_zone_bool_is_a_divergence() {
    let mut s = session_over(
        delay_offered(true, true),
        vec![
            use_delay(0),
            bool_row(1, true),
            sel(2, serde_json::json!({"prompt": "SelectCardEffect", "card_ids": ["EVOLUTION"]})),
        ],
    );
    s.step();
    s.step();
    let reason = halted_at(&s, 1);
    assert!(reason.contains("BT20-095"), "{reason}");
    assert!(s.game.players[0].breeding_area.is_some(), "nothing applied for the halted row");
}

/// "From trash" followed by a HAND pick contradicts the zone bool.
#[test]
fn bt20_095_delay_zone_bool_contradicted_by_the_pick_is_a_divergence() {
    let mut s = session_over(
        delay_offered(true, true),
        vec![
            use_delay(0),
            bool_row(1, true),
            bool_row(2, false),
            sel(3, serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": ["EVOLUTION"]})),
        ],
    );
    s.step();
    s.step();
    let reason = halted_at(&s, 1);
    assert!(reason.contains("BT20-095"), "{reason}");
}

/// "Digivolve? yes" with no pick row after it: the decision is missing.
#[test]
fn bt20_095_delay_yes_without_a_pick_row_is_a_divergence() {
    let mut s = session_over(delay_offered(false, true), vec![use_delay(0), bool_row(1, true)]);
    s.step();
    s.step();
    halted_at(&s, 1);
}

/// The pick's own `canNoSelect` (`CardEffectCommons.cs:1083`): DCGO said
/// "Digivolve" and then cancelled the card pick. The breeder is still moved and
/// nothing digivolves.
#[test]
fn bt20_095_delay_yes_then_cancelled_pick_declines_the_digivolution() {
    let mut s = session_over(
        delay_offered(false, true),
        vec![
            use_delay(0),
            bool_row(1, true),
            sel(2, serde_json::json!({"prompt": "SelectCardEffect", "cancel": true})),
        ],
    );
    replay_all(&mut s);
    assert!(s.game.players[0].breeding_area.is_none());
    assert_eq!(moved_top(&s.game).as_deref(), Some("CHRON"));
    assert!(s.game.pending_selection.is_none());
}

// ── (2) a bool answers only a prompt DCGO asks as a yes/no bool ────────────────

/// Main phase, P0 to act, `source` on P0's field, and a hand-built two-branch
/// "do it / don't" `EffectChoice` of `source` parked for P0.
fn yes_no_choice_of(source: CardData, labels: [&str; 2]) -> Game {
    let id = source.card_id.clone();
    let mut r = DebugRunner::builder()
        .add_card(source)
        .add_card(make_test_card("MON", "MON"))
        .deck(0, &["MON"; 5])
        .deck(1, &["MON"; 5])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    let h = r.place_on_field(0, &id, Some(0));
    let source_card = r.top_card(h);
    r.game.pending_selection = Some(PendingSelection {
        card_choice: None,
        zone_owner: None,
        kind: SelectionKind::EffectChoice,
        selecting_player: 0,
        previous_phase: GamePhase::Main,
        valid_action_ids: vec![space::TRASH_EFFECT_START, space::TRASH_EFFECT_START + 1],
        is_optional: false,
        prompt: "test choice".into(),
        effect_choices: Some(
            labels
                .iter()
                .enumerate()
                .map(|(i, l)| EffectChoiceEntry {
                    label: (*l).into(),
                    action_id: space::TRASH_EFFECT_START + i as u16,
                    source_card: Some(source_card),
                    source_kind: Some(EffectSourceKind::Digimon),
                    timing: None,
                    is_optional: false,
                    keyword: None,
                    observation_metadata: Default::default(),
                    count_choice: None,
                })
                .collect(),
        ),
        source_card,
        source_permanent: Some(h),
        source_kind: EffectSourceKind::Digimon,
        callback: Box::new(|_, _| {}),
        on_decline: None,
    });
    r.game
}

/// RizeGreymon (ST24-06): DCGO never asks "Trash 2 face-down cards?" as a bool
/// (ST24_06.cs has no `SetBoolSelection`), so a recorded bool cannot be its
/// answer, whatever its value.
#[test]
fn stamped_bool_never_answers_st24_06_trash_two_choice() {
    for value in [true, false] {
        let mut rize = make_test_card("ST24-06", "RizeGreymon");
        rize.level = Some(5);
        let game = yes_no_choice_of(rize, ["Trash 2 face-down cards", "Don't"]);
        let mut s = session_over(game, vec![bool_row(0, value)]);
        s.step();
        let reason = halted_at(&s, 0);
        assert!(reason.contains("ST24-06"), "{value}: {reason}");
        assert!(s.game.pending_selection.is_some(), "{value}: the choice is still parked");
    }
}

/// Regression guard: a card whose own DCGO script has a yes/no bool menu
/// (AD1-006 is listed in `dcgo_bool_yes_no_cards.json`) keeps the yes/no
/// mapping onto its two-branch choice.
#[test]
fn stamped_bool_answers_the_yes_no_choice_of_a_card_with_a_dcgo_yes_no_menu() {
    for (value, want) in [(true, space::TRASH_EFFECT_START), (false, space::TRASH_EFFECT_START + 1)] {
        let game = yes_no_choice_of(make_test_card("AD1-006", "Listed"), ["Do it", "Don't"]);
        let mut s = session_over(game, vec![bool_row(0, value)]);
        let r = s.step();
        assert!(s.divergences().is_empty() && !s.is_paused(), "{value}: {:?}", s.divergences());
        assert_eq!(r.action_id, want, "{value}");
    }
}

/// A bool "no" against an optional pick of a card whose DCGO script asks no
/// yes/no bool: the positional reading declined the pick (PASS). DCGO's bool
/// was some other question, so the replay stops instead.
#[test]
fn stamped_no_never_declines_an_optional_pick_of_an_unlisted_card() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("MON", "MON"))
        .deck(0, &["MON"; 5])
        .deck(1, &["MON"; 5])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    let h = r.place_on_field(0, "MON", Some(0));
    r.place_on_field(0, "MON", Some(0));
    let source_card = r.top_card(h);
    r.game.pending_selection = Some(PendingSelection {
        card_choice: None,
        zone_owner: None,
        kind: SelectionKind::OwnField,
        selecting_player: 0,
        previous_phase: GamePhase::Main,
        valid_action_ids: vec![space::ATTACK_START, space::ATTACK_START + 1],
        is_optional: true,
        prompt: "Choose 1 of your Digimon".into(),
        effect_choices: None,
        source_card,
        source_permanent: Some(h),
        source_kind: EffectSourceKind::Digimon,
        callback: Box::new(|_, _| {}),
        on_decline: None,
    });
    let mut s = session_over(r.game, vec![bool_row(0, false)]);
    s.step();
    let reason = halted_at(&s, 0);
    assert!(reason.contains("MON"), "{reason}");
    assert!(s.game.pending_selection.is_some(), "not declined");
}
