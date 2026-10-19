//! Field DUAL information uses the Digimon face (CRM 4-6-3, 4-6-5-2).
//! Effective color overlays still apply after that base identity.
use digimon_dsl::compiled::{CompiledCardKind, CompiledPlayerRef, CompiledPredicate, CompiledStep};
use digimon_engine::action::{build_action_mask, space::PLAY_HAND_START};
use digimon_engine::card_data::{DualCardData, DualDigimonFace, DualOptionFace};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::{bindings::Bindings, step::run_steps};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::{ModifierEntry, ModifierPayload};

fn check(dual: bool, replace: bool, add: bool, bound: bool) {
    let mut field = make_test_card("FIELD", "Field");
    field.colors = vec![if dual { CardColor::Purple } else { CardColor::Green }];
    if dual {
        field.card_kind = CardKind::Dual;
        field.dual = Some(DualCardData {
            digimon: DualDigimonFace { level: 4, dp: 5000, colors: vec![CardColor::Green],
                traits: vec![], evo_costs: vec![], effect_text: String::new(),
                inherited_text: String::new(), keywords: vec![] },
            option: DualOptionFace { use_cost: 3, colors: vec![CardColor::Purple],
                effect_text: String::new(), security_text: String::new(), keywords: vec![] },
        });
    }
    let colors = [CardColor::Green, CardColor::Purple, CardColor::Blue, CardColor::Yellow];
    let ids = ["GREEN", "PURPLE", "BLUE", "YELLOW"];
    let mut builder = DebugRunner::builder().add_card(field);
    for (id, color) in ids.iter().zip(colors) {
        let mut card = make_test_card(id, id);
        card.card_kind = CardKind::Tamer;
        card.colors = vec![color];
        builder = builder.add_card(card);
    }
    let mut r = builder.hand(0, &["FIELD", "GREEN", "PURPLE", "BLUE", "YELLOW"]).build();
    let source = r.game.players[0].hand[0].handle();
    let handle = r.place_on_field(0, "FIELD", None);
    if replace {
        r.game.modifiers.add(handle, ModifierEntry::simple(ModifierType::ChangeBaseCardColor, 0, Expiry::Permanent, 0)
            .with_payload(ModifierPayload::Colors { value: vec![CardColor::Yellow], base: true }));
    }
    if add {
        r.game.modifiers.add(handle, ModifierEntry::simple(ModifierType::AddColor, 0, Expiry::Permanent, 0)
            .with_payload(ModifierPayload::Colors { value: vec![CardColor::Blue], base: false }));
    }
    let mut expected_colors = vec![if replace { CardColor::Yellow } else { CardColor::Green }];
    if add { expected_colors.push(CardColor::Blue); }
    let permanent = &r.game.players[0].battle_area[0];
    assert_eq!(permanent.synth_identity(&r.game.card_data, &r.game.modifiers, handle).colors, expected_colors);
    assert_eq!(permanent.colors_for_rules(&r.game.card_data, &r.game.modifiers, handle), expected_colors);
    // Hand/Option metadata must not be globally rewritten to the field face.
    if dual {
        assert_eq!(r.game.players[0].hand[0].colors(&r.game.card_data), &[CardColor::Purple]);
        assert_eq!(r.game.players[0].hand[0].option_colors(&r.game.card_data), &[CardColor::Purple]);
    }
    let mut filter = CompiledPredicate { kind: Some(CompiledCardKind::Tamer), ..Default::default() };
    if bound { filter.color_matches_binding = Some("field".into()); }
    else { filter.color_matches_any_field_digimon = Some(CompiledPlayerRef::You); }
    let steps = vec![CompiledStep::SelectHand { then: vec![], of: CompiledPlayerRef::You,
        filter, bind_as: Some("pick".into()), prompt: "Match effective field colors".into(),
        prompt_key: None, optional: false, cost: false }];
    let mut bindings = Bindings::new();
    bindings.insert_permanent("field", handle);
    run_steps(&steps, &mut EffectContext::new(&mut r.game, source, None, 0), &mut bindings);
    let expected: Vec<u16> = colors.iter().enumerate().filter(|(_, c)| expected_colors.contains(c))
        .map(|(i, _)| PLAY_HAND_START + 1 + i as u16).collect();
    let pending = r.game.pending_selection.as_ref().expect("matching-color selection");
    assert_eq!(pending.valid_action_ids, expected);
    let mask = build_action_mask(&r.game, pending.selecting_player);
    for i in 0..4 { let action = PLAY_HAND_START + 1 + i;
        assert_eq!(mask[action as usize], if expected.contains(&action) { 1.0 } else { 0.0 }); }
}

#[test]
fn dual_field_base_colors_use_digimon_face_for_identity_and_comparisons() {
    for bound in [false, true] { check(true, false, false, bound); }
}
#[test]
fn dual_field_add_color_keeps_digimon_face_and_added_color() {
    for bound in [false, true] { check(true, false, true, bound); }
}
#[test]
fn dual_field_replacement_and_addition_override_face_in_order() {
    for add in [false, true] { for bound in [false, true] { check(true, true, add, bound); } }
}
#[test]
fn ordinary_field_colors_and_overlays_remain_unchanged() {
    for replace in [false, true] { for add in [false, true] { for bound in [false, true] {
        check(false, replace, add, bound);
    } } }
}
