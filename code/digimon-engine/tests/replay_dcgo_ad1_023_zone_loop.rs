//! DCGO replay: J.P., Koji, & Koichi (AD1-023)'s "[On Play] [Start of Your
//! Main Phase] You may place up to 2 [Hybrid] trait cards with different
//! colors from your hand or trash under this Tamer", answered from DCGO's
//! per-card loop.
//!
//! DCGO (`CardEffect/AD1/Black/AD1_023.cs`, `SharedActivateCoroutine`):
//! `while (selectedCards.Count < 2)` it lists the hand and trash cards that
//! are still choosable (`CanSelectCardCondition`: [Hybrid], not already
//! chosen, a colour other than the first choice's); with none in either zone
//! it leaves the loop WITHOUT asking (`goto END_LOOP`). Otherwise it asks the
//! int menu "From which area ...?" (from Hand = 1, only with a hand
//! candidate; from Trash = 2, only with a trash candidate; Do not place = 3)
//! and then ONE card (`maxCount: 1`, `canNoSelect: true`) in that zone -
//! `SelectHandEffect` for the hand, `SelectCardEffect` rooted at the trash.
//! Menu 3 ends the loop. The chosen cards go under the Tamer at the bottom in
//! pick order (`AddDigivolutionCardsBottom(selectedCards)`, AD1_023.cs:175):
//! each is appended to `cardSources` (Permanent.cs:1246), whose index 0 is the
//! top (`TopCard`, :1407-1414), so the LAST pick ends up at the very bottom.
//!
//! The engine asks ONE optional up-to-2 pick over the hand and the trash
//! (AD1-023.yaml `select_zone_cards`, `distinct_by: color`), so the whole
//! loop answers it. Field witness: raw-20261113T011922Z#g3 raw seq 364-368
//! (menu 2, trash pick, menu 2, trash pick), where the replay closed the
//! engine pick after the first card and then met the second menu with
//! nothing parked.
//!
//! Staged with `DebugRunner` (real AD1-023 YAML, synthetic [Hybrid] cards).
//! No recording content.

use std::collections::HashMap;

use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::card_data::CardData;
use digimon_engine::dcgo_recording::SelectionRow;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::game::Game;
use digimon_engine::runners::replay::{DivergenceKind, RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec};

const TAMER: &str = "AD1-023";

fn hybrid(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.colors = vec![color];
    c.traits = vec!["Hybrid".to_string()];
    c
}

struct StagedDcgo {
    game: Game,
    steps: Vec<StepSpec>,
}

impl RecordingSource for StagedDcgo {
    fn build_initial_game(&self, _: &HashMap<String, CardData>) -> Result<Game, ReplayError> {
        Ok(self.game.clone())
    }
    fn relay_initial_state(&self, game: &mut Game) -> Result<(), ReplayError> {
        *game = self.game.clone();
        Ok(())
    }
    fn steps(&self) -> &[StepSpec] {
        &self.steps
    }
    fn default_policy(&self) -> StepPolicy {
        StepPolicy::CheckThenApply
    }
    fn omits_dcgo_silent_prompts(&self) -> bool {
        true
    }
}

fn spec(action_id: u16, source: &str, selection: Option<SelectionRow>) -> StepSpec {
    StepSpec {
        actor: 0,
        action_id,
        phase: "Main".into(),
        source: source.into(),
        memory_after: None,
        dcgo_memory: None,
        turn: None,
        is_game_over: None,
        expected_digest: None,
        selection,
        board_p0: None,
        board_p1: None,
        card_id: None,
        dna_materials: None,
        inferred: false,
    }
}

fn menu(v: i64) -> StepSpec {
    let s: SelectionRow =
        serde_json::from_value(serde_json::json!({"step":0,"actor":0,"prompt":"generic_int","int_value":v})).unwrap();
    spec(0, "selection", Some(s))
}

fn trash_pick(id: &str) -> StepSpec {
    let s: SelectionRow = serde_json::from_value(
        serde_json::json!({"step":0,"actor":0,"prompt":"SelectCardEffect","card_ids":[id]}),
    )
    .unwrap();
    spec(0, "selection", Some(s))
}

fn hand_pick(id: &str) -> StepSpec {
    let s: SelectionRow = serde_json::from_value(
        serde_json::json!({"step":0,"actor":0,"prompt":"SelectHandEffect","zone":"Hand","card_ids":[id]}),
    )
    .unwrap();
    spec(0, "selection", Some(s))
}

/// P0 holds AD1-023 (hand slot 0) and `hand`; `trash` is in P0's trash.
fn staged(hand: &[&str], trash: &[&str]) -> Game {
    let mut cards = vec![TAMER];
    cards.extend_from_slice(hand);
    let mut runner = DebugRunner::builder()
        .dsl_card(TAMER)
        .expect("AD1-023 YAML loads")
        .add_card(hybrid("HYB-RED", CardColor::Red))
        .add_card(hybrid("HYB-BLUE", CardColor::Blue))
        .add_card(hybrid("HYB-RED2", CardColor::Red))
        .add_card(make_test_card("MISS", "MISS"))
        .hand(0, &cards)
        .deck(0, &["MISS"; 6])
        .deck(1, &["MISS"; 6])
        .memory(10)
        .start();
    for id in trash {
        runner.inject_trash(0, id);
    }
    runner.game
}

fn play_tamer() -> StepSpec {
    spec(PLAY_HAND_START, "main_phase", None)
}

fn run(game: Game, steps: Vec<StepSpec>) -> ReplaySession {
    let mut s = ReplaySession::with_source(Box::new(StagedDcgo { game, steps }), &HashMap::new(), false)
        .expect("staged session");
    for _ in 0..20 {
        if s.is_complete() || s.is_paused() {
            break;
        }
        s.step();
    }
    s
}

/// The tamer's digivolution cards, bottom first.
fn under_tamer(game: &Game) -> Vec<String> {
    let p = game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&game.card_data) == TAMER)
        .expect("AD1-023 on the field");
    p.card_sources.iter().map(|c| c.card_id(&game.card_data).to_string()).filter(|c| c != TAMER).collect()
}

fn selection_reason(s: &ReplaySession) -> String {
    s.divergences()
        .iter()
        .find_map(|d| match &d.kind {
            DivergenceKind::SelectionResolution { reason } => Some(reason.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a SelectionResolution divergence: {:?}", s.divergences()))
}

/// The g3 shape: menu 2, a trash card, menu 2, a trash card of another
/// colour. Both go under the Tamer; the second pick is the very bottom.
#[test]
fn two_trash_passes_answer_the_one_engine_pick() {
    let s = run(
        staged(&[], &["HYB-RED", "HYB-BLUE"]),
        vec![play_tamer(), menu(2), trash_pick("HYB-BLUE"), menu(2), trash_pick("HYB-RED")],
    );
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete(), "every row replays");
    assert_eq!(under_tamer(&s.game), vec!["HYB-RED", "HYB-BLUE"], "bottom first: the last pick is the bottom");
}

/// A hand pass and a trash pass: each card is taken from the zone its row
/// names.
#[test]
fn a_hand_pass_then_a_trash_pass() {
    let s = run(
        staged(&["HYB-RED"], &["HYB-BLUE"]),
        vec![play_tamer(), menu(1), hand_pick("HYB-RED"), menu(2), trash_pick("HYB-BLUE")],
    );
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert_eq!(under_tamer(&s.game), vec!["HYB-BLUE", "HYB-RED"], "bottom first: the last pick is the bottom");
    assert!(s.game.players[0].trash.is_empty(), "the trash card left the trash");
}

/// One card, then "Do not place" (3): the engine pick is closed with that
/// one card.
#[test]
fn one_card_then_do_not_place() {
    let s = run(
        staged(&[], &["HYB-RED", "HYB-BLUE"]),
        vec![play_tamer(), menu(2), trash_pick("HYB-RED"), menu(3)],
    );
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete());
    assert_eq!(under_tamer(&s.game), vec!["HYB-RED"]);
}

/// One card, and no card of another colour is left in either zone: DCGO
/// leaves the loop without asking (`goto END_LOOP`), so no menu row
/// follows. The engine pick, which offers nothing more, is closed.
#[test]
fn one_card_with_no_candidate_left_ends_without_a_menu() {
    let s = run(
        staged(&[], &["HYB-RED", "HYB-RED2"]),
        vec![play_tamer(), menu(2), trash_pick("HYB-RED")],
    );
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete());
    assert_eq!(under_tamer(&s.game), vec!["HYB-RED"]);
}

/// One card while a card of another colour is still choosable, and no
/// menu follows: DCGO would have asked again, so the replay stops instead
/// of closing the pick.
#[test]
fn a_missing_menu_while_a_candidate_remains_is_a_divergence() {
    let s = run(
        staged(&[], &["HYB-RED", "HYB-BLUE"]),
        vec![play_tamer(), menu(2), trash_pick("HYB-RED"), play_tamer()],
    );
    assert!(s.is_paused(), "the replay stops");
    let reason = selection_reason(&s);
    assert!(reason.contains("AD1-023"), "{reason}");
}

/// "Do not place" first: the optional pick is declined, nothing goes under.
#[test]
fn do_not_place_first_declines() {
    let s = run(staged(&[], &["HYB-RED", "HYB-BLUE"]), vec![play_tamer(), menu(3)]);
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete());
    assert!(under_tamer(&s.game).is_empty());
}
