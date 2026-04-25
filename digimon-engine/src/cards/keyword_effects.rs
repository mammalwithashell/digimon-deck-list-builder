//! Phase 7 Task 6 — keyword-derived auto-install replacement effects.
//!
//! `keyword_to_auto_effect(kw, card)` maps a printed-keyword entry on a
//! card's `CardData::keywords` to a synthesized [`Effect`] that installs the
//! matching `WhenWouldBe*` replacement. `Game::effects_for_card` calls this
//! for every keyword after retrieving registry effects and appends the
//! non-`None` results so that cards with printed Barrier / Evade /
//! Fragment(N) / Decode behave as printed without a hand-authored
//! `CardEffect` script.
//!
//! Most replacement keywords here produce **optional** replacements per
//! printed rules ("you may"). Declining the optional selection leaves the
//! original event (deletion / return-to-deck) to proceed normally. The
//! optional auto-installs (Phase 7 + Phase D so far): Barrier, Evade, Decode,
//! Save, Decoy. Mandatory ones: Fragment(N), Armor Purge, Fortitude.
//!
//! ## Trigger-based keywords
//!
//! Not all keywords ride on a `WhenWouldBe*` replacement window — some are
//! triggers that observe a state change without modifying it. Currently
//! installed: Save (`OnDeletion`, optional), Fortitude (`OnDeletion`,
//! mandatory; uses the post-deletion replay substrate to play self from
//! trash after `delete_permanent` finalizes).
//!
//! ## Active-skill keywords (Phase D Task 10)
//!
//! `MaterialSave(N)` is a `[Main]` active skill — neither a replacement nor
//! a deletion trigger. It auto-installs as an `EffectTiming::MainOnField`
//! effect; the action mask exposes a single `FIELD_EFFECT_SLOT_FOR_MAIN`
//! bit when both halves of the gate hold (carrier has ≥1 source under top
//! AND controller has ≥1 own Tamer). Activation is free (no memory cost,
//! no suspension). The body parks an own-Tamer pick (mandatory) followed by
//! an up-to-N source pick (`is_optional_zero=true`), then tucks the picks
//! at the bottom of the chosen Tamer's stack via
//! `place_card_under_permanent_bottom`.
//!
//! ## Partition (Phase D Task 9)
//!
//! Printed `<Partition>` is a leave-field trigger (not a replacement) that
//! plays cards from the deleted permanent's digivolution sources back to
//! the field, free + unsuspended. The auto-install mounts as an
//! `OnDeletion` trigger with a cause filter (`!Battle && !OwnEffect`) and
//! a 2-pick selection over the carrier's `card_sources`. Picked cards are
//! stashed in `Game.pending_post_deletion_replays` so they replay from
//! trash (via `play_from_trash_free_unsuspended`) AFTER `delete_permanent`
//! finalizes the carrier but BEFORE the `OnAnyDeletion` broadcast.
//!
//! Color grouping (`firstSources`/`secondSources` in DCGO) is per-card and
//! comes from injected `partitionConditions`; the auto-install offers ANY
//! source (no color filter). Per-card-text overrides apply color grouping
//! via hand-rolled `CardEffect` if needed.

use crate::card_source::CardHandle;
use crate::effect::Effect;
use crate::effect_context::CountCappedZone;
use crate::enums::{EffectTiming, Keyword, Zone};
use crate::replacement::ReplacementSubject;

/// Map a printed keyword to zero-or-more synthesized `Effect`s that install
/// the matching `WhenWouldBe*` replacements. Returns an empty `Vec` for
/// keywords that do not carry replacement semantics (or that are explicitly
/// deferred per the module docstring).
///
/// Most replacement keywords produce a single effect, but `Keyword::Decode`
/// needs to cover BOTH the return-to-deck and return-to-hand timings per
/// printed rules ("returned to your opponent's deck/hand"), so this helper
/// returns a `Vec<Effect>` rather than `Option<Effect>`.
pub fn keyword_to_auto_effect(keyword: Keyword, card: CardHandle) -> Vec<Effect> {
    match keyword {
        // Printed Barrier: "When this Digimon would be deleted, you may trash
        // the top card of your deck. If you do, it isn't deleted."
        Keyword::Barrier => vec![Effect::when_would_be_deleted(card)
            .name("<Barrier>")
            .optional()
            .replacement_process(|rctx| {
                // Only replace the subject's own deletion (the keyword is
                // printed on this card). Without the self-scope guard the
                // replacement could re-fire for neighboring deletions.
                let me = rctx.effect.source_permanent;
                if let ReplacementSubject::Permanent(subject) = rctx.subject {
                    if Some(subject) != me {
                        return;
                    }
                    let owner = subject.player;
                    let game = &mut *rctx.effect.game;
                    if let Some(top) = game.players[owner as usize].deck.pop() {
                        game.players[owner as usize].trash.push(top);
                    }
                    // Empty-deck case: replacement still fires (spec §14 Q2).
                    rctx.handled();
                }
            })
            .build()],

        // Printed Evade: "When this Digimon would be deleted, you may place
        // it at the bottom of your deck instead."
        // Redirecting to Zone::Deck is honored by
        // `delete_permanent_with_cause`'s `Redirected(Zone::Deck)` arm,
        // which routes through `return_to_deck(StackPosition::Bottom)`.
        Keyword::Evade => vec![Effect::when_would_be_deleted(card)
            .name("<Evade>")
            .optional()
            .replacement_process(|rctx| {
                let me = rctx.effect.source_permanent;
                if let ReplacementSubject::Permanent(subject) = rctx.subject {
                    if Some(subject) != me {
                        return;
                    }
                    rctx.redirect_to(Zone::Deck);
                }
            })
            .build()],

        // Phase D Task 4 — printed Fragment(N): "When this Digimon would be
        // deleted, you may trash N of its digivolution cards. If you do, it
        // isn't deleted." DCGO `Fragment.cs:23-77`.
        //
        // Gate: `card_sources.len() >= N + 1` (top + at least N sources).
        // When the gate fails, the auto-install body returns early without
        // parking and the original deletion proceeds.
        //
        // Mandatory semantics — matches DCGO `Fragment.cs:38`'s
        // `canNoSelect: () => false`. Once the gate passes, the carrier's
        // controller MUST pick N sources to trash; there is no outer accept
        // dialog. This is enabled by the Phase C substrate extension that
        // lets a MANDATORY replacement-process park a nested selection: the
        // candidate-walk in `replacement::try_replace_inner` yields on
        // `pending_selection.is_some()` after running the candidate, and
        // the post-callback drain hook commits `cancel_leave()` after the
        // user resolves the source-pick chain.
        Keyword::Fragment(n) => vec![Effect::when_would_be_deleted(card)
            .name(&format!("<Fragment ({n})>"))
            // Gate on stack size at candidate-collection time so the
            // mandatory selection is suppressed when there aren't enough
            // sources to pay the trash cost. DCGO `Fragment.cs:23`
            // `CanReplace` checks
            // `DigivolutionCards.Count >= N` (DCGO `DigivolutionCards`
            // excludes the top card), which translates to
            // `card_sources.len() >= N + 1` here.
            //
            // The condition is evaluated inside `collect_candidates` with
            // `source_permanent` set to the carrier — so we read the stack
            // size off `source_permanent()`. Subject-mismatch self-scoping
            // can't be done here (the condition signature has no subject
            // parameter); the closure body handles that.
            .condition(move |ctx| {
                let Some(perm) = ctx.source_permanent() else {
                    return false;
                };
                perm.card_sources.len() >= (n as usize) + 1
            })
            .replacement_process(move |rctx| {
                // Self-scope guard: only fire on the carrier's own deletion.
                // The candidate-walk in `collect_candidates` pushes this
                // effect for every battle-area permanent's deletion whose
                // top-card-effects match this timing, so the body must
                // self-scope to avoid running for a neighbor's deletion.
                let me_perm = rctx.effect.source_permanent;
                let subject = match rctx.subject {
                    ReplacementSubject::Permanent(h) => h,
                    _ => return,
                };
                if Some(subject) != me_perm {
                    return;
                }

                // Re-check the gate at process time. `condition` already
                // gated candidate inclusion, but a stack-mutating earlier
                // replacement in the same chain could have shrunk the stack
                // between collection and process. Belt-and-suspenders.
                let n_usize = n as usize;
                let stack_len = rctx
                    .effect
                    .game
                    .player(subject.player)
                    .battle_area
                    .get(subject.index as usize)
                    .map(|p| p.card_sources.len())
                    .unwrap_or(0);
                if stack_len < n_usize + 1 {
                    return;
                }

                // Park: select exactly N sources from the carrier's stack to
                // trash. `is_optional_zero=false` matches DCGO's
                // `canNoSelect: () => false` (Fragment.cs:38) — the inner
                // pick can't be passed once the gate passes.
                let controller = subject.player;
                rctx.effect.select_count_capped_multi(
                    controller,
                    CountCappedZone::Material(subject),
                    n,
                    "trash N digivolution cards",
                    /*is_optional_zero=*/ false,
                    |_g, _src| true,
                    move |ctx, picks| {
                        // Trash each picked source from the carrier's stack
                        // into the controller's trash via the EffectContext
                        // primitive (stays within the API boundary).
                        for handle in picks {
                            ctx.trash_card_source(subject, handle);
                        }
                        // Cancel the original deletion — carrier survives
                        // with its remaining sources + top.
                        ctx.cancel_leave();
                    },
                );
            })
            .build()],

        // Printed Decode: "When this Digimon would be returned to your
        // opponent's deck/hand, you may return it to your hand instead."
        //
        // Two effects are installed — one per printed timing. The deck route
        // is the non-trivial redirect (deck → hand). The hand route is a
        // redirect-to-hand when the original destination is ALSO hand, which
        // is logically a no-op; it's included for symmetry with printed
        // rules so the replacement offer actually fires on opponent return-
        // to-hand effects too.
        Keyword::Decode => vec![
            Effect::when_would_be_returned_to_deck(card)
                .name("<Decode> (deck)")
                .optional()
                .replacement_process(|rctx| {
                    let me = rctx.effect.source_permanent;
                    if let ReplacementSubject::Permanent(subject) = rctx.subject {
                        if Some(subject) != me {
                            return;
                        }
                        rctx.redirect_to(Zone::Hand);
                    }
                })
                .build(),
            Effect::when_would_be_returned_to_hand(card)
                .name("<Decode> (hand)")
                .optional()
                .replacement_process(|rctx| {
                    let me = rctx.effect.source_permanent;
                    if let ReplacementSubject::Permanent(subject) = rctx.subject {
                        if Some(subject) != me {
                            return;
                        }
                        rctx.redirect_to(Zone::Hand);
                    }
                })
                .build(),
        ],

        // Phase D Task 5 — printed Armor Purge: "When this Digimon would be
        // deleted, trash the top card of this Digimon. If you do, it isn't
        // deleted." DCGO `ArmorPurge.cs:40-78`.
        //
        // Gate: `card_sources.len() >= 2` (top + ≥1 source under it). When the
        // gate fails the auto-install body returns early; original deletion
        // proceeds normally.
        //
        // Synchronous + mandatory. Unlike Fragment(N) (which goes through the
        // Phase C parked-replacement substrate because it carries a nested
        // selection), ArmorPurge has no player choice — it's purely a
        // top-swap. The replacement-process closure calls `rctx.cancel()`
        // directly, an outcome-setter that works in mandatory contexts (the
        // `debug_assert!` in `run_candidate_inner` only trips when a mandatory
        // process installs a `pending_selection`, which we do not). No
        // `.optional()` wrapper required, no parked-replacement plumbing.
        //
        // The event-fire (OnDigivolutionCardTrashed) for the trashed top is
        // handled by `EffectContext::armor_purge_top` itself; see Phase D
        // Task 5 commit log.
        Keyword::ArmorPurge => vec![Effect::when_would_be_deleted(card)
            .name("<Armor Purge>")
            // Gate at candidate-collection time so the dispatcher skips this
            // candidate entirely when the stack is too small. Mirrors
            // Fragment(N)'s condition pattern.
            .condition(|ctx| {
                let Some(perm) = ctx.source_permanent() else {
                    return false;
                };
                perm.card_sources.len() >= 2
            })
            .replacement_process(|rctx| {
                // Self-scope guard: only fire on the carrier's own deletion.
                // `collect_candidates` enumerates this effect for every
                // battle-area permanent's deletion at this timing, so the
                // body must self-scope to avoid running for a neighbor.
                let me_perm = rctx.effect.source_permanent;
                let subject = match rctx.subject {
                    ReplacementSubject::Permanent(h) => h,
                    _ => return,
                };
                if Some(subject) != me_perm {
                    return;
                }

                // Re-check the gate at process time. Belt-and-suspenders: a
                // stack-mutating earlier replacement in the same chain could
                // have shrunk the stack between collection and process.
                let stack_len = rctx
                    .effect
                    .game
                    .player(subject.player)
                    .battle_area
                    .get(subject.index as usize)
                    .map(|p| p.card_sources.len())
                    .unwrap_or(0);
                if stack_len < 2 {
                    return;
                }

                // Trash the top card and promote the next source — and fire
                // OnDigivolutionCardTrashed (handled inside the primitive).
                rctx.effect.armor_purge_top(subject);
                // Cancel the original deletion synchronously. Honored by
                // `delete_permanent_with_cause` (`Cancelled` arm — skip the
                // commit, no OnDeletion / OnAnyDeletion fires).
                rctx.cancel();
            })
            .build()],

        // Phase D Task 6 — printed Save: "When this Digimon would be deleted,
        // you may place it at the bottom of one of your Tamers. If you do, it
        // isn't deleted." DCGO `Save.cs:24-65`.
        //
        // ## Why this is an OnDeletion trigger, not a WhenWouldBeDeleted
        // ## replacement
        //
        // DCGO `Save.cs` mounts as a **post-deletion** trigger:
        // `CanActivateSave` requires `IsTopCardInTrashOnDeletion` (the top
        // has already moved to trash), and the body retrieves the
        // now-trashed card and tucks it under a Tamer. Save **never** sets
        // `willBeRemoveField = false` (compare `ArmorPurge.cs:63` which
        // DOES). Deletion proceeds normally; Save just plucks the top out
        // of trash.
        //
        // Modeling Save as a `WhenWouldBeDeleted` replacement that calls
        // `cancel_leave()` is semantically wrong:
        //   1. **OnDeletion / OnAnyDeletion observers don't fire.** Cards
        //      like Fortitude (Phase D Task 8) listen for "when an ally is
        //      deleted" and would miss a Save'd deletion. DCGO's Fortitude
        //      DOES fire on Save'd cards because the deletion fully
        //      committed before Save retrieved the card.
        //   2. **Save can re-arm.** A cancelled deletion leaves the
        //      carrier on field; the next deletion attempt fires Save
        //      again. DCGO can't because the carrier is gone.
        //   3. **Manual stack-drain duplicates engine deletion logic.**
        //
        // ## Mechanics in the Rust engine
        //
        // `Effect::on_deletion(card)` fires from
        // `commit_permanent_deletion` BEFORE `Player::delete_permanent`
        // runs. At fire time the carrier is still in `battle_area`, so we
        // snapshot the top-card handle and park an OPTIONAL Tamer-pick
        // (`is_optional=true`; PASS = "decline Save").
        //
        // **Substrate hook (Task 6).** When the OnDeletion drain pauses on
        // a parked selection, `commit_permanent_deletion` defers the rest
        // of the deletion sequence (linked-card cascade,
        // `Player::delete_permanent`, modifier cleanup, `OnAnyDeletion`)
        // by stashing the carrier handle in `Game.pending_deletion_resume`
        // and returning. This avoids the synchronous mid-stream
        // `delete_permanent` that would otherwise shift later permanents'
        // indices and invalidate the parked selection's
        // `valid_action_ids`. The resume hook in
        // `effect_queue::resolve_generic_selection` calls
        // `Game::resume_pending_deletion` after the parked selection's
        // callback resolves and the post-callback drain settles —
        // running `finalize_permanent_deletion` to close out the deletion.
        //
        // The Save callback runs FIRST (before the resume hook), while
        // indices are still stable: it lifts the top card off the carrier
        // via `place_card_under_permanent_bottom` (zone-walker finds it
        // in the carrier's `card_sources`) and inserts it at the bottom
        // of the chosen Tamer's stack. The carrier's stack is then empty;
        // the resume hook's `delete_permanent` tolerates an empty stack
        // (see `Player::delete_permanent`) and removes the now-empty
        // carrier slot. `OnAnyDeletion` then fires — Fortitude observers
        // will see the Save'd carrier's deletion as expected.
        //
        // PASS / no-Tamer paths: handler returns without parking the
        // Tamer-pick; the OnDeletion drain unwinds with no
        // `pending_selection` set; `commit_permanent_deletion` continues
        // synchronously through `delete_permanent` (carrier + sources to
        // trash) and `OnAnyDeletion` fires immediately.
        //
        // ## DCGO filter
        //
        // `customMessageArrayTemplate(CanSelectDigimon: false,
        // CanSelectTamer: true)` against own permanents. The inner filter
        // here restricts to (a) same controller as the carrier and (b)
        // Tamer kind. `select_own_permanent` does not auto-scope by owner
        // — the closure must.
        Keyword::Save => vec![Effect::on_deletion(card)
            .name("<Save>")
            .process(|ctx| {
                // OnDeletion is keyed on the carrier's permanent handle —
                // `enqueue_from_permanent` only enumerates effects on the
                // specific deleted permanent, so this trigger is naturally
                // self-scoped (no `subject != me` guard required as in
                // the replacement-window mounting).
                let Some(subject) = ctx.source_permanent else {
                    // Defensive — OnDeletion always carries source_permanent.
                    return;
                };
                let owner = subject.player;

                // Snapshot the carrier's top-card handle. The card hasn't
                // moved to trash yet (deletion is paused on this trigger);
                // when the callback fires, the card is still in the
                // carrier's `card_sources`.
                let self_card = match ctx
                    .game
                    .player(owner)
                    .battle_area
                    .get(subject.index as usize)
                {
                    Some(p) => p.top_card().handle(),
                    None => return,
                };

                // Park the optional Tamer-pick. `is_optional=true` admits
                // PASS as "decline Save". `select_own_permanent` no-ops
                // silently when the candidate filter yields zero matches
                // (no own Tamers) — handler returns; the OnDeletion drain
                // unwinds with no `pending_selection` set;
                // `commit_permanent_deletion` continues to natural
                // finalization on the same call frame.
                ctx.select_own_permanent(
                    "you may place this card under one of your Tamers",
                    /*is_optional=*/ true,
                    move |g, h| {
                        if h.player != owner {
                            return false;
                        }
                        let p = match g.players[h.player as usize]
                            .battle_area
                            .get(h.index as usize)
                        {
                            Some(p) => p,
                            None => return false,
                        };
                        p.is_tamer(&g.card_data)
                    },
                    move |ctx, tamer| {
                        // Lift the saved top card off the carrier and
                        // place it at the bottom of the chosen Tamer's
                        // stack. Indices are stable here: the deferred
                        // `delete_permanent` hasn't run yet; both the
                        // carrier and the Tamer are still on field.
                        //
                        // After this returns, `resolve_generic_selection`
                        // calls `resume_pending_deletion`, which removes
                        // the (now empty-stacked) carrier from
                        // `battle_area` and fires `OnAnyDeletion` — so
                        // observers like Fortitude will see the
                        // deletion event.
                        ctx.place_card_under_permanent_bottom(self_card, tamer);
                    },
                );
            })
            .build()],

        // Phase D Task 7 — printed Decoy: "When one of your other [Digimon]
        // would be deleted, you may delete this Digimon instead." DCGO
        // `Decoy.cs:24-69`.
        //
        // Optional ('may'): the controller may decline by PASSing the outer
        // accept dialog. Synchronous outcome — the body does not park a
        // nested selection; on accept, `rctx.substitute(...)` runs in-place
        // and the dispatcher commits via the `Substituted` arm.
        //
        // Filters (in body, not condition):
        //   1. `subject != me_perm` — never self-redirect (would loop on
        //      the substituted deletion firing Decoy again).
        //   2. `subject.player == me_perm.player` — same controller only;
        //      do NOT protect opponent permanents.
        //   3. Subject's top card must be a Digimon (DCGO restricts to
        //      `[Digimon]`-typed allies, not Tamers).
        //
        // Note on UX: the outer optional dialog is parked at candidate-
        // collection time, BEFORE the body runs. So when the body's filter
        // would reject (subject==self, cross-controller, or non-Digimon),
        // the dialog still appears. On accept, the body falls through
        // without setting an outcome and the original deletion proceeds.
        // This matches the Phase C `nested_select_decoy.rs` precedent and
        // is acceptable for v1; cards needing finer pre-dialog filtering
        // (e.g. printed "Decoy: Black" color filter) override via a
        // hand-rolled `CardEffect`.
        //
        // Color/parameter filtering is NOT in scope here — `Keyword::Decoy`
        // is parsed un-parameterized (`card_data.rs:314`). The auto-install
        // offers any same-controller Digimon. Per-card-text restrictions
        // (e.g. "Decoy: Black") are applied by hand-rolled overrides.
        Keyword::Decoy => vec![Effect::when_would_be_deleted(card)
            .name("<Decoy>")
            .optional()
            .replacement_process(|rctx| {
                // Self-scope guard: never substitute self for self
                // (infinite-loop prevention).
                let me_perm = match rctx.effect.source_permanent {
                    Some(h) => h,
                    None => return,
                };
                let subject = match rctx.subject {
                    ReplacementSubject::Permanent(h) => h,
                    _ => return,
                };
                if subject == me_perm {
                    return;
                }
                // Same-controller filter: Decoy only protects ally
                // Digimon, never opponent permanents.
                if subject.player != me_perm.player {
                    return;
                }
                // Subject must be a Digimon (DCGO `Decoy.cs` predicate
                // restricts to `[Digimon]` typed permanents).
                let game = &*rctx.effect.game;
                let Some(subject_perm) = game
                    .players
                    .get(subject.player as usize)
                    .and_then(|p| p.battle_area.get(subject.index as usize))
                else {
                    return;
                };
                if !subject_perm.is_digimon(&game.card_data) {
                    return;
                }

                // Substitute: redirect deletion to self. Synchronous; the
                // dispatcher's `Substituted` commit arm handles the
                // re-deletion of the carrier in place of the ally.
                rctx.substitute(ReplacementSubject::Permanent(me_perm));
            })
            .build()],

        // Phase D Task 8 — printed Fortitude: "When this Digimon is deleted
        // (and the deleted stack had ≥1 digivolution source under the top),
        // play it from trash without paying its cost and without suspending
        // it." DCGO `Fortitude.cs:14-63`.
        //
        // ## Why this is an `OnDeletion` trigger (gate-detected pre-finalize,
        // ## played post-finalize)
        //
        // DCGO `Fortitude.cs` mounts as a post-deletion trigger:
        // `CanActivateFortitude` requires `IsExistOnTrash(card)` — the card
        // must be in trash at fire time. The body re-plays the card via
        // `PlayPermanentCards(payCost: false, isTapped: false, root: Trash)`.
        //
        // In the Rust engine, `OnDeletion` fires BEFORE `delete_permanent`
        // (carrier still in `battle_area`), and `OnAnyDeletion` is enqueued
        // via `TriggerSource::PlayerBattleArea` AFTER `delete_permanent` —
        // which scans only currently-live permanents. The just-deleted
        // carrier is in trash by then and is NOT picked up by that scan, so
        // modeling Fortitude as a pure `OnAnyDeletion` observer would
        // silently miss its own trigger.
        //
        // The auto-install therefore mounts at `OnDeletion` (so the carrier
        // is still scannable for its effects, and `card_sources.len()` is
        // readable for the gate check) and stashes a deferred replay in
        // `Game.pending_post_deletion_replays`. The substrate hook in
        // `combat::finalize_permanent_deletion` drains the slot AFTER
        // `delete_permanent` moves the carrier + sources to trash but
        // BEFORE the global `OnAnyDeletion` broadcast — running
        // `play_from_trash_free_unsuspended(self_card)` for each entry. So:
        //   - The card is in trash when retrieval runs (DCGO parity:
        //     `IsExistOnTrash` holds at fire time).
        //   - Subsequent `OnAnyDeletion` observers see the replayed
        //     permanent already on field.
        //
        // ## Mandatory semantics
        //
        // DCGO Fortitude has no "may" clause (RULES_CONTEXT 16-26). The
        // trigger fires unconditionally when the gate passes; this matches
        // a non-`.optional()` `OnDeletion` process (no PASS dialog).
        //
        // ## Self-scope
        //
        // The trigger is keyed on the carrier's permanent handle — the
        // OnDeletion enqueue path (`enqueue_triggered(OnDeletion,
        // TriggerSource::Permanent(handle))` in
        // `combat::commit_permanent_deletion`) only enumerates effects on
        // the specific deleted permanent. So Fortitude is naturally
        // self-scoped and does not fire on a neighbor's deletion. The
        // carrier-side guard in the body (`source_permanent` matches) is a
        // belt-and-suspenders defense.
        //
        // ## Known scope: source-card Fortitude (out of Phase D)
        //
        // DCGO `Fortitude.cs:32` filters via `CardStack.Contains(card)` —
        // i.e. Fortitude on a digi source under the top fires when the
        // stack containing it (as a source) is deleted. This auto-install
        // covers only the top-card case (most common). Source-card
        // Fortitude is rare and can be covered by a hand-rolled
        // `CardEffect` override.
        Keyword::Fortitude => vec![Effect::on_deletion(card)
            .name("<Fortitude>")
            .process(|ctx| {
                // Self-scope: OnDeletion is keyed on the carrier's handle, so
                // `source_permanent` should be Some(carrier).
                let Some(handle) = ctx.source_permanent else {
                    return;
                };
                let owner = handle.player;

                // Gate: deleted stack had ≥1 source under the top — i.e.
                // `card_sources.len() >= 2`. The carrier is still in
                // `battle_area` at this timing (OnDeletion fires before
                // `delete_permanent`).
                let Some(perm) = ctx
                    .game
                    .player(owner)
                    .battle_area
                    .get(handle.index as usize)
                else {
                    return;
                };
                if perm.card_sources.len() < 2 {
                    return;
                }

                // Capture self_card (the carrier's top card handle) and
                // stash for the post-finalize replay.
                let self_card = perm.top_card().handle();
                ctx.game
                    .pending_post_deletion_replays
                    .push((owner, self_card));
            })
            .build()],

        // Phase D Task 9 — printed Partition: "When this Digimon leaves the
        // battle area (other than via battle or your own effect), play 2 of
        // its digivolution cards." DCGO `Partition.cs:9-23`, `:71-162`.
        //
        // ## Why this is an `OnDeletion` trigger (not a replacement)
        //
        // DCGO `Partition.cs` mounts as `CanTriggerWhenPermanentRemoveField`
        // and never sets `willBeRemoveField = false` — the parent removal
        // is NOT cancelled. Partition fires concurrent with the deletion
        // and plays cards from the disposed digivolution sources.
        //
        // ## Cause filter (DCGO `Partition.cs:14-19`)
        //
        // ```
        // if (!IsByBattle(...))
        //     if (!IsByEffect(..., cardEffect => IsOwnerEffect(cardEffect, card)))
        //         return true;
        // ```
        //
        // Partition fires when the carrier is deleted by:
        //   - Opponent's effect (`ReplacementCause::OpponentEffect`)
        //   - SecurityCheck / Cost (rare; defensive coverage)
        //
        // Partition does NOT fire when the carrier is deleted by:
        //   - Own effect (`ReplacementCause::OwnEffect`)
        //   - Battle (`ReplacementCause::Battle`)
        //
        // ## Mechanics
        //
        // `Effect::on_deletion(card)` fires from `commit_permanent_deletion`
        // BEFORE `delete_permanent` runs. At fire time the carrier is still
        // in `battle_area` with its full `card_sources` stack — the 2-pick
        // selection enumerates `CountCappedZone::Material(carrier)` (which
        // excludes the top card by construction). When the parked selection
        // resolves, the callback pushes the 2 picks into
        // `Game.pending_post_deletion_replays`; the substrate hook in
        // `finalize_permanent_deletion` drains the slot AFTER the carrier
        // and its sources move to trash, calling
        // `play_from_trash_free_unsuspended` per entry.
        //
        // **Substrate reuse.** This consumes the Phase D Task 8 slot
        // (`pending_post_deletion_replays: Vec<(PlayerId, CardHandle)>`)
        // and Phase D Task 6 deferred-deletion hook
        // (`pending_deletion_resume`). No new substrate added — Partition
        // is an exact composition of the two.
        //
        // ## Color grouping (out of Phase D scope)
        //
        // DCGO injects per-card `firstSources`/`secondSources` via
        // `partitionConditions`. The auto-install offers ANY source from
        // the carrier's stack with no color filter. Per-card overrides
        // apply color grouping via hand-rolled `CardEffect`.
        //
        // ## Mandatory selection (DCGO `Partition.cs:98,126`)
        //
        // `canNoSelect: () => false` — once the gate (`>=2 sources` under
        // top) passes, the controller MUST pick exactly 2 sources. We use
        // `is_optional_zero=false` and rely on `select_count_capped_multi`'s
        // auto-commit when `picked == max`.
        //
        // ## Self-scope
        //
        // OnDeletion is keyed on `TriggerSource::Permanent(carrier)` — the
        // enqueue path (`enqueue_from_permanent`) only enumerates effects
        // on the specific deleted permanent. So the trigger is naturally
        // self-scoped (a neighbor's deletion doesn't fire Partition on
        // this carrier). No subject-mismatch guard required.
        Keyword::Partition => vec![Effect::on_deletion(card)
            .name("<Partition>")
            .process(|ctx| {
                use crate::replacement::ReplacementCause;

                // Cause filter: skip Battle and same-controller (OwnEffect).
                // Note: this matches DCGO exactly. SecurityCheck / Cost
                // causes DO trigger Partition (rare in practice).
                let cause = ctx.deletion_cause();
                if matches!(
                    cause,
                    Some(ReplacementCause::Battle | ReplacementCause::OwnEffect)
                ) {
                    return;
                }

                // Self-scope: OnDeletion is keyed on the carrier handle.
                let Some(carrier) = ctx.source_permanent else {
                    return;
                };
                let owner = carrier.player;

                // Gate: ≥2 selectable sources under the top
                // (`card_sources.len() >= 3` = top + 2+ sources).
                // `Material` zone excludes the top, so the actual
                // candidate count is `card_sources.len() - 1`.
                let stack_len = match ctx
                    .game
                    .player(owner)
                    .battle_area
                    .get(carrier.index as usize)
                {
                    Some(p) => p.card_sources.len(),
                    None => return,
                };
                if stack_len < 3 {
                    return;
                }

                // Park the 2-pick. `select_count_capped_multi` auto-commits
                // when `picked == max` so the callback runs after exactly
                // two picks. The carrier is still on field — the parked
                // selection's `valid_action_ids` map cleanly to live
                // `card_sources` indices on `carrier`.
                ctx.select_count_capped_multi(
                    owner,
                    CountCappedZone::Material(carrier),
                    /*max=*/ 2,
                    "select 2 cards to play",
                    /*is_optional_zero=*/ false,
                    |_g, _src| true,
                    move |ctx, picks| {
                        // Defensive: only act on a complete 2-pick. The
                        // helper will short-circuit and pass an incomplete
                        // accum if all candidates were exhausted (gate
                        // ensures ≥2 candidates, so this should be
                        // unreachable in practice).
                        if picks.len() != 2 {
                            return;
                        }

                        // Stash the picks for post-finalize replay. The
                        // drain in `finalize_permanent_deletion` plays
                        // each from trash, free, unsuspended.
                        for handle in picks {
                            ctx.game
                                .pending_post_deletion_replays
                                .push((owner, handle));
                        }
                    },
                );
            })
            .build()],

        // Phase D Task 10 — printed MaterialSave(N): a `[Main]` active skill
        // (NOT a replacement, NOT a deletion trigger). DCGO `MaterialSave.cs`.
        //
        // ## Activation gate (DCGO `CanActivateMaterialSave`, lines 10-24)
        //
        //   1. Self is on battle area (`IsExistOnBattleArea(card)`).
        //   2. Self has ≥1 selectable digivolution source under top
        //      (`DigivolutionCards.Count(...) >= 1` — DCGO `DigivolutionCards`
        //      excludes the top card, which translates to
        //      `card_sources.len() >= 2` here).
        //   3. Controller has ≥1 selectable target permanent. For the auto-
        //      install we restrict to OWN Tamers (DCGO's most common
        //      `customMessageArrayTemplate(CanSelectDigimon: false,
        //      CanSelectTamer: true)` template); per-card-text overrides
        //      apply tighter filters via hand-rolled `CardEffect`.
        //
        // ## Body (DCGO `MaterialSaveProcess`, lines 28-120)
        //
        //   1. Player picks 1 own Tamer (mandatory once gate passes).
        //   2. Player picks up to N own card_sources from self
        //      (`is_optional_zero=true` — DCGO `canNoSelect: () => true`,
        //      line 76; the player MAY pick fewer than N if they choose).
        //   3. Selected sources are placed at the bottom of the chosen
        //      Tamer's stack via `place_card_under_permanent_bottom`,
        //      mirroring DCGO `selectedPermanent.AddDigivolutionCardsBottom`.
        //
        // ## Cost
        //
        // Zero. DCGO `MaterialSave` is a `[Main]` active skill that does
        // NOT consume memory and does NOT suspend self. The
        // `EffectTiming::MainOnField` machinery exposes the activation in
        // the action mask without any cost gating, matching this.
        //
        // ## Filter scope (Phase D)
        //
        // Per the Phase D plan note: the auto-install offers "any source"
        // filter on the source pick (no DigiXros-style restrictions) and
        // "own Tamer only" on the target pick. Per-card-text restrictions
        // (e.g., DigiXros source filter) are a hand-rolled override on top
        // of the auto-install — out of Phase D scope.
        //
        // ## Self-scope
        //
        // The `[Main]` mask emission iterates the carrier's stack; the
        // `MainOnField` timing on the keyword auto-effect is naturally
        // self-scoped because `activate_field_main` runs only the matched
        // permanent's effects, with `source_permanent` set to the carrier.
        // The closures below use `ctx.source_permanent` to identify the
        // carrier handle for the source-pick zone.
        Keyword::MaterialSave(n) => vec![Effect::declarative(card)
            .name(&format!("<Material Save {n}>"))
            .timing(EffectTiming::MainOnField)
            // Gate at mask-build time so the activation only appears when
            // both halves of CanActivateMaterialSave hold:
            //   - carrier has ≥1 source under top (`card_sources.len() >= 2`)
            //   - carrier's controller has ≥1 own Tamer on field
            .condition(|ctx| {
                let Some(perm) = ctx.source_permanent() else {
                    return false;
                };
                if perm.card_sources.len() < 2 {
                    return false;
                }
                // OWN-Tamer existence check. `EffectReadContext::player`
                // is the controller of the activating effect (the carrier's
                // owner for a MainOnField skill).
                let owner = ctx.player;
                ctx.battle_area(owner)
                    .iter()
                    .any(|p| p.is_tamer(&ctx.game.card_data))
            })
            .process(move |ctx| {
                let Some(me) = ctx.source_permanent else {
                    return;
                };
                let owner = me.player;

                // Step 1: pick a Tamer (own, mandatory).
                // mandatory once activation is chosen — the [Main] activation itself is
                // the "may" hook; unlike Save's post-deletion Tamer pick, declining here
                // has no semantic meaning (player can simply not activate).
                ctx.select_own_permanent(
                    "select a Tamer to receive digivolution cards",
                    /*is_optional=*/ false,
                    move |g, h| {
                        if h.player != owner {
                            return false;
                        }
                        let p = match g.players[h.player as usize]
                            .battle_area
                            .get(h.index as usize)
                        {
                            Some(p) => p,
                            None => return false,
                        };
                        p.is_tamer(&g.card_data)
                    },
                    move |ctx, tamer| {
                        // Step 2: pick up to N sources from self
                        // (`is_optional_zero=true` — DCGO line 76
                        // `canNoSelect: () => true` lets the player pick 0).
                        ctx.select_count_capped_multi(
                            owner,
                            CountCappedZone::Material(me),
                            n,
                            "select cards to place under Tamer",
                            /*is_optional_zero=*/ true,
                            |_g, _src| true,
                            move |ctx, picks| {
                                // Place each picked source at the bottom
                                // of the Tamer's stack, mirroring DCGO's
                                // `AddDigivolutionCardsBottom`.
                                for source in picks {
                                    ctx.place_card_under_permanent_bottom(source, tamer);
                                }
                            },
                        );
                    },
                );
            })
            .build()],

        // Non-replacement keywords — handled elsewhere (combat, mask, etc.).
        _ => Vec::new(),
    }
}
