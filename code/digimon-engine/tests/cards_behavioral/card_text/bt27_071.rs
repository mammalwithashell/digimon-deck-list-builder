//! Source: `data/card_bundles/BT27-071.md` (printed text and official Q&A).
//!
//! Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! Standard digivolve cost circles
//! - Black Lv.5 / cost 4
//! - Purple Lv.5 / cost 4
//! Special digivolution condition (DNA / alt-digivolve black text)
//! [Digivolve] [Thunderbirdmon] w/[Bio] trait: Cost 5 [Digivolve] Tamer w/[Kurata's Army] trait: Cost 7
//!
//! Printed text (official)
//! Effect
//! ＜Reboot＞ ＜Blocker＞ ＜Armor Purge＞ [When Digivolving] Your opponent's Digimon effects don't affect this Digimon until their turn ends. [When Attacking] [Counter] [Once Per Turn] ＜De-Digivolve 2＞ 1 of your opponent's Digimon. [All Turns] This Digimon can't digivolve.
//!
//! Official Q&A
//! - No. Digivolve from the Tamer as such, and do not treat it as if it is a digivolving Digimon. "When digivolving" effects and "when a Digimon digivolves" effects don't trigger. Additionally, if a "Digimon can't digivolve" effect has activated, you can still digivolve from a Tamer with this digivolve requirement.

use super::bt27_common::*;
use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

#[test]
#[ignore = "G-ENGINE-TAMER-BASE-AS-TAMER"]
fn tamer_evolution_does_not_trigger_when_digivolving() {
    tamer_evolution(
        "BT27-071",
        7,
        &[Keyword::Reboot, Keyword::Blocker, Keyword::ArmorPurge],
    );
}

#[test]
fn when_attacking_de_digivolves_an_opposing_stack_to_level_three() {
    let mut r = start(builder("BT27-071"));
    let attacker = r.place_on_field(0, "BT27-071", Some(0));
    let opponent = r.place_stack(1, &[BOUNDARY, EVOLUTION, EVOLUTION]);
    action(
        &mut r,
        0,
        encode_attack(attacker.index as u16, SECURITY_TARGET),
    );
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), [EVOLUTION]);
    r.choose_prompt_card(EVOLUTION, None).unwrap();
    let stack = &r.game.players[1].battle_area[opponent.index as usize];
    assert_eq!(stack.card_sources.len(), 1);
    assert_eq!(stack.top_card().card_id(&r.game.card_data), BOUNDARY);
    assert!(r.pending_selection().is_none());
}
