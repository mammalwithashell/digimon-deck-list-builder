//! DCGO replay: a stamped `yes` in front of an UNSTAMPED `generic_bool` is an
//! answer, not the "proceed to the pick" gate.
//!
//! BT25-030 Elecmon's inherited [When Attacking] "You may add your top security
//! card to the hand" is ONE optional engine prompt (accept = take the card,
//! `PASS` = decline). DCGO asks it as an `OptionalSkill` Yes/No, so the recording
//! carries a stamped `generic_bool`. `ReplaySession::step` folds a recorded yes
//! against an optional prompt with several accept ids into "the gate in front of
//! a pick" when the actor's next row exists - and, finding no card/target pick
//! behind it, rewrote the yes into a DECLINE. The row behind it was not a pick:
//! it was another optional effect's UNSTAMPED `generic_bool` (BT24-084 Inori
//! Misono's [All Turns] "when your security stack is removed from", which the
//! added card triggers; or an Aegiomon (BT24-034)'s Barrier accept). The security
//! card was never added, the effect that depended on it never queued, and the
//! unstamped bool met nothing.
//!
//! CR 4.3 15-7-1 / 15-7-2 give nothing to a declined "may": the player chose yes.
//!
//! Staged with `DebugRunner` (real BT25-030 / BT24-034 / BT24-084 YAML,
//! synthetic Aegiochusmon and security cards). No recording content.

use std::collections::HashMap;

use digimon_engine::action::space;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::game::Game;
use digimon_engine::runners::replay::{RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec};

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

fn step_spec(action_id: u16, selection: Option<digimon_engine::dcgo_recording::SelectionRow>) -> StepSpec {
    StepSpec {
        actor: 0,
        action_id,
        phase: "Main".into(),
        source: if selection.is_some() { "selection".into() } else { "main_phase".into() },
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
    let mut v = serde_json::json!({ "step": 0, "actor": 0, "phase": "Main" });
    for (k, val) in json.as_object().unwrap() {
        v[k] = val.clone();
    }
    let row: digimon_engine::dcgo_recording::SelectionRow =
        serde_json::from_value(v).expect("selection row");
    step_spec(0, Some(row))
}

fn bool_row(v: bool, unstamped: bool) -> StepSpec {
    if unstamped {
        sel(serde_json::json!({"prompt": "generic_bool", "bool_value": v, "unstamped": true}))
    } else {
        sel(serde_json::json!({"prompt": "generic_bool", "bool_value": v}))
    }
}

fn staged(game: Game, steps: Vec<StepSpec>) -> ReplaySession {
    ReplaySession::with_source(Box::new(StagedDcgo { game, steps }), &HashMap::new(), false)
        .expect("staged session")
}

fn replay_all(s: &mut ReplaySession) {
    while s.current_step() < s.total_steps() {
        let before = s.current_step();
        s.step();
        assert!(s.divergences().is_empty() && !s.is_paused(), "row {before} diverged: {:?}", s.divergences());
        assert!(s.current_step() > before, "row {before} did not advance the cursor");
    }
}

fn digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card_with_level(id, name, level);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(dp);
    c.play_cost = 5;
    c.colors = vec![CardColor::Yellow];
    c
}

/// A Lv.5 [Aegiochusmon] hand card that digivolves from a Lv.4 yellow Digimon.
fn aegiochusmon() -> CardData {
    let mut c = digimon("EVO", "Aegiochusmon: Test", 5, 8000);
    c.evo_costs = vec![EvoCost { card_color: CardColor::Yellow as u8, level: 4, memory_cost: 3 }];
    c
}

/// P0's Aegiomon (BT24-034) with an Elecmon (BT25-030) source in slot 0, P0's
/// Inori Misono (BT24-084) in slot 1, an Aegiochusmon in hand. P0 is about to
/// attack P1's security (`security_dp` on every card) with 3 security each.
fn staged_attack(security_dp: i32, with_inori: bool) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card("BT25-030")
        .expect("BT25-030 YAML")
        .dsl_card("BT24-034")
        .expect("BT24-034 YAML")
        .dsl_card("BT24-084")
        .expect("BT24-084 YAML")
        .add_card(aegiochusmon())
        .add_card(digimon("SEC", "Security Digimon", 4, security_dp))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["EVO"])
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .security(0, &["FILL"; 3])
        .security(1, &["SEC"; 3])
        .memory(10)
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    r.place_stack(0, &["BT25-030", "BT24-034"]);
    if with_inori {
        r.place_on_field(0, "BT24-084", Some(1));
    }
    r
}

fn attack_row() -> StepSpec {
    step_spec(space::encode_attack(0, space::SECURITY_TARGET), None)
}

fn count(cards: &[digimon_engine::card_source::CardSource], game: &Game, id: &str) -> usize {
    cards.iter().filter(|c| c.card_id(&game.card_data) == id).count()
}

/// The yes adds the top security card (the BT25-030 prompt was offered with
/// more than one accept id). With nothing after it the row still resolves as
/// yes, as it did before the gate existed.
#[test]
fn stamped_yes_adds_top_security_when_it_is_the_last_row() {
    let r = staged_attack(1000, false);
    let hand_before = r.game.players[0].hand.len();
    let mut s = staged(r.game, vec![attack_row(), bool_row(true, false)]);
    replay_all(&mut s);
    assert_eq!(s.game.players[0].security.len(), 2, "the top security card was added");
    assert_eq!(s.game.players[0].hand.len(), hand_before + 1);
}

/// The yes, then ANOTHER effect's unstamped bool: the yes is still the answer
/// to BT25-030's prompt (the card is added), and the unstamped bool is
/// BT24-084's optional trigger (its pick follows). Rewriting the yes into a
/// decline left the trigger unqueued and the bool answering nothing.
#[test]
fn stamped_yes_then_unstamped_bool_adds_security_and_runs_the_trigger() {
    let r = staged_attack(1000, true);
    let mut s = staged(
        r.game,
        vec![
            attack_row(),
            bool_row(true, false),
            bool_row(true, true),
            sel(serde_json::json!({
                "prompt": "SelectPermanentEffect",
                "targets": [{"player": 0, "frame": 0}],
            })),
            sel(serde_json::json!({
                "prompt": "SelectHandEffect", "zone": "Hand", "card_ids": ["EVO"],
            })),
        ],
    );
    replay_all(&mut s);
    assert_eq!(s.game.players[0].security.len(), 2, "the top security card was added");
    assert_eq!(count(&s.game.players[0].hand, &s.game, "EVO"), 0, "Aegiochusmon left the hand");
    assert!(
        s.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&s.game.card_data) == "EVO"),
        "Aegiomon digivolved into Aegiochusmon"
    );
}

/// The yes adds the card, the attack then LOSES to a higher-DP security
/// Digimon, and the unstamped bool is the Aegiomon's own <Barrier> accept
/// ("When this Digimon would be deleted, you may trash the top security card").
/// With the yes rewritten into a decline the Barrier was never accepted and the
/// next row met a still-parked prompt.
#[test]
fn stamped_yes_then_unstamped_bool_accepts_the_barrier() {
    let r = staged_attack(9000, false);
    let hand_before = r.game.players[0].hand.len();
    let mut s = staged(r.game, vec![attack_row(), bool_row(true, false), bool_row(true, true)]);
    replay_all(&mut s);
    assert_eq!(s.game.players[0].hand.len(), hand_before + 1, "the top security card was added");
    assert_eq!(s.game.players[0].security.len(), 1, "added one, then <Barrier> trashed one");
    assert!(
        s.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&s.game.card_data) == "BT24-034"),
        "the Aegiomon survived through <Barrier>"
    );
    assert!(s.game.pending_selection.is_none(), "nothing is left parked");
}

/// A stamped `no` still declines.
#[test]
fn stamped_no_declines_the_add() {
    let r = staged_attack(1000, false);
    let mut s = staged(r.game, vec![attack_row(), bool_row(false, false)]);
    replay_all(&mut s);
    assert_eq!(s.game.players[0].security.len(), 3, "nothing was added");
}
