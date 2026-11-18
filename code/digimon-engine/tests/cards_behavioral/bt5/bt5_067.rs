//! BT5-067 -- synthetic-board behavioural tests for the Keramon line
//! (dcgo-replay hunt). Fixtures: `keramon_line_support`.
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::keramon_line_support::*;
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, ModifierType};
use digimon_engine::permanent::PermanentHandle;

#[test]
fn bt5_067_keramon_may_digivolve_from_hand_for_4_ignoring_requirements_both_seats() {
    for seat in 0..=1u8 {
        let mut r = infermon_runner(seat, 6);
        // BT22-053 Keramon is a Lv.3 -- NOT the Lv.4 the printed route needs.
        let keramon = r.place_on_field(seat, "BT22-053", Some(0));
        let hand = r.add_to_hand(seat, "BT5-067");
        let id = encode_digivolve(hand as u16, keramon.index as u16);
        assert!(
            legal(&r, seat, id),
            "[Keramon] can digivolve into BT5-067 for 4, ignoring requirements (seat {seat})"
        );
        act(&mut r, seat, id);
        assert_eq!(top_id(&r, keramon), "BT5-067");
        assert_eq!(r.memory(), 2, "memory cost is exactly 4 (6 - 4)");
    }
}

#[test]
fn bt5_067_special_route_needs_the_keramon_name() {
    let mut r = infermon_runner(0, 6);
    let other = r.place_on_field(0, "BLACK3", Some(0));
    let hand = r.add_to_hand(0, "BT5-067");
    let id = encode_digivolve(hand as u16, other.index as u16);
    assert!(
        !legal(&r, 0, id),
        "a Lv.3 black Digimon that is not [Keramon] cannot digivolve into BT5-067"
    );
}

#[test]
fn bt5_067_inherited_on_deletion_plays_a_diaboromon_token_optionally() {
    for seat in 0..=1u8 {
        let mut r = infermon_runner(seat, 6);
        // Infermon is the digivolution source (inherited text); the host is a
        // plain Digimon, so the effect cannot come from the host's own text.
        let host = r.place_stack(seat, &["BT5-067", "BLACK3"]);
        r.game.delete_permanent_with_effects(host);
        r.game.drain_effect_queue();
        assert!(r.pending_is_optional(), "[On Deletion] is a 'you may' (seat {seat})");
        r.accept_optional_trigger().unwrap();
        let _ = r.auto_resolve();
        let tokens = token_on_field(&r, seat);
        assert_eq!(tokens.len(), 1, "one [Diaboromon] Token (seat {seat})");
        let t = &r.game.players[seat as usize].battle_area[tokens[0].index as usize];
        let data = &r.game.card_data[t.top_card().data_index];
        assert_eq!(data.card_name, "Diaboromon");
        assert_eq!(data.level, Some(6));
        assert_eq!(data.dp, Some(3000));
        assert_eq!(data.play_cost, 14);
        assert_eq!(data.colors, vec![CardColor::White]);
        for t in ["Mega", "Unknown", "Unidentified"] {
            assert!(data.traits.iter().any(|x| x == t), "trait {t}");
        }
        assert_eq!(r.memory(), 6, "played without paying");
    }
}

#[test]
fn bt5_067_inherited_on_deletion_can_be_declined() {
    let mut r = infermon_runner(0, 6);
    let host = r.place_stack(0, &["BT5-067", "BLACK3"]);
    r.game.delete_permanent_with_effects(host);
    r.game.drain_effect_queue();
    assert!(r.pending_is_optional());
    r.decline_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert!(token_on_field(&r, 0).is_empty());
}

#[test]
fn bt5_067_on_deletion_is_inherited_only_not_the_top_cards_own_effect() {
    // Infermon on top (its own text has no [On Deletion]; only the INHERITED
    // effect does): deleting a stack topped by Infermon grants nothing.
    let mut r = infermon_runner(0, 6);
    let top = r.place_stack(0, &["BLACK3", "BT5-067"]);
    r.game.delete_permanent_with_effects(top);
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    assert!(token_on_field(&r, 0).is_empty());
}

#[test]
fn bt5_067_special_route_is_exact_keramon_name_not_a_substring() {
    let mut kx = make_test_card("KERAMON-X", "Keramon X");
    kx.level = Some(3);
    kx.dp = Some(1000);
    kx.play_cost = 3;
    kx.colors = vec![CardColor::Black];
    let r = base().add_card(kx).dsl_card("BT5-067").unwrap().build();
    let mut r = into_main(r, 0, 6);
    let host = r.place_on_field(0, "KERAMON-X", Some(0));
    let hand = r.add_to_hand(0, "BT5-067");
    let id = encode_digivolve(hand as u16, host.index as u16);
    assert!(
        !legal(&r, 0, id),
        "DCGO CardNames.Contains(\"Keramon\") is exact: [Keramon X] is not [Keramon]"
    );
}

#[test]
fn bt5_067_special_route_cannot_use_a_keramon_in_the_breeding_area() {
    // Official Q&A (BT5-067): "No, you can't digivolve from a [Keramon] in
    // your breeding area using this card's effect."
    let mut r = infermon_runner(0, 6);
    r.place_in_breeding(0, "BT17-053");
    let hand = r.add_to_hand(0, "BT5-067");
    let id = encode_digivolve(hand as u16, digimon_engine::action::space::BREEDING_TARGET);
    assert!(!legal(&r, 0, id));
}

#[test]
fn bt5_067_special_route_only_applies_from_the_hand_not_to_other_infermon_cards() {
    // The route is on BT5-067 itself: a different Lv.5 in hand gets no such route.
    let mut r = infermon_runner(0, 6);
    let keramon = r.place_on_field(0, "BT17-053", Some(0));
    let hand = r.add_to_hand(0, "INERT");
    let id = encode_digivolve(hand as u16, keramon.index as u16);
    assert!(!legal(&r, 0, id));
}
