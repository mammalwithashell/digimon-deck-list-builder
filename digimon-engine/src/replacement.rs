//! Replacement-effect framework — "Would*" timings + dispatcher.
//!
//! See docs/superpowers/specs/2026-04-21-would-replacement-timings-design.md.
//!
//! Task 1 landed the data types. Task 2 (this module's body) adds the
//! `try_replace` dispatcher, candidate collection/layering, and the
//! `PendingSelection::Replacement` installer for optional replacements.

use crate::action::space::REPLACEMENT_ACCEPT;
use crate::card_source::CardHandle;
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::{EffectTiming, GamePhase, PlayerId, Zone};
use crate::permanent::PermanentHandle;
use crate::selection::{PendingSelection, SelectionKind};

/// Maximum nesting depth for `try_replace`. If recursion reaches this value,
/// further replacement attempts short-circuit to `ReplacementOutcome::None`
/// to prevent a pathological self-referential chain from hanging the engine.
pub const MAX_REPLACEMENT_DEPTH: u8 = 8;

/// Why a state change is happening — consumed by replacement effects that
/// filter on cause (e.g. "cannot be trashed by your opponent's effects").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReplacementCause {
    Battle,
    OwnEffect,
    OpponentEffect,
    SecurityCheck,
    Cost,
}

/// What's about to happen — a permanent leaving the field, a card being
/// trashed from hand, a player about to draw, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplacementSubject {
    Permanent(PermanentHandle),
    Card(CardHandle, Zone),
    Player(PlayerId),
}

/// The outcome a replacement effect sets. Mutually exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplacementOutcome {
    None,
    Cancelled,
    Redirected(Zone),
    Substituted(ReplacementSubject),
    CustomHandled,
}

/// Closure type for passive modifier replacement conditions. Evaluated at
/// try_replace time to decide whether the modifier applies.
pub type ReplacementConditionFn =
    Box<dyn Fn(&EffectReadContext, &ReplacementSubject) -> bool + Send + Sync + 'static>;

/// Closure type for replacement effect processes. Receives a
/// ReplacementContext so the process can mutate state AND set the outcome.
pub type ReplacementProcessFn =
    Box<dyn Fn(&mut ReplacementContext<'_>) + Send + Sync + 'static>;

/// Passed to Would* effect processes. `effect` is the underlying effect ctx;
/// `cause`, `subject`, `original_destination` are snapshot event data; the
/// process sets `outcome` via helpers to tell the dispatcher what to do.
pub struct ReplacementContext<'g> {
    pub effect: &'g mut EffectContext<'g>,
    pub cause: ReplacementCause,
    pub subject: ReplacementSubject,
    pub original_destination: Option<Zone>,
    pub(crate) outcome: ReplacementOutcome,
}

impl<'g> ReplacementContext<'g> {
    pub fn cancel(&mut self) {
        self.outcome = ReplacementOutcome::Cancelled;
    }
    pub fn redirect_to(&mut self, dest: Zone) {
        self.outcome = ReplacementOutcome::Redirected(dest);
    }
    pub fn substitute(&mut self, subject: ReplacementSubject) {
        self.outcome = ReplacementOutcome::Substituted(subject);
    }
    pub fn handled(&mut self) {
        self.outcome = ReplacementOutcome::CustomHandled;
    }
}

// ─── Dispatcher ────────────────────────────────────────────────────

/// Controller of the subject for layering purposes.
///
/// Stub: `Card(_, Zone)` resolves via the card's owning player derived by
/// scanning zones — but the hot path here is `Permanent` (which carries
/// `handle.player`) and `Player`. For Task 2 the Card path is approximated
/// by returning the first player (no callers yet); Task 3+ fire-sites will
/// extend this when they start using the Card subject.
pub(crate) fn subject_controller_id(
    game: &crate::game::Game,
    subject: ReplacementSubject,
) -> PlayerId {
    match subject {
        ReplacementSubject::Permanent(h) => h.player,
        ReplacementSubject::Player(p) => p,
        ReplacementSubject::Card(handle, _zone) => {
            // Scan zones to find the owning player. Best-effort for Task 2;
            // Task 3+ fire-sites are expected to always pass a Permanent or
            // Player subject. Fall back to player 0 if not found.
            for (pid, player) in game.players.iter().enumerate() {
                let found = player
                    .hand
                    .iter()
                    .chain(player.trash.iter())
                    .chain(player.deck.iter())
                    .chain(player.security.iter())
                    .any(|cs| cs.handle() == handle);
                if found {
                    return pid as PlayerId;
                }
            }
            0
        }
    }
}

/// One replacement candidate — effect-face or passive-modifier. Stored as
/// `(card_id, effect_slot)` indices so we can re-look-up the `Effect` in
/// `run_candidate` without holding a borrow across `&mut Game` calls.
///
/// Mirrors the `QueuedEffect` pattern in `selection.rs`.
#[derive(Debug, Clone)]
struct Candidate {
    source_card: CardHandle,
    source_permanent: Option<PermanentHandle>,
    source_controller: PlayerId,
    is_mandatory: bool,
    effect_name: String,
    card_id: String,
    effect_slot: u8,
}

/// The `Game::try_replace` free-function body. Called via the thin wrapper
/// on `Game` so the `impl Game` block stays readable.
pub(crate) fn try_replace_impl(
    game: &mut crate::game::Game,
    timing: EffectTiming,
    subject: ReplacementSubject,
    cause: ReplacementCause,
    original_destination: Option<Zone>,
) -> ReplacementOutcome {
    // Depth guard.
    if game.replacement_depth >= MAX_REPLACEMENT_DEPTH {
        return ReplacementOutcome::None;
    }
    game.replacement_depth = game.replacement_depth.saturating_add(1);

    let outcome =
        try_replace_inner(game, timing, subject, cause, original_destination);

    game.replacement_depth = game.replacement_depth.saturating_sub(1);
    outcome
}

fn try_replace_inner(
    game: &mut crate::game::Game,
    timing: EffectTiming,
    subject: ReplacementSubject,
    cause: ReplacementCause,
    original_destination: Option<Zone>,
) -> ReplacementOutcome {
    // Collect candidates. We layer by controller: own_reps (subject's
    // controller) first, then opp_reps.
    let subject_controller = subject_controller_id(game, subject);
    let candidates = collect_candidates(game, timing, subject, cause);

    if candidates.is_empty() {
        return ReplacementOutcome::None;
    }

    let (own_reps, opp_reps): (Vec<_>, Vec<_>) = candidates
        .into_iter()
        .partition(|c| c.source_controller == subject_controller);

    // TODO(phase-7-followup): when either side has >1 candidates, install a
    // TriggerOrder-style selection so the controller picks the order. v1
    // runs them in collection order.
    let ordered: Vec<Candidate> =
        own_reps.into_iter().chain(opp_reps.into_iter()).collect();

    // Walk candidates. Mandatory candidates compose (the last non-None
    // outcome wins for v1 — documented). Optional candidates install a
    // PendingSelection and break the loop; when the selection resolves,
    // the callback writes `game.replacement_pending_outcome` and the caller
    // is responsible for re-entering if more candidates remain.
    let mut acc_outcome = ReplacementOutcome::None;
    for cand in ordered {
        if cand.is_mandatory {
            let o = run_mandatory_candidate(
                game,
                &cand,
                subject,
                cause,
                original_destination,
            );
            if o != ReplacementOutcome::None {
                acc_outcome = o;
            }
        } else {
            install_optional_selection(
                game,
                &cand,
                subject_controller,
                subject,
                cause,
                original_destination,
            );
            // Selection is pending — caller resumes via resolve_selection.
            // Outcome will be written to game.replacement_pending_outcome.
            return acc_outcome;
        }
    }

    acc_outcome
}

/// Walk the sources of candidate replacement effects and emit one
/// `Candidate` per match. Order:
/// 1. Subject's own effects at `timing` (only relevant for `Permanent` subject).
/// 2. Other battle-area permanents' effects at `timing`.
/// 3. Passive modifier candidates for the target (Task 2 stub — returns `Vec::new()`).
fn collect_candidates(
    game: &crate::game::Game,
    timing: EffectTiming,
    subject: ReplacementSubject,
    cause: ReplacementCause,
) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();

    // Snapshot subject's permanent (if any) so we can order subject-first.
    let subject_perm: Option<PermanentHandle> = match subject {
        ReplacementSubject::Permanent(h) => Some(h),
        _ => None,
    };

    // Collect from a permanent's effects, filtering by timing and
    // replacement_process presence.
    let push_from_perm = |out: &mut Vec<Candidate>, h: PermanentHandle| {
        let Some(player) = game.players.get(h.player as usize) else {
            return;
        };
        let Some(perm) = player.battle_area.get(h.index as usize) else {
            return;
        };
        let top = perm.top_card();
        let card_id = top.card_id(&game.card_data).to_string();
        let source_card = top.handle();

        let Some(effects) = game.effects_for_card(&card_id, source_card) else {
            return;
        };

        for (slot, effect) in effects.iter().enumerate() {
            if effect.timing != timing {
                continue;
            }
            if effect.replacement_process.is_none() {
                // Would-timed effect with no replacement_process is a
                // structural gap; skip silently for now (card registry
                // authors are expected to always attach one).
                continue;
            }
            if let Some(cond) = &effect.condition {
                let ctx = EffectReadContext::new(game, source_card, Some(h), h.player);
                if !cond(&ctx) {
                    continue;
                }
            }
            let _ = cause; // cause_filter migrates in Task 5 (passive modifiers)
            out.push(Candidate {
                source_card,
                source_permanent: Some(h),
                source_controller: h.player,
                is_mandatory: !effect.optional,
                effect_name: effect.name.clone(),
                card_id: card_id.clone(),
                effect_slot: slot as u8,
            });
        }
    };

    // (1) Subject's own effects first (for Permanent subjects).
    if let Some(h) = subject_perm {
        push_from_perm(&mut out, h);
    }

    // (2) Other battle-area permanents' effects.
    for player_idx in 0..game.players.len() {
        let player = &game.players[player_idx];
        for idx in 0..player.battle_area.len() {
            let h = PermanentHandle {
                player: player_idx as PlayerId,
                index: idx as u8,
            };
            if Some(h) == subject_perm {
                continue;
            }
            push_from_perm(&mut out, h);
        }
    }

    // (3) Passive modifier candidates. TODO(phase-7 Task 5): wire the
    // `passive_modifier_to_would` mapping. v1 stub returns nothing.
    out.extend(passive_modifier_candidates(game, timing, subject, cause));

    out
}

/// Passive-modifier replacement collector. Task 5 lands the full
/// mapping from `ModifierType` (e.g. `CannotBeDeleted`,
/// `CannotBeReturnedToHand`, `Security*` flood-gates) into synthetic
/// `Candidate` entries. v1 stub.
fn passive_modifier_candidates(
    _game: &crate::game::Game,
    _timing: EffectTiming,
    _subject: ReplacementSubject,
    _cause: ReplacementCause,
) -> Vec<Candidate> {
    Vec::new()
}

/// Run a mandatory candidate by re-looking-up the effect via
/// `(card_id, effect_slot)`, building a `ReplacementContext`, and invoking
/// the `replacement_process` closure.
fn run_mandatory_candidate(
    game: &mut crate::game::Game,
    cand: &Candidate,
    subject: ReplacementSubject,
    cause: ReplacementCause,
    original_destination: Option<Zone>,
) -> ReplacementOutcome {
    run_candidate_inner(
        game,
        &cand.card_id,
        cand.source_card,
        cand.source_permanent,
        cand.source_controller,
        cand.effect_slot,
        subject,
        cause,
        original_destination,
    )
}

/// Shared run-a-replacement-process body. Used by `run_mandatory_candidate`
/// and by the optional-accept callback.
fn run_candidate_inner(
    game: &mut crate::game::Game,
    card_id: &str,
    source_card: CardHandle,
    source_permanent: Option<PermanentHandle>,
    controller: PlayerId,
    effect_slot: u8,
    subject: ReplacementSubject,
    cause: ReplacementCause,
    original_destination: Option<Zone>,
) -> ReplacementOutcome {
    let Some(effects) = game.effects_for_card(card_id, source_card) else {
        return ReplacementOutcome::None;
    };
    let Some(effect) = effects.get(effect_slot as usize) else {
        return ReplacementOutcome::None;
    };
    let Some(process) = effect.replacement_process.as_ref() else {
        return ReplacementOutcome::None;
    };

    // Build the EffectContext then the ReplacementContext. The
    // ReplacementContext holds `&mut EffectContext` so we must keep the
    // EffectContext alive for the duration of the process call.
    let mut ctx = EffectContext::new(game, source_card, source_permanent, controller);
    let mut rep_ctx = ReplacementContext {
        effect: &mut ctx,
        cause,
        subject,
        original_destination,
        outcome: ReplacementOutcome::None,
    };
    process(&mut rep_ctx);
    rep_ctx.outcome
}

/// Install a `PendingSelection::Replacement` prompt for an optional
/// candidate. The callback captures the re-lookup indices + the subject
/// metadata and writes the resulting outcome into
/// `game.replacement_pending_outcome`.
fn install_optional_selection(
    game: &mut crate::game::Game,
    cand: &Candidate,
    subject_controller: PlayerId,
    subject: ReplacementSubject,
    cause: ReplacementCause,
    original_destination: Option<Zone>,
) {
    debug_assert!(
        game.pending_selection.is_none(),
        "install_optional_selection must not overwrite a live pending selection; \
         Task 3+ fire-sites should only call try_replace at decision points \
         where no selection is already in flight"
    );
    let previous_phase = game.current_phase;
    game.current_phase = GamePhase::EffectChoice;
    game.replacement_pending_outcome = None;

    let card_id = cand.card_id.clone();
    let source_card = cand.source_card;
    let source_permanent = cand.source_permanent;
    let effect_slot = cand.effect_slot;
    let controller = cand.source_controller;

    let callback = make_accept_callback(
        card_id,
        source_card,
        source_permanent,
        controller,
        effect_slot,
        subject,
        cause,
        original_destination,
    );
    let on_decline = make_decline_callback();

    game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::Replacement,
        selecting_player: subject_controller,
        previous_phase,
        valid_action_ids: vec![REPLACEMENT_ACCEPT],
        is_optional: true,
        prompt: format!("May accept replacement: {}", cand.effect_name),
        effect_choices: None,
        source_card: cand.source_card,
        source_permanent: cand.source_permanent,
        callback,
        on_decline: Some(on_decline),
    });
}

/// Build the accept-side callback for an optional replacement. When fired
/// via `resolve_selection(player, REPLACEMENT_ACCEPT)`, re-looks-up the
/// effect and runs its `replacement_process`, writing the outcome to
/// `game.replacement_pending_outcome`.
fn make_accept_callback(
    card_id: String,
    source_card: CardHandle,
    source_permanent: Option<PermanentHandle>,
    controller: PlayerId,
    effect_slot: u8,
    subject: ReplacementSubject,
    cause: ReplacementCause,
    original_destination: Option<Zone>,
) -> crate::selection::SelectionCallback {
    Box::new(move |game: &mut crate::game::Game, _action_id: u16| {
        let outcome = run_candidate_inner(
            game,
            &card_id,
            source_card,
            source_permanent,
            controller,
            effect_slot,
            subject,
            cause,
            original_destination,
        );
        game.replacement_pending_outcome = Some(outcome);
    })
}

/// Build the decline-side callback for an optional replacement. Leaves
/// `game.replacement_pending_outcome` at `None`.
fn make_decline_callback() -> crate::selection::DeclineCallback {
    Box::new(move |_game: &mut crate::game::Game| {
        // No-op — outcome stays None. (Caller already cleared
        // replacement_pending_outcome in install_optional_selection.)
    })
}
