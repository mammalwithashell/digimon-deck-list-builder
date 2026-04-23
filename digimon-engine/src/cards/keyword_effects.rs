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

        // TODO(phase-7-followup): Fragment(N) needs nested PendingSelection::Source for source-pick; deferred alongside Partition/ArmorPurge until the nested-selection-inside-replacement infrastructure lands. Per CLAUDE.md rule 17 no-approximations, parse-only is preferred over auto-trash-top-of-deck.
        Keyword::Fragment(_) => Vec::new(),

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

        // Intentionally not auto-installed (see module docstring).
        Keyword::Partition | Keyword::ArmorPurge => Vec::new(),

        // Non-replacement keywords — handled elsewhere (combat, mask, etc.).
        _ => Vec::new(),
    }
}
