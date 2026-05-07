//! Card effect representation — Effect struct and EffectBuilder.

use crate::card_source::CardHandle;
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::{DelayTrigger, EffectTiming, Keyword};
use crate::permanent::PermanentHandle;
use digimon_dsl::compiled::{CompiledAltPath, CompiledPredicate};
use std::sync::Arc;

/// Condition closures run during effect evaluation and during tensor-time
/// inspection (for static DP modifiers / OPT state). They receive a
/// read-only view of game state; they must not mutate.
pub type ConditionFn = Box<dyn Fn(&EffectReadContext) -> bool + Send + Sync + 'static>;

/// Replacement-effect candidate-filter closure. Evaluated in
/// `replacement::collect_candidates` after `condition` for `WhenWouldBe*`
/// timings, with cause/source/subject replacement context attached to the
/// read context. Returns `true` to keep the candidate in the dispatch list,
/// `false` to skip — used by `<Scapegoat>` to suppress the outer "may" dialog
/// when the deletion cause is `OwnEffect` (RULES_CONTEXT 16-31) and to
/// suppress the dialog when there are no substitute candidates (mirrors DCGO
/// `CanActivateScapegoat`'s `HasMatchConditionPermanent` gate).
///
/// Distinct from `condition`:
/// - `condition` is cause-agnostic and evaluated for every effect timing.
/// - `replacement_condition` is cause-aware and ONLY consulted by the
///   replacement dispatcher when collecting candidates.
///
/// If both are set on a single Effect, both must return true for the
/// candidate to be kept — `condition` runs first.
///
/// **Naming:** distinct from `replacement::ReplacementConditionFn`, which
/// lives on passive modifier registry entries and takes a different
/// signature `(&EffectReadContext, &ReplacementSubject)`. The two types
/// serve adjacent but separate roles in the candidate collection pipeline
/// — passives use the registry's `ReplacementConditionFn`; effect-side
/// candidate filtering uses this `EffectReplacementConditionFn`.
pub type EffectReplacementConditionFn = Box<
    dyn Fn(&EffectReadContext, &crate::replacement::ReplacementSubject) -> bool
        + Send
        + Sync
        + 'static,
>;

pub type ProcessFn = Box<dyn Fn(&mut EffectContext) + Send + Sync + 'static>;
pub type EffectId = u8;
pub type CostReductionFn = Box<dyn Fn(&EffectReadContext) -> i32 + Send + Sync + 'static>;
pub type PayCostFn = Box<dyn Fn(&mut EffectContext) -> bool + Send + Sync + 'static>;
pub type DynamicModifierFn =
    Box<dyn Fn(&EffectReadContext, PermanentHandle) -> Option<i32> + Send + Sync + 'static>;
/// Closure that accepts a read-only context and a candidate host handle,
/// returning `true` iff the host is a legal target for a Link Option.
/// Phase 8: consumed by Link dispatch to mask host-selection prompts.
pub type LinkFilterFn =
    Box<dyn Fn(&EffectReadContext, PermanentHandle) -> bool + Send + Sync + 'static>;
/// Predicate used by parameterized `<Overclock (...)>` to decide which own
/// battle-area permanents can be deleted as the cost.
pub type OverclockCostFilterFn =
    Box<dyn Fn(&EffectReadContext, PermanentHandle) -> bool + Send + Sync + 'static>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReFireableEffect {
    pub effect_id: EffectId,
    pub source_card: CardHandle,
    pub source: PermanentHandle,
    pub source_kind: crate::enums::EffectSourceKind,
    pub controller: crate::enums::PlayerId,
    pub card_id: String,
    pub timing: EffectTiming,
    pub timing_key: String,
}

#[derive(Clone)]
pub struct AltPathRegistrationEffect {
    pub applies_to: Option<Arc<CompiledPredicate>>,
    pub registers: Arc<CompiledAltPath>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EffectObservationMetadata {
    pub categories: EffectCategoryFlags,
    pub target_profile: TargetProfileFlags,
    pub duration: EffectDurationKind,
    pub numeric_buckets: EffectNumericBuckets,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EffectCategoryFlags {
    pub delete: bool,
    pub suspend: bool,
    pub unsuspend: bool,
    pub bounce: bool,
    pub bottom_deck: bool,
    pub dp_change: bool,
    pub draw_search: bool,
    pub memory: bool,
    pub play: bool,
    pub digivolve: bool,
    pub recover: bool,
    pub trash_security: bool,
    pub grant_keyword: bool,
    pub grant_immunity: bool,
    pub protection: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TargetProfileFlags {
    pub own: bool,
    pub opponent: bool,
    pub digimon: bool,
    pub tamer: bool,
    pub option: bool,
    pub battle_area: bool,
    pub breeding_area: bool,
    pub security: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EffectDurationKind {
    #[default]
    Unknown,
    Immediate,
    UntilEndOfTurn,
    UntilOpponentEndOfTurn,
    Persistent,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EffectNumericBuckets {
    pub dp_amount: i16,
    pub memory_amount: i8,
    pub count: u8,
    pub level_threshold: u8,
    pub play_cost_threshold: u8,
}

/// A single card effect with timing and behavior.
pub struct Effect {
    pub timing: EffectTiming,
    pub name: String,
    pub source_card: CardHandle,
    pub observation_metadata: EffectObservationMetadata,

    // Timing / kind flags
    pub on_play: bool,
    pub when_digivolving: bool,
    pub on_attack: bool,
    pub on_deletion: bool,
    pub inherited: bool,
    pub security: bool,
    pub counter: bool,
    pub declarative: bool,
    /// True only for process-backed declaratives that materialize static
    /// modifier/keyword state during `Game::tick_declarative_effects`.
    pub materializes_declarative_state: bool,
    pub optional: bool,
    /// Heuristic metadata for policies: this effect produces or preserves
    /// hand/resource flow, such as drawing, searching/add-to-hand, filtering
    /// hand, or Save-like deletion value. This is deliberately advisory; it
    /// never affects rules execution.
    pub resource_flow: bool,
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
    /// Context-aware candidate filter for `WhenWouldBe*` replacement timings.
    /// Evaluated in `replacement::collect_candidates` AFTER `condition`,
    /// with replacement cause/source/subject context attached to the read
    /// context. Both must return true for the candidate to be kept. See
    /// `EffectReplacementConditionFn`.
    pub replacement_condition: Option<EffectReplacementConditionFn>,
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
    pub dp_modifier_fn: Option<DynamicModifierFn>,
    pub security_attack_fn: Option<DynamicModifierFn>,
    pub cost_reduction: i32,
    pub granted_keyword: Option<Keyword>,
    pub overclock_cost_filter: Option<OverclockCostFilterFn>,

    /// When `true`, this effect's `dp_modifier` is applied to the opposing
    /// security Digimon's DP during the §2.5 security DP battle. Used for
    /// inherited effects like "This Digimon gains +3000 DP when attacking
    /// security". Set by `.applies_to_opponent_security_dp()`.
    pub applies_to_opponent_security_dp: bool,

    /// Replacement-effect process closure — wired for "Would*" timings in
    /// Phase 7. Receives a `ReplacementContext` so the process can mutate
    /// game state AND set the replacement outcome (cancel / redirect /
    /// substitute / handled). Dispatch lands in Task 2.
    pub replacement_process: Option<crate::replacement::ReplacementProcessFn>,

    // ── Phase 8 Option flags ─────────────────────────────────────────────
    /// True for an Option's [Main] body effect. Set by `.option_main()` /
    /// `.link()` / `.training()`. Dispatch wires in Tasks 2-6.
    pub option_main: bool,
    /// Optional condition that lets this Option satisfy its play color
    /// requirement through printed `<Use Req. (...)>` text.
    pub option_color_requirement_bypass: Option<ConditionFn>,
    /// Delay trigger for a Delay Option's body. Set by `.delay(trigger)`.
    pub delay_trigger: Option<DelayTrigger>,
    /// Memory cost paid to link this card to a host Digimon. Set by
    /// `.link(cost, filter)`. `None` for non-link Options.
    pub link_cost: Option<u16>,
    /// Filter closure selecting legal host Digimon for a Link Option.
    /// Set by `.link(cost, filter)`.
    pub link_filter: Option<LinkFilterFn>,
    pub alt_path_registration: Option<AltPathRegistrationEffect>,
    /// True for a Training card — placed into the Breeding Area with the
    /// Training state. Set by `.training()`.
    pub training: bool,
    /// True for an effect that sideways-inherits from a linked card onto its
    /// host. When a host Digimon's timing `T` fires, the host's own
    /// `timing == T` effects fire; if `linked == true`, effects on any card
    /// in `host.linked_cards` also fire at timing `T`, attributed to the
    /// host's controller. Set by `.linked()`. Phase 8 Task 4. Mirrors DCGO's
    /// "Plug-In gives the host this effect" semantics for Appmon-trait cards.
    pub linked: bool,

    /// When `true`, this `BeforePayCost` cost-reduction effect is scoped to
    /// "when THIS specific card is being played from hand."  The effect must
    /// NOT fire when the card is already on the field and another card is being
    /// played. Set by `.when_playing_this()` in the builder; enforced by
    /// `scan_before_pay_cost_reduction` (which skips field-permanent sources
    /// whose effect has this flag) and
    /// `scan_before_pay_cost_reduction_for_hand_card` (which evaluates it).
    pub when_playing_this: bool,
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
    pub fn timing_key(&self) -> &'static str {
        match self.timing {
            EffectTiming::WhenDigivolving => "when_digivolving",
            EffectTiming::OnPlay => "on_play",
            EffectTiming::WhenAttacking => "when_attacking",
            EffectTiming::OnDeletion => "on_deletion",
            _ => "",
        }
    }

    pub fn can_be_refired(&self) -> bool {
        matches!(self.timing, EffectTiming::WhenDigivolving)
            && self.when_digivolving
            && !self.inherited
            && !self.security
            && !self.declarative
            && !self.linked
    }

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

    /// Fires when this Digimon DNA digivolves (as the merged result card).
    /// Refines `OnDigivolve`: a card with both an `OnDigivolve` and an
    /// `OnDnaDigivolve` clause sees the broader timing fire first, then the
    /// more-specific one, on the same merged permanent.
    pub fn on_dna_digivolve(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnDnaDigivolve)
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

    /// Fires when a blocker is declared. Global observer timing — both
    /// players' battle areas are scanned. Spec §7 / Phase 9 Task 8.
    pub fn on_block(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnBlock)
    }

    /// Fires when an allied Digimon (same controller, excluding the attacker
    /// itself) attacks. Observer timing — scans the attacker-controller's
    /// battle area. Spec §7 / Phase 9 Task 8.
    pub fn on_ally_attack(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnAllyAttack)
    }

    /// Fires when an opposing Digimon attacks. Observer timing — scans the
    /// non-attacker controller's battle area. Spec §7 / Phase 9 Task 8.
    pub fn on_opponent_attack(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnOpponentAttack)
    }

    /// Fires when a Digimon hatches from the breeding area into the battle area.
    pub fn on_hatch(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnHatch)
    }

    /// Fires when a breeding-area Digimon moves into the battle area.
    pub fn on_move(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnMove)
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

    /// Fires after a persistent Option card is placed in the battle area.
    pub fn on_option_placed(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::OnOptionPlaced)
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
    pub fn when_permanent_would_digivolve(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::WhenPermanentWouldDigivolve)
    }
    pub fn when_permanent_would_play(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::WhenPermanentWouldPlay)
    }
    pub fn when_would_link(card: CardHandle) -> EffectBuilder {
        EffectBuilder::new(card, EffectTiming::WhenWouldLink)
    }
}

/// Builder for constructing effects ergonomically.
pub struct EffectBuilder {
    inner: Effect,
}

impl EffectBuilder {
    /// Low-level constructor. Prefer the typed entry points on `Effect`
    /// (`Effect::on_play`, `Effect::on_attack`, `Effect::option_main`, etc.)
    /// which pre-set the relevant flags and timing — reach for
    /// `EffectBuilder::new` only when you need a timing that does not yet
    /// have a sugar constructor, or when building a test double that needs
    /// to start from `EffectTiming::None`.
    pub fn new(card: CardHandle, timing: EffectTiming) -> Self {
        Self {
            inner: Effect {
                timing,
                name: String::new(),
                source_card: card,
                observation_metadata: EffectObservationMetadata::default(),
                on_play: false,
                when_digivolving: false,
                on_attack: false,
                on_deletion: false,
                inherited: false,
                security: false,
                counter: false,
                declarative: false,
                materializes_declarative_state: false,
                optional: false,
                resource_flow: false,
                blast_digivolve: false,
                max_per_turn: 0,
                condition: None,
                replacement_condition: None,
                process: None,
                cost_reduction_fn: None,
                pay_cost_fn: None,
                dp_modifier: 0,
                dp_modifier_fn: None,
                security_attack_fn: None,
                cost_reduction: 0,
                granted_keyword: None,
                overclock_cost_filter: None,
                applies_to_opponent_security_dp: false,
                replacement_process: None,
                option_main: false,
                option_color_requirement_bypass: None,
                delay_trigger: None,
                link_cost: None,
                link_filter: None,
                alt_path_registration: None,
                training: false,
                linked: false,
                when_playing_this: false,
            },
        }
    }

    /// Mark this `BeforePayCost` effect as "when THIS card is being played
    /// from hand." The scan in `scan_before_pay_cost_reduction` will skip
    /// this effect when the source is a field permanent; it is evaluated
    /// separately in `scan_before_pay_cost_reduction_for_hand_card`.
    pub fn when_playing_this(mut self) -> Self {
        self.inner.when_playing_this = true;
        self
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

    pub fn materializes_declarative_state(mut self) -> Self {
        self.inner.materializes_declarative_state = true;
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

    /// Mark this effect as a Counter candidate. Used by Phase 9 Task 3 to
    /// broaden the CounterTiming window beyond blast-digivolve:
    ///
    /// - Applied to an Option card's effect (via `.counter()` +
    ///   `.timing(EffectTiming::CounterEffect)`) → the card becomes a hand
    ///   Counter Option candidate during the defender's Counter window,
    ///   routed through Phase 8's `play_option_from_hand` pipeline with
    ///   CounterEffect firing BEFORE OptionMain.
    /// - Applied to a battle-area Digimon's triggered ability (via
    ///   `.counter()` + `.timing(EffectTiming::CounterEffect)`) → the
    ///   permanent becomes a field Counter ability candidate; selection
    ///   fires the body directly at zero cost.
    ///
    /// Distinct from `.blast_digivolve()`, which sets BOTH
    /// `blast_digivolve = true` AND `counter = true`. `.counter()` alone
    /// does NOT imply blast. See spec
    /// `docs/superpowers/specs/2026-04-21-combat-interrupt-completion-design.md`
    /// §5 for the full window broadening semantics.
    pub fn counter(mut self) -> Self {
        self.inner.counter = true;
        self
    }

    pub fn optional(mut self) -> Self {
        self.inner.optional = true;
        self
    }

    pub fn observation_metadata(mut self, metadata: EffectObservationMetadata) -> Self {
        self.inner.observation_metadata = metadata;
        self
    }

    /// Mark the effect as card/resource-flow positive for policy heuristics.
    pub fn resource_flow(mut self) -> Self {
        self.inner.resource_flow = true;
        self
    }

    /// Alias for effects that draw or add cards to hand.
    pub fn adds_cards_to_hand(self) -> Self {
        self.resource_flow()
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

    /// Attach a context-aware candidate filter for `WhenWouldBe*` replacements.
    /// Evaluated in `replacement::collect_candidates` after `condition`.
    /// See `EffectReplacementConditionFn` doc for the full contract; primary
    /// use is `<Scapegoat>` suppressing the outer "may" dialog on
    /// `ReplacementCause::OwnEffect` per RULES_CONTEXT 16-31, and on the
    /// no-substitute case mirroring DCGO `HasMatchConditionPermanent`.
    pub fn replacement_condition(
        mut self,
        f: impl Fn(&EffectReadContext, &crate::replacement::ReplacementSubject) -> bool
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.inner.replacement_condition = Some(Box::new(f));
        self
    }

    pub fn process(mut self, f: impl Fn(&mut EffectContext) + Send + Sync + 'static) -> Self {
        self.inner.process = Some(Box::new(f));
        self
    }

    pub fn dp_modifier(mut self, n: i32) -> Self {
        self.inner.dp_modifier = n;
        self
    }

    pub fn dp_modifier_fn<F>(mut self, f: F) -> Self
    where
        F: Fn(&EffectReadContext, PermanentHandle) -> Option<i32> + Send + Sync + 'static,
    {
        self.inner.dp_modifier_fn = Some(Box::new(f));
        self
    }

    pub fn security_attack_fn<F>(mut self, f: F) -> Self
    where
        F: Fn(&EffectReadContext, PermanentHandle) -> Option<i32> + Send + Sync + 'static,
    {
        self.inner.security_attack_fn = Some(Box::new(f));
        self
    }

    pub fn alt_path_registration(
        mut self,
        applies_to: Option<Arc<CompiledPredicate>>,
        registers: Arc<CompiledAltPath>,
    ) -> Self {
        self.inner.alt_path_registration = Some(AltPathRegistrationEffect {
            applies_to,
            registers,
        });
        self
    }

    /// Flag this effect as a DP adjustment applied to the opposing security
    /// Digimon during the security DP battle (§2.5e). Intended for inherited
    /// effects like "This Digimon gains +3000 DP when attacking security" —
    /// set on an `inherited()` effect in tandem with `.dp_modifier(n)` and
    /// the iterator walks the attacker's digivolution stack to sum the
    /// contributions.
    pub fn applies_to_opponent_security_dp(mut self) -> Self {
        self.inner.applies_to_opponent_security_dp = true;
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

    pub fn granted_keyword(mut self, keyword: Keyword) -> Self {
        self.inner.granted_keyword = Some(keyword);
        self
    }

    pub fn overclock_with_cost_filter<F>(mut self, filter: F) -> Self
    where
        F: Fn(&EffectReadContext, PermanentHandle) -> bool + Send + Sync + 'static,
    {
        self.inner.overclock_cost_filter = Some(Box::new(filter));
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
    /// Queued triggered effects may install a `PendingSelection` while paying
    /// the cost. In that case the effect parks before `process`, resumes only
    /// after the selection chain resolves, and can be discarded by calling
    /// `EffectContext::decline_pending_pay_cost()` from the selection
    /// callback.
    ///
    /// Phase 5 dispatch wires up in Tasks 3-4.
    pub fn pay_cost_fn<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut EffectContext) -> bool + Send + Sync + 'static,
    {
        self.inner.pay_cost_fn = Some(Box::new(f));
        self
    }

    /// Alias for [`Self::pay_cost_fn`].
    pub fn pay_cost<F>(self, f: F) -> Self
    where
        F: Fn(&mut EffectContext) -> bool + Send + Sync + 'static,
    {
        self.pay_cost_fn(f)
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

    // ── Phase 8 Option builder methods ───────────────────────────────────

    /// Mark this effect as an Option's [Main] body. Sets timing to
    /// `OptionMain` and raises `option_main`. Dispatch lands in Task 2.
    pub fn option_main(mut self) -> Self {
        self.inner.timing = EffectTiming::OptionMain;
        self.inner.option_main = true;
        self
    }

    /// Attach an Option-use requirement predicate. When this condition is
    /// true, the option may be used even if no matching-color Digimon/Tamer is
    /// present.
    pub fn option_color_requirement_bypass_condition<F>(mut self, f: F) -> Self
    where
        F: Fn(&EffectReadContext) -> bool + Send + Sync + 'static,
    {
        self.inner.option_color_requirement_bypass = Some(Box::new(f));
        self
    }

    /// Mark this effect as a Delay Option's body. Sets timing to
    /// `DelayEffect` and records the trigger. Dispatch lands in Task 3.
    pub fn delay(mut self, trigger: DelayTrigger) -> Self {
        self.inner.timing = EffectTiming::DelayEffect;
        self.inner.delay_trigger = Some(trigger);
        self
    }

    /// Mark this effect as a Link Option — attaches to a host Digimon for
    /// `cost` memory. `digimon_filter` selects legal hosts at mask time.
    /// Sets timing to `OptionMain` so the effect fires as the Option's
    /// body. Dispatch lands in Task 4.
    pub fn link<F>(mut self, cost: u16, digimon_filter: F) -> Self
    where
        F: Fn(&EffectReadContext, PermanentHandle) -> bool + Send + Sync + 'static,
    {
        self.inner.timing = EffectTiming::OptionMain;
        self.inner.link_cost = Some(cost);
        self.inner.link_filter = Some(Box::new(digimon_filter));
        self
    }

    /// Mark this effect as a Training Option body. Sets timing to
    /// `OptionMain` and raises `training`. Dispatch lands in Task 5.
    pub fn training(mut self) -> Self {
        self.inner.timing = EffectTiming::OptionMain;
        self.inner.training = true;
        self
    }

    /// Mark this effect as sideways-inherited from a linked card onto its
    /// host. The effect's timing is preserved (e.g. `.start_of_your_turn()`
    /// + `.linked()` fires at the host's StartOfYourTurn, not at OnPlay).
    /// Phase 8 Task 4 — dispatched in `enqueue_from_permanent` which scans
    /// the host's `linked_cards` for `.linked` effects at every triggered
    /// timing dispatch.
    ///
    /// Mutually exclusive with `.inherited()` — linked cards grant their
    /// effect to the host, not to stack-above levels.
    pub fn linked(mut self) -> Self {
        self.inner.linked = true;
        self
    }

    /// Chainable `inherited` flag — raises the `inherited` marker without
    /// clobbering timing. Use when a non-digivolution-stack effect needs
    /// the sideways-inheritance flag (Phase 8 Task 5: Training Option
    /// sideways scan matches `effect.inherited` on the Training card).
    ///
    /// Distinct from the `Effect::inherited(card)` constructor, which
    /// forces `timing = None` for declarative digivolution-stack effects.
    pub fn inherited(mut self) -> Self {
        self.inner.inherited = true;
        self
    }

    pub fn build(self) -> Effect {
        debug_assert!(
            !(self.inner.linked && self.inner.inherited),
            ".linked() and .inherited() are mutually exclusive — a linked card's effects apply to its host, not to stack-above levels"
        );
        self.inner
    }
}

/// Trait implemented by each card's effect script.
/// One struct per card_id; returns the card's effects parameterized by handle.
pub trait CardEffect: Send + Sync {
    fn effects(&self, card: CardHandle) -> Vec<Effect>;
}

pub fn enumerate_refireable_effects(
    game: &crate::game::Game,
    source: PermanentHandle,
    timing_key: &str,
) -> Vec<ReFireableEffect> {
    let Some(perm) = game
        .players
        .get(source.player as usize)
        .and_then(|p| p.battle_area.get(source.index as usize))
    else {
        return Vec::new();
    };
    let top = perm.top_card();
    let source_card = top.handle();
    let card_id = top.card_id(&game.card_data).to_string();
    let source_kind = match top.card_kind(&game.card_data) {
        crate::enums::CardKind::Digimon
        | crate::enums::CardKind::DigiEgg
        | crate::enums::CardKind::Dual => crate::enums::EffectSourceKind::Digimon,
        crate::enums::CardKind::Tamer => crate::enums::EffectSourceKind::Tamer,
        crate::enums::CardKind::Option => crate::enums::EffectSourceKind::Option,
        crate::enums::CardKind::Token => crate::enums::EffectSourceKind::Rule,
    };

    let Some(effects) = game.effects_for_card(&card_id, source_card) else {
        return Vec::new();
    };

    effects
        .iter()
        .enumerate()
        .filter(|(_, effect)| effect.timing_key() == timing_key)
        .filter(|(_, effect)| effect.can_be_refired())
        .map(|(slot, effect)| ReFireableEffect {
            effect_id: slot as EffectId,
            source_card,
            source,
            source_kind,
            controller: source.player,
            card_id: card_id.clone(),
            timing: effect.timing,
            timing_key: timing_key.to_string(),
        })
        .collect()
}
