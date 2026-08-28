//! The callable core of running ONE exam scenario -- shared by the CLI's
//! `exam` command and the MCP's `run_scenario` / `exam_probe` tools.
//!
//! [`lower_and_run`] is the part that matters most: it is the ONLY place that
//! calls [`ScenarioAdapter::from_scenario`] and steps the resulting
//! [`ReplaySession`]. Both `main.rs`'s `exam_one` (the CLI's per-scenario
//! loop, oracle diffing included) and [`run_one`] below call it. A second,
//! independent implementation of that lowering-and-stepping walk -- even one
//! that started out byte-for-byte identical -- would drift the moment either
//! copy got a bugfix the other didn't, and a scenario that then lowered
//! differently between the CLI and the MCP would manufacture a "divergence"
//! that is really just two tools disagreeing with each other, not with DCGO.
//! That is exactly the class of tooling artifact this project keeps having to
//! rule out (see the first exam campaign: 6 sim-green scenarios, 6 oracle
//! failures, every one on prompt sequence).
//!
//! [`run_one`] is the MCP-facing entry point. Unlike the CLI's `exam`
//! subcommand it has no `--cards-json` / `--decks` flags to read, so it falls
//! back to the repo's default data files ([`DEFAULT_CARDS_JSON`],
//! [`DEFAULT_DECK_POOL`]) -- sensible because every committed scenario is
//! authored against the real card pool, and the EX12 pool is where the
//! current campaign lives.

use std::collections::HashMap;
use std::path::Path;

use digimon_engine::runners::replay::ReplaySession;
use digimon_engine::CardData;

use crate::exam::adapter::{LoweredStep, ScenarioAdapter};
use crate::exam::assertions::check_assertions;
use crate::exam::deckbook::{ordered_deck, DeckBook};
use crate::exam::differ::{diff, DiffReport};
use crate::exam::projection::StateProjection;
use crate::exam::scenario::Scenario;

/// Default `cards.json`, used when the caller (the MCP) has no natural place
/// to source one from.
pub const DEFAULT_CARDS_JSON: &str = "data/cards.json";
/// Default deck pool overlay (see `DeckBook::load`), covering the EX12 exam
/// campaign's `toho-*` / `st19-*` seat names. Combined with the stock starter
/// decks that `DeckBook::load` always tries first.
pub const DEFAULT_DECK_POOL: &str = "qa/dcgo-exams/EX12/toho_pool.json";

/// What lowering and stepping one scenario through our engine produced.
///
/// Deliberately does NOT fail when the replay session stalls partway through
/// (`complete` is `false` instead) -- lowering and stepping are still facts
/// that happened, and the CLI's denominator (`exam: scenarios seen N /
/// lowered N / run N / ...`) counts them as such even when the line never
/// reaches its last step. Only [`ScenarioAdapter::from_scenario`] and
/// [`ReplaySession`] construction are true failures to lower.
pub struct LoweredRun {
    /// Every step's lowered form, in scenario order -- what `--emit-job`
    /// turns into a DCGO scripted job.
    pub lowered_steps: Vec<LoweredStep>,
    /// How many DCGO wire rows each scenario step consumes, in step order --
    /// what the differ uses to pair our per-step trace against DCGO's
    /// per-decision one (see `projection::pair_by_wire_rows`).
    pub wire_rows_per_step: Vec<usize>,
    /// The normalized state after every step run, plus one leading entry for
    /// the position before step 0.
    pub projections: Vec<StateProjection>,
    /// Whether the replay session ran every step of the line. `false` means
    /// it stalled -- still lowered, still (partially) stepped, just short.
    pub complete: bool,
    /// Steps the session actually advanced through.
    pub steps_run: u32,
    /// Steps the scenario asked for.
    pub steps_total: u32,
}

/// Lower `s` against a freshly-built game for the two given decks, then step
/// exactly as many times as the line is long, projecting the state after
/// each step.
///
/// A bounded loop rather than `while !is_complete()`: a source that stops
/// advancing would otherwise spin forever and read as a hang.
pub fn lower_and_run(
    s: &Scenario,
    deck_p0: Vec<String>,
    deck_p1: Vec<String>,
    card_data: &HashMap<String, CardData>,
) -> Result<LoweredRun, String> {
    let adapter = ScenarioAdapter::from_scenario(s, deck_p0, deck_p1, card_data)?;
    let lowered_steps = adapter.lowered_steps().to_vec();
    // Captured BEFORE the adapter moves into the replay session below.
    let wire_rows_per_step = adapter.dcgo_wire_rows_per_step();

    let mut session = ReplaySession::with_source(Box::new(adapter), card_data, false)
        .map_err(|e| format!("building the replay session: {e:?}"))?;

    let mut projections = vec![StateProjection::from_game(&session.game, 0)];
    for i in 0..s.steps.len() as u32 {
        session.step();
        projections.push(StateProjection::from_game(&session.game, i + 1));
    }

    Ok(LoweredRun {
        lowered_steps,
        wire_rows_per_step,
        complete: session.is_complete(),
        steps_run: session.current_step(),
        steps_total: session.total_steps(),
        projections,
    })
}

/// Run one scenario **sim-only** and report what happened, without touching
/// the verdict store or emitting a DCGO job.
///
/// `sim_only: false` is refused rather than faked: an oracle diff needs a
/// DCGO state sidecar next to a real recording, and nothing upstream of this
/// call (a freshly-written probe scratch file, or a scenario path handed to
/// `run_scenario`) has run Unity to produce one. Pretending to answer that
/// question with sim-only data is exactly the mistake this tool exists to
/// prevent -- see the module doc and `SIM_ONLY_NOTE` in `mcp::handlers`.
///
/// `root` is accepted for the day this queues a real harness job against
/// `root`'s job directories and polls for the sidecar it writes; today it is
/// unused because that queueing does not exist yet.
pub fn run_one(
    scenario: &Path,
    sim_only: bool,
    root: Option<&Path>,
) -> Result<DiffReport, String> {
    let text = std::fs::read_to_string(scenario)
        .map_err(|e| format!("reading {}: {e}", scenario.display()))?;
    let s = Scenario::from_yaml(&text)?;

    if !sim_only {
        let _ = root;
        return Err(
            "oracle mode (sim_only: false) needs a DCGO state sidecar next to a real \
             recording, and this probe has not run one -- there is no Unity trace behind \
             a scratch scenario or a bare scenario path. Run the CLI's \
             `dcgo-harness exam --sidecar <dir>` against a recording captured through the \
             harness queue once one exists."
                .to_string(),
        );
    }

    let cards_json = Path::new(DEFAULT_CARDS_JSON);
    let card_data = dcgo_replay::load_card_data_at(cards_json)
        .map_err(|e| format!("loading {}: {e}", cards_json.display()))?;
    let book = DeckBook::load(Some(Path::new(DEFAULT_DECK_POOL)), cards_json)?;
    let deck_p0 = ordered_deck(&s.decks.p0, &book)?;
    let deck_p1 = ordered_deck(&s.decks.p1, &book)?;

    let run = lower_and_run(&s, deck_p0, deck_p1, &card_data)?;
    if !run.complete {
        return Err(format!(
            "the line did not run to completion: {} of {} steps",
            run.steps_run, run.steps_total
        ));
    }

    let (checked, failures) = check_assertions(&s, &run.projections);
    if !failures.is_empty() {
        return Err(format!(
            "{} of {checked} assertion check(s) failed: {}",
            failures.len(),
            failures.join("; ")
        ));
    }

    // No oracle ran, so nothing was actually compared against DCGO. `diff`
    // against an empty trace reports that honestly: `compared_steps` stays 0
    // and `is_clean()` is false, rather than manufacturing an oracle
    // agreement nobody measured. The caller (`mcp::handlers::exam_probe`)
    // states this in its own "clean" / "note" fields; this report is the raw
    // material for that, not the final word.
    Ok(diff(&run.projections, &[]))
}
