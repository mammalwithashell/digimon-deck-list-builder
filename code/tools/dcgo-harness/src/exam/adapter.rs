//! `RecordingSource` over a hand-authored scenario line.
//!
//! The third implementation of the trait, alongside `NativeAdapter` (engine
//! recordings) and `DcgoAdapter` (the oracle). Being a `RecordingSource` rather
//! than a bespoke runner is the single most load-bearing reuse decision in this
//! work: the divergence machinery, step policy, reset-and-replay backward seek,
//! and player-perspective conversion are all inherited, and a scenario run is
//! structurally the same object as a corpus replay.
//!
//! **Lowering happens at construction, not at run time.** `from_scenario` walks
//! the line in a throwaway live game, resolving each symbolic step against that
//! position's mask, so an illegal or ambiguous scenario fails in milliseconds
//! -- before Unity is ever launched. That is the whole reason the exam lowers
//! up front rather than letting each engine interpret the symbolic form itself.

use std::collections::HashMap;

use digimon_engine::runners::replay::{RecordingSource, ReplayError, StepPolicy, StepSpec};
use digimon_engine::{CardData, Game, PlayerId, Rules};

use crate::exam::lower::{lower_step, LowerError};
use crate::exam::scenario::Scenario;

/// Seat that acts first in a scenario game.
///
/// Fixed rather than derived from the seed: `Game::new`'s `seed % 2` pick would
/// otherwise decide which seat a scenario's hard-coded `actor: 0` refers to,
/// making the same YAML mean two different lines depending on the seed. DCGO
/// does not honor a job's `first_player` (a standing phase-1 gap), so the
/// reconciliation between the two sides is the runner's problem, not this
/// adapter's -- but our side must at least be deterministic.
const SCENARIO_FIRST_PLAYER: PlayerId = 0;

#[derive(Debug)]
pub struct ScenarioAdapter {
    steps: Vec<StepSpec>,
    lowered: Vec<u16>,
    deck_p0: Vec<String>,
    deck_p1: Vec<String>,
    seed: u64,
    first_player: PlayerId,
    /// Owned pool, so `relay_initial_state` (which gets no `card_data`
    /// argument) can rebuild the game. `VerificationReplayAdapter` does the
    /// same for the same reason.
    card_data: HashMap<String, CardData>,
}

impl ScenarioAdapter {
    pub fn from_scenario(
        s: &Scenario,
        deck_p0: Vec<String>,
        deck_p1: Vec<String>,
        card_data: &HashMap<String, CardData>,
    ) -> Result<ScenarioAdapter, String> {
        let first_player = SCENARIO_FIRST_PLAYER;

        // Lowering must walk the line in a live game, because each step's legal
        // set depends on every step before it. A one-shot pass over a static
        // position could only ever lower step 0.
        let mut game = construct(&deck_p0, &deck_p1, card_data, s.seed, first_player)
            .map_err(|e| format!("scenario game setup failed: {e}"))?;

        let mut steps = Vec::with_capacity(s.steps.len());
        let mut lowered = Vec::with_capacity(s.steps.len());

        for (i, step) in s.steps.iter().enumerate() {
            let actor = step.actor as PlayerId;
            let action_id = lower_step(&game, actor, &step.act).map_err(|e| match e {
                LowerError::NoMatch { intent, legal } => format!(
                    "step {i}: no legal action matches {intent}\n  legal here:\n    {}",
                    legal.join("\n    ")
                ),
                LowerError::Ambiguous { intent, matches } => format!(
                    "step {i}: {intent} is ambiguous -- {matches:?} all match. \
                     Narrow the step; picking arbitrarily would silently answer \
                     a different question than the scenario asks."
                ),
            })?;

            steps.push(StepSpec {
                actor,
                action_id,
                phase: game.current_phase.py_name().to_string(),
                source: "scenario".to_string(),
                memory_after: None,
                dcgo_memory: None,
                turn: Some(game.turn_count as u64),
                is_game_over: None,
                expected_digest: None,
                selection: None,
                board_p0: None,
                board_p1: None,
            });
            lowered.push(action_id);

            // `Game::decode_action` returns unit and SILENTLY IGNORES an
            // illegal or out-of-range id, so there is no error to propagate
            // from the apply itself. The mask check below is what makes a bad
            // lowering loud instead of silent; without it a scenario could
            // "run" while every step after the first was a no-op.
            let mask = digimon_engine::action::mask::build_action_mask(&game, actor);
            if mask[action_id as usize] != 1.0 {
                return Err(format!(
                    "step {i}: lowered action {action_id} is not in the mask"
                ));
            }
            game.decode_action(action_id, actor);
        }

        Ok(ScenarioAdapter {
            steps,
            lowered,
            deck_p0,
            deck_p1,
            seed: s.seed,
            first_player,
            card_data: card_data.clone(),
        })
    }

    pub fn lowered_action_ids(&self) -> &[u16] {
        &self.lowered
    }
}

/// The one place a scenario game is constructed, shared by lowering and by the
/// `RecordingSource` build/relay hooks.
///
/// Both sides MUST start from an identical position or the lowered ids stop
/// meaning what they meant when they were resolved, so this is deliberately a
/// single function rather than two similar-looking call sites.
fn construct(
    deck_p0: &[String],
    deck_p1: &[String],
    card_data: &HashMap<String, CardData>,
    seed: u64,
    first_player: PlayerId,
) -> Result<Game, String> {
    let mut game = Game::new_with_ordered_decks(
        &[deck_p0.to_vec(), deck_p1.to_vec()],
        card_data,
        Rules::standard(),
        Some(seed),
        first_player,
    )?;
    // Resolve both mulligans (keep) and enter turn 1. The scenario step
    // vocabulary has no mulligan verb -- on the DCGO side the mulligan is its
    // own recorder row type, not one of the scripted prompts -- so a scenario
    // line always begins after the mulligan on both sides.
    game.start_game();
    Ok(game)
}

impl RecordingSource for ScenarioAdapter {
    fn build_initial_game(
        &self,
        _card_data: &HashMap<String, CardData>,
    ) -> Result<Game, ReplayError> {
        construct(
            &self.deck_p0,
            &self.deck_p1,
            &self.card_data,
            self.seed,
            self.first_player,
        )
        .map_err(ReplayError::GameConstruction)
    }

    fn relay_initial_state(&self, game: &mut Game) -> Result<(), ReplayError> {
        // A scenario has no post-mulligan snapshot to re-lay: the game is
        // rebuilt deterministically from (decks, seed, first_player), so
        // reset-and-replay just reconstructs it.
        *game = self.build_initial_game(&self.card_data)?;
        Ok(())
    }

    fn steps(&self) -> &[StepSpec] {
        &self.steps
    }

    fn default_policy(&self) -> StepPolicy {
        // Trust: this line came from our own mask, so there is nothing to check
        // it against at this layer. The oracle comparison is the differ's job,
        // over state projections.
        StepPolicy::Trust
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exam::scenario::Scenario;
    use crate::exam::test_support;
    use digimon_engine::runners::replay::ReplaySession;

    const LINE: &str = r#"
card: ST1-02
clause: ST1-02#effect#0
seed: 7
decks:
  p0: { stack: [], rest: simple }
  p1: { stack: [], rest: simple }
steps:
  - actor: 0
    do: { pass: {} }
"#;

    /// The stock ST-1 list (50 main + 4 egg), in printed order.
    ///
    /// Mirrors `lower.rs`'s helper rather than adding a shared fixture: a
    /// tournament-legal list matters because DCGO gates battles on
    /// `DeckData.IsValidDeckData()`, so a scenario meant to mirror DCGO cannot
    /// use an ad-hoc deck.
    fn simple_deck() -> Vec<String> {
        let (mut main, egg) = test_support::st1_decks();
        main.extend(egg);
        main
    }

    #[test]
    fn adapter_builds_a_game_and_lowers_the_line() {
        let card_data = test_support::load_card_data();
        let deck = simple_deck();
        let s = Scenario::from_yaml(LINE).unwrap();
        let a = ScenarioAdapter::from_scenario(&s, deck.clone(), deck, &card_data)
            .expect("adapter should build");
        assert_eq!(a.lowered_action_ids().len(), 1);
        assert_eq!(a.steps().len(), 1);
    }

    #[test]
    fn adapter_default_policy_is_trust() {
        // Our engine generated this line itself, so there is no oracle to check
        // it against at this layer. The DCGO comparison happens in the differ,
        // over state projections -- not by re-checking our own actions.
        let card_data = test_support::load_card_data();
        let deck = simple_deck();
        let s = Scenario::from_yaml(LINE).unwrap();
        let a = ScenarioAdapter::from_scenario(&s, deck.clone(), deck, &card_data).unwrap();
        assert_eq!(a.default_policy(), StepPolicy::Trust);
    }

    #[test]
    fn session_runs_the_line_to_completion() {
        let card_data = test_support::load_card_data();
        let deck = simple_deck();
        let s = Scenario::from_yaml(LINE).unwrap();
        let a = ScenarioAdapter::from_scenario(&s, deck.clone(), deck, &card_data).unwrap();
        let mut session = ReplaySession::with_source(Box::new(a), &card_data, false)
            .expect("session should build");
        session.run_to_completion();
        assert!(session.is_complete());
        assert!(
            session.divergences().is_empty(),
            "{:?}",
            session.divergences()
        );
    }

    #[test]
    fn an_illegal_line_fails_to_build_not_at_run_time() {
        // The whole point of lowering up front: a malformed scenario must fail
        // in milliseconds, before any Unity launch.
        let bad = LINE.replace(
            "do: { pass: {} }",
            "do: { play: { card: ZZ99-999, from: hand } }",
        );
        let card_data = test_support::load_card_data();
        let deck = simple_deck();
        let s = Scenario::from_yaml(&bad).unwrap();
        let err =
            ScenarioAdapter::from_scenario(&s, deck.clone(), deck, &card_data).unwrap_err();
        assert!(err.contains("ZZ99-999"), "got: {err}");
    }
}
