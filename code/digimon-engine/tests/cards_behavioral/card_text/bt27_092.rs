//! Source: `data/card_bundles/BT27-092.md` (printed text and official Q&A).
//!
//! Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! Standard digivolve cost circles
//! - (none — DigiEgg / Lv.2 base, or DNA/special only)
//!
//! Printed text (official)
//! Effect
//! [Security] Play this card without paying the cost. [Start of Your Main Phase] By placing 1 [Kurata's Army] trait card from your hand face down under this Tamer, ＜Draw 1＞ and gain 1 memory. [Main] [Once Per Turn] By trashing 2 bottom face-down cards from under any of your Tamers, this Tamer may digivolve into a [Kurata's Army] trait Digimon card in the hand or trash with the cost reduced by 2.
//!
//! Inherited Effect
//! [End of Attack] [Once Per Turn] By choosing 1 of your Digimon, delete it and 1 of your opponent's Digimon with as much DP as this Digimon or less.
//!
//! Official Q&A
//! - The card is placed on the bottom of the cards under the Tamer.

use super::bt27_common::*;

#[test]
fn security_plays_tamer_for_free() {
    security_plays_self("BT27-092");
}
