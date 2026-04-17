//! Card effect representation — Effect struct and EffectBuilder.

use crate::card_source::CardHandle;
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::EffectTiming;

/// Condition closures run during effect evaluation and during tensor-time
/// inspection (for static DP modifiers / OPT state). They receive a
/// read-only view of game state; they must not mutate.
pub type ConditionFn = Box<dyn Fn(&EffectReadContext) -> bool + Send + Sync>;
pub type ProcessFn = Box<dyn Fn(&mut EffectContext) + Send + Sync>;

/// A single card effect with timing and behavior.
pub struct Effect {
    pub timing: EffectTiming,
    pub name: String,
    pub source_card: CardHandle,

    // Timing / kind flags
    pub on_play: bool,
    pub when_digivolving: bool,
    pub on_attack: bool,
    pub on_deletion: bool,
    pub inherited: bool,
    pub security: bool,
    pub counter: bool,
    pub declarative: bool,
    pub optional: bool,
    /// Marks this effect as a blast-digivolve declaration — the card can be
    /// stacked onto a battle-area Digimon during the defender's
    /// `CounterTiming` window at zero memory cost. Consumed by
    /// `combat::try_enter_counter` (RUST_PYTHON_PARITY §2.3). Mirrors
    /// Python's `effect._is_blast_digivolve` flag.
    pub blast_digivolve: bool,

    /// 0 = unlimited, 1 = once per turn, etc.
    pub max_per_turn: u8,

    // Behavior
    pub condition: Option<ConditionFn>,
    pub process: Option<ProcessFn>,

    // Declarative modifier values (set by builder for static modifiers)
    pub dp_modifier: i32,
    pub cost_reduction: i32,
}

impl std::fmt::Debug for Effect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Effect")
            .field("timing", &self.timing)
            .field("name", &self.name)
            .field("on_play", &self.on_play)
            .field("when_digivolving", &self.when_digivolving)
            .field("inherited", &self.inherited)
            .finish()
    }
}

impl Effect {
    /// Builder constructor for an On Play effect.
    pub fn on_play(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnPlay).on_play_flag()
    }

    /// Builder constructor for a When Digivolving effect.
    pub fn when_digivolving(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::WhenDigivolving).when_digivolving_flag()
    }

    /// Builder constructor for an On Attack effect.
    pub fn on_attack(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnAttack).on_attack_flag()
    }

    /// Builder constructor for an On Deletion effect.
    pub fn on_deletion(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnDeletion).on_deletion_flag()
    }

    /// Builder constructor for an inherited effect.
    pub fn inherited(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::None).inherited_flag()
    }

    /// Builder constructor for a security effect (the primary per-card
    /// trigger that fires when this card is revealed from security). Sets
    /// timing = `SecuritySkill` and raises the `security` flag.
    pub fn security(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::SecuritySkill).security_flag()
    }

    /// Builder constructor for an observer that fires **once per security
    /// check** after the revealed card's own `SecuritySkill` effects resolve.
    /// Used for field/trash/hand effects that react to security checks
    /// globally. Mirrors Python's `EffectTiming.OnSecurityCheck`.
    pub fn on_security_check(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnSecurityCheck)
    }

    /// Builder constructor for an effect that fires when a card leaves the
    /// security stack — triggered on both trash-after-reveal and
    /// play-from-security paths. Mirrors Python's `EffectTiming.OnLoseSecurity`.
    pub fn on_lose_security(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnLoseSecurity)
    }

    /// Builder constructor for a declarative (always-on) effect.
    pub fn declarative(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::Declarative).declarative_flag()
    }

    /// Builder constructor for an `End of Your Turn` effect.
    pub fn end_of_your_turn(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::EndOfYourTurn)
    }
}

/// Builder for constructing effects ergonomically.
pub struct EffectBuilder {
    inner: Effect,
}

impl EffectBuilder {
    fn new(card: CardHandle, timing: EffectTiming) -> Self {
        Self {
            inner: Effect {
                timing,
                name: String::new(),
                source_card: card,
                on_play: false,
                when_digivolving: false,
                on_attack: false,
                on_deletion: false,
                inherited: false,
                security: false,
                counter: false,
                declarative: false,
                optional: false,
                blast_digivolve: false,
                max_per_turn: 0,
                condition: None,
                process: None,
                dp_modifier: 0,
                cost_reduction: 0,
            },
        }
    }

    fn on_play_flag(mut self) -> Self {
        self.inner.on_play = true;
        self
    }
    fn when_digivolving_flag(mut self) -> Self {
        self.inner.when_digivolving = true;
        self
    }
    fn on_attack_flag(mut self) -> Self {
        self.inner.on_attack = true;
        self
    }
    fn on_deletion_flag(mut self) -> Self {
        self.inner.on_deletion = true;
        self
    }
    fn inherited_flag(mut self) -> Self {
        self.inner.inherited = true;
        self
    }
    fn security_flag(mut self) -> Self {
        self.inner.security = true;
        self
    }
    fn declarative_flag(mut self) -> Self {
        self.inner.declarative = true;
        self
    }

    pub fn name(mut self, n: &str) -> Self {
        self.inner.name = n.to_string();
        self
    }

    /// Mark this effect as a blast-digivolve declaration. The card becomes
    /// a candidate in the defender's `CounterTiming` window. See
    /// RUST_PYTHON_PARITY §2.3 for semantics.
    pub fn blast_digivolve(mut self) -> Self {
        self.inner.blast_digivolve = true;
        self.inner.counter = true; // diagnostic consistency with Python's is_counter_effect
        self
    }

    pub fn optional(mut self) -> Self {
        self.inner.optional = true;
        self
    }

    pub fn once_per_turn(mut self) -> Self {
        self.inner.max_per_turn = 1;
        self
    }

    pub fn timing(mut self, t: EffectTiming) -> Self {
        self.inner.timing = t;
        self
    }

    pub fn condition(
        mut self,
        f: impl Fn(&EffectReadContext) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.inner.condition = Some(Box::new(f));
        self
    }

    pub fn process(
        mut self,
        f: impl Fn(&mut EffectContext) + Send + Sync + 'static,
    ) -> Self {
        self.inner.process = Some(Box::new(f));
        self
    }

    pub fn dp_modifier(mut self, n: i32) -> Self {
        self.inner.dp_modifier = n;
        self
    }

    pub fn cost_reduction(mut self, n: i32) -> Self {
        self.inner.cost_reduction = n;
        self
    }

    pub fn build(self) -> Effect {
        self.inner
    }
}

/// Trait implemented by each card's effect script.
/// One struct per card_id; returns the card's effects parameterized by handle.
pub trait CardEffect: Send + Sync {
    fn effects(&self, card: CardHandle) -> Vec<Effect>;
}
