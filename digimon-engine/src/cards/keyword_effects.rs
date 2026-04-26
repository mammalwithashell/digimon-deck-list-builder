//! Phase 7 Task 6 — keyword-derived auto-install replacement effects.
//!
//! `keyword_to_auto_effect(kw, card)` maps a printed-keyword entry on a
//! card's `CardData::keywords` to a synthesized [`Effect`] that installs the
//! matching `WhenWouldBe*` replacement. `Game::effects_for_card` calls this
//! for every keyword after retrieving registry effects and appends the
//! non-`None` results so that cards with printed Barrier / Evade /
//! Fragment(N) / Decode / Armor Purge behave as printed without a hand-authored
//! `CardEffect` script.
//!
//! ## Coverage matrix (Phase D — landed 2026-04-25, Phase E — landed 2026-04-25)
//!
//! Auto-installed: Barrier, Evade, Decode (Phase 7); Fragment(N), ArmorPurge,
//! Save, Decoy, Fortitude, Partition, MaterialSave(N) (Phase D);
//! Retaliation, Scapegoat (Phase E).
//!
//! Selection-bearing replacements consume Phase C's parked-replacement
//! substrate via `ctx.cancel_leave / handle_replacement / substitute_replacement`.
//! Trigger-based keywords (Fortitude, Partition, Retaliation) use the standard
//! observer pattern. Scapegoat is an optional `WhenWouldBeDeleted` substitute
//! replacement. MaterialSave(N) is a `[Main]` active skill (neither a replacement
//! nor a deletion trigger) — see §Active-skill keywords below.
//!
//! Phase F so far: Execute (Task 3 — auto-installed `EndOfYourTurn`
//! optional grant + `EndOfAttack` self-delete observer); MindLink (Task 5
//! — `[Main]` active skill on Tamers, optional own-Digimon pick + tuck-
//! under-target via `attach_tamer_to_digimon`). Iceclad is consumed at the
//! combat resolver (no auto-install). Training remains deferred to Task 6.
//!
//! Intentionally NOT auto-installed (per Phase E cards.json survey — zero
//! bare printings; auto-install would double-fire alongside hand-rolled
//! effect text on every card): DeDigivolve(N), DrawX(N).
//!
//! Consumed at resolution site (no auto-install needed): SecurityAttackPlus(N),
//! SecurityAttackMinus(N) — see `Game::security_attack_keyword_bonus` (Phase A §A3).
//!
//! Most replacement keywords here produce **optional** replacements per
//! printed rules ("you may" / "by [cost]"). Declining the optional selection
//! leaves the original event (deletion / return-to-deck) to proceed normally.
//! The optional auto-installs (Phase 7 + Phase D so far): Barrier, Evade,
//! Decode, Save, Decoy, Fragment(N), ArmorPurge. Mandatory ones: Fortitude.
//!
//! ## Trigger-based keywords
//!
//! Not all keywords ride on a `WhenWouldBe*` replacement window — some are
//! triggers that observe a state change without modifying it. Currently
//! installed: Fortitude (`OnDeletion`, mandatory; uses the post-deletion
//! replay substrate to play self from trash after `delete_permanent`
//! finalizes) and Partition (`OnDeletion` with cause filter, trigger-based
//! play of digivolution source cards).
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
use crate::enums::{EffectTiming, Expiry, Keyword, ModifierType, Zone};
use crate::modifiers::ModifierEntry;
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
        // Optional ("by [Effect]") per RULES_CONTEXT 16-36 ("Processing:
        // Optional (choosing and trashing digi cards); if executed, prevention
        // is mandatory") and DCGO `Fragment.cs:37`
        // `SetUpActivateClass(..., isOptional=true, ...)`. The outer accept
        // dialog represents the printed "you may" — declining proceeds with
        // the original deletion. The `canNoSelect: () => false` flag in DCGO
        // `Fragment.cs:38` governs only the INNER source-pick UI (once you've
        // accepted, you must pick exactly N), and `is_optional_zero=false`
        // below mirrors that.
        Keyword::Fragment(n) => vec![Effect::when_would_be_deleted(card)
            .name(&format!("<Fragment ({n})>"))
            .optional()
            // Gate on stack size at candidate-collection time so the outer
            // accept dialog is suppressed when there aren't enough sources to
            // pay the trash cost. DCGO `Fragment.cs:23` `CanReplace` checks
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
                // trash. The outer accept already fired (`.optional()` on
                // the effect); inside the accepted activation, picking is
                // mandatory once the gate passes. `is_optional_zero=false`
                // matches DCGO `Fragment.cs:38` `canNoSelect: () => false`
                // — the inner pick UI does not offer "no selection".
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
        // deleted, by trashing the top card of this Digimon, it isn't
        // deleted." DCGO `ArmorPurge.cs:40-78`.
        //
        // Optional ("by [cost]") per RULES_CONTEXT 16-18 ("Processing:
        // Optional ('by trashing the top card of the Digimon')"). The outer
        // accept dialog represents the printed "by trashing" optional cost —
        // declining proceeds with the original deletion. Once accepted, the
        // synchronous body trashes the top via `armor_purge_top` and cancels
        // the deletion; no nested player selection is required.
        //
        // Gate: `card_sources.len() >= 2` (top + ≥1 source under it). The
        // condition runs inside `collect_candidates`, so when the gate fails
        // the candidate is never produced — no outer accept dialog is offered
        // and the original deletion proceeds normally. (The closure body's
        // re-check is belt-and-suspenders for an earlier same-chain replacement
        // shrinking the stack between collection and process.)
        //
        // The event-fire (OnDigivolutionCardTrashed) for the trashed top is
        // handled by `EffectContext::armor_purge_top` itself; see Phase D
        // Task 5 commit log.
        Keyword::ArmorPurge => vec![Effect::when_would_be_deleted(card)
            .name("<Armor Purge>")
            .optional()
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

        // Phase E §E1 — printed Retaliation: "When this Digimon is deleted
        // in battle, delete the battled opponent's Digimon." DCGO
        // `Retaliation.cs`. RULES_CONTEXT 16-12 (Trigger-type, Mandatory).
        //
        // ## Cause filter
        //
        //   - `deletion_cause() == Some(Battle)` only — RULES_CONTEXT 16-12
        //     specifies battle deletion. Effect, SecurityCheck, and Cost
        //     causes do NOT trigger Retaliation.
        //
        // ## Target identification
        //
        // `ctx.battle_opponent_of(self)` reads the live `Game.pending_attack`
        // (set in `combat::resolve_battle` and not cleared until after
        // `delete_permanent_with_cause` returns) and returns the opposing
        // combatant — i.e., the battle winner, since the loser is the one
        // calling this OnDeletion observer. Returns None for direct-player
        // attacks (no Digimon target) and for non-combatants.
        //
        // ## Mandatory semantics
        //
        // No "may" clause (RULES_CONTEXT 16-12). The trigger fires
        // unconditionally when the Battle cause gate passes; no
        // `.optional()` call on the builder.
        //
        // ## Self-scope
        //
        // The `OnDeletion` enqueue path keys on `TriggerSource::Permanent(h)`
        // — natural self-scoping (a neighbor's deletion doesn't fire
        // Retaliation on this carrier). The `source_permanent` guard in
        // the body is belt-and-suspenders.
        //
        // ## Mutual destruction (RULES_CONTEXT 16-12-4 multi-instance hint)
        //
        // When both combatants have Retaliation and tie in DP, both die in
        // battle (`combat::resolve_battle::MutualDestruction` path). The
        // defender's Retaliation fires first; it deletes the attacker. The
        // attacker's Retaliation then fires, but `battle_opponent_of` may
        // still return `Some(defender)` since `pending_attack` remains live.
        // The guard `battle_area.get(winner.index).is_none()` prevents
        // double-delete on an already-departed permanent — silent no-op
        // rather than routing through a deletion that would be a no-op at
        // `finalize_permanent_deletion` but incurs unnecessary work.
        //
        // ## Cause = OwnEffect (explicit)
        //
        // We bypass `ctx.delete_permanent` (which routes through
        // `infer_deletion_cause`) because `pending_attack` is still live
        // during the OnDeletion drain — `infer_deletion_cause` would return
        // `Battle` even though Retaliation is the carrier's own triggered
        // effect, not a new battle initiation. Using `OwnEffect` explicitly:
        //   - correctly labels the cascade delete for downstream Battle-gated
        //     triggers (a winner with its own Retaliation sees `OwnEffect`
        //     and correctly does NOT re-fire its own cause gate);
        //   - is accurate: the winner is deleted by the loser's keyword effect,
        //     not by the battle resolution itself.
        // Progress guard (Phase B §B4) is reproduced inline — Retaliation is
        // an opponent-sourced effect from the winner's perspective, so
        // `ctx.player` (the loser's controller) is the correct acting player.
        Keyword::Retaliation => vec![Effect::on_deletion(card)
            .name("<Retaliation>")
            .process(|ctx| {
                use crate::replacement::ReplacementCause;
                // Cause gate: Battle only.
                if !matches!(ctx.deletion_cause(), Some(ReplacementCause::Battle)) {
                    return;
                }
                let Some(me) = ctx.source_permanent else {
                    return;
                };
                let Some(winner) = ctx.battle_opponent_of(me) else {
                    return;
                };
                // Mutual-destruction guard: in a tied-DP battle, the attacker
                // may already have been deleted (by the defender's Retaliation
                // firing first) by the time this side's Retaliation runs.
                // Use a `battle_area` slot check rather than `handle_valid`
                // (which is module-private to `combat`).
                if ctx
                    .game
                    .player(winner.player)
                    .battle_area
                    .get(winner.index as usize)
                    .is_none()
                {
                    return;
                }
                // Progress guard (Phase B §B4): replicated from
                // `ctx.delete_permanent`. `ctx.player` is the loser's
                // controller — the acting player for this effect.
                if ctx.game.progress_excludes(winner, Some(ctx.player)) {
                    return;
                }
                // Explicit cause=OwnEffect — bypasses infer_deletion_cause's
                // pending_attack→Battle short-circuit. See doc comment above.
                ctx.game
                    .delete_permanent_with_cause(winner, ReplacementCause::OwnEffect);
            })
            .build()],

        // Phase F Task 1 — printed Scapegoat: "When this Digimon would be
        // deleted [other than by your own effect], you may delete another
        // of your Digimon to prevent it." DCGO `Scapegoat.cs`.
        // RULES_CONTEXT 16-31 (Immediate-type, Optional).
        //
        // ## Cause filter (UPSTREAM)
        //
        // `cause != OwnEffect` — RULES_CONTEXT 16-31: a player's own
        // effect cannot trigger their Scapegoat. Battle, OpponentEffect,
        // SecurityCheck, and Cost all DO trigger.
        //
        // Phase F migrated this from the in-body fallback to the
        // upstream candidate filter via `.replacement_condition(...)`.
        // The dispatcher consults this BEFORE installing the outer
        // optional accept dialog, so the spurious-dialog UX divergence
        // from Phase E is closed. (Phase E note: the cause filter ran
        // in-body because `collect_candidates` did not yet thread `cause`
        // into the effect-side filter; Task 1 added that substrate.)
        //
        // ## No-substitute filter (UPSTREAM)
        //
        // Mirrors DCGO `CanActivateScapegoat`'s `HasMatchConditionPermanent`
        // gate: the keyword is inactive when the controller has no other
        // own permanents to substitute. Suppressed at candidate-collection
        // time so the outer dialog never parks in this case.
        //
        // ## Selection chain
        //
        //   1. Outer optional accept dialog ("may"). PASS leaves the
        //      original deletion to proceed.
        //   2. On ACCEPT: parked own-permanent pick via
        //      `rctx.effect.select_own_permanent(...)`. Filter: same-
        //      controller, non-self. Mandatory once accepted (DCGO: once
        //      committed to substitute, must pick).
        //   3. On pick: `ctx.substitute_replacement(Permanent(picked))`
        //      writes `Substituted` to the parked slot. The dispatcher's
        //      post-callback hook commits the substituted deletion.
        //
        // ## Self-scope
        //
        // `WhenWouldBeDeleted` enumerates only effects on the deletion
        // subject's permanent, so this body is naturally self-scoped.
        // The `subject == me_perm` guard is an explicit belt-and-suspenders
        // defense against any future cross-permanent enumeration changes.
        Keyword::Scapegoat => vec![Effect::when_would_be_deleted(card)
            .name("<Scapegoat>")
            .optional()
            .replacement_condition(|ctx, cause| {
                use crate::replacement::ReplacementCause;
                // RULES_CONTEXT 16-31: skip own-effect deletions.
                if matches!(cause, ReplacementCause::OwnEffect) {
                    return false;
                }
                // DCGO HasMatchConditionPermanent: at least one other own
                // permanent must exist as a substitute candidate.
                let Some(me) = ctx.source_permanent else {
                    return false;
                };
                let owner = me.player;
                let battle = ctx.battle_area(owner);
                battle
                    .iter()
                    .enumerate()
                    .any(|(i, _)| i as u8 != me.index)
            })
            .replacement_process(|rctx| {
                use crate::replacement::ReplacementSubject;

                // Self-scope guard (defense-in-depth — the candidate
                // collector only enumerates the subject's own permanent
                // for `WhenWouldBeDeleted`).
                let me_perm = match rctx.effect.source_permanent {
                    Some(h) => h,
                    None => return,
                };
                let subject = match rctx.subject {
                    ReplacementSubject::Permanent(h) => h,
                    _ => return,
                };
                if subject != me_perm {
                    return;
                }

                let owner = me_perm.player;

                // Inner pick: another of own permanents. Filter: same-
                // controller, non-self. Mandatory once accepted
                // (is_optional=false). The upstream
                // `replacement_condition` already guarantees at least one
                // candidate exists, so this select_own_permanent will
                // always install a pending_selection.
                rctx.effect.select_own_permanent(
                    "select another of your Digimon to delete instead",
                    /*is_optional=*/ false,
                    move |_g, h| h.player == owner && h != me_perm,
                    move |ctx, picked| {
                        // Substitute the deletion subject to the picked
                        // permanent. The dispatcher's Substituted commit
                        // arm finalizes the redirected deletion.
                        ctx.substitute_replacement(ReplacementSubject::Permanent(picked));
                    },
                );
            })
            .build()],

        // Phase F Task 3 — printed Execute: "At end of your turn, this
        // Digimon may attack — including unsuspended Digimon — and when
        // the attack ends it is deleted." DCGO `Execute.cs:18-87`.
        // RULES_CONTEXT 16-37 (Trigger-type, Optional).
        //
        // ## Two-effect install
        //
        // 1. `EndOfYourTurn` triggered effect (NOT `.optional()` — see
        //    "Where the 'may' lives" below). Body unconditionally
        //    grants `MayAttack` + `CanAttackUnsuspended` modifiers on
        //    self with `Expiry::EndOfTurn`. The end-of-turn-attack flow
        //    in `game_phases::end_turn` reads `has_end_of_turn_keywords`,
        //    sees the `MayAttack` modifier, and parks the phase in
        //    `EndOfTurnAction` so the player can spend the granted
        //    attack via the §4.6 attack mask. The
        //    `CanAttackUnsuspended` half widens that mask to also offer
        //    unsuspended-target attack bits — equivalent to DCGO's
        //    `CanAttackTargetDefendingPermanentClass` with
        //    `defenderCondition: !defender.IsSuspended`.
        //
        // 2. `EndOfAttack` observer. Self-deletes the carrier with
        //    `ReplacementCause::OwnEffect` when the attack ends — gated
        //    on `pa.attacker == me` so it only fires for attacks the
        //    Execute carrier itself initiated, not for other attacks
        //    that resolve while it's on field. Cause = OwnEffect
        //    matches the keyword being the carrier's own triggered
        //    effect (cf. Retaliation's same cause-labeling rationale).
        //
        // ## Optionality (RULES_CONTEXT 16-37: Optional)
        //
        // The printed "may" surfaces at the EOT-action phase PASS
        // exit, not at the EndOfYourTurn trigger — see "Where the
        // 'may' lives" below for the design rationale. PASS at
        // EOT-action skips the granted attack; the `EndOfAttack`
        // observer is gated on `pa.attacker == me`, which never holds
        // when no attack initiates, so a declined Execute leaves the
        // carrier on field and the `Expiry::EndOfTurn` modifiers expire
        // cleanly on rotation. DCGO arrives at the same observable
        // outcome via `UntilEndAttackEffects` only firing when
        // `SelectAttackEffect` actually runs.
        //
        // ## Self-scope
        //
        // The `EndOfYourTurn` enqueue path in
        // `enqueue_from_permanent` keys on the carrier's permanent
        // handle, so the trigger fires per-carrier (a sibling
        // permanent's EndOfYourTurn does not trigger this Execute).
        // The `EndOfAttack` observer additionally checks
        // `pa.attacker == me` so it only fires for the carrier's own
        // attack, not for any other end-of-attack on the same field.
        Keyword::Execute => vec![
            // (1) EndOfYourTurn — grant attack modifiers unconditionally.
            //
            // ## Where the "may" lives
            //
            // RULES_CONTEXT 16-37 / DCGO `Execute.cs` describe an
            // optional trigger: the player may decline the entire
            // sequence at the OnEndYourTurn dialog, and on decline no
            // modifiers / attack / self-delete happen.
            //
            // The Rust engine's effect-queue drainer auto-fires single
            // optional triggers without a dedicated may-dialog (see
            // `effect_queue::drain_effect_queue`, single-trigger fast
            // path). And nesting an inner `select_own_permanent` as a
            // makeshift may-prompt doesn't help: when the inner select
            // parks, control returns to `Game::end_turn` BEFORE the
            // modifiers land, so `has_end_of_turn_keywords` finds
            // nothing to park for and the phase rotates straight
            // through. By the time the player accepts the inner pick
            // and the modifiers land, the EOT-action phase window has
            // already passed.
            //
            // We therefore push the "may" decision down to the EOT-
            // action phase itself, which already exposes PASS as the
            // standard exit. The modifier grant is unconditional in
            // the EndOfYourTurn body; the player either uses the
            // granted attack (via the §4.6 mask) or PASSes the
            // EOT-action phase to decline. PASS skips the attack —
            // the `EndOfAttack` self-delete observer is gated on
            // `pa.attacker == me`, which never holds when no attack
            // initiates, so a declined Execute leaves the carrier on
            // field. Modifiers carry `Expiry::EndOfTurn` so they
            // expire cleanly on the eventual turn rotation.
            //
            // Observable parity with DCGO:
            //   - Accept + attack: modifiers granted, attack runs,
            //     carrier deletes via EndOfAttack observer.
            //   - Decline (PASS at EOT-action): no attack, no
            //     self-delete, modifiers expire. Identical
            //     observable outcome to DCGO's "decline OnEndYourTurn
            //     trigger" path — `UntilEndAttackEffects` only fires
            //     when a SelectAttackEffect actually runs in DCGO,
            //     and the EndOfAttack observer here only fires when
            //     `pending_attack` actually carries this carrier as
            //     the attacker.
            Effect::end_of_your_turn(card)
                .name("<Execute>")
                .process(|ctx| {
                    let Some(me) = ctx.source_permanent else {
                        return;
                    };
                    let owner = me.player;
                    // Grant MayAttack — drives `has_end_of_turn_keywords`
                    // to park the phase in EOT-action and the §4.6 mask
                    // emitter to surface attack bits for this permanent.
                    ctx.game.modifiers.add(
                        me,
                        ModifierEntry::simple(
                            ModifierType::MayAttack,
                            1,
                            Expiry::EndOfTurn,
                            owner,
                        ),
                    );
                    // Grant CanAttackUnsuspended — widens the §4.6 mask
                    // to include unsuspended-target attack bits, matching
                    // DCGO's `defenderCondition: !defender.IsSuspended`.
                    ctx.game.modifiers.add(
                        me,
                        ModifierEntry::simple(
                            ModifierType::CanAttackUnsuspended,
                            1,
                            Expiry::EndOfTurn,
                            owner,
                        ),
                    );
                })
                .build(),

            // (2) EndOfAttack — self-delete when the carrier was the
            //     attacker of the just-resolved attack.
            Effect::end_of_attack(card)
                .name("<Execute> self-delete")
                .process(|ctx| {
                    use crate::replacement::ReplacementCause;
                    let Some(me) = ctx.source_permanent else {
                        return;
                    };
                    // Gate: only fire on the Execute carrier's own
                    // attack. EndOfAttack is a global timing — it
                    // would otherwise fire for any attack while the
                    // carrier sits on the field, e.g. an attack
                    // initiated next turn by some other Digimon.
                    let attacker_is_me = ctx
                        .game
                        .pending_attack
                        .as_ref()
                        .map(|pa| pa.attacker == me)
                        .unwrap_or(false);
                    if !attacker_is_me {
                        return;
                    }
                    // Cause = OwnEffect — the deletion is driven by the
                    // carrier's own triggered keyword, not by combat
                    // resolution or any opponent effect. Matches the
                    // labeling pattern Retaliation uses (see
                    // `Keyword::Retaliation` arm above) and is
                    // accurate per DCGO `Execute.cs:74-83` (the
                    // `DeleteSelfEffect` is the carrier's own
                    // queued ICardEffect, not a battle outcome).
                    ctx.game.delete_permanent_with_cause(
                        me,
                        ReplacementCause::OwnEffect,
                    );
                })
                .build(),
        ],

        // Phase F §F3 — printed Mind Link: `[Main]` active skill on Tamers.
        // "Place this Tamer at the bottom of one of your Digimon's
        // digivolution stack. Target Digimon must have no Tamer cards in its
        // digivolution stack (face-down Tamer sources don't count)." DCGO
        // `MindLink.cs`. RULES_CONTEXT 16-27.
        //
        // ## Activation gate
        //
        //   1. Self is a Tamer on battle area.
        //   2. Controller has ≥1 own non-Tamer permanent with no
        //      non-face-down Tamer source (DCGO line 25:
        //      `cardSource.IsTamer && !cardSource.IsFlipped`).
        //   3. Target is not a token (DCGO line 23: `!permanent.IsToken`).
        //
        // ## Body
        //
        // Optional pick (DCGO `canNoSelect: true`, line 60). Player selects
        // the target Digimon; on pick, `attach_tamer_to_digimon(self, picked)`
        // moves the Tamer's top card to the bottom of the Digimon's stack
        // and removes the Tamer permanent from battle area (mirroring DCGO
        // `IPlacePermanentToDigivolutionCards(new[] { tamer, digimon })`).
        //
        // ## Cost
        //
        // Zero. `[Main]` active skill — `EffectTiming::MainOnField` exposes
        // the activation in the action mask without any cost gating, mirroring
        // MaterialSave's parity treatment.
        //
        // ## Self-scope
        //
        // The `[Main]` mask emission iterates the carrier's stack; the
        // `MainOnField` timing on the keyword auto-effect is naturally
        // self-scoped because `activate_field_main` runs only the matched
        // permanent's effects, with `source_permanent` set to the carrier.
        Keyword::MindLink => vec![Effect::declarative(card)
            .name("<Mind Link>")
            .timing(EffectTiming::MainOnField)
            // Gate at mask-build time so the activation only appears when
            // the carrier is a Tamer on field AND there is at least one
            // valid target Digimon (non-Tamer top, no token, no
            // non-face-down Tamer source).
            .condition(|ctx| {
                let Some(perm) = ctx.source_permanent() else {
                    return false;
                };
                if !perm.is_tamer(&ctx.game.card_data) {
                    return false;
                }
                let owner = ctx.player;
                ctx.battle_area(owner).iter().any(|p| {
                    !p.is_tamer(&ctx.game.card_data)
                        && !p.top_card().is_token
                        && !p.has_non_facedown_tamer_source(&ctx.game.card_data)
                })
            })
            .process(move |ctx| {
                let Some(me) = ctx.source_permanent else {
                    return;
                };
                let owner = me.player;

                // Optional own-permanent pick (DCGO `canNoSelect: true`).
                // Filter mirrors the gate plus a self-exclusion (the
                // carrier is itself on its controller's battle area; we
                // must never target it).
                ctx.select_own_permanent(
                    "select a Digimon to receive the Mind Link Tamer",
                    /*is_optional=*/ true,
                    move |g, h| {
                        if h.player != owner || h == me {
                            return false;
                        }
                        let Some(p) = g.players[h.player as usize]
                            .battle_area
                            .get(h.index as usize)
                        else {
                            return false;
                        };
                        // Same constraints as the activation gate: non-Tamer
                        // top, non-token, no non-face-down Tamer source.
                        if p.is_tamer(&g.card_data) {
                            return false;
                        }
                        if p.top_card().is_token {
                            return false;
                        }
                        !p.has_non_facedown_tamer_source(&g.card_data)
                    },
                    move |ctx, picked| {
                        ctx.attach_tamer_to_digimon(me, picked);
                    },
                );
            })
            .build()],

        // Phase F §F4 — printed Training: `[Main]` active skill, usable from
        // BATTLE area OR BREEDING area (RULES_CONTEXT 16-40 / DCGO
        // `Training.cs`). Cost: suspend self (must be unsuspended). Effect:
        // place top deck card at the BOTTOM of self's digivolution stack,
        // FACE-DOWN (DCGO `isFacedown: true`).
        //
        // ## Activation gate
        //
        // `!perm.is_suspended` — DCGO line 23
        // `if (thisPermanent.IsSuspended || !thisPermanent.CanSuspend) yield break;`.
        // We omit the `CanSuspend` check (no analog in this engine; suspension
        // is universally allowed except via inert lock-out modifiers, which
        // would manifest as `is_suspended` already being set or pre-empted).
        //
        // No deck-size gate — matches DCGO's `SetUpActivateClass` framework
        // (which never pre-checks `LibraryCards[0]`). On empty deck, the
        // body's `training_place_deck_top_under_self_face_down` no-ops; the
        // suspend cost is still paid (mirrors the documented "no-op on empty
        // source" pattern, e.g. `Player::draw`).
        //
        // ## Cost payment in `process` (not `pay_cost_fn`)
        //
        // `pay_cost_fn` fires only on the queue-driven trigger path
        // (`effect_queue.rs:524`), not on the synchronous `[Main]` activation
        // path (`activate_field_main`). For `MainOnField` skills the cost
        // must be folded into the body — the precedent here is implicit
        // since MaterialSave / MindLink are zero-cost; Training is the first
        // `MainOnField` auto-install with a state-cost (suspend).
        //
        // ## Self-scope
        //
        // The `[Main]` mask emission iterates the carrier's stack; the
        // `MainOnField` timing on the keyword auto-effect is naturally
        // self-scoped because `activate_field_main` runs only the matched
        // permanent's effects, with `source_permanent` set to the carrier.
        //
        // ## Battle-area vs. breeding-area dispatch
        //
        // Battle-area Training is dispatched by the existing `[Main]` machinery
        // unchanged. Breeding-area Training requires a parallel mask emitter
        // and dispatcher path (Phase F §F4 substrate work) under the field
        // index `BREEDING_TARGET (=14)` — which is gated to Training-bearing
        // carriers only, since RULES_CONTEXT 16-40 specifies that ONLY
        // `<Training>` activates from breeding (surfacing all `MainOnField`
        // effects from breeding would inadvertently expose Save / MaterialSave
        // / MindLink from breeding too, which would be wrong).
        Keyword::Training => vec![Effect::declarative(card)
            .name("<Training>")
            .timing(EffectTiming::MainOnField)
            .condition(|ctx| {
                let Some(perm) = ctx.source_permanent() else {
                    return false;
                };
                !perm.is_suspended
            })
            .process(|ctx| {
                let Some(me) = ctx.source_permanent else {
                    return;
                };
                // Pay cost: suspend self. Deliberately direct on
                // `players[..].battle_area[..].is_suspended` /
                // `breeding_area.is_suspended` rather than via
                // `EffectContext::suspend`, because:
                //   1. `EffectContext::suspend` delegates to
                //      `Game::suspend`, which only finds permanents in
                //      `battle_area` — breeding-area carriers would silently
                //      not suspend.
                //   2. The carrier's own self-suspend doesn't need
                //      `OnSuspend` observer firing (DCGO uses
                //      `SuspendPermanentsClass.Tap` which fires its own
                //      tap event, but the Rust observer hooks are listening
                //      on battle-area permanents only — and there are no
                //      cards that observe own-self breeding-area tap).
                let owner = me.player;
                let player = ctx.game.player_mut(owner);
                if let Some(p) = player.battle_area.get_mut(me.index as usize) {
                    p.is_suspended = true;
                } else if let Some(ref mut breeding) = player.breeding_area {
                    breeding.is_suspended = true;
                }
                // Effect: deck-top → bottom of self's stack, face-down.
                ctx.training_place_deck_top_under_self_face_down(me);
            })
            .build()],

        // Non-replacement keywords — handled elsewhere (combat, mask, etc.).
        _ => Vec::new(),
    }
}
