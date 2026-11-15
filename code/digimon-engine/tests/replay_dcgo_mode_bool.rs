//! DCGO replay: a card's two-mode BODY choice that DCGO asks as a bool.
//!
//! Omnimon (EX12-037)'s [When Digivolving] [When Attacking] [Once Per Turn]
//! "Delete 1 of your opponent's Digimon. Then, for every 5 of this Digimon's
//! digivolution cards, activate 1 of the effects below: - 1 of your
//! opponent's Digimon gets -13000 DP ... - Trash your opponent's top security
//! card. Then, <Recovery +1>." DCGO asks the mode as a `SetBoolSelection`:
//! "DP - 13K" = true, "Trash security" = false (EX12_037.cs:157-170), then
//! runs the -13000 pick (:172-199) or the security trash + Recovery
//! (:201-213). The engine asks the same choice as a two-branch
//! `EffectChoice` ("1 opponent Digimon gets -13000 DP" / "Trash opponent top
//! security, then Recovery +1", EX12-037.yaml `repeat_effect_choice`).
//!
//! The replay refused every bool on that prompt (Omnimon's script asks no
//! yes/no bool, and the branches are not yes/no-shaped), so
//! raw-20261113T025510Z#g3 halted at replay step 70 (raw seq 385-387).
//!
//! Staged with `DebugRunner` (real EX12-037 YAML, synthetic other cards). No
//! recording content.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::game::Game;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::runners::replay::{
    DivergenceKind, RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec,
};
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 5;
    c.colors = vec![CardColor::Blue];
    c
}

/// P0's Omnimon (EX12-037) over 5 digivolution cards (one mode) has its
/// [When Digivolving] waiting on the delete pick. P1 has `opp` Digimon.
fn omnimon_digivolved(opp: &[&str]) -> (DebugRunner, Vec<PermanentHandle>) {
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-037")
        .expect("EX12-037 production YAML")
        .add_card(digimon("BASE", 6, 7000))
        .add_card(digimon("OPP", 4, 5000))
        .add_card(digimon("OPP2", 5, 15000))
        .deck(0, &["BASE"; 20])
        .deck(1, &["BASE"; 20])
        .memory(5)
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    for pid in 0..2 {
        for _ in 0..3 {
            let c = r.game.players[pid].deck.pop().expect("deck card");
            r.game.players[pid].security.push(c);
        }
    }
    let me = r.place_stack(0, &["BASE", "BASE", "BASE", "BASE", "BASE", "EX12-037"]);
    let opps: Vec<PermanentHandle> = opp.iter().map(|id| r.place_on_field(1, id, None)).collect();
    r.game.enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    let p = r.pending_selection().expect("the delete pick waits");
    assert!(matches!(p.kind, SelectionKind::OppField { .. }) || p.selecting_player == 0, "{:?} {}", p.kind, p.prompt);
    (r, opps)
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

fn sel(json: serde_json::Value) -> StepSpec {
    let mut v = serde_json::json!({ "step": 0, "actor": 0, "phase": "Main" });
    for (k, val) in json.as_object().unwrap() {
        v[k] = val.clone();
    }
    let row: digimon_engine::dcgo_recording::SelectionRow =
        serde_json::from_value(v).expect("selection row");
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

fn replay_all(s: &mut ReplaySession) {
    while s.current_step() < s.total_steps() {
        let before = s.current_step();
        s.step();
        assert!(s.divergences().is_empty() && !s.is_paused(), "row {before} diverged: {:?}", s.divergences());
        assert!(s.current_step() > before, "row {before} did not advance the cursor");
    }
}

fn bool_row(v: bool) -> StepSpec {
    sel(serde_json::json!({"prompt": "generic_bool", "bool_value": v}))
}

fn opp_pick(board_p1: &[&str]) -> StepSpec {
    sel(serde_json::json!({
        "prompt": "SelectPermanentEffect",
        "targets": [{"player": 1, "frame": 0}],
        "board_p0": ["EX12-037"],
        "board_p1": board_p1,
    }))
}

/// "Trash security" (false): the opponent's top security card is trashed and
/// P0 recovers 1 (EX12_037.cs:201-213).
#[test]
fn false_is_trash_security_then_recovery() {
    let (r, _) = omnimon_digivolved(&["OPP"]);
    let (sec0, sec1) = (r.security_count(0), r.security_count(1));
    let trash1 = r.trash_size(1);
    let mut s = staged(r.game, vec![opp_pick(&["OPP"]), bool_row(false)]);
    replay_all(&mut s);
    assert!(s.game.pending_selection.is_none(), "{:?}", s.game.pending_selection.as_ref().map(|p| &p.prompt));
    assert_eq!(s.game.players[1].battle_area.len(), 0, "OPP deleted");
    assert_eq!(s.game.players[1].security.len(), sec1 - 1, "P1's top security trashed");
    assert_eq!(s.game.players[0].security.len(), sec0 + 1, "P0 <Recovery +1>");
    // OPP and the trashed security card.
    assert_eq!(s.game.players[1].trash.len(), trash1 + 2);
}

/// "DP - 13K" (true): the next pick gives 1 opponent Digimon -13000 DP
/// (EX12_037.cs:172-199); security is untouched.
#[test]
fn true_is_minus_13000_then_its_pick() {
    let (r, opps) = omnimon_digivolved(&["OPP", "OPP2"]);
    let (sec0, sec1) = (r.security_count(0), r.security_count(1));
    let mut s = staged(
        r.game,
        vec![opp_pick(&["OPP", "OPP2"]), bool_row(true), opp_pick(&["OPP2"])],
    );
    replay_all(&mut s);
    assert!(s.game.pending_selection.is_none(), "{:?}", s.game.pending_selection.as_ref().map(|p| &p.prompt));
    assert_eq!(s.game.players[1].battle_area.len(), 1, "OPP deleted, OPP2 stays");
    // OPP2 slid to slot 0 after OPP's deletion.
    let opp2 = PermanentHandle { player: 1, index: 0 };
    let _ = opps;
    assert_eq!(s.game.effective_dp(opp2), Some(2000));
    assert_eq!(s.game.players[1].security.len(), sec1);
    assert_eq!(s.game.players[0].security.len(), sec0);
}

/// A bool whose mode the next rows contradict is not guessed: "DP - 13K"
/// (true) followed by no -13000 pick (another seat's row) halts on that row.
#[test]
fn true_without_its_pick_does_not_resolve_silently() {
    let (r, _) = omnimon_digivolved(&["OPP", "OPP2"]);
    let mut s = staged(
        r.game,
        vec![
            opp_pick(&["OPP", "OPP2"]),
            bool_row(true),
            sel(serde_json::json!({"actor": 1, "prompt": "generic_bool", "bool_value": true})),
        ],
    );
    s.step();
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    s.step();
    assert_eq!(s.divergences().len(), 1, "{:?}", s.divergences());
    assert!(s.is_paused());
}

/// The mapping is the card's own: the same two-branch shape installed by a
/// card whose DCGO script has no such bool still halts (guard for the
/// generic refusal in `stamped_bool_meaning_mismatch`).
#[test]
fn a_bool_on_an_unlisted_card_mode_choice_still_halts() {
    let yaml = r#"
card: MODE-1
name: ModeCard
kind: digimon
level: 6
color: [blue]
cost: 5
dp: 7000
effects:
  - when: [when_digivolving]
    summary: "test: choose one of two modes"
    process:
      - repeat_effect_choice:
          count: 1
          labels:
            - "1 opponent Digimon gets -13000 DP"
            - "Trash opponent top security, then Recovery +1"
          bind_as: mode
          prompt: "Choose 1 effect to activate"
          body:
            - if:
                condition: { equals: [mode, 1] }
                then:
                  - trash_top_security: { of: opponent }
"#;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("test YAML")
        .add_card(digimon("BASE", 5, 7000))
        .deck(0, &["BASE"; 20])
        .deck(1, &["BASE"; 20])
        .memory(5)
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    let me = r.place_stack(0, &["BASE", "MODE-1"]);
    r.game.enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    let p = r.pending_selection().expect("the mode choice waits");
    assert!(matches!(p.kind, SelectionKind::EffectChoice), "{:?} {}", p.kind, p.prompt);
    let mut s = staged(r.game, vec![bool_row(false)]);
    s.step();
    match s.divergences() {
        [d] => assert!(matches!(d.kind, DivergenceKind::SelectionResolution { .. }), "{d:?}"),
        other => panic!("expected one divergence, got {other:?}"),
    }
    assert!(s.is_paused());
}
