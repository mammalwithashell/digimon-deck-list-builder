//! Player-driven game actions — split out of `game.rs` for readability.
//!
//! Everything here lives in `impl Game` blocks so the call surface is unchanged.
//! This is where `play_from_hand`, `digivolve_from_hand`, `initiate_dna_digivolve`,
//! and the `activate_*_main` [Main] effect dispatchers live. All three are invoked
//! by the action decoder and the Tauri/PyO3 bindings; none of them move here.

pub(crate) mod play_cost_assignment;
pub(crate) use play_cost_assignment::{CostPaymentReceipt, PlayCostAdjustments};

use crate::card_source::{CardHandle, CardSource};
use crate::digixros::DigiXrosMaterialOrigin;
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::{
    CardKind, CostDelta, EffectSourceKind, EffectTiming, GamePhase, Keyword, ModifierType,
    PlaySource, PlayerId, Zone,
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
pub(crate) mod effect_dna;
mod field_top_source_move;
mod helpers; // facade-decomposition Tier-2 primitives (trash_source_and_fire, would_replacement_*, …)
pub use helpers::DeDigivolveCountState;
mod link; // DigiLink Shape-B (Digimon-link) — activate/begin/commit/absorb + facet-#9 chosen-card link
pub(crate) use link::{LinkCardTrashChoiceState, LinkTrashPurpose};
pub use link::LinkRuleTarget;
mod misc;
mod movement;
mod grouped_return;
pub(crate) mod options;
mod play;
mod security;
mod sources;
mod zones;

/// Source zone for `play_option_core`. Private to this module — the public
/// API is the pair of `play_option_from_hand` / `play_option_from_trash`
/// entry points, plus the effect-driven origin-parameterized
/// `use_option_from_*` helpers (Gap 2, `G-DSL-USE-OPTION-FROM-SOURCES`).
///
/// Every variant routes through the SAME `play_option_core` pipeline (correct
/// [Main] resolution + `dispose_option`); only source validation (step 2) and
/// removal-from-source (step 5) fork on the variant.
#[derive(Clone, Copy, Debug)]
pub(crate) enum OptionSource {
    Hand(usize),
    Trash(usize),
    /// A card currently exposed in `Game::revealed_cards` (EX7-048: "reveal top
    /// 6 … you may use 1 Option among them free"). Identified by stable
    /// `CardHandle` — the reveal-pool index shifts as cards are consumed.
    Revealed(crate::card_source::CardHandle),
    /// A card sitting in `host`'s digivolution stack as a source (BT25-085:
    /// "use 1 Option from this Digimon's digivolution cards free"). Identified
    /// by `(host, card)` so a battle-area index shift can't misroute it.
    Source {
        host: crate::permanent::PermanentHandle,
        card: crate::card_source::CardHandle,
    },
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
    restore_pending_security_for: Option<PlayerId>,
}

/// Captured state for resuming `run_digixros_leave_windows_then_commit` after a
/// battle-area DigiXros material's `WhenWouldLeaveBattleArea` window parked a
/// `pending_selection` (the material is now in the leaving/limbo slot). Once the
/// parked reward settles, the loop re-enters at `next_material` and ultimately
/// finalizes the host play (G-DIGIXROS-REDIRECT-EXTRACTION).
#[derive(Debug, Clone)]
pub(crate) struct DigiXrosResumeContinuation {
    pub(crate) effect_controller: Option<PlayerId>,
    player: PlayerId,
    target_card: CardHandle,
    total_reduction: crate::game_actions::PlayCostAdjustments,
    source: PlaySource,
    suppress_on_play: bool,
    suspended: bool,
    source_play_receipt: Option<u64>,
    next_material: usize,
}

impl DigiXrosResumeContinuation {
    pub(crate) fn pending_cost_payment(&self) -> Option<u64> {
        self.total_reduction.pending_payment
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CostReductionKind {
    Play,
    Digivolve,
    OptionUse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OptionCostPolicy {
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
            OptionSource::Revealed(_) => OptionUseSource::Revealed,
            OptionSource::Source { .. } => OptionUseSource::Source,
        }
    }
}

/// One available play mode for an Option card. `classify_option_modes`
/// derives the set of modes from the card's effect list. Most Options have
/// exactly one mode; a dual-mode Plug-In Option that is both a Standard
/// `[Main]` Option and a Link Option has two (`Standard`, then `Link`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OptionPlayMode {
    /// Persistent field Option with no printed Delay activation.
    OrdinaryField,
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
            OptionPlayMode::OrdinaryField => OptionSubtype::OrdinaryField,
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
    EffectSourceKind::for_card_kind(kind)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CostReductionKey {
    source_card: crate::card_source::CardHandle,
    source_permanent: Option<crate::permanent::PermanentHandle>,
    controller: PlayerId,
    card_id: String,
    effect_slot: u8,
    is_under: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct CostReductionCandidate {
    assignment: Option<u16>,
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

pub(crate) struct BeforePayCostSourceInfo {
    source_permanent: Option<crate::permanent::PermanentHandle>,
    source_card: crate::card_source::CardHandle,
    card_id: String,
    is_under: bool,
    controller: PlayerId,
    effect_slot: u8,
}

#[derive(Debug, Clone)]
pub(crate) struct ArtsDigivolveSelectionState {
    pub(crate) owner: PlayerId,
    pub(crate) targets: Vec<PermanentHandle>,
    pub(crate) outer_conts: Vec<crate::resume::OuterContinuation>,
}

#[derive(Debug, Clone)]
pub(crate) struct LinkOptionHostSelectionState {
    pub(crate) owner: PlayerId,
    pub(crate) candidates: Vec<PermanentHandle>,
    pub(crate) optional: bool,
    pub(crate) outer_conts: Vec<crate::resume::OuterContinuation>,
}

/// The from-hand Plug-In **Option** link host prompt (`§6-5-1-4` declaration).
/// Unlike [`LinkOptionHostSelectionState`] — which is installed while the
/// Option is already mid-resolution, after its use cost was paid — this one is
/// installed BEFORE anything is paid or removed from the hand, because
/// `general_rule.pdf` (Ver.4.3) §10-1-3 pays the link cost only after the host
/// is chosen. The resolved pick re-enters `play_option_core` with the host
/// pinned on `Game::pending_option_link_host`.
#[derive(Debug, Clone)]
pub(crate) struct OptionHandLinkHostSelectionState {
    pub(crate) owner: PlayerId,
    pub(crate) hand_index: usize,
    /// The declared Option, addressed by handle so the re-entry can verify the
    /// hand slot still holds the same card.
    pub(crate) card: crate::card_source::CardHandle,
    pub(crate) cost: u16,
    pub(crate) candidates: Vec<PermanentHandle>,
}

#[derive(Debug, Clone)]
pub(crate) struct DigimonLinkHostSelectionState {
    pub(crate) owner: PlayerId,
    /// The linking card's origin (standing permanent or hand) — see
    /// `game::DigimonLinkOrigin`.
    pub(crate) origin: crate::game::DigimonLinkOrigin,
    /// The linking card itself. For a standing origin this is the top card at
    /// the source slot when the ability was activated; for a hand origin it is
    /// the hand card, addressed by handle because the hand can shift.
    pub(crate) card: crate::card_source::CardHandle,
    pub(crate) cost: u16,
    pub(crate) candidates: Vec<PermanentHandle>,
}

#[derive(Debug, Clone)]
pub(crate) struct PlayFromHandCostReductionPromptState {
    pub(crate) order: Option<play_cost_assignment::PlayCostOrder>,
    pub(crate) effect_controller: Option<PlayerId>,
    pub(crate) player_id: PlayerId,
    pub(crate) hand_index: usize,
    pub(crate) target: CostTargetContext,
    pub(crate) cost_delta: CostDelta,
    pub(crate) source: PlaySource,
    pub(crate) origin: crate::game::PendingWouldPlayOrigin,
    pub(crate) suppress_on_play: bool,
    pub(crate) accumulated_reduction: crate::game_actions::PlayCostAdjustments,
    pub(crate) processed: Vec<CostReductionKey>,
    pub(crate) key: CostReductionKey,
    pub(crate) outer_conts: Vec<crate::resume::OuterContinuation>,
}

#[derive(Debug, Clone)]
pub(crate) struct InteractiveDigivolveCostReductionPromptState {
    pub(crate) acting_player: PlayerId,
    pub(crate) target: CostTargetContext,
    pub(crate) hand_index: usize,
    pub(crate) field_index: usize,
    pub(crate) source: PlaySource,
    pub(crate) key: CostReductionKey,
    pub(crate) outer_conts: Vec<crate::resume::OuterContinuation>,
}

#[derive(Debug, Clone)]
pub(crate) struct InteractiveOptionUseCostReductionPromptState {
    pub(crate) player_id: PlayerId,
    pub(crate) cost_target: CostTargetContext,
    pub(crate) source: OptionSource,
    pub(crate) mode: OptionPlayMode,
    pub(crate) cost_policy: OptionCostPolicy,
    pub(crate) key: CostReductionKey,
    pub(crate) outer_conts: Vec<crate::resume::OuterContinuation>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CostTargetContext {
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
    if effects.iter().any(|effect| effect.ordinary_field_option) {
        return vec![OptionPlayMode::OrdinaryField];
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

/// One digivolution card leaving `host`'s stack for the trash, as one member
/// of a source-trash event (`Game::enqueue_digivolution_cards_trashed`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct TrashedSource {
    pub player: PlayerId,
    pub host: PermanentHandle,
    pub host_card: CardHandle,
    pub card: CardHandle,
}

/// Token for an open source-move event (`Game::begin_source_move_event`): one
/// step trashing digivolution cards, or returning them to the bottom of a
/// deck (renamed from `SourceTrashEvent` when returns joined the scope).
#[must_use]
pub(crate) struct SourceMoveEvent {
    outermost: bool,
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

        if source == PlaySource::ByEffect && self.effect_play_is_prohibited(player_id, card_kind) {
            return PlayFromHandCostResult::Failed;
        }
        let controller = if source == PlaySource::ByEffect {
            self.effect_source_player.unwrap_or(player_id)
        } else {
            player_id
        };
        if self.card_play_is_prohibited(controller, &self.player(player_id).hand[hand_index]) {
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
            PlayCostAdjustments::new(cost_delta),
            Vec::new(),
        )
    }

    fn pending_digixros_material_candidates(
        &self,
        player_id: PlayerId,
    ) -> Vec<(u16, DigiXrosMaterialOrigin)> {
        let Some(transaction) = self.pending_digixros_transaction.as_ref() else {
            return Vec::new();
        };
        self.digixros_material_candidates_for(transaction, player_id)
    }

    /// Candidate material origins (with their selection action ids) for a
    /// DigiXros / cast-time-assembly `transaction`, in the mask's selection
    /// order. Extracted from `pending_digixros_material_candidates` so the
    /// mask-time availability probe (`cast_time_assembly_play_reduction_for_
    /// hand_card`) can enumerate against a scratch transaction that is not
    /// (yet) the pending one.
    fn digixros_material_candidates_for(
        &self,
        transaction: &crate::digixros::DigiXrosTransaction,
        player_id: PlayerId,
    ) -> Vec<(u16, DigiXrosMaterialOrigin)> {
        use crate::action::space::{
            encode_source_select, HAND_EFFECT_START, HAND_MAIN_LIMIT, MAX_FIELD_SLOTS,
            TRASH_EFFECT_START, TRASH_MAIN_LIMIT,
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
        use digimon_dsl::compiled::{
            CompiledAltPathKind, CompiledCost, CompiledRepeat, CompiledZone,
        };

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
        // stack under the played card. `distinct_by` is carried per element
        // (G-ASSEMBLY-NO-DISTINCT-BY): "3 [Huckmon] text Digimon cards
        // w/different names" — EX13-061 Gankoomon / EX13-063 / EX12-076.
        let mut elements: Vec<(digimon_dsl::compiled::CompiledPredicate, u8)> = Vec::new();
        let mut distincts: Vec<Option<digimon_dsl::compiled::CompiledDistinctBy>> = Vec::new();
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
            distincts.push(m.distinct_by);
        }
        if elements.is_empty() {
            return None;
        }

        // Eligibility: each element satisfiable from trash with distinct cards
        // (DCGO `CanFulfillConditions` / recursive distinct-assignment check),
        // honoring each element's `distinct_by` key.
        if !self.assembly_can_fulfill(player_id, target_card, &elements, &distincts) {
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

    /// The maximum cost reduction a `cast_time_assembly` alt-path (BT15-102
    /// Apocalymon) could realize for hand card `hand_index` right now, or `0`
    /// when the card has no such path. Mirrors DCGO's hidden availability
    /// `ChangeCostClass` (`isCheckAvailability: true` — `min(3, unique-name
    /// candidate count) × 4`): the action mask offers the play under
    /// declare-then-pay legality against the REDUCED cost, exactly like the
    /// `[Assembly]` reduction above.
    ///
    /// Computed by greedily filling a scratch transaction from the same
    /// candidate enumeration the real selection loop uses, so distinct-name
    /// masking, zone allowances, and slot caps all apply. Greedy fill is
    /// exact for a single recipe slot (the whole `cast_time_assembly` corpus
    /// today); a future multi-slot path with overlapping filters would need
    /// a matching pass instead.
    ///
    /// Printed-[DigiXros] paths return 0 — their mask affordability keeps the
    /// pre-existing printed-cost behavior.
    pub(crate) fn cast_time_assembly_play_reduction_for_hand_card(
        &self,
        player_id: PlayerId,
        hand_index: usize,
    ) -> i32 {
        // Cheap pre-gate: only cards whose FIRST matching transaction path is
        // `cast_time_assembly` pay for the scratch-transaction probe below
        // (the mask hot loop calls this for every hand card; printed-DigiXros
        // hosts return 0 without building a transaction).
        #[cfg(feature = "dsl-yaml-loader")]
        {
            use digimon_dsl::compiled::CompiledAltPathKind as K;
            let Some(card) = self.player(player_id).hand.get(hand_index) else {
                return 0;
            };
            let card_id = card.card_id(&self.card_data);
            let is_cast_time = self.alt_path_registry.get(card_id).is_some_and(|paths| {
                paths
                    .iter()
                    .find(|path| matches!(path.kind, K::DigiXros | K::CastTimeAssembly))
                    .is_some_and(|path| matches!(path.kind, K::CastTimeAssembly))
            });
            if !is_cast_time {
                return 0;
            }
        }
        let Some(mut transaction) =
            self.build_digixros_transaction_for_hand_card(player_id, hand_index)
        else {
            return 0;
        };
        if transaction.is_digixros {
            return 0;
        }
        let base = transaction.final_cost() as i32;
        loop {
            let candidates = self.digixros_material_candidates_for(&transaction, player_id);
            let Some((_, origin)) = candidates.first().copied() else {
                break;
            };
            let Some(card_data) = self.card_data_for_handle(origin.card()) else {
                break;
            };
            let card_data = card_data.clone();
            if transaction.try_select_material(origin, &card_data).is_err() {
                break;
            }
        }
        (base - transaction.final_cost() as i32).max(0)
    }

    /// True when each assembly element can be matched to a DISTINCT card in
    /// the controller's trash (a system of distinct representatives exists).
    #[cfg(feature = "dsl-yaml-loader")]
    fn assembly_can_fulfill(
        &self,
        player_id: PlayerId,
        source_card: CardHandle,
        elements: &[(digimon_dsl::compiled::CompiledPredicate, u8)],
        distincts: &[Option<digimon_dsl::compiled::CompiledDistinctBy>],
    ) -> bool {
        use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};

        let rctx = EffectReadContext::new(self, source_card, None, player_id);
        let trash = &self.player(player_id).trash;

        // Expand elements into one slot per required card, remembering each
        // slot's element index (for the per-element `distinct_by` key).
        let mut slots: Vec<&digimon_dsl::compiled::CompiledPredicate> = Vec::new();
        let mut slot_element: Vec<usize> = Vec::new();
        for (e, (pred, count)) in elements.iter().enumerate() {
            for _ in 0..*count {
                slots.push(pred);
                slot_element.push(e);
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

        // Per-element distinctness constraint over trash indices.
        let element_distinct: Vec<AssemblyDistinct> = (0..elements.len())
            .map(|e| match distincts.get(e).copied().flatten() {
                None => AssemblyDistinct::None,
                Some(digimon_dsl::compiled::CompiledDistinctBy::Color) => AssemblyDistinct::Colors(
                    trash
                        .iter()
                        .map(|cs| {
                            crate::effect_context::selections::color_mask(
                                &self.card_data[cs.data_index].colors,
                            )
                        })
                        .collect(),
                ),
                Some(mode) => AssemblyDistinct::Keys(
                    trash
                        .iter()
                        .map(|cs| assembly_distinct_key(mode, &self.card_data[cs.data_index]))
                        .collect(),
                ),
            })
            .collect();

        // Cheap necessary condition for a `Color` element: its filter-passing
        // cards must contain `count` cards w/different colors (the transversal
        // matroid's rank). Rejects the common unaffordable case without the
        // backtracking search below (mask hot path).
        for (e, (_, count)) in elements.iter().enumerate() {
            if let AssemblyDistinct::Colors(masks) = &element_distinct[e] {
                let Some(first_slot) = slot_element.iter().position(|&se| se == e) else {
                    continue;
                };
                let element_masks: Vec<u8> =
                    match_table[first_slot].iter().map(|&t| masks[t]).collect();
                if crate::effect_context::selections::max_color_matching(&element_masks)
                    < usize::from(*count)
                {
                    return false;
                }
            }
        }

        let mut used = vec![false; trash.len()];
        let mut chosen: Vec<Option<usize>> = vec![None; slots.len()];
        assembly_assign_keyed(
            &match_table,
            &slot_element,
            &element_distinct,
            0,
            &mut used,
            &mut chosen,
        )
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
                if let Some(prepared) = self.prepare_source_play_if_registered(resume.clone()) {
                    if !prepared { self.restore_pending_would_play_origin(resume); }
                    self.pending_digixros_transaction = None;
                    return;
                }
                let previous_controller = self.effect_source_player;
                self.effect_source_player = Some(resume.play_controller);
                let played = self.commit_play_from_hand_card_no_replace(
                    resume.player,
                    resume.card,
                    resume.effective_cost,
                    resume.effect_initiated,
                    resume.suppress_on_play,
                    resume.origin.enters_suspended(),
                    resume.origin.source_play_receipt(),
                    matches!(resume.origin, PendingWouldPlayOrigin::SecurityTop { .. }),
                );
                self.effect_source_player = previous_controller;
                if played.is_none() {
                    self.restore_pending_would_play_origin(resume);
                }
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
    /// own Digimon that passes the card's `link_filter`. A host at its link
    /// limit stays a candidate: the link replaces a link card (§4-9-5).
    /// Shared by `dispose_option`'s Link arm and the dual-mode legality check
    /// in `option_legal_play_modes`.
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
            // The same requirement test the §17-1-3-2-6 / -7 rule check applies.
            if self.link_requirement_met_by(source_card, effects, handle, owner) {
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
    /// Two things are filtered here: affordability, and — for the Link mode —
    /// the existence of at least one legal HOST.
    ///
    /// `F-ENGINE-PLUGIN-MODE-SELECT-WITHOUT-HOST`. general_rule.pdf §10-1-3-1:
    /// "The player declares a link and reveals 1 card to link. 1 link
    /// requirement is chosen on the revealed card, then the player chooses 1 of
    /// their Digimon that meets the requirement." With no Digimon meeting the
    /// requirement the link procedure cannot be carried out, so the §6-5-1-4
    /// "Linking a Card in the Hand or Battle Area" main-phase action is not
    /// available at all. DCGO enforces exactly this: `CardEffectFactory
    /// .LinkEffect` returns `null` when `!HasMatchConditionPermanent(
    /// CanSelectPermanentCondition)` (`Link.cs:24`, re-checked in
    /// `CanUseCondition` at `Link.cs:53`), so the declaration is never offered.
    /// The Digimon-side (Shape-B) hand link already holds this contract via
    /// `hand_digimon_link_available`; this is the Option-side half.
    ///
    /// `dispose_option`'s "no candidate → trash" arm stays as the resolution-
    /// time safety net for a host that disappears mid-resolution; it is no
    /// longer reachable from a player-declared Link play.
    pub(crate) fn option_legal_play_modes(
        &self,
        card: &CardSource,
        player_id: PlayerId,
        source: OptionSource,
    ) -> Vec<OptionPlayMode> {
        let effects = self
            .effects_for_card(card.card_id(&self.card_data), card.handle())
            .unwrap_or_default();
        let use_cost = self.option_use_cost(card, player_id);
        let memory_min = self.rules.memory_range.0;
        let mut modes = classify_option_modes(&effects);
        modes.retain(|mode| {
            let cost = match mode {
                OptionPlayMode::Link { cost } => (*cost as i32
                    + self.modifiers.link_cost_delta_for_player(player_id))
                .max(0) as i16,
                _ => use_cost as i16,
            };
            (self.memory - cost) >= memory_min
        });
        if modes.iter().any(|m| m.is_link())
            && self
                .link_host_candidates(player_id, card.handle(), &effects)
                .is_empty()
        {
            modes.retain(|m| !m.is_link());
        }
        // `G-ENGINE-OPTION-LINK-FROM-HAND`. From the HAND the Link mode is not
        // a mode of this action at all. general_rule.pdf (Ver.4.3) §6-5-1 lists
        // "use an Option card from the hand" (§6-5-1-3) and "link a card from
        // the hand or battle area" (§6-5-1-4) as two SEPARATE main-phase
        // actions, and §10-1-1 says a card is linked from the hand "by paying
        // the cost as part of the main phase actions" — its own declaration,
        // not a branch of using the card. So the from-hand link lives on the
        // `HAND_EFFECT` bit (`hand_option_link_available` /
        // `activate_hand_link`), exactly where the Link DIGIMON's already does,
        // and the PLAY bit means only §6-5-1-3.
        //
        // DCGO draws the same line: the from-hand link is an
        // `ActivateCardAction` over `CardEffectFactory.LinkEffect`
        // (`Link.cs:19`), never a `PlayCardAction`.
        //
        // Every OTHER source keeps the Link mode: a `[Security]`-flipped
        // Option resolves into play with no main-phase declaration to make, so
        // its link is a mode of that resolution (G-ENGINE-SECURITY-OPTION-LINK-
        // TO-OWN-DIGIMON, `35958972b`), and the same holds for the trash /
        // revealed / digivolution-source uses an effect drives.
        if matches!(source, OptionSource::Hand(_)) {
            modes.retain(|m| !m.is_link());
        }
        // G-ENGINE-CANNOT-USE-OPTION-CARDS: Standard / Delay / Training are
        // all "use an Option card" (§6-5-1-3); only the Link mode survives.
        if self.option_use_blocked(player_id) {
            modes.retain(|m| m.is_link());
        }
        modes
    }

    /// <Arts Digivolve> base check: "your cards may digivolve into this card"
    /// honours EVERY printed digivolution requirement of the DUAL card's
    /// Digimon face — the standard level/colour circles (`can_digivolve`) AND
    /// the special conditions authored as DSL alt-digivolve paths
    /// ("[Digivolve] [Sistermon Blanc]: Cost 0", "Lv.2 w/[Huckmon] in text").
    /// EX13-065 / EX13-066 print NO standard circles, so a circles-only check
    /// left their Arts clause with no legal base. G-ENGINE-ARTS-ALT-PATH-BASE.
    fn arts_can_digivolve_onto(
        &self,
        card: &crate::card_source::CardSource,
        base: &crate::permanent::Permanent,
        base_handle: PermanentHandle,
    ) -> bool {
        self.can_digivolve(card, base)
            || !self
                .all_digivolve_routes_for_card(card, base_handle)
                .is_empty()
    }

    fn arts_breeding_handle(owner: PlayerId) -> PermanentHandle {
        PermanentHandle {
            player: owner,
            index: crate::action::space::BREEDING_TARGET as u8,
        }
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
                if self.arts_can_digivolve_onto(&pending.card, perm, handle) {
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
        self.arts_can_digivolve_onto(&pending.card, breeding, Self::arts_breeding_handle(owner))
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
            zone_owner: Some(owner),
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
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::ArtsDigivolveSelection(
                ArtsDigivolveSelectionState {
                    owner,
                    targets,
                    outer_conts: Vec::new(),
                },
            )],
        });
        true
    }

    pub(crate) fn run_arts_digivolve_selection_step(
        &mut self,
        state: ArtsDigivolveSelectionState,
        action_id: u16,
        is_pass: bool,
    ) {
        use crate::action::space::{ATTACK_START, BREEDING_SELECTION_TARGET, TARGETS_PER_ATTACKER};

        if is_pass {
            self.dispose_option();
            self.check_turn_end();
            return;
        }

        if action_id == BREEDING_SELECTION_TARGET {
            let _ = self.arts_digivolve_pending_option_onto_breeding(state.owner);
            return;
        }

        let offset = action_id.saturating_sub(ATTACK_START);
        let target_index = (offset % TARGETS_PER_ATTACKER) as u8;
        if state.targets.iter().any(|h| h.index == target_index) {
            let target = PermanentHandle {
                player: state.owner,
                index: target_index,
            };
            let _ = self.arts_digivolve_pending_option_onto_battle(target);
        }
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
        if !self.arts_can_digivolve_onto(&pending_ref.card, perm, target) {
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
        if !self.arts_can_digivolve_onto(
            &pending_ref.card,
            breeding,
            Self::arts_breeding_handle(owner),
        ) {
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

    /// The rule check (§17-1-3): every target is processed SIMULTANEOUSLY.
    ///
    /// Runs at a timing when rule checks are possible (§17-1-1): the
    /// outermost effect-queue drain calls it before activating queued
    /// triggers, between top-level queued effects, and in a fixpoint loop
    /// after the queue empties — never mid-effect (§17-1-2-2).
    ///
    /// Shape: **collect → apply trashes → ONE deletion batch → link-excess
    /// choice → ONE flush.**
    ///
    /// 1. Collect every target from the current board before touching it:
    ///    - battle area, per permanent: a face-down top (§17-1-3-2-4); a
    ///      DP-less Digimon / Digi-Egg / DUAL top or an exposed `Standard`
    ///      Option top (§17-1-3-2-1, Q1921 for §17-1-3-2-2); a Digimon at
    ///      0 DP or less (§17-1-3-1-1, the only deletion target);
    ///    - breeding area: a face-down top, or a non-Digimon top an effect did
    ///      not place there (§17-1-3-2-3 / -4,
    ///      [`Self::breeding_area_is_rule_trash_target`]);
    ///    - per link host (`collect_link_rule_targets`, hosts that are
    ///      themselves leaving skipped): unqualified link cards (§17-1-3-2-6 /
    ///      -7) and a host over its link limit (§17-1-3-2-5).
    /// 2. Inside ONE `enter_deferred_drain` scope: trash every trash target
    ///    (breeding, battle area in descending `(player, index)` so a removal
    ///    never shifts a pending handle, link cards), apply their `<Overflow>`
    ///    once (turn player first, §4-19-5), then delete every 0-DP Digimon in
    ///    ONE `delete_permanents_batch` (its handles remapped past the
    ///    battle-area trashes), then park the §17-1-3-2-5 choice of the first
    ///    over-limit host (its controller picks, Q6370) from a fresh scan.
    /// 3. `exit_deferred_drain_and_flush` once: every `[On Deletion]`, deletion
    ///    observer and `OnLinkedCardTrashed` this check triggered pends
    ///    together (§15-4-3-2) and the turn player orders theirs first
    ///    (§15-4-3-5). A parked selection (the link-excess choice, or a
    ///    replacement window inside the batch) holds the flush until it
    ///    resolves, so no trigger activates before every target is processed.
    ///
    /// Trashes are not deletions: they fire no `[On Deletion]` / leave
    /// triggers. Rule processing is not an effect, so "can't be affected by
    /// effects" does not stop it. The deletion is a rule action: its observer
    /// cause is `EventCause::Rule` (the `current_deletion_event_cause_override`
    /// slot, captured into each trigger's snapshot at enqueue); the batch's
    /// replacement cause is inferred as before (there is no
    /// `ReplacementCause::Rule`).
    ///
    /// A replacement window parked inside the batch
    /// (`G-ENGINE-RULES-CHECK-CONTINUES-PAST-PARKED-DELETION`): the batch
    /// stashes its other members in `pending_deletion_batch_rest`, which the
    /// post-selection path deletes; the link-excess choice is then left to the
    /// rule check that the resuming drain runs before any trigger activates.
    ///
    /// Returns `true` if it applied at least one target (deleted or trashed a
    /// permanent, trashed link cards, or parked / applied the §17-1-3-2-5
    /// trim); the drain's fixpoint loop re-runs it until it returns `false`
    /// or a selection parks. Idempotent on a legal board.
    pub(crate) fn run_state_based_rules_check(&mut self) -> bool {
        // Continuous effects must include new entrants (and removed aura
        // sources) before rule processing reads their effective DP. Callers
        // already restrict this check to completed resolution boundaries.
        self.tick_declarative_effects();
        // Per-permanent battle-area rule-check action. The trash targets and
        // the deletion target are disjoint per permanent: a face-down top is
        // trashed whatever its DP; a DP-less top has no DP to be ≤ 0.
        enum RuleAction {
            /// §17-1-3-1-1 — a Digimon at ≤0 DP is DELETED (fires OnDeletion).
            DeleteZeroDp,
            /// §17-1-3-2-1 / §17-1-3-2-2 (Q1921) / §17-1-3-2-4 — the
            /// permanent "is considered to not be in any area and is trashed":
            /// a DP-less Digimon/Digi-Egg/DUAL or exposed Option top, or a
            /// face-down top. A TRASH of the whole permanent, NOT a
            /// deletion-from-battle-area, so it fires no OnDeletion observers.
            TrashPermanent,
            /// §17-1-3-2-6 / -7 — link cards the host no longer qualifies
            /// for are trashed (no choice; the host stays).
            TrashUnqualifiedLinks(Vec<CardHandle>),
            /// §17-1-3-2-5 — the host is this many link cards over its link
            /// limit; its controller chooses which (Q6370), parked after the
            /// other targets are applied.
            TrimLinks(usize),
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
                // §17-1-3-2-4 "Face-down cards placed on the field": keyed on
                // the TOP card (a face-down card stacked under a permanent is
                // that permanent's card, not a card placed on the field; DCGO
                // `AutoProcessing.IsPermanentFaceDown` reads `TopCard`).
                if self.players[pid].battle_area[idx].top_card().face_down {
                    actions.push((handle, RuleAction::TrashPermanent));
                    continue;
                }
                // §17-1-3-2-1 — "A Digimon without DP in the battle area is
                // considered to not be in any area and is trashed." The rule
                // keys on the ABSENCE of a DP value (general_rule.pdf
                // §17-1-3-2-1; DCGO `Permanent.HasDP` likewise only fails for
                // a top that lacks DP), NOT on the Digi-Egg card type: the
                // canonical case is a stack peeled/de-digivolved down to its
                // egg (every printed Digi-Egg has no DP), while a
                // Digimon-presenting top WITH a DP value is a legal
                // battle-area Digimon. A legitimately-played Tamer/Option
                // permanent is NOT caught (its top kind is Tamer/Option,
                // governed by §17-1-3-2-2/-3 instead).
                let presents_as_digimon = matches!(
                    self.players[pid].battle_area[idx]
                        .top_card()
                        .card_kind(&self.card_data),
                    crate::enums::CardKind::Digimon
                        | crate::enums::CardKind::DigiEgg
                        | crate::enums::CardKind::Dual
                );
                // Q1921: an Option exposed on a former Digimon stack has no
                // legitimate field-Option state. It is likewise in no area and
                // trashed at this resolution boundary, not deleted. Real Delay,
                // Training and OrdinaryFieldOption permanents remain valid.
                let exposed_option = self.players[pid].battle_area[idx].is_option(&self.card_data)
                    && matches!(
                        self.players[pid].battle_area[idx].option_state,
                        crate::permanent::OptionState::Standard
                    );
                if (presents_as_digimon || exposed_option) && self.effective_dp(handle).is_none() {
                    actions.push((handle, RuleAction::TrashPermanent));
                    continue;
                }
                if self.permanent_is_digimon_for_rules(handle)
                    && self.effective_dp(handle).unwrap_or(1) <= 0
                {
                    actions.push((handle, RuleAction::DeleteZeroDp));
                }
            }
        }
        // §17-1-3-2-3 / -4 in the breeding area.
        let breeding_targets: Vec<PlayerId> = (0..self.players.len())
            .map(|pid| pid as PlayerId)
            .filter(|&pid| self.breeding_area_is_rule_trash_target(pid))
            .collect();
        // §17-1-3-2-5..7 per-host link targets (hosts leaving the area are
        // skipped: their link cards go with them).
        let leaving: Vec<PermanentHandle> = actions.iter().map(|(h, _)| *h).collect();
        for (host, target) in self.collect_link_rule_targets() {
            if leaving.contains(&host) {
                continue;
            }
            actions.push(match target {
                LinkRuleTarget::Unqualified(cards) => {
                    (host, RuleAction::TrashUnqualifiedLinks(cards))
                }
                LinkRuleTarget::Excess(n) => (host, RuleAction::TrimLinks(n)),
            });
        }
        if actions.is_empty() && breeding_targets.is_empty() {
            return false;
        }
        actions.sort_by_key(|(h, _)| (h.player, h.index));

        // §17-1-3: one simultaneous process. Every trigger it causes is held
        // until the single flush at the end of this scope (§15-4-3-2).
        self.enter_deferred_drain();
        let mut acted = false;
        let mut trim_pending = false;
        let mut overflow_sources: Vec<crate::card_source::CardSource> = Vec::new();
        for pid in breeding_targets {
            overflow_sources.extend(self.trash_breeding_permanent_by_rule(pid));
            acted = true;
        }
        let mut deletions: Vec<PermanentHandle> = Vec::new();
        let mut trashed_slots: Vec<PermanentHandle> = Vec::new();
        // Apply in descending (player, index) order so each removal — which
        // shifts higher battle-area indices via `Vec::remove` — does not
        // invalidate the handles still pending.
        for (handle, action) in actions.into_iter().rev() {
            match action {
                RuleAction::TrashUnqualifiedLinks(cards) => {
                    self.trash_link_cards_by_rule(handle, &cards);
                    acted = true;
                }
                RuleAction::TrimLinks(excess) => trim_pending |= excess > 0,
                RuleAction::DeleteZeroDp => deletions.push(handle),
                RuleAction::TrashPermanent => {
                    overflow_sources.extend(self.trash_battle_permanent_by_rule(handle));
                    trashed_slots.push(handle);
                    acted = true;
                }
            }
        }
        // <Overflow> observes the trashed cards' physical movement (§4-19-1)
        // even though nothing here is a deletion. The deletion batch below
        // runs its own single pass for the deleted stacks, so the trash
        // targets' instances are split around it by owner: the turn player's
        // BEFORE the batch, the non-turn player's AFTER it. Every
        // turn-player instance of this rule check (trashed, then deleted)
        // is processed before every non-turn-player instance (deleted, then
        // trashed), as §4-19-5-1 / -2 require; within one player the order
        // is immaterial (every instance moves the marker the same way).
        // When the batch parks on a replacement window, the parked members'
        // instances follow on resume, after the non-turn player's trashed
        // ones: that sequence is already non-simultaneous (pre-existing,
        // the batch's own park path).
        let turn_player = self.turn_player();
        let (turn_player_overflow, other_overflow): (Vec<_>, Vec<_>) = overflow_sources
            .into_iter()
            .partition(|card| card.owner == turn_player);
        if !turn_player_overflow.is_empty() {
            self.apply_ace_overflow_for_sources(&turn_player_overflow);
        }
        if !deletions.is_empty() {
            acted = true;
            // The deletion targets were collected before the trashes; move
            // each past the battle-area slots trashed below it.
            let mut kill_list: Vec<PermanentHandle> = deletions
                .into_iter()
                .map(|h| PermanentHandle {
                    player: h.player,
                    index: h.index
                        - trashed_slots
                            .iter()
                            .filter(|t| t.player == h.player && t.index < h.index)
                            .count() as u8,
                })
                .collect();
            kill_list.sort_by_key(|h| (h.player, h.index));
            // A ≤0-DP deletion is a STATE-BASED RULE action, not an effect.
            // `infer_deletion_cause` would otherwise attribute it to
            // OwnEffect/OpponentEffect (no battle/security/effect-source is
            // live between top-level effects), which is wrong for observers
            // that distinguish "deleted by having 0 DP" from "deleted by an
            // effect" (e.g. BT16-101's [All Turns] gain-2-memory clause keys
            // on battle OR 0-DP, NOT effect deletion). Refine the OnDeletion /
            // OnAnyDeletion observer payload to `EventCause::Rule` via the
            // same override slot Overclock uses (game_phases.rs); it only
            // refines the observer cause (captured into each trigger's
            // snapshot at enqueue, so it survives the deferred flush) —
            // replacement-window filtering still reads `current_deletion_cause`
            // — and the deferred-replacement path (e.g. an Armor Purge window
            // opened by this deletion) threads it through ParkedReplacement.
            let cause = self.infer_deletion_cause(kill_list[0]);
            let previous = self.current_deletion_event_cause_override;
            self.current_deletion_event_cause_override =
                Some(crate::trigger_context::EventCause::Rule);
            // ONE batch: every 0-DP Digimon is deleted simultaneously, so
            // their `[On Deletion]` effects pend together (§15-4-3-2).
            self.delete_permanents_batch(kill_list, cause);
            self.current_deletion_event_cause_override = previous;
        }
        if !other_overflow.is_empty() {
            self.apply_ace_overflow_for_sources(&other_overflow);
        }
        // §17-1-3-2-5: the controller's choice of excess link cards, from a
        // fresh scan (removals above may have shifted hosts). Skipped while a
        // replacement window the batch opened is parked: the drain that
        // resumes after it runs the rule check before any trigger activates,
        // and parks the choice then.
        //
        // Order dependence: this relies on `resolve_generic_selection`
        // calling `drain_effect_queue` BEFORE `resume_pending_deletion`, and
        // on the outermost drain running this rule check BEFORE
        // `drain_effect_queue_inner` activates any queued `[On Deletion]`.
        // That re-run parks the choice before any trigger of this check
        // activates, and it also deletes any 0 DP batch member still on the
        // field (with the `EventCause::Rule` override set here; the
        // rest-of-batch stash carries only a `ReplacementCause`), so
        // `resume_deletion_batch_rest_for_receipt` then finds them gone. If
        // those two calls are ever reordered, both guarantees break.
        if trim_pending && self.pending_selection.is_none() && self.park_first_link_excess_trim() {
            acted = true;
        }
        self.exit_deferred_drain_and_flush();
        acted
    }

    /// Trash a battle-area permanent by rule (§17-1-3-2-1 / -2 / -4): every
    /// stacked card, then its link cards, goes to its owner's trash face up
    /// (§3-6-3) with no deletion/leave triggers ("considered to not be in any
    /// area"). Returns the leaving cards for the caller's single `<Overflow>`
    /// pass (§4-19-5).
    fn trash_battle_permanent_by_rule(
        &mut self,
        handle: PermanentHandle,
    ) -> Vec<crate::card_source::CardSource> {
        let movement_snapshot = self.movement_snapshot(handle);
        let permanent =
            &self.players[handle.player as usize].battle_area[handle.index as usize];
        // Overflow observes physical movement even though this is not
        // deletion or an OnLeaveField event. Include linked cards too.
        let mut leaving_cards = permanent.card_sources.clone();
        leaving_cards.extend(permanent.linked_cards.iter().cloned());
        self.clear_permanent_full(handle);
        self.modifiers.expire_player_on_permanent_leave(handle);
        self.trash_permanent_stack(handle.player, handle.index as usize);
        self.modifiers
            .shift_after_battle_area_remove(handle.player, handle.index);
        self.shift_pending_attack_after_battle_area_remove(handle.player, handle.index);
        self.shift_effect_queue_after_battle_area_remove(handle.player, handle.index);
        self.mark_until_condition_dirty();
        self.observe_rule_trash_completed(movement_snapshot);
        leaving_cards
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
        //
        // G-ENGINE-DELAY-OPTION-CONDITIONAL-PLACEMENT: a Delay Option whose
        // [Main] body authors its OWN `place_self_as_delay_option` (flagged
        // `explicit_self_placement` at lowering) placed itself already if it
        // was going to — the step claims `pending_option`, so reaching here
        // with the slot still occupied means the body ended without placing
        // it (BT24-099 official Q&A: decline the "By trashing …" cost and
        // nothing after "then" happens). Such a card is disposed of like a
        // Standard Option (trash). Delay Options WITHOUT the step keep the
        // implicit dispose-time placement below.
        let subtype = match pending.subtype {
            OptionSubtype::Delay(_)
                if effects.iter().any(|e| {
                    e.explicit_self_placement
                        && matches!(
                            e.timing,
                            EffectTiming::OptionMain | EffectTiming::MainFromHand
                        )
                }) =>
            {
                OptionSubtype::Standard
            }
            other => other,
        };

        match subtype {
            OptionSubtype::OrdinaryField => {
                self.install_field_option_as_ordinary(pending.card, pending.owner);
            }
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
                    // §9-1-5: no host to link to -- trashed like any used
                    // Option, with its `GameEvent::Trash`.
                    self.trash_card(owner, pending.card);
                    return;
                }

                // `G-ENGINE-OPTION-HAND-LINK-COST-TIMING`: a from-hand §6-5-1-4
                // declaration already asked §10-1-3-1's host question BEFORE
                // anything was paid (`install_option_hand_link_host_selection`),
                // and pinned the answer here. Plug straight into it — asking
                // again would be a second prompt for one choice. The pin is
                // re-checked against the live candidate set because the
                // link-mode body could have moved the board between the pick
                // and the disposal; a pin that no longer qualifies falls back
                // to the ordinary prompt rather than forcing an illegal host.
                let pinned = self
                    .pending_option_link_host
                    .take()
                    .filter(|h| candidates.contains(h));

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
                match pinned {
                    Some(host) => self.attach_linked_card(host),
                    None => self.install_link_host_selection(owner, source_card, candidates, false),
                }
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
    /// - `None` — commit the original event: trash the card (§9-1-5) through
    ///   `Game::trash_card`, which logs the `GameEvent::Trash`
    ///   (G-ENGINE-OPTION-TRASH-NO-EVENT). The hand / deck redirects log
    ///   `GameEvent::CardToHand` / `GameEvent::CardToDeck`.
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
        let card_id = pending.card.card_id(&self.card_data).to_string();
        let card_name = self.card_data[pending.card.data_index].card_name.clone();
        match outcome {
            ReplacementOutcome::None => {
                self.trash_card(pending.owner, pending.card);
            }
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                self.player_mut(pending.owner).put_in_hand(pending.card);
                self.log_card_to_hand(pending.owner, card_id, card_name);
            }
            ReplacementOutcome::Redirected(crate::enums::Zone::Deck) => {
                self.player_mut(pending.owner).insert_in_deck(0, pending.card);
                self.log_card_to_deck(pending.owner, card_id, card_name, "bottom");
            }
            ReplacementOutcome::Redirected(crate::enums::Zone::Hand) => {
                self.player_mut(pending.owner).put_in_hand(pending.card);
                self.log_card_to_hand(pending.owner, card_id, card_name);
            }
            ReplacementOutcome::Redirected(other) => {
                debug_assert!(
                    false,
                    "unexpected Redirected({:?}) for Option trash — only Deck/Hand supported in v1",
                    other
                );
                self.trash_card(pending.owner, pending.card);
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(
                    false,
                    "Substituted not supported for Option WhenWouldBeTrashed v1"
                );
                self.trash_card(pending.owner, pending.card);
            }
        }
    }

    /// Log `GameEvent::CardToHand` for a used Option a replacement outcome
    /// returned to its owner's hand instead of the trash.
    fn log_card_to_hand(&mut self, player: PlayerId, card_id: String, card_name: String) {
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::CardToHand {
            seq,
            player,
            card_id,
            card_name,
            source: "option_use".to_string(),
        });
    }

    /// Log `GameEvent::CardToDeck` for a used Option a replacement outcome
    /// redirected to its owner's deck instead of the trash.
    fn log_card_to_deck(
        &mut self,
        player: PlayerId,
        card_id: String,
        card_name: String,
        position: &str,
    ) {
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::CardToDeck {
            seq,
            player,
            card_id,
            card_name,
            position: position.to_string(),
            source: "option_use".to_string(),
        });
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
            zone_owner: Some(owner),
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
                        game.player_mut(pending.owner).put_in_trash(pending.card);
                        game.check_turn_end();
                    }
                }) as Box<dyn FnOnce(&mut Game) + Send + Sync>
            }),
        });
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::LinkOptionHostSelection(
                LinkOptionHostSelectionState {
                    owner,
                    candidates,
                    optional,
                    outer_conts: Vec::new(),
                },
            )],
        });
    }

    pub(crate) fn run_link_option_host_selection_step(
        &mut self,
        state: LinkOptionHostSelectionState,
        action_id: u16,
        is_pass: bool,
    ) {
        use crate::action::space::{ATTACK_START, TARGETS_PER_ATTACKER};

        if is_pass {
            if state.optional {
                if let Some(pending) = self.pending_option.take() {
                    self.player_mut(pending.owner).put_in_trash(pending.card);
                    self.check_turn_end();
                }
            }
            return;
        }

        let offset = action_id.saturating_sub(ATTACK_START);
        let target_index = (offset % TARGETS_PER_ATTACKER) as u8;
        let picked = state
            .candidates
            .iter()
            .copied()
            .find(|h| h.index == target_index)
            .unwrap_or(PermanentHandle {
                player: state.owner,
                index: target_index,
            });
        self.attach_linked_card(picked);
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
            self.player_mut(pending.owner).put_in_trash(pending.card);
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
                    self.player_mut(pending.owner).put_in_trash(pending.card);
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
            // has no scheduled turn, and `ExternallyGated` hands the whole
            // window to another clause (a `kind: replacement` on the same
            // card) — all three park indefinitely, per 16-17-1's
            // "while a card with this effect is in the battle area".
            DelayTrigger::MainPhaseActivated
            | DelayTrigger::OnEvent(_)
            | DelayTrigger::ExternallyGated => u16::MAX,
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
        self.enqueue_triggered_with_context(
            crate::enums::EffectTiming::OnLeaveField,
            crate::selection::TriggerSource::EventObserved {
                player: handle.player,
                permanent: handle,
                card,
                // Leave-field observers read the cause from the snapshot
                // (`trigger.cause`), not this bit.
                effect_initiated: false,
            },
            |trigger| {
                trigger.deleted_object = Some(snapshot.clone());
                trigger.cause = Some(snapshot.cause);
                trigger.affected_player = Some(snapshot.former_controller);
                trigger.subject = Some(crate::trigger_context::EventSubject::Permanent(handle));
            },
        );
        // G-DSL-OUTER-TAIL-NESTED-PARK: maybe_drain defers when inside
        // a select-callback / outer-tail scope.
        self.maybe_drain_effect_queue();
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
    }

    /// Fire `OnDigivolutionCardTrashed` for ONE card trashed from `host`'s
    /// digivolution cards. A lone call is its own trashing event; inside an
    /// open [`Self::begin_source_move_event`] scope it joins that event.
    ///
    /// Never drains inline: "trigger-type effects can't activate during the
    /// processing for a rule or effect" (§15-8-3-2). Inside a resolving
    /// effect (a deferred-drain scope) the observers stay queued until that
    /// effect has finished, together with every sibling trigger already
    /// pending (§15-4-3-2); at top level they resolve once the event closes.
    /// DCGO `ITrashDigivolutionCards.TrashDigivolutionCards()` stacks one
    /// `OnDigivolutionCardDiscarded` for its whole list.
    /// G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS.
    pub(crate) fn fire_digivolution_card_trashed(
        &mut self,
        player: PlayerId,
        host: PermanentHandle,
        host_card: CardHandle,
        card: CardHandle,
        cause: crate::trigger_context::EventCause,
    ) {
        let event = self.begin_source_move_event();
        self.enqueue_digivolution_card_trashed(player, host, host_card, card, cause);
        self.end_source_move_event(event);
    }

    /// Open one source-move EVENT: every digivolution card trashed until the
    /// matching [`Self::end_source_move_event`] is trashed "at the same
    /// time" by one effect step (one `<Digi-Burst>` / cost payment / "trash N
    /// digivolution cards"), so each `OnDigivolutionCardTrashed` observer
    /// triggers ONCE for the whole event, however many of its cards meet the
    /// condition (§15-5-2; owner ruling 2026-10-10 #4). Separate events stay
    /// separate triggers (§15-4-2-4; ruling #5). The scope is also a
    /// deferred-drain scope, so nothing pending activates mid-event
    /// (§15-8-3-2). Scopes nest; only the outermost one coalesces.
    ///
    /// Digivolution cards RETURNED to the bottom of a deck by one step use the
    /// same scope: each `OnDigivolutionCardReturnedToDeckBottom` observer
    /// triggers once for the whole return (§15-5-2; DCGO
    /// `ReturnToLibraryBottomDigivolutionCardsClass` stacks one trigger for
    /// the returned list). The two timings coalesce separately.
    pub(crate) fn begin_source_move_event(&mut self) -> SourceMoveEvent {
        self.enter_deferred_drain();
        let outermost = !self.source_move_event_open;
        self.source_move_event_open = true;
        SourceMoveEvent { outermost }
    }

    /// Close a scope from [`Self::begin_source_move_event`]: the outermost
    /// close coalesces the event's observers (one trigger per observer, all
    /// trashed cards retained as the event's members), then the queue drains
    /// only if no effect is still resolving around the event (§15-8-3-2).
    pub(crate) fn end_source_move_event(&mut self, event: SourceMoveEvent) {
        if event.outermost {
            self.source_move_event_open = false;
            self.coalesce_source_move_event();
        }
        self.exit_deferred_drain_and_flush();
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
    }

    /// Enqueue `OnDigivolutionCardTrashed` for several digivolution cards
    /// trashed by ONE rule or effect process (a permanent leaving the field
    /// with its stack, a grouped return, ...) as ONE event (§15-5-2), without
    /// draining: the caller's process still has its own observers to enqueue
    /// (leave-field, linked-card trash) before anything activates.
    pub(crate) fn enqueue_digivolution_cards_trashed(
        &mut self,
        trashed: Vec<TrashedSource>,
        cause: crate::trigger_context::EventCause,
    ) {
        // Inside an open event the outermost close coalesces everything;
        // otherwise these cards are an event of their own.
        let outermost = !self.source_move_event_open;
        self.source_move_event_open = true;
        for t in trashed {
            self.enqueue_digivolution_card_trashed(t.player, t.host, t.host_card, t.card, cause);
        }
        if outermost {
            self.source_move_event_open = false;
            self.coalesce_source_move_event();
        }
    }

    /// Enqueue `OnTopCardTrashed` ONCE for the top card(s) one event trashed
    /// from `host`'s stack: every card of one <De-Digivolve> instance
    /// (§16-12-1 "starting with the top card"; §16-12-5 "considered to be
    /// trashed at the same time"), or <Armor Purge>'s top card (§16-19). These
    /// are top cards, not digivolution cards (§4-8-1), so they never fire
    /// `OnDigivolutionCardTrashed`. Enqueue-only: the observers wait for the
    /// causing effect to finish and are drained by the effect-queue loop. DCGO
    /// `StackSkillInfos(.., EffectTiming.WhenTopCardTrashed)` with the
    /// permanent and every trashed card. G-ENGINE-DEDIGIVOLVE-FIRES-
    /// DIGIVOLUTION-CARD-TRASHED, G-DSL-TOP-CARD-TRASHED-TIMING.
    pub(crate) fn enqueue_top_cards_trashed(
        &mut self,
        player: PlayerId,
        host: PermanentHandle,
        host_card: crate::card_source::CardHandle,
        cards: Vec<crate::card_source::CardHandle>,
        cause: crate::trigger_context::EventCause,
    ) {
        self.tick_declarative_effects();
        self.enqueue_triggered(
            crate::enums::EffectTiming::OnTopCardTrashed,
            crate::selection::TriggerSource::TopCardsTrashed {
                player,
                host,
                host_card,
                cards,
                cause,
            },
        );
    }

    pub(crate) fn enqueue_digivolution_card_trashed(
        &mut self,
        player: PlayerId,
        host: PermanentHandle,
        host_card: crate::card_source::CardHandle,
        card: crate::card_source::CardHandle,
        cause: crate::trigger_context::EventCause,
    ) {
        // A digivolution card just LEFT the stack (every source-trash path —
        // De-Digivolve, Digi-Burst / cost trashes, bottom-source strips,
        // armor purge — fires this AFTER the `card_sources` mutation).
        // Re-materialize declarative state NOW so continuous grants sourced
        // from the departed card (e.g. an inherited ＜Security A. +1＞
        // grant_keyword, a flat inherited `security_attack` aura) stop
        // applying immediately: the observers enqueued below, and any read
        // surface consulted before the next decode-boundary tick
        // (`effective_security_strike`, `has_keyword`,
        // `security_attack_keyword_bonus`), must not see the stale
        // materialized entry. Cheap when nothing changed (fingerprint memo);
        // after a real removal this is exactly one full rebuild.
        self.tick_declarative_effects();
        // Enqueueing is pure (no drain), so the new entries are exactly
        // `start..`; stamp them for the open event's coalescing.
        let start = self.effect_queue.len();
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
        if self.source_move_event_open {
            for qe in self.effect_queue.range_mut(start..) {
                if qe.timing == crate::enums::EffectTiming::OnDigivolutionCardTrashed {
                    if let Some(context) = qe.trigger_context.as_mut() {
                        context.in_source_move_event = true;
                    }
                }
            }
        }
    }

    /// Sibling of [`Self::fire_digivolution_card_trashed`], for the RETURN of a
    /// digivolution source to the BOTTOM of a player's deck. Fires
    /// `OnDigivolutionCardReturnedToDeckBottom` carrying the former host
    /// (`host` / `host_card`) and the returned `card`, one call per returned
    /// source.
    /// G-ENGINE-DIGIVOLUTION-CARD-RETURNED-TO-DECK-BOTTOM.
    /// Like the trash sibling it never drains while an effect is resolving
    /// (§15-8-3-2: its observers wait for that effect and pend with the rest
    /// of what it triggered, §15-4-3-2), and inside an open source event
    /// (`begin_source_move_event`, opened by a step returning several cards)
    /// its entries are stamped so the event's close coalesces them: one
    /// return of N cards triggers each observer once (§15-5-2).
    /// G-ENGINE-RESIDUAL-INLINE-DRAINS-OUTSIDE-SOURCE-TRASH.
    pub(crate) fn fire_digivolution_card_returned_to_deck_bottom(
        &mut self,
        player: PlayerId,
        host: PermanentHandle,
        host_card: crate::card_source::CardHandle,
        card: crate::card_source::CardHandle,
        cause: crate::trigger_context::EventCause,
    ) {
        // As in `enqueue_digivolution_card_trashed`: the source has already
        // left the stack, so refresh materialized declaratives before
        // observers (and any pre-tick read) run.
        self.tick_declarative_effects();
        // Enqueueing is pure (no drain), so the new entries are exactly
        // `start..`; stamp them for the open event's coalescing.
        let start = self.effect_queue.len();
        self.enqueue_triggered(
            crate::enums::EffectTiming::OnDigivolutionCardReturnedToDeckBottom,
            crate::selection::TriggerSource::SourceReturnedToDeckBottom {
                player,
                host,
                host_card,
                card,
                cause,
            },
        );
        if self.source_move_event_open {
            for qe in self.effect_queue.range_mut(start..) {
                if qe.timing == crate::enums::EffectTiming::OnDigivolutionCardReturnedToDeckBottom
                {
                    if let Some(context) = qe.trigger_context.as_mut() {
                        context.in_source_move_event = true;
                    }
                }
            }
        }
        self.maybe_drain_effect_queue();
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
        self.clear_would_digivolve_identity(&resume);
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

    /// Borrow the `CardSource` a `CardSourceRef` currently points at, without
    /// removing it. Sibling of `card_source_ref_snapshot` for callers that
    /// need the full card (e.g. the digivolve-route machinery in
    /// `effect_initiated_digivolve_from_source_inner`) rather than the
    /// `(handle, data_index, zone)` tuple.
    pub(crate) fn card_source_ref_peek(
        &self,
        source: crate::enums::CardSourceRef,
    ) -> Option<&crate::card_source::CardSource> {
        use crate::enums::CardSourceRef;
        match source {
            CardSourceRef::Hand(p, i) => self.player(p).hand.get(i),
            CardSourceRef::Trash(p, i) => self.player(p).trash.get(i),
            CardSourceRef::DeckTop(p) => self.player(p).deck.last(),
            CardSourceRef::Security(p, i) => self.player(p).security.get(i),
            CardSourceRef::PendingSecurity => self
                .pending_security
                .as_ref()
                .filter(|pending| !pending.played)
                .map(|pending| &pending.card),
            CardSourceRef::Material(h, i) => self
                .player(h.player)
                .battle_area
                .get(h.index as usize)
                .and_then(|perm| perm.card_sources.get(i)),
            CardSourceRef::Reveal(h) => self.revealed_cards.iter().find(|c| c.handle() == h),
            CardSourceRef::Link(h, i) => self
                .player(h.player)
                .battle_area
                .get(h.index as usize)
                .and_then(|perm| perm.linked_cards.get(i)),
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
            CardSourceRef::PendingSecurity => self
                .pending_security
                .as_ref()
                .filter(|pending| !pending.played)
                .map(|pending| {
                    (
                        pending.card.handle(),
                        pending.card.data_index,
                        Zone::Security,
                    )
                }),
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
            CardSourceRef::Link(h, i) => self
                .player(h.player)
                .battle_area
                .get(h.index as usize)
                .and_then(|perm| perm.linked_cards.get(i))
                .map(|c| (c.handle(), c.data_index, Zone::BattleArea)),
        }
    }

    pub(crate) fn place_as_bottom_source_observed(
        &mut self,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
        observer_player: PlayerId,
        face_down: bool,
    ) -> bool {
        self.place_as_source_observed(
            source,
            target,
            observer_player,
            face_down,
            false,
            None,
            EffectSourceKind::Rule,
        )
    }

    /// `place_as_bottom_source_observed` with the placing effect's source card
    /// attached, so the `OnAddDigivolutionCards` batch it opens carries the
    /// full `EffectAttribution` (controller + source card). The controller is
    /// `observer_player` on both entry points. G-ENGINE-ON-ADD-DIGIVOLUTION-CARDS.
    pub(crate) fn place_as_bottom_source_by_effect(
        &mut self,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
        observer_player: PlayerId,
        face_down: bool,
        effect_source_card: CardHandle,
        source_kind: EffectSourceKind,
    ) -> bool {
        self.place_as_source_observed(
            source,
            target,
            observer_player,
            face_down,
            false,
            Some(effect_source_card),
            source_kind,
        )
    }

    /// Top-position sibling of `place_as_bottom_source_observed`: the card is
    /// inserted as `target`'s TOP digivolution source (directly beneath the
    /// active top card — `Permanent::push_as_top_source`) instead of at the
    /// bottom of the stack. Same source zones, same security-materialization
    /// observers, same Material-carrier soft-remove lifecycle.
    /// G-DSL-PLACE-AS-TOP-SOURCE (resolved 2026-07-05).
    pub(crate) fn place_as_top_source_observed(
        &mut self,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
        observer_player: PlayerId,
        face_down: bool,
    ) -> bool {
        self.place_as_source_observed(
            source,
            target,
            observer_player,
            face_down,
            true,
            None,
            EffectSourceKind::Rule,
        )
    }

    /// Top-position sibling of `place_as_bottom_source_by_effect`.
    pub(crate) fn place_as_top_source_by_effect(
        &mut self,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
        observer_player: PlayerId,
        face_down: bool,
        effect_source_card: CardHandle,
        source_kind: EffectSourceKind,
    ) -> bool {
        self.place_as_source_observed(
            source,
            target,
            observer_player,
            face_down,
            true,
            Some(effect_source_card),
            source_kind,
        )
    }

    /// Rule 4-21-3: a token cannot receive stacked cards.
    /// Check before extracting the proposed source or opening movement events.
    pub(crate) fn is_token_source_destination(&self, target: PermanentHandle) -> bool {
        let Some(player) = self.players.get(target.player as usize) else { return false; };
        let permanent = if target.index == crate::action::space::BREEDING_TARGET as u8 {
            player.breeding_area.as_ref()
        } else {
            player.battle_area.get(target.index as usize)
        };
        permanent.is_some_and(|p| p.card_sources.last().is_some_and(|c| c.is_token))
    }

    /// Shared body for `place_as_bottom_source_observed` /
    /// `place_as_top_source_observed`. `top: false` inserts at the bottom of
    /// the digivolution stack (`Permanent::push_under`, source index 0);
    /// `top: true` inserts at the top-source position
    /// (`Permanent::push_as_top_source`, directly beneath the top card).
    /// Both fire the identical lifecycle: security sources materialize and
    /// route through `fire_effect_security_removal`, and Material-source
    /// extraction soft-removes an emptied carrier.
    ///
    /// Every successful placement here is EFFECT-driven (this body is reached
    /// only from `EffectContext` placement verbs and the public
    /// `Game::place_as_*_source` effect API — never from normal / DNA
    /// digivolution, DigiXros / Assembly materials or App Fusion), so it
    /// opens the host's `OnAddDigivolutionCards` batch window with
    /// `observer_player` as the placing effect's controller
    /// (`note_effect_added_sources`; G-ENGINE-ON-ADD-DIGIVOLUTION-CARDS).
    /// `effect_source_card` is the placing effect's carrier when the caller
    /// knows it. The Security branch notes nothing here — the card is only
    /// placed inside `complete_effect_security_removal`, which notes it.
    fn place_as_source_observed(
        &mut self,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
        observer_player: PlayerId,
        face_down: bool,
        top: bool,
        effect_source_card: Option<CardHandle>,
        source_kind: EffectSourceKind,
    ) -> bool {
        if self.is_token_source_destination(target) { return false; }
        if let crate::enums::CardSourceRef::Material(donor, index) = source {
            if self
                .player(donor.player)
                .battle_area
                .get(donor.index as usize)
                .is_some_and(|p| index + 1 == p.card_sources.len())
            {
                return self.begin_field_top_source_move(
                    donor,
                    target,
                    observer_player,
                    face_down,
                    top,
                    effect_source_card,
                    source_kind,
                );
            }
        }
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
            let destination = if top {
                crate::selection::SecurityRemovalDestination::TopSource(target)
            } else {
                crate::selection::SecurityRemovalDestination::BottomSource(target)
            };
            // The placing effect's carrier, read back by
            // `complete_effect_security_removal` when it seats the card.
            self.pending_security_source_cause_card = effect_source_card;
            self.pending_security_source_face_down = face_down;
            self.fire_effect_security_removal(
                defender,
                observer_player,
                observer_player,
                cause,
                card,
                destination,
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
            let placed = card.handle();
            if top {
                breeding.push_as_top_source(card);
            } else {
                breeding.push_under(card);
            }
            // Soft-remove the carrier slot if Material extraction emptied it.
            // Target is in breeding (not battle_area), so no shift needed.
            // Sibling of the digivolve-from-material fix landed in PR #533.
            // See `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` in
            // `qa/archetype-qa/engine-gaps.md`.
            if let crate::enums::CardSourceRef::Material(carrier, _) = source {
                let _ = self.soft_remove_if_emptied(carrier);
            }
            self.note_effect_added_sources(
                target,
                vec![placed],
                crate::trigger_context::EffectAttribution {
                    controller: observer_player,
                    source_card: effect_source_card,
                    source_permanent: None,
                },
            );
            return true;
        }

        let target_player = self.player_mut(target.player);
        if (target.index as usize) >= target_player.battle_area.len() {
            let _ = self.restore_card_source_ref(source, taken);
            return false;
        }
        let mut card = taken.card;
        card.face_down = face_down;
        let placed = card.handle();
        if top {
            target_player.battle_area[target.index as usize].push_as_top_source(card);
        } else {
            target_player.battle_area[target.index as usize].push_under(card);
        }
        // Note the batch BEFORE the carrier soft-remove below: the note keys
        // the host by its stable top card, but `target` is only guaranteed
        // positionally valid until that soft-remove shifts indices.
        self.note_effect_added_sources(
            target,
            vec![placed],
            crate::trigger_context::EffectAttribution {
                controller: observer_player,
                source_card: effect_source_card,
                source_permanent: None,
            },
        );
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

    pub(crate) fn effect_source_kind_for_handle(
        &self,
        handle: crate::card_source::CardHandle,
    ) -> EffectSourceKind {
        // Only a physical current top can contribute a native field effect.
        // Inherited/link origins are classified separately as Digimon effects.
        for (player, state) in self.players.iter().enumerate() {
            for (index, permanent) in state.battle_area.iter().enumerate() {
                if permanent.card_sources.last().map(|c| c.handle()) != Some(handle) {
                    if permanent.card_sources.iter().any(|c| c.handle() == handle)
                        || permanent.linked_cards.iter().any(|c| c.handle() == handle) {
                        // Inherited and link effects are Digimon effects, even
                        // when their physical source is a printed Tamer.
                        return EffectSourceKind::Digimon;
                    }
                    continue;
                }
                let h = PermanentHandle { player: player as PlayerId, index: index as u8 };
                let id = permanent.synth_identity(&self.card_data, &self.modifiers, h);
                if (id.kind == CardKind::Digimon && id.retained_kind == Some(CardKind::Tamer))
                    || (id.kind == CardKind::Tamer && id.retained_kind == Some(CardKind::Digimon)) {
                    return EffectSourceKind::DigimonAndTamer;
                }
                break;
            }
        }
        // A deleted source's `[On Deletion]` reads its category from the
        // deletion snapshot (rule 25): a token has left the game by then
        // (§4-21-5) and is in no zone, yet its effect is a Digimon effect
        // (§15-2-1-1). G-ENGINE-DEPARTED-TOKEN-CONTEXT-FALLS-BACK-TO-RULE.
        if let Some(snap) = self
            .current_trigger_context
            .as_ref()
            .and_then(|t| t.deleted_object.as_ref())
        {
            if snap.top_card == handle {
                return source_kind_for_card_kind(snap.card_kind);
            }
        }
        // `card_data_for_handle` also resolves the in-flight transients (an
        // Option between pay-cost and dispose, the security card being
        // checked) that the zone scan misses.
        self.card_data_for_handle(handle)
            .map(|data| source_kind_for_card_kind(data.card_kind))
            .unwrap_or(EffectSourceKind::Rule)
    }

    /// The per-turn OPT accounting slot for a cost reducer. When the effect
    /// carries a `shared_opt_group` (the two `scope: both` copies, FaceUp +
    /// Inherited), both copies map to that single group id so they share ONE
    /// once-per-turn counter — a card can't reduce a cost once as an active
    /// top and again the same turn as a digivolution source. Otherwise the raw
    /// `effect_slot` (vec index) is the key, unchanged.
    /// G-ENGINE-SHARED-OPT-SCOPE-BOTH-REDUCER (mirrors the triggered-queue
    /// `Game::opt_slot_key`).
    pub(crate) fn cost_reducer_opt_slot(&self, key: &CostReductionKey) -> u8 {
        // `shared_opt_group` is `Copy` (`Option<u8>`), so extract it before the
        // (possibly owned) effects collection is dropped.
        let group = self
            .effects_for_card(&key.card_id, key.source_card)
            .and_then(|effects| {
                effects
                    .get(key.effect_slot as usize)
                    .and_then(|effect| effect.shared_opt_group)
            });
        let slot = group.unwrap_or(key.effect_slot);
        // A `<Succession>` copy keeps its own counter (G-ENGINE-SUCCESSION-KEYWORD).
        match key.source_permanent {
            Some(perm) if perm.index != crate::action::space::BREEDING_TARGET as u8 => {
                self.opt_key_for_source(perm, key.source_card, key.is_under, false, slot)
            }
            _ => slot,
        }
    }

    fn cost_reducer_activation_count(&self, key: &CostReductionKey) -> u8 {
        let Some(source) = key.source_permanent else {
            return 0;
        };
        let opt_slot = self.cost_reducer_opt_slot(key);
        if source.index == crate::action::space::BREEDING_TARGET as u8 {
            return self
                .player(source.player)
                .breeding_area
                .as_ref()
                .map(|perm| perm.activation_count(key.source_card, opt_slot))
                .unwrap_or(0);
        }
        self.player(source.player)
            .battle_area
            .get(source.index as usize)
            .map(|perm| perm.activation_count(key.source_card, opt_slot))
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
                // `<Succession>`: an adopted source's top-scope reducers too.
                for source_idx in self.succession_source_indices(perm_handle) {
                    let source =
                        &self.player(player_id).battle_area[perm_idx].card_sources[source_idx];
                    self.push_cost_source_info(
                        &mut infos,
                        Some(perm_handle),
                        source,
                        false,
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
    /// Per general_rule.pdf 3-4-7-4 ("Effects on cards can't trigger or
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
                // `<Succession>`: an adopted source's top-scope reducers too.
                for source_idx in self.succession_source_indices(perm_handle) {
                    let source =
                        &self.player(player_id).battle_area[perm_idx].card_sources[source_idx];
                    self.push_observer_source_info(
                        &mut infos,
                        Some(perm_handle),
                        source,
                        false,
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
    /// `push_breeding_cost_sources` (general_rule.pdf 3-4-7-4).
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

    /// OPT key for a `BeforePayCostObserve` source (a `<Succession>` copy
    /// keeps its own counter).
    pub(crate) fn observer_opt_slot(&self, info: &BeforePayCostSourceInfo) -> u8 {
        match info.source_permanent {
            Some(perm) if perm.index != crate::action::space::BREEDING_TARGET as u8 => self
                .opt_key_for_source(
                    perm,
                    info.source_card,
                    info.is_under,
                    false,
                    info.effect_slot,
                ),
            _ => info.effect_slot,
        }
    }

    fn observer_activation_count(&self, info: &BeforePayCostSourceInfo) -> u8 {
        let Some(source) = info.source_permanent else {
            return 0;
        };
        let opt_slot = self.observer_opt_slot(info);
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
            .map(|perm| perm.activation_count(info.source_card, opt_slot))
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
        // The `remove()` above shifted every higher-indexed permanent down;
        // re-key the modifier registry and the in-flight attack's handles
        // (EX8-028 pays a battle-area Digimon into security MID-ATTACK —
        // without these, higher-indexed permanents' modifiers migrate to the
        // wrong permanent and the attacker's handle goes stale, fizzling the
        // attack before its security check).
        self.modifiers
            .shift_after_battle_area_remove(target.player, target.index);
        self.shift_pending_attack_after_battle_area_remove(target.player, target.index);
        self.shift_effect_queue_after_battle_area_remove(target.player, target.index);

        // Sources-below-top → each source's owner's trash, all at the same
        // time: ONE `OnDigivolutionCardTrashed` event (§15-5-2). Nothing
        // activates before this placement has finished (§15-8-3-2): the
        // queue is drained (if no effect is resolving) after the top card
        // reaches security.
        let had_sources = !permanent.card_sources.is_empty();
        for source in permanent.card_sources.drain(..) {
            let owner = source.owner;
            self.player_mut(owner).put_in_trash(source);
        }
        if had_sources {
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    EffectTiming::OnDigivolutionCardTrashed,
                    TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
                );
            }
        }

        // Linked cards → controller's trash; fire OnLinkedCardTrashed once
        // if any were present. Mirrors `attach_tamer_to_digimon`.
        let had_linked = !permanent.linked_cards.is_empty();
        for linked in permanent.linked_cards.drain(..) {
            let owner = linked.owner;
            self.player_mut(owner).put_in_trash(linked);
        }
        if had_linked {
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    EffectTiming::OnLinkedCardTrashed,
                    TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
                );
            }
        }

        // Place top card in security at the requested position.
        let face_up_key = card.card_index;
        match position {
            crate::enums::StackPosition::Top => {
                self.player_mut(player_id).put_on_security_top(card);
            }
            crate::enums::StackPosition::Bottom => {
                self.player_mut(player_id).insert_in_security(0, card);
            }
            crate::enums::StackPosition::Random => {
                use rand::Rng;
                let sec_len = self.player(player_id).security.len();
                let idx = if sec_len == 0 {
                    0
                } else {
                    self.rng.gen_range(0..=sec_len)
                };
                self.player_mut(player_id).insert_in_security(idx, card);
            }
        }
        if face_up {
            self.player_mut(player_id)
                .face_up_security
                .insert(face_up_key);
        }
        // The placement has finished: the trash observers enqueued above may
        // activate now, unless an effect is still resolving (§15-8-3-2).
        self.maybe_drain_effect_queue();
        true
    }

    /// Place a battle-area Digimon with no digivolution cards at the bottom of
    /// `player_id`'s security stack (BT24-040 Venusmon's leave-replacement
    /// cost; EX8-028). Routed through the resumable
    /// [`Self::place_permanent_on_security`]: the chosen Digimon's own
    /// would-leave / would-place replacements (e.g. its controller's <Guard>,
    /// §16-45) may park a prompt; the move is remembered in
    /// `pending_security_moves` and completes when that prompt resolves
    /// without replacing it (`resume_security_move`). Returns `true` only when
    /// the placement completed synchronously; a parked placement returns
    /// `false` with `pending_selection` set. The leave cause is keyed on the
    /// leaving Digimon's controller (`run_permanent_on_security`), not on the
    /// destination stack.
    ///
    /// Gates: the acting player's AND the destination player's
    /// `CannotAddSecurityByEffect` (DCGO `IPutSecurityPermanent` checks
    /// `EffectSourceCard.Owner.CanAddSecurity` and
    /// `TopCard.Owner.CanAddSecurity`).
    pub(crate) fn place_sourceless_permanent_on_security_bottom(
        &mut self,
        player_id: PlayerId,
        target: PermanentHandle,
        observer_player: PlayerId,
    ) -> bool {
        if [observer_player, player_id].into_iter().any(|p| {
            self.modifiers
                .player_has(p, ModifierType::CannotAddSecurityByEffect)
        }) {
            return false;
        }
        let sourceless = self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .is_some_and(|p| p.card_sources.len() == 1);
        if !sourceless {
            return false;
        }
        self.place_permanent_on_security(
            player_id,
            target,
            crate::enums::StackPosition::Bottom,
            false,
            observer_player,
        )
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

    fn remember_security_move(
        &mut self,
        target: PermanentHandle,
        player: PlayerId,
        position: crate::enums::StackPosition,
        face_up: bool,
        observer: PlayerId,
        leave_complete: bool,
    ) -> Option<(
        PlayerId,
        u64,
        PlayerId,
        crate::enums::StackPosition,
        bool,
        PlayerId,
    )> {
        let id = self.capture_source_play_host_identity(target)?;
        let donor = (target.player, id);
        let key = (target.player, id, player, position, face_up, observer);
        if let Some(state) = self.pending_security_moves.iter_mut().rfind(|s| {
            !s.top_only
                && s.donor == donor
                && s.player == player
                && s.position == position
                && s.face_up == face_up
                && s.observer == observer
        }) {
            state.leave_complete |= leave_complete;
        } else {
            let card = self
                .player(target.player)
                .battle_area
                .get(target.index as usize)?
                .top_card()
                .handle();
            self.pending_security_moves
                .push(crate::game::PendingSecurityMove {
                    top_only: false,
                    donor,
                    card,
                    player,
                    position,
                    face_up,
                    observer,
                    leave_complete,
                });
        }
        Some(key)
    }
    fn finish_security_move_if_settled(
        &mut self,
        key: (
            PlayerId,
            u64,
            PlayerId,
            crate::enums::StackPosition,
            bool,
            PlayerId,
        ),
    ) {
        if self.pending_selection.is_none()
            || self.resolve_digivolve_host_identity(key.0, key.1).is_none()
        {
            self.pending_security_moves.retain(|s| {
                !(!s.top_only
                    && s.donor == (key.0, key.1)
                    && s.player == key.2
                    && s.position == key.3
                    && s.face_up == key.4
                    && s.observer == key.5)
            });
        }
    }
    pub(crate) fn place_permanent_on_security(
        &mut self,
        player: PlayerId,
        target: PermanentHandle,
        position: crate::enums::StackPosition,
        face_up: bool,
        observer: PlayerId,
    ) -> bool {
        let Some(key) =
            self.remember_security_move(target, player, position, face_up, observer, false)
        else {
            return false;
        };
        let result = self.run_permanent_on_security(player, target, position, face_up, observer);
        self.finish_security_move_if_settled(key);
        result
    }
    pub(crate) fn place_permanent_on_security_without_leave_replacement(
        &mut self,
        player: PlayerId,
        target: PermanentHandle,
        position: crate::enums::StackPosition,
        face_up: bool,
        observer: PlayerId,
    ) -> bool {
        let Some(key) =
            self.remember_security_move(target, player, position, face_up, observer, true)
        else {
            return false;
        };
        let result = self.run_permanent_on_security_without_leave_replacement(
            player, target, position, face_up, observer,
        );
        self.finish_security_move_if_settled(key);
        result
    }
    pub(crate) fn resume_security_move(
        &mut self,
        subject: crate::replacement::ReplacementSubject,
        cause: crate::replacement::ReplacementCause,
        outcome: crate::replacement::ReplacementOutcome,
    ) -> bool {
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};
        let key = match subject {
            ReplacementSubject::Permanent(h) => self
                .player(h.player)
                .battle_area
                .get(h.index as usize)
                .and_then(|p| p.battle_identity)
                .map(|id| (h.player, id)),
            ReplacementSubject::Card(card, _) => self
                .pending_security_moves
                .iter()
                .rfind(|s| {
                    s.card == card
                        || self
                            .resolve_digivolve_host_identity(s.donor.0, s.donor.1)
                            .and_then(|h| {
                                self.player(h.player)
                                    .battle_area
                                    .get(h.index as usize)
                                    .map(|p| p.top_card().handle())
                            })
                            == Some(card)
                })
                .map(|s| s.donor),
            _ => None,
        };
        let Some(key) = key else { return false };
        let Some(i) = self
            .pending_security_moves
            .iter()
            .rposition(|s| s.donor == key)
        else {
            return false;
        };
        let state = self.pending_security_moves.remove(i);
        let Some(h) = self.resolve_digivolve_host_identity(key.0, key.1) else {
            return true;
        };
        if state.top_only {
            if outcome == ReplacementOutcome::None {
                let still_exact_top = self
                    .player(h.player)
                    .battle_area
                    .get(h.index as usize)
                    .is_some_and(|p| {
                        p.card_sources.len() >= 2 && p.top_card().handle() == state.card
                    });
                if still_exact_top {
                    // Re-enter the same WouldPlace window so a declined or
                    // non-replacing optional effect cannot skip remaining candidates.
                    self.place_top_stacked_on_security(
                        state.player,
                        h,
                        state.position,
                        state.face_up,
                        state.observer,
                    );
                }
            } else {
                self.finish_top_stacked_security_move(state, outcome);
            }
            return true;
        }
        match outcome {
            ReplacementOutcome::None => {
                if state.leave_complete {
                    self.place_permanent_on_security_without_leave_replacement(
                        state.player,
                        h,
                        state.position,
                        state.face_up,
                        state.observer,
                    );
                } else {
                    self.place_permanent_on_security(
                        state.player,
                        h,
                        state.position,
                        state.face_up,
                        state.observer,
                    );
                }
            }
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {}
            ReplacementOutcome::Redirected(Zone::Hand) => {
                self.return_to_hand(h);
            }
            ReplacementOutcome::Redirected(Zone::Deck) => {
                self.return_to_deck(h, crate::enums::StackPosition::Bottom);
            }
            ReplacementOutcome::Redirected(Zone::Trash) => {
                self.delete_permanent_with_cause(h, cause);
            }
            ReplacementOutcome::Redirected(Zone::Security) => {
                self.place_permanent_on_security_without_leave_replacement(
                    state.player,
                    h,
                    state.position,
                    state.face_up,
                    state.observer,
                );
            }
            ReplacementOutcome::Substituted(ReplacementSubject::Permanent(other)) => {
                self.place_permanent_on_security(
                    state.player,
                    other,
                    state.position,
                    state.face_up,
                    state.observer,
                );
            }
            _ => {}
        }
        true
    }

    fn run_permanent_on_security(
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

    fn run_permanent_on_security_without_leave_replacement(
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

        // Capture event identity before removal; source disposal is a rule consequence.
        let permanent = &self.player(target.player).battle_area[target.index as usize];
        let data = self
            .card_data_for_handle(source_card)
            .expect("validated card data");
        let source_handles = permanent
            .card_sources
            .iter()
            .take(permanent.card_sources.len().saturating_sub(1))
            .map(|c| c.handle())
            .collect::<Vec<_>>();
        let dp = self.effective_dp(target);
        let identity = permanent.synth_identity(&self.card_data, &self.modifiers, target);
        let snapshot = crate::trigger_context::DeletedObjectSnapshot {
            former_controller: target.player,
            top_card: source_card,
            card_kind: identity.kind,
            retained_kind: identity.retained_kind,
            traits: identity.traits.clone(),
            level: identity.level,
            dp,
            cause: crate::trigger_context::EventCause::SecurityPlacement,
            dp_just_before: dp,
            level_just_before: identity.level,
            cost_just_before: Some(data.play_cost),
            names_just_before: identity.card_names.clone(),
            traits_just_before: identity.traits.clone(),
            source_count_just_before: source_handles.len(),
            digisources_just_before: source_handles.clone(),
            is_token: permanent.top_card().is_token,
        };
        let mut permanent = self
            .player_mut(target.player)
            .battle_area
            .remove(target.index as usize);
        self.clear_permanent_full(target);
        self.modifiers.expire_player_on_permanent_leave(target);
        self.modifiers
            .shift_after_battle_area_remove(target.player, target.index);
        self.shift_pending_attack_after_battle_area_remove(target.player, target.index);
        self.shift_effect_queue_after_battle_area_remove(target.player, target.index);
        let top = permanent
            .card_sources
            .pop()
            .expect("validated permanent top");
        let top_handle = top.handle();
        let face_up_key = top.card_index;
        let egg = top.card_kind(&self.card_data) == crate::enums::CardKind::DigiEgg;
        let token = top.is_token;
        let mut leaving_sources = permanent.card_sources.clone();
        leaving_sources.extend(permanent.linked_cards.iter().cloned());
        leaving_sources.push(top.clone());
        if egg {
            // 3-1-3-9-2: a Digi-Egg entering security goes to its owner's egg deck bottom.
            self.player_mut(top.owner).insert_in_digi_egg_deck(0, top);
        } else if !token {
            match position {
                crate::enums::StackPosition::Top => self.player_mut(player_id).put_on_security_top(top),
                crate::enums::StackPosition::Bottom => {
                    self.player_mut(player_id).insert_in_security(0, top)
                }
                crate::enums::StackPosition::Random => {
                    use rand::Rng;
                    let len = self.player(player_id).security.len();
                    let index = self.rng.gen_range(0..=len);
                    self.player_mut(player_id).insert_in_security(index, top);
                }
            }
            if face_up {
                self.player_mut(player_id)
                    .face_up_security
                    .insert(face_up_key);
            }
        }
        for card in permanent.card_sources {
            self.player_mut(card.owner).put_in_trash(card);
        }
        let had_linked = !permanent.linked_cards.is_empty();
        for card in permanent.linked_cards {
            self.player_mut(card.owner).put_in_trash(card);
        }
        // No observer runs before every card reaches its destination and handles are shifted.
        self.apply_ace_overflow_for_sources(&leaving_sources);
        self.tick_declarative_effects();
        // The stack's digivolution cards are trashed at the same time: one
        // event (§15-5-2).
        self.enqueue_digivolution_cards_trashed(
            source_handles
                .into_iter()
                .map(|card| TrashedSource {
                    player: target.player,
                    host: target,
                    host_card: top_handle,
                    card,
                })
                .collect(),
            crate::trigger_context::EventCause::Rule,
        );
        if had_linked {
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    EffectTiming::OnLinkedCardTrashed,
                    TriggerSource::PlayerBattleArea(pid as PlayerId),
                );
            }
        }
        self.fire_on_leave_field(target, snapshot);
        // The placement was processed — into security, or under Digi-Egg /
        // token rules — which meets a "by placing it" condition (§15-1-9).
        if let Some(receipt) = self
            .security_placement_receipts
            .iter_mut()
            .rfind(|(card, _)| *card == top_handle.0)
        {
            receipt.1 = true;
        }
        if !egg && !token {
            self.fire_on_place_security(player_id, observer_player, top_handle);
        }
        !egg && !token
    }

    /// Explicit printed top-card extraction (BT20-055 Q4388/Q4723).
    /// A separate entry point preserves the semantic distinction from a
    /// field Digimon selected through generic CardSourceRef::Material.
    pub(crate) fn place_top_stacked_on_security(
        &mut self,
        player: PlayerId,
        target: PermanentHandle,
        position: crate::enums::StackPosition,
        face_up: bool,
        observer: PlayerId,
    ) -> bool {
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};
        if self
            .modifiers
            .player_has(observer, ModifierType::CannotAddSecurityByEffect)
            || self
                .modifiers
                .player_has(player, ModifierType::CannotAddSecurityByEffect)
            || self
                .player(observer)
                .battle_area
                .iter()
                .enumerate()
                .any(|(i, _)| {
                    self.modifiers.has(
                        PermanentHandle {
                            player: observer,
                            index: i as u8,
                        },
                        ModifierType::CannotAddSecurity,
                    )
                })
        {
            return false;
        }
        let Some(id) = self.capture_source_play_host_identity(target) else {
            return false;
        };
        let Some(permanent) = self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
        else {
            return false;
        };
        if permanent.card_sources.len() < 2 {
            return false;
        }
        let card = permanent.top_card().handle();
        let state = crate::game::PendingSecurityMove {
            top_only: true,
            donor: (target.player, id),
            card,
            player,
            position,
            face_up,
            observer,
            leave_complete: true,
        };
        // Remember before opening replacement so nested requests and clones
        // retain exact physical donor/top card and destination metadata.
        self.pending_security_moves.push(state.clone());
        let cause = self.infer_effect_cause(target.player);
        let outcome = self.try_replace(
            EffectTiming::WhenWouldPlaceInSecurity,
            ReplacementSubject::Card(card, Zone::BattleArea),
            cause,
            Some(Zone::Security),
        );
        if self.pending_selection.is_some() {
            return false;
        }
        if let Some(i) = self.pending_security_moves.iter().rposition(|s| {
            s.top_only
                && s.donor == state.donor
                && s.card == card
                && s.player == player
                && s.position == position
                && s.face_up == face_up
                && s.observer == observer
        }) {
            self.pending_security_moves.remove(i);
        }
        self.finish_top_stacked_security_move(state, outcome)
    }

    fn finish_top_stacked_security_move(
        &mut self,
        state: crate::game::PendingSecurityMove,
        outcome: crate::replacement::ReplacementOutcome,
    ) -> bool {
        use crate::replacement::ReplacementOutcome;
        if matches!(
            outcome,
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled
        ) {
            return false;
        }
        let Some(h) = self.resolve_digivolve_host_identity(state.donor.0, state.donor.1) else {
            return false;
        };
        let p = &self.player(h.player).battle_area[h.index as usize];
        // A nested operation cannot redirect this request onto a fresh top.
        if p.card_sources.len() < 2 || p.top_card().handle() != state.card {
            return false;
        }
        let destination = match outcome {
            ReplacementOutcome::None => Zone::Security,
            ReplacementOutcome::Redirected(z) => z,
            _ => return false,
        };
        if !matches!(
            destination,
            Zone::Security | Zone::Trash | Zone::Hand | Zone::Deck
        ) {
            return false;
        }
        let card = self.player_mut(h.player).battle_area[h.index as usize]
            .card_sources
            .pop()
            .unwrap();
        let leaving = vec![card.clone()];
        let is_egg = card.card_kind(&self.card_data) == crate::enums::CardKind::DigiEgg;
        let owner = card.owner;
        let security = destination == Zone::Security && !is_egg;
        match destination {
            Zone::Security if is_egg => self.player_mut(owner).insert_in_digi_egg_deck(0, card),
            Zone::Security => {
                match state.position {
                    crate::enums::StackPosition::Top => {
                        self.player_mut(state.player).put_on_security_top(card)
                    }
                    crate::enums::StackPosition::Bottom => {
                        self.player_mut(state.player).insert_in_security(0, card)
                    }
                    crate::enums::StackPosition::Random => {
                        use rand::Rng;
                        let len = self.player(state.player).security.len();
                        let i = self.rng.gen_range(0..=len);
                        self.player_mut(state.player).insert_in_security(i, card);
                    }
                }
                if state.face_up {
                    self.player_mut(state.player)
                        .face_up_security
                        .insert(state.card.0);
                }
            }
            Zone::Trash => self.player_mut(owner).put_in_trash(card),
            Zone::Hand if is_egg => self.player_mut(owner).insert_in_digi_egg_deck(0, card),
            Zone::Hand => self.player_mut(owner).put_in_hand(card),
            Zone::Deck if is_egg => self.player_mut(owner).insert_in_digi_egg_deck(0, card),
            Zone::Deck => self.player_mut(owner).insert_in_deck(0, card),
            _ => unreachable!(),
        }
        // Memory mutation (including Overflow) reevaluates UntilCondition.
        // Publish the changed top/zone and remove its old aura first, so
        // those predicates cannot observe a stale departed-top modifier.
        self.tick_declarative_effects();
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
        self.apply_ace_overflow_for_sources(&leaving);
        if security {
            self.fire_on_place_security(state.player, state.observer, state.card);
        }
        security
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

        if let crate::enums::CardSourceRef::Material(carrier, index) = source {
            let is_top = self
                .player(carrier.player)
                .battle_area
                .get(carrier.index as usize)
                .is_some_and(|p| index + 1 == p.card_sources.len());
            if is_top {
                return self.place_permanent_on_security(
                    player_id,
                    carrier,
                    position,
                    face_up,
                    observer_player,
                );
            }
        }

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
                self.player_mut(owner).put_in_trash(taken);
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
        if taken.card_kind(&self.card_data) == crate::enums::CardKind::DigiEgg {
            self.player_mut(taken.owner).insert_in_digi_egg_deck(0, taken);
            return false;
        }

        // face_up_security is HashSet<u16> keyed by card_index.
        let face_up_key = taken.card_index;

        match position {
            crate::enums::StackPosition::Top => {
                self.player_mut(player_id).put_on_security_top(taken);
            }
            crate::enums::StackPosition::Bottom => {
                self.player_mut(player_id).insert_in_security(0, taken);
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
                self.player_mut(player_id).insert_in_security(idx, taken);
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
#[derive(Clone, Copy, Debug)]
pub struct AssemblyPlayParams {
    pub(crate) effect_controller: Option<PlayerId>,
    pub(crate) cost_delta: crate::enums::CostDelta,
    pub(crate) source: PlaySource,
    pub(crate) origin: PendingWouldPlayOrigin,
    pub(crate) suppress_on_play: bool,
    /// Generic cost reduction accumulated by the BeforePayCost chain.
    pub(crate) total_reduction: crate::game_actions::PlayCostAdjustments,
    /// The assembly-specific reduction (D5 — `cost:` on the alt-path).
    pub(crate) assembly_reduction: i32,
}

/// Recursive system-of-distinct-representatives check, extended with
/// per-element distinctness keys (see `assembly_assign_keyed`). Tiny inputs
/// (≤ a few slots, trash bounded by deck size) keep the backtracking cheap.
///
/// Distinctness key of a trash card under an Assembly element's `distinct_by`
/// mode (`None` key ⇒ the card carries no value for the mode, e.g. a
/// level-less card under `level`, and never collides).
#[cfg(feature = "dsl-yaml-loader")]
fn assembly_distinct_key(
    mode: digimon_dsl::compiled::CompiledDistinctBy,
    data: &crate::card_data::CardData,
) -> Option<String> {
    use digimon_dsl::compiled::CompiledDistinctBy as D;
    match mode {
        D::Name => Some(data.card_name.clone()),
        D::CardNumber => Some(data.card_id.clone()),
        D::Level => data.level.map(|l| l.to_string()),
        // Set-level — handled by `AssemblyDistinct::Colors`, never keyed.
        D::Color => None,
    }
}

/// An Assembly element's distinctness constraint, pre-resolved per trash index.
#[cfg(feature = "dsl-yaml-loader")]
enum AssemblyDistinct {
    /// No constraint beyond handle-distinctness.
    None,
    /// Pairwise key equality (`name` / `card_number` / `level`); a `None` key
    /// never collides.
    Keys(Vec<Option<String>>),
    /// "w/different colors": per trash index the printed-color bitmask; the
    /// element's chosen cards must be perfectly color-assignable.
    Colors(Vec<u8>),
}

/// `assembly_assign` with per-element distinctness: a slot whose element has
/// a `distinct_by` key may not take a card whose key matches a card already
/// chosen for ANOTHER slot of the same element (G-ASSEMBLY-NO-DISTINCT-BY);
/// a `Color` element's chosen cards must stay perfectly color-assignable
/// (G-ASSEMBLY-DISTINCT-BY-COLOR).
///
/// Slots of one element are interchangeable (same filter, symmetric
/// constraint), so a slot only takes trash indices ABOVE the previous
/// same-element slot's pick — combinations, not permutations.
#[cfg(feature = "dsl-yaml-loader")]
fn assembly_assign_keyed(
    match_table: &[Vec<usize>],
    slot_element: &[usize],
    element_distinct: &[AssemblyDistinct],
    slot: usize,
    used: &mut [bool],
    chosen: &mut [Option<usize>],
) -> bool {
    if slot >= match_table.len() {
        return true;
    }
    let element = slot_element[slot];
    let same_element_prev: Vec<usize> = (0..slot)
        .filter(|&prev| slot_element[prev] == element)
        .filter_map(|prev| chosen[prev])
        .collect();
    let floor = same_element_prev.last().copied();
    for &t in &match_table[slot] {
        if used[t] || floor.is_some_and(|f| t <= f) {
            continue;
        }
        match &element_distinct[element] {
            AssemblyDistinct::None => {}
            AssemblyDistinct::Keys(keys) => {
                if let Some(key) = &keys[t] {
                    if same_element_prev
                        .iter()
                        .any(|&c| keys[c].as_ref() == Some(key))
                    {
                        continue;
                    }
                }
            }
            AssemblyDistinct::Colors(masks) => {
                let mut set: Vec<u8> = same_element_prev.iter().map(|&c| masks[c]).collect();
                set.push(masks[t]);
                if !crate::effect_context::selections::colors_assignable(&set) {
                    continue;
                }
            }
        }
        used[t] = true;
        chosen[slot] = Some(t);
        if assembly_assign_keyed(
            match_table,
            slot_element,
            element_distinct,
            slot + 1,
            used,
            chosen,
        ) {
            used[t] = false;
            chosen[slot] = None;
            return true;
        }
        used[t] = false;
        chosen[slot] = None;
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
    let distinct_by = assembly_element_distinct_mode(game, target_card, element_idx);

    // Distinctness across elements: exclude handles already picked.
    let exclude = picked_so_far.clone();
    let filter = move |g: &Game, cs: &CardSource| -> bool {
        if exclude.contains(&cs.handle()) {
            return false;
        }
        let rctx = EffectReadContext::new(g, source_card, None, player);
        eval_predicate(&filter_pred, &rctx, PredicateSubject::Card(cs.handle()))
    };

    // Every element requires its exact printed count. The first element may
    // decline at zero via is_optional_zero below; after a pick it cannot end
    // early (Assembly rules 7-3-2-4/5, optional zero 7-3-2-7/9).
    let min = count;
    let max = count;
    let prompt = format!(
        "Assembly: select {} card(s) from trash to place under (No Selection to skip).",
        count
    );

    let elements_for_cb = std::sync::Arc::clone(&elements);
    let picked_so_far_for_resume = picked_so_far.clone();
    let mut ctx = EffectContext::new(game, source_card, None, player);
    let prov = crate::resume::ResumeProvenance {
        source_card: ctx.source_card,
        source_permanent: ctx.source_permanent,
        source_kind: ctx.source_kind,
        controller: ctx.player,
        override_pin: ctx.override_selecting_player(),
    };
    ctx.select_count_capped_multi_min(
        player,
        CountCappedZone::Trash,
        min,
        max,
        &prompt,
        is_first, // is_optional_zero: only the gate (element 0) may finish at 0 picks
        distinct_by,
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
    if let Some(pending) = ctx.game.pending_selection.as_ref() {
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::NonDslCountCappedStep(
                crate::resume::NonDslCountCappedState {
                    prov,
                    of_player: player,
                    zone: CountCappedZone::Trash,
                    min,
                    max,
                    is_optional_zero: is_first,
                    distinct_by,
                    candidate_actions: pending.valid_action_ids.clone(),
                    accum: Vec::new(),
                    prompt: pending.prompt.clone(),
                    previous_phase: pending.previous_phase,
                    terminal: crate::resume::NonDslCountCappedTerminal::Assembly {
                        player,
                        target_card,
                        params,
                        elements,
                        element_idx,
                        picked_so_far: picked_so_far_for_resume,
                    },
                    outer_conts: Vec::new(),
                },
            )],
        });
    }
}

/// The `distinct_by` mode of the hand card's Assembly element `element_idx`,
/// read back from the alt-path registry (so the per-element tuple type that
/// threads through the resume frames stays unchanged).
/// G-ASSEMBLY-NO-DISTINCT-BY.
#[cfg(feature = "dsl-yaml-loader")]
fn assembly_element_distinct_mode(
    game: &Game,
    target_card: CardHandle,
    element_idx: usize,
) -> Option<crate::effect_context::DistinctByMode> {
    use crate::effect_context::DistinctByMode;
    use digimon_dsl::compiled::{CompiledAltPathKind, CompiledDistinctBy as D};
    let card_id = game.card_data_for_handle(target_card)?.card_id.clone();
    let path = game
        .alt_path_registry
        .get(&card_id)?
        .iter()
        .find(|p| matches!(p.kind, CompiledAltPathKind::Assembly))?;
    path.materials
        .get(element_idx)?
        .distinct_by
        .map(|d| match d {
            D::CardNumber => DistinctByMode::CardNumber,
            D::Level => DistinctByMode::Level,
            D::Name => DistinctByMode::Name,
            D::Color => DistinctByMode::Color,
        })
}

#[cfg(feature = "dsl-yaml-loader")]
pub(crate) fn continue_assembly_after_material_picks(
    game: &mut Game,
    player: PlayerId,
    target_card: CardHandle,
    params: AssemblyPlayParams,
    elements: std::sync::Arc<Vec<(digimon_dsl::compiled::CompiledPredicate, u8)>>,
    element_idx: usize,
    picked_so_far: Vec<CardHandle>,
    picks: Vec<CardHandle>,
) {
    let is_first = element_idx == 0;
    let is_last = element_idx + 1 >= elements.len();
    if is_first && picks.is_empty() {
        let _ = assembly_finish(game, player, target_card, params, params.total_reduction);
        return;
    }

    let mut all_picked = picked_so_far;
    all_picked.extend(picks);
    if is_last {
        let total = params.total_reduction + params.assembly_reduction;
        game.pending_assembly_materials = Some((target_card, all_picked));
        let _ = assembly_finish(game, player, target_card, params, total);
        game.pending_assembly_materials = None;
    } else {
        install_assembly_element(
            game,
            player,
            target_card,
            params,
            elements,
            element_idx + 1,
            all_picked,
        );
    }
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
    total_reduction: crate::game_actions::PlayCostAdjustments,
) -> PlayFromHandCostResult {
    let Some(hand_index) = game
        .player(player)
        .hand
        .iter()
        .position(|c| c.handle() == target_card)
    else {
        return PlayFromHandCostResult::Failed;
    };
    game.with_play_declaration_controller(params.effect_controller, |game| {
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
    })
}

#[cfg(test)]
mod security_move_lifecycle_tests {
    use crate::card_source::CardHandle;
    use crate::debug_runner::{make_test_card, DebugRunner};
    use crate::effect::{CardEffect, Effect};
    use crate::enums::{CardSourceRef, StackPosition};
    use crate::permanent::PermanentHandle;
    struct Protect;
    impl CardEffect for Protect {
        fn effects(&self, c: CardHandle) -> Vec<Effect> {
            vec![Effect::when_would_leave_battle_area(c)
                .optional()
                .replacement_process(|ctx| ctx.cancel())
                .build()]
        }
    }
    fn parked() -> DebugRunner {
        let mut r = DebugRunner::builder()
            .add_card(make_test_card("TOP", "Top"))
            .add_card(make_test_card("SOURCE", "Source"))
            .deck(0, &["SOURCE"; 8])
            .deck(1, &["SOURCE"; 8])
            .build();
        r.register_effect("TOP", std::sync::Arc::new(Protect));
        r.place_stack(0, &["SOURCE", "TOP"]);
        r.game.place_on_security(
            1,
            CardSourceRef::Material(
                PermanentHandle {
                    player: 0,
                    index: 0,
                },
                1,
            ),
            StackPosition::Bottom,
            true,
        );
        assert!(r.pending_kind().is_some());
        assert_eq!(r.game.pending_security_moves.len(), 1);
        r
    }
    #[test]
    fn terminal_winner_clears_security_move_without_mutating_clone() {
        let mut r = parked();
        let saved = r.game.clone();
        r.game
            .declare_winner_with_reason(0, crate::TerminalOutcomeReason::EngineDeclared);
        assert!(r.game.pending_security_moves.is_empty());
        assert_eq!(saved.pending_security_moves.len(), 1);
    }
    #[test]
    fn terminal_elimination_clears_security_move() {
        let mut r = parked();
        r.game.eliminate_player(1);
        assert!(r.game.pending_security_moves.is_empty());
    }
    #[test]
    fn terminal_deckout_clears_security_move() {
        let mut r = parked();
        r.game.handle_deckout(1);
        assert!(r.game.pending_security_moves.is_empty());
    }
    #[test]
    fn pending_security_digest_binds_destination_orientation_stage_and_physical_identity() {
        let r = parked();
        let baseline = r.game.verification_digest();
        for variant in 0..6 {
            let mut branch = r.game.clone();
            let p = &mut branch.pending_security_moves[0];
            match variant {
                0 => p.player = 0,
                1 => p.position = StackPosition::Top,
                2 => p.face_up = false,
                3 => p.leave_complete = true,
                4 => p.observer ^= 1,
                5 => p.donor.1 += 1,
                _ => unreachable!(),
            };
            assert_ne!(
                branch.verification_digest(),
                baseline,
                "pending payload variant {variant}"
            );
        }
        assert_eq!(r.game.verification_digest(), baseline);
    }
    #[test]
    fn declined_security_move_clears_intent_and_clone_digest_agrees() {
        let mut r = parked();
        r.execute_action(0, crate::action::space::PASS).unwrap();
        assert!(r.game.pending_security_moves.is_empty());
        assert!(r.game.players[0].battle_area.is_empty());
        let clone = r.game.clone();
        assert_eq!(clone.verification_digest(), r.game.verification_digest());
    }
}


#[cfg(test)]
mod physical_effect_origin_tests {
    use crate::{debug_runner::DebugRunner, enums::EffectSourceKind};
    fn source_kind(linked: bool) {
        let mut r = DebugRunner::builder().dsl_card("BT12-092").unwrap()
            .dsl_card("ST1-03").unwrap().start();
        let h = r.place_on_field(0, "ST1-03", Some(0));
        let index = r.add_to_hand(0, "BT12-092");
        let card = r.game.players[0].hand.remove(index);
        let physical = card.handle();
        if linked { r.game.players[0].battle_area[h.index as usize].linked_cards.push(card); }
        else { r.game.players[0].battle_area[h.index as usize].card_sources.insert(0, card); }
        // This helper supplies explicit provenance to cost/replacement choosers.
        // A separate same-ID hand copy remains a printed Tamer origin.
        let other = r.add_to_hand(0, "BT12-092");
        let other = r.game.players[0].hand[other].handle();
        for g in [&r.game, &r.game.clone()] {
            assert_eq!(g.effect_source_kind_for_handle(physical), EffectSourceKind::Digimon);
            assert_eq!(g.effect_source_kind_for_handle(other), EffectSourceKind::Tamer);
        }
    }
    #[test] fn inherited_tamer_physical_origin_for_cost_and_replacement_is_digimon() { source_kind(false); }
    #[test] fn linked_tamer_physical_origin_for_cost_and_replacement_is_digimon() { source_kind(true); }
}
