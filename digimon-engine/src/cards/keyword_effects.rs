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
//! All four keywords produce **optional** replacements per printed rules
//! ("you may"). Declining the optional selection leaves the original event
//! (deletion / return-to-deck) to proceed normally.
//!
//! ## Deferred: Partition / ArmorPurge
//!
//! Printed `<Partition>` and `<Armor Purge>` require the replacement process
//! to install a nested `PendingSelection::Source` so the player can pick
//! which source in the permanent's digivolution stack to substitute/trash.
//! The v1 replacement framework does not yet support nested selections
//! inside a replacement process (the `rctx` borrow does not survive the
//! callback boundary). Their enum variants (`Keyword::Partition`,
//! `Keyword::ArmorPurge`) are parsed by `parse_printed_keywords` but this
//! module intentionally returns `None` for them — a hand-authored
//! `CardEffect` can still cover any such cards in the interim.
//!
//! TODO(phase-7-followup): Once nested-selection-inside-replacement lands,
//! map Partition → "pick a source → substitute(Permanent(source_as_handle))"
//! and ArmorPurge → same, but trash the picked source rather than delete it.

use crate::card_source::CardHandle;
use crate::effect::Effect;
use crate::enums::{Keyword, Zone};
use crate::replacement::ReplacementSubject;

/// Map a printed keyword to a synthesized `Effect` that installs the
/// matching `WhenWouldBe*` replacement. Returns `None` for keywords that
/// do not carry replacement semantics (or that are explicitly deferred per
/// the module docstring).
pub fn keyword_to_auto_effect(keyword: Keyword, card: CardHandle) -> Option<Effect> {
    match keyword {
        // Printed Barrier: "When this Digimon would be deleted, you may trash
        // the top card of your deck. If you do, it isn't deleted."
        Keyword::Barrier => Some(
            Effect::when_would_be_deleted(card)
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
                .build(),
        ),

        // Printed Evade: "When this Digimon would be deleted, you may place
        // it at the bottom of your deck instead."
        // Redirecting to Zone::Deck is honored by
        // `delete_permanent_with_cause`'s `Redirected(Zone::Deck)` arm,
        // which routes through `return_to_deck(StackPosition::Bottom)`.
        Keyword::Evade => Some(
            Effect::when_would_be_deleted(card)
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
                .build(),
        ),

        // Printed Fragment(N): "When this Digimon would be deleted, you may
        // trash any N of its digivolution cards. If you do, it isn't deleted."
        //
        // v1 simplification: trash from the top of the owner's deck instead
        // of prompting for digivolution-card selection. This preserves the
        // spirit of "pay a cost to survive" while deferring the source-pick
        // UI plumbing to the Partition/ArmorPurge follow-up.
        // TODO(phase-7-followup): prompt for N sources from the permanent's
        // digivolution stack and trash those, matching printed rules exactly.
        Keyword::Fragment(n) => Some(
            Effect::when_would_be_deleted(card)
                .name("<Fragment>")
                .optional()
                .replacement_process(move |rctx| {
                    let me = rctx.effect.source_permanent;
                    if let ReplacementSubject::Permanent(subject) = rctx.subject {
                        if Some(subject) != me {
                            return;
                        }
                        let owner = subject.player;
                        let game = &mut *rctx.effect.game;
                        for _ in 0..n {
                            let Some(top) = game.players[owner as usize].deck.pop() else {
                                break;
                            };
                            game.players[owner as usize].trash.push(top);
                        }
                        rctx.handled();
                    }
                })
                .build(),
        ),

        // Printed Decode: "When this Digimon would be returned to your
        // opponent's deck/hand, you may return it to your hand instead."
        //
        // The deck route is the interesting one — it's a redirect from deck
        // to hand. For the hand route (WhenWouldBeReturnedToHand), redirect
        // to Zone::Hand is effectively a no-op; we register a single effect
        // on the deck timing here since that covers the non-trivial case.
        // A card that needs both timings can still hand-author the second
        // effect if a future printed variant differentiates them.
        Keyword::Decode => Some(
            Effect::when_would_be_returned_to_deck(card)
                .name("<Decode>")
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
        ),

        // Intentionally not auto-installed (see module docstring).
        Keyword::Partition | Keyword::ArmorPurge => None,

        // Non-replacement keywords — handled elsewhere (combat, mask, etc.).
        _ => None,
    }
}
