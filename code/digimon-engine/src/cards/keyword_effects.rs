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
//! ## Coverage matrix (Phase D — landed 2026-04-25, Phase E — landed 2026-04-25, Phase F — landed 2026-04-25)
//!
//! Auto-installed: Barrier, Evade, Decode (Phase 7); Fragment(N), ArmorPurge,
//! Save, Decoy, Fortitude, Partition, MaterialSave(N) (Phase D);
//! Retaliation, Scapegoat (Phase E); Execute, MindLink, Training (Phase F);
//! Engage and Guard (EX12).
//!
//! Selection-bearing replacements consume Phase C's parked-replacement
//! substrate via `ctx.cancel_leave / handle_replacement / substitute_replacement`.
//! Trigger-based keywords (Fortitude, Partition, Retaliation) use the standard
//! observer pattern. Scapegoat is an optional `WhenWouldBeDeleted` substitute
//! replacement. Guard is an optional cross-permanent `WhenWouldLeaveBattleArea`
//! cancel replacement. MindLink and Training are `[Main]` active skills;
//! MaterialSave(N) is a deletion-timed source rescue trigger. Execute is an
//! `EndOfYourTurn` triggered effect granting `MayAttack` +
//! `CanAttackUnsuspended` for the EOT-attack window with an `EndOfAttack`
//! self-delete observer; Engage grants only normal `MayAttack` for that same
//! EOT-attack window.
//!
//! The legacy DCGO `KeyWordEffects/*.cs` keyword set has matching Rust enum
//! variants + consumers (auto-install or resolution-site consumption). Later
//! card sets can still add new printed keywords such as EX12's Engage/Guard.
//!
//! Intentionally NOT auto-installed (per Phase E cards.json survey — zero
//! bare printings; auto-install would double-fire alongside hand-rolled
//! effect text on every card): DeDigivolve(N), DrawX(N).
//!
//! Also intentionally NOT auto-installed: DigiBurst(N). The printed token is
//! a **cost prefix** for an effect body that lives in the same `[Main]`
//! activation, but the body's text varies per card and cannot be synthesized
//! from the keyword alone. The reusable authoring surface is the DSL
//! `digi_burst: { count: N, then: [...] }` step (lowers to `SelectOwnSources`
//! with `target: source`, `min=max=N`, plus `TrashSelectedSources` prepended
//! to the body) — see `code/digimon-dsl/src/compile.rs` and
//! `RUST_ENGINE_GAPS.md` "<Digi-Burst N> keyword". An auto-install that
//! merely paid the cost without a body would be strictly worse for the
//! player; declining to install matches DCGO, which also implements
//! Digi-Burst inline per card rather than via a shared `KeyWordEffects/*`
//! file. Card data parsing still produces `Keyword::DigiBurst(N)` for tensor
//! / mask awareness and for any future "cards with `<Digi-Burst>`" filter
//! predicates (e.g. BT4-076 reveal-and-add).
//!
//! ## Combat-only consumption (no auto-install)
//!
//! Iceclad (Phase F) does not have a `keyword_to_auto_effect` arm — it is
//! consumed directly in `combat::resolve_battle`, which swaps the DP compare
//! for a `card_sources.len()` compare when either combatant has the keyword.
//! Security battles route through a different path (`resolve_player_security_loop`)
//! and are unaffected, matching the RULES_CONTEXT 16-34 exception.
//!
//! ## Replacement-condition substrate (Phase F)
//!
//! `Effect.replacement_condition` (closure form `EffectReplacementConditionFn`)
//! threads `cause` into the candidate-collection path via
//! `replacement::collect_candidates`. Scapegoat was promoted onto this surface
//! to gate the outer "may" dialog on `cause != OwnEffect` AND ≥1 substitute
//! candidate, matching DCGO's `CanActivateScapegoat` pre-filter. Future
//! cause-aware replacement keywords should use the same builder hook rather
//! than relying on the inner-pick PASS to suppress spurious dialogs.
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
//! ## Material Save
//!
//! `MaterialSave(N)` is a deletion-timed optional rescue. It reads the deleted
//! permanent's source snapshot, filters those handles through the carrier's
//! printed DigiXros recipe when one is authored, then parks an own-Tamer pick
//! followed by an up-to-N source pick from the moved cards in trash.
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
use crate::effect_context::{CountCappedZone, EffectContext};
use crate::enums::{EffectTiming, Expiry, Keyword, ModifierType, Zone};
use crate::modifiers::ModifierEntry;
use crate::replacement::ReplacementSubject;
use crate::resume::{
    KeywordAscensionChoiceState, KeywordMaterialSaveTamerSelectionState,
    KeywordMindLinkSelectionState, KeywordSaveSelectionState, KeywordScapegoatSelectionState,
    NonDslCountCappedState, NonDslCountCappedTerminal, ResumeFrame, ResumeProvenance, ResumeStack,
};

fn resume_provenance(ctx: &EffectContext<'_>) -> ResumeProvenance {
    ResumeProvenance {
        source_card: ctx.source_card,
        source_permanent: ctx.source_permanent,
        source_kind: ctx.source_kind,
        controller: ctx.player,
        override_pin: ctx.override_selecting_player(),
    }
}

fn park_keyword_frame(ctx: &mut EffectContext<'_>, frame: ResumeFrame) {
    if ctx.game.pending_selection.is_some() {
        ctx.game.pending_selection_resume = Some(ResumeStack {
            frames: vec![frame],
        });
    }
}

fn park_keyword_count_capped_frame(
    ctx: &mut EffectContext<'_>,
    prov: ResumeProvenance,
    of_player: crate::enums::PlayerId,
    zone: CountCappedZone,
    min: u8,
    max: u8,
    is_optional_zero: bool,
    terminal: NonDslCountCappedTerminal,
) {
    if let Some(pending) = ctx.game.pending_selection.as_ref() {
        ctx.game.pending_selection_resume = Some(ResumeStack {
            frames: vec![ResumeFrame::NonDslCountCappedStep(NonDslCountCappedState {
                prov,
                of_player,
                zone,
                min,
                max,
                is_optional_zero,
                distinct_by: None,
                candidate_actions: pending.valid_action_ids.clone(),
                accum: Vec::new(),
                prompt: pending.prompt.clone(),
                previous_phase: pending.previous_phase,
                terminal,
                outer_conts: Vec::new(),
            })],
        });
    }
}

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
        // Printed [Hand][Counter] <Blast Digivolve>: the Counter window scans
        // hand-card effects for this marker and then validates the actual
        // digivolution route against CardData. No process body is needed here.
        Keyword::BlastDigivolve => vec![Effect::declarative(card)
            .name("<Blast Digivolve>")
            .blast_digivolve()
            .build()],

        // Printed Barrier: "When this Digimon would be deleted, you may trash
        // the top card of your security stack. If you do, it isn't deleted."
        // DCGO `Barrier.cs` gates on `SecurityCards.Count >= 1` before the
        // optional replacement is offered.
        Keyword::Barrier => vec![Effect::when_would_be_deleted(card)
            .name("<Barrier>")
            .optional()
            .condition(|ctx| {
                let Some(perm) = ctx.source_permanent() else {
                    return false;
                };
                !ctx.game.players[perm.top_card().owner as usize]
                    .security
                    .is_empty()
            })
            .replacement_condition(|ctx, _subject| {
                use crate::replacement::ReplacementCause;
                matches!(ctx.replacement_cause(), Some(ReplacementCause::Battle))
            })
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
                    if rctx.effect.trash_top_security(owner) {
                        rctx.handled();
                    }
                }
            })
            .build()],

        // Printed Evade: "When this Digimon would be deleted, you may suspend
        // it to prevent that deletion." DCGO `Evade.cs:38-49`.
        //
        // Cost: suspend the carrier (paid via `EffectContext::suspend`, which
        // fires `OnSuspend` observers as a regular suspension would). Effect:
        // cancel the deletion. The carrier survives on the field, suspended.
        //
        // Gate (DCGO `CanActivateEvade` → `CanActivatePermanentSuspendCostEffect`):
        // the carrier must NOT already be suspended. An already-suspended
        // carrier cannot pay the cost. We check this at candidate-collection
        // time so the outer accept dialog is suppressed when the cost cannot
        // be paid; the body re-checks belt-and-suspenders.
        //
        // `ModifierType::CannotSuspend` IS consulted here: DCGO's gate
        // (`Evade.cs:28` `CanActivatePermanentSuspendCostEffect` →
        // `Permanent.CanSuspend`) requires the carrier to be suspendABLE,
        // not just unsuspended — a locked carrier cannot pay the cost and
        // the outer accept dialog is suppressed (prohibition precedence
        // 15-1-3).
        Keyword::Evade => vec![Effect::when_would_be_deleted(card)
            .name("<Evade>")
            .optional()
            .condition(|ctx| {
                let Some(handle) = ctx.source_permanent else {
                    return false;
                };
                let Some(perm) = ctx.source_permanent() else {
                    return false;
                };
                !perm.is_suspended
                    && !ctx
                        .game
                        .modifiers
                        .has(handle, crate::enums::ModifierType::CannotSuspend)
            })
            .replacement_process(|rctx| {
                // Self-scope guard: only fire on the carrier's own deletion.
                let me_perm = rctx.effect.source_permanent;
                let subject = match rctx.subject {
                    ReplacementSubject::Permanent(h) => h,
                    _ => return,
                };
                if Some(subject) != me_perm {
                    return;
                }
                // Re-check the gate at process time. A prior replacement in
                // the same chain could have suspended the carrier between
                // collection and process.
                let already_suspended = rctx
                    .effect
                    .game
                    .player(subject.player)
                    .battle_area
                    .get(subject.index as usize)
                    .map(|p| p.is_suspended)
                    .unwrap_or(true);
                if already_suspended {
                    return;
                }
                // Re-check the suspend-cost prohibition too (see the
                // condition gate above): a same-chain effect could have
                // installed `CannotSuspend` between collection and process.
                if rctx
                    .effect
                    .game
                    .modifiers
                    .has(subject, crate::enums::ModifierType::CannotSuspend)
                {
                    return;
                }
                // Pay the cost: suspend the carrier (fires OnSuspend).
                rctx.effect.suspend(subject);
                // Cancel the original deletion. Honored by
                // `delete_permanent_with_cause`'s `Cancelled` arm.
                rctx.cancel();
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
                let prov = resume_provenance(&rctx.effect);
                rctx.effect.select_count_capped_multi(
                    controller,
                    CountCappedZone::Material(subject),
                    n,
                    "trash N digivolution cards",
                    /*is_optional_zero=*/ false,
                    /*distinct_by=*/ None,
                    |_g, _src| true,
                    move |ctx, picks| {
                        // Trash each picked source from the carrier's stack
                        // into the controller's trash via the EffectContext
                        // primitive (stays within the API boundary).
                        for handle in picks {
                            // Soft-fail bool discarded — picks are validated
                            // by `select_count_capped_multi` upstream and the
                            // `<Fragment>` flow is single-carrier (`subject`),
                            // so a stale handle here would be an engine bug,
                            // not a rules-natural fizzle. Discard for parity
                            // with the new `trash_card_source` signature.
                            let _ = ctx.trash_card_source(subject, handle);
                        }
                        // Cancel the original deletion — carrier survives
                        // with its remaining sources + top.
                        ctx.cancel_leave();
                    },
                );
                park_keyword_count_capped_frame(
                    &mut rctx.effect,
                    prov,
                    controller,
                    CountCappedZone::Material(subject),
                    0,
                    n,
                    false,
                    NonDslCountCappedTerminal::KeywordFragment { subject },
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
            // Self-scope UPSTREAM: "when THIS Digimon would be deleted" —
            // drop the candidate when the deletion subject is a different
            // permanent, so the outer accept dialog never parks on a
            // neighbor's deletion (judge-quiz Q24 surfaced the phantom
            // prompt: Rapidmon X's <Armor Purge> offered on the opponent's
            // Tentomon dying to the 0-DP rules check).
            .replacement_condition(|ctx, subject| {
                let Some(me) = ctx.source_permanent else {
                    return false;
                };
                matches!(subject, ReplacementSubject::Permanent(h) if *h == me)
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
        // **Post-batched-refactor (2026-05-23):** Save fires AFTER the
        // carrier's top card has moved to trash (DCGO parity:
        // `IsTopCardInTrashOnDeletion` predicate). The handler reads
        // `self_card` from the snapshot's `top_card` field, then uses
        // `place_card_under_permanent_bottom` whose zone-walker locates
        // the card in trash and moves it under the chosen Tamer.
        //
        // No live `battle_area.get(subject.index)` lookup — the carrier
        // is gone from the field by the time OnDeletion handlers fire.
        Keyword::Save => vec![Effect::on_deletion(card)
            .name("<Save>")
            .process(|ctx| {
                // Read the carrier's identity from the snapshot, not from
                // the field. `deleted_object` is threaded in by
                // `delete_permanents_batch`'s OnDeletion enqueue stage.
                let Some(snap) = ctx.deleted_object_snapshot().cloned() else {
                    // Defensive — batched OnDeletion always carries a snapshot.
                    return;
                };
                let owner = snap.former_controller;
                let self_card = snap.top_card;
                let prov = resume_provenance(ctx);

                // Park the optional Tamer-pick. `select_own_permanent`
                // no-ops silently with no parking when the candidate
                // filter yields zero matches (no own Tamers).
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
                        // Retrieve `self_card` from trash via the
                        // zone-walking helper and tuck it under the
                        // chosen Tamer's stack.
                        ctx.place_card_under_permanent_bottom(self_card, tamer, false);
                    },
                );
                park_keyword_frame(
                    ctx,
                    ResumeFrame::KeywordSaveSelection(KeywordSaveSelectionState {
                        prov,
                        owner,
                        self_card,
                        outer_conts: Vec::new(),
                    }),
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
        // would reject (subject==self, cross-controller, non-Digimon, or
        // color-mismatch under a non-zero color_mask), the dialog still
        // appears. On accept, the body falls through without setting an
        // outcome and the original deletion proceeds. This matches the
        // Phase C `nested_select_decoy.rs` precedent.
        //
        // Color filter (Track G close): `Keyword::Decoy(u8)` carries a
        // CardColor bitmask. `0` = no filter (un-parameterized printed form
        // and prior behavior). Non-zero filters narrow eligible allies to
        // those whose `colors_for_rules` overlaps the bitmask. Trait-filter
        // forms (`<Decoy ([Bagra Army] trait)>`) parse to `Decoy(0)` and
        // require hand-rolled overrides for the trait gate (existing
        // precedent; trait id is not stored in the keyword variant to keep
        // `Keyword: Copy`).
        Keyword::Decoy(color_mask) => vec![Effect::when_would_be_deleted(card)
            .name(
                if color_mask == 0 {
                    "<Decoy>".to_string()
                } else {
                    format!("<Decoy ({color_mask:#04x})>")
                }
                .as_str(),
            )
            .optional()
            .replacement_process(move |rctx| {
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
                // Color-filter gate: when `color_mask != 0`, the subject's
                // rules-facing colors must overlap the bitmask. Bit `n` set
                // ⇒ color index `n` (CardColor as u8) is eligible.
                if color_mask != 0 {
                    let subject_colors =
                        subject_perm.colors_for_rules(&game.card_data, &game.modifiers, subject);
                    let any_match = subject_colors
                        .iter()
                        .any(|c| (color_mask & (1u8 << (*c as u8))) != 0);
                    if !any_match {
                        return;
                    }
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
        // **Post-batched-refactor (2026-05-23):** Fortitude reads the
        // snapshot's `source_count_just_before` for the "≥1 source under
        // the top" gate and plays `self_card` from trash directly inside
        // the OnDeletion drain. The legacy `pending_post_deletion_replays`
        // slot is no longer pushed — Phase 5 will retire the slot.
        //
        // DCGO `Fortitude.cs:14-63`: `CanActivateFortitude` requires
        // `IsExistOnTrash(card)` — the card must be in trash at fire time.
        // Under the batched flow, that's automatically true: the
        // OnDeletion drain runs post-trash.
        Keyword::Fortitude => vec![Effect::on_deletion(card)
            .name("<Fortitude>")
            .process(|ctx| {
                let Some(snap) = ctx.deleted_object_snapshot().cloned() else {
                    return;
                };
                // Gate: ≥1 source UNDER the top — i.e. snapshot's
                // pre-removal `source_count_just_before >= 1`. (The
                // snapshot records sources *under* the top, so 1 source
                // under means stack length 2.)
                if snap.source_count_just_before < 1 {
                    return;
                }
                // Play `self_card` from trash, free + unsuspended.
                let _ = ctx.play_from_trash_free_unsuspended(snap.top_card);
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
        // **Post-batched-refactor (2026-05-23):** Partition reads the
        // snapshot's `digisources_just_before` (which are now in the
        // controller's trash) and picks 2 to play from trash inline
        // during the OnDeletion drain. No more `pending_post_deletion_replays`
        // push — the replay happens here.
        //
        // DCGO `Partition.cs:9-23, 71-162`: fires post-trash, walks the
        // permanent's pre-removal `cardSources` (preserved via
        // `PermanentJustBeforeRemoveField`) — we mirror via the snapshot.
        // ## INTERRUPTIVE re-timing (judge-quiz Q30, 2026-06-11)
        //
        // The printed reminder text reads "When this Digimon ... WOULD LEAVE
        // the battle area...", and the judge ruling (quiz Q30 feedback) is
        // explicit: "<Partition> is an interruptive effect which activates
        // BEFORE Chaosmon: Valdur Arm is deleted" — the carrier is still in
        // the battle area while the partition plays (and their would-play
        // interrupts, e.g. MedievalGallantmon's suspend-2 cost reduction)
        // resolve, so the carrier itself is a legal target for those
        // interrupts. The earlier post-trash `OnDeletion` model (copied from
        // DCGO `Partition.cs`) was unfaithful to both.
        //
        // Shape: an OPTIONAL, NON-CANCELLING `WhenWouldLeaveBattleArea`
        // replacement (the Fragment/Scapegoat family):
        //   - accept dialog = the printed "you may" ("chooses to activate
        //     <Partition>" in the quiz);
        //   - cause filter unchanged (skips Battle | OwnEffect);
        //   - mandatory 2-source pick from the carrier's LIVE stack
        //     (`CountCappedZone::Material`);
        //   - both picks are extracted from the stack FIRST (silently — the
        //     cards are PLAYED, not trashed, so no on-trash event fires;
        //     they only transit the trash zone), so neither is in the battle
        //     area while the other's would-play interrupts resolve (the
        //     judge's "played out simultaneously" observable);
        //   - the second play is chained via `run_after_selections_drain` so
        //     it starts only after the first play's interrupt chain settles;
        //   - NO `cancel_leave()` — the leave proceeds with the remaining
        //     stack (interruptive, not preventive).
        Keyword::Partition => vec![Effect::when_would_leave_battle_area(card)
            .name("<Partition>")
            .optional()
            .replacement_condition(|ctx, subject| {
                use crate::replacement::{ReplacementCause, ReplacementSubject};
                // Cause filter: skip Battle and same-controller (OwnEffect).
                if matches!(
                    ctx.replacement_cause(),
                    Some(ReplacementCause::Battle | ReplacementCause::OwnEffect)
                ) {
                    return false;
                }
                // Self-scope: only the carrier's own departure.
                let ReplacementSubject::Permanent(subject) = subject else {
                    return false;
                };
                if ctx.source_permanent != Some(*subject) {
                    return false;
                }
                // Gate: >=2 sources under the top card, read off the LIVE
                // stack (the carrier has not left yet).
                ctx.game
                    .player(subject.player)
                    .battle_area
                    .get(subject.index as usize)
                    .is_some_and(|p| p.card_sources.len() >= 3)
            })
            .replacement_process(|rctx| {
                use crate::replacement::ReplacementSubject;

                let me_perm = rctx.effect.source_permanent;
                let subject = match rctx.subject {
                    ReplacementSubject::Permanent(h) => h,
                    _ => return,
                };
                if Some(subject) != me_perm {
                    return;
                }
                // Re-check the gate at process time (an earlier replacement
                // in the chain could have shrunk the stack).
                let stack_len = rctx
                    .effect
                    .game
                    .player(subject.player)
                    .battle_area
                    .get(subject.index as usize)
                    .map(|p| p.card_sources.len())
                    .unwrap_or(0);
                if stack_len < 3 {
                    return;
                }

                let controller = subject.player;
                let prov = resume_provenance(&rctx.effect);
                rctx.effect.select_count_capped_multi(
                    controller,
                    CountCappedZone::Material(subject),
                    /*max=*/ 2,
                    "select 2 cards to play",
                    /*is_optional_zero=*/ false,
                    /*distinct_by=*/ None,
                    |_g, _src| true,
                    move |ctx, picks| {
                        if picks.len() != 2 {
                            return;
                        }
                        // Extract BOTH picks from the live stack first —
                        // silently (these cards are played, not trashed;
                        // no on-trash event). With both out of the stack,
                        // neither is on the field while the other's
                        // would-play interrupts resolve.
                        let mut extracted: Vec<crate::card_source::CardHandle> = Vec::new();
                        for handle in picks {
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
                                    .position(|c| c.handle() == handle)
                                else {
                                    continue;
                                };
                                permanent.card_sources.remove(pos)
                            };
                            let owner = removed.owner;
                            ctx.game.player_mut(owner).trash.push(removed);
                            extracted.push(handle);
                        }
                        // Play sequentially: the second play starts only
                        // after the first play's interrupt chain (if any)
                        // fully resolves.
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
                        // No cancel_leave(): the carrier's departure
                        // proceeds with the remaining stack.
                    },
                );
                park_keyword_count_capped_frame(
                    &mut rctx.effect,
                    prov,
                    controller,
                    CountCappedZone::Material(subject),
                    0,
                    2,
                    false,
                    NonDslCountCappedTerminal::KeywordPartition { subject },
                );
            })
            .build()],

        // Xros Heart change 4.x — printed MaterialSave(N) is no longer a
        // `[Main]` activation. It is an optional deletion-timed source rescue
        // that uses the pre-removal snapshot because the carrier has already
        // moved to trash by the time OnDeletion handlers drain.
        Keyword::MaterialSave(n) => vec![Effect::on_deletion(card)
            .name(&format!("<Material Save {n}>"))
            .process(move |ctx| {
                let Some(snapshot) = ctx.deleted_object_snapshot().cloned() else {
                    return;
                };
                let owner = snapshot.former_controller;
                let carrier = snapshot.top_card;
                let eligible_sources = snapshot
                    .digisources_just_before
                    .iter()
                    .copied()
                    .filter(|source| ctx.game.card_matches_digixros_recipe(carrier, *source))
                    .collect::<Vec<_>>();
                if eligible_sources.is_empty() {
                    return;
                }
                if !ctx
                    .game
                    .player(owner)
                    .battle_area
                    .iter()
                    .any(|permanent| permanent.is_tamer(&ctx.game.card_data))
                {
                    return;
                }
                let prov = resume_provenance(ctx);
                let eligible_sources_for_frame = eligible_sources.clone();

                ctx.select_own_permanent(
                    "you may place Material Save sources under one of your Tamers",
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
                        let eligible_sources = eligible_sources;
                        ctx.select_snapshot_digisources_from_trash(
                            owner,
                            eligible_sources.clone(),
                            n,
                            "select Material Save sources to place under Tamer",
                            /*is_optional_zero=*/ true,
                            move |_g, source| eligible_sources.contains(&source),
                            move |ctx, picks| {
                                for source in picks {
                                    ctx.place_card_under_permanent_bottom(source, tamer, false);
                                }
                            },
                        );
                    },
                );
                park_keyword_frame(
                    ctx,
                    ResumeFrame::KeywordMaterialSaveTamerSelection(
                        KeywordMaterialSaveTamerSelectionState {
                            prov,
                            owner,
                            eligible_sources: eligible_sources_for_frame,
                            max: n,
                            outer_conts: Vec::new(),
                        },
                    ),
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
        // of your **Digimon** to prevent it." DCGO `Scapegoat.cs`.
        // RULES_CONTEXT 16-31 (Immediate-type, Optional).
        //
        // The substitute MUST be a Digimon — Tamers are not valid
        // substitutes per printed text. Both the upstream candidate-
        // existence gate and the inner pick filter consult
        // `Permanent::is_tamer(&card_data)` to exclude Tamer permanents.
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
        // own **Digimon** to substitute. Suppressed at candidate-collection
        // time so the outer dialog never parks in this case. Tamers are
        // excluded per printed text ("another of your **Digimon**" —
        // RULES_CONTEXT 16-31).
        //
        // ## Selection chain
        //
        //   1. Outer optional accept dialog ("may"). PASS leaves the
        //      original deletion to proceed.
        //   2. On ACCEPT: parked own-permanent pick via
        //      `rctx.effect.select_own_permanent(...)`. Filter: same-
        //      controller, non-self, non-Tamer. Mandatory once accepted
        //      (DCGO: once committed to substitute, must pick).
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
            .replacement_condition(|ctx, _subject| {
                use crate::replacement::ReplacementCause;
                // RULES_CONTEXT 16-31: skip own-effect deletions.
                if matches!(ctx.replacement_cause(), Some(ReplacementCause::OwnEffect)) {
                    return false;
                }
                // DCGO HasMatchConditionPermanent: at least one other own
                // **Digimon** must exist as a substitute candidate.
                // Printed text restricts the substitute to "another of your
                // Digimon" — Tamers are not valid candidates
                // (RULES_CONTEXT 16-31).
                let Some(me) = ctx.source_permanent else {
                    return false;
                };
                let owner = me.player;
                let battle = ctx.battle_area(owner);
                battle
                    .iter()
                    .enumerate()
                    .any(|(i, p)| i as u8 != me.index && !p.is_tamer(&ctx.game.card_data))
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

                // Inner pick: another of own **Digimon**. Filter: same-
                // controller, non-self, non-Tamer (printed text:
                // "another of your Digimon" — RULES_CONTEXT 16-31).
                // Mandatory once accepted (is_optional=false). The
                // upstream `replacement_condition` already guarantees at
                // least one Digimon candidate exists, so this
                // select_own_permanent will always install a
                // pending_selection.
                let prov = resume_provenance(&rctx.effect);
                rctx.effect.select_own_permanent(
                    "select another of your Digimon to delete instead",
                    /*is_optional=*/ false,
                    move |g, h| {
                        if h.player != owner || h == me_perm {
                            return false;
                        }
                        g.players[h.player as usize]
                            .battle_area
                            .get(h.index as usize)
                            .is_some_and(|p| !p.is_tamer(&g.card_data))
                    },
                    move |ctx, picked| {
                        // Substitute the deletion subject to the picked
                        // permanent. The dispatcher's Substituted commit
                        // arm finalizes the redirected deletion.
                        ctx.substitute_replacement(ReplacementSubject::Permanent(picked));
                    },
                );
                park_keyword_frame(
                    &mut rctx.effect,
                    ResumeFrame::KeywordScapegoatSelection(KeywordScapegoatSelectionState {
                        prov,
                        owner,
                        self_perm: me_perm,
                        outer_conts: Vec::new(),
                    }),
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
                        ModifierEntry::simple(ModifierType::MayAttack, 1, Expiry::EndOfTurn, owner),
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
                    ctx.game
                        .delete_permanent_with_cause(me, ReplacementCause::OwnEffect);
                })
                .build(),
        ],

        // EX12 — printed Engage: "At the end of your turn, this Digimon may
        // attack." Official rules 16-44. This is intentionally narrower than
        // Execute/Vortex:
        //   - normal attack legality (`game.can_attack(..., false)`) controls
        //     whether `has_end_of_turn_keywords` parks the phase;
        //   - no `CanAttackUnsuspended` grant, so the action mask offers only
        //     normal targets;
        //   - no `EndOfAttack` self-delete observer.
        //
        // Optionality follows the same practical shape as Execute: the EOT
        // trigger grants the temporary attack permission, and PASS in
        // EndOfTurnAction is the player-facing decline path.
        Keyword::Engage => vec![Effect::end_of_your_turn(card)
            .name("<Engage>")
            .process(|ctx| {
                let Some(me) = ctx.source_permanent else {
                    return;
                };
                let owner = me.player;
                ctx.game.modifiers.add(
                    me,
                    ModifierEntry::simple(ModifierType::MayAttack, 1, Expiry::EndOfTurn, owner),
                );
            })
            .build()],

        // EX12 — printed Guard: "When another of your Digimon would leave the
        // battle area by an opponent's effect, by deleting this Digimon,
        // prevent that Digimon from leaving." Official rules 16-45. Model as
        // an optional replacement on the leave-field route:
        //   - opponent-effect cause only;
        //   - own Digimon subject only;
        //   - non-self subject ("another");
        //   - process pays the delete-self cost, then cancels the original
        //     leave event.
        Keyword::Guard => vec![Effect::when_would_leave_battle_area(card)
            .name("<Guard>")
            .optional()
            .replacement_condition(|ctx, subject| {
                use crate::replacement::{ReplacementCause, ReplacementSubject};
                if !matches!(
                    ctx.replacement_cause(),
                    Some(ReplacementCause::OpponentEffect)
                ) {
                    return false;
                }
                let Some(me) = ctx.source_permanent else {
                    return false;
                };
                let ReplacementSubject::Permanent(subject) = subject else {
                    return false;
                };
                if *subject == me || subject.player != me.player {
                    return false;
                }
                ctx.game.permanent_is_digimon_for_rules(*subject)
            })
            .replacement_process(|rctx| {
                use crate::replacement::{ReplacementCause, ReplacementSubject};
                let Some(me) = rctx.effect.source_permanent else {
                    return;
                };
                let subject = match rctx.subject {
                    ReplacementSubject::Permanent(h) => h,
                    _ => return,
                };
                if subject == me || subject.player != me.player {
                    return;
                }
                if !rctx.effect.game.permanent_is_digimon_for_rules(subject) {
                    return;
                }
                rctx.effect
                    .game
                    .delete_permanent_with_cause(me, ReplacementCause::OwnEffect);
                rctx.cancel();
            })
            .build()],

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
                let prov = resume_provenance(ctx);

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
                park_keyword_frame(
                    ctx,
                    ResumeFrame::KeywordMindLinkSelection(KeywordMindLinkSelectionState {
                        prov,
                        owner,
                        tamer: me,
                        outer_conts: Vec::new(),
                    }),
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
                let Some(handle) = ctx.source_permanent else {
                    return false;
                };
                let Some(perm) = ctx.source_permanent() else {
                    return false;
                };
                // Training's self-suspend is a cost — a `CannotSuspend`
                // carrier can't pay it (DCGO `Training.cs:23` checks
                // `thisPermanent.CanSuspend`). Battle-area path only: the
                // breeding dispatcher skips `condition` (see
                // `activate_breeding_main_training`).
                !perm.is_suspended
                    && !ctx
                        .game
                        .modifiers
                        .has(handle, crate::enums::ModifierType::CannotSuspend)
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

        // DCGO `CardEffectCommons/KeyWordEffects/Ascension.cs` — OnDeletion
        // trigger (`CanTriggerOnDeletion`, no cause filter). The carrier has
        // already moved to trash when this drains (rule 25), so it is read from
        // the pre-removal snapshot and rescued from trash to the TOP of the
        // owner's security stack, face-down. "You may" is a real Yes/No branch
        // (no auto-resolution) — declining ("No") is a clean no-op.
        Keyword::Ascension => vec![Effect::on_deletion(card)
            .name("<Ascension>")
            .process(|ctx| {
                let Some(snap) = ctx.deleted_object_snapshot().cloned() else {
                    return;
                };
                let owner = snap.former_controller;
                let self_card = snap.top_card;
                let prov = resume_provenance(ctx);
                // Only offer if the carrier actually landed in the owner's
                // trash (always true under the batched deletion flow).
                if !ctx
                    .game
                    .player(owner)
                    .trash
                    .iter()
                    .any(|c| c.handle() == self_card)
                {
                    return;
                }
                ctx.select_effect_choice(
                    "you may place this card as the top security card",
                    vec!["Yes".to_string(), "No".to_string()],
                    move |ctx, choice| {
                        if choice != 0 {
                            return; // declined ("No")
                        }
                        // Re-locate by stable handle (robust to intervening
                        // trash shifts), then place on security top face-down.
                        // Routes through `place_on_security_observed`, so
                        // `WhenWouldPlaceInSecurity` replacements and
                        // `CannotAddSecurity` gates apply.
                        let Some(idx) = ctx
                            .game
                            .player(owner)
                            .trash
                            .iter()
                            .position(|c| c.handle() == self_card)
                        else {
                            return;
                        };
                        ctx.place_on_security(
                            owner,
                            crate::enums::CardSourceRef::Trash(owner, idx),
                            crate::enums::StackPosition::Top,
                            /*face_up=*/ false,
                        );
                    },
                );
                park_keyword_frame(
                    ctx,
                    ResumeFrame::KeywordAscensionChoice(KeywordAscensionChoiceState {
                        prov,
                        owner,
                        self_card,
                        outer_conts: Vec::new(),
                    }),
                );
            })
            .build()],

        // Intentionally NOT auto-installed (see file-header docstring for
        // rationale). The printed `<Digi-Burst N>` token is a cost prefix for
        // a per-card body that DSL `digi_burst: { count: N, then: [...] }`
        // expresses inline. Synthesizing the cost without a body here would
        // be strictly worse for the player. Cards using the printed keyword
        // must author the `[Main]` body via DSL or hand-rolled `CardEffect`.
        Keyword::DigiBurst(_) => Vec::new(),

        // Non-replacement keywords — handled elsewhere (combat, mask, etc.).
        _ => Vec::new(),
    }
}
