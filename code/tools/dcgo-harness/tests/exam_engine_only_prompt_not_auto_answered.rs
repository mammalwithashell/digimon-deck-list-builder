//! A scenario's OWN row must be the thing that answers an ENGINE-ONLY prompt.
//!
//! `ReplayDriver::auto_answer_option_trash_order` exists for the DCGO corpus:
//! our engine parks the 9-1-5 / 18-1-2 -> 15-4-3-5-1 "trash the used Option, or
//! resolve one of your own triggers first" pick, DCGO makes no such decision,
//! and no recording can carry an answer — so the driver answers it the way DCGO
//! behaves (entry 0, trash now).
//!
//! An exam scenario is the opposite case: the author writes that decision
//! explicitly as a `sim_only:` row. When the driver ALSO answered it, the row
//! landed on whatever our engine parked NEXT and spent it, shifting every later
//! row one decision forward.
//!
//! Measured witness, and what this test pins: `BT25-091-effect2.yaml`. Monica
//! Simmons' `[Your Turn] When you use [TS] trait Option cards, **by suspending
//! this Tamer**, …` is a §15-7-4 optional processing condition — the player
//! CHOOSES whether to pay it — so our engine parks a decline gate before the
//! suspend, exactly where DCGO parks its `OptionalSkill`
//! (`BT25_091.cs:104`, `SetUpActivateClass(…, -1, true, …)`). With the double
//! answer, the scenario's trash-order row spent that gate, the cost auto-paid,
//! and the oracle diff read `p0.field[0].suspended: ours=true dcgo=false`
//! against the preserved sidecar — a divergence manufactured entirely by the
//! harness.
//!
//! The assertion is on the projection rather than on prompt text: the scenario
//! step that answers the decline gate must see Monica UNSUSPENDED (the cost has
//! not been paid yet), and only the following step — the target pick — sees her
//! suspended. That is the two-row shape DCGO's own sidecar has.

use dcgo_harness::exam::run::lower_and_run;
use dcgo_harness::exam::scenario::Scenario;
use dcgo_harness::exam::{ordered_deck, DeckBook};

/// Scenario step index of `select: { yes: true }` — the §15-7-4 decline gate.
/// `projections[N]` is the state BEFORE step N's decision is submitted.
const DECLINE_GATE_STEP: usize = 9;
/// Scenario step index of the mandatory `1 of your opponent's Digimon` pick.
const TARGET_PICK_STEP: usize = 10;

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repo root")
}

#[test]
fn a_scenarios_sim_only_row_answers_its_own_engine_only_prompt() {
    let root = repo_root();
    let cards_json = root.join("data/cards.json");
    let text = std::fs::read_to_string(&cards_json).expect("cards.json");
    let card_data = digimon_engine::CardData::load_from_str(&text).expect("parse cards.json");

    let scenario_path = root.join("qa/dcgo-exams/BT25/BT25-091-effect2.yaml");
    let yaml = std::fs::read_to_string(&scenario_path).expect("BT25-091-effect2.yaml");
    let scenario = Scenario::from_yaml(&yaml).expect("scenario parses");

    let decks_json = root.join("qa/dcgo-exams/BT25/tm_bt25_091_pool.json");
    let book = DeckBook::load(Some(decks_json.as_path()), cards_json.as_path()).expect("deck book");
    let deck_p0 = ordered_deck(&scenario.decks.p0, &book).expect("p0 deck");
    let deck_p1 = ordered_deck(&scenario.decks.p1, &book).expect("p1 deck");

    let run = lower_and_run(&scenario, deck_p0, deck_p1, &card_data).expect("line lowers and runs");
    assert!(
        run.complete,
        "the line did not run to completion: {:?}",
        run.stall_reasons
    );

    let monica_suspended_at = |step: usize| -> bool {
        let p = run
            .projections
            .get(step)
            .unwrap_or_else(|| panic!("no projection for step {step}"));
        let m = p
            .p0
            .field
            .iter()
            .find(|perm| perm.card_id == "BT25-091")
            .unwrap_or_else(|| panic!("Monica Simmons is not on p0's field at step {step}"));
        m.suspended
    };

    assert!(
        !monica_suspended_at(DECLINE_GATE_STEP),
        "step {DECLINE_GATE_STEP} answers Monica's \"by suspending this Tamer\" decline gate \
         (general_rule.pdf 15-7-4), so the cost CANNOT be paid yet. Seeing her already \
         suspended means the gate was consumed by an earlier row -- the driver auto-answered \
         the engine-only trash-order prompt that the scenario's own sim_only row was written \
         for, and every later row is one decision ahead of where its author put it."
    );
    assert!(
        monica_suspended_at(TARGET_PICK_STEP),
        "step {TARGET_PICK_STEP} is the mandatory opponent-Digimon pick, which DCGO \
         (BT25_091.cs: SuspendPermanentsClass then SelectPermanentEffect) reaches only after \
         the suspend cost is paid."
    );
}
