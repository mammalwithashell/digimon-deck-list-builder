//! Player-driven game actions — split out of `game.rs` for readability.
//!
//! Everything here lives in `impl Game` blocks so the call surface is unchanged.
//! This is where `play_from_hand`, `digivolve_from_hand`, `initiate_dna_digivolve`,
//! and the `activate_*_main` [Main] effect dispatchers live. All three are invoked
//! by the action decoder and the Tauri/PyO3 bindings; none of them move here.

use crate::card_source::{CardHandle, CardSource};
use crate::digixros::DigiXrosMaterialOrigin;
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::{
    CardKind, EffectSourceKind, EffectTiming, GamePhase, Keyword, ModifierType, PlaySource,
    PlayerId, Zone,
};
use crate::game::Game;
use crate::game::{
    PendingWouldDigivolveResume, PendingWouldLinkResume, PendingWouldPlayOrigin,
    PendingWouldPlayResume,
};
use crate::permanent::PermanentHandle;
use crate::selection::{
    OptionPlayResult, OptionResolutionPhase, OptionSubtype, OptionUseSource, PendingOption,
    PendingSelection, QueuedEffect, SelectionKind, TriggerSource,
};
use rand::seq::SliceRandom;

// Tier-2 operations split by mechanic (parallel to effect_context/action/).
// See docs/RUST_ENGINE_API.md §3.
mod breeding;
mod cost;
pub(crate) mod digivolve;
mod helpers; // facade-decomposition Tier-2 primitives (trash_source_and_fire, would_replacement_*, …)
mod link; // DigiLink Shape-B (Digimon-link) — activate/begin/commit/absorb + facet-#9 chosen-card link
mod misc;
mod movement;
mod options;
mod play;
mod security;
mod sources;
mod zones;

/// Source zone for `play_option_core`. Private to this module — the public
/// API is the pair of `play_option_from_hand` / `play_option_from_trash`
/// entry points.
#[derive(Clone, Copy, Debug)]
enum OptionSource {
    Hand(usize),
    Trash(usize),
}

/// Source zone for the result (fusing-in) card in an effect-initiated App Fuse
/// (`Game::commit_effect_app_fuse`). The shipped riders fuse from hand
/// (BT21-084 / BT23-079 / P-241 / BT25-089) or trash (BT24-087).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppFuseSourceZone {
    Hand,
    Trash,
}

struct TakenCardSource {
    card: CardSource,
    restore_face_up_security_for: Option<PlayerId>,
}

/// Captured state for resuming `run_digixros_leave_windows_then_commit` after a
/// battle-area DigiXros material's `WhenWouldLeaveBattleArea` window parked a
/// `pending_selection` (the material is now in the leaving/limbo slot). Once the
/// parked reward settles, the loop re-enters at `next_material` and ultimately
/// finalizes the host play (G-DIGIXROS-REDIRECT-EXTRACTION).
#[derive(Clone)]
struct DigiXrosResumeContinuation {
    player: PlayerId,
    target_card: CardHandle,
    total_reduction: i32,
    source: PlaySource,
    suppress_on_play: bool,
    next_material: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CostReductionKind {
    Play,
    Digivolve,
    OptionUse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptionCostPolicy {
    /// Pay the printed use cost (less any field-hosted `BeforePayCost`
    /// `OptionUse` reductions). Ordinary "use this Option" path.
    Pay,
    /// Effect-driven "use 1 Option without paying its cost" — pay 0 memory
    /// regardless of printed cost, while preserving the full Option lifecycle.
    Free,
    /// Effect-driven "use 1 Option with the cost reduced by N" — pay
    /// `max(0, printed_use_cost - field_reductions - N)`. The flat reduction
    /// `N` STACKS on top of the field-hosted `BeforePayCost` reductions the
    /// `Pay` path already consults (same precedent as `CostDelta::Reduce` on
    /// the play half). Used by the unified `play_or_use_from_hand_with_cost`
    /// helper for "play or use … with the cost reduced by 3" wording
    /// (ST23-04 / ST23-08 / BT25-041). `G-PLAY-OR-USE-FROM-HAND`.
    Reduce(i16),
    /// Effect-driven "use 1 Option for exactly N memory" — pay `max(0, N)`,
    /// ignoring the printed use cost and field reductions. Mirrors
    /// `CostDelta::Fixed` on the play half. No shipped card needs it yet; it
    /// exists so the unified helper can route every `CostDelta` variant.
    Fixed(i16),
}

impl OptionSource {
    fn use_source(self) -> OptionUseSource {
        match self {
            OptionSource::Hand(_) => OptionUseSource::Hand,
            OptionSource::Trash(_) => OptionUseSource::Trash,
        }
    }
}

/// One available play mode for an Option card. `classify_option_modes`
/// derives the set of modes from the card's effect list. Most Options have
/// exactly one mode; a dual-mode Plug-In Option that is both a Standard
/// `[Main]` Option and a Link Option has two (`Standard`, then `Link`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OptionPlayMode {
    /// Played as a normal `[Main]` Option — pay the printed use cost,
    /// resolve the `OptionMain` body, dispose to trash.
    Standard,
    /// A Delay Option — parks on the field until its delay trigger.
    Delay(crate::enums::DelayTrigger),
    /// Plugged in via Link Requirements — pay `cost`, attach sideways to a
    /// host Digimon. No `[Main]` / `[Security]` effect runs.
    Link { cost: u16 },
    /// A Training Option.
    Training,
}

impl OptionPlayMode {
    /// The disposal subtype this play mode resolves to.
    fn subtype(self) -> OptionSubtype {
        match self {
            OptionPlayMode::Standard => OptionSubtype::Standard,
            OptionPlayMode::Delay(trigger) => OptionSubtype::Delay(trigger),
            OptionPlayMode::Link { .. } => OptionSubtype::Link,
            OptionPlayMode::Training => OptionSubtype::Training,
        }
    }

    fn is_link(self) -> bool {
        matches!(self, OptionPlayMode::Link { .. })
    }
}

fn source_kind_for_card_kind(kind: CardKind) -> EffectSourceKind {
    match kind {
        CardKind::Digimon | CardKind::DigiEgg | CardKind::Dual => EffectSourceKind::Digimon,
        CardKind::Tamer => EffectSourceKind::Tamer,
        CardKind::Option => EffectSourceKind::Option,
        CardKind::Token => EffectSourceKind::Rule,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CostReductionKey {
    source_card: crate::card_source::CardHandle,
    source_permanent: Option<crate::permanent::PermanentHandle>,
    controller: PlayerId,
    card_id: String,
    effect_slot: u8,
    is_under: bool,
}

struct CostReductionCandidate {
    key: CostReductionKey,
    label: String,
    amount: i32,
    optional: bool,
    has_pay_cost: bool,
    /// The `pay_cost` begins with a declinable selection, so it carries its
    /// own opt-in/opt-out and can be auto-applied without the redundant
    /// acceptance gate. Mirrors `Effect::pay_cost_self_gated`.
    pay_cost_self_gated: bool,
    /// The `pay_cost` installs a selection (parks). Mirrors
    /// `Effect::pay_cost_interactive`. The digivolve / Option-use synchronous
    /// scan routes such a reducer through a dedicated interactive prompt.
    /// `G-COST-REDUCTION-INTERACTIVE-PAY-COST`.
    pay_cost_interactive: bool,
}

struct BeforePayCostSourceInfo {
    source_permanent: Option<crate::permanent::PermanentHandle>,
    source_card: crate::card_source::CardHandle,
    card_id: String,
    is_under: bool,
    controller: PlayerId,
    effect_slot: u8,
}

#[derive(Debug, Clone, Copy)]
struct CostTargetContext {
    card: crate::card_source::CardHandle,
    from_hand: bool,
    /// True when this cost is a DIGIVOLVE cost (normal or DNA). Surfaced to
    /// predicates via `EffectReadContext::cost_is_digivolve` so the
    /// `when_any_ally_digivolves_into` cost-reduction trigger fires only for
    /// digivolutions. `G-COST-REDUCTION-DIGIVOLVE-INTO`.
    is_digivolve: bool,
    /// Permanents being digivolved (single entry for normal digivolve,
    /// two for DNA digivolve; both `None` for play-from-hand / option
    /// use). Fixed-size to preserve `Copy`; surfaced to predicates via
    /// `EffectReadContext::cost_target_permanents` as a `Vec`. Used by
    /// the `source_is_cost_target_permanent` predicate
    /// (G-BEFORE-PAY-COST-DIGIVOLVE-TARGET).
    target_permanents: [Option<crate::permanent::PermanentHandle>; 2],
}

impl CostTargetContext {
    fn target_permanents_vec(&self) -> Vec<crate::permanent::PermanentHandle> {
        self.target_permanents.iter().filter_map(|h| *h).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayFromHandCostResult {
    Played(usize),
    Pending,
    Failed,
}

impl PlayFromHandCostResult {
    fn into_option(self) -> Option<usize> {
        match self {
            PlayFromHandCostResult::Played(index) => Some(index),
            PlayFromHandCostResult::Pending | PlayFromHandCostResult::Failed => None,
        }
    }
}

/// Inspect an Option's effect list to derive every available play mode.
///
/// Returns a 1- or 2-element list. `Delay` and `Training` are exclusive
/// whole-card subtypes (a card carrying either has exactly that one mode).
/// Otherwise the card may have a Standard `[Main]` mode (it carries a
/// non-link `OptionMain` body) and/or a Link mode (it carries a
/// `link_requirement` effect with a `link_cost`). A Plug-In Option that
/// has both is **dual-mode**: the list is `[Standard, Link]`, in that
/// order, and the player picks via a mode-select prompt.
fn classify_option_modes(effects: &[crate::effect::Effect]) -> Vec<OptionPlayMode> {
    let mut delay = None;
    let mut training = false;
    let mut link_cost: Option<u16> = None;
    let mut has_standard_main = false;
    for eff in effects {
        if let Some(trigger) = eff.delay_trigger {
            delay = Some(trigger);
        }
        if eff.training {
            training = true;
        }
        if let Some(cost) = eff.link_cost {
            link_cost = Some(cost);
        } else if eff.timing == EffectTiming::OptionMain {
            // A non-link `OptionMain` effect is a Standard `[Main]` body.
            has_standard_main = true;
        }
    }
    // Delay / Training are exclusive whole-card subtypes.
    if let Some(trigger) = delay {
        return vec![OptionPlayMode::Delay(trigger)];
    }
    if training {
        return vec![OptionPlayMode::Training];
    }
    let mut modes = Vec::new();
    // A card with no link effect is always Standard (the fallback for
    // `[Security]`-only Options too); a card with a link effect is
    // Standard only when it additionally carries a `[Main]` body.
    if has_standard_main || link_cost.is_none() {
        modes.push(OptionPlayMode::Standard);
    }
    if let Some(cost) = link_cost {
        modes.push(OptionPlayMode::Link { cost });
    }
    modes
}

impl Game {
    pub(crate) fn play_from_hand_with_cost_result(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
        cost_target_from_hand: bool,
    ) -> PlayFromHandCostResult {
        self.play_from_hand_with_cost_result_from_origin(
            player_id,
            hand_index,
            cost_delta,
            source,
            cost_target_from_hand,
            PendingWouldPlayOrigin::Hand,
        )
    }

    pub(crate) fn play_from_hand_with_cost_result_from_origin(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
        cost_target_from_hand: bool,
        origin: PendingWouldPlayOrigin,
    ) -> PlayFromHandCostResult {
        self.play_from_hand_with_cost_result_from_origin_suppress(
            player_id,
            hand_index,
            cost_delta,
            source,
            cost_target_from_hand,
            origin,
            false,
        )
    }

    /// As [`Self::play_from_hand_with_cost_result_from_origin`], but threads a
    /// `suppress_on_play` flag (PUPPETS-G030). When `true`, the just-played
    /// permanent's own `[On Play]` effects are skipped for this play event;
    /// every other timing and every other permanent are unaffected.
    pub(crate) fn play_from_hand_with_cost_result_from_origin_suppress(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
        cost_target_from_hand: bool,
        origin: PendingWouldPlayOrigin,
        suppress_on_play: bool,
    ) -> PlayFromHandCostResult {
        let field_slots = self.rules.field_slots;
        // Borrow-check-friendly pre-checks: gather everything we need from
        // immutable borrows before taking a mutable borrow.
        let card_kind = {
            let player = self.player(player_id);
            if hand_index >= player.hand.len() {
                return PlayFromHandCostResult::Failed;
            }
            if player.battle_area.len() >= field_slots as usize {
                return PlayFromHandCostResult::Failed;
            }
            let card = &player.hand[hand_index];
            card.card_kind(&self.card_data)
        };

        // Phase 6: CannotPlayDigimonByEffect — when source is ByEffect and the
        // card is a Digimon, gate on the player-scoped modifier.
        if source == PlaySource::ByEffect
            && card_kind == CardKind::Digimon
            && self
                .modifiers
                .player_has(player_id, ModifierType::CannotPlayDigimonByEffect)
        {
            return PlayFromHandCostResult::Failed;
        }

        // Phase 6: CannotPlayTamerByEffect — when source is ByEffect and the
        // card is a Tamer, gate on the player-scoped modifier.
        if source == PlaySource::ByEffect
            && card_kind == CardKind::Tamer
            && self
                .modifiers
                .player_has(player_id, ModifierType::CannotPlayTamerByEffect)
        {
            return PlayFromHandCostResult::Failed;
        }

        self.pending_digixros_transaction = match origin {
            PendingWouldPlayOrigin::Hand => {
                self.build_digixros_transaction_for_hand_card(player_id, hand_index)
            }
            _ => None,
        };

        let target_card = self.player(player_id).hand[hand_index].handle();
        self.continue_play_from_hand_cost_reduction_chain(
            player_id,
            hand_index,
            CostTargetContext {
                card: target_card,
                from_hand: cost_target_from_hand,
                is_digivolve: false,
                target_permanents: [None, None],
            },
            cost_delta,
            source,
            origin,
            suppress_on_play,
            0,
            Vec::new(),
        )
    }

    fn pending_digixros_material_candidates(
        &self,
        player_id: PlayerId,
    ) -> Vec<(u16, DigiXrosMaterialOrigin)> {
        use crate::action::space::{
            encode_source_select, HAND_EFFECT_START, HAND_MAIN_LIMIT, MAX_FIELD_SLOTS,
            TRASH_EFFECT_START, TRASH_MAIN_LIMIT,
        };

        let Some(transaction) = self.pending_digixros_transaction.as_ref() else {
            return Vec::new();
        };
        if transaction.controller != player_id {
            return Vec::new();
        }

        let player = self.player(player_id);
        let mut candidates = Vec::new();

        for (index, card) in player.hand.iter().enumerate().take(HAND_MAIN_LIMIT) {
            if card.handle() == transaction.played_card {
                continue;
            }
            let origin = DigiXrosMaterialOrigin::Hand {
                player: player_id,
                index,
                card: card.handle(),
            };
            if transaction
                .validate_material_origin(origin, &self.card_data[card.data_index])
                .is_ok()
            {
                candidates.push((HAND_EFFECT_START + index as u16, origin));
            }
        }

        for (index, permanent) in player
            .battle_area
            .iter()
            .enumerate()
            .take(MAX_FIELD_SLOTS as usize)
        {
            let card = permanent.top_card();
            let origin = DigiXrosMaterialOrigin::BattleArea {
                permanent: PermanentHandle {
                    player: player_id,
                    index: index as u8,
                },
                card: card.handle(),
            };
            if transaction
                .validate_material_origin(origin, &self.card_data[card.data_index])
                .is_ok()
            {
                candidates.push((index as u16, origin));
            }
        }

        for (index, card) in player.trash.iter().enumerate().take(TRASH_MAIN_LIMIT) {
            let origin = DigiXrosMaterialOrigin::Trash {
                player: player_id,
                index,
                card: card.handle(),
            };
            if transaction
                .validate_material_origin(origin, &self.card_data[card.data_index])
                .is_ok()
            {
                candidates.push((TRASH_EFFECT_START + index as u16, origin));
            }
        }

        for (field_index, permanent) in player
            .battle_area
            .iter()
            .enumerate()
            .take(MAX_FIELD_SLOTS as usize)
        {
            if permanent.top_card().card_kind(&self.card_data) != CardKind::Tamer {
                continue;
            }
            for (source_index, card) in permanent.card_sources.iter().enumerate() {
                let Some(action_id) = encode_source_select(field_index as u16, source_index as u16)
                else {
                    continue;
                };
                let origin = DigiXrosMaterialOrigin::UnderTamer {
                    tamer: PermanentHandle {
                        player: player_id,
                        index: field_index as u8,
                    },
                    source_index,
                    card: card.handle(),
                };
                if transaction
                    .validate_material_origin(origin, &self.card_data[card.data_index])
                    .is_ok()
                {
                    candidates.push((action_id, origin));
                }
            }
        }

        candidates
    }

    // ── Assembly play execution (G-ASSEMBLY-PLAY-EXECUTION) ──────────────
    //
    // Faithful to DCGO `SelectAssemblyClass.cs`: a hand card with an
    // `assembly` alt-path may be played by placing its named materials from
    // the controller's TRASH under it (digivolution-stack bottom) and paying
    // a reduced cost. The flow rides the normal PLAY_HAND action (no new
    // action-space range): after the cost-reduction chain resolves we offer
    // an optional gate ("use the assembly pieces from trash?") followed by a
    // per-element surfaced trash selection. Declining the gate plays the card
    // at full cost. See change `fix-ad1-025-assembly-data`.

    /// Resolve a hand card's eligible `assembly` alt-path: returns the cost
    /// reduction (D5) and the per-element `(filter, count)` list when the card
    /// carries an assembly path whose materials are each satisfiable from the
    /// controller's trash with DISTINCT cards. `None` when the card is not
    /// assembly-capable or its materials are unavailable. Shared by the play
    /// flow (`try_begin_assembly_flow`) and the action mask (declare-then-pay
    /// legality against the reduced cost).
    #[cfg(feature = "dsl-yaml-loader")]
    fn resolve_eligible_assembly(
        &self,
        player_id: PlayerId,
        hand_index: usize,
        target_card: CardHandle,
    ) -> Option<(i32, Vec<(digimon_dsl::compiled::CompiledPredicate, u8)>)> {
        use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost, CompiledRepeat, CompiledZone};

        let card_id = {
            let card = self.player(player_id).hand.get(hand_index)?;
            if card.handle() != target_card {
                return None;
            }
            card.card_id(&self.card_data).to_string()
        };

        let path = self
            .alt_path_registry
            .get(&card_id)?
            .iter()
            .find(|p| matches!(p.kind, CompiledAltPathKind::Assembly))?;

        // D5: an assembly alt-path `cost:` is the REDUCTION amount.
        let assembly_reduction = match &path.cost {
            None => 0,
            Some(CompiledCost::Literal(n)) => *n,
            // Formula reductions are not yet supported for assembly.
            Some(CompiledCost::Formula(_)) => return None,
        };

        // Per-element (filter, count). Assembly materials come from trash and
        // stack under the played card.
        let mut elements: Vec<(digimon_dsl::compiled::CompiledPredicate, u8)> = Vec::new();
        for m in &path.materials {
            if !m.stack_under || !m.zones.iter().any(|z| matches!(z, CompiledZone::Trash)) {
                return None; // unsupported material shape for assembly
            }
            let count = match &m.repeat {
                None => 1u8,
                Some(CompiledRepeat::Range { min, max }) if min == max => *max,
                // Variable / unbounded counts not supported for assembly yet.
                _ => return None,
            };
            elements.push((m.filter.clone(), count));
        }
        if elements.is_empty() {
            return None;
        }

        // Eligibility: each element satisfiable from trash with distinct cards
        // (DCGO `CanFulfillConditions` / recursive distinct-assignment check).
        if !self.assembly_can_fulfill(player_id, target_card, &elements) {
            return None;
        }

        Some((assembly_reduction, elements))
    }

    /// The assembly cost-reduction available for hand card `hand_index` right
    /// now, or `0` when the card has no eligible assembly path. Used by the
    /// action mask to offer the play under declare-then-pay legality against
    /// the REDUCED cost (Q5). Always `0` without the DSL loader feature.
    pub(crate) fn assembly_play_reduction_for_hand_card(
        &self,
        player_id: PlayerId,
        hand_index: usize,
    ) -> i32 {
        #[cfg(feature = "dsl-yaml-loader")]
        {
            let Some(target) = self
                .player(player_id)
                .hand
                .get(hand_index)
                .map(|c| c.handle())
            else {
                return 0;
            };
            if let Some((reduction, _)) =
                self.resolve_eligible_assembly(player_id, hand_index, target)
            {
                return reduction.max(0);
            }
        }
        #[cfg(not(feature = "dsl-yaml-loader"))]
        {
            let _ = (player_id, hand_index);
        }
        0
    }

    /// True when each assembly element can be matched to a DISTINCT card in
    /// the controller's trash (a system of distinct representatives exists).
    #[cfg(feature = "dsl-yaml-loader")]
    fn assembly_can_fulfill(
        &self,
        player_id: PlayerId,
        source_card: CardHandle,
        elements: &[(digimon_dsl::compiled::CompiledPredicate, u8)],
    ) -> bool {
        use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};

        let rctx = EffectReadContext::new(self, source_card, None, player_id);
        let trash = &self.player(player_id).trash;

        // Expand elements into one slot per required card.
        let mut slots: Vec<&digimon_dsl::compiled::CompiledPredicate> = Vec::new();
        for (pred, count) in elements {
            for _ in 0..*count {
                slots.push(pred);
            }
        }
        // For each slot, the trash indices that satisfy its filter.
        let match_table: Vec<Vec<usize>> = slots
            .iter()
            .map(|pred| {
                trash
                    .iter()
                    .enumerate()
                    .filter(|(_, cs)| {
                        eval_predicate(pred, &rctx, PredicateSubject::Card(cs.handle()))
                    })
                    .map(|(i, _)| i)
                    .collect()
            })
            .collect();

        let mut used = vec![false; trash.len()];
        assembly_assign(&match_table, 0, &mut used)
    }

    pub(crate) fn commit_pending_would_play(
        &mut self,
        outcome: crate::replacement::ReplacementOutcome,
    ) {
        let Some(resume) = self.pending_would_play_resume.take() else {
            return;
        };
        match outcome {
            crate::replacement::ReplacementOutcome::None => {
                let _ = self.commit_play_from_hand_card_no_replace(
                    resume.player,
                    resume.card,
                    resume.effective_cost,
                    resume.effect_initiated,
                    resume.suppress_on_play,
                );
                self.pending_digixros_transaction = None;
            }
            crate::replacement::ReplacementOutcome::Cancelled
            | crate::replacement::ReplacementOutcome::CustomHandled => {
                self.restore_pending_would_play_origin(resume);
                self.pending_digixros_transaction = None;
            }
            crate::replacement::ReplacementOutcome::Redirected(_)
            | crate::replacement::ReplacementOutcome::Substituted(_) => {
                self.restore_pending_would_play_origin(resume);
                self.pending_digixros_transaction = None;
            }
        }
    }

    /// Candidate host Digimon for a Link Option attach: every Standard-state
    /// own Digimon that is below its link cap and passes the card's
    /// `link_filter`. Shared by `dispose_option`'s Link arm and the
    /// dual-mode legality check in `option_legal_play_modes`.
    pub(crate) fn link_host_candidates(
        &self,
        owner: PlayerId,
        source_card: crate::card_source::CardHandle,
        effects: &[crate::effect::Effect],
    ) -> Vec<PermanentHandle> {
        let mut out: Vec<PermanentHandle> = Vec::new();
        for (i, perm) in self.player(owner).battle_area.iter().enumerate() {
            let handle = PermanentHandle {
                player: owner,
                index: i as u8,
            };
            if !self.permanent_is_digimon_for_rules(handle) {
                continue;
            }
            if !matches!(perm.option_state, crate::permanent::OptionState::Standard) {
                continue;
            }
            let link_max =
                (5 + self.modifiers.link_max_delta(handle)).clamp(0, u8::MAX as i32) as usize;
            if perm.linked_cards.len() >= link_max {
                continue;
            }
            let filter_ok =
                effects
                    .iter()
                    .find(|e| e.link_cost.is_some())
                    .map_or(true, |link_effect| {
                        if let Some(f) = &link_effect.link_filter {
                            let read_ctx = EffectReadContext::new(self, source_card, None, owner);
                            f(&read_ctx, handle)
                        } else {
                            true
                        }
                    });
            if filter_ok {
                out.push(handle);
            }
        }
        out
    }

    /// The set of play modes `player_id` may **afford** for `card` right
    /// now. A dual-mode Plug-In Option yields `[Standard, Link]` when both
    /// fit the memory budget (the player then picks via the mode-select);
    /// a single affordable mode plays directly; an empty result means the
    /// Option cannot be played at all.
    ///
    /// Only affordability is filtered here — host availability for a Link
    /// play is resolved later by `dispose_option` (a Link play with no
    /// eligible host trashes the card, identical to a single-mode Link
    /// Option). This keeps `PLAY_HAND` masking and the mode-select offer
    /// consistent with the engine's existing Link-Option contract.
    pub(crate) fn option_legal_play_modes(
        &self,
        card: &CardSource,
        player_id: PlayerId,
    ) -> Vec<OptionPlayMode> {
        let effects = self
            .effects_for_card(card.card_id(&self.card_data), card.handle())
            .unwrap_or_default();
        let use_cost = card
            .option_use_cost(&self.card_data)
            .unwrap_or_else(|| card.play_cost(&self.card_data));
        let memory_min = self.rules.memory_range.0;
        classify_option_modes(&effects)
            .into_iter()
            .filter(|mode| {
                let cost = match mode {
                    OptionPlayMode::Link { cost } => (*cost as i32
                        + self.modifiers.link_cost_delta_for_player(player_id))
                    .max(0) as i16,
                    _ => use_cost as i16,
                };
                (self.memory - cost) >= memory_min
            })
            .collect()
    }

    pub(crate) fn pending_option_can_arts_digivolve(&self) -> bool {
        let Some(pending) = self.pending_option.as_ref() else {
            return false;
        };
        if pending.card.card_kind(&self.card_data) != CardKind::Dual {
            return false;
        }
        let data = &self.card_data[pending.card.data_index];
        data.dual
            .as_ref()
            .map(|dual| {
                data.keywords.contains(&Keyword::ArtsDigivolve)
                    || dual.option.keywords.contains(&Keyword::ArtsDigivolve)
                    || dual.digimon.keywords.contains(&Keyword::ArtsDigivolve)
            })
            .unwrap_or(false)
    }

    fn arts_digivolve_battle_targets(&self, owner: PlayerId) -> Vec<PermanentHandle> {
        let Some(pending) = self.pending_option.as_ref() else {
            return Vec::new();
        };
        let player = self.player(owner);
        player
            .battle_area
            .iter()
            .enumerate()
            .filter_map(|(i, perm)| {
                let handle = PermanentHandle {
                    player: owner,
                    index: i as u8,
                };
                if self.modifiers.has(handle, ModifierType::CannotDigivolve) {
                    return None;
                }
                if self.can_digivolve(&pending.card, perm) {
                    Some(handle)
                } else {
                    None
                }
            })
            .collect()
    }

    fn arts_digivolve_has_breeding_target(&self, owner: PlayerId) -> bool {
        let Some(pending) = self.pending_option.as_ref() else {
            return false;
        };
        let Some(breeding) = self.player(owner).breeding_area.as_ref() else {
            return false;
        };
        self.can_digivolve(&pending.card, breeding)
    }

    pub(crate) fn install_arts_digivolve_selection(&mut self) -> bool {
        use crate::action::space::encode_attack;

        let Some(pending) = self.pending_option.as_ref() else {
            return false;
        };
        let owner = pending.owner;
        let source_card = pending.card.handle();
        let targets = self.arts_digivolve_battle_targets(owner);
        let has_breeding = self.arts_digivolve_has_breeding_target(owner);
        if targets.is_empty() && !has_breeding {
            return false;
        }

        let mut valid_action_ids: Vec<u16> = targets
            .iter()
            .map(|h| encode_attack(0, h.index as u16))
            .collect();
        if has_breeding {
            valid_action_ids.push(crate::action::space::BREEDING_SELECTION_TARGET);
        }
        let target_snapshot = targets.clone();
        let previous_phase = self.current_phase;

        if let Some(pending) = self.pending_option.as_mut() {
            pending.resolution_phase = OptionResolutionPhase::ArtsSelectTarget;
        }
        self.current_phase = GamePhase::SelectTarget;
        self.pending_selection = Some(PendingSelection {
            zone_owner: None,
            kind: SelectionKind::OwnField,
            selecting_player: owner,
            previous_phase,
            valid_action_ids,
            is_optional: true,
            prompt: "Choose a card for Arts Digivolve, or pass to trash this Option".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: None,
            source_kind: EffectSourceKind::Option,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                use crate::action::space::{ATTACK_START, TARGETS_PER_ATTACKER};
                if action_id == crate::action::space::BREEDING_SELECTION_TARGET {
                    let _ = game.arts_digivolve_pending_option_onto_breeding(owner);
                    return;
                }
                let offset = action_id.saturating_sub(ATTACK_START);
                let target_index = (offset % TARGETS_PER_ATTACKER) as u8;
                if target_snapshot.iter().any(|h| h.index == target_index) {
                    let target = PermanentHandle {
                        player: owner,
                        index: target_index,
                    };
                    let _ = game.arts_digivolve_pending_option_onto_battle(target);
                }
            }),
            on_decline: Some(Box::new(|game: &mut Game| {
                game.dispose_option();
                game.check_turn_end();
            })),
        });
        true
    }

    pub(crate) fn arts_digivolve_pending_option_onto_battle(
        &mut self,
        target: PermanentHandle,
    ) -> bool {
        if !self.pending_option_can_arts_digivolve() {
            return false;
        }
        let Some(pending_ref) = self.pending_option.as_ref() else {
            return false;
        };
        if pending_ref.owner != target.player {
            return false;
        }
        let Some(perm) = self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
        else {
            return false;
        };
        if self.modifiers.has(target, ModifierType::CannotDigivolve) {
            return false;
        }
        if !self.can_digivolve(&pending_ref.card, perm) {
            return false;
        }
        // Q3 (G-DIGIVOLVE-TARGET-RESTRICTION): honor a `CanOnlyDigivolveInto`
        // restriction on the arts-digivolve base too (the restriction applies to
        // every digivolve route, not just normal digivolve).
        if self.digivolve_target_blocked_by_restriction(target, &pending_ref.card) {
            return false;
        }

        let pending = self.pending_option.take().expect("checked above");
        let arts_card_id = pending.card.card_id(&self.card_data).to_string();
        let arts_card_handle = pending.card.handle();
        let arts_owner = pending.owner;
        let turn = self.turn_count;
        self.player_mut(target.player).battle_area[target.index as usize]
            .digivolve(pending.card, turn);
        self.player_mut(target.player).draw();

        // Arts digivolve runs the ≤0-DP rules-check immediately (before the
        // WhenDigivolving enqueue) because the next branch keys off whether the
        // Arts target survived the digivolve. The trailing `drain_effect_queue`
        // re-runs the general check at the resolution boundary.
        self.run_state_based_rules_check();

        if self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .is_some()
        {
            self.enqueue_triggered(
                EffectTiming::WhenDigivolving,
                TriggerSource::Permanent(target),
            );
        } else {
            self.enqueue_when_digivolving_from_arts_card(
                &arts_card_id,
                arts_card_handle,
                arts_owner,
            );
        }
        self.drain_effect_queue();
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                EffectTiming::OnDigivolve,
                TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }
        self.drain_effect_queue();
        self.check_turn_end();
        true
    }

    pub(crate) fn arts_digivolve_pending_option_onto_breeding(&mut self, owner: PlayerId) -> bool {
        if !self.pending_option_can_arts_digivolve() {
            return false;
        }
        let Some(pending_ref) = self.pending_option.as_ref() else {
            return false;
        };
        if pending_ref.owner != owner {
            return false;
        }
        let Some(breeding) = self.player(owner).breeding_area.as_ref() else {
            return false;
        };
        if !self.can_digivolve(&pending_ref.card, breeding) {
            return false;
        }

        let pending = self.pending_option.take().expect("checked above");
        let turn = self.turn_count;
        if let Some(breeding) = self.player_mut(owner).breeding_area.as_mut() {
            breeding.digivolve(pending.card, turn);
        }
        self.player_mut(owner).draw();
        self.check_turn_end();
        true
    }

    /// General state-based ≤0-DP rules-check (`G-NO-GENERAL-ZERO-DP-RULES-CHECK`).
    ///
    /// Deletes every battle-area Digimon whose effective DP is ≤ 0 — the
    /// game's "checkup" / DCGO `CheckDeleteDigimon` state-based action. This is
    /// a SINGLE pass: it collects the current ≤0-DP Digimon and deletes them
    /// (highest index first, per player, so removals don't shift pending
    /// handles). The fixpoint loop (re-check after `OnDeletion` handlers and
    /// aura expiry mutate DP) lives in `drain_effect_queue`'s outermost-drain
    /// wrapper, which calls this only at top-level resolution boundaries — never
    /// mid-effect.
    ///
    /// Returns `true` if it deleted at least one Digimon (drives the wrapper's
    /// fixpoint). Deletion routes through `delete_permanent_with_effects` (the
    /// batched flow + inferred cause), so `OnDeletion` handlers fire post-trash
    /// per CLAUDE.md §25.
    ///
    /// Idempotent: a handle whose slot is already empty is filtered by the
    /// batched-deletion entrypoint, so a re-run is a no-op.
    pub(crate) fn run_state_based_rules_check(&mut self) -> bool {
        // Per-permanent rule-check action. Both §17-1-3-1 and §17-1-3-2 are
        // evaluated in one scan; they target disjoint permanents (a ≤0-DP
        // Digimon has a DP value, a DP-less top has none).
        enum RuleAction {
            /// §17-1-3-1 — a Digimon at ≤0 DP is DELETED (fires OnDeletion).
            DeleteZeroDp,
            /// §17-1-3-2 — a DP-less card (a Digi-Egg) exposed as the live top
            /// of a battle-area permanent "is considered to not be in any area
            /// and is trashed." This is a TRASH of the whole permanent, NOT a
            /// deletion-from-battle-area, so it fires no OnDeletion observers.
            TrashDpLess,
        }
        let mut actions: Vec<(PermanentHandle, RuleAction)> = Vec::new();
        for pid in 0..self.players.len() {
            for idx in 0..self.players[pid].battle_area.len() {
                // Skip transiently-empty (zombie) slots. A permanent mid-cleanup
                // (a just-emptied DigiXros source carrier, a trashed last
                // card_source) can have 0 card_sources before its slot is
                // removed. `permanent_is_digimon_for_rules` / `effective_dp`
                // read `top_card()`, which panics on an empty stack — and an
                // empty slot is not a live Digimon, so it is never a ≤0-DP
                // deletion candidate anyway.
                if self.players[pid].battle_area[idx].card_sources.is_empty() {
                    continue;
                }
                let handle = PermanentHandle {
                    player: pid as PlayerId,
                    index: idx as u8,
                };
                // §17-1-3-2 — "A Digimon without DP in the battle area is
                // considered to not be in any area and is trashed." The rule
                // keys on the ABSENCE of a DP value (general_rule.pdf
                // §17-1-3-2; DCGO `Permanent.HasDP` likewise only fails for
                // a top that lacks DP), NOT on the Digi-Egg card type: the
                // canonical case is a stack peeled/de-digivolved down to its
                // egg (every printed Digi-Egg has no DP), while a
                // Digimon-presenting top WITH a DP value is a legal
                // battle-area Digimon. A legitimately-played Tamer/Option
                // permanent is NOT caught (its top kind is Tamer/Option,
                // governed by §17-1-3-3/-4 instead).
                let presents_as_digimon = matches!(
                    self.players[pid].battle_area[idx]
                        .top_card()
                        .card_kind(&self.card_data),
                    crate::enums::CardKind::Digimon
                        | crate::enums::CardKind::DigiEgg
                        | crate::enums::CardKind::Dual
                );
                if presents_as_digimon && self.effective_dp(handle).is_none() {
                    actions.push((handle, RuleAction::TrashDpLess));
                    continue;
                }
                if self.permanent_is_digimon_for_rules(handle)
                    && self.effective_dp(handle).unwrap_or(1) <= 0
                {
                    actions.push((handle, RuleAction::DeleteZeroDp));
                }
            }
        }
        if actions.is_empty() {
            return false;
        }
        // Apply in descending (player, index) order so each removal — which
        // shifts higher battle-area indices via `Vec::remove` — does not
        // invalidate the handles still pending.
        for (handle, action) in actions.into_iter().rev() {
            if let RuleAction::TrashDpLess = action {
                // §17-1-3-2 trash: route every stacked card to the owner's trash
                // with no deletion/leave triggers ("not considered trashing from
                // the battle area").
                self.trash_permanent_stack(handle.player, handle.index as usize);
                continue;
            }
            // A ≤0-DP deletion is a STATE-BASED RULE action, not an effect.
            // `infer_deletion_cause` would otherwise attribute it to
            // OwnEffect/OpponentEffect (no battle/security/effect-source is
            // live between top-level effects), which is wrong for observers
            // that distinguish "deleted by having 0 DP" from "deleted by an
            // effect" (e.g. BT16-101's [All Turns] gain-2-memory clause keys
            // on battle OR 0-DP, NOT effect deletion). Refine the OnDeletion /
            // OnAnyDeletion observer payload to `EventCause::Rule` via the
            // same override slot Overclock uses (game_phases.rs); it only
            // refines the observer cause — replacement-window filtering still
            // reads `current_deletion_cause` — and the deferred-replacement
            // path (e.g. an Armor Purge window opened by this deletion)
            // threads it through ParkedReplacement.
            let previous = self.current_deletion_event_cause_override;
            self.current_deletion_event_cause_override =
                Some(crate::trigger_context::EventCause::Rule);
            self.delete_permanent_with_effects(handle);
            self.current_deletion_event_cause_override = previous;
        }
        true
    }

    /// Dispose an Option that has finished resolving its `OptionMain`
    /// body. Branches on the card's subtype:
    ///
    /// - **Standard** — route to the owner's trash through Phase 7's
    ///   `WhenWouldBeTrashed` replacement window (cause=Cost). A mandatory
    ///   cancel keeps the card in the owner's hand; a redirect routes to
    ///   Deck (bottom) or Hand. An optional replacement installs a
    ///   `PendingSelection` and re-parks `pending_option` in `Disposing`
    ///   so `advance_pending_option` can commit once the selection
    ///   resolves.
    /// - **Delay** — park on the owner's battle_area as a Permanent with
    ///   `OptionState::Delayed`. The end-of-turn scan in
    ///   [`Game::scan_delayed_options_at_end_of_turn`] fires `DelayEffect`
    ///   and trashes via `delete_permanent_with_cause(Cost)` when
    ///   `turn_count == trash_on_turn`. That delete path fires
    ///   `WhenWouldLeaveBattleArea` + `WhenWouldBeDeleted` (Phase 7
    ///   integration for Delay flows through the Permanent fire-site, not
    ///   `WhenWouldBeTrashed`).
    /// - **Link** — install host-select prompt; the selection callback
    ///   calls `attach_linked_card` directly.
    /// - **Training** — park as `OptionState::Training` on the owner's
    ///   battle_area.
    pub(crate) fn dispose_option(&mut self) {
        let Some(pending) = self.pending_option.take() else {
            return;
        };

        let card_id = pending.card.card_id(&self.card_data).to_string();
        let effects = self
            .effects_for_card(&card_id, pending.card.handle())
            .unwrap_or_default();
        // The disposal subtype was fixed at play time (`play_option_core`
        // stores the resolved mode on `pending_option`) — a dual-mode
        // Plug-In Option must not be re-classified here.
        let subtype = pending.subtype;

        match subtype {
            OptionSubtype::Standard => {
                use crate::replacement::{ReplacementCause, ReplacementSubject};

                // Phase 8 Task 6: route the dispose-trash through
                // `try_replace(WhenWouldBeTrashed, ...)`. Cause is Cost
                // (the Option was played from hand/trash and is being
                // disposed as part of the play cost/resolution). Source
                // zone reflects where the Option was used from.
                let card_handle = pending.card.handle();
                let subject = ReplacementSubject::Card(card_handle, pending.source_kind.zone());
                self.pending_option = Some(pending);
                let outcome = self.try_replace(
                    EffectTiming::WhenWouldBeTrashed,
                    subject,
                    ReplacementCause::Cost,
                    Some(crate::enums::Zone::Trash),
                );
                let Some(pending) = self.pending_option.take() else {
                    return;
                };

                if self.pending_selection.is_some() {
                    // Optional replacement installed a selection. Re-park
                    // `pending_option` in `Disposing` so
                    // `advance_pending_option` can commit the trash
                    // outcome once the selection resolves.
                    self.pending_option = Some(PendingOption {
                        owner: pending.owner,
                        card: pending.card,
                        source_kind: pending.source_kind,
                        resolution_phase: OptionResolutionPhase::Disposing,
                        subtype: pending.subtype,
                    });
                    return;
                }

                self.commit_option_trash_outcome(pending, outcome);
            }
            OptionSubtype::Delay(trigger) => {
                let owner = pending.owner;
                let placed_card = pending.card.handle();
                let trash_turn = self.compute_delay_trash_turn(pending.owner, trigger);
                let turn = self.turn_count;
                let mut perm = crate::permanent::Permanent::new(pending.card, turn);
                perm.option_state = crate::permanent::OptionState::Delayed {
                    owner,
                    trash_on_turn: trash_turn,
                    trigger,
                    placed_on_turn: turn,
                };
                self.player_mut(owner).battle_area.push(perm);
                let permanent = PermanentHandle {
                    player: owner,
                    index: (self.player(owner).battle_area.len() - 1) as u8,
                };
                self.enqueue_triggered(
                    EffectTiming::OnOptionPlaced,
                    TriggerSource::OptionPlaced {
                        player: owner,
                        permanent: Some(permanent),
                        linked_host: None,
                        card: placed_card,
                    },
                );
                self.drain_effect_queue();
                if self.pending_selection.is_some() {
                    self.pending_option_placed_turn_check = true;
                }
            }
            OptionSubtype::Link => {
                // Phase 8 Task 4: evaluate link_filter against every
                // Standard-state Digimon on the owner's battle_area (shared
                // helper `link_host_candidates`). If no candidate passes,
                // trash the card silently (mirrors "no legal target" for
                // other effect selections). Otherwise install a
                // PendingSelection routed to `attach_linked_card` and park
                // `pending_option` in `LinkSelectHost`.
                let owner = pending.owner;
                let source_card = pending.card.handle();
                let candidates = self.link_host_candidates(owner, source_card, &effects);

                if candidates.is_empty() {
                    self.player_mut(owner).trash.push(pending.card);
                    return;
                }

                // Re-install pending_option in LinkSelectHost and park a
                // field-selection prompt. The selection callback threads
                // straight into `attach_linked_card`.
                self.pending_option = Some(PendingOption {
                    owner,
                    card: pending.card,
                    source_kind: pending.source_kind,
                    resolution_phase: OptionResolutionPhase::LinkSelectHost,
                    subtype: pending.subtype,
                });
                self.install_link_host_selection(owner, source_card, candidates, false);
            }
            OptionSubtype::Training => {
                // Phase 8 Task 5: park as an `OptionState::Training` permanent on
                // the owner's battle_area. Stays there until the owner hatches
                // an egg via `move_from_breeding`, at which point every Training
                // permanent the owner controls fires `OnTrainingTrash` and is
                // trashed (see `Game::move_from_breeding`). Training sideways-
                // inheritance is dispatched in `enqueue_from_permanent`.
                let owner = pending.owner;
                let placed_card = pending.card.handle();
                let turn = self.turn_count;
                let mut perm = crate::permanent::Permanent::new(pending.card, turn);
                perm.option_state = crate::permanent::OptionState::Training {
                    owner,
                    trained: None,
                };
                self.player_mut(owner).battle_area.push(perm);
                let permanent = PermanentHandle {
                    player: owner,
                    index: (self.player(owner).battle_area.len() - 1) as u8,
                };
                self.enqueue_triggered(
                    EffectTiming::OnOptionPlaced,
                    TriggerSource::OptionPlaced {
                        player: owner,
                        permanent: Some(permanent),
                        linked_host: None,
                        card: placed_card,
                    },
                );
                self.drain_effect_queue();
                if self.pending_selection.is_some() {
                    self.pending_option_placed_turn_check = true;
                }
            }
        }
    }

    /// Commit a Standard Option's dispose-trash given the
    /// `WhenWouldBeTrashed` outcome produced by `try_replace`. Shared by
    /// the synchronous path in `dispose_option` and the deferred path in
    /// `advance_pending_option::Disposing` (where an optional replacement
    /// installed a selection that has since resolved).
    ///
    /// Outcome routing (per Phase 7 spec §7.6):
    /// - `None` — commit the original event: trash the card.
    /// - `Cancelled` / `CustomHandled` — return the Option to the owner's
    ///   hand (cancel restores the original zone for Card subjects).
    /// - `Redirected(Deck)` — insert at the bottom of the owner's deck.
    /// - `Redirected(Hand)` — push to the owner's hand.
    /// - Other variants (`Redirected(other)`, `Substituted(_)`) are not
    ///   meaningful for Option trash in v1; debug_assert catches the
    ///   regression and falls back to trash.
    pub(crate) fn commit_option_trash_outcome(
        &mut self,
        pending: PendingOption,
        outcome: crate::replacement::ReplacementOutcome,
    ) {
        use crate::replacement::ReplacementOutcome;
        match outcome {
            ReplacementOutcome::None => {
                self.player_mut(pending.owner).trash.push(pending.card);
            }
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                self.player_mut(pending.owner).hand.push(pending.card);
            }
            ReplacementOutcome::Redirected(crate::enums::Zone::Deck) => {
                self.player_mut(pending.owner).deck.insert(0, pending.card);
            }
            ReplacementOutcome::Redirected(crate::enums::Zone::Hand) => {
                self.player_mut(pending.owner).hand.push(pending.card);
            }
            ReplacementOutcome::Redirected(other) => {
                debug_assert!(
                    false,
                    "unexpected Redirected({:?}) for Option trash — only Deck/Hand supported in v1",
                    other
                );
                self.player_mut(pending.owner).trash.push(pending.card);
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(
                    false,
                    "Substituted not supported for Option WhenWouldBeTrashed v1"
                );
                self.player_mut(pending.owner).trash.push(pending.card);
            }
        }
    }

    /// Install a field-selection prompt listing `candidates` as legal host
    /// Digimon for a Link Option. On resolve, the callback invokes
    /// `attach_linked_card(host)` which attaches the card + fires OnLink.
    pub(crate) fn install_link_host_selection(
        &mut self,
        owner: PlayerId,
        source_card: crate::card_source::CardHandle,
        candidates: Vec<PermanentHandle>,
        optional: bool,
    ) {
        use crate::action::space::{encode_attack, ATTACK_START, TARGETS_PER_ATTACKER};
        use crate::selection::SelectionKind;

        // Encode via attack-id space — same convention as
        // `select_own_permanent` / `install_field_selection`. The candidates
        // list restricts which indices are valid; no need for a reserved
        // action-ID namespace.
        let valid_action_ids: Vec<u16> = candidates
            .iter()
            .map(|h| encode_attack(0, h.index as u16))
            .collect();

        // Keep the candidate set in a closure-owned snapshot so the callback
        // decodes the picked index correctly even if new permanents are added
        // mid-selection (queue is paused, but this is the defensive choice).
        let candidate_snapshot = candidates.clone();

        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::SelectTarget;
        self.pending_selection = Some(PendingSelection {
            zone_owner: None,
            kind: SelectionKind::OwnField,
            selecting_player: owner,
            previous_phase,
            valid_action_ids,
            is_optional: optional,
            prompt: "Choose a Digimon to link this Option to".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: None,
            source_kind: EffectSourceKind::Option,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let offset = action_id.saturating_sub(ATTACK_START);
                let target_index = (offset % TARGETS_PER_ATTACKER) as u8;
                let picked = candidate_snapshot
                    .iter()
                    .copied()
                    .find(|h| h.index == target_index)
                    .unwrap_or(PermanentHandle {
                        player: owner,
                        index: target_index,
                    });
                game.attach_linked_card(picked);
            }),
            on_decline: optional.then(|| {
                Box::new(move |game: &mut Game| {
                    if let Some(pending) = game.pending_option.take() {
                        game.player_mut(pending.owner).trash.push(pending.card);
                        game.check_turn_end();
                    }
                }) as Box<dyn FnOnce(&mut Game) + Send + Sync>
            }),
        });
    }

    /// Complete a Link Option's attach: push the pending card into the
    /// host's `linked_cards`, fire `OnLink` globally, and clear
    /// `pending_option`. The caller has already validated that `host` was
    /// in the candidate list at selection install-time, but we re-check the
    /// handle is still live in case an intervening effect moved things.
    pub(crate) fn attach_linked_card(&mut self, host: PermanentHandle) {
        let Some(pending_card_handle) = self
            .pending_option
            .as_ref()
            .map(|pending| pending.card.handle())
        else {
            return;
        };

        // If the host vanished (e.g. deleted mid-selection by an interposing
        // effect), fall back to trashing the Option — mirrors other
        // "target vanished" paths elsewhere in the engine.
        let host_live = self
            .player(host.player)
            .battle_area
            .get(host.index as usize)
            .map(|p| {
                self.permanent_is_digimon_for_rules(host)
                    && matches!(p.option_state, crate::permanent::OptionState::Standard)
            })
            .unwrap_or(false);
        if !host_live {
            let Some(pending) = self.pending_option.take() else {
                return;
            };
            self.player_mut(pending.owner).trash.push(pending.card);
            self.check_turn_end();
            return;
        }

        self.pending_would_link_resume = Some(PendingWouldLinkResume {
            host,
            card: pending_card_handle,
        });
        let outcome = self.try_replace(
            EffectTiming::WhenWouldLink,
            crate::replacement::ReplacementSubject::Card(pending_card_handle, Zone::Reveal),
            crate::replacement::ReplacementCause::OwnEffect,
            Some(Zone::BattleArea),
        );
        if self.pending_selection.is_some() {
            return;
        }
        self.commit_pending_would_link(outcome);
    }

    pub(crate) fn commit_pending_would_link(
        &mut self,
        outcome: crate::replacement::ReplacementOutcome,
    ) {
        let Some(resume) = self.pending_would_link_resume.take() else {
            return;
        };
        match outcome {
            crate::replacement::ReplacementOutcome::None => {
                self.commit_linked_card_no_replace(resume);
            }
            crate::replacement::ReplacementOutcome::Cancelled
            | crate::replacement::ReplacementOutcome::CustomHandled
            | crate::replacement::ReplacementOutcome::Redirected(_)
            | crate::replacement::ReplacementOutcome::Substituted(_) => {
                if let Some(pending) = self.pending_option.take() {
                    self.player_mut(pending.owner).trash.push(pending.card);
                }
                self.check_turn_end();
            }
        }
    }

    pub(crate) fn resume_pending_option_placed_link(&mut self) {
        let Some((host, linked_card)) = self.pending_option_placed_link_resume else {
            return;
        };
        if self.pending_selection.is_some() || !self.effect_queue.is_empty() {
            return;
        }
        self.pending_option_placed_link_resume = None;
        self.fire_on_link_after_option_placed(host, linked_card);
    }

    /// Compute the absolute `turn_count` at which a delayed Option should
    /// self-trash. The rule is "end/start of the **owner**'s next turn" for
    /// next-turn triggers, and the current turn for `EndOfThisTurn`.
    ///
    /// In a 2-player round-robin:
    /// - If `owner == turn_player` (the common case — played on own turn),
    ///   "next own turn" lands `turn_count + 2` (skip the opponent's turn).
    /// - If `owner != turn_player` (played during opponent's turn, e.g. via
    ///   a Counter window), "next own turn" lands `turn_count + 1`.
    ///
    /// Multi-player extension is deferred — the plan locks 2-player
    /// semantics for v1.
    pub(crate) fn compute_delay_trash_turn(
        &self,
        owner: PlayerId,
        trigger: crate::enums::DelayTrigger,
    ) -> u16 {
        use crate::enums::DelayTrigger;
        match trigger {
            DelayTrigger::EndOfThisTurn => self.turn_count,
            DelayTrigger::EndOfYourNextTurn | DelayTrigger::StartOfYourNextTurn => {
                self.next_owner_turn_count(owner)
            }
            // Standard `<Delay>` is activated by a player `[Main]`-phase
            // action, not a turn-keyed auto-trash scan. `OnEvent` likewise
            // has no scheduled turn — both park indefinitely.
            DelayTrigger::MainPhaseActivated | DelayTrigger::OnEvent(_) => u16::MAX,
        }
    }

    fn next_owner_turn_count(&self, owner: PlayerId) -> u16 {
        let Some(owner_idx) = self.turn_order.iter().position(|&p| p == owner) else {
            return self.turn_count;
        };
        let turn_delta = if owner_idx > self.turn_player_idx {
            owner_idx - self.turn_player_idx
        } else {
            owner_idx + self.turn_order.len() - self.turn_player_idx
        };
        self.turn_count + turn_delta as u16
    }

    pub(crate) fn finish_pending_option_placed_turn_check(&mut self) {
        if !self.pending_option_placed_turn_check {
            return;
        }
        if self.pending_selection.is_some() || !self.effect_queue.is_empty() {
            return;
        }
        self.pending_option_placed_turn_check = false;
        self.check_turn_end();
    }

    /// Fire the global `OnLeaveField` observer for a permanent that has just
    /// left the battle area by a non-deletion route (return-to-hand,
    /// return-to-deck). The deletion route fires `OnLeaveField` from
    /// `finalize_permanent_deletion_with_event_card`. `snapshot` carries the
    /// leaving permanent's identity so `event_target_*` predicates resolve
    /// against it, exactly as the deletion path does. Called AFTER the
    /// permanent is removed from `battle_area`.
    pub(crate) fn fire_on_leave_field(
        &mut self,
        handle: PermanentHandle,
        snapshot: crate::trigger_context::DeletedObjectSnapshot,
    ) {
        let card = snapshot.top_card;
        let queue_start = self.effect_queue.len();
        self.enqueue_triggered(
            crate::enums::EffectTiming::OnLeaveField,
            crate::selection::TriggerSource::EventObserved {
                player: handle.player,
                permanent: handle,
                card,
                // Leave-field observers read the cause from the snapshot
                // (`trigger.cause`), not this bit.
                effect_initiated: false,
            },
        );
        for queued in self.effect_queue.iter_mut().skip(queue_start) {
            if queued.timing != crate::enums::EffectTiming::OnLeaveField {
                continue;
            }
            if let Some(trigger) = queued.trigger_context.as_mut() {
                trigger.deleted_object = Some(snapshot.clone());
                trigger.cause = Some(snapshot.cause);
                trigger.affected_player = Some(snapshot.former_controller);
                trigger.subject = Some(crate::trigger_context::EventSubject::Permanent(handle));
            }
        }
        // G-DSL-OUTER-TAIL-NESTED-PARK: maybe_drain defers when inside
        // a select-callback / outer-tail scope.
        self.maybe_drain_effect_queue();
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
    }

    pub(crate) fn fire_digivolution_card_trashed(
        &mut self,
        player: PlayerId,
        host: PermanentHandle,
        host_card: crate::card_source::CardHandle,
        card: crate::card_source::CardHandle,
        cause: crate::trigger_context::EventCause,
    ) {
        self.enqueue_triggered(
            crate::enums::EffectTiming::OnDigivolutionCardTrashed,
            crate::selection::TriggerSource::SourceTrashedFromStack {
                player,
                host,
                host_card,
                card,
                cause,
            },
        );
        // Intentionally NOT routed through maybe_drain: EX10-036 (and
        // similar multi-source trash chains) rely on observers firing
        // synchronously between source trashes so secondary clauses can
        // pick up the just-trashed cards mid-resolution. Behavioral test
        // `ex10_036_clause_a_after_source_trash_prompts_opp_field_delete`
        // documents the expected interleaving. Other observer fires
        // (place_security, leave_field, link, attack, play) are deferred.
        self.drain_effect_queue();
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
    }

    /// Battle-area field indices of `player`'s unsuspended Digimon — the
    /// legal suspend-cost targets for `G-PAY-COST-SELECT-ARBITRARY-SUSPEND`.
    fn suspendable_own_digimon(&self, player: PlayerId) -> Vec<usize> {
        self.player(player)
            .battle_area
            .iter()
            .enumerate()
            .filter(|(i, perm)| {
                // Suspend-cost candidacy requires the permanent to actually
                // be suspendable: a `CannotSuspend` permanent can't pay the
                // cost (DCGO `CanActivatePermanentSuspendCostEffect` →
                // `CanSuspend`; prohibition precedence 15-1-3).
                !perm.is_suspended
                    && perm.is_digimon(&self.card_data)
                    && !self.modifiers.has(
                        PermanentHandle {
                            player,
                            index: *i as u8,
                        },
                        ModifierType::CannotSuspend,
                    )
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub(crate) fn commit_pending_would_digivolve(
        &mut self,
        outcome: crate::replacement::ReplacementOutcome,
    ) {
        let Some(resume) = self.pending_would_digivolve_resume.take() else {
            return;
        };
        match outcome {
            crate::replacement::ReplacementOutcome::None => {
                let _ = self.commit_digivolve_from_hand_no_replace(resume);
            }
            crate::replacement::ReplacementOutcome::Cancelled
            | crate::replacement::ReplacementOutcome::CustomHandled => {}
            crate::replacement::ReplacementOutcome::Redirected(_)
            | crate::replacement::ReplacementOutcome::Substituted(_) => {}
        }
    }

    fn card_source_ref_snapshot(
        &self,
        source: crate::enums::CardSourceRef,
    ) -> Option<(crate::card_source::CardHandle, usize, crate::enums::Zone)> {
        use crate::enums::{CardSourceRef, Zone};
        match source {
            CardSourceRef::Hand(p, i) => self
                .player(p)
                .hand
                .get(i)
                .map(|c| (c.handle(), c.data_index, Zone::Hand)),
            CardSourceRef::Trash(p, i) => self
                .player(p)
                .trash
                .get(i)
                .map(|c| (c.handle(), c.data_index, Zone::Trash)),
            CardSourceRef::DeckTop(p) => self
                .player(p)
                .deck
                .last()
                .map(|c| (c.handle(), c.data_index, Zone::Deck)),
            CardSourceRef::Security(p, i) => self
                .player(p)
                .security
                .get(i)
                .map(|c| (c.handle(), c.data_index, Zone::Security)),
            CardSourceRef::Material(h, i) => self
                .player(h.player)
                .battle_area
                .get(h.index as usize)
                .and_then(|perm| perm.card_sources.get(i))
                .map(|c| (c.handle(), c.data_index, Zone::BattleArea)),
            CardSourceRef::Reveal(h) => self
                .revealed_cards
                .iter()
                .find(|c| c.handle() == h)
                .map(|c| (c.handle(), c.data_index, Zone::Reveal)),
        }
    }

    pub(crate) fn place_as_bottom_source_observed(
        &mut self,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
        observer_player: PlayerId,
        face_down: bool,
    ) -> bool {
        if let crate::enums::CardSourceRef::Security(defender, index) = source {
            if target.index == crate::action::space::BREEDING_TARGET as u8 {
                if self.player(target.player).breeding_area.is_none() {
                    return false;
                }
            } else if self
                .player(target.player)
                .battle_area
                .get(target.index as usize)
                .is_none()
            {
                return false;
            }

            // Opaque-aware: materialize before placing-as-bottom-source.
            // The card becomes a digivolution stack source; its real
            // identity matters for source-iteration effects and Mind Link
            // gating.
            if index >= self.player(defender).security.len() {
                return false;
            }
            self.ensure_security_materialized(defender, index);
            let player = self.player_mut(defender);
            let card = player.security.remove(index);
            player.face_up_security.remove(&card.card_index);
            let cause = crate::trigger_context::EventCause::from(self.infer_effect_cause(defender));
            self.fire_effect_security_removal(
                defender,
                observer_player,
                observer_player,
                cause,
                card,
                crate::selection::SecurityRemovalDestination::BottomSource(target),
            );
            return true;
        }

        let Some(taken) = self.take_card_source_ref(source) else {
            return false;
        };

        if target.index == crate::action::space::BREEDING_TARGET as u8 {
            let Some(breeding) = self.player_mut(target.player).breeding_area.as_mut() else {
                let _ = self.restore_card_source_ref(source, taken);
                return false;
            };
            let mut card = taken.card;
            card.face_down = face_down;
            breeding.push_under(card);
            // Soft-remove the carrier slot if Material extraction emptied it.
            // Target is in breeding (not battle_area), so no shift needed.
            // Sibling of the digivolve-from-material fix landed in PR #533.
            // See `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` in
            // `qa/archetype-qa/engine-gaps.md`.
            if let crate::enums::CardSourceRef::Material(carrier, _) = source {
                let _ = self.soft_remove_if_emptied(carrier);
            }
            return true;
        }

        let target_player = self.player_mut(target.player);
        if (target.index as usize) >= target_player.battle_area.len() {
            let _ = self.restore_card_source_ref(source, taken);
            return false;
        }
        let mut card = taken.card;
        card.face_down = face_down;
        target_player.battle_area[target.index as usize].push_under(card);
        // Soft-remove the carrier slot if Material extraction emptied it.
        // Sibling of the digivolve-from-material fix landed in PR #533. The
        // soft-remove runs AFTER push_under so the target index is still
        // valid for the push; the soft-remove of the carrier (which is now
        // empty AND distinct from the target — the target just received the
        // pushed card so it's non-empty) only shifts unrelated indices, not
        // this function's `target` (which we've already finished mutating).
        // See `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` in
        // `qa/archetype-qa/engine-gaps.md`.
        if let crate::enums::CardSourceRef::Material(carrier, _) = source {
            let _ = self.soft_remove_if_emptied(carrier);
        }
        true
    }

    /// Search all zones of all players for a card matching `handle` and remove
    /// it, returning the `CardSource`. Returns `None` if the card is not found
    /// in any zone.
    ///
    /// Zones scanned (in order):
    ///   1. Each player's `hand`
    ///   2. Each player's `trash`
    ///   3. Each player's `deck`
    ///   4. Each player's `security`
    ///   5. Each player's `battle_area` permanent card stacks (all sources)
    ///   6. Each player's `breeding_area` card stack
    ///   7. The game-level `revealed_cards` transient pool
    ///
    /// Used by `EffectContext::place_card_under_permanent_bottom` to locate
    /// cards before tucking them under a permanent regardless of which zone
    /// they currently live in.
    pub(crate) fn remove_card_from_any_zone(
        &mut self,
        handle: crate::card_source::CardHandle,
    ) -> Option<crate::card_source::CardSource> {
        let player_count = self.players.len();

        for pid in 0..player_count {
            // --- hand ---
            if let Some(pos) = self.players[pid]
                .hand
                .iter()
                .position(|c| c.handle() == handle)
            {
                return Some(self.players[pid].hand.remove(pos));
            }
            // --- trash ---
            if let Some(pos) = self.players[pid]
                .trash
                .iter()
                .position(|c| c.handle() == handle)
            {
                return Some(self.players[pid].trash.remove(pos));
            }
            // --- deck ---
            if let Some(pos) = self.players[pid]
                .deck
                .iter()
                .position(|c| c.handle() == handle)
            {
                return Some(self.players[pid].deck.remove(pos));
            }
            // --- security ---
            if let Some(pos) = self.players[pid]
                .security
                .iter()
                .position(|c| c.handle() == handle)
            {
                // Opaque-aware: handle-based generic-zone remove. The
                // caller routes the returned card somewhere observable;
                // materialize so it has real identity.
                self.ensure_security_materialized(pid as PlayerId, pos);
                return Some(self.players[pid].security.remove(pos));
            }
            // --- battle_area permanent stacks ---
            for perm_idx in 0..self.players[pid].battle_area.len() {
                let stack = &self.players[pid].battle_area[perm_idx].card_sources;
                if let Some(src_pos) = stack.iter().position(|c| c.handle() == handle) {
                    return Some(
                        self.players[pid].battle_area[perm_idx]
                            .card_sources
                            .remove(src_pos),
                    );
                }
            }
            // --- breeding_area ---
            if let Some(ref breeding) = self.players[pid].breeding_area {
                if let Some(src_pos) = breeding
                    .card_sources
                    .iter()
                    .position(|c| c.handle() == handle)
                {
                    return Some(
                        self.players[pid]
                            .breeding_area
                            .as_mut()
                            .unwrap()
                            .card_sources
                            .remove(src_pos),
                    );
                }
            }
        }

        // --- revealed_cards transient pool ---
        if let Some(pos) = self
            .revealed_cards
            .iter()
            .position(|c| c.handle() == handle)
        {
            let mut taken = self.revealed_cards.remove(pos);
            taken.clear_reveal_overlay();
            return Some(taken);
        }

        None
    }

    fn effect_source_kind_for_handle(
        &self,
        handle: crate::card_source::CardHandle,
    ) -> EffectSourceKind {
        self.card_kind_for_handle(handle)
            .map(source_kind_for_card_kind)
            .unwrap_or(EffectSourceKind::Rule)
    }

    fn cost_reducer_activation_count(&self, key: &CostReductionKey) -> u8 {
        let Some(source) = key.source_permanent else {
            return 0;
        };
        if source.index == crate::action::space::BREEDING_TARGET as u8 {
            return self
                .player(source.player)
                .breeding_area
                .as_ref()
                .map(|perm| perm.activation_count(key.source_card, key.effect_slot))
                .unwrap_or(0);
        }
        self.player(source.player)
            .battle_area
            .get(source.index as usize)
            .map(|perm| perm.activation_count(key.source_card, key.effect_slot))
            .unwrap_or(0)
    }

    fn before_pay_cost_source_infos(
        &self,
        acting_player: PlayerId,
        cost_target_card: Option<crate::card_source::CardHandle>,
    ) -> Vec<BeforePayCostSourceInfo> {
        let mut infos = Vec::new();
        self.push_breeding_cost_sources(acting_player, &mut infos);
        for pid in 0..self.players.len() {
            let player_id = pid as PlayerId;
            let perm_count = self.player(player_id).battle_area.len();
            for perm_idx in 0..perm_count {
                let perm_handle = PermanentHandle {
                    player: player_id,
                    index: perm_idx as u8,
                };
                let stack_size = self.player(player_id).battle_area[perm_idx]
                    .card_sources
                    .len();
                for source_idx in 0..stack_size {
                    let source =
                        &self.player(player_id).battle_area[perm_idx].card_sources[source_idx];
                    self.push_cost_source_info(
                        &mut infos,
                        Some(perm_handle),
                        source,
                        source_idx + 1 < stack_size,
                        player_id,
                        false,
                    );
                }
            }
            if player_id != acting_player {
                self.push_breeding_cost_sources(player_id, &mut infos);
            }
        }
        if let Some(target) = cost_target_card {
            if let Some((card_id, controller)) = self.card_id_and_owner_for_handle(target) {
                let Some(effects) = self.effects_for_card(&card_id, target) else {
                    return infos;
                };
                for (slot, effect) in effects.iter().enumerate() {
                    if effect.timing == EffectTiming::BeforePayCost && effect.when_playing_this {
                        infos.push(BeforePayCostSourceInfo {
                            source_permanent: None,
                            source_card: target,
                            card_id: card_id.clone(),
                            is_under: false,
                            controller,
                            effect_slot: slot as u8,
                        });
                    }
                }
            }
        }
        infos
    }

    /// Push BeforePayCost cost-reduction sources from a player's breeding area.
    ///
    /// Per general_rule.pdf 3-4-6-4 ("Effects on cards can't trigger or
    /// activate unless the effect explicitly specifies/references breeding
    /// areas"), a Digimon being *raised* in the breeding area does NOT have its
    /// battle-area effects active there (e.g. BT23-005 Elizamon's "[Your Turn]
    /// reduce digivolution cost" must not fire while it raises). Only Digi-Eggs
    /// are breeding-resident by nature, so their effects are the only ones that
    /// legitimately activate in the breeding area (e.g. BT13-007 King Drasil_7D6
    /// reducing Royal Knight play cost). Gate on card kind accordingly.
    fn push_breeding_cost_sources(
        &self,
        player_id: PlayerId,
        infos: &mut Vec<BeforePayCostSourceInfo>,
    ) {
        let Some(perm) = self.player(player_id).breeding_area.as_ref() else {
            return;
        };
        let stack_size = perm.card_sources.len();
        let handle = PermanentHandle {
            player: player_id,
            index: crate::action::space::BREEDING_TARGET as u8,
        };
        for source_idx in 0..stack_size {
            let source = &perm.card_sources[source_idx];
            if source.card_kind(&self.card_data) != CardKind::DigiEgg {
                continue;
            }
            self.push_cost_source_info(
                infos,
                Some(handle),
                source,
                source_idx + 1 < stack_size,
                player_id,
                false,
            );
        }
    }

    fn push_cost_source_info(
        &self,
        infos: &mut Vec<BeforePayCostSourceInfo>,
        source_permanent: Option<PermanentHandle>,
        source: &CardSource,
        is_under: bool,
        controller: PlayerId,
        allow_when_playing_this: bool,
    ) {
        let card_id = source.card_id(&self.card_data).to_string();
        let source_card = source.handle();
        let Some(effects) = self.effects_for_card(&card_id, source_card) else {
            return;
        };
        for (slot, effect) in effects.iter().enumerate() {
            if effect.timing != EffectTiming::BeforePayCost {
                continue;
            }
            if effect.when_playing_this && !allow_when_playing_this {
                continue;
            }
            infos.push(BeforePayCostSourceInfo {
                source_permanent,
                source_card,
                card_id: card_id.clone(),
                is_under,
                controller,
                effect_slot: slot as u8,
            });
        }
    }

    // ── BeforePayCostObserve dispatch (G-BEFORE-PAY-COST-GAIN-MEMORY) ──
    //
    // Walks the same source list as the cost-reduction scan but matches
    // effects with timing `BeforePayCostObserve` and fires their `process`
    // bodies. Observer bodies typically gain memory or otherwise mutate
    // state during cost calculation; they MUST NOT install a pending
    // selection in v1 (no-approximations §17: surface choices through
    // pending_selection — observer-with-selection support is planned but
    // out of scope for Phase 2 Track H, since BG Imperial's six refs are
    // all scalar `gain_memory` bodies).

    fn before_pay_cost_observer_infos(
        &self,
        acting_player: PlayerId,
        cost_target_card: Option<crate::card_source::CardHandle>,
    ) -> Vec<BeforePayCostSourceInfo> {
        let mut infos = Vec::new();
        self.push_breeding_observer_sources(acting_player, &mut infos);
        for pid in 0..self.players.len() {
            let player_id = pid as PlayerId;
            let perm_count = self.player(player_id).battle_area.len();
            for perm_idx in 0..perm_count {
                let perm_handle = PermanentHandle {
                    player: player_id,
                    index: perm_idx as u8,
                };
                let stack_size = self.player(player_id).battle_area[perm_idx]
                    .card_sources
                    .len();
                for source_idx in 0..stack_size {
                    let source =
                        &self.player(player_id).battle_area[perm_idx].card_sources[source_idx];
                    self.push_observer_source_info(
                        &mut infos,
                        Some(perm_handle),
                        source,
                        source_idx + 1 < stack_size,
                        player_id,
                        false,
                    );
                }
            }
            if player_id != acting_player {
                self.push_breeding_observer_sources(player_id, &mut infos);
            }
        }
        if let Some(target) = cost_target_card {
            if let Some((card_id, controller)) = self.card_id_and_owner_for_handle(target) {
                if let Some(effects) = self.effects_for_card(&card_id, target) {
                    for (slot, effect) in effects.iter().enumerate() {
                        if effect.timing == EffectTiming::BeforePayCostObserve
                            && effect.when_playing_this
                        {
                            infos.push(BeforePayCostSourceInfo {
                                source_permanent: None,
                                source_card: target,
                                card_id: card_id.clone(),
                                is_under: false,
                                controller,
                                effect_slot: slot as u8,
                            });
                        }
                    }
                }
            }
        }
        infos
    }

    /// Breeding-area BeforePayCostObserve sources — same Digi-Egg-only gate as
    /// `push_breeding_cost_sources` (general_rule.pdf 3-4-6-4).
    fn push_breeding_observer_sources(
        &self,
        player_id: PlayerId,
        infos: &mut Vec<BeforePayCostSourceInfo>,
    ) {
        let Some(perm) = self.player(player_id).breeding_area.as_ref() else {
            return;
        };
        let stack_size = perm.card_sources.len();
        let handle = PermanentHandle {
            player: player_id,
            index: crate::action::space::BREEDING_TARGET as u8,
        };
        for source_idx in 0..stack_size {
            let source = &perm.card_sources[source_idx];
            if source.card_kind(&self.card_data) != CardKind::DigiEgg {
                continue;
            }
            self.push_observer_source_info(
                infos,
                Some(handle),
                source,
                source_idx + 1 < stack_size,
                player_id,
                false,
            );
        }
    }

    fn push_observer_source_info(
        &self,
        infos: &mut Vec<BeforePayCostSourceInfo>,
        source_permanent: Option<PermanentHandle>,
        source: &CardSource,
        is_under: bool,
        controller: PlayerId,
        allow_when_playing_this: bool,
    ) {
        let card_id = source.card_id(&self.card_data).to_string();
        let source_card = source.handle();
        let Some(effects) = self.effects_for_card(&card_id, source_card) else {
            return;
        };
        for (slot, effect) in effects.iter().enumerate() {
            if effect.timing != EffectTiming::BeforePayCostObserve {
                continue;
            }
            if effect.when_playing_this && !allow_when_playing_this {
                continue;
            }
            infos.push(BeforePayCostSourceInfo {
                source_permanent,
                source_card,
                card_id: card_id.clone(),
                is_under,
                controller,
                effect_slot: slot as u8,
            });
        }
    }

    fn observer_activation_count(&self, info: &BeforePayCostSourceInfo) -> u8 {
        let Some(source) = info.source_permanent else {
            return 0;
        };
        if source.index == crate::action::space::BREEDING_TARGET as u8 {
            return self
                .player(source.player)
                .breeding_area
                .as_ref()
                .map(|perm| perm.activation_count(info.source_card, info.effect_slot))
                .unwrap_or(0);
        }
        self.player(source.player)
            .battle_area
            .get(source.index as usize)
            .map(|perm| perm.activation_count(info.source_card, info.effect_slot))
            .unwrap_or(0)
    }

    fn card_id_and_owner_for_handle(
        &self,
        handle: crate::card_source::CardHandle,
    ) -> Option<(String, PlayerId)> {
        for player in &self.players {
            for card in &player.hand {
                if card.handle() == handle {
                    return Some((card.card_id(&self.card_data).to_string(), card.owner));
                }
            }
        }
        None
    }

    /// Generalized "move this permanent into the security stack" primitive.
    /// Routes the permanent's top card to `player_id`'s security at `position`
    /// (face-up if `face_up`); routes sources-below-top to each source's
    /// owner's trash, firing `OnDigivolutionCardTrashed` per source; routes
    /// linked cards to the controller's trash, firing `OnLinkedCardTrashed`
    /// once if any were present. Mirrors the source-disposition shape used by
    /// `Game::return_to_deck` and `EffectContext::attach_tamer_to_digimon`.
    ///
    /// Gates on `CannotAddSecurityByEffect` (player-scoped, checked against
    /// `observer_player`). Routes through `WhenWouldLeaveBattleArea` then
    /// `WhenWouldPlaceInSecurity` replacements; bails (`false`) on any
    /// non-`None` outcome or installed pending selection.
    ///
    /// Used by `EffectContext::place_self_at_security` (Track E) — printed
    /// text "place this Digimon at the bottom of your security stack face
    /// down" (EX4-060), "place this Digimon as your top security card"
    /// (EX9-021), etc. DCGO `IPutSecurityPermanent` covers the same shape.
    ///
    /// **Engine divergence vs DCGO:** DCGO bundles the entire permanent
    /// (top + sources + linked) under a single security slot. The Rust
    /// engine's `Player.security: Vec<CardSource>` is flat (one card per
    /// slot), so the bundle is unrepresentable. We route sources to trash
    /// instead, matching the rules-default behavior for permanents leaving
    /// the field to a non-stack destination. Documented in
    /// `docs/RUST_PYTHON_PARITY.md` (Track E divergence note).
    pub(crate) fn place_permanent_on_security_observed(
        &mut self,
        player_id: PlayerId,
        target: PermanentHandle,
        position: crate::enums::StackPosition,
        face_up: bool,
        observer_player: PlayerId,
    ) -> bool {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        if self
            .modifiers
            .player_has(observer_player, ModifierType::CannotAddSecurityByEffect)
        {
            return false;
        }

        let Some(permanent) = self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
        else {
            return false;
        };
        if permanent.card_sources.is_empty() {
            return false;
        }

        let source_card = permanent.top_card().handle();
        let cause = self.infer_effect_cause(player_id);
        let leave_subject = ReplacementSubject::Permanent(target);
        let leave_outcome = self.try_replace(
            EffectTiming::WhenWouldLeaveBattleArea,
            leave_subject,
            cause,
            Some(Zone::Security),
        );
        if self.pending_selection.is_some() || !matches!(leave_outcome, ReplacementOutcome::None) {
            return false;
        }

        let place_subject = ReplacementSubject::Card(source_card, Zone::BattleArea);
        let place_outcome = self.try_replace(
            EffectTiming::WhenWouldPlaceInSecurity,
            place_subject,
            cause,
            Some(Zone::Security),
        );
        if self.pending_selection.is_some() || !matches!(place_outcome, ReplacementOutcome::None) {
            return false;
        }

        let mut permanent = self
            .player_mut(target.player)
            .battle_area
            .remove(target.index as usize);

        // Pop top card; if somehow empty (shouldn't happen — we checked
        // above), bail without further state changes.
        let Some(card) = permanent.card_sources.pop() else {
            return false;
        };

        // Modifier cleanup BEFORE the source-trash dispatch — modifiers are
        // keyed on `PermanentHandle`, which becomes invalid after `remove()`
        // shifts indices. Mirrors `attach_tamer_to_digimon`.
        self.clear_permanent_full(target);
        self.modifiers.expire_player_on_permanent_leave(target);

        // Sources-below-top → each source's owner's trash. Per source: push,
        // enqueue OnDigivolutionCardTrashed for each player, drain queue.
        // Mirrors `EffectContext::attach_tamer_to_digimon`.
        for source in permanent.card_sources.drain(..) {
            let owner = source.owner;
            self.player_mut(owner).trash.push(source);
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    EffectTiming::OnDigivolutionCardTrashed,
                    TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
                );
            }
            // Intentionally inline-drain (see `fire_digivolution_card_trashed`):
            // EX10-036's behavioral test depends on synchronous between-source
            // observer firing for chained trash-pickup clauses.
            self.drain_effect_queue();
        }

        // Linked cards → controller's trash; fire OnLinkedCardTrashed once
        // if any were present. Mirrors `attach_tamer_to_digimon`.
        let had_linked = !permanent.linked_cards.is_empty();
        for linked in permanent.linked_cards.drain(..) {
            let owner = linked.owner;
            self.player_mut(owner).trash.push(linked);
        }
        if had_linked {
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    EffectTiming::OnLinkedCardTrashed,
                    TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
                );
            }
            // Intentionally inline-drain — same rationale as above.
            self.drain_effect_queue();
        }

        // Place top card in security at the requested position.
        let face_up_key = card.card_index;
        match position {
            crate::enums::StackPosition::Top => {
                self.player_mut(player_id).security.push(card);
            }
            crate::enums::StackPosition::Bottom => {
                self.player_mut(player_id).security.insert(0, card);
            }
            crate::enums::StackPosition::Random => {
                use rand::Rng;
                let sec_len = self.player(player_id).security.len();
                let idx = if sec_len == 0 {
                    0
                } else {
                    self.rng.gen_range(0..=sec_len)
                };
                self.player_mut(player_id).security.insert(idx, card);
            }
        }
        if face_up {
            self.player_mut(player_id)
                .face_up_security
                .insert(face_up_key);
        }
        true
    }

    pub(crate) fn place_sourceless_permanent_on_security_bottom(
        &mut self,
        player_id: PlayerId,
        target: PermanentHandle,
        observer_player: PlayerId,
    ) -> bool {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        if self
            .modifiers
            .player_has(observer_player, ModifierType::CannotAddSecurityByEffect)
        {
            return false;
        }

        let Some(permanent) = self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
        else {
            return false;
        };
        if permanent.card_sources.len() != 1 {
            return false;
        }

        let source_card = permanent.top_card().handle();
        let cause = self.infer_effect_cause(player_id);
        let leave_subject = ReplacementSubject::Permanent(target);
        let leave_outcome = self.try_replace(
            EffectTiming::WhenWouldLeaveBattleArea,
            leave_subject,
            cause,
            Some(Zone::Security),
        );
        if self.pending_selection.is_some() || !matches!(leave_outcome, ReplacementOutcome::None) {
            return false;
        }

        let place_subject = ReplacementSubject::Card(source_card, Zone::BattleArea);
        let place_outcome = self.try_replace(
            EffectTiming::WhenWouldPlaceInSecurity,
            place_subject,
            cause,
            Some(Zone::Security),
        );
        if self.pending_selection.is_some() || !matches!(place_outcome, ReplacementOutcome::None) {
            return false;
        }

        let mut permanent = self
            .player_mut(target.player)
            .battle_area
            .remove(target.index as usize);
        let Some(card) = permanent.card_sources.pop() else {
            return false;
        };

        self.clear_permanent_full(target);
        self.modifiers.expire_player_on_permanent_leave(target);

        let had_linked = !permanent.linked_cards.is_empty();
        for linked in permanent.linked_cards {
            self.player_mut(target.player).trash.push(linked);
        }
        if had_linked {
            self.enqueue_triggered(
                EffectTiming::OnLinkedCardTrashed,
                TriggerSource::PlayerBattleArea(observer_player),
            );
            self.drain_effect_queue();
        }

        self.player_mut(player_id).security.insert(0, card);
        self.fire_on_place_security(player_id, observer_player, source_card);
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
        true
    }

    pub(crate) fn fire_on_place_security(
        &mut self,
        affected_player: PlayerId,
        source_player: PlayerId,
        card: crate::card_source::CardHandle,
    ) {
        self.enqueue_triggered(
            EffectTiming::OnPlaceSecurity,
            TriggerSource::SecurityPlaced {
                affected_player,
                source_player,
                card,
                cause: crate::trigger_context::EventCause::SecurityPlacement,
            },
        );
        // G-DSL-OUTER-TAIL-NESTED-PARK fix: this was previously the dominant
        // collision site — `place_on_security` called from inside a Lamiamon
        // clause-2 inner-tail callback would inline-drain a second copy of
        // the same triggered effect, parking on top of the first's outer
        // tail. `maybe_drain` defers the drain to the outer-tail scope's
        // exit.
        self.maybe_drain_effect_queue();
    }

    pub(crate) fn place_permanent_on_security(
        &mut self,
        player_id: PlayerId,
        target: PermanentHandle,
        position: crate::enums::StackPosition,
        face_up: bool,
        observer_player: PlayerId,
    ) -> bool {
        use crate::enums::{EffectTiming, StackPosition, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        if self
            .modifiers
            .player_has(observer_player, ModifierType::CannotAddSecurityByEffect)
        {
            return false;
        }

        if self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .is_none()
        {
            return false;
        }

        let cause = self.infer_effect_cause(target.player);
        let leave_outcome = self.try_replace(
            EffectTiming::WhenWouldLeaveBattleArea,
            ReplacementSubject::Permanent(target),
            cause,
            Some(Zone::Security),
        );
        if self.pending_selection.is_some() {
            return false;
        }
        match leave_outcome {
            ReplacementOutcome::None => {}
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => return false,
            ReplacementOutcome::Redirected(Zone::Security) => {}
            ReplacementOutcome::Redirected(Zone::Trash) => {
                self.delete_permanent_with_cause(target, cause);
                return false;
            }
            ReplacementOutcome::Redirected(Zone::Hand) => {
                return self.return_to_hand(target).is_some()
            }
            ReplacementOutcome::Redirected(Zone::Deck) => {
                return self.return_to_deck(target, StackPosition::Bottom);
            }
            ReplacementOutcome::Redirected(other) => {
                debug_assert!(
                    false,
                    "unexpected redirect destination for permanent-to-security: {:?}",
                    other
                );
            }
            ReplacementOutcome::Substituted(ReplacementSubject::Permanent(other)) => {
                return self.place_permanent_on_security(
                    player_id,
                    other,
                    position,
                    face_up,
                    observer_player,
                );
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(
                    false,
                    "non-permanent substitute is unsupported for permanent-to-security"
                );
            }
        }

        self.place_permanent_on_security_without_leave_replacement(
            player_id,
            target,
            position,
            face_up,
            observer_player,
        )
    }

    pub(crate) fn place_permanent_on_security_without_leave_replacement(
        &mut self,
        player_id: PlayerId,
        target: PermanentHandle,
        position: crate::enums::StackPosition,
        face_up: bool,
        observer_player: PlayerId,
    ) -> bool {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        if self
            .modifiers
            .player_has(observer_player, ModifierType::CannotAddSecurityByEffect)
        {
            return false;
        }

        let Some(permanent) = self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
        else {
            return false;
        };
        let source_card = permanent.top_card().handle();
        let cause = self.infer_effect_cause(player_id);
        let place_subject = ReplacementSubject::Card(source_card, Zone::BattleArea);
        let place_outcome = self.try_replace(
            EffectTiming::WhenWouldPlaceInSecurity,
            place_subject,
            cause,
            Some(Zone::Security),
        );
        if self.pending_selection.is_some() || !matches!(place_outcome, ReplacementOutcome::None) {
            return false;
        }

        let mut permanent = self
            .player_mut(target.player)
            .battle_area
            .remove(target.index as usize);
        let Some(top) = permanent.card_sources.pop() else {
            return false;
        };
        let top_handle = top.handle();
        let face_up_key = top.card_index;

        let mut leaving_sources = permanent.card_sources.clone();
        leaving_sources.push(top.clone());
        self.apply_ace_overflow_for_sources(&leaving_sources);

        match position {
            crate::enums::StackPosition::Top => {
                self.player_mut(player_id).security.push(top);
            }
            crate::enums::StackPosition::Bottom => {
                self.player_mut(player_id).security.insert(0, top);
            }
            crate::enums::StackPosition::Random => {
                use rand::Rng;
                let sec_len = self.player(player_id).security.len();
                let idx = if sec_len == 0 {
                    0
                } else {
                    self.rng.gen_range(0..=sec_len)
                };
                self.player_mut(player_id).security.insert(idx, top);
            }
        }

        if face_up {
            self.player_mut(player_id)
                .face_up_security
                .insert(face_up_key);
        }

        for card in permanent.card_sources {
            let source_card = card.handle();
            // Owner-routed (Track E correctness): each source returns to
            // its OWN owner's trash. Identical to controller-routed when
            // owner == controller (the common case).
            let owner = card.owner;
            self.player_mut(owner).trash.push(card);
            self.enqueue_triggered(
                EffectTiming::OnDigivolutionCardTrashed,
                TriggerSource::SourceTrashedFromStack {
                    player: target.player,
                    host: target,
                    host_card: top_handle,
                    card: source_card,
                    cause: crate::trigger_context::EventCause::SecurityPlacement,
                },
            );
            self.drain_effect_queue();
        }

        let had_linked = !permanent.linked_cards.is_empty();
        for linked in permanent.linked_cards {
            // Owner-routed: linked cards return to their own owner's trash.
            let owner = linked.owner;
            self.player_mut(owner).trash.push(linked);
        }
        if had_linked {
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    EffectTiming::OnLinkedCardTrashed,
                    TriggerSource::PlayerBattleArea(pid as PlayerId),
                );
            }
            self.drain_effect_queue();
        }

        self.clear_permanent_full(target);
        self.modifiers.expire_player_on_permanent_leave(target);
        self.fire_on_place_security(player_id, observer_player, top_handle);
        true
    }

    pub(crate) fn place_on_security_observed(
        &mut self,
        player_id: PlayerId,
        source: crate::enums::CardSourceRef,
        position: crate::enums::StackPosition,
        face_up: bool,
        observer_player: PlayerId,
    ) -> bool {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        // Snapshot the source card's handle before the take so we can build
        // a meaningful ReplacementSubject. Return false early if the source
        // is invalid (matches the existing pre-flight behavior of the take).
        let Some((source_card, _, source_zone)) = self.card_source_ref_snapshot(source) else {
            return false;
        };

        let cause = self.infer_effect_cause(player_id);
        let subject = ReplacementSubject::Card(source_card, source_zone);

        let outcome = self.try_replace(
            EffectTiming::WhenWouldPlaceInSecurity,
            subject,
            cause,
            Some(Zone::Security),
        );
        if self.pending_selection.is_some() {
            return false;
        }
        match outcome {
            ReplacementOutcome::None => {}
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                return false;
            }
            ReplacementOutcome::Redirected(Zone::Trash) => {
                // Redirect: card goes to its owner's trash instead of
                // security. Take the source and route it.
                let taken = match source {
                    crate::enums::CardSourceRef::Security(defender, index) => {
                        // Opaque-aware: materialize before the redirect-
                        // to-trash so the observer sees real card data.
                        if index >= self.player(defender).security.len() {
                            return false;
                        }
                        self.ensure_security_materialized(defender, index);
                        let player = self.player_mut(defender);
                        let card = player.security.remove(index);
                        player.face_up_security.remove(&card.card_index);
                        let cause = crate::trigger_context::EventCause::from(
                            self.infer_effect_cause(defender),
                        );
                        self.fire_effect_security_removal(
                            defender,
                            observer_player,
                            observer_player,
                            cause,
                            card,
                            crate::selection::SecurityRemovalDestination::Trash,
                        );
                        return false;
                    }
                    crate::enums::CardSourceRef::Trash(source_p, source_i) => {
                        // Task 4 v1: cross-player trash-to-trash redirects are rare
                        // in printed cards (a trash-to-security play being redirected
                        // TO trash is niche). For source_p == player_id this is a
                        // true no-op. For source_p != player_id, a strict reading
                        // would move the card from source_p.trash to player_id.trash;
                        // we preserve source location to avoid a hidden cross-player
                        // move. TODO(phase-7-followup): verify printed-card need.
                        debug_assert!(
                            source_p == player_id,
                            "redirect-to-Trash from cross-player trash is a v1 no-op; card stayed in source_p={} trash (player_id={}, source_i={})",
                            source_p, player_id, source_i
                        );
                        return false;
                    }
                    other => {
                        let Some(taken) = self.take_card_source_ref(other) else {
                            return false;
                        };
                        taken.card
                    }
                };
                let owner = taken.owner;
                self.player_mut(owner).trash.push(taken);
                // Soft-remove the carrier slot if Material extraction
                // emptied it. Sibling of the digivolve-from-material fix
                // landed in PR #533. See
                // `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` in
                // `qa/archetype-qa/engine-gaps.md`.
                if let crate::enums::CardSourceRef::Material(carrier, _) = source {
                    let _ = self.soft_remove_if_emptied(carrier);
                }
                return false;
            }
            ReplacementOutcome::Redirected(other) => {
                debug_assert!(
                    false,
                    "unexpected redirect destination for WhenWouldPlaceInSecurity: {:?}",
                    other
                );
                // Fallthrough and commit the original place.
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(
                    false,
                    "substitute subject not supported for WhenWouldPlaceInSecurity v1"
                );
                // Fallthrough.
            }
        }

        // Take the card out of its source zone. Mirror the pattern from
        // place_as_bottom_source.
        if let crate::enums::CardSourceRef::Security(defender, index) = source {
            // Opaque-aware: materialize before moving to another zone.
            if index >= self.player(defender).security.len() {
                return false;
            }
            self.ensure_security_materialized(defender, index);
            let player = self.player_mut(defender);
            let card = player.security.remove(index);
            player.face_up_security.remove(&card.card_index);
            let cause = crate::trigger_context::EventCause::from(self.infer_effect_cause(defender));
            self.fire_effect_security_removal(
                defender,
                observer_player,
                observer_player,
                cause,
                card,
                crate::selection::SecurityRemovalDestination::Security {
                    player: player_id,
                    position,
                    face_up,
                },
            );
            return true;
        }
        let Some(taken) = self.take_card_source_ref(source) else {
            return false;
        };
        let taken = taken.card;

        // face_up_security is HashSet<u16> keyed by card_index.
        let face_up_key = taken.card_index;

        match position {
            crate::enums::StackPosition::Top => {
                self.player_mut(player_id).security.push(taken);
            }
            crate::enums::StackPosition::Bottom => {
                self.player_mut(player_id).security.insert(0, taken);
            }
            crate::enums::StackPosition::Random => {
                use rand::Rng;
                // Split-borrow: read length from immutable borrow first, then
                // mutably insert — mirrors the pattern in return_to_deck.
                let sec_len = self.player(player_id).security.len();
                let idx = if sec_len == 0 {
                    0
                } else {
                    self.rng.gen_range(0..=sec_len)
                };
                self.player_mut(player_id).security.insert(idx, taken);
            }
        }

        if face_up {
            self.player_mut(player_id)
                .face_up_security
                .insert(face_up_key);
        }
        // Soft-remove the carrier slot if Material extraction emptied it.
        // Sibling of the digivolve-from-material fix landed in PR #533.
        // The push to security is already complete; we just need to clean
        // up the now-empty carrier slot before any downstream trigger
        // observers iterate `battle_area`. See
        // `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` in
        // `qa/archetype-qa/engine-gaps.md`.
        if let crate::enums::CardSourceRef::Material(carrier, _) = source {
            let _ = self.soft_remove_if_emptied(carrier);
        }
        self.fire_on_place_security(player_id, observer_player, source_card);
        true
    }

}

// ── Assembly play-execution free helpers (G-ASSEMBLY-PLAY-EXECUTION) ──────
//
// These live at module scope (not on `impl Game`) so the per-element
// selection chain can recurse from inside a `Box<dyn FnOnce>` selection
// callback without any self-reference. See `Game::try_begin_assembly_flow`.

/// Immutable bundle of the play parameters threaded through the Assembly
/// selection chain. `Copy` so each recursive step / closure can take it.
#[cfg(feature = "dsl-yaml-loader")]
#[derive(Clone, Copy)]
struct AssemblyPlayParams {
    cost_delta: crate::enums::CostDelta,
    source: PlaySource,
    origin: PendingWouldPlayOrigin,
    suppress_on_play: bool,
    /// Generic cost reduction accumulated by the BeforePayCost chain.
    total_reduction: i32,
    /// The assembly-specific reduction (D5 — `cost:` on the alt-path).
    assembly_reduction: i32,
}

/// Recursive system-of-distinct-representatives check: can each assembly
/// slot be matched to a DISTINCT trash card? `match_table[slot]` lists the
/// trash indices satisfying that slot's filter. Tiny inputs (≤ a few slots,
/// trash bounded by deck size) keep the backtracking cheap.
#[cfg(feature = "dsl-yaml-loader")]
fn assembly_assign(match_table: &[Vec<usize>], slot: usize, used: &mut [bool]) -> bool {
    if slot >= match_table.len() {
        return true;
    }
    for &t in &match_table[slot] {
        if !used[t] {
            used[t] = true;
            if assembly_assign(match_table, slot + 1, used) {
                used[t] = false;
                return true;
            }
            used[t] = false;
        }
    }
    false
}

/// Install the trash selection for assembly element `element_idx`, chaining
/// to the next element on resolution. Element 0 is the optional gate
/// (declining plays the card at full cost); later elements are required
/// (exact count, DCGO `canEndNotMax: false`). After the final element the
/// play is finished at the reduced cost and the chosen materials are placed
/// under the new permanent.
#[cfg(feature = "dsl-yaml-loader")]
#[allow(clippy::too_many_arguments)]
fn install_assembly_element(
    game: &mut Game,
    player: PlayerId,
    target_card: CardHandle,
    params: AssemblyPlayParams,
    elements: std::sync::Arc<Vec<(digimon_dsl::compiled::CompiledPredicate, u8)>>,
    element_idx: usize,
    picked_so_far: Vec<CardHandle>,
) {
    use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
    use crate::effect_context::CountCappedZone;

    let (filter_pred, count) = elements[element_idx].clone();
    let is_first = element_idx == 0;
    let is_last = element_idx + 1 >= elements.len();
    let source_card = target_card;

    // Distinctness across elements: exclude handles already picked.
    let exclude = picked_so_far.clone();
    let filter = move |g: &Game, cs: &CardSource| -> bool {
        if exclude.contains(&cs.handle()) {
            return false;
        }
        let rctx = EffectReadContext::new(g, source_card, None, player);
        eval_predicate(&filter_pred, &rctx, PredicateSubject::Card(cs.handle()))
    };

    // Element 0 is the optional "use assembly?" gate (declineable at zero
    // picks); subsequent elements require their exact count.
    let min = if is_first { 0 } else { count };
    let max = count;
    let prompt = format!(
        "Assembly: select {} card(s) from trash to place under (No Selection to skip).",
        count
    );

    let elements_for_cb = std::sync::Arc::clone(&elements);
    let mut ctx = EffectContext::new(game, source_card, None, player);
    ctx.select_count_capped_multi_min(
        player,
        CountCappedZone::Trash,
        min,
        max,
        &prompt,
        is_first, // is_optional_zero: only the gate (element 0) may finish at 0 picks
        None,
        filter,
        move |cctx, picks| {
            let game: &mut Game = cctx.game;
            // Decline at the gate → play at full cost (no assembly used).
            if is_first && picks.is_empty() {
                let _ = assembly_finish(game, player, target_card, params, params.total_reduction);
                return;
            }
            let mut all_picked = picked_so_far;
            all_picked.extend(picks.iter().copied());
            if is_last {
                let total = params.total_reduction + params.assembly_reduction;
                // Hand the chosen materials to the commit, which places them
                // under the new permanent BEFORE its [On Play] effects fire.
                game.pending_assembly_materials = Some((target_card, all_picked));
                let _ = assembly_finish(game, player, target_card, params, total);
                // Clear any leftover (e.g. the play failed before commit) so
                // the slot can't leak into a later play.
                game.pending_assembly_materials = None;
            } else {
                install_assembly_element(
                    game,
                    player,
                    target_card,
                    params,
                    elements_for_cb,
                    element_idx + 1,
                    all_picked,
                );
            }
        },
    );
}

/// Re-resolve the hand index for `target_card` (the selection callbacks fire
/// after `decode_action` returns, but the queue is paused so the hand should
/// be unchanged) and finish the play at `total_reduction`.
#[cfg(feature = "dsl-yaml-loader")]
fn assembly_finish(
    game: &mut Game,
    player: PlayerId,
    target_card: CardHandle,
    params: AssemblyPlayParams,
    total_reduction: i32,
) -> PlayFromHandCostResult {
    let Some(hand_index) = game
        .player(player)
        .hand
        .iter()
        .position(|c| c.handle() == target_card)
    else {
        return PlayFromHandCostResult::Failed;
    };
    game.finish_play_from_hand_after_reductions(
        player,
        hand_index,
        target_card,
        params.cost_delta,
        params.source,
        params.origin,
        params.suppress_on_play,
        total_reduction,
    )
}
