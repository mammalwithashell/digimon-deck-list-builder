// Included inside replacement.rs; this is an owned continuation for ONE event.
#[derive(Clone)]
pub struct ReplacementWindow {
    timing: EffectTiming,
    subject: ReplacementSubject,
    stable_subject: Option<(PlayerId, CardHandle, u64)>,
    cause: ReplacementCause,
    destination: Option<Zone>,
    cause_player: Option<PlayerId>,
    initiating_source_card: Option<CardHandle>,
    effect_digivolve_card: Option<CardHandle>,
    play_declaration: Option<crate::game::PendingWouldPlayResume>,
    event_cause: Option<crate::trigger_context::EventCause>,
    trigger_context: Option<crate::trigger_context::TriggerContext>,
    deletion_receipt: Option<u64>,
    turn_player: PlayerId,
    remaining: Vec<WindowCandidate>,
    // Preventing departure does not resolve the interrupted cause (Q7274; 16-24-4 [3.6 numbering; deleted in 4.3 — see changelog-3.6-to-4.3.md]).
    cancelled: bool,
    deletion_batch: Option<DeletionWindowBatch>,
    departure: Option<DepartureWindow>,
}
// Preserve the established window payload. An owned play declaration is
// separately hashed only when it is not already represented by global pending state.
impl std::fmt::Debug for ReplacementWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("ReplacementWindow");
        d.field("timing", &self.timing);
        d.field("subject", &self.subject);
        d.field("stable_subject", &self.stable_subject);
        d.field("cause", &self.cause);
        d.field("destination", &self.destination);
        d.field("cause_player", &self.cause_player);
        d.field("initiating_source_card", &self.initiating_source_card);
        d.field("effect_digivolve_card", &self.effect_digivolve_card);
        d.field("event_cause", &self.event_cause);
        d.field("trigger_context", &self.trigger_context);
        d.field("deletion_receipt", &self.deletion_receipt);
        d.field("turn_player", &self.turn_player);
        d.field("remaining", &self.remaining);
        d.field("cancelled", &self.cancelled);
        if let Some(departure) = &self.departure { d.field("departure", departure); }
        if let Some(batch) = &self.deletion_batch { d.field("deletion_batch", batch); }
        d.finish()
    }
}
impl ReplacementWindow {
    pub(crate) fn verification_play_declaration(
        &self,
        current: Option<&crate::game::PendingWouldPlayResume>,
    ) -> Option<&crate::game::PendingWouldPlayResume> {
        let owned = self.play_declaration.as_ref()?;
        if current.is_some_and(|current| current == owned) {
            None
        } else {
            Some(owned)
        }
    }

    pub(crate) fn owns_outer_tail(&self, source: CardHandle, player: PlayerId) -> bool {
        self.initiating_source_card == Some(source) && self.cause_player == Some(player)
    }
}
#[derive(Debug, Clone)]
struct WindowCandidate {
    candidate: Candidate,
    host_identity: Option<(PlayerId, CardHandle, u64)>,
}
#[derive(Debug, Clone)]
pub(crate) struct ReplacementOrder {
    choices: Vec<WindowCandidate>,
    page: usize,
}

fn capture_window_candidate(game: &mut crate::game::Game, candidate: Candidate) -> WindowCandidate {
    let host_identity = candidate
        .source_permanent
        .and_then(|h| capture_subject_card(game, ReplacementSubject::Permanent(h)));
    WindowCandidate {
        candidate,
        host_identity,
    }
}
fn window_candidate_matches(
    game: &crate::game::Game,
    old: &WindowCandidate,
    current: &Candidate,
) -> bool {
    if candidate_identity(old.candidate.source_card, &old.candidate.kind)
        != candidate_identity(current.source_card, &current.kind)
        || old.candidate.source_controller != current.source_controller
    {
        return false;
    }
    match (old.host_identity, current.source_permanent) {
        (Some((owner, _, identity)), Some(h)) => {
            h.player == owner
                && replacement_carrier(game, h).is_some_and(|p| p.battle_identity == Some(identity))
        }
        (None, None) => true,
        _ => false,
    }
}
fn eligible_window_candidates(
    game: &mut crate::game::Game,
    window: &mut ReplacementWindow,
) -> Vec<WindowCandidate> {
    let Some(subject) = resolve_subject_card(game, window.subject, window.stable_subject) else {
        return Vec::new();
    };
    window.subject = subject;
    let previous = game.effect_source_player;
    game.effect_source_player = window.cause_player;
    let prior_trigger = game.current_trigger_context.clone();
    game.current_trigger_context = window.trigger_context.clone();
    game.tick_declarative_effects();
    let current = collect_candidates(game, window.timing, subject, window.cause);
    game.effect_source_player = previous;
    game.current_trigger_context = prior_trigger;
    current
        .into_iter()
        .filter_map(|c| {
            window
                .remaining
                .iter()
                .find(|old| window_candidate_matches(game, old, &c))
                .map(|old| WindowCandidate {
                    candidate: c.clone(),
                    host_identity: old.host_identity,
                })
        })
        .collect()
}
fn start_replacement_window(
    game: &mut crate::game::Game,
    timing: EffectTiming,
    subject: ReplacementSubject,
    cause: ReplacementCause,
    destination: Option<Zone>,
) -> ReplacementOutcome {
    // If nothing is eligible, no first replacement can change eligibility.
    // Preserve the full candidate set below when any first effect can run.
    if collect_candidates(game, timing, subject, cause).is_empty() {
        return ReplacementOutcome::None;
    }
    let all = collect_event_candidates(game, timing, subject, cause, false);
    // No candidate can park or mutate this event: no identity pin is needed.
    if all.is_empty() {
        return ReplacementOutcome::None;
    }
    let remaining = all
        .into_iter()
        .map(|c| capture_window_candidate(game, c))
        .collect();
    let window = ReplacementWindow {
        timing,
        subject,
        stable_subject: capture_subject_card(game, subject),
        cause,
        destination,
        cause_player: game.effect_source_player,
        initiating_source_card: game.effect_source_card,
        event_cause: game.current_deletion_event_cause_override,
        trigger_context: game.current_trigger_context.clone(),
        deletion_receipt: game.current_deletion_receipt,
        turn_player: game.turn_player(),
        remaining,
        cancelled: false,
        deletion_batch: None, departure: None,
        play_declaration: if timing == EffectTiming::WhenPermanentWouldPlay {
            game.pending_would_play_resume.as_ref().filter(|r|
                matches!(subject, ReplacementSubject::Card(card, _) if card == r.card)).cloned()
        } else { None },
        effect_digivolve_card: if timing == EffectTiming::WhenPermanentWouldDigivolve {
            game.effect_digivolve_window_card(subject)
        } else {
            None
        },
    };
    advance_replacement_window(game, window, None).unwrap_or(ReplacementOutcome::None)
}
// Some(outcome) means the caller owns final commit. None means a typed decision owns it.
fn advance_replacement_window(
    game: &mut crate::game::Game,
    mut window: ReplacementWindow,
    mut chosen: Option<WindowCandidate>,
) -> Option<ReplacementOutcome> {
    loop {
        let eligible = eligible_window_candidates(game, &mut window);
        if eligible.is_empty() {
            return Some(if window.cancelled {
                ReplacementOutcome::Cancelled
            } else {
                ReplacementOutcome::None
            });
        }
        // Passive restrictions are continuous prohibitions, not selectable triggered effects.
        if eligible
            .iter()
            .any(|c| matches!(c.candidate.kind, CandidateKind::PassiveCancel))
        {
            return Some(ReplacementOutcome::Cancelled);
        }
        let side = if eligible
            .iter()
            .any(|c| c.candidate.source_controller == window.turn_player)
        {
            window.turn_player
        } else {
            1 - window.turn_player
        };
        let choices: Vec<_> = eligible
            .into_iter()
            .filter(|c| c.candidate.source_controller == side)
            .collect();
        let explicitly_selected = chosen.is_some();
        let selected = chosen.take().and_then(|old| {
            choices
                .iter()
                .find(|c| window_candidate_matches(game, &old, &c.candidate))
                .cloned()
        });
        if explicitly_selected && selected.is_none() {
            install_window_order(game, window, choices, 0);
            return None;
        }
        let selected = if let Some(c) = selected {
            c
        } else if choices.len() == 1 {
            choices[0].clone()
        } else {
            install_window_order(game, window, choices, 0);
            return None;
        };
        window
            .remaining
            .retain(|c| !window_candidate_matches(game, c, &selected.candidate));
        let c = &selected.candidate;
        if !c.is_mandatory {
            install_optional_selection(
                game,
                window.timing,
                c,
                c.source_controller,
                window.subject,
                window.cause,
                window.destination,
            );
            if let Some(crate::resume::ResumeFrame::OptionalReplacement(state)) = game
                .pending_selection_resume
                .as_mut()
                .and_then(|s| s.frames.last_mut())
            {
                state.window = Some(window);
                state.source_identity = selected.host_identity;
            }
            return None;
        }
        let base = game.parked_replacement.len();
        let outcome = run_window_candidate(game, &window, c);
        let outcome = if window.cancelled && outcome == ReplacementOutcome::CustomHandled {
            ReplacementOutcome::Cancelled
        } else { outcome };
        if game.pending_selection.is_some() {
            if let Some(parked) = game.parked_replacement.get_mut(base) {
                parked.window = Some(window);
            }
            wrap_replacement_body(game);
            return None;
        }
        if outcome == ReplacementOutcome::Cancelled {
            window.cancelled = true;
        } else if outcome != ReplacementOutcome::None {
            return Some(outcome);
        }
    }
}
fn finish_window_result(
    game: &mut crate::game::Game,
    mut window: ReplacementWindow,
    outcome: ReplacementOutcome,
) {
    if window.deletion_batch.is_some() {
        finish_deletion_candidate(game, window, outcome, false);
        return;
    }
    // A handled body supplies no new subject or destination and cannot revoke
    // prevention already remembered by this same immediate-effect window.
    let outcome = if window.cancelled && outcome == ReplacementOutcome::CustomHandled {
        ReplacementOutcome::Cancelled
    } else { outcome };
    if outcome == ReplacementOutcome::Cancelled {
        window.cancelled = true;
    }
    if matches!(
        outcome,
        ReplacementOutcome::None | ReplacementOutcome::Cancelled
    ) {
        let commit = window.clone();
        if let Some(outcome) = advance_replacement_window(game, window, None) {
            commit_window(game, commit, outcome);
        }
    } else {
        commit_window(game, window, outcome);
    }
}
fn commit_window(
    game: &mut crate::game::Game,
    mut window: ReplacementWindow,
    outcome: ReplacementOutcome,
) {
    if let Some(declaration) = window.play_declaration.take() {
        let original_card=declaration.card;
        let previous=game.pending_would_play_resume.replace(declaration);
        game.commit_pending_would_play(outcome);
        if let Some(previous)=previous.filter(|r|r.card!=original_card && !game.game_over && !game.player(r.player).is_eliminated) {
            game.pending_would_play_resume=Some(previous);
        }
        game.replacement_pending_outcome=None;
        return;
    }
    if let Some(card) = window.effect_digivolve_card {
        let outcome = if resolve_subject_card(game, window.subject, window.stable_subject).is_some()
        {
            outcome
        } else {
            ReplacementOutcome::Cancelled
        };
        game.commit_effect_would_digivolve(card, outcome, true);
        game.replacement_pending_outcome = None;
        return;
    }
    // The declaration still owns cleanup when its physical base left during
    // an immediate effect. Do not consume it from nested leave windows.
    if window.timing == EffectTiming::WhenPermanentWouldDigivolve
        && game
            .pending_would_digivolve_resume
            .as_ref()
            .is_some_and(|r| {
                r.window_started
                    && window
                        .stable_subject
                        .is_some_and(|(p, _, id)| p == r.player && id == r.host_identity)
            })
    {
        let resolved = resolve_subject_card(game, window.subject, window.stable_subject);
        let previous = game.effect_source_player;
        game.effect_source_player = window.cause_player;
        game.commit_pending_would_digivolve(if resolved.is_some() {
            outcome
        } else {
            ReplacementOutcome::Cancelled
        });
        game.effect_source_player = previous;
        game.replacement_pending_outcome = None;
        return;
    }
    let Some(subject) = resolve_subject_card(game, window.subject, window.stable_subject) else {
        return;
    };
    window.subject = subject;
    let previous = game.effect_source_player;
    game.effect_source_player = window.cause_player;
    // The Option disposal state machine consumes this result after the final
    // immediate decision; do not lose accepted cancellation for Card subjects.
    if let ReplacementSubject::Card(card, _) = subject {
        if game.pending_option.as_ref().is_some_and(|p| {
            p.card.handle() == card
                && p.resolution_phase == crate::selection::OptionResolutionPhase::Disposing
        }) {
            game.replacement_pending_outcome = Some(outcome);
        }
    }
    game.with_deletion_receipt(window.deletion_receipt, |g| {
        run_commit_with_flag(
            g,
            outcome != ReplacementOutcome::None,
            Some((window.timing, subject)),
            |g| {
                commit_deferred_outcome(
                    g,
                    subject,
                    window.cause,
                    window.event_cause,
                    window.destination,
                    outcome,
                )
            },
        )
    });
    game.effect_source_player = previous;
}
fn install_window_order(
    game: &mut crate::game::Game,
    window: ReplacementWindow,
    choices: Vec<WindowCandidate>,
    page: usize,
) {
    let c = &choices[0].candidate;
    install_optional_selection(
        game,
        window.timing,
        c,
        c.source_controller,
        window.subject,
        window.cause,
        window.destination,
    );
    if let Some(crate::resume::ResumeFrame::OptionalReplacement(state)) = game
        .pending_selection_resume
        .as_mut()
        .and_then(|s| s.frames.last_mut())
    {
        state.window = Some(window);
        state.order = Some(ReplacementOrder { choices, page });
    }
    refresh_window_order(game);
}
/// The exact declaration owned by the currently offered replacement window.
/// An enclosing payment hook must not capture this inner evolution's DSL tail.
pub(crate) fn active_effect_digivolve_window_card(
    stack: &crate::resume::ResumeStack,
) -> Option<CardHandle> {
    use crate::resume::ResumeFrame as F;
    match stack.frames.last()? {
        F::OptionalReplacement(s) => s.window.as_ref().and_then(|w| w.effect_digivolve_card),
        F::ReplacementBody { inner, .. }
        | F::FieldTopSourceBatch { inner, .. }
        | F::DelayCancelAfterSelection { inner, .. }
        | F::DelayHandDigivolveAfterSelection { inner, .. }
        | F::DelayDnaAfterSelection { inner, .. }
        | F::DelayFieldDnaAfterPayment { inner, .. }
        | F::DelayPlayFromHandAfterSelection { inner, .. }
        | F::DelayPlayFromUnionAfterSelection { inner, .. }
        | F::DelayBodyAfterCost { inner, .. } => active_effect_digivolve_window_card(inner),
        _ => None,
    }
}
fn active_window_order(stack: &crate::resume::ResumeStack) -> Option<&OptionalReplacementState> {
    use crate::resume::ResumeFrame as F;
    match stack.frames.last()? {
        F::OptionalReplacement(s) if s.order.is_some() => Some(s),
        F::ReplacementBody { inner, .. }
        | F::FieldTopSourceBatch { inner, .. }
        | F::DelayCancelAfterSelection { inner, .. }
        | F::DelayHandDigivolveAfterSelection { inner, .. }
        | F::DelayDnaAfterSelection { inner, .. }
        | F::DelayFieldDnaAfterPayment { inner, .. }
        | F::DelayPlayFromHandAfterSelection { inner, .. }
        | F::DelayPlayFromUnionAfterSelection { inner, .. }
        | F::DelayBodyAfterCost { inner, .. } => active_window_order(inner),
        _ => None,
    }
}
fn active_window_order_mut(
    stack: &mut crate::resume::ResumeStack,
) -> Option<&mut OptionalReplacementState> {
    use crate::resume::ResumeFrame as F;
    match stack.frames.last_mut()? {
        F::OptionalReplacement(s) if s.order.is_some() => Some(s),
        F::ReplacementBody { inner, .. }
        | F::FieldTopSourceBatch { inner, .. }
        | F::DelayCancelAfterSelection { inner, .. }
        | F::DelayHandDigivolveAfterSelection { inner, .. }
        | F::DelayDnaAfterSelection { inner, .. }
        | F::DelayFieldDnaAfterPayment { inner, .. }
        | F::DelayPlayFromHandAfterSelection { inner, .. }
        | F::DelayPlayFromUnionAfterSelection { inner, .. }
        | F::DelayBodyAfterCost { inner, .. } => active_window_order_mut(inner),
        _ => None,
    }
}
fn refresh_window_order(game: &mut crate::game::Game) {
    let Some(state) = game
        .pending_selection_resume
        .as_ref()
        .and_then(active_window_order)
    else {
        return;
    };
    let order = state.order.as_ref().unwrap();
    let mut rows: Vec<_> = order
        .choices
        .iter()
        .skip(order.page * 28)
        .take(28)
        .enumerate()
        .map(|(i, c)| crate::selection::EffectChoiceEntry {
            count_choice: None,
            label: c.candidate.effect_name.clone(),
            action_id: 30 + i as u16,
            source_card: Some(c.candidate.source_card),
            source_kind: Some(game.effect_source_kind_for_handle(c.candidate.source_card)),
            timing: Some(state.timing),
            is_optional: false,
            keyword: None,
            observation_metadata: Default::default(),
        })
        .collect();
    for (show, id, label) in [
        (order.page > 0, 58, "Previous page"),
        ((order.page + 1) * 28 < order.choices.len(), 59, "Next page"),
    ] {
        if show {
            rows.push(crate::selection::EffectChoiceEntry {
                count_choice: None,
                label: label.into(),
                action_id: id,
                source_card: None,
                source_kind: None,
                timing: None,
                is_optional: false,
                keyword: None,
                observation_metadata: Default::default(),
            });
        }
    }
    if let Some(p) = game.pending_selection.as_mut() {
        p.kind = SelectionKind::EffectChoice;
        p.is_optional = false;
        p.prompt = "Choose the next immediate replacement effect".into();
        p.valid_action_ids = rows.iter().map(|r| r.action_id).collect();
        p.effect_choices = Some(rows);
    }
}
pub(crate) fn navigate_replacement_order(game: &mut crate::game::Game, action: u16) -> bool {
    if action != 58 && action != 59 {
        return false;
    }
    let Some(state) = game
        .pending_selection_resume
        .as_mut()
        .and_then(active_window_order_mut)
    else {
        return false;
    };
    let order = state.order.as_mut().unwrap();
    if action == 58 {
        if order.page == 0 {
            return false;
        }
        order.page -= 1;
    } else {
        if (order.page + 1) * 28 >= order.choices.len() {
            return false;
        }
        order.page += 1;
    }
    refresh_window_order(game);
    true
}
fn run_window_step(
    game: &mut crate::game::Game,
    mut state: OptionalReplacementState,
    action: u16,
    is_pass: bool,
) {
    let mut window = state.window.take().expect("owned replacement window");
    if window.deletion_batch.is_some() {
        run_deletion_window_step(game, window, state, action, is_pass);
        return;
    }
    if let Some(order) = state.order.take() {
        let Some(index) = action
            .checked_sub(30)
            .filter(|n| *n < 28)
            .map(|n| order.page * 28 + n as usize)
        else {
            return;
        };
        let Some(chosen) = order.choices.get(index).cloned() else {
            return;
        };
        let commit = window.clone();
        if let Some(outcome) = advance_replacement_window(game, window, Some(chosen)) {
            commit_window(game, commit, outcome);
        }
        return;
    }
    if is_pass {
        finish_window_result(game, window, ReplacementOutcome::None);
        return;
    }
    let Some(subject) = resolve_subject_card(game, window.subject, window.stable_subject) else {
        return;
    };
    window.subject = subject;
    // Revalidate the selected physical effect; never substitute a fresh same-ID card.
    let old = WindowCandidate {
        candidate: Candidate {
            source_card: state.source_card,
            source_permanent: state.source_permanent,
            source_controller: state.controller,
            is_mandatory: false,
            effect_name: String::new(),
            kind: state.candidate_kind.clone(),
        },
        host_identity: state.source_identity,
    };
    let previous = game.effect_source_player;
    game.effect_source_player = window.cause_player;
    let prior_trigger = game.current_trigger_context.clone();
    game.current_trigger_context = window.trigger_context.clone();
    let candidate = collect_candidates(game, window.timing, subject, window.cause)
        .into_iter()
        .find(|c| window_candidate_matches(game, &old, c));
    game.effect_source_player = previous;
    game.current_trigger_context = prior_trigger;
    let Some(candidate) = candidate else {
        finish_window_result(game, window, ReplacementOutcome::None);
        return;
    };
    let base = game.parked_replacement.len();
    let outcome = run_window_candidate(game, &window, &candidate);
    if let Some(parked) = game.parked_replacement.get_mut(base) {
        if parked.subject == subject && parked.source_card == candidate.source_card {
            parked.timing = Some(window.timing);
            parked.window = Some(window);
            wrap_replacement_body(game);
            return;
        }
    }
    finish_window_result(game, window, outcome);
}

fn wrap_replacement_body(game: &mut crate::game::Game) {
    if let Some(inner) = game.pending_selection_resume.take() {
        game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::ReplacementBody {
                inner: Box::new(inner),
                outer_conts: Vec::new(),
            }],
        });
    }
}
fn run_window_candidate(
    game: &mut crate::game::Game,
    window: &ReplacementWindow,
    c: &Candidate,
) -> ReplacementOutcome {
    let prior_player = game.effect_source_player;
    let prior_card = game.effect_source_card;
    let prior_cause = game.current_deletion_event_cause_override;
    let prior_depth = game.replacement_depth;
    let prior_trigger = game.current_trigger_context.clone();
    game.current_trigger_context = window.trigger_context.clone();
    game.effect_source_player = window.cause_player;
    game.current_deletion_event_cause_override = window.event_cause;
    game.replacement_depth = prior_depth.saturating_add(1);
    if window.deletion_batch.is_none() {
        if let Some(context)=game.current_trigger_context.as_mut(){context.replacement_deletion_members.clear(); context.replacement_paid_prevention=None; context.replacement_deletion_cause=None;}
    }
    refresh_batch_candidate_coverage(game, window, c);
    // Eligibility belongs to the original event. Newly activated replacement
    // actions belong to the replacement's controller and physical source.
    game.effect_source_player = Some(c.source_controller);
    game.effect_source_card = Some(c.source_card);
    let result = game.with_deletion_receipt(window.deletion_receipt, |g| {
        run_mandatory_candidate(g, c, window.subject, window.cause, window.destination)
    });
    game.effect_source_player = prior_player;
    game.effect_source_card = prior_card;
    game.current_deletion_event_cause_override = prior_cause;
    game.replacement_depth = prior_depth;
    game.current_trigger_context = prior_trigger;
    result
}
fn refresh_batch_candidate_coverage(
    game: &mut crate::game::Game,
    window: &ReplacementWindow,
    c: &Candidate,
) {
    let Some(h) = window.subject.permanent() else {
        return;
    };
    let Some(parked) = game
        .player(h.player)
        .battle_area
        .get(h.index as usize)
        .map(|p| (h.player, p.top_card().handle()))
    else {
        return;
    };
    let id = candidate_identity(c.source_card, &c.kind);
    let updates: Vec<_> = game
        .pending_deletion_batch_rest
        .iter()
        .enumerate()
        .filter(|(_, r)| r.receipt == window.deletion_receipt && r.parked_member == Some(parked))
        .map(|(i, r)| {
            let member = game
                .player(r.player)
                .battle_area
                .iter()
                .position(|p| p.top_card().handle() == r.card)
                .map(|index| PermanentHandle {
                    player: r.player,
                    index: index as u8,
                });
            let covers = member.is_some_and(|h| {
                active_candidate_identities(
                    game,
                    window.timing,
                    ReplacementSubject::Permanent(h),
                    window.cause,
                )
                .contains(&id)
            });
            (i, covers)
        })
        .collect();
    for (i, covers) in updates {
        game.pending_deletion_batch_rest[i].covered_by_parked = covers;
    }
}

#[cfg(test)]
mod empty_window_identity_tests {
    use super::*;
    use crate::{
        card_source::CardHandle,
        debug_runner::{make_test_card, DebugRunner},
        effect::{CardEffect, Effect},
    };
    struct Protect;
    impl CardEffect for Protect {
        fn effects(&self, c: CardHandle) -> Vec<Effect> {
            vec![Effect::when_would_leave_battle_area(c)
                .optional()
                .replacement_process(|ctx| ctx.cancel())
                .build()]
        }
    }
    fn fixture(protect: bool) -> (DebugRunner, PermanentHandle) {
        let mut r = DebugRunner::builder()
            .add_card(make_test_card("SUBJECT", "Subject"))
            .start();
        if protect {
            r.register_effect("SUBJECT", std::sync::Arc::new(Protect));
        }
        r.skip_mulligan();
        let h = r.place_on_field(0, "SUBJECT", None);
        (r, h)
    }
    #[test]
    fn empty_replacement_window_does_not_allocate_an_unused_identity() {
        let (mut r, h) = fixture(false);
        let next = r.game.next_battle_identity;
        assert!(r.game.players[0].battle_area[0].battle_identity.is_none());
        let result = start_replacement_window(
            &mut r.game,
            EffectTiming::WhenWouldLeaveBattleArea,
            ReplacementSubject::Permanent(h),
            ReplacementCause::OwnEffect,
            Some(Zone::Deck),
        );
        assert_eq!(result, ReplacementOutcome::None);
        assert_eq!(r.game.next_battle_identity, next);
        assert!(r.game.players[0].battle_area[0].battle_identity.is_none());
        assert!(r.game.pending_selection.is_none());
    }
    #[test]
    fn genuine_optional_window_still_pins_identity_and_clone_declines_equally() {
        let (mut r, h) = fixture(true);
        let next = r.game.next_battle_identity;
        start_replacement_window(
            &mut r.game,
            EffectTiming::WhenWouldLeaveBattleArea,
            ReplacementSubject::Permanent(h),
            ReplacementCause::OwnEffect,
            Some(Zone::Deck),
        );
        assert!(r.game.pending_selection.is_some());
        assert_eq!(r.game.players[0].battle_area[0].battle_identity, Some(next));
        assert_eq!(r.game.next_battle_identity, next + 1);
        let mut branch = r.game.clone();
        for g in [&mut r.game, &mut branch] {
            let p = g.pending_selection.as_ref().unwrap().selecting_player;
            assert!(
                crate::action::build_action_mask(g, p)[crate::action::space::PASS as usize] > 0.0
            );
            g.resolve_selection(p, crate::action::space::PASS).unwrap();
        }
        assert_eq!(r.game.verification_digest(), branch.verification_digest());
    }
}

#[cfg(test)]
mod initially_ineligible_window_identity_tests {
    use super::*;
    use crate::{
        card_source::CardHandle,
        debug_runner::{make_test_card, DebugRunner},
        effect::{CardEffect, Effect},
    };
    struct Gated;
    impl CardEffect for Gated {
        fn effects(&self, c: CardHandle) -> Vec<Effect> {
            vec![Effect::when_would_leave_battle_area(c)
                .condition(|_| false)
                .optional()
                .replacement_process(|ctx| ctx.cancel())
                .build()]
        }
    }
    #[test]
    fn unrelated_ineligible_candidate_does_not_allocate_subject_or_source_identity() {
        let mut r = DebugRunner::builder()
            .add_card(make_test_card("SUBJECT", "Subject"))
            .add_card(make_test_card("GATED", "Gated"))
            .start();
        r.register_effect("GATED", std::sync::Arc::new(Gated));
        r.skip_mulligan();
        let subject = r.place_on_field(0, "SUBJECT", None);
        r.place_on_field(0, "GATED", None);
        let next = r.game.next_battle_identity;
        assert!(
            !collect_candidates_inner(
                &r.game,
                EffectTiming::WhenWouldLeaveBattleArea,
                ReplacementSubject::Permanent(subject),
                ReplacementCause::OwnEffect,
                false
            )
            .is_empty(),
            "control: unfiltered candidate genuinely exists"
        );
        assert!(
            collect_candidates(
                &r.game,
                EffectTiming::WhenWouldLeaveBattleArea,
                ReplacementSubject::Permanent(subject),
                ReplacementCause::OwnEffect
            )
            .is_empty(),
            "control: candidate condition is false"
        );
        let result = start_replacement_window(
            &mut r.game,
            EffectTiming::WhenWouldLeaveBattleArea,
            ReplacementSubject::Permanent(subject),
            ReplacementCause::OwnEffect,
            Some(Zone::Deck),
        );
        assert_eq!(result, ReplacementOutcome::None);
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.game.next_battle_identity, next);
        assert!(r.game.players[0]
            .battle_area
            .iter()
            .all(|p| p.battle_identity.is_none()));
    }
}


#[cfg(test)]
mod source_batch_window_digest {
    use super::*;
    fn runner() -> crate::debug_runner::DebugRunner {
        use crate::{debug_runner::{DebugRunner,make_test_card},effect::{CardEffect,Effect}};
        struct Gate;
        impl CardEffect for Gate {
            fn effects(&self,c:CardHandle)->Vec<Effect>{vec![Effect::when_permanent_would_play(c)
                .name("Ordinary optional would-play gate").optional()
                .replacement_process(|ctx|ctx.cancel()).build()]}
        }
        let mut r=DebugRunner::builder().add_card(make_test_card("WDG-PLAY","Play"))
            .add_card(make_test_card("WDG-GATE","Gate"))
            .hand(0,&["WDG-PLAY"]).deck(0,&["WDG-PLAY";10]).deck(1,&["WDG-PLAY";10]).memory(10).start();
        r.skip_mulligan();r.game.enter_main_phase();
        r.register_effect("WDG-GATE",std::sync::Arc::new(Gate));r.place_on_field(0,"WDG-GATE",None);
        let a=crate::action::space::PLAY_HAND_START;
        assert_eq!(crate::action::build_action_mask(&r.game,0)[a as usize],1.0);
        r.game.decode_action(a,0);
        assert!(r.game.pending_selection.is_some());r
    }
    fn window(g:&mut crate::game::Game)->&mut ReplacementWindow {
        g.pending_selection_resume.as_mut().unwrap().frames.iter_mut().find_map(|f|{
            if let crate::resume::ResumeFrame::OptionalReplacement(s)=f{s.window.as_mut()}else{None}
        }).expect("owned ordinary would-play window")
    }
    fn historical(w:&ReplacementWindow)->String {
        // Exact pre-extension derived Debug field sequence; no golden digest replacement.
        format!("ReplacementWindow {{ timing: {:?}, subject: {:?}, stable_subject: {:?}, cause: {:?}, destination: {:?}, cause_player: {:?}, initiating_source_card: {:?}, effect_digivolve_card: {:?}, event_cause: {:?}, trigger_context: {:?}, deletion_receipt: {:?}, turn_player: {:?}, remaining: {:?}, cancelled: {:?} }}",
            w.timing,w.subject,w.stable_subject,w.cause,w.destination,w.cause_player,w.initiating_source_card,w.effect_digivolve_card,w.event_cause,w.trigger_context,w.deletion_receipt,w.turn_player,w.remaining,w.cancelled)
    }
    #[test]
    fn ordinary_populated_window_keeps_historical_debug_payload() {
        let mut r=runner();let w=window(&mut r.game);
        assert!(w.play_declaration.is_some(),"ordinary hand play is captured too");
        assert_eq!(format!("{w:?}"),historical(w));
    }
    #[test]
    fn redundant_owned_declaration_does_not_change_ordinary_digest() {
        let mut r=runner();let before=r.game.verification_digest();
        window(&mut r.game).play_declaration=None;
        assert_eq!(r.game.verification_digest(),before,"global pending declaration already represents ordinary state");
    }
    #[test]
    fn distinct_owned_declaration_still_changes_digest_and_clones() {
        let r=runner();let mut other=r.game.clone();
        window(&mut other).play_declaration.as_mut().unwrap().effective_cost+=1;
        assert_ne!(r.game.verification_digest(),other.verification_digest());
        assert_eq!(other.verification_digest(),other.clone().verification_digest());
    }
}
