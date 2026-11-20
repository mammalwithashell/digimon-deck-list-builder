//! DCGO replay: the extra "Select 1 effect to activate." row after BT24-102
//! Homeros refires an `[On Play] [When Digivolving]` effect.
//!
//! DCGO (BT24_102.cs:173-215) lists the chosen [Olympos XII] Digimon's
//! candidate effects and, from two on, opens a mandatory `SelectCardEffect`
//! over their source cards. A printed `[On Play] [When Digivolving] X` effect is
//! built as ONE effect object PER TIMING TAG (`ActivateClassesForSharedEffects`,
//! CardEffectFactory.cs:812-880; Minervamon BT24_041.cs:142-166), so a Digimon
//! with that single effect already yields two candidates naming the same card,
//! and every choice runs the same shared coroutine. The engine models the
//! printed effect as ONE clause (CR 4.3 15-15-7-2; `Effect::clause_group`),
//! refires it once and parks nothing. The recorder logs only the picked card
//! (plus its position in DCGO's list), so the row met the NEXT prompt of the
//! refired effect ("You may play 1 play cost 5 or lower [Iliad] card from your
//! hand": `card pick 'BT24-041' not found in Hand prompt`) and shifted every
//! later row (owner games 20261116T164448Z#g6 step 12 and 20261115T153822Z#g3
//! step 41, Homeros x Minervamon (BT24-041)).
//!
//! The replay consumes that row only when the engine's own state proves it: see
//! `ReplayDriver::refire_duplicate_effect_pick_fold`. A Digimon with TWO real
//! effects keeps the engine's own effect-pick prompt (no-approximations), and
//! the replay never swallows that prompt's row.
//!
//! Staged with `DebugRunner` (the real BT24-102 and BT24-041 YAML, synthetic
//! cards otherwise). No recording content.

use std::collections::HashMap;
use std::sync::Arc;

use digimon_engine::card_data::CardData;
use digimon_engine::dcgo_recording::SelectionRow;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::game::Game;
use digimon_engine::runners::replay::{
    DivergenceKind, RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec,
};
use digimon_engine::selection::TriggerSource;
use digimon_engine::{CardEffect, CardHandle, Effect};

fn iliad_digimon() -> CardData {
    let mut c = make_test_card("ILIAD-3", "Iliad Rookie");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["Iliad".into()];
    c
}

fn filler() -> CardData {
    make_test_card("FILL", "Filler")
}

/// P0's main phase, the engine's own staging: Homeros (BT24-102, unsuspended)
/// and Minervamon (BT24-041) on the battle area, an [Iliad] play-cost-3 Digimon
/// in hand. Passing the turn fires Homeros's `[End of Your Turn]`.
fn board() -> Game {
    let mut r = DebugRunner::builder()
        .dsl_card("BT24-102")
        .expect("BT24-102 production YAML")
        .dsl_card("BT24-041")
        .expect("BT24-041 production YAML")
        .add_card(iliad_digimon())
        .add_card(filler())
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .security(0, &["FILL"; 5])
        .security(1, &["FILL"; 5])
        .memory(5)
        .start();
    r.set_first_player(0);
    let homeros = r.place_on_field(0, "BT24-102", Some(0));
    r.place_on_field(0, "BT24-041", Some(0));
    r.add_to_hand(0, "ILIAD-3");
    end_of_turn(&mut r, homeros);
    r.game
}

/// Homeros's `[End of Your Turn]` has triggered and its "By suspending ..." gate
/// is parked, exactly where the recorded OptionalSkill row meets it.
fn end_of_turn(r: &mut DebugRunner, homeros: digimon_engine::permanent::PermanentHandle) {
    r.game.enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(homeros));
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_some(), "Homeros's gate is parked");
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

fn sel(json: serde_json::Value) -> StepSpec {
    let mut v = serde_json::json!({ "step": 1, "actor": 0, "phase": "Main" });
    for (k, val) in json.as_object().unwrap() {
        v[k] = val.clone();
    }
    spec(0, "selection", Some(serde_json::from_value(v).expect("selection row")))
}

fn staged(game: Game, steps: Vec<StepSpec>) -> ReplaySession {
    ReplaySession::with_source(Box::new(StagedDcgo { game, steps }), &HashMap::new(), false)
        .expect("staged session")
}

/// Homeros's "By suspending ..." gate, as the converter writes a remote seat's
/// `OptionalSkill` yes.
fn homeros_gate() -> StepSpec {
    sel(serde_json::json!({ "prompt": "generic_bool", "bool_value": true, "unstamped": true }))
}

/// The [Olympos XII] pick: Minervamon is frame 1 of P0's board.
fn olympos_pick(board: &[&str], frame: i32) -> StepSpec {
    sel(serde_json::json!({
        "prompt": "SelectPermanentEffect",
        "targets": [{"player": 0, "frame": frame}],
        "board_p0": board, "board_p1": [],
    }))
}

fn effect_pick(card: &str, position: i32) -> StepSpec {
    sel(serde_json::json!({
        "prompt": "SelectCardEffect",
        "card_ids": [card], "card_indexes": [68], "indexes": [position],
    }))
}

fn hand_pick(card: &str) -> StepSpec {
    sel(serde_json::json!({
        "prompt": "SelectHandEffect", "zone": "Hand", "card_ids": [card], "card_indexes": [7],
    }))
}

fn field(game: &Game, player: usize) -> Vec<String> {
    game.players[player]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&game.card_data).to_string())
        .collect()
}

fn run(s: &mut ReplaySession, rows: usize) {
    for _ in 0..rows {
        s.step();
    }
}

/// Minervamon's ONE printed `[On Play] [When Digivolving] [On Deletion]` effect
/// is refired once; DCGO's duplicate "Select 1 effect" row is consumed and the
/// rows after it (the hand pick) resolve against the engine's own prompt.
#[test]
fn dcgo_duplicate_effect_pick_after_homeros_refire_is_consumed() {
    let mut s = staged(
        board(),
        vec![
            homeros_gate(),
            olympos_pick(&["BT24-102", "BT24-041"], 1),
            effect_pick("BT24-041", 1),
            hand_pick("ILIAD-3"),
        ],
    );
    run(&mut s, 4);
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(!s.is_paused());
    assert!(
        field(&s.game, 0).contains(&"ILIAD-3".to_string()),
        "Minervamon's refired effect played the hand pick: {:?}",
        field(&s.game, 0)
    );
    assert!(
        s.replay_notes().iter().any(|n| n.contains("Select 1 effect to") && n.contains("BT24-041")),
        "the consumed row is reported: {:?}",
        s.replay_notes()
    );
}

/// DCGO's position is 0 or 1 of the 2 entries: both are the same effect.
#[test]
fn either_dcgo_position_of_the_two_entries_is_consumed() {
    for position in 0..2 {
        let mut s = staged(
            board(),
            vec![
                    homeros_gate(),
                olympos_pick(&["BT24-102", "BT24-041"], 1),
                effect_pick("BT24-041", position),
                hand_pick("ILIAD-3"),
            ],
        );
        run(&mut s, 4);
        assert!(s.divergences().is_empty(), "position {position}: {:?}", s.divergences());
        assert!(field(&s.game, 0).contains(&"ILIAD-3".to_string()), "position {position}");
    }
}

/// A recording whose converter predates the position list (`indexes`) still
/// replays: the position adds no information the engine needs.
#[test]
fn a_row_without_a_recorded_position_is_still_consumed() {
    let no_position = sel(serde_json::json!({
        "prompt": "SelectCardEffect", "card_ids": ["BT24-041"], "card_indexes": [68],
    }));
    let mut s = staged(
        board(),
        vec![
            homeros_gate(),
            olympos_pick(&["BT24-102", "BT24-041"], 1),
            no_position,
            hand_pick("ILIAD-3"),
        ],
    );
    run(&mut s, 4);
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(field(&s.game, 0).contains(&"ILIAD-3".to_string()));
}

/// A position past DCGO's two entries is not this prompt: halt, name it.
#[test]
fn a_position_outside_dcgos_entries_halts() {
    let mut s = staged(
        board(),
        vec![
            homeros_gate(),
            olympos_pick(&["BT24-102", "BT24-041"], 1),
            effect_pick("BT24-041", 2),
            hand_pick("ILIAD-3"),
        ],
    );
    run(&mut s, 4);
    match s.divergences() {
        [d] => {
            assert!(d.step == 2, "{d:?}");
            match &d.kind {
                DivergenceKind::SelectionResolution { reason } => {
                    assert!(reason.contains("position in DCGO's candidate list"), "{reason}");
                }
                other => panic!("expected a selection resolution divergence, got {other:?}"),
            }
        }
        other => panic!("expected one divergence, got {other:?}"),
    }
    assert!(s.is_paused());
    assert!(!field(&s.game, 0).contains(&"ILIAD-3".to_string()), "nothing was guessed");
}

/// The same card named by a row that does NOT directly follow the refire is
/// not the effect pick: the refire was two rows ago, so the fold refuses.
#[test]
fn a_later_row_naming_the_carrier_is_not_taken_for_the_effect_pick() {
    let mut s = staged(
        board(),
        vec![
            homeros_gate(),
            olympos_pick(&["BT24-102", "BT24-041"], 1),
            hand_pick("ILIAD-3"),
            effect_pick("BT24-041", 1),
        ],
    );
    run(&mut s, 4);
    match s.divergences() {
        [d] => match &d.kind {
            DivergenceKind::SelectionResolution { reason } => {
                assert!(reason.contains("did not happen while row"), "{reason}");
            }
            other => panic!("expected a selection resolution divergence, got {other:?}"),
        },
        other => panic!("expected one divergence, got {other:?}"),
    }
}

/// A row naming a different card than the refired Digimon is not this prompt:
/// the normal resolver reads it (here against Minervamon's hand prompt, where
/// the card is absent, so the replay halts instead of consuming it).
#[test]
fn a_row_naming_another_card_is_not_consumed() {
    let mut s = staged(
        board(),
        vec![
            homeros_gate(),
            olympos_pick(&["BT24-102", "BT24-041"], 1),
            effect_pick("BT24-102", 1),
            hand_pick("ILIAD-3"),
        ],
    );
    run(&mut s, 4);
    assert!(!s.divergences().is_empty(), "{:?}", s.replay_notes());
}

// ---------------------------------------------------------------------------
// The opposite shape: a Digimon with TWO real effects ([On Play] and
// [When Digivolving] as separate printed effects). The engine's own
// effect-pick prompt must surface, and the replay must not swallow it.
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct TwoSeparateEffects;

impl CardEffect for TwoSeparateEffects {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            Effect::on_play(card).name("separate on play").process(|ctx| ctx.gain_memory(2)).build(),
            Effect::when_digivolving(card)
                .name("separate when digivolving")
                .process(|ctx| ctx.gain_memory(5))
                .build(),
        ]
    }
}

fn two_effect_board() -> Game {
    let mut olympos = make_test_card("OLY-TWO", "Two Effects");
    olympos.card_kind = CardKind::Digimon;
    olympos.level = Some(5);
    olympos.dp = Some(7000);
    olympos.traits = vec!["Olympos XII".into()];
    let mut r = DebugRunner::builder()
        .dsl_card("BT24-102")
        .expect("BT24-102 production YAML")
        .add_card(olympos)
        .add_card(filler())
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .security(0, &["FILL"; 5])
        .security(1, &["FILL"; 5])
        .memory(5)
        .start();
    r.register_effect("OLY-TWO", Arc::new(TwoSeparateEffects));
    r.set_first_player(0);
    let homeros = r.place_on_field(0, "BT24-102", Some(0));
    r.place_on_field(0, "OLY-TWO", Some(0));
    end_of_turn(&mut r, homeros);
    r.game
}

/// Two separate effects: the engine parks its own "Choose an effect" prompt, so
/// DCGO's row is NOT consumed as a duplicate. The prompt stays parked (the two
/// entries name one card, so the card alone cannot answer it: an honest stop).
#[test]
fn two_real_effects_keep_the_engines_own_effect_pick() {
    let mut s = staged(
        two_effect_board(),
        vec![
            homeros_gate(),
            olympos_pick(&["BT24-102", "OLY-TWO"], 1),
            effect_pick("OLY-TWO", 1),
        ],
    );
    run(&mut s, 3);
    let pending = s.game.pending_selection.as_ref().expect("the engine's own effect choice is still parked");
    assert_eq!(pending.kind, digimon_engine::selection::SelectionKind::EffectChoice);
    assert_eq!(pending.valid_action_ids.len(), 2, "On Play and When Digivolving are two real choices");
    assert!(!s.divergences().is_empty(), "DCGO's row names a card both entries share: halt, do not guess");
    assert!(
        !s.replay_notes().iter().any(|n| n.contains("Select 1 effect to")),
        "the row was not taken for a duplicate: {:?}",
        s.replay_notes()
    );
    assert_eq!(s.game.memory, 5, "neither effect ran");
}
