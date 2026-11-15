//! DCGO replay: a card's menu int that meets an engine trigger order DCGO
//! never asked.
//!
//! DCGO's `MultipleSkills` lists only the stacked effects whose `CanActivate`
//! holds (MultipleSkills.cs:217-221, :261-262) and, with exactly one left,
//! activates it without asking (`skillInfos_active.Count == 1`, :273-278), so
//! no `SetTargetSkill` row is written. Two or more always go through
//! `SetTargetSkill` (:354, :420), which the recorder logs. The engine orders
//! every queued trigger, including an optional one whose body has nothing to
//! do - Virus Busters (EX12-069)'s [Security] "play 1 [VB] Digimon card of the
//! same level from your hand", whose DCGO `CanActivateCondition` needs such a
//! hand card (EX12_069.cs:43-50).
//!
//! So a recorded `generic_int` of the actor that meets the actor's engine
//! trigger order proves DCGO had one activatable effect there: the entry
//! whose card-authored menu (`SelectionElement<int>`) the value is a constant
//! of. Hiro Amanokawa (EX12-066)'s menu is numbered from 1: 3 "Suspend only",
//! 4 "Do not use" (EX12_066.cs:84-100). raw-20261113T025510Z#g3 halted at
//! replay step 73 (raw seq 388) on exactly this, once the order before it
//! was resolved.
//!
//! Staged with `DebugRunner` (real EX12-066 / EX12-069 YAML, synthetic other
//! cards). No recording content.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::game::Game;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::runners::replay::{
    DivergenceKind, RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec,
};
use digimon_engine::selection::SelectionKind;

fn digimon(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(7000);
    c.play_cost = 7;
    c.colors = vec![CardColor::White];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// P0's [VB] level-5 Digimon attacks with Hiro Amanokawa (EX12-066) on the
/// field and Virus Busters (EX12-069) face up in P0's security; P0's hand
/// holds no [VB] level-5 card (Virus Busters can do nothing) and no Option
/// (Hiro's "Use an Option" is not offered). The engine orders both triggers.
fn attack_with_hiro_and_virus_busters_in(hand: &[&str], p1_security: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX12-066")
        .expect("EX12-066 YAML loads")
        .dsl_card("EX12-069")
        .expect("EX12-069 YAML loads")
        .add_card(digimon("ANGEL", 5, &["VB"]))
        .add_card(digimon("MISS", 3, &[]))
        .add_card(digimon("SAME", 5, &["VB"]))
        .hand(0, hand)
        .deck(0, &["MISS"; 6])
        .deck(1, &["MISS"; 6])
        .security(0, &["EX12-069"])
        .security(1, p1_security)
        .memory(4)
        .start();
    runner.set_turn(1);
    // Face up, as its [Main] leaves it (the g3 shape): only a face-up
    // security card's [Security] [Your Turn] trigger works.
    let security_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0].face_up_security.insert(security_index);
    let hiro = runner.place_on_field(0, "EX12-066", Some(0));
    let attacker = runner.place_on_field(0, "ANGEL", Some(0));
    runner.attack_player(attacker, 1, false);
    let p = runner.pending_selection().expect("the engine orders the two triggers");
    assert!(matches!(p.kind, SelectionKind::TriggerOrder), "{:?} {}", p.kind, p.prompt);
    assert_eq!(p.selecting_player, 0);
    let labels: Vec<String> = p.effect_choices.iter().flatten().map(|e| e.label.clone()).collect();
    assert_eq!(labels.len(), 2, "{labels:?}");
    assert!(labels.iter().any(|l| l.starts_with("EX12-066")) && labels.iter().any(|l| l.starts_with("EX12-069")), "{labels:?}");
    (runner, hiro)
}

fn attack_with_hiro_and_virus_busters() -> (DebugRunner, PermanentHandle) {
    attack_with_hiro_and_virus_busters_in(&["MISS"], &["MISS", "MISS"])
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

fn int_row(v: i64) -> StepSpec {
    let row: digimon_engine::dcgo_recording::SelectionRow = serde_json::from_value(serde_json::json!({
        "step": 0, "actor": 0, "phase": "Main", "prompt": "generic_int", "int_value": v
    }))
    .expect("selection row");
    StepSpec {
        actor: row.actor,
        action_id: 0,
        phase: "Main".into(),
        source: "selection".into(),
        memory_after: None,
        dcgo_memory: None,
        turn: None,
        is_game_over: None,
        expected_digest: None,
        selection: Some(row),
        board_p0: None,
        board_p1: None,
        card_id: None,
        dna_materials: None,
        inferred: false,
    }
}

fn staged(game: Game, steps: Vec<StepSpec>) -> ReplaySession {
    ReplaySession::with_source(Box::new(StagedDcgo { game, steps }), &HashMap::new(), false)
        .expect("staged session")
}

fn hiro_suspended(game: &Game, hiro: PermanentHandle) -> bool {
    game.players[0].battle_area[hiro.index as usize].is_suspended
}

/// Hiro's "Do not use" (4): Hiro's entry is the one DCGO activated; Hiro is
/// not suspended (EX12_066.cs:104-106 suspends for anything but 4).
#[test]
fn hiro_do_not_use_answers_the_order_through_hiros_own_menu() {
    let (r, hiro) = attack_with_hiro_and_virus_busters();
    let mut s = staged(r.game, vec![int_row(4)]);
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert_eq!(s.current_step(), 1);
    assert!(!hiro_suspended(&s.game, hiro));
}

/// Hiro's "Suspend only" (3): Hiro is suspended, nothing else happens.
#[test]
fn hiro_suspend_only_answers_the_order_through_hiros_own_menu() {
    let (r, hiro) = attack_with_hiro_and_virus_busters();
    let mut s = staged(r.game, vec![int_row(3)]);
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(hiro_suspended(&s.game, hiro));
}

/// "Use an Option" (2) is not on Hiro's menu here (no Option in hand): no
/// entry's menu takes it, so the replay stops instead of guessing.
#[test]
fn a_value_no_entrys_menu_offers_still_halts() {
    let (r, hiro) = attack_with_hiro_and_virus_busters();
    let mut s = staged(r.game, vec![int_row(2)]);
    s.step();
    match s.divergences() {
        [d] => assert!(matches!(d.kind, DivergenceKind::SelectionResolution { .. }), "{d:?}"),
        other => panic!("expected one divergence, got {other:?}"),
    }
    assert!(s.is_paused());
    assert!(!hiro_suspended(&s.game, hiro));
}

// ── The trailing gate DCGO never offered ─────────────────────────────────
//
// After Hiro's menu the engine still parks Virus Busters (EX12-069)'s "you
// may" gate. DCGO dropped that effect (`CanActivate` false, MultipleSkills.cs:
// 217-221) and the recording ends with no `OptionalSkill.SetUseOptional`
// answer for it - the g3 end (raw seq 388-389: Hiro's int, then EndGame).

/// The recording ends; accepting and declining lead to the same game (no
/// same-level [VB] card in hand, so the body does nothing), so the choice is
/// unobservable: the replay declines it - DCGO did not activate it - and the
/// attack finishes. P1 has no security, so P0 wins, as recorded.
#[test]
fn a_trailing_gate_whose_answer_changes_nothing_is_declined() {
    let (r, _) = attack_with_hiro_and_virus_busters_in(&["MISS"], &[]);
    let mut s = staged(r.game, vec![int_row(4)]);
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete());
    assert!(s.game.pending_selection.is_none(), "{:?}", s.game.pending_selection.as_ref().map(|p| &p.prompt));
    assert!(s.game.game_over);
    assert_eq!(s.game.winner, Some(0));
    assert!(
        s.replay_notes().iter().any(|n| n.contains("EX12-069")),
        "the fold is reported: {:?}",
        s.replay_notes()
    );
}

/// With a same-level [VB] card in hand, accepting opens a pick that declining
/// does not: the answer is observable, the recording does not give it, and
/// the gate stays parked (the end-of-game winner check then fails honestly).
#[test]
fn a_trailing_gate_whose_answer_matters_is_left_parked() {
    let (r, _) = attack_with_hiro_and_virus_busters_in(&["SAME"], &[]);
    let mut s = staged(r.game, vec![int_row(4)]);
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete());
    let p = s.game.pending_selection.as_ref().expect("the gate stays parked");
    assert!(p.is_optional, "{}", p.prompt);
    assert!(!s.game.game_over);
}
