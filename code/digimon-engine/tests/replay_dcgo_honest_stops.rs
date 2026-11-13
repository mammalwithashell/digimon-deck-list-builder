//! DCGO replay: rows the replay cannot apply must stop it honestly.
//!
//! A recorded DCGO selection row is evidence of what DCGO did. When the engine
//! has nothing that row can answer, consuming it silently hides drift (the
//! Data Squad vs Chronicle PvP game of 2026-11-13 lost BT20-095's <Delay> that
//! way: its pick rows met no prompt and were skipped). These tests pin the
//! policy:
//!
//! 1. A selection row of one seat never answers a prompt the engine parked for
//!    the OTHER seat: the engine rejects such an answer (`WrongPlayer`), so the
//!    replay reports an actor divergence instead of counting the row as
//!    consumed (Virus Busters vs Toho Braves, 2026-11-13: P1's
//!    SelectCardEffect [EX12-007] was "applied" to P0's "Return 2 cards from
//!    your opponent's trash" pick).
//! 2. A row that names cards, targets, a zone or a choice, met with NO prompt
//!    parked, is a halting divergence. A contentless row (a cancel) becomes a
//!    non-halting note.
//! 3. An unstamped DCGO "use this optional effect?" answer (`generic_bool`,
//!    `unstamped`) halts when it cannot have meant nothing: a `false` while the
//!    actor has an optional prompt parked that is not a recognised activation
//!    gate (DCGO's decline would be lost), or a `true` with nothing parked while
//!    the actor's next row is a selection (DCGO used an effect the engine never
//!    offered). A `true` with nothing parked and no selection after it stays a
//!    note.
//!
//! The game is staged with `DebugRunner` and served through a minimal
//! DCGO-policy `RecordingSource` whose steps are the rows under test.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use digimon_engine::action::space::ATTACK_START;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::dcgo_recording::SelectionRow;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectSourceKind, GamePhase};
use digimon_engine::game::Game;
use digimon_engine::runners::replay::{
    DivergenceKind, RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec,
};
use digimon_engine::selection::{PendingSelection, SelectionKind};

/// A DCGO-policy source over a staged game.
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

fn session(game: Game, steps: Vec<StepSpec>) -> ReplaySession {
    ReplaySession::with_source(Box::new(StagedDcgo { game, steps }), &HashMap::new(), false)
        .expect("staged session")
}

/// Main phase, P0 to act, three P0 permanents at slots 0..=2, nothing parked.
fn quiet_board() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("MON", "MON"))
        .hand(0, &["MON"])
        .deck(0, &["MON"; 5])
        .deck(1, &["MON"; 5])
        .start();
    for _ in 0..3 {
        r.place_stack(0, &["MON"]);
    }
    assert!(r.game.pending_selection.is_none());
    r
}

/// `quiet_board` with a hand-built `OwnField` pick of `selecting` parked over
/// P0's slots 0 and 1. The callback records the id it is resolved with.
fn parked_field_pick(selecting: u8, optional: bool) -> (Game, Arc<Mutex<Option<u16>>>) {
    let mut r = quiet_board();
    let source_card = CardHandle(r.game.players[0].battle_area[0].card_sources[0].card_index);
    let chosen: Arc<Mutex<Option<u16>>> = Arc::new(Mutex::new(None));
    let sink = chosen.clone();
    r.game.pending_selection = Some(PendingSelection {
        card_choice: None,
        zone_owner: None,
        kind: SelectionKind::OwnField,
        selecting_player: selecting,
        previous_phase: GamePhase::Main,
        valid_action_ids: vec![ATTACK_START, ATTACK_START + 1],
        is_optional: optional,
        prompt: "Choose 1 of your permanents".into(),
        effect_choices: None,
        source_card,
        source_permanent: None,
        source_kind: EffectSourceKind::Digimon,
        callback: Box::new(move |_, id| *sink.lock().unwrap() = Some(id)),
        on_decline: None,
    });
    (r.game, chosen)
}

/// Exactly one divergence, at row `at`; the replay is halted there.
fn halted_at(s: &ReplaySession, at: u32) -> DivergenceKind {
    let kind = match s.divergences() {
        [d] => {
            assert_eq!(d.step, at, "{d:?}");
            d.kind.clone()
        }
        other => panic!("expected one divergence at row {at}, got {other:?}"),
    };
    assert!(s.is_paused(), "the divergence halts the replay");
    assert_eq!(s.current_step(), at, "the row is not consumed");
    kind
}

// ── (1) actor check ──────────────────────────────────────────────────────────

/// P1's row meets P0's pick. Resolving it positionally would pick slot 0, and
/// the engine would reject that answer as P1's (`WrongPlayer`); the replay
/// must not count the row as consumed.
#[test]
fn selection_row_of_the_other_seat_is_an_actor_divergence() {
    let (game, chosen) = parked_field_pick(0, false);
    let mut s = session(
        game,
        vec![row(serde_json::json!({"step":0,"actor":1,"prompt":"generic_int","int_value":0}))],
    );
    s.step();
    let kind = halted_at(&s, 0);
    assert!(matches!(kind, DivergenceKind::Actor { expected: 0, recorded: 1 }), "{kind:?}");
    assert_eq!(*chosen.lock().unwrap(), None, "nothing chosen for P0");
    assert!(s.game.pending_selection.is_some(), "P0's pick is still parked");
}

/// The same with a card-naming row (the g4 shape: a SelectCardEffect of the
/// seat that is not asked).
#[test]
fn card_row_of_the_other_seat_is_an_actor_divergence() {
    let (game, chosen) = parked_field_pick(0, true);
    let mut s = session(
        game,
        vec![row(serde_json::json!({"step":0,"actor":1,"prompt":"SelectPermanentEffect","targets":[{"player":0,"frame":1}]}))],
    );
    s.step();
    let kind = halted_at(&s, 0);
    assert!(matches!(kind, DivergenceKind::Actor { expected: 0, recorded: 1 }), "{kind:?}");
    assert_eq!(*chosen.lock().unwrap(), None);
}

// ── (2) rows met with no prompt ──────────────────────────────────────────────

/// A row that names cards, targets or a choice has nothing to answer: the
/// engine never asked. That is drift (an effect the engine did not run, or ran
/// without this decision), so the replay stops.
#[test]
fn meaningful_row_with_no_prompt_parked_is_a_halting_divergence() {
    for payload in [
        serde_json::json!({"step":0,"actor":0,"prompt":"SelectCardEffect","card_ids":["MON"]}),
        serde_json::json!({"step":0,"actor":0,"prompt":"SelectPermanentEffect","targets":[{"player":0,"frame":1}]}),
        serde_json::json!({"step":0,"actor":0,"prompt":"generic_int","int_value":2}),
        serde_json::json!({"step":0,"actor":0,"prompt":"SelectHandEffect","zone":"Hand","card_ids":["MON"]}),
    ] {
        let r = quiet_board();
        let before = r.game.verification_digest();
        let mut s = session(r.game, vec![row(payload.clone())]);
        s.step();
        let kind = halted_at(&s, 0);
        assert!(
            matches!(&kind, DivergenceKind::SelectionResolution { reason } if reason.contains("no prompt")),
            "{payload}: {kind:?}"
        );
        assert_eq!(s.game.verification_digest(), before, "{payload}: nothing applied");
    }
}

/// A cancel with no prompt parked declined something the engine never asked:
/// it carries no decision to replay, so it is consumed with a NON-halting note.
#[test]
fn cancel_row_with_no_prompt_parked_is_a_note() {
    let r = quiet_board();
    let before = r.game.verification_digest();
    let mut s = session(
        r.game,
        vec![row(serde_json::json!({"step":0,"actor":0,"prompt":"SelectCardEffect","cancel":true}))],
    );
    s.step();
    assert!(s.divergences().is_empty() && !s.is_paused(), "{:?}", s.divergences());
    assert_eq!(s.current_step(), 1);
    assert_eq!(s.game.verification_digest(), before);
    match s.replay_notes() {
        [note] => assert!(note.contains("row 0") && note.contains("cancel"), "{note}"),
        other => panic!("expected one note, got {other:?}"),
    }
}

/// DCGO's `MultipleSkills` -1 is "Don't activate these effects"
/// (`MultipleSkills.cs:474-480`: the stacked effects are dropped and nothing
/// activates). With no prompt parked the engine had no such effects pending,
/// and declining them changes nothing: like a cancel, it is a note. (Field
/// witness: the sim1b playback recordings, row 69.)
#[test]
fn decline_all_multiple_skills_row_with_no_prompt_parked_is_a_note() {
    let r = quiet_board();
    let before = r.game.verification_digest();
    let mut s = session(
        r.game,
        vec![row(serde_json::json!({"step":0,"actor":0,"prompt":"MultipleSkills","int_value":-1}))],
    );
    s.step();
    assert!(s.divergences().is_empty() && !s.is_paused(), "{:?}", s.divergences());
    assert_eq!(s.current_step(), 1);
    assert_eq!(s.game.verification_digest(), before);
    match s.replay_notes() {
        [note] => assert!(note.contains("row 0") && note.contains("MultipleSkills"), "{note}"),
        other => panic!("expected one note, got {other:?}"),
    }
}

/// A non-negative `MultipleSkills` index with no prompt parked only ORDERED
/// effects the engine has nothing left to order (the replay's existing fold
/// consumes it). The effects' own decisions are the evidence - their pick rows
/// halt if the engine never asks them - so the index is consumed, but no
/// longer silently: a note names it.
#[test]
fn multiple_skills_index_with_no_prompt_parked_is_noted() {
    let r = quiet_board();
    let mut s = session(
        r.game,
        vec![row(serde_json::json!({"step":0,"actor":0,"prompt":"MultipleSkills","int_value":1,"chosen_effect_card":"MON"}))],
    );
    s.step();
    assert!(s.divergences().is_empty() && !s.is_paused(), "{:?}", s.divergences());
    assert_eq!(s.current_step(), 1);
    match s.replay_notes() {
        [note] => assert!(note.contains("row 0") && note.contains("MultipleSkills"), "{note}"),
        other => panic!("expected one note, got {other:?}"),
    }
}

// ── (3) unstamped yes/no answers ─────────────────────────────────────────────

/// An unstamped `false` (DCGO declined an optional effect) while P0 has an
/// optional pick parked that is NOT an optional-activation gate: the replay
/// cannot tell whether the decline was this pick's, so it stops rather than
/// lose DCGO's decline.
#[test]
fn unstamped_false_against_a_non_gate_optional_prompt_halts() {
    let (game, chosen) = parked_field_pick(0, true);
    let mut s = session(
        game,
        vec![row(serde_json::json!({"step":0,"actor":0,"prompt":"generic_bool","bool_value":false,"unstamped":true}))],
    );
    s.step();
    let kind = halted_at(&s, 0);
    assert!(matches!(kind, DivergenceKind::SelectionResolution { .. }), "{kind:?}");
    assert_eq!(*chosen.lock().unwrap(), None);
    assert!(s.game.pending_selection.is_some());
}

/// An unstamped `true` (DCGO USED an optional effect) with nothing parked,
/// followed by P0's own pick row: the engine never offered the effect whose
/// picks follow. Halt at the yes, where the drift is.
#[test]
fn unstamped_true_with_nothing_parked_before_the_actors_selection_halts() {
    let r = quiet_board();
    let mut s = session(
        r.game,
        vec![
            row(serde_json::json!({"step":0,"actor":0,"prompt":"generic_bool","bool_value":true,"unstamped":true})),
            row(serde_json::json!({"step":1,"actor":0,"prompt":"SelectCardEffect","card_ids":["MON"]})),
        ],
    );
    s.step();
    let kind = halted_at(&s, 0);
    assert!(matches!(kind, DivergenceKind::SelectionResolution { .. }), "{kind:?}");
}

/// Regression guard: an unstamped `true` with nothing parked and no selection
/// of that seat after it stays a non-halting note.
#[test]
fn unstamped_true_with_nothing_parked_and_no_following_selection_is_a_note() {
    let r = quiet_board();
    let mut s = session(
        r.game,
        vec![
            row(serde_json::json!({"step":0,"actor":0,"prompt":"generic_bool","bool_value":true,"unstamped":true})),
            row(serde_json::json!({"step":1,"actor":1,"prompt":"SelectCardEffect","cancel":true})),
        ],
    );
    s.step();
    assert!(s.divergences().is_empty() && !s.is_paused(), "{:?}", s.divergences());
    assert_eq!(s.current_step(), 1);
    assert_eq!(s.replay_notes().len(), 1, "{:?}", s.replay_notes());
}

// ── (4) an order DCGO never recorded ─────────────────────────────────────────

/// Cards returned to the deck together: the engine asks their order (an
/// `AnyField` "Choose the bottommost remaining card" pick). DCGO asks nothing
/// - `DeckBottomBounceClass.DeckBounce` places them in the order of the list
/// the card script built (CardController.cs:2650-2668, then
/// `AddLibraryBottomCards` appends, so the LAST is the very bottom;
/// BT20-045's list is `GetBattleAreaDigimons()`, ascending DCGO frame,
/// Player.cs:621-640) - and records no row. The replay used to take the
/// lowest board slot as the bottommost, the reverse of that BT20-045 order and
/// a guess for every other card, so it now stops there.
#[test]
fn unrecorded_deck_bottom_order_halts_instead_of_taking_the_lowest_slot() {
    let mut r = quiet_board();
    let source_card = CardHandle(r.game.players[0].battle_area[0].card_sources[0].card_index);
    let chosen: Arc<Mutex<Option<u16>>> = Arc::new(Mutex::new(None));
    let sink = chosen.clone();
    r.game.pending_selection = Some(PendingSelection {
        card_choice: None,
        zone_owner: None,
        kind: SelectionKind::AnyField,
        selecting_player: 0,
        previous_phase: GamePhase::Main,
        valid_action_ids: vec![ATTACK_START, ATTACK_START + 1],
        is_optional: false,
        prompt: "Choose the bottommost remaining card".into(),
        effect_choices: None,
        source_card,
        source_permanent: None,
        source_kind: EffectSourceKind::Digimon,
        callback: Box::new(move |_, id| *sink.lock().unwrap() = Some(id)),
        on_decline: None,
    });
    let mut pass = row(serde_json::json!({"step":0,"actor":0,"prompt":"SelectCardEffect","cancel":true}));
    pass.selection = None;
    pass.action_id = digimon_engine::action::space::PASS;
    pass.source = "main_phase".into();
    let mut s = session(r.game, vec![pass]);
    s.step();
    let kind = halted_at(&s, 0);
    assert!(
        matches!(&kind, DivergenceKind::SelectionResolution { reason } if reason.contains("order")),
        "{kind:?}"
    );
    assert_eq!(*chosen.lock().unwrap(), None, "no slot was taken as the bottommost");
}
