//! Zone-card multi-pick: pick concrete CARDS across hand / trash / battle-area
//! stacks / link cards in one prompt, then act on the picked list.
//!
//! - `select_zone_cards` — BT26-081 Mervamon ("up to 8 play cost's total worth of
//!   [Iliad] trait cards from your hand or trash",
//!   G-ENGINE-PLAY-COST-BUDGET-FROM-HAND-OR-TRASH) and BT26-102 Seven Code PAD
//!   ("6 [Seven Code] trait Digimon cards from your battle area, link cards or
//!   trash", G-DSL-PLACE-MATERIALS-MULTI-ZONE); binds the picks as a card list.
//! - `return_top_stacked_to_deck` — BT26-060 Chronomon: Destroy Mode ("return the
//!   top 5 stacked cards of 3 of your opponent's Digimon to the top of the deck",
//!   G-ENGINE-RETURN-TOP-N-STACKED-TO-DECK); an ORDERED pick over the cards still
//!   on the field, then the ordered return.
//! - `play_cards_free` / `place_cards_as_bottom_sources` — the synchronous actions
//!   that consume a picked card list.
//!
//! ## Action space
//!
//! Every candidate is encoded in an EXISTING range, so the selection is fully in
//! the 2192-action space and needs no spec change:
//! - hand card `i`  → `PLAY_HAND_START + i`
//! - trash card `i` → `TRASH_EFFECT_START + i`
//! - a card at stack index `s` (top card included) of field `f`
//!   → `encode_source_select(f, s)`
//! - link card `k` of field `f` → `encode_source_select(f, stack_len + k)`
//!
//! `PendingSelection::zone_owner` names whose zones the ids index (absent = the
//! selecting player). The frontend's card-zone picker
//! (`code/frontend/src/utils/cardZoneSelection.ts`) decodes the same ranges.
//!
//! Clone-safe: each decision parks a [`ZoneCardPickState`] data frame
//! (`ResumeFrame::ZoneCardPickStep`); the `PendingSelection` callback is inert.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPredicate, CompiledStep};
use digimon_dsl::step::ZoneCardZone;

use crate::action::space::{
    encode_source_select, PASS, PLAY_HAND_END, PLAY_HAND_START, TRASH_EFFECT_END,
    TRASH_EFFECT_START,
};
use crate::card_source::CardHandle;
use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use crate::dsl_cards::step::selections::run_outer_conts;
use crate::dsl_cards::step::selections::run_tail_preserving_trigger_context;
use crate::dsl_cards::step::StepRuntime;
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::{CardSourceRef, GamePhase, PlayerId, StackPosition};
use crate::permanent::PermanentHandle;

/// What a finished zone-card pick does with its picks.
#[derive(Debug, Clone)]
pub(crate) enum ZoneCardTerminal {
    /// Bind the picked cards (in pick order) as a card list, run the tail.
    Bind { bind_as: Option<String> },
    /// Return the picked cards to their owners' decks in pick order (pick 1
    /// ends up topmost of the returned group), then run the tail.
    ReturnToDeck { position: StackPosition },
}

/// In-flight state of a zone-card pick, as data (re-parked once per pick).
#[derive(Debug, Clone)]
pub(crate) struct ZoneCardPickState {
    pub prov: crate::resume::ResumeProvenance,
    pub selecting_player: PlayerId,
    pub previous_phase: GamePhase,
    pub zone_owner: Option<PlayerId>,
    /// Ordered pick (a permutation) rather than a set pick.
    pub ordered: bool,
    /// Candidate cards, fixed at install; each is re-located on every step
    /// (a card that has left its zone is no longer offered).
    pub candidates: Vec<CardHandle>,
    pub picked: Vec<CardHandle>,
    pub min: u8,
    pub max: u8,
    pub optional_zero: bool,
    /// Remaining "play cost's total worth" budget, if any.
    pub budget: Option<i32>,
    pub prompt: String,
    pub terminal: ZoneCardTerminal,
    pub inner_tail: Arc<Vec<CompiledStep>>,
    pub bindings: Bindings,
    pub runtime: StepRuntime,
    pub trigger_context: Option<crate::trigger_context::TriggerContext>,
    pub outer_conts: Vec<crate::resume::OuterContinuation>,
}

/// The action id a card currently answers to, or `None` if it can't be
/// encoded (or isn't in a pickable zone any more).
pub(crate) fn encode_zone_card(game: &crate::game::Game, card: CardHandle) -> Option<u16> {
    match game.locate_card_source_ref(card)? {
        CardSourceRef::Hand(_, i) => {
            let id = PLAY_HAND_START + i as u16;
            (id < PLAY_HAND_END).then_some(id)
        }
        CardSourceRef::Trash(_, i) => {
            let id = TRASH_EFFECT_START + i as u16;
            (id < TRASH_EFFECT_END).then_some(id)
        }
        CardSourceRef::Material(h, s) if h.index != crate::action::space::BREEDING_TARGET as u8 => {
            encode_source_select(h.index as u16, s as u16)
        }
        CardSourceRef::Link(h, k) => {
            let stack_len = game
                .player(h.player)
                .battle_area
                .get(h.index as usize)?
                .card_sources
                .len();
            encode_source_select(h.index as u16, (stack_len + k) as u16)
        }
        _ => None,
    }
}

fn play_cost_of(game: &crate::game::Game, card: CardHandle) -> i32 {
    game.card_data_for_handle(card)
        .map(|d| d.play_cost as i32)
        .unwrap_or(0)
}

/// Candidate cards for a `select_zone_cards` over `player`'s `zones`.
#[allow(clippy::too_many_arguments)]
fn collect_zone_candidates(
    ctx: &EffectContext<'_>,
    player: PlayerId,
    zones: &[ZoneCardZone],
    filter: &CompiledPredicate,
    exclude: Option<PermanentHandle>,
    bindings: &Bindings,
) -> Vec<CardHandle> {
    let game = &*ctx.game;
    let rctx = EffectReadContext::new(game, ctx.source_card, ctx.source_permanent, ctx.player);
    let matches = |h: CardHandle| {
        eval_predicate_with_bindings(filter, &rctx, PredicateSubject::Card(h), Some(bindings))
    };
    let p = game.player(player);
    let mut out = Vec::new();
    for zone in zones {
        match zone {
            ZoneCardZone::Hand => {
                out.extend(p.hand.iter().map(|c| c.handle()).filter(|h| matches(*h)));
            }
            ZoneCardZone::Trash => {
                out.extend(p.trash.iter().map(|c| c.handle()).filter(|h| matches(*h)));
            }
            ZoneCardZone::BattleArea => {
                for (i, perm) in p.battle_area.iter().enumerate() {
                    let handle = PermanentHandle {
                        player,
                        index: i as u8,
                    };
                    if Some(handle) == exclude || perm.card_sources.is_empty() {
                        continue;
                    }
                    let top = perm.top_card().handle();
                    if matches(top) {
                        out.push(top);
                    }
                }
            }
            ZoneCardZone::LinkCards => {
                // `exclude` only removes the excluded permanent as a
                // battle-area pick; its LINK cards stay eligible (DCGO
                // BT26_102 CanSelectLinkPermanentCondition).
                for perm in p.battle_area.iter() {
                    out.extend(
                        perm.linked_cards
                            .iter()
                            .map(|c| c.handle())
                            .filter(|h| matches(*h)),
                    );
                }
            }
        }
    }
    out
}

fn prov_of(ctx: &EffectContext<'_>) -> crate::resume::ResumeProvenance {
    crate::resume::ResumeProvenance {
        source_card: ctx.source_card,
        source_permanent: ctx.source_permanent,
        source_kind: ctx.source_kind,
        controller: ctx.player,
        override_pin: ctx.override_selecting_player(),
    }
}

/// Handle `select_zone_cards` / `return_top_stacked_to_deck`. Returns `true`
/// when `step` was one of them: the step either parked a selection or ran the
/// captured tail itself, so the caller must stop its loop.
pub(crate) fn try_install(
    step: &CompiledStep,
    tail: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: Bindings,
    runtime: &StepRuntime,
) -> bool {
    match step {
        CompiledStep::SelectZoneCards {
            of,
            zones,
            filter,
            min,
            max,
            optional_zero,
            play_cost_budget,
            exclude,
            bind_as,
            prompt,
        } => {
            let player = super::resolve_player(ctx, *of);
            let exclude =
                exclude
                    .as_ref()
                    .and_then(|r| match resolve_binding_ref(r, ctx, &bindings) {
                        Some(ResolvedBinding::Permanent(h)) => Some(h),
                        _ => None,
                    });
            let budget = play_cost_budget.as_ref().map(|f| {
                let target = ctx.source_permanent.unwrap_or(PermanentHandle {
                    player: ctx.player,
                    index: 0,
                });
                crate::dsl_cards::formula_eval::evaluate_with_bindings(
                    f,
                    ctx,
                    target,
                    Some(&bindings),
                )
            });
            let mut candidates =
                collect_zone_candidates(ctx, player, zones, filter, exclude, &bindings);
            // Fewer candidates than a required count: the count can't be met,
            // so nothing is offered (an unpayable "by placing 6 …" cost).
            if candidates.len() < *min as usize {
                candidates.clear();
            }
            let selecting_player = ctx.override_selecting_player().unwrap_or(ctx.player);
            let state = ZoneCardPickState {
                prov: prov_of(ctx),
                selecting_player,
                previous_phase: ctx.game.current_phase,
                zone_owner: (player != selecting_player).then_some(player),
                ordered: false,
                candidates,
                picked: Vec::new(),
                min: *min,
                max: *max,
                optional_zero: *optional_zero,
                budget,
                prompt: prompt.clone(),
                terminal: ZoneCardTerminal::Bind {
                    bind_as: bind_as.clone(),
                },
                inner_tail: Arc::new(tail.to_vec()),
                bindings,
                runtime: runtime.clone(),
                trigger_context: ctx.game.current_trigger_context.clone(),
                outer_conts: Vec::new(),
            };
            step_or_finish(ctx.game, state);
            true
        }
        CompiledStep::ReturnTopStackedToDeck {
            targets,
            count,
            position,
            prompt,
        } => {
            let targets: Vec<PermanentHandle> = match resolve_binding_ref(targets, ctx, &bindings) {
                Some(ResolvedBinding::Permanent(h)) => vec![h],
                Some(ResolvedBinding::PermanentList(v)) => v,
                _ => Vec::new(),
            };
            let target = ctx.source_permanent.unwrap_or(PermanentHandle {
                player: ctx.player,
                index: 0,
            });
            let count = crate::dsl_cards::formula_eval::evaluate_with_bindings(
                count,
                ctx,
                target,
                Some(&bindings),
            )
            .max(0) as usize;
            let mut cards = Vec::new();
            let mut owner = None;
            for h in targets {
                if stack_return_blocked(ctx, h) {
                    continue;
                }
                let Some(perm) = ctx.game.player(h.player).battle_area.get(h.index as usize) else {
                    continue;
                };
                // Top card included; at least one card always stays.
                let n = count.min(perm.card_sources.len().saturating_sub(1));
                let len = perm.card_sources.len();
                cards.extend(
                    perm.card_sources[len - n..]
                        .iter()
                        .rev()
                        .map(|c| c.handle()),
                );
                owner.get_or_insert(h.player);
            }
            let selecting_player = ctx.override_selecting_player().unwrap_or(ctx.player);
            let state = ZoneCardPickState {
                prov: prov_of(ctx),
                selecting_player,
                previous_phase: ctx.game.current_phase,
                zone_owner: owner.filter(|o| *o != selecting_player),
                ordered: true,
                min: cards.len() as u8,
                max: cards.len() as u8,
                // DCGO asks for the order only when there are 2+ cards.
                picked: if cards.len() == 1 {
                    cards.clone()
                } else {
                    Vec::new()
                },
                candidates: cards,
                optional_zero: false,
                budget: None,
                prompt: prompt.clone(),
                terminal: ZoneCardTerminal::ReturnToDeck {
                    position: super::map_stack_position(*position),
                },
                inner_tail: Arc::new(tail.to_vec()),
                bindings,
                runtime: runtime.clone(),
                trigger_context: ctx.game.current_trigger_context.clone(),
                outer_conts: Vec::new(),
            };
            step_or_finish(ctx.game, state);
            true
        }
        _ => false,
    }
}

/// `ImmuneFromStackReturnToLibrary` / "can't be affected" (DCGO BT26_060):
/// a permanent that this effect can't affect, or whose cards can't be returned
/// to the deck by this effect's controller, is skipped.
fn stack_return_blocked(ctx: &EffectContext<'_>, target: PermanentHandle) -> bool {
    use crate::replacement::ReplacementCause;
    if !ctx.can_affect_permanent(target) {
        return true;
    }
    let from_opponent = ctx.player != target.player;
    ctx.game
        .modifiers
        .get(target, crate::enums::ModifierType::CannotBeReturnedToDeck)
        .iter()
        .any(|entry| match entry.cause_filter {
            None => true,
            Some(ReplacementCause::OpponentEffect) => from_opponent,
            Some(ReplacementCause::OwnEffect) => !from_opponent,
            Some(_) => false,
        })
}

/// Live candidates: still locatable, encodable, not yet picked, within budget.
fn live_options(game: &crate::game::Game, state: &ZoneCardPickState) -> Vec<(u16, CardHandle)> {
    state
        .candidates
        .iter()
        .copied()
        .filter(|c| !state.picked.contains(c))
        .filter(|c| state.budget.is_none_or(|b| play_cost_of(game, *c) <= b))
        .filter_map(|c| encode_zone_card(game, c).map(|id| (id, c)))
        .collect()
}

/// Park the next decision, or run the terminal when the pick is complete.
fn step_or_finish(game: &mut crate::game::Game, state: ZoneCardPickState) {
    use crate::selection::{PendingSelection, SelectionKind};
    let options = live_options(game, &state);
    let picked = state.picked.len() as u8;
    if picked >= state.max || options.is_empty() {
        // An all-or-nothing count that can no longer be met pays nothing.
        if picked < state.min && !(state.optional_zero && picked == 0) {
            let mut state = state;
            state.picked.clear();
            finish(game, state);
            return;
        }
        finish(game, state);
        return;
    }
    let is_optional = picked >= state.min || (state.optional_zero && picked == 0);
    let mut valid_action_ids: Vec<u16> = options.iter().map(|(id, _)| *id).collect();
    if is_optional {
        valid_action_ids.push(PASS);
    }
    let (phase, kind) = if state.ordered {
        (
            GamePhase::SelectPermutation,
            SelectionKind::OrderedPermutation {
                remaining: options.len() as u8,
            },
        )
    } else if let Some(remaining) = state.budget {
        (
            GamePhase::SelectBudgeted,
            SelectionKind::PlayCostBudget {
                remaining_play_cost: remaining,
                picked,
            },
        )
    } else {
        (
            GamePhase::SelectBudgeted,
            SelectionKind::CountCappedMultiSelect {
                min: state.min,
                max: state.max,
                picked,
                distinct: false,
            },
        )
    };
    game.current_phase = phase;
    game.pending_selection = Some(PendingSelection {
        zone_owner: state.zone_owner,
        kind,
        selecting_player: state.selecting_player,
        previous_phase: state.previous_phase,
        valid_action_ids,
        is_optional,
        prompt: state.prompt.clone(),
        effect_choices: None,
        source_card: state.prov.source_card,
        source_permanent: state.prov.source_permanent,
        source_kind: state.prov.source_kind,
        callback: Box::new(|_g, _a| {}),
        on_decline: None,
    });
    game.pending_selection_resume = Some(crate::resume::ResumeStack {
        frames: vec![crate::resume::ResumeFrame::ZoneCardPickStep(state)],
    });
}

/// Resume executor: one pick (or PASS).
pub(crate) fn run_zone_card_pick_step(
    game: &mut crate::game::Game,
    mut state: ZoneCardPickState,
    action_id: u16,
    is_pass: bool,
) {
    if is_pass {
        let picked = state.picked.len() as u8;
        if picked < state.min && !(state.optional_zero && picked == 0) {
            // PASS is only offered when committable; stay defensive.
            step_or_finish(game, state);
            return;
        }
        finish(game, state);
        return;
    }
    let chosen = live_options(game, &state)
        .into_iter()
        .find(|(id, _)| *id == action_id)
        .map(|(_, c)| c);
    if let Some(card) = chosen {
        if let Some(b) = state.budget.as_mut() {
            *b -= play_cost_of(game, card);
        }
        state.picked.push(card);
    }
    step_or_finish(game, state);
}

fn finish(game: &mut crate::game::Game, state: ZoneCardPickState) {
    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    let mut b = state.bindings;
    match &state.terminal {
        ZoneCardTerminal::Bind { bind_as } => {
            // No picks → the name stays UNBOUND, so `binding_present` reads
            // "nothing was chosen" (an all-or-nothing cost that wasn't paid).
            if let (Some(name), false) = (bind_as, state.picked.is_empty()) {
                b.insert_card_list(name, state.picked.clone());
            }
        }
        ZoneCardTerminal::ReturnToDeck { position } => {
            return_cards_to_deck(&mut ctx, &state.picked, *position);
        }
    }
    run_tail_preserving_trigger_context(
        &mut ctx,
        state.trigger_context,
        &state.inner_tail,
        &mut b,
        &state.runtime,
    );
    run_outer_conts(game, state.outer_conts);
}

/// Move `ordered` cards (pick 1 first) from wherever they are to their
/// owners' decks; pick 1 ends up topmost of the returned group.
fn return_cards_to_deck(
    ctx: &mut EffectContext<'_>,
    ordered: &[CardHandle],
    position: StackPosition,
) {
    let iter: Box<dyn Iterator<Item = &CardHandle>> = match position {
        StackPosition::Top => Box::new(ordered.iter().rev()),
        _ => Box::new(ordered.iter()),
    };
    let mut moved = false;
    for &card in iter {
        let Some(mut c) = ctx.game.take_card_by_handle(card) else {
            continue;
        };
        c.face_down = false;
        let owner = c.owner;
        let deck = &mut ctx.game.player_mut(owner).deck;
        match position {
            StackPosition::Top => deck.push(c),
            _ => deck.insert(0, c),
        }
        moved = true;
    }
    if moved {
        let controller = ctx.player;
        ctx.game.note_effect_deck_add(controller);
        ctx.game.tick_declarative_effects();
    }
}

/// `play_cards_free` — play the listed hand/trash cards simultaneously, free.
pub(crate) fn play_cards_free(ctx: &mut EffectContext<'_>, cards: &[CardHandle]) {
    let controller = ctx.player;
    ctx.game.play_cards_from_zones_free(controller, cards);
}

/// `place_cards_as_bottom_sources` — place each listed card under `target`.
/// Returns the host's handle afterwards (moving a whole battle-area Digimon
/// shifts later permanent indices), or `None` if it left the field.
pub(crate) fn place_cards_as_bottom_sources(
    ctx: &mut EffectContext<'_>,
    cards: &[CardHandle],
    target: PermanentHandle,
) -> Option<PermanentHandle> {
    let target_card = ctx
        .game
        .player(target.player)
        .battle_area
        .get(target.index as usize)
        .map(|p| p.top_card().handle());
    let target_card = target_card?;
    for &card in cards {
        // Re-resolve the target each time: moving a whole battle-area Digimon
        // shifts later permanent indices.
        let host = ctx.game.permanent_with_top_card(target_card)?;
        let Some(src) = ctx.game.locate_card_source_ref(card) else {
            continue;
        };
        let _ = ctx.place_as_bottom_source(src, host, false);
    }
    ctx.game.permanent_with_top_card(target_card)
}
