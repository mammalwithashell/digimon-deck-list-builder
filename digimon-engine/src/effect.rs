//! Card effect representation — Effect struct and EffectBuilder.

use crate::card_source::CardHandle;
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::EffectTiming;

/// Condition closures run during effect evaluation and during tensor-time
/// inspection (for static DP modifiers / OPT state). They receive a
/// read-only view of game state; they must not mutate.
pub type ConditionFn = Box<dyn Fn(&EffectReadContext) -> bool + Send + Sync + 'static>;
pub type ProcessFn = Box<dyn Fn(&mut EffectContext) + Send + Sync + 'static>;
pub type CostReductionFn = Box<dyn Fn(&EffectReadContext) -> i32 + Send + Sync + 'static>;
pub type PayCostFn = Box<dyn Fn(&mut EffectContext) -> bool + Send + Sync + 'static>;

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

    // Phase 5 closure-valued cost hooks (dispatch wired in Tasks 2-4).
    /// Returns the amount by which to reduce the play/digivolve cost when this
    /// effect is active. Takes a read-only context because the reduction
    /// calculation must be pure (called during cost inspection and masking).
    pub cost_reduction_fn: Option<CostReductionFn>,
    /// Custom cost-payment logic — invoked in place of the default memory
    /// deduction when this effect fires. Returns `true` if the cost was
    /// successfully paid, `false` to abort the action. Takes a mutable context
    /// because paying costs may trash cards, suspend permanents, etc.
    pub pay_cost_fn: Option<PayCostFn>,

    // Declarative modifier values (set by builder for static modifiers)
    pub dp_modifier: i32,
    pub cost_reduction: i32,

    /// Replacement-effect process closure — wired for "Would*" timings in
    /// Phase 7. Receives a `ReplacementContext` so the process can mutate
    /// game state AND set the replacement outcome (cancel / redirect /
    /// substitute / handled). Dispatch lands in Task 2.
    pub replacement_process: Option<crate::replacement::ReplacementProcessFn>,
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

    /// Fires at the start of the controller's turn (before draw).
    pub fn start_of_your_turn(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::StartOfYourTurn)
    }

    /// Fires at the start of the opponent's turn.
    pub fn start_of_opponents_turn(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::StartOfOpponentsTurn)
    }

    /// Fires at the start of the controller's Main phase (after Draw).
    pub fn start_of_your_main_phase(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::StartOfYourMainPhase)
    }

    /// Fires at the end of the opponent's turn.
    pub fn end_of_opponents_turn(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::EndOfOpponentsTurn)
    }

    /// Fires when this Digimon is attacking (before security check).
    pub fn when_attacking(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::WhenAttacking)
    }

    /// Fires when an attack sequence ends (after all battle resolution).
    pub fn end_of_attack(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::EndOfAttack)
    }

    /// Fires when a battle resolves (DP comparison done) but before `EndOfAttack`.
    pub fn end_of_battle(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::EndOfBattle)
    }

    /// Fires when any Digimon enters any player's battle area (global observer).
    pub fn on_enter_field_anyone(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnEnterFieldAnyone)
    }

    /// Fires when any permanent is deleted, for either player's battle area.
    pub fn on_any_deletion(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnAnyDeletion)
    }

    /// Fires when this Digimon digivolves (as the evolving card).
    pub fn on_digivolve(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnDigivolve)
    }

    /// Fires when this permanent becomes suspended.
    pub fn on_suspend(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnSuspend)
    }

    /// Fires when this permanent becomes unsuspended.
    pub fn on_unsuspend(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnUnsuspend)
    }

    /// Fires when an attack declaration's target changes mid-combat.
    pub fn on_attack_target_change(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnAttackTargetChange)
    }

    /// Fires when a Digimon hatches from the breeding area into the battle area.
    pub fn on_hatch(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnHatch)
    }

    /// Fires when an opponent's security card is removed from their security stack.
    /// Medusamon core archetype observer.
    pub fn on_opponent_security_removed(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnOpponentSecurityRemoved)
    }

    /// Fires when a card is trashed from a permanent's digivolution stack.
    /// Rocks core archetype observer.
    pub fn on_digivolution_card_trashed(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnDigivolutionCardTrashed)
    }

    /// Builder constructor for a BeforePayCost effect — fires during cost
    /// calculation before memory is deducted. Use with `.cost_reduction_fn`
    /// for dynamic cost reduction or `.pay_cost_fn` for custom payment logic.
    /// Phase 5 dispatch wires up in Tasks 2-4.
    pub fn before_pay_cost(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::BeforePayCost)
    }

    // ── Phase 7 "Would*" replacement-effect constructors ─────────────────
    // Dispatch via Game::try_replace lands in Task 2. These are pure
    // builder entry points for now; attach a `.replacement_process(...)`
    // closure to install the replacement logic.

    pub fn when_would_be_deleted(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::WhenWouldBeDeleted)
    }
    pub fn when_would_leave_battle_area(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::WhenWouldLeaveBattleArea)
    }
    pub fn when_would_be_returned_to_hand(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::WhenWouldBeReturnedToHand)
    }
    pub fn when_would_be_returned_to_deck(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::WhenWouldBeReturnedToDeck)
    }
    pub fn when_would_be_trashed(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::WhenWouldBeTrashed)
    }
    pub fn when_would_be_de_digivolved(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::WhenWouldBeDeDigivolved)
    }
    pub fn when_would_lose_security(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::WhenWouldLoseSecurity)
    }
    pub fn when_would_draw(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::WhenWouldDraw)
    }
    pub fn when_would_place_in_security(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::WhenWouldPlaceInSecurity)
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
                cost_reduction_fn: None,
                pay_cost_fn: None,
                dp_modifier: 0,
                cost_reduction: 0,
                replacement_process: None,
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

    /// Attach a closure that computes how much to reduce the play/digivolve
    /// cost when this BeforePayCost effect is active. The closure receives a
    /// read-only context — it must not mutate game state.
    ///
    /// Dispatched in Task 2 at `EffectTiming::BeforePayCost` scan during
    /// play/digivolve cost calculation.
    pub fn cost_reduction_fn<F>(mut self, f: F) -> Self
    where
        F: Fn(&EffectReadContext) -> i32 + Send + Sync + 'static,
    {
        self.inner.cost_reduction_fn = Some(Box::new(f));
        self
    }

    pub fn cost_reduction(mut self, n: i32) -> Self {
        self.inner.cost_reduction = n;
        self
    }

    /// Install a pay-cost closure that gates this effect's execution.
    ///
    /// Dispatch depends on the effect's timing:
    /// - For `EffectTiming::BeforePayCost`: fires during play/digivolve cost
    ///   calculation AFTER reduction accumulation, BEFORE `pay_memory`. Returning
    ///   `false` skips the reduction contribution (the play itself still
    ///   proceeds at full cost). Returning `true` means "cost paid; apply
    ///   reduction".
    /// - For any other triggered timing: fires in `run_queued_effect` AFTER
    ///   the condition passes, BEFORE `process` runs. Returning `false` aborts
    ///   the effect silently (process does not fire). Returning `true` means
    ///   "cost paid; continue to process".
    ///
    /// The closure receives `&mut EffectContext` so it can trash cards,
    /// suspend permanents, or otherwise mutate game state to pay the cost.
    ///
    /// **v1 constraint:** synchronous — the closure must NOT install a
    /// `PendingSelection`. For selection-gated pay-costs, fold the selection
    /// into `process` for now. See Phase 5 non-goals.
    ///
    /// Phase 5 dispatch wires up in Tasks 3-4.
    pub fn pay_cost_fn<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut EffectContext) -> bool + Send + Sync + 'static,
    {
        self.inner.pay_cost_fn = Some(Box::new(f));
        self
    }

    /// Attach a replacement-effect process for "Would*" timings.
    /// The closure receives a `ReplacementContext` and sets the outcome
    /// (cancel / redirect / substitute / handled) via its helper methods.
    /// Dispatch through `Game::try_replace` lands in Task 2.
    pub fn replacement_process<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut crate::replacement::ReplacementContext<'_>) + Send + Sync + 'static,
    {
        self.inner.replacement_process = Some(Box::new(f));
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
