# DCGO Exam Scenario Format and Differ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn a hand-authored scenario YAML into a validated line both engines can run, execute it in our engine, and diff the result against DCGO's state sidecar — leading with the first divergence.

**Architecture:** A new `dcgo-exam` module inside the existing `dcgo-harness` crate. Symbolic steps are lowered to action IDs by matching `explain_action` output against the live `build_action_mask`, so an illegal or ambiguous intent fails in milliseconds instead of after sixty seconds of Unity. Our engine runs the line as a third `RecordingSource` implementation (`ScenarioAdapter`) alongside the existing `NativeAdapter` and `DcgoAdapter`, inheriting the whole `ReplaySession` core rather than reimplementing it.

**Tech Stack:** Rust 2021, `serde_yml` 0.0.12 (the repo's YAML crate — *not* `serde_yaml`), `serde` / `serde_json`, `clap` 4 derive.

## Global Constraints

- **Per-worktree `CARGO_TARGET_DIR`** (CLAUDE.md rule 31). A shared target dir links one worktree's `.rmeta` into another's build and produces phantom compile errors in files you never edited. If the harness inherited a stale env, prefix every cargo command with `CARGO_TARGET_DIR='D:\cargo-target-wt\bold-bassi-d34dc7'`.
- The YAML crate is **`serde_yml = "0.0.12"`**, matching `code/digimon-dsl/Cargo.toml`. Do not introduce `serde_yaml`.
- **Never hand-maintain a second copy of the action space.** All lowering goes through `digimon_engine::action::explain::explain_action` and `digimon_engine::action::mask::build_action_mask`.
- **`job.first_player` is not honored by DCGO** (a standing phase-1 known gap, unchanged by the Unity plan). Our engine *can* honor it via `Game::new_with_ordered_decks`. Task 6 must therefore not assume the two agree — see its seat-reconciliation step.
- Every report prints the **full denominator**. A run where most scenarios failed to lower must never read as a pass.
- `dcgo-harness` is dev/test tooling: never imported by `server.*` or `digimon_gym.*`, never bundled into a production build.

## File Structure

| File | Responsibility |
|---|---|
| `code/tools/dcgo-harness/src/exam/mod.rs` (create) | Module root + re-exports. |
| `code/tools/dcgo-harness/src/exam/scenario.rs` (create) | The YAML schema types and their parse/validate rules. |
| `code/tools/dcgo-harness/src/exam/lower.rs` (create) | Symbolic step → action ID, against the live mask. |
| `code/tools/dcgo-harness/src/exam/projection.rs` (create) | The normalized per-step state projection + its DCGO-sidecar parser. |
| `code/tools/dcgo-harness/src/exam/adapter.rs` (create) | `ScenarioAdapter: RecordingSource`. |
| `code/tools/dcgo-harness/src/exam/differ.rs` (create) | Align two projections, report the first divergence. |
| `code/tools/dcgo-harness/src/main.rs` (modify) | Add the `exam` subcommand. |
| `code/tools/dcgo-harness/src/lib.rs` (modify) | `pub mod exam;` |
| `code/tools/dcgo-harness/Cargo.toml` (modify) | Add `serde_yml`. |
| `code/tools/dcgo-harness/tests/exam_*.rs` (create) | Integration tests over committed fixtures. |

**Why a module inside `dcgo-harness` and not a new crate:** the existing binary already owns the job queue, the manifest gate, and triage. The agent surface (plan 3) is a subcommand of the same binary so every behavior stays unit-testable with no MCP client — the same reasoning that keeps `dcgo-replay` and `digimon-engine-mcp` sharing one core.

---

### Task 1: Scenario schema and validation

**Files:**
- Create: `code/tools/dcgo-harness/src/exam/mod.rs`, `code/tools/dcgo-harness/src/exam/scenario.rs`
- Modify: `code/tools/dcgo-harness/src/lib.rs`, `code/tools/dcgo-harness/Cargo.toml`
- Test: `code/tools/dcgo-harness/src/exam/scenario.rs` (inline `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: nothing.
- Produces:
  ```rust
  pub struct Scenario {
      pub card: String,
      pub clause: String,
      pub seed: u64,
      pub decks: ScenarioDecks,
      pub steps: Vec<ScenarioStep>,
      pub assertions: Vec<Assertion>,   // YAML key: `assert`
  }
  pub struct ScenarioDecks { pub p0: ScenarioSeat, pub p1: ScenarioSeat }
  pub struct ScenarioSeat { pub stack: Vec<String>, pub rest: String }
  pub struct ScenarioStep { pub actor: u8, pub act: StepAction, pub expect: Option<Expect> }
  pub enum StepAction { Hatch, Pass, Play{card:String, from:String},
                        Digivolve{from:String, using:String},
                        Attack{attacker:String, target:String},
                        Select{targets:Vec<String>} }
  pub struct Expect { pub prompt: Option<String>, pub count: Option<u16>,
                      pub candidates: Vec<String> }
  pub struct Assertion { pub at: u32, pub that: BTreeMap<String, serde_yml::Value> }
  impl Scenario { pub fn from_yaml(text: &str) -> Result<Scenario, String>; }
  ```

- [ ] **Step 1: Add the YAML dependency**

In `code/tools/dcgo-harness/Cargo.toml`, under `[dependencies]`:

```toml
serde_yml = "0.0.12"
```

- [ ] **Step 2: Write the failing test**

Create `code/tools/dcgo-harness/src/exam/scenario.rs` containing only the test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
card: EX12-035
clause: EX12-035#effect#0
seed: 424242
decks:
  p0: { stack: [ST1-02, EX12-035], rest: vb-standard }
  p1: { stack: [], rest: vb-standard }
steps:
  - actor: 0
    do: { hatch: {} }
    expect: { prompt: main_phase }
  - actor: 0
    do: { play: { card: EX12-035, from: hand } }
  - actor: 0
    do: { select: { targets: [opp.field.0] } }
    expect: { prompt: select_permanent, count: 1 }
assert:
  - at: 3
    that: { p0.memory: -2 }
"#;

    #[test]
    fn parses_a_well_formed_scenario() {
        let s = Scenario::from_yaml(GOOD).expect("should parse");
        assert_eq!(s.card, "EX12-035");
        assert_eq!(s.clause, "EX12-035#effect#0");
        assert_eq!(s.seed, 424242);
        assert_eq!(s.steps.len(), 3);
        assert_eq!(s.decks.p0.stack, vec!["ST1-02", "EX12-035"]);
        assert_eq!(s.assertions.len(), 1);
        assert_eq!(s.assertions[0].at, 3);
    }

    #[test]
    fn step_without_expect_is_allowed() {
        let s = Scenario::from_yaml(GOOD).unwrap();
        assert!(s.steps[1].expect.is_none());
    }

    #[test]
    fn expect_carries_prompt_and_count() {
        let s = Scenario::from_yaml(GOOD).unwrap();
        let e = s.steps[2].expect.as_ref().unwrap();
        assert_eq!(e.prompt.as_deref(), Some("select_permanent"));
        assert_eq!(e.count, Some(1));
    }

    #[test]
    fn clause_id_must_start_with_the_card_id() {
        // A clause id that does not belong to this card would key the verdict
        // onto a different card's denominator -- silently covering nothing
        // while reporting a pass.
        let bad = GOOD.replace("clause: EX12-035#effect#0", "clause: BT16-082#effect#0");
        let err = Scenario::from_yaml(&bad).unwrap_err();
        assert!(err.contains("BT16-082"), "got: {err}");
        assert!(err.contains("EX12-035"), "got: {err}");
    }

    #[test]
    fn clause_id_must_have_the_three_part_shape() {
        let bad = GOOD.replace("clause: EX12-035#effect#0", "clause: on_play");
        let err = Scenario::from_yaml(&bad).unwrap_err();
        assert!(err.contains("card_id#zone#index"), "got: {err}");
    }

    #[test]
    fn empty_steps_is_rejected() {
        let bad = GOOD.replace(
            "steps:",
            "steps: []\nunused:",
        );
        assert!(Scenario::from_yaml(&bad).is_err());
    }

    #[test]
    fn unknown_step_verb_is_rejected_loudly() {
        // Silently ignoring an unknown verb would drop a step from the line and
        // desynchronize everything after it.
        let bad = GOOD.replace("do: { hatch: {} }", "do: { teleport: {} }");
        let err = Scenario::from_yaml(&bad).unwrap_err();
        assert!(err.contains("teleport"), "got: {err}");
    }

    #[test]
    fn actor_must_be_0_or_1() {
        let bad = GOOD.replace("- actor: 0\n    do: { hatch: {} }", "- actor: 5\n    do: { hatch: {} }");
        let err = Scenario::from_yaml(&bad).unwrap_err();
        assert!(err.contains("actor"), "got: {err}");
    }

    #[test]
    fn assertion_step_index_must_be_within_the_line() {
        // `at: 99` on a 3-step line can never fire, so the assertion would
        // silently never be checked and the scenario would read as passing.
        let bad = GOOD.replace("at: 3", "at: 99");
        let err = Scenario::from_yaml(&bad).unwrap_err();
        assert!(err.contains("99"), "got: {err}");
    }
}
```

- [ ] **Step 3: Run the test to verify it fails**

```bash
cargo test -p dcgo-harness --lib exam::scenario
```

Expected: FAIL to compile — `cannot find type 'Scenario' in this scope`.

- [ ] **Step 4: Write the implementation**

Prepend to `code/tools/dcgo-harness/src/exam/scenario.rs` (above the test module):

```rust
//! The exam scenario artifact: a legal line from game start over a stacked
//! deck, plus the assertions that survive into CI after the oracle is gone.
//!
//! Deliberately NOT a `DebugRunner`-style staged board. DCGO can only reach a
//! position by legally playing to it, and a hand-built board can miss internal
//! wiring the normal play path sets up -- so a divergence might be the staging's
//! fault rather than a real parity bug. An oracle must never do that.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSeat {
    /// Fixed prefix of the initial draw order. A PREFIX, not the whole deck:
    /// requiring all 50 cards would make every scenario unauthorable.
    #[serde(default)]
    pub stack: Vec<String>,
    /// Named deck the remainder is seeded-shuffled from.
    pub rest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioDecks {
    pub p0: ScenarioSeat,
    pub p1: ScenarioSeat,
}

/// One symbolic action. Lowered to a concrete action ID by `exam::lower`
/// against the engine's live mask, so a scenario survives action-space
/// renumbering and fails loudly on illegal or ambiguous intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepAction {
    Hatch(EmptyArgs),
    Pass(EmptyArgs),
    Play { card: String, #[serde(default = "hand")] from: String },
    Digivolve { from: String, using: String },
    Attack { attacker: String, target: String },
    Select { targets: Vec<String> },
}

fn hand() -> String { "hand".to_string() }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmptyArgs {}

/// The prompt the author expects DCGO to be asking at this step.
///
/// Asserted BEFORE the step is answered. A driver that answers whatever it is
/// asked will, on a single ordering mismatch, desynchronize the entire
/// remainder of the line while every step still looks successful.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Expect {
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub count: Option<u16>,
    #[serde(default)]
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioStep {
    pub actor: u8,
    #[serde(rename = "do")]
    pub act: StepAction,
    #[serde(default)]
    pub expect: Option<Expect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assertion {
    /// Step index this assertion is checked after.
    pub at: u32,
    pub that: BTreeMap<String, serde_yml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub card: String,
    /// A `clause_coverage.models.Clause.id`: `{card_id}#{zone}#{idx}`.
    pub clause: String,
    pub seed: u64,
    pub decks: ScenarioDecks,
    pub steps: Vec<ScenarioStep>,
    #[serde(rename = "assert", default)]
    pub assertions: Vec<Assertion>,
}

impl Scenario {
    pub fn from_yaml(text: &str) -> Result<Scenario, String> {
        let s: Scenario = serde_yml::from_str(text).map_err(|e| e.to_string())?;
        s.validate()?;
        Ok(s)
    }

    fn validate(&self) -> Result<(), String> {
        if self.steps.is_empty() {
            return Err("scenario has no steps: a line nobody drives hangs DCGO \
                        until the timeout, which is indistinguishable from a hung Unity"
                .to_string());
        }

        // The clause id keys this scenario into the clause_coverage
        // denominator. A malformed or foreign id would silently create an
        // invisible sixth verdict class: a scenario that passes while covering
        // nothing.
        let parts: Vec<&str> = self.clause.split('#').collect();
        if parts.len() != 3 {
            return Err(format!(
                "clause '{}' is not a clause_coverage id (expected card_id#zone#index)",
                self.clause
            ));
        }
        if parts[0] != self.card {
            return Err(format!(
                "clause '{}' belongs to card '{}', but this scenario declares card '{}'",
                self.clause, parts[0], self.card
            ));
        }

        for (i, step) in self.steps.iter().enumerate() {
            if step.actor > 1 {
                return Err(format!("step {i}: actor {} is not 0 or 1", step.actor));
            }
        }

        for a in &self.assertions {
            if a.at as usize > self.steps.len() {
                return Err(format!(
                    "assertion at step {} can never fire: the line is {} steps long",
                    a.at,
                    self.steps.len()
                ));
            }
        }

        Ok(())
    }
}
```

Create `code/tools/dcgo-harness/src/exam/mod.rs`:

```rust
//! The card-clause exam: scenario artifact, lowering, sim-side runner, and the
//! differ that compares our engine against the DCGO oracle.

pub mod scenario;

pub use scenario::{Assertion, Expect, Scenario, ScenarioDecks, ScenarioSeat, ScenarioStep, StepAction};
```

Add to `code/tools/dcgo-harness/src/lib.rs`:

```rust
pub mod exam;
```

- [ ] **Step 5: Run the test to verify it passes**

```bash
cargo test -p dcgo-harness --lib exam::scenario
```

Expected: `test result: ok. 9 passed`.

> If `unknown_step_verb_is_rejected_loudly` fails, `serde_yml`'s enum
> representation is accepting the unknown key rather than erroring. Add
> `#[serde(deny_unknown_fields)]` to `Scenario` and `ScenarioStep`, and re-run.
> Do **not** relax the test — a silently-dropped step desynchronizes the line.

- [ ] **Step 6: Commit**

```bash
git add code/tools/dcgo-harness/Cargo.toml code/tools/dcgo-harness/src/lib.rs code/tools/dcgo-harness/src/exam/
git commit -m "exam: scenario schema with clause-id and line validation"
```

---

### Task 2: Lowering — symbolic step to action ID

**Files:**
- Create: `code/tools/dcgo-harness/src/exam/lower.rs`
- Modify: `code/tools/dcgo-harness/src/exam/mod.rs`
- Test: inline `#[cfg(test)] mod tests` in `lower.rs`

**Interfaces:**
- Consumes: `StepAction` (Task 1); `digimon_engine::action::mask::build_action_mask`; `digimon_engine::action::explain::{explain_action, ActionExplanation, ActionKind, ActionZone}`; `digimon_engine::Game`.
- Produces:
  ```rust
  pub fn lower_step(game: &Game, actor: PlayerId, act: &StepAction)
      -> Result<u16, LowerError>;
  pub enum LowerError {
      NoMatch { intent: String, legal: Vec<String> },
      Ambiguous { intent: String, matches: Vec<u16> },
  }
  ```

- [ ] **Step 1: Write the failing test**

Create `code/tools/dcgo-harness/src/exam/lower.rs` with only its test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::exam::scenario::{EmptyArgs, StepAction};

    // Builds a real 2-player game from the tested-card pool, so lowering is
    // exercised against the true mask rather than a mock. Mirrors the fixture
    // construction in code/tools/dcgo-replay/tests/integration.rs.
    fn game() -> digimon_engine::Game {
        let card_data = crate::exam::test_support::load_card_data();
        let deck = crate::exam::test_support::simple_deck();
        digimon_engine::Game::new(&[deck.clone(), deck], &card_data,
                                  digimon_engine::Rules::standard(), Some(42))
            .expect("game should build")
    }

    #[test]
    fn pass_lowers_to_a_legal_action() {
        let g = game();
        let id = lower_step(&g, 0, &StepAction::Pass(EmptyArgs {})).expect("pass is legal");
        let mask = digimon_engine::action::mask::build_action_mask(&g, 0);
        assert_eq!(mask[id as usize], 1.0, "lowered to an action the mask forbids");
    }

    #[test]
    fn unmatchable_intent_lists_what_was_legal() {
        let g = game();
        let err = lower_step(&g, 0, &StepAction::Play {
            card: "ZZ99-999".to_string(), from: "hand".to_string(),
        }).unwrap_err();
        match err {
            LowerError::NoMatch { intent, legal } => {
                assert!(intent.contains("ZZ99-999"));
                // The legal list is the whole point: a bare "no match" makes an
                // author guess, and guessing costs a 40s Unity round trip.
                assert!(!legal.is_empty(), "must report what WAS legal");
            }
            other => panic!("expected NoMatch, got {other:?}"),
        }
    }

    #[test]
    fn every_lowered_id_is_set_in_the_mask() {
        // The invariant the whole design rests on: lowering never emits an
        // action the engine would reject, so a scenario that lowers is
        // guaranteed to be a legal line before Unity is ever launched.
        let g = game();
        for act in [StepAction::Pass(EmptyArgs {}), StepAction::Hatch(EmptyArgs {})] {
            if let Ok(id) = lower_step(&g, 0, &act) {
                let mask = digimon_engine::action::mask::build_action_mask(&g, 0);
                assert_eq!(mask[id as usize], 1.0, "{act:?} lowered to an illegal id");
            }
        }
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test -p dcgo-harness --lib exam::lower
```

Expected: FAIL — `cannot find function 'lower_step'`.

- [ ] **Step 3: Write the implementation**

Prepend to `lower.rs`:

```rust
//! Symbolic step -> action ID, resolved against the engine's LIVE mask.
//!
//! This is the cheap gate. A malformed scenario fails here in milliseconds
//! instead of after sixty seconds of Unity, and the action IDs this produces
//! are written into the DCGO job file -- so both engines consume literally the
//! same integers rather than each interpreting the symbolic form themselves.

use crate::exam::scenario::StepAction;
use digimon_engine::action::explain::{explain_action, ActionExplanation, ActionKind, ActionZone};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::{Game, PlayerId};

#[derive(Debug)]
pub enum LowerError {
    /// No legal action matches the intent. Carries the legal set, because a
    /// bare "no match" makes the author guess and a guess costs a Unity round
    /// trip.
    NoMatch { intent: String, legal: Vec<String> },
    /// More than one legal action matches. Never picked arbitrarily: an
    /// arbitrary pick would silently answer a different question than the one
    /// the scenario asks.
    Ambiguous { intent: String, matches: Vec<u16> },
}

/// Every action ID currently legal for `actor`, with its explanation.
fn legal_explanations(game: &Game, actor: PlayerId) -> Vec<(u16, ActionExplanation)> {
    let mask = build_action_mask(game, actor);
    mask.iter()
        .enumerate()
        .filter(|(_, &v)| v == 1.0)
        .map(|(i, _)| (i as u16, explain_action(game, actor, i as u16)))
        .collect()
}

pub fn lower_step(game: &Game, actor: PlayerId, act: &StepAction) -> Result<u16, LowerError> {
    let legal = legal_explanations(game, actor);
    let intent = format!("{act:?}");

    let matches: Vec<u16> = legal
        .iter()
        .filter(|(_, e)| matches_intent(e, act))
        .map(|(id, _)| *id)
        .collect();

    match matches.len() {
        1 => Ok(matches[0]),
        0 => Err(LowerError::NoMatch {
            intent,
            legal: legal.iter().map(|(id, e)| format!("{id}: {}", e.label)).collect(),
        }),
        _ => Err(LowerError::Ambiguous { intent, matches }),
    }
}

fn matches_intent(e: &ActionExplanation, act: &StepAction) -> bool {
    match act {
        StepAction::Pass(_) => e.kind == ActionKind::Pass,
        StepAction::Hatch(_) => e.kind == ActionKind::Hatch,
        StepAction::Play { card, from } => {
            e.kind == ActionKind::Play
                && e.card_id.as_deref() == Some(card.as_str())
                && zone_name(e.source_zone) == from.as_str()
        }
        StepAction::Digivolve { from, using } => {
            e.kind == ActionKind::Digivolve
                && e.card_id.as_deref() == Some(using.as_str())
                && slot_matches(e.target_zone, e.target_index, from)
        }
        StepAction::Attack { attacker, target } => {
            e.kind == ActionKind::Attack
                && slot_matches(e.source_zone, e.source_index, attacker)
                && target_matches(e, target)
        }
        // Selections resolve against the live PendingSelection rather than the
        // main mask; Task 3 threads them through ScenarioAdapter, which is why
        // they never match here.
        StepAction::Select { .. } => false,
    }
}

fn zone_name(z: Option<ActionZone>) -> &'static str {
    match z {
        Some(ActionZone::Hand) => "hand",
        Some(ActionZone::Battle) => "field",
        Some(ActionZone::Breeding) => "breeding",
        Some(ActionZone::Security) => "security",
        Some(ActionZone::Trash) => "trash",
        Some(ActionZone::Source) => "source",
        Some(ActionZone::Revealed) => "revealed",
        Some(ActionZone::EffectChoice) => "effect_choice",
        None => "",
    }
}

/// Matches a `"field.0"` / `"breeding.0"` style reference.
fn slot_matches(zone: Option<ActionZone>, index: Option<u16>, reference: &str) -> bool {
    let Some((z, i)) = reference.rsplit_once('.') else { return false };
    let Ok(want) = i.parse::<u16>() else { return false };
    zone_name(zone) == z && index == Some(want)
}

fn target_matches(e: &ActionExplanation, target: &str) -> bool {
    if target == "player" {
        // The engine encodes "attack the player" with a sentinel rather than a
        // board slot; an explanation for it carries no target index.
        return e.target_index.is_none();
    }
    slot_matches(e.target_zone, e.target_index, target)
}
```

Add to `mod.rs`:

```rust
pub mod lower;
pub use lower::{lower_step, LowerError};
```

- [ ] **Step 4: Add the shared test support module**

Create `code/tools/dcgo-harness/src/exam/test_support.rs`:

```rust
//! Fixtures shared by the exam module's unit tests. Behind `cfg(test)` so it
//! never ships in the binary.

use digimon_engine::CardData;
use std::collections::HashMap;

/// Loads the real card pool. Uses the same path resolution the rest of the
/// harness does, so a test failure means the pool changed, not that the test
/// invented its own data.
pub fn load_card_data() -> HashMap<String, CardData> {
    let root = std::env::var("DIGIMON_REPO_ROOT")
        .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../..").to_string());
    let path = format!("{root}/data/cards.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
    digimon_engine::card_data::from_json(&text).expect("cards.json should parse")
}

/// A minimal legal deck for lowering tests.
pub fn simple_deck() -> Vec<String> {
    let mut deck = vec!["ST1-02".to_string(); 4];
    deck.extend(vec!["ST1-03".to_string(); 46]);
    deck
}
```

> `digimon_engine::card_data::from_json` is a **placeholder for the real
> loader**. Resolve it with:
> ```bash
> grep -rn "pub fn.*cards.json\|pub fn load_card_data\|pub fn from_json" code/digimon-engine/src/card_data.rs code/tools/dcgo-replay/src/*.rs | head
> ```
> `dcgo-replay` already loads `cards.json` for its own tests — reuse that path
> rather than adding a second loader. Likewise confirm `simple_deck` is a legal
> 50-card deck for `Game::new`, adjusting the card IDs if it is rejected.

Register it in `mod.rs`:

```rust
#[cfg(test)]
pub mod test_support;
```

- [ ] **Step 5: Run the test to verify it passes**

```bash
cargo test -p dcgo-harness --lib exam::lower
```

Expected: `test result: ok. 3 passed`.

- [ ] **Step 6: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/
git commit -m "exam: lower symbolic steps against the live action mask"
```

---

### Task 3: `ScenarioAdapter` — run the line in our engine

**Files:**
- Create: `code/tools/dcgo-harness/src/exam/adapter.rs`
- Modify: `code/tools/dcgo-harness/src/exam/mod.rs`
- Test: inline tests in `adapter.rs`

**Interfaces:**
- Consumes: `Scenario` (Task 1), `lower_step` (Task 2), `digimon_engine::runners::replay::{RecordingSource, StepSpec, ReplaySession, ReplayError, StepPolicy}`, `Game::new_with_ordered_decks`.
- Produces:
  ```rust
  pub struct ScenarioAdapter { /* … */ }
  impl ScenarioAdapter {
      pub fn from_scenario(s: &Scenario, deck_p0: Vec<String>, deck_p1: Vec<String>,
                           card_data: &HashMap<String, CardData>)
          -> Result<ScenarioAdapter, String>;
      pub fn lowered_action_ids(&self) -> &[u16];
  }
  impl RecordingSource for ScenarioAdapter { /* … */ }
  ```

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::exam::scenario::Scenario;

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

    #[test]
    fn adapter_builds_a_game_and_lowers_the_line() {
        let card_data = crate::exam::test_support::load_card_data();
        let deck = crate::exam::test_support::simple_deck();
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
        let card_data = crate::exam::test_support::load_card_data();
        let deck = crate::exam::test_support::simple_deck();
        let s = Scenario::from_yaml(LINE).unwrap();
        let a = ScenarioAdapter::from_scenario(&s, deck.clone(), deck, &card_data).unwrap();
        assert_eq!(a.default_policy(), StepPolicy::Trust);
    }

    #[test]
    fn session_runs_the_line_to_completion() {
        let card_data = crate::exam::test_support::load_card_data();
        let deck = crate::exam::test_support::simple_deck();
        let s = Scenario::from_yaml(LINE).unwrap();
        let a = ScenarioAdapter::from_scenario(&s, deck.clone(), deck, &card_data).unwrap();
        let mut session = ReplaySession::with_source(Box::new(a), &card_data, false)
            .expect("session should build");
        session.run_to_completion();
        assert!(session.is_complete());
        assert!(session.divergences().is_empty(), "{:?}", session.divergences());
    }

    #[test]
    fn an_illegal_line_fails_to_build_not_at_run_time() {
        // The whole point of lowering up front: a malformed scenario must fail
        // in milliseconds, before any Unity launch.
        let bad = LINE.replace("do: { pass: {} }",
                               "do: { play: { card: ZZ99-999, from: hand } }");
        let card_data = crate::exam::test_support::load_card_data();
        let deck = crate::exam::test_support::simple_deck();
        let s = Scenario::from_yaml(&bad).unwrap();
        let err = ScenarioAdapter::from_scenario(&s, deck.clone(), deck, &card_data).unwrap_err();
        assert!(err.contains("ZZ99-999"), "got: {err}");
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test -p dcgo-harness --lib exam::adapter
```

Expected: FAIL — `cannot find type 'ScenarioAdapter'`.

- [ ] **Step 3: Write the implementation**

```rust
//! `RecordingSource` over a hand-authored scenario line.
//!
//! The third implementation of the trait, alongside `NativeAdapter` (engine
//! recordings) and `DcgoAdapter` (the oracle). Being a `RecordingSource` rather
//! than a bespoke runner is the single most load-bearing reuse decision in this
//! work: the divergence machinery, step policy, reset-and-replay backward seek,
//! and player-perspective conversion are all inherited, and a scenario run is
//! structurally the same object as a corpus replay.

use crate::exam::lower::{lower_step, LowerError};
use crate::exam::scenario::Scenario;
use digimon_engine::runners::replay::{RecordingSource, ReplayError, StepPolicy, StepSpec};
use digimon_engine::{CardData, Game, PlayerId, Rules};
use std::collections::HashMap;

pub struct ScenarioAdapter {
    steps: Vec<StepSpec>,
    lowered: Vec<u16>,
    deck_p0: Vec<String>,
    deck_p1: Vec<String>,
    seed: u64,
    first_player: PlayerId,
}

impl ScenarioAdapter {
    pub fn from_scenario(
        s: &Scenario,
        deck_p0: Vec<String>,
        deck_p1: Vec<String>,
        card_data: &HashMap<String, CardData>,
    ) -> Result<ScenarioAdapter, String> {
        let first_player: PlayerId = 0;

        // Lowering must walk the line in a live game, because each step's legal
        // set depends on every step before it. A one-shot pass over a static
        // position could only ever lower step 0.
        let mut game = Game::new_with_ordered_decks(
            &[deck_p0.clone(), deck_p1.clone()],
            card_data,
            Rules::standard(),
            Some(s.seed),
            first_player,
        )
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
                phase: format!("{:?}", game.phase),
                source: "scenario".to_string(),
                memory_after: None,
                dcgo_memory: None,
                turn: Some(game.turn),
                is_game_over: None,
                expected_digest: None,
                selection: None,
                board_p0: None,
                board_p1: None,
            });
            lowered.push(action_id);

            game.apply_action(actor, action_id)
                .map_err(|e| format!("step {i}: lowered action {action_id} rejected: {e}"))?;
        }

        Ok(ScenarioAdapter {
            steps,
            lowered,
            deck_p0,
            deck_p1,
            seed: s.seed,
            first_player,
        })
    }

    pub fn lowered_action_ids(&self) -> &[u16] {
        &self.lowered
    }

    fn build(&self, card_data: &HashMap<String, CardData>) -> Result<Game, ReplayError> {
        Game::new_with_ordered_decks(
            &[self.deck_p0.clone(), self.deck_p1.clone()],
            card_data,
            Rules::standard(),
            Some(self.seed),
            self.first_player,
        )
        .map_err(|e| ReplayError::Setup(e))
    }
}

impl RecordingSource for ScenarioAdapter {
    fn build_initial_game(&self, card_data: &HashMap<String, CardData>) -> Result<Game, ReplayError> {
        self.build(card_data)
    }

    fn relay_initial_state(&self, _game: &mut Game) -> Result<(), ReplayError> {
        // A scenario has no post-mulligan snapshot to re-lay: the deck order is
        // fixed and the game is rebuilt deterministically from (decks, seed,
        // first_player). Reset-and-replay therefore needs nothing here.
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
```

> `game.phase`, `game.turn`, `game.apply_action`, and `ReplayError::Setup` are
> **placeholders for the real API**. Resolve each with:
> ```bash
> grep -rn "pub fn apply_action\|pub phase\|pub turn" code/digimon-engine/src/game/mod.rs | head
> grep -n "pub enum ReplayError" -A 15 code/digimon-engine/src/runners/replay.rs
> ```
> `dcgo-replay/src/replay.rs` already drives the engine this way — copy its call
> shape rather than inventing one.

Add to `mod.rs`:

```rust
pub mod adapter;
pub use adapter::ScenarioAdapter;
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cargo test -p dcgo-harness --lib exam::adapter
```

Expected: `test result: ok. 4 passed`.

- [ ] **Step 5: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/
git commit -m "exam: ScenarioAdapter runs a line as a third RecordingSource"
```

---

### Task 4: The normalized projection

**Files:**
- Create: `code/tools/dcgo-harness/src/exam/projection.rs`
- Modify: `code/tools/dcgo-harness/src/exam/mod.rs`
- Test: inline tests

**Interfaces:**
- Consumes: `digimon_engine::Game`; the DCGO `.state.jsonl` sidecar from the Unity plan's Task 7.
- Produces:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct PermanentProjection { pub card_id: String, pub dp: i64,
                                   pub suspended: bool, pub sources: Vec<String>,
                                   pub keywords: Vec<String> }
  pub struct SeatProjection { pub security: usize, pub hand: Vec<String>,
                              pub trash: Vec<String>, pub field: Vec<PermanentProjection> }
  pub struct StateProjection { pub step: u32, pub turn: u64, pub phase: String,
                               pub memory: i64, pub p0: SeatProjection, pub p1: SeatProjection }
  impl StateProjection {
      pub fn from_game(game: &Game, step: u32) -> StateProjection;
      pub fn from_sidecar_line(line: &str) -> Result<StateProjection, String>;
  }
  pub fn parse_sidecar(text: &str) -> Result<Vec<StateProjection>, String>;
  ```

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const ROW: &str = r#"{"step":3,"turn":2,"phase":"Main","memory":-2,
        "p0":{"security":5,"hand":["ST1-02","ST1-03"],"trash":[],
              "field":[{"card_id":"EX12-035","dp":4000,"suspended":false,
                        "sources":["ST1-02"],"keywords":["Blocker"]}]},
        "p1":{"security":4,"hand":[],"trash":["BT16-082"],"field":[]}}"#;

    #[test]
    fn parses_a_sidecar_row() {
        let p = StateProjection::from_sidecar_line(ROW).unwrap();
        assert_eq!(p.step, 3);
        assert_eq!(p.memory, -2);
        assert_eq!(p.p0.security, 5);
        assert_eq!(p.p0.field.len(), 1);
        assert_eq!(p.p0.field[0].dp, 4000);
        assert_eq!(p.p1.trash, vec!["BT16-082"]);
    }

    #[test]
    fn hand_and_trash_compare_as_multisets_not_sequences() {
        // Zone ORDER is representation -- the two engines order zones
        // differently and an order-sensitive diff would be pure noise.
        let a = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":["A","B"],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#).unwrap();
        let b = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":["B","A"],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#).unwrap();
        assert_eq!(a, b, "hand order must not be a divergence");
    }

    #[test]
    fn duplicate_counts_still_matter() {
        // Multiset, not set: holding two copies is not the same as one.
        let a = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":["A","A"],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#).unwrap();
        let b = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":["A"],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn suspended_is_semantics_and_must_differ() {
        let a = StateProjection::from_sidecar_line(ROW).unwrap();
        let b = StateProjection::from_sidecar_line(&ROW.replace("\"suspended\":false", "\"suspended\":true")).unwrap();
        assert_ne!(a, b, "suspended is semantics, not representation");
    }

    #[test]
    fn parse_sidecar_reads_every_row() {
        let text = format!("{}\n{}\n", ROW.replace("\"step\":3", "\"step\":0"), ROW);
        let rows = parse_sidecar(&text).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].step, 0);
        assert_eq!(rows[1].step, 3);
    }

    #[test]
    fn a_malformed_row_fails_loudly_rather_than_being_skipped() {
        // Silently dropping an unparseable row would shorten one side of the
        // diff and misalign every step after it.
        let text = format!("{ROW}\nnot json\n");
        assert!(parse_sidecar(&text).is_err());
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test -p dcgo-harness --lib exam::projection
```

Expected: FAIL — `cannot find type 'StateProjection'`.

- [ ] **Step 3: Write the implementation**

Write `projection.rs` with `serde` derives matching the sidecar shape, and normalize in the constructors:

- `hand`, `trash`, and `sources` are **sorted** on construction, so `PartialEq` is a multiset compare without a custom impl.
- `field` is sorted by `(card_id, dp, suspended, sources)`.
- `keywords` are sorted.
- `security` stays a **count**; the contents are hidden information, and diffing them would let the differ "confirm" a line whose legality depended on knowing them.
- `dp` is **effective** DP, after modifiers — the printed value would make every buffed Digimon read as a divergence.

`from_game` reads the same fields off `digimon_engine::Game` and applies identical sorting, so the two sides are comparable by construction rather than by a comparison function that could drift from the constructor.

- [ ] **Step 4: Run the test to verify it passes**

```bash
cargo test -p dcgo-harness --lib exam::projection
```

Expected: `test result: ok. 6 passed`.

- [ ] **Step 5: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/
git commit -m "exam: normalized state projection over both engines"
```

---

### Task 5: The differ

**Files:**
- Create: `code/tools/dcgo-harness/src/exam/differ.rs`
- Modify: `code/tools/dcgo-harness/src/exam/mod.rs`
- Test: inline tests

**Interfaces:**
- Consumes: `StateProjection` (Task 4).
- Produces:
  ```rust
  pub struct FieldDiff { pub path: String, pub ours: String, pub dcgo: String }
  pub struct StepDivergence { pub step: u32, pub diffs: Vec<FieldDiff>, pub downstream: bool }
  pub struct DiffReport { pub compared_steps: u32, pub ours_steps: u32, pub dcgo_steps: u32,
                          pub divergences: Vec<StepDivergence> }
  impl DiffReport { pub fn first(&self) -> Option<&StepDivergence>; pub fn is_clean(&self) -> bool; }
  pub fn diff(ours: &[StateProjection], dcgo: &[StateProjection]) -> DiffReport;
  ```

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::exam::projection::StateProjection;

    fn row(step: u32, memory: i64) -> StateProjection {
        StateProjection::from_sidecar_line(&format!(
            r#"{{"step":{step},"turn":1,"phase":"Main","memory":{memory},
               "p0":{{"security":5,"hand":[],"trash":[],"field":[]}},
               "p1":{{"security":5,"hand":[],"trash":[],"field":[]}}}}"#
        )).unwrap()
    }

    #[test]
    fn identical_traces_are_clean() {
        let a = vec![row(0, 0), row(1, 1)];
        let r = diff(&a, &a);
        assert!(r.is_clean());
        assert_eq!(r.compared_steps, 2);
    }

    #[test]
    fn the_first_divergence_leads_and_the_rest_are_downstream() {
        // Once two engines part they are playing different games. A report
        // ranking fifty consequences beside one cause is a report nobody
        // finishes.
        let ours = vec![row(0, 0), row(1, 1), row(2, 2)];
        let dcgo = vec![row(0, 0), row(1, 9), row(2, 9)];
        let r = diff(&ours, &dcgo);
        assert!(!r.is_clean());
        let first = r.first().unwrap();
        assert_eq!(first.step, 1);
        assert!(!first.downstream);
        assert!(r.divergences.iter().skip(1).all(|d| d.downstream),
                "everything after the first divergence must be marked downstream");
    }

    #[test]
    fn a_diff_names_the_field_path_and_both_values() {
        let ours = vec![row(0, 3)];
        let dcgo = vec![row(0, 7)];
        let r = diff(&ours, &dcgo);
        let d = &r.first().unwrap().diffs[0];
        assert_eq!(d.path, "memory");
        assert_eq!(d.ours, "3");
        assert_eq!(d.dcgo, "7");
    }

    #[test]
    fn unequal_lengths_report_the_full_denominator() {
        // "Ran 2 of 5 steps" must never read the same as "ran all 5 clean".
        let ours = vec![row(0, 0), row(1, 0), row(2, 0)];
        let dcgo = vec![row(0, 0)];
        let r = diff(&ours, &dcgo);
        assert_eq!(r.ours_steps, 3);
        assert_eq!(r.dcgo_steps, 1);
        assert_eq!(r.compared_steps, 1);
        assert!(!r.is_clean(), "a truncated comparison is not a clean pass");
    }

    #[test]
    fn empty_dcgo_trace_is_not_a_pass() {
        let ours = vec![row(0, 0)];
        let r = diff(&ours, &[]);
        assert!(!r.is_clean());
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test -p dcgo-harness --lib exam::differ
```

Expected: FAIL — `cannot find function 'diff'`.

- [ ] **Step 3: Write the implementation**

Align by **step index**, not by position — the two traces may skip different rows. Walk ascending; for each step present in both, compare field-by-field and emit a `StepDivergence` with `downstream: false` for the first and `true` for all later ones. `is_clean()` returns true only when there are no divergences **and** `ours_steps == dcgo_steps == compared_steps`.

- [ ] **Step 4: Run the test to verify it passes**

```bash
cargo test -p dcgo-harness --lib exam::differ
```

Expected: `test result: ok. 5 passed`.

- [ ] **Step 5: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/
git commit -m "exam: differ leads with the first divergence, prints the denominator"
```

---

### Task 6: The `exam` subcommand and the golden end-to-end test

**Files:**
- Modify: `code/tools/dcgo-harness/src/main.rs`
- Create: `code/tools/dcgo-harness/tests/exam_golden.rs`
- Create: `qa/dcgo-exams/EX12/EX12-035.yaml`

**Interfaces:**
- Consumes: everything above.
- Produces: `dcgo-harness exam --sim-only <path>` and `dcgo-harness exam --scenario <path> --sidecar <path>`.

- [ ] **Step 1: Write the failing golden test**

Create `code/tools/dcgo-harness/tests/exam_golden.rs` asserting that the committed golden scenario (from the Unity plan's Task 8) lowers, runs sim-only, and diffs clean against its committed `.state.jsonl` sidecar:

```rust
#[test]
fn golden_scenario_lowers_runs_and_diffs_clean() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../..");
    let scenario = std::fs::read_to_string(format!("{root}/qa/dcgo-harness/golden-scripted.yaml"))
        .expect("golden scenario should exist -- produced by the Unity plan's Task 8");
    let sidecar = std::fs::read_to_string(format!("{root}/qa/dcgo-harness/golden/scripted.state.jsonl"))
        .expect("golden sidecar should exist");

    let s = dcgo_harness::exam::Scenario::from_yaml(&scenario).expect("golden should parse");
    // ... build adapter, run session, project, diff ...
    // assert!(report.is_clean(), "{report:#?}");
}
```

> **This test is the CI fixture the spec calls for** — it runs the differ with
> no Unity anywhere, which is what makes "the only thing that needs Unity to
> test is Unity" true.
>
> **It depends on the Unity plan's Task 8 having produced the golden pair.**
> If that has not happened yet, mark this `#[ignore]` with a comment naming the
> dependency rather than fabricating a sidecar — a hand-written sidecar would
> make the differ agree with a file nobody's DCGO ever produced.

- [ ] **Step 2: Add the subcommand**

In `main.rs`'s `Command` enum:

```rust
    /// Run exam scenarios: sim-only (no Unity), or diff against a DCGO sidecar.
    Exam {
        /// Scenario file or directory of scenario files.
        #[arg(long)]
        scenario: std::path::PathBuf,
        /// Run the line in our engine and check `assert:` only. No Unity.
        #[arg(long)]
        sim_only: bool,
        /// DCGO state sidecar to diff against. Required unless --sim-only.
        #[arg(long)]
        sidecar: Option<std::path::PathBuf>,
        /// Path to data/cards.json.
        #[arg(long)]
        cards_json: std::path::PathBuf,
    },
```

The handler must print the **full denominator** on every run — scenarios seen, lowered, run, diffed, and failed — and exit non-zero if any scenario failed to lower. A batch where most scenarios died must never read as a pass.

- [ ] **Step 3: Reconcile the seat, or refuse**

`job.first_player` is **not honored by DCGO**. `ScenarioAdapter` hard-codes `first_player: 0`. If DCGO's own roll gave the other seat, every step's actor is inverted and the line fails its very first prompt assertion.

Handle it explicitly — do not leave it to chance:

- Read the DCGO recording's `my_player_id` from the paired `.jsonl`.
- If it disagrees with the scenario's assumption, either re-run with the seats swapped, or **fail with a message naming the mismatch**.
- Never silently swap the projections to make the diff line up. That would convert a real seat bug into a clean pass.

- [ ] **Step 4: Run the test**

```bash
cargo test -p dcgo-harness --test exam_golden
```

Expected: PASS (or a clearly-reported `ignored` if the Unity golden pair does not exist yet).

- [ ] **Step 5: Run the whole crate's tests**

```bash
cargo test -p dcgo-harness
```

Expected: all suites pass. Record the counts.

- [ ] **Step 6: Commit**

```bash
git add code/tools/dcgo-harness/ qa/dcgo-exams/
git commit -m "exam: CLI subcommand + golden end-to-end test"
```

---

## Self-Review

**Spec coverage.** Implements the spec's Rust half: the scenario artifact and its validation (Task 1), symbolic lowering against the live mask (Task 2), `ScenarioAdapter` as a third `RecordingSource` (Task 3), the normalized projection with the representation-vs-semantics rule (Task 4), the first-divergence differ (Task 5), and the `--sim-only` / oracle CLI split (Task 6). Not covered here: the verdict store, clause binding, assertion backfill, test drafter, CI workflow, MCP, and skill — those are plan 3.

**Dependency on plan 1.** Tasks 4–6 consume the `.state.jsonl` sidecar format that the Unity plan's Task 7 defines, and Task 6's golden test consumes the fixture its Task 8 produces. Tasks 1–3 have **no** Unity dependency and can proceed in parallel with the Unity work.

**Known gap carried forward.** `job.first_player` remains unhonored by DCGO. Task 6 Step 3 makes this an explicit reconcile-or-refuse rather than a silent hazard, but it is a workaround: the real fix is in DCGO's seat roll and is out of scope for both plans.

**Placeholder scan.** Four sites carry explicitly-marked API-resolution points — Task 2's card-data loader and `simple_deck` legality, Task 3's `game.phase` / `game.turn` / `apply_action` / `ReplayError::Setup`. Each names the exact `grep` that resolves it and points at `dcgo-replay/src/replay.rs` as the existing call-shape precedent. These are unavoidable without reading several thousand more lines of engine source; naming the lookup is better than inventing a signature that compiles into the plan but not the codebase. Tasks 4 and 5 describe their implementations in prose rather than full code because both are mechanical given their fully-specified types and tests — the tests are the specification.

**Type consistency.** `Scenario` / `ScenarioStep` / `StepAction` / `Expect` are defined in Task 1 and consumed unchanged in Tasks 2, 3, and 6. `lower_step` / `LowerError` are defined in Task 2 and consumed in Task 3. `StateProjection` is defined in Task 4 and consumed in Tasks 5 and 6. `ScenarioAdapter::lowered_action_ids` (Task 3) is what plan 3's job emitter writes into the DCGO job's `inputs[].action_id`, matching the Unity plan's Task 2 schema.
