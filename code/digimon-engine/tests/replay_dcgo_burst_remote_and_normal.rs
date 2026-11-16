//! DCGO replay: a REMOTE seat's Burst Digivolve, and the "Normal" answer to the
//! Normal / Burst panel.
//!
//! Remote seat. The digivolver's own Tamer pick
//! (`SelectBurstDigivolutionEffect.SelectTamer`, `isLocal: true`, :107-190)
//! runs on its own client only, so a recorder on the OTHER client logs no pick.
//! `convert_raw.py` rebuilds it from the play packet's `BurstTamerFrameID`
//! (TurnStateMachine.cs:3016-3030) through the frame model and writes it as a
//! `SelectPermanentEffect` row with `mechanic: "burst"` and
//! `synthesized: "burst_tamer_frame"` right after the digivolve action, exactly
//! where a local seat's own pick lands. The replay must answer the engine's
//! "Choose which digivolution cost to pay" prompt from it for the SECOND seat
//! as it does for the first (CR 4.3 8-3-3: return the specified Tamer, pay the
//! cost). The row below is the exact shape the converter writes
//! (`code/tests/tools/test_dcgo_convert_raw_burst_remote.py`).
//!
//! Normal panel. DCGO asks "With which method would you like to Digivolve?"
//! (`SelectWheterToBurst`, a `selectCardPanel`, TurnStateMachine.cs:2874-2882)
//! before anything else; no recorder hook sees that panel. Choosing "Normal"
//! queues a play packet with `BurstTamerFrameID` -1 and logs NO Tamer pick, so
//! the recording is the digivolve action alone. The engine, which lists the
//! burst route beside the normal one whenever a Tamer qualifies, parks the cost
//! choice; the replay answers it with the ordinary route
//! (`answer_dcgo_ordinary_cost_route`), because DCGO paid the printed cost.
//!
//! Staged with `DebugRunner` (real BT25-104 YAML, synthetic host and Tamers).
//! No recording content.

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

/// `seat`'s main phase, 8 memory: [MARCUS-A, HOST, MARCUS-B, OTHER] in its
/// battle area, BT25-104 in its hand. Both Marcus Damon Tamers can pay the burst.
fn board(seat: u8) -> (Game, u16) {
    board_with_memory(seat, 8)
}

/// `board` with `memory` on `seat`'s side of the gauge.
fn board_with_memory(seat: u8, memory: i16) -> (Game, u16) {
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
        .memory(memory)
        .start();
    r.set_first_player(seat);
    r.place_on_field(seat, "MARCUS-A", Some(0));
    let h = r.place_on_field(seat, "HOST", Some(0));
    r.place_on_field(seat, "MARCUS-B", Some(0));
    r.place_on_field(seat, "OTHER", Some(0));
    let hand = r.add_to_hand(seat, "BT25-104");
    r.game.enter_main_phase();
    (r.game, encode_digivolve(hand as u16, h.index as u16))
}

struct StagedDcgo {
    game: Game,
    steps: Vec<StepSpec>,
    /// DCGO card index of each action row's played card, by row (empty: none known).
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

fn spec(actor: u8, action_id: u16, source: &str, selection: Option<digimon_engine::dcgo_recording::SelectionRow>) -> StepSpec {
    StepSpec {
        actor: actor as _,
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

/// The row `convert_raw.py` writes for a remote seat (`seat`): the Tamer at
/// DCGO's battle position `frame` of `seat`'s own board, named by the frame
/// model from the play packet's `BurstTamerFrameID`.
fn synthesized_pick(seat: u8, frame: i32) -> StepSpec {
    let board = ["MARCUS-A", "HOST", "MARCUS-B", "OTHER"];
    let (b0, b1): (Vec<&str>, Vec<&str>) = if seat == 0 { (board.to_vec(), vec![]) } else { (vec![], board.to_vec()) };
    let row = serde_json::from_value(serde_json::json!({
        "step": 1, "actor": seat, "phase": "Main", "prompt": "SelectPermanentEffect", "mechanic": "burst",
        "synthesized": "burst_tamer_frame",
        "targets": [{"player": seat, "frame": frame}],
        "board_p0": b0, "board_p1": b1,
    }))
    .expect("selection row");
    spec(seat, 0, "selection", Some(row))
}

fn staged(game: Game, steps: Vec<StepSpec>) -> ReplaySession {
    ReplaySession::with_source(Box::new(StagedDcgo { game, steps, action_indexes: vec![] }), &HashMap::new(), false)
        .expect("staged session")
}

fn ids(cards: &[digimon_engine::card_source::CardSource], game: &Game) -> Vec<String> {
    cards.iter().map(|c| c.card_id(&game.card_data).to_string()).collect()
}

fn field(game: &Game, seat: usize) -> Vec<String> {
    game.players[seat].battle_area.iter().map(|p| p.top_card().card_id(&game.card_data).to_string()).collect()
}

/// The remote seat (P1) bursts: the synthesized pick (frame 2 = MARCUS-B)
/// answers the route prompt and returns that Tamer, burst cost 0.
#[test]
fn a_remote_seats_synthesized_pick_names_the_burst_route_that_returns_it() {
    let (game, digivolve) = board(1);
    let mut s = staged(game, vec![spec(1, digivolve, "main_phase", None), synthesized_pick(1, 2)]);
    for at in 0..2 {
        s.step();
        assert!(s.divergences().is_empty() && !s.is_paused(), "row {at}: {:?}", s.divergences());
    }
    assert!(ids(&s.game.players[1].hand, &s.game).contains(&"MARCUS-B".to_string()));
    let f = field(&s.game, 1);
    assert!(f.contains(&"MARCUS-A".to_string()) && !f.contains(&"MARCUS-B".to_string()), "{f:?}");
    assert!(f.contains(&"BT25-104".to_string()), "{f:?}");
    assert_eq!(s.game.memory, 8, "burst cost 0: no memory spent");
}

/// The other copy for the other frame, still on the remote seat.
#[test]
fn a_remote_seats_other_frame_returns_the_other_tamer() {
    let (game, digivolve) = board(1);
    let mut s = staged(game, vec![spec(1, digivolve, "main_phase", None), synthesized_pick(1, 0)]);
    for _ in 0..2 {
        s.step();
    }
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    let f = field(&s.game, 1);
    assert!(f.contains(&"MARCUS-B".to_string()) && !f.contains(&"MARCUS-A".to_string()), "{f:?}");
}

/// A pick naming a permanent no burst route returns (OTHER, frame 3) still halts for the remote seat.
#[test]
fn a_remote_pick_naming_no_burst_routes_tamer_halts() {
    let (game, digivolve) = board(1);
    let mut s = staged(game, vec![spec(1, digivolve, "main_phase", None), synthesized_pick(1, 3)]);
    s.step();
    s.step();
    match s.divergences() {
        [d] => assert!(matches!(d.kind, DivergenceKind::SelectionResolution { .. }), "{d:?}"),
        other => panic!("expected one divergence, got {other:?}"),
    }
    assert!(s.is_paused());
    assert!(field(&s.game, 1).contains(&"HOST".to_string()), "nothing digivolved");
}

/// "Normal" on the Normal / Burst panel: the recording is the digivolve action
/// alone (no Tamer pick), followed by the seat's next decision (here PASS). The
/// engine offered the burst route beside the normal one; DCGO paid the printed
/// cost (5), so the replay takes the ordinary route before applying the next
/// row: both Tamers stay on the field, none is returned to the hand, and the
/// 5 memory is spent. (The answer is given when the NEXT row is met,
/// `answer_dcgo_ordinary_cost_route`, so a lone trailing digivolve row proves
/// nothing and the test carries a PASS.) The seat starts at 0 memory: paying
/// the printed 5 leaves it at -5, and a pass at negative memory hands the
/// opponent exactly that 5 (a pass at 0 or more would set 3), so 5 on the
/// opponent's side proves the printed route was paid and not the 0-cost burst.
#[test]
fn choosing_normal_on_the_panel_keeps_every_tamer_and_pays_the_printed_route() {
    for seat in [0u8, 1u8] {
        let (game, digivolve) = board_with_memory(seat, 0);
        let mut s = staged(
            game,
            vec![
                spec(seat, digivolve, "main_phase", None),
                spec(seat, digimon_engine::action::space::PASS, "main_phase", None),
            ],
        );
        s.step();
        s.step();
        assert!(s.divergences().is_empty() && !s.is_paused(), "seat {seat}: {:?}", s.divergences());
        assert_eq!(s.game.memory.abs(), 5, "seat {seat}: the printed 5 was paid (a 0-cost burst pass would leave 3)");
        let f = field(&s.game, seat as usize);
        assert!(f.contains(&"MARCUS-A".to_string()) && f.contains(&"MARCUS-B".to_string()), "seat {seat}: {f:?}");
        assert!(f.contains(&"BT25-104".to_string()) && !f.contains(&"HOST".to_string()), "seat {seat}: digivolved: {f:?}");
        assert!(
            !ids(&s.game.players[seat as usize].hand, &s.game).iter().any(|c| c.starts_with("MARCUS")),
            "seat {seat}: no Tamer returned to the hand"
        );
    }
}

/// DCGO's CardIndex of the hand MARCUS the remote seat plays (any value: DCGO's own numbering).
const PLAYED: i32 = 87;

/// The remote seat's board for a burst among COPIES of one Tamer: [MARCUS (an older copy with a face-down card under
/// it), HOST], hand [MARCUS, BT25-104].
fn copies_board(seat: u8) -> Game {
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
    r.set_first_player(seat);
    let old = r.place_on_field(seat, "MARCUS", Some(0));
    r.push_source(old, "OTHER");
    r.place_on_field(seat, "HOST", Some(0));
    r.add_to_hand(seat, "MARCUS");
    r.add_to_hand(seat, "BT25-104");
    r.game.enter_main_phase();
    r.game
}

/// The exact selection row `convert_raw.py` writes for a remote seat's burst when the Tamer has copies: the frame
/// model names the permanent's battle position AND DCGO's CardIndex (`card_index_source: "frame_model"`,
/// `test_remote_burst_tamer_is_named_from_the_packet_frame_and_noted`). DCGO's frame order lists the NEWER copy first,
/// so position 1 would pair with the engine's older copy; the CardIndex says the copy just played.
fn frame_model_pick(seat: u8, card_index: i32) -> StepSpec {
    let board = ["HOST", "MARCUS", "MARCUS"];
    let (b0, b1): (Vec<&str>, Vec<&str>) = if seat == 0 { (board.to_vec(), vec![]) } else { (vec![], board.to_vec()) };
    let row = serde_json::from_value(serde_json::json!({
        "step": 2, "actor": seat, "phase": "Main", "prompt": "SelectPermanentEffect", "mechanic": "burst",
        "synthesized": "burst_tamer_frame",
        "targets": [{"player": seat, "frame": 1, "card_index": card_index, "card_index_source": "frame_model"}],
        "board_p0": b0, "board_p1": b1,
    }))
    .expect("selection row");
    spec(seat, 0, "selection", Some(row))
}

fn marcus_left(game: &Game, seat: usize) -> Vec<bool> {
    game.players[seat]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_id(&game.card_data) == "MARCUS")
        .map(|p| p.card_sources.len() > 1)
        .collect()
}

/// The remote seat's synthesized pick carries the converter's `card_index` + `card_index_source`: the burst returns the
/// copy DCGO's CardIndex names (the one the play row put on the field), not the copy the position would suggest.
#[test]
fn a_remote_seats_frame_model_card_index_names_the_copy_the_burst_returns() {
    let game = copies_board(1);
    let hand = |id: &str| game.players[1].hand.iter().position(|c| c.card_id(&game.card_data) == id).expect("in hand") as u16;
    let host_slot = game.players[1]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&game.card_data) == "HOST")
        .expect("host") as u16;
    let mut play = spec(1, PLAY_HAND_START + hand("MARCUS"), "main_phase", None);
    play.card_id = Some("MARCUS".into());
    let mut digivolve = spec(1, encode_digivolve(0, host_slot), "main_phase", None);
    digivolve.card_id = Some("BT25-104".into());
    let steps = vec![play, digivolve, frame_model_pick(1, PLAYED)];
    let mut s = ReplaySession::with_source(
        Box::new(StagedDcgo { game, steps, action_indexes: vec![Some(PLAYED), None, None] }),
        &HashMap::new(),
        false,
    )
    .expect("staged session");
    for _ in 0..3 {
        if s.is_complete() || s.is_paused() {
            break;
        }
        s.step();
    }
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete(), "every row replays");
    assert_eq!(marcus_left(&s.game, 1), vec![true], "the older copy (with its source) stays; the played one returned");
    assert!(ids(&s.game.players[1].hand, &s.game).contains(&"MARCUS".to_string()), "the played copy is back in the hand");
}
