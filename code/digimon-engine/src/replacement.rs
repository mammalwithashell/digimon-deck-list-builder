//! Replacement-effect framework — "Would*" timings + dispatcher.
//!
//! See docs/superpowers/specs/2026-04-21-would-replacement-timings-design.md.
//!
//! Task 1 landed the data types. Task 2 (this module's body) adds the
//! `try_replace` dispatcher, candidate collection/layering, and the
//! `PendingSelection::Replacement` installer for optional replacements.

use crate::action::space::{BREEDING_TARGET, REPLACEMENT_ACCEPT};
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
    Overclock,
}

/// What's about to happen — a permanent leaving the field, a card being
/// trashed from hand, a player about to draw, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReplacementSubject {
    Permanent(PermanentHandle),
    Card(CardHandle, Zone),
    Player(PlayerId),
}

impl ReplacementSubject {
    pub fn permanent(self) -> Option<PermanentHandle> {
        match self {
            ReplacementSubject::Permanent(handle) => Some(handle),
            _ => None,
        }
    }

    pub fn controller(self, game: &crate::game::Game) -> Option<PlayerId> {
        match self {
            ReplacementSubject::Permanent(handle) => Some(handle.player),
            ReplacementSubject::Card(handle, _zone) => game.owner_of_card(handle),
            ReplacementSubject::Player(player) => Some(player),
        }
    }
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
///
/// **`EffectReadContext` caveat:** passives have no installer `CardHandle`
/// (modifiers live in the registry, not on a specific effect), so the
/// context passed to this closure uses a sentinel `CardHandle(0)` for
/// `source_card`. **Do not read `ctx.source_card` from this closure** —
/// it aliases the first card allocated in the game, not the modifier's
/// installer. The meaningful inputs are the `&ReplacementSubject`
/// parameter (the event's target) and `ctx.source_permanent`, which for
/// permanent-scoped passives points at the *target* permanent, and for
/// player-scoped passives points at the installer permanent (if any).
pub type ReplacementConditionFn =
    Box<dyn Fn(&EffectReadContext, &ReplacementSubject) -> bool + Send + Sync + 'static>;

/// Closure type for replacement effect processes. Receives a
/// ReplacementContext so the process can mutate state AND set the outcome.
pub type ReplacementProcessFn = Box<dyn Fn(&mut ReplacementContext<'_>) + Send + Sync + 'static>;

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

/// Captures the replacement-context state needed to resume a process closure
/// after a nested player selection. Set by the dispatcher's post-process hook
/// in `run_candidate_inner` when the process closure parks a `PendingSelection`;
/// drained by `try_drain_parked_replacement_with_guard` in
/// `effect_queue::resolve_generic_selection` after the user's callback runs.
///
/// **Single-outstanding invariant:** at most one parked replacement at a time.
/// The post-process hook `debug_assert!`s on entry. If a real card surfaces a
/// nested-park (callback's body itself fires another deletion that parks),
/// escalate to a follow-up plan that converts the slot to a `Vec`-stack.
///
/// **Coexistence with `Game.dsl_outer_tail`** (Phase 2d): independent slots
/// for independent concerns. Both can be `Some(_)` simultaneously; cross-
/// references are at each field's doc comment.
///
/// Phase C §4.1.
#[doc(hidden)]
#[derive(Debug)]
pub struct ParkedReplacement {
    pub subject: ReplacementSubject,
    pub cause: ReplacementCause,
    pub original_destination: Option<Zone>,
    pub source_card: CardHandle,
    pub source_permanent: Option<PermanentHandle>,
    pub controller: PlayerId,
    /// Outcome the in-flight callback writes via `EffectContext::cancel_leave()`
    /// etc. Read by the dispatcher post-callback hook after the user closure
    /// returns. Defaults to `ReplacementOutcome::None` — original event proceeds
    /// when no outcome-setter is called.
    pub outcome: ReplacementOutcome,
}

// ─── Dispatcher ────────────────────────────────────────────────────

/// Controller of the subject for layering and optional-prompt ownership.
pub(crate) fn subject_controller_id(
    game: &crate::game::Game,
    subject: ReplacementSubject,
) -> Option<PlayerId> {
    subject.controller(game)
}

/// What kind of candidate this is, for dispatch at run time.
///
/// - `EffectClosure { card_id, effect_slot }` — a card effect with a
///   `replacement_process` closure. The dispatcher re-looks-up the effect
///   by `(card_id, effect_slot)` and runs its process closure.
/// - `PassiveCancel` — a passive restriction modifier (e.g.
///   `CannotBeReturnedToDeck`). Passives always cancel mandatorily; the
///   dispatcher synthesizes a `rctx.cancel()` directly.
#[derive(Debug, Clone)]
enum CandidateKind {
    EffectClosure { card_id: String, effect_slot: u8 },
    PassiveCancel,
}

/// One replacement candidate — effect-face or passive-modifier. Stored as
/// `(card_id, effect_slot)` indices for the `EffectClosure` variant so we
/// can re-look-up the `Effect` in `run_candidate` without holding a borrow
/// across `&mut Game` calls.
///
/// Mirrors the `QueuedEffect` pattern in `selection.rs`.
#[derive(Debug, Clone)]
struct Candidate {
    source_card: CardHandle,
    source_permanent: Option<PermanentHandle>,
    source_controller: PlayerId,
    is_mandatory: bool,
    effect_name: String,
    kind: CandidateKind,
}

/// The `Game::try_replace` free-function body. Called via the thin wrapper
/// on `Game` so the `impl Game` block stays readable.
///
/// Implements spec §7.5 — two recursion guards:
///
/// 1. **Depth guard** — `replacement_depth < MAX_REPLACEMENT_DEPTH` or the
///    dispatcher short-circuits. Belt-and-suspenders against pathological
///    mutual-cancellation loops.
/// 2. **Once-per-event guard** (§7.5) — a `(timing, subject)` pair that has
///    already fired within this call chain is skipped. Prevents the
///    `WhenWouldLeaveBattleArea` super-timing from re-firing when its
///    redirect routes through another leave-the-field helper that would
///    dispatch the same timing again (e.g. `delete → Redirected(Deck) →
///    return_to_deck → would-re-fire-leave-battle-area`).
///
/// The fired-set is cleared at the outermost entry (`replacement_depth == 0`)
/// so back-to-back unrelated replacement events each start with a clean slate.
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

    // §7.5: at the outermost entry, clear the fired-set so a fresh call chain
    // starts clean. EXCEPT when `in_replacement_commit` is set — that marks a
    // callback-commit continuation (the original top-level fire-site unwound
    // when the optional selection installed; the callback is re-entering
    // `try_replace` via a zone-mover to finish committing the outcome) and
    // the fired-set must carry through so the super-timing + route-specific
    // timing already fired in the original chain stay blocked. Nested calls
    // (depth > 0) inherit the outer chain's set unconditionally.
    if game.replacement_depth == 0 && !game.in_replacement_commit {
        game.replacement_fired.clear();
    }

    // §7.5 once-per-event guard — if this (timing, subject) already fired in
    // the current call chain, skip re-dispatch. Critical for redirect routes
    // that funnel through the super-timing `WhenWouldLeaveBattleArea`.
    //
    // During a callback-commit continuation (`in_replacement_commit`), we
    // treat the guard more strictly: ANY prior fire for this subject blocks
    // new replacements, regardless of timing. Rationale: once the player has
    // accepted/declined a replacement for this event, the redirect route must
    // not re-prompt for a *different* Would* timing on the same subject (the
    // canonical case is Decode's deck→hand redirect, where the `return_to_hand`
    // commit must NOT install a second PendingSelection for the hand-timing
    // Decode replacement — that would double-prompt for what is logically a
    // single event).
    let key = (timing, subject);
    let blocked = if game.in_replacement_commit {
        game.replacement_fired.iter().any(|(_t, s)| *s == subject)
    } else {
        game.replacement_fired.contains(&key)
    };
    if blocked {
        return ReplacementOutcome::None;
    }
    game.replacement_fired.insert(key);

    game.replacement_depth = game.replacement_depth.saturating_add(1);

    let outcome = try_replace_inner(game, timing, subject, cause, original_destination);

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
    let Some(subject_controller) = subject_controller_id(game, subject) else {
        return ReplacementOutcome::None;
    };
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
    let ordered: Vec<Candidate> = own_reps.into_iter().chain(opp_reps.into_iter()).collect();

    // Walk candidates. Mandatory candidates compose (the last non-None
    // outcome wins for v1 — documented). Optional candidates install a
    // PendingSelection and break the loop; when the selection resolves,
    // the callback writes `game.replacement_pending_outcome` and the caller
    // is responsible for re-entering if more candidates remain.
    //
    // A mandatory candidate may also park a nested selection (e.g.
    // `Keyword::Fragment(N)`: pick N sources to trash). When that happens,
    // `run_candidate_inner` populates `Game.parked_replacement` and yields
    // — we break out of the candidate walk so the parked-replacement drain
    // hook (`try_drain_parked_replacement_with_guard` in `effect_queue.rs`)
    // commits the outcome after the user resolves the nested chain. Any
    // remaining candidates are dropped, mirroring the optional-install break
    // below; v1 supports at most one parked replacement per call chain.
    let mut acc_outcome = ReplacementOutcome::None;
    for cand in ordered {
        if cand.is_mandatory {
            let o = run_mandatory_candidate(game, &cand, subject, cause, original_destination);
            if o != ReplacementOutcome::None {
                acc_outcome = o;
            }
            if game.pending_selection.is_some() {
                // Mandatory candidate parked a nested selection. Yield to
                // the user; the drain hook commits when the chain unwinds.
                return acc_outcome;
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
/// 3. Passive modifier candidates via `passive_modifier_candidates` — scans
///    both permanent- and player-scoped `CannotBe*` / `CannotBeDestroyed*`
///    modifiers, honoring `cause_filter` and `replacement_condition`. Emits
///    `PassiveCancel` candidates that synthesize `ReplacementOutcome::Cancelled`
///    at dispatch. See §10 of the spec.
fn collect_candidates(
    game: &crate::game::Game,
    timing: EffectTiming,
    subject: ReplacementSubject,
    cause: ReplacementCause,
) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();
    let replacement_source_controller = game.effect_source_player;
    let replacement_subject_controller = subject.controller(game);

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
            if effect.max_per_turn > 0 {
                let Some(activation_count) =
                    source_permanent_activation_count(game, h, source_card, slot as u8)
                else {
                    continue;
                };
                if activation_count >= effect.max_per_turn {
                    continue;
                }
            }
            if let Some(cond) = &effect.condition {
                let ctx = EffectReadContext::new(game, source_card, Some(h), h.player)
                    .with_replacement_context(
                        cause,
                        replacement_source_controller,
                        replacement_subject_controller,
                    );
                if !cond(&ctx) {
                    continue;
                }
            }
            // Phase F Task 1: cause-aware candidate filter for WhenWouldBe*
            // timings. Runs AFTER `condition`. Used by `<Scapegoat>` to drop
            // the candidate before the outer accept dialog parks on
            // `ReplacementCause::OwnEffect` (RULES_CONTEXT 16-31) and on the
            // no-substitute case (DCGO `HasMatchConditionPermanent`).
            if let Some(rcond) = &effect.replacement_condition {
                let ctx = EffectReadContext::new(game, source_card, Some(h), h.player)
                    .with_replacement_context(
                        cause,
                        replacement_source_controller,
                        replacement_subject_controller,
                    );
                if !rcond(&ctx, &subject) {
                    continue;
                }
            }
            out.push(Candidate {
                source_card,
                source_permanent: Some(h),
                source_controller: h.player,
                is_mandatory: !effect.optional,
                effect_name: effect.name.clone(),
                kind: CandidateKind::EffectClosure {
                    card_id: card_id.clone(),
                    effect_slot: slot as u8,
                },
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

    // (3) Passive modifier candidates — scans the modifier registry for
    // permanent- and player-scoped `CannotBe*` entries that map to this
    // timing via `passive_modifier_to_would`. Each match emits a
    // `PassiveCancel` candidate that synthesizes `Cancelled` at dispatch.
    out.extend(passive_modifier_candidates(game, timing, subject, cause));

    out
}

/// Map a passive restriction `ModifierType` to the `EffectTiming` at which
/// it should fire as a cancel-replacement. Returns `None` for modifier
/// variants that are not passive replacements (e.g. DP changes, granted
/// keywords, etc.).
///
/// This is the single source of truth for the Task 5 migration of Phase 6
/// (`CannotBeReturnedToDeck` / `…Hand` / `CannotBeTrashedByEffect` /
/// `CannotBeDeDigivolved`) + Phase 0 (`CannotBeDestroyed` /
/// `CannotBeDestroyedByBattle` / `CannotBeDestroyedByEffect`) modifiers
/// onto the would-replacement framework.
pub(crate) fn passive_modifier_to_would(
    modifier: crate::enums::ModifierType,
) -> Option<EffectTiming> {
    use crate::enums::ModifierType;
    match modifier {
        ModifierType::CannotBeReturnedToDeck => Some(EffectTiming::WhenWouldBeReturnedToDeck),
        ModifierType::CannotBeReturnedToHand => Some(EffectTiming::WhenWouldBeReturnedToHand),
        ModifierType::CannotBeTrashedByEffect => Some(EffectTiming::WhenWouldBeTrashed),
        ModifierType::CannotBeDeDigivolved => Some(EffectTiming::WhenWouldBeDeDigivolved),
        // Phase 0 destruction-protection: all fire at WhenWouldBeDeleted.
        // The cause_filter distinguishes them:
        //   - CannotBeDestroyed         → None (cause-agnostic)
        //   - CannotBeDestroyedByBattle → Some(Battle)
        //   - CannotBeDestroyedByEffect → None (both own + opp effects)
        ModifierType::CannotBeDestroyed => Some(EffectTiming::WhenWouldBeDeleted),
        ModifierType::CannotBeDestroyedByBattle => Some(EffectTiming::WhenWouldBeDeleted),
        ModifierType::CannotBeDestroyedByEffect => Some(EffectTiming::WhenWouldBeDeleted),
        // Phase 9 combat restrictions — map Phase 6 attack-restriction
        // modifiers onto Phase 7/9 replacement timings so the attack
        // declaration fire site can dispatch them without a parallel scan.
        // `CannotAttack` on a permanent cancels when that permanent is the
        // ATTACKER subject of `WhenWouldAttack`; `CannotAttackTarget` cancels
        // when the permanent is the TARGET subject of `WhenWouldBeAttackTarget`.
        ModifierType::CannotAttack => Some(EffectTiming::WhenWouldAttack),
        ModifierType::CannotAttackTarget => Some(EffectTiming::WhenWouldBeAttackTarget),
        _ => None,
    }
}

/// Passive-modifier replacement collector (Phase 7 Task 5).
///
/// Scans `ModifierRegistry` for entries whose `ModifierType` maps to the
/// current `timing` via `passive_modifier_to_would` and whose `cause_filter`
/// and `replacement_condition` pass. Each match emits a mandatory
/// `PassiveCancel` candidate.
fn passive_modifier_candidates(
    game: &crate::game::Game,
    timing: EffectTiming,
    subject: ReplacementSubject,
    cause: ReplacementCause,
) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();

    // For `Card(_, Zone)` subjects the v1 model does not scope passive
    // modifiers (no per-card-in-hand modifier store). Skip.
    let target_player: PlayerId = match subject {
        ReplacementSubject::Permanent(h) => h.player,
        ReplacementSubject::Player(p) => p,
        ReplacementSubject::Card(_, _) => return out,
    };

    // (a) Permanent-scoped modifiers — only for Permanent subjects.
    if let ReplacementSubject::Permanent(handle) = subject {
        for entry in game.modifiers.permanent_modifiers_iter(handle) {
            let Some(entry_timing) = passive_modifier_to_would(entry.modifier) else {
                continue;
            };
            if entry_timing != timing {
                continue;
            }
            if let Some(cf) = entry.cause_filter {
                if cf != cause {
                    continue;
                }
            }
            if let Some(cond) = &entry.replacement_condition {
                let read_ctx =
                    EffectReadContext::new(game, CardHandle(0), Some(handle), handle.player)
                        .with_replacement_context(
                            cause,
                            game.effect_source_player,
                            subject.controller(game),
                        );
                if !cond(&read_ctx, &subject) {
                    continue;
                }
            }
            out.push(Candidate {
                source_card: CardHandle(0),
                source_permanent: Some(handle),
                source_controller: entry.source_player,
                is_mandatory: true,
                effect_name: format!("Passive {:?}", entry.modifier),
                kind: CandidateKind::PassiveCancel,
            });
        }
    }

    // (b) Player-scoped modifiers — applicable to all subject kinds where
    // `target_player` is resolved. A player-scoped "cannot be returned to
    // hand" modifier on P1 means P1's permanents cannot be returned.
    for entry in game.modifiers.player_modifiers_iter(target_player) {
        let Some(entry_timing) = passive_modifier_to_would(entry.modifier) else {
            continue;
        };
        if entry_timing != timing {
            continue;
        }
        if let Some(cf) = entry.cause_filter {
            if cf != cause {
                continue;
            }
        }
        if let Some(cond) = &entry.replacement_condition {
            // Use the subject's permanent as source_permanent when available.
            let source_perm = match subject {
                ReplacementSubject::Permanent(h) => Some(h),
                _ => entry.source_permanent,
            };
            let read_ctx = EffectReadContext::new(game, CardHandle(0), source_perm, target_player)
                .with_replacement_context(
                    cause,
                    game.effect_source_player,
                    subject.controller(game),
                );
            if !cond(&read_ctx, &subject) {
                continue;
            }
        }
        out.push(Candidate {
            source_card: CardHandle(0),
            source_permanent: entry.source_permanent,
            source_controller: entry.source_player,
            is_mandatory: true,
            effect_name: format!("Passive {:?} (player-scoped)", entry.modifier),
            kind: CandidateKind::PassiveCancel,
        });
    }

    out
}

/// Run a mandatory candidate by dispatching on its kind.
///
/// - `EffectClosure` — re-looks-up the effect via `(card_id, effect_slot)`,
///   builds a `ReplacementContext`, and invokes the `replacement_process`
///   closure.
/// - `PassiveCancel` — synthesizes a `ctx.cancel()` directly (passive
///   restriction modifiers always cancel).
fn run_mandatory_candidate(
    game: &mut crate::game::Game,
    cand: &Candidate,
    subject: ReplacementSubject,
    cause: ReplacementCause,
    original_destination: Option<Zone>,
) -> ReplacementOutcome {
    match &cand.kind {
        CandidateKind::EffectClosure {
            card_id,
            effect_slot,
        } => run_candidate_inner(
            game,
            card_id,
            cand.source_card,
            cand.source_permanent,
            cand.source_controller,
            *effect_slot,
            subject,
            cause,
            original_destination,
        ),
        CandidateKind::PassiveCancel => {
            // Passive restriction modifiers always cancel. No effect closure
            // to run; just return Cancelled.
            let _ = (game, subject, cause, original_destination);
            ReplacementOutcome::Cancelled
        }
    }
}

/// Shared run-a-replacement-process body. Used by `run_mandatory_candidate`
/// and by the optional-accept callback.
///
/// If the process closure installed a `pending_selection` (via
/// `ctx.select_*`), we snapshot a `ParkedReplacement` and return
/// `ReplacementOutcome::None` so the caller can yield without committing —
/// the parked outcome is later drained by the post-callback hook. Both the
/// mandatory candidate-walk in `try_replace_inner` and the optional-accept
/// callback in `make_accept_callback` rely on this: the candidate-walk
/// breaks out of its loop on `pending_selection.is_some()`, and the accept
/// callback skips its eager-commit branch on `parked_replacement.is_some()`.
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

    if effect.max_per_turn > 0 {
        let Some(source_permanent) = source_permanent else {
            return ReplacementOutcome::None;
        };
        let Some(activation_count) =
            source_permanent_activation_count(game, source_permanent, source_card, effect_slot)
        else {
            return ReplacementOutcome::None;
        };
        if activation_count >= effect.max_per_turn {
            return ReplacementOutcome::None;
        }
        record_source_permanent_activation(game, source_permanent, source_card, effect_slot);
    }

    // Build the EffectContext then the ReplacementContext in an inner scope
    // so their borrows on `game` end before the post-process hook reads
    // `game.pending_selection` and writes `game.parked_replacement`.
    // ReplacementContext holds `&mut EffectContext` which holds `&mut Game`,
    // so both must drop before we can re-borrow `game` mutably below.
    let outcome = {
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
    };

    // Phase C §4.3 — POST-PROCESS HOOK: detect nested-select park.
    // If the process closure installed a PendingSelection (via ctx.select_*),
    // snapshot the replacement context state into Game.parked_replacement
    // so the user's callback can later set the outcome via
    // EffectContext::cancel_leave / etc., and the post-callback hook in
    // resolve_generic_selection can drain the slot via commit_deferred_outcome.
    //
    // Single-outstanding invariant: at most one parked replacement at a time.
    //
    // Both mandatory and optional dispatch arm this hook. Mandatory parking
    // (post-Phase-C-substrate-mandatory) lets keywords like `Fragment(N)`
    // install a nested source-pick selection directly without an outer
    // accept dialog, faithfully matching DCGO's `canNoSelect: () => false`.
    // The candidate-walk in `try_replace_inner` breaks on `pending_selection
    // .is_some()` after a mandatory candidate to mirror the optional yield.
    if game.pending_selection.is_some() {
        debug_assert!(
            game.parked_replacement.is_none(),
            "nested replacement park; outer outcome would be lost. Phase C \
             scope assumes a callback that itself fires a deletion will not \
             also install a select_* selection. If a real card requires \
             nested-park, extend ParkedReplacement into a Vec-stack."
        );
        game.parked_replacement = Some(ParkedReplacement {
            subject,
            cause,
            original_destination,
            source_card,
            source_permanent,
            controller,
            // Preserve any synchronous outcome the process set before parking
            // (e.g. `rctx.cancel()` followed by `rctx.effect.select_*`). The
            // user's nested callback can override via `ctx.*_replacement` (last
            // write wins on parked.outcome); if it doesn't, the synchronous
            // outcome takes effect on commit.
            outcome,
        });
        // Caller (e.g. make_accept_callback) checks pending_selection.is_some()
        // and yields without committing — the parked outcome will be drained
        // after the user's select_* callback fires.
        return ReplacementOutcome::None;
    }

    outcome
}

fn source_permanent_activation_count(
    game: &crate::game::Game,
    handle: PermanentHandle,
    source_card: CardHandle,
    effect_slot: u8,
) -> Option<u8> {
    if handle.index == BREEDING_TARGET as u8 {
        return game
            .players
            .get(handle.player as usize)
            .and_then(|p| p.breeding_area.as_ref())
            .map(|perm| perm.activation_count(source_card, effect_slot));
    }
    game.players
        .get(handle.player as usize)
        .and_then(|p| p.battle_area.get(handle.index as usize))
        .map(|perm| perm.activation_count(source_card, effect_slot))
}

fn record_source_permanent_activation(
    game: &mut crate::game::Game,
    handle: PermanentHandle,
    source_card: CardHandle,
    effect_slot: u8,
) {
    if handle.index == BREEDING_TARGET as u8 {
        if let Some(perm) = game
            .players
            .get_mut(handle.player as usize)
            .and_then(|p| p.breeding_area.as_mut())
        {
            perm.record_activation(source_card, effect_slot);
        }
        return;
    }
    if let Some(perm) = game
        .players
        .get_mut(handle.player as usize)
        .and_then(|p| p.battle_area.get_mut(handle.index as usize))
    {
        perm.record_activation(source_card, effect_slot);
    }
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

    // Passive modifiers are always mandatory — install_optional_selection
    // should never be reached for a PassiveCancel candidate.
    let (card_id, effect_slot) = match &cand.kind {
        CandidateKind::EffectClosure {
            card_id,
            effect_slot,
        } => (card_id.clone(), *effect_slot),
        CandidateKind::PassiveCancel => {
            debug_assert!(
                false,
                "install_optional_selection called with PassiveCancel candidate — \
                 passive modifiers must always be mandatory"
            );
            return;
        }
    };
    let source_card = cand.source_card;
    let source_permanent = cand.source_permanent;
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
    let on_decline = make_decline_callback(subject, cause, original_destination);

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
        source_kind: crate::enums::EffectSourceKind::Rule,
        callback,
        on_decline: Some(on_decline),
    });
}

/// Panic-safe helper for the `in_replacement_commit` flag around
/// `commit_deferred_outcome`. Sets the flag, invokes `body`, and restores the
/// prior value — even if `body` panics. Without this, a panic inside a
/// replacement commit would leak `in_replacement_commit = true` and every
/// subsequent top-level `try_replace` would spuriously treat itself as a
/// callback-commit continuation (failing to clear `replacement_fired`,
/// blocking legitimate replacement dispatch).
fn run_commit_with_flag<F>(game: &mut crate::game::Game, body: F)
where
    F: FnOnce(&mut crate::game::Game),
{
    use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

    let prior = game.in_replacement_commit;
    game.in_replacement_commit = true;

    // `Game` isn't `UnwindSafe` in general (it holds mutable state, interior
    // mutability via closures, etc.), but for the purposes of restoring this
    // single boolean on panic, `AssertUnwindSafe` is the documented pattern.
    // Any further observation of `Game` after a caught panic would be unsound;
    // we do not observe it — we restore the flag and resume the unwind.
    let result = catch_unwind(AssertUnwindSafe(|| body(game)));

    game.in_replacement_commit = prior;

    if let Err(payload) = result {
        resume_unwind(payload);
    }
}

/// Phase C §4.4 — POST-CALLBACK DRAIN: take `Game.parked_replacement` (if any)
/// and route its outcome through `commit_deferred_outcome`. Called from
/// `effect_queue::resolve_generic_selection` after the user's selection
/// callback returns. Reuses `run_commit_with_flag` for panic-safe
/// `in_replacement_commit` flag management.
///
/// No-op when `parked_replacement.is_none()` — the vast majority of
/// selection resolutions don't involve a parked replacement.
///
/// Phase C §4.3 (I-2 follow-up): `replacement_pending_outcome` is expected
/// to be `None` here — Task 6's accept-callback skip-when-parked path
/// intentionally avoids writing it. The `debug_assert!` catches future
/// regressions that would leak a stale outcome.
pub(crate) fn try_drain_parked_replacement_with_guard(game: &mut crate::game::Game) {
    let Some(parked) = game.parked_replacement.take() else {
        return;
    };

    debug_assert!(
        game.replacement_pending_outcome.is_none(),
        "parked-replacement drain expects replacement_pending_outcome unset; \
         a writer (likely make_accept_callback or a fire-site that pre-populates \
         the slot) leaked a value between accept-callback and drain. Check \
         callers of `game.replacement_pending_outcome = Some(_)`."
    );

    run_commit_with_flag(game, move |game| {
        commit_deferred_outcome(
            game,
            parked.subject,
            parked.cause,
            parked.original_destination,
            parked.outcome,
        );
    });
}

/// Build the accept-side callback for an optional replacement. When fired
/// via `resolve_selection(player, REPLACEMENT_ACCEPT)`, re-looks-up the
/// effect and runs its `replacement_process`, then commits the resulting
/// outcome at the appropriate fire-site (since the original fire-site
/// call has already unwound by the time the selection resolves).
///
/// Phase 7 Task 6: previously this only wrote to
/// `replacement_pending_outcome`; now it also commits the outcome so that
/// printed optional keywords (Barrier/Evade/Fragment/Decode) behave
/// end-to-end without additional fire-site plumbing.
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

        // Phase C §4.3: if the process parked a selection, leave
        // `replacement_pending_outcome` UNSET — the post-callback drain hook
        // (Task 7) reads `parked_replacement.outcome` instead. Writing
        // `Some(outcome)` here (where `outcome == ReplacementOutcome::None`
        // because the post-process hook returned `None`) would leak a stale
        // value that other consumers (e.g. `OptionResolutionPhase::Disposing`
        // in `effect_queue.rs`, which `take()`s the slot and treats `None`
        // as "decline → original event proceeds") misinterpret.
        // Order matters: parked-check BEFORE the slot write.
        let current_accept_parked = game.parked_replacement.as_ref().is_some_and(|parked| {
            parked.subject == subject
                && parked.cause == cause
                && parked.original_destination == original_destination
                && parked.source_card == source_card
                && parked.source_permanent == source_permanent
                && parked.controller == controller
        });
        if current_accept_parked {
            return;
        }
        let has_unrelated_parked_replacement = game.parked_replacement.is_some();

        game.replacement_pending_outcome = Some(outcome);

        // §7.5: mark this as a callback-commit continuation so the fired-set
        // from the original call chain survives the zone-mover re-entry.
        // `run_commit_with_flag` is panic-safe — if anything in the commit
        // body panics, the flag is restored before the unwind propagates.
        run_commit_with_flag(game, |game| {
            commit_deferred_outcome(game, subject, cause, original_destination, outcome);
        });
        if has_unrelated_parked_replacement {
            game.replacement_pending_outcome = None;
        }
    })
}

/// Build the decline-side callback for an optional replacement. The caller's
/// fire-site already early-returned when the selection installed, so we must
/// now commit the ORIGINAL event (what was about to happen before the
/// replacement was offered) using `original_destination` as a routing hint.
/// Leaves `replacement_pending_outcome` at `None` per prior behavior.
fn make_decline_callback(
    subject: ReplacementSubject,
    cause: ReplacementCause,
    original_destination: Option<Zone>,
) -> crate::selection::DeclineCallback {
    Box::new(move |game: &mut crate::game::Game| {
        // Decline → outcome stays None → commit the original event.
        // §7.5: mark this as a callback-commit continuation so the fired-set
        // from the original call chain survives the zone-mover re-entry.
        // `run_commit_with_flag` is panic-safe — if anything in the commit
        // body panics, the flag is restored before the unwind propagates.
        run_commit_with_flag(game, |game| {
            commit_deferred_outcome(
                game,
                subject,
                cause,
                original_destination,
                ReplacementOutcome::None,
            );
        });
    })
}

/// Commit a `ReplacementOutcome` at the appropriate fire-site now that the
/// optional-selection has resolved and the original fire-site call has
/// unwound. Dispatches on (`subject`, `original_destination`) to pick the
/// right commit path; a no-op outcome for the None case routes through the
/// original event (e.g. actual deletion) so the decline path behaves as if
/// no replacement was in scope at all.
///
/// This bridges the "optional replacement vs synchronous fire-site" gap
/// introduced in Task 3/4 without modifying those fire-sites directly.
fn commit_deferred_outcome(
    game: &mut crate::game::Game,
    subject: ReplacementSubject,
    cause: ReplacementCause,
    original_destination: Option<Zone>,
    outcome: ReplacementOutcome,
) {
    // TODO(phase-7-followup): parallel commit logic. The outcome-commit arms
    // here MUST stay in sync with the synchronous commit arms in each fire-site
    // (combat.rs::delete_permanent_with_cause,
    // game_actions.rs::{return_to_hand, return_to_deck, place_on_security},
    // effect_context::{draw, de_digivolve, trash_*}). If a fire-site adds a
    // new outcome variant or new destination Zone, update here too.
    // Only Permanent subjects with a known original_destination have a
    // known fire-site to re-enter. Other subject kinds (Card-in-zone,
    // Player) currently cover trash-by-effect / draw — those fire-sites
    // apply the outcome synchronously and don't use optional replacements
    // in v1.
    if original_destination.is_none()
        && cause == ReplacementCause::Battle
        && game.pending_attack.is_some()
    {
        match outcome {
            ReplacementOutcome::None | ReplacementOutcome::CustomHandled => {}
            ReplacementOutcome::Cancelled => {
                if let Some(pa) = game.pending_attack.as_mut() {
                    pa.cancelled = true;
                }
            }
            ReplacementOutcome::Substituted(new_subject) => {
                let is_attacker_subject = game
                    .pending_attack
                    .as_ref()
                    .map(|pa| subject == ReplacementSubject::Permanent(pa.attacker))
                    .unwrap_or(false);
                if is_attacker_subject {
                    debug_assert!(
                        false,
                        "WhenWouldAttack Substituted outcome not supported in v1; \
                         use WhenWouldBeAttackTarget for attack redirects"
                    );
                } else {
                    let new_target = match new_subject {
                        ReplacementSubject::Permanent(h) => crate::AttackTarget::Digimon(h),
                        ReplacementSubject::Player(pid) => crate::AttackTarget::Player(pid),
                        ReplacementSubject::Card(_, _) => {
                            debug_assert!(
                                false,
                                "Attack target substitution does not accept Card subjects"
                            );
                            game.replacement_pending_outcome = None;
                            return;
                        }
                    };
                    game.apply_attack_target_substitution(new_target);
                }
            }
            ReplacementOutcome::Redirected(_) => {
                debug_assert!(
                    false,
                    "Battle replacement Redirected outcome is not meaningful for attack shape"
                );
            }
        }
        game.replacement_pending_outcome = None;
        return;
    }

    if original_destination == Some(Zone::BattleArea) {
        match subject {
            ReplacementSubject::Card(card, Zone::Hand)
                if game
                    .pending_would_play_resume
                    .as_ref()
                    .is_some_and(|resume| resume.card == card) =>
            {
                game.commit_pending_would_play(outcome);
                game.replacement_pending_outcome = None;
                return;
            }
            ReplacementSubject::Card(card, Zone::Reveal)
                if game
                    .pending_would_link_resume
                    .as_ref()
                    .is_some_and(|resume| resume.card == card) =>
            {
                game.commit_pending_would_link(outcome);
                game.replacement_pending_outcome = None;
                return;
            }
            ReplacementSubject::Permanent(perm)
                if game
                    .pending_would_digivolve_resume
                    .as_ref()
                    .is_some_and(|resume| resume.permanent == perm) =>
            {
                game.commit_pending_would_digivolve(outcome);
                game.replacement_pending_outcome = None;
                return;
            }
            _ => {}
        }
    }

    if original_destination == Some(Zone::Trash) {
        if let ReplacementSubject::Card(card, Zone::Security) = subject {
            if game.security_resolution.as_ref().is_some_and(|state| {
                state.revealed_card == card
                    && game
                        .pending_security
                        .as_ref()
                        .is_some_and(|pending| pending.card.handle() == card)
            }) {
                game.commit_pending_security_loss_replacement(outcome);
                game.replacement_pending_outcome = None;
                return;
            }
        }
    }

    let perm = match subject {
        ReplacementSubject::Permanent(h) => h,
        other => {
            // v1 commit_deferred_outcome only handles Permanent subjects.
            // Card/Player optional replacements either (a) have a fire-site
            // that re-enters via a parked state machine (Phase 8 Task 6's
            // Option-dispose flow parks `pending_option` in `Disposing`
            // and re-dispatches from `advance_pending_option`), or (b)
            // have no optional-replacement pathway in v1 (draw, trash-by-
            // effect are mandatory-only today). If neither applies, the
            // outcome is silently dropped — that would be a correctness
            // bug. The debug_assert catches unexpected callers in dev.
            debug_assert!(
                game.pending_option.is_some(),
                "commit_deferred_outcome called with non-Permanent subject ({other:?}); \
                 fire-site must implement its own deferred commit path (or \
                 park a pending_* slot that re-dispatches)"
            );
            let _ = other;
            return;
        }
    };
    let Some(dest) = original_destination else {
        return;
    };

    match (dest, outcome) {
        // Deletion path — original_destination == Trash.
        (Zone::Trash, ReplacementOutcome::None) => {
            // Decline path for a deletion: actually delete now, bypassing
            // the replacement window (already offered and declined).
            commit_permanent_deletion_no_replace(game, perm);
        }
        (Zone::Trash, ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled) => {
            // Already handled by process — nothing to do.
        }
        (Zone::Trash, ReplacementOutcome::Redirected(Zone::Deck)) => {
            game.return_to_deck(perm, crate::enums::StackPosition::Bottom);
        }
        (Zone::Trash, ReplacementOutcome::Redirected(Zone::Hand)) => {
            let _ = game.return_to_hand(perm);
        }
        (Zone::Trash, ReplacementOutcome::Redirected(_)) => {}
        (Zone::Trash, ReplacementOutcome::Substituted(ReplacementSubject::Permanent(other))) => {
            game.delete_permanent_with_cause(other, cause);
        }
        (Zone::Trash, ReplacementOutcome::Substituted(_)) => {}

        // Return-to-hand path.
        (Zone::Hand, ReplacementOutcome::None) => {
            let _ = game.return_to_hand(perm);
        }
        (Zone::Hand, ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled) => {}
        (Zone::Hand, ReplacementOutcome::Redirected(Zone::Deck)) => {
            game.return_to_deck(perm, crate::enums::StackPosition::Bottom);
        }
        (Zone::Hand, ReplacementOutcome::Redirected(Zone::Trash)) => {
            game.delete_permanent_with_cause(perm, cause);
        }
        (Zone::Hand, ReplacementOutcome::Redirected(Zone::Hand)) => {
            // Redirect-to-Hand when already going to Hand: redundant in terms
            // of final destination but we still must commit the move, since
            // the original fire-site unwound on selection-install.
            //
            // Post-§7.5 guard: safe to call `return_to_hand` directly — the
            // once-per-event guard prevents `WhenWouldBeReturnedToHand` from
            // re-firing for the same subject within this call chain.
            let _ = game.return_to_hand(perm);
        }
        (Zone::Hand, ReplacementOutcome::Redirected(_)) => {}
        (Zone::Hand, ReplacementOutcome::Substituted(ReplacementSubject::Permanent(other))) => {
            let _ = game.return_to_hand(other);
        }
        (Zone::Hand, ReplacementOutcome::Substituted(_)) => {}

        // Return-to-deck path.
        (Zone::Deck, ReplacementOutcome::None) => {
            game.return_to_deck(perm, crate::enums::StackPosition::Bottom);
        }
        (Zone::Deck, ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled) => {}
        (Zone::Deck, ReplacementOutcome::Redirected(Zone::Hand)) => {
            let _ = game.return_to_hand(perm);
        }
        (Zone::Deck, ReplacementOutcome::Redirected(Zone::Trash)) => {
            game.delete_permanent_with_cause(perm, cause);
        }
        (Zone::Deck, ReplacementOutcome::Redirected(_)) => {}
        (Zone::Deck, ReplacementOutcome::Substituted(ReplacementSubject::Permanent(other))) => {
            game.return_to_deck(other, crate::enums::StackPosition::Bottom);
        }
        (Zone::Deck, ReplacementOutcome::Substituted(_)) => {}

        // Other destinations (BattleArea, Security, …) — not produced by
        // any current fire-site for Permanent subjects.
        _ => {}
    }
}

/// Commit a permanent's deletion without re-running the replacement window.
/// Mirrors the post-replacement arm of `Game::delete_permanent_with_cause`
/// (OnDeletion → finalize: linked-card cascade, `delete_permanent`, modifier
/// cleanup, post-deletion replays drain, OnAnyDeletion) but bypasses
/// `try_replace` because the replacement window has already been offered
/// and declined.
fn commit_permanent_deletion_no_replace(game: &mut crate::game::Game, handle: PermanentHandle) {
    use crate::enums::EffectTiming;
    use crate::selection::TriggerSource;

    game.enqueue_triggered(EffectTiming::OnDeletion, TriggerSource::Permanent(handle));
    game.drain_effect_queue();

    // Phase D Task 6 followup (2026-04-25): an OnDeletion handler may have
    // installed a player choice (e.g. printed `<Save>` parks an optional
    // Tamer-pick). Park the rest of the deletion sequence to be finalized
    // after the selection resolves; running `Player::delete_permanent`
    // synchronously here would shift later permanents' indices and
    // invalidate the parked selection. Mirrors the guard in
    // `combat.rs::Game::commit_permanent_deletion`.
    //
    // Reachable when: a card hosts BOTH an optional `WhenWouldBeDeleted`
    // replacement (e.g. `<Decoy>`, `<Barrier>`) AND an OnDeletion-parking
    // handler (`<Save>`). The optional replacement was offered, the user
    // declined, and the deferred-decline path lands here.
    //
    // `Game::resume_pending_deletion` (called from
    // `effect_queue::resolve_generic_selection` after the parked selection
    // resolves and the post-callback drain settles) runs
    // `finalize_permanent_deletion` against the saved handle — the same
    // resume hook used by the synchronous `commit_permanent_deletion` path,
    // so `pending_post_deletion_replays` (Fortitude/Partition) and the
    // linked-card cascade all run uniformly.
    if game.pending_selection.is_some() {
        debug_assert!(
            game.pending_deletion_resume.is_none(),
            "nested deferred deletion not supported (single-outstanding invariant)"
        );
        game.pending_deletion_resume = Some(handle);
        return;
    }

    game.finalize_permanent_deletion(handle);
}
