//! P-153 MagnaGarurumon: official observation 6fbee7bf, verified card image.
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::TriggerSource;

#[test]
fn p_153_production_card_loads() {
    DebugRunner::builder()
        .dsl_card("P-153")
        .expect("P-153 production YAML");
}

fn runner() -> DebugRunner {
    let mut b = DebugRunner::builder().dsl_card("P-153").unwrap();
    for (id, level) in [("L3", 3), ("L4", 4), ("L5", 5), ("L6", 6), ("FILL", 3)] {
        let mut c = make_test_card(id, id);
        c.level = Some(level);
        c.dp = Some(1000);
        b = b.add_card(c);
    }
    let mut t = make_test_card("TAMER", "Tamer");
    t.card_kind = digimon_engine::enums::CardKind::Tamer;
    t.level = None;
    let mut enemy = make_test_card("BIG", "Big");
    enemy.dp = Some(15000);
    b.add_card(enemy)
        .add_card(t)
        .security(0, &["FILL"])
        .security(1, &["FILL"])
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .memory(10)
        .start()
}
fn fire(r: &mut DebugRunner, h: digimon_engine::permanent::PermanentHandle, timing: EffectTiming) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
fn pick(r: &mut DebugRunner, index: usize) {
    let p = r.pending_selection().expect("player-visible choice");
    let a = p.valid_action_ids[index];
    let player = p.selecting_player;
    let mask = digimon_engine::action::mask::build_action_mask(&r.game, player);
    assert!(mask[a as usize] > 0.0);
    r.execute_action(player, a).unwrap();
}
#[test]
fn p_153_grouped_return_picks_each_level_before_any_departure() {
    let mut r = runner();
    let h = r.place_stack(0, &["L5", "P-153"]);
    for id in ["L3", "L3", "L4", "L4", "L5", "L5", "L6", "TAMER"] {
        r.place_stack(1, &[id]);
    }
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    for _ in 0..2 {
        assert_eq!(r.pending_selection().unwrap().valid_action_ids.len(), 2);
        pick(&mut r, 1);
        assert_eq!(r.game.players[1].battle_area.len(), 8);
    }
    assert_eq!(r.pending_selection().unwrap().valid_action_ids.len(), 2);
    pick(&mut r, 1);
    assert_eq!(r.game.players[1].battle_area.len(), 5);
    let ids: Vec<_> = r.game.players[1]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data))
        .collect();
    assert_eq!(ids, vec!["L3", "L4", "L5"]);
}
#[test]
fn p_153_missing_levels_do_not_expand_group_to_unselected_targets() {
    for ids in [vec!["L6", "TAMER"], vec!["L4", "L6", "TAMER"]] {
        let mut r = runner();
        let h = r.place_stack(0, &["L5", "P-153"]);
        for id in &ids {
            r.place_stack(1, &[id]);
        }
        fire(&mut r, h, EffectTiming::WhenDigivolving);
        if ids.contains(&"L4") {
            pick(&mut r, 0);
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(r.game.players[1].battle_area.len(), 2);
    }
}
#[test]
fn p_153_end_attack_places_exact_top_card_and_unsuspends_underlying_digimon_or_tamer() {
    for under in ["L5", "TAMER"] {
        let mut r = runner();
        let h = r.place_stack(0, &[under, "P-153"]);
        let original = r.game.players[0].battle_area[h.index as usize]
            .top_card()
            .handle();
        r.game.players[0].battle_area[h.index as usize].is_suspended = true;
        fire(&mut r, h, EffectTiming::EndOfAttack);
        assert!(r.pending_selection().is_some());
        r.accept_optional_trigger().unwrap();
        r.auto_resolve().unwrap();
        assert_eq!(
            r.game.players[0].security.last().unwrap().handle(),
            original
        );
        assert_eq!(r.game.players[0].security.len(), 2);
        let p = &r.game.players[0].battle_area[h.index as usize];
        assert_eq!(p.top_card().card_id(&r.game.card_data), under);
        assert!(!p.is_suspended);
    }
}
#[test]
fn p_153_end_attack_can_decline_and_single_card_has_no_payment() {
    for stack in [vec!["L5", "P-153"], vec!["P-153"]] {
        let mut r = runner();
        let h = r.place_stack(0, &stack);
        r.game.players[0].battle_area[h.index as usize].is_suspended = true;
        fire(&mut r, h, EffectTiming::EndOfAttack);
        if stack.len() == 2 {
            assert!(r.pending_selection().is_some());
            r.decline_optional_trigger().unwrap();
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(r.game.players[0].security.len(), 1);
        assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
    }
}
#[test]
fn p_153_end_attack_security_prohibition_prevents_cost_and_unsuspend() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut r = runner();
    let h = r.place_stack(0, &["L5", "P-153"]);
    r.game.players[0].battle_area[0].is_suspended = true;
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::CannotAddSecurityByEffect,
            1,
            Expiry::Permanent,
            None,
            1,
        ),
    );
    fire(&mut r, h, EffectTiming::EndOfAttack);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[0].security.len(), 1);
    assert!(r.game.players[0].battle_area[0].is_suspended);
}
#[test]
fn p_153_armor_purge_real_battle_can_accept_or_decline() {
    use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
    for accept in [true, false] {
        let mut r = runner();
        let attacker = r.place_stack(0, &["L5", "P-153"]);
        let defender = r.place_stack(1, &["BIG"]);
        r.game.players[1].battle_area[0].is_suspended = true;
        let original = r.game.players[0].battle_area[0].top_card().handle();
        let _ = r.attack_digimon(attacker, defender, true);
        let p = r
            .pending_selection()
            .expect("Armor Purge replacement prompt");
        assert!(p.valid_action_ids.contains(&REPLACEMENT_ACCEPT));
        r.execute_action(0, if accept { REPLACEMENT_ACCEPT } else { PASS })
            .unwrap();
        r.auto_resolve().unwrap();
        assert_eq!(r.game.players[0].battle_area.len(), usize::from(accept));
        assert!(r.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == original));
        if accept {
            assert_eq!(
                r.game.players[0].battle_area[0]
                    .top_card()
                    .card_id(&r.game.card_data),
                "L5"
            );
        }
    }
}

#[test]
fn p_153_end_attack_clone_keeps_physical_source_and_sibling_identity() {
    let mut r = runner();
    let h = r.place_stack(0, &["TAMER", "P-153"]);
    let sibling = r.place_stack(0, &["L5", "P-153"]);
    let original = r.game.players[0].battle_area[0].top_card().handle();
    let untouched = r.game.players[0].battle_area[sibling.index as usize]
        .top_card()
        .handle();
    r.game.players[0].battle_area[0].is_suspended = true;
    fire(&mut r, h, EffectTiming::EndOfAttack);
    let mut clone = DebugRunner::wrap(r.game.clone());
    for g in [&mut r, &mut clone] {
        g.accept_optional_trigger().unwrap();
        g.auto_resolve().unwrap();
        assert_eq!(
            g.game.players[0].security.last().unwrap().handle(),
            original
        );
        assert_eq!(
            g.game.players[0].battle_area[sibling.index as usize]
                .top_card()
                .handle(),
            untouched
        );
        assert_eq!(
            g.game.players[0].battle_area[0]
                .top_card()
                .card_id(&g.game.card_data),
            "TAMER"
        );
        assert!(!g.game.players[0].battle_area[0].is_suspended);
    }
}
#[test]
fn p_153_printed_metadata_and_all_digivolution_paths() {
    let r = runner();
    let c = r.compiled_card("P-153").unwrap();
    assert_eq!(c.name, "MagnaGarurumon");
    assert_eq!(c.form.as_deref(), Some("Hybrid"));
    assert_eq!(c.level, Some(6));
    assert_eq!(c.dp, Some(12000));
    assert_eq!(c.cost, Some(12));
    assert_eq!(c.alt_paths.len(), 3);
    assert_eq!(
        c.alt_paths
            .iter()
            .map(|p| match p.cost {
                Some(digimon_dsl::compiled::CompiledCost::Literal(n)) => n,
                _ => panic!("literal printed cost"),
            })
            .collect::<Vec<_>>(),
        vec![4, 4, 2]
    );
}

// Test-only replacement witness: prevents the actual payment, independently of
// the card's activation-time owner_can_add_security affordance check.
struct CancelSecurityPlacement {
    optional: bool,
}
impl digimon_engine::effect::CardEffect for CancelSecurityPlacement {
    fn effects(
        &self,
        c: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        let b =
            digimon_engine::effect::EffectBuilder::new(c, EffectTiming::WhenWouldPlaceInSecurity)
                .replacement_process(|ctx| ctx.cancel());
        vec![if self.optional {
            b.optional().build()
        } else {
            b.build()
        }]
    }
}
#[test]
fn p_153_prevented_payment_never_unsuspends_and_pending_clones_resume_correctly() {
    use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
    for optional in [false, true] {
        let mut r = runner();
        let h = r.place_stack(0, &["TAMER", "P-153"]);
        r.game.players[0].battle_area[0].is_suspended = true;
        r.register_effect(
            "L6",
            std::sync::Arc::new(CancelSecurityPlacement { optional }),
        );
        r.place_stack(0, &["L6"]);
        fire(&mut r, h, EffectTiming::EndOfAttack);
        r.accept_optional_trigger().unwrap();
        if optional {
            assert!(r.pending_selection().is_some());
            assert!(
                r.game.players[0].battle_area[0].is_suspended,
                "tail must wait for payment"
            );
            let saved = r.game.clone();
            for (action, paid) in [(REPLACEMENT_ACCEPT, false), (PASS, true)] {
                r.game = saved.clone();
                assert!(
                    digimon_engine::action::mask::build_action_mask(&r.game, 0)[action as usize]
                        > 0.0
                );
                r.execute_action(0, action).unwrap();
                r.auto_resolve().unwrap();
                assert_eq!(r.game.players[0].security.len(), 1 + usize::from(paid));
                assert_eq!(r.game.players[0].battle_area[0].is_suspended, !paid);
                assert_eq!(
                    r.game.players[0].battle_area[0].card_sources.len(),
                    if paid { 1 } else { 2 }
                );
            }
        } else {
            r.auto_resolve().unwrap();
            assert_eq!(r.game.players[0].security.len(), 1);
            assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 2);
            assert!(
                r.game.players[0].battle_area[0].is_suspended,
                "unpaid cost cannot unsuspend"
            );
        }
    }
}
#[test]
fn p_153_foreign_owned_top_goes_to_its_actual_owners_top_security() {
    let mut r = runner();
    let h = r.place_stack(0, &["TAMER", "P-153"]);
    r.game.players[0].battle_area[0].is_suspended = true;
    r.game.players[0].battle_area[0]
        .card_sources
        .last_mut()
        .unwrap()
        .owner = 1;
    let top = r.game.players[0].battle_area[0].top_card().handle();
    fire(&mut r, h, EffectTiming::EndOfAttack);
    r.accept_optional_trigger().unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].security.len(), 1);
    assert_eq!(r.game.players[1].security.len(), 2);
    assert_eq!(r.game.players[1].security.last().unwrap().handle(), top);
    assert!(!r.game.players[0].battle_area[0].is_suspended);
}

#[test]
fn p_153_foreign_top_cost_requires_both_controller_and_recipient_security_permission() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    for actor in [0, 1] {
        let recipient = 1 - actor;
        for forbidden in [None, Some(actor), Some(recipient)] {
            let mut r = runner();
            r.set_first_player(actor);
            let host = r.place_stack(actor, &["TAMER", "P-153"]);
            let permanent = &mut r.game.players[actor as usize].battle_area[host.index as usize];
            permanent.is_suspended = true;
            permanent.card_sources.last_mut().unwrap().owner = recipient;
            let paid_card = permanent.top_card().handle();
            if let Some(player) = forbidden {
                r.game.modifiers.add_player_modifier(
                    player,
                    PlayerModifierEntry::simple(
                        ModifierType::CannotAddSecurityByEffect,
                        1,
                        Expiry::Permanent,
                        None,
                        1,
                    ),
                );
            }
            fire(&mut r, host, EffectTiming::EndOfAttack);
            let payable = forbidden.is_none();
            assert_eq!(
                r.pending_selection().is_some(),
                payable,
                "actor {actor}, physical top recipient {recipient}, prohibited {forbidden:?}"
            );
            if payable {
                let pending = r.pending_selection().unwrap();
                let chooser = pending.selecting_player;
                assert_eq!(chooser, actor);
                let mask = digimon_engine::action::build_action_mask(&r.game, actor);
                for &action in &pending.valid_action_ids {
                    assert_eq!(mask[action as usize], 1.0);
                }
                r.accept_optional_trigger().unwrap();
                r.auto_resolve().unwrap();
                assert_eq!(
                    r.game.players[recipient as usize]
                        .security
                        .last()
                        .unwrap()
                        .handle(),
                    paid_card
                );
                assert!(
                    !r.game.players[actor as usize].battle_area[host.index as usize].is_suspended
                );
                assert_eq!(r.game.players[recipient as usize].security.len(), 2);
            } else {
                assert_eq!(
                    r.game.players[actor as usize].battle_area[host.index as usize]
                        .top_card()
                        .handle(),
                    paid_card
                );
                assert!(
                    r.game.players[actor as usize].battle_area[host.index as usize].is_suspended
                );
                assert_eq!(r.game.players[recipient as usize].security.len(), 1);
            }
        }
    }
}

#[test]
fn p_153_paid_pending_tail_follows_carrier_after_field_compaction() {
    use digimon_engine::action::space::PASS;
    let mut r = runner();
    r.place_stack(0, &["L6"]);
    let h = r.place_stack(0, &["TAMER", "P-153"]);
    r.game.players[0].battle_area[1].is_suspended = true;
    r.register_effect(
        "L6",
        std::sync::Arc::new(CancelSecurityPlacement { optional: true }),
    );
    fire(&mut r, h, EffectTiming::EndOfAttack);
    r.accept_optional_trigger().unwrap();
    assert!(r.pending_selection().is_some());
    // Fixture removal models an interrupt compacting a lower field slot.
    r.game.players[0].battle_area.remove(0);
    let saved = r.game.clone();
    for _ in 0..2 {
        r.game = saved.clone();
        assert!(digimon_engine::action::mask::build_action_mask(&r.game, 0)[PASS as usize] > 0.0);
        r.execute_action(0, PASS).unwrap();
        r.auto_resolve().unwrap();
        assert_eq!(r.game.players[0].security.len(), 2);
        assert_eq!(
            r.game.players[0].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            "TAMER"
        );
        assert!(!r.game.players[0].battle_area[0].is_suspended);
    }
}

#[test]
fn p_153_legal_route_masks_and_actual_payment_enforce_exactly_three_colors() {
    use digimon_engine::{
        action::{build_action_mask, space::encode_digivolve},
        enums::CardColor,
    };
    for (level, colors, allowed, cost) in [
        (5, vec![CardColor::Yellow], true, 4),
        (5, vec![CardColor::Blue], true, 4),
        (5, vec![CardColor::Red], false, 0),
        (6, vec![CardColor::Yellow, CardColor::Blue], false, 0),
        (
            6,
            vec![CardColor::Yellow, CardColor::Blue, CardColor::Red],
            true,
            2,
        ),
        (
            6,
            vec![
                CardColor::Yellow,
                CardColor::Blue,
                CardColor::Red,
                CardColor::Purple,
            ],
            false,
            0,
        ),
    ] {
        let mut base = make_test_card("TEST-MAGNA-BASE", "MagnaGarurumon");
        base.level = Some(level);
        base.colors = colors;
        let mut r = DebugRunner::builder()
            .dsl_card("P-153")
            .unwrap()
            .add_card(base)
            .add_card(make_test_card("TEST-DRAW", "Draw"))
            .hand(0, &["P-153"])
            .deck(0, &["TEST-DRAW"; 5])
            .memory(10)
            .start();
        r.skip_mulligan();
        r.place_on_field(0, "TEST-MAGNA-BASE", None);
        let action = encode_digivolve(0, 0);
        assert_eq!(
            build_action_mask(&r.game, 0)[action as usize] > 0.0,
            allowed
        );
        if allowed {
            r.game.decode_action(action, 0);
            r.auto_resolve().unwrap();
            assert_eq!(r.memory(), 10 - cost);
            assert_eq!(
                r.game.players[0].battle_area[0]
                    .top_card()
                    .card_id(&r.game.card_data),
                "P-153"
            );
        }
    }
}
