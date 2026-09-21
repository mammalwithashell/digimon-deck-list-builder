//! Shared "declare this hand card" helper for the Option-flow suites.
//!
//! `G-ENGINE-OPTION-LINK-FROM-HAND` (2026-09-20) split ONE engine entry point
//! into two main-phase actions, because `general_rule.pdf` (Ver.3.6) §6-5-1
//! lists them separately: **§6-5-1-3 "Use an Option Card From the Hand"** on
//! the `PLAY_HAND` bit, and **§6-5-1-4 "Linking a Card in the Hand or Battle
//! Area"** on the `HAND_EFFECT` bit. §10-1-1: a card is linked from the hand
//! "by paying the cost as part of the main phase actions" — its own
//! declaration, not a play mode. DCGO agreed all along
//! (`CardEffectFactory.LinkEffect`, `Link.cs:19`, reached as an
//! `ActivateCardAction`, never a `PlayCardAction`).
//!
//! So `Game::play_option_from_hand` no longer plugs a Plug-In Option in; for a
//! link-only Option it now (correctly) reports `Invalid`, because that card has
//! no legal §6-5-1-3 mode at all. These suites are about the LINK LIFECYCLE,
//! not about which bit carries it, so they route through this helper: it asks
//! the engine's own published predicate which decision the hand slot's
//! `HAND_EFFECT` bit is (`Game::hand_effect_slot_is_link` — the same one the
//! mask, `explain_action` and the exam's lowering read) and takes that bit when
//! it is the link, otherwise plays the card as before.
//!
//! The `OptionPlayResult` is reconstructed from the post-decode state, because
//! `decode_action` returns unit. Only the three outcomes a link declaration can
//! reach are distinguished; anything else is `Invalid`, which fails loudly.

use digimon_engine::action::space::HAND_EFFECT_START;
use digimon_engine::selection::OptionPlayResult;
use digimon_engine::Game;
use digimon_engine::PlayerId;

/// Declare the card at `player`'s hand slot `hand_index`: the §6-5-1-4 link if
/// that is what the slot's `HAND_EFFECT` bit means right now, else the
/// §6-5-1-3 Option use.
pub fn declare_from_hand(
    game: &mut Game,
    player: PlayerId,
    hand_index: usize,
) -> OptionPlayResult {
    if !game.hand_effect_slot_is_link(player, hand_index) {
        return game.play_option_from_hand(player, hand_index);
    }
    let card_id = game.player(player).hand[hand_index]
        .card_id(&game.card_data)
        .to_string();
    game.decode_action(HAND_EFFECT_START + hand_index as u16, player);

    if game.pending_selection.is_some() {
        return OptionPlayResult::Pending;
    }
    // Attached: the card is a linked card on one of the controller's
    // permanents. `Linked { source }` names the permanent it plugged into.
    for (i, perm) in game.player(player).battle_area.iter().enumerate() {
        if perm
            .linked_cards
            .iter()
            .any(|c| c.card_id(&game.card_data) == card_id)
        {
            return OptionPlayResult::Linked {
                source: digimon_engine::permanent::PermanentHandle {
                    player,
                    index: i as u8,
                },
            };
        }
    }
    if game
        .player(player)
        .trash
        .iter()
        .any(|c| c.card_id(&game.card_data) == card_id)
    {
        return OptionPlayResult::Trashed;
    }
    OptionPlayResult::Invalid
}
