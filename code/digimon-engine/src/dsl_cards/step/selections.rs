//! Selection-step lowering: install a `PendingSelection` with the
//! remainder of the process-step slice as its callback.
//!
//! Phase 2b: `SelectHand`, `SelectTrash`, `SelectOwnPermanent`,
//! `SelectOpponentPermanent`.
//!
//! **Known limitation (Phase 2b):** the `EffectContext::select_*` filter
//! closure is `Fn(&Game, ...) -> bool`, not `Fn(&EffectReadContext, ...)`.
//! Evaluating a `CompiledPredicate` needs the full read-context tuple
//! (`source_card`, `source_permanent`, `player`), so Phase 2b accepts
//! all candidates at install time. Phase 2c widens the filter signature.

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledCountBound, CompiledFieldSelector, CompiledPlayerRef,
    CompiledPredicate, CompiledRemainderDestination, CompiledRevealBucket,
    CompiledRevealDestination, CompiledRevealRemainder, CompiledRevealSearchDest,
    CompiledStackPosition, CompiledStep, CompiledZone,
};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::formula_eval;
use crate::dsl_cards::predicate::{eval_predicate, eval_predicate_with_bindings, PredicateSubject};
use crate::dsl_cards::step::{
    drain_or_rewrap_pending_tail, resolve_player, run_steps_with_runtime, StepRuntime,
};
use crate::effect_context::{
    CountCappedZone, DistinctByMode, EffectContext, EffectReadContext, RevealBucketSelection,
};
use crate::enums::{CardKind, GamePhase, PlayerId};
use crate::permanent::PermanentHandle;
use crate::selection::{PendingSelection, SelectionKind};
use crate::trigger_context::TriggerContext;

fn map_distinct_by(d: Option<digimon_dsl::compiled::CompiledDistinctBy>) -> Option<DistinctByMode> {
    use digimon_dsl::compiled::CompiledDistinctBy;
    d.map(|c| match c {
        CompiledDistinctBy::CardNumber => DistinctByMode::CardNumber,
        CompiledDistinctBy::Level => DistinctByMode::Level,
        CompiledDistinctBy::Name => DistinctByMode::Name,
    })
}

fn formula_value(
    formula: &digimon_dsl::compiled::CompiledFormula,
    ctx: &EffectContext<'_>,
    bindings: &Bindings,
) -> i32 {
    let target = ctx.source_permanent.unwrap_or(PermanentHandle {
        player: ctx.player,
        index: u8::MAX,
    });
    formula_eval::evaluate_with_bindings(formula, ctx, target, Some(bindings))
}

fn collect_matching_permanents(
    ctx: &EffectContext<'_>,
    player: u8,
    filter: &CompiledPredicate,
    bindings: Option<&Bindings>,
) -> Vec<PermanentHandle> {
    let read = ctx.as_read();
    let mut handles = Vec::new();
    for index in 0..read.game.player(player).battle_area.len() {
        let handle = PermanentHandle {
            player,
            index: index as u8,
        };
        if eval_predicate_with_bindings(
            filter,
            &read,
            PredicateSubject::Permanent(handle),
            bindings,
        ) {
            handles.push(handle);
        }
    }
    handles
}

fn collect_matching_any_permanents(
    ctx: &EffectContext<'_>,
    excluded: Option<PermanentHandle>,
    filter: &CompiledPredicate,
    bindings: Option<&Bindings>,
) -> Vec<PermanentHandle> {
    let read = ctx.as_read();
    let mut handles = Vec::new();
    for player in 0..read.game.players.len() {
        let player = player as u8;
        for index in 0..read.game.player(player).battle_area.len() {
            let handle = PermanentHandle {
                player,
                index: index as u8,
            };
            if Some(handle) == excluded {
                continue;
            }
            if eval_predicate_with_bindings(
                filter,
                &read,
                PredicateSubject::Permanent(handle),
                bindings,
            ) {
                handles.push(handle);
            }
        }
    }
    handles
}

/// Read a permanent's value in the unit of the given field selector:
/// effective DP for DP selectors, printed play cost for play-cost selectors.
fn field_value(
    game: &crate::game::Game,
    handle: PermanentHandle,
    selector: CompiledFieldSelector,
) -> Option<i32> {
    match selector {
        CompiledFieldSelector::LowestDp | CompiledFieldSelector::HighestDp => {
            game.effective_dp(handle)
        }
        CompiledFieldSelector::LowestPlayCost | CompiledFieldSelector::HighestPlayCost => game
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|perm| i32::from(perm.top_card().play_cost(&game.card_data))),
        CompiledFieldSelector::LowestMaterialCount => game
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|perm| perm.card_sources.len().saturating_sub(1) as i32),
    }
}

fn selected_field_extreme(
    game: &crate::game::Game,
    handles: &[PermanentHandle],
    selector: CompiledFieldSelector,
) -> Option<i32> {
    let values = handles
        .iter()
        .filter_map(|h| field_value(game, *h, selector));
    match selector {
        CompiledFieldSelector::LowestDp | CompiledFieldSelector::LowestPlayCost => values.min(),
        CompiledFieldSelector::HighestDp | CompiledFieldSelector::HighestPlayCost => values.max(),
        CompiledFieldSelector::LowestMaterialCount => values.min(),
    }
}

/// Result of resolving an optional `CompiledFieldSelector` over a
/// candidate set. `Exact` carries the extreme value in the selector's
/// own unit (DP or play cost).
#[derive(Clone, Copy)]
enum SelectedField {
    Any,
    Exact(i32),
    None,
}

fn select_field_extreme(
    game: &crate::game::Game,
    handles: &[PermanentHandle],
    selector: Option<CompiledFieldSelector>,
) -> SelectedField {
    match selector {
        None => SelectedField::Any,
        Some(selector) => match selected_field_extreme(game, handles, selector) {
            Some(value) => SelectedField::Exact(value),
            None => SelectedField::None,
        },
    }
}

fn matches_selected_field(
    game: &crate::game::Game,
    handle: PermanentHandle,
    selector: Option<CompiledFieldSelector>,
    selected: SelectedField,
) -> bool {
    match selected {
        SelectedField::Any => true,
        SelectedField::Exact(want) => selector
            .and_then(|selector| field_value(game, handle, selector))
            .is_some_and(|candidate| candidate == want),
        SelectedField::None => false,
    }
}

pub(crate) fn run_tail_preserving_trigger_context(
    cb_ctx: &mut EffectContext<'_>,
    trigger_context: Option<TriggerContext>,
    tail: &[CompiledStep],
    bindings: &mut Bindings,
    runtime: &StepRuntime,
) {
    let previous = cb_ctx.game.current_trigger_context.clone();
    cb_ctx.game.current_trigger_context = trigger_context;
    run_steps_with_runtime(tail, cb_ctx, bindings, runtime);
    // Overlay this resolution's bindings onto the parked outer tail so
    // binding-gated siblings after a nested select see the pick.
    crate::dsl_cards::step::drain_dsl_outer_tail_with_bindings(cb_ctx, Some(bindings));
    // Publish the freshest bindings of this resolution for any WRAPPED outer
    // tail (`wrap_pending_selection_with_tail` composed around this callback)
    // — its wrap-time snapshot predates the pick this resolution just made.
    // The wrapper clears the channel before invoking us and takes it right
    // after, so the value never leaks across resolutions.
    cb_ctx.game.dsl_resolved_tail_bindings = Some(bindings.clone());
    cb_ctx.game.current_trigger_context = previous;
}

/// Coexistence-phase resumable-VM executor (make-engine-cloneable spike,
/// task 0.2). Runs a `ResumeStack` of plain-data frames in place of the legacy
/// `PendingSelection.callback`. Each arm mirrors the corresponding install
/// closure body exactly and reuses `run_tail_preserving_trigger_context`, so a
/// resolved data frame is behaviorally identical to the closure path. Invoked
/// by `resolve_generic_selection` when `Game::pending_selection_resume` is set.
pub(crate) fn run_resume(
    game: &mut crate::game::Game,
    mut stack: crate::resume::ResumeStack,
    action_id: u16,
    is_pass: bool,
) {
    use crate::resume::{ResumeFrame, ResumeSelectKind};
    let Some(frame) = stack.frames.pop() else {
        return;
    };
    match frame {
        ResumeFrame::RunTail {
            prov,
            select_kind,
            bind_as,
            inner_tail,
            outer_conts,
            bindings,
            runtime,
            trigger_context,
            decline,
        } => {
            if is_pass {
                // Decline mirrors each installer's on_decline:
                //   None       → no on_decline installed (e.g. select_security): do nothing.
                //   RunTail{..} → run the decline tail; aborts_clause first sets
                //                 dsl_clause_aborted (G-OPTIONAL-COST-DECLINE-ABORTS-CLAUSE).
                if let crate::resume::ResumeDecline::RunTail {
                    tail,
                    aborts_clause,
                } = decline
                {
                    if aborts_clause {
                        game.dsl_clause_aborted = true;
                    }
                    let mut ctx = EffectContext::new_with_source_kind_and_override(
                        game,
                        prov.source_card,
                        prov.source_permanent,
                        prov.source_kind,
                        prov.controller,
                        prov.override_pin,
                    );
                    let mut b = bindings.clone();
                    run_tail_preserving_trigger_context(
                        &mut ctx,
                        trigger_context,
                        &tail,
                        &mut b,
                        &runtime,
                    );
                    // Nested-composition: an interrupt's outer clause may have
                    // wrapped its tail onto this (resume-driven) select. Run it
                    // after the decline tail, exactly as the wrapped on_decline
                    // closure did (cost-declined → dsl_clause_aborted is set, so
                    // drain_or_rewrap's run_steps short-circuits the outer tail).
                    run_outer_conts(ctx.game, outer_conts);
                }
                // ResumeDecline::None mirrors a select with no on_decline: PASS
                // runs nothing, so the outer conts are dropped (parity).
                return;
            }
            match select_kind {
                ResumeSelectKind::Hand { of_player } => {
                    let hand_index =
                        action_id.saturating_sub(crate::action::space::PLAY_HAND_START) as usize;
                    // Target tracking (mirrors `ctx.select_hand`'s wrapper).
                    if let Some(card) = game.player(of_player).hand.get(hand_index) {
                        let tid = card.card_id(&game.card_data).to_string();
                        let tname = card.card_name(&game.card_data).to_string();
                        crate::effect_context::selections::push_effect_target(
                            game,
                            prov.controller,
                            prov.source_card,
                            tid,
                            tname,
                        );
                    }
                    let mut ctx = EffectContext::new_with_source_kind_and_override(
                        game,
                        prov.source_card,
                        prov.source_permanent,
                        prov.source_kind,
                        prov.controller,
                        prov.override_pin,
                    );
                    let mut b = bindings;
                    if let Some(name) = &bind_as {
                        b.insert_hand_index(name, of_player, hand_index as u16);
                    }
                    run_tail_preserving_trigger_context(
                        &mut ctx,
                        trigger_context,
                        &inner_tail,
                        &mut b,
                        &runtime,
                    );
                }
                ResumeSelectKind::Trash { of_player } => {
                    let trash_index =
                        action_id.saturating_sub(crate::action::space::TRASH_EFFECT_START) as usize;
                    // Target tracking (mirrors `ctx.select_trash`'s wrapper).
                    if let Some(card) = game.player(of_player).trash.get(trash_index) {
                        let tid = card.card_id(&game.card_data).to_string();
                        let tname = card.card_name(&game.card_data).to_string();
                        crate::effect_context::selections::push_effect_target(
                            game,
                            prov.controller,
                            prov.source_card,
                            tid,
                            tname,
                        );
                    }
                    let mut ctx = EffectContext::new_with_source_kind_and_override(
                        game,
                        prov.source_card,
                        prov.source_permanent,
                        prov.source_kind,
                        prov.controller,
                        prov.override_pin,
                    );
                    let mut b = bindings;
                    if let Some(name) = &bind_as {
                        b.insert_trash_index(name, of_player, trash_index as u16);
                    }
                    run_tail_preserving_trigger_context(
                        &mut ctx,
                        trigger_context,
                        &inner_tail,
                        &mut b,
                        &runtime,
                    );
                }
                ResumeSelectKind::FieldPermanent { of_player, post } => {
                    let offset = action_id.saturating_sub(crate::action::space::ATTACK_START);
                    let target_index = (offset % crate::action::space::TARGETS_PER_ATTACKER) as u8;
                    let h = crate::permanent::PermanentHandle {
                        player: of_player,
                        index: target_index,
                    };
                    // Target tracking (mirrors install_field_selection's wrapper).
                    if let Some(perm) = game
                        .player(of_player)
                        .battle_area
                        .get(target_index as usize)
                    {
                        let top = perm.top_card();
                        let tid = top.card_id(&game.card_data).to_string();
                        let tname = top.card_name(&game.card_data).to_string();
                        crate::effect_context::selections::push_effect_target(
                            game,
                            prov.controller,
                            prov.source_card,
                            tid,
                            tname,
                        );
                    }
                    // Mirror install_field_selection's effect_source_player scoping.
                    let previous_effect_source = game.effect_source_player;
                    game.effect_source_player = Some(prov.controller);
                    let mut ctx = EffectContext::new_with_source_kind_and_override(
                        game,
                        prov.source_card,
                        prov.source_permanent,
                        prov.source_kind,
                        prov.controller,
                        prov.override_pin,
                    );
                    let mut b = bindings;
                    if let Some(name) = &bind_as {
                        b.insert_permanent(name, h);
                    }
                    match post {
                        None => {
                            run_tail_preserving_trigger_context(
                                &mut ctx,
                                trigger_context,
                                &inner_tail,
                                &mut b,
                                &runtime,
                            );
                        }
                        Some(
                            crate::resume::FieldPermanentPostAction::TrashBottomFaceDownSource,
                        ) => {
                            // Cost post-action (mirrors
                            // install_trash_bottom_face_down_source_under_tamer's
                            // callback): trash the picked Tamer's bottom face-down
                            // source, then run the tail ONLY if the cost was paid.
                            // The trash's synchronous drain may park a nested
                            // selection; run_tail_preserving_trigger_context then
                            // re-parks the remainder via park_pending_selection_tail
                            // (identical to the closure path).
                            let trashed = ctx.trash_bottom_face_down_source(h);
                            debug_assert!(
                                trashed,
                                "trash_bottom_face_down_source_under_tamer: eligibility \
                                 filter (has_face_down_source) offered a Tamer whose bottom \
                                 source is not face-down — filter and action have desynced"
                            );
                            if trashed {
                                run_tail_preserving_trigger_context(
                                    &mut ctx,
                                    trigger_context,
                                    &inner_tail,
                                    &mut b,
                                    &runtime,
                                );
                            }
                        }
                        Some(crate::resume::FieldPermanentPostAction::AbsorbStandingAsLink {
                            source,
                        }) => {
                            // Mirror try_run_relink's host callback: absorb the
                            // effect's own standing source permanent onto the
                            // picked host (`h`) as a link card. The absorb drains
                            // (OnDigivolutionCardTrashed/OnLinkedCardTrashed/OnLink)
                            // — NOT atomic; the empty inner_tail is a no-op and the
                            // dispatcher tail (wrapped as outer_conts) composes onto
                            // any nested park via run_outer_conts. No bind.
                            ctx.game.absorb_standing_digimon_as_link(source, h);
                            run_tail_preserving_trigger_context(
                                &mut ctx,
                                trigger_context,
                                &inner_tail,
                                &mut b,
                                &runtime,
                            );
                        }
                        Some(crate::resume::FieldPermanentPostAction::SelectAndTrashLinkCard {
                            optional,
                        }) => {
                            // G-DSL-LINK-TRASH-AS-COST (BT25-073): the first pick
                            // chose one of the controller's Digimon with ≥1 link
                            // card (`h`). Install the SECOND selection over ITS
                            // link cards, carrying `inner_tail` as the cost-gated
                            // tail. The installer parks a
                            // `TrashLinkCardOfDigimonSelection` frame; the frame's
                            // own `outer_conts` compose onto that nested park via
                            // `run_outer_conts` below (empty `outer_conts` passed
                            // here — identical to the closure path).
                            install_link_card_trash_second_selection(
                                &mut ctx,
                                h,
                                optional,
                                inner_tail.clone(),
                                b.clone(),
                                runtime.clone(),
                                trigger_context.clone(),
                                Vec::new(),
                            );
                        }
                        Some(
                            crate::resume::FieldPermanentPostAction::SelectAndTrashStackOption {
                                optional,
                            },
                        ) => {
                            // G-DSL-TRASH-OPTION-FROM-SOURCES-AS-COST (BT25-085):
                            // the first pick chose one of the controller's Digimon
                            // whose digivolution/link cards carry an Option (`h`).
                            // Install the SECOND selection over the UNION of ITS
                            // digivolution-source Options + link-card Options,
                            // carrying `inner_tail` as the cost-gated tail. Mirrors
                            // the `SelectAndTrashLinkCard` arm.
                            install_stack_option_trash_second_selection(
                                &mut ctx,
                                h,
                                optional,
                                inner_tail.clone(),
                                b.clone(),
                                runtime.clone(),
                                trigger_context.clone(),
                                Vec::new(),
                            );
                        }
                    }
                    ctx.game.effect_source_player = previous_effect_source;
                }
                ResumeSelectKind::Security { of_player, post } => {
                    let base = if of_player == prov.controller {
                        crate::action::space::SEL_MY_SECURITY_START
                    } else {
                        crate::action::space::SEL_OPP_SECURITY_START
                    };
                    let index = action_id.saturating_sub(base) as usize;
                    // Target tracking (mirrors `ctx.select_security`'s wrapper).
                    if let Some(card) = game.player(of_player).security.get(index) {
                        let tid = card.card_id(&game.card_data).to_string();
                        let tname = card.card_name(&game.card_data).to_string();
                        crate::effect_context::selections::push_effect_target(
                            game,
                            prov.controller,
                            prov.source_card,
                            tid,
                            tname,
                        );
                    }
                    let mut ctx = EffectContext::new_with_source_kind_and_override(
                        game,
                        prov.source_card,
                        prov.source_permanent,
                        prov.source_kind,
                        prov.controller,
                        prov.override_pin,
                    );
                    let mut b = bindings;
                    match post {
                        None => {
                            if let Some(name) = &bind_as {
                                if let Some(card) = ctx.game.player(of_player).security.get(index) {
                                    b.insert_card(name, card.handle());
                                }
                            }
                        }
                        Some(crate::resume::SecurityPostAction::AddTopToHand) => {
                            // Mirror install_may_add_top_security_to_hand's callback:
                            // add the target player's TOP security to hand (the
                            // pinned slot is the top; the action adds the top
                            // regardless). No bind. The drain may park a nested
                            // selection — the (empty) inner_tail is a no-op and the
                            // outer conts compose onto it via `run_outer_conts`
                            // (→ `drain_or_rewrap_pending_tail`), so a deep chain
                            // threads exactly as the wrapped accept closure did.
                            ctx.add_top_security_to_hand(of_player);
                        }
                    }
                    run_tail_preserving_trigger_context(
                        &mut ctx,
                        trigger_context,
                        &inner_tail,
                        &mut b,
                        &runtime,
                    );
                }
                ResumeSelectKind::BreedingPermanent { of_player } => {
                    // The single breeding permanent is reconstructed from state
                    // (mirrors ctx.select_own_breeding_permanent), not decoded.
                    let Some(card) = game
                        .player(of_player)
                        .breeding_area
                        .as_ref()
                        .map(|p| p.top_card().handle())
                    else {
                        return;
                    };
                    let selection_ref = crate::selection::BreedingPermanentSelectionRef {
                        player: of_player,
                        card,
                    };
                    if let Some(cd) = game.card_data_for_handle(card) {
                        let (tid, tname) = (cd.card_id.clone(), cd.card_name.clone());
                        crate::effect_context::selections::push_effect_target(
                            game,
                            prov.controller,
                            prov.source_card,
                            tid,
                            tname,
                        );
                    }
                    let mut ctx = EffectContext::new_with_source_kind_and_override(
                        game,
                        prov.source_card,
                        prov.source_permanent,
                        prov.source_kind,
                        prov.controller,
                        prov.override_pin,
                    );
                    let mut b = bindings;
                    if let Some(name) = &bind_as {
                        b.insert_breeding_permanent_ref(name, selection_ref);
                    }
                    run_tail_preserving_trigger_context(
                        &mut ctx,
                        trigger_context,
                        &inner_tail,
                        &mut b,
                        &runtime,
                    );
                }
                ResumeSelectKind::AnyPermanent { candidates } => {
                    // Both-battle-area domain: resolve by linear search over the
                    // captured candidates (mirrors install_select_any_permanent).
                    let Some((_, handle)) = candidates
                        .iter()
                        .find(|(candidate_action, _)| *candidate_action == action_id)
                        .copied()
                    else {
                        return;
                    };
                    let mut ctx = EffectContext::new_with_source_kind_and_override(
                        game,
                        prov.source_card,
                        prov.source_permanent,
                        prov.source_kind,
                        prov.controller,
                        prov.override_pin,
                    );
                    let mut b = bindings;
                    if let Some(name) = &bind_as {
                        b.insert_permanent(name, handle);
                    }
                    run_tail_preserving_trigger_context(
                        &mut ctx,
                        trigger_context,
                        &inner_tail,
                        &mut b,
                        &runtime,
                    );
                }
                ResumeSelectKind::Reveal { route } => {
                    let index =
                        action_id.saturating_sub(crate::action::space::SEL_REVEAL_START) as usize;
                    // Resolve the picked handle BEFORE routing moves it out of the
                    // reveal pool (choose_from_reveal). Stale index → skip bind +
                    // route, matching the legacy callbacks' 2b/2c convention.
                    let picked = game.revealed_cards.get(index).map(|c| c.handle());
                    // Target tracking (mirrors ctx.select_reveal's wrapper).
                    if let Some(card) = game.revealed_cards.get(index) {
                        let tid = card.card_id(&game.card_data).to_string();
                        let tname = card.card_name(&game.card_data).to_string();
                        crate::effect_context::selections::push_effect_target(
                            game,
                            prov.controller,
                            prov.source_card,
                            tid,
                            tname,
                        );
                    }
                    let mut ctx = EffectContext::new_with_source_kind_and_override(
                        game,
                        prov.source_card,
                        prov.source_permanent,
                        prov.source_kind,
                        prov.controller,
                        prov.override_pin,
                    );
                    let mut b = bindings;
                    if let Some(handle) = picked {
                        if let Some(name) = &bind_as {
                            b.insert_card(name, handle);
                        }
                        // choose_from_reveal: route the picked card to its
                        // destination (mirrors the closure's route_chosen_reveal).
                        if let Some(route) = &route {
                            route_chosen_reveal(
                                &mut ctx,
                                route.target_player,
                                handle,
                                &route.destination,
                                route.target_permanent,
                            );
                        }
                    }
                    run_tail_preserving_trigger_context(
                        &mut ctx,
                        trigger_context,
                        &inner_tail,
                        &mut b,
                        &runtime,
                    );
                }
                ResumeSelectKind::Material { perm } => {
                    let Some((_, range_start)) =
                        crate::effect_context::selections::material_zone_geometry(game, perm)
                    else {
                        return;
                    };
                    let source_idx = action_id.saturating_sub(range_start) as usize;
                    // Target tracking (mirrors ctx.select_material's battle-area lookup).
                    if let Some(card) = game
                        .player(perm.player)
                        .battle_area
                        .get(perm.index as usize)
                        .and_then(|p| p.card_sources.get(source_idx))
                    {
                        let tid = card.card_id(&game.card_data).to_string();
                        let tname = card.card_name(&game.card_data).to_string();
                        crate::effect_context::selections::push_effect_target(
                            game,
                            prov.controller,
                            prov.source_card,
                            tid,
                            tname,
                        );
                    }
                    let mut ctx = EffectContext::new_with_source_kind_and_override(
                        game,
                        prov.source_card,
                        prov.source_permanent,
                        prov.source_kind,
                        prov.controller,
                        prov.override_pin,
                    );
                    let mut b = bindings;
                    if let Some(name) = &bind_as {
                        // material_carrier_permanent branches battle vs breeding.
                        if let Some(card) =
                            crate::effect_context::selections::material_carrier_permanent(
                                ctx.game, perm,
                            )
                            .and_then(|p| p.card_sources.get(source_idx))
                        {
                            b.insert_card(name, card.handle());
                        }
                    }
                    run_tail_preserving_trigger_context(
                        &mut ctx,
                        trigger_context,
                        &inner_tail,
                        &mut b,
                        &runtime,
                    );
                }
                ResumeSelectKind::EffectChoice { post } => {
                    // "choose one": action_id - HAND_EFFECT_START → label index.
                    let choice_index =
                        action_id.saturating_sub(crate::action::space::HAND_EFFECT_START) as usize;
                    let mut ctx = EffectContext::new_with_source_kind_and_override(
                        game,
                        prov.source_card,
                        prov.source_permanent,
                        prov.source_kind,
                        prov.controller,
                        prov.override_pin,
                    );
                    let mut b = bindings;
                    match post {
                        None => {
                            // Plain choice: bind the index then run inner_tail
                            // (mirrors ctx.select_effect_choice's callback).
                            if let Some(name) = &bind_as {
                                b.insert_literal(name, choice_index as i64);
                            }
                            run_tail_preserving_trigger_context(
                                &mut ctx,
                                trigger_context,
                                &inner_tail,
                                &mut b,
                                &runtime,
                            );
                        }
                        Some(crate::resume::EffectChoicePostAction::OrderRemainder {
                            positions,
                            player,
                        }) => {
                            // Multi-destination order_remainder: map the chosen
                            // index → deck position, then chain into the
                            // already-flipped permutation installer (which parks
                            // its own PermutationStep). Frame-installs-frame: we
                            // do NOT run inner_tail here — it becomes the
                            // permutation's tail. Mirrors install_order_remainder's
                            // select_effect_choice callback, including the
                            // post-install trigger-context restore. outer_conts
                            // thread onto the permutation frame via the shared
                            // run_outer_conts below.
                            let Some(position) = positions.get(choice_index).copied() else {
                                return;
                            };
                            let _ = install_remainder_permutation_with_tail(
                                &mut ctx,
                                player,
                                position,
                                (*inner_tail).clone(),
                                b,
                                runtime,
                            );
                            ctx.game.current_trigger_context = trigger_context;
                        }
                        Some(crate::resume::EffectChoicePostAction::RunTailBranch { branches }) => {
                            let Some(branch) = branches.get(choice_index) else {
                                return;
                            };
                            run_tail_preserving_trigger_context(
                                &mut ctx,
                                trigger_context,
                                branch,
                                &mut b,
                                &runtime,
                            );
                        }
                    }
                }
                ResumeSelectKind::DnaPairLeft {
                    candidates,
                    right_filter,
                    bind_right_as,
                    right_prompt,
                    optional,
                } => {
                    // Resolve the LEFT pick (mirrors install_select_dna_pair's
                    // left callback), bind it, then chain into the already-flipped
                    // install_select_any_permanent for the RIGHT pick (excluding
                    // left). That parks an AnyPermanent frame whose tail is this
                    // frame's inner_tail. We do NOT run inner_tail here. The shared
                    // run_outer_conts after this match transfers any outer_conts
                    // onto the right frame (via drain_or_rewrap_pending_tail).
                    let Some((_, left)) = candidates
                        .iter()
                        .find(|(candidate_action, _)| *candidate_action == action_id)
                        .copied()
                    else {
                        return;
                    };
                    let mut ctx = EffectContext::new_with_source_kind_and_override(
                        game,
                        prov.source_card,
                        prov.source_permanent,
                        prov.source_kind,
                        prov.controller,
                        prov.override_pin,
                    );
                    let mut b = bindings;
                    if let Some(name) = &bind_as {
                        b.insert_permanent(name, left);
                    }
                    install_select_any_permanent(
                        &mut ctx,
                        right_filter,
                        Some(left),
                        None,
                        Some(bind_right_as),
                        right_prompt,
                        optional,
                        (*inner_tail).clone(),
                        b,
                        runtime,
                    );
                }
                ResumeSelectKind::UnionZone {
                    of_player,
                    candidates,
                } => {
                    // Tri-range (hand/trash/material) decode is captured at
                    // install as candidates; resolve by linear search and bind
                    // via insert_union_card (mirrors install_select_union_zone).
                    let Some((_, handle, origin)) = candidates
                        .iter()
                        .find(|(candidate_action, _, _)| *candidate_action == action_id)
                        .copied()
                    else {
                        return;
                    };
                    let mut ctx = EffectContext::new_with_source_kind_and_override(
                        game,
                        prov.source_card,
                        prov.source_permanent,
                        prov.source_kind,
                        prov.controller,
                        prov.override_pin,
                    );
                    let mut b = bindings;
                    if let Some(name) = &bind_as {
                        b.insert_union_card(name, handle, origin, of_player);
                    }
                    run_tail_preserving_trigger_context(
                        &mut ctx,
                        trigger_context,
                        &inner_tail,
                        &mut b,
                        &runtime,
                    );
                }
                ResumeSelectKind::AttackTarget { attacker } => {
                    // Mirror select_redirect_attack_target's callback: decode the
                    // chosen attack target, validate the redirect, and substitute
                    // it (reason EffectRedirect(source_card)). The substitution
                    // fires OnAttackTargetChange + drains (NOT atomic) — a nested
                    // park threads via the empty inner_tail + run_outer_conts
                    // (below), exactly as the wrapped accept closure did.
                    let (decoded_attacker, decoded_target) =
                        crate::action::space::decode_attack(action_id);
                    if decoded_attacker as u8 == attacker.index {
                        let opponent = game.next_clockwise(attacker.player);
                        let target = if decoded_target == crate::action::space::SECURITY_TARGET {
                            crate::selection::AttackTarget::Player(opponent)
                        } else {
                            crate::selection::AttackTarget::Digimon(
                                crate::permanent::PermanentHandle {
                                    player: opponent,
                                    index: decoded_target as u8,
                                },
                            )
                        };
                        if game
                            .validate_attack_redirect_target(attacker, target)
                            .is_ok()
                        {
                            game.apply_attack_target_substitution_with_reason(
                                target,
                                crate::trigger_context::AttackTargetChangeReason::EffectRedirect(
                                    Some(prov.source_card),
                                ),
                            );
                        }
                    }
                    let mut ctx = EffectContext::new_with_source_kind_and_override(
                        game,
                        prov.source_card,
                        prov.source_permanent,
                        prov.source_kind,
                        prov.controller,
                        prov.override_pin,
                    );
                    let mut b = bindings;
                    run_tail_preserving_trigger_context(
                        &mut ctx,
                        trigger_context,
                        &inner_tail,
                        &mut b,
                        &runtime,
                    );
                }
                ResumeSelectKind::BeginAttack {
                    attacker,
                    without_suspending,
                    ignore_summoning_sickness,
                    optional,
                    cost_upgrade,
                } => {
                    // Mirror may_attack_now_*'s callback: decode the chosen target,
                    // then begin_attack_open. That STARTS the attack sub-machine
                    // (counter/block/alliance interrupts), which may park a nested
                    // selection — threaded via the empty inner_tail + run_outer_conts
                    // (below), exactly as the wrapped accept closure did.
                    let (decoded_attacker, decoded_target) =
                        crate::action::space::decode_attack(action_id);
                    if decoded_attacker as u8 == attacker.index {
                        let opponent = game.next_clockwise(attacker.player);
                        let target = if decoded_target == crate::action::space::SECURITY_TARGET {
                            crate::selection::AttackTarget::Player(opponent)
                        } else {
                            crate::selection::AttackTarget::Digimon(
                                crate::permanent::PermanentHandle {
                                    player: opponent,
                                    index: decoded_target as u8,
                                },
                            )
                        };
                        let _ = game.begin_attack_open(crate::combat::AttackOpen {
                            attacker,
                            initiator: crate::combat::AttackInitiator::Effect {
                                source: Some(prov.source_card),
                                optional,
                            },
                            suspend_attacker: !without_suspending,
                            ignore_summoning_sickness,
                            target_constraint: crate::combat::TargetConstraint::Forced(target),
                            allow_cancel: optional,
                            cost_upgrade,
                        });
                    }
                    let mut ctx = EffectContext::new_with_source_kind_and_override(
                        game,
                        prov.source_card,
                        prov.source_permanent,
                        prov.source_kind,
                        prov.controller,
                        prov.override_pin,
                    );
                    let mut b = bindings;
                    run_tail_preserving_trigger_context(
                        &mut ctx,
                        trigger_context,
                        &inner_tail,
                        &mut b,
                        &runtime,
                    );
                }
            }
            // Nested-composition: run any outer-clause tails that were wrapped
            // onto this resume-driven select (see `OuterContinuation`). Runs
            // after the inner tail, mirroring the wrapped accept closure.
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::MultiPickStep(state) => {
            run_multipick_step(game, state, action_id, is_pass);
        }
        ResumeFrame::PermutationStep(state) => {
            run_permutation_step(game, state, action_id);
        }
        ResumeFrame::BudgetStep(state) => {
            run_budget_step(game, state, action_id, is_pass);
        }
        ResumeFrame::SourceMultiStep(state) => {
            run_source_multi_step(game, state, action_id, is_pass);
        }
        ResumeFrame::CountCappedPermanentsStep(state) => {
            run_count_capped_permanent_step(game, state, action_id, is_pass);
        }
        ResumeFrame::PerColorDeleteStep(state) => {
            run_per_color_delete_step(game, state, action_id, is_pass);
        }
        ResumeFrame::NonDslCountCappedStep(state) => {
            run_non_dsl_count_capped_step(game, state, action_id, is_pass);
        }
        ResumeFrame::RevealBucketStep(state) => {
            run_reveal_bucket_step(game, state, action_id, is_pass);
        }
        ResumeFrame::UseOptionFromHandStep(state) => {
            run_use_option_from_hand_step(game, state, action_id, is_pass);
        }
        ResumeFrame::UseOptionFromTrashStep(state) => {
            run_use_option_from_trash_step(game, state, action_id, is_pass);
        }
        ResumeFrame::LinkPickStep(state) => {
            crate::dsl_cards::step::link_cards::run_link_pick_step(game, state, action_id, is_pass);
        }
        ResumeFrame::DigivolveCostChoice(state) => {
            game.run_digivolve_cost_choice_step(state, action_id);
        }
        ResumeFrame::DigivolveReducerPrompt(state) => {
            game.run_digivolve_reducer_prompt_step(state, is_pass);
        }
        ResumeFrame::DigivolveReducerSuspend(state) => {
            game.run_digivolve_reducer_suspend_step(state, action_id);
        }
        ResumeFrame::DnaDigivolveFirstMaterial(state) => {
            game.run_dna_digivolve_first_material_step(state, action_id);
        }
        ResumeFrame::DnaDigivolveSecondMaterial(state) => {
            game.run_dna_digivolve_second_material_step(state, action_id);
        }
        ResumeFrame::RefireEffectChoice(state) => {
            game.run_refire_effect_choice_step(state, action_id, is_pass);
        }
        ResumeFrame::OptionModeSelect(state) => {
            game.run_option_mode_select_step(state, action_id);
        }
        ResumeFrame::DigiXrosMaterialSelection(mut state) => {
            let outer_conts = std::mem::take(&mut state.outer_conts);
            game.run_digixros_material_selection_step(state, action_id, is_pass);
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::OuterOptionalTrigger(mut state) => {
            let outer_conts = std::mem::take(&mut state.outer_conts);
            game.run_outer_optional_trigger_step(state, is_pass);
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::TriggerOrderSelection(mut state) => {
            let outer_conts = std::mem::take(&mut state.outer_conts);
            game.run_trigger_order_selection_step(state, action_id, is_pass);
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::OptionalReplacement(mut state) => {
            let outer_conts = std::mem::take(&mut state.outer_conts);
            crate::replacement::run_optional_replacement_step(game, state, is_pass);
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::DelayCancelAfterSelection {
            inner,
            continuation,
            outer_conts,
        } => {
            run_resume(game, *inner, action_id, is_pass);
            crate::dsl_cards::lower_replacement::continue_delay_cancel_after_selection(
                game,
                continuation,
            );
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::DelayHandDigivolveAfterSelection {
            inner,
            continuation,
            outer_conts,
        } => {
            run_resume(game, *inner, action_id, is_pass);
            crate::dsl_cards::lower_replacement::continue_delay_cost_after_selection(
                game,
                continuation,
            );
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::DelayDnaAfterSelection {
            inner,
            continuation,
            outer_conts,
        } => {
            run_resume(game, *inner, action_id, is_pass);
            crate::dsl_cards::lower_replacement::continue_delay_dna_after_selection(
                game,
                continuation,
            );
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::DelayHandDigivolveSelection(state) => {
            crate::dsl_cards::lower_replacement::run_delay_hand_digivolve_selection_step(
                game, state, action_id, is_pass,
            );
        }
        ResumeFrame::DelayDnaCardSelection(state) => {
            crate::dsl_cards::lower_replacement::run_delay_dna_card_selection_step(
                game, state, action_id, is_pass,
            );
        }
        ResumeFrame::DelayPlayFromHandAfterSelection {
            inner,
            continuation,
            outer_conts,
        } => {
            run_resume(game, *inner, action_id, is_pass);
            crate::dsl_cards::lower_replacement::continue_delay_play_from_hand_after_selection(
                game,
                continuation,
            );
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::DelayPlayFromHandSelection(state) => {
            crate::dsl_cards::lower_replacement::run_delay_play_from_hand_selection_step(
                game, state, action_id, is_pass,
            );
        }
        ResumeFrame::DelayPlayFromUnionAfterSelection {
            inner,
            continuation,
            outer_conts,
        } => {
            run_resume(game, *inner, action_id, is_pass);
            crate::dsl_cards::lower_replacement::continue_delay_play_from_union_after_selection(
                game,
                continuation,
            );
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::AppFuseHostSelection(mut state) => {
            if is_pass {
                return;
            }
            let outer_conts = std::mem::take(&mut state.outer_conts);
            crate::effect_context::run_app_fuse_host_selection_step(game, state, action_id);
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::AppFuseResultSelection(mut state) => {
            if is_pass {
                return;
            }
            let outer_conts = std::mem::take(&mut state.outer_conts);
            crate::effect_context::run_app_fuse_result_selection_step(game, state, action_id);
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::ArtsDigivolveSelection(mut state) => {
            let outer_conts = std::mem::take(&mut state.outer_conts);
            game.run_arts_digivolve_selection_step(state, action_id, is_pass);
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::LinkOptionHostSelection(mut state) => {
            let outer_conts = std::mem::take(&mut state.outer_conts);
            game.run_link_option_host_selection_step(state, action_id, is_pass);
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::DigimonLinkHostSelection(state) => {
            game.run_digimon_link_host_selection_step(state, action_id);
        }
        ResumeFrame::PlayFromHandCostReductionPrompt(mut state) => {
            let outer_conts = std::mem::take(&mut state.outer_conts);
            game.run_play_from_hand_cost_reduction_prompt_step(state, is_pass);
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::InteractiveDigivolveCostReductionPrompt(mut state) => {
            let outer_conts = std::mem::take(&mut state.outer_conts);
            game.run_interactive_digivolve_cost_reduction_prompt_step(state, is_pass);
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::InteractiveOptionUseCostReductionPrompt(mut state) => {
            let outer_conts = std::mem::take(&mut state.outer_conts);
            game.run_interactive_option_use_cost_reduction_prompt_step(state, is_pass);
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::AllianceSelection(state) => {
            let crate::resume::CombatAllianceState {
                attacker,
                outer_conts,
            } = state;
            game.run_alliance_selection_step(attacker, action_id, is_pass);
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::BlockSelection(state) => {
            let crate::resume::CombatBlockState {
                attacker,
                defender_player,
                outer_conts,
            } = state;
            game.run_block_selection_step(attacker, defender_player, action_id, is_pass);
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::RaidSelection(state) => {
            let crate::resume::CombatRaidSelectionState {
                attacker,
                opponent_player,
                mode,
                outer_conts,
            } = state;
            game.run_raid_selection_step(attacker, opponent_player, mode, action_id, is_pass);
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::CounterSelection(state) => {
            let crate::resume::CombatCounterState {
                defender_player,
                valid_action_ids,
                candidates,
                outer_conts,
            } = state;
            game.run_counter_selection_step(
                defender_player,
                &valid_action_ids,
                &candidates,
                action_id,
                is_pass,
            );
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::CounterBlastDnaFieldMaterial(state) => {
            let crate::resume::CombatBlastDnaFieldMaterialState {
                defender,
                result_hand_index,
                source_card,
                previous_phase,
                outer_conts,
            } = state;
            game.run_counter_blast_dna_field_material_step(
                defender,
                result_hand_index,
                source_card,
                previous_phase,
                action_id,
            );
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::CounterBlastDnaHandMaterial(state) => {
            let crate::resume::CombatBlastDnaHandMaterialState {
                defender,
                result_hand_index,
                field_idx,
                outer_conts,
            } = state;
            game.run_counter_blast_dna_hand_material_step(
                defender,
                result_hand_index,
                field_idx,
                action_id,
            );
            run_outer_conts(game, outer_conts);
        }
        ResumeFrame::OverclockSelection(state) => {
            game.run_overclock_selection_step(state, action_id, is_pass);
        }
        ResumeFrame::PlayOrderSelection => {
            game.last_play_order_choice = Some(if action_id == crate::action::space::PLAY_FIRST {
                crate::selection::PlayOrder::First
            } else {
                crate::selection::PlayOrder::Second
            });
        }
        ResumeFrame::KeywordSaveSelection(state) => {
            run_keyword_save_selection_step(game, state, action_id, is_pass);
        }
        ResumeFrame::KeywordMaterialSaveTamerSelection(state) => {
            run_keyword_material_save_tamer_selection_step(game, state, action_id, is_pass);
        }
        ResumeFrame::KeywordScapegoatSelection(state) => {
            run_keyword_scapegoat_selection_step(game, state, action_id, is_pass);
        }
        ResumeFrame::KeywordMindLinkSelection(state) => {
            run_keyword_mind_link_selection_step(game, state, action_id, is_pass);
        }
        ResumeFrame::KeywordAscensionChoice(state) => {
            run_keyword_ascension_choice_step(game, state, action_id);
        }
        ResumeFrame::FamiliarTokenOnDeletionSelection(state) => {
            run_familiar_token_on_deletion_selection_step(game, state, action_id, is_pass);
        }
        ResumeFrame::LinkCardLeaveSelection(state) => {
            run_link_card_leave_selection_step(game, state, action_id, is_pass);
        }
        ResumeFrame::TrashLinkCardOfDigimonSelection(state) => {
            run_trash_link_card_of_digimon_selection_step(game, state, action_id, is_pass);
        }
        ResumeFrame::TrashOptionFromStackSelection(state) => {
            run_trash_option_from_stack_selection_step(game, state, action_id, is_pass);
        }
        ResumeFrame::PlayOrUseDualChoice(state) => {
            run_play_or_use_dual_choice_step(game, state, action_id, is_pass);
        }
        ResumeFrame::MayDnaPartnerSelection(state) => {
            run_may_dna_partner_selection_step(game, state, action_id, is_pass);
        }
        ResumeFrame::MayDnaResultSelection(state) => {
            run_may_dna_result_selection_step(game, state, action_id, is_pass);
        }
    }
}

fn field_handle_from_action(of_player: PlayerId, action_id: u16) -> PermanentHandle {
    let offset = action_id.saturating_sub(crate::action::space::ATTACK_START);
    PermanentHandle {
        player: of_player,
        index: (offset % crate::action::space::TARGETS_PER_ATTACKER) as u8,
    }
}

fn push_field_effect_target(
    game: &mut crate::game::Game,
    controller: PlayerId,
    source_card: crate::card_source::CardHandle,
    handle: PermanentHandle,
) {
    if let Some(perm) = game
        .player(handle.player)
        .battle_area
        .get(handle.index as usize)
    {
        let top = perm.top_card();
        let tid = top.card_id(&game.card_data).to_string();
        let tname = top.card_name(&game.card_data).to_string();
        crate::effect_context::selections::push_effect_target(
            game,
            controller,
            source_card,
            tid,
            tname,
        );
    }
}

fn count_capped_handle_for_action(
    game: &crate::game::Game,
    of_player: PlayerId,
    zone: CountCappedZone,
    action_id: u16,
) -> Option<crate::card_source::CardHandle> {
    match zone {
        CountCappedZone::Hand => {
            let idx = action_id.saturating_sub(crate::action::space::PLAY_HAND_START) as usize;
            game.player(of_player).hand.get(idx).map(|c| c.handle())
        }
        CountCappedZone::Trash => {
            let idx = action_id.saturating_sub(crate::action::space::TRASH_EFFECT_START) as usize;
            game.player(of_player).trash.get(idx).map(|c| c.handle())
        }
        CountCappedZone::Material(perm) => {
            let (_, range_start) =
                crate::effect_context::selections::material_zone_geometry(game, perm)?;
            let idx = action_id.saturating_sub(range_start) as usize;
            crate::effect_context::selections::material_zone_slice(game, perm)?
                .get(idx)
                .map(|c| c.handle())
        }
    }
}

fn count_capped_data_index_for_action(
    game: &crate::game::Game,
    of_player: PlayerId,
    zone: CountCappedZone,
    action_id: u16,
) -> Option<usize> {
    match zone {
        CountCappedZone::Hand => {
            let idx = action_id.saturating_sub(crate::action::space::PLAY_HAND_START) as usize;
            game.player(of_player).hand.get(idx).map(|c| c.data_index)
        }
        CountCappedZone::Trash => {
            let idx = action_id.saturating_sub(crate::action::space::TRASH_EFFECT_START) as usize;
            game.player(of_player).trash.get(idx).map(|c| c.data_index)
        }
        CountCappedZone::Material(perm) => {
            let (_, range_start) =
                crate::effect_context::selections::material_zone_geometry(game, perm)?;
            let idx = action_id.saturating_sub(range_start) as usize;
            crate::effect_context::selections::material_zone_slice(game, perm)?
                .get(idx)
                .map(|c| c.data_index)
        }
    }
}

fn count_capped_rejects_distinct(
    game: &crate::game::Game,
    mode: DistinctByMode,
    picked_data_indices: &[usize],
    candidate_data_index: usize,
) -> bool {
    let candidate = &game.card_data[candidate_data_index];
    picked_data_indices.iter().any(|&picked_idx| {
        let picked = &game.card_data[picked_idx];
        match mode {
            DistinctByMode::CardNumber => picked.card_id == candidate.card_id,
            DistinctByMode::Level => {
                matches!((picked.level, candidate.level), (Some(p), Some(c)) if p == c)
            }
            DistinctByMode::Name => picked.card_name == candidate.card_name,
        }
    })
}

fn park_non_dsl_count_capped_state(
    game: &mut crate::game::Game,
    state: crate::resume::NonDslCountCappedState,
) {
    let effective_min = state.min.max(if state.is_optional_zero { 0 } else { 1 });
    let picked = state.accum.len() as u8;
    game.current_phase = GamePhase::SelectBudgeted;
    game.pending_selection = Some(PendingSelection {
        zone_owner: None,
        kind: SelectionKind::CountCappedMultiSelect {
            min: effective_min,
            max: state.max,
            picked,
            distinct: state.distinct_by.is_some(),
        },
        selecting_player: state.prov.override_pin.unwrap_or(state.prov.controller),
        previous_phase: state.previous_phase,
        valid_action_ids: state.candidate_actions.clone(),
        is_optional: picked >= effective_min,
        prompt: state.prompt.clone(),
        effect_choices: None,
        source_card: state.prov.source_card,
        source_permanent: state.prov.source_permanent,
        source_kind: state.prov.source_kind,
        callback: Box::new(|_, _| {
            panic!("non-DSL count-capped selection must resolve through ResumeFrame")
        }),
        on_decline: Some(Box::new(|_| {
            panic!("non-DSL count-capped selection must resolve through ResumeFrame")
        })),
    });
    game.pending_selection_resume = Some(crate::resume::ResumeStack {
        frames: vec![crate::resume::ResumeFrame::NonDslCountCappedStep(state)],
    });
}

fn run_non_dsl_count_capped_step(
    game: &mut crate::game::Game,
    mut state: crate::resume::NonDslCountCappedState,
    action_id: u16,
    is_pass: bool,
) {
    if is_pass {
        finish_non_dsl_count_capped(game, state);
        return;
    }
    if !state.candidate_actions.contains(&action_id) {
        return;
    }
    let Some(card) = count_capped_handle_for_action(game, state.of_player, state.zone, action_id)
    else {
        return;
    };
    state.accum.push(card);
    if state.accum.len() == state.max as usize {
        finish_non_dsl_count_capped(game, state);
        return;
    }

    let picked_data_indices: Vec<usize> = state
        .accum
        .iter()
        .filter_map(|&picked| {
            state
                .candidate_actions
                .iter()
                .copied()
                .find(|&candidate| {
                    count_capped_handle_for_action(game, state.of_player, state.zone, candidate)
                        == Some(picked)
                })
                .and_then(|candidate| {
                    count_capped_data_index_for_action(game, state.of_player, state.zone, candidate)
                })
        })
        .collect();

    state.candidate_actions = state
        .candidate_actions
        .into_iter()
        .filter(|&candidate| candidate != action_id)
        .filter(|&candidate| {
            let Some(mode) = state.distinct_by else {
                return true;
            };
            let Some(candidate_idx) =
                count_capped_data_index_for_action(game, state.of_player, state.zone, candidate)
            else {
                return false;
            };
            !count_capped_rejects_distinct(game, mode, &picked_data_indices, candidate_idx)
        })
        .collect();

    if state.candidate_actions.is_empty() {
        finish_non_dsl_count_capped(game, state);
    } else {
        park_non_dsl_count_capped_state(game, state);
    }
}

fn finish_non_dsl_count_capped(
    game: &mut crate::game::Game,
    state: crate::resume::NonDslCountCappedState,
) {
    let refs: Vec<crate::events::EventCardRef> = state
        .accum
        .iter()
        .filter_map(|h| {
            game.card_data_for_handle(*h)
                .map(|cd| crate::events::EventCardRef {
                    card_id: cd.card_id.clone(),
                    card_name: cd.card_name.clone(),
                })
        })
        .collect();
    crate::effect_context::selections::push_effect_target_multi(
        game,
        state.prov.controller,
        state.prov.source_card,
        refs,
    );

    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    match state.terminal {
        crate::resume::NonDslCountCappedTerminal::KeywordFragment { subject } => {
            for handle in &state.accum {
                let _ = ctx.trash_card_source(subject, *handle);
            }
            ctx.cancel_leave();
        }
        crate::resume::NonDslCountCappedTerminal::KeywordPartition { subject } => {
            let mut extracted: Vec<crate::card_source::CardHandle> = Vec::new();
            for handle in &state.accum {
                let removed = {
                    let Some(permanent) = ctx
                        .game
                        .player_mut(subject.player)
                        .battle_area
                        .get_mut(subject.index as usize)
                    else {
                        continue;
                    };
                    let Some(pos) = permanent
                        .card_sources
                        .iter()
                        .position(|c| c.handle() == *handle)
                    else {
                        continue;
                    };
                    permanent.card_sources.remove(pos)
                };
                let owner = removed.owner;
                ctx.game.player_mut(owner).trash.push(removed);
                extracted.push(*handle);
            }
            // Sources left the stack without the trash observer (partition
            // trashes are intentionally observer-silent) — still refresh
            // materialized declaratives so grants sourced from the departed
            // cards stop applying before the follow-up plays below (same
            // contract as `fire_digivolution_card_trashed`).
            ctx.game.tick_declarative_effects();
            let mut iter = extracted.into_iter();
            if let Some(first) = iter.next() {
                let second = iter.next();
                let source_card = ctx.source_card;
                let player = ctx.player;
                let _ = ctx.play_from_trash_free_unsuspended(first);
                if let Some(second) = second {
                    ctx.game
                        .queue_partition_second_play(player, source_card, second);
                }
            }
        }
        crate::resume::NonDslCountCappedTerminal::KeywordMaterialSave { tamer } => {
            for source in &state.accum {
                ctx.place_card_under_permanent_bottom(*source, tamer, false);
            }
        }
        crate::resume::NonDslCountCappedTerminal::TrashOpponentHandToCount {
            opponent,
            bind_count_as,
        } => {
            let mut trashed = 0usize;
            for card_handle in &state.accum {
                let idx = ctx
                    .game
                    .player(opponent)
                    .hand
                    .iter()
                    .position(|c| c.handle() == *card_handle);
                if let Some(i) = idx {
                    ctx.trash_from_hand_by_index(opponent, i);
                    trashed += 1;
                }
            }
            // Publish the count actually trashed into the freshness channel so
            // `run_outer_conts` merges it into the resolving DSL tail's bindings
            // (consumed by a downstream `binding_value`/`floor_div`).
            // G-DSL-TRASH-COUNT-RESULT-BINDING.
            if let Some(name) = &bind_count_as {
                let fresh = ctx
                    .game
                    .dsl_resolved_tail_bindings
                    .get_or_insert_with(crate::dsl_cards::bindings::Bindings::new);
                fresh.insert_literal(name, trashed as i64);
            }
        }
        crate::resume::NonDslCountCappedTerminal::Assembly {
            player,
            target_card,
            params,
            elements,
            element_idx,
            picked_so_far,
        } => {
            crate::game_actions::continue_assembly_after_material_picks(
                ctx.game,
                player,
                target_card,
                params,
                elements,
                element_idx,
                picked_so_far,
                state.accum.clone(),
            );
        }
    }
    run_outer_conts(ctx.game, state.outer_conts);
}

fn run_keyword_save_selection_step(
    game: &mut crate::game::Game,
    state: crate::resume::KeywordSaveSelectionState,
    action_id: u16,
    is_pass: bool,
) {
    if is_pass {
        return;
    }
    let tamer = field_handle_from_action(state.owner, action_id);
    push_field_effect_target(game, state.prov.controller, state.prov.source_card, tamer);
    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    ctx.place_card_under_permanent_bottom(state.self_card, tamer, false);
    run_outer_conts(ctx.game, state.outer_conts);
}

fn run_keyword_material_save_tamer_selection_step(
    game: &mut crate::game::Game,
    state: crate::resume::KeywordMaterialSaveTamerSelectionState,
    action_id: u16,
    is_pass: bool,
) {
    if is_pass {
        return;
    }
    let tamer = field_handle_from_action(state.owner, action_id);
    push_field_effect_target(game, state.prov.controller, state.prov.source_card, tamer);
    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    let eligible_sources = state.eligible_sources.clone();
    ctx.select_count_capped_multi(
        state.owner,
        CountCappedZone::Trash,
        state.max,
        "select Material Save sources to place under Tamer",
        true,
        None,
        move |_g, card| eligible_sources.contains(&card.handle()),
        move |ctx, picks| {
            for source in picks {
                ctx.place_card_under_permanent_bottom(source, tamer, false);
            }
        },
    );
    if let Some(pending) = ctx.game.pending_selection.as_ref() {
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::NonDslCountCappedStep(
                crate::resume::NonDslCountCappedState {
                    prov: state.prov,
                    of_player: state.owner,
                    zone: CountCappedZone::Trash,
                    min: 0,
                    max: state.max,
                    is_optional_zero: true,
                    distinct_by: None,
                    candidate_actions: pending.valid_action_ids.clone(),
                    accum: Vec::new(),
                    prompt: pending.prompt.clone(),
                    previous_phase: pending.previous_phase,
                    terminal: crate::resume::NonDslCountCappedTerminal::KeywordMaterialSave {
                        tamer,
                    },
                    outer_conts: Vec::new(),
                },
            )],
        });
    }
    run_outer_conts(ctx.game, state.outer_conts);
}

fn run_keyword_scapegoat_selection_step(
    game: &mut crate::game::Game,
    state: crate::resume::KeywordScapegoatSelectionState,
    action_id: u16,
    is_pass: bool,
) {
    if is_pass {
        return;
    }
    let picked = field_handle_from_action(state.owner, action_id);
    push_field_effect_target(game, state.prov.controller, state.prov.source_card, picked);
    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    if picked != state.self_perm {
        ctx.substitute_replacement(crate::replacement::ReplacementSubject::Permanent(picked));
    }
    run_outer_conts(ctx.game, state.outer_conts);
}

fn run_keyword_mind_link_selection_step(
    game: &mut crate::game::Game,
    state: crate::resume::KeywordMindLinkSelectionState,
    action_id: u16,
    is_pass: bool,
) {
    if is_pass {
        return;
    }
    let picked = field_handle_from_action(state.owner, action_id);
    push_field_effect_target(game, state.prov.controller, state.prov.source_card, picked);
    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    ctx.attach_tamer_to_digimon(state.tamer, picked);
    run_outer_conts(ctx.game, state.outer_conts);
}

fn run_keyword_ascension_choice_step(
    game: &mut crate::game::Game,
    state: crate::resume::KeywordAscensionChoiceState,
    action_id: u16,
) {
    let choice = action_id.saturating_sub(crate::action::space::HAND_EFFECT_START) as usize;
    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    if choice == 0 {
        if let Some(idx) = ctx
            .game
            .player(state.owner)
            .trash
            .iter()
            .position(|c| c.handle() == state.self_card)
        {
            ctx.place_on_security(
                state.owner,
                crate::enums::CardSourceRef::Trash(state.owner, idx),
                crate::enums::StackPosition::Top,
                false,
            );
        }
    }
    run_outer_conts(ctx.game, state.outer_conts);
}

fn run_familiar_token_on_deletion_selection_step(
    game: &mut crate::game::Game,
    state: crate::resume::FamiliarTokenOnDeletionSelectionState,
    action_id: u16,
    is_pass: bool,
) {
    if is_pass {
        return;
    }
    let target = field_handle_from_action(state.target_player, action_id);
    push_field_effect_target(game, state.prov.controller, state.prov.source_card, target);
    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    ctx.add_dp_modifier(target, -3000, crate::enums::Expiry::EndOfTurn);
    run_outer_conts(ctx.game, state.outer_conts);
}

fn run_link_card_leave_selection_step(
    game: &mut crate::game::Game,
    state: crate::resume::LinkCardLeaveSelectionState,
    action_id: u16,
    is_pass: bool,
) {
    if is_pass {
        return;
    }
    let choice = action_id.saturating_sub(crate::action::space::HAND_EFFECT_START) as usize;
    let Some(card) = state.cards.get(choice).copied() else {
        return;
    };
    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    let paid = match state.mode {
        crate::resume::LinkCardLeaveMode::TrashAndCancel => {
            ctx.game.trash_specific_link_card(state.host, card)
        }
        crate::resume::LinkCardLeaveMode::PlaceAsBottomSourceAndCancel => ctx
            .game
            .place_specific_link_card_as_bottom_source(state.host, card),
        crate::resume::LinkCardLeaveMode::TrashDigivolutionOptionAndCancel => {
            ctx.game.trash_specific_source_card(state.host, card)
        }
    };
    if paid {
        ctx.cancel_leave();
    }
    run_outer_conts(ctx.game, state.outer_conts);
}

/// Resume the SECOND selection of the `trash_link_card_of_own_digimon` cost:
/// trash the chosen link card of `host`, then run the cost-gated tail only if a
/// card was trashed. On PASS (optional) nothing is trashed and the tail is
/// skipped. Mirrors `install_link_card_trash_second_selection`'s closure.
/// `G-DSL-LINK-TRASH-AS-COST`.
fn run_trash_link_card_of_digimon_selection_step(
    game: &mut crate::game::Game,
    state: crate::resume::TrashLinkCardOfDigimonSelectionState,
    action_id: u16,
    is_pass: bool,
) {
    if is_pass {
        // Declined link-card pick: no trash, tail skipped; still run outer conts.
        run_outer_conts(game, state.outer_conts);
        return;
    }
    let choice = action_id.saturating_sub(crate::action::space::HAND_EFFECT_START) as usize;
    let Some(card) = state.cards.get(choice).copied() else {
        run_outer_conts(game, state.outer_conts);
        return;
    };
    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    if ctx.game.trash_specific_link_card(state.host, card) {
        let mut b = state.bindings.clone();
        run_tail_preserving_trigger_context(
            &mut ctx,
            state.trigger_context,
            &state.tail,
            &mut b,
            &state.runtime,
        );
    }
    run_outer_conts(ctx.game, state.outer_conts);
}

fn run_play_or_use_dual_choice_step(
    game: &mut crate::game::Game,
    state: crate::resume::PlayOrUseDualChoiceState,
    action_id: u16,
    is_pass: bool,
) {
    if is_pass {
        return;
    }
    let choice = action_id.saturating_sub(crate::action::space::HAND_EFFECT_START) as usize;
    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    if ctx
        .game
        .player(state.player)
        .hand
        .get(state.hand_index)
        .is_none()
    {
        run_outer_conts(ctx.game, state.outer_conts);
        return;
    }
    match choice {
        0 => {
            let _ = ctx.play_from_hand_with_cost(state.player, state.hand_index, state.cost_delta);
        }
        _ => {
            let _ = ctx.use_option_from_hand_with_cost(
                state.player,
                state.hand_index,
                state.cost_delta,
            );
        }
    }
    run_outer_conts(ctx.game, state.outer_conts);
}

fn run_may_dna_partner_selection_step(
    game: &mut crate::game::Game,
    state: crate::resume::MayDnaPartnerSelectionState,
    action_id: u16,
    is_pass: bool,
) {
    if is_pass {
        return;
    }
    let partner = field_handle_from_action(state.controller, action_id);
    push_field_effect_target(game, state.prov.controller, state.prov.source_card, partner);

    let valid_action_ids: Vec<u16> = state
        .target_candidate_actions
        .iter()
        .copied()
        .filter(|&candidate| {
            let hand_idx = candidate.saturating_sub(crate::action::space::PLAY_HAND_START) as usize;
            if game.player(state.controller).hand.get(hand_idx).is_none() {
                return false;
            }
            state.ignore_requirements
                || crate::effect_context::dna_pair_can_reach_hand_card(
                    game,
                    state.controller,
                    state.anchor,
                    partner,
                    hand_idx,
                )
        })
        .collect();

    if valid_action_ids.is_empty() {
        run_outer_conts(game, state.outer_conts);
        return;
    }

    let previous_phase = game.current_phase;
    game.current_phase = GamePhase::SelectHand;
    game.pending_selection = Some(PendingSelection {
        zone_owner: Some(state.controller),
        kind: SelectionKind::Hand,
        selecting_player: state.prov.override_pin.unwrap_or(state.prov.controller),
        previous_phase,
        valid_action_ids: valid_action_ids.clone(),
        is_optional: state.optional,
        prompt: state.target_prompt.clone(),
        effect_choices: None,
        source_card: state.prov.source_card,
        source_permanent: state.prov.source_permanent,
        source_kind: state.prov.source_kind,
        callback: Box::new(|_, _| {
            panic!("may-DNA result selection must resolve through ResumeFrame")
        }),
        on_decline: None,
    });
    game.pending_selection_resume = Some(crate::resume::ResumeStack {
        frames: vec![crate::resume::ResumeFrame::MayDnaResultSelection(
            crate::resume::MayDnaResultSelectionState {
                prov: state.prov,
                controller: state.controller,
                anchor: state.anchor,
                partner,
                cost: state.cost,
                ignore_requirements: state.ignore_requirements,
                valid_action_ids,
                outer_conts: state.outer_conts,
            },
        )],
    });
}

fn run_may_dna_result_selection_step(
    game: &mut crate::game::Game,
    state: crate::resume::MayDnaResultSelectionState,
    action_id: u16,
    is_pass: bool,
) {
    if is_pass {
        return;
    }
    if !state.valid_action_ids.contains(&action_id) {
        return;
    }
    let hand_idx = action_id.saturating_sub(crate::action::space::PLAY_HAND_START) as usize;
    if let Some(card) = game.player(state.controller).hand.get(hand_idx) {
        let tid = card.card_id(&game.card_data).to_string();
        let tname = card.card_name(&game.card_data).to_string();
        crate::effect_context::selections::push_effect_target(
            game,
            state.prov.controller,
            state.prov.source_card,
            tid,
            tname,
        );
    }
    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    let Some(card) = ctx
        .game
        .player(state.controller)
        .hand
        .get(hand_idx)
        .map(|c| c.handle())
    else {
        run_outer_conts(ctx.game, state.outer_conts);
        return;
    };
    let charge = if state.ignore_requirements {
        state.cost as i32
    } else {
        crate::effect_context::dna_pair_cost_for_hand_card(
            ctx.game,
            state.controller,
            state.anchor,
            state.partner,
            hand_idx,
        )
        .unwrap_or(state.cost as i32)
    };
    ctx.effect_initiated_dna_digivolve(
        state.anchor,
        state.partner,
        card,
        charge,
        state.ignore_requirements,
    );
    run_outer_conts(ctx.game, state.outer_conts);
}

/// Executor for a `use_option_from_hand` selection (mirrors
/// `install_use_option_from_hand`'s accept/decline closures exactly).
///
/// ACCEPT: decode the hand index, emit the effect-target (as `select_hand`'s
/// wrapper does), play the option under the authored trigger context, then —
/// unless the play was Invalid — compose the tail via `drain_or_rewrap_pending_tail`
/// (which threads it onto any selection the option's effect parked, rather than
/// running it inline). DECLINE (optional only): run the SAME tail (continue-tail;
/// no `dsl_clause_aborted`). `outer_conts` run after, via `run_outer_conts`.
fn run_use_option_from_hand_step(
    game: &mut crate::game::Game,
    state: crate::resume::UseOptionFromHandState,
    action_id: u16,
    is_pass: bool,
) {
    let crate::resume::UseOptionFromHandState {
        prov,
        of_player,
        tail,
        bindings,
        runtime,
        trigger_context,
        outer_conts,
        optional: _optional,
    } = state;

    if is_pass {
        // Optional decline (reachable only when optional — resolve_generic_selection
        // rejects PASS otherwise): run the SAME tail. Mirrors the installer's
        // on_decline: set the authored trigger context, drain_or_rewrap, restore.
        let previous = game.current_trigger_context.clone();
        game.current_trigger_context = trigger_context.clone();
        drain_or_rewrap_pending_tail(
            game,
            prov.source_card,
            prov.source_permanent,
            prov.controller,
            (*tail).clone(),
            bindings,
            runtime,
            trigger_context,
        );
        game.current_trigger_context = previous;
        run_outer_conts(game, outer_conts);
        return;
    }

    let idx = action_id.saturating_sub(crate::action::space::PLAY_HAND_START) as usize;
    // Target tracking (mirrors ctx.select_hand's wrapper).
    if let Some(card) = game.player(of_player).hand.get(idx) {
        let tid = card.card_id(&game.card_data).to_string();
        let tname = card.card_name(&game.card_data).to_string();
        crate::effect_context::selections::push_effect_target(
            game,
            prov.controller,
            prov.source_card,
            tid,
            tname,
        );
    }
    // Play the option under the authored trigger context, then restore.
    let previous = game.current_trigger_context.clone();
    game.current_trigger_context = trigger_context.clone();
    let result = game.use_option_from_hand_without_paying_cost(of_player, idx);
    game.current_trigger_context = previous;
    if matches!(result, crate::selection::OptionPlayResult::Invalid) {
        // Parity with the closure's early-return: no tail, no outer_conts.
        return;
    }
    // The option play may have parked a nested selection; drain_or_rewrap
    // composes the tail onto it (else runs inline). tail_context = the authored
    // trigger context.
    drain_or_rewrap_pending_tail(
        game,
        prov.source_card,
        prov.source_permanent,
        prov.controller,
        (*tail).clone(),
        bindings,
        runtime,
        trigger_context,
    );
    run_outer_conts(game, outer_conts);
}

/// Data terminal for a multi-bucket reveal selection: bind each bucket's picked
/// list by its `bind_as`, then run the inner tail. Mirrors `select_reveal_buckets`'
/// final callback (no EffectTarget push).
fn run_reveal_bucket_terminal(
    game: &mut crate::game::Game,
    state: crate::resume::RevealBucketState,
) {
    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    let mut b = state.bindings;
    for (name, cards) in state.picked_buckets {
        b.insert_card_list(&name, cards);
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

/// Drive the multi-bucket reveal state machine: advance past `max==0`/no-candidate
/// buckets, park the first bucket that has candidates, or run the terminal when
/// all buckets are done. Mirrors `install_reveal_bucket_step` exactly, as data.
pub(crate) fn install_reveal_bucket_resume_step(
    game: &mut crate::game::Game,
    mut state: crate::resume::RevealBucketState,
) {
    use crate::action::space::{MAX_REVEALED, SEL_REVEAL_START};
    use crate::selection::{PendingSelection, SelectionKind};
    loop {
        let Some(bucket) = state.buckets.get(state.bucket_index).cloned() else {
            // All buckets resolved → terminal.
            run_reveal_bucket_terminal(game, state);
            return;
        };
        if bucket.max == 0 {
            state.picked_buckets.push((bucket.bind_as, Vec::new()));
            state.bucket_index += 1;
            state.current_bucket_picks = Vec::new();
            continue;
        }
        let already_chosen: std::collections::HashSet<crate::card_source::CardHandle> = state
            .picked_buckets
            .iter()
            .flat_map(|(_, cards)| cards.iter().copied())
            .chain(state.current_bucket_picks.iter().copied())
            .collect();
        let cap = game.revealed_cards.len().min(MAX_REVEALED);
        let mut valid_action_ids = Vec::new();
        for idx in 0..cap {
            let handle = game.revealed_cards[idx].handle();
            if !bucket.candidates.contains(&handle) {
                continue;
            }
            if state.current_bucket_picks.contains(&handle) {
                continue;
            }
            if state.no_duplicate_cards && already_chosen.contains(&handle) {
                continue;
            }
            valid_action_ids.push(SEL_REVEAL_START + idx as u16);
        }
        if valid_action_ids.is_empty() {
            let current = std::mem::take(&mut state.current_bucket_picks);
            state.picked_buckets.push((bucket.bind_as, current));
            state.bucket_index += 1;
            continue;
        }
        let picked = state.current_bucket_picks.len() as u8;
        let is_optional = picked >= bucket.min;
        game.current_phase = crate::enums::GamePhase::SelectReveal;
        game.pending_selection = Some(PendingSelection {
            zone_owner: None,
            kind: SelectionKind::RevealBucket {
                bucket_index: state.bucket_index as u8,
                min: bucket.min,
                max: bucket.max,
                picked,
            },
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
            frames: vec![crate::resume::ResumeFrame::RevealBucketStep(state)],
        });
        return;
    }
}

/// Executor for one reveal-bucket pick (or PASS). Mirrors the closure: a pick
/// reaching the bucket `max` (or PASS) completes the bucket and advances; a
/// sub-max pick re-parks the same bucket.
fn run_reveal_bucket_step(
    game: &mut crate::game::Game,
    mut state: crate::resume::RevealBucketState,
    action_id: u16,
    is_pass: bool,
) {
    let Some(bucket) = state.buckets.get(state.bucket_index).cloned() else {
        run_reveal_bucket_terminal(game, state);
        return;
    };
    if is_pass {
        let current = std::mem::take(&mut state.current_bucket_picks);
        state.picked_buckets.push((bucket.bind_as, current));
        state.bucket_index += 1;
        install_reveal_bucket_resume_step(game, state);
        return;
    }
    let reveal_index = action_id.saturating_sub(crate::action::space::SEL_REVEAL_START) as usize;
    let Some(card) = game.revealed_cards.get(reveal_index) else {
        return; // stale/invalid pick (defensive)
    };
    state.current_bucket_picks.push(card.handle());
    if state.current_bucket_picks.len() >= bucket.max as usize {
        let current = std::mem::take(&mut state.current_bucket_picks);
        state.picked_buckets.push((bucket.bind_as, current));
        state.bucket_index += 1;
    }
    // Re-enter the state machine: advance (bucket completed) or re-park the same
    // bucket with the updated current picks.
    install_reveal_bucket_resume_step(game, state);
}

/// Data terminal for a battle-area permanent multi-pick: bind the picked
/// permanents as a permanent list, then run the inner tail. (Mirrors the
/// closure `final_callback` — which does NOT push EffectTargets here.)
fn run_count_capped_permanent_terminal(
    game: &mut crate::game::Game,
    state: crate::resume::CountCappedPermanentsState,
) {
    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    let mut b = state.bindings;
    if let Some(name) = &state.bind_as {
        b.insert_permanent_list(name, state.accum);
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

/// Install (or re-park) a battle-area permanent multi-pick step over the carried
/// candidate snapshot. Mirrors `install_count_capped_permanent_step` (OppField/
/// OwnField kind, PASS gating at/above the effective floor).
pub(crate) fn install_count_capped_permanent_resume_step(
    game: &mut crate::game::Game,
    state: crate::resume::CountCappedPermanentsState,
) {
    use crate::selection::{PendingSelection, SelectionKind};
    let picked = state.accum.len() as u8;
    let effective_min = state.min.max(if state.optional_zero { 0 } else { 1 });
    let is_optional = picked >= effective_min;
    let valid_action_ids: Vec<u16> = state.candidates.iter().map(|(a, _)| *a).collect();
    game.current_phase = crate::enums::GamePhase::SelectBudgeted;
    game.pending_selection = Some(PendingSelection {
        zone_owner: None,
        kind: if state.target_is_opponent {
            SelectionKind::OppField
        } else {
            SelectionKind::OwnField
        },
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
        frames: vec![crate::resume::ResumeFrame::CountCappedPermanentsStep(state)],
    });
}

/// Executor for one battle-area permanent pick (or PASS). Mirrors
/// `install_count_capped_permanent_step`: decode against the snapshot, append,
/// shrink the snapshot (remove the picked action), then terminal (max / empty)
/// or re-park; PASS commits.
fn run_count_capped_permanent_step(
    game: &mut crate::game::Game,
    mut state: crate::resume::CountCappedPermanentsState,
    action_id: u16,
    is_pass: bool,
) {
    if is_pass {
        run_count_capped_permanent_terminal(game, state);
        return;
    }
    let Some((_, handle)) = state
        .candidates
        .iter()
        .find(|(candidate_action, _)| *candidate_action == action_id)
        .copied()
    else {
        return; // stale/invalid pick (defensive)
    };
    state.accum.push(handle);
    if state.accum.len() == state.max as usize {
        run_count_capped_permanent_terminal(game, state);
        return;
    }
    state.candidates.retain(|(a, _)| *a != action_id);
    if state.candidates.is_empty() {
        run_count_capped_permanent_terminal(game, state);
        return;
    }
    install_count_capped_permanent_resume_step(game, state);
}

// ─────────────────────────────────────────────────────────────────────────────
// delete_one_per_opponent_color — per-color mandatory pick + batch delete
// (G-DSL-DELETE-ONE-PER-DISTINCT-OPPONENT-COLOR, EX9-074 Kimeramon Branch B).
// ─────────────────────────────────────────────────────────────────────────────

/// The fixed 7-color game order Branch B iterates (DCGO iterates
/// `DataBase.CardColorNameDictionary.Values`; the concrete order only affects the
/// per-color prompt sequence, never which Digimon end up deleted).
const ALL_CARD_COLORS: [crate::enums::CardColor; 7] = [
    crate::enums::CardColor::Red,
    crate::enums::CardColor::Blue,
    crate::enums::CardColor::Yellow,
    crate::enums::CardColor::Green,
    crate::enums::CardColor::White,
    crate::enums::CardColor::Black,
    crate::enums::CardColor::Purple,
];

/// Human name for a color, substituted into the per-color prompt (`{color}`).
fn color_name(c: crate::enums::CardColor) -> &'static str {
    use crate::enums::CardColor::*;
    match c {
        Red => "Red",
        Blue => "Blue",
        Yellow => "Yellow",
        Green => "Green",
        White => "White",
        Black => "Black",
        Purple => "Purple",
    }
}

/// Re-derive the current color's candidates `(action_id, handle)`: opponent
/// Digimon whose synth-identity colors CONTAIN `color`, matching `filter` (if
/// any), excluding any permanent already in `picked`. Data-pure (read borrow
/// released before returning), so the frame stays `Clone` (rule 28).
fn per_color_delete_candidates(
    game: &crate::game::Game,
    state: &crate::resume::PerColorDeleteState,
    color: crate::enums::CardColor,
) -> Vec<(u16, PermanentHandle)> {
    use crate::action::space::encode_attack;
    let read = EffectReadContext::new_with_source_kind(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
    );
    let mut out = Vec::new();
    for index in 0..game.player(state.opponent).battle_area.len() {
        let handle = PermanentHandle {
            player: state.opponent,
            index: index as u8,
        };
        if state.picked.contains(&handle) {
            continue;
        }
        let perm = &game.player(state.opponent).battle_area[index];
        // Digimon-only (DCGO `IsPermanentExistsOnOpponentBattleAreaDigimon`).
        if !perm.is_digimon(&game.card_data) {
            continue;
        }
        // Synth-aware color read (ChangeColor overlays respected) —
        // `TopCard.CardColors.Contains(color)`.
        let colors = perm
            .synth_identity(&game.card_data, &game.modifiers, handle)
            .colors;
        if !colors.contains(&color) {
            continue;
        }
        // <Progress> / immunity: a Digimon the effect can't affect is no target.
        if game.progress_excludes(handle, Some(state.prov.controller)) {
            continue;
        }
        // Extra author restriction (default: none).
        if let Some(filter) = &state.filter {
            if !eval_predicate_with_bindings(
                filter,
                &read,
                PredicateSubject::Permanent(handle),
                Some(&state.filter_bindings),
            ) {
                continue;
            }
        }
        out.push((encode_attack(state.opponent as u16, index as u16), handle));
    }
    out
}

/// Advance `state.color_index` to the next color with ≥1 legal candidate,
/// filling `state.candidates`. Returns `true` if a pickable color was found
/// (caller parks the mandatory pick); `false` if the color list is exhausted
/// (caller runs the terminal). Colors with no legal target are skipped
/// (DCGO's `if (HasMatchConditionPermanent(...))` gate).
fn per_color_delete_advance(
    game: &crate::game::Game,
    state: &mut crate::resume::PerColorDeleteState,
) -> bool {
    while state.color_index < state.colors.len() {
        let color = state.colors[state.color_index];
        let candidates = per_color_delete_candidates(game, state, color);
        if !candidates.is_empty() {
            state.candidates = candidates;
            return true;
        }
        state.color_index += 1;
    }
    false
}

/// Terminal: batch-delete every accumulated pick as a single unit (DCGO
/// `DestroyPermanentsClass(permanentToDelete).Destroy()`), then run the tail.
/// A granted/effect-driven deletion is the controller's own effect.
fn run_per_color_delete_terminal(
    game: &mut crate::game::Game,
    state: crate::resume::PerColorDeleteState,
) {
    if !state.picked.is_empty() {
        // Attribute the deletion to the controller's effect so the cause is the
        // opponent's-Digimon-deleted-by-your-effect one DCGO uses (`OpponentEffect`
        // from the target's perspective). Pin `effect_source_player` = controller
        // for the batch (the resume terminal runs outside the queued-effect scope
        // that normally sets it), then infer the cause exactly like
        // `delete_permanent_with_effects`.
        let prev_source = game.effect_source_player;
        game.effect_source_player = Some(state.prov.controller);
        let cause = game.infer_deletion_cause(state.picked[0]);
        game.delete_permanents_batch(state.picked.clone(), cause);
        game.effect_source_player = prev_source;
    }
    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    let mut b = state.bindings;
    run_tail_preserving_trigger_context(
        &mut ctx,
        state.trigger_context,
        &state.inner_tail,
        &mut b,
        &state.runtime,
    );
    run_outer_conts(ctx.game, state.outer_conts);
}

/// Park (or re-park) the current color's MANDATORY OppField pick.
fn install_per_color_delete_resume_step(
    game: &mut crate::game::Game,
    state: crate::resume::PerColorDeleteState,
) {
    let valid_action_ids: Vec<u16> = state.candidates.iter().map(|(a, _)| *a).collect();
    let color = state.colors[state.color_index];
    let prompt = state.prompt.replace("{color}", color_name(color));
    game.current_phase = GamePhase::SelectTarget;
    game.pending_selection = Some(PendingSelection {
        zone_owner: None,
        kind: SelectionKind::OppField,
        selecting_player: state.selecting_player,
        previous_phase: state.previous_phase,
        valid_action_ids,
        is_optional: false, // mandatory — DCGO `canNoSelect: false`
        prompt,
        effect_choices: None,
        source_card: state.prov.source_card,
        source_permanent: state.prov.source_permanent,
        source_kind: state.prov.source_kind,
        callback: Box::new(|_g, _a| {}),
        on_decline: None,
    });
    game.pending_selection_resume = Some(crate::resume::ResumeStack {
        frames: vec![crate::resume::ResumeFrame::PerColorDeleteStep(state)],
    });
}

/// Entry point for `delete_one_per_opponent_color`: build the initial
/// per-color-delete state and either park the FIRST color's mandatory pick or —
/// if no opponent color has a legal target — run the terminal immediately
/// (batch-delete an empty list = no-op, then the tail). See the frame docs on
/// `crate::resume::PerColorDeleteState` for the full flow.
fn install_delete_one_per_opponent_color(
    ctx: &mut EffectContext<'_>,
    filter: Option<CompiledPredicate>,
    prompt: Option<String>,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let opponent = ctx.game.next_clockwise(ctx.player);
    let mut state = crate::resume::PerColorDeleteState {
        prov: crate::resume::ResumeProvenance {
            source_card: ctx.source_card,
            source_permanent: ctx.source_permanent,
            source_kind: ctx.source_kind,
            controller: ctx.player,
            override_pin: ctx.override_selecting_player(),
        },
        opponent,
        selecting_player: ctx.override_selecting_player().unwrap_or(ctx.player),
        previous_phase: ctx.game.current_phase,
        colors: ALL_CARD_COLORS.to_vec(),
        color_index: 0,
        filter,
        filter_bindings: bindings.clone(),
        picked: Vec::new(),
        candidates: Vec::new(),
        prompt: prompt.unwrap_or_else(|| "Select 1 {color} Digimon to delete".to_string()),
        inner_tail: Arc::new(tail),
        bindings,
        runtime,
        trigger_context: ctx.game.current_trigger_context.clone(),
        outer_conts: Vec::new(),
    };
    if per_color_delete_advance(ctx.game, &mut state) {
        install_per_color_delete_resume_step(ctx.game, state);
    } else {
        run_per_color_delete_terminal(ctx.game, state);
    }
}

/// Executor for one per-color pick. Decode the picked handle against the current
/// color's snapshot, append to `picked`, advance to the next color-with-a-target,
/// then re-park or run the terminal (batch-delete + tail).
fn run_per_color_delete_step(
    game: &mut crate::game::Game,
    mut state: crate::resume::PerColorDeleteState,
    action_id: u16,
    is_pass: bool,
) {
    if is_pass {
        // Mandatory pick — PASS is rejected upstream (is_optional=false), but be
        // defensive: treat an unexpected PASS as "no more to do" and finalize.
        run_per_color_delete_terminal(game, state);
        return;
    }
    let Some((_, handle)) = state
        .candidates
        .iter()
        .find(|(candidate_action, _)| *candidate_action == action_id)
        .copied()
    else {
        return; // stale/invalid pick (defensive)
    };
    state.picked.push(handle);
    // Move past the color we just resolved and find the next pickable color.
    state.color_index += 1;
    if per_color_delete_advance(game, &mut state) {
        install_per_color_delete_resume_step(game, state);
    } else {
        run_per_color_delete_terminal(game, state);
    }
}

/// Re-derive the source-multi candidates `(action_id, source_ref)` for a
/// `SourceMultiState`, mirroring `source_multi_candidates` + the DSL installers'
/// filter (target restriction + `target_resolution_failed` + the
/// `CompiledPredicate` on `Source`/`Card` per `eval_on_card`). Data-pure.
pub(crate) fn source_multi_candidates_data(
    game: &crate::game::Game,
    state: &crate::resume::SourceMultiState,
) -> Vec<(u16, crate::selection::SourceSelectionRef)> {
    use crate::action::space::encode_source_select;
    use crate::selection::SourceSelectionRef;
    if state.target_resolution_failed {
        return Vec::new();
    }
    // Phase 1: collect (field, source, card) tuples (releases battle_area borrow
    // before the predicate read context borrows the game).
    let mut raw: Vec<(u8, u8, crate::card_source::CardHandle)> = Vec::new();
    for field_index in 0..game.player(state.of_player).battle_area.len() {
        let n = game.player(state.of_player).battle_area[field_index]
            .card_sources
            .len();
        if n <= 1 {
            continue;
        }
        for source_index in 0..(n - 1) {
            let card = game.player(state.of_player).battle_area[field_index].card_sources
                [source_index]
                .handle();
            raw.push((field_index as u8, source_index as u8, card));
        }
    }
    // Phase 2: filter + encode.
    let read = crate::effect_context::EffectReadContext::new_with_source_kind(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
    );
    let mut out = Vec::new();
    for (field_index, source_index, card) in raw {
        if state.picked.iter().any(|p| p.card == card) {
            continue;
        }
        let permanent = PermanentHandle {
            player: state.of_player,
            index: field_index,
        };
        let source = SourceSelectionRef {
            permanent,
            field_index,
            source_index,
            card,
        };
        if state
            .target_permanent
            .is_some_and(|handle| source.permanent != handle)
        {
            continue;
        }
        let passes = if state.eval_on_card {
            eval_predicate_with_bindings(
                &state.filter,
                &read,
                PredicateSubject::Card(source.card),
                Some(&state.filter_bindings),
            )
        } else {
            eval_predicate_with_bindings(
                &state.filter,
                &read,
                PredicateSubject::Source(source),
                Some(&state.filter_bindings),
            )
        };
        if !passes {
            continue;
        }
        if let Some(action) = encode_source_select(field_index as u16, source_index as u16) {
            out.push((action, source));
        }
    }
    out
}

/// Data terminal for a source multi-pick: bind the picked sources as a
/// source-ref list, then run the inner tail (the trampoline's `final_callback`).
fn run_source_multi_terminal(game: &mut crate::game::Game, state: crate::resume::SourceMultiState) {
    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    let mut b = state.bindings;
    if let Some(name) = &state.bind_as {
        b.insert_source_refs(name, state.picked);
    }
    run_tail_preserving_trigger_context(
        &mut ctx,
        state.trigger_context,
        &state.inner_tail,
        &mut b,
        &state.runtime,
    );
    // Nested multi-pick: run any outer-clause tails wrapped onto this frame, now
    // that the accumulated list is bound + the inner tail ran (the multi-pick has
    // terminated). Empty for a top-level (un-nested) multi-pick.
    run_outer_conts(game, state.outer_conts);
}

/// Install (or re-park) a source-multi step over the carried candidate snapshot.
/// Mirrors `install_source_multi_selection`'s PendingSelection (phase, kind,
/// optional + PASS gating at/above `min`).
pub(crate) fn install_source_multi_resume_step(
    game: &mut crate::game::Game,
    state: crate::resume::SourceMultiState,
) {
    use crate::action::space::PASS;
    use crate::selection::{PendingSelection, SelectionKind};
    let picked = state.picked.len() as u8;
    let is_optional = picked >= state.min;
    let mut valid_action_ids: Vec<u16> = state.candidates.iter().map(|(a, _)| *a).collect();
    if is_optional {
        valid_action_ids.push(PASS);
    }
    game.current_phase = crate::enums::GamePhase::SelectSource;
    game.pending_selection = Some(PendingSelection {
        zone_owner: None,
        kind: SelectionKind::SourceMulti {
            min: state.min,
            max: state.max,
            picked,
        },
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
        frames: vec![crate::resume::ResumeFrame::SourceMultiStep(state)],
    });
}

/// Executor for one source-multi pick (or PASS). Mirrors the recursive
/// `install_source_multi_selection`: decode against the carried snapshot,
/// live-revalidate the picked card, recompute candidates, then terminal
/// (exhausted/max at/above min) or re-park; PASS commits.
fn run_source_multi_step(
    game: &mut crate::game::Game,
    mut state: crate::resume::SourceMultiState,
    action_id: u16,
    is_pass: bool,
) {
    if is_pass {
        // PASS is only offered at/above min (the install gates it).
        run_source_multi_terminal(game, state);
        return;
    }
    let Some((_, source_ref)) = state
        .candidates
        .iter()
        .find(|(candidate_action, _)| *candidate_action == action_id)
        .copied()
    else {
        return; // stale/invalid pick (defensive)
    };
    // DCGO-parity live revalidation: only add if the snapshot card is still
    // present under its carrier (an intervening observer may have removed it).
    let still_present = game
        .player(source_ref.permanent.player)
        .battle_area
        .get(source_ref.permanent.index as usize)
        .map(|perm| {
            perm.card_sources
                .iter()
                .any(|c| c.handle() == source_ref.card)
        })
        .unwrap_or(false);
    if still_present {
        state.picked.push(source_ref);
    }
    // Recompute candidates from live state (the recursive install's enumeration).
    let next_candidates = source_multi_candidates_data(game, &state);
    if next_candidates.is_empty() || state.picked.len() == state.max as usize {
        if state.picked.len() >= state.min as usize {
            run_source_multi_terminal(game, state);
        }
        return;
    }
    state.candidates = next_candidates;
    install_source_multi_resume_step(game, state);
}

/// Re-derive the cost-budget candidates `(action_id, handle, cost)` for a
/// `BudgetState`, mirroring `dp_budget_candidates` / `play_cost_budget_candidates`
/// but evaluating the carried `CompiledPredicate` (data-pure). Cost is DP or
/// printed play cost per the budget kind.
fn budget_candidates_data(
    game: &crate::game::Game,
    state: &crate::resume::BudgetState,
) -> Vec<(u16, PermanentHandle, i32)> {
    use crate::action::space::encode_attack;
    use crate::resume::BudgetKind;
    let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
    );
    let mut out = Vec::new();
    for index in 0..game.player(state.opponent).battle_area.len() {
        let handle = PermanentHandle {
            player: state.opponent,
            index: index as u8,
        };
        if state.picked.contains(&handle) {
            continue;
        }
        if !eval_predicate_with_bindings(
            &state.filter,
            &read_ctx,
            PredicateSubject::Permanent(handle),
            Some(&state.filter_bindings),
        ) {
            continue;
        }
        let cost = match state.kind {
            BudgetKind::Dp => game.effective_dp(handle).unwrap_or(0),
            BudgetKind::PlayCost => i32::from(
                game.player(state.opponent).battle_area[index]
                    .top_card()
                    .play_cost(&game.card_data),
            ),
        };
        if cost <= state.remaining {
            out.push((encode_attack(0, index as u16), handle, cost));
        }
    }
    out
}

/// Data terminal for a cost-budget multi-pick: push effect targets for the
/// picked permanents (mirrors the trampoline's `final_callback` wrapper), bind
/// them as a permanent list, then run the inner tail.
fn run_budget_terminal(game: &mut crate::game::Game, state: crate::resume::BudgetState) {
    // Mirror select_opponent_permanents_by_*_budget's final_callback wrapper:
    // emit EffectTarget for the picked permanents before the body runs.
    let refs = crate::effect_context::selections::permanents_to_refs(game, &state.picked);
    crate::effect_context::selections::push_effect_target_multi(
        game,
        state.prov.controller,
        state.prov.source_card,
        refs,
    );
    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    let mut b = state.bindings;
    if let Some(name) = &state.bind_as {
        b.insert_permanent_list(name, state.picked);
    }
    run_tail_preserving_trigger_context(
        &mut ctx,
        state.trigger_context,
        &state.inner_tail,
        &mut b,
        &state.runtime,
    );
    // Nested multi-pick: run any outer-clause tails wrapped onto this frame, now
    // that the accumulated list is bound + the inner tail ran (the multi-pick has
    // terminated). Empty for a top-level (un-nested) multi-pick.
    run_outer_conts(game, state.outer_conts);
}

/// Install (or re-park) a cost-budget step: build the `PendingSelection` over the
/// recomputed candidates and stash the data frame. Mirrors
/// `install_dp_budget_selection` / `install_play_cost_budget_selection`
/// (phase/kind/optional + PASS gating at/above `min_picks`).
pub(crate) fn install_budget_resume_step(
    game: &mut crate::game::Game,
    state: crate::resume::BudgetState,
    candidates: Vec<(u16, PermanentHandle, i32)>,
) {
    use crate::action::space::PASS;
    use crate::resume::BudgetKind;
    use crate::selection::{PendingSelection, SelectionKind};
    let picked = state.picked.len() as u8;
    let is_optional = picked >= state.min_picks;
    let mut valid_action_ids: Vec<u16> = candidates.iter().map(|(a, _, _)| *a).collect();
    if is_optional {
        valid_action_ids.push(PASS);
    }
    let kind = match state.kind {
        BudgetKind::Dp => SelectionKind::DpBudget {
            remaining_dp: state.remaining,
            picked,
        },
        BudgetKind::PlayCost => SelectionKind::PlayCostBudget {
            remaining_play_cost: state.remaining,
            picked,
        },
    };
    game.current_phase = crate::enums::GamePhase::SelectBudgeted;
    game.pending_selection = Some(PendingSelection {
        zone_owner: None,
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
        frames: vec![crate::resume::ResumeFrame::BudgetStep(state)],
    });
}

/// Executor for one cost-budget pick (or PASS). Mirrors the recursive
/// `install_dp_budget_selection` as data: decode -> subtract cost -> recompute
/// candidates -> terminal (exhausted at/above min) or re-park; PASS commits.
fn run_budget_step(
    game: &mut crate::game::Game,
    mut state: crate::resume::BudgetState,
    action_id: u16,
    is_pass: bool,
) {
    if is_pass {
        // PASS is only offered at/above min_picks (the install gates it), so the
        // accumulated list is committable.
        run_budget_terminal(game, state);
        return;
    }
    let candidates = budget_candidates_data(game, &state);
    let Some((_, chosen, cost)) = candidates
        .iter()
        .find(|(candidate_action, _, _)| *candidate_action == action_id)
        .copied()
    else {
        return; // stale/invalid pick (defensive)
    };
    state.picked.push(chosen);
    state.remaining -= cost;
    // Recompute candidates with the new pick + reduced budget (the recursive
    // install's candidate computation).
    let next_candidates = budget_candidates_data(game, &state);
    if next_candidates.is_empty() {
        // Mirror install_dp_budget_selection's empty-candidates branch: terminal
        // only if the floor is met; otherwise the clause fizzles (no bind/tail).
        if state.picked.len() >= state.min_picks as usize {
            run_budget_terminal(game, state);
        }
        return;
    }
    install_budget_resume_step(game, state, next_candidates);
}

/// Data terminal for an ordered permutation: bind the accumulated handles as an
/// (ordered) card list, then run the inner tail. Mirrors the closure trampoline's
/// `final_callback` (and `run_multipick_terminal`).
fn run_permutation_terminal(game: &mut crate::game::Game, state: crate::resume::PermutationState) {
    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    let mut b = state.bindings;
    if let Some(placement) = &state.placement {
        // order_remainder / remainder_permutation: place the ordered list back on
        // the deck (mirrors place_remainder_in_order); does NOT bind.
        place_remainder_in_order(&mut ctx, placement.player, &state.accum, placement.position);
    } else if let Some(name) = &state.bind_as {
        b.insert_card_list(name, state.accum);
    }
    run_tail_preserving_trigger_context(
        &mut ctx,
        state.trigger_context,
        &state.inner_tail,
        &mut b,
        &state.runtime,
    );
    // Nested multi-pick: run any outer-clause tails wrapped onto this frame, now
    // that the accumulated list is bound/placed + the inner tail ran (the
    // multi-pick has terminated). Empty for a top-level (un-nested) multi-pick.
    run_outer_conts(game, state.outer_conts);
}

/// Install (or re-park) an ordered-permutation step: build the `PendingSelection`
/// over the remaining items and stash the data frame so `resolve_generic_selection`
/// resumes through `run_resume`. Mirrors `install_permutation_step`'s
/// PendingSelection (phase, kind, mandatory).
pub(crate) fn install_permutation_resume_step(
    game: &mut crate::game::Game,
    state: crate::resume::PermutationState,
) {
    use crate::action::space::SEL_REVEAL_START;
    use crate::selection::{PendingSelection, SelectionKind};
    let n = state.remaining.len() as u8;
    let valid_action_ids: Vec<u16> = (0..state.remaining.len())
        .map(|i| SEL_REVEAL_START + i as u16)
        .collect();
    game.current_phase = crate::enums::GamePhase::SelectPermutation;
    game.pending_selection = Some(PendingSelection {
        zone_owner: None,
        kind: SelectionKind::OrderedPermutation { remaining: n },
        selecting_player: state.selecting_player,
        previous_phase: state.previous_phase,
        valid_action_ids,
        is_optional: false,
        prompt: state.prompt.clone(),
        effect_choices: None,
        source_card: state.prov.source_card,
        source_permanent: state.prov.source_permanent,
        source_kind: state.prov.source_kind,
        // Vestigial during coexistence: run_resume is authoritative.
        callback: Box::new(|_g, _a| {}),
        on_decline: None,
    });
    game.pending_selection_resume = Some(crate::resume::ResumeStack {
        frames: vec![crate::resume::ResumeFrame::PermutationStep(state)],
    });
}

/// Executor for one ordered-permutation pick. Mirrors `install_permutation_step`
/// as data: decode the pick index into `remaining`, append to `accum`, then run
/// the terminal (list exhausted) or re-park for the next pick.
fn run_permutation_step(
    game: &mut crate::game::Game,
    mut state: crate::resume::PermutationState,
    action_id: u16,
) {
    let pick_idx = action_id.saturating_sub(crate::action::space::SEL_REVEAL_START) as usize;
    if pick_idx >= state.remaining.len() {
        return; // stale/invalid pick (defensive)
    }
    let picked = state.remaining.remove(pick_idx);
    state.accum.push(picked);
    if state.remaining.is_empty() {
        run_permutation_terminal(game, state);
    } else {
        install_permutation_resume_step(game, state);
    }
}

/// Run the outer-tail continuations composed onto a resume-driven selection by
/// `wrap_pending_selection_with_tail` (the data analog of the wrapped
/// callback). Each is run via `drain_or_rewrap_pending_tail` — so a cont that
/// itself parks a further select re-wraps onto it (threading deep nesting) —
/// after merging the bindings the just-run tail published (the
/// `dsl_resolved_tail_bindings` freshness channel, exactly as the closure
/// wrapper did). Conts run in push order.
pub(crate) fn run_outer_conts(
    game: &mut crate::game::Game,
    conts: Vec<crate::resume::OuterContinuation>,
) {
    for cont in conts {
        let mut merged = cont.bindings;
        if let Some(fresh) = game.dsl_resolved_tail_bindings.take() {
            merged.merge_slots_from(&fresh);
        }
        drain_or_rewrap_pending_tail(
            game,
            cont.source_card,
            cont.source_permanent,
            cont.player,
            (*cont.tail).clone(),
            merged,
            cont.runtime,
            cont.trigger_context,
        );
    }
}

/// Data terminal for a count_capped multi-pick: bind the accumulated handles as
/// a card list, then run the inner tail (the data form of the trampoline's
/// `final_callback`).
fn run_multipick_terminal(game: &mut crate::game::Game, state: crate::resume::MultiPickState) {
    // Mirror select_count_capped_multi_min's final_callback: emit EffectTarget for
    // the picked cards before the body runs.
    let refs: Vec<crate::events::EventCardRef> = state
        .accum
        .iter()
        .filter_map(|h| {
            game.card_data_for_handle(*h)
                .map(|cd| crate::events::EventCardRef {
                    card_id: cd.card_id.clone(),
                    card_name: cd.card_name.clone(),
                })
        })
        .collect();
    crate::effect_context::selections::push_effect_target_multi(
        game,
        state.prov.controller,
        state.prov.source_card,
        refs,
    );
    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    let mut b = state.bindings;
    if let Some(name) = &state.bind_as {
        b.insert_card_list(name, state.accum);
    }
    run_tail_preserving_trigger_context(
        &mut ctx,
        state.trigger_context,
        &state.inner_tail,
        &mut b,
        &state.runtime,
    );
    // Nested multi-pick: run any outer-clause tails wrapped onto this frame, now
    // that the accumulated list is bound + the inner tail ran (the multi-pick has
    // terminated). Empty for a top-level (un-nested) multi-pick.
    run_outer_conts(game, state.outer_conts);
}

/// Install (or re-park) a count_capped multi-pick step: build the
/// `PendingSelection` over the remaining candidates and stash the data frame so
/// `resolve_generic_selection` resumes through `run_resume`. Mirrors
/// `install_count_capped_step`'s PendingSelection (phase, kind, is_optional).
pub(crate) fn install_multipick_step(
    game: &mut crate::game::Game,
    state: crate::resume::MultiPickState,
) {
    use crate::selection::{PendingSelection, SelectionKind};
    let picked = state.accum.len() as u8;
    let effective_min = state.min.max(if state.is_optional_zero { 0 } else { 1 });
    let is_optional = picked >= effective_min;
    let valid_action_ids: Vec<u16> = state
        .candidate_indices
        .iter()
        .map(|&i| state.range_start + i as u16)
        .collect();
    game.current_phase = crate::enums::GamePhase::SelectBudgeted;
    game.pending_selection = Some(PendingSelection {
        zone_owner: None,
        kind: SelectionKind::CountCappedMultiSelect {
            min: effective_min,
            max: state.max,
            picked,
            distinct: state.distinct_by.is_some(),
        },
        selecting_player: state.selecting_player,
        previous_phase: state.previous_phase,
        valid_action_ids,
        is_optional,
        prompt: String::new(),
        effect_choices: None,
        source_card: state.prov.source_card,
        source_permanent: state.prov.source_permanent,
        source_kind: state.prov.source_kind,
        // Vestigial during coexistence: the resume data path is authoritative,
        // so this closure is never invoked (resolve_generic_selection dispatches
        // to run_resume whenever pending_selection_resume is set).
        callback: Box::new(|_g, _a| {}),
        on_decline: None,
    });
    game.pending_selection_resume = Some(crate::resume::ResumeStack {
        frames: vec![crate::resume::ResumeFrame::MultiPickStep(state)],
    });
}

/// Executor for one count_capped pick (or PASS). Mirrors
/// `install_count_capped_step`'s per-pick logic as data: decode → accumulate →
/// auto-commit at `max` or when candidates are exhausted, else re-park; PASS
/// commits with whatever is accumulated (gated at/above the floor upstream).
fn run_multipick_step(
    game: &mut crate::game::Game,
    mut state: crate::resume::MultiPickState,
    action_id: u16,
    is_pass: bool,
) {
    use crate::effect_context::selections::{material_zone_slice, CountCappedZone, DistinctByMode};
    if is_pass {
        run_multipick_terminal(game, state);
        return;
    }
    let pick_zone_idx = action_id.saturating_sub(state.range_start) as usize;
    let card_handle = match state.zone {
        CountCappedZone::Hand => game
            .player(state.of_player)
            .hand
            .get(pick_zone_idx)
            .map(|c| c.handle()),
        CountCappedZone::Trash => game
            .player(state.of_player)
            .trash
            .get(pick_zone_idx)
            .map(|c| c.handle()),
        CountCappedZone::Material(ph) => material_zone_slice(game, ph)
            .and_then(|s| s.get(pick_zone_idx))
            .map(|c| c.handle()),
    };
    let Some(card_handle) = card_handle else {
        return; // stale/invalid pick (defensive)
    };
    state.accum.push(card_handle);
    // Auto-commit when max reached.
    if state.accum.len() == state.max as usize {
        run_multipick_terminal(game, state);
        return;
    }
    // Recompute candidates: drop the picked index; with distinct_by, also drop
    // any remaining index sharing the constrained attribute with a pick.
    let old_candidates = std::mem::take(&mut state.candidate_indices);
    let new_candidates: Vec<usize> = if let Some(mode) = state.distinct_by {
        let accum_data_indices: Vec<usize> = state
            .accum
            .iter()
            .filter_map(|&h| {
                let slice: &[crate::card_source::CardSource] = match state.zone {
                    CountCappedZone::Hand => &game.player(state.of_player).hand,
                    CountCappedZone::Trash => &game.player(state.of_player).trash,
                    CountCappedZone::Material(ph) => match material_zone_slice(game, ph) {
                        Some(s) => s,
                        None => return None,
                    },
                };
                slice.iter().find(|c| c.handle() == h).map(|c| c.data_index)
            })
            .collect();
        old_candidates
            .into_iter()
            .filter(|&i| i != pick_zone_idx)
            .filter(|&i| {
                let cand_data_idx = match state.zone {
                    CountCappedZone::Hand => game.player(state.of_player).hand[i].data_index,
                    CountCappedZone::Trash => game.player(state.of_player).trash[i].data_index,
                    CountCappedZone::Material(ph) => match material_zone_slice(game, ph) {
                        Some(s) => s[i].data_index,
                        None => return false,
                    },
                };
                let cand_data = &game.card_data[cand_data_idx];
                !accum_data_indices.iter().any(|&pdi| {
                    let pd = &game.card_data[pdi];
                    match mode {
                        DistinctByMode::CardNumber => pd.card_id == cand_data.card_id,
                        DistinctByMode::Level => {
                            matches!((pd.level, cand_data.level), (Some(p), Some(c)) if p == c)
                        }
                        DistinctByMode::Name => pd.card_name == cand_data.card_name,
                    }
                })
            })
            .collect()
    } else {
        old_candidates
            .into_iter()
            .filter(|&i| i != pick_zone_idx)
            .collect()
    };
    // No candidates left → commit with what we have.
    if new_candidates.is_empty() {
        run_multipick_terminal(game, state);
        return;
    }
    state.candidate_indices = new_candidates;
    install_multipick_step(game, state);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallResult {
    NotSelection,
    Continue,
    TailAlreadyRan,
    Parked,
}

fn selection_result(ctx: &EffectContext<'_>) -> InstallResult {
    if ctx.game.pending_selection.is_some() {
        InstallResult::Parked
    } else {
        InstallResult::Continue
    }
}

/// collapse §1 — compose a select step's explicit scoped `then` action-tail
/// with the implicit dispatcher `tail` into the single tail the install helper
/// captures: `then ++ tail`. `then` runs first (on accept, binding in scope),
/// then the rest of the process body. The install helper either parks (running
/// the composed tail via its callback on resolve) or no-ops when there are no
/// candidates (the outer loop then runs `tail` alone via `Continue`) — so the
/// implicit `tail` runs exactly once and `then` runs only on a pick. This
/// mirrors the `SelectOwnSources` exemplar and is closure-free (cloneable VM
/// `ResumeFrame::RunTail` data).
fn compose_then_tail(then: &[CompiledStep], tail: &[CompiledStep]) -> Vec<CompiledStep> {
    let mut inner = then.to_vec();
    inner.extend_from_slice(tail);
    inner
}

/// Installs a selection step or reports how the dispatcher should advance.
///
/// Most selection steps park by installing the remainder as their callback.
/// Some unsupported or empty selections no-op and let the dispatcher continue
/// with the next step. Reveal-bucket selections can complete synchronously and
/// run the captured tail from inside their callback.
pub fn try_install(
    step: &CompiledStep,
    tail: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: Bindings,
    runtime: &StepRuntime,
) -> InstallResult {
    match step {
        CompiledStep::SelectHand {
            of,
            filter,
            bind_as,
            prompt,
            optional,
            cost,
            then,
            ..
        } => {
            install_select_hand(
                ctx,
                *of,
                filter.clone(),
                bind_as.clone(),
                prompt.clone(),
                *optional,
                *cost,
                compose_then_tail(then, tail),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectTrash {
            of,
            filter,
            bind_as,
            prompt,
            optional,
            cost,
            then,
            ..
        } => {
            install_select_trash(
                ctx,
                *of,
                filter.clone(),
                bind_as.clone(),
                prompt.clone(),
                *optional,
                *cost,
                compose_then_tail(then, tail),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::UseOptionFromHand {
            of,
            filter,
            use_cost_lte_opponent_memory,
            optional,
            prompt,
        } => {
            install_use_option_from_hand(
                ctx,
                *of,
                filter.clone(),
                *use_cost_lte_opponent_memory,
                *optional,
                prompt.clone(),
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::UseOptionFromTrash {
            of,
            filter,
            cost_delta,
            optional,
            prompt,
        } => {
            let delta = crate::dsl_cards::step::play_digivolve::lower_cost_delta(
                cost_delta.as_ref(),
                ctx,
                &bindings,
            );
            install_use_option_from_trash(
                ctx,
                *of,
                filter.clone(),
                delta,
                *optional,
                prompt.clone(),
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectOwnPermanent {
            filter,
            bind_as,
            selector,
            prompt,
            optional,
            continue_on_decline,
            then,
            ..
        } => {
            install_select_own_permanent(
                ctx,
                filter.clone(),
                *selector,
                bind_as.clone(),
                prompt.clone(),
                *optional,
                *continue_on_decline,
                compose_then_tail(then, tail),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::TrashBottomFaceDownSourceUnderTamer { of, optional } => {
            // Bundled activation cost: "pick one of `of`'s Tamers that carries
            // a face-down stash → trash its bottom face-down source". Pre-filter
            // the controller's permanents with the fixed predicate
            // `{ kind: tamer, has_face_down_source: true }`.
            let player = resolve_player(ctx, *of);
            let filter = CompiledPredicate {
                kind: Some(digimon_dsl::compiled::CompiledCardKind::Tamer),
                has_face_down_source: Some(true),
                ..CompiledPredicate::default()
            };
            let candidates = collect_matching_permanents(ctx, player, &filter, Some(&bindings));
            if candidates.is_empty() {
                // The cost is unpayable: no Tamer has a face-down stash. Abort
                // the clause — the dispatcher must stop and the tail (the rest
                // of the process) must NOT run. `TailAlreadyRan` here means
                // "dispatcher, stop; do not run the remaining steps".
                //
                // Flag the unpayable abort so the `cost_reduction` `pay_cost_fn`
                // lowering does NOT credit a reduction that was never paid for
                // (this clean abort maps to `RunOutcome::Synchronous`, otherwise
                // indistinguishable from a genuinely-paid synchronous cost).
                // `G-COST-REDUCTION-INTERACTIVE-PAY-COST`.
                ctx.cost_unpayable = true;
                return InstallResult::TailAlreadyRan;
            }
            install_trash_bottom_face_down_source_under_tamer(
                ctx,
                filter,
                *optional,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            // With ≥1 candidate, `select_own_permanent` always installs a
            // pending selection (even a single candidate is exposed as a
            // 1-option selection — no auto-resolve; `optional` adds a PASS), so
            // this parks. On a PASS decline (`optional`), the callback never
            // runs, so nothing is trashed and the tail is skipped.
            selection_result(ctx)
        }
        CompiledStep::TrashBottomFaceDownSourcesUnderTamers { of, count } => {
            // Multi-count / multi-Tamer activation cost: trash `count` bottom
            // face-down sources total, distributed across `of`'s Tamers. The
            // cost is unpayable (abort the clause) when fewer than `count`
            // face-down sources exist across all the controller's Tamers.
            // G-TRASH-N-BOTTOM-FACE-DOWN-UNDER-TAMER.
            let player = resolve_player(ctx, *of);
            let available = total_face_down_sources_under_tamers(ctx, player, &bindings);
            if (*count as usize) == 0 {
                // Degenerate count: nothing to pay, run the tail directly.
                let mut b = bindings.clone();
                let trigger_context = ctx.game.current_trigger_context.clone();
                run_tail_preserving_trigger_context(ctx, trigger_context, tail, &mut b, runtime);
                return InstallResult::TailAlreadyRan;
            }
            if available < *count as usize {
                // Fewer than `count` face-down sources ⇒ unpayable cost. Abort
                // the clause: stop the dispatcher and do NOT run the tail
                // (the digivolve). Mirrors the single-trash unpayable abort.
                ctx.cost_unpayable = true;
                return InstallResult::TailAlreadyRan;
            }
            install_trash_n_bottom_face_down_sources_under_tamers(
                ctx,
                *of,
                player,
                *count,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            // The first Tamer pick always installs (≥`count`≥1 face-down source
            // exists ⇒ ≥1 eligible Tamer), so this parks.
            selection_result(ctx)
        }
        CompiledStep::TrashLinkCardOfOwnDigimon { of, optional } => {
            // Link-card-trash ACTIVATION cost (BT25-073 Dragomon). Pre-filter the
            // controller's Digimon that carry ≥1 link card (DCGO
            // `CanSelectPermanentCondition = IsPermanentExistsOnOwnerBattleAreaDigimon
            // && !HasNoLinkCards`). When none qualify the cost is unpayable — abort
            // the clause (stop the dispatcher, do NOT run the tail).
            let player = resolve_player(ctx, *of);
            let candidates = own_digimon_with_link_cards(ctx, player);
            if candidates.is_empty() {
                // Same unpayable-abort contract as the face-down-trash cost:
                // flag it so an interactive cost-reduction pay_cost is not
                // credited for a cost that never resolved. G-DSL-LINK-TRASH-AS-COST
                // / G-COST-REDUCTION-INTERACTIVE-PAY-COST.
                ctx.cost_unpayable = true;
                return InstallResult::TailAlreadyRan;
            }
            install_trash_link_card_of_own_digimon(
                ctx,
                player,
                *optional,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            // With ≥1 candidate the first `select_own_permanent` always installs a
            // pending selection (single candidate is a 1-option select — no
            // auto-resolve; `optional` adds a PASS), so this parks. On PASS the
            // callback never runs → nothing trashed → tail skipped.
            selection_result(ctx)
        }
        CompiledStep::TrashOptionFromOwnStacks { of, optional } => {
            // Trash-Option-from-{digivolution|link}-cards ACTIVATION cost
            // (BT25-085 BeelStarmon). Pre-filter the controller's Digimon whose
            // digivolution cards (below the top) OR link cards carry ≥1 Option
            // (DCGO `PermanentWithTrashableCard = IsPermanentExistsOnOwnerBattle
            // AreaDigimon && DigivolutionOrLinkCards.Any(IsOption)`). When none
            // qualify the cost is unpayable — abort the clause (stop the
            // dispatcher, do NOT run the tail).
            let player = resolve_player(ctx, *of);
            let candidates = own_digimon_with_stack_options(ctx, player);
            if candidates.is_empty() {
                // Same unpayable-abort contract as the link-card-trash cost.
                // G-DSL-TRASH-OPTION-FROM-SOURCES-AS-COST /
                // G-COST-REDUCTION-INTERACTIVE-PAY-COST.
                ctx.cost_unpayable = true;
                return InstallResult::TailAlreadyRan;
            }
            install_trash_option_from_own_stacks(
                ctx,
                player,
                *optional,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            // With ≥1 candidate the first `select_own_permanent` always installs a
            // pending selection, so this parks. On PASS the callback never runs →
            // nothing trashed → tail skipped.
            selection_result(ctx)
        }
        CompiledStep::SelectOpponentPermanent {
            filter,
            bind_as,
            selector,
            prompt,
            optional,
            continue_on_decline,
            then,
            ..
        } => {
            install_select_opponent_permanent(
                ctx,
                filter.clone(),
                *selector,
                bind_as.clone(),
                prompt.clone(),
                *optional,
                *continue_on_decline,
                compose_then_tail(then, tail),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectAnyPermanent {
            filter,
            bind_as,
            selector,
            prompt,
            optional,
            then,
            ..
        } => {
            install_select_any_permanent(
                ctx,
                filter.clone(),
                None,
                *selector,
                bind_as.clone(),
                prompt.clone(),
                *optional,
                compose_then_tail(then, tail),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectDnaPair {
            left_filter,
            right_filter,
            bind_left_as,
            bind_right_as,
            prompt,
            optional,
            ..
        } => {
            install_select_dna_pair(
                ctx,
                left_filter.clone(),
                right_filter.clone(),
                bind_left_as.clone(),
                bind_right_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectCountCappedMulti {
            of,
            zone,
            max,
            min,
            clamp_to_available,
            bind_as,
            prompt,
            optional_zero,
            distinct_by,
            filter,
            ..
        } => {
            let target_player = resolve_player(ctx, *of);
            let max_value = resolve_count_bound(ctx, max, &bindings);
            let Some(candidate_count) =
                count_capped_candidate_count(ctx, target_player, *zone, filter, Some(&bindings))
            else {
                return InstallResult::Continue;
            };
            // When fewer candidates exist than the required minimum, the step
            // installs nothing AND does not run the captured tail — the
            // required cost is unpayable. G-SELECT-MULTI-MIN. A
            // `clamp_to_available` (MP-30/31 effect-target) selection is never
            // unpayable: it affects `min(max, available)` instead.
            let min_unpayable = !*clamp_to_available && *min > 0 && candidate_count < *min as usize;
            let completes_synchronously =
                !min_unpayable && (candidate_count == 0 || max_value == 0);
            install_select_count_capped_multi(
                ctx,
                *of,
                *zone,
                *min,
                max_value,
                *clamp_to_available,
                filter.clone(),
                bind_as.clone(),
                prompt.clone(),
                *optional_zero,
                map_distinct_by(*distinct_by),
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            if ctx.game.pending_selection.is_some() {
                InstallResult::Parked
            } else if completes_synchronously {
                InstallResult::TailAlreadyRan
            } else {
                // min_unpayable: the required cost cannot be paid. Nothing was
                // installed and the captured tail (e.g. a later
                // `cancel_replacement`) must NOT run — report TailAlreadyRan so
                // the dispatcher stops the slice atomically. This is the
                // cost-then-cancel guard: an unpayable cost aborts the whole
                // process rather than letting the cancel fire for free.
                debug_assert!(min_unpayable);
                InstallResult::TailAlreadyRan
            }
        }
        CompiledStep::DeleteOnePerOpponentColor { filter, prompt } => {
            install_delete_one_per_opponent_color(
                ctx,
                filter.as_deref().cloned(),
                prompt.clone(),
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            if ctx.game.pending_selection.is_some() {
                InstallResult::Parked
            } else {
                // No opponent color had a legal target: the terminal already ran
                // (batch-delete of an empty list is a no-op) and consumed the tail.
                InstallResult::TailAlreadyRan
            }
        }
        CompiledStep::SelectEffectChoice {
            labels,
            bind_as,
            prompt,
            ..
        } => {
            install_select_effect_choice(
                ctx,
                labels.clone(),
                bind_as.clone(),
                prompt.clone(),
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectReveal {
            of,
            filter,
            bind_as,
            prompt,
            optional,
            then,
            ..
        } => {
            return if install_select_reveal(
                ctx,
                *of,
                filter.clone(),
                bind_as.clone(),
                prompt.clone(),
                *optional,
                compose_then_tail(then, tail),
                bindings,
                runtime.clone(),
            ) {
                InstallResult::Parked
            } else {
                InstallResult::Continue
            };
        }
        CompiledStep::PlaceRemainderOnDeck { of, position }
            if *position != CompiledStackPosition::Choice =>
        {
            let player = resolve_player(ctx, *of);
            install_remainder_permutation_with_tail(
                ctx,
                player,
                super::map_stack_position(*position),
                tail.to_vec(),
                bindings,
                runtime.clone(),
            )
        }
        // collapse §3.1 — `place_remainder_on_deck` with `position: choice`.
        // Install a binary top/bottom pick; the chosen branch re-runs
        // place_remainder at the concrete end THROUGH run_steps, so the
        // remainder-ordering selection it installs and the outer tail are both
        // captured/parked correctly. Non-Choice positions are matched above so
        // fixed-position remainder placement also uses the resume-backed
        // permutation frame instead of the legacy callback helper.
        CompiledStep::PlaceRemainderOnDeck {
            of,
            position: CompiledStackPosition::Choice,
        } => {
            let branch = |pos: CompiledStackPosition| {
                let mut t = vec![CompiledStep::PlaceRemainderOnDeck {
                    of: *of,
                    position: pos,
                }];
                t.extend_from_slice(tail);
                Arc::new(t)
            };
            let tail_top = branch(CompiledStackPosition::Top);
            let tail_bottom = branch(CompiledStackPosition::Bottom);
            let trigger_context = ctx.game.current_trigger_context.clone();
            let runtime = runtime.clone();
            let branches_for_resume = vec![Arc::clone(&tail_top), Arc::clone(&tail_bottom)];
            let bindings_for_resume = bindings.clone();
            let runtime_for_resume = runtime.clone();
            let trigger_for_resume = trigger_context.clone();
            let source_card = ctx.source_card;
            let source_permanent = ctx.source_permanent;
            let source_kind = ctx.source_kind;
            let player = ctx.player;
            let override_pin = ctx.override_selecting_player();
            ctx.select_effect_choice(
                "Place the remaining cards on the top or bottom of the deck",
                vec!["Top of deck".to_string(), "Bottom of deck".to_string()],
                move |cb_ctx, idx| {
                    let branch = if idx == 0 { &tail_top } else { &tail_bottom };
                    let mut b = bindings.clone();
                    run_tail_preserving_trigger_context(
                        cb_ctx,
                        trigger_context,
                        branch,
                        &mut b,
                        &runtime,
                    );
                },
            );
            if ctx.game.pending_selection.is_some() {
                ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
                    frames: vec![crate::resume::ResumeFrame::RunTail {
                        prov: crate::resume::ResumeProvenance {
                            source_card,
                            source_permanent,
                            source_kind,
                            controller: player,
                            override_pin,
                        },
                        select_kind: crate::resume::ResumeSelectKind::EffectChoice {
                            post: Some(crate::resume::EffectChoicePostAction::RunTailBranch {
                                branches: branches_for_resume,
                            }),
                        },
                        bind_as: None,
                        inner_tail: Arc::new(Vec::new()),
                        outer_conts: Vec::new(),
                        bindings: bindings_for_resume,
                        runtime: runtime_for_resume,
                        trigger_context: trigger_for_resume,
                        decline: crate::resume::ResumeDecline::None,
                    }],
                });
            }
            selection_result(ctx)
        }
        // collapse §3.1/§3.2 — `place_on_security` (card source) with
        // `position: choice`. Same binary-pick pattern as place_remainder:
        // install a top/bottom EffectChoice, then re-run the placement at the
        // concrete end through run_steps so the tail parks correctly. Drives
        // BT25-038's hand path (collapsing its manual select_effect_choice +
        // two branches into one step). Non-Choice positions fall through to the
        // synchronous runner.
        CompiledStep::PlaceOnSecurity {
            of,
            source,
            position: CompiledStackPosition::Choice,
            face_up,
        } => {
            let branch = |pos: CompiledStackPosition| {
                let mut t = vec![CompiledStep::PlaceOnSecurity {
                    of: *of,
                    source: source.clone(),
                    position: pos,
                    face_up: *face_up,
                }];
                t.extend_from_slice(tail);
                Arc::new(t)
            };
            let tail_top = branch(CompiledStackPosition::Top);
            let tail_bottom = branch(CompiledStackPosition::Bottom);
            let trigger_context = ctx.game.current_trigger_context.clone();
            let runtime = runtime.clone();
            let branches_for_resume = vec![Arc::clone(&tail_top), Arc::clone(&tail_bottom)];
            let bindings_for_resume = bindings.clone();
            let runtime_for_resume = runtime.clone();
            let trigger_for_resume = trigger_context.clone();
            let source_card = ctx.source_card;
            let source_permanent = ctx.source_permanent;
            let source_kind = ctx.source_kind;
            let player = ctx.player;
            let override_pin = ctx.override_selecting_player();
            ctx.select_effect_choice(
                "Place as the top or bottom security card",
                vec![
                    "Top of security".to_string(),
                    "Bottom of security".to_string(),
                ],
                move |cb_ctx, idx| {
                    let branch = if idx == 0 { &tail_top } else { &tail_bottom };
                    let mut b = bindings.clone();
                    run_tail_preserving_trigger_context(
                        cb_ctx,
                        trigger_context,
                        branch,
                        &mut b,
                        &runtime,
                    );
                },
            );
            if ctx.game.pending_selection.is_some() {
                ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
                    frames: vec![crate::resume::ResumeFrame::RunTail {
                        prov: crate::resume::ResumeProvenance {
                            source_card,
                            source_permanent,
                            source_kind,
                            controller: player,
                            override_pin,
                        },
                        select_kind: crate::resume::ResumeSelectKind::EffectChoice {
                            post: Some(crate::resume::EffectChoicePostAction::RunTailBranch {
                                branches: branches_for_resume,
                            }),
                        },
                        bind_as: None,
                        inner_tail: Arc::new(Vec::new()),
                        outer_conts: Vec::new(),
                        bindings: bindings_for_resume,
                        runtime: runtime_for_resume,
                        trigger_context: trigger_for_resume,
                        decline: crate::resume::ResumeDecline::None,
                    }],
                });
            }
            selection_result(ctx)
        }
        CompiledStep::SelectRevealBuckets {
            from,
            buckets,
            no_duplicate_cards,
            prompt,
        } => {
            return install_select_reveal_buckets(
                ctx,
                from,
                buckets,
                *no_duplicate_cards,
                prompt.clone(),
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
        }
        // collapse §2 — `reveal_search` composite. Expand into the existing
        // sequence at run time: reveal the top `count`, run ONE
        // `select_reveal_buckets` over all buckets (cross-bucket de-dup on), then
        // a per-bucket reveal-move + `place_remainder_on_deck` as the captured
        // tail. Reuses `install_select_reveal_buckets` wholesale, so it parks /
        // resumes exactly like the longhand idiom. Pure data → deterministic
        // expansion (cloneable-aligned, reset-and-replay safe).
        CompiledStep::RevealSearch {
            of,
            count,
            buckets,
            remainder,
        } => {
            const POOL: &str = "__reveal_search_pool";
            let bucket_binding = |i: usize| format!("__reveal_search_bucket_{i}");

            let player = resolve_player(ctx, *of);
            // 1. Reveal the top `count` (populates `ctx.game.revealed_cards`).
            let pool = ctx.reveal_top_deck(player, *count);

            // 2. Bind the revealed pool for `select_reveal_buckets`' `from`.
            let mut bindings = bindings;
            bindings.insert_card_list(POOL, pool);

            // 3. One reveal-bucket selection over all buckets. A non-optional
            //    bucket forces `min == max` (the install advances when the
            //    candidate pool is exhausted, so this never soft-locks);
            //    `optional` keeps `min == 0`.
            let engine_buckets: Vec<CompiledRevealBucket> = buckets
                .iter()
                .enumerate()
                .map(|(i, bk)| CompiledRevealBucket {
                    bind_as: bucket_binding(i),
                    filter: Some(bk.filter.clone()),
                    min: if bk.optional { 0 } else { bk.max },
                    max: bk.max,
                })
                .collect();

            // 4. Captured tail: per-bucket reveal-move (consumes the bucket's
            //    bound CardList via the §2.1 multi-card move verbs), then place
            //    the remainder, then the outer dispatcher tail.
            let mut inner_tail: Vec<CompiledStep> =
                Vec::with_capacity(buckets.len() + 1 + tail.len());
            for (i, bk) in buckets.iter().enumerate() {
                let card = CompiledBindingRef::Named(bucket_binding(i));
                inner_tail.push(match bk.to {
                    CompiledRevealSearchDest::Hand => {
                        CompiledStep::AddToHandFromReveal { of: *of, card }
                    }
                    CompiledRevealSearchDest::Trash => {
                        CompiledStep::TrashFromReveal { of: *of, card }
                    }
                    CompiledRevealSearchDest::Deck => CompiledStep::ReturnToDeckFromReveal {
                        of: *of,
                        card,
                        position: CompiledStackPosition::Bottom,
                    },
                });
            }
            inner_tail.push(CompiledStep::PlaceRemainderOnDeck {
                of: *of,
                position: match remainder {
                    CompiledRevealRemainder::Top => CompiledStackPosition::Top,
                    CompiledRevealRemainder::Bottom => CompiledStackPosition::Bottom,
                    CompiledRevealRemainder::Choose => CompiledStackPosition::Choice,
                },
            });
            inner_tail.extend_from_slice(tail);

            let prompt = buckets.iter().find_map(|bk| bk.prompt.clone());
            return install_select_reveal_buckets(
                ctx,
                POOL,
                &engine_buckets,
                true,
                prompt,
                inner_tail,
                bindings,
                runtime.clone(),
            );
        }
        CompiledStep::SelectSecurity {
            of,
            filter,
            bind_as,
            prompt,
            optional,
            then,
            ..
        } => {
            install_select_security(
                ctx,
                *of,
                filter.clone(),
                bind_as.clone(),
                prompt.clone(),
                *optional,
                compose_then_tail(then, tail),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectMaterial {
            of_permanent,
            filter,
            bind_as,
            prompt,
            optional,
            ..
        } => {
            use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
            let perm = match resolve_binding_ref(of_permanent, ctx, &bindings) {
                Some(ResolvedBinding::Permanent(h)) => h,
                // Missing binding or wrong type: silent no-op (2b/2c convention).
                _ => return InstallResult::Continue,
            };
            if !has_material_candidates(ctx, perm, filter, Some(&bindings)) {
                return InstallResult::Continue;
            }
            install_select_material(
                ctx,
                perm,
                filter.clone(),
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectMaterials {
            of_permanent,
            max,
            filter,
            uniqueness,
            bind_as,
            prompt,
            optional_zero,
            ..
        } => {
            let perm = match resolve_binding_ref(of_permanent, ctx, &bindings) {
                Some(ResolvedBinding::Permanent(h)) => h,
                // Missing binding or wrong type: silent no-op (2b/2c convention).
                _ => return InstallResult::Continue,
            };
            let max_value = resolve_count_bound(ctx, max, &bindings);
            // Whether the carrier yields any source matching the filter at
            // install time — drives the synchronous-completion accounting
            // (mirrors `SelectCountCappedMulti`). Battle-area AND breeding-area
            // (`BREEDING_TARGET` sentinel) carriers are both handled: the
            // engine multi-pick encodes breeding sources in the
            // `BREEDING_SOURCE_SELECT` action range (Task S1.3).
            let has_candidates = has_material_candidates(ctx, perm, filter, Some(&bindings));
            let completes_synchronously = !has_candidates || max_value == 0;
            install_select_materials(
                ctx,
                perm,
                max_value,
                filter.clone(),
                map_distinct_by(*uniqueness),
                bind_as.clone(),
                prompt.clone(),
                *optional_zero,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            if ctx.game.pending_selection.is_some() {
                InstallResult::Parked
            } else if completes_synchronously {
                InstallResult::TailAlreadyRan
            } else {
                InstallResult::Continue
            }
        }
        CompiledStep::SelectOwnSources {
            target,
            filter,
            min,
            max,
            bind_as,
            prompt,
            then,
        } => {
            if min > max || *max == 0 {
                return InstallResult::Continue;
            }
            // When no digivolution sources exist at all (unfiltered), and the
            // clause requires at least one pick (min > 0), the cost cannot be
            // paid: abort the outer continuation silently. For min == 0, no
            // sources means the player implicitly picks 0 — the `then` body
            // with an empty binding is a no-op anyway, so advancing to the
            // outer tail (`Continue`) is correct for that case.
            if !has_own_source_candidates(ctx) {
                return if *min > 0 {
                    InstallResult::TailAlreadyRan
                } else {
                    InstallResult::Continue
                };
            }
            let mut inner_tail = then.clone();
            inner_tail.extend_from_slice(tail);
            install_select_own_sources(
                ctx,
                filter.clone(),
                *min,
                *max,
                target.clone(),
                bind_as.clone(),
                prompt.clone(),
                inner_tail,
                bindings,
                runtime.clone(),
            );
            // The outer tail was captured inside `inner_tail` and passed
            // entirely into `install_select_own_sources`. The outer loop must
            // NOT advance into those same steps again. If a selection was
            // installed, `Parked` stops the loop. If no selection was
            // installed (callback ran synchronously for min=0, or filtered
            // candidates = 0 for min>0 so the callback was dropped), the
            // outer tail already ran or was intentionally discarded —
            // `TailAlreadyRan` stops the loop in both sub-cases.
            if ctx.game.pending_selection.is_some() {
                InstallResult::Parked
            } else {
                InstallResult::TailAlreadyRan
            }
        }
        CompiledStep::SelectOpponentSources {
            target,
            filter,
            min,
            max,
            clamp_to_available,
            bind_as,
            prompt,
            then,
        } => {
            // G-DSL-SELECT-SOURCES-FORMULA-COUNT: literal-or-formula bounds
            // resolve ONCE here (both against the same board snapshot).
            let mut min_value = resolve_count_bound(ctx, min, &bindings);
            let mut max_value = resolve_count_bound(ctx, max, &bindings);
            if *clamp_to_available {
                // DCGO TrashDigivolutionCards.cs parity:
                // `maxDigivolutionDiscardCount = Math.Min(digivolutionCardsSum,
                // maxCount)` — the pick affects min(N, available) and never
                // becomes an unpayable-cost abort. Zero candidates (or a
                // formula resolving to 0) fall through to the `max_value == 0`
                // skip below (`if maxCount <= 0 yield break`), letting the
                // rest of the clause run.
                let available = count_opponent_source_candidates(ctx, target, filter, &bindings);
                max_value = max_value.min(available);
                min_value = min_value.min(available);
            }
            if min_value > max_value || max_value == 0 {
                return InstallResult::Continue;
            }
            // Mirror of SelectOwnSources early-abort logic: when no opponent
            // sources exist and min > 0, the cost cannot be paid — abort.
            // (Unreachable with `clamp_to_available` — min already clamped.)
            if !has_opponent_source_candidates(ctx) {
                return if min_value > 0 {
                    InstallResult::TailAlreadyRan
                } else {
                    InstallResult::Continue
                };
            }
            let mut inner_tail = then.clone();
            inner_tail.extend_from_slice(tail);
            install_select_opponent_sources(
                ctx,
                filter.clone(),
                min_value,
                max_value,
                target.clone(),
                bind_as.clone(),
                prompt.clone(),
                inner_tail,
                bindings,
                runtime.clone(),
            );
            // Same tail-capture semantics as SelectOwnSources: the outer tail
            // is embedded in inner_tail, so the outer loop must not advance
            // into those steps regardless of how the install resolved.
            if ctx.game.pending_selection.is_some() {
                InstallResult::Parked
            } else {
                InstallResult::TailAlreadyRan
            }
        }
        CompiledStep::SelectOpponentDpBudget {
            dp_budget,
            min_picks,
            filter,
            bind_as,
            prompt,
            then,
        } => {
            let dp_budget = formula_value(dp_budget, ctx, &bindings);
            if !has_opponent_dp_budget_candidates(ctx, dp_budget, filter, &bindings) {
                return InstallResult::Continue;
            }
            let mut inner_tail = then.clone();
            inner_tail.extend_from_slice(tail);
            install_select_opponent_dp_budget(
                ctx,
                dp_budget,
                *min_picks,
                filter.clone(),
                bind_as.clone(),
                prompt.clone(),
                inner_tail,
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectOpponentPlayCostBudget {
            play_cost_budget,
            min_picks,
            filter,
            bind_as,
            prompt,
            then,
        } => {
            // Evaluate the (possibly scaling) budget formula against the running
            // effect context, mirroring `SelectOpponentDpBudget` above. A bare
            // integer literal compiles to `CompiledFormula::Literal` and evaluates
            // to that constant, so scalar users (EX4-073) are unchanged. P-094
            // Destromon uses `{ base: 3, per: { source_stack_count: { filter: {
            // name_is: "Vemmon" } } }, delta: 1 }` → 3 + 1 per [Vemmon] source.
            let play_cost_budget = formula_value(play_cost_budget, ctx, &bindings);
            if !has_opponent_play_cost_budget_candidates(ctx, play_cost_budget, filter, &bindings) {
                return InstallResult::Continue;
            }
            let mut inner_tail = then.clone();
            inner_tail.extend_from_slice(tail);
            install_select_opponent_play_cost_budget(
                ctx,
                play_cost_budget,
                *min_picks,
                filter.clone(),
                bind_as.clone(),
                prompt.clone(),
                inner_tail,
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectOwnBreedingPermanent {
            bind_as,
            prompt,
            filter,
            optional,
            then,
        } => {
            if !has_own_breeding_candidate_matching(ctx, filter, Some(&bindings)) {
                return InstallResult::Continue;
            }
            let mut inner_tail = then.clone();
            inner_tail.extend_from_slice(tail);
            install_select_own_breeding_permanent(
                ctx,
                bind_as.clone(),
                prompt.clone(),
                filter.clone(),
                *optional,
                inner_tail,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectUnionZone {
            of,
            zones,
            material_of,
            filter,
            zone_filters,
            material_carrier_filter,
            bind_as,
            prompt,
            optional,
            cost,
            then,
            ..
        } => {
            use crate::selection::UnionZoneSet;
            let mut zoneset = UnionZoneSet(0);
            for z in zones {
                match z {
                    CompiledZone::Hand => zoneset |= UnionZoneSet::HAND,
                    CompiledZone::Trash => zoneset |= UnionZoneSet::TRASH,
                    CompiledZone::Material => zoneset |= UnionZoneSet::MATERIAL,
                    // Other zones not yet exposed by UnionZoneSet bitfield.
                    // Silently skip — future tasks widen engine API as needed.
                    _ => {}
                }
            }
            if zoneset.0 == 0 {
                // No supported zones: silent no-op; tail runs synchronously.
                return InstallResult::Continue;
            }
            let mut success_tail = then.clone();
            success_tail.extend_from_slice(tail);
            install_select_union_zone(
                ctx,
                *of,
                zoneset,
                material_of.clone(),
                filter.clone(),
                zone_filters.clone(),
                material_carrier_filter.clone(),
                bind_as.clone(),
                prompt.clone(),
                *optional,
                *cost,
                success_tail,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            selection_result(ctx)
        }
        CompiledStep::SelectOrderedPermutation {
            items,
            bind_as,
            prompt,
            ..
        } => {
            use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
            let item_list = match resolve_binding_ref(items, ctx, &bindings) {
                Some(ResolvedBinding::CardList(v)) => v,
                // Missing binding or wrong type: silent no-op.
                _ => return InstallResult::Continue,
            };
            let completes_synchronously = item_list.is_empty();
            install_select_ordered_permutation(
                ctx,
                item_list,
                bind_as.clone(),
                prompt.clone(),
                tail.to_vec(),
                bindings,
                runtime.clone(),
            );
            if ctx.game.pending_selection.is_some() {
                InstallResult::Parked
            } else if completes_synchronously {
                InstallResult::TailAlreadyRan
            } else {
                InstallResult::Continue
            }
        }
        // Phase 2 Track E (2026-05-17): pick one revealed card, route to a
        // typed destination. Lowers as a single `select_reveal` install whose
        // callback dispatches to the destination's engine helper.
        CompiledStep::ChooseFromReveal {
            of,
            filter,
            destination,
            bind_as,
            prompt,
            optional,
            ..
        } => {
            if install_choose_from_reveal(
                ctx,
                *of,
                filter.clone(),
                destination.clone(),
                bind_as.clone(),
                prompt.clone(),
                *optional,
                tail.to_vec(),
                bindings,
                runtime.clone(),
            ) {
                InstallResult::Parked
            } else {
                InstallResult::Continue
            }
        }
        // Phase 2 Track E (2026-05-17): place reveal pool back onto deck. When
        // multiple destinations are listed, prompt the player to choose; the
        // permutation is always exposed (no auto-determinism, Working Rule §17).
        CompiledStep::OrderRemainder {
            of,
            destinations,
            prompt,
            ..
        } => install_order_remainder(
            ctx,
            *of,
            destinations.clone(),
            prompt.clone(),
            tail.to_vec(),
            bindings,
            runtime.clone(),
        ),
        _ => InstallResult::NotSelection,
    }
}

fn has_own_source_candidates(ctx: &EffectContext<'_>) -> bool {
    ctx.game
        .player(ctx.player)
        .battle_area
        .iter()
        .any(|perm| perm.card_sources.len() > 1)
}

/// Opponent-side mirror of `has_own_source_candidates`: true when at least one
/// of the controller's OPPONENT's battle-area permanents has a digivolution
/// source below its top card. G-SELECT-OPPONENT-SOURCES.
fn has_opponent_source_candidates(ctx: &EffectContext<'_>) -> bool {
    let opponent = ctx.game.next_clockwise(ctx.player);
    ctx.game
        .player(opponent)
        .battle_area
        .iter()
        .any(|perm| perm.card_sources.len() > 1)
}

/// Count of legal `select_opponent_sources` candidates at execution time:
/// every card below the top card of each opponent battle-area permanent,
/// restricted by the optional `target:` binding and the card `filter:` —
/// the same shape `install_select_opponent_sources`'s per-pick filter
/// closure enforces. Drives the `clamp_to_available` min(N, available)
/// clamp (DCGO TrashDigivolutionCards.cs `digivolutionCardsSum`).
/// G-DSL-SELECT-SOURCES-FORMULA-COUNT.
fn count_opponent_source_candidates(
    ctx: &EffectContext<'_>,
    target: &Option<CompiledBindingRef>,
    filter: &CompiledPredicate,
    bindings: &Bindings,
) -> u8 {
    let target_permanent =
        target
            .as_ref()
            .and_then(|target| match resolve_binding_ref(target, ctx, bindings) {
                Some(ResolvedBinding::Permanent(handle)) => Some(handle),
                _ => None,
            });
    if target.is_some() && target_permanent.is_none() {
        // Target binding failed to resolve: the install-time filter rejects
        // every candidate, so the clamped count is 0.
        return 0;
    }
    let opponent = ctx.game.next_clockwise(ctx.player);
    let read = ctx.as_read();
    let mut count: usize = 0;
    for (index, perm) in ctx.game.player(opponent).battle_area.iter().enumerate() {
        let handle = PermanentHandle {
            player: opponent,
            index: index as u8,
        };
        if target_permanent.is_some_and(|target| target != handle) {
            continue;
        }
        count += perm
            .card_sources
            .iter()
            .take(perm.card_sources.len().saturating_sub(1))
            .filter(|card| {
                eval_predicate_with_bindings(
                    filter,
                    &read,
                    PredicateSubject::Card(card.handle()),
                    Some(bindings),
                )
            })
            .count();
    }
    count.min(usize::from(u8::MAX)) as u8
}

fn has_material_candidates(
    ctx: &EffectContext<'_>,
    perm: PermanentHandle,
    filter: &CompiledPredicate,
    bindings: Option<&Bindings>,
) -> bool {
    let read = ctx.as_read();
    // material_carrier_permanent branches battle-area vs. breeding-area
    // (BREEDING_TARGET sentinel) carriers — so a King Drasil breeding
    // carrier yields its real digivolution sources here, keeping the
    // `completes_synchronously` accounting correct for breeding carriers.
    crate::effect_context::material_carrier_permanent(ctx.game, perm)
        .map(|p| {
            p.card_sources
                .iter()
                .take(p.card_sources.len().saturating_sub(1))
                .any(|card| {
                    eval_predicate_with_bindings(
                        filter,
                        &read,
                        PredicateSubject::Card(card.handle()),
                        bindings,
                    )
                })
        })
        .unwrap_or(false)
}

fn has_opponent_dp_budget_candidates(
    ctx: &EffectContext<'_>,
    dp_budget: i32,
    filter: &CompiledPredicate,
    bindings: &Bindings,
) -> bool {
    let opponent = ctx.game.next_clockwise(ctx.player);
    let read = ctx.as_read();
    ctx.game
        .player(opponent)
        .battle_area
        .iter()
        .enumerate()
        .any(|(index, _)| {
            let handle = PermanentHandle {
                player: opponent,
                index: index as u8,
            };
            if ctx.game.effective_dp(handle).unwrap_or(0) > dp_budget {
                return false;
            }
            eval_predicate_with_bindings(
                filter,
                &read,
                PredicateSubject::Permanent(handle),
                Some(bindings),
            )
        })
}

/// True when at least one opponent permanent satisfies the filter AND has a
/// printed play cost within the budget — at least one pickable candidate
/// exists for a `select_opponent_play_cost_budget` step.
/// G-MULTI-SELECT-OPP-PLAY-COST-SUM.
fn has_opponent_play_cost_budget_candidates(
    ctx: &EffectContext<'_>,
    play_cost_budget: i32,
    filter: &CompiledPredicate,
    bindings: &Bindings,
) -> bool {
    let opponent = ctx.game.next_clockwise(ctx.player);
    let read = ctx.as_read();
    ctx.game
        .player(opponent)
        .battle_area
        .iter()
        .enumerate()
        .any(|(index, perm)| {
            if i32::from(perm.top_card().play_cost(&ctx.game.card_data)) > play_cost_budget {
                return false;
            }
            let handle = PermanentHandle {
                player: opponent,
                index: index as u8,
            };
            eval_predicate_with_bindings(
                filter,
                &read,
                PredicateSubject::Permanent(handle),
                Some(bindings),
            )
        })
}

/// The breeding area must be non-empty AND the breeding permanent must
/// satisfy the predicate. An empty predicate passes everything,
/// matching the historical contract.
fn has_own_breeding_candidate_matching(
    ctx: &EffectContext<'_>,
    filter: &CompiledPredicate,
    bindings: Option<&Bindings>,
) -> bool {
    if ctx.game.player(ctx.player).breeding_area.is_none() {
        return false;
    }
    let read = ctx.as_read();
    eval_predicate_with_bindings(
        filter,
        &read,
        PredicateSubject::BreedingPermanent(ctx.player),
        bindings,
    )
}

fn resolve_count_bound(
    ctx: &EffectContext<'_>,
    max: &CompiledCountBound,
    bindings: &Bindings,
) -> u8 {
    let value = match max {
        CompiledCountBound::Literal(n) => i32::from(*n),
        CompiledCountBound::Formula(formula) => ctx
            .source_permanent
            .map(|target| {
                formula_eval::evaluate_with_bindings(formula, ctx, target, Some(bindings))
            })
            .unwrap_or(0),
    };
    value.clamp(0, 10) as u8
}

fn count_capped_candidate_count(
    ctx: &EffectContext<'_>,
    of_player: u8,
    zone: CompiledZone,
    filter: &CompiledPredicate,
    bindings: Option<&Bindings>,
) -> Option<usize> {
    match zone {
        CompiledZone::Hand => {
            let read = ctx.as_read();
            Some(
                ctx.game
                    .player(of_player)
                    .hand
                    .iter()
                    .filter(|card| {
                        eval_predicate_with_bindings(
                            filter,
                            &read,
                            PredicateSubject::Card(card.handle()),
                            bindings,
                        )
                    })
                    .count(),
            )
        }
        CompiledZone::Trash => {
            let read = ctx.as_read();
            Some(
                ctx.game
                    .player(of_player)
                    .trash
                    .iter()
                    .filter(|card| {
                        eval_predicate_with_bindings(
                            filter,
                            &read,
                            PredicateSubject::Card(card.handle()),
                            bindings,
                        )
                    })
                    .count(),
            )
        }
        CompiledZone::BattleArea => {
            Some(collect_matching_permanents(ctx, of_player, filter, bindings).len())
        }
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn install_select_hand(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    cost: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let target_player = resolve_player(ctx, of);
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    let tail_for_decline = Arc::clone(&tail);
    let bindings_for_decline = bindings.clone();
    let runtime_for_decline = runtime.clone();
    let trigger_for_decline = trigger_context.clone();
    // Resumable-VM captures (mirrors `install_select_trash`): the RunTail
    // Hand decode arm needs its own copies since the closures move theirs.
    let bind_as_for_resume = bind_as.clone();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    let override_pin = ctx.override_selecting_player();
    ctx.select_hand(
        target_player,
        &prompt,
        optional,
        move |game, idx| {
            let Some(card) = game.player(target_player).hand.get(idx).map(|c| c.handle()) else {
                return false;
            };
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::Card(card),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_hand_index(name, target_player, idx as u16);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
    if optional {
        if let Some(pending) = ctx.game.pending_selection.as_mut() {
            pending.on_decline = Some(Box::new(move |game: &mut crate::game::Game| {
                // Cost-pay selects (`cost: true` in YAML) abort the clause
                // on decline: set `dsl_clause_aborted = true` so the step
                // runner short-circuits the captured tail AND any parked
                // outer tail. See `G-OPTIONAL-COST-DECLINE-ABORTS-CLAUSE`
                // and DCGO `ActivateClass.SetUpICardEffect`. Non-cost
                // optional picks keep the historical "decline runs the
                // tail" semantic so mandatory housekeeping steps (e.g.
                // `add_this_option_to_hand` after a "you may" trash play)
                // execute regardless. The captured tail is always invoked
                // — when `cost: true` it short-circuits via the flag; when
                // not, it runs the tail directly.
                if cost {
                    game.dsl_clause_aborted = true;
                }
                let mut decline_ctx = EffectContext::new_with_source_kind(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    player,
                );
                let mut b = bindings_for_decline.clone();
                run_tail_preserving_trigger_context(
                    &mut decline_ctx,
                    trigger_for_decline.clone(),
                    &tail_for_decline,
                    &mut b,
                    &runtime_for_decline,
                );
            }));
        }
    }
    // Park the data frame alongside the closure (coexistence): if a selection
    // was installed, `resolve_generic_selection` drives it via `run_resume`
    // (the Hand decode arm), bypassing the closure. The `ResumeDecline`
    // mirrors the optional on_decline above — same tail, `aborts_clause =
    // cost`. (This installer was the clone-fuzz spike's one closure-only
    // production finding — every other DSL step site was already flipped.)
    if ctx.game.pending_selection.is_some() {
        let decline = if optional {
            crate::resume::ResumeDecline::RunTail {
                tail: Arc::clone(&tail_for_resume),
                aborts_clause: cost,
            }
        } else {
            crate::resume::ResumeDecline::None
        };
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RunTail {
                prov: crate::resume::ResumeProvenance {
                    source_card,
                    source_permanent,
                    source_kind,
                    controller: player,
                    override_pin,
                },
                select_kind: crate::resume::ResumeSelectKind::Hand {
                    of_player: target_player,
                },
                bind_as: bind_as_for_resume,
                inner_tail: tail_for_resume,
                outer_conts: Vec::new(),
                bindings: bindings_for_resume,
                runtime: runtime_for_resume,
                trigger_context: trigger_for_resume,
                decline,
            }],
        });
    }
}

fn player_visible_memory(game: &crate::game::Game, player: PlayerId) -> i16 {
    if game.turn_player() == player {
        game.memory.max(0)
    } else {
        (-game.memory).max(0)
    }
}

fn install_use_option_from_hand(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    filter: CompiledPredicate,
    use_cost_lte_opponent_memory: bool,
    optional: bool,
    prompt: Option<String>,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let target_player = resolve_player(ctx, of);
    let prompt = prompt.unwrap_or_else(|| "Choose an Option card to use".to_string());
    // Resumable-VM: capture for the UseOptionFromHandStep data frame before the
    // accept/decline closures consume tail/runtime/trigger. of_player = the hand
    // owner (target_player); the option play + tail run at resume time.
    let of_player_for_resume = target_player;
    let override_pin_for_resume = ctx.override_selecting_player();
    let source_card_for_resume = ctx.source_card;
    let source_permanent_for_resume = ctx.source_permanent;
    let source_kind_for_resume = ctx.source_kind;
    let controller_for_resume = ctx.player;
    let tail_for_resume = Arc::new(tail.clone());
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = ctx.game.current_trigger_context.clone();
    let optional_for_resume = optional;

    let tail_for_accept = tail.clone();
    let tail_for_decline = tail;
    let runtime_for_accept = runtime.clone();
    let runtime_for_decline = runtime;
    let bindings_for_accept = bindings.clone();
    let bindings_for_decline = bindings.clone();
    let trigger_context = ctx.game.current_trigger_context.clone();
    let trigger_for_accept = trigger_context.clone();
    let trigger_for_decline = trigger_context;
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();

    ctx.select_hand(
        target_player,
        &prompt,
        optional,
        move |game, idx| {
            let Some(card) = game.player(target_player).hand.get(idx) else {
                return false;
            };
            let kind = card.card_kind(&game.card_data);
            if !matches!(kind, CardKind::Option | CardKind::Dual) {
                return false;
            }
            if use_cost_lte_opponent_memory {
                let opponent = game.next_clockwise(player);
                let ceiling = player_visible_memory(game, opponent);
                let use_cost = card
                    .option_use_cost(&game.card_data)
                    .unwrap_or_else(|| card.play_cost(&game.card_data))
                    as i16;
                if use_cost > ceiling {
                    return false;
                }
            }
            let read_ctx = EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::Card(card.handle()),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, idx| {
            let previous = cb_ctx.game.current_trigger_context.clone();
            cb_ctx.game.current_trigger_context = trigger_for_accept.clone();
            let result = cb_ctx
                .game
                .use_option_from_hand_without_paying_cost(target_player, idx);
            cb_ctx.game.current_trigger_context = previous;
            if matches!(result, crate::selection::OptionPlayResult::Invalid) {
                return;
            }
            let tail_context = trigger_for_accept.clone();
            drain_or_rewrap_pending_tail(
                cb_ctx.game,
                source_card,
                source_permanent,
                player,
                tail_for_accept,
                bindings_for_accept,
                runtime_for_accept,
                tail_context,
            );
        },
    );

    if optional {
        if let Some(pending) = ctx.game.pending_selection.as_mut() {
            pending.on_decline = Some(Box::new(move |game: &mut crate::game::Game| {
                let previous = game.current_trigger_context.clone();
                game.current_trigger_context = trigger_for_decline.clone();
                let tail_context = trigger_for_decline.clone();
                drain_or_rewrap_pending_tail(
                    game,
                    source_card,
                    source_permanent,
                    player,
                    tail_for_decline,
                    bindings_for_decline,
                    runtime_for_decline,
                    tail_context,
                );
                game.current_trigger_context = previous;
            }));
        }
    }
    // Park the data frame alongside the closure (coexistence): driven by
    // run_resume's UseOptionFromHandStep arm. select_hand returns WITHOUT
    // installing when no eligible Option/Dual in hand, so guard on the install.
    if ctx.game.pending_selection.is_some() {
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::UseOptionFromHandStep(
                crate::resume::UseOptionFromHandState {
                    prov: crate::resume::ResumeProvenance {
                        source_card: source_card_for_resume,
                        source_permanent: source_permanent_for_resume,
                        source_kind: source_kind_for_resume,
                        controller: controller_for_resume,
                        override_pin: override_pin_for_resume,
                    },
                    of_player: of_player_for_resume,
                    tail: tail_for_resume,
                    bindings: bindings_for_resume,
                    runtime: runtime_for_resume,
                    trigger_context: trigger_for_resume,
                    outer_conts: Vec::new(),
                    optional: optional_for_resume,
                },
            )],
        });
    }
}

/// Trash analogue of [`install_use_option_from_hand`] (Gap 2,
/// `G-DSL-USE-OPTION-FROM-SOURCES`). Surfaces a `select_trash` prompt over the
/// controller's Option/Dual trash cards passing `filter`; the pick is USED via
/// `Game::use_option_from(Trash, cost_delta)` (full [Main] lifecycle +
/// disposal), then the tail composes onto any nested selection the Option
/// installed. Clone-safe: the pick parks through the resumable VM
/// (`UseOptionFromTrashStep` data frame) alongside the closure, and the
/// downstream Option-use itself parks through the data VM (`play_option_core`).
#[allow(clippy::too_many_arguments)]
fn install_use_option_from_trash(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    filter: CompiledPredicate,
    cost_delta: crate::enums::CostDelta,
    optional: bool,
    prompt: Option<String>,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let target_player = resolve_player(ctx, of);
    let prompt = prompt.unwrap_or_else(|| "Choose an Option card to use from trash".to_string());

    // Resume-frame capture (before the accept/decline closures consume state).
    let of_player_for_resume = target_player;
    let override_pin_for_resume = ctx.override_selecting_player();
    let source_card_for_resume = ctx.source_card;
    let source_permanent_for_resume = ctx.source_permanent;
    let source_kind_for_resume = ctx.source_kind;
    let controller_for_resume = ctx.player;
    let tail_for_resume = Arc::new(tail.clone());
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = ctx.game.current_trigger_context.clone();
    let optional_for_resume = optional;

    let tail_for_accept = tail.clone();
    let tail_for_decline = tail;
    let runtime_for_accept = runtime.clone();
    let runtime_for_decline = runtime;
    let bindings_for_accept = bindings.clone();
    let bindings_for_decline = bindings.clone();
    let trigger_context = ctx.game.current_trigger_context.clone();
    let trigger_for_accept = trigger_context.clone();
    let trigger_for_decline = trigger_context;
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();

    ctx.select_trash(
        target_player,
        &prompt,
        optional,
        move |game, idx| {
            let Some(card) = game.player(target_player).trash.get(idx) else {
                return false;
            };
            let kind = card.card_kind(&game.card_data);
            if !matches!(kind, CardKind::Option | CardKind::Dual) {
                return false;
            }
            let read_ctx = EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::Card(card.handle()),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, idx| {
            let previous = cb_ctx.game.current_trigger_context.clone();
            cb_ctx.game.current_trigger_context = trigger_for_accept.clone();
            let result = cb_ctx.game.use_option_from(
                target_player,
                crate::game_actions::OptionSource::Trash(idx),
                cost_delta,
            );
            cb_ctx.game.current_trigger_context = previous;
            if matches!(result, crate::selection::OptionPlayResult::Invalid) {
                return;
            }
            let tail_context = trigger_for_accept.clone();
            drain_or_rewrap_pending_tail(
                cb_ctx.game,
                source_card,
                source_permanent,
                player,
                tail_for_accept,
                bindings_for_accept,
                runtime_for_accept,
                tail_context,
            );
        },
    );

    if optional {
        if let Some(pending) = ctx.game.pending_selection.as_mut() {
            pending.on_decline = Some(Box::new(move |game: &mut crate::game::Game| {
                let previous = game.current_trigger_context.clone();
                game.current_trigger_context = trigger_for_decline.clone();
                let tail_context = trigger_for_decline.clone();
                drain_or_rewrap_pending_tail(
                    game,
                    source_card,
                    source_permanent,
                    player,
                    tail_for_decline,
                    bindings_for_decline,
                    runtime_for_decline,
                    tail_context,
                );
                game.current_trigger_context = previous;
            }));
        }
    }
    // Park the data frame alongside the closure (coexistence). select_trash
    // returns WITHOUT installing when no eligible Option/Dual in trash.
    if ctx.game.pending_selection.is_some() {
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::UseOptionFromTrashStep(
                crate::resume::UseOptionFromTrashState {
                    prov: crate::resume::ResumeProvenance {
                        source_card: source_card_for_resume,
                        source_permanent: source_permanent_for_resume,
                        source_kind: source_kind_for_resume,
                        controller: controller_for_resume,
                        override_pin: override_pin_for_resume,
                    },
                    of_player: of_player_for_resume,
                    cost_delta,
                    tail: tail_for_resume,
                    bindings: bindings_for_resume,
                    runtime: runtime_for_resume,
                    trigger_context: trigger_for_resume,
                    outer_conts: Vec::new(),
                    optional: optional_for_resume,
                },
            )],
        });
    }
}

/// Executor for a `use_option_from_trash` selection (Gap 2) — mirrors
/// [`run_use_option_from_hand_step`] with `TRASH`-space decoding + the trash
/// origin + `cost_delta`.
fn run_use_option_from_trash_step(
    game: &mut crate::game::Game,
    state: crate::resume::UseOptionFromTrashState,
    action_id: u16,
    is_pass: bool,
) {
    let crate::resume::UseOptionFromTrashState {
        prov,
        of_player,
        cost_delta,
        tail,
        bindings,
        runtime,
        trigger_context,
        outer_conts,
        optional: _optional,
    } = state;

    if is_pass {
        let previous = game.current_trigger_context.clone();
        game.current_trigger_context = trigger_context.clone();
        drain_or_rewrap_pending_tail(
            game,
            prov.source_card,
            prov.source_permanent,
            prov.controller,
            (*tail).clone(),
            bindings,
            runtime,
            trigger_context,
        );
        game.current_trigger_context = previous;
        run_outer_conts(game, outer_conts);
        return;
    }

    let idx = action_id.saturating_sub(crate::action::space::TRASH_EFFECT_START) as usize;
    if let Some(card) = game.player(of_player).trash.get(idx) {
        let tid = card.card_id(&game.card_data).to_string();
        let tname = card.card_name(&game.card_data).to_string();
        crate::effect_context::selections::push_effect_target(
            game,
            prov.controller,
            prov.source_card,
            tid,
            tname,
        );
    }
    let previous = game.current_trigger_context.clone();
    game.current_trigger_context = trigger_context.clone();
    let result = game.use_option_from(
        of_player,
        crate::game_actions::OptionSource::Trash(idx),
        cost_delta,
    );
    game.current_trigger_context = previous;
    if matches!(result, crate::selection::OptionPlayResult::Invalid) {
        return;
    }
    drain_or_rewrap_pending_tail(
        game,
        prov.source_card,
        prov.source_permanent,
        prov.controller,
        (*tail).clone(),
        bindings,
        runtime,
        trigger_context,
    );
    run_outer_conts(game, outer_conts);
}

/// Count how many hand cards of `of`'s player satisfy `filter` under the
/// supplied bindings. Used by the `- optional: [...]` substep guard to
/// detect an empty-candidate leading `select_hand` (G-SELECT-EMPTY-OUTER-TAIL):
/// when zero candidates exist the entire optional substep body is skipped
/// rather than silently falling through to subsequent mandatory steps.
pub(crate) fn select_hand_candidate_count(
    ctx: &EffectContext<'_>,
    of: CompiledPlayerRef,
    filter: &CompiledPredicate,
    bindings: &Bindings,
) -> usize {
    let target_player = resolve_player(ctx, of);
    let read_ctx = ctx.as_read();
    ctx.game
        .player(target_player)
        .hand
        .iter()
        .filter(|card| {
            eval_predicate_with_bindings(
                filter,
                &read_ctx,
                PredicateSubject::Card(card.handle()),
                Some(bindings),
            )
        })
        .count()
}

#[allow(clippy::too_many_arguments)]
fn install_select_trash(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    cost: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let target_player = resolve_player(ctx, of);
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    // Clone state for the decline path before the success callback consumes
    // them. G-OPTIONAL-SELECTION-CONTINUE-TAIL — declining an optional
    // trash selection must still run the outer tail so subsequent
    // mandatory steps (e.g. `add_this_option_to_hand`) execute.
    let tail_for_decline = Arc::clone(&tail);
    let bindings_for_decline = bindings.clone();
    let runtime_for_decline = runtime.clone();
    let trigger_for_decline = trigger_context.clone();
    // Resumable-VM (make-engine-cloneable Batch 1): capture the same state the
    // success/decline closures close over so we can ALSO park a data frame. The
    // closures move `tail`/`bindings`/`runtime`/`trigger_context`/`bind_as`, so
    // clone first. `override_pin` mirrors `select_trash`'s captured value.
    let bind_as_for_resume = bind_as.clone();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    let override_pin = ctx.override_selecting_player();
    ctx.select_trash(
        target_player,
        &prompt,
        optional,
        move |game, idx| {
            let Some(card) = game
                .player(target_player)
                .trash
                .get(idx)
                .map(|c| c.handle())
            else {
                return false;
            };
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::Card(card),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_trash_index(name, target_player, idx as u16);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
    // Attach decline-tail callback for optional selections.
    if optional {
        if let Some(pending) = ctx.game.pending_selection.as_mut() {
            pending.on_decline = Some(Box::new(move |game: &mut crate::game::Game| {
                // See `install_select_hand` for the cost vs. continue-tail
                // distinction. `cost: true` sets the abort flag and the
                // captured tail short-circuits; otherwise the existing
                // mandatory-tail semantics for `G-OPTIONAL-SELECTION-CONTINUE-TAIL`
                // (e.g. `add_this_option_to_hand` after a "you may" trash
                // play) keep working.
                if cost {
                    game.dsl_clause_aborted = true;
                }
                let mut decline_ctx = EffectContext::new_with_source_kind(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    player,
                );
                let mut b = bindings_for_decline.clone();
                run_tail_preserving_trigger_context(
                    &mut decline_ctx,
                    trigger_for_decline.clone(),
                    &tail_for_decline,
                    &mut b,
                    &runtime_for_decline,
                );
            }));
        }
    }
    // Park the data frame alongside the closure (coexistence): if a selection
    // was installed, `resolve_generic_selection` will drive it via `run_resume`
    // (the Trash decode arm), bypassing the closure. The `ResumeDecline` mirrors
    // the optional on_decline above — same tail, `aborts_clause = cost`.
    if ctx.game.pending_selection.is_some() {
        let decline = if optional {
            crate::resume::ResumeDecline::RunTail {
                tail: Arc::clone(&tail_for_resume),
                aborts_clause: cost,
            }
        } else {
            crate::resume::ResumeDecline::None
        };
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RunTail {
                prov: crate::resume::ResumeProvenance {
                    source_card,
                    source_permanent,
                    source_kind,
                    controller: player,
                    override_pin,
                },
                select_kind: crate::resume::ResumeSelectKind::Trash {
                    of_player: target_player,
                },
                bind_as: bind_as_for_resume,
                inner_tail: tail_for_resume,
                outer_conts: Vec::new(),
                bindings: bindings_for_resume,
                runtime: runtime_for_resume,
                trigger_context: trigger_for_resume,
                decline,
            }],
        });
    }
}

fn install_select_own_permanent(
    ctx: &mut EffectContext<'_>,
    filter: CompiledPredicate,
    selector: Option<CompiledFieldSelector>,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    continue_on_decline: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    // Pre-filter candidates using the compiled predicate so that an empty
    // result (e.g. "kind: token" with no tokens on field) short-circuits
    // without installing a PendingSelection. Mirrors install_select_any_permanent.
    let target_player = ctx.player;
    let candidates = collect_matching_permanents(ctx, target_player, &filter, Some(&bindings));
    let selected_field = select_field_extreme(ctx.game, &candidates, selector);
    if candidates.is_empty() {
        return;
    }

    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    let tail_for_decline = tail.clone();
    let bindings_for_decline = bindings.clone();
    let runtime_for_decline = runtime.clone();
    let trigger_for_decline = trigger_context.clone();
    // Resumable-VM (Batch 2): capture for the data frame before the closures
    // consume them. selector/selected_field are NOT carried — the filter already
    // baked the matching slots into valid_action_ids, so the FieldPermanent
    // decode arm just maps the resolved action_id → handle.
    let bind_as_for_resume = bind_as.clone();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    let override_pin = ctx.override_selecting_player();
    ctx.select_own_permanent(
        &prompt,
        optional,
        move |game, handle| {
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::Permanent(handle),
                Some(&filter_bindings),
            ) && matches_selected_field(game, handle, selector, selected_field)
        },
        move |cb_ctx, handle: PermanentHandle| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_permanent(name, handle);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
    // `continue_on_decline: true` — PASS leaves the binding unresolved and the
    // clause CONTINUES (DCGO's declined SelectPermanentEffect resolves with an
    // empty list and the coroutine carries on), so binding-gated follow-ups
    // run. The default keeps the historical permanent-select semantic (decline
    // drops the tail) which existing cards use as a cost/accept gate.
    // G-OPT-REFUND-ON-DECLINE.
    if optional && continue_on_decline {
        if let Some(pending) = ctx.game.pending_selection.as_mut() {
            pending.on_decline = Some(Box::new(move |game: &mut crate::game::Game| {
                let mut decline_ctx = EffectContext::new_with_source_kind(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    player,
                );
                let mut b = bindings_for_decline.clone();
                run_tail_preserving_trigger_context(
                    &mut decline_ctx,
                    trigger_for_decline.clone(),
                    &tail_for_decline,
                    &mut b,
                    &runtime_for_decline,
                );
            }));
        }
    }
    // Park the data frame alongside the closure (coexistence): driven by
    // `run_resume`'s FieldPermanent arm. Decline mirrors the on_decline above —
    // `continue_on_decline` runs the same tail (no bind, no cost-abort).
    if ctx.game.pending_selection.is_some() {
        let decline = if optional && continue_on_decline {
            crate::resume::ResumeDecline::RunTail {
                tail: Arc::clone(&tail_for_resume),
                aborts_clause: false,
            }
        } else {
            crate::resume::ResumeDecline::None
        };
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RunTail {
                prov: crate::resume::ResumeProvenance {
                    source_card,
                    source_permanent,
                    source_kind,
                    controller: player,
                    override_pin,
                },
                select_kind: crate::resume::ResumeSelectKind::FieldPermanent {
                    of_player: target_player,
                    post: None,
                },
                bind_as: bind_as_for_resume,
                inner_tail: tail_for_resume,
                outer_conts: Vec::new(),
                bindings: bindings_for_resume,
                runtime: runtime_for_resume,
                trigger_context: trigger_for_resume,
                decline,
            }],
        });
    }
}

/// Install the Tamer pick for `trash_bottom_face_down_source_under_tamer`.
///
/// Mirrors `install_select_own_permanent` but with a fixed predicate and no
/// `selector` / `bind_as` / `optional`. The empty-candidates short-circuit is
/// NOT here — the caller (`try_install`) already handled the no-eligible-Tamer
/// case by returning `TailAlreadyRan`. The pick is mandatory (`optional:
/// false`): once the player activates the clause, they must choose which
/// eligible Tamer pays the cost. Whether to activate/decline at all is the
/// clause's own `optional`, governed one level up.
///
/// The callback trashes the picked Tamer's bottom face-down source via
/// `EffectContext::trash_bottom_face_down_source`; the captured tail runs ONLY
/// if that trash succeeded. The eligibility filter (`has_face_down_source`)
/// matches a Tamer if ANY source is face-down, but `trash_bottom_face_down_source`
/// only succeeds when `card_sources[0]` (the BOTTOM) is face-down. In the
/// current ST-23/ST-24 pool these always agree (stashes insert at index 0), but
/// this substrate is reusable: guarding the tail on the trash's `bool` keeps a
/// future filter/action desync from running the effect without paying the cost
/// (a no-approximations violation). The `debug_assert!` makes such a desync
/// fail loudly in dev/test; production degrades gracefully by skipping the tail.
fn install_trash_bottom_face_down_source_under_tamer(
    ctx: &mut EffectContext<'_>,
    filter: CompiledPredicate,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    // Resumable-VM: capture for the FieldPermanent{post:TrashBottomFaceDownSource}
    // frame before the closures consume tail/bindings/runtime/trigger. of_player =
    // ctx.player (select_own_permanent selects own field).
    let override_pin = ctx.override_selecting_player();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    ctx.select_own_permanent(
        "Choose a Tamer to trash a face-down card from under",
        optional,
        move |game, handle| {
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::Permanent(handle),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, handle: PermanentHandle| {
            let trashed = cb_ctx.trash_bottom_face_down_source(handle);
            debug_assert!(
                trashed,
                "trash_bottom_face_down_source_under_tamer: eligibility filter \
                 (has_face_down_source) offered a Tamer whose bottom source is not \
                 face-down — filter and action have desynced"
            );
            // No-approximations: the tail (the effect) runs ONLY if the cost was
            // actually paid. A `false` return means nothing was trashed, so the
            // tail must not run.
            if trashed {
                let mut b = bindings.clone();
                run_tail_preserving_trigger_context(
                    cb_ctx,
                    trigger_context,
                    &tail,
                    &mut b,
                    &runtime,
                );
            }
        },
    );
    // Park the data frame alongside the closure (coexistence): driven by
    // run_resume's FieldPermanent{post:Some(TrashBottomFaceDownSource)} arm
    // (trash the picked Tamer's bottom face-down source, then cost-gated tail).
    // install_field_selection sets on_decline:None → ResumeDecline::None.
    if ctx.game.pending_selection.is_some() {
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RunTail {
                prov: crate::resume::ResumeProvenance {
                    source_card,
                    source_permanent,
                    source_kind,
                    controller: player,
                    override_pin,
                },
                select_kind: crate::resume::ResumeSelectKind::FieldPermanent {
                    of_player: player,
                    post: Some(crate::resume::FieldPermanentPostAction::TrashBottomFaceDownSource),
                },
                bind_as: None,
                inner_tail: tail_for_resume,
                outer_conts: Vec::new(),
                bindings: bindings_for_resume,
                runtime: runtime_for_resume,
                trigger_context: trigger_for_resume,
                decline: crate::resume::ResumeDecline::None,
            }],
        });
    }
}

/// The controller's battle-area Digimon that carry ≥1 link card — the candidate
/// set for the `trash_link_card_of_own_digimon` cost's first selection (DCGO
/// `CanSelectPermanentCondition = IsPermanentExistsOnOwnerBattleAreaDigimon
/// && !HasNoLinkCards`). No compiled predicate is used: `linked_cards` is a
/// direct per-permanent field, so a manual scan is both simplest and avoids
/// widening the predicate surface for a single cost step.
fn own_digimon_with_link_cards(ctx: &EffectContext<'_>, player: PlayerId) -> Vec<PermanentHandle> {
    ctx.game
        .player(player)
        .battle_area
        .iter()
        .enumerate()
        .filter(|(_, perm)| perm.is_digimon(&ctx.game.card_data) && !perm.linked_cards.is_empty())
        .map(|(index, _)| PermanentHandle {
            player,
            index: index as u8,
        })
        .collect()
}

/// The UNION of a permanent's trashable-Option candidates for the
/// `trash_option_from_own_stacks` cost: every below-top digivolution SOURCE that
/// is an Option (tagged `Digivolution`) plus every LINK CARD that is an Option
/// (tagged `Link`). Mirrors DCGO `permanent.DigivolutionOrLinkCards.Where(IsOption)`
/// (`DigivolutionCards` excludes the top card, so only below-top sources qualify).
fn stack_option_candidates(
    perm: &crate::permanent::Permanent,
    data: &[crate::card_data::CardData],
) -> Vec<(
    crate::card_source::CardHandle,
    crate::resume::StackOptionZone,
)> {
    let mut out = Vec::new();
    let n = perm.card_sources.len();
    if n > 1 {
        for source in perm.card_sources.iter().take(n - 1) {
            if source.is_option(data) {
                out.push((
                    source.handle(),
                    crate::resume::StackOptionZone::Digivolution,
                ));
            }
        }
    }
    for linked in perm.linked_cards.iter() {
        if linked.is_option(data) {
            out.push((linked.handle(), crate::resume::StackOptionZone::Link));
        }
    }
    out
}

/// The controller's battle-area Digimon whose digivolution cards (below the top)
/// OR link cards carry ≥1 Option — the candidate set for the
/// `trash_option_from_own_stacks` cost's first selection (DCGO
/// `PermanentWithTrashableCard = IsPermanentExistsOnOwnerBattleAreaDigimon
/// && DigivolutionOrLinkCards.Any(IsOption)`).
fn own_digimon_with_stack_options(
    ctx: &EffectContext<'_>,
    player: PlayerId,
) -> Vec<PermanentHandle> {
    ctx.game
        .player(player)
        .battle_area
        .iter()
        .enumerate()
        .filter(|(_, perm)| {
            perm.is_digimon(&ctx.game.card_data)
                && !stack_option_candidates(perm, &ctx.game.card_data).is_empty()
        })
        .map(|(index, _)| PermanentHandle {
            player,
            index: index as u8,
        })
        .collect()
}

/// Install the FIRST selection of the `trash_option_from_own_stacks` activation
/// cost: pick one of `player`'s Digimon whose digivolution/link cards carry an
/// Option. On pick, [`install_stack_option_trash_second_selection`] installs the
/// SECOND selection (which Option among ITS digivolution + link cards to trash).
/// Clone-safe: the first pick parks a `RunTail`/`FieldPermanent{post:
/// SelectAndTrashStackOption}` frame, and the second pick a
/// `TrashOptionFromStackSelection` frame — no bespoke closure-only park.
/// `G-DSL-TRASH-OPTION-FROM-SOURCES-AS-COST`.
fn install_trash_option_from_own_stacks(
    ctx: &mut EffectContext<'_>,
    player: PlayerId,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let override_pin = ctx.override_selecting_player();

    // Closure-path captures (non-cloned resolution).
    let tail_cb = Arc::clone(&tail);
    let bindings_cb = bindings.clone();
    let runtime_cb = runtime.clone();
    let trigger_cb = trigger_context.clone();

    // Resume-frame captures (cloned resolution).
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();

    ctx.select_own_permanent(
        "Choose 1 of your Digimon to trash an Option from",
        optional,
        move |game, handle| {
            game.player(handle.player)
                .battle_area
                .get(handle.index as usize)
                .is_some_and(|perm| {
                    perm.is_digimon(&game.card_data)
                        && !stack_option_candidates(perm, &game.card_data).is_empty()
                })
        },
        move |cb_ctx, host: PermanentHandle| {
            install_stack_option_trash_second_selection(
                cb_ctx,
                host,
                optional,
                Arc::clone(&tail_cb),
                bindings_cb.clone(),
                runtime_cb.clone(),
                trigger_cb.clone(),
                Vec::new(),
            );
        },
    );
    // Park the first-selection data frame (clone-safe): driven by run_resume's
    // FieldPermanent{post: SelectAndTrashStackOption} arm.
    if ctx.game.pending_selection.is_some() {
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RunTail {
                prov: crate::resume::ResumeProvenance {
                    source_card,
                    source_permanent,
                    source_kind,
                    controller: player,
                    override_pin,
                },
                select_kind: crate::resume::ResumeSelectKind::FieldPermanent {
                    of_player: player,
                    post: Some(
                        crate::resume::FieldPermanentPostAction::SelectAndTrashStackOption {
                            optional,
                        },
                    ),
                },
                bind_as: None,
                inner_tail: tail_for_resume,
                outer_conts: Vec::new(),
                bindings: bindings_for_resume,
                runtime: runtime_for_resume,
                trigger_context: trigger_for_resume,
                decline: crate::resume::ResumeDecline::None,
            }],
        });
    }
}

/// Install the SECOND selection of the `trash_option_from_own_stacks` cost: pick
/// which Option — among the UNION of `host`'s digivolution-source Options +
/// link-card Options — to trash, then run the cost-gated `tail` only if a card
/// was trashed. The correct per-zone trash primitive fires: a digivolution-
/// source Option via `trash_specific_source_card` (fires `OnDigivolutionCardTrashed`),
/// a link-card Option via `trash_specific_link_card` (fires `OnLinkedCardTrashed`).
/// Shared by the closure path and the resume path. Clone-safe: parks a
/// `TrashOptionFromStackSelection` frame alongside the closure.
/// `G-DSL-TRASH-OPTION-FROM-SOURCES-AS-COST`.
#[allow(clippy::too_many_arguments)]
fn install_stack_option_trash_second_selection(
    ctx: &mut EffectContext<'_>,
    host: PermanentHandle,
    optional: bool,
    tail: Arc<Vec<CompiledStep>>,
    bindings: Bindings,
    runtime: StepRuntime,
    trigger_context: Option<crate::trigger_context::TriggerContext>,
    outer_conts: Vec<crate::resume::OuterContinuation>,
) {
    let Some(perm) = ctx
        .game
        .player(host.player)
        .battle_area
        .get(host.index as usize)
    else {
        return;
    };
    let cards = stack_option_candidates(perm, &ctx.game.card_data);
    if cards.is_empty() {
        return;
    }
    let labels: Vec<String> = cards
        .iter()
        .map(|(h, _)| {
            ctx.game
                .card_data_for_handle(*h)
                .map(|d| d.card_name.clone())
                .unwrap_or_else(|| "Option".to_string())
        })
        .collect();

    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let override_pin = ctx.override_selecting_player();

    // Closure-path captures.
    let cards_cb = cards.clone();
    let tail_cb = Arc::clone(&tail);
    let bindings_cb = bindings.clone();
    let runtime_cb = runtime.clone();
    let trigger_cb = trigger_context.clone();
    let outer_cb = outer_conts.clone();

    ctx.select_effect_choice(
        "Choose 1 Option to trash (activation cost)",
        labels,
        move |cb_ctx, idx| {
            let Some((card, zone)) = cards_cb.get(idx).copied() else {
                return;
            };
            // No-approximations cost gate: the tail runs ONLY if the Option was
            // actually trashed. Route to the per-zone trash primitive so the
            // correct observer fires.
            let trashed = match zone {
                crate::resume::StackOptionZone::Digivolution => {
                    cb_ctx.game.trash_specific_source_card(host, card)
                }
                crate::resume::StackOptionZone::Link => {
                    cb_ctx.game.trash_specific_link_card(host, card)
                }
            };
            if trashed {
                let mut b = bindings_cb.clone();
                run_tail_preserving_trigger_context(
                    cb_ctx,
                    trigger_cb.clone(),
                    &tail_cb,
                    &mut b,
                    &runtime_cb,
                );
            }
            run_outer_conts(cb_ctx.game, outer_cb.clone());
        },
    );
    // Optional decline: PASS on the Option pick skips the trash and the tail
    // (DCGO `canNoSelect: true`).
    if optional {
        if let Some(pending) = ctx.game.pending_selection.as_mut() {
            let outer_decline = outer_conts.clone();
            pending.on_decline = Some(Box::new(move |game: &mut crate::game::Game| {
                run_outer_conts(game, outer_decline.clone());
            }));
        }
    }
    // Park the data frame (clone-safe): driven by run_resume's
    // TrashOptionFromStackSelection arm.
    if ctx.game.pending_selection.is_some() {
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::TrashOptionFromStackSelection(
                crate::resume::TrashOptionFromStackSelectionState {
                    prov: crate::resume::ResumeProvenance {
                        source_card,
                        source_permanent,
                        source_kind,
                        controller: ctx.player,
                        override_pin,
                    },
                    host,
                    cards,
                    tail,
                    bindings,
                    runtime,
                    trigger_context,
                    outer_conts,
                },
            )],
        });
    }
}

/// Resume-path driver for the SECOND selection of `trash_option_from_own_stacks`
/// (`TrashOptionFromStackSelection` frame). Mirrors the closure callback in
/// [`install_stack_option_trash_second_selection`].
fn run_trash_option_from_stack_selection_step(
    game: &mut crate::game::Game,
    state: crate::resume::TrashOptionFromStackSelectionState,
    action_id: u16,
    is_pass: bool,
) {
    if is_pass {
        // Declined Option pick: no trash, tail skipped; still run outer conts.
        run_outer_conts(game, state.outer_conts);
        return;
    }
    let choice = action_id.saturating_sub(crate::action::space::HAND_EFFECT_START) as usize;
    let Some((card, zone)) = state.cards.get(choice).copied() else {
        run_outer_conts(game, state.outer_conts);
        return;
    };
    let mut ctx = EffectContext::new_with_source_kind_and_override(
        game,
        state.prov.source_card,
        state.prov.source_permanent,
        state.prov.source_kind,
        state.prov.controller,
        state.prov.override_pin,
    );
    let trashed = match zone {
        crate::resume::StackOptionZone::Digivolution => {
            ctx.game.trash_specific_source_card(state.host, card)
        }
        crate::resume::StackOptionZone::Link => ctx.game.trash_specific_link_card(state.host, card),
    };
    if trashed {
        let mut b = state.bindings.clone();
        run_tail_preserving_trigger_context(
            &mut ctx,
            state.trigger_context,
            &state.tail,
            &mut b,
            &state.runtime,
        );
    }
    run_outer_conts(ctx.game, state.outer_conts);
}

/// Install the FIRST selection of the `trash_link_card_of_own_digimon`
/// activation cost: pick one of `player`'s Digimon with ≥1 link card. On pick,
/// [`install_link_card_trash_second_selection`] installs the SECOND selection
/// (which of ITS link cards to trash). The whole thing is clone-safe: the first
/// pick parks a `RunTail`/`FieldPermanent{post: SelectAndTrashLinkCard}` frame,
/// and the second pick a `TrashLinkCardOfDigimonSelection` frame — no bespoke
/// closure-only park. `G-DSL-LINK-TRASH-AS-COST`.
fn install_trash_link_card_of_own_digimon(
    ctx: &mut EffectContext<'_>,
    player: PlayerId,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let controller = ctx.player;
    let override_pin = ctx.override_selecting_player();

    // Closure-path captures (non-cloned resolution).
    let tail_cb = Arc::clone(&tail);
    let bindings_cb = bindings.clone();
    let runtime_cb = runtime.clone();
    let trigger_cb = trigger_context.clone();

    // Resume-frame captures (cloned resolution).
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();

    ctx.select_own_permanent(
        "Choose 1 of your Digimon to trash a link card from",
        optional,
        move |game, handle| {
            game.player(handle.player)
                .battle_area
                .get(handle.index as usize)
                .is_some_and(|perm| {
                    perm.is_digimon(&game.card_data) && !perm.linked_cards.is_empty()
                })
        },
        move |cb_ctx, host: PermanentHandle| {
            install_link_card_trash_second_selection(
                cb_ctx,
                host,
                optional,
                Arc::clone(&tail_cb),
                bindings_cb.clone(),
                runtime_cb.clone(),
                trigger_cb.clone(),
                Vec::new(),
            );
        },
    );
    // Park the first-selection data frame alongside the closure (coexistence):
    // driven by run_resume's FieldPermanent{post: SelectAndTrashLinkCard} arm,
    // which installs the second selection identically to the closure above.
    if ctx.game.pending_selection.is_some() {
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RunTail {
                prov: crate::resume::ResumeProvenance {
                    source_card,
                    source_permanent,
                    source_kind,
                    controller,
                    override_pin,
                },
                select_kind: crate::resume::ResumeSelectKind::FieldPermanent {
                    of_player: player,
                    post: Some(
                        crate::resume::FieldPermanentPostAction::SelectAndTrashLinkCard {
                            optional,
                        },
                    ),
                },
                bind_as: None,
                inner_tail: tail_for_resume,
                outer_conts: Vec::new(),
                bindings: bindings_for_resume,
                runtime: runtime_for_resume,
                trigger_context: trigger_for_resume,
                decline: crate::resume::ResumeDecline::None,
            }],
        });
    }
}

/// Install the SECOND selection of the `trash_link_card_of_own_digimon` cost:
/// pick which of `host`'s link cards to trash, then run the cost-gated `tail`
/// only if a card was trashed. Shared by the closure path
/// ([`install_trash_link_card_of_own_digimon`]'s callback) and the resume path
/// (the `SelectAndTrashLinkCard` post-action) so both drive it identically.
/// Clone-safe: the selection parks a `TrashLinkCardOfDigimonSelection` frame
/// (data-only) alongside the closure. `G-DSL-LINK-TRASH-AS-COST`.
#[allow(clippy::too_many_arguments)]
fn install_link_card_trash_second_selection(
    ctx: &mut EffectContext<'_>,
    host: PermanentHandle,
    optional: bool,
    tail: Arc<Vec<CompiledStep>>,
    bindings: Bindings,
    runtime: StepRuntime,
    trigger_context: Option<crate::trigger_context::TriggerContext>,
    outer_conts: Vec<crate::resume::OuterContinuation>,
) {
    let Some(perm) = ctx
        .game
        .player(host.player)
        .battle_area
        .get(host.index as usize)
    else {
        return;
    };
    if perm.linked_cards.is_empty() {
        return;
    }
    let cards: Vec<crate::card_source::CardHandle> =
        perm.linked_cards.iter().map(|c| c.handle()).collect();
    let labels: Vec<String> = cards
        .iter()
        .map(|h| {
            ctx.game
                .card_data_for_handle(*h)
                .map(|d| d.card_name.clone())
                .unwrap_or_else(|| "Link card".to_string())
        })
        .collect();

    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let controller = ctx.player;
    let override_pin = ctx.override_selecting_player();

    // Closure-path captures.
    let cards_cb = cards.clone();
    let tail_cb = Arc::clone(&tail);
    let bindings_cb = bindings.clone();
    let runtime_cb = runtime.clone();
    let trigger_cb = trigger_context.clone();
    let outer_cb = outer_conts.clone();

    ctx.select_effect_choice(
        "Choose 1 link card to trash (activation cost)",
        labels,
        move |cb_ctx, idx| {
            let Some(card) = cards_cb.get(idx).copied() else {
                return;
            };
            // No-approximations cost gate: the tail runs ONLY if a link card was
            // actually trashed. `trash_specific_link_card` fires OnLinkedCardTrashed.
            if cb_ctx.game.trash_specific_link_card(host, card) {
                let mut b = bindings_cb.clone();
                run_tail_preserving_trigger_context(
                    cb_ctx,
                    trigger_cb.clone(),
                    &tail_cb,
                    &mut b,
                    &runtime_cb,
                );
            }
            run_outer_conts(cb_ctx.game, outer_cb.clone());
        },
    );
    // Optional decline: PASS on the link-card pick skips the trash and the tail
    // (DCGO BT25_073.cs:99 `canNoSelect: () => true` on the SelectCardEffect).
    // `select_effect_choice` installs `is_optional: false` by default and
    // `resolve_generic_selection` rejects PASS on a non-optional selection, so
    // the flag must be flipped alongside `on_decline` or the pick is silently
    // mandatory (same bug class as the trash_option_from_own_stacks sibling).
    if optional {
        if let Some(pending) = ctx.game.pending_selection.as_mut() {
            pending.is_optional = true;
            let outer_decline = outer_conts.clone();
            pending.on_decline = Some(Box::new(move |game: &mut crate::game::Game| {
                run_outer_conts(game, outer_decline.clone());
            }));
        }
    }
    // Park the data frame (clone-safe): driven by run_resume's
    // TrashLinkCardOfDigimonSelection arm.
    if ctx.game.pending_selection.is_some() {
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::TrashLinkCardOfDigimonSelection(
                crate::resume::TrashLinkCardOfDigimonSelectionState {
                    prov: crate::resume::ResumeProvenance {
                        source_card,
                        source_permanent,
                        source_kind,
                        controller,
                        override_pin,
                    },
                    host,
                    cards,
                    tail,
                    bindings,
                    runtime,
                    trigger_context,
                    outer_conts,
                },
            )],
        });
    }
}

/// Total bottom-reachable face-down digivolution sources across `player`'s
/// Tamers — the eligibility gate for the multi-count trash cost. A "face-down
/// source" is any `CardSource` marked `face_down`; in the BEATBREAK / DATA
/// SQUAD stash family these always sit contiguously from the bottom of the
/// stack (stashes insert at index 0), so the count equals the number of
/// reachable bottom-trash picks.
fn total_face_down_sources_under_tamers(
    ctx: &EffectContext<'_>,
    player: PlayerId,
    bindings: &Bindings,
) -> usize {
    let filter = CompiledPredicate {
        kind: Some(digimon_dsl::compiled::CompiledCardKind::Tamer),
        has_face_down_source: Some(true),
        ..CompiledPredicate::default()
    };
    let tamers = collect_matching_permanents(ctx, player, &filter, Some(bindings));
    tamers
        .iter()
        .map(|handle| {
            ctx.game
                .player(handle.player)
                .battle_area
                .get(handle.index as usize)
                .map(|perm| perm.card_sources.iter().filter(|s| s.face_down).count())
                .unwrap_or(0)
        })
        .sum()
}

/// Install the next Tamer pick for the multi-count
/// `trash_bottom_face_down_sources_under_tamers` cost. `remaining` is the
/// number of bottom-face-down sources still to trash. Each pick trashes ONE
/// bottom face-down source from the chosen Tamer; the callback then either
/// re-installs the next pick (`remaining - 1 > 0`) or runs the captured `tail`
/// (the rest of the clause — the free digivolve). Eligible Tamers are
/// re-evaluated before every pick, so a single Tamer holding ≥`remaining`
/// face-down sources can be re-picked to satisfy the whole cost, and the "1
/// from each of two Tamers" distribution is equally reachable. The caller
/// (`try_install`) has already verified `remaining` total face-down sources
/// exist, so each install finds ≥1 eligible Tamer and always parks.
fn install_trash_n_bottom_face_down_sources_under_tamers(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    player: PlayerId,
    remaining: u8,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let filter = CompiledPredicate {
        kind: Some(digimon_dsl::compiled::CompiledCardKind::Tamer),
        has_face_down_source: Some(true),
        ..CompiledPredicate::default()
    };
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player_for_filter = ctx.player;
    let filter_for_pred = filter.clone();
    let filter_bindings = bindings.clone();
    // Resumable-VM (make-engine-cloneable): capture for the data frame before
    // the closures consume tail/bindings/runtime/trigger. The whole multi-pick
    // cost is modeled as a single-pick FieldPermanent{post:TrashBottomFaceDownSource}
    // frame (the slice #4 arm) whose continuation re-enters
    // `TrashBottomFaceDownSourcesUnderTamers{count: remaining-1}` until the cost
    // is fully paid — so a per-pick trash that parks a nested OnDigivolutionCard-
    // Trashed observer threads via the SAME park_pending_selection_tail recursion
    // the single-pick under-tamer trash uses, with no bespoke continuation channel.
    let override_pin = ctx.override_selecting_player();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    ctx.select_own_permanent(
        "Choose a Tamer to trash a face-down card from under",
        false,
        move |game, handle| {
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player_for_filter,
            );
            eval_predicate_with_bindings(
                &filter_for_pred,
                &read_ctx,
                PredicateSubject::Permanent(handle),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, handle: PermanentHandle| {
            let trashed = cb_ctx.trash_bottom_face_down_source(handle);
            debug_assert!(
                trashed,
                "trash_bottom_face_down_sources_under_tamers: eligibility filter \
                 (has_face_down_source) offered a Tamer whose bottom source is not \
                 face-down — filter and action have desynced"
            );
            if !trashed {
                // No-approximations: a desync means nothing was paid; do NOT run
                // the tail and do NOT re-install (which would offer a free
                // digivolve for an unpaid cost).
                return;
            }
            let now_remaining = remaining.saturating_sub(1);
            if now_remaining == 0 {
                // Cost fully paid: run the captured tail (the free digivolve).
                let mut b = bindings.clone();
                run_tail_preserving_trigger_context(
                    cb_ctx,
                    trigger_context.clone(),
                    &tail,
                    &mut b,
                    &runtime,
                );
            } else {
                // More to trash: re-install the next Tamer pick. Eligibility is
                // re-evaluated (the just-trashed Tamer may have dropped below 1
                // face-down source). The earlier total-source gate guarantees
                // enough sources remain.
                install_trash_n_bottom_face_down_sources_under_tamers(
                    cb_ctx,
                    of,
                    player,
                    now_remaining,
                    tail.as_ref().clone(),
                    bindings.clone(),
                    runtime.clone(),
                );
            }
        },
    );
    // Park the data frame alongside the closure (coexistence): driven by
    // run_resume's FieldPermanent{post:Some(TrashBottomFaceDownSource)} arm. The
    // frame trashes ONE bottom face-down source from the picked Tamer, then runs
    // the continuation: when `remaining > 1`, re-enter the multi-trash step for
    // the next `remaining - 1` (which re-parks the next pick); at `remaining ==
    // 1` run the real captured tail (the free digivolve). `select_own_permanent`
    // installs a MANDATORY pick (the caller pre-gated ≥`remaining` sources exist,
    // so ≥1 eligible Tamer), so on_decline is None ⇒ ResumeDecline::None. Whether
    // to activate the whole cost at all is the clause's `optional`, one level up.
    if ctx.game.pending_selection.is_some() {
        let continuation_tail: Vec<CompiledStep> = if remaining <= 1 {
            (*tail_for_resume).clone()
        } else {
            let mut v = Vec::with_capacity(1 + tail_for_resume.len());
            v.push(CompiledStep::TrashBottomFaceDownSourcesUnderTamers {
                of,
                count: remaining - 1,
            });
            v.extend_from_slice(&tail_for_resume);
            v
        };
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RunTail {
                prov: crate::resume::ResumeProvenance {
                    source_card,
                    source_permanent,
                    source_kind,
                    controller: player,
                    override_pin,
                },
                select_kind: crate::resume::ResumeSelectKind::FieldPermanent {
                    of_player: player,
                    post: Some(crate::resume::FieldPermanentPostAction::TrashBottomFaceDownSource),
                },
                bind_as: None,
                inner_tail: Arc::new(continuation_tail),
                outer_conts: Vec::new(),
                bindings: bindings_for_resume,
                runtime: runtime_for_resume,
                trigger_context: trigger_for_resume,
                decline: crate::resume::ResumeDecline::None,
            }],
        });
    }
}

fn install_select_opponent_permanent(
    ctx: &mut EffectContext<'_>,
    filter: CompiledPredicate,
    selector: Option<CompiledFieldSelector>,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    continue_on_decline: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    // Pre-filter candidates using the compiled predicate so that an empty
    // result short-circuits without installing a PendingSelection.
    let opponent = ctx.game.next_clockwise(ctx.player);
    let candidates: Vec<_> = collect_matching_permanents(ctx, opponent, &filter, Some(&bindings))
        .into_iter()
        .filter(|handle| !ctx.game.progress_excludes(*handle, Some(ctx.player)))
        .collect();
    let selected_field = select_field_extreme(ctx.game, &candidates, selector);
    if candidates.is_empty() {
        return;
    }

    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    let tail_for_decline = tail.clone();
    let bindings_for_decline = bindings.clone();
    let runtime_for_decline = runtime.clone();
    let trigger_for_decline = trigger_context.clone();
    // Resumable-VM (Batch 2): capture for the data frame; `of_player` is the
    // already-computed `opponent` (= next_clockwise(player)).
    let bind_as_for_resume = bind_as.clone();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    let override_pin = ctx.override_selecting_player();
    ctx.select_opponent_permanent(
        &prompt,
        optional,
        move |game, handle| {
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            !game.progress_excludes(handle, Some(player))
                && eval_predicate_with_bindings(
                    &filter,
                    &read_ctx,
                    PredicateSubject::Permanent(handle),
                    Some(&filter_bindings),
                )
                && matches_selected_field(game, handle, selector, selected_field)
        },
        move |cb_ctx, handle: PermanentHandle| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_permanent(name, handle);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
    // `continue_on_decline: true` — PASS leaves the binding unresolved and
    // CONTINUES the clause (same semantic the hand/trash selects carry; DCGO's
    // declined SelectPermanentEffect resolves with an empty list and the
    // coroutine continues), so binding-gated follow-ups (`binding_exists` /
    // `binding_absent`) and independent legs still execute. The default keeps
    // the historical drop-the-tail semantic existing cards rely on as their
    // accept/decline cost gate. G-OPT-REFUND-ON-DECLINE.
    if optional && continue_on_decline {
        if let Some(pending) = ctx.game.pending_selection.as_mut() {
            pending.on_decline = Some(Box::new(move |game: &mut crate::game::Game| {
                let mut decline_ctx = EffectContext::new_with_source_kind(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    player,
                );
                let mut b = bindings_for_decline.clone();
                run_tail_preserving_trigger_context(
                    &mut decline_ctx,
                    trigger_for_decline.clone(),
                    &tail_for_decline,
                    &mut b,
                    &runtime_for_decline,
                );
            }));
        }
    }
    // Park the data frame alongside the closure (coexistence): driven by
    // `run_resume`'s FieldPermanent arm with `of_player: opponent`.
    if ctx.game.pending_selection.is_some() {
        let decline = if optional && continue_on_decline {
            crate::resume::ResumeDecline::RunTail {
                tail: Arc::clone(&tail_for_resume),
                aborts_clause: false,
            }
        } else {
            crate::resume::ResumeDecline::None
        };
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RunTail {
                prov: crate::resume::ResumeProvenance {
                    source_card,
                    source_permanent,
                    source_kind,
                    controller: player,
                    override_pin,
                },
                select_kind: crate::resume::ResumeSelectKind::FieldPermanent {
                    of_player: opponent,
                    post: None,
                },
                bind_as: bind_as_for_resume,
                inner_tail: tail_for_resume,
                outer_conts: Vec::new(),
                bindings: bindings_for_resume,
                runtime: runtime_for_resume,
                trigger_context: trigger_for_resume,
                decline,
            }],
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn install_select_any_permanent(
    ctx: &mut EffectContext<'_>,
    filter: CompiledPredicate,
    excluded: Option<PermanentHandle>,
    selector: Option<CompiledFieldSelector>,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    use crate::action::space::encode_attack;

    let handles = collect_matching_any_permanents(ctx, excluded, &filter, Some(&bindings));
    let selected_field = select_field_extreme(ctx.game, &handles, selector);
    let candidates: Vec<(u16, PermanentHandle)> = handles
        .into_iter()
        .filter(|handle| matches_selected_field(ctx.game, *handle, selector, selected_field))
        .map(|handle| {
            (
                encode_attack(handle.player as u16, handle.index as u16),
                handle,
            )
        })
        .collect();

    if candidates.is_empty() {
        return;
    }

    let valid_action_ids = candidates.iter().map(|(action, _)| *action).collect();
    let selecting_player = ctx.override_selecting_player().unwrap_or(ctx.player);
    let controller = ctx.player;
    let override_pin = ctx.override_selecting_player();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    // Resumable-VM (Batch 2): clone for the data frame before the closure
    // consumes them. The AnyPermanent arm linear-searches the captured
    // candidates (heterogeneous both-player domain), so it carries the full vec.
    let candidates_for_resume = candidates.clone();
    let bind_as_for_resume = bind_as.clone();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();

    let previous_phase = ctx.game.current_phase;
    ctx.game.current_phase = GamePhase::SelectTarget;
    ctx.game.pending_selection = Some(PendingSelection {
        zone_owner: None,
        // AnyField (not Target): candidates span BOTH battle areas and are
        // encoded `encode_attack(player, index)`, so the UI decodes the side
        // from the id. Routing this as `Target` left the board unable to map
        // clicks (its field helpers only handle OwnField/OppField) — the
        // EX8-028 "place 1 Digimon as bottom security" softlock.
        kind: SelectionKind::AnyField,
        selecting_player,
        previous_phase,
        valid_action_ids,
        is_optional: optional,
        prompt,
        effect_choices: None,
        source_card,
        source_permanent,
        source_kind,
        callback: Box::new(move |game, action_id| {
            let Some((_, handle)) = candidates
                .iter()
                .find(|(candidate_action, _)| *candidate_action == action_id)
                .copied()
            else {
                return;
            };

            let mut cb_ctx = EffectContext::new_with_source_kind_and_override(
                game,
                source_card,
                source_permanent,
                source_kind,
                controller,
                override_pin,
            );
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_permanent(name, handle);
            }
            run_tail_preserving_trigger_context(
                &mut cb_ctx,
                trigger_context,
                &tail,
                &mut b,
                &runtime,
            );
        }),
        on_decline: None,
    });
    // Park the data frame alongside the closure (coexistence): driven by
    // `run_resume`'s AnyPermanent arm (linear search over the candidates).
    if ctx.game.pending_selection.is_some() {
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RunTail {
                prov: crate::resume::ResumeProvenance {
                    source_card,
                    source_permanent,
                    source_kind,
                    controller,
                    override_pin,
                },
                select_kind: crate::resume::ResumeSelectKind::AnyPermanent {
                    candidates: candidates_for_resume,
                },
                bind_as: bind_as_for_resume,
                inner_tail: tail_for_resume,
                outer_conts: Vec::new(),
                bindings: bindings_for_resume,
                runtime: runtime_for_resume,
                trigger_context: trigger_for_resume,
                decline: crate::resume::ResumeDecline::None,
            }],
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn install_select_dna_pair(
    ctx: &mut EffectContext<'_>,
    left_filter: CompiledPredicate,
    right_filter: CompiledPredicate,
    bind_left_as: String,
    bind_right_as: String,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    use crate::action::space::encode_attack;

    let candidates: Vec<(u16, PermanentHandle)> = {
        let read = ctx.as_read();
        let mut candidates = Vec::new();
        for player in 0..read.game.players.len() {
            let player = player as u8;
            for index in 0..read.game.player(player).battle_area.len() {
                let handle = PermanentHandle {
                    player,
                    index: index as u8,
                };
                if eval_predicate(&left_filter, &read, PredicateSubject::Permanent(handle)) {
                    candidates.push((encode_attack(player as u16, index as u16), handle));
                }
            }
        }
        candidates
    };

    if candidates.is_empty() {
        return;
    }

    let valid_action_ids = candidates.iter().map(|(action, _)| *action).collect();
    let selecting_player = ctx.override_selecting_player().unwrap_or(ctx.player);
    let controller = ctx.player;
    let override_pin = ctx.override_selecting_player();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let previous_phase = ctx.game.current_phase;

    // Resumable-VM: capture for the RunTail{DnaPairLeft} data frame before the
    // LEFT closure moves these. The right pick chains into the already-flipped
    // install_select_any_permanent, so only the LEFT frame is new here.
    let candidates_for_resume = candidates.clone();
    let right_filter_for_resume = right_filter.clone();
    let bind_left_as_for_resume = bind_left_as.clone();
    let bind_right_as_for_resume = bind_right_as.clone();
    let right_prompt_for_resume = prompt.clone();
    let tail_for_resume = Arc::new(tail.clone());
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = ctx.game.current_trigger_context.clone();

    ctx.game.current_phase = GamePhase::SelectTarget;
    ctx.game.pending_selection = Some(PendingSelection {
        zone_owner: None,
        // AnyField: the DNA-pair left pick spans both battle areas, same
        // `encode_attack(player, index)` encoding as select_any_permanent (the
        // right pick chains into install_select_any_permanent, also AnyField).
        kind: SelectionKind::AnyField,
        selecting_player,
        previous_phase,
        valid_action_ids,
        is_optional: optional,
        prompt: prompt.clone(),
        effect_choices: None,
        source_card,
        source_permanent,
        source_kind,
        callback: Box::new(move |game, action_id| {
            let Some((_, left)) = candidates
                .iter()
                .find(|(candidate_action, _)| *candidate_action == action_id)
                .copied()
            else {
                return;
            };

            let mut cb_ctx = EffectContext::new_with_source_kind_and_override(
                game,
                source_card,
                source_permanent,
                source_kind,
                controller,
                override_pin,
            );
            let mut b = bindings.clone();
            b.insert_permanent(&bind_left_as, left);
            install_select_any_permanent(
                &mut cb_ctx,
                right_filter,
                Some(left),
                None,
                Some(bind_right_as),
                prompt,
                optional,
                tail,
                b,
                runtime,
            );
        }),
        on_decline: None,
    });
    // Park the data frame alongside the closure (coexistence): driven by
    // run_resume's DnaPairLeft arm (resolve left → bind via bind_as → chain into
    // install_select_any_permanent for the right pick). The right pick parks its
    // own AnyPermanent frame; this frame's inner_tail becomes the right tail.
    // install_field_selection / the left install set on_decline:None → decline None.
    if ctx.game.pending_selection.is_some() {
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RunTail {
                prov: crate::resume::ResumeProvenance {
                    source_card,
                    source_permanent,
                    source_kind,
                    controller,
                    override_pin,
                },
                select_kind: crate::resume::ResumeSelectKind::DnaPairLeft {
                    candidates: candidates_for_resume,
                    right_filter: right_filter_for_resume,
                    bind_right_as: bind_right_as_for_resume,
                    right_prompt: right_prompt_for_resume,
                    optional,
                },
                bind_as: Some(bind_left_as_for_resume),
                inner_tail: tail_for_resume,
                outer_conts: Vec::new(),
                bindings: bindings_for_resume,
                runtime: runtime_for_resume,
                trigger_context: trigger_for_resume,
                decline: crate::resume::ResumeDecline::None,
            }],
        });
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
fn install_select_count_capped_multi(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    zone: CompiledZone,
    min: u8,
    max: u8,
    clamp_to_available: bool,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
    optional_zero: bool,
    distinct_by: Option<DistinctByMode>,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let target_player = resolve_player(ctx, of);
    if max == 0 {
        let mut b = bindings.clone();
        if let Some(name) = &bind_as {
            match zone {
                CompiledZone::BattleArea => b.insert_permanent_list(name, Vec::new()),
                _ => b.insert_card_list(name, Vec::new()),
            }
        }
        run_tail_preserving_trigger_context(
            ctx,
            ctx.game.current_trigger_context.clone(),
            &tail,
            &mut b,
            &runtime,
        );
        return;
    }
    if matches!(zone, CompiledZone::BattleArea) {
        install_select_count_capped_permanents(
            ctx,
            target_player,
            min,
            max,
            clamp_to_available,
            filter,
            bind_as,
            prompt,
            optional_zero,
            tail,
            bindings,
            runtime,
        );
        return;
    }
    let engine_zone = match zone {
        CompiledZone::Hand => CountCappedZone::Hand,
        CompiledZone::Trash => CountCappedZone::Trash,
        // Phase 2d scope: only Hand/Trash supported. Other zones (Materials,
        // Security, Reveal, Source, Field, Deck, Breeding) silently no-op
        // here; Phase 2e+ adds the missing engine API hooks.
        _ => return,
    };
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    // Resumable-VM (Batch 4): capture for the MultiPickStep frame (Hand/Trash
    // card-based count_capped). filter carried as a CompiledPredicate (data-pure).
    let filter_for_resume = filter.clone();
    let filter_bindings_for_resume = bindings.clone();
    let bind_as_for_resume = bind_as.clone();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    let override_pin = ctx.override_selecting_player();
    ctx.select_count_capped_multi_min(
        target_player,
        engine_zone,
        min,
        max,
        &prompt,
        optional_zero,
        distinct_by,
        move |game, card| {
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::Card(card.handle()),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, picks| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_card_list(name, picks);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
    // Park the MultiPickStep data frame. Clobber guard: only park when OUR
    // candidate set is non-empty (empty ⇒ the closure ran the tail inline; and a
    // `candidates < min` short-circuit installs no pending so the `Some(pending)`
    // guard skips it too).
    let candidate_indices = count_capped_card_candidate_indices(
        ctx.game,
        target_player,
        engine_zone,
        &filter_for_resume,
        &filter_bindings_for_resume,
        source_card,
        source_permanent,
        source_kind,
        player,
    );
    if !candidate_indices.is_empty() {
        if let Some(pending) = ctx.game.pending_selection.as_ref() {
            let selecting_player = pending.selecting_player;
            let previous_phase = pending.previous_phase;
            let range_start = match engine_zone {
                CountCappedZone::Hand => crate::action::space::PLAY_HAND_START,
                CountCappedZone::Trash => crate::action::space::TRASH_EFFECT_START,
                CountCappedZone::Material(ph) => {
                    crate::effect_context::selections::material_zone_geometry(ctx.game, ph)
                        .map(|(_, rs)| rs)
                        .unwrap_or(0)
                }
            };
            ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
                frames: vec![crate::resume::ResumeFrame::MultiPickStep(
                    crate::resume::MultiPickState {
                        prov: crate::resume::ResumeProvenance {
                            source_card,
                            source_permanent,
                            source_kind,
                            controller: player,
                            override_pin,
                        },
                        of_player: target_player,
                        selecting_player,
                        previous_phase,
                        zone: engine_zone,
                        range_start,
                        min,
                        max,
                        is_optional_zero: optional_zero,
                        distinct_by,
                        candidate_indices,
                        accum: Vec::new(),
                        bind_as: bind_as_for_resume,
                        inner_tail: tail_for_resume,
                        bindings: bindings_for_resume,
                        runtime: runtime_for_resume,
                        trigger_context: trigger_for_resume,
                        outer_conts: Vec::new(),
                    },
                )],
            });
        }
    }
}

/// Re-derive the filter-passing zone indices for a card-based count_capped
/// (Hand/Trash/Material), mirroring `select_count_capped_multi_min`'s install-time
/// candidate scan with the `CompiledPredicate` on `PredicateSubject::Card`.
/// Data-pure. `distinct_by` is NOT applied here (the executor applies it on
/// re-park, matching the closure).
#[allow(clippy::too_many_arguments)]
fn count_capped_card_candidate_indices(
    game: &crate::game::Game,
    of_player: PlayerId,
    zone: CountCappedZone,
    filter: &CompiledPredicate,
    filter_bindings: &Bindings,
    source_card: crate::card_source::CardHandle,
    source_permanent: Option<PermanentHandle>,
    source_kind: crate::enums::EffectSourceKind,
    player: PlayerId,
) -> Vec<usize> {
    use crate::action::space::{HAND_MAIN_LIMIT, TRASH_MAIN_LIMIT};
    let zone_len = match zone {
        CountCappedZone::Hand => game.player(of_player).hand.len().min(HAND_MAIN_LIMIT),
        CountCappedZone::Trash => game.player(of_player).trash.len().min(TRASH_MAIN_LIMIT),
        CountCappedZone::Material(ph) => {
            crate::effect_context::selections::material_zone_slice(game, ph)
                .map(|s| s.len().saturating_sub(1))
                .unwrap_or(0)
        }
    };
    // Collect handles first (release the zone borrow before the read context).
    let mut handles: Vec<crate::card_source::CardHandle> = Vec::with_capacity(zone_len);
    for i in 0..zone_len {
        let h = match zone {
            CountCappedZone::Hand => game.player(of_player).hand[i].handle(),
            CountCappedZone::Trash => game.player(of_player).trash[i].handle(),
            CountCappedZone::Material(ph) => {
                match crate::effect_context::selections::material_zone_slice(game, ph) {
                    Some(s) => s[i].handle(),
                    None => continue,
                }
            }
        };
        handles.push(h);
    }
    let read = crate::effect_context::EffectReadContext::new_with_source_kind(
        game,
        source_card,
        source_permanent,
        source_kind,
        player,
    );
    let mut out = Vec::new();
    for (i, h) in handles.into_iter().enumerate() {
        if eval_predicate_with_bindings(
            filter,
            &read,
            PredicateSubject::Card(h),
            Some(filter_bindings),
        ) {
            out.push(i);
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn install_select_count_capped_permanents(
    ctx: &mut EffectContext<'_>,
    target_player: u8,
    min: u8,
    max: u8,
    clamp_to_available: bool,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
    optional_zero: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    use crate::action::space::encode_attack;

    let candidates: Vec<(u16, PermanentHandle)> =
        collect_matching_permanents(ctx, target_player, &filter, Some(&bindings))
            .into_iter()
            .map(|handle| {
                // Encode opponent- AND own-field targets identically as
                // `encode_attack(0, slot)` = ATTACK_START + slot — the SAME
                // encoding the single-target `install_field_selection` path
                // uses. The frontend's field-click router disambiguates which
                // player's field by the `OppField`/`OwnField` selection kind
                // (set in `install_count_capped_permanent_step`), never by id
                // range. Passing the target's player id as the attacker pushed
                // opponent targets out of the field-click range and left
                // "delete an opponent's Digimon" prompts unclickable.
                (encode_attack(0, handle.index as u16), handle)
            })
            .collect();
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    // MP-30/31 (General Rules/FAQ): a mandatory "N of your opponent's Digimon"
    // effect-target selection affects `min(max, available)` — it is never
    // unpayable for "fewer than N in play", but it MUST NOT stop early when N
    // are available. Raise the required floor to `min(max, candidates)`.
    let min = if clamp_to_available {
        max.min(candidates.len() as u8)
    } else {
        min
    };
    // Fewer candidates than the required minimum → unpayable required cost;
    // silently no-op without running the tail. G-SELECT-MULTI-MIN. (Unreachable
    // when `clamp_to_available`, since the floor was clamped to `candidates`.)
    if min > 0 && candidates.len() < min as usize {
        return;
    }
    if candidates.is_empty() {
        let mut b = bindings.clone();
        if let Some(name) = &bind_as {
            b.insert_permanent_list(name, Vec::new());
        }
        run_tail_preserving_trigger_context(ctx, trigger_context, &tail, &mut b, &runtime);
        return;
    }

    let selecting_player = ctx.override_selecting_player().unwrap_or(ctx.player);
    // Targets on a player OTHER than the one making the choice render on the
    // selecting player's opponent half; drives the OppField/OwnField kind so
    // the frontend's field-click router can map board clicks to these picks.
    let target_is_opponent = target_player != selecting_player;
    let controller = ctx.player;
    let override_pin = ctx.override_selecting_player();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let previous_phase = ctx.game.current_phase;
    // Resumable-VM (Batch 4): capture for the CountCappedPermanentsStep frame
    // (snapshot-minus-picked; `min` is already clamped). candidates non-empty here
    // (the empty + `< min` cases returned above), so install always parks our
    // permanent select — no clobber risk.
    let candidates_for_resume = candidates.clone();
    let prompt_for_resume = prompt.clone();
    let bind_as_for_resume = bind_as.clone();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    let final_callback: Box<
        dyn FnOnce(&mut crate::game::Game, Vec<PermanentHandle>) + Send + Sync,
    > = Box::new(move |game, picks| {
        let mut cb_ctx = EffectContext::new_with_source_kind_and_override(
            game,
            source_card,
            source_permanent,
            source_kind,
            controller,
            override_pin,
        );
        let mut b = bindings.clone();
        if let Some(name) = &bind_as {
            b.insert_permanent_list(name, picks);
        }
        run_tail_preserving_trigger_context(&mut cb_ctx, trigger_context, &tail, &mut b, &runtime);
    });

    install_count_capped_permanent_step(
        ctx.game,
        candidates,
        Vec::new(),
        min,
        max,
        optional_zero,
        prompt,
        source_card,
        source_permanent,
        source_kind,
        selecting_player,
        target_is_opponent,
        previous_phase,
        final_callback,
    );
    if let Some(pending) = ctx.game.pending_selection.as_ref() {
        let selecting_player = pending.selecting_player;
        let previous_phase = pending.previous_phase;
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::CountCappedPermanentsStep(
                crate::resume::CountCappedPermanentsState {
                    prov: crate::resume::ResumeProvenance {
                        source_card,
                        source_permanent,
                        source_kind,
                        controller,
                        override_pin,
                    },
                    selecting_player,
                    previous_phase,
                    target_is_opponent,
                    min,
                    max,
                    optional_zero,
                    candidates: candidates_for_resume,
                    accum: Vec::new(),
                    prompt: prompt_for_resume,
                    bind_as: bind_as_for_resume,
                    inner_tail: tail_for_resume,
                    bindings: bindings_for_resume,
                    runtime: runtime_for_resume,
                    trigger_context: trigger_for_resume,
                    outer_conts: Vec::new(),
                },
            )],
        });
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
fn install_count_capped_permanent_step(
    game: &mut crate::game::Game,
    candidates: Vec<(u16, PermanentHandle)>,
    accum: Vec<PermanentHandle>,
    min: u8,
    max: u8,
    optional_zero: bool,
    prompt: String,
    source_card: crate::card_source::CardHandle,
    source_permanent: Option<PermanentHandle>,
    source_kind: crate::enums::EffectSourceKind,
    selecting_player: u8,
    target_is_opponent: bool,
    previous_phase: GamePhase,
    final_callback: Box<dyn FnOnce(&mut crate::game::Game, Vec<PermanentHandle>) + Send + Sync>,
) {
    use std::sync::{Arc, Mutex};

    let picked = accum.len() as u8;
    // PASS gating: the player may commit early once `picked` reaches the
    // required floor. G-SELECT-MULTI-MIN.
    let effective_min = min.max(if optional_zero { 0 } else { 1 });
    let is_optional = picked >= effective_min;
    let valid_action_ids = candidates.iter().map(|(action, _)| *action).collect();
    let shared_cb: Arc<
        Mutex<Option<Box<dyn FnOnce(&mut crate::game::Game, Vec<PermanentHandle>) + Send + Sync>>>,
    > = Arc::new(Mutex::new(Some(final_callback)));
    let shared_cb_decline = Arc::clone(&shared_cb);
    let accum_for_decline = accum.clone();

    game.current_phase = GamePhase::SelectBudgeted;
    game.pending_selection = Some(PendingSelection {
        zone_owner: None,
        // Reuse the single-target field-selection kind so the frontend's
        // field-click router (which keys off `OppField`/`OwnField`, not the
        // phase or id range) can map board clicks to these picks. The
        // multi-select / accumulate-and-commit semantics are carried by the
        // re-installed step + `is_optional` PASS gating, not by the kind tag.
        kind: if target_is_opponent {
            SelectionKind::OppField
        } else {
            SelectionKind::OwnField
        },
        selecting_player,
        previous_phase,
        valid_action_ids,
        is_optional,
        prompt: prompt.clone(),
        effect_choices: None,
        source_card,
        source_permanent,
        source_kind,
        callback: Box::new(move |game, action_id| {
            let Some((_, handle)) = candidates
                .iter()
                .find(|(candidate_action, _)| *candidate_action == action_id)
                .copied()
            else {
                return;
            };

            let mut new_accum = accum;
            new_accum.push(handle);
            if new_accum.len() == max as usize {
                if let Some(cb) = shared_cb.lock().unwrap().take() {
                    cb(game, new_accum);
                }
                return;
            }

            let new_candidates: Vec<(u16, PermanentHandle)> = candidates
                .into_iter()
                .filter(|(candidate_action, _)| *candidate_action != action_id)
                .collect();
            if new_candidates.is_empty() {
                if let Some(cb) = shared_cb.lock().unwrap().take() {
                    cb(game, new_accum);
                }
                return;
            }

            let next_cb: Box<
                dyn FnOnce(&mut crate::game::Game, Vec<PermanentHandle>) + Send + Sync,
            > = Box::new(move |game, picks| {
                if let Some(cb) = shared_cb.lock().unwrap().take() {
                    cb(game, picks);
                }
            });
            install_count_capped_permanent_step(
                game,
                new_candidates,
                new_accum,
                min,
                max,
                optional_zero,
                prompt,
                source_card,
                source_permanent,
                source_kind,
                selecting_player,
                target_is_opponent,
                previous_phase,
                next_cb,
            );
        }),
        on_decline: Some(Box::new(move |game| {
            if let Some(cb) = shared_cb_decline.lock().unwrap().take() {
                cb(game, accum_for_decline);
            }
        })),
    });
}

fn install_select_effect_choice(
    ctx: &mut EffectContext<'_>,
    labels: Vec<String>,
    bind_as: Option<String>,
    prompt: String,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    // Resumable-VM (post-Batch-4): capture for the RunTail/EffectChoice frame.
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let override_pin = ctx.override_selecting_player();
    let bind_as_for_resume = bind_as.clone();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    ctx.select_effect_choice(&prompt, labels, move |cb_ctx, idx| {
        let mut b = bindings.clone();
        if let Some(name) = &bind_as {
            b.insert_literal(name, idx as i64);
        }
        run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
    });
    // Park the data frame (coexistence): driven by run_resume's EffectChoice arm.
    // Not optional (a branch must be picked) → decline None.
    if ctx.game.pending_selection.is_some() {
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RunTail {
                prov: crate::resume::ResumeProvenance {
                    source_card,
                    source_permanent,
                    source_kind,
                    controller: player,
                    override_pin,
                },
                select_kind: crate::resume::ResumeSelectKind::EffectChoice { post: None },
                bind_as: bind_as_for_resume,
                inner_tail: tail_for_resume,
                outer_conts: Vec::new(),
                bindings: bindings_for_resume,
                runtime: runtime_for_resume,
                trigger_context: trigger_for_resume,
                decline: crate::resume::ResumeDecline::None,
            }],
        });
    }
}

fn install_select_reveal(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) -> bool {
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    // Resumable-VM (Batch 2): capture for the data frame. Ownership is enforced
    // in the install filter (revealed_owner_matches), so valid_action_ids is
    // already restricted; the Reveal arm just decodes + binds (stale → skip).
    let bind_as_for_resume = bind_as.clone();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    let override_pin = ctx.override_selecting_player();
    let parked = ctx.select_reveal(
        &prompt,
        optional,
        move |game, idx| {
            let Some(card) = game.revealed_cards.get(idx) else {
                return false;
            };
            if !revealed_owner_matches(of, card.owner, player, game) {
                return false;
            }
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::RevealedCard(card.handle()),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                // Resolve the picked reveal index into a stable CardHandle.
                if let Some(card) = cb_ctx.game.revealed_cards.get(idx) {
                    b.insert_card(name, card.handle());
                }
                // If the index has gone stale (the reveal pile mutated mid-
                // resolution — currently impossible but defensive), silently
                // skip the binding; downstream verbs that consume it no-op
                // per the 2b/2c missing-binding convention.
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
    // Park the data frame alongside the closure (coexistence): driven by
    // `run_resume`'s Reveal arm. No on_decline → decline None.
    if ctx.game.pending_selection.is_some() {
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RunTail {
                prov: crate::resume::ResumeProvenance {
                    source_card,
                    source_permanent,
                    source_kind,
                    controller: player,
                    override_pin,
                },
                select_kind: crate::resume::ResumeSelectKind::Reveal { route: None },
                bind_as: bind_as_for_resume,
                inner_tail: tail_for_resume,
                outer_conts: Vec::new(),
                bindings: bindings_for_resume,
                runtime: runtime_for_resume,
                trigger_context: trigger_for_resume,
                decline: crate::resume::ResumeDecline::None,
            }],
        });
    }
    parked
}

fn install_select_reveal_buckets(
    ctx: &mut EffectContext<'_>,
    from: &str,
    buckets: &[digimon_dsl::compiled::CompiledRevealBucket],
    no_duplicate_cards: bool,
    prompt: Option<String>,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) -> InstallResult {
    let source_handles = if let Some(list) = bindings.get_card_list(from) {
        list
    } else if let Some(card) = bindings.get_card(from) {
        vec![card]
    } else {
        return InstallResult::Continue;
    };

    let engine_buckets = {
        let read_ctx = ctx.as_read();
        buckets
            .iter()
            .map(|bucket| {
                let candidates = source_handles
                    .iter()
                    .copied()
                    .filter(|handle| {
                        ctx.game
                            .revealed_cards
                            .iter()
                            .any(|card| card.handle() == *handle)
                    })
                    .filter(|handle| {
                        bucket.filter.as_ref().is_none_or(|filter| {
                            eval_predicate_with_bindings(
                                filter,
                                &read_ctx,
                                PredicateSubject::RevealedCard(*handle),
                                Some(&bindings),
                            )
                        })
                    })
                    .collect::<Vec<_>>();
                RevealBucketSelection {
                    bind_as: bucket.bind_as.clone(),
                    min: bucket.min,
                    max: bucket.max,
                    candidates,
                }
            })
            .collect::<Vec<_>>()
    };

    let prompt = prompt.unwrap_or_else(|| "Choose revealed cards".to_string());
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    // Resumable-VM (Batch 4): reveal-bucket is FULLY data-driven — no legacy
    // closure. The bucket-advance state machine IS install_reveal_bucket_resume_step;
    // it parks a RevealBucketStep frame (run by run_resume) or runs the terminal
    // synchronously (all buckets max==0 / no candidates). Buckets carry concrete
    // candidate handles (the per-bucket predicate was evaluated above), so the
    // state is pure data — ideal for clone (no closure to panic-stub).
    let selecting_player = ctx.override_selecting_player().unwrap_or(ctx.player);
    let state = crate::resume::RevealBucketState {
        prov: crate::resume::ResumeProvenance {
            source_card: ctx.source_card,
            source_permanent: ctx.source_permanent,
            source_kind: ctx.source_kind,
            controller: ctx.player,
            override_pin: ctx.override_selecting_player(),
        },
        selecting_player,
        previous_phase: ctx.game.current_phase,
        buckets: engine_buckets,
        bucket_index: 0,
        picked_buckets: Vec::new(),
        current_bucket_picks: Vec::new(),
        no_duplicate_cards,
        prompt,
        inner_tail: tail,
        bindings,
        runtime,
        trigger_context,
        outer_conts: Vec::new(),
    };
    install_reveal_bucket_resume_step(ctx.game, state);
    if ctx.game.pending_selection.is_some() {
        InstallResult::Parked
    } else {
        InstallResult::TailAlreadyRan
    }
}

fn revealed_owner_matches(
    of: CompiledPlayerRef,
    owner: u8,
    player: u8,
    game: &crate::game::Game,
) -> bool {
    match of {
        CompiledPlayerRef::You => owner == player,
        CompiledPlayerRef::Opponent => owner == game.next_clockwise(player),
        CompiledPlayerRef::Active => owner == game.turn_player(),
        CompiledPlayerRef::Any => true,
    }
}

fn install_select_security(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let target_player = resolve_player(ctx, of);
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    // Snapshot the source identity so the per-index filter closure can build
    // an `EffectReadContext` to evaluate the `CompiledPredicate` (mirrors
    // `install_select_material`). Without this the `filter` field on
    // `SelectSecurity` was silently ignored — every security card matched.
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    // Resumable-VM (make-engine-cloneable Batch 1): capture for the data frame
    // before the success closure consumes them. `select_security` installs no
    // on_decline, so the data-frame decline is always `None`.
    let bind_as_for_resume = bind_as.clone();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    let override_pin = ctx.override_selecting_player();
    ctx.select_security(
        target_player,
        &prompt,
        optional,
        move |game, idx| {
            let Some(card) = game.player(target_player).security.get(idx) else {
                return false;
            };
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::Card(card.handle()),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                if let Some(card) = cb_ctx.game.player(target_player).security.get(idx) {
                    b.insert_card(name, card.handle());
                }
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
    // Park the data frame alongside the closure (coexistence): driven by
    // `run_resume`'s Security decode arm. Decline is `None` (no on_decline).
    if ctx.game.pending_selection.is_some() {
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RunTail {
                prov: crate::resume::ResumeProvenance {
                    source_card,
                    source_permanent,
                    source_kind,
                    controller: player,
                    override_pin,
                },
                select_kind: crate::resume::ResumeSelectKind::Security {
                    of_player: target_player,
                    post: None,
                },
                bind_as: bind_as_for_resume,
                inner_tail: tail_for_resume,
                outer_conts: Vec::new(),
                bindings: bindings_for_resume,
                runtime: runtime_for_resume,
                trigger_context: trigger_for_resume,
                decline: crate::resume::ResumeDecline::None,
            }],
        });
    }
}

fn install_select_material(
    ctx: &mut EffectContext<'_>,
    perm: PermanentHandle,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    // Resumable-VM (Batch 2): capture for the data frame. The Material arm
    // decodes via material_zone_geometry(perm) (battle-vs-breeding carrier).
    let bind_as_for_resume = bind_as.clone();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    let override_pin = ctx.override_selecting_player();
    // Top-card exclusion is enforced by EffectContext::select_material itself
    // (matches CountCappedZone::Material).
    ctx.select_material(
        perm,
        &prompt,
        optional,
        move |game, src_idx| {
            // material_carrier_permanent branches battle vs. breeding carrier.
            let Some(card) = crate::effect_context::material_carrier_permanent(game, perm)
                .and_then(|p| p.card_sources.get(src_idx))
            else {
                return false;
            };
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::Card(card.handle()),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, src_idx| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                // material_carrier_permanent branches battle vs. breeding.
                if let Some(card) =
                    crate::effect_context::material_carrier_permanent(cb_ctx.game, perm)
                        .and_then(|p| p.card_sources.get(src_idx))
                {
                    b.insert_card(name, card.handle());
                }
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
    // Park the data frame alongside the closure (coexistence): driven by
    // `run_resume`'s Material arm (material_zone_geometry decode). Decline None.
    if ctx.game.pending_selection.is_some() {
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RunTail {
                prov: crate::resume::ResumeProvenance {
                    source_card,
                    source_permanent,
                    source_kind,
                    controller: player,
                    override_pin,
                },
                select_kind: crate::resume::ResumeSelectKind::Material { perm },
                bind_as: bind_as_for_resume,
                inner_tail: tail_for_resume,
                outer_conts: Vec::new(),
                bindings: bindings_for_resume,
                runtime: runtime_for_resume,
                trigger_context: trigger_for_resume,
                decline: crate::resume::ResumeDecline::None,
            }],
        });
    }
}

/// Install a count-capped / name-unique multi-pick over `perm`'s
/// digivolution-source stack. The batch sibling of `install_select_material`.
///
/// Lowers to `EffectContext::select_count_capped_multi` with
/// `CountCappedZone::Material(perm)` — REUSING the existing count-capped
/// selection mask/decoder (no `ACTION_SPACE_SIZE` change). `uniqueness`
/// is forwarded as the engine's `DistinctByMode`, which shapes the legal
/// action mask after each pick (no auto-selection — every pick is a
/// player choice, per CLAUDE.md §17).
///
/// The picked sources are bound as a `CardList`, so a follow-on
/// `play_from_materials` can consume the whole batch.
#[allow(clippy::too_many_arguments)]
fn install_select_materials(
    ctx: &mut EffectContext<'_>,
    perm: PermanentHandle,
    max: u8,
    filter: CompiledPredicate,
    uniqueness: Option<DistinctByMode>,
    bind_as: Option<String>,
    prompt: String,
    optional_zero: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    if max == 0 {
        // Zero-cap: bind an empty pick list and run the tail synchronously.
        let mut b = bindings.clone();
        if let Some(name) = &bind_as {
            b.insert_card_list(name, Vec::new());
        }
        run_tail_preserving_trigger_context(
            ctx,
            ctx.game.current_trigger_context.clone(),
            &tail,
            &mut b,
            &runtime,
        );
        return;
    }

    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    // Resumable-VM (Batch 4): capture for the MultiPickStep frame (Material-source
    // count_capped — same MultiPickState as Hand/Trash, zone = Material(perm),
    // of_player = perm.player, min = 0).
    let filter_for_resume = filter.clone();
    let filter_bindings_for_resume = bindings.clone();
    let bind_as_for_resume = bind_as.clone();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    let override_pin = ctx.override_selecting_player();
    ctx.select_count_capped_multi(
        // The carrier's owner — `Material` candidates come from
        // `perm.player`'s battle area regardless of `of_player`.
        perm.player,
        CountCappedZone::Material(perm),
        max,
        &prompt,
        optional_zero,
        uniqueness,
        move |game, card| {
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::Card(card.handle()),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, picks| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_card_list(name, picks);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
    // Park the MultiPickStep frame (clobber guard: park only if our candidate set
    // is non-empty).
    let candidate_indices = count_capped_card_candidate_indices(
        ctx.game,
        perm.player,
        CountCappedZone::Material(perm),
        &filter_for_resume,
        &filter_bindings_for_resume,
        source_card,
        source_permanent,
        source_kind,
        player,
    );
    if !candidate_indices.is_empty() {
        if let Some(pending) = ctx.game.pending_selection.as_ref() {
            let selecting_player = pending.selecting_player;
            let previous_phase = pending.previous_phase;
            let range_start =
                crate::effect_context::selections::material_zone_geometry(ctx.game, perm)
                    .map(|(_, rs)| rs)
                    .unwrap_or(0);
            ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
                frames: vec![crate::resume::ResumeFrame::MultiPickStep(
                    crate::resume::MultiPickState {
                        prov: crate::resume::ResumeProvenance {
                            source_card,
                            source_permanent,
                            source_kind,
                            controller: player,
                            override_pin,
                        },
                        of_player: perm.player,
                        selecting_player,
                        previous_phase,
                        zone: CountCappedZone::Material(perm),
                        range_start,
                        min: 0,
                        max,
                        is_optional_zero: optional_zero,
                        distinct_by: uniqueness,
                        candidate_indices,
                        accum: Vec::new(),
                        bind_as: bind_as_for_resume,
                        inner_tail: tail_for_resume,
                        bindings: bindings_for_resume,
                        runtime: runtime_for_resume,
                        trigger_context: trigger_for_resume,
                        outer_conts: Vec::new(),
                    },
                )],
            });
        }
    }
}

fn install_select_own_sources(
    ctx: &mut EffectContext<'_>,
    filter: CompiledPredicate,
    min: u8,
    max: u8,
    target: Option<CompiledBindingRef>,
    bind_as: Option<String>,
    prompt: String,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    if min > max || max == 0 {
        return;
    }

    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let target_permanent =
        target
            .as_ref()
            .and_then(|target| match resolve_binding_ref(target, ctx, &bindings) {
                Some(ResolvedBinding::Permanent(handle)) => Some(handle),
                _ => None,
            });
    let target_resolution_failed = target.is_some() && target_permanent.is_none();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    // Resumable-VM (Batch 4): capture for the SourceMultiStep frame (own → eval
    // on PredicateSubject::Source; of_player = you).
    let filter_for_resume = filter.clone();
    let filter_bindings_for_resume = bindings.clone();
    let bind_as_for_resume = bind_as.clone();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    let prompt_for_resume = prompt.clone();
    let override_pin = ctx.override_selecting_player();
    ctx.select_own_sources(
        &prompt,
        min,
        max,
        move |game, source| {
            if target_resolution_failed {
                return false;
            }
            if target_permanent.is_some_and(|handle| source.permanent != handle) {
                return false;
            }
            let read = EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read,
                PredicateSubject::Source(source),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, source_refs| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_source_refs(name, source_refs);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
    park_source_multi_resume_frame(
        ctx,
        player,
        false,
        min,
        max,
        target_permanent,
        target_resolution_failed,
        filter_for_resume,
        filter_bindings_for_resume,
        prompt_for_resume,
        bind_as_for_resume,
        tail_for_resume,
        bindings_for_resume,
        runtime_for_resume,
        trigger_for_resume,
        override_pin,
    );
}

/// Shared resume-frame park for the own/opponent source-multi DSL installers.
#[allow(clippy::too_many_arguments)]
fn park_source_multi_resume_frame(
    ctx: &mut EffectContext<'_>,
    of_player: PlayerId,
    eval_on_card: bool,
    min: u8,
    max: u8,
    target_permanent: Option<PermanentHandle>,
    target_resolution_failed: bool,
    filter: CompiledPredicate,
    filter_bindings: Bindings,
    prompt: String,
    bind_as: Option<String>,
    inner_tail: Arc<Vec<CompiledStep>>,
    bindings: Bindings,
    runtime: StepRuntime,
    trigger_context: Option<TriggerContext>,
    override_pin: Option<PlayerId>,
) {
    let player = ctx.player;
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let mut state = crate::resume::SourceMultiState {
        prov: crate::resume::ResumeProvenance {
            source_card,
            source_permanent,
            source_kind,
            controller: player,
            override_pin,
        },
        of_player,
        // selecting_player / previous_phase are corrected from the parked
        // selection below (placeholders; the candidate scan doesn't use them).
        selecting_player: of_player,
        previous_phase: crate::enums::GamePhase::Main,
        min,
        max,
        picked: Vec::new(),
        candidates: Vec::new(),
        filter,
        filter_bindings,
        target_permanent,
        target_resolution_failed,
        eval_on_card,
        prompt,
        bind_as,
        inner_tail,
        bindings,
        runtime,
        trigger_context,
        outer_conts: Vec::new(),
    };
    // Compute the initial snapshot (matches the closure install's candidates).
    // If empty, the closure SHORT-CIRCUITED (it ran the tail inline, which may
    // have installed a NESTED selection + its own resume frame) — do NOT clobber
    // it. A non-empty snapshot means the closure parked OUR source-multi select.
    state.candidates = source_multi_candidates_data(ctx.game, &state);
    if state.candidates.is_empty() {
        return;
    }
    let Some(pending) = ctx.game.pending_selection.as_ref() else {
        return;
    };
    state.selecting_player = pending.selecting_player;
    state.previous_phase = pending.previous_phase;
    ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
        frames: vec![crate::resume::ResumeFrame::SourceMultiStep(state)],
    });
}

/// Opponent-side mirror of `install_select_own_sources`. The candidate set is
/// drawn from the controller's OPPONENT's battle-area stacks via
/// `EffectContext::select_opponent_sources`; `target` (when present) restricts
/// the picker to a single opponent permanent binding. G-SELECT-OPPONENT-SOURCES.
#[allow(clippy::too_many_arguments)]
fn install_select_opponent_sources(
    ctx: &mut EffectContext<'_>,
    filter: CompiledPredicate,
    min: u8,
    max: u8,
    target: Option<CompiledBindingRef>,
    bind_as: Option<String>,
    prompt: String,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    if min > max || max == 0 {
        return;
    }

    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let target_permanent =
        target
            .as_ref()
            .and_then(|target| match resolve_binding_ref(target, ctx, &bindings) {
                Some(ResolvedBinding::Permanent(handle)) => Some(handle),
                _ => None,
            });
    let target_resolution_failed = target.is_some() && target_permanent.is_none();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    // Resumable-VM (Batch 4): capture for the SourceMultiStep frame (opponent →
    // eval on PredicateSubject::Card; of_player = next_clockwise(player)).
    let opponent = ctx.game.next_clockwise(player);
    let filter_for_resume = filter.clone();
    let filter_bindings_for_resume = bindings.clone();
    let bind_as_for_resume = bind_as.clone();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    let prompt_for_resume = prompt.clone();
    let override_pin = ctx.override_selecting_player();
    ctx.select_opponent_sources(
        &prompt,
        min,
        max,
        move |game, source| {
            if target_resolution_failed {
                return false;
            }
            if target_permanent.is_some_and(|handle| source.permanent != handle) {
                return false;
            }
            let read = EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read,
                PredicateSubject::Card(source.card),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, source_refs| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_source_refs(name, source_refs);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
    park_source_multi_resume_frame(
        ctx,
        opponent,
        true,
        min,
        max,
        target_permanent,
        target_resolution_failed,
        filter_for_resume,
        filter_bindings_for_resume,
        prompt_for_resume,
        bind_as_for_resume,
        tail_for_resume,
        bindings_for_resume,
        runtime_for_resume,
        trigger_for_resume,
        override_pin,
    );
}

#[allow(clippy::too_many_arguments)]
fn install_select_opponent_dp_budget(
    ctx: &mut EffectContext<'_>,
    dp_budget: i32,
    min_picks: u8,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    // Resumable-VM (Batch 4): capture for the BudgetStep frame. The filter is
    // carried as a CompiledPredicate (re-evaluated each step — data-pure), not a
    // closure. opponent = next_clockwise(player).
    let opponent = ctx.game.next_clockwise(player);
    let filter_for_resume = filter.clone();
    let filter_bindings_for_resume = bindings.clone();
    let bind_as_for_resume = bind_as.clone();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    let prompt_for_resume = prompt.clone();
    let override_pin = ctx.override_selecting_player();
    ctx.select_opponent_permanents_by_dp_budget(
        &prompt,
        dp_budget,
        min_picks,
        move |game, handle| {
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::Permanent(handle),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, handles| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_permanent_list(name, handles);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
    park_budget_resume_frame(
        ctx,
        crate::resume::BudgetKind::Dp,
        opponent,
        dp_budget,
        min_picks,
        filter_for_resume,
        filter_bindings_for_resume,
        prompt_for_resume,
        bind_as_for_resume,
        tail_for_resume,
        bindings_for_resume,
        runtime_for_resume,
        trigger_for_resume,
        override_pin,
    );
}

/// Shared resume-frame park for the dp/play-cost budget DSL installers: if the
/// closure path installed a selection, park a `BudgetStep` frame mirroring it.
#[allow(clippy::too_many_arguments)]
fn park_budget_resume_frame(
    ctx: &mut EffectContext<'_>,
    kind: crate::resume::BudgetKind,
    opponent: PlayerId,
    remaining: i32,
    min_picks: u8,
    filter: CompiledPredicate,
    filter_bindings: Bindings,
    prompt: String,
    bind_as: Option<String>,
    inner_tail: Arc<Vec<CompiledStep>>,
    bindings: Bindings,
    runtime: StepRuntime,
    trigger_context: Option<TriggerContext>,
    override_pin: Option<PlayerId>,
) {
    let player = ctx.player;
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let mut state = crate::resume::BudgetState {
        prov: crate::resume::ResumeProvenance {
            source_card,
            source_permanent,
            source_kind,
            controller: player,
            override_pin,
        },
        kind,
        opponent,
        // Corrected from the parked selection below (placeholders).
        selecting_player: player,
        previous_phase: crate::enums::GamePhase::Main,
        remaining,
        min_picks,
        picked: Vec::new(),
        filter,
        filter_bindings,
        prompt,
        bind_as,
        inner_tail,
        bindings,
        runtime,
        trigger_context,
        outer_conts: Vec::new(),
    };
    // If the closure found no candidates it SHORT-CIRCUITED (ran the tail inline,
    // possibly installing a nested selection + frame) — don't clobber it.
    if budget_candidates_data(ctx.game, &state).is_empty() {
        return;
    }
    let Some(pending) = ctx.game.pending_selection.as_ref() else {
        return;
    };
    state.selecting_player = pending.selecting_player;
    state.previous_phase = pending.previous_phase;
    ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
        frames: vec![crate::resume::ResumeFrame::BudgetStep(state)],
    });
}

#[allow(clippy::too_many_arguments)]
fn install_select_opponent_play_cost_budget(
    ctx: &mut EffectContext<'_>,
    play_cost_budget: i32,
    min_picks: u8,
    filter: CompiledPredicate,
    bind_as: Option<String>,
    prompt: String,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    // Resumable-VM (Batch 4): capture for the BudgetStep frame (play-cost kind).
    let opponent = ctx.game.next_clockwise(player);
    let filter_for_resume = filter.clone();
    let filter_bindings_for_resume = bindings.clone();
    let bind_as_for_resume = bind_as.clone();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    let prompt_for_resume = prompt.clone();
    let override_pin = ctx.override_selecting_player();
    ctx.select_opponent_permanents_by_play_cost_budget(
        &prompt,
        play_cost_budget,
        min_picks,
        move |game, handle| {
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::Permanent(handle),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, handles| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_permanent_list(name, handles);
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
    park_budget_resume_frame(
        ctx,
        crate::resume::BudgetKind::PlayCost,
        opponent,
        play_cost_budget,
        min_picks,
        filter_for_resume,
        filter_bindings_for_resume,
        prompt_for_resume,
        bind_as_for_resume,
        tail_for_resume,
        bindings_for_resume,
        runtime_for_resume,
        trigger_for_resume,
        override_pin,
    );
}

fn install_select_own_breeding_permanent(
    ctx: &mut EffectContext<'_>,
    bind_as: Option<String>,
    prompt: String,
    filter: CompiledPredicate,
    optional: bool,
    success_tail: Vec<CompiledStep>,
    decline_tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let success_tail = Arc::new(success_tail);
    let decline_tail = Arc::new(decline_tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    let tail_for_decline = Arc::clone(&decline_tail);
    let bindings_for_decline = bindings.clone();
    let runtime_for_decline = runtime.clone();
    let trigger_for_decline = trigger_context.clone();
    // Resumable-VM (make-engine-cloneable Batch 1): capture for the dual-tail
    // data frame. success → `inner_tail`, decline → `ResumeDecline::RunTail`
    // over `decline_tail` (no cost, so `aborts_clause = false`).
    let bind_as_for_resume = bind_as.clone();
    let success_tail_for_resume = Arc::clone(&success_tail);
    let decline_tail_for_resume = Arc::clone(&decline_tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    let override_pin = ctx.override_selecting_player();
    ctx.select_own_breeding_permanent(
        &prompt,
        optional,
        move |game, target| {
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::BreedingPermanent(target.player),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, target| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                b.insert_breeding_permanent_ref(name, target);
            }
            run_tail_preserving_trigger_context(
                cb_ctx,
                trigger_context,
                &success_tail,
                &mut b,
                &runtime,
            );
        },
    );
    if optional {
        if let Some(pending) = ctx.game.pending_selection.as_mut() {
            pending.on_decline = Some(Box::new(move |game: &mut crate::game::Game| {
                let mut decline_ctx = EffectContext::new_with_source_kind(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    player,
                );
                let mut b = bindings_for_decline.clone();
                run_tail_preserving_trigger_context(
                    &mut decline_ctx,
                    trigger_for_decline.clone(),
                    &tail_for_decline,
                    &mut b,
                    &runtime_for_decline,
                );
            }));
        }
    }
    // Park the data frame alongside the closure (coexistence): driven by
    // `run_resume`'s BreedingPermanent arm. Dual-tail — decline runs the
    // separate `decline_tail` (only when optional; no cost → no clause abort).
    if ctx.game.pending_selection.is_some() {
        let decline = if optional {
            crate::resume::ResumeDecline::RunTail {
                tail: decline_tail_for_resume,
                aborts_clause: false,
            }
        } else {
            crate::resume::ResumeDecline::None
        };
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RunTail {
                prov: crate::resume::ResumeProvenance {
                    source_card,
                    source_permanent,
                    source_kind,
                    controller: player,
                    override_pin,
                },
                select_kind: crate::resume::ResumeSelectKind::BreedingPermanent {
                    of_player: player,
                },
                bind_as: bind_as_for_resume,
                inner_tail: success_tail_for_resume,
                outer_conts: Vec::new(),
                bindings: bindings_for_resume,
                runtime: runtime_for_resume,
                trigger_context: trigger_for_resume,
                decline,
            }],
        });
    }
}

fn install_select_ordered_permutation(
    ctx: &mut EffectContext<'_>,
    items: Vec<crate::card_source::CardHandle>,
    bind_as: Option<String>,
    prompt: String,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    // Resumable-VM (Batch 4): capture for the PermutationStep data frame before
    // the closure consumes them. The first `install_permutation_step` parks over
    // the full `items` (accum empty); the frame mirrors that, then re-parks via
    // the data path on each pick.
    let items_for_resume = items.clone();
    let bind_as_for_resume = bind_as.clone();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let override_pin = ctx.override_selecting_player();
    ctx.select_ordered_permutation(items, &prompt, move |cb_ctx, ordered| {
        let mut b = bindings.clone();
        if let Some(name) = &bind_as {
            b.insert_card_list(name, ordered);
        }
        run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
    });
    // Park the data frame alongside the closure (coexistence): driven by
    // `run_resume`'s PermutationStep arm. `selecting_player`/`previous_phase` are
    // read back from the installed selection so the re-park matches exactly.
    // EMPTY items → select_ordered_permutation ran the tail INLINE (which may have
    // installed a nested selection + its own resume frame) — don't clobber it.
    // Cap to 10 to match select_ordered_permutation's truncation.
    let capped_items: Vec<_> = items_for_resume.into_iter().take(10).collect();
    if !capped_items.is_empty() {
        if let Some(pending) = ctx.game.pending_selection.as_ref() {
            let selecting_player = pending.selecting_player;
            let previous_phase = pending.previous_phase;
            ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
                frames: vec![crate::resume::ResumeFrame::PermutationStep(
                    crate::resume::PermutationState {
                        prov: crate::resume::ResumeProvenance {
                            source_card,
                            source_permanent,
                            source_kind,
                            controller: player,
                            override_pin,
                        },
                        selecting_player,
                        previous_phase,
                        remaining: capped_items,
                        accum: Vec::new(),
                        prompt,
                        bind_as: bind_as_for_resume,
                        inner_tail: tail_for_resume,
                        bindings: bindings_for_resume,
                        runtime: runtime_for_resume,
                        trigger_context: trigger_for_resume,
                        placement: None,
                        outer_conts: Vec::new(),
                    },
                )],
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
/// Reconstruct the `(action_id, handle, origin)` candidates of a parked
/// union-zone selection, mirroring `select_union_zone`'s callback decode across
/// the four ranges (breeding-source / field-source / trash / hand). Built at
/// install time — the selection is parked, so handles are stable — for the
/// resumable-VM `UnionZone` frame, which linear-searches rather than decodes.
fn union_zone_candidates(
    game: &crate::game::Game,
    of_player: PlayerId,
    valid_action_ids: &[u16],
) -> Vec<(
    u16,
    crate::card_source::CardHandle,
    crate::selection::UnionZoneOrigin,
)> {
    use crate::action::space::{
        decode_breeding_source_select, decode_source_select, BREEDING_SOURCE_SELECT_END,
        BREEDING_SOURCE_SELECT_START, BREEDING_TARGET, PLAY_HAND_START, SOURCE_SELECT_END,
        SOURCE_SELECT_START, TRASH_EFFECT_END, TRASH_EFFECT_START,
    };
    use crate::selection::UnionZoneOrigin;
    let mut out = Vec::with_capacity(valid_action_ids.len());
    for &action in valid_action_ids {
        let resolved =
            if (BREEDING_SOURCE_SELECT_START..BREEDING_SOURCE_SELECT_END).contains(&action) {
                let (player, source_index) = decode_breeding_source_select(action);
                let (player, source_index) = (player as u8, source_index as u8);
                let carrier = PermanentHandle {
                    player,
                    index: BREEDING_TARGET as u8,
                };
                game.player(player)
                    .breeding_area
                    .as_ref()
                    .and_then(|p| p.card_sources.get(source_index as usize))
                    .map(|c| {
                        (
                            c.handle(),
                            UnionZoneOrigin::Material {
                                carrier,
                                source_index,
                            },
                        )
                    })
            } else if (SOURCE_SELECT_START..SOURCE_SELECT_END).contains(&action) {
                let (field_index, source_index) = decode_source_select(action);
                let (field_index, source_index) = (field_index as u8, source_index as u8);
                let carrier = PermanentHandle {
                    player: of_player,
                    index: field_index,
                };
                game.player(of_player)
                    .battle_area
                    .get(field_index as usize)
                    .and_then(|p| p.card_sources.get(source_index as usize))
                    .map(|c| {
                        (
                            c.handle(),
                            UnionZoneOrigin::Material {
                                carrier,
                                source_index,
                            },
                        )
                    })
            } else if (TRASH_EFFECT_START..TRASH_EFFECT_END).contains(&action) {
                let idx = (action - TRASH_EFFECT_START) as usize;
                game.player(of_player)
                    .trash
                    .get(idx)
                    .map(|c| (c.handle(), UnionZoneOrigin::Trash))
            } else {
                let idx = action.saturating_sub(PLAY_HAND_START) as usize;
                game.player(of_player)
                    .hand
                    .get(idx)
                    .map(|c| (c.handle(), UnionZoneOrigin::Hand))
            };
        if let Some((handle, origin)) = resolved {
            out.push((action, handle, origin));
        }
    }
    out
}

fn install_select_union_zone(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    zoneset: crate::selection::UnionZoneSet,
    material_of: Option<CompiledBindingRef>,
    filter: CompiledPredicate,
    zone_filters: digimon_dsl::compiled::CompiledUnionZoneFilters,
    material_carrier_filter: Option<CompiledPredicate>,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    cost: bool,
    success_tail: Vec<CompiledStep>,
    decline_tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let target_player = resolve_player(ctx, of);
    let material_target = material_of.as_ref().and_then(|binding| {
        match resolve_binding_ref(binding, ctx, &bindings) {
            Some(ResolvedBinding::Permanent(handle)) => Some(handle),
            _ => None,
        }
    });
    let success_tail = Arc::new(success_tail);
    let decline_tail = Arc::new(decline_tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    // G-UNION-HAND-SOURCE-PLAY: build the carrier-restriction predicate closure
    // for the `material_of: None` scan. Each candidate carrier's TOP CARD is
    // evaluated against `material_carrier_filter` (a permanent subject) before
    // its sources are enumerated; `None` → every field carrier is scanned.
    let carrier_filter_bindings = bindings.clone();
    type CarrierFilter =
        Box<dyn Fn(&crate::game::Game, crate::permanent::PermanentHandle) -> bool + Send + Sync>;
    let carrier_filter: Option<CarrierFilter> = material_carrier_filter.map(|pred| {
        let b = carrier_filter_bindings;
        let f: CarrierFilter = Box::new(
            move |game: &crate::game::Game, handle: crate::permanent::PermanentHandle| {
                let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    player,
                );
                eval_predicate_with_bindings(
                    &pred,
                    &read_ctx,
                    PredicateSubject::Permanent(handle),
                    Some(&b),
                )
            },
        );
        f
    });
    // Clone state for the decline path before the success callback consumes
    // them. G-OPTIONAL-SELECTION-CONTINUE-TAIL — declining an optional
    // union-zone selection must still run the outer tail so subsequent
    // mandatory steps (e.g. `play_union_bound_free`'s neighboring steps, or
    // a mandatory `gain_memory`) execute. Mirrors `install_select_trash`.
    let tail_for_decline = Arc::clone(&decline_tail);
    let bindings_for_decline = bindings.clone();
    let runtime_for_decline = runtime.clone();
    let trigger_for_decline = trigger_context.clone();
    // Resumable-VM (Batch 3): capture for the dual-tail data frame. success →
    // `inner_tail` (success_tail), decline → `ResumeDecline::RunTail` over
    // decline_tail with `aborts_clause = cost`. Candidates are reconstructed
    // from the parked selection's valid_action_ids below (tri-range decode).
    let bind_as_for_resume = bind_as.clone();
    let success_tail_for_resume = Arc::clone(&success_tail);
    let decline_tail_for_resume = Arc::clone(&decline_tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    let override_pin = ctx.override_selecting_player();
    ctx.select_union_zone(
        target_player,
        zoneset,
        material_target,
        carrier_filter,
        &prompt,
        optional,
        // PUPPETS-G021: evaluate the compiled predicate against the card's
        // CardData (via PredicateSubject::Card) so that DP constraints such
        // as `dp_lte: 4000` work for hidden-zone (hand/trash) candidates.
        //
        // G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY: select the predicate by the
        // candidate's origin zone. A per-zone override (`zone_filters.{hand,
        // trash,material}`) replaces the shared `filter` for that zone; zones
        // without an override fall back to `filter`. This lets one prompt offer
        // "1 X from trash OR 1 Y from breeding sources" with different filters.
        move |game, card, zone| {
            use crate::selection::UnionZoneSet;
            let active = if zone == UnionZoneSet::HAND {
                zone_filters.hand.as_ref()
            } else if zone == UnionZoneSet::TRASH {
                zone_filters.trash.as_ref()
            } else {
                zone_filters.material.as_ref()
            }
            .unwrap_or(&filter);
            let handle = card.handle();
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                active,
                &read_ctx,
                PredicateSubject::Card(handle),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, handle, origin| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_as {
                // PUPPETS-G014: record the origin zone alongside the handle
                // so a `play_union_bound_free` tail step can replay the card
                // from its true zone (hand, trash, or material).
                b.insert_union_card(name, handle, origin, target_player);
            }
            run_tail_preserving_trigger_context(
                cb_ctx,
                trigger_context,
                &success_tail,
                &mut b,
                &runtime,
            );
        },
    );
    // Attach decline-tail callback for optional selections — carry-over from
    // Task 6 code review. Mirrors `install_select_trash`.
    if optional {
        if let Some(pending) = ctx.game.pending_selection.as_mut() {
            pending.on_decline = Some(Box::new(move |game: &mut crate::game::Game| {
                // See `install_select_hand` for the cost vs. continue-tail
                // distinction. `cost: true` sets the abort flag and the
                // captured decline_tail short-circuits.
                if cost {
                    game.dsl_clause_aborted = true;
                }
                let mut decline_ctx = EffectContext::new_with_source_kind(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    player,
                );
                let mut b = bindings_for_decline.clone();
                run_tail_preserving_trigger_context(
                    &mut decline_ctx,
                    trigger_for_decline.clone(),
                    &tail_for_decline,
                    &mut b,
                    &runtime_for_decline,
                );
            }));
        }
    }
    // Park the dual-tail data frame alongside the closure (coexistence): driven
    // by `run_resume`'s UnionZone arm (linear search over the reconstructed
    // tri-range candidates). Decline runs the separate decline_tail with
    // `aborts_clause = cost`, mirroring the on_decline above.
    if ctx.game.pending_selection.is_some() {
        let valid = ctx
            .game
            .pending_selection
            .as_ref()
            .map(|p| p.valid_action_ids.clone())
            .unwrap_or_default();
        let candidates = union_zone_candidates(ctx.game, target_player, &valid);
        let decline = if optional {
            crate::resume::ResumeDecline::RunTail {
                tail: decline_tail_for_resume,
                aborts_clause: cost,
            }
        } else {
            crate::resume::ResumeDecline::None
        };
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RunTail {
                prov: crate::resume::ResumeProvenance {
                    source_card,
                    source_permanent,
                    source_kind,
                    controller: player,
                    override_pin,
                },
                select_kind: crate::resume::ResumeSelectKind::UnionZone {
                    of_player: target_player,
                    candidates,
                },
                bind_as: bind_as_for_resume,
                inner_tail: success_tail_for_resume,
                outer_conts: Vec::new(),
                bindings: bindings_for_resume,
                runtime: runtime_for_resume,
                trigger_context: trigger_for_resume,
                decline,
            }],
        });
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Phase 2 Track E (2026-05-17): reveal-ordering DSL verbs.
//
// `choose_from_reveal` and `order_remainder` are the author-facing combo for
// the "reveal N, pick one matching to <destination>, place rest top-or-bottom
// in any order" pattern that recurs across Rocks searchers (P-167 et al) and
// every Training / Memory Boost / search effect. Both lower as selection
// installs that consume the existing `select_reveal` / `select_effect_choice`
// / `select_ordered_permutation` engine surface — no new substrate hooks.
//
// AUTHOR NOTE — `optional: true` semantics:
// `choose_from_reveal { optional: true }` is permissible ONLY when the
// printed card text explicitly grants the player a "may" at THAT specific
// pick (e.g. "you may add 1 ... to your hand"). When the printed text says
// "Add 1 card ..." without "may", the pick is mandatory and `optional`
// MUST be `false` (or omitted — `false` is the default). The "no eligible
// candidates" case is handled by the engine's natural fizzle path
// (`install_choose_from_reveal` returns `false` and skips the pending
// install), NOT by a player-driven decline.
//
// For the canonical "Add 1 X and 1 Y" two-pick reveal-search pattern,
// prefer `select_reveal_buckets` (see BT24-031 Elecmon's YAML for the
// reference): one combined bucket prompt, `min: 1, max: 1` per bucket,
// `no_duplicate_cards: true`, mandatory by construction.
//
// See `openspec/specs/dsl-card-scripting-vocabulary/spec.md` for the
// requirement ("`choose_from_reveal { optional: true }` requires
// printed-text 'may'") added by `fix-qa-bugs-aura-tick-reveal-picks`.
// ───────────────────────────────────────────────────────────────────────────

fn revealed_owner_matches_for_choose(
    of: CompiledPlayerRef,
    owner: u8,
    player: u8,
    game: &crate::game::Game,
) -> bool {
    revealed_owner_matches(of, owner, player, game)
}

/// Install a `select_reveal`-style pick that routes the chosen revealed card
/// to a typed destination on success.
///
/// Returns `true` iff a `PendingSelection` was installed (i.e. the install
/// did NOT short-circuit because no candidate revealed cards matched the
/// filter / `of`-owner constraint).
#[allow(clippy::too_many_arguments)]
fn install_choose_from_reveal(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    filter: CompiledPredicate,
    destination: CompiledRevealDestination,
    bind_as: Option<String>,
    prompt: String,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) -> bool {
    let target_player = resolve_player(ctx, of);
    // For BottomSourceOf, resolve the target permanent now so we can short-
    // circuit later. If resolution fails (binding missing or wrong kind),
    // routing silently no-ops on the destination — matching the 2b/2c missing-
    // binding convention used elsewhere in this module.
    let target_permanent = match &destination {
        CompiledRevealDestination::BottomSourceOf(binding_ref) => {
            match resolve_binding_ref(binding_ref, ctx, &bindings) {
                Some(ResolvedBinding::Permanent(h)) => Some(h),
                _ => None,
            }
        }
        _ => None,
    };
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    let filter_bindings = bindings.clone();
    let destination_for_callback = destination;
    // Resumable-VM: capture for the Reveal{route:Some} data frame. The filter
    // restricts valid_action_ids at install, so the resume arm just decodes +
    // binds + routes (no re-eval of the filter).
    let override_pin = ctx.override_selecting_player();
    let bind_as_for_resume = bind_as.clone();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    let route_for_resume = crate::resume::RevealRoute {
        destination: destination_for_callback.clone(),
        target_player,
        target_permanent,
    };
    let parked = ctx.select_reveal(
        &prompt,
        optional,
        move |game, idx| {
            let Some(card) = game.revealed_cards.get(idx) else {
                return false;
            };
            if !revealed_owner_matches_for_choose(of, card.owner, player, game) {
                return false;
            }
            let read_ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            eval_predicate_with_bindings(
                &filter,
                &read_ctx,
                PredicateSubject::RevealedCard(card.handle()),
                Some(&filter_bindings),
            )
        },
        move |cb_ctx, idx| {
            let mut b = bindings.clone();
            // Resolve the picked reveal index to a stable CardHandle and
            // route it to the destination. If the index has gone stale
            // (reveal pool mutated mid-resolution), silently skip routing —
            // bindings are best-effort per the 2b/2c convention.
            let picked = cb_ctx.game.revealed_cards.get(idx).map(|c| c.handle());
            if let Some(handle) = picked {
                if let Some(name) = &bind_as {
                    b.insert_card(name, handle);
                }
                route_chosen_reveal(
                    cb_ctx,
                    target_player,
                    handle,
                    &destination_for_callback,
                    target_permanent,
                );
            }
            run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
        },
    );
    // Park the data frame alongside the closure (coexistence): driven by
    // run_resume's Reveal{route:Some} arm. No on_decline → decline None.
    if ctx.game.pending_selection.is_some() {
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RunTail {
                prov: crate::resume::ResumeProvenance {
                    source_card,
                    source_permanent,
                    source_kind,
                    controller: player,
                    override_pin,
                },
                select_kind: crate::resume::ResumeSelectKind::Reveal {
                    route: Some(route_for_resume),
                },
                bind_as: bind_as_for_resume,
                inner_tail: tail_for_resume,
                outer_conts: Vec::new(),
                bindings: bindings_for_resume,
                runtime: runtime_for_resume,
                trigger_context: trigger_for_resume,
                decline: crate::resume::ResumeDecline::None,
            }],
        });
    }
    parked
}

/// Route a single chosen revealed card to its destination. Pure dispatch —
/// the engine helpers handle the actual zone movement and event emission.
fn route_chosen_reveal(
    ctx: &mut EffectContext<'_>,
    player: crate::enums::PlayerId,
    handle: crate::card_source::CardHandle,
    destination: &CompiledRevealDestination,
    target_permanent: Option<PermanentHandle>,
) {
    use crate::enums::StackPosition;
    use crate::CardSourceRef;
    match destination {
        CompiledRevealDestination::Hand => {
            ctx.add_to_hand_from_reveal(player, handle);
        }
        CompiledRevealDestination::DeckTop => {
            ctx.return_to_deck_from_reveal(player, handle, StackPosition::Top);
        }
        CompiledRevealDestination::DeckBottom => {
            ctx.return_to_deck_from_reveal(player, handle, StackPosition::Bottom);
        }
        CompiledRevealDestination::PlayFree => {
            ctx.play_from_reveal_free(player, handle);
        }
        CompiledRevealDestination::BottomSourceOf(_) => {
            if let Some(perm) = target_permanent {
                let _ = ctx.place_as_bottom_source(CardSourceRef::Reveal(handle), perm, false);
            }
        }
    }
}

/// Install `order_remainder` — drives placement of the entire reveal pool back
/// onto the controller's deck, surfacing an ordered-permutation selection for
/// the cards and (when multiple destinations are listed) a destination
/// effect-choice. Empty reveal pool → silent no-op (no selection installed).
fn install_order_remainder(
    ctx: &mut EffectContext<'_>,
    of: CompiledPlayerRef,
    destinations: Vec<CompiledRemainderDestination>,
    prompt: Option<String>,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) -> InstallResult {
    let target_player = resolve_player(ctx, of);
    // Empty reveal pool → silent no-op; tail proceeds inline.
    if ctx.game.revealed_cards.is_empty() {
        return InstallResult::Continue;
    }
    // Defensive: compile validated 1..=2 destinations, but treat empty as no-op.
    if destinations.is_empty() {
        return InstallResult::Continue;
    }

    // Single destination → equivalent to `place_remainder_on_deck` (which
    // surfaces ordering directly via select_ordered_permutation).
    if destinations.len() == 1 {
        let position = remainder_destination_position(destinations[0]);
        // Snapshot remainder + install ordered permutation with the placement
        // closure AND tail-run. Cannot delegate to `place_remainder_on_deck`
        // because we need to chain the tail onto the same selection callback.
        return install_remainder_permutation_with_tail(
            ctx,
            target_player,
            position,
            tail,
            bindings,
            runtime,
        );
    }

    // Multi-destination → prompt for top vs bottom, then install ordered
    // permutation with chosen position. Both selections are surfaced as
    // separate action-mask windows per Working Rule §17.
    let labels: Vec<String> = destinations
        .iter()
        .map(|d| remainder_destination_label(*d).to_string())
        .collect();
    let prompt =
        prompt.unwrap_or_else(|| "Place remaining cards on top or bottom of the deck?".to_string());
    let destinations_capture: Vec<CompiledRemainderDestination> = destinations;
    // Resumable-VM: map each destination to a deck position for the
    // EffectChoice{post: OrderRemainder} data frame (parked below). The index
    // chosen in the prompt selects into this list, exactly as the closure
    // indexes `destinations_capture`.
    let positions_for_resume: Vec<crate::enums::StackPosition> = destinations_capture
        .iter()
        .map(|d| remainder_destination_position(*d))
        .collect();
    let tail_arc = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    // Capture provenance + the carried tail/bindings/runtime for the data frame
    // BEFORE the closure moves them (coexistence: both paths install).
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let controller = ctx.player;
    let override_pin = ctx.override_selecting_player();
    let tail_for_resume = Arc::clone(&tail_arc);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    ctx.select_effect_choice(&prompt, labels, move |cb_ctx, idx| {
        let position = remainder_destination_position(destinations_capture[idx]);
        // The select_effect_choice callback runs in a fresh selection scope
        // (pending_selection is now None). Install the ordered permutation
        // with the tail, exactly as the single-destination path does.
        let _ = install_remainder_permutation_with_tail(
            cb_ctx,
            target_player,
            position,
            (*tail_arc).clone(),
            bindings.clone(),
            runtime.clone(),
        );
        // Preserve trigger context across the inner install + tail.
        cb_ctx.game.current_trigger_context = trigger_context.clone();
    });
    if ctx.game.pending_selection.is_some() {
        // Park the data frame (coexistence): driven by run_resume's
        // EffectChoice{post: OrderRemainder} arm, which chains into
        // install_remainder_permutation_with_tail (frame-installs-frame). The
        // destination choice is not optional (a branch must be picked) → decline
        // None. inner_tail becomes the permutation's tail, not run in the choice.
        ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RunTail {
                prov: crate::resume::ResumeProvenance {
                    source_card,
                    source_permanent,
                    source_kind,
                    controller,
                    override_pin,
                },
                select_kind: crate::resume::ResumeSelectKind::EffectChoice {
                    post: Some(crate::resume::EffectChoicePostAction::OrderRemainder {
                        positions: positions_for_resume,
                        player: target_player,
                    }),
                },
                bind_as: None,
                inner_tail: tail_for_resume,
                outer_conts: Vec::new(),
                bindings: bindings_for_resume,
                runtime: runtime_for_resume,
                trigger_context: trigger_for_resume,
                decline: crate::resume::ResumeDecline::None,
            }],
        });
        InstallResult::Parked
    } else {
        InstallResult::Continue
    }
}

fn remainder_destination_position(d: CompiledRemainderDestination) -> crate::enums::StackPosition {
    match d {
        CompiledRemainderDestination::DeckTop => crate::enums::StackPosition::Top,
        CompiledRemainderDestination::DeckBottom => crate::enums::StackPosition::Bottom,
    }
}

fn remainder_destination_label(d: CompiledRemainderDestination) -> &'static str {
    match d {
        CompiledRemainderDestination::DeckTop => "Top of deck",
        CompiledRemainderDestination::DeckBottom => "Bottom of deck",
    }
}

/// Install an ordered-permutation selection over the current reveal pool that,
/// on resolution, places each card at `position` (top or bottom of `player`'s
/// deck) per the `place_remainder_on_deck` order rules, then runs `tail`.
fn install_remainder_permutation_with_tail(
    ctx: &mut EffectContext<'_>,
    player: crate::enums::PlayerId,
    position: crate::enums::StackPosition,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) -> InstallResult {
    use crate::card_source::CardHandle;
    let remainder: Vec<CardHandle> = ctx
        .game
        .revealed_cards
        .iter()
        .map(|cs| cs.handle())
        .collect();
    if remainder.is_empty() {
        return InstallResult::Continue;
    }
    debug_assert!(
        remainder.len() <= 10,
        "order_remainder: reveal pool has {} cards; select_ordered_permutation is capped at 10",
        remainder.len()
    );
    let tail = Arc::new(tail);
    let trigger_context = ctx.game.current_trigger_context.clone();
    // Resumable-VM: capture for the PermutationStep{placement:Some} data frame.
    // The terminal places the ordered list back on the deck (no bind).
    const PROMPT: &str = "Place remaining cards on deck in any order";
    let remainder_for_resume = remainder.clone();
    let tail_for_resume = Arc::clone(&tail);
    let bindings_for_resume = bindings.clone();
    let runtime_for_resume = runtime.clone();
    let trigger_for_resume = trigger_context.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let controller = ctx.player;
    let override_pin = ctx.override_selecting_player();
    ctx.select_ordered_permutation(remainder, PROMPT, move |cb_ctx, ordered_vec| {
        place_remainder_in_order(cb_ctx, player, &ordered_vec, position);
        let mut b = bindings.clone();
        run_tail_preserving_trigger_context(cb_ctx, trigger_context, &tail, &mut b, &runtime);
    });
    if ctx.game.pending_selection.is_some() {
        // Park the data frame: placement-terminal (no bind), driven by
        // run_resume's PermutationStep arm. selecting_player/previous_phase read
        // back from the installed selection so the re-park matches exactly.
        let capped: Vec<_> = remainder_for_resume.into_iter().take(10).collect();
        if let Some(pending) = ctx.game.pending_selection.as_ref() {
            let selecting_player = pending.selecting_player;
            let previous_phase = pending.previous_phase;
            ctx.game.pending_selection_resume = Some(crate::resume::ResumeStack {
                frames: vec![crate::resume::ResumeFrame::PermutationStep(
                    crate::resume::PermutationState {
                        prov: crate::resume::ResumeProvenance {
                            source_card,
                            source_permanent,
                            source_kind,
                            controller,
                            override_pin,
                        },
                        selecting_player,
                        previous_phase,
                        remaining: capped,
                        accum: Vec::new(),
                        prompt: PROMPT.to_string(),
                        bind_as: None,
                        inner_tail: tail_for_resume,
                        bindings: bindings_for_resume,
                        runtime: runtime_for_resume,
                        trigger_context: trigger_for_resume,
                        placement: Some(crate::resume::RemainderPlacement { player, position }),
                        outer_conts: Vec::new(),
                    },
                )],
            });
        }
        InstallResult::Parked
    } else {
        // Empty / capped — selection completed synchronously; tail already ran.
        InstallResult::TailAlreadyRan
    }
}

/// Replicate `EffectContext::place_remainder_on_deck`'s placement loop
/// directly (we control the surrounding selection install ourselves).
///
/// - `Top`: reverse-iterate so `ordered_vec[0]` lands at the highest deck
///   index (drawn first).
/// - `Bottom`: forward-iterate with bottom inserts so `ordered_vec[0]` ends up
///   at the highest index among the placed group (drawn first of the bottom
///   group).
/// - `Random`: forward-iterate placing each at a random position. The order
///   selection is still surfaced (Working Rule §17) even though placement is
///   non-deterministic.
fn place_remainder_in_order(
    ctx: &mut EffectContext<'_>,
    player: crate::enums::PlayerId,
    ordered: &[crate::card_source::CardHandle],
    position: crate::enums::StackPosition,
) {
    use crate::enums::StackPosition;
    match position {
        StackPosition::Top => {
            for handle in ordered.iter().rev() {
                let placed =
                    ctx.game
                        .return_to_deck_from_reveal(player, *handle, StackPosition::Top);
                debug_assert!(
                    placed,
                    "place_remainder_in_order: handle {:?} not found in revealed_cards",
                    handle
                );
            }
        }
        StackPosition::Bottom => {
            for handle in ordered.iter() {
                let placed =
                    ctx.game
                        .return_to_deck_from_reveal(player, *handle, StackPosition::Bottom);
                debug_assert!(
                    placed,
                    "place_remainder_in_order: handle {:?} not found in revealed_cards",
                    handle
                );
            }
        }
        StackPosition::Random => {
            for handle in ordered.iter() {
                let placed =
                    ctx.game
                        .return_to_deck_from_reveal(player, *handle, StackPosition::Random);
                debug_assert!(
                    placed,
                    "place_remainder_in_order: handle {:?} not found in revealed_cards",
                    handle
                );
            }
        }
    }
}
