//! Physical bottom-prefix payment. The chooser sees declared and payable counts.
use crate::count_choice::{CountChoiceHost, CountChoicePage, CountChoiceValue};
use crate::dsl_cards::{
    bindings::Bindings,
    predicate::{eval_predicate, PredicateSubject},
    step::StepRuntime,
};
use crate::resume::{ResumeDecline, ResumeFrame, ResumeProvenance, ResumeSelectKind, ResumeStack};
use crate::{card_source::CardHandle, effect_context::EffectContext, permanent::PermanentHandle};
use digimon_dsl::compiled::{CompiledPredicate, CompiledStep};
#[derive(Debug, Clone)]
pub struct SourcePrefixHostState {
    pub filter: CompiledPredicate,
    pub minimum: u64,
    pub candidates: Vec<(u16, CountChoiceHost)>,
}
#[derive(Debug, Clone)]
pub struct SourcePrefixCountState {
    pub host: CountChoiceHost,
    pub filter: CompiledPredicate,
    pub minimum: u64,
    pub sources: Vec<CardHandle>,
}
fn candidates(
    ctx: &crate::effect_context::EffectReadContext<'_>,
    filter: &CompiledPredicate,
    minimum: u64,
) -> Vec<PermanentHandle> {
    ctx.game
        .player(ctx.player)
        .battle_area
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let h = PermanentHandle {
                player: ctx.player,
                index: i as u8,
            };
            let count = p
                .card_sources
                .iter()
                .take(p.card_sources.len().saturating_sub(1))
                .filter(|c| {
                    !ctx.game
                        .source_card_trash_blocked(h, c.handle(), ctx.player)
                })
                .count() as u64;
            (count >= minimum
                && eval_predicate(filter, ctx, PredicateSubject::Permanent(h))
                && !ctx
                    .game
                    .permanent_is_unaffected_by_effect(h, ctx.player, ctx.source_kind))
            .then_some(h)
        })
        .collect()
}
pub fn is_payable(
    ctx: &crate::effect_context::EffectReadContext<'_>,
    filter: &CompiledPredicate,
    minimum: u64,
) -> bool {
    !candidates(ctx, filter, minimum).is_empty()
}
pub fn install(
    ctx: &mut EffectContext<'_>,
    filter: CompiledPredicate,
    minimum: u64,
    optional: bool,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let hosts = candidates(&ctx.as_read(), &filter, minimum);
    if hosts.is_empty() {
        ctx.cost_unpayable = true;
        ctx.game.dsl_clause_aborted = true;
        return;
    }
    let candidates = hosts
        .into_iter()
        .map(|h| {
            (
                100 + h.index as u16,
                CountChoiceHost {
                    player: h.player,
                    field_index: h.index,
                    battle_identity: ctx.game.capture_digivolve_host_identity(h),
                },
            )
        })
        .collect::<Vec<_>>();
    let state = SourcePrefixHostState {
        filter,
        minimum,
        candidates,
    };
    let prov = ResumeProvenance {
        source_card: ctx.source_card,
        source_permanent: ctx.source_permanent,
        source_kind: ctx.source_kind,
        controller: ctx.player,
        override_pin: ctx.override_selecting_player(),
    };
    let previous_phase = ctx.game.current_phase;
    ctx.game.current_phase = crate::enums::GamePhase::SelectTarget;
    ctx.game.pending_selection = Some(crate::selection::PendingSelection {
        kind: crate::selection::SelectionKind::OwnField,
        selecting_player: ctx.override_selecting_player().unwrap_or(ctx.player),
        previous_phase,
        valid_action_ids: state.candidates.iter().map(|c| c.0).collect(),
        is_optional: optional,
        prompt: "Choose a host for the bottom-source payment".into(),
        effect_choices: None,
        source_card: ctx.source_card,
        source_permanent: ctx.source_permanent,
        source_kind: ctx.source_kind,
        callback: Box::new(|_, _| panic!("source prefix payment requires typed resume")),
        on_decline: None,
        zone_owner: Some(ctx.player),
    });
    ctx.game.pending_selection_resume = Some(ResumeStack {
        frames: vec![ResumeFrame::RunTail {
            prov,
            select_kind: ResumeSelectKind::SourcePrefixHost(state),
            bind_as: None,
            inner_tail: std::sync::Arc::new(tail),
            outer_conts: Vec::new(),
            bindings,
            runtime,
            trigger_context: ctx.game.current_trigger_context.clone(),
            decline: if optional {
                ResumeDecline::RunTail {
                    tail: std::sync::Arc::new(Vec::new()),
                    aborts_clause: true,
                }
            } else {
                ResumeDecline::None
            },
        }],
    });
}
pub fn select_host(
    ctx: &mut EffectContext<'_>,
    state: SourcePrefixHostState,
    action: u16,
    tail: Vec<CompiledStep>,
    bindings: Bindings,
    runtime: StepRuntime,
) {
    let Some(pin) = state.candidates.iter().find(|v| v.0 == action).map(|v| v.1) else {
        ctx.game.dsl_clause_aborted = true;
        return;
    };
    let Some(host) = ctx
        .game
        .resolve_digivolve_host_identity(pin.player, pin.battle_identity)
    else {
        ctx.game.dsl_clause_aborted = true;
        return;
    };
    if !candidates(&ctx.as_read(), &state.filter, state.minimum).contains(&host) {
        ctx.game.dsl_clause_aborted = true;
        return;
    }
    let p = &ctx.game.player(host.player).battle_area[host.index as usize];
    let sources = p
        .card_sources
        .iter()
        .take(p.card_sources.len().saturating_sub(1))
        .map(|c| c.handle())
        .collect::<Vec<_>>();
    let mut payable = 0;
    let mut values = Vec::new();
    for (i, c) in sources.iter().enumerate() {
        if !ctx.game.source_card_trash_blocked(host, *c, ctx.player) {
            payable += 1;
        }
        if i as u64 + 1 >= state.minimum && payable >= state.minimum {
            values.push(CountChoiceValue {
                declared: i as u64 + 1,
                payable,
            });
        }
    }
    let current = CountChoiceHost {
        field_index: host.index,
        ..pin
    };
    let Some(page) = CountChoicePage::new(values, Some(current)) else {
        ctx.game.dsl_clause_aborted = true;
        return;
    };
    let payment = SourcePrefixCountState {
        host: current,
        filter: state.filter,
        minimum: state.minimum,
        sources,
    };
    crate::count_choice::install_count_choice(
        ctx,
        page.clone(),
        false,
        "Choose a physical bottom-source prefix".into(),
        None,
        tail,
        bindings,
        runtime,
    );
    if let Some(ResumeFrame::RunTail { select_kind, .. }) = ctx
        .game
        .pending_selection_resume
        .as_mut()
        .and_then(|s| s.frames.last_mut())
    {
        *select_kind = ResumeSelectKind::SourcePrefixCount {
            page,
            state: payment,
        };
    }
}
pub fn commit(
    ctx: &mut EffectContext<'_>,
    state: &SourcePrefixCountState,
    value: CountChoiceValue,
) -> bool {
    let Some(host) = ctx
        .game
        .resolve_digivolve_host_identity(state.host.player, state.host.battle_identity)
    else {
        return false;
    };
    let Ok(n) = usize::try_from(value.declared) else {
        return false;
    };
    if n > state.sources.len() || value.declared < state.minimum {
        return false;
    }
    if !candidates(&ctx.as_read(), &state.filter, state.minimum).contains(&host) {
        return false;
    }
    let p = &ctx.game.player(host.player).battle_area[host.index as usize];
    if p.card_sources.len() <= n
        || !p
            .card_sources
            .iter()
            .take(n)
            .map(|c| c.handle())
            .eq(state.sources[..n].iter().copied())
    {
        return false;
    }
    let cards = state.sources[..n]
        .iter()
        .copied()
        .filter(|c| !ctx.game.source_card_trash_blocked(host, *c, ctx.player))
        .collect::<Vec<_>>();
    if cards.len() as u64 != value.payable || value.payable < state.minimum {
        return false;
    }
    let old_controller = ctx.game.effect_source_player.replace(ctx.player);
    let paid = ctx.game.trash_source_prefix_payment_batch(host, &cards);
    ctx.game.effect_source_player = old_controller;
    paid as u64 >= state.minimum
}

/// Hash only typed immutable payment pins, including inside parked outer costs.
pub(crate) fn active_prefix_pins(stack: &ResumeStack) -> Option<String> {
    match stack.frames.last()? {
        ResumeFrame::RunTail {
            select_kind: ResumeSelectKind::SourcePrefixHost(state),
            ..
        } => Some(format!("{state:?}")),
        ResumeFrame::RunTail {
            select_kind: ResumeSelectKind::SourcePrefixCount { state, .. },
            ..
        } => Some(format!("{state:?}")),
        ResumeFrame::FieldTopSourceBatch { inner, .. }
        | ResumeFrame::DelayCancelAfterSelection { inner, .. }
        | ResumeFrame::DelayHandDigivolveAfterSelection { inner, .. }
        | ResumeFrame::DelayDnaAfterSelection { inner, .. }
        | ResumeFrame::DelayFieldDnaAfterPayment { inner, .. }
        | ResumeFrame::DelayPlayFromHandAfterSelection { inner, .. }
        | ResumeFrame::DelayPlayFromUnionAfterSelection { inner, .. }
        | ResumeFrame::DelayBodyAfterCost { inner, .. } => active_prefix_pins(inner),
        _ => None,
    }
}
