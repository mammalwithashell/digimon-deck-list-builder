//! DCGO replay: four `generic_bool` answers the replay could not map onto the
//! engine's equivalent prompt (hunt-2 cluster `replay-bool-answer-maps`).
//!
//! 1. A stamped `yes` on BT25-030 Elecmon's "add top security to hand?" prompt,
//!    followed by a `MultipleSkills` row (the order of the security-removed
//!    triggers the added card queues: BT24-084 Inori Misono, BT24-093 Temple of
//!    Beginnings), was read as "the gate in front of a pick" and rewritten into a
//!    decline. The `MultipleSkills` menu is never a pick a yes opens.
//! 2. A keyword <Ascension>'s "place this card as the top security card?" Yes /
//!    No is a `SetBoolSelection` of the shared keyword helper
//!    (`Ascension.cs:33-39`, Yes = true), not of the carrier's script, so no card
//!    lists it as asking a yes/no bool and the replay refused it. The engine's
//!    `KeywordAscensionChoice` frame identifies the question.
//! 3. An UNSTAMPED `generic_bool` on the `BeforePayCost` "by X, reduce the play
//!    cost" gate of Yuugo Kamishiro (BT22-094): DCGO asks the optional effect as
//!    an `OptionalSkill` Yes/No before its cost is paid; the engine parks a one
//!    entry accept/decline `EffectChoice`. The yes was dropped as a note.
//! 4. Jesmon (BT23-013)'s "Choose what Jesmon plays": DCGO asks it as a
//!    `SelectionElement<bool>` menu (the Token = true, a [Sistermon] = false,
//!    BT23_013.cs:121-141).
//!
//! Staged with `DebugRunner` (real BT25-030 / BT24-034 / BT24-084 / BT22-094 /
//! BT23-013 YAML, synthetic other cards). No recording content.

use std::collections::HashMap;

use digimon_engine::action::space;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::game::Game;
use digimon_engine::runners::replay::{
    DivergenceKind, RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec,
};
use digimon_engine::selection::{SelectionKind, TriggerSource};

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

// ── 1. stamped yes on "add top security to hand?", then a MultipleSkills row ──

/// A Lv.5 [Aegiochusmon] hand card that digivolves from a Lv.4 yellow Digimon.
fn aegiochusmon() -> CardData {
    let mut c = digimon("EVO", "Aegiochusmon: Test", 5, 8000);
    c.evo_costs = vec![EvoCost { card_color: CardColor::Yellow as u8, level: 4, memory_cost: 3 }];
    c
}

const ORDER_TAMER: &str = r#"
card: ORD-1
name: OrderTamer
kind: tamer
color: [yellow]
cost: 1
effects:
  - when: on_own_security_removed
    active_when: { all_turns: true }
    summary: "test: a second security-removed trigger, so the two are ordered"
    process:
      - gain_memory: 1
"#;

/// P0's Aegiomon (BT24-034) over an Elecmon (BT25-030), P0's Inori Misono
/// (BT24-084) and a test tamer whose own security-removed trigger forces the
/// two to be ordered, attacking P1's security.
fn staged_attack_with_two_removal_triggers() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card("BT25-030")
        .expect("BT25-030 YAML")
        .dsl_card("BT24-034")
        .expect("BT24-034 YAML")
        .dsl_card("BT24-084")
        .expect("BT24-084 YAML")
        .from_dsl_yaml(ORDER_TAMER)
        .expect("ORD-1 YAML")
        .add_card(aegiochusmon())
        .add_card(digimon("SEC", "Security Digimon", 4, 1000))
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
    r.place_on_field(0, "BT24-084", Some(1));
    r.place_on_field(0, "ORD-1", Some(2));
    r
}

fn attack_row() -> StepSpec {
    step_spec(space::encode_attack(0, space::SECURITY_TARGET), None)
}

/// The yes adds the top security card; the `MultipleSkills` row behind it
/// orders the two triggers the removal queued. It is the order menu, not a
/// pick the yes opens, so the yes must not be rewritten into a decline.
#[test]
fn stamped_yes_then_trigger_order_row_adds_the_security_card() {
    let r = staged_attack_with_two_removal_triggers();
    let hand_before = r.game.players[0].hand.len();
    let mut s = staged(
        r.game,
        vec![
            attack_row(),
            bool_row(true, false),
            sel(serde_json::json!({"prompt": "MultipleSkills", "int_value": 0, "chosen_effect_card": "BT24-084"})),
        ],
    );
    replay_all(&mut s);
    assert_eq!(s.game.players[0].hand.len(), hand_before + 1, "the top security card was added to the hand");
    assert_eq!(s.game.players[0].security.len(), 2, "one of three security cards was added");
}

/// A stamped no on the same prompt, with the same row behind it, still
/// declines: nothing is added.
#[test]
fn stamped_no_then_trigger_order_row_still_declines() {
    let r = staged_attack_with_two_removal_triggers();
    let hand_before = r.game.players[0].hand.len();
    let mut s = staged(r.game, vec![attack_row(), bool_row(false, false)]);
    replay_all(&mut s);
    assert_eq!(s.game.players[0].hand.len(), hand_before, "nothing was added");
    assert_eq!(s.game.players[0].security.len(), 3);
}

// ── 2. <Ascension> Yes / No ──────────────────────────────────────────────────

fn ascension_card(id: &str) -> CardData {
    let mut c = make_test_card_with_level(id, id, 4);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Yellow];
    c.keywords = vec![Keyword::Ascension];
    c
}

/// P0's keyword-<Ascension> Digimon has just been deleted: the Yes / No waits.
fn ascension_deleted() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(ascension_card("ASCENSION"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .security(0, &["FILL"; 3])
        .memory(5)
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    let carrier = r.place_on_field(0, "ASCENSION", None);
    r.game.delete_permanent_with_effects(carrier);
    let p = r.pending_selection().expect("the Ascension Yes / No waits");
    assert!(matches!(p.kind, SelectionKind::EffectChoice), "{:?} {}", p.kind, p.prompt);
    r
}

/// Yes (true): the carrier goes from the trash to the top of the security stack
/// (`Ascension.cs:43`, `AddSecurityCard(toTop: true)`).
#[test]
fn ascension_true_places_the_card_as_top_security() {
    let r = ascension_deleted();
    let mut s = staged(r.game, vec![bool_row(true, false)]);
    replay_all(&mut s);
    assert!(s.game.pending_selection.is_none());
    assert_eq!(s.game.players[0].security.len(), 4, "the deleted card joined the security stack");
}

/// No (false): the card stays in the trash.
#[test]
fn ascension_false_leaves_the_card_in_the_trash() {
    let r = ascension_deleted();
    let mut s = staged(r.game, vec![bool_row(false, false)]);
    replay_all(&mut s);
    assert!(s.game.pending_selection.is_none());
    assert_eq!(s.game.players[0].security.len(), 3, "declined: the security stack is unchanged");
}

/// P0's card-authored "Yes" / "No" body choice (a `select_effect_choice` inside
/// an `[On Play]`, NOT an <Ascension> frame, NOT a `BeforePayCost` reduction)
/// waits.
fn yes_no_body_choice() -> DebugRunner {
    let yaml = r#"
card: YN-1
name: YesNoCard
kind: digimon
level: 4
color: [yellow]
cost: 4
dp: 4000
effects:
  - when: [on_play]
    summary: "test: a Yes / No body choice"
    process:
      - select_effect_choice:
          bind_as: pick
          labels:
            - "Yes"
            - "No"
          prompt: "you may place this card as the top security card"
      - if:
          condition: { equals: [pick, 0] }
          then:
            - gain_memory: 1
"#;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("test YAML")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .memory(5)
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    let me = r.place_on_field(0, "YN-1", None);
    r.game.enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    let p = r.pending_selection().expect("the Yes / No waits");
    assert!(matches!(p.kind, SelectionKind::EffectChoice), "{:?} {}", p.kind, p.prompt);
    r
}

/// The mapping is the keyword's own: a card-authored Yes / No `EffectChoice`
/// outside an Ascension frame (a card whose script asks no such bool) still
/// halts on a stamped bool.
#[test]
fn a_bool_on_an_unlisted_yes_no_effect_choice_still_halts() {
    let r = yes_no_body_choice();
    let mut s = staged(r.game, vec![bool_row(true, false)]);
    s.step();
    match s.divergences() {
        [d] => assert!(matches!(d.kind, DivergenceKind::SelectionResolution { .. }), "{d:?}"),
        other => panic!("expected one divergence, got {other:?}"),
    }
}

/// The cost-reduction mapping is the engine's own `PlayFromHandCostReductionPrompt`
/// frame: an UNSTAMPED yes against a card-authored body choice is not that
/// gate, so it must not answer the choice (it is a note, the prompt stays).
#[test]
fn an_unstamped_yes_on_a_body_choice_does_not_answer_it() {
    let r = yes_no_body_choice();
    let memory = r.game.memory;
    let mut s = staged(r.game, vec![bool_row(true, true)]);
    s.step();
    assert!(
        s.game.pending_selection.is_some(),
        "the body choice must still be parked: an unstamped yes is no answer to it"
    );
    assert_eq!(s.game.memory, memory, "nothing resolved");
}

// ── 3. BT22-094 BeforePayCost cost-reduction gate ────────────────────────────

fn cs_digimon(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(4000);
    c.play_cost = cost;
    c.colors = vec![CardColor::White];
    c.traits = vec!["CS".to_string()];
    c
}

/// P0's Yuugo Kamishiro (BT22-094) is on the field and a [CS] Digimon (cost 5)
/// is in hand slot 0, in P0's Main phase with 10 memory.
fn cost_reduction_staged() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card("BT22-094")
        .expect("BT22-094 YAML")
        .add_card(cs_digimon("CS-1", 5))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["CS-1"])
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .memory(10)
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    r.place_on_field(0, "BT22-094", None);
    r
}

fn play_row() -> StepSpec {
    step_spec(space::PLAY_HAND_START, None)
}

fn field_has(s: &ReplaySession, id: &str) -> bool {
    s.game.players[0].battle_area.iter().any(|p| p.top_card().card_id(&s.game.card_data) == id)
}

/// An unstamped yes accepts the optional cost reduction: the Tamer returns to
/// the deck bottom (the cost) and the play costs 2 less.
#[test]
fn unstamped_yes_accepts_the_before_pay_cost_reduction() {
    let r = cost_reduction_staged();
    let deck_before = r.game.players[0].deck.len();
    let mut s = staged(r.game, vec![play_row(), bool_row(true, true)]);
    replay_all(&mut s);
    assert!(field_has(&s, "CS-1"), "the [CS] Digimon was played");
    assert!(!field_has(&s, "BT22-094"), "the Tamer was returned to the deck as the reduction's cost");
    assert_eq!(s.game.players[0].deck.len(), deck_before + 1);
    assert_eq!(s.game.memory, 7, "10 memory - (5 - 2)");
}

/// An unstamped no declines it: the Tamer stays and the play costs 5.
#[test]
fn unstamped_no_declines_the_before_pay_cost_reduction() {
    let r = cost_reduction_staged();
    let mut s = staged(r.game, vec![play_row(), bool_row(false, true)]);
    replay_all(&mut s);
    assert!(field_has(&s, "CS-1"), "the [CS] Digimon was played");
    assert!(field_has(&s, "BT22-094"), "the Tamer stayed");
    assert_eq!(s.game.memory, 5, "10 memory - 5");
}

// ── 4. BT23-013 Jesmon's two-mode bool ───────────────────────────────────────

/// P0's Jesmon (BT23-013) has its [When Digivolving] "Choose what Jesmon plays"
/// waiting, with a Sistermon in hand so both options are offered.
fn jesmon_choice() -> DebugRunner {
    let mut sistermon = digimon("SIS-1", "Sistermon Test", 4, 3000);
    sistermon.play_cost = 3;
    let mut r = DebugRunner::builder()
        .dsl_card("BT23-013")
        .expect("BT23-013 YAML")
        .add_card(sistermon)
        .add_card(digimon("BASE", "Base", 5, 7000))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["SIS-1"])
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .memory(5)
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    let me = r.place_stack(0, &["BASE", "BT23-013"]);
    r.game.enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    r
}

/// Drive the optional "may activate" gate, then return the mode prompt.
fn jesmon_mode_prompt() -> DebugRunner {
    let mut r = jesmon_choice();
    // The clause is optional: accept its gate until the mode choice is parked.
    for _ in 0..3 {
        let Some(p) = r.pending_selection() else { break };
        if p.prompt == "Choose what Jesmon plays" {
            break;
        }
        let accept = p.valid_action_ids.iter().copied().find(|&a| a != space::PASS).expect("an accept");
        r.execute_action(0, accept).expect("accept the gate");
    }
    let p = r.pending_selection().expect("the mode choice waits");
    assert_eq!(p.prompt, "Choose what Jesmon plays", "{:?}", p.kind);
    r
}

/// true = the Atho, Rene & Por Token (BT23_013.cs:121-141).
#[test]
fn jesmon_true_plays_the_token() {
    let r = jesmon_mode_prompt();
    let me = r.game.players[0].battle_area.len();
    let mut s = staged(r.game, vec![bool_row(true, false)]);
    replay_all(&mut s);
    assert_eq!(s.game.players[0].battle_area.len(), me + 1, "the token entered play");
    assert!(
        s.game.players[0].hand.iter().any(|c| c.card_id(&s.game.card_data) == "SIS-1"),
        "the Sistermon stayed in hand"
    );
}

/// false = a [Sistermon] from hand or trash: the Sistermon pick opens.
#[test]
fn jesmon_false_opens_the_sistermon_pick() {
    let r = jesmon_mode_prompt();
    let mut s = staged(r.game, vec![bool_row(false, false)]);
    replay_all(&mut s);
    let p = s.game.pending_selection.as_ref().expect("the Sistermon pick waits");
    assert!(p.prompt.contains("Sistermon"), "{}", p.prompt);
}
