//! Source: `data/card_bundles/BT27-055.md` (printed text and official Q&A).
//!
//! Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! Standard digivolve cost circles
//! - Green Lv.5 / cost 4
//! - Purple Lv.5 / cost 4
//! Special digivolution condition (DNA / alt-digivolve black text)
//! [Digivolve] [Quetzalmon] w/[Bio] trait: Cost 5 [Digivolve] Tamer w/[Kurata's Army] trait: Cost 7
//!
//! Printed text (official)
//! Effect
//! ＜Piercing＞ ＜Vortex＞ ＜Armor Purge＞ [When Digivolving] [When Attacking] [Once Per Turn] Suspend up to 2 of your opponent's Digimon or Tamers. Then, 1 of their Digimon or Tamers can't unsuspend until their turn ends. [All Turns] [Once Per Turn] When any Digimon suspend, this Digimon may unsuspend. [All Turns] This Digimon can't digivolve.
//!
//! Official Q&A
//! - No. Digivolve from the Tamer as such, and do not treat it as if it is a digivolving Digimon. "When digivolving" effects and "when a Digimon digivolves" effects don't trigger. Additionally, if a "Digimon can't digivolve" effect has activated, you can still digivolve from a Tamer with this digivolve requirement.

use super::bt27_common::*;
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::debug_runner::make_test_card;
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

#[test]
#[ignore = "G-ENGINE-TAMER-BASE-AS-TAMER"]
fn tamer_evolution_does_not_trigger_when_digivolving() {
    tamer_evolution(
        "BT27-055",
        7,
        &[Keyword::Piercing, Keyword::Vortex, Keyword::ArmorPurge],
    );
}

#[test]
fn bio_quetzalmon_evolves_for_five_and_locks_the_chosen_tamer() {
    let mut quetzalmon = make_test_card("TEXT-QUETZALMON", "Quetzalmon");
    quetzalmon.level = Some(4);
    quetzalmon.traits = vec!["Bio".into()];
    let mut r = start(
        builder("BT27-055")
            .add_card(quetzalmon)
            .hand(0, &["BT27-055"]),
    );
    let base = r.place_on_field(0, "TEXT-QUETZALMON", Some(0));
    r.place_on_field(1, TAMER, Some(0));
    let before = r.memory();
    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    assert_eq!(before - r.memory(), 5);
    let kind = r.pending_selection().unwrap().kind.clone();
    assert!(matches!(kind, SelectionKind::CountCappedMultiSelect { .. }));
    r.assert_prompt(kind, true, 0);
    assert_eq!(candidates(&r), [TAMER]);
    r.decline_prompt().unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), [TAMER]);
    r.choose_prompt_card(TAMER, None).unwrap();
    assert!(r.game.modifiers.has(
        digimon_engine::permanent::PermanentHandle {
            player: 1,
            index: 0
        },
        digimon_engine::enums::ModifierType::CannotUnsuspend,
    ));
    assert!(r.pending_selection().is_none());
}
