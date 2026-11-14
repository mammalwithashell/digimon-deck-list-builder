//! DCGO replay: a recorded target naming one of two copies of a card.
//!
//! The recorded boards name each permanent by its card id only, in DCGO's
//! frame order (`GetFieldPermanents()`, Player.cs:41-54 / :669-684), while the
//! engine's battle area is in play order. With two copies of one card on a
//! board the id cannot tell them apart, and the occurrence-order slot map
//! (`build_slot_map`) silently paired DCGO's first copy with the engine's
//! first - which need not be the same permanent.
//!
//! Field witness: raw-20261113T011922Z#g3 raw seq 409-411. Owen Dreadnought
//! (BT21-081)'s [End of Your Turn] gives a [Reptile]/[Dragonkin] Digimon
//! <Piercing> and makes it attack, if it can (BT21_081.cs:98-145: the pick
//! offers every such Digimon, then `selectedPermanent.CanAttack` gates the
//! attack). DCGO picked frame 0 of `['EX11-012', 'EX11-012', ...]` - the
//! Medusamon (EX11-012) on the newer frame, unsuspended - and it attacked;
//! the replay paired it with the engine's first Medusamon, which had already
//! attacked and was suspended, so no attack followed and DCGO's attack row
//! met nothing.
//!
//! The rows that follow are the evidence: each copy is tried, and only a copy
//! the next rows accept is DCGO's (the trigger-order lookahead, run over
//! copies instead of order entries). When they cannot tell, the replay
//! stops.
//!
//! Staged with `DebugRunner` (real BT21-081 YAML, synthetic other cards). No
//! recording content.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::dcgo_recording::SelectionRow;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::game::Game;
use digimon_engine::runners::replay::{DivergenceKind, RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec};
use digimon_engine::selection::SelectionKind;

fn reptile(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(9000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Reptile".to_string()];
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

fn row(json: serde_json::Value) -> StepSpec {
    let selection: SelectionRow = serde_json::from_value(json).expect("selection row");
    StepSpec {
        actor: selection.actor,
        action_id: 0,
        phase: "Main".into(),
        source: "selection".into(),
        memory_after: None,
        dcgo_memory: None,
        turn: None,
        is_game_over: None,
        expected_digest: None,
        selection: Some(selection),
        board_p0: None,
        board_p1: None,
        card_id: None,
        dna_materials: None,
        inferred: false,
    }
}

/// P0: Owen Dreadnought (BT21-081) in slot 0, then two copies of a
/// [Reptile] Digimon; the copy in slot `suspended_slot` is suspended. P1: one
/// suspended Digimon. P0's turn ends, and Owen's "use?" gate is parked.
fn staged(suspended_slot: Option<usize>) -> Game {
    staged_with(suspended_slot, false)
}

/// As `staged`; with `second_has_source` the second copy carries a
/// digivolution card, so the two copies are different permanents.
fn staged_with(suspended_slot: Option<usize>, second_has_source: bool) -> Game {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-081")
        .expect("BT21-081 YAML loads")
        .add_card(reptile("REP"))
        .add_card(reptile("VICTIM"))
        .add_card(make_test_card("MISS", "MISS"))
        .deck(0, &["MISS"; 6])
        .deck(1, &["MISS"; 6])
        .security(1, &["MISS", "MISS"])
        .memory(3)
        .start();
    runner.place_on_field(0, "BT21-081", None);
    // Both copies have been in play since an earlier turn (no summoning sickness).
    runner.place_on_field(0, "REP", Some(0));
    let second = runner.place_on_field(0, "REP", Some(0));
    if second_has_source {
        runner.push_source(second, "MISS");
    }
    // P1 has a suspended Digimon, so the attack has two targets and the
    // engine asks.
    let victim = runner.place_on_field(1, "VICTIM", None);
    runner.game.players[1].battle_area[victim.index as usize].is_suspended = true;
    if let Some(slot) = suspended_slot {
        runner.game.players[0].battle_area[slot].is_suspended = true;
    }
    runner.game.end_turn();
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Replacement), "Owen's gate is parked");
    runner.game
}

fn session(game: Game, steps: Vec<StepSpec>) -> ReplaySession {
    let mut s = ReplaySession::with_source(Box::new(StagedDcgo { game, steps }), &HashMap::new(), false)
        .expect("staged session");
    for _ in 0..8 {
        if s.is_complete() || s.is_paused() {
            break;
        }
        s.step();
    }
    s
}

fn dcgo_rows() -> Vec<StepSpec> {
    vec![
        row(serde_json::json!({"step":0,"actor":0,"prompt":"generic_bool","bool_value":true,"unstamped":true})),
        // DCGO's board lists the unsuspended copy first.
        row(serde_json::json!({"step":1,"actor":0,"prompt":"SelectPermanentEffect",
            "board_p0":["REP","REP","BT21-081"], "targets":[{"player":0,"frame":0}]})),
        row(serde_json::json!({"step":2,"actor":0,"prompt":"SelectAttackEffect","targets":[{"player":1,"frame":-1}]})),
    ]
}

/// The engine's first copy (slot 1) is suspended and cannot attack, so only
/// the second copy (slot 2) leads to the attack DCGO recorded next.
#[test]
fn the_copy_the_next_rows_prove_is_the_one_picked() {
    let s = session(staged(Some(1)), dcgo_rows());
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete(), "every row replays");
    assert!(s.game.players[0].battle_area[2].is_suspended, "the second copy attacked");
}

/// Both copies unsuspended but different (the second has a digivolution
/// card): either attacks, and the rows cannot tell which DCGO picked, so the
/// replay stops instead of pairing the copies by position.
#[test]
fn copies_the_rows_cannot_tell_apart_stop_the_replay() {
    let s = session(staged_with(None, true), dcgo_rows());
    assert!(s.is_paused(), "the replay stops; divergences {:?}", s.divergences());
    let reason = s
        .divergences()
        .iter()
        .find_map(|d| match &d.kind {
            DivergenceKind::SelectionResolution { reason } => Some(reason.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a SelectionResolution divergence: {:?}", s.divergences()));
    assert!(reason.contains("REP"), "{reason}");
    assert!(reason.contains("copies"), "{reason}");
}


/// Two copies identical in every respect (same stack, both unsuspended, both
/// in play since the same turn, no effect on either): which one DCGO picked
/// cannot change the game, so the first is taken, with a note.
#[test]
fn interchangeable_copies_are_taken_in_order_with_a_note() {
    let s = session(staged(None), dcgo_rows());
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete(), "every row replays");
    assert!(
        s.replay_notes().iter().any(|n| n.contains("interchangeable")),
        "{:?}",
        s.replay_notes()
    );
}
