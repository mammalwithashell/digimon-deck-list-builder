//! `link_card_to_self` step lowering (facet #9 — G-DSL-LINK-CARD-FROM-ZONE).
//!
//! The effect's own permanent (`ctx.source_permanent`) links 1 chosen card
//! matching `filter` out of one of the requested `from` zones (hand / trash /
//! its own digivolution sources) onto itself. The candidate set spans all
//! requested zones in a single RL-visible `SelectionKind::Target` prompt; on
//! resolution we compute the effective link cost (`cost − ChangeLinkCost`),
//! pay it, then call the already-tested engine primitive
//! `Game::link_chosen_card_into_host` (which lifts the card, attaches it onto
//! the host's `linked_cards`, and fires `OnLink`).
//!
//! Mirrors DCGO `ILinkCard.LinkCard` with `root != None` → `Permanent.AddLinkCard`.
//! Distinct from `LinkToOwnDigimon` (the Plug-In Option self-link bound to
//! `pending_option`): this verb is for a Digimon's *own* activated/triggered
//! effect linking a card from a zone onto itself.

use digimon_dsl::compiled::{CompiledPredicate, CompiledStep};
use digimon_dsl::step::{LinkFromZone, LinkToHost};

use crate::action::space::{
    SOURCES_PER_FIELD, SOURCE_SELECT_START, TRASH_EFFECT_START, HAND_EFFECT_START,
};
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::GamePhase;
use crate::enums::LinkCardSource;
use crate::permanent::PermanentHandle;
use crate::selection::{PendingSelection, SelectionKind};

/// One resolved candidate: the action id that selects it, the `CardHandle`
/// being linked, and the `LinkCardSource` describing where to lift it from.
#[derive(Clone)]
struct LinkCandidate {
    action_id: u16,
    card: crate::card_source::CardHandle,
    source: LinkCardSource,
}

/// Returns `true` iff the step was a `LinkCardToSelf` (handled here).
pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>) -> bool {
    let CompiledStep::LinkCardToSelf {
        from,
        filter,
        to,
        cost,
        optional,
    } = step
    else {
        return false;
    };

    // `host_for_sources` is the effect's own permanent — it anchors the
    // digivolution-source candidate zone and (for `SelfPermanent`) the link
    // target. If there is no source permanent the effect can't proceed.
    let Some(host_for_sources) = ctx.source_permanent else {
        return true;
    };
    // Self-host requires the own permanent to be a live standing Digimon.
    if matches!(to, LinkToHost::SelfPermanent)
        && !ctx.game.permanent_is_digimon_for_rules(host_for_sources)
    {
        return true;
    }

    let controller = ctx.player;
    let source_card = ctx.source_card;
    let source_kind = ctx.source_kind;

    let candidates = gather_candidates(
        ctx,
        host_for_sources,
        controller,
        from,
        filter,
        source_card,
        source_kind,
    );

    // No legal card → nothing to do (mandatory no-op, optional offers nothing).
    if candidates.is_empty() {
        return true;
    }

    let cost = *cost;
    let optional = *optional;
    let to = *to;
    let source_permanent = ctx.source_permanent;
    let selecting_player = ctx.override_selecting_player().unwrap_or(controller);
    let previous_phase = ctx.game.current_phase;
    let prompt = "Choose a card to link".to_string();

    let valid_action_ids: Vec<u16> = candidates.iter().map(|c| c.action_id).collect();
    let candidates_for_cb = candidates.clone();

    ctx.game.current_phase = GamePhase::SelectTarget;
    ctx.game.pending_selection = Some(PendingSelection {
        zone_owner: None,
        kind: SelectionKind::Target,
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
            let Some(chosen) = candidates_for_cb
                .iter()
                .find(|c| c.action_id == action_id)
                .cloned()
            else {
                return;
            };
            match to {
                LinkToHost::SelfPermanent => {
                    pay_and_link(game, controller, cost, host_for_sources, &chosen);
                }
                LinkToHost::ChosenOwnDigimon => {
                    // Second RL-visible selection: which of the controller's
                    // standing Digimon receives the link.
                    install_host_selection(
                        game,
                        controller,
                        cost,
                        chosen,
                        selecting_player,
                        source_card,
                        source_permanent,
                        source_kind,
                    );
                }
            }
        }),
        on_decline: None,
    });
    true
}

/// Pay the effective link cost (printed `cost` reduced by any `ChangeLinkCost`,
/// clamped at 0) and call the primitive to attach `chosen` onto `host`.
fn pay_and_link(
    game: &mut crate::game::Game,
    controller: crate::enums::PlayerId,
    cost: u16,
    host: PermanentHandle,
    chosen: &LinkCandidate,
) {
    let delta = game.modifiers.link_cost_delta_for_player(controller);
    let effective = (cost as i32 + delta).max(0) as i16;
    if effective > 0 {
        let new_memory = game.memory - effective;
        game.set_memory(new_memory);
    }
    game.link_chosen_card_into_host(host, chosen.card, chosen.source);
}

/// Install the host-selection prompt over the controller's standing Digimon
/// (for `to: ChosenOwnDigimon`). On resolution, pay cost and link.
#[allow(clippy::too_many_arguments)]
fn install_host_selection(
    game: &mut crate::game::Game,
    controller: crate::enums::PlayerId,
    cost: u16,
    chosen: LinkCandidate,
    selecting_player: crate::enums::PlayerId,
    source_card: crate::card_source::CardHandle,
    source_permanent: Option<PermanentHandle>,
    source_kind: crate::enums::EffectSourceKind,
) {
    use crate::action::space::encode_attack;

    // Candidate hosts: the controller's live standing Digimon.
    let hosts: Vec<(u16, PermanentHandle)> = (0..game.player(controller).battle_area.len())
        .filter_map(|i| {
            let h = PermanentHandle {
                player: controller,
                index: i as u8,
            };
            if game.permanent_is_digimon_for_rules(h)
                && matches!(
                    game.player(controller).battle_area[i].option_state,
                    crate::permanent::OptionState::Standard
                )
            {
                Some((encode_attack(controller as u16, i as u16), h))
            } else {
                None
            }
        })
        .collect();

    if hosts.is_empty() {
        // No eligible host — the lifted card stays where it was (no link). The
        // card was never removed (the primitive does removal), so nothing leaks.
        return;
    }

    let valid_action_ids: Vec<u16> = hosts.iter().map(|(a, _)| *a).collect();
    let previous_phase = game.current_phase;
    game.current_phase = GamePhase::SelectTarget;
    game.pending_selection = Some(PendingSelection {
        zone_owner: None,
        kind: SelectionKind::Target,
        selecting_player,
        previous_phase,
        valid_action_ids,
        is_optional: false,
        prompt: "Choose 1 of your Digimon to link the card to".to_string(),
        effect_choices: None,
        source_card,
        source_permanent,
        source_kind,
        callback: Box::new(move |game, action_id| {
            let Some((_, host)) = hosts.iter().find(|(a, _)| *a == action_id).copied() else {
                return;
            };
            pay_and_link(game, controller, cost, host, &chosen);
        }),
        on_decline: None,
    });
}

/// Collect link candidates across the requested zones. Action ids use each
/// zone's existing disjoint sub-range so a single selection over the union is
/// unambiguous:
/// - Hand  → `HAND_EFFECT_START + i`
/// - Trash → `TRASH_EFFECT_START + i`
/// - Own digivolution sources → `SOURCE_SELECT_START + field*SOURCES_PER_FIELD + src`
///   (the live stack top is excluded — it is the Digimon, not a source).
fn gather_candidates(
    ctx: &EffectContext<'_>,
    host: PermanentHandle,
    controller: crate::enums::PlayerId,
    from: &[LinkFromZone],
    filter: &CompiledPredicate,
    source_card: crate::card_source::CardHandle,
    source_kind: crate::enums::EffectSourceKind,
) -> Vec<LinkCandidate> {
    let mut out: Vec<LinkCandidate> = Vec::new();
    let read_ctx = EffectReadContext::new_with_source_kind(
        ctx.game,
        source_card,
        Some(host),
        source_kind,
        controller,
    );

    for zone in from {
        match zone {
            LinkFromZone::Hand => {
                for (i, c) in ctx.game.player(controller).hand.iter().enumerate() {
                    let card = c.handle();
                    if eval_predicate(filter, &read_ctx, PredicateSubject::Card(card)) {
                        out.push(LinkCandidate {
                            action_id: HAND_EFFECT_START + i as u16,
                            card,
                            source: LinkCardSource::Hand(controller),
                        });
                    }
                }
            }
            LinkFromZone::Trash => {
                for (i, c) in ctx.game.player(controller).trash.iter().enumerate() {
                    let card = c.handle();
                    if eval_predicate(filter, &read_ctx, PredicateSubject::Card(card)) {
                        out.push(LinkCandidate {
                            action_id: TRASH_EFFECT_START + i as u16,
                            card,
                            source: LinkCardSource::Trash(controller),
                        });
                    }
                }
            }
            LinkFromZone::DigivolutionSources => {
                // Lift from THIS Digimon's own digivolution sources (the host's
                // under-cards). The stack top is the live Digimon and is not an
                // eligible source.
                if let Some(perm) = ctx
                    .game
                    .player(host.player)
                    .battle_area
                    .get(host.index as usize)
                {
                    let top_pos = perm.card_sources.len().saturating_sub(1);
                    let base = SOURCE_SELECT_START + host.index as u16 * SOURCES_PER_FIELD;
                    for (i, c) in perm.card_sources.iter().enumerate() {
                        if i == top_pos {
                            continue;
                        }
                        let card = c.handle();
                        if eval_predicate(filter, &read_ctx, PredicateSubject::Card(card)) {
                            out.push(LinkCandidate {
                                action_id: base + i as u16,
                                card,
                                source: LinkCardSource::DigivolutionSource(host),
                            });
                        }
                    }
                }
            }
        }
    }

    out
}
