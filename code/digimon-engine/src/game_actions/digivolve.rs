//! Digivolve / DNA-digivolve operations (Tier 2) — `impl Game`.

#![allow(unused_imports)]
use super::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::combat::*;
use crate::digixros::*;
use crate::effect::*;
use crate::effect_context::*;
use crate::enums::*;
use crate::game::*;
use crate::modifiers::*;
use crate::permanent::*;
use crate::player::*;
use crate::replacement::*;
use crate::rules::*;
use crate::scheduled_effects::*;
use crate::selection::*;
use crate::token_registry::*;
use crate::trigger_context::*;
use rand::seq::SliceRandom;

/// Which digivolution requirements an effect-initiated digivolve waives —
/// the engine-side mirror of DCGO `CardEffectCommons.IgnoreRequirement`.
///
/// - `None`: every requirement applies (route machinery, printed cost).
/// - `Color`: colour waived on printed circles (level still applies).
/// - `Level`: level waived (colour-matching circles of any level and
///   level-stripped alt-paths; the printed cost is still paid).
///   G-DIGIVOLVE-IGNORE-LEVEL-PRINTED-COST.
/// - `All`: "ignoring digivolution requirements" — no route needed, base
///   cost 0 before the effect's `CostDelta` (normally `Fixed(n)`); also the
///   only mode besides `Level` that accepts a level-less (Tamer) base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DigivolveRequirementWaiver {
    None,
    Color,
    Level,
    All,
}

impl DigivolveRequirementWaiver {
    fn color_if(ignore_color: bool) -> Self {
        if ignore_color {
            Self::Color
        } else {
            Self::None
        }
    }
}

/// make-engine-cloneable: resumable-VM frame state for the digivolve cost-choice
/// prompt (rule 17 — when a base satisfies >1 of a hand card's digivolution
/// requirements at different costs). Plain data mirroring
/// `install_digivolve_cost_choice_prompt`'s closure captures, so a clone paused
/// at the cost choice replays the chosen cost faithfully. The prompt is a
/// top-level player action (dispatched from `action/decode.rs`), never nested in
/// a DSL clause — so it needs no `outer_conts`.
#[derive(Debug, Clone)]
pub(crate) struct DigivolveCostChoiceState {
    pub acting_player: PlayerId,
    pub hand_index: usize,
    pub field_index: usize,
    pub source: PlaySource,
    pub routes: Vec<crate::dna_digivolve::DigivolveRouteMatch>,
}

/// A cost choice inside an effect, independent of the main-action resume.
/// Zone indices may move while parked; physical cards and the original zones
/// remain pinned. A changed host top fails instead of following DNA materials.
#[derive(Clone)]
pub(crate) struct EffectDigivolveCostChoiceState {
    pub acting_player: PlayerId,
    pub source_ref: CardSourceRef,
    pub result_card: CardHandle,
    pub source_carrier_top: Option<CardHandle>,
    pub source_carrier_identity: Option<u64>,
    pub target_player: PlayerId,
    pub target_zone: Zone,
    pub target_top: CardHandle,
    pub target_identity: u64,
    pub dna_origin: Option<bool>,
    pub cost_delta: CostDelta,
    pub waiver: DigivolveRequirementWaiver,
    pub source: PlaySource,
    pub printed_costs: Vec<crate::dna_digivolve::DigivolveRouteMatch>,
    pub effect_source_player: Option<PlayerId>,
    pub trigger_context: Option<TriggerContext>,
    /// Only the initiating DSL clause receives the completed result.
    pub result_clause: Option<(CardHandle, PlayerId)>,
    pub outer_conts: Vec<crate::resume::OuterContinuation>,
}

impl EffectDigivolveCostChoiceState {
    pub(crate) fn belief_safe(
        &self,
        viewer: PlayerId,
        allowed: &std::collections::HashSet<CardHandle>,
    ) -> bool {
        self.acting_player == viewer
            && allowed.contains(&self.result_card)
            && allowed.contains(&self.target_top)
            && self.source_carrier_top.is_none_or(|h| allowed.contains(&h))
            && self.result_clause.is_none_or(|(h, _)| allowed.contains(&h))
            && self
                .trigger_context
                .as_ref()
                .is_none_or(|c| c.belief_safe(allowed))
            && self.outer_conts.iter().all(|c| {
                allowed.contains(&c.source_card)
                    && c.bindings.belief_safe(viewer, allowed)
                    && c.trigger_context
                        .as_ref()
                        .is_none_or(|t| t.belief_safe(allowed))
            })
    }
}

// Preserve the archival battle-only declaration projection. Breeding is a
// new declaration kind and includes its zone in every current digest.
impl std::fmt::Debug for EffectDigivolveCostChoiceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("EffectDigivolveCostChoiceState");
        d.field("acting_player", &self.acting_player);
        d.field("source_ref", &self.source_ref);
        d.field("result_card", &self.result_card);
        d.field("source_carrier_top", &self.source_carrier_top);
        d.field("source_carrier_identity", &self.source_carrier_identity);
        d.field("target_player", &self.target_player);
        if self.target_zone != Zone::BattleArea { d.field("target_zone", &self.target_zone); }
        d.field("target_top", &self.target_top);
        d.field("target_identity", &self.target_identity);
        d.field("dna_origin", &self.dna_origin);
        d.field("cost_delta", &self.cost_delta);
        d.field("waiver", &self.waiver);
        d.field("source", &self.source);
        d.field("printed_costs", &self.printed_costs);
        d.field("effect_source_player", &self.effect_source_player);
        d.field("trigger_context", &self.trigger_context);
        d.field("result_clause", &self.result_clause);
        d.field("outer_conts", &self.outer_conts);
        d.finish()
    }
}

/// An effect-owned declaration; distinct from the native action's single slot.
#[derive(Debug, Clone)]
pub(crate) struct EffectDigivolveScope {
    pub(crate) declaration: EffectDigivolveCostChoiceState,
    pub(crate) identity: PendingWouldDigivolveResume,
}

/// make-engine-cloneable: resumable-VM frame state for the player-scoped
/// digivolve cost-reducer prompts (`G-COST-REDUCE-ALLY-DIGIVOLVE`). Shared by
/// the accept/decline prompt frame and the suspend-cost frame it chains into.
/// All a player-digivolve action (never mid-DSL-clause) → no `outer_conts`.
#[derive(Debug, Clone)]
pub(crate) enum DigivolveReducerChoice {
    Player {
        index: usize,
        reducer: crate::player_cost_reducer::PlayerDigivolveCostReducer,
    },
    Field(CostReductionCandidate),
    /// One printed `<Digisorption -N>` instance of the hand card being
    /// digivolved into (§16-10-1). Its optional processing condition is
    /// "by suspending 1 of your Digimon" (§16-10-3); every instance is its
    /// own entry (§16-10-4).
    Digisorption {
        source_card: CardHandle,
        amount: i32,
    },
}

impl DigivolveReducerChoice {
    fn source_card(&self) -> CardHandle {
        match self {
            Self::Player { reducer, .. } => reducer.source_card,
            Self::Field(candidate) => candidate.key.source_card,
            Self::Digisorption { source_card, .. } => *source_card,
        }
    }

    /// The suspend-cost reduction, for the choices whose payment is
    /// "suspend 1 of your Digimon".
    fn suspend_reduction(&self) -> Option<i32> {
        match self {
            Self::Player { reducer, .. } if reducer.suspend_cost => Some(reducer.amount),
            Self::Digisorption { amount, .. } => Some(*amount),
            _ => None,
        }
    }

    fn keyword(&self) -> Option<Keyword> {
        match self {
            Self::Digisorption { amount, .. } => Some(Keyword::Digisorption(
                (*amount).clamp(0, u8::MAX as i32) as u8,
            )),
            _ => None,
        }
    }

    /// Two identical `<Digisorption>` instances: resolving either first is
    /// the same game, so no order decision is offered between them.
    fn same_digisorption(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (
                Self::Digisorption { source_card: a, amount: x },
                Self::Digisorption { source_card: b, amount: y },
            ) if a == b && x == y
        )
    }

    fn label(&self) -> String {
        match self {
            Self::Player { reducer, .. } => {
                format!("Suspend 1 Digimon (digivolution cost -{})", reducer.amount)
            }
            Self::Field(candidate) => format!("{} (-{})", candidate.label, candidate.amount),
            Self::Digisorption { amount, .. } => format!("<Digisorption -{amount}>"),
        }
    }

    fn optional(&self) -> bool {
        match self {
            Self::Player { .. } => true,
            Self::Field(candidate) => candidate.optional && !candidate.pay_cost_self_gated,
            // §16-10-3: the suspend is an optional processing condition.
            Self::Digisorption { .. } => true,
        }
    }
}

/// Entire before-payment chain, including field-hosted payment effects.
/// A parked field payment carries this same state in AfterSelectionHook;
/// selected effects are removed from `remaining` before any payment can park.
#[derive(Debug, Clone)]
pub(crate) struct DigivolveReducerState {
    pub current: Option<DigivolveReducerChoice>,
    pub acting_player: PlayerId,
    pub host_identity: u64,
    pub source: PlaySource,
    pub target: CostTargetContext,
    pub remaining: Vec<DigivolveReducerChoice>,
    pub reduction: i32,
    pub choosing_order: bool,
    /// Isolated affordability search stops before committing the evolution.
    pub affordability_probe: bool,
    /// Effect declarations retain their zone, selected route and DSL continuations.
    pub effect: Option<EffectDigivolveCostChoiceState>,
    /// DNA owns two material origins and resumes its merge, never ordinary evolution.
    pub dna_effect: Option<super::effect_dna::EffectDnaHandDeclaration>,
}

/// make-engine-cloneable: first material pick for a normal DNA digivolve from
/// hand. This mirrors the captures in `initiate_dna_digivolve`'s stage-1
/// closure; resolving it installs the second material prompt as another data
/// frame.
#[derive(Debug, Clone)]
pub struct DnaDigivolveFirstMaterialState {
    pub player_id: PlayerId,
    pub hand_index: usize,
    pub route_window: crate::dna_digivolve::DnaRouteWindow,
    pub source_card: CardHandle,
    pub previous_phase: GamePhase,
}

/// make-engine-cloneable: second material pick for a normal DNA digivolve from
/// hand. Resolving it calls the existing stage-2 resolver.
#[derive(Debug, Clone)]
pub struct DnaDigivolveSecondMaterialState {
    pub player_id: PlayerId,
    pub hand_index: usize,
    pub first_idx: usize,
    pub route_window: crate::dna_digivolve::DnaRouteWindow,
}

impl DigivolveReducerState {
    pub(crate) fn belief_safe(
        &self,
        viewer: PlayerId,
        allowed: &std::collections::HashSet<CardHandle>,
    ) -> bool {
        self.acting_player == viewer
            && self.target.belief_safe(allowed)
            && self
                .current
                .iter()
                .chain(&self.remaining)
                .all(|c| allowed.contains(&c.source_card()))
            && self
                .effect
                .as_ref()
                .is_none_or(|e| e.belief_safe(viewer, allowed))
            && self.dna_effect.is_none()
    }
    pub(crate) fn result_card(&self) -> CardHandle {
        self.target.card
    }
}

impl Game {
    /// Physical declaration owning the current selection, captured before its
    /// resolution can remove an inner scope. An inner reducer gate can have no
    /// payment hook yet, which must not assign its decline to an outer hook.
    pub(crate) fn digivolve_selection_owner(
        &self, stack: &crate::resume::ResumeStack,
    ) -> Option<CardHandle> {
        fn direct(stack: &crate::resume::ResumeStack) -> Option<CardHandle> {
            use crate::resume::ResumeFrame as F;
            match stack.frames.last()? {
                F::EffectDigivolveCostChoice(s) => Some(s.result_card),
                F::EffectDnaHandRoute(s) => Some(s.result),
                F::DigivolveReducerPrompt(s) | F::DigivolveReducerSuspend(s) => {
                    Some(s.result_card())
                }
                F::ReplacementBody { inner, .. }
                | F::FieldTopSourceBatch { inner, .. }
                | F::DelayCancelAfterSelection { inner, .. }
                | F::DelayHandDigivolveAfterSelection { inner, .. }
                | F::DelayDnaAfterSelection { inner, .. }
                | F::DelayFieldDnaAfterPayment { inner, .. }
                | F::DelayPlayFromHandAfterSelection { inner, .. }
                | F::DelayPlayFromUnionAfterSelection { inner, .. }
                | F::DelayBodyAfterCost { inner, .. } => direct(inner),
                _ => None,
            }
        }
        direct(stack)
            .or_else(|| crate::replacement::active_effect_digivolve_window_card(stack))
            .or_else(|| {
                self.effect_digivolve_scopes
                    .last()
                    .map(|s| s.declaration.result_card)
            })
            .or_else(|| self.pending_would_digivolve_resume.as_ref().map(|s| s.card))
            .or_else(|| {
                self.after_selection_resume_hooks
                    .0
                    .iter()
                    .rev()
                    .find_map(|hook| match hook {
                        crate::resume::AfterSelectionHook::DigivolveReducerChain {
                            state, ..
                        } => Some(state.result_card()),
                        _ => None,
                    })
            })
    }

    pub(crate) fn remove_effect_digivolve_scope(&mut self, card: CardHandle) {
        if let Some(index) = self
            .effect_digivolve_scopes
            .iter()
            .rposition(|s| s.declaration.result_card == card)
        {
            let scope = self.effect_digivolve_scopes.remove(index);
            self.clear_would_digivolve_identity(&scope.identity);
        }
    }
    pub(crate) fn abort_effect_digivolve_scopes(&mut self) {
        for scope in std::mem::take(&mut self.effect_digivolve_scopes) {
            self.clear_would_digivolve_identity(&scope.identity);
        }
    }
    pub(crate) fn effect_digivolve_window_card(
        &self,
        subject: crate::replacement::ReplacementSubject,
    ) -> Option<CardHandle> {
        let crate::replacement::ReplacementSubject::Permanent(h) = subject else {
            return None;
        };
        self.effect_digivolve_scopes
            .iter()
            .rev()
            .find(|s| {
                s.identity.window_started
                    && self.resolve_effect_digivolve_host(&s.declaration) == Some(h)
            })
            .map(|s| s.declaration.result_card)
    }
    pub(crate) fn commit_effect_would_digivolve(
        &mut self,
        card: CardHandle,
        outcome: crate::replacement::ReplacementOutcome,
        run_continuations: bool,
    ) -> bool {
        let Some(index) = self
            .effect_digivolve_scopes
            .iter()
            .rposition(|s| s.declaration.result_card == card)
        else {
            return false;
        };
        let scope = self.effect_digivolve_scopes.remove(index);
        let blocked = self
            .resolve_effect_digivolve_host(&scope.declaration)
            .is_some_and(|h| self.modifiers.has(h, ModifierType::CannotDigivolve));
        self.clear_would_digivolve_identity(&scope.identity);
        let previous_source = self.effect_source_player;
        let previous_trigger = self.current_trigger_context.clone();
        let previous_dna = self.current_dna_origin;
        self.effect_source_player = scope.declaration.effect_source_player;
        self.current_trigger_context = scope.declaration.trigger_context.clone();
        self.current_dna_origin = scope.declaration.dna_origin;
        let result = outcome == crate::replacement::ReplacementOutcome::None
            && !blocked
            && !self.game_over
            && self.commit_effect_digivolve_no_replace(
                &scope.declaration,
                scope.identity.effective_cost,
            );
        if run_continuations {
            self.finish_effect_digivolve_continuations(scope.declaration, result);
        }
        self.effect_source_player = previous_source;
        self.current_trigger_context = previous_trigger;
        self.current_dna_origin = previous_dna;
        result
    }
    fn resolve_effect_digivolve_choice_source(
        &self,
        state: &EffectDigivolveCostChoiceState,
    ) -> Option<CardSourceRef> {
        use CardSourceRef::*;
        let located = match state.source_ref {
            DeckTop(_) | PendingSecurity => state.source_ref,
            _ => self.locate_card_source_ref(state.result_card)?,
        };
        let same_zone = match (state.source_ref, located) {
            (Hand(a, _), Hand(b, _))
            | (Trash(a, _), Trash(b, _))
            | (Security(a, _), Security(b, _)) => a == b,
            (DeckTop(a), DeckTop(b)) => a == b,
            (PendingSecurity, PendingSecurity) | (Reveal(_), Reveal(_)) => true,
            (Material(a, _), Material(b, _)) | (Link(a, _), Link(b, _)) => {
                a.player == b.player
                    && self
                        .player(b.player)
                        .battle_area
                        .get(b.index as usize)
                        .and_then(|p| p.card_sources.last())
                        .map(CardSource::handle)
                        == state.source_carrier_top
                    && self
                        .player(b.player)
                        .battle_area
                        .get(b.index as usize)
                        .and_then(|p| p.battle_identity)
                        == state.source_carrier_identity
            }
            _ => false,
        };
        (same_zone
            && self
                .card_source_ref_peek(located)
                .is_some_and(|card| card.handle() == state.result_card))
        .then_some(located)
    }

    /// The active effect declaration can be parked directly or behind a field
    /// payment's DSL selection. The latter is owned by the chain hook.
    /// Attribute the initiating clause to its exact physical declaration even
    /// when a mandatory payment has already parked a nested declaration.
    pub(crate) fn bind_effect_digivolve_clause_owner(
        &mut self, result: CardHandle, clause: CardHandle, player: PlayerId,
    ) -> bool {
        fn bind(state: &mut EffectDigivolveCostChoiceState, result: CardHandle,
                clause: CardHandle, player: PlayerId) -> bool {
            if state.result_card != result { return false; }
            state.result_clause = Some((clause, player));
            state.effect_source_player = Some(player);
            true
        }
        fn frame(stack: &mut crate::resume::ResumeStack, result: CardHandle,
                 clause: CardHandle, player: PlayerId) -> bool {
            use crate::resume::ResumeFrame as F;
            let mut found = false;
            for item in &mut stack.frames {
                found |= match item {
                    F::EffectDigivolveCostChoice(s) => bind(s,result,clause,player),
                    F::DigivolveReducerPrompt(s) | F::DigivolveReducerSuspend(s) =>
                        s.effect.as_mut().is_some_and(|s| bind(s,result,clause,player)),
                    F::ReplacementBody { inner, .. } | F::FieldTopSourceBatch { inner, .. }
                    | F::DelayCancelAfterSelection { inner, .. } | F::DelayHandDigivolveAfterSelection { inner, .. }
                    | F::DelayDnaAfterSelection { inner, .. } | F::DelayFieldDnaAfterPayment { inner, .. }
                    | F::DelayPlayFromHandAfterSelection { inner, .. } | F::DelayPlayFromUnionAfterSelection { inner, .. }
                    | F::DelayBodyAfterCost { inner, .. } => frame(inner,result,clause,player),
                    _ => false,
                };
            }
            found
        }
        let mut found = self.pending_selection_resume.as_mut()
            .is_some_and(|s| frame(s,result,clause,player));
        for hook in &mut self.after_selection_resume_hooks.0 {
            if let crate::resume::AfterSelectionHook::DigivolveReducerChain { state, .. } = hook {
                found |= state.effect.as_mut().is_some_and(|s| bind(s,result,clause,player));
            }
        }
        for scope in &mut self.effect_digivolve_scopes {
            found |= bind(&mut scope.declaration,result,clause,player);
        }
        found
    }

    /// Find the parked declaration of this initiating clause, including an
    /// enclosing payment whose child declaration owns the visible prompt.
    pub(crate) fn effect_digivolve_declaration_for_clause_mut(
        &mut self, clause: CardHandle, player: PlayerId,
    ) -> Option<&mut EffectDigivolveCostChoiceState> {
        fn frame(stack: &mut crate::resume::ResumeStack, clause: CardHandle,
                 player: PlayerId) -> Option<&mut EffectDigivolveCostChoiceState> {
            use crate::resume::ResumeFrame as F;
            for item in stack.frames.iter_mut().rev() {
                let found = match item {
                    F::EffectDigivolveCostChoice(s)
                        if s.result_clause == Some((clause, player)) =>
                    {
                        Some(s)
                    }
                    F::DigivolveReducerPrompt(s) | F::DigivolveReducerSuspend(s) => s
                        .effect
                        .as_mut()
                        .filter(|s| s.result_clause == Some((clause, player))),
                    F::ReplacementBody { inner, .. }
                    | F::FieldTopSourceBatch { inner, .. }
                    | F::DelayCancelAfterSelection { inner, .. }
                    | F::DelayHandDigivolveAfterSelection { inner, .. }
                    | F::DelayDnaAfterSelection { inner, .. }
                    | F::DelayFieldDnaAfterPayment { inner, .. }
                    | F::DelayPlayFromHandAfterSelection { inner, .. }
                    | F::DelayPlayFromUnionAfterSelection { inner, .. }
                    | F::DelayBodyAfterCost { inner, .. } => frame(inner, clause, player),
                    _ => None,
                };
                if found.is_some() { return found; }
            }
            None
        }
        if let Some(s) = self.pending_selection_resume.as_mut()
            .and_then(|s| frame(s,clause,player)) { return Some(s); }
        for hook in self.after_selection_resume_hooks.0.iter_mut().rev() {
            if let crate::resume::AfterSelectionHook::DigivolveReducerChain { state, .. } = hook {
                if state.effect.as_ref().is_some_and(|s| s.result_clause == Some((clause,player))) {
                    return state.effect.as_mut();
                }
            }
        }
        self.effect_digivolve_scopes.iter_mut().rev()
            .find(|s| s.declaration.result_clause == Some((clause,player)))
            .map(|s| &mut s.declaration)
    }

    pub(crate) fn pending_effect_digivolve_declaration_mut(
        &mut self,
    ) -> Option<&mut EffectDigivolveCostChoiceState> {
        let active_card = self
            .pending_selection_resume
            .as_ref()
            .and_then(crate::replacement::active_effect_digivolve_window_card);
        // A field payment can itself start an effect evolution. Its current
        // frame is more deeply nested than the parked outer payment hook.
        if let Some(frame) = self
            .pending_selection_resume
            .as_mut()
            .and_then(|s| s.frames.last_mut())
        {
            match frame {
                crate::resume::ResumeFrame::EffectDigivolveCostChoice(state) => return Some(state),
                crate::resume::ResumeFrame::DigivolveReducerPrompt(state)
                | crate::resume::ResumeFrame::DigivolveReducerSuspend(state)
                    if state.effect.is_some() =>
                {
                    return state.effect.as_mut();
                }
                _ => {}
            }
        }
        // A would-evolve decision belongs to the nested declaration that
        // opened it, before any enclosing reducer's parked continuation.
        if let Some(index) = active_card.and_then(|card| {
            self.effect_digivolve_scopes.iter().rposition(|scope| {
                scope.declaration.result_card == card && scope.identity.window_started
            })
        }) {
            return Some(&mut self.effect_digivolve_scopes[index].declaration);
        }
        for hook in self.after_selection_resume_hooks.0.iter_mut().rev() {
            if let crate::resume::AfterSelectionHook::DigivolveReducerChain { state, .. } = hook {
                if state.effect.is_some() {
                    return state.effect.as_mut();
                }
            }
        }
        self.effect_digivolve_scopes
            .iter_mut()
            .rev()
            .find(|scope| scope.identity.window_started)
            .map(|scope| &mut scope.declaration)
    }

    fn resolve_effect_digivolve_declaration(
        &self,
        state: &EffectDigivolveCostChoiceState,
    ) -> Option<(CardSourceRef, PermanentHandle)> {
        let target = self.resolve_effect_digivolve_host(state)?;
        if self.digivolve_base_permanent(target)?
            .top_card()
            .handle()
            != state.target_top
        {
            return None;
        }
        Some((self.resolve_effect_digivolve_choice_source(state)?, target))
    }

    fn effect_digivolve_target_zone(target: PermanentHandle) -> Zone {
        if target.index == crate::action::space::BREEDING_TARGET as u8 {
            Zone::BreedingArea
        } else {
            Zone::BattleArea
        }
    }

    fn resolve_effect_digivolve_host(
        &self, state: &EffectDigivolveCostChoiceState,
    ) -> Option<PermanentHandle> {
        let index = match state.target_zone {
            Zone::BreedingArea => crate::action::space::BREEDING_TARGET as u8,
            Zone::BattleArea => 0,
            _ => return None,
        };
        self.resolve_source_play_host_identity(
            PermanentHandle { player: state.target_player, index }, state.target_identity,
        )
    }

    fn finish_effect_digivolve_continuations(
        &mut self,
        mut state: EffectDigivolveCostChoiceState,
        success: bool,
    ) {
        // A route selection can transition into a reducer/payment selection.
        // Transfer ownership, rather than treating an asynchronous false as failure.
        if !success {
            if let Some(parked) = self.pending_effect_digivolve_declaration_mut() {
                if parked.result_card == state.result_card {
                    parked.result_clause = state.result_clause;
                    parked.outer_conts.append(&mut state.outer_conts);
                    return;
                }
            }
        }
        if success {
            if let Some(result) = self.resolve_effect_digivolve_host(&state).filter(|h| {
                self.digivolve_base_permanent(*h).is_some_and(|p| {
                    p.card_sources.last().is_some_and(|c| c.handle() == state.result_card)
                })
            })
            {
                for cont in &mut state.outer_conts {
                    if state.result_clause == Some((cont.source_card, cont.player)) {
                        cont.bindings.record_digivolved(result);
                    }
                }
            }
        }
        self.remove_effect_digivolve_scope(state.result_card);
        crate::dsl_cards::step::selections::run_outer_conts(self, state.outer_conts);
    }

    fn resume_effect_digivolve_declaration(
        &mut self,
        state: EffectDigivolveCostChoiceState,
        chosen: Option<crate::dna_digivolve::DigivolveRouteMatch>,
        reducers_resolved: bool,
    ) -> bool {
        self.remove_effect_digivolve_scope(state.result_card);
        let previous_source = self.effect_source_player;
        let previous_trigger = self.current_trigger_context.clone();
        let previous_dna = self.current_dna_origin;
        self.effect_source_player = state.effect_source_player;
        self.current_trigger_context = state.trigger_context.clone();
        self.current_dna_origin = state.dna_origin;
        let success = if let (Some(cost), Some((source_ref, target))) =
            (chosen, self.resolve_effect_digivolve_declaration(&state))
        {
            self.effect_initiated_digivolve_with_chosen_cost_inner(
                state.acting_player,
                source_ref,
                target,
                state.cost_delta,
                state.waiver,
                state.source,
                Some(cost),
                reducers_resolved,
            )
        } else {
            self.pending_player_digivolve_reduction = 0;
            self.pending_interactive_digivolve_reduction = 0;
            self.pending_cost_reduction_amount_override = None;
            false
        };
        if !success && self.pending_selection.is_none() {
            self.pending_player_digivolve_reduction = 0;
            self.pending_interactive_digivolve_reduction = 0;
            self.pending_cost_reduction_amount_override = None;
        }
        self.finish_effect_digivolve_continuations(state, success);
        self.effect_source_player = previous_source;
        self.current_trigger_context = previous_trigger;
        self.current_dna_origin = previous_dna;
        success
    }

    pub(crate) fn run_effect_digivolve_cost_choice_step(
        &mut self,
        state: EffectDigivolveCostChoiceState,
        action_id: u16,
    ) {
        let chosen = action_id
            .checked_sub(crate::action::space::HAND_EFFECT_START)
            .and_then(|i| state.printed_costs.get(i as usize))
            .copied();
        self.resume_effect_digivolve_declaration(state, chosen, false);
    }

    fn install_effect_digivolve_cost_choice(
        &mut self,
        state: EffectDigivolveCostChoiceState,
        delta: CostDelta,
    ) {
        let source_card = state.result_card;
        let entries: Vec<EffectChoiceEntry> = state
            .printed_costs
            .iter()
            .enumerate()
            .map(|(i, cost)| EffectChoiceEntry {
                count_choice: None,
                label: format!(
                    "Digivolve for cost {}{}",
                    delta.resolve(cost.memory_cost),
                    if cost.treated_as.is_some() {
                        " (treated as Digimon)"
                    } else {
                        ""
                    }
                ),
                action_id: crate::action::space::HAND_EFFECT_START + i as u16,
                source_card: Some(source_card),
                source_kind: Some(EffectSourceKind::Digimon),
                timing: None,
                is_optional: false,
                keyword: None,
                observation_metadata: Default::default(),
            })
            .collect();
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::EffectChoice;
        let callback_state = state.clone();
        self.pending_selection = Some(PendingSelection {
            card_choice: None,
            zone_owner: None,
            kind: SelectionKind::EffectChoice,
            selecting_player: state.acting_player,
            previous_phase,
            valid_action_ids: entries.iter().map(|entry| entry.action_id).collect(),
            is_optional: false,
            prompt: "Choose which digivolution cost to pay".into(),
            effect_choices: Some(entries),
            source_card,
            source_permanent: None,
            source_kind: EffectSourceKind::Digimon,
            callback: Box::new(move |game, action| {
                game.run_effect_digivolve_cost_choice_step(callback_state, action)
            }),
            on_decline: None,
        });
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::EffectDigivolveCostChoice(state)],
        });
    }

    /// Resolve a parked digivolve cost-choice (resumable VM). Mirrors
    /// `install_digivolve_cost_choice_prompt`'s callback exactly: decode the
    /// chosen route index, pin it as `pending_digivolve_route_choice`, then
    /// re-enter the digivolve.
    pub(crate) fn run_digivolve_cost_choice_step(
        &mut self,
        state: DigivolveCostChoiceState,
        action_id: u16,
    ) {
        let idx = action_id.saturating_sub(crate::action::space::HAND_EFFECT_START) as usize;
        if let Some(chosen) = state.routes.get(idx).copied() {
            self.pending_digivolve_route_choice = Some(chosen);
        }
        // `field_index == BREEDING_TARGET` is the breeding-area sentinel: the
        // cost choice was installed by `digivolve_from_hand_onto_breeding`
        // and must re-enter it (the battle-area path rejects the sentinel).
        if state.field_index == crate::action::space::BREEDING_TARGET as usize {
            self.digivolve_from_hand_onto_breeding(
                state.acting_player,
                state.hand_index,
                state.source,
            );
            return;
        }
        self.digivolve_from_hand_inner(
            state.acting_player,
            state.hand_index,
            state.field_index,
            state.source,
            false,
        );
    }

    /// Resolve ordering or one effect's optional gate without losing other
    /// effects triggered by this same digivolution.
    pub(crate) fn run_digivolve_reducer_prompt_step(
        &mut self,
        mut state: DigivolveReducerState,
        action_id: u16,
        is_pass: bool,
    ) {
        if state.choosing_order {
            let index = action_id.saturating_sub(crate::action::space::HAND_EFFECT_START) as usize;
            if index >= state.remaining.len() {
                return;
            }
            state.current = Some(state.remaining.remove(index));
            state.choosing_order = false;
            self.install_player_digivolve_reducer_prompt(state);
        } else if is_pass {
            if let Some(DigivolveReducerChoice::Player { index, reducer }) = &state.current {
                self.consume_player_digivolve_reducer(*index, reducer, reducer.single_fire);
            }
            self.resume_player_digivolve_reducers(state);
        } else {
            self.player_digivolve_reducer_accept(state);
        }
    }

    pub(crate) fn run_digivolve_reducer_suspend_step(
        &mut self,
        mut state: DigivolveReducerState,
        action_id: u16,
    ) {
        use crate::action::space::{ATTACK_START, TARGETS_PER_ATTACKER};
        let Some(current) = state.current.clone() else {
            return;
        };
        let Some(amount) = current.suspend_reduction() else {
            return;
        };
        let index = (action_id.saturating_sub(ATTACK_START) % TARGETS_PER_ATTACKER) as usize;
        if self
            .suspendable_own_digimon(state.acting_player)
            .contains(&index)
        {
            // §16-10-3: once the Digimon is suspended, the reduction is
            // mandatory (likewise for the player-scoped suspend reducers).
            self.suspend(PermanentHandle {
                player: state.acting_player,
                index: index as u8,
            });
            state.reduction += amount;
        }
        if let DigivolveReducerChoice::Player {
            index: reducer_index,
            reducer,
        } = &current
        {
            self.consume_player_digivolve_reducer(*reducer_index, reducer, reducer.single_fire);
        }
        self.resume_player_digivolve_reducers(state);
    }

    /// Resolve a parked normal DNA-digivolve first-material prompt. Mirrors
    /// `initiate_dna_digivolve`'s stage-1 callback: validate the first material,
    /// derive valid second-material targets, install that second prompt, and
    /// keep the phase in `SelectMaterial`.
    pub(crate) fn run_dna_digivolve_first_material_step(
        &mut self,
        state: DnaDigivolveFirstMaterialState,
        action_id: u16,
    ) {
        let first_idx = action_id as usize;

        if first_idx >= self.player(state.player_id).battle_area.len() {
            self.logger.log(&format!(
                "[Rejected] dna_digivolve stage 1: first index {} out of range (battle_area size={})",
                first_idx,
                self.player(state.player_id).battle_area.len()
            ));
            return;
        }
        if state.hand_index >= self.player(state.player_id).hand.len() {
            self.logger.log(&format!(
                "[Rejected] dna_digivolve stage 1: evo hand index {} out of range (hand size={})",
                state.hand_index,
                self.player(state.player_id).hand.len()
            ));
            return;
        }

        let second_targets: Vec<u16> = self
            .valid_dna_second_targets_for_hand_card(
                state.player_id,
                state.hand_index,
                first_idx,
                state.route_window,
            )
            .collect();
        if second_targets.is_empty() {
            self.logger.log(&format!(
                "[Rejected] dna_digivolve stage 1: no valid second-material targets for first index {}",
                first_idx
            ));
            return;
        }

        self.pending_selection = Some(PendingSelection {
            card_choice: None,
            zone_owner: None,
            kind: SelectionKind::Material,
            selecting_player: state.player_id,
            previous_phase: state.previous_phase,
            valid_action_ids: second_targets,
            is_optional: false,
            prompt: crate::selection::DNA_SECOND_MATERIAL_PROMPT.to_string(),
            effect_choices: None,
            source_card: state.source_card,
            source_permanent: None,
            source_kind: EffectSourceKind::Digimon,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let second_idx = action_id as usize;
                game.resolve_dna_digivolve_stage2_with_window(
                    state.player_id,
                    first_idx,
                    second_idx,
                    state.hand_index,
                    state.route_window,
                );
            }),
            on_decline: None,
        });
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::DnaDigivolveSecondMaterial(
                DnaDigivolveSecondMaterialState {
                    player_id: state.player_id,
                    hand_index: state.hand_index,
                    first_idx,
                    route_window: state.route_window,
                },
            )],
        });
        self.current_phase = GamePhase::SelectMaterial;
    }

    /// Resolve a parked normal DNA-digivolve second-material prompt. Mirrors
    /// the stage-2 callback by forwarding to the shared resolver.
    pub(crate) fn run_dna_digivolve_second_material_step(
        &mut self,
        state: DnaDigivolveSecondMaterialState,
        action_id: u16,
    ) {
        let second_idx = action_id as usize;
        self.resolve_dna_digivolve_stage2_with_window(
            state.player_id,
            state.first_idx,
            second_idx,
            state.hand_index,
            state.route_window,
        );
    }
}

impl Game {
    /// Digivolve: push a card onto a permanent's stack.
    pub fn digivolve_onto(
        &mut self,
        player_id: PlayerId,
        field_index: usize,
        card: CardSource,
    ) -> bool {
        let turn = self.turn_count;
        let player = self.player_mut(player_id);
        if field_index >= player.battle_area.len() {
            return false;
        }
        player.battle_area[field_index].digivolve(card, turn);
        self.emit_eval_digivolve(PermanentHandle { player: player_id, index: field_index as u8 }, 0, false, false);
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
        true
    }

    /// Low-level source-attribution helper for tests and engine internals.
    ///
    /// Uses the standard De-Digivolve floor (`stop_at_level = Some(3)`) and
    /// returns whether at least one card was popped. Replacement windows are
    /// resolved by `EffectContext::de_digivolve` under the supplied source
    /// attribution. Production card effects should prefer an existing
    /// `EffectContext` so `can_affect_permanent` and source-kind metadata come
    /// from the real resolving card.
    #[doc(hidden)]
    pub fn de_digivolve_from_effect(
        &mut self,
        handle: PermanentHandle,
        effect_player: PlayerId,
        amount: u8,
    ) -> bool {
        let previous = self.effect_source_player;
        self.effect_source_player = Some(effect_player);
        let popped = {
            let mut ctx =
                EffectContext::new(self, crate::card_source::CardHandle(0), None, effect_player);
            ctx.de_digivolve(handle, Some(3), Some(amount))
        };
        self.effect_source_player = previous;
        popped > 0
    }

    /// Full "digivolve from hand" action — Python parity for
    /// `action_digivolve(field_idx, hand_idx)`. Validates phase, indices,
    /// `CannotDigivolve` modifier, and evo-cost fit; pays memory; removes
    /// the card from hand; stacks it onto the target permanent; draws 1;
    /// fires `WhenDigivolving` triggers and drains the effect queue;
    /// finally calls `check_turn_end`.
    ///
    /// Deferred (see RUST_PYTHON_PARITY.md):
    /// - Cost reductions (`WhenWouldDigivolve`, `CHANGE_DIGIVOLUTION_COST`)
    /// - `digivolve_observer` mechanism (no Rust equivalent yet)
    /// - Contextual modifier predicates (`{'digivolving_card': card}`)
    pub fn digivolve_from_hand(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        field_index: usize,
        source: PlaySource,
    ) -> bool {
        if self.game_over {
            return false;
        }
        // Fresh user-initiated attempt: drop any stale cost-choice. The
        // cost-choice `EffectChoice` callback re-enters `digivolve_from_hand_inner`
        // DIRECTLY (not through here), so the player's pick is preserved across
        // the reducer / replacement re-entries.
        self.pending_digivolve_route_choice = None;
        self.digivolve_from_hand_inner(player_id, hand_index, field_index, source, false)
    }

    /// As [`Self::digivolve_from_hand`], but threads a `player_reducer_resolved`
    /// flag (`G-COST-REDUCE-ALLY-DIGIVOLVE`). When `false` (the default user
    /// call), the function first consults `Game::player_digivolve_cost_reducers`
    /// and — if a reducer qualifies and is payable — installs an interactive
    /// accept/decline prompt, returning `false` without performing the
    /// digivolution; the accept/decline callbacks re-invoke this function with
    /// the flag `true` and a pre-resolved `pending_player_digivolve_reduction`.
    /// When `true`, the player-scoped reducer prompt is skipped (it has already
    /// been resolved this attempt).
    pub(crate) fn digivolve_from_hand_inner(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        field_index: usize,
        source: PlaySource,
        player_reducer_resolved: bool,
    ) -> bool {
        // A parked cost re-enters against the printed source to validate the
        // selected route again, then reinstalls its temporary identity before
        // further cost effects. Do not leave an overlay after any failed exit.
        self.abort_would_digivolve_declaration();
        let result = self.digivolve_from_hand_inner_body(
            player_id,
            hand_index,
            field_index,
            source,
            player_reducer_resolved,
        );
        if self.pending_selection.is_none() {
            self.abort_would_digivolve_declaration();
        }
        result
    }

    fn digivolve_from_hand_inner_body(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        field_index: usize,
        source: PlaySource,
        player_reducer_resolved: bool,
    ) -> bool {
        if self.current_phase != GamePhase::Main {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: not in Main phase (phase={:?})",
                self.current_phase
            ));
            return false;
        }
        let player = self.player(player_id);
        if hand_index >= player.hand.len() {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: hand index {} out of range (hand size={})",
                hand_index,
                player.hand.len()
            ));
            return false;
        }
        if field_index >= player.battle_area.len() {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: field index {} out of range (battle_area size={})",
                field_index,
                player.battle_area.len()
            ));
            return false;
        }
        let handle = PermanentHandle {
            player: player_id,
            index: field_index as u8,
        };
        if self.modifiers.has(handle, ModifierType::CannotDigivolve) {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: permanent at field index {} blocked by CannotDigivolve modifier",
                field_index
            ));
            return false;
        }

        let card = player.hand[hand_index].clone();
        let perm = &player.battle_area[field_index];
        // Validity gate (cheapest route). The action mask and the Blast counter
        // path consult the same `normal_digivolve_route_for_hand_card`.
        let Some(min_route) =
            self.normal_digivolve_route_for_hand_card(player_id, hand_index, handle)
        else {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: card {} cannot digivolve onto {} (evo-cost mismatch)",
                card.card_id(&self.card_data),
                perm.top_card().card_id(&self.card_data),
            ));
            return false;
        };

        // Rule 17 — when the base satisfies MORE THAN ONE distinct-cost route
        // (e.g. BT16-040 Wormmon over Minomon: "[Minomon]: Cost 0" vs "any Lv.2:
        // Cost 1") and the player has not yet chosen, prompt for WHICH cost to
        // pay rather than auto-selecting the cheapest. The cost-choice callback
        // re-enters this function with `pending_digivolve_route_choice` set. DCGO
        // registers each requirement with its own `digivolutionCost`
        // (`AddSelfDigivolutionRequirementStaticEffect`).
        if self.pending_digivolve_route_choice.is_none() {
            let routes =
                self.distinct_digivolve_routes_for_hand_card(player_id, hand_index, handle);
            if routes.len() > 1 {
                self.install_digivolve_cost_choice_prompt(
                    player_id,
                    hand_index,
                    field_index,
                    source,
                    routes,
                );
                return false;
            }
        }
        // The route the player chose (peeked here; the choice field is consumed
        // when the resume is built below) or, when no choice was needed, the
        // cheapest. `route` carries both the printed cost and app-fusion-ness.
        let route = self.pending_digivolve_route_choice.unwrap_or(min_route);
        let printed_cost = route.memory_cost;
        let host_identity = self.capture_digivolve_host_identity(handle);
        let treated_identity_orders = route
            .treated_as
            .and_then(|profile| self.install_would_digivolve_identity(handle, profile));
        if treated_identity_orders.is_some() {
            self.pending_would_digivolve_resume = Some(PendingWouldDigivolveResume {
                host_identity,
                treated_identity_orders,
                window_started: false,
                player: player_id,
                permanent: handle,
                card: card.handle(),
                effective_cost: 0,
                app_fusion: route.app_fusion,
            });
        }

        // G-COST-REDUCE-ALLY-DIGIVOLVE — player-scoped one-shot future-digivolve
        // cost reducer prompt. Runs BEFORE the synchronous field-hosted
        // BeforePayCost scan; if a reducer qualifies and is payable, this
        // installs an interactive accept/decline PendingSelection and returns
        // — the accept/decline callbacks re-enter `digivolve_from_hand_inner`
        // with `player_reducer_resolved = true`.
        if !player_reducer_resolved {
            if let Some(result) =
                self.try_prompt_player_digivolve_cost_reducer(player_id, handle, hand_index, source)
            {
                return result;
            }
        }
        // Pre-resolved player-scoped reduction (set by the accept callback,
        // 0 on decline). Consumed once here.
        let player_reduction = std::mem::take(&mut self.pending_player_digivolve_reduction);

        // Pass the hand card being digivolved into as the cost-target so
        // target-aware predicates (`cost_target: { trait_has: Free }`)
        // can fire — G-BEFORE-PAY-COST-DIGIVOLVE-TARGET. `handle` is the
        // permanent being digivolved into; threaded as a target permanent
        // so `source_is_cost_target_permanent` gates self-targeted
        // observers.
        let target = CostTargetContext {
            card: card.handle(),
            from_hand: true,
            is_digivolve: true,
            target_permanents: [Some(handle), None],
        };

        // Pre-resolved interactive field reduction (set by the prompt's parked
        // pay_cost success continuation, 0 otherwise). Consumed once here.
        let interactive_reduction =
            std::mem::take(&mut self.pending_interactive_digivolve_reduction);

        let total_reduction = self.scan_before_pay_cost_reduction_with_target(
            player_id,
            CostReductionKind::Digivolve,
            Some(target),
        ) + player_reduction
            + interactive_reduction;
        let Some(handle) = self.resolve_digivolve_host_identity(player_id, host_identity) else {
            self.clear_digivolve_reducer_chain_credit();
            return false;
        };
        if !self
            .player(player_id)
            .hand
            .iter()
            .any(|hand_card| hand_card.handle() == target.card)
        {
            self.clear_digivolve_reducer_chain_credit();
            return false;
        }
        let target = CostTargetContext {
            target_permanents: [Some(handle), None],
            ..target
        };
        // Fire BeforePayCost observers (e.g. gain_memory) AFTER reduction
        // is computed but BEFORE pay_memory — G-BEFORE-PAY-COST-GAIN-MEMORY.
        self.scan_before_pay_cost_observers(player_id, Some(target));
        let Some(handle) = self.resolve_digivolve_host_identity(player_id, host_identity) else {
            self.clear_digivolve_reducer_chain_credit();
            return false;
        };
        if !self
            .player(player_id)
            .hand
            .iter()
            .any(|hand_card| hand_card.handle() == target.card)
        {
            self.clear_digivolve_reducer_chain_credit();
            return false;
        }
        let effective_cost = (printed_cost as i32 - total_reduction).max(0) as u16;

        // Consume the cost choice now — past this point the resume carries the
        // route's app-fusion-ness, so the choice field is no longer needed and
        // must be cleared before the replacement resume / next attempt.
        self.pending_digivolve_route_choice = None;
        if self.game_over {
            return false;
        }
        let resume =
            self.pending_would_digivolve_resume
                .get_or_insert(PendingWouldDigivolveResume {
                    host_identity,
                    treated_identity_orders: None,
                    window_started: false,
                    player: player_id,
                    permanent: handle,
                    card: card.handle(),
                    effective_cost,
                    app_fusion: route.app_fusion,
                });
        resume.permanent = handle;
        resume.effective_cost = effective_cost;
        resume.window_started = true;
        let cause = match source {
            PlaySource::ByEffect => crate::replacement::ReplacementCause::OwnEffect,
            PlaySource::ByHand | PlaySource::ByDigivolve => {
                crate::replacement::ReplacementCause::OwnEffect
            }
        };
        let outcome = self.try_replace(
            EffectTiming::WhenPermanentWouldDigivolve,
            crate::replacement::ReplacementSubject::Permanent(handle),
            cause,
            Some(Zone::BattleArea),
        );
        if self.pending_selection.is_some() {
            return false;
        }
        let Some(resume) = self.pending_would_digivolve_resume.take() else {
            return false;
        };
        self.clear_would_digivolve_identity(&resume);
        match outcome {
            crate::replacement::ReplacementOutcome::None => {
                self.commit_digivolve_from_hand_no_replace(resume)
            }
            crate::replacement::ReplacementOutcome::Cancelled
            | crate::replacement::ReplacementOutcome::CustomHandled
            | crate::replacement::ReplacementOutcome::Redirected(_)
            | crate::replacement::ReplacementOutcome::Substituted(_) => false,
        }
    }

    /// Install the `EffectChoice` "which digivolution cost do you pay?" prompt
    /// (rule 17 — no auto-selection). One option per distinct route (sorted
    /// cheapest-first), action IDs `HAND_EFFECT_START + i`. The single callback
    /// maps the chosen action id back to its route, stores it in
    /// `pending_digivolve_route_choice`, and re-enters
    /// `digivolve_from_hand_inner` (reducer NOT yet resolved → `false`) so the
    /// digivolution proceeds at the chosen base cost. Mandatory (no PASS): the
    /// player already committed to the digivolve action by reaching here.
    pub(crate) fn install_digivolve_cost_choice_prompt(
        &mut self,
        acting_player: PlayerId,
        hand_index: usize,
        field_index: usize,
        source: PlaySource,
        routes: Vec<crate::dna_digivolve::DigivolveRouteMatch>,
    ) {
        use crate::action::space::HAND_EFFECT_START;
        use crate::selection::{EffectChoiceEntry, PendingSelection, SelectionKind};

        // Cheapest-first, stable so the option order is deterministic.
        let mut routes = routes;
        routes.sort_by_key(|r| (r.memory_cost, r.app_fusion));

        let source_card = self
            .player(acting_player)
            .hand
            .get(hand_index)
            .map(|c| c.handle())
            .unwrap_or(crate::card_source::CardHandle(0));
        let source_kind = EffectSourceKind::Digimon;

        let entries: Vec<EffectChoiceEntry> = routes
            .iter()
            .enumerate()
            .map(|(i, r)| EffectChoiceEntry {
                count_choice: None,
                label: if r.app_fusion {
                    format!("Digivolve for cost {} (App Fusion)", r.memory_cost)
                } else if let Some(profile) = r.treated_as {
                    format!(
                        "Digivolve for cost {} (treat base as Lv.{} {:?} {:?})",
                        r.memory_cost,
                        profile.level,
                        profile.colors(),
                        profile.kind
                    )
                } else {
                    format!("Digivolve for cost {}", r.memory_cost)
                },
                action_id: HAND_EFFECT_START + i as u16,
                source_card: Some(source_card),
                source_kind: Some(source_kind),
                timing: None,
                is_optional: false,
                keyword: None,
                observation_metadata: Default::default(),
            })
            .collect();
        let valid_action_ids: Vec<u16> = entries.iter().map(|e| e.action_id).collect();

        // make-engine-cloneable: capture a copy of the routes for the data frame
        // before the closure moves them (coexistence: both paths install).
        let routes_for_resume = routes.clone();

        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::EffectChoice;
        self.pending_selection = Some(PendingSelection {
            card_choice: None,
            zone_owner: None,
            kind: SelectionKind::EffectChoice,
            selecting_player: acting_player,
            previous_phase,
            valid_action_ids,
            is_optional: false,
            prompt: "Choose which digivolution cost to pay".to_string(),
            effect_choices: Some(entries),
            source_card,
            source_permanent: None,
            source_kind,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let idx = action_id.saturating_sub(HAND_EFFECT_START) as usize;
                if let Some(chosen) = routes.get(idx).copied() {
                    game.pending_digivolve_route_choice = Some(chosen);
                }
                // BREEDING_TARGET field-index sentinel → the choice belongs
                // to a breeding-area digivolve; re-enter that path.
                if field_index == crate::action::space::BREEDING_TARGET as usize {
                    game.digivolve_from_hand_onto_breeding(acting_player, hand_index, source);
                    return;
                }
                game.digivolve_from_hand_inner(
                    acting_player,
                    hand_index,
                    field_index,
                    source,
                    false,
                );
            }),
            on_decline: None,
        });
        // Park the data frame alongside the closure. The cost-choice resolves via
        // `run_digivolve_cost_choice_step` when driven through the resumable VM.
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::DigivolveCostChoice(
                DigivolveCostChoiceState {
                    acting_player,
                    hand_index,
                    field_index,
                    source,
                    routes: routes_for_resume,
                },
            )],
        });
    }

    /// Re-check remaining payment preconditions after every resolution. All
    /// player one-shots expire on the event; field effects retain ordinary OPT
    /// accounting and are never selected twice from this snapshot.
    pub(crate) fn advance_player_digivolve_reducers(
        &mut self,
        mut state: DigivolveReducerState,
    ) -> bool {
        let Some((_, field)) = self.resolve_digivolve_reducer_target(&state) else {
            if let Some(effect) = &state.effect {
                // Failure belongs to this effect declaration, not an enclosing
                // native declaration waiting for its payment to finish.
                self.remove_effect_digivolve_scope(effect.result_card);
            } else if state.dna_effect.is_none() {
                self.clear_digivolve_reducer_chain_credit();
            }
            return false;
        };
        state.target.target_permanents = [
            Some(PermanentHandle {
                player: state.acting_player,
                index: field as u8,
            }),
            None,
        ];
        let prohibited = self
            .modifiers
            .player_has(state.acting_player, ModifierType::CannotReduceCost)
            || self
                .modifiers
                .player_has(state.acting_player, ModifierType::CannotReduceDigivolveCost)
            || self.modifiers.any_other_player_has(
                state.acting_player,
                ModifierType::OpponentCannotReduceDigivolveCost,
            );
        let payable = !self.suspendable_own_digimon(state.acting_player).is_empty();
        let live_fields = self.collect_before_pay_cost_reducers(
            state.acting_player,
            Some(state.target),
            &[],
            CostReductionKind::Digivolve,
        );
        let remaining = std::mem::take(&mut state.remaining);
        for choice in remaining {
            match choice {
                DigivolveReducerChoice::Player { index, reducer } => {
                    if prohibited || (reducer.suspend_cost && !payable) {
                        self.consume_player_digivolve_reducer(index, &reducer, reducer.single_fire);
                    } else if !reducer.suspend_cost {
                        state.reduction += reducer.amount;
                        self.consume_player_digivolve_reducer(index, &reducer, reducer.single_fire);
                    } else {
                        state
                            .remaining
                            .push(DigivolveReducerChoice::Player { index, reducer });
                    }
                }
                // §16-10-1/-3: offered only while a Digimon can be suspended
                // to pay its condition; a prohibited reduction offers nothing.
                DigivolveReducerChoice::Digisorption { .. } => {
                    if !prohibited && payable {
                        state.remaining.push(choice);
                    }
                }
                DigivolveReducerChoice::Field(candidate) => {
                    // Resolve by physical source, not an index shifted by an
                    // earlier payment deleting another battle-area permanent.
                    if let Some(live) = live_fields.iter().find(|live| {
                        live.key.source_card == candidate.key.source_card
                            && live.key.controller == candidate.key.controller
                            && live.key.effect_slot == candidate.key.effect_slot
                            && live.key.is_under == candidate.key.is_under
                    }) {
                        if self.cost_reducer_pay_cost_payable(&live.key) {
                            state
                                .remaining
                                .push(DigivolveReducerChoice::Field(live.clone()));
                        }
                    }
                }
            }
        }
        if prohibited {
            state.reduction = 0;
        }
        if state.remaining.is_empty() {
            self.pending_player_digivolve_reduction = state.reduction;
            return false;
        }
        state.choosing_order = state.remaining.len() > 1
            && !state
                .remaining
                .iter()
                .all(|choice| choice.same_digisorption(&state.remaining[0]));
        if !state.choosing_order {
            state.current = Some(state.remaining.remove(0));
            if let Some(DigivolveReducerChoice::Field(candidate)) = state.current.clone() {
                if !state.current.as_ref().unwrap().optional() {
                    return match self.run_digivolve_reducer_chain_payment(state, candidate) {
                        Some(next) => self.advance_player_digivolve_reducers(next),
                        None => true,
                    };
                }
            }
        }
        self.install_player_digivolve_reducer_prompt(state);
        true
    }

    /// Security removal has its own replacement/event continuation, whose
    /// digivolve destination currently supports battle-area hosts only.
    pub(crate) fn effect_digivolve_source_supported(
        &self,
        source_ref: CardSourceRef,
        target: PermanentHandle,
    ) -> bool {
        target.index != crate::action::space::BREEDING_TARGET as u8
            || !matches!(source_ref, CardSourceRef::Security(_, _))
    }

    /// Read-only candidate legality for an effect that pays the specified
    /// cost. Existing route-only predicates intentionally do not call this:
    /// their consuming effects may waive or reduce the printed cost.
    pub(crate) fn can_effect_digivolve_from_source_with_cost(
        &self,
        acting_player: PlayerId,
        source_ref: CardSourceRef,
        target: PermanentHandle,
        cost_delta: CostDelta,
    ) -> bool {
        if !self.effect_digivolve_source_supported(source_ref, target) {
            return false;
        }
        let Some(host) = self.digivolve_base_permanent(target) else {
            return false;
        };
        if self.modifiers.has(target, ModifierType::CannotDigivolve)
            || self.modifiers.player_has(acting_player, ModifierType::CannotDigivolveDigimonByEffect)
        {
            return false;
        }
        let Some(card) = self.card_source_ref_peek(source_ref) else { return false; };
        let result_card = card.handle();
        let target_top = host.top_card().handle();
        let routes: Vec<_> = self.all_digivolve_routes_for_card(card, target).into_iter()
            .filter(|route| !route.app_fusion).collect();
        if routes.is_empty() { return false; }

        let mut probe = self.clone();
        // An installed caller choice is not part of this payment search.
        // Keep physical state/provenance, but isolate the probe's own frames.
        probe.pending_selection = None;
        probe.pending_selection_resume = None;
        probe.after_selection_resume_hooks.0.clear();
        probe.pending_player_digivolve_reduction = 0;
        probe.pending_interactive_digivolve_reduction = 0;
        probe.pending_cost_reduction_amount_override = None;
        let Some(target_identity) = probe.capture_source_play_host_identity(target) else { return false; };
        let (source_carrier_identity, source_carrier_top) = match source_ref {
            CardSourceRef::Material(carrier, _) | CardSourceRef::Link(carrier, _) => {
                let identity = probe.capture_digivolve_host_identity(carrier);
                let top = probe.player(carrier.player).battle_area[carrier.index as usize].top_card().handle();
                (Some(identity), Some(top))
            }
            _ => (None, None),
        };
        let declaration = EffectDigivolveCostChoiceState {
            acting_player, source_ref, result_card, source_carrier_top, source_carrier_identity,
            target_player: target.player, target_zone: Self::effect_digivolve_target_zone(target), target_identity, target_top,
            dna_origin: probe.current_dna_origin, cost_delta,
            waiver: DigivolveRequirementWaiver::None, source: PlaySource::ByEffect,
            printed_costs: Vec::new(), effect_source_player: probe.effect_source_player,
            trigger_context: probe.current_trigger_context.clone(), result_clause: None,
            outer_conts: Vec::new(),
        };
        let cost_target = CostTargetContext {
            card: result_card, from_hand: matches!(source_ref, CardSourceRef::Hand(_, _)),
            is_digivolve: true, target_permanents: [Some(target), None],
        };
        let effective_delta = probe.effect_digivolve_cost_delta(acting_player, cost_delta);
        routes.into_iter().any(|route| {
            probe.effect_digivolve_route_is_affordable(
                &declaration,
                cost_target,
                effective_delta.resolve(route.memory_cost),
                route,
            )
        })
    }

    /// The effect's explicit reduction is subject to the same prohibition
    /// whether checking a candidate or committing a selected route.
    fn effect_digivolve_cost_delta(&self, player: PlayerId, delta: CostDelta) -> CostDelta {
        match delta {
            CostDelta::Reduce(n)
                if n > 0
                    && (self
                        .modifiers
                        .player_has(player, ModifierType::CannotReduceCost)
                        || self
                            .modifiers
                            .player_has(player, ModifierType::CannotReduceDigivolveCost)
                        || self.modifiers.any_other_player_has(
                            player,
                            ModifierType::OpponentCannotReduceDigivolveCost,
                        )) =>
            {
                CostDelta::Reduce(0)
            }
            other => other,
        }
    }

    /// Existential search over the same legal payment prompts used at commit.
    /// Only isolated clones consume payments or OPTs; the live declaration
    /// retains every player choice, including an optional decline.
    fn effect_digivolve_route_is_affordable(
        &self,
        declaration: &EffectDigivolveCostChoiceState,
        target: CostTargetContext,
        base_cost: u16,
        route: crate::dna_digivolve::DigivolveRouteMatch,
    ) -> bool {
        let mut initial = self.clone();
        // Keep synchronous payment triggers deferred throughout this clone-only
        // search. The speculative scope is discarded, never flushed.
        initial.enter_deferred_drain();
        // Caller continuations are outside this payment. Retain only hooks
        // installed by the probe's own nested cost effects.
        initial.after_selection_resume_hooks.0.clear();
        let Some((_, host)) = initial.resolve_effect_digivolve_declaration(declaration) else {
            return false;
        };
        let orders = route.treated_as
            .and_then(|profile| initial.install_would_digivolve_identity(host, profile));
        // The speculative declaration owns its payment choices just as the
        // real declaration does. Caller scopes remain for physical identity,
        // but cannot receive this probe's decline/abort attribution.
        initial.effect_digivolve_scopes.push(EffectDigivolveScope {
            declaration: declaration.clone(),
            identity: PendingWouldDigivolveResume {
                window_started: false,
                host_identity: declaration.target_identity,
                treated_identity_orders: orders,
                player: host.player,
                permanent: host,
                card: declaration.result_card,
                effective_cost: 0,
                app_fusion: false,
            },
        });
        initial.try_prompt_digivolve_reducer_chain_mode(
            declaration.acting_player,
            host,
            target,
            declaration.source,
            Some(declaration.clone()),
            true,
        );
        let mut frontier = vec![initial];
        while let Some(mut branch) = frontier.pop() {
            if let Some(pending) = branch.pending_selection.as_ref() {
                let player = pending.selecting_player;
                let mask = crate::action::mask::build_action_mask(&branch, player);
                // Stack in reverse order so the first offered legal choice is
                // explored first. No choices are silently auto-selected.
                for action in (0..mask.len()).rev() {
                    if mask[action] == 0.0 {
                        continue;
                    }
                    let mut next = branch.clone();
                    if next
                        .resolve_payment_probe_selection(player, action as u16)
                        .is_ok()
                    {
                        frontier.push(next);
                    }
                }
                continue;
            }
            let Some((source, host)) = branch.resolve_effect_digivolve_declaration(declaration)
            else {
                continue;
            };
            let cost_target = CostTargetContext {
                card: declaration.result_card,
                from_hand: matches!(source, CardSourceRef::Hand(_, _)),
                is_digivolve: true,
                target_permanents: [Some(host), None],
            };
            let reduction = if declaration.target_zone == Zone::BreedingArea { 0 } else {
                let reduction = std::mem::take(&mut branch.pending_player_digivolve_reduction)
                    + branch.scan_before_pay_cost_reduction_with_target(
                    declaration.acting_player,
                    CostReductionKind::Digivolve,
                    Some(cost_target),
                );
                branch.scan_before_pay_cost_observers(declaration.acting_player, Some(cost_target));
                reduction
            };
            if branch
                .resolve_effect_digivolve_declaration(declaration)
                .is_none()
            {
                continue;
            }
            let payable = (i32::from(base_cost) - reduction).max(0) as u16;
            if branch.can_pay_memory_for(declaration.acting_player, payable) {
                return true;
            }
        }
        false
    }

    pub(crate) fn capture_digivolve_host_identity(&mut self, handle: PermanentHandle) -> u64 {
        if let Some(identity) =
            self.player(handle.player).battle_area[handle.index as usize].battle_identity
        {
            return identity;
        }
        let identity = self.next_battle_identity;
        self.next_battle_identity = identity.checked_add(1).expect("stack identity exhausted");
        self.player_mut(handle.player).battle_area[handle.index as usize].battle_identity =
            Some(identity);
        identity
    }

    pub(crate) fn resolve_digivolve_host_identity(
        &self,
        player: PlayerId,
        identity: u64,
    ) -> Option<PermanentHandle> {
        self.player(player)
            .battle_area
            .iter()
            .position(|perm| perm.battle_identity == Some(identity))
            .map(|index| PermanentHandle {
                player,
                index: index as u8,
            })
    }

    fn resolve_digivolve_reducer_target(
        &self,
        state: &DigivolveReducerState,
    ) -> Option<(usize, usize)> {
        if let Some(dna) = &state.dna_effect {
            let (field, _, result) = self.resolve_effect_dna_hand(dna)?;
            return Some((result, field.index as usize));
        }
        if let Some(effect) = &state.effect {
            let (source, target) = self.resolve_effect_digivolve_declaration(effect)?;
            let source_index = match source {
                CardSourceRef::Hand(_, index) => index,
                _ => 0,
            };
            return Some((source_index, target.index as usize));
        }
        let player = self.player(state.acting_player);
        let hand = player
            .hand
            .iter()
            .position(|card| card.handle() == state.target.card)?;
        let field = player
            .battle_area
            .iter()
            .position(|permanent| permanent.battle_identity == Some(state.host_identity))?;
        Some((hand, field))
    }

    fn clear_digivolve_reducer_chain_credit(&mut self) {
        self.abort_would_digivolve_declaration();
        self.pending_player_digivolve_reduction = 0;
        self.pending_interactive_digivolve_reduction = 0;
        self.pending_cost_reduction_amount_override = None;
        self.pending_digivolve_route_choice = None;
    }

    pub(crate) fn finish_player_digivolve_reducer_chain(
        &mut self,
        state: DigivolveReducerState,
    ) -> bool {
        let previous_source = self.effect_source_player;
        let previous_trigger = self.current_trigger_context.clone();
        let previous_dna = self.current_dna_origin;
        if let Some(effect) = &state.effect {
            self.effect_source_player = effect.effect_source_player;
            self.current_trigger_context = effect.trigger_context.clone();
            self.current_dna_origin = effect.dna_origin;
        }
        if let Some(dna) = &state.dna_effect {
            self.effect_source_player = Some(dna.source_controller);
            self.current_trigger_context = dna.trigger_context.clone();
            self.current_dna_origin = Some(true);
        }
        let result = self.finish_player_digivolve_reducer_chain_inner(state);
        self.effect_source_player = previous_source;
        self.current_trigger_context = previous_trigger;
        self.current_dna_origin = previous_dna;
        result
    }

    fn finish_player_digivolve_reducer_chain_inner(
        &mut self,
        state: DigivolveReducerState,
    ) -> bool {
        // Payment effects may compact either zone, or remove/replay the host.
        // Preserve the declaration's physical card and battle-stack identity.
        let declaration = state.clone();
        if self.advance_player_digivolve_reducers(state) {
            return false;
        }
        if declaration.affordability_probe {
            return false;
        }
        if let Some(dna) = declaration.dna_effect.clone() {
            return self.resume_effect_dna_hand(dna, true).is_some();
        }
        if let Some(effect) = declaration.effect.clone() {
            let chosen = effect.printed_costs.first().copied();
            return self.resume_effect_digivolve_declaration(effect, chosen, true);
        }
        let Some((hand, field)) = self.resolve_digivolve_reducer_target(&declaration) else {
            self.clear_digivolve_reducer_chain_credit();
            return false;
        };
        let result = self.digivolve_from_hand_inner(
            declaration.acting_player,
            hand,
            field,
            declaration.source,
            true,
        );
        if !result && self.pending_selection.is_none() {
            self.clear_digivolve_reducer_chain_credit();
        }
        result
    }

    pub(crate) fn resume_player_digivolve_reducers(&mut self, state: DigivolveReducerState) {
        self.finish_player_digivolve_reducer_chain(state);
    }

    /// Ordering covers both field and player effects: either kind can consume
    /// a permanent needed by another payment. The payment's own optionality is
    /// independent of this mandatory order choice.
    pub(crate) fn install_player_digivolve_reducer_prompt(&mut self, state: DigivolveReducerState) {
        use crate::action::space::HAND_EFFECT_START;
        let ordering = state.choosing_order;
        if !ordering && !state.current.as_ref().expect("selected reducer").optional() {
            self.player_digivolve_reducer_accept(state);
            return;
        }
        let choices_to_show: Vec<_> = if ordering {
            state.remaining.iter().collect()
        } else {
            vec![state.current.as_ref().expect("selected reducer")]
        };
        let source_card = choices_to_show[0].source_card();
        let source_kind = self.effect_source_kind_for_handle(source_card);
        let source_permanent = match state.current.as_ref() {
            Some(DigivolveReducerChoice::Field(candidate)) if !ordering => {
                candidate.key.source_permanent
            }
            _ => None,
        };
        let choices: Vec<_> = choices_to_show
            .iter()
            .enumerate()
            .map(|(index, choice)| EffectChoiceEntry {
                count_choice: None,
                label: choice.label(),
                action_id: HAND_EFFECT_START + index as u16,
                source_card: Some(choice.source_card()),
                source_kind: Some(self.effect_source_kind_for_handle(choice.source_card())),
                timing: Some(EffectTiming::BeforePayCost),
                is_optional: !ordering,
                keyword: choice.keyword(),
                observation_metadata: Default::default(),
            })
            .collect();
        let prompt = if ordering {
            "Choose which digivolution cost reducer to resolve next".to_string()
        } else {
            format!("Use {} to reduce digivolution cost?", choices[0].label)
        };
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::EffectChoice;
        let accepted = state.clone();
        let declined = state.clone();
        self.pending_selection = Some(PendingSelection {
            card_choice: None,
            zone_owner: None,
            kind: SelectionKind::EffectChoice,
            selecting_player: state.acting_player,
            previous_phase,
            valid_action_ids: choices.iter().map(|entry| entry.action_id).collect(),
            is_optional: !ordering,
            prompt,
            effect_choices: Some(choices),
            source_card,
            source_permanent,
            source_kind,
            callback: Box::new(move |game, action| {
                game.run_digivolve_reducer_prompt_step(accepted.clone(), action, false)
            }),
            on_decline: if ordering {
                None
            } else {
                Some(Box::new(move |game: &mut Game| {
                    game.run_digivolve_reducer_prompt_step(
                        declined.clone(),
                        crate::action::space::PASS,
                        true,
                    )
                }))
            },
        });
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::DigivolveReducerPrompt(state)],
        });
    }

    pub(crate) fn player_digivolve_reducer_accept(&mut self, state: DigivolveReducerState) {
        use crate::action::space::encode_attack;
        let current = state.current.as_ref().expect("selected reducer").clone();
        if let DigivolveReducerChoice::Field(candidate) = current {
            if let Some(next) = self.run_digivolve_reducer_chain_payment(state, candidate) {
                self.resume_player_digivolve_reducers(next);
            }
            return;
        }
        // Player-scoped suspend reducers and `<Digisorption>` (§16-10-1):
        // choose 1 of your unsuspended Digimon to suspend; the Digimon that
        // will become a digivolution card is a candidate (§16-10-5).
        let suspendable = self.suspendable_own_digimon(state.acting_player);
        if suspendable.is_empty() {
            if let DigivolveReducerChoice::Player { index, reducer } = &current {
                self.consume_player_digivolve_reducer(*index, reducer, reducer.single_fire);
            }
            self.resume_player_digivolve_reducers(state);
            return;
        }
        let source_card = current.source_card();
        let source_kind = self.effect_source_kind_for_handle(source_card);
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::SelectTarget;
        let resumed = state.clone();
        self.pending_selection = Some(PendingSelection {
            card_choice: None,
            zone_owner: Some(state.acting_player),
            kind: SelectionKind::OwnField,
            selecting_player: state.acting_player,
            previous_phase,
            valid_action_ids: suspendable
                .iter()
                .map(|index| encode_attack(0, *index as u16))
                .collect(),
            is_optional: false,
            prompt: "Suspend 1 of your Digimon (digivolution cost reduction)".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: None,
            source_kind,
            callback: Box::new(move |game, action| {
                game.run_digivolve_reducer_suspend_step(resumed.clone(), action)
            }),
            on_decline: None,
        });
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::DigivolveReducerSuspend(state)],
        });
    }

    /// Consume a next-event reducer by identity; earlier resolutions may have
    /// shifted its recorded vector index. Reusable reducers are retained.
    pub(crate) fn consume_player_digivolve_reducer(
        &mut self,
        reducer_idx: usize,
        reducer: &crate::player_cost_reducer::PlayerDigivolveCostReducer,
        single_fire: bool,
    ) {
        if !single_fire {
            return;
        }
        // Prefer the recorded index when it still points at the same reducer;
        // otherwise re-locate by identity.
        if self
            .player_digivolve_cost_reducers
            .get(reducer_idx)
            .is_some_and(|r| {
                r.player == reducer.player
                    && r.source_card == reducer.source_card
                    && r.amount == reducer.amount
            })
        {
            self.player_digivolve_cost_reducers.remove(reducer_idx);
            return;
        }
        if let Some(pos) = self.player_digivolve_cost_reducers.iter().position(|r| {
            r.player == reducer.player
                && r.source_card == reducer.source_card
                && r.amount == reducer.amount
        }) {
            self.player_digivolve_cost_reducers.remove(pos);
        }
    }

    fn install_would_digivolve_identity(
        &mut self,
        handle: PermanentHandle,
        profile: crate::dna_digivolve::DigivolveTreatedIdentity,
    ) -> Option<(u64, Option<u64>)> {
        use crate::modifiers::{ModifierEntry, ModifierPayload};
        let identity = self
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)?
            .synth_identity(&self.card_data, &self.modifiers, handle);
        self.modifiers.add(
            handle,
            ModifierEntry::simple(
                ModifierType::TreatAsDigimon,
                0,
                crate::enums::Expiry::Permanent,
                handle.player,
            )
            .with_payload(ModifierPayload::SynthIdentity {
                kind: profile.kind,
                level: profile.level,
                colors: profile.colors(),
                traits: identity.traits,
                dp: identity.dp.unwrap_or(0),
            }),
        );
        let identity_order = self
            .modifiers
            .get(handle, ModifierType::TreatAsDigimon)
            .last()
            .expect("just installed identity")
            .install_order;
        let no_dp_order = if identity.dp.is_none() {
            self.modifiers.add(
                handle,
                ModifierEntry::simple(
                    ModifierType::DontHaveDp,
                    0,
                    crate::enums::Expiry::Permanent,
                    handle.player,
                ),
            );
            Some(
                self.modifiers
                    .get(handle, ModifierType::DontHaveDp)
                    .last()
                    .expect("just installed no-DP identity")
                    .install_order,
            )
        } else {
            None
        };
        self.modifiers.invalidate_declarative_memo();
        self.tick_declarative_effects();
        Some((identity_order, no_dp_order))
    }

    pub(crate) fn abort_would_digivolve_declaration(&mut self) {
        if let Some(resume) = self.pending_would_digivolve_resume.take() {
            self.clear_would_digivolve_identity(&resume);
        }
    }

    pub(crate) fn clear_would_digivolve_identity(&mut self, resume: &PendingWouldDigivolveResume) {
        let Some((identity_order, no_dp_order)) = resume.treated_identity_orders else {
            return;
        };
        if let Some(handle) =
            self.resolve_source_play_host_identity(resume.permanent, resume.host_identity)
        {
            let subject = crate::modifiers::ModifierSubject::Permanent(handle);
            self.modifiers
                .remove_until_condition_by_order(subject, identity_order);
            if let Some(order) = no_dp_order {
                self.modifiers
                    .remove_until_condition_by_order(subject, order);
            }
        }
        self.modifiers.invalidate_declarative_memo();
        self.tick_declarative_effects();
    }

    pub(crate) fn commit_digivolve_from_hand_no_replace(
        &mut self,
        mut resume: PendingWouldDigivolveResume,
    ) -> bool {
        let Some(handle) =
            self.resolve_digivolve_host_identity(resume.player, resume.host_identity)
        else {
            return false;
        };
        resume.permanent = handle;
        let field_index = handle.index as usize;
        if resume.permanent.player != resume.player {
            return false;
        }
        if self
            .player(resume.player)
            .battle_area
            .get(field_index)
            .is_none()
        {
            return false;
        }
        if self
            .modifiers
            .has(resume.permanent, ModifierType::CannotDigivolve)
        {
            return false;
        }

        let Some(hand_index) = self
            .player(resume.player)
            .hand
            .iter()
            .position(|card| card.handle() == resume.card)
        else {
            return false;
        };
        // Re-validate that SOME route still applies (the board may have shifted
        // since the resume was built), but use the player's CHOSEN route's
        // app-fusion-ness — not the re-derived min — so the commit honors the
        // cost choice.
        if self
            .normal_digivolve_route_for_hand_card(resume.player, hand_index, resume.permanent)
            .is_none()
        {
            return false;
        }
        let is_app_fusion = resume.app_fusion;

        let (from_stack_top, top_card_id) = {
            let player = self.player(resume.player);
            let perm = &player.battle_area[field_index];
            let card = &player.hand[hand_index];
            (
                perm.top_card().card_id(&self.card_data).to_string(),
                card.card_id(&self.card_data).to_string(),
            )
        };

        if !self.pay_memory(resume.effective_cost) {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: cannot pay memory cost {} (current memory={})",
                resume.effective_cost, self.memory
            ));
            return false;
        }

        let turn = self.turn_count;
        self.remember_belief_public_hand_departure(resume.player, hand_index);
        let removed = self.player_mut(resume.player).hand.remove(hand_index);
        self.player_mut(resume.player).battle_area[field_index].digivolve(removed, turn);
        // App Fusion: after stacking the App-Fusion card on top of the host
        // (the digivolve above), the host's existing LINKED cards are moved
        // UNDER the new top as digivolution sources (consumed). Mirrors DCGO
        // `CardController.cs` `AddToSources(LinkedCard(card))`. Order is
        // preserved bottom-to-top by draining front-to-back through
        // `push_under` (which inserts at position 0): draining in reverse so
        // the first linked card ends up deepest.
        if is_app_fusion {
            let linked: Vec<crate::card_source::CardSource> = std::mem::take(
                &mut self.player_mut(resume.player).battle_area[field_index].linked_cards,
            );
            for card in linked.into_iter().rev() {
                self.player_mut(resume.player).battle_area[field_index].push_under(card);
            }
        }
        let event_card = self
            .player(resume.player)
            .battle_area
            .get(field_index)
            .map(|perm| perm.top_card().handle())
            .expect("digivolve target remains in battle area after stack mutation");
        let top_card_name = self
            .player(resume.player)
            .battle_area
            .get(field_index)
            .map(|perm| perm.top_card().card_name(&self.card_data).to_string())
            .unwrap_or_default();

        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Digivolve {
            seq,
            player: resume.player,
            top_card_id,
            card_name: top_card_name,
            field_index: field_index as u8,
            from_stack_top,
            // Regular evo-cost path from hand — not DNA, not Blast DNA.
            was_dna: false,
            was_blast_dna: false,
            memory_paid: resume.effective_cost as i16,
        });

        let _ = self.draw_one_for_player(resume.player);

        self.fire_digivolve_event_triggers(resume.permanent, event_card, false, false);

        // Reward-shaping counter — bumped here at the user-action choke
        // point that both `digivolve_from_hand_inner` and the
        // replacement-resume path (`commit_pending_would_digivolve`)
        // funnel through. Effect-initiated digivolves (`digivolve_onto`)
        // intentionally do NOT bump. See
        // docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md.
        self.n_digivolutions[resume.player as usize] += 1;

        // 6-1-4-1: the turn ends only once the digivolution's processing is
        // over. A `<Blitz>` attack decision parked by a [When Digivolving]
        // (§16-16-3, `ResumeFrame::AttackDeclarationDecision`) is a pending
        // selection, so this is a no-op until it resolves; the decline, or
        // the attack's cleanup, re-runs the check.
        self.check_turn_end();
        true
    }

    /// Commit an effect-initiated App Fuse: stack `result_card` (pulled from
    /// `zone`) on top of `host`, then fold `host`'s existing linked cards under
    /// the new top as digivolution sources (consumed) — exactly like the
    /// alt-play app-fusion commit. App Fusion is printed Cost 0 and bypasses the
    /// normal evo-cost / level / colour requirement check (it is gated only by
    /// the host having the named materials, validated by the caller). Fires
    /// WhenDigivolving + OnDigivolve with `effect_initiated: true`. Returns true
    /// on success.
    ///
    /// Mirrors DCGO `PlayCardClass(result, …, host).SetAppFusion(…)` (see
    /// `BT23_079.cs`) and reuses the linked-card-consume step from
    /// `commit_digivolve_from_hand_no_replace`'s `is_app_fusion` branch.
    pub fn commit_effect_app_fuse(
        &mut self,
        player_id: PlayerId,
        host: PermanentHandle,
        result_card: crate::card_source::CardHandle,
        zone: crate::game_actions::AppFuseSourceZone,
    ) -> bool {
        let field_index = host.index as usize;
        if host.player != player_id {
            return false;
        }
        if self
            .player(player_id)
            .battle_area
            .get(field_index)
            .is_none()
        {
            self.logger
                .log("[Rejected] app_fuse: host index out of range");
            return false;
        }
        if self.modifiers.has(host, ModifierType::CannotDigivolve) {
            return false;
        }

        // Resolve the result card to a zone-tagged CardSourceRef.
        let source_ref = match zone {
            crate::game_actions::AppFuseSourceZone::Hand => {
                let Some(i) = self
                    .player(player_id)
                    .hand
                    .iter()
                    .position(|c| c.handle() == result_card)
                else {
                    self.logger
                        .log("[Rejected] app_fuse: result card not in hand");
                    return false;
                };
                crate::enums::CardSourceRef::Hand(player_id, i)
            }
            crate::game_actions::AppFuseSourceZone::Trash => {
                let Some(i) = self
                    .player(player_id)
                    .trash
                    .iter()
                    .position(|c| c.handle() == result_card)
                else {
                    self.logger
                        .log("[Rejected] app_fuse: result card not in trash");
                    return false;
                };
                crate::enums::CardSourceRef::Trash(player_id, i)
            }
        };

        let from_stack_top = self.player(host.player).battle_area[field_index]
            .top_card()
            .card_id(&self.card_data)
            .to_string();

        // App Fusion is Cost 0 — no memory payment, no evo-requirement check.
        let Some(taken) = self.take_card_source_ref(source_ref) else {
            self.logger
                .log("[Rejected] app_fuse: result card ref changed before take");
            return false;
        };
        let top_card_id = taken.card.card_id(&self.card_data).to_string();

        let turn = self.turn_count;
        self.player_mut(host.player).battle_area[field_index].digivolve(taken.card, turn);

        // Consume the host's linked cards under the new top as digivolution
        // sources (drained in reverse so the first linked card lands deepest),
        // identical to `commit_digivolve_from_hand_no_replace`'s app-fusion step.
        let linked: Vec<crate::card_source::CardSource> =
            std::mem::take(&mut self.player_mut(host.player).battle_area[field_index].linked_cards);
        for card in linked.into_iter().rev() {
            self.player_mut(host.player).battle_area[field_index].push_under(card);
        }

        let event_card = self
            .player(host.player)
            .battle_area
            .get(field_index)
            .map(|perm| perm.top_card().handle())
            .expect("app-fuse host remains in battle area after stack mutation");
        let top_card_name = self
            .player(host.player)
            .battle_area
            .get(field_index)
            .map(|perm| perm.top_card().card_name(&self.card_data).to_string())
            .unwrap_or_default();

        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Digivolve {
            seq,
            player: player_id,
            top_card_id,
            card_name: top_card_name,
            field_index: field_index as u8,
            from_stack_top,
            was_dna: false,
            was_blast_dna: false,
            memory_paid: 0,
        });

        self.fire_digivolve_event_triggers(host, event_card, true, false);

        true
    }

    /// Install a `SelectMaterial` pending selection for DNA digivolve.
    /// Drives a two-stage resolution: stage 1 picks the first material;
    /// stage 2 (installed by the stage-1 callback) picks the second
    /// material. Stage 2 resolves into `Game::dna_digivolve_inner`,
    /// computes the matching `DnaCost` via `get_dna_stacking_order`,
    /// applies `BeforePayCost` reductions, and pays memory.
    pub fn initiate_dna_digivolve(&mut self, player_id: PlayerId, hand_index: usize) -> bool {
        if self.game_over {
            return false;
        }
        let Some(route_window) = self.current_dna_route_window() else {
            self.logger.log(&format!(
                "[Rejected] initiate_dna_digivolve: not in DNA action phase (phase={:?})",
                self.current_phase
            ));
            return false;
        };
        let player = self.player(player_id);
        if hand_index >= player.hand.len() {
            self.logger.log(&format!(
                "[Rejected] initiate_dna_digivolve: hand index {} out of range (hand size={})",
                hand_index,
                player.hand.len()
            ));
            return false;
        }
        let card = player.hand[hand_index].clone();

        // Collect valid first-material battle_area indices: those that
        // appear in at least one valid pair (either ordering).
        let mut first_targets: Vec<u16> = self
            .valid_dna_first_targets_for_hand_card(player_id, hand_index, route_window)
            .collect();
        first_targets.sort();
        first_targets.dedup();
        if first_targets.is_empty() {
            self.logger.log(&format!(
                "[Rejected] initiate_dna_digivolve: no valid DNA material pair for {}",
                card.card_id(&self.card_data)
            ));
            return false;
        }

        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::SelectMaterial;

        let selecting_player = player_id;
        let source_card = card.handle();

        self.pending_selection = Some(PendingSelection {
            card_choice: None,
            zone_owner: None,
            kind: SelectionKind::Material,
            selecting_player,
            previous_phase,
            valid_action_ids: first_targets,
            is_optional: false,
            prompt: crate::selection::DNA_FIRST_MATERIAL_PROMPT.to_string(),
            effect_choices: None,
            source_card,
            source_permanent: None,
            source_kind: EffectSourceKind::Digimon,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                // Stage 1 resolution: action_id is the chosen first-material
                // battle_area index for `selecting_player`.
                let first_idx = action_id as usize;
                let first_player = selecting_player;
                let evo_hand_index = hand_index;

                // Defensive: validate the first index against the
                // controller's current battle_area (it could have shifted
                // since selection was installed if a triggered effect
                // removed a permanent during the install drain).
                if first_idx >= game.player(first_player).battle_area.len() {
                    game.logger.log(&format!(
                        "[Rejected] dna_digivolve stage 1: first index {} out of range (battle_area size={})",
                        first_idx,
                        game.player(first_player).battle_area.len()
                    ));
                    return;
                }
                if evo_hand_index >= game.player(first_player).hand.len() {
                    game.logger.log(&format!(
                        "[Rejected] dna_digivolve stage 1: evo hand index {} out of range (hand size={})",
                        evo_hand_index,
                        game.player(first_player).hand.len()
                    ));
                    return;
                }

                // Build valid second-material list for the chosen first.
                let second_targets: Vec<u16> = game
                    .valid_dna_second_targets_for_hand_card(
                        first_player,
                        evo_hand_index,
                        first_idx,
                        route_window,
                    )
                    .collect();
                if second_targets.is_empty() {
                    game.logger.log(&format!(
                        "[Rejected] dna_digivolve stage 1: no valid second-material targets for first index {}",
                        first_idx
                    ));
                    return;
                }

                // Install stage-2 selection. previous_phase was Main when
                // stage-1 was installed; we preserve it through the chain
                // so the final resolution returns to Main.
                game.pending_selection = Some(PendingSelection {
                    card_choice: None,
                    zone_owner: None,
                    kind: SelectionKind::Material,
                    selecting_player: first_player,
                    previous_phase,
                    valid_action_ids: second_targets,
                    is_optional: false,
                    prompt: crate::selection::DNA_SECOND_MATERIAL_PROMPT.to_string(),
                    effect_choices: None,
                    source_card,
                    source_permanent: None,
                    source_kind: EffectSourceKind::Digimon,
                    callback: Box::new(move |game: &mut Game, action_id: u16| {
                        let second_idx = action_id as usize;
                        game.resolve_dna_digivolve_stage2_with_window(
                            first_player,
                            first_idx,
                            second_idx,
                            evo_hand_index,
                            route_window,
                        );
                    }),
                    on_decline: None,
                });
                // resolve_generic_selection restored current_phase to
                // previous_phase (Main) before invoking this callback. We
                // re-flip it back to SelectMaterial so the action mask
                // reflects the live stage-2 prompt.
                game.current_phase = GamePhase::SelectMaterial;
            }),
            on_decline: None,
        });
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::DnaDigivolveFirstMaterial(
                DnaDigivolveFirstMaterialState {
                    player_id,
                    hand_index,
                    route_window,
                    source_card,
                    previous_phase,
                },
            )],
        });
        true
    }

    /// Stage-2 resolution of `Game::initiate_dna_digivolve`'s two-stage
    /// selection chain. Receives the chosen second-material `battle_area`
    /// index and the captured stage-1 state. Re-resolves the matching
    /// DNA route orientation, applies `BeforePayCost` reductions, calls
    /// `Game::dna_digivolve_inner`, and triggers the auto-end-of-turn
    /// check (mirroring `digivolve_from_hand`).
    ///
    /// Defensively re-validates indices because triggered effects fired
    /// during stage-1 install can mutate the battle area between selection
    /// install and resolution.
    ///
    /// Failure paths log `[Rejected] ...` via `self.logger` and return
    /// without mutating game state. The `pending_selection` was already
    /// consumed by `resolve_generic_selection` before this method ran.
    pub fn resolve_dna_digivolve_stage2_with_window(
        &mut self,
        first_player: PlayerId,
        first_idx: usize,
        second_idx: usize,
        evo_hand_index: usize,
        route_window: crate::dna_digivolve::DnaRouteWindow,
    ) {
        if second_idx >= self.player(first_player).battle_area.len() {
            self.logger.log(&format!(
                "[Rejected] resolve_dna_digivolve_stage2: second index {} out of range (battle_area size={})",
                second_idx,
                self.player(first_player).battle_area.len()
            ));
            return;
        }
        if first_idx == second_idx {
            self.logger
                .log("[Rejected] resolve_dna_digivolve_stage2: first and second indices are equal");
            return;
        }
        if evo_hand_index >= self.player(first_player).hand.len() {
            self.logger.log(&format!(
                "[Rejected] resolve_dna_digivolve_stage2: evo hand index {} out of range (hand size={})",
                evo_hand_index,
                self.player(first_player).hand.len()
            ));
            return;
        }

        let Some(route_match) = self.dna_route_for_hand_card(
            first_player,
            evo_hand_index,
            first_idx,
            second_idx,
            route_window,
        ) else {
            self.logger.log(
                "[Rejected] resolve_dna_digivolve_stage2: no matching DNA route for chosen pair",
            );
            return;
        };
        let printed_cost = route_match.memory_cost;

        let (target_a, target_b) = if route_match.first_is_top {
            (
                PermanentHandle {
                    player: first_player,
                    index: first_idx as u8,
                },
                PermanentHandle {
                    player: first_player,
                    index: second_idx as u8,
                },
            )
        } else {
            (
                PermanentHandle {
                    player: first_player,
                    index: second_idx as u8,
                },
                PermanentHandle {
                    player: first_player,
                    index: first_idx as u8,
                },
            )
        };

        // Pass the DNA-result hand card as the cost-target so target-aware
        // predicates (`cost_target: { color_is: green }`) can fire for the
        // DNA digivolve path — G-BEFORE-PAY-COST-DIGIVOLVE-TARGET. Both
        // DNA materials get threaded as target permanents so per-material
        // self-scoped observers ("when THIS Digimon would DNA digivolve")
        // can fire on either side.
        let evo_card_target = self.player(first_player).hand[evo_hand_index].handle();
        let target = CostTargetContext {
            card: evo_card_target,
            from_hand: true,
            is_digivolve: true,
            target_permanents: [Some(target_a), Some(target_b)],
        };
        // Set the DNA-origin context so cost-calc-time predicates like
        // `dna_origin: true` evaluate true for both the reducer scan and
        // the observer scan. Restored after both scans complete so the
        // marker doesn't leak into downstream effect-queue drains (those
        // re-set it themselves per-effect).
        let prev_dna_origin = self.current_dna_origin;
        self.current_dna_origin = Some(true);
        let total_reduction = self.scan_before_pay_cost_reduction_with_target(
            first_player,
            CostReductionKind::Digivolve,
            Some(target),
        );
        // Observer dispatch — G-BEFORE-PAY-COST-GAIN-MEMORY.
        self.scan_before_pay_cost_observers(first_player, Some(target));
        self.current_dna_origin = prev_dna_origin;
        let effective_cost = (printed_cost as i32 - total_reduction).max(0) as u16;

        let _ = self.dna_digivolve_inner(
            target_a,
            target_b,
            first_player,
            evo_hand_index,
            effective_cost,
            false,
        );

        if route_window == crate::dna_digivolve::DnaRouteWindow::EndOfTurnAction {
            if self.memory < 0 && !self.game_over {
                self.pass_end_of_turn_action();
            }
        } else {
            self.check_turn_end();
        }
    }

    /// Script-initiated digivolve: place the card at `hand_index` from
    /// `player_id`'s hand onto `target`, bypassing the phase check and
    /// optionally the color check. Memory is paid according to `cost_delta`.
    ///
    /// Unlike `digivolve_from_hand`, this does **not** check `GamePhase::Main`
    /// or fire `check_turn_end` — it is designed for use inside effect
    /// callbacks where those invariants don't apply. It DOES draw the
    /// §8-1-3-3 card: the draw is part of the digivolution *procedure*
    /// ("places the Digimon ... on top of the chosen card, draws 1 card, and
    /// the digivolution process is resolved"), not a benefit of the main-phase
    /// action, so it applies however the digivolution was initiated.
    ///
    /// Returns `true` on synchronous success. `false` means validation failed
    /// or a mandatory cost-choice prompt was installed; the latter resumes
    /// through `EffectDigivolveCostChoice` before any payment is made.
    pub fn effect_initiated_digivolve(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        ignore_color: bool,
        source: PlaySource,
    ) -> bool {
        self.effect_initiated_digivolve_from_source_inner(
            player_id,
            crate::enums::CardSourceRef::Hand(player_id, hand_index),
            target,
            cost_delta,
            DigivolveRequirementWaiver::color_if(ignore_color),
            source,
        )
    }

    pub fn effect_initiated_digivolve_ignore_requirements(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
    ) -> bool {
        self.effect_initiated_digivolve_from_source_inner(
            player_id,
            crate::enums::CardSourceRef::Hand(player_id, hand_index),
            target,
            cost_delta,
            DigivolveRequirementWaiver::All,
            source,
        )
    }

    /// Source-general script-initiated digivolve. The result card is taken
    /// from any `CardSourceRef`, placed on top of `target`, and restored to
    /// its source zone if a post-take failure occurs.
    pub fn effect_initiated_digivolve_from_source(
        &mut self,
        player_id: PlayerId,
        source_ref: crate::enums::CardSourceRef,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        ignore_color: bool,
        source: PlaySource,
    ) -> bool {
        self.effect_initiated_digivolve_from_source_inner(
            player_id,
            source_ref,
            target,
            cost_delta,
            DigivolveRequirementWaiver::color_if(ignore_color),
            source,
        )
    }

    pub fn effect_initiated_digivolve_from_source_ignore_requirements(
        &mut self,
        player_id: PlayerId,
        source_ref: crate::enums::CardSourceRef,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
    ) -> bool {
        self.effect_initiated_digivolve_from_source_inner(
            player_id,
            source_ref,
            target,
            cost_delta,
            DigivolveRequirementWaiver::All,
            source,
        )
    }

    /// Source-general script-initiated digivolve "ignoring level" (DCGO
    /// `CardEffectCommons.IgnoreRequirement.Level`). Only the base's LEVEL
    /// requirement is waived: the result card's colour-matching printed
    /// circles (any level) and its DSL alt-digivolve paths (with their
    /// subject-level leaves dropped) still decide whether the digivolve is
    /// legal, and the chosen route's printed memory cost is the base that
    /// `cost_delta` adjusts (e.g. EX13-071 "ignoring level and with the
    /// cost reduced by 1" = `CostDelta::Reduce(1)` off that printed cost).
    /// See `Game::digivolve_routes_ignoring_level`.
    /// G-DIGIVOLVE-IGNORE-LEVEL-PRINTED-COST.
    pub fn effect_initiated_digivolve_from_source_ignore_level(
        &mut self,
        player_id: PlayerId,
        source_ref: crate::enums::CardSourceRef,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
    ) -> bool {
        self.effect_initiated_digivolve_from_source_inner(
            player_id,
            source_ref,
            target,
            cost_delta,
            DigivolveRequirementWaiver::Level,
            source,
        )
    }

    pub(crate) fn effect_initiated_digivolve_from_source_inner(
        &mut self,
        player_id: PlayerId,
        source_ref: crate::enums::CardSourceRef,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        waiver: DigivolveRequirementWaiver,
        source: PlaySource,
    ) -> bool {
        self.effect_initiated_digivolve_with_chosen_cost(
            player_id, source_ref, target, cost_delta, waiver, source, None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn effect_initiated_digivolve_with_chosen_cost(
        &mut self,
        player_id: PlayerId,
        source_ref: CardSourceRef,
        target: PermanentHandle,
        cost_delta: CostDelta,
        waiver: DigivolveRequirementWaiver,
        source: PlaySource,
        chosen_cost: Option<crate::dna_digivolve::DigivolveRouteMatch>,
    ) -> bool {
        if !self.effect_digivolve_source_supported(source_ref, target) {
            return false;
        }
        let card = self
            .card_source_ref_peek(source_ref)
            .map(CardSource::handle);
        let result = self.effect_initiated_digivolve_with_chosen_cost_inner(
            player_id,
            source_ref,
            target,
            cost_delta,
            waiver,
            source,
            chosen_cost,
            false,
        );
        if self.pending_selection.is_none() {
            if let Some(card) = card {
                self.remove_effect_digivolve_scope(card);
            }
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn effect_initiated_digivolve_with_chosen_cost_inner(
        &mut self,
        player_id: PlayerId,
        source_ref: CardSourceRef,
        target: PermanentHandle,
        cost_delta: CostDelta,
        waiver: DigivolveRequirementWaiver,
        source: PlaySource,
        chosen_cost: Option<crate::dna_digivolve::DigivolveRouteMatch>,
        reducers_resolved: bool,
    ) -> bool {
        if !self.effect_digivolve_source_supported(source_ref, target) {
            return false;
        }
        let ignore_color = matches!(
            waiver,
            DigivolveRequirementWaiver::Color | DigivolveRequirementWaiver::All
        );
        let ignore_requirements = waiver == DigivolveRequirementWaiver::All;
        if source == PlaySource::ByEffect
            && self
                .modifiers
                .player_has(player_id, ModifierType::CannotDigivolveDigimonByEffect)
        {
            self.logger.log(
                "[Rejected] effect_initiated_digivolve: blocked by CannotDigivolveDigimonByEffect",
            );
            return false;
        }

        // Recheck the host prohibition after a hand/source or cost-choice
        // prompt resumes, before taking cards or paying memory. Waiving
        // digivolution requirements never waives a prohibiting effect.
        if self.modifiers.has(target, ModifierType::CannotDigivolve) {
            self.logger
                .log("[Rejected] effect_initiated_digivolve: target blocked by CannotDigivolve");
            return false;
        }

        let Some((evo_card_handle, evo_card_data_index, _)) =
            self.card_source_ref_snapshot(source_ref)
        else {
            self.logger
                .log("[Rejected] effect_initiated_digivolve: source ref out of range");
            return false;
        };

        let Some(host) = self.digivolve_base_permanent(target) else {
            self.logger.log("[Rejected] effect_initiated_digivolve: target no longer present");
            return false;
        };
        let target_zone = Self::effect_digivolve_target_zone(target);

        // 2. Find a matching evo cost.
        // A level-less target can still digivolve when it has no printed
        // level-matched circle to satisfy: through a special `alt_paths:
        // kind: digivolve` route (BT26-085 Giant Slayer "[Giant Slayer]:
        // Cost 5", G-ENGINE-EFFECT-DIGIVOLVE-FROM-LEVELLESS), or when the
        // level requirement is waived — "ignoring digivolution requirements"
        // (EX13-074 Rie Kishibe "[Main] … this Tamer may digivolve into
        // [LordKnightmon] … for a digivolution cost of 3, ignoring
        // digivolution requirements") or "ignoring level". The Tamer card
        // and every card under it simply become the new Digimon's
        // digivolution cards (DCGO stacks onto `card.PermanentOfThisCard()`
        // the same way, BT22_090.cs). G-TAMER-DIGIVOLVE-INTO-DIGIMON. Only
        // the printed level-matched circles need a level; with none, the
        // route scan below simply finds no printed match.
        let base_level: Option<u8> = {
            let perm = host;
            let identity = perm.synth_identity(&self.card_data, &self.modifiers, target);
            identity.level
        };

        let mut route_costs: Vec<crate::dna_digivolve::DigivolveRouteMatch> = if ignore_requirements
        {
            vec![crate::dna_digivolve::DigivolveRouteMatch::digivolve(0)]
        } else if waiver == DigivolveRequirementWaiver::Level {
            self.card_source_ref_peek(source_ref)
                .map(|card| {
                    self.digivolve_routes_ignoring_level(card, target)
                        .into_iter()
                        .collect()
                })
                .unwrap_or_default()
        } else {
            // Route through the SAME machinery as the player main-phase
            // digivolve action (`all_digivolve_routes_for_card`) so
            // effect-initiated digivolves honor everything the player action
            // does: printed evo-cost circles (`collect_rules_digivolve_routes`
            // performs the identical synth_identity level+color match the old
            // raw `evo_costs` scan did), trait-gated alternative digivolution
            // circles authored as DSL `alt_paths: kind: digivolve` (e.g.
            // EX12-047 "Lv.5 w/[Shambala] trait: Cost 3"), and the
            // `CanOnlyDigivolveInto` restriction gate. App Fusion is excluded:
            // it is an alt-PLAY mechanic with its own linked-card consumption
            // commit path, not a digivolve circle.
            let mut route_costs: Vec<crate::dna_digivolve::DigivolveRouteMatch> = self
                .card_source_ref_peek(source_ref)
                .map(|card| {
                    self.all_digivolve_routes_for_card(card, target)
                        .into_iter()
                        .filter(|route| !route.app_fusion)
                        .collect()
                })
                .unwrap_or_default();
            if ignore_color {
                // The alt-path predicate evaluation has no color-waiver hook,
                // so keep the color-waived printed-circle scan as an
                // additional candidates, preserving every printed cost.
                route_costs.extend(
                    self.card_data[evo_card_data_index]
                        .evo_costs
                        .iter()
                        .filter(|ec| Some(ec.level) == base_level)
                        .map(|ec| {
                            crate::dna_digivolve::DigivolveRouteMatch::digivolve(ec.memory_cost)
                        }),
                );
            }
            route_costs
        };
        route_costs.sort_by_key(|r| {
            (
                r.memory_cost,
                r.treated_as.map(|p| (p.kind as u8, p.level, p.colors)),
            )
        });
        route_costs.dedup();
        if route_costs.is_empty() {
            self.logger.log(&format!(
                "[Rejected] effect_initiated_digivolve: no matching evo cost (base_level={:?}, waiver={:?})",
                base_level, waiver
            ));
            return false;
        }
        // An effect-granted "cost reduced by N" is still a cost REDUCTION: a
        // flood-gate such as BT5-021 Syakomon's "[Opponent's Turn] your opponent
        // can't reduce digivolution costs" voids it (Q&A Q7148: the digivolve
        // still happens, at the unreduced cost). Mirrors the gates in
        // `collect_before_pay_cost_reducers`.
        let effective_delta = self.effect_digivolve_cost_delta(player_id, cost_delta);
        let cost_target_ctx = CostTargetContext {
            card: evo_card_handle,
            from_hand: matches!(source_ref, CardSourceRef::Hand(_, _)),
            is_digivolve: true,
            target_permanents: [Some(target), None],
        };
        let source_carrier_identity = match source_ref {
            CardSourceRef::Material(h, _) | CardSourceRef::Link(h, _) => {
                Some(self.capture_digivolve_host_identity(h))
            }
            _ => None,
        };
        let Some(target_identity) = self.capture_source_play_host_identity(target) else { return false; };
        let target_top = self.digivolve_base_permanent(target).expect("captured evolution host")
            .top_card()
            .handle();
        let source_carrier_top = match source_ref {
            CardSourceRef::Material(h, _) | CardSourceRef::Link(h, _) => self
                .player(h.player)
                .battle_area
                .get(h.index as usize)
                .and_then(|p| p.card_sources.last())
                .map(CardSource::handle),
            _ => None,
        };
        let matching_memory_cost = if let Some(chosen) = chosen_cost {
            // A vanished route is not permission to choose a different cost.
            if !route_costs.contains(&chosen) {
                return false;
            }
            chosen
        } else {
            // Fixed/free effects and reductions that collapse both routes to
            // the same cost have no distinct payable decision.
            route_costs.dedup_by_key(|route| {
                (effective_delta.resolve(route.memory_cost), route.treated_as)
            });
            if route_costs.len() > 1 {
                // Search real payment choices on clones. Summing optional
                // reductions would reuse the same suspend/payment resource.
                let declaration = EffectDigivolveCostChoiceState {
                    acting_player: player_id,
                    source_ref,
                    result_card: evo_card_handle,
                    source_carrier_top,
                    source_carrier_identity,
                    target_player: target.player,
                    target_zone: Self::effect_digivolve_target_zone(target),
                    target_identity,
                    target_top,
                    dna_origin: self.current_dna_origin,
                    cost_delta,
                    waiver,
                    source,
                    printed_costs: Vec::new(),
                    effect_source_player: self.effect_source_player,
                    trigger_context: self.current_trigger_context.clone(),
                    result_clause: None,
                    outer_conts: Vec::new(),
                };
                route_costs.retain(|cost| {
                    self.effect_digivolve_route_is_affordable(
                        &declaration,
                        cost_target_ctx,
                        effective_delta.resolve(cost.memory_cost),
                        *cost,
                    )
                });
                if route_costs.is_empty() {
                    return false;
                }
            }
            if route_costs.len() > 1 {
                let source_carrier_top = match source_ref {
                    CardSourceRef::Material(h, _) | CardSourceRef::Link(h, _) => self
                        .player(h.player)
                        .battle_area
                        .get(h.index as usize)
                        .and_then(|p| p.card_sources.last())
                        .map(CardSource::handle),
                    _ => None,
                };
                self.install_effect_digivolve_cost_choice(
                    EffectDigivolveCostChoiceState {
                        acting_player: player_id,
                        source_ref,
                        result_card: evo_card_handle,
                        source_carrier_top,
                        source_carrier_identity,
                        target_player: target.player,
                        target_zone: Self::effect_digivolve_target_zone(target),
                        target_identity,
                        dna_origin: self.current_dna_origin,
                        target_top: self.digivolve_base_permanent(target).expect("captured evolution host")
                            .top_card()
                            .handle(),
                        cost_delta,
                        waiver,
                        source,
                        printed_costs: route_costs,
                        effect_source_player: self.effect_source_player,
                        trigger_context: self.current_trigger_context.clone(),
                        result_clause: None,
                        outer_conts: Vec::new(),
                    },
                    effective_delta,
                );
                return false;
            }
            route_costs[0]
        };
        let declaration = EffectDigivolveCostChoiceState {
            acting_player: player_id,
            source_ref,
            result_card: evo_card_handle,
            source_carrier_top,
            source_carrier_identity,
            target_player: target.player,
            target_zone: Self::effect_digivolve_target_zone(target),
            target_identity,
            target_top,
            dna_origin: self.current_dna_origin,
            cost_delta,
            waiver,
            source,
            printed_costs: vec![matching_memory_cost],
            effect_source_player: self.effect_source_player,
            trigger_context: self.current_trigger_context.clone(),
            result_clause: None,
            outer_conts: Vec::new(),
        };
        self.remove_effect_digivolve_scope(evo_card_handle);
        let orders = matching_memory_cost
            .treated_as
            .and_then(|p| self.install_would_digivolve_identity(target, p));
        self.effect_digivolve_scopes.push(EffectDigivolveScope {
            declaration,
            identity: PendingWouldDigivolveResume {
                window_started: false,
                host_identity: target_identity,
                treated_identity_orders: orders,
                player: target.player,
                permanent: target,
                card: evo_card_handle,
                effective_cost: 0,
                app_fusion: false,
            },
        });
        if !reducers_resolved {
            let declaration = EffectDigivolveCostChoiceState {
                acting_player: player_id,
                source_ref,
                result_card: evo_card_handle,
                source_carrier_top,
                source_carrier_identity,
                target_player: target.player,
                target_zone: Self::effect_digivolve_target_zone(target),
                target_identity,
                target_top: self.digivolve_base_permanent(target).expect("captured evolution host")
                    .top_card()
                    .handle(),
                dna_origin: self.current_dna_origin,
                cost_delta,
                waiver,
                source,
                printed_costs: vec![matching_memory_cost],
                effect_source_player: self.effect_source_player,
                trigger_context: self.current_trigger_context.clone(),
                result_clause: None,
                outer_conts: Vec::new(),
            };
            if let Some(result) = self.try_prompt_digivolve_reducer_chain(
                player_id,
                target,
                cost_target_ctx,
                source,
                Some(declaration),
            ) {
                return result;
            }
        }
        let chain_reduction = if reducers_resolved {
            std::mem::take(&mut self.pending_player_digivolve_reduction)
        } else {
            0
        };
        let base_cost = effective_delta.resolve(matching_memory_cost.memory_cost);
        // Pass the evolving card as the cost-target so target-aware
        // predicates can fire — G-BEFORE-PAY-COST-DIGIVOLVE-TARGET. The
        // `target` permanent here is the one being digivolved into
        // (effect-initiated digivolves stack a hand/trash/security card
        // onto an existing battle-area permanent).
        // CR 3-4-7-6: ordinary would-digivolve effects cannot trigger on a
        // breeding host. The initiating effect's explicit reduction remains.
        let total_reduction = if target_zone == Zone::BreedingArea { 0 } else {
            let amount = self.scan_before_pay_cost_reduction_with_target(
                player_id, CostReductionKind::Digivolve, Some(cost_target_ctx),
            );
            self.scan_before_pay_cost_observers(player_id, Some(cost_target_ctx));
            amount
        };
        let effective_cost = (base_cost as i32 - total_reduction - chain_reduction).max(0) as u16;

        let Some(target) = self.resolve_source_play_host_identity(target, target_identity)
        else {
            return false;
        };
        if self.digivolve_base_permanent(target).expect("captured evolution host")
            .top_card()
            .handle()
            != target_top
        {
            return false;
        }
        let source_check = EffectDigivolveCostChoiceState {
            acting_player: player_id,
            source_ref,
            result_card: evo_card_handle,
            source_carrier_top,
            source_carrier_identity,
            target_player: target.player,
            target_zone: Self::effect_digivolve_target_zone(target),
            target_identity,
            target_top: self.digivolve_base_permanent(target).expect("captured evolution host")
                .top_card()
                .handle(),
            dna_origin: self.current_dna_origin,
            cost_delta,
            waiver,
            source,
            printed_costs: vec![matching_memory_cost],
            effect_source_player: self.effect_source_player,
            trigger_context: self.current_trigger_context.clone(),
            result_clause: None,
            outer_conts: Vec::new(),
        };
        if self.resolve_effect_digivolve_choice_source(&source_check).is_none() {
            return false;
        }
        if let Some(scope) = self
            .effect_digivolve_scopes
            .iter_mut()
            .rev()
            .find(|s| s.declaration.result_card == evo_card_handle)
        {
            scope.identity.effective_cost = effective_cost;
            scope.identity.window_started = true;
        }
        // A source clause being active in breeding does not grant permission
        // to observe another breeding evolution. No ordinary would event fires.
        let outcome = if target_zone == Zone::BreedingArea {
            crate::replacement::ReplacementOutcome::None
        } else {
            self.try_replace(
                EffectTiming::WhenPermanentWouldDigivolve,
                crate::replacement::ReplacementSubject::Permanent(target),
                crate::replacement::ReplacementCause::OwnEffect,
                Some(Zone::BattleArea),
            )
        };
        if self.pending_selection.is_some() {
            return false;
        }
        self.commit_effect_would_digivolve(evo_card_handle, outcome, false)
    }

    fn commit_effect_digivolve_no_replace(
        &mut self,
        state: &EffectDigivolveCostChoiceState,
        effective_cost: u16,
    ) -> bool {
        let Some((source_ref, target)) = self.resolve_effect_digivolve_declaration(state) else {
            return false;
        };
        if !self.effect_digivolve_source_supported(source_ref, target) {
            return false;
        }
        if self.modifiers.has(target, ModifierType::CannotDigivolve)
            || (state.source == PlaySource::ByEffect
                && self.modifiers.player_has(
                    state.acting_player,
                    ModifierType::CannotDigivolveDigimonByEffect,
                ))
        {
            return false;
        }
        let player_id = state.acting_player;
        // 3. Remove the card from its source and pay memory. If payment fails,
        // restore the source exactly where it came from.
        if let crate::enums::CardSourceRef::Security(defender, index) = source_ref {
            if !self.pay_memory_for(player_id, effective_cost) {
                self.logger.log(&format!(
                    "[Rejected] effect_initiated_digivolve: cannot pay memory cost {} (current memory={})",
                    effective_cost, self.memory
                ));
                return false;
            }

            let player = self.player_mut(defender);
            if index >= player.security.len() {
                self.logger
                    .log("[Rejected] effect_initiated_digivolve: source ref changed before take");
                return false;
            }
            let card = player.security.remove(index);
            player.face_up_security.remove(&card.card_index);
            let cause = crate::trigger_context::EventCause::from(self.infer_effect_cause(defender));
            self.fire_effect_security_removal(
                defender,
                player_id,
                player_id,
                cause,
                card,
                crate::selection::SecurityRemovalDestination::Digivolve {
                    player: player_id,
                    target,
                    turn: self.turn_count,
                },
            );
            return true;
        }

        let Some(taken) = self.take_card_source_ref(source_ref) else {
            self.logger
                .log("[Rejected] effect_initiated_digivolve: source ref changed before take");
            return false;
        };
        if !self.pay_memory_for(player_id, effective_cost) {
            self.logger.log(&format!(
                "[Rejected] effect_initiated_digivolve: cannot pay memory cost {} (current memory={})",
                effective_cost, self.memory
            ));
            let _ = self.restore_card_source_ref(source_ref, taken);
            return false;
        }

        // 4. Move the card onto the target permanent's stack.
        let turn = self.turn_count;
        if state.target_zone == Zone::BreedingArea {
            let from_stack_top = self.digivolve_base_permanent(target).unwrap()
                .top_card().card_id(&self.card_data).to_owned();
            let top_card_id = taken.card.card_id(&self.card_data).to_owned();
            let card_name = taken.card.card_name(&self.card_data).to_owned();
            self.player_mut(target.player).breeding_area.as_mut().unwrap().digivolve(taken.card, turn);
            if let CardSourceRef::Material(source, _) = source_ref {
                self.soft_remove_if_emptied(source);
            }
            let _ = self.draw_one_for_player(player_id);
            let seq = self.next_event_seq();
            self.events.push(crate::events::GameEvent::Digivolve {
                seq, player: target.player, top_card_id, card_name,
                field_index: crate::action::space::BREEDING_TARGET as u8,
                from_stack_top, was_dna: false, was_blast_dna: false,
                memory_paid: effective_cost as i16,
            });
            self.n_digivolutions[target.player as usize] += 1;
            // Q4300/Q4389/Q4429: neither WhenDigivolving nor ordinary
            // observers trigger. The caller still receives a completed result.
            return true;
        }
        self.player_mut(target.player).battle_area[target.index as usize].digivolve(taken.card, turn);

        // 4a. Soft-remove an emptied Material source. If `source_ref` was
        // `Material(src, _)`, `take_card_source_ref` may have removed the
        // source permanent's only card. Left in `battle_area`, that "zombie"
        // permanent panics any trigger fan-out that iterates all permanents
        // and calls `top_card()`. NOT a deletion: no OnDeletion fires, no
        // replacement window, no trash for the body card (already moved to
        // target). Matches DCGO's caller-side `RemoveField(permanent)`
        // pattern (e.g. Jogress at
        // `DCGO/Assets/Scripts/Script/CardController.cs:1509`). See
        // `Game::soft_remove_if_emptied` doc + the `G-PERMANENT-EMPTY-…`
        // entry in `qa/archetype-qa/engine-gaps.md`.
        let mut target = target;
        if let crate::enums::CardSourceRef::Material(src_handle, _) = source_ref {
            if self.soft_remove_if_emptied(src_handle) {
                target = Self::shift_handle_after_soft_remove(src_handle, target);
            }
        }

        let event_card = self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .map(|perm| perm.top_card().handle())
            .expect("effect digivolve target remains in battle area after stack mutation");

        // 4b. §8-1-3-3: the digivolution procedure is "places the Digimon ...
        // on top of the chosen card, draws 1 card, and the digivolution
        // process is resolved". The draw belongs to the PROCEDURE, so an
        // effect-initiated digivolve draws exactly like the main-phase action
        // — and it lands here, after the stack mutation and BEFORE the
        // WhenDigivolving fan-out, mirroring the player-action path above.
        // (§8-1-2-8 covers the empty-deck case: digivolution still happens,
        // just "without drawing a card" — which `draw()` already models.)
        let _ = self.draw_one_for_player(player_id);

        self.emit_eval_digivolve(target, effective_cost as i16, false, false);

        // 5. Fire WhenDigivolving triggers.
        self.fire_digivolve_event_triggers(target, event_card, true, false);

        true
    }
}

#[cfg(test)]
mod q1157_ordinary_cost_state_tests {
    const OBSERVER: &str = r#"
card: TEST-Q1157-OBSERVER
name: Would Evolve Cost Observer
kind: tamer
color: [white]
cost: 2
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    optional: true
    when_any_ally_digivolves_into: {card_number_is: BT4-011}
    condition:
      any_permanent: {of: you, zone: [battle_area], kind: digimon}
    amount: 1
    pay_cost:
      - suspend: {target: source}
"#;
    use crate::{
        action::{build_action_mask, encode_digivolve, space::PLAY_HAND_START},
        card_data::EvoCost,
        debug_runner::DebugRunner,
        enums::{CardKind, GamePhase},
    };
    fn setup(option: bool) -> DebugRunner {
        setup_observer(option, OBSERVER)
    }
    fn setup_observer(option: bool, observer: &str) -> DebugRunner {
        let path = std::env::var("BT4_011_TEST_YAML")
            .unwrap_or_else(|_| format!("{}/cards/bt4/BT4-011.yaml", env!("CARGO_MANIFEST_DIR")));
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(&std::fs::read_to_string(path).unwrap())
            .unwrap()
            .from_dsl_yaml(include_str!("../../cards/st1/ST1-12.yaml"))
            .unwrap()
            .from_dsl_yaml(include_str!("../../cards/st2/ST2-12.yaml"))
            .unwrap()
            .from_dsl_yaml(include_str!("../../cards/st1/ST1-03.yaml"))
            .unwrap()
            .from_dsl_yaml(include_str!("../../cards/lm/LM-027.yaml"))
            .unwrap()
            .from_dsl_yaml(observer)
            .unwrap()
            .hand(
                0,
                if option {
                    &["LM-027", "BT4-011"][..]
                } else {
                    &["BT4-011"][..]
                },
            )
            .deck(0, &["ST1-03"; 12])
            .deck(1, &["ST1-03"; 12])
            .memory(5)
            .start();
        // Exact printed circle; DebugRunner compiled metadata omits compatibility evo_costs.
        r.game
            .card_data
            .iter_mut()
            .find(|c| c.card_id == "BT4-011")
            .unwrap()
            .evo_costs = vec![EvoCost {
            card_color: 0,
            level: 3,
            memory_cost: 2,
        }];
        r.game.current_phase = GamePhase::Main;
        r
    }

    #[test]
    fn ordinary_cost_prompt_has_no_treated_declaration_or_digest_extension() {
        let mut r = setup(false);
        r.place_on_field(0, "ST1-03", None);
        r.place_on_field(0, "TEST-Q1157-OBSERVER", None);
        r.game.tick_declarative_effects();
        r.game.decode_action(encode_digivolve(0, 0), 0);
        assert!(r.game.pending_selection.is_some());
        let mut legacy_state = r.game.clone();
        legacy_state.pending_would_digivolve_resume = None;
        assert_eq!(
            r.game.verification_digest(),
            legacy_state.verification_digest(),
            "ordinary cost prompt must retain the existing hidden-state contract"
        );
        assert!(r.game.pending_would_digivolve_resume.is_none());
    }
    #[test]
    fn treated_cost_prompt_retains_owned_identity_receipt() {
        let mut r = setup(false);
        r.place_on_field(0, "ST1-12", None);
        r.place_on_field(0, "TEST-Q1157-OBSERVER", None);
        r.game.tick_declarative_effects();
        r.game.decode_action(encode_digivolve(0, 0), 0);
        assert!(r.game.pending_selection.is_some());
        assert!(r
            .game
            .pending_would_digivolve_resume
            .as_ref()
            .unwrap()
            .treated_identity_orders
            .is_some());
    }
}
