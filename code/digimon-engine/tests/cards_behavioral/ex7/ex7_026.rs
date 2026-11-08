//! EX7-026 Starmon: actual image + official bundle; CR 4.3 16-25 Barrier.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_digivolve, PASS},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, Keyword};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::TriggerSource;
const ID: &str = "EX7-026";
fn base() -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder()
        .dsl_card(ID)
        .expect("production Starmon YAML");
    for (id, color, level, nsp, dp) in [
        ("Y", CardColor::Yellow, 3, false, 8000),
        ("R", CardColor::Red, 3, true, 8000),
        ("B", CardColor::Blue, 3, true, 8000),
        ("P", CardColor::Purple, 3, true, 8000),
        ("G", CardColor::Green, 3, true, 8000),
        ("K", CardColor::Black, 3, true, 8000),
        ("W", CardColor::White, 3, true, 8000),
        ("BAD", CardColor::Red, 3, false, 8000),
        ("LV4", CardColor::Red, 4, true, 8000),
        ("SMALL", CardColor::Yellow, 3, false, 3000),
        ("BIG", CardColor::Red, 5, false, 15000),
    ] {
        let mut c = make_test_card(id, id);
        c.colors = vec![color];
        c.level = Some(level);
        c.traits = if nsp {
            vec!["NSp".into()]
        } else {
            vec!["Beast".into()]
        };
        c.dp = Some(dp);
        b = b.add_card(c);
    }
    let mut t = make_test_card("T", "Tamer");
    t.card_kind = CardKind::Tamer;
    t.dp = None;
    b.add_card(t)
        .deck(0, &["Y"; 20])
        .deck(1, &["Y"; 20])
        .memory(10)
}
fn main(r: &mut DebugRunner) {
    r.game.current_phase = GamePhase::Main;
    r.game.turn_count = 2;
}
fn pick(r: &mut DebugRunner, index: usize) {
    let v = r
        .pending_selection_view()
        .expect("mandatory opponent Digimon choice");
    assert!(!v.is_optional);
    let m = build_action_mask(&r.game, v.selecting_player);
    assert_eq!(m[PASS as usize], 0.0);
    for a in &v.valid_action_ids {
        assert_eq!(m[*a as usize], 1.0);
    }
    r.execute_action(v.selecting_player, v.valid_action_ids[index])
        .unwrap();
}
#[test]
fn play_mandatory_all_opponent_digimon_only_selected_reduced_and_expires() {
    let mut r = base().hand(0, &[ID]).start();
    main(&mut r);
    let own = r.place_on_field(0, "Y", Some(0));
    let a = r.place_on_field(1, "Y", Some(0));
    let b = r.place_on_field(1, "R", Some(0));
    r.place_on_field(1, "T", Some(0));
    r.place_in_breeding(1, "Y");
    assert_eq!(build_action_mask(&r.game, 0)[0], 1.0);
    r.game.decode_action(0, 0);
    assert_eq!(r.memory(), 6);
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids.len(),
        2
    );
    pick(&mut r, 1);
    assert_eq!(r.effective_dp(a), Some(8000));
    assert_eq!(r.effective_dp(b), Some(5000));
    assert_eq!(r.effective_dp(own), Some(8000));
    r.end_turn();
    assert_eq!(r.effective_dp(b), Some(8000));
}
#[test]
fn normal_and_all_color_nsp_evolutions_pay_two_and_trigger() {
    for src in ["Y", "R", "B", "P", "G", "K", "W"] {
        let mut r = base().hand(0, &[ID]).start();
        main(&mut r);
        let h = r.place_on_field(0, src, Some(0));
        let opp = r.place_on_field(1, "Y", Some(0));
        let a = encode_digivolve(0, h.index as u16);
        assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0, "{src}");
        r.game.decode_action(a, 0);
        assert_eq!(r.memory(), 8, "{src}");
        pick(&mut r, 0);
        assert_eq!(r.effective_dp(opp), Some(5000));
        assert_eq!(
            r.game.players[0].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            ID
        );
    }
}
#[test]
fn wrong_color_trait_or_level_cannot_evolve() {
    for src in ["BAD", "LV4"] {
        let mut r = base().hand(0, &[ID]).start();
        main(&mut r);
        let h = r.place_on_field(0, src, Some(0));
        assert_eq!(
            build_action_mask(&r.game, 0)[encode_digivolve(0, h.index as u16) as usize],
            0.0,
            "{src}"
        );
    }
}
#[test]
fn empty_opponent_board_play_and_evolution_resolve() {
    for evo in [false, true] {
        let mut r = base().hand(0, &[ID]).start();
        main(&mut r);
        let a = if evo {
            let h = r.place_on_field(0, "R", Some(0));
            encode_digivolve(0, h.index as u16)
        } else {
            0
        };
        assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
        r.game.decode_action(a, 0);
        assert!(r.pending_selection().is_none());
        assert_eq!(r.memory(), if evo { 8 } else { 6 });
    }
}
#[test]
fn zero_dp_is_deleted() {
    let mut r = base().hand(0, &[ID]).start();
    main(&mut r);
    r.place_on_field(1, "SMALL", Some(0));
    r.game.decode_action(0, 0);
    pick(&mut r, 0);
    assert_eq!(r.battle_area_size(1), 0);
    assert!(r.game.players[1]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "SMALL"));
}
#[test]
fn same_source_repeated_trigger_has_no_once_per_turn() {
    let mut r = base().start();
    let h = r.place_on_field(0, ID, Some(0));
    let o = r.place_on_field(1, "Y", Some(0));
    for timing in [EffectTiming::OnPlay, EffectTiming::WhenDigivolving] {
        r.game
            .enqueue_triggered(timing, TriggerSource::Permanent(h));
        r.game.drain_effect_queue();
        pick(&mut r, 0);
    }
    assert_eq!(r.effective_dp(o), Some(2000));
}
#[test]
fn barrier_is_inherited_only() {
    let mut r = base().start();
    let top = r.place_on_field(0, ID, Some(0));
    assert!(!r.game.has_keyword(top, Keyword::Barrier));
    let h = r.place_stack(0, &[ID, "Y"]);
    assert!(r.game.has_keyword(h, Keyword::Barrier));
}
#[test]
fn inherited_barrier_real_battle_accept_decline_both_seats() {
    for owner in [0u8, 1] {
        for accept in [false, true] {
            let mut r = base().security(owner, &["SMALL", "Y"]).start();
            let h = r.place_stack(owner, &[ID, "Y"]);
            let big = r.place_on_field(1 - owner, "BIG", Some(0));
            let top = r.game.players[owner as usize]
                .security
                .last()
                .unwrap()
                .handle();
            r.battle_digimon(h, big);
            let v = r.pending_selection_view().expect("Barrier payment");
            assert!(v.is_optional);
            assert_eq!(v.selecting_player, owner);
            assert_eq!(build_action_mask(&r.game, owner)[PASS as usize], 1.0);
            if accept {
                r.accept_optional_trigger().unwrap();
            } else {
                r.decline_optional_trigger().unwrap();
            }
            r.auto_resolve().unwrap();
            assert_eq!(r.battle_area_size(owner), usize::from(accept));
            assert_eq!(r.security_count(owner), if accept { 1 } else { 2 });
            assert_eq!(
                r.game.players[owner as usize]
                    .trash
                    .iter()
                    .any(|c| c.handle() == top),
                accept
            );
        }
    }
}
#[test]
fn inherited_barrier_no_security_or_effect_deletion() {
    for effect in [false, true] {
        let mut r = base()
            .security(0, if effect { &["Y"][..] } else { &[][..] })
            .start();
        let h = r.place_stack(0, &[ID, "Y"]);
        if effect {
            r.game
                .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
        } else {
            let big = r.place_on_field(1, "BIG", Some(0));
            r.battle_digimon(h, big);
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(r.battle_area_size(0), 0);
        assert_eq!(r.security_count(0), usize::from(effect));
    }
}

struct SecurityGuard {
    optional: bool,
}
impl digimon_engine::effect::CardEffect for SecurityGuard {
    fn effects(
        &self,
        c: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        let b = digimon_engine::effect::Effect::when_would_be_trashed(c)
            .replacement_process(|ctx| ctx.cancel());
        vec![if self.optional {
            b.optional().build()
        } else {
            b.build()
        }]
    }
}
#[test]
fn barrier_canceled_security_payment_does_not_save_carrier() {
    let mut r = base().security(0, &["SMALL"]).start();
    let h = r.place_stack(0, &[ID, "Y"]);
    let big = r.place_on_field(1, "BIG", Some(0));
    r.register_effect("T", std::sync::Arc::new(SecurityGuard { optional: false }));
    r.place_on_field(0, "T", Some(0));
    r.battle_digimon(h, big);
    r.accept_optional_trigger().unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.security_count(0), 1);
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "T"
    );
}
#[test]
fn barrier_parked_security_payment_waits_then_survives_only_if_paid() {
    for cancel in [false, true] {
        let mut r = base().security(0, &["SMALL"]).start();
        let h = r.place_stack(0, &[ID, "Y"]);
        let big = r.place_on_field(1, "BIG", Some(0));
        r.register_effect("T", std::sync::Arc::new(SecurityGuard { optional: true }));
        r.place_on_field(0, "T", Some(0));
        r.battle_digimon(h, big);
        r.accept_optional_trigger().unwrap();
        let v = r.pending_selection_view().expect("parked security payment");
        assert!(v.is_optional);
        assert_eq!(r.security_count(0), 1);
        assert_eq!(r.battle_area_size(0), 2);
        if cancel {
            r.accept_optional_trigger().unwrap();
        } else {
            r.decline_optional_trigger().unwrap();
        }
        r.auto_resolve().unwrap();
        assert_eq!(r.security_count(0), usize::from(cancel));
        assert_eq!(r.battle_area_size(0), if cancel { 1 } else { 2 });
    }
}
