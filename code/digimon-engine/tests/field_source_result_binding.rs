use digimon_dsl::compiled::{CompiledBindingRef, CompiledStep};
use digimon_engine::{
    debug_runner::{make_test_card, DebugRunner},
    dsl_cards::{bindings::Bindings, step::run_step},
    effect_context::EffectContext,
};

#[test]
fn top_of_resolves_only_a_single_permanent_and_preserves_physical_top_identity() {
    use digimon_engine::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TOP-HOST", "Host"))
        .add_card(make_test_card("TOP-LOWER", "Lower"))
        .start();
    r.skip_mulligan();
    let host = r.place_stack(0, &["TOP-LOWER", "TOP-HOST"]);
    let top = r.game.players[0].battle_area[host.index as usize].top_card().handle();
    let lower = r.game.players[0].battle_area[host.index as usize].card_sources[0].handle();
    let mut bindings = Bindings::new();
    bindings.insert_permanent("host", host);
    bindings.insert_card("card", lower);
    bindings.insert_card_list("cards", vec![lower, top]);
    bindings.insert_permanent_list("hosts", vec![host]);
    let ctx = EffectContext::new(&mut r.game, top, Some(host), 0);
    assert_eq!(resolve_binding_ref(&CompiledBindingRef::TopOf("host".into()), &ctx, &bindings), Some(ResolvedBinding::Card(top)));
    for name in ["missing", "card", "cards", "hosts"] {
        assert_eq!(resolve_binding_ref(&CompiledBindingRef::TopOf(name.into()), &ctx, &bindings), None, "{name} must not select an arbitrary card or list member");
    }
}
#[test]
fn successful_placement_does_not_leak_result_into_a_failed_repeat_and_may_reuse_input_name() {
    for missing_target in [false, true] {
        let mut r = DebugRunner::builder()
            .add_card(make_test_card("RESULT-HOST", "Result host"))
            .add_card(make_test_card("RESULT-DONOR", "Result donor"))
            .start();
        r.skip_mulligan();
        let host = r.place_on_field(0, "RESULT-HOST", None);
        let donor = r.place_on_field(0, "RESULT-DONOR", None);
        let source = r.game.players[0].battle_area[0].top_card().handle();
        let paid = r.game.players[0].battle_area[donor.index as usize]
            .top_card()
            .handle();
        let mut bindings = Bindings::new();
        bindings.insert_card("paid", paid);
        let first = CompiledStep::PlaceAsBottomSource {
            source: CompiledBindingRef::Named("paid".into()),
            target: CompiledBindingRef::Source,
            face_down: false,
            bind_placed_as: Some("paid".into()),
        };
        let mut ctx = EffectContext::new(&mut r.game, source, Some(host), 0);
        run_step(&first, &mut ctx, &mut bindings);
        assert!(
            bindings.get("paid").is_some(),
            "input and output slot may share a name"
        );
        assert_eq!(
            ctx.game.players[0].battle_area[0].card_sources[0].handle(),
            paid
        );
        let second = CompiledStep::PlaceAsBottomSource {
            source: CompiledBindingRef::Named(
                if missing_target { "paid" } else { "missing" }.into(),
            ),
            target: if missing_target {
                CompiledBindingRef::Named("missing".into())
            } else {
                CompiledBindingRef::Source
            },
            face_down: false,
            bind_placed_as: Some("paid".into()),
        };
        run_step(&second, &mut ctx, &mut bindings);
        assert!(
            bindings.get("paid").is_none(),
            "failed repeat cannot retain an earlier success binding"
        );
        assert_eq!(
            ctx.game.players[0].battle_area[0].card_sources[0].handle(),
            paid
        );
    }
}
