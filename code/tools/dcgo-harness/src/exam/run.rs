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

use crate::exam::adapter::{LoweredStep, ScenarioAdapter, SelectSource};
use crate::exam::assertions::check_assertions;
use crate::exam::deckbook::{ordered_deck, DeckBook};
use crate::exam::differ::{diff, DiffReport};
use crate::exam::projection::StateProjection;
use crate::exam::scenario::StepAction;
use crate::exam::scenario::Scenario;

/// Default `cards.json`, used when the caller (the MCP) has no natural place
/// to source one from.
pub const DEFAULT_CARDS_JSON: &str = "data/cards.json";
/// Default deck pool overlay (see `DeckBook::load`), covering the EX12 exam
/// campaign's `toho-*` / `st19-*` seat names. Combined with the stock starter
/// decks that `DeckBook::load` always tries first.
pub const DEFAULT_DECK_POOL: &str = "qa/dcgo-exams/EX12/toho_pool.json";

/// Resolve a repo-relative default path.
///
/// In production this crate is always invoked from the repo root (the CLI's
/// own `--cards-json`/`--decks` are given relative to it, same convention),
/// so the bare relative path resolves as-is. `DIGIMON_REPO_ROOT` is consulted
/// first so a test -- whose CWD under `cargo test` is the *package* root, not
/// the repo root (see `exam::test_support::load_card_data`, which resolves
/// the same override) -- can point this at the real repo without mutating the
/// process's working directory, which would race every other test thread
/// sharing that CWD.
pub(crate) fn resolve_default(rel: &str) -> std::path::PathBuf {
    match std::env::var("DIGIMON_REPO_ROOT") {
        Ok(root) => std::path::Path::new(&root).join(rel),
        Err(_) => std::path::PathBuf::from(rel),
    }
}

/// Our engine's live prompt immediately BEFORE one scenario step -- what that
/// step answers (`exam --inspect N`, MCP `exam_probe(inspect_step: N)`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StepSnapshot {
    pub step: usize,
    /// Debug-formatted `SelectionKind` of the prompt live before this step, if any.
    pub pending_kind: Option<String>,
    pub pending_optional: Option<bool>,
    pub pending_prompt: Option<String>,
    /// The prompt's legal action ids, each with the card it picks when the id
    /// names one (`explain_action`).
    pub candidates: Vec<(u16, Option<String>)>,
}

impl StepSnapshot {
    fn of(game: &digimon_engine::Game, step: usize) -> StepSnapshot {
        let p = game.pending_selection.as_ref();
        StepSnapshot {
            step,
            pending_kind: p.map(|p| format!("{:?}", p.kind)),
            pending_optional: p.map(|p| p.is_optional),
            pending_prompt: p.map(|p| p.prompt.clone()),
            candidates: p
                .map(|p| {
                    p.valid_action_ids
                        .iter()
                        .map(|id| (*id, candidate_card(game, p, *id)))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

/// The card a live prompt's action id picks, decoded with the same encodings
/// the scenario resolver (`runners::selection_resolve::resolve_next`) answers
/// by identity: battle-area targets (`100 + slot`, side from the kind; `AnyField`
/// carries the player), hand / trash / reveal indices of the zone's owner.
/// Other kinds fall back to `explain_action`; `None` means "not resolved
/// here", never "no card".
fn candidate_card(
    game: &digimon_engine::Game,
    p: &digimon_engine::selection::PendingSelection,
    id: u16,
) -> Option<String> {
    use digimon_engine::action::space::{
        ATTACK_START, PLAY_HAND_START, SEL_REVEAL_START, TARGETS_PER_ATTACKER, TRASH_EFFECT_START,
    };
    use digimon_engine::selection::SelectionKind;
    let cards = &game.card_data;
    let field = |player: digimon_engine::PlayerId, slot: u16| {
        game.player(player)
            .battle_area
            .get(slot as usize)
            .map(|perm| perm.top_card().card_id(cards).to_string())
    };
    let at = |zone: &[digimon_engine::CardSource], base: u16| {
        id.checked_sub(base)
            .and_then(|i| zone.get(i as usize))
            .map(|c| c.card_id(cards).to_string())
    };
    let owner = game.player(p.zone_owner.unwrap_or(p.selecting_player));
    let rel = id.checked_sub(ATTACK_START);
    match &p.kind {
        SelectionKind::OwnField => rel.filter(|s| *s < TARGETS_PER_ATTACKER).and_then(|s| field(p.selecting_player, s)),
        SelectionKind::OppField => rel
            .filter(|s| *s < TARGETS_PER_ATTACKER)
            .and_then(|s| field(game.next_clockwise(p.selecting_player), s)),
        SelectionKind::AnyField => rel.and_then(|abs| {
            field((abs / TARGETS_PER_ATTACKER) as digimon_engine::PlayerId, abs % TARGETS_PER_ATTACKER)
        }),
        SelectionKind::Hand | SelectionKind::UnionZone { .. } if id < SEL_REVEAL_START => {
            at(&owner.hand, PLAY_HAND_START)
        }
        SelectionKind::Hand | SelectionKind::UnionZone { .. } | SelectionKind::Trash
            if id >= TRASH_EFFECT_START =>
        {
            at(&owner.trash, TRASH_EFFECT_START)
        }
        SelectionKind::Reveal | SelectionKind::RevealBucket { .. } => {
            at(&game.revealed_cards, SEL_REVEAL_START)
        }
        _ => digimon_engine::action::explain::explain_action(game, p.selecting_player, id).card_id,
    }
}

/// What lowering and stepping one scenario through our engine produced.
///
/// Deliberately does NOT fail when the replay session stalls partway through
/// (`complete` is `false` instead) -- lowering and stepping are still facts
/// that happened, and the CLI's denominator (`exam: scenarios seen N /
/// lowered N / run N / ...`) counts them as such even when the line never
/// reaches its last step. Only [`ScenarioAdapter::from_scenario`] and
/// [`ReplaySession`] construction are true failures to lower.
pub struct LoweredRun {
    /// Our live prompt before every scenario step, in step order.
    pub snapshots: Vec<StepSnapshot>,
    /// The card that raised each select step's prompt (`exam --explain-selects`).
    pub select_sources: Vec<SelectSource>,
    /// Every step's lowered form, in scenario order -- what `--emit-job`
    /// turns into a DCGO scripted job.
    pub lowered_steps: Vec<LoweredStep>,
    /// Scenario-step index of each `lowered_steps` entry — see
    /// `ScenarioAdapter::lowered_owners`.
    pub lowered_owners: Vec<usize>,
    /// How many DCGO wire rows each scenario step consumes, in step order --
    /// what the differ uses to pair our per-step trace against DCGO's
    /// per-decision one (see `projection::pair_by_wire_rows`).
    pub wire_rows_per_step: Vec<usize>,
    /// The normalized state after every step run, plus one leading entry for
    /// the position before step 0.
    pub projections: Vec<StateProjection>,
    /// Per scenario step: is this a decision OUR engine also makes?
    pub ours_present_per_step: Vec<bool>,
    /// Whether the replay session ran every step of the line. `false` means
    /// it stalled -- still lowered, still (partially) stepped, just short.
    pub complete: bool,
    /// Steps the session actually advanced through.
    pub steps_run: u32,
    /// Steps the scenario asked for.
    pub steps_total: u32,
    /// Why the session stalled, when `complete` is false. Empty otherwise.
    /// Printed with the "did not run to completion" failure, because "15 of
    /// 16 steps" on its own tells an author nothing about WHICH step refused.
    pub stall_reasons: Vec<String>,
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
    let lowered_owners = adapter.lowered_owners().to_vec();
    let select_sources = adapter.select_sources().to_vec();
    // Captured BEFORE the adapter moves into the replay session below.
    let wire_rows_per_step = adapter.dcgo_wire_rows_per_step(s.steps.len());
    let ours_present_per_step = adapter.ours_present_per_step(s.steps.len());
    // How many `StepSpec`s each SCENARIO step contributes. Not always 1: a
    // `dcgo_only` row contributes none, a `materials:` / `dna:` declaration
    // several. See `ScenarioAdapter::specs_per_scenario_step`.
    let specs_per_step = adapter.specs_per_scenario_step(s.steps.len());

    let mut session = ReplaySession::with_source(Box::new(adapter), card_data, false)
        .map_err(|e| format!("building the replay session: {e:?}"))?;

    // One projection per SCENARIO step (plus a trailing one), because that is
    // the index `assert: { at: N }` and the differ's pairing both speak.
    //
    // A `dcgo_only` step has NO StepSpec -- it is a decision only DCGO makes --
    // so it must NOT consume one from the replay session. Calling `step()` for
    // it would pull the NEXT StepSpec forward, and every projection after it
    // would describe a game one step further along than the scenario index says.
    // That is not a hypothetical: it made BT24-017#effect#3 report a divergence
    // in which "our" post-clause state was compared against DCGO's pre-clause
    // row. Our state genuinely does not move at such a step, so the previous
    // projection is repeated.
    let mut projections = vec![StateProjection::from_game(&session.game, 0)];
    let mut snapshots = Vec::with_capacity(s.steps.len());
    for i in 0..s.steps.len() {
        snapshots.push(StepSnapshot::of(&session.game, i));
        for _ in 0..specs_per_step[i] {
            session.step();
        }
        projections.push(StateProjection::from_game(&session.game, i as u32 + 1));
    }

    Ok(LoweredRun {
        snapshots,
        select_sources,
        lowered_steps,
        lowered_owners,
        wire_rows_per_step,
        complete: session.is_complete(),
        steps_run: session.current_step(),
        steps_total: session.total_steps(),
        stall_reasons: session
            .divergences()
            .iter()
            .map(|d| format!("step {} action {} actor {}: {:?}", d.step, d.action_id, d.actor, d.kind))
            .collect(),
        projections,
        ours_present_per_step,
    })
}

/// The deck book a scenario's `rest:` names live in: a pool beside it, else
/// anywhere under `qa/dcgo-exams/` ([`crate::exam::deckbook::book_for`]),
/// else [`DEFAULT_DECK_POOL`] (the stock starter decks resolve under any book).
pub fn deck_book_for(scenario: &Path, s: &Scenario) -> std::path::PathBuf {
    crate::exam::deckbook::book_for(
        scenario,
        &[s.decks.p0.rest.as_str(), s.decks.p1.rest.as_str()],
        &resolve_default("qa/dcgo-exams"),
    )
    .unwrap_or_else(|| resolve_default(DEFAULT_DECK_POOL))
}

/// Read, lower and step one scenario file sim-only over its deck book (`decks`,
/// else [`deck_book_for`]) and the default `cards.json` -- whether or not the
/// line runs to completion.
fn lower_scenario_file(scenario: &Path, decks: Option<&Path>) -> Result<(Scenario, LoweredRun), String> {
    let text = std::fs::read_to_string(scenario)
        .map_err(|e| format!("reading {}: {e}", scenario.display()))?;
    let s = Scenario::from_yaml(&text)?;
    let cards_json = resolve_default(DEFAULT_CARDS_JSON);
    let card_data = dcgo_replay::load_card_data_at(&cards_json)
        .map_err(|e| format!("loading {}: {e}", cards_json.display()))?;
    let deck_pool = match decks {
        Some(d) => d.to_path_buf(),
        None => deck_book_for(scenario, &s),
    };
    let book = DeckBook::load(Some(&deck_pool), &cards_json)?;
    let deck_p0 = ordered_deck(&s.decks.p0, &book)?;
    let deck_p1 = ordered_deck(&s.decks.p1, &book)?;
    let run = lower_and_run(&s, deck_p0, deck_p1, &card_data)?;
    Ok((s, run))
}

/// `{snapshot, projection, complete, steps_run}` for step `n`: our live prompt
/// BEFORE step `n` (what that step answers) and the board it sees.
pub fn inspect_payload(run: &LoweredRun, n: usize) -> Result<serde_json::Value, String> {
    let out_of_range = || {
        format!(
            "inspect step {n} is out of range: the line has {} step(s), numbered from 0",
            run.snapshots.len()
        )
    };
    let snapshot = run.snapshots.get(n).ok_or_else(out_of_range)?;
    let projection = run.projections.get(n).ok_or_else(out_of_range)?;
    Ok(serde_json::json!({
        "snapshot": snapshot,
        "projection": projection,
        "complete": run.complete,
        "steps_run": run.steps_run,
    }))
}

/// [`inspect_payload`] for a scenario file, lowered sim-only. Works on a line
/// that stalls or fails its asserts -- exactly when an author needs it.
pub fn inspect_one(scenario: &Path, decks: Option<&Path>, n: usize) -> Result<serde_json::Value, String> {
    let (_, run) = lower_scenario_file(scenario, decks)?;
    inspect_payload(&run, n)
}

/// Run one scenario **sim-only** and report what happened, without touching
/// the verdict store or emitting a DCGO job. `decks` overrides the deck book
/// [`deck_book_for`] would pick. The oracle route is
/// [`crate::exam::oracle::run_oracle_exam`].
pub fn run_one(scenario: &Path, decks: Option<&Path>) -> Result<DiffReport, String> {
    let (s, run) = lower_scenario_file(scenario, decks)?;
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

#[cfg(test)]
mod snapshot_tests {
    use super::*;

    fn ex10_run() -> (Scenario, LoweredRun) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let yaml = std::fs::read_to_string(root.join("qa/dcgo-exams/EX10/EX10-025-effect0.yaml")).unwrap();
        let s = Scenario::from_yaml(&yaml).unwrap();
        let book = DeckBook::load(
            Some(root.join("qa/dcgo-exams/EX10/rocks_pool.json").as_path()),
            root.join("data/cards.json").as_path(),
        )
        .unwrap();
        let p0 = ordered_deck(&s.decks.p0, &book).unwrap();
        let p1 = ordered_deck(&s.decks.p1, &book).unwrap();
        let run = lower_and_run(&s, p0, p1, &crate::exam::test_support::load_card_data()).unwrap();
        (s, run)
    }

    #[test]
    fn lowered_run_snapshots_every_step() {
        let (s, run) = ex10_run();
        assert_eq!(run.snapshots.len(), s.steps.len());
        assert!(
            run.snapshots.iter().any(|sn| sn.pending_kind.is_some()),
            "a line with select rows must show at least one live prompt"
        );
        assert!(run.snapshots.iter().enumerate().all(|(i, sn)| sn.step == i));
    }

    #[test]
    fn inspect_pairs_the_live_prompt_with_the_board_before_the_step() {
        let (s, run) = ex10_run();
        let n = run.snapshots.iter().position(|sn| sn.pending_kind.is_some()).unwrap();
        let v = inspect_payload(&run, n).unwrap();
        assert_eq!(v["snapshot"]["step"], serde_json::json!(n));
        assert!(v["snapshot"]["pending_kind"].is_string(), "{v}");
        assert_eq!(v["projection"]["step"], serde_json::json!(n), "the board BEFORE step n: {v}");
        let err = inspect_payload(&run, s.steps.len()).unwrap_err();
        assert!(err.contains("out of range"), "{err}");
    }

    #[test]
    fn a_live_prompts_candidates_name_the_cards_they_pick() {
        let (_, run) = ex10_run();
        let named = run
            .snapshots
            .iter()
            .filter(|sn| sn.pending_kind.is_some())
            .flat_map(|sn| sn.candidates.iter())
            .any(|(_, card)| card.is_some());
        assert!(named, "candidates must carry card identities, not bare ids: {:?}", run.snapshots);
    }
}
