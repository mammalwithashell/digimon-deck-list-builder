//! DCGO replay: a card-less `MultipleSkills` index whose evidence row meets a
//! one-candidate pick DCGO never asked.
//!
//! Owen Dreadnought (EX11-054)'s [All Turns] trigger, accepted, draws 1 and
//! then gives "1 of your Digimon with <Progress>" +3000 DP. DCGO asks that
//! pick only when there are two or more such Digimon; with one it takes it
//! without a prompt (EX11_054.cs:66-69, `validTargets.Count == 1`), so the
//! recording has no row for it. `step()` answers such a lone pick before the
//! recorded row that cannot resolve against it (the mandatory/actor's
//! one-option loop in `apply_selection_step`). The trigger-order probe
//! (`remote_trigger_order_choice`) must meet its evidence row the same way,
//! or it rejects the entry DCGO actually chose.
//!
//! Field witness: raw-20261113T011922Z#g3, raw seq 342-347. Medusamon
//! (EX11-012) digivolved with Owen on the field; P1 (no recorder) ordered
//! [Medusamon's When Digivolving, Owen] with a card-less index, answered
//! Owen's "use this effect?" yes (seq 343; Medusamon's When Digivolving is not
//! an optional activation in DCGO, EX11_012.cs:108), then picked the
//! opponent's Digimon for Medusamon's delete (seq 344). The replay rejected
//! Owen ("leads to OwnField 'Choose a Digimon with Progress'"), took
//! Medusamon, and later met DCGO's attack with Owen's trigger still pending.
//!
//! Staged with `DebugRunner` (real EX11-054 / EX11-012 YAML, synthetic other
//! cards). No recording content.

use std::collections::HashMap;

use digimon_engine::action::space::{ATTACK_START, DIGIVOLVE_START, FIELDS_PER_HAND, SECURITY_TARGET, TARGETS_PER_ATTACKER};
use digimon_engine::card_data::CardData;
use digimon_engine::dcgo_recording::SelectionRow;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::game::Game;
use digimon_engine::runners::replay::{RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec};

fn digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 5;
    c.colors = vec![CardColor::Red];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
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

fn spec(actor: u8, action_id: u16, source: &str, selection: Option<SelectionRow>) -> StepSpec {
    StepSpec {
        actor,
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

fn row(json: serde_json::Value) -> StepSpec {
    let selection: SelectionRow = serde_json::from_value(json).expect("selection row");
    spec(selection.actor, 0, "selection", Some(selection))
}

fn action(action_id: u16) -> StepSpec {
    spec(0, action_id, "main_phase", None)
}

/// P0: Owen Dreadnought (EX11-054) and a red level-5 [Dragonkin] on the
/// field, Medusamon (EX11-012) in hand. P1: one 5000 DP Digimon, one card in
/// the trash, one security card.
fn staged_board() -> (DebugRunner, u16) {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-054")
        .expect("EX11-054 YAML loads")
        .dsl_card("EX11-012")
        .expect("EX11-012 YAML loads")
        .add_card(digimon("DRAGON5", 5, 6000, &["Dragonkin"]))
        .add_card(digimon("VICTIM", 4, 5000, &[]))
        .add_card(digimon("MISS", 3, 1000, &[]))
        .hand(0, &["EX11-012"])
        .deck(0, &["MISS"; 6])
        .deck(1, &["MISS"; 6])
        .security(1, &["MISS"])
        .memory(5)
        .start();
    runner.place_on_field(0, "EX11-054", None);
    runner.place_on_field(0, "DRAGON5", None);
    runner.place_on_field(1, "VICTIM", None);
    runner.inject_trash(1, "MISS");
    let digivolve = DIGIVOLVE_START + 0 * FIELDS_PER_HAND + 1;
    (runner, digivolve)
}

fn session(game: Game, steps: Vec<StepSpec>) -> ReplaySession {
    ReplaySession::with_source(Box::new(StagedDcgo { game, steps }), &HashMap::new(), false)
        .expect("staged session")
}

fn top_id(game: &Game, player: usize, slot: usize) -> String {
    game.players[player].battle_area[slot].top_card().card_id(&game.card_data).to_string()
}

/// DCGO's rows after the digivolve: the card-less order index, Owen's "use
/// this effect?" yes, Medusamon's delete pick, its trash pick, then the
/// attack. Owen's lone Progress pick has no row. The order resolves to Owen,
/// and the whole line replays with no divergence.
#[test]
fn order_evidence_meets_dcgo_silent_lone_pick_like_step_does() {
    let (runner, digivolve) = staged_board();
    let medusamon_slot = 1u16;
    let attack = ATTACK_START + medusamon_slot * TARGETS_PER_ATTACKER + SECURITY_TARGET;
    let mut s = session(
        runner.game,
        vec![
            action(digivolve),
            row(serde_json::json!({"step":1,"actor":0,"prompt":"MultipleSkills","int_value":1})),
            row(serde_json::json!({"step":2,"actor":0,"prompt":"generic_bool","bool_value":true,"unstamped":true})),
            row(serde_json::json!({"step":3,"actor":0,"prompt":"SelectPermanentEffect","targets":[{"player":1,"frame":0}]})),
            row(serde_json::json!({"step":4,"actor":0,"prompt":"SelectCardEffect","card_ids":["MISS"]})),
            action(attack),
        ],
    );

    s.step();
    assert!(s.divergences().is_empty(), "digivolve: {:?}", s.divergences());
    assert_eq!(top_id(&s.game, 0, 1), "EX11-012", "Medusamon digivolved onto DRAGON5");
    let order = s.game.pending_selection.as_ref().expect("the engine orders the two triggers");
    assert_eq!(order.effect_choices.as_ref().map(|e| e.len()), Some(2), "{}", order.prompt);

    for _ in 0..12 {
        if s.is_complete() || s.is_paused() {
            break;
        }
        s.step();
    }
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete(), "every row replays");
    assert!(s.game.players[0].battle_area[0].is_suspended, "Owen paid its suspend cost: DCGO ran Owen's effect");
    assert!(
        s.game.players[1].battle_area.iter().all(|p| p.top_card().card_id(&s.game.card_data) != "VICTIM"),
        "Medusamon deleted P1's Digimon"
    );
}

/// The same line without Owen's "use this effect?" yes cannot be Owen's: the
/// order goes to Medusamon, whose delete the next row answers, and the attack
/// then meets Owen's still-pending trigger. A guard: the evidence fold must
/// not make the order prompt accept anything.
#[test]
fn without_owens_yes_the_order_is_not_resolved_to_owen() {
    let (runner, digivolve) = staged_board();
    let attack = ATTACK_START + TARGETS_PER_ATTACKER + SECURITY_TARGET;
    let mut s = session(
        runner.game,
        vec![
            action(digivolve),
            row(serde_json::json!({"step":1,"actor":0,"prompt":"MultipleSkills","int_value":1})),
            row(serde_json::json!({"step":3,"actor":0,"prompt":"SelectPermanentEffect","targets":[{"player":1,"frame":0}]})),
            row(serde_json::json!({"step":4,"actor":0,"prompt":"SelectCardEffect","card_ids":["MISS"]})),
            action(attack),
        ],
    );
    for _ in 0..12 {
        if s.is_complete() || s.is_paused() {
            break;
        }
        s.step();
    }
    assert!(!s.game.players[0].battle_area[0].is_suspended, "Owen's effect never ran");
}
