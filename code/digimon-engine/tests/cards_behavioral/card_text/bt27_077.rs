//! Source: `data/card_bundles/BT27-077.md`.
//!
//! ## Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! ### Standard digivolve cost circles
//!   - Purple Lv.3 / cost 2
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] Lv.3 w/[Glowing Dawn] trait: Cost 2
//!
//! ## Printed text (official)
//! ### Effect
//! [Security] [When Digivolving] If you have 1 or fewer Tamers, you may play 1 Tamer card with [Makoto Kuonji] in its name from your hand without paying the cost.
//!
//! ### Inherited Effect
//! ＜Retaliation＞ (When only this Digimon is deleted in battle, delete the Digimon it battled.)
//!
//! ## Official Q&A
//! - Yes, its [Security] effect activates, then it battles.

use super::common::champions_offer_only_the_matching_tamer_on_digivolve_and_can_decline;

#[test]
fn printed_champions_offer_only_the_matching_tamer_on_digivolve_and_can_decline() {
    champions_offer_only_the_matching_tamer_on_digivolve_and_can_decline(
        "BT27-077",
        "Makoto Kuonji",
    );
}

#[test]
fn printed_standard_and_glowing_dawn_routes_charge_their_listed_costs() {
    use digimon_engine::enums::CardColor::Purple;
    super::common::digivolve_paths("BT27-077", 3, &[(Purple, 2)], 2);
}

#[test]
fn security_plays_the_matching_tamer_before_battle() {
    super::common::champion_security_precedes_battle("BT27-077", "Makoto Kuonji");
}

#[test]
fn retaliation_is_inherited_by_the_evolved_digimon() {
    use super::common::{builder, start};
    use digimon_engine::enums::Keyword;
    let mut r = start(builder("BT27-077"));
    let inherited = r.place_stack(0, &["BT27-077", "TEXT-5"]);
    let bare = r.place_on_field(0, "BT27-077", Some(0));
    assert!(r.game.has_keyword(inherited, Keyword::Retaliation));
    assert!(!r.game.has_keyword(bare, Keyword::Retaliation));
}
