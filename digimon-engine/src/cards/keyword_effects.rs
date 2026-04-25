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
//! Save. Mandatory ones: Fragment(N), Armor Purge.
//!
//! ## Deferred: Partition
//!
//! Printed `<Partition>` is a leave-field trigger (not a replacement) that
//! plays cards from two color-grouped subsets of the deleted permanent's
//! digivolution sources, and is not yet auto-installed — see Phase D Task 9.
//! `Keyword::Partition` is parsed by `parse_printed_keywords` but this module
//! intentionally returns `Vec::new()` for it; hand-authored `CardEffect`s can
//! cover Partition cards until Task 9 lands.

use crate::card_source::CardHandle;
use crate::effect::Effect;
use crate::effect_context::CountCappedZone;
use crate::enums::{Keyword, Zone};
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
        // Optional outer accept ("you may"). On accept, an inner own-Tamer
        // selection is parked; the controller picks a Tamer; the carrier's
        // top card is moved to the bottom of that Tamer's stack and the
        // original deletion is cancelled. On decline OR when no own Tamers
        // exist (inner select sees zero candidates), original deletion
        // proceeds.
        //
        // DCGO filter: `customMessageArrayTemplate(CanSelectDigimon: false,
        // CanSelectTamer: true)` against own permanents. The inner filter
        // here checks `is_tamer` AND same-controller (`select_own_permanent`
        // does NOT auto-filter by owner — the candidate-walk enumerates
        // every battle-area permanent in the field; the closure restricts
        // to the carrier's owner).
        Keyword::Save => vec![Effect::when_would_be_deleted(card)
            .name("<Save>")
            .optional()
            .replacement_process(|rctx| {
                // Self-scope guard.
                let me_perm = rctx.effect.source_permanent;
                let subject = match rctx.subject {
                    ReplacementSubject::Permanent(h) => h,
                    _ => return,
                };
                if Some(subject) != me_perm {
                    return;
                }

                // Snapshot the carrier's identity for the inner callback.
                // The carrier's top card is what gets tucked under the Tamer.
                let owner = subject.player;
                let self_card = match rctx
                    .effect
                    .game
                    .player(owner)
                    .battle_area
                    .get(subject.index as usize)
                {
                    Some(p) => p.top_card().handle(),
                    None => return,
                };

                // Park the inner Tamer-pick. `select_own_permanent`'s filter
                // is `Fn(&Game, PermanentHandle) -> bool`; we restrict to
                // (a) same controller as the carrier and (b) Tamer kind. The
                // candidate-walk enumerates BOTH players' battle areas via
                // `OwnField`'s install path, so the controller check is
                // load-bearing — without it, opponent Tamers would be
                // candidates.
                rctx.effect.select_own_permanent(
                    "place this card under one of your Tamers",
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
                        // Move the carrier's top card under the Tamer. The
                        // primitive removes it from the carrier's
                        // `card_sources` and inserts at index 0 of the
                        // Tamer's stack.
                        ctx.place_card_under_permanent_bottom(self_card, tamer);

                        // After the move the carrier's stack may now be
                        // empty (or contain only digivolution sources whose
                        // top is gone). Cancel the original deletion (no
                        // OnDeletion side effects) and explicitly remove the
                        // carrier permanent from `battle_area`. Any
                        // remaining `card_sources` (the carrier's own
                        // digivolution sources) are routed to the
                        // controller's trash, which is the natural fate when
                        // a permanent's top card moves elsewhere — DCGO's
                        // `Save.cs` similarly leaves the underlying sources
                        // in trash because the carrier ceases to exist as a
                        // separate permanent.
                        //
                        // `tamer.index` may be in the same battle_area as
                        // the carrier; locate the carrier by its handle
                        // (which has not been recycled — the index reflects
                        // its slot at park time) and bail out gracefully if
                        // it has already been cleaned up.
                        if (subject.index as usize)
                            < ctx.game.player(subject.player).battle_area.len()
                        {
                            // Move the carrier's remaining card_sources to
                            // the controller's trash (these were the
                            // carrier's digivolution sources beneath the
                            // saved top, which has already been moved).
                            let drained: Vec<_> = ctx
                                .game
                                .player_mut(subject.player)
                                .battle_area[subject.index as usize]
                                .card_sources
                                .drain(..)
                                .collect();
                            ctx.game.player_mut(subject.player).trash.extend(drained);
                            // Remove the now-empty carrier slot. NOTE: any
                            // PermanentHandle indices >= subject.index in
                            // the same battle_area shift down by 1 after
                            // this remove. The `tamer` handle stored in
                            // this closure could be invalidated if the
                            // tamer was at a higher index than the carrier
                            // in player 0's battle_area. The actual move
                            // operation has already completed using that
                            // handle, so no further use is made of it
                            // post-remove — safe.
                            ctx.game
                                .player_mut(subject.player)
                                .battle_area
                                .remove(subject.index as usize);
                        }

                        ctx.cancel_leave();
                    },
                );
            })
            .build()],

        // Intentionally not auto-installed (see module docstring).
        Keyword::Partition => Vec::new(),

        // Non-replacement keywords — handled elsewhere (combat, mask, etc.).
        _ => Vec::new(),
    }
}
