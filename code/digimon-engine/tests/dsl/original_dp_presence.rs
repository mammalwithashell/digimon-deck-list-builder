//! Original-DP changes cannot create a property absent from the rules identity.
use digimon_dsl::compiled::{CompiledBindingRef, CompiledModifierPayload, CompiledModifierTarget, CompiledModifierValue, CompiledStep};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::{bindings::Bindings, step::run_step};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::{ModifierEntry, ModifierPayload};
use digimon_engine::permanent::PermanentHandle;

fn fixture(dp: Option<i32>) -> (DebugRunner, PermanentHandle) {
    let mut target = make_test_card("TARGET", "Target");
    target.dp = dp;
    if dp.is_none() { target.card_kind = CardKind::Tamer; target.level = None; }
    let mut top = make_test_card("TOP", "Top"); top.level = Some(4); top.dp = Some(8000);
    let mut r = DebugRunner::builder().add_card(make_test_card("SRC", "Source"))
        .add_card(target).add_card(top).start();
    r.place_on_field(0, "SRC", Some(0));
    let target = r.place_on_field(1, "TARGET", Some(0));
    (r, target)
}
fn apply(r: &mut DebugRunner, h: PermanentHandle, modifier: &str, typed: bool) {
    let source = r.game.players[0].battle_area[0].top_card().handle();
    let mut ctx = EffectContext::new(&mut r.game, source, Some(PermanentHandle { player: 0, index: 0 }), 0);
    let mut bindings = Bindings::new(); bindings.insert_permanent("target", h);
    run_step(&CompiledStep::AddModifier {
        opponent_only: false,
        target: CompiledModifierTarget::Binding(CompiledBindingRef::Named("target".into())),
        modifier: modifier.into(), value: CompiledModifierValue::Literal(if typed { 0 } else { 6000 }),
        expiry: "end_of_opponents_turn".into(), synth_identity: None,
        payload: typed.then_some(CompiledModifierPayload::Dp(6000)), continuous: false,
    }, &mut ctx, &mut bindings);
}
fn dp(r: &DebugRunner, h: PermanentHandle) -> Option<i32> {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .synth_identity(&r.game.card_data, &r.game.modifiers, h).dp
}
#[test]
fn original_dp_does_not_create_dp_on_a_card_without_dp() {
    for modifier in ["ChangeOriginDP", "ChangeCardDP"] {
        for typed in [false, true] {
            let (mut r, h) = fixture(None);
            apply(&mut r, h, modifier, typed);
            assert_eq!(dp(&r, h), None, "{modifier}, typed={typed}");
        }
    }
}
#[test]
fn original_dp_change_preserves_additive_dp_changes() {
    let (mut r, h) = fixture(Some(8000));
    r.game.modifiers.add(h, ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::Permanent, 0));
    apply(&mut r, h, "ChangeOriginDP", true);
    assert_eq!(dp(&r, h), Some(6000));
    assert_eq!(r.game.effective_dp(h), Some(7000));
}
#[test]
fn original_dp_stops_applying_after_de_digivolve_exposes_a_tamer() {
    let (mut r, previous) = fixture(None);
    r.game.players[1].battle_area.remove(previous.index as usize);
    let h = r.place_stack(1, &["TARGET", "TOP"]);
    apply(&mut r, h, "ChangeOriginDP", true);
    assert_eq!(dp(&r, h), Some(6000));
    let source = r.game.players[0].battle_area[0].top_card().handle();
    let mut ctx = EffectContext::new(&mut r.game, source, Some(PermanentHandle { player: 0, index: 0 }), 0);
    assert_eq!(ctx.de_digivolve(h, Some(3), Some(1)), 1);
    assert_eq!(r.game.players[1].battle_area[h.index as usize].top_card().card_id(&r.game.card_data), "TARGET");
    assert_eq!(dp(&r, h), None);
}
#[test]
fn explicitly_granted_dp_can_change_and_dont_have_dp_remains_authoritative() {
    let (mut r, h) = fixture(None);
    r.game.modifiers.add(h, ModifierEntry::simple(ModifierType::TreatAsDigimon, 0, Expiry::Permanent, 0)
        .with_payload(ModifierPayload::SynthIdentity {
            kind: CardKind::Digimon, level: 3, colors: vec![CardColor::Yellow], traits: vec![], dp: 4000,
        }));
    apply(&mut r, h, "ChangeOriginDP", true);
    assert_eq!(dp(&r, h), Some(6000), "explicitly granted DP is an existing property");
    r.game.modifiers.add(h, ModifierEntry::simple(ModifierType::DontHaveDp, 0, Expiry::Permanent, 0));
    assert_eq!(dp(&r, h), None);
}
