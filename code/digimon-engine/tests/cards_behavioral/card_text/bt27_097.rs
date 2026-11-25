//! Source: `data/card_bundles/BT27-097.md`.
//!
//! ## Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! ### Standard digivolve cost circles
//!   - (none — DigiEgg / Lv.2 base, or DNA/special only)
//!
//! ## Printed text (official)
//! ### Effect
//! [Start of Your Turn] If you have 2 or less memory, set it to 3. [All Turns] When a Digimon attacks, by suspending this Tamer, you may place the top 2 cards of your deck face down under this Tamer. After, if its your turn, by trashing 4 bottom face-down cards from under any of your Tamers, you may use 1 purple [BEATBREAK] trait Option card from your trash with the cost reduced by 3.
//!
//! ### Security Effect
//! [Security] Play this card without paying the cost.
//!
//! ## Official Q&A
//! - No, you can't. If you don't suspend this card, you can't process the part after "after" in its [All Turns] effect.

use super::common::reina_and_makoto_can_stash_two_or_suspend_without_stashing;

#[test]
fn printed_reina_and_makoto_can_stash_two_or_suspend_without_stashing() {
    reina_and_makoto_can_stash_two_or_suspend_without_stashing("BT27-097");
}

#[test]
fn security_plays_the_tamer_without_cost() {
    super::common::security_tamer_play("BT27-097");
}

#[test]
fn start_of_turn_sets_low_memory_to_three() {
    super::common::memory_at_start_of_turn("BT27-097");
}

#[test]
fn own_turn_option_payment_uses_only_a_purple_beatbreak_option_from_trash_at_minus_three() {
    super::common::tamer_option_payment("BT27-097", true, "purple");
}
