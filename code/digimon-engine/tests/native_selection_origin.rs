//! Native callbacks retain activated origin through compaction, departure and captured absence.
use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::modifiers::EffectControllerFilter;
use digimon_engine::{EffectSourceKind, Expiry};
fn setup() -> DebugRunner {
    let mut c = make_test_card("SELF", "Self");
    c.dp = Some(12000);
    DebugRunner::builder()
        .add_card(c)
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
        .hand(0, &["PAD"])
        .memory(0)
        .start()
}
fn choose(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("native pending choice");
    let a = v.valid_action_ids[0];
    assert_eq!(
        build_action_mask(&r.game, v.selecting_player)[a as usize],
        1.0
    );
    assert_eq!(
        build_action_mask(&r.game, v.selecting_player)[PASS as usize],
        0.0
    );
    r.execute_action(v.selecting_player, a).unwrap();
}
#[test]
fn native_chained_choice_after_lower_slot_compaction_keeps_own_carrier_exemption() {
    let mut r = setup();
    let lower = r.place_on_field(0, "PAD", Some(0));
    let host = r.place_on_field(0, "SELF", Some(0));
    r.place_on_field(0, "PAD", Some(0));
    let card = r.game.players[0].battle_area[host.index as usize]
        .top_card()
        .handle();
    {
        let mut ctx = EffectContext::new(&mut r.game, card, Some(host), 0);
        let origin = ctx.source_identity;
        ctx.add_effect_immunity_modifier_excluding_self(
            host,
            EffectSourceKind::Digimon,
            EffectControllerFilter::Any,
            Expiry::EndOfTurn,
            true,
        );
        ctx.select_effect_choice(
            "First native choice",
            vec!["Continue".into()],
            move |ctx, _| {
                ctx.game.delete_permanent_with_effects(lower);
                assert_eq!(ctx.game.players[0].battle_area.len(), 2);
                ctx.select_effect_choice(
                    "Continue after lower slot leaves",
                    vec!["Continue".into()],
                    move |ctx, _| {
                        assert_eq!(ctx.source_identity, origin);
                        assert_eq!(ctx.source_kind, EffectSourceKind::Digimon);
                        let h = ctx
                            .source_permanent
                            .expect("same original carrier remains live");
                        assert_eq!(
                            ctx.game.players[0].battle_area[h.index as usize]
                                .top_card()
                                .handle(),
                            card
                        );
                        ctx.add_dp_modifier(h, 1000, Expiry::EndOfTurn);
                    },
                );
            },
        );
    }
    choose(&mut r);
    assert_eq!(
        r.pending_selection().unwrap().source_permanent,
        Some(r.perm_handle(0, 0)),
        "installed source metadata follows the original live carrier"
    );
    choose(&mut r);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(13000));
}
#[test]
fn native_chained_hand_callback_keeps_departed_identity_and_does_not_follow_replayed_card() {
    let mut r = setup();
    // Pay the normal replay cost without ending the turn and expiring the tested buff.
    r.game.set_memory(10);
    let source = r.place_on_field(0, "SELF", Some(0));
    let target = r.place_on_field(0, "PAD", Some(0));
    let card = r.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();
    let target_card = r.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();
    {
        let mut ctx = EffectContext::new(&mut r.game, card, Some(source), 0);
        let origin = ctx.source_identity;
        ctx.add_effect_immunity_modifier_excluding_self(
            target,
            EffectSourceKind::Digimon,
            EffectControllerFilter::Any,
            Expiry::EndOfTurn,
            true,
        );
        ctx.select_effect_choice("Leave and replay before native hand choice",vec!["Continue".into()],move|ctx,_|{
    ctx.game.delete_permanent_with_effects(source);
    let i=ctx.game.players[0].trash.iter().position(|c|c.handle()==card).unwrap();ctx.game.play_from_trash(0,i).unwrap();
    ctx.select_hand(0,"Native hand choice",false,|_,_|true,move|ctx,_|{
        assert_eq!(ctx.source_identity,origin);assert!(ctx.source_permanent.is_none(),"old carrier cannot become replayed physical card or compacted neighbor");
        let i=ctx.game.players[0].battle_area.iter().position(|p|p.top_card().handle()==target_card).unwrap();ctx.add_dp_modifier(digimon_engine::permanent::PermanentHandle{player:0,index:i as u8},1000,Expiry::EndOfTurn);
    });
 });
    }
    choose(&mut r);
    assert_eq!(
        r.pending_selection().unwrap().source_permanent,
        None,
        "metadata must not identify the compacted neighbor or replayed card as the departed source"
    );
    choose(&mut r);
    assert_eq!(r.game.turn_player_idx, 0);
    assert_eq!(r.memory(), 7);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(3000));
}
#[test]
fn native_callback_preserves_explicit_no_carrier_origin_and_activated_category() {
    let mut r = setup();
    let host = r.place_on_field(0, "SELF", Some(0));
    let card = r.game.players[0].battle_area[0].top_card().handle();
    {
        let mut ctx = EffectContext::new_with_source_kind(
            &mut r.game,
            card,
            Some(host),
            EffectSourceKind::Option,
            0,
        );
        ctx.source_identity = None;
        ctx.select_effect_choice("No field origin", vec!["Continue".into()], |ctx, _| {
            assert_eq!(ctx.source_identity, None);
            assert_eq!(ctx.source_permanent, None);
            assert_eq!(ctx.source_kind, EffectSourceKind::Option);
            ctx.gain_memory(1);
        });
    }
    assert_eq!(r.pending_selection().unwrap().source_permanent, None);
    choose(&mut r);
    assert_eq!(r.memory(), 1);
}
