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
    Select(SelectPayload),
}

/// The answer a `select:` step gives to the engine's parked selection prompt.
///
/// Exactly one form per step, validated loudly at parse time: a step carrying
/// two forms is answering two different questions, and silently preferring one
/// would desynchronize the line the same way an unknown verb would.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectPayload {
    /// Identity picks, in pick order, resolved in occurrence order against the
    /// prompt's candidate list (documented limitation for duplicate ids).
    Cards(Vec<String>),
    /// Field-permanent picks as OUR slot references (`own.field.N` /
    /// `opp.field.N`), resolved at lowering time against the live game.
    Targets(Vec<String>),
    /// Count / generic-int prompts: the VALUE chosen (a cost, a quantity),
    /// never a branch index.
    Value(i32),
    /// Affirm an optional (yes/no) prompt.
    Yes,
    /// Cancel / decline an optional prompt.
    Decline,
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

/// The raw YAML surface of a `select:` step. All five forms are optional here
/// so the exactly-one rule can be validated with a message that names every
/// form, instead of serde's opaque "unknown field" refusal.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cards: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    targets: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    value: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    yes: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    decline: Option<bool>,
}

/// The five select forms, for the exactly-one error message.
const SELECT_FORMS: &str = "cards: [..] / targets: [..] / value: N / yes: true / decline: true";

impl SelectArgs {
    fn into_payload(self) -> Result<SelectPayload, String> {
        let present = [
            self.cards.is_some(),
            self.targets.is_some(),
            self.value.is_some(),
            self.yes.is_some(),
            self.decline.is_some(),
        ]
        .iter()
        .filter(|p| **p)
        .count();
        if present != 1 {
            return Err(format!(
                "a select step must carry exactly one of {SELECT_FORMS}, got {present}"
            ));
        }
        if let Some(cards) = self.cards {
            if cards.is_empty() {
                return Err("select `cards:` must name at least one card id".to_string());
            }
            return Ok(SelectPayload::Cards(cards));
        }
        if let Some(targets) = self.targets {
            if targets.is_empty() {
                return Err("select `targets:` must name at least one slot".to_string());
            }
            return Ok(SelectPayload::Targets(targets));
        }
        if let Some(value) = self.value {
            return Ok(SelectPayload::Value(value));
        }
        if let Some(yes) = self.yes {
            if !yes {
                return Err(
                    "select `yes:` must be true; to answer no, write `decline: true`".to_string(),
                );
            }
            return Ok(SelectPayload::Yes);
        }
        if let Some(decline) = self.decline {
            if !decline {
                return Err(
                    "select `decline:` must be true; to affirm, write `yes: true`".to_string(),
                );
            }
            return Ok(SelectPayload::Decline);
        }
        unreachable!("exactly-one check above guarantees a form is present")
    }

    fn from_payload(p: &SelectPayload) -> SelectArgs {
        let mut a = SelectArgs::default();
        match p {
            SelectPayload::Cards(c) => a.cards = Some(c.clone()),
            SelectPayload::Targets(t) => a.targets = Some(t.clone()),
            SelectPayload::Value(v) => a.value = Some(*v),
            SelectPayload::Yes => a.yes = Some(true),
            SelectPayload::Decline => a.decline = Some(true),
        }
        a
    }
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
                StepAction::Select(
                    a.into_payload()
                        .map_err(|e| D::Error::custom(format!("step `do: select`: {e}")))?,
                )
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
            StepAction::Select(payload) => {
                map.serialize_entry("select", &SelectArgs::from_payload(payload))?
            }
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

    // ── select: the five symbolic forms ─────────────────────────────────

    /// Parse GOOD with its select step's args replaced by `args`, returning
    /// the parsed payload of that step.
    fn select_payload_of(args: &str) -> Result<SelectPayload, String> {
        let text = GOOD.replace(
            "do: { select: { targets: [opp.field.0] } }",
            &format!("do: {{ select: {args} }}"),
        );
        let s = Scenario::from_yaml(&text)?;
        match &s.steps[2].act {
            StepAction::Select(p) => Ok(p.clone()),
            other => Err(format!("expected a select step, got {other:?}")),
        }
    }

    #[test]
    fn select_targets_form_still_parses() {
        // Backward compat: every already-authored scenario writes `targets:`.
        let s = Scenario::from_yaml(GOOD).unwrap();
        assert_eq!(
            s.steps[2].act,
            StepAction::Select(SelectPayload::Targets(vec!["opp.field.0".to_string()]))
        );
    }

    #[test]
    fn select_cards_form_parses() {
        let p = select_payload_of("{ cards: [EX12-020, EX12-020] }").unwrap();
        assert_eq!(
            p,
            SelectPayload::Cards(vec!["EX12-020".to_string(), "EX12-020".to_string()])
        );
    }

    #[test]
    fn select_value_yes_and_decline_forms_parse() {
        assert_eq!(select_payload_of("{ value: 3 }").unwrap(), SelectPayload::Value(3));
        assert_eq!(select_payload_of("{ value: -1 }").unwrap(), SelectPayload::Value(-1));
        assert_eq!(select_payload_of("{ yes: true }").unwrap(), SelectPayload::Yes);
        assert_eq!(
            select_payload_of("{ decline: true }").unwrap(),
            SelectPayload::Decline
        );
    }

    #[test]
    fn select_with_two_forms_is_rejected_loudly() {
        // Two forms answer two different questions; preferring one silently
        // would desynchronize the line.
        let err = select_payload_of("{ value: 3, yes: true }").unwrap_err();
        assert!(err.contains("exactly one"), "got: {err}");
        assert!(err.contains("decline"), "must list the forms: {err}");
    }

    #[test]
    fn select_with_no_form_is_rejected_loudly() {
        let err = select_payload_of("{}").unwrap_err();
        assert!(err.contains("exactly one"), "got: {err}");
    }

    #[test]
    fn select_empty_lists_are_rejected() {
        assert!(select_payload_of("{ cards: [] }").is_err());
        assert!(select_payload_of("{ targets: [] }").is_err());
    }

    #[test]
    fn select_yes_false_and_decline_false_are_rejected() {
        // `yes: false` is an author trying to decline through the wrong form;
        // silently treating it as either answer would be a coin flip.
        let err = select_payload_of("{ yes: false }").unwrap_err();
        assert!(err.contains("decline: true"), "got: {err}");
        let err = select_payload_of("{ decline: false }").unwrap_err();
        assert!(err.contains("yes: true"), "got: {err}");
    }

    #[test]
    fn select_forms_round_trip_through_yaml() {
        // The drafter serializes StepAction back to YAML; an asymmetric codec
        // would emit scenarios that fail to re-parse.
        for act in [
            StepAction::Select(SelectPayload::Cards(vec!["ST1-03".to_string()])),
            StepAction::Select(SelectPayload::Targets(vec!["own.field.1".to_string()])),
            StepAction::Select(SelectPayload::Value(3)),
            StepAction::Select(SelectPayload::Yes),
            StepAction::Select(SelectPayload::Decline),
        ] {
            let yaml = serde_yml::to_string(&act).expect("serializes");
            let back: StepAction = serde_yml::from_str(&yaml).expect("re-parses");
            assert_eq!(back, act, "round trip of {yaml}");
        }
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
