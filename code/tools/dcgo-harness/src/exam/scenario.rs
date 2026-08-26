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
    /// Activate a `[Main]` effect on a permanent ALREADY in play — the
    /// field-activation surface, distinct from `play`, which puts a `[Main]`
    /// Option onto the field from hand.
    ///
    /// `on:` is a slot reference in the same grammar `attack:` uses
    /// (`field.0`, or the bare `breeding` sentinel for the breeding area's
    /// `<Training>` [Main]). This is also the surface for `<Delay>`: our
    /// engine gates a placed Delay Option's activation behind
    /// `turn_count > placed_on_turn` and offers it on the SAME action bit, and
    /// DCGO's `CanDeclareOptionDelayEffect` gates its own on the same
    /// not-the-placing-turn rule.
    Main { on: String },
    Select(SelectPayload),
    /// A selection DCGO asks that OUR engine never parks -- authored
    /// `select: { ..., dcgo_only: true }`.
    ///
    /// DCGO batches same-timing triggers into one `MultipleSkills` prompt; where
    /// our engine models one of those triggers as a combat-state window rather
    /// than a queued trigger, it opens only the other pick and the line is one
    /// DCGO answer short. Without this the scenario is unauthorable: the run
    /// aborts on a prompt mismatch no board arrangement can dodge. EX12-076
    /// Susanoomon's `<Raid>` clause is the motivating case -- it REQUIRES Raid
    /// activatable, which is exactly the condition that stacks it beside the
    /// card's own `[When Attacking]` trigger.
    ///
    /// The row still answers by card IDENTITY (with `ordinal:` where the
    /// stacked candidates share an id), so DCGO resolves it against its own
    /// candidate list like any other select. This widens the vocabulary; it
    /// does not weaken the comparison.
    SelectDcgoOnly(SelectPayload),
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
    ///
    /// `ordinal` disambiguates the one prompt where duplicate ids are NOT
    /// interchangeable copies: DCGO's `MultipleSkills` (our `TriggerOrder`),
    /// whose candidates are stacked TRIGGERS. A deleted carrier with an
    /// `[On Deletion]` and an `<Ascension>` offers its own identity twice, and
    /// those are different decisions — so occurrence order must not silently
    /// pick between them. It is the 0-based position AMONG the candidates
    /// carrying that same id, NOT an index into the whole candidate list.
    ///
    /// `trigger` is the SEMANTIC form of the same disambiguation, and is
    /// preferred wherever it applies: it names the branch's KEYWORD
    /// (`<Fortitude>`, `<Ascension>`, an aura-granted `<Retaliation>`) rather
    /// than its POSITION. Position is per-engine by construction -- both engines
    /// resolve the same authored step against their OWN candidate list -- so an
    /// `ordinal:` can silently mean different triggers on the two sides, while a
    /// keyword means the same thing in both. Mutually exclusive with `ordinal`.
    Cards {
        ids: Vec<String>,
        ordinal: Option<i32>,
        trigger: Option<String>,
        /// Names the wanted branch by what it is NOT -- the complement of
        /// `trigger`, for the branch that carries no keyword of its own.
        ///
        /// EX12-047 Amaterasumon is why this exists. Its deletion stack is
        /// [`<Ascension>`, the printed `[On Deletion]`], and only the first has
        /// a keyword. Nothing else separates them: same source card, same
        /// timing (both register under `OnDestroyedAnyone`), same optionality.
        /// `trigger_not: Ascension` says "this card's OTHER branch", which both
        /// engines can resolve against their own list without either needing a
        /// registry of what counts as a keyword.
        trigger_not: Option<String>,
    },
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
    /// A MULTI-PICK material declaration -- `[Assembly]` / `[DigiXros]`.
    ///
    /// The one place the two engines disagree on CARDINALITY rather than on
    /// prompt kind. Our engine walks the recipe one element at a time
    /// (`install_assembly_element` re-installs per element), so N successive
    /// `Material` prompts; DCGO declares the whole set in ONE row
    /// (`SelectAssemblyClass` / `SelectDigiXrosClass`, recorded as a single
    /// `selection` with every id and `mechanic: "assembly"`). So one authored
    /// step answers N sim prompts and emits ONE wire row -- the mirror of
    /// `optional_gate_fold`, which is one sim prompt over two wire rows.
    ///
    /// Ids are given in RECIPE-ELEMENT ORDER, which is the order our engine
    /// asks in. Without this form a card whose only affordable line is its
    /// Assembly path is unplayable in an exam at all -- EX12-076 Susanoomon
    /// costs 16 against a +10 memory ceiling and only `[Assembly -9]` brings it
    /// to 7, so every clause needing it PLAYED was unreachable.
    Materials(Vec<String>),
}

/// Every verb this format understands, in the order shown to an author whose
/// spelling was wrong.
const STEP_VERBS: &[&str] =
    &["hatch", "pass", "move", "play", "digivolve", "attack", "main", "select"];

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

/// `do: { main: { on: field.0 } }`.
///
/// `on:` is required and carries no default. `play`'s `from: hand` and
/// `move`'s `from: breeding` each have exactly one possible source, so
/// defaulting them is unambiguous; a board can hold many permanents, so a
/// defaulted `on:` would silently mean "whichever one lowered first".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MainArgs {
    on: String,
}

/// The raw YAML surface of a `select:` step. All five forms are optional here
/// so the exactly-one rule can be validated with a message that names every
/// form, instead of serde's opaque "unknown field" refusal.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cards: Option<Vec<String>>,
    /// Only legal alongside `cards:` — see [`SelectPayload::Cards`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ordinal: Option<i32>,
    /// Only legal alongside `cards:`, and never alongside `ordinal:` — see
    /// [`SelectPayload::Cards`]. Names the branch's KEYWORD; matching is
    /// case-insensitive and tolerant of the printed angle brackets, so
    /// `fortitude`, `Fortitude` and `"<Fortitude>"` are one answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    trigger: Option<String>,
    /// Only legal alongside `cards:`, and never alongside `trigger:` or
    /// `ordinal:` -- see [`SelectPayload::Cards`]. Names the branch to EXCLUDE,
    /// leaving exactly one survivor; normalized identically to `trigger:`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    trigger_not: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    targets: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    value: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    yes: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    decline: Option<bool>,
    /// Marks the step as a DCGO-ONLY row (see `StepAction::SelectDcgoOnly`).
    /// A MODIFIER like `ordinal:`, not one of the answer forms, so it does not
    /// count toward the exactly-one rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dcgo_only: Option<bool>,
    /// Multi-pick material declaration -- see [`SelectPayload::Materials`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    materials: Option<Vec<String>>,
}

/// The five select forms, for the exactly-one error message.
const SELECT_FORMS: &str =
    "cards: [..] / materials: [..] / targets: [..] / value: N / yes: true / decline: true";

/// Rendered in the error an author gets for `ordinal:` without `cards:`.
const ORDINAL_RULE: &str = "select `ordinal:` is only legal alongside `cards:`: it is the 0-based \
position among the candidates carrying THAT card id, so without an id it \
addresses nothing. To answer a prompt by raw DCGO index, use `value: N`";

/// Rendered in the error an author gets for `trigger:` without `cards:`.
const TRIGGER_RULE: &str = "select `trigger:` is only legal alongside `cards:`: it names WHICH \
of that card's stacked triggers to resolve (`<Fortitude>`, `<Ascension>`, an aura-granted \
`<Retaliation>`), so without an id it addresses nothing. To answer a prompt by raw DCGO index, \
use `value: N`";

/// Rendered when a step carries BOTH disambiguators.
///
/// Not a style preference: `ordinal:` is a POSITION in the prompt's candidate
/// list, and each engine builds that list itself, so one ordinal can name a
/// different trigger on the two sides. A keyword names the same trigger in both.
const TRIGGER_VS_ORDINAL_RULE: &str = "select `trigger:` and `ordinal:` both disambiguate one \
card's stacked triggers, so a step carrying both gives two answers to one question. Prefer \
`trigger:`: `ordinal:` is a POSITION in the prompt's candidate list and each engine builds that \
list itself, so the same ordinal can name a DIFFERENT trigger on the two sides; a keyword names \
the same trigger in both";

/// Rendered when `trigger_not:` is written without `cards:`.
const TRIGGER_NOT_RULE: &str = "select `trigger_not:` is only legal alongside `cards:`: it \
names WHICH of that card's stacked triggers to EXCLUDE, leaving exactly one survivor";

/// Rendered when both naming forms are written on one step.
const TRIGGER_NOT_VS_TRIGGER_RULE: &str = "select `trigger:` and `trigger_not:` are two ways \
to name one branch -- positively and by exclusion -- so a step carrying both gives two answers \
to one question. Keep whichever the wanted branch actually supports: `trigger:` when it has a \
keyword of its own, `trigger_not:` when it is the keyword-LESS branch and can only be named by \
what it is not";

/// Rendered when `trigger_not:` is paired with the positional disambiguator.
const TRIGGER_NOT_VS_ORDINAL_RULE: &str = "select `trigger_not:` and `ordinal:` both \
disambiguate one card's stacked triggers. Prefer `trigger_not:`: `ordinal:` is a POSITION in \
the prompt's candidate list and each engine builds that list itself, so the same ordinal can \
name a DIFFERENT trigger on the two sides";

/// Canonical spelling of a trigger name, for cross-engine comparison.
///
/// Lowercases and drops `<`, `>` and whitespace, so every way one keyword can be
/// written collapses onto a single string:
///   * the author's `trigger: fortitude` / `Fortitude` / `"<Fortitude>"`;
///   * our sim side, which spells a branch's keyword from the `Keyword` variant
///     (`adapter::keyword_display_name` -> `ArmorPurge`) -- the same word
///     `keyword_to_auto_effect` prints as `Effect::name = "<Armor Purge>"`;
///   * DCGO's `ICardEffect.EffectName`, which names keywords WITHOUT brackets
///     and WITH spaces (`SetUpICardEffect("Armor Purge", ...)`).
///
/// Dropping whitespace is what makes those agree: `ArmorPurge`, `<Armor Purge>`
/// and `Armor Purge` all land on `armorpurge`.
pub fn normalize_trigger_name(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_whitespace() && *c != '<' && *c != '>')
        .flat_map(|c| c.to_lowercase())
        .collect()
}

impl SelectArgs {
    fn into_payload(self) -> Result<SelectPayload, String> {
        let present = [
            self.cards.is_some(),
            self.materials.is_some(),
            self.targets.is_some(),
            self.value.is_some(),
            self.yes.is_some(),
            self.decline.is_some(),
        ]
        .iter()
        .filter(|p| **p)
        .count();
        // Checked BEFORE the exactly-one rule: a bare `ordinal:` carries no
        // form at all, so the generic "got 0 forms" message would never
        // mention the key the author actually wrote.
        if self.ordinal.is_some() && self.cards.is_none() {
            return Err(ORDINAL_RULE.to_string());
        }
        // Same placement, same reason: a bare `trigger:` carries no answer
        // form, so the generic "got 0 forms" message below would never mention
        // the key the author actually wrote.
        if self.trigger.is_some() && self.cards.is_none() {
            return Err(TRIGGER_RULE.to_string());
        }
        // Also before the exactly-one rule, so an author who wrote both
        // disambiguators is told WHICH to keep instead of being told they
        // wrote zero answer forms.
        if self.trigger.is_some() && self.ordinal.is_some() {
            return Err(TRIGGER_VS_ORDINAL_RULE.to_string());
        }
        // Same placement and the same reasoning as the three rules above: name
        // the key the author actually wrote before falling back to the generic
        // "got 0 answer forms" message.
        if self.trigger_not.is_some() && self.cards.is_none() {
            return Err(TRIGGER_NOT_RULE.to_string());
        }
        if self.trigger_not.is_some() && self.trigger.is_some() {
            return Err(TRIGGER_NOT_VS_TRIGGER_RULE.to_string());
        }
        if self.trigger_not.is_some() && self.ordinal.is_some() {
            return Err(TRIGGER_NOT_VS_ORDINAL_RULE.to_string());
        }
        if present != 1 {
            return Err(format!(
                "a select step must carry exactly one of {SELECT_FORMS}, got {present}"
            ));
        }
        if let Some(ordinal) = self.ordinal {
            if ordinal < 0 {
                return Err(format!(
                    "select `ordinal: {ordinal}` is negative; it is a 0-based position \
                     among that card's own candidates. To DECLINE the prompt, write \
                     `decline: true`"
                ));
            }
        }
        if let Some(materials) = self.materials {
            if materials.is_empty() {
                return Err(
                    "select `materials:` is empty: a material declaration with no cards                      declares nothing"
                        .to_string(),
                );
            }
            return Ok(SelectPayload::Materials(materials));
        }
        if let Some(cards) = self.cards {
            if cards.is_empty() {
                return Err("select `cards:` must name at least one card id".to_string());
            }
            if self.ordinal.is_some() && cards.len() != 1 {
                return Err(format!(
                    "select `ordinal:` names WHICH of one card's own candidates to take, \
                     so it cannot accompany a {}-card pick list -- the prompts that \
                     accept an ordinal (DCGO MultipleSkills / our TriggerOrder) are \
                     single-pick",
                    cards.len()
                ));
            }
            if let Some(trigger) = self.trigger.as_deref() {
                if normalize_trigger_name(trigger).is_empty() {
                    return Err(format!(
                        "select `trigger: {trigger:?}` names no keyword once the angle \
                         brackets and spacing are stripped. Write the keyword itself, \
                         e.g. `trigger: Fortitude`"
                    ));
                }
                if cards.len() != 1 {
                    return Err(format!(
                        "select `trigger:` names WHICH of one card's own stacked triggers \
                         to take, so it cannot accompany a {}-card pick list -- the \
                         prompts that stack triggers (DCGO MultipleSkills / our \
                         TriggerOrder) are single-pick",
                        cards.len()
                    ));
                }
            }
            if let Some(trigger_not) = self.trigger_not.as_deref() {
                if normalize_trigger_name(trigger_not).is_empty() {
                    return Err(format!(
                        "select `trigger_not: {trigger_not:?}` names no keyword once the \
                         angle brackets and spacing are stripped. Write the keyword to \
                         exclude, e.g. `trigger_not: Ascension`"
                    ));
                }
                if cards.len() != 1 {
                    return Err(format!(
                        "select `trigger_not:` names WHICH of one card's own stacked \
                         triggers to exclude, so it cannot accompany a {}-card pick list",
                        cards.len()
                    ));
                }
            }
            return Ok(SelectPayload::Cards {
                ids: cards,
                ordinal: self.ordinal,
                trigger: self.trigger,
                trigger_not: self.trigger_not,
            });
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
            SelectPayload::Cards {
                ids,
                ordinal,
                trigger,
                trigger_not,
            } => {
                a.cards = Some(ids.clone());
                a.ordinal = *ordinal;
                a.trigger = trigger.clone();
                a.trigger_not = trigger_not.clone();
            }
            SelectPayload::Materials(m) => a.materials = Some(m.clone()),
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
            "main" => {
                let a: MainArgs = args_of::<MainArgs, D::Error>(verb, args)?;
                StepAction::Main { on: a.on }
            }
            "select" => {
                let a: SelectArgs = args_of::<SelectArgs, D::Error>(verb, args)?;
                let dcgo_only = a.dcgo_only.unwrap_or(false);
                let payload = a
                    .into_payload()
                    .map_err(|e| D::Error::custom(format!("step `do: select`: {e}")))?;
                if dcgo_only {
                    StepAction::SelectDcgoOnly(payload)
                } else {
                    StepAction::Select(payload)
                }
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
            StepAction::Main { on } => {
                map.serialize_entry("main", &MainArgs { on: on.clone() })?
            }
            StepAction::Select(payload) => {
                map.serialize_entry("select", &SelectArgs::from_payload(payload))?
            }
            StepAction::SelectDcgoOnly(payload) => {
                let mut a = SelectArgs::from_payload(payload);
                a.dcgo_only = Some(true);
                map.serialize_entry("select", &a)?
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

    // ── main: field [Main] / <Delay> activation ─────────────────────────

    #[test]
    fn main_verb_parses_with_a_pinned_field_slot() {
        let s = Scenario::from_yaml(&GOOD.replace(
            "do: { hatch: {} }",
            "do: { main: { on: field.0 } }",
        ))
        .expect("main step should parse");
        assert_eq!(
            s.steps[0].act,
            StepAction::Main {
                on: "field.0".to_string()
            }
        );
    }

    #[test]
    fn main_verb_accepts_the_bare_breeding_sentinel() {
        // The breeding area's `<Training>` [Main] is encoded with the
        // BREEDING_TARGET sentinel — a place, not a slot — so the bare form
        // has to be expressible.
        let s = Scenario::from_yaml(&GOOD.replace(
            "do: { hatch: {} }",
            "do: { main: { on: breeding } }",
        ))
        .unwrap();
        assert_eq!(
            s.steps[0].act,
            StepAction::Main {
                on: "breeding".to_string()
            }
        );
    }

    #[test]
    fn main_verb_requires_on() {
        // A board can hold many permanents, so a defaulted `on:` would
        // silently mean "whichever one lowered first".
        let err = Scenario::from_yaml(&GOOD.replace(
            "do: { hatch: {} }",
            "do: { main: {} }",
        ))
        .unwrap_err();
        assert!(err.contains("main"), "got: {err}");
        assert!(err.contains("on"), "must name the missing field: {err}");
    }

    #[test]
    fn main_verb_rejects_an_unknown_argument() {
        let err = Scenario::from_yaml(&GOOD.replace(
            "do: { hatch: {} }",
            "do: { main: { on: field.0, effect: Digiburst } }",
        ))
        .unwrap_err();
        assert!(err.contains("effect"), "got: {err}");
    }

    #[test]
    fn main_verb_round_trips_through_yaml() {
        // The codec is hand-written, so an asymmetric arm would emit
        // drafter/backfill output that fails to re-parse.
        let act = StepAction::Main {
            on: "field.2".to_string(),
        };
        let yaml = serde_yml::to_string(&act).expect("serializes");
        let back: StepAction = serde_yml::from_str(&yaml).expect("re-parses");
        assert_eq!(back, act, "round trip of {yaml}");
    }

    #[test]
    fn main_is_listed_among_the_verbs_an_author_is_offered() {
        let bad = GOOD.replace("do: { hatch: {} }", "do: { teleport: {} }");
        let err = Scenario::from_yaml(&bad).unwrap_err();
        assert!(err.contains("main"), "the verb list must name `main`: {err}");
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
            SelectPayload::Cards {
                ids: vec!["EX12-020".to_string(), "EX12-020".to_string()],
                ordinal: None,
                trigger: None,
                trigger_not: None,
            }
        );
    }

    // ── select `ordinal:` (the MultipleSkills trigger disambiguator) ────

    #[test]
    fn select_ordinal_parses_alongside_cards() {
        let p = select_payload_of("{ cards: [EX12-047], ordinal: 1 }").unwrap();
        assert_eq!(
            p,
            SelectPayload::Cards {
                ids: vec!["EX12-047".to_string()],
                ordinal: Some(1),
                trigger: None,
                trigger_not: None,
            }
        );
    }

    #[test]
    fn select_ordinal_without_cards_is_rejected_loudly() {
        // An ordinal is a position among the candidates carrying a given id;
        // without an id it addresses nothing, and quietly treating it as a raw
        // index is exactly the value-space confusion the field exists to end.
        let err = select_payload_of("{ ordinal: 1 }").unwrap_err();
        assert!(err.contains("only legal alongside `cards:`"), "got: {err}");
        assert!(err.contains("value: N"), "must name the alternative: {err}");
    }

    #[test]
    fn select_ordinal_alongside_value_is_rejected() {
        // `value:` is the raw DCGO-index fallback; combining an index and an
        // identity in one answer is the abort DCGO's own hook raises.
        let err = select_payload_of("{ value: 2, ordinal: 1 }").unwrap_err();
        assert!(err.contains("cards:"), "got: {err}");
    }

    #[test]
    fn select_ordinal_with_a_multi_card_pick_is_rejected() {
        let err = select_payload_of("{ cards: [A, B], ordinal: 0 }").unwrap_err();
        assert!(err.contains("single-pick"), "got: {err}");
    }

    #[test]
    fn a_negative_select_ordinal_is_rejected() {
        // -1 is DCGO's "decline the stack" sentinel on a DIFFERENT field;
        // accepting it here would let a scenario decline a trigger stack while
        // reading as a pick.
        let err = select_payload_of("{ cards: [A], ordinal: -1 }").unwrap_err();
        assert!(err.contains("decline: true"), "got: {err}");
    }

    #[test]
    fn select_ordinal_round_trips_through_yaml() {
        let act = StepAction::Select(SelectPayload::Cards {
            ids: vec!["EX12-047".to_string()],
            ordinal: Some(1),
            trigger: None,
            trigger_not: None,
        });
        let yaml = serde_yml::to_string(&act).expect("serializes");
        assert!(yaml.contains("ordinal"), "the key must survive: {yaml}");
        let back: StepAction = serde_yml::from_str(&yaml).expect("re-parses");
        assert_eq!(back, act);
    }

    #[test]
    fn an_absent_ordinal_is_omitted_from_the_yaml_not_written_as_null() {
        let act = StepAction::Select(SelectPayload::Cards {
            ids: vec!["EX12-047".to_string()],
            ordinal: None,
            trigger: None,
            trigger_not: None,
        });
        let yaml = serde_yml::to_string(&act).expect("serializes");
        assert!(!yaml.contains("ordinal"), "got: {yaml}");
    }

    // ── select `trigger:` (the SEMANTIC MultipleSkills disambiguator) ──

    #[test]
    fn select_trigger_parses_alongside_cards() {
        let p = select_payload_of("{ cards: [EX12-065], trigger: Fortitude }").unwrap();
        assert_eq!(
            p,
            SelectPayload::Cards {
                ids: vec!["EX12-065".to_string()],
                ordinal: None,
                trigger: Some("Fortitude".to_string()),
                trigger_not: None,
            }
        );
    }

    #[test]
    fn select_trigger_without_cards_is_rejected_loudly() {
        // A trigger names WHICH of a card's stacked triggers to resolve; with
        // no card id it addresses nothing, exactly as with `ordinal:`.
        let err = select_payload_of("{ trigger: Fortitude }").unwrap_err();
        assert!(err.contains("only legal alongside `cards:`"), "got: {err}");
        assert!(err.contains("value: N"), "must name the alternative: {err}");
    }

    // -- select `trigger_not:` (the keyword-LESS branch) ------------------

    /// The complement form parses beside `cards:`, exactly as `trigger:` does.
    #[test]
    fn select_trigger_not_parses_alongside_cards() {
        let p = select_payload_of("{ cards: [EX12-047], trigger_not: Ascension }").unwrap();
        assert_eq!(
            p,
            SelectPayload::Cards {
                ids: vec!["EX12-047".to_string()],
                ordinal: None,
                trigger: None,
                trigger_not: Some("Ascension".to_string()),
            }
        );
    }

    /// `trigger_not:` names WHICH branch to drop, so without `cards:` it has
    /// nothing to drop it from.
    #[test]
    fn select_trigger_not_without_cards_is_rejected_loudly() {
        let err = select_payload_of("{ trigger_not: Ascension }").unwrap_err();
        assert!(
            err.contains("trigger_not"),
            "the message must name the key the author wrote: {err}"
        );
    }

    /// The two naming forms answer one question two ways, so a step carrying
    /// both is refused rather than silently preferring one.
    #[test]
    fn select_trigger_and_trigger_not_together_are_rejected() {
        let err = select_payload_of("{ cards: [EX12-047], trigger: Ascension, trigger_not: Fortitude }").unwrap_err();
        assert!(
            err.contains("trigger:") && err.contains("trigger_not:"),
            "the message must name BOTH keys so the author knows which to drop: {err}"
        );
    }

    /// Pairing exclusion with the POSITIONAL disambiguator is the same mistake
    /// as pairing the positive form with it.
    #[test]
    fn select_trigger_not_and_ordinal_together_are_rejected() {
        let err = select_payload_of("{ cards: [EX12-047], trigger_not: Ascension, ordinal: 0 }").unwrap_err();
        assert!(
            err.contains("trigger_not") && err.contains("ordinal"),
            "the message must name both: {err}"
        );
    }

    /// A `trigger_not:` that normalizes away entirely would exclude nothing and
    /// silently match the first branch -- refuse instead.
    #[test]
    fn an_empty_trigger_not_is_rejected_rather_than_excluding_nothing() {
        let err = select_payload_of("{ cards: [EX12-047], trigger_not: \"<>\" }").unwrap_err();
        assert!(
            err.contains("names no keyword"),
            "an all-punctuation exclusion must be refused: {err}"
        );
    }

    /// Same normalization as `trigger:` -- the angle brackets and case are
    /// noise, so all three spellings are one answer.
    #[test]
    fn trigger_not_spellings_normalize_to_one_answer() {
        for spelling in ["Ascension", "ascension", "<Ascension>"] {
            let p = select_payload_of(&format!(
                "{{ cards: [EX12-047], trigger_not: \"{spelling}\" }}"
            ))
            .unwrap();
            match p {
                SelectPayload::Cards {
                    trigger_not: Some(ref t),
                    ..
                } => assert_eq!(
                    normalize_trigger_name(t),
                    "ascension",
                    "{spelling} must normalize onto the same answer"
                ),
                other => panic!("expected trigger_not, got {other:?}"),
            }
        }
    }

    #[test]
    fn select_trigger_and_ordinal_together_are_rejected_preferring_trigger() {
        // Two answers to one question. The refusal must also say WHICH to
        // keep: an ordinal is a per-engine POSITION, so it can name a
        // different trigger on the two sides; a keyword cannot.
        let err =
            select_payload_of("{ cards: [EX12-065], trigger: Fortitude, ordinal: 1 }")
                .unwrap_err();
        assert!(err.contains("cannot be combined") || err.contains("two answers"), "got: {err}");
        assert!(err.contains("Prefer `trigger:`"), "must say which to keep: {err}");
        assert!(err.contains("POSITION"), "must say why: {err}");
    }

    #[test]
    fn select_trigger_with_a_multi_card_pick_is_rejected() {
        let err = select_payload_of("{ cards: [A, B], trigger: Fortitude }").unwrap_err();
        assert!(err.contains("single-pick"), "got: {err}");
    }

    #[test]
    fn an_empty_trigger_is_rejected_rather_than_matching_everything() {
        // `trigger: "<>"` normalizes to the empty string, which would compare
        // equal to nothing and silently behave like "no trigger given".
        let err = select_payload_of("{ cards: [A], trigger: \"<>\" }").unwrap_err();
        assert!(err.contains("names no keyword"), "got: {err}");
    }

    #[test]
    fn trigger_spellings_normalize_to_one_answer() {
        // The three ways an author will write the same keyword. The RAW
        // spelling is preserved in the payload (so the YAML round-trips), and
        // the normalizer is what makes them one answer.
        for raw in ["fortitude", "Fortitude", "<Fortitude>", "  <Fortitude> "] {
            let p = select_payload_of(&format!("{{ cards: [EX12-065], trigger: \"{raw}\" }}"))
                .unwrap();
            let SelectPayload::Cards { trigger, .. } = p else {
                panic!("expected a Cards payload");
            };
            let trigger = trigger.expect("the key survives parsing");
            assert_eq!(trigger, raw, "the AUTHORED spelling is what round-trips");
            assert_eq!(
                normalize_trigger_name(&trigger),
                "fortitude",
                "every spelling of one keyword must compare equal"
            );
        }
    }

    #[test]
    fn normalization_bridges_our_bracketed_name_and_dcgos_spaced_one() {
        // Our engine names keyword bodies `<Armor Purge>`; DCGO names the same
        // effect `Armor Purge` (`SetUpICardEffect("Armor Purge", ...)`).
        // Dropping whitespace as well as the brackets is what makes the two
        // sides land on one string.
        assert_eq!(normalize_trigger_name("<Armor Purge>"), "armorpurge");
        assert_eq!(normalize_trigger_name("Armor Purge"), "armorpurge");
        assert_eq!(normalize_trigger_name("armorpurge"), "armorpurge");
        // And distinct keywords stay distinct.
        assert_ne!(
            normalize_trigger_name("<Fortitude>"),
            normalize_trigger_name("<Retaliation>")
        );
    }

    #[test]
    fn select_trigger_round_trips_through_yaml_and_is_omitted_when_absent() {
        let act = StepAction::Select(SelectPayload::Cards {
            ids: vec!["EX12-065".to_string()],
            ordinal: None,
            trigger: Some("<Fortitude>".to_string()),
            trigger_not: None,
        });
        let yaml = serde_yml::to_string(&act).expect("serializes");
        assert!(yaml.contains("trigger"), "the key must survive: {yaml}");
        let back: StepAction = serde_yml::from_str(&yaml).expect("re-parses");
        assert_eq!(back, act);

        let bare = StepAction::Select(SelectPayload::Cards {
            ids: vec!["EX12-065".to_string()],
            ordinal: None,
            trigger: None,
            trigger_not: None,
        });
        let yaml = serde_yml::to_string(&bare).expect("serializes");
        assert!(!yaml.contains("trigger"), "absent must OMIT the key: {yaml}");
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
            StepAction::Select(SelectPayload::Cards {
                ids: vec!["ST1-03".to_string()],
                ordinal: None,
                trigger: None,
                trigger_not: None,
            }),
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

#[cfg(test)]
mod dcgo_only_tests {
    use super::*;

    fn step_yaml(sel: &str) -> String {
        let head = "card: EX12-076\nclause: EX12-076#effect#1\nseed: 1\ndecks:\n  p0: { rest: toho-braves }\n  p1: { rest: toho-braves }\nsteps:\n  - actor: 0\n    do: { select: ";
        format!("{head}{sel} }}\n")
    }

    /// `dcgo_only: true` selects the DCGO-ONLY carrier, not the shared one.
    #[test]
    fn dcgo_only_parses_into_its_own_variant() {
        let s = Scenario::from_yaml(&step_yaml("{ cards: [EX12-076], ordinal: 1, dcgo_only: true }"))
            .expect("dcgo_only step parses");
        match &s.steps[0].act {
            StepAction::SelectDcgoOnly(SelectPayload::Cards { ids, ordinal, .. }) => {
                assert_eq!(ids, &vec!["EX12-076".to_string()]);
                assert_eq!(*ordinal, Some(1));
            }
            other => panic!("expected SelectDcgoOnly, got {other:?}"),
        }
    }

    /// Absent or false, the step stays an ordinary SHARED select. The flag must
    /// never be the default: a mislabelled row would leave our engine's real
    /// prompt unanswered and desync every later step.
    #[test]
    fn without_the_flag_the_step_is_a_normal_select() {
        let s = Scenario::from_yaml(&step_yaml("{ cards: [EX12-076] }")).expect("parses");
        assert!(matches!(&s.steps[0].act, StepAction::Select(_)));
        let s = Scenario::from_yaml(&step_yaml("{ cards: [EX12-076], dcgo_only: false }"))
            .expect("parses");
        assert!(matches!(&s.steps[0].act, StepAction::Select(_)));
    }

    /// It is a MODIFIER, not an answer form: it must not satisfy the
    /// exactly-one-form rule on its own.
    #[test]
    fn dcgo_only_alone_is_still_no_answer_form() {
        let err = Scenario::from_yaml(&step_yaml("{ dcgo_only: true }"))
            .expect_err("a bare dcgo_only carries no answer");
        assert!(err.contains("select"), "got: {err}");
    }

    /// Round-trips, so a re-serialized scenario keeps the flag.
    #[test]
    fn dcgo_only_round_trips() {
        let s = Scenario::from_yaml(&step_yaml("{ cards: [EX12-076], ordinal: 1, dcgo_only: true }"))
            .expect("parses");
        let yaml = serde_yml::to_string(&s.steps[0].act).expect("serializes");
        assert!(yaml.contains("dcgo_only"), "flag survives serialization: {yaml}");
        let back: StepAction = serde_yml::from_str(&yaml).expect("re-parses");
        assert_eq!(back, s.steps[0].act);
    }
}

#[cfg(test)]
mod materials_tests {
    use super::*;

    fn step_yaml(sel: &str) -> String {
        let head = "card: EX12-076\nclause: EX12-076#effect#0\nseed: 1\ndecks:\n  p0: { rest: toho-braves }\n  p1: { rest: toho-braves }\nsteps:\n  - actor: 0\n    do: { select: ";
        format!("{head}{sel} }}\n")
    }

    #[test]
    fn materials_parses_in_recipe_element_order() {
        let s = Scenario::from_yaml(&step_yaml("{ materials: [EX12-004, EX12-011, EX12-020] }"))
            .expect("materials step parses");
        match &s.steps[0].act {
            StepAction::Select(SelectPayload::Materials(ids)) => assert_eq!(
                ids,
                &vec![
                    "EX12-004".to_string(),
                    "EX12-011".to_string(),
                    "EX12-020".to_string()
                ]
            ),
            other => panic!("expected Materials, got {other:?}"),
        }
    }

    /// It is an ANSWER FORM, so it collides with the others.
    #[test]
    fn materials_and_cards_together_are_refused() {
        let err = Scenario::from_yaml(&step_yaml("{ materials: [EX12-004], cards: [EX12-011] }"))
            .expect_err("two answer forms must be refused");
        assert!(err.contains("materials"), "the message names the form: {err}");
    }

    /// An empty declaration declares nothing -- refused rather than silently
    /// answering zero prompts and desyncing the line.
    #[test]
    fn empty_materials_is_refused() {
        let err = Scenario::from_yaml(&step_yaml("{ materials: [] }"))
            .expect_err("empty materials must be refused");
        assert!(err.contains("declares nothing"), "got: {err}");
    }

    /// A material declaration is a SHARED decision; it can never be DCGO-only.
    #[test]
    fn materials_with_dcgo_only_is_refused() {
        let s = Scenario::from_yaml(&step_yaml(
            "{ materials: [EX12-004], dcgo_only: true }",
        ))
        .expect("parses -- the clash is caught at lowering, with a fuller message");
        assert!(matches!(
            &s.steps[0].act,
            StepAction::SelectDcgoOnly(SelectPayload::Materials(_))
        ));
    }

    #[test]
    fn materials_round_trips() {
        let s = Scenario::from_yaml(&step_yaml("{ materials: [EX12-004, EX12-011] }"))
            .expect("parses");
        let yaml = serde_yml::to_string(&s.steps[0].act).expect("serializes");
        let back: StepAction = serde_yml::from_str(&yaml).expect("re-parses");
        assert_eq!(back, s.steps[0].act);
    }
}
