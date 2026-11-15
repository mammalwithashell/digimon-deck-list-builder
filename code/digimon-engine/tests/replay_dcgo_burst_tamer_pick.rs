//! DCGO replay: a Burst Digivolve's returned Tamer, recorded as the seat's own
//! `SelectPermanentEffect` pick, answers the engine's "which digivolution cost
//! to pay?" route choice.
//!
//! DCGO's human UI picks the Tamer to return BEFORE it queues the play
//! (`SelectBurstDigivolutionEffect.SelectTamer`, a `SelectPermanentEffect` over
//! the seat's own Tamers that meet the burst condition, SelectBurstDigivolution
//! Effect.cs:107-190) and queues the `PlayCardAction` with that Tamer's
//! FrameID (TurnStateMachine.cs:3016-3030); the play returns exactly that
//! Tamer (`BurstTamer` looks it up by frame, CardController.cs:252-258;
//! `BounceTamer` returns it, :942). CR 4.3 8-3-3: return the specified
//! Tamer, pay the cost. The engine offers one "Burst Digivolve ... (return
//! [X] in slot N to the hand)" route per eligible Tamer. `convert_raw.py`
//! writes the pick right after the digivolve action, so it meets that route
//! prompt: the route whose Tamer is the picked permanent is the answer.
//! raw-20261113T025510Z#g6 replay step 125 (raw seq 1026-1029: ShineGreymon:
//! Burst Mode (BT25-104) onto ShineGreymon (ST24-07), returning one of three
//! Marcus Damon & Thomas H. Norstein (ST24-13)).
//!
//! Staged with `DebugRunner` (real BT25-104 YAML, synthetic host and Tamers).
//! No recording content.

use std::collections::HashMap;

use digimon_engine::action::space::encode_digivolve;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::game::Game;
use digimon_engine::runners::replay::{
    DivergenceKind, RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec,
};

fn tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::White];
    c
}

fn host() -> CardData {
    let mut c = make_test_card("HOST", "ShineGreymon");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(6);
    c.dp = Some(12000);
    c.play_cost = 12;
    c.traits = vec!["DATA SQUAD".into()];
    c
}

/// P0's main phase, 8 memory: [MARCUS-A, HOST, MARCUS-B, OTHER] in the battle
/// area, BT25-104 in hand. Both Marcus Damon Tamers can pay the burst.
fn board() -> (Game, u16) {
    let mut r = DebugRunner::builder()
        .dsl_card("BT25-104")
        .expect("BT25-104 production YAML")
        .add_card(host())
        .add_card(tamer("MARCUS-A", "Marcus Damon"))
        .add_card(tamer("MARCUS-B", "Marcus Damon"))
        .add_card(tamer("OTHER", "Some Other Tamer"))
        .deck(0, &["OTHER"; 10])
        .deck(1, &["OTHER"; 10])
        .security(0, &["OTHER"; 5])
        .security(1, &["OTHER"; 5])
        .memory(8)
        .start();
    r.set_first_player(0);
    r.place_on_field(0, "MARCUS-A", Some(0));
    let h = r.place_on_field(0, "HOST", Some(0));
    r.place_on_field(0, "MARCUS-B", Some(0));
    r.place_on_field(0, "OTHER", Some(0));
    let hand = r.add_to_hand(0, "BT25-104");
    r.game.enter_main_phase();
    (r.game, encode_digivolve(hand as u16, h.index as u16))
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

fn spec(action_id: u16, source: &str, selection: Option<digimon_engine::dcgo_recording::SelectionRow>) -> StepSpec {
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

fn tamer_pick(frame: i32) -> StepSpec {
    let row = serde_json::from_value(serde_json::json!({
        "step": 1, "actor": 0, "phase": "Main", "prompt": "SelectPermanentEffect", "mechanic": "burst",
        "targets": [{"player": 0, "frame": frame}],
        "board_p0": ["MARCUS-A", "HOST", "MARCUS-B", "OTHER"], "board_p1": [],
    }))
    .expect("selection row");
    spec(0, "selection", Some(row))
}

fn staged(game: Game, steps: Vec<StepSpec>) -> ReplaySession {
    ReplaySession::with_source(Box::new(StagedDcgo { game, steps }), &HashMap::new(), false)
        .expect("staged session")
}

fn ids(cards: &[digimon_engine::card_source::CardSource], game: &Game) -> Vec<String> {
    cards.iter().map(|c| c.card_id(&game.card_data).to_string()).collect()
}

fn field(game: &Game) -> Vec<String> {
    game.players[0].battle_area.iter().map(|p| p.top_card().card_id(&game.card_data).to_string()).collect()
}

/// The pick names MARCUS-B (frame 2): the burst route that returns it.
#[test]
fn the_tamer_pick_names_the_burst_route_that_returns_it() {
    let (game, digivolve) = board();
    let mut s = staged(game, vec![spec(digivolve, "main_phase", None), tamer_pick(2)]);
    for at in 0..2 {
        s.step();
        assert!(s.divergences().is_empty() && !s.is_paused(), "row {at}: {:?}", s.divergences());
    }
    assert!(ids(&s.game.players[0].hand, &s.game).contains(&"MARCUS-B".to_string()), "{:?}", ids(&s.game.players[0].hand, &s.game));
    let f = field(&s.game);
    assert!(f.contains(&"MARCUS-A".to_string()) && !f.contains(&"MARCUS-B".to_string()), "{f:?}");
    assert!(f.contains(&"BT25-104".to_string()), "{f:?}");
    assert_eq!(s.game.memory, 8, "burst cost 0");
}

/// The same with MARCUS-A (frame 0).
#[test]
fn the_other_tamer_pick_returns_the_other_tamer() {
    let (game, digivolve) = board();
    let mut s = staged(game, vec![spec(digivolve, "main_phase", None), tamer_pick(0)]);
    for _ in 0..2 {
        s.step();
    }
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    let f = field(&s.game);
    assert!(f.contains(&"MARCUS-B".to_string()) && !f.contains(&"MARCUS-A".to_string()), "{f:?}");
}

/// A pick naming a Tamer no burst route returns (OTHER, frame 3) halts.
#[test]
fn a_pick_naming_no_burst_routes_tamer_halts() {
    let (game, digivolve) = board();
    let mut s = staged(game, vec![spec(digivolve, "main_phase", None), tamer_pick(3)]);
    s.step();
    s.step();
    match s.divergences() {
        [d] => assert!(matches!(d.kind, DivergenceKind::SelectionResolution { .. }) && d.step == 1, "{d:?}"),
        other => panic!("expected one divergence, got {other:?}"),
    }
    assert!(s.is_paused());
    assert!(field(&s.game).contains(&"HOST".to_string()), "nothing digivolved");
}
