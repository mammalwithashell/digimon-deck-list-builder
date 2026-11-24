//! Source: `data/card_bundles/BT27-003.md` (printed text).
//!
//! ### Inherited Effect
//! [When Attacking] [Once Per Turn] This Digimon may digivolve into a Digimon card with [Leomon] in its name or the [ACCEL] trait in the hand.
//!
//! ## Official Q&A
//! - Yes, you pay the digivolution cost.

use super::common::*;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-003";

#[test]
fn can_decline_or_pay_to_digivolve_into_accel_while_attacking() {
    for accept in [false, true] {
        let mut evo = digimon("TEXT-EVO", 4, 6000, false);
        evo.traits.push("ACCEL".into());
        let mut r = start(
            builder(CARD)
                .add_card(evo)
                .add_card(digimon("WEAK", 3, 1000, false))
                .hand(0, &["TEXT-EVO", "TEXT-4"])
                .security(1, &["WEAK"]),
        );
        let carrier = r.place_stack(0, &[CARD, "TEXT-3"]);
        r.attack_player(carrier, 1, false);
        if !accept {
            r.decline_optional_trigger().unwrap();
            assert_eq!(r.memory(), 10);
            assert_eq!(r.dp_of(carrier), Some(2000));
            continue;
        }
        r.accept_optional_trigger().unwrap();
        r.assert_prompt(SelectionKind::Hand, false, 0);
        assert_eq!(candidates(&r), ["TEXT-EVO"]);
        r.choose_prompt_card("TEXT-EVO", None).unwrap();
        assert_eq!(r.memory(), 6);
        assert_eq!(r.dp_of(carrier), Some(6000));
    }
}
