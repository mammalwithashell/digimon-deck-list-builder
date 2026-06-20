//! Lowering for the `link_cards` DSL step (Gap 2).
//!
//! The authoring verb over the shipped `Game::link_chosen_card_into_host`
//! primitive. Drives BT25-060 Rebootmon / BT25-075 Vulcanusmon / BT25-089
//! Kazuki & Itsuki: "link 1..N [Appmon] cards from your hand / trash /
//! digivolution sources to a Digimon without paying the cost".
//!
//! ## Selection flow (faithful to DCGO `ST22_12`)
//!
//! Per pick (looping up to `count`):
//!   1. Determine which `from` zones currently hold ≥1 filter-matching card.
//!   2. If ≥2 zones qualify → install a zone-choice (`select_effect_choice`)
//!      first, exposing the choice to the RL action space; if exactly 1 → use
//!      it directly; if 0 → stop (the first mandatory `Exactly` pick with no
//!      candidate is a no-op).
//!   3. Install a single-zone card select over that zone's filtered cards
//!      (`UpTo` ⇒ declinable/PASS-able; `Exactly` ⇒ mandatory).
//!   4. For `to: own_digimon` → install an own-Digimon host select; for
//!      `to: self` → the host is the effect's own permanent.
//!   5. Attach via `ctx.link_chosen_card_into_host(host, card, source)` and
//!      loop for the next pick.
//!
//! Cost: `cost` is the per-card memory the controller pays; the cards this step
//! serves all pay 0 (free / reduced-to-0), so the cost is paid up-front before
//! the attach. The primitive itself never pays.

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledLinkCount, CompiledLinkSourceZone, CompiledLinkTo, CompiledPredicate, CompiledStep,
};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use crate::dsl_cards::step::{run_steps_with_runtime, StepRuntime};
use crate::effect_context::EffectContext;
use crate::enums::LinkCardSource;
use crate::permanent::{OptionState, PermanentHandle};
use crate::selection::SourceSelectionRef;

/// Try to lower a `CompiledStep::LinkCards`. Returns `true` if it handled the
/// step (installing a selection or completing synchronously), `false` if the
/// step is not a `LinkCards`.
pub fn try_install(
    step: &CompiledStep,
    tail: &[CompiledStep],
    ctx: &mut EffectContext<'_>,
    bindings: Bindings,
    runtime: &StepRuntime,
) -> bool {
    let CompiledStep::LinkCards {
        from,
        filter,
        to,
        count,
        cost,
        prompt,
        bind_as,
        host_filter,
        exclude_source,
    } = step
    else {
        return false;
    };

    let spec = Arc::new(LinkCardsSpec {
        from: from.clone(),
        filter: filter.clone(),
        to: *to,
        count: *count,
        cost: *cost,
        prompt: prompt.clone(),
        bind_as: bind_as.clone(),
        host_filter: host_filter.clone(),
        exclude_source: *exclude_source,
    });

    install_pick(ctx, spec, 0, Arc::new(tail.to_vec()), bindings, runtime.clone());
    true
}

/// Immutable per-step configuration, shared across the recursive pick loop.
struct LinkCardsSpec {
    from: Vec<CompiledLinkSourceZone>,
    filter: CompiledPredicate,
    to: CompiledLinkTo,
    count: CompiledLinkCount,
    cost: u8,
    prompt: Option<String>,
    /// Optional binding capturing the linked card(s); appended to on each real
    /// link in `attach_and_continue` (created lazily, so it stays absent after a
    /// full decline). `None` ⇒ no capture.
    bind_as: Option<String>,
    /// Optional host predicate (`to: own_digimon`) — the link card's link
    /// requirement, applied in `install_host_select` on top of the base
    /// Digimon/Standard gate. `None` ⇒ any standing own Digimon.
    host_filter: Option<CompiledPredicate>,
    /// Exclude the effect's source permanent from host candidates ("other
    /// Digimon", EX11-027).
    exclude_source: bool,
}

impl LinkCardsSpec {
    /// The maximum number of picks (`Exactly(n)` and `UpTo(n)` both cap at n).
    fn max_picks(&self) -> u8 {
        match self.count {
            CompiledLinkCount::Exactly(n) | CompiledLinkCount::UpTo(n) => n,
        }
    }

    /// Whether each pick is declinable (PASS exposed). `UpTo` ⇒ yes.
    fn pick_is_optional(&self) -> bool {
        matches!(self.count, CompiledLinkCount::UpTo(_))
    }
}

/// Whether a single source zone currently holds ≥1 filter-matching card.
/// Drives the per-pick eligibility / zone-choice decision; the actual card
/// identity is re-derived inside the card-select install.
fn zone_has_candidate(
    ctx: &EffectContext<'_>,
    zone: CompiledLinkSourceZone,
    filter: &CompiledPredicate,
    bindings: &Bindings,
) -> bool {
    let player = ctx.player;
    let read = ctx.as_read();
    let matches = |card: CardHandle| {
        eval_predicate_with_bindings(filter, &read, PredicateSubject::Card(card), Some(bindings))
    };
    match zone {
        CompiledLinkSourceZone::Hand => ctx
            .game
            .player(player)
            .hand
            .iter()
            .any(|c| matches(c.handle())),
        CompiledLinkSourceZone::Trash => ctx
            .game
            .player(player)
            .trash
            .iter()
            .any(|c| matches(c.handle())),
        CompiledLinkSourceZone::SelfSources => ctx
            .source_permanent
            .is_some_and(|host| permanent_has_source_candidate(ctx, host, filter, bindings)),
        CompiledLinkSourceZone::OwnDigimonSources => (0..ctx.game.player(player).battle_area.len())
            .any(|index| {
                permanent_has_source_candidate(
                    ctx,
                    PermanentHandle {
                        player,
                        index: index as u8,
                    },
                    filter,
                    bindings,
                )
            }),
        // Gap 3b — the in-play Option is a candidate iff it is currently held in
        // `pending_option` and owned by the controller. The filter is ignored:
        // "link this card" names the Option itself, not a filtered pool.
        CompiledLinkSourceZone::SelfOption => ctx
            .game
            .pending_option
            .as_ref()
            .is_some_and(|p| p.owner == player),
    }
}

/// Whether `perm` has a filter-matching under-source (any card BELOW the top).
fn permanent_has_source_candidate(
    ctx: &EffectContext<'_>,
    perm: PermanentHandle,
    filter: &CompiledPredicate,
    bindings: &Bindings,
) -> bool {
    let read = ctx.as_read();
    let Some(p) = ctx
        .game
        .player(perm.player)
        .battle_area
        .get(perm.index as usize)
    else {
        return false;
    };
    let top = p.card_sources.len().saturating_sub(1);
    p.card_sources.iter().take(top).any(|c| {
        eval_predicate_with_bindings(filter, &read, PredicateSubject::Card(c.handle()), Some(bindings))
    })
}

/// Install one pick of the link loop (`pick_index` is 0-based). Determines the
/// eligible zones, branches on zone-count, and chains forward.
fn install_pick(
    ctx: &mut EffectContext<'_>,
    spec: Arc<LinkCardsSpec>,
    pick_index: u8,
    tail: Arc<Vec<CompiledStep>>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    // Loop bound reached → run the captured tail and stop.
    if pick_index >= spec.max_picks() {
        run_captured_tail(ctx, &tail, bindings, &runtime);
        return;
    }

    // Which `from` zones currently hold a candidate?
    let eligible: Vec<CompiledLinkSourceZone> = spec
        .from
        .iter()
        .copied()
        .filter(|z| zone_has_candidate(ctx, *z, &spec.filter, &bindings))
        .collect();

    if eligible.is_empty() {
        // No candidate in any zone. For the very first mandatory pick this
        // means the clause does nothing; for a later pick it ends the loop.
        // Either way, the captured tail runs (it follows the whole step).
        run_captured_tail(ctx, &tail, bindings, &runtime);
        return;
    }

    if eligible.len() == 1 {
        install_card_select(ctx, spec, pick_index, eligible[0], tail, bindings, runtime);
        return;
    }

    // ≥2 zones with candidates → zone-choice prompt first (DCGO ST22_12).
    install_zone_choice(ctx, spec, pick_index, eligible, tail, bindings, runtime);
}

/// Install the zone-choice prompt, then chain into the card select for the
/// chosen zone. Faithful to DCGO's `SetBoolSelection` "From hand / From
/// digivolution cards" prompt; generalised to ≥2 zones via `select_effect_choice`.
fn install_zone_choice(
    ctx: &mut EffectContext<'_>,
    spec: Arc<LinkCardsSpec>,
    pick_index: u8,
    eligible: Vec<CompiledLinkSourceZone>,
    tail: Arc<Vec<CompiledStep>>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let labels: Vec<String> = eligible.iter().map(|z| zone_label(*z)).collect();
    let prompt = "From which area do you select a card to link?".to_string();
    ctx.select_effect_choice(&prompt, labels, move |cb_ctx, choice_index| {
        let Some(zone) = eligible.get(choice_index).copied() else {
            return;
        };
        install_card_select(cb_ctx, spec, pick_index, zone, tail, bindings, runtime);
    });
}

fn zone_label(zone: CompiledLinkSourceZone) -> String {
    match zone {
        CompiledLinkSourceZone::Hand => "From hand".to_string(),
        CompiledLinkSourceZone::Trash => "From trash".to_string(),
        CompiledLinkSourceZone::SelfSources => "From this Digimon's digivolution cards".to_string(),
        CompiledLinkSourceZone::OwnDigimonSources => {
            "From your Digimon's digivolution cards".to_string()
        }
        CompiledLinkSourceZone::SelfOption => "This card".to_string(),
    }
}

/// Install a single-zone card select over `zone`'s filtered candidates. The
/// chosen card is carried forward to the host select / attach.
fn install_card_select(
    ctx: &mut EffectContext<'_>,
    spec: Arc<LinkCardsSpec>,
    pick_index: u8,
    zone: CompiledLinkSourceZone,
    tail: Arc<Vec<CompiledStep>>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let optional = spec.pick_is_optional();
    let prompt = spec
        .prompt
        .clone()
        .unwrap_or_else(|| "Choose a card to link".to_string());

    match zone {
        CompiledLinkSourceZone::Hand | CompiledLinkSourceZone::Trash => {
            install_card_select_zone_indexed(
                ctx, spec, pick_index, zone, optional, prompt, tail, bindings, runtime,
            );
        }
        CompiledLinkSourceZone::SelfSources | CompiledLinkSourceZone::OwnDigimonSources => {
            install_card_select_sources(
                ctx, spec, pick_index, zone, optional, prompt, tail, bindings, runtime,
            );
        }
        CompiledLinkSourceZone::SelfOption => {
            // Gap 3b — "link THIS card": the card is fixed (the in-play Option),
            // so there is no card-pick to surface. Resolve to the option card
            // and proceed directly to host selection.
            let card = ctx.game.pending_option.as_ref().map(|p| p.card.handle());
            let player = ctx.player;
            match card {
                Some(card) => after_card_chosen(
                    ctx,
                    spec,
                    pick_index,
                    card,
                    LinkCardSource::OptionInPlay(player),
                    tail,
                    bindings,
                    runtime,
                ),
                None => run_captured_tail(ctx, &tail, bindings, &runtime),
            }
        }
    }
}

/// Card select for the hand / trash zones (index-addressed). On pick, the
/// chosen card handle is captured and the flow proceeds to host selection.
#[allow(clippy::too_many_arguments)]
fn install_card_select_zone_indexed(
    ctx: &mut EffectContext<'_>,
    spec: Arc<LinkCardsSpec>,
    pick_index: u8,
    zone: CompiledLinkSourceZone,
    optional: bool,
    prompt: String,
    tail: Arc<Vec<CompiledStep>>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let player = ctx.player;
    let filter = spec.filter.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let filter_bindings = bindings.clone();
    let is_trash = matches!(zone, CompiledLinkSourceZone::Trash);

    let candidate_at = move |game: &crate::game::Game, idx: usize| -> Option<CardHandle> {
        let card = if is_trash {
            game.player(player).trash.get(idx)
        } else {
            game.player(player).hand.get(idx)
        }
        .map(|c| c.handle())?;
        let read = crate::effect_context::EffectReadContext::new_with_source_kind(
            game,
            source_card,
            source_permanent,
            source_kind,
            player,
        );
        eval_predicate_with_bindings(
            &filter,
            &read,
            PredicateSubject::Card(card),
            Some(&filter_bindings),
        )
        .then_some(card)
    };

    // Clone the captured state for the decline path before `on_pick` moves it.
    let decline_state = optional.then(|| {
        (
            Arc::clone(&spec),
            Arc::clone(&tail),
            bindings.clone(),
            runtime.clone(),
        )
    });

    let on_pick = {
        let candidate_at = candidate_at.clone();
        move |cb_ctx: &mut EffectContext<'_>, idx: usize| {
            let Some(card) = candidate_at(cb_ctx.game, idx) else {
                // Stale index: do not run the tail here — declining/empty paths
                // are handled by the decline callback / next-pick chain.
                return;
            };
            let source = if is_trash {
                LinkCardSource::Trash(player)
            } else {
                LinkCardSource::Hand(player)
            };
            after_card_chosen(cb_ctx, spec, pick_index, card, source, tail, bindings, runtime);
        }
    };

    let filter_fn = move |game: &crate::game::Game, idx: usize| candidate_at(game, idx).is_some();

    if is_trash {
        ctx.select_trash(player, &prompt, optional, filter_fn, on_pick);
    } else {
        ctx.select_hand(player, &prompt, optional, filter_fn, on_pick);
    }
    if let Some((spec, tail, bindings, runtime)) = decline_state {
        install_optional_decline(ctx, spec, pick_index, tail, bindings, runtime);
    }
}

/// Card select for the digivolution-source zones. Uses `select_own_sources`
/// with min/max = 0/1 (the loop owns the multi-pick count, so each source pick
/// links exactly one card). For `SelfSources` the picker is restricted to the
/// effect's own permanent; for `OwnDigimonSources` any own permanent's sources
/// are eligible.
#[allow(clippy::too_many_arguments)]
fn install_card_select_sources(
    ctx: &mut EffectContext<'_>,
    spec: Arc<LinkCardsSpec>,
    pick_index: u8,
    zone: CompiledLinkSourceZone,
    optional: bool,
    prompt: String,
    tail: Arc<Vec<CompiledStep>>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let player = ctx.player;
    let filter = spec.filter.clone();
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let restrict_to: Option<PermanentHandle> = match zone {
        CompiledLinkSourceZone::SelfSources => ctx.source_permanent,
        _ => None,
    };
    let filter_bindings = bindings.clone();

    // `select_own_sources` exposes PASS once min is met; min=0 for UpTo,
    // min=1 for Exactly (the pick is mandatory). max=1: exactly one card.
    let min = if optional { 0 } else { 1 };

    let matches_source = move |game: &crate::game::Game, sref: SourceSelectionRef| -> bool {
        if sref.permanent.player != player {
            return false;
        }
        if let Some(only) = restrict_to {
            if sref.permanent != only {
                return false;
            }
        }
        let read = crate::effect_context::EffectReadContext::new_with_source_kind(
            game,
            source_card,
            source_permanent,
            source_kind,
            player,
        );
        eval_predicate_with_bindings(
            &filter,
            &read,
            PredicateSubject::Card(sref.card),
            Some(&filter_bindings),
        )
    };

    ctx.select_own_sources(
        &prompt,
        min,
        1,
        matches_source,
        move |cb_ctx, picked: Vec<SourceSelectionRef>| {
            let Some(sref) = picked.first().copied() else {
                // PASS / no pick (only reachable when min == 0): end the loop.
                run_captured_tail(cb_ctx, &tail, bindings, &runtime);
                return;
            };
            let source = LinkCardSource::DigivolutionSource(sref.permanent);
            after_card_chosen(
                cb_ctx, spec, pick_index, sref.card, source, tail, bindings, runtime,
            );
        },
    );
}

/// Attach the optional-decline callback to a just-installed hand/trash select:
/// declining (PASS) ends the loop and runs the captured tail.
fn install_optional_decline(
    ctx: &mut EffectContext<'_>,
    _spec: Arc<LinkCardsSpec>,
    _pick_index: u8,
    tail: Arc<Vec<CompiledStep>>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let player = ctx.player;
    if let Some(pending) = ctx.game.pending_selection.as_mut() {
        pending.on_decline = Some(Box::new(move |game: &mut crate::game::Game| {
            let mut decline_ctx = EffectContext::new_with_source_kind(
                game,
                source_card,
                source_permanent,
                source_kind,
                player,
            );
            run_captured_tail(&mut decline_ctx, &tail, bindings, &runtime);
        }));
    }
}

/// After a card has been chosen, resolve the host (self or a selected own
/// Digimon), pay the per-card cost, attach, then loop for the next pick.
#[allow(clippy::too_many_arguments)]
fn after_card_chosen(
    ctx: &mut EffectContext<'_>,
    spec: Arc<LinkCardsSpec>,
    pick_index: u8,
    card: CardHandle,
    source: LinkCardSource,
    tail: Arc<Vec<CompiledStep>>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    match spec.to {
        CompiledLinkTo::SelfPermanent => {
            let Some(host) = ctx.source_permanent else {
                // No own permanent to attach to (e.g. source already gone):
                // skip the attach but keep the loop honest by ending it.
                run_captured_tail(ctx, &tail, bindings, &runtime);
                return;
            };
            attach_and_continue(ctx, spec, pick_index, host, card, source, tail, bindings, runtime);
        }
        CompiledLinkTo::OwnDigimon => {
            install_host_select(ctx, spec, pick_index, card, source, tail, bindings, runtime);
        }
    }
}

/// Install the own-Digimon host select for `to: own_digimon`. The host is any
/// standing own Digimon. The pick is mandatory (the card was already chosen).
#[allow(clippy::too_many_arguments)]
fn install_host_select(
    ctx: &mut EffectContext<'_>,
    spec: Arc<LinkCardsSpec>,
    pick_index: u8,
    card: CardHandle,
    source: LinkCardSource,
    tail: Arc<Vec<CompiledStep>>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let player = ctx.player;
    // §4.5.2 — the optional author host predicate (the link card's link
    // requirement, e.g. EX11-027 "host has [Maquinamon] in text"). Evaluated
    // against each candidate host on top of the base Digimon/Standard gate, so
    // the select never offers an illegal host. `None` ⇒ any standing Digimon.
    let auth_host_filter = spec.host_filter.clone();
    let exclude_source = spec.exclude_source;
    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    let source_kind = ctx.source_kind;
    let filter_bindings = bindings.clone();
    let host_filter = move |game: &crate::game::Game, handle: PermanentHandle| -> bool {
        // "1 of your OTHER Digimon" — drop the effect's own source permanent.
        if exclude_source && source_permanent == Some(handle) {
            return false;
        }
        let base_ok = game
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|p| {
                game.permanent_is_digimon_for_rules(handle)
                    && matches!(p.option_state, OptionState::Standard)
            })
            .unwrap_or(false);
        if !base_ok {
            return false;
        }
        match &auth_host_filter {
            None => true,
            Some(pred) => {
                let read = crate::effect_context::EffectReadContext::new_with_source_kind(
                    game,
                    source_card,
                    source_permanent,
                    source_kind,
                    player,
                );
                eval_predicate_with_bindings(
                    pred,
                    &read,
                    PredicateSubject::Permanent(handle),
                    Some(&filter_bindings),
                )
            }
        }
    };

    // No eligible host → cannot attach this pick; end the loop. (The card was
    // chosen but there is nowhere to put it — degrade gracefully without a
    // silent auto-pick, matching "1 of your Digimon" with none present.)
    let any_host = (0..ctx.game.player(player).battle_area.len()).any(|i| {
        host_filter(
            ctx.game,
            PermanentHandle {
                player,
                index: i as u8,
            },
        )
    });
    if !any_host {
        run_captured_tail(ctx, &tail, bindings, &runtime);
        return;
    }

    ctx.select_own_permanent(
        "Choose a Digimon to link the card to",
        false,
        host_filter,
        move |cb_ctx, host: PermanentHandle| {
            attach_and_continue(
                cb_ctx, spec, pick_index, host, card, source, tail, bindings, runtime,
            );
        },
    );
}

/// Pay the per-card cost, attach the card to the host via the primitive, then
/// install the next pick.
#[allow(clippy::too_many_arguments)]
fn attach_and_continue(
    ctx: &mut EffectContext<'_>,
    spec: Arc<LinkCardsSpec>,
    pick_index: u8,
    host: PermanentHandle,
    card: CardHandle,
    source: LinkCardSource,
    tail: Arc<Vec<CompiledStep>>,
    mut bindings: Bindings,
    runtime: StepRuntime,
) {
    // Pay the per-card cost (currently always 0 for the cards this serves; the
    // primitive does not pay). `pay_memory` deducts from the active player's
    // memory exactly like `commit_digimon_link`.
    if spec.cost > 0 {
        let _ = ctx.game.pay_memory(spec.cost as u16);
    }

    let _ = ctx.link_chosen_card_into_host(host, card, source);

    // Capture the just-linked card into the optional `bind_as` CardList. The
    // slot is created only here (on a real link), so it stays absent after a
    // full decline — a downstream `if { binding_present }` then runs the
    // follow-up exclusively when ≥1 card was linked (BT25-060's `if (linked)`).
    if let Some(name) = &spec.bind_as {
        let mut list = bindings.get_card_list(name).unwrap_or_default();
        list.push(card);
        bindings.insert_card_list(name, list);
    }

    install_pick(ctx, spec, pick_index + 1, tail, bindings, runtime);
}

/// Run the steps that followed the whole `link_cards` step, preserving the
/// active trigger context.
fn run_captured_tail(
    ctx: &mut EffectContext<'_>,
    tail: &[CompiledStep],
    mut bindings: Bindings,
    runtime: &StepRuntime,
) {
    if tail.is_empty() {
        return;
    }
    let previous = ctx.game.current_trigger_context.clone();
    run_steps_with_runtime(tail, ctx, &mut bindings, runtime);
    crate::dsl_cards::step::drain_dsl_outer_tail(ctx);
    ctx.game.current_trigger_context = previous;
}
