//! DCGO replay: a pick that names its copy by DCGO card identity still meets
//! the engine's own optional-activation gate first.
//!
//! `duplicate_copy_target` resolves a target naming one of several copies of a
//! card. When the row carries the copy's DCGO `CardIndex` (`targets[].
//! card_index`, written by `convert_raw.py` from a 0.2.0 `board_indexes` or
//! the frame model), the copy is the engine permanent paired with that index.
//! The engine may park a prompt DCGO never asked before the pick - the
//! optional activation of an [On Play] whose DCGO script folds the decline
//! into the pick itself (Falcomon (ST24-12): ST24_12.cs, `canNoSelect: true`
//! on the Tamer pick; the engine's clause is `optional: true`). The ordinary
//! selection path answers that gate (the source omits DCGO-silent prompts);
//! trying the named copy on a bare clone does not, so the identity must pin the
//! engine slot and hand the row to the ordinary path, never stop there nor fall
//! back to pairing copies by position.
//!
//! Field witness: raw-20261113T025510Z#g6 raw seq 1003 (Falcomon (ST24-12)'s
//! [On Play] trashing the bottom face-down card under one of two Marcus Damon &
//! Thomas H. Norstein (ST24-13)). Staged with `DebugRunner` (real ST24-12
//! YAML, synthetic Tamers). No recording content.

use std::collections::HashMap;

use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::game::Game;
use digimon_engine::runners::replay::{
    DivergenceKind, RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec,
};

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Purple];
    c
}

fn filler() -> CardData {
    let mut c = make_test_card("FILLER", "Filler");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c
}

/// P0's main phase, 8 memory. Battle area: [MARCUS with a face-down card under
/// it]. Hand: [MARCUS, ST24-12].
fn board() -> Game {
    let mut r = DebugRunner::builder()
        .dsl_card("ST24-12")
        .expect("ST24-12 production YAML")
        .add_card(tamer("MARCUS"))
        .add_card(filler())
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .security(0, &["FILLER"; 5])
        .security(1, &["FILLER"; 5])
        .memory(8)
        .start();
    r.set_first_player(0);
    let old = r.place_stack(0, &["FILLER", "MARCUS"]);
    r.game.players[0].battle_area[old.index as usize].card_sources[0].face_down = true;
    r.add_to_hand(0, "MARCUS");
    r.add_to_hand(0, "ST24-12");
    r.game.enter_main_phase();
    r.game
}

/// DCGO's CardIndex of the hand MARCUS (any value: DCGO's own numbering).
const PLAYED: i32 = 87;

struct StagedDcgo {
    game: Game,
    steps: Vec<StepSpec>,
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

/// Falcomon's Tamer pick. DCGO's frame order lists the played MARCUS first.
fn tamer_pick(card_index: Option<i32>) -> StepSpec {
    let mut target = serde_json::json!({"player": 0, "frame": 0});
    if let Some(k) = card_index {
        target["card_index"] = serde_json::json!(k);
    }
    let row = serde_json::from_value(serde_json::json!({
        "step": 2, "actor": 0, "phase": "Main", "prompt": "SelectPermanentEffect",
        "targets": [target], "board_p0": ["MARCUS", "MARCUS", "ST24-12"], "board_p1": [],
    }))
    .expect("selection row");
    spec(0, None, Some(row))
}

/// Play the hand MARCUS (DCGO index `PLAYED`); give it a face-down card of its
/// own (staged: both copies then answer Falcomon's pick); play ST24-12; pick.
fn session(pick: StepSpec) -> ReplaySession {
    let game = board();
    let hand = |g: &Game, id: &str| {
        g.players[0].hand.iter().position(|c| c.card_id(&g.card_data) == id).expect("in hand") as u16
    };
    let play_marcus = spec(PLAY_HAND_START + hand(&game, "MARCUS"), Some("MARCUS"), None);
    // After the MARCUS play the hand holds ST24-12 alone (slot 0).
    let play_falcomon = spec(PLAY_HAND_START, Some("ST24-12"), None);
    let steps = vec![play_marcus, play_falcomon, pick];
    let mut s = ReplaySession::with_source(
        Box::new(StagedDcgo { game, steps, action_indexes: vec![Some(PLAYED), None, None] }),
        &HashMap::new(),
        false,
    )
    .expect("staged session");
    s.step();
    let g = &mut s.game;
    let slot = g.players[0].battle_area.len() - 1;
    let data = g.card_data.iter().position(|c| c.card_id == "FILLER").expect("filler");
    let idx = g.next_card_index();
    let mut fd = CardSource::new(data, 0, idx);
    fd.face_down = true;
    g.players[0].battle_area[slot].card_sources.insert(0, fd);
    for _ in 0..2 {
        if s.is_complete() || s.is_paused() {
            break;
        }
        s.step();
    }
    s
}

/// Face-down cards left under each MARCUS: (the older copy, the played copy).
fn face_down_left(game: &Game) -> (usize, usize) {
    let marcus: Vec<usize> = game.players[0]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_id(&game.card_data) == "MARCUS")
        .map(|p| p.card_sources.iter().filter(|c| c.face_down).count())
        .collect();
    assert_eq!(marcus.len(), 2, "both MARCUS stay on the field");
    (marcus[0], marcus[1])
}

/// The pick names the played copy by its DCGO index: the engine answers its
/// own activation gate, then trashes that copy's face-down card - not the
/// older copy's, whatever the position says.
#[test]
fn a_named_copy_behind_the_activation_gate_is_the_one_picked() {
    let s = session(tamer_pick(Some(PLAYED)));
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete(), "every row replays");
    assert_eq!(face_down_left(&s.game), (1, 0), "the played copy paid Falcomon's cost");
}

/// A DCGO index the replay never paired names no engine card: the copies stay
/// for the rows to tell apart, and these rows cannot, so the replay stops.
#[test]
fn an_unpaired_index_behind_the_gate_does_not_pick_by_position() {
    let s = session(tamer_pick(Some(12345)));
    assert_eq!(face_down_left(&s.game), (1, 1), "no copy was charged on a position guess");
    assert!(
        s.is_paused()
            && s.divergences()
                .iter()
                .any(|d| matches!(&d.kind, DivergenceKind::SelectionResolution { .. })),
        "the replay stops; divergences {:?}",
        s.divergences()
    );
}
