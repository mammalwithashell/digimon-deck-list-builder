//! DCGO replay: a Burst Digivolve's Tamer pick names one of several copies of
//! that Tamer, told apart by DCGO's own card identity instead of by position.
//!
//! The recorded boards name a permanent by its card id only, in DCGO's frame
//! order (`GetFieldPermanents()`, Player.cs:669-684), while the engine's battle
//! area is in play order; with several copies of the picked Tamer the position
//! cannot say which (`duplicate_copy_target` stops). DCGO's `CardSource.
//! CardIndex` can: `convert_raw.py` writes it on the rows that move a card
//! (`card_index` on a play action, `card_indexes` on a hand / card pick) and on
//! the burst pick's target (`targets[].card_index`, the returned Tamer read
//! from the owner's next hand listing - DCGO returns the picked Tamer to the
//! hand, `BounceTamer` at CardController.cs:942 - :254-256 is only the
//! `BurstTamer` frame lookup; CR 4.3 8-3). The replay pairs each DCGO
//! index with the engine card its own applied row moved, then resolves the
//! target to the permanent whose top card is that engine card.
//!
//! Field witness: raw-20261113T025510Z#g6, the burst of ShineGreymon: Burst
//! Mode (BT25-104) picking one of three Marcus Damon & Thomas H. Norstein
//! (ST24-13) (raw seq 1026-1034; the returned copy's index appears in the next
//! hand listing). Staged with `DebugRunner` (real BT25-104 YAML, synthetic
//! host and Tamers). No recording content.

use std::collections::HashMap;

use digimon_engine::action::space::{encode_digivolve, PLAY_HAND_START};
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

/// P0's main phase, 8 memory. Battle area: [MARCUS (an older copy with a
/// face-down card under it), HOST]. Hand: [MARCUS, BT25-104]. The rows play
/// the hand MARCUS, so the engine then holds two MARCUS that differ (one has
/// a source), in slots 0 and 2.
fn board() -> Game {
    let mut r = DebugRunner::builder()
        .dsl_card("BT25-104")
        .expect("BT25-104 production YAML")
        .add_card(host())
        .add_card(tamer("MARCUS", "Marcus Damon"))
        .add_card(tamer("OTHER", "Some Other Tamer"))
        .deck(0, &["OTHER"; 10])
        .deck(1, &["OTHER"; 10])
        .security(0, &["OTHER"; 5])
        .security(1, &["OTHER"; 5])
        .memory(8)
        .start();
    r.set_first_player(0);
    let old = r.place_on_field(0, "MARCUS", Some(0));
    r.push_source(old, "OTHER");
    r.place_on_field(0, "HOST", Some(0));
    r.add_to_hand(0, "MARCUS");
    r.add_to_hand(0, "BT25-104");
    r.game.enter_main_phase();
    r.game
}

/// DCGO's CardIndex of the hand MARCUS (any value: DCGO's own numbering).
const PLAYED: i32 = 87;

struct StagedDcgo {
    game: Game,
    steps: Vec<StepSpec>,
    /// DCGO card index of each action row's played card, by row.
    action_indexes: Vec<Option<i32>>,
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
    fn dcgo_action_card_index(&self, row: u32) -> Option<i32> {
        self.action_indexes.get(row as usize).copied().flatten()
    }
}

fn spec(action_id: u16, card_id: Option<&str>, selection: Option<digimon_engine::dcgo_recording::SelectionRow>) -> StepSpec {
    StepSpec {
        actor: 0,
        action_id,
        phase: "Main".into(),
        source: if selection.is_some() { "selection" } else { "main_phase" }.into(),
        memory_after: None,
        dcgo_memory: None,
        turn: None,
        is_game_over: None,
        expected_digest: None,
        selection,
        board_p0: None,
        board_p1: None,
        card_id: card_id.map(str::to_string),
        dna_materials: None,
        inferred: false,
    }
}

/// The burst pick: DCGO's frame order lists the NEWER MARCUS first, so the
/// position (frame 1, its first MARCUS) would pair with the engine's older
/// copy in slot 0.
fn burst_pick(card_index: Option<i32>) -> StepSpec {
    let mut target = serde_json::json!({"player": 0, "frame": 1});
    if let Some(k) = card_index {
        target["card_index"] = serde_json::json!(k);
    }
    let row = serde_json::from_value(serde_json::json!({
        "step": 2, "actor": 0, "phase": "Main", "prompt": "SelectPermanentEffect", "mechanic": "burst",
        "targets": [target], "board_p0": ["HOST", "MARCUS", "MARCUS"], "board_p1": [],
    }))
    .expect("selection row");
    spec(0, None, Some(row))
}

/// Play the hand MARCUS (DCGO index `PLAYED`), burst BT25-104 onto HOST, pick.
fn rows(game: &Game, pick: StepSpec) -> (Vec<StepSpec>, Vec<Option<i32>>) {
    let hand = |id: &str| {
        game.players[0].hand.iter().position(|c| c.card_id(&game.card_data) == id).expect("in hand") as u16
    };
    let host = game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&game.card_data) == "HOST")
        .expect("host") as u16;
    // After the play the hand holds BT25-104 alone (slot 0).
    let _ = hand("BT25-104");
    let play = spec(PLAY_HAND_START + hand("MARCUS"), Some("MARCUS"), None);
    let digivolve = spec(encode_digivolve(0, host), Some("BT25-104"), None);
    (vec![play, digivolve, pick], vec![Some(PLAYED), None, None])
}

fn session(pick: StepSpec) -> ReplaySession {
    let game = board();
    let (steps, action_indexes) = rows(&game, pick);
    let mut s = ReplaySession::with_source(Box::new(StagedDcgo { game, steps, action_indexes }), &HashMap::new(), false)
        .expect("staged session");
    for _ in 0..3 {
        if s.is_complete() || s.is_paused() {
            break;
        }
        s.step();
    }
    s
}

/// The MARCUS permanents left on the field: (has a source under it).
fn marcus_left(game: &Game) -> Vec<bool> {
    game.players[0]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_id(&game.card_data) == "MARCUS")
        .map(|p| p.card_sources.len() > 1)
        .collect()
}

/// The pick's `card_index` is the MARCUS the play row put on the field: the
/// burst returns that copy, wherever DCGO's frame order lists it.
#[test]
fn the_burst_returns_the_copy_its_card_index_names() {
    let s = session(burst_pick(Some(PLAYED)));
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete(), "every row replays");
    assert_eq!(marcus_left(&s.game), vec![true], "the older copy (with its source) stays; the played one returned");
    assert!(
        s.game.players[0].hand.iter().any(|c| c.card_id(&s.game.card_data) == "MARCUS"),
        "the played copy is back in the hand"
    );
}

/// Without the card index the two copies (one with a source) cannot be told
/// apart from the rows: the replay stops rather than pair them by position.
#[test]
fn without_a_card_index_the_copies_stop_the_replay() {
    let s = session(burst_pick(None));
    assert!(s.is_paused(), "the replay stops; divergences {:?}", s.divergences());
    assert!(
        s.divergences()
            .iter()
            .any(|d| matches!(&d.kind, DivergenceKind::SelectionResolution { reason } if reason.contains("copies"))),
        "{:?}",
        s.divergences()
    );
}

/// A card index the replay never saw move names no engine card: the pick is
/// left to the copy evidence of the rows, which cannot decide here, so the
/// replay stops (never a position guess).
#[test]
fn an_unknown_card_index_falls_back_to_the_copy_evidence() {
    let s = session(burst_pick(Some(12345)));
    assert!(s.is_paused(), "the replay stops; divergences {:?}", s.divergences());
}
