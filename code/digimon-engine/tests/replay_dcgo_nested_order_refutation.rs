//! DCGO replay: a card-less `MultipleSkills` order whose candidates are told
//! apart only by a NESTED order further down the line.
//!
//! Shape of raw-20261113T011922Z#g3, raw seq 331-347 (the remote seat P1, no
//! recorder, so every `MultipleSkills` row is card-less). Cyberdramon
//! (BT21-024) digivolves onto Dimetromon (BT21-017), over Elizamon (EX11-008)
//! over Gigimon (BT21-001), with Owen Dreadnought (EX11-054) on the field.
//! Its [When Digivolving] trashes the opponent's top security card, which
//! triggers three inherited "[Your Turn] When your opponent's security stack
//! is removed from" effects at once: Gigimon's "1 of your Digimon may
//! digivolve into a [Reptile]/[Dragonkin] card in the hand, cost -1",
//! Elizamon's and Dimetromon's "gain 1 memory". DCGO's rows: index 1, index
//! 0, a "use this effect?" yes, the digivolve pick (Medusamon (EX11-012)),
//! index 1 between Medusamon's [When Digivolving] and Owen, Owen's yes, the
//! delete pick, the trash pick, then the attack.
//!
//! * Elizamon first and Dimetromon first are the same game once both memory
//!   gains have resolved (CR 4.3 15-4-3-4: the order is chosen one effect at
//!   a time; the two gains commute).
//! * Gigimon first is refuted, but only two orders deep: under it the
//!   Medusamon/Owen order has no continuation the rows accept (both leave an
//!   ordering decision DCGO never recorded in front of the attack -
//!   MultipleSkills.cs:261-262, 354, 420: two or more activatable effects
//!   always produce a `SetTargetSkill` row).
//!
//! A nested order whose every entry the rows refute refutes the branch that
//! led to it; it is not an ambiguity that merely ends the evidence.
//!
//! Staged with `DebugRunner` (real BT21-001 / EX11-008 / BT21-017 / BT21-024
//! / EX11-012 / EX11-054 YAML, synthetic other cards). No recording content.

use std::collections::HashMap;

use digimon_engine::action::space::{ATTACK_START, DIGIVOLVE_START, FIELDS_PER_HAND, SECURITY_TARGET, TARGETS_PER_ATTACKER};
use digimon_engine::card_data::CardData;
use digimon_engine::dcgo_recording::SelectionRow;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::game::Game;
use digimon_engine::runners::replay::{DivergenceKind, RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec};

fn digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 5;
    c.colors = vec![CardColor::Yellow];
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

/// P0 (turn player): Owen Dreadnought (EX11-054) in slot 0, Dimetromon
/// (BT21-017) over Elizamon (EX11-008) over Gigimon (BT21-001) in slot 1;
/// Cyberdramon (BT21-024) and Medusamon (EX11-012) in hand; 4 memory. P1: one
/// 5000 DP Digimon, one hand card, two security cards.
fn staged_board() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-001")
        .expect("BT21-001 YAML loads")
        .dsl_card("EX11-008")
        .expect("EX11-008 YAML loads")
        .dsl_card("BT21-017")
        .expect("BT21-017 YAML loads")
        .dsl_card("BT21-024")
        .expect("BT21-024 YAML loads")
        .dsl_card("EX11-012")
        .expect("EX11-012 YAML loads")
        .dsl_card("EX11-054")
        .expect("EX11-054 YAML loads")
        .add_card(digimon("VICTIM", 4, 5000))
        .add_card(digimon("SECBOT", 3, 1000))
        .add_card(digimon("SEC1", 3, 1000))
        .add_card(digimon("MISS", 3, 1000))
        .hand(0, &["BT21-024", "EX11-012"])
        .hand(1, &["SECBOT"])
        .deck(0, &["MISS"; 6])
        .deck(1, &["MISS"; 6])
        .security(1, &["SEC1", "SEC1"])
        .memory(4)
        .start();
    runner.place_on_field(0, "EX11-054", None);
    runner.place_stack(0, &["BT21-001", "EX11-008", "BT21-017"]);
    runner.place_on_field(1, "VICTIM", None);
    runner
}

fn session(game: Game, steps: Vec<StepSpec>) -> ReplaySession {
    ReplaySession::with_source(Box::new(StagedDcgo { game, steps }), &HashMap::new(), false)
        .expect("staged session")
}

fn dcgo_rows() -> Vec<StepSpec> {
    let stack_slot = 1u16;
    let digivolve = DIGIVOLVE_START + 0 * FIELDS_PER_HAND + stack_slot;
    let attack = ATTACK_START + stack_slot * TARGETS_PER_ATTACKER + SECURITY_TARGET;
    vec![
        action(digivolve),
        // Cyberdramon's [When Digivolving] or Owen.
        row(serde_json::json!({"step":1,"actor":0,"prompt":"MultipleSkills","int_value":0})),
        // P1 places a hand card at the bottom of its security.
        row(serde_json::json!({"step":2,"actor":1,"prompt":"SelectHandEffect","zone":"Hand","card_ids":["SECBOT"]})),
        // The three security-removed triggers.
        row(serde_json::json!({"step":3,"actor":0,"prompt":"MultipleSkills","int_value":1})),
        row(serde_json::json!({"step":4,"actor":0,"prompt":"MultipleSkills","int_value":0})),
        row(serde_json::json!({"step":5,"actor":0,"prompt":"generic_bool","bool_value":true,"unstamped":true})),
        row(serde_json::json!({"step":6,"actor":0,"prompt":"SelectPermanentEffect","targets":[{"player":0,"frame":1}]})),
        row(serde_json::json!({"step":7,"actor":0,"prompt":"SelectHandEffect","zone":"Hand","card_ids":["EX11-012"]})),
        // Medusamon's [When Digivolving] or Owen.
        row(serde_json::json!({"step":8,"actor":0,"prompt":"MultipleSkills","int_value":1})),
        row(serde_json::json!({"step":9,"actor":0,"prompt":"generic_bool","bool_value":true,"unstamped":true})),
        row(serde_json::json!({"step":10,"actor":0,"prompt":"SelectPermanentEffect","targets":[{"player":1,"frame":0}]})),
        row(serde_json::json!({"step":11,"actor":0,"prompt":"SelectCardEffect","card_ids":["SEC1"]})),
        action(attack),
    ]
}

/// The whole line replays: the three-way order resolves (the two memory
/// gains commute, Gigimon first is refuted two orders deep), Owen pays its
/// suspend, Medusamon deletes P1's Digimon and attacks.
#[test]
fn nested_refutation_resolves_the_three_way_security_order() {
    let runner = staged_board();
    let mut s = session(runner.game, dcgo_rows());
    for _ in 0..30 {
        if s.is_complete() || s.is_paused() {
            break;
        }
        s.step();
    }
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete(), "every row replays");
    let g = &s.game;
    assert_eq!(
        g.players[0].battle_area[1].top_card().card_id(&g.card_data),
        "EX11-012",
        "Gigimon's effect digivolved Cyberdramon into Medusamon"
    );
    assert!(g.players[0].battle_area[0].is_suspended, "Owen paid its suspend cost");
    assert!(
        g.players[1].battle_area.iter().all(|p| p.top_card().card_id(&g.card_data) != "VICTIM"),
        "Medusamon deleted P1's Digimon"
    );
}

/// Every entry of an order refuted by the rows that follow is reported as
/// such, not as an ambiguity: here every row from the second order index on
/// is one nothing in the line can answer (P1 picking a hand card), so no
/// order survives the rows - not even the first one, whose lookahead now
/// counts the nested all-refuted order against each of its branches. The
/// replay still stops.
#[test]
fn an_order_every_entry_of_which_the_rows_refute_is_not_called_ambiguous() {
    let runner = staged_board();
    let mut rows = dcgo_rows();
    // From the second index on, the rows say something no order can lead to:
    // the opponent answers a hand pick where P0's decisions belong.
    for r in rows.iter_mut().skip(4) {
        *r = row(serde_json::json!({"step":9,"actor":1,"prompt":"SelectHandEffect","zone":"Hand","card_ids":["MISS"]}));
    }
    let mut s = session(runner.game, rows);
    for _ in 0..30 {
        if s.is_complete() || s.is_paused() {
            break;
        }
        s.step();
    }
    assert!(s.is_paused(), "the replay stops");
    let reason = s
        .divergences()
        .iter()
        .find_map(|d| match &d.kind {
            DivergenceKind::SelectionResolution { reason } if reason.contains("MultipleSkills index") => Some(reason.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a SelectionResolution on an order: {:?}", s.divergences()));
    assert!(!reason.contains("is ambiguous"), "{reason}");
    assert!(reason.contains("refute each"), "{reason}");
}

/// A `MultipleSkills` index right after the one that chose Gigimon's
/// effect cannot be "consumed by the chosen effect's own prompt": DCGO runs
/// the effect it chose before it orders again (MultipleSkills.cs:487-545:
/// `Activate` runs the chosen effect - its "use?" prompt first when it is
/// optional, ICardEffect.cs:1241-1246 - and only then the loop asks the next
/// order), so the effect's first decision would be recorded first. Here the
/// recorder seat names Gigimon (BT21-001) as its choice, and the very next
/// row is another card-less order index while the engine parks Gigimon's
/// digivolve pick: the replay stops instead of dropping the index.
#[test]
fn an_order_index_right_after_choosing_an_effect_is_not_consumed_by_that_effects_prompt() {
    let runner = staged_board();
    let rows = vec![
        dcgo_rows().remove(0),
        row(serde_json::json!({"step":1,"actor":0,"prompt":"MultipleSkills","int_value":0})),
        row(serde_json::json!({"step":2,"actor":1,"prompt":"SelectHandEffect","zone":"Hand","card_ids":["SECBOT"]})),
        row(serde_json::json!({"step":3,"actor":0,"prompt":"MultipleSkills","int_value":0,"chosen_effect_card":"BT21-001"})),
        row(serde_json::json!({"step":4,"actor":0,"prompt":"MultipleSkills","int_value":0})),
        row(serde_json::json!({"step":5,"actor":0,"prompt":"SelectPermanentEffect","targets":[{"player":0,"frame":1}]})),
    ];
    let mut s = session(runner.game, rows);
    for _ in 0..10 {
        if s.is_complete() || s.is_paused() {
            break;
        }
        s.step();
    }
    assert!(s.is_paused(), "the replay stops; notes {:?}", s.replay_notes());
    // (The lookahead of the FIRST order already meets this row under every
    // branch, so the stop may be reported there.)
    assert!(s.current_step() <= 4, "{:?}", s.divergences());
    assert!(
        !s.replay_notes().iter().any(|n| n.contains("consumed by the chosen effect's own")),
        "the index was not dropped: {:?}",
        s.replay_notes()
    );
}
