//! Source: `data/card_bundles/BT27-049.md` (printed text).
//!
//! ### Effect
//! [When Attacking] This Digimon may digivolve into a Digimon card with [Leomon] in its name or the [ACCEL] trait in the hand. For each of your opponent's Digimon and Tamers, reduce this effect's paid cost by 1. (Rule) Name: Treated as including [Leomon].
//!
//! ### Inherited Effect
//! ＜Barrier＞ (When this Digimon would be deleted in battle, by trashing your top security card, it isn't deleted.)
//!
//! ## Official Q&A
//! - Yes, you pay the digivolution cost.

use super::common::*;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-049";

#[test]
fn attacking_evolution_discounts_each_opponent_digimon_and_tamer() {
    let mut evo = digimon("TEXT-EVO", 5, 7000, false);
    evo.card_name = "Leomon".into();
    let mut r = start(builder(CARD).add_card(evo).hand(0, &["TEXT-EVO", "TEXT-5"]));
    let carrier = r.place_on_field(0, CARD, Some(0));
    r.place_on_field(1, "TEXT-TAMER", Some(0));
    r.place_on_field(1, "TEXT-4", Some(0));
    r.attack_player(carrier, 1, false);
    r.assert_prompt(SelectionKind::Hand, true, 0);
    assert_eq!(candidates(&r), ["TEXT-EVO"]);
    r.choose_prompt_card("TEXT-EVO", None).unwrap();
    assert_eq!(r.dp_of(carrier), Some(7000));
    assert_eq!(r.memory(), 8);
}

#[test]
fn attacking_evolution_excludes_an_unaffordable_accel_card() {
    let mut evo = digimon("TEXT-EVO", 5, 7000, false);
    evo.traits.push("ACCEL".into());
    for route in &mut evo.evo_costs {
        route.memory_cost = 12;
    }
    let mut r = start(builder(CARD).add_card(evo).hand(0, &["TEXT-EVO"]).memory(0));
    let carrier = r.place_on_field(0, CARD, Some(0));
    r.place_on_field(1, "TEXT-TAMER", Some(0));
    r.attack_player(carrier, 1, false);
    assert!(r.pending_selection().is_none());
    assert_eq!(ids(&r, &r.game.players[0].hand), ["TEXT-EVO"]);
    assert_eq!(r.dp_of(carrier), Some(5000));
}

#[test]
fn inherited_barrier_can_trash_security_or_be_declined() {
    for accept in [false, true] {
        let mut r = start(builder(CARD));
        let carrier = r.place_stack(0, &[CARD, "TEXT-5"]);
        let enemy = r.place_on_field(1, "TEXT-6", Some(0));
        r.battle_digimon(carrier, enemy);
        r.assert_prompt(SelectionKind::Replacement, true, 0);
        if accept {
            r.accept_optional_trigger().unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
        assert_eq!(r.battle_area_size(0), usize::from(accept));
        assert_eq!(r.security_count(0), if accept { 4 } else { 5 });
    }
}
