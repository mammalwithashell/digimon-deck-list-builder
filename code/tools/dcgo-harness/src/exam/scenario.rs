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
///
/// **Serde is hand-written, not derived.** `serde_yml` 0.0.12 encodes an
/// externally-tagged enum as a YAML *tag* (`do: !hatch {}`), but the scenario
/// format authors verbs as an ordinary single-key mapping
/// (`do: { hatch: {} }`) -- which is what a human writes and what the drafter
/// emits. The derived impl rejects that with "expected a YAML tag starting
/// with '!'", so the codec below reads and writes the mapping form directly.
/// It also lets an unknown verb fail with a message that names the verb, which
/// matters: silently dropping a step would desynchronize the whole line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepAction {
    Hatch(EmptyArgs),
    Pass(EmptyArgs),
    /// Move the breeding-area Digimon to the battle area (the second breeding
    /// action besides `hatch`). `from` defaults to `breeding` — the only zone
    /// a move can come from — but accepts an explicit `breeding` / `breeding.0`
    /// so authors who pin slots everywhere else can here too.
    Move { from: String },
    Play { card: String, from: String },
    Digivolve { from: String, using: String },
    Attack { attacker: String, target: String },
    Select { targets: Vec<String> },
}

/// Every verb this format understands, in the order shown to an author whose
/// spelling was wrong.
const STEP_VERBS: &[&str] = &["hatch", "pass", "move", "play", "digivolve", "attack", "select"];

fn hand() -> String {
    "hand".to_string()
}

fn breeding() -> String {
    "breeding".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MoveArgs {
    #[serde(default = "breeding")]
    from: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlayArgs {
    card: String,
    #[serde(default = "hand")]
    from: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DigivolveArgs {
    from: String,
    using: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AttackArgs {
    attacker: String,
    target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectArgs {
    targets: Vec<String>,
}

impl<'de> Deserialize<'de> for StepAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;

        let value = serde_yml::Value::deserialize(deserializer)?;
        let mapping = value.as_mapping().ok_or_else(|| {
            D::Error::custom(
                "step `do` must be a single-key mapping, e.g. `do: { play: { card: ST1-02 } }`",
            )
        })?;
        if mapping.len() != 1 {
            return Err(D::Error::custom(format!(
                "step `do` must name exactly one verb, got {} keys",
                mapping.len()
            )));
        }
        let (verb_key, args) = mapping.iter().next().expect("len checked above");
        let verb = verb_key
            .as_str()
            .ok_or_else(|| D::Error::custom("step `do` verb must be a string"))?;

        fn args_of<T, E>(verb: &str, args: &serde_yml::Value) -> Result<T, E>
        where
            T: serde::de::DeserializeOwned,
            E: serde::de::Error,
        {
            serde_yml::from_value(args.clone())
                .map_err(|e| E::custom(format!("step `do: {verb}`: {e}")))
        }

        Ok(match verb {
            "hatch" => StepAction::Hatch(args_of::<EmptyArgs, D::Error>(verb, args)?),
            "pass" => StepAction::Pass(args_of::<EmptyArgs, D::Error>(verb, args)?),
            "move" => {
                let a: MoveArgs = args_of::<MoveArgs, D::Error>(verb, args)?;
                StepAction::Move { from: a.from }
            }
            "play" => {
                let a: PlayArgs = args_of::<PlayArgs, D::Error>(verb, args)?;
                StepAction::Play {
                    card: a.card,
                    from: a.from,
                }
            }
            "digivolve" => {
                let a: DigivolveArgs = args_of::<DigivolveArgs, D::Error>(verb, args)?;
                StepAction::Digivolve {
                    from: a.from,
                    using: a.using,
                }
            }
            "attack" => {
                let a: AttackArgs = args_of::<AttackArgs, D::Error>(verb, args)?;
                StepAction::Attack {
                    attacker: a.attacker,
                    target: a.target,
                }
            }
            "select" => {
                let a: SelectArgs = args_of::<SelectArgs, D::Error>(verb, args)?;
                StepAction::Select { targets: a.targets }
            }
            other => {
                return Err(D::Error::custom(format!(
                    "unknown step verb `{other}`: expected one of {}",
                    STEP_VERBS.join(", ")
                )))
            }
        })
    }
}

impl Serialize for StepAction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            StepAction::Hatch(a) => map.serialize_entry("hatch", a)?,
            StepAction::Pass(a) => map.serialize_entry("pass", a)?,
            StepAction::Move { from } => {
                map.serialize_entry("move", &MoveArgs { from: from.clone() })?
            }
            StepAction::Play { card, from } => map.serialize_entry(
                "play",
                &PlayArgs {
                    card: card.clone(),
                    from: from.clone(),
                },
            )?,
            StepAction::Digivolve { from, using } => map.serialize_entry(
                "digivolve",
                &DigivolveArgs {
                    from: from.clone(),
                    using: using.clone(),
                },
            )?,
            StepAction::Attack { attacker, target } => map.serialize_entry(
                "attack",
                &AttackArgs {
                    attacker: attacker.clone(),
                    target: target.clone(),
                },
            )?,
            StepAction::Select { targets } => map.serialize_entry(
                "select",
                &SelectArgs {
                    targets: targets.clone(),
                },
            )?,
        }
        map.end()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    fn move_verb_parses_with_a_pinned_breeding_slot() {
        let s = Scenario::from_yaml(&GOOD.replace(
            "do: { hatch: {} }",
            "do: { move: { from: breeding.0 } }",
        ))
        .expect("move step should parse");
        assert_eq!(
            s.steps[0].act,
            StepAction::Move {
                from: "breeding.0".to_string()
            }
        );
    }

    #[test]
    fn move_verb_defaults_from_to_breeding() {
        // The breeding area is the ONLY zone a move can come from, so an
        // author should be allowed to omit it -- like `play`'s `from: hand`.
        let s = Scenario::from_yaml(&GOOD.replace("do: { hatch: {} }", "do: { move: {} }"))
            .expect("bare move step should parse");
        assert_eq!(
            s.steps[0].act,
            StepAction::Move {
                from: "breeding".to_string()
            }
        );
    }

    #[test]
    fn move_verb_round_trips_through_yaml() {
        // The drafter serializes StepAction back to YAML; an asymmetric codec
        // would emit scenarios that fail to re-parse.
        let s = Scenario::from_yaml(&GOOD.replace(
            "do: { hatch: {} }",
            "do: { move: { from: breeding.0 } }",
        ))
        .unwrap();
        let yaml = serde_yml::to_string(&s.steps[0].act).expect("serializes");
        let back: StepAction = serde_yml::from_str(&yaml).expect("re-parses");
        assert_eq!(back, s.steps[0].act);
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
