//! EX10-041 Wizardmon: printed face + official bundle; DCGO EX10_041.cs.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_digivolve, PASS},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::TriggerSource;
const ID: &str = "EX10-041";
fn base() -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder()
        .dsl_card(ID)
        .expect("production Wizardmon YAML");
    for (id, color, level, evil, dp) in [
        ("Y", CardColor::Yellow, 3, false, 8000),
        ("P", CardColor::Purple, 3, false, 8000),
        ("R", CardColor::Red, 3, true, 8000),
        ("B", CardColor::Blue, 3, true, 8000),
        ("G", CardColor::Green, 3, true, 8000),
        ("K", CardColor::Black, 3, true, 8000),
        ("W", CardColor::White, 3, true, 8000),
        ("PE", CardColor::Purple, 3, true, 8000),
        ("YE", CardColor::Yellow, 3, true, 8000),
        ("BAD", CardColor::Red, 3, false, 8000),
        ("LV4", CardColor::Purple, 4, true, 8000),
        ("BIG", CardColor::Red, 5, false, 15000),
        ("SMALL", CardColor::Yellow, 3, false, 3000),
    ] {
        let mut c = make_test_card(id, id);
        c.colors = vec![color];
        c.level = Some(level);
        c.traits = vec![if evil { "Evil" } else { "Beast" }.into()];
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
fn main_phase(r: &mut DebugRunner) {
    r.game.current_phase = GamePhase::Main;
    r.game.turn_count = 2;
}
fn optional(r: &DebugRunner) {
    let v = r
        .pending_selection_view()
        .expect("optional security payment");
    assert!(v.is_optional);
    assert_eq!(
        build_action_mask(&r.game, v.selecting_player)[PASS as usize],
        1.0
    );
}
fn pick(r: &mut DebugRunner, index: usize) {
    let v = r
        .pending_selection_view()
        .expect("mandatory one opponent Digimon");
    assert!(!v.is_optional);
    let m = build_action_mask(&r.game, v.selecting_player);
    assert_eq!(m[PASS as usize], 0.0);
    for a in &v.valid_action_ids {
        assert_eq!(m[*a as usize], 1.0);
    }
    r.execute_action(v.selecting_player, v.valid_action_ids[index])
        .unwrap();
}
fn sec(r: &DebugRunner, h: PermanentHandle) -> i32 {
    r.game.modifiers.sum(h, ModifierType::SecurityAttackChange)
}
#[test]
fn main_play_accept_trashes_exact_top_security_and_two_deck_cards_mass_dp_expires() {
    let mut r = base().hand(0, &[ID]).security(0, &["P", "Y"]).start();
    main_phase(&mut r);
    let own = r.place_on_field(0, "Y", Some(0));
    let a = r.place_on_field(1, "Y", Some(0));
    let b = r.place_on_field(1, "P", Some(0));
    r.place_on_field(1, "T", Some(0));
    r.place_in_breeding(1, "Y");
    let paid = r.game.players[0].security.last().unwrap().handle();
    let cards: Vec<_> = r.game.players[0]
        .deck
        .iter()
        .rev()
        .take(2)
        .map(|c| c.handle())
        .collect();
    let n = r.game.players[0].deck.len();
    assert_eq!(build_action_mask(&r.game, 0)[0], 1.0);
    r.game.decode_action(0, 0);
    assert_eq!(r.memory(), 5);
    optional(&r);
    assert_eq!(r.game.players[0].deck.len(), n);
    r.accept_optional_trigger().unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.security_count(0), 1);
    assert_eq!(r.game.players[0].deck.len(), n - 2);
    for c in cards.into_iter().chain([paid]) {
        assert!(r.game.players[0].trash.iter().any(|x| x.handle() == c));
    }
    assert_eq!(r.effective_dp(a), Some(5000));
    assert_eq!(r.effective_dp(b), Some(5000));
    assert_eq!(r.effective_dp(own), Some(8000));
    r.add_to_hand(1, "Y");
    r.register_effect("BAD", std::sync::Arc::new(MoveEffect { owner: 1, mode: 6 }));
    let e = r.place_on_field(0, "BAD", Some(0));
    trigger(&mut r, e);
    let late = r.perm_handle(1, 3);
    assert_eq!(r.effective_dp(late), Some(5000));
    r.end_turn();
    assert_eq!(r.effective_dp(a), Some(8000));
    assert_eq!(r.effective_dp(late), Some(8000));
}
#[test]
fn all_normal_and_any_color_evil_evolution_routes_pay_and_offer_real_choice() {
    for (src, cost) in [
        ("Y", 3),
        ("P", 3),
        ("R", 2),
        ("B", 2),
        ("G", 2),
        ("K", 2),
        ("W", 2),
        ("PE", 2),
        ("YE", 2),
    ] {
        let mut r = base().hand(0, &[ID]).security(0, &["Y"]).start();
        main_phase(&mut r);
        let h = r.place_on_field(0, src, Some(0));
        let o = r.place_on_field(1, "Y", Some(0));
        let a = encode_digivolve(0, h.index as u16);
        assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0, "{src}");
        r.game.decode_action(a, 0);
        if src == "PE" || src == "YE" {
            let v = r
                .pending_selection_view()
                .expect("choose both legal evolution routes");
            let labels = v.effect_choices.unwrap();
            assert_eq!(labels.len(), 2);
            let i = labels
                .iter()
                .position(|x| x.label.ends_with("cost 2"))
                .unwrap();
            r.execute_branch(i).unwrap();
        }
        assert_eq!(r.memory(), 10 - cost, "{src}");
        optional(&r);
        r.accept_optional_trigger().unwrap();
        assert_eq!(r.effective_dp(o), Some(5000));
        assert_eq!(r.security_count(0), 0);
    }
}
#[test]
fn illegal_color_and_level_routes_masked() {
    for src in ["BAD", "LV4"] {
        let mut r = base().hand(0, &[ID]).start();
        main_phase(&mut r);
        let h = r.place_on_field(0, src, Some(0));
        assert_eq!(
            build_action_mask(&r.game, 0)[encode_digivolve(0, h.index as u16) as usize],
            0.0
        );
    }
}
#[test]
fn declined_or_unpayable_onplay_and_evolution_never_mill_or_debuff() {
    for evo in [false, true] {
        for payable in [false, true] {
            let mut r = base()
                .hand(0, &[ID])
                .security(0, if payable { &["Y"][..] } else { &[][..] })
                .start();
            main_phase(&mut r);
            let o = r.place_on_field(1, "Y", Some(0));
            let a = if evo {
                let h = r.place_on_field(0, "P", Some(0));
                encode_digivolve(0, h.index as u16)
            } else {
                0
            };
            let n = r.game.players[0].deck.len();
            r.game.decode_action(a, 0);
            if payable {
                optional(&r);
                r.decline_optional_trigger().unwrap();
            }
            assert!(r.pending_selection().is_none());
            assert_eq!(r.game.players[0].deck.len(), n - usize::from(evo));
            assert_eq!(r.effective_dp(o), Some(8000));
            assert_eq!(r.security_count(0), usize::from(payable));
        }
    }
}
#[test]
fn short_or_empty_deck_still_reduces_dp_and_zero_dp_dies() {
    for count in [0, 1, 2] {
        let mut r = base()
            .hand(0, &[ID])
            .security(0, &["P"])
            .deck(0, &vec!["Y"; count])
            .start();
        main_phase(&mut r);
        let o = r.place_on_field(1, "Y", Some(0));
        r.place_on_field(1, "SMALL", Some(0));
        r.game.decode_action(0, 0);
        r.accept_optional_trigger().unwrap();
        assert_eq!(r.game.players[0].deck.len(), 0);
        assert_eq!(r.game.players[0].trash.len(), count + 1);
        assert_eq!(r.effective_dp(o), Some(5000));
        assert_eq!(r.battle_area_size(1), 1);
    }
}
fn trigger(r: &mut DebugRunner, h: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
#[test]
fn parent_tail_finishes_before_two_milled_physical_wizardmon_triggers_each_once() {
    let mut r = base().security(0, &["Y"]).deck(0, &["P", ID, ID]).start();
    let h = r.place_on_field(0, ID, Some(0));
    let a = r.place_on_field(1, "Y", Some(0));
    let b = r.place_on_field(1, "P", Some(0));
    r.place_on_field(1, "T", Some(0));
    r.place_in_breeding(1, "Y");
    r.place_on_field(0, "Y", Some(0));
    trigger(&mut r, h);
    r.accept_optional_trigger().unwrap();
    assert_eq!(
        r.effective_dp(a),
        Some(5000),
        "parent mass DP applied before derived prompt"
    );
    // Two simultaneous copies expose trigger-order choice before mandatory targets.
    if r.pending_selection_view().unwrap().kind
        == digimon_engine::selection::SelectionKind::TriggerOrder
    {
        let v = r.pending_selection_view().unwrap();
        r.execute_action(v.selecting_player, v.valid_action_ids[0])
            .unwrap();
    }
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids.len(),
        2
    );
    pick(&mut r, 0);
    if r.pending_selection_view().unwrap().kind
        == digimon_engine::selection::SelectionKind::TriggerOrder
    {
        let v = r.pending_selection_view().unwrap();
        r.execute_action(v.selecting_player, v.valid_action_ids[0])
            .unwrap();
    }
    pick(&mut r, 1);
    assert!(r.pending_selection().is_none());
    assert_eq!(sec(&r, a), -1);
    assert_eq!(sec(&r, b), -1);
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

struct MoveEffect {
    owner: u8,
    mode: u8,
}
impl digimon_engine::effect::CardEffect for MoveEffect {
    fn effects(
        &self,
        c: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        let owner = self.owner;
        let mode = self.mode;
        vec![digimon_engine::effect::Effect::on_play(c)
            .process(move |ctx| match mode {
                0 => {
                    ctx.trash_from_top(owner, 1);
                }
                1 => {
                    ctx.trash_top_security(owner);
                }
                2 => {
                    ctx.draw(owner, 1);
                }
                3 => {
                    for h in ctx.reveal_top_deck(owner, 1) {
                        ctx.trash_from_reveal(owner, h);
                    }
                }
                4 => {
                    ctx.trash_from_hand_by_index(owner, 0);
                }
                5 => {
                    ctx.trash_bottom_sources(
                        PermanentHandle {
                            player: owner,
                            index: 0,
                        },
                        1,
                    );
                }
                6 => {
                    ctx.play_from_hand_free(owner, 0).unwrap();
                }
                _ => unreachable!(),
            })
            .build()]
    }
}
#[test]
fn exact_card_deck_and_security_effect_trash_mandatory_targets_both_owners_and_turns() {
    for owner in [0u8, 1] {
        for turn in [0usize, 1] {
            for mode in [0, 1] {
                let mut r = base()
                    .deck(owner, &["P", ID])
                    .security(owner, &["P", ID])
                    .start();
                r.game.turn_player_idx = turn;
                main_phase(&mut r);
                let a = r.place_on_field(1 - owner, "Y", Some(0));
                let b = r.place_on_field(1 - owner, "P", Some(0));
                let own = r.place_on_field(owner, "Y", Some(0));
                r.place_on_field(1 - owner, "T", Some(0));
                r.place_in_breeding(1 - owner, "Y");
                // An unrelated copy already in trash must not double-trigger.
                r.add_to_hand(owner, ID);
                let c = r.game.players[owner as usize].hand.pop().unwrap();
                r.game.players[owner as usize].put_in_trash(c);
                let expected = if mode == 0 {
                    r.game.players[owner as usize].deck.last().unwrap().handle()
                } else {
                    r.game.players[owner as usize]
                        .security
                        .last()
                        .unwrap()
                        .handle()
                };
                r.register_effect("T", std::sync::Arc::new(MoveEffect { owner, mode }));
                let h = r.place_on_field(owner, "T", Some(0));
                trigger(&mut r, h);
                let v = r.pending_selection_view().expect("one trigger only");
                assert_eq!(v.selecting_player, owner);
                assert_eq!(v.valid_action_ids.len(), 2);
                pick(&mut r, 1);
                assert!(r.pending_selection().is_none());
                assert_eq!(sec(&r, a), 0);
                assert_eq!(sec(&r, b), -1);
                assert_eq!(sec(&r, own), 0);
                assert!(r.game.players[owner as usize]
                    .trash
                    .iter()
                    .any(|c| c.handle() == expected));
                if turn == owner as usize {
                    r.end_turn();
                    assert_eq!(sec(&r, b), -1);
                }
                r.end_turn();
                assert_eq!(sec(&r, b), 0);
            }
        }
    }
}
#[test]
fn draw_reveal_trash_hand_trash_and_source_trash_are_not_deck_or_security_trash() {
    for mode in [2, 3, 4, 5] {
        let mut r = base().deck(0, &["P", ID]).hand(0, &[ID]).start();
        let opp = r.place_on_field(1, "Y", Some(0));
        if mode == 5 {
            r.place_stack(0, &[ID, "Y"]);
        }
        r.register_effect("T", std::sync::Arc::new(MoveEffect { owner: 0, mode }));
        let h = r.place_on_field(0, "T", Some(0));
        trigger(&mut r, h);
        assert!(r.pending_selection().is_none(), "mode {mode}");
        assert_eq!(sec(&r, opp), 0);
    }
}
#[test]
fn checked_security_wizardmon_does_not_fire_discard_trigger() {
    let mut r = base().security(1, &[ID]).start();
    main_phase(&mut r);
    let h = r.place_on_field(0, "BIG", Some(0));
    let o = r.place_on_field(0, "Y", Some(0));
    r.attack_player(h, 1, false);
    r.auto_resolve().unwrap();
    assert_eq!(r.security_count(1), 0);
    assert_eq!(sec(&r, h), 0);
    assert_eq!(sec(&r, o), 0);
}
struct RedirectSecurity;
impl digimon_engine::effect::CardEffect for RedirectSecurity {
    fn effects(
        &self,
        c: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        vec![digimon_engine::effect::Effect::when_would_be_trashed(c)
            .replacement_process(|ctx| ctx.redirect_to(digimon_engine::enums::Zone::Hand))
            .build()]
    }
}
#[test]
fn canceled_or_redirected_security_cost_never_mills_or_debuffs_real_play_and_evolution() {
    for evo in [false, true] {
        for redirect in [false, true] {
            let mut r = base().hand(0, &[ID]).security(0, &["Y"]).start();
            main_phase(&mut r);
            let opp = r.place_on_field(1, "Y", Some(0));
            let paid = r.game.players[0].security.last().unwrap().handle();
            if redirect {
                r.register_effect("T", std::sync::Arc::new(RedirectSecurity));
            } else {
                r.register_effect("T", std::sync::Arc::new(SecurityGuard { optional: false }));
            }
            r.place_on_field(0, "T", Some(0));
            let a = if evo {
                let h = r.place_on_field(0, "P", Some(0));
                encode_digivolve(0, h.index as u16)
            } else {
                0
            };
            let n = r.game.players[0].deck.len();
            r.game.decode_action(a, 0);
            r.accept_optional_trigger().unwrap();
            r.auto_resolve().unwrap();
            assert_eq!(r.game.players[0].deck.len(), n - usize::from(evo));
            assert_eq!(r.effective_dp(opp), Some(8000));
            assert!(!r.game.players[0].trash.iter().any(|c| c.handle() == paid));
            if redirect {
                assert!(r.game.players[0].hand.iter().any(|c| c.handle() == paid));
            } else {
                assert_eq!(r.security_count(0), 1);
            }
        }
    }
}
#[test]
fn repeated_payment_no_once_per_turn_and_floating_debuff_survives_source_leaving() {
    let mut r = base().security(0, &["Y", "P"]).start();
    let h = r.place_on_field(0, ID, Some(0));
    let o = r.place_on_field(1, "BIG", Some(0));
    for timing in [EffectTiming::OnPlay, EffectTiming::WhenDigivolving] {
        r.game
            .enqueue_triggered(timing, TriggerSource::Permanent(h));
        r.game.drain_effect_queue();
        optional(&r);
        r.accept_optional_trigger().unwrap();
    }
    assert_eq!(r.effective_dp(o), Some(9000));
    r.game.delete_permanent_with_effects(h);
    r.auto_resolve().unwrap();
    assert_eq!(r.effective_dp(o), Some(9000));
    r.add_to_hand(1, "BIG");
    r.register_effect("T", std::sync::Arc::new(MoveEffect { owner: 1, mode: 6 }));
    let e = r.place_on_field(0, "T", Some(0));
    trigger(&mut r, e);
    assert_eq!(r.effective_dp(r.perm_handle(1, 1)), Some(9000));
}

#[test]
fn own_security_payment_wizardmon_waits_for_parent_mill_and_mass_dp() {
    let mut r = base().hand(0, &[ID]).security(0, &[ID]).start();
    main_phase(&mut r);
    let o = r.place_on_field(1, "Y", Some(0));
    let n = r.game.players[0].deck.len();
    r.game.decode_action(0, 0);
    r.accept_optional_trigger().unwrap();
    assert_eq!(r.game.players[0].deck.len(), n - 2);
    assert_eq!(r.effective_dp(o), Some(5000));
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids.len(),
        1
    );
    pick(&mut r, 0);
    assert_eq!(sec(&r, o), -1);
    assert!(r.pending_selection().is_none());
}
#[test]
fn no_opposing_digimon_deck_security_and_paid_play_finish_without_illegal_target() {
    for mode in [0, 1] {
        let mut r = base().deck(0, &["Y", ID]).security(0, &[ID]).start();
        r.place_on_field(1, "T", Some(0));
        r.place_in_breeding(1, "Y");
        r.register_effect("Y", std::sync::Arc::new(MoveEffect { owner: 0, mode }));
        let h = r.place_on_field(0, "Y", Some(0));
        trigger(&mut r, h);
        assert!(r.pending_selection().is_none());
    }
    let mut r = base().hand(0, &[ID]).security(0, &["Y"]).start();
    main_phase(&mut r);
    let n = r.game.players[0].deck.len();
    r.game.decode_action(0, 0);
    r.accept_optional_trigger().unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[0].deck.len(), n - 2);
}
