//! BT18-042 MagnaGarurumon: official observation 29a8a226 and inspected image.
use digimon_engine::{
    action::{build_action_mask, space::PASS},
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    effect::{CardEffect, Effect, EffectBuilder},
    enums::{CardKind, EffectTiming, Zone},
    owned_card_choice::CardChoiceOperation,
    permanent::PermanentHandle,
    selection::TriggerSource,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

fn runner() -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card("BT18-042")
        .expect("real production YAML");
    for (id, level) in [("M-L3", 3), ("M-L4", 4), ("M-L5", 5), ("M-L6", 6)] {
        let mut card = make_test_card(id, id);
        card.level = Some(level);
        card.dp = Some(1000);
        b = b.add_card(card);
    }
    let mut tamer = make_test_card("M-TAMER", "Koji Minamoto");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    let mut r = b
        .add_card(tamer)
        .add_card(make_test_card("M-OBSERVER", "Observer"))
        .security(0, &["M-L3", "M-L4"])
        .security(1, &["M-L3", "M-L4"])
        .deck(0, &["M-L3"; 10])
        .deck(1, &["M-L3"; 10])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.memory = 10;
    r
}
fn fire(r: &mut DebugRunner, host: PermanentHandle, timing: EffectTiming) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(host));
    r.game.drain_effect_queue();
}
fn action(r: &mut DebugRunner, a: u16) {
    let p = r
        .pending_selection()
        .expect("visible legal decision")
        .selecting_player;
    assert_eq!(build_action_mask(&r.game, p)[a as usize], 1.0);
    r.execute_action(p, a).unwrap();
}
fn accept(r: &mut DebugRunner) {
    let a = *r
        .pending_selection()
        .unwrap()
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .unwrap();
    action(r, a);
}
fn choose_index(r: &mut DebugRunner, index: usize) {
    let mut visited = std::collections::HashSet::new();
    loop {
        let view = r
            .pending_selection_view()
            .unwrap()
            .card_choice
            .expect("owned material page");
        assert!(
            visited.insert(view.page_index.clone()),
            "Next must advance the physical-card page"
        );
        for row in &view.rows {
            assert_eq!(
                build_action_mask(&r.game, r.pending_selection().unwrap().selecting_player)
                    [row.action_id as usize],
                1.0
            );
        }
        if let Some(row) = view.rows.iter().find(|row| {
            matches!(row.operation, CardChoiceOperation::CommitCard)
                && row.position.as_deref() == Some(&index.to_string())
        }) {
            assert!(row.identity_visible);
            action(r, row.action_id);
            return;
        }
        let next = view
            .rows
            .iter()
            .find(|row| matches!(row.operation, CardChoiceOperation::NextPage))
            .expect("requested source reachable")
            .action_id;
        action(r, next);
    }
}
#[test]
fn bt18_042_production_card_loads() {
    let r = runner();
    let c = r.compiled_card("BT18-042").unwrap();
    assert_eq!(c.name, "MagnaGarurumon");
    assert_eq!(c.level, Some(6));
    assert_eq!(c.cost, Some(13));
    assert_eq!(c.dp, Some(13000));
    for trait_name in ["Hybrid", "Variable", "Cyborg"] {
        assert!(c.all_traits().contains(&trait_name.into()));
    }
}
#[test]
fn bt18_042_places_selected_source_bottom_deletes_all_same_level_both_seats_and_timings() {
    for seat in [0, 1] {
        for timing in [
            EffectTiming::WhenDigivolving,
            EffectTiming::EndOfOpponentsTurn,
        ] {
            let mut r = runner();
            let host = r.place_stack(seat, &["M-L3", "M-TAMER", "M-L4", "BT18-042"]);
            for id in ["M-L3", "M-L4", "M-L4", "M-L5", "M-TAMER"] {
                r.place_stack(1 - seat, &[id]);
            }
            let paid = r.game.players[seat as usize].battle_area[0].card_sources[2].handle();
            fire(&mut r, host, timing);
            accept(&mut r);
            choose_index(&mut r, 2);
            assert!(r.pending_selection().is_none());
            assert_eq!(
                r.game.players[seat as usize]
                    .security
                    .first()
                    .unwrap()
                    .handle(),
                paid
            );
            let remaining: Vec<_> = r.game.players[(1 - seat) as usize]
                .battle_area
                .iter()
                .map(|p| p.top_card().card_id(&r.game.card_data))
                .collect();
            assert_eq!(remaining, ["M-L3", "M-L5", "M-TAMER"]);
        }
    }
}
#[test]
fn bt18_042_every_deep_physical_source_is_reachable_after_clone() {
    for depth in [13, 39] {
        for index in 0..depth {
            let mut r = runner();
            let mut stack = vec!["M-L4"; depth];
            stack.push("BT18-042");
            let host = r.place_stack(0, &stack);
            let paid = r.game.players[0].battle_area[0].card_sources[index].handle();
            fire(&mut r, host, EffectTiming::WhenDigivolving);
            accept(&mut r);
            r.game = r.game.clone();
            choose_index(&mut r, index);
            assert_eq!(r.game.players[0].security.first().unwrap().handle(), paid);
            assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), depth);
        }
    }
}
#[test]
fn bt18_042_outer_decline_preserves_opt_but_accepted_cost_decline_spends_it() {
    let mut r = runner();
    let host = r.place_stack(0, &["M-L4", "BT18-042"]);
    fire(&mut r, host, EffectTiming::WhenDigivolving);
    action(&mut r, PASS);
    fire(&mut r, host, EffectTiming::EndOfOpponentsTurn);
    assert!(r.pending_selection().is_some());
    accept(&mut r);
    action(&mut r, PASS);
    assert_eq!(r.game.players[0].security.len(), 2);
    fire(&mut r, host, EffectTiming::WhenDigivolving);
    assert!(
        r.pending_selection().is_none(),
        "accepted processing uses shared OPT"
    );
}
#[test]
fn bt18_042_only_digimon_sources_pay_but_missing_reward_targets_do_not_block_payment() {
    let mut r = runner();
    let host = r.place_stack(0, &["M-TAMER", "BT18-042"]);
    fire(&mut r, host, EffectTiming::WhenDigivolving);
    assert!(r.pending_selection().is_none());
    let mut r = runner();
    let host = r.place_stack(0, &["M-L5", "BT18-042"]);
    fire(&mut r, host, EffectTiming::WhenDigivolving);
    accept(&mut r);
    choose_index(&mut r, 0);
    assert_eq!(r.game.players[0].security.len(), 3);
    fire(&mut r, host, EffectTiming::EndOfOpponentsTurn);
    assert!(r.pending_selection().is_none());
}
#[derive(Clone, Copy)]
enum Outcome {
    Cancel,
    Trash,
    Security,
    Custom,
}
struct Replace(Outcome);
impl CardEffect for Replace {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let outcome = self.0;
        vec![
            EffectBuilder::new(card, EffectTiming::WhenWouldPlaceInSecurity)
                .replacement_process(move |ctx| match outcome {
                    Outcome::Cancel => ctx.cancel(),
                    Outcome::Trash => ctx.redirect_to(Zone::Trash),
                    Outcome::Security => ctx.redirect_to(Zone::Security),
                    Outcome::Custom => ctx.handled(),
                })
                .build(),
        ]
    }
}
#[test]
fn bt18_042_only_completed_security_payment_deletes_failed_payment_spends_opt() {
    for (outcome, paid) in [
        (Outcome::Cancel, false),
        (Outcome::Trash, false),
        (Outcome::Security, true),
        (Outcome::Custom, false),
    ] {
        let mut r = runner();
        r.register_effect("M-OBSERVER", Arc::new(Replace(outcome)));
        let host = r.place_stack(0, &["M-L4", "M-L4", "BT18-042"]);
        r.place_stack(0, &["M-OBSERVER"]);
        r.place_stack(1, &["M-L4"]);
        fire(&mut r, host, EffectTiming::WhenDigivolving);
        accept(&mut r);
        choose_index(&mut r, 0);
        assert_eq!(r.game.players[1].battle_area.len(), usize::from(!paid));
        fire(&mut r, host, EffectTiming::EndOfOpponentsTurn);
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn bt18_042_source_payment_uses_card_owner_security() {
    let mut r = runner();
    let host = r.place_stack(0, &["M-L4", "BT18-042"]);
    r.game.players[0].battle_area[0].card_sources[0].owner = 1;
    let paid = r.game.players[0].battle_area[0].card_sources[0].handle();
    r.place_stack(1, &["M-L4"]);
    fire(&mut r, host, EffectTiming::WhenDigivolving);
    accept(&mut r);
    choose_index(&mut r, 0);
    assert_eq!(r.game.players[1].security.first().unwrap().handle(), paid);
    assert_eq!(r.game.players[0].security.len(), 2);
    assert!(r.game.players[1].battle_area.is_empty());
}
#[test]
fn bt18_042_attack_cost_adds_exact_top_unsuspends_and_shares_opt() {
    for seat in [0, 1] {
        for timing in [EffectTiming::OnAllyAttack, EffectTiming::OnOpponentAttack] {
            let mut r = runner();
            let host = r.place_stack(seat, &["BT18-042"]);
            r.game.players[seat as usize].battle_area[0].is_suspended = true;
            let top = r.game.players[seat as usize]
                .security
                .last()
                .unwrap()
                .handle();
            fire(&mut r, host, timing);
            accept(&mut r);
            assert!(r.game.players[seat as usize]
                .hand
                .iter()
                .any(|c| c.handle() == top));
            assert!(!r.game.players[seat as usize].battle_area[0].is_suspended);
            r.game.players[seat as usize].battle_area[0].is_suspended = true;
            fire(
                &mut r,
                host,
                if timing == EffectTiming::OnAllyAttack {
                    EffectTiming::OnOpponentAttack
                } else {
                    EffectTiming::OnAllyAttack
                },
            );
            assert!(r.pending_selection().is_none());
            assert!(r.game.players[seat as usize].battle_area[0].is_suspended);
            assert_eq!(r.game.players[seat as usize].security.len(), 1);
        }
    }
}
#[test]
fn bt18_042_attack_decline_preserves_opt_empty_security_never_unsuspends() {
    let mut r = runner();
    let host = r.place_stack(0, &["BT18-042"]);
    r.game.players[0].battle_area[0].is_suspended = true;
    fire(&mut r, host, EffectTiming::OnAllyAttack);
    action(&mut r, PASS);
    assert!(r.game.players[0].battle_area[0].is_suspended);
    assert_eq!(r.game.players[0].security.len(), 2);
    fire(&mut r, host, EffectTiming::OnOpponentAttack);
    accept(&mut r);
    assert!(!r.game.players[0].battle_area[0].is_suspended);
    let mut r = runner();
    let host = r.place_stack(0, &["BT18-042"]);
    r.game.players[0].security.clear();
    r.game.players[0].battle_area[0].is_suspended = true;
    fire(&mut r, host, EffectTiming::OnOpponentAttack);
    assert!(r.pending_selection().is_none());
    assert!(r.game.players[0].battle_area[0].is_suspended);
}
#[test]
fn bt18_042_unsuspend_cost_to_foreign_owners_hand_still_pays() {
    let mut r = runner();
    let host = r.place_stack(0, &["BT18-042"]);
    r.game.players[0].battle_area[0].is_suspended = true;
    r.game.players[0].security.last_mut().unwrap().owner = 1;
    let top = r.game.players[0].security.last().unwrap().handle();
    fire(&mut r, host, EffectTiming::OnAllyAttack);
    accept(&mut r);
    assert!(r.game.players[1].hand.iter().any(|c| c.handle() == top));
    assert!(!r.game.players[0].battle_area[0].is_suspended);
}

struct ObservePayment(Arc<AtomicUsize>);
impl CardEffect for ObservePayment {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        [EffectTiming::OnLoseSecurity, EffectTiming::OnAddToHand]
            .into_iter()
            .map(|timing| {
                let calls = self.0.clone();
                EffectBuilder::new(card, timing)
                    .process(move |ctx| {
                        let host = ctx.game.players[0]
                            .battle_area
                            .iter()
                            .find(|p| p.top_card().card_id(&ctx.game.card_data) == "BT18-042")
                            .unwrap();
                        assert!(
                            !host.is_suspended,
                            "security and hand observers wait until unsuspend finishes"
                        );
                        assert!(
                            ctx.game.players[0]
                                .hand
                                .iter()
                                .any(|c| c.card_id(&ctx.game.card_data) == "M-L4"),
                            "cost actually delivered before observer"
                        );
                        calls.fetch_add(1, Ordering::SeqCst);
                    })
                    .build()
            })
            .collect()
    }
}
#[test]
fn bt18_042_security_and_hand_observers_run_after_paid_unsuspend() {
    let mut r = runner();
    let host = r.place_stack(0, &["BT18-042"]);
    let calls = Arc::new(AtomicUsize::new(0));
    r.register_effect("M-OBSERVER", Arc::new(ObservePayment(calls.clone())));
    r.place_stack(0, &["M-OBSERVER"]);
    r.game.players[0].battle_area[0].is_suspended = true;
    fire(&mut r, host, EffectTiming::OnAllyAttack);
    accept(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "both observers resolved after the original body"
    );
}
#[test]
fn bt18_042_paying_without_possible_unsuspend_is_legal_and_consumes_opt() {
    use digimon_engine::{
        enums::{Expiry, ModifierType},
        modifiers::ModifierEntry,
    };
    for prohibited in [false, true] {
        let mut r = runner();
        let host = r.place_stack(0, &["BT18-042"]);
        r.game.players[0].battle_area[0].is_suspended = prohibited;
        if prohibited {
            r.game.modifiers.add(
                host,
                ModifierEntry::simple(ModifierType::CannotUnsuspend, 1, Expiry::Permanent, 1),
            );
        }
        fire(&mut r, host, EffectTiming::OnAllyAttack);
        accept(&mut r);
        assert_eq!(r.game.players[0].security.len(), 1);
        assert_eq!(r.game.players[0].battle_area[0].is_suspended, prohibited);
        fire(&mut r, host, EffectTiming::OnOpponentAttack);
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn bt18_042_security_add_prohibition_suppresses_unpayable_source_cost() {
    use digimon_engine::{
        enums::{Expiry, ModifierType},
        modifiers::PlayerModifierEntry,
    };
    let mut r = runner();
    let host = r.place_stack(0, &["M-L4", "BT18-042"]);
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
    fire(&mut r, host, EffectTiming::WhenDigivolving);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[0].security.len(), 2);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 2);
}
#[test]
fn bt18_042_printed_normal_and_named_five_hybrid_routes_use_legal_mask_and_cost_five() {
    use digimon_engine::{
        action::space::encode_digivolve,
        enums::{CardColor, GamePhase},
    };
    for (color, level, name, count, legal) in [
        (CardColor::Yellow, Some(5), "Normal", 0, true),
        (CardColor::Purple, Some(5), "Normal", 0, true),
        (CardColor::Black, Some(5), "Normal", 0, true),
        (CardColor::Blue, Some(5), "Normal", 0, false),
        (CardColor::Blue, None, "Koji Minamoto", 4, false),
        (CardColor::Blue, None, "Koji Minamoto", 5, true),
        (CardColor::Blue, None, "Koji Minamoto", 6, true),
    ] {
        let mut base = make_test_card("M-BASE", name);
        base.colors = vec![color];
        base.level = level;
        if level.is_none() {
            base.card_kind = CardKind::Tamer;
            base.dp = None;
        }
        let mut hybrid = make_test_card("M-HYBRID", "Hybrid");
        hybrid.traits = vec!["Hybrid".into()];
        let mut r = DebugRunner::builder()
            .dsl_card("BT18-042")
            .unwrap()
            .add_card(base)
            .add_card(hybrid)
            .add_card(make_test_card("M-DRAW", "Draw"))
            .hand(0, &["BT18-042"])
            .deck(0, &["M-DRAW"; 10])
            .memory(10)
            .start();
        r.skip_mulligan();
        let mut stack = vec!["M-HYBRID"; count];
        stack.push("M-BASE");
        r.place_stack(0, &stack);
        r.set_phase(GamePhase::Main);
        r.game.memory = 10;
        let a = encode_digivolve(0, 0);
        assert_eq!(
            build_action_mask(&r.game, 0)[a as usize] > 0.5,
            legal,
            "{name}/{color:?}/{count}"
        );
        if legal {
            r.game.decode_action(a, 0);
            if r.pending_selection().is_some() {
                action(&mut r, PASS);
            }
            assert_eq!(r.memory(), 5, "printed evolution cost");
            assert_eq!(
                r.game.players[0].battle_area[0]
                    .top_card()
                    .card_id(&r.game.card_data),
                "BT18-042"
            );
        }
    }
}

struct ObserveDeletionBatch(Arc<AtomicUsize>);
impl CardEffect for ObserveDeletionBatch {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let calls = self.0.clone();
        vec![Effect::on_deletion(card)
            .process(move |ctx| {
                assert!(
                    ctx.game.players[1]
                        .battle_area
                        .iter()
                        .all(|p| p.top_card().card_id(&ctx.game.card_data) != "M-L4"),
                    "all same-level targets leave before any deletion observer resolves"
                );
                calls.fetch_add(1, Ordering::SeqCst);
            })
            .build()]
    }
}
#[test]
fn bt18_042_same_level_deletions_complete_before_any_deletion_observer() {
    let mut r = runner();
    let calls = Arc::new(AtomicUsize::new(0));
    r.register_effect("M-L4", Arc::new(ObserveDeletionBatch(calls.clone())));
    let host = r.place_stack(0, &["M-L4", "BT18-042"]);
    r.place_stack(1, &["M-L4"]);
    r.place_stack(1, &["M-L4"]);
    r.place_stack(1, &["M-L5"]);
    fire(&mut r, host, EffectTiming::WhenDigivolving);
    accept(&mut r);
    choose_index(&mut r, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "both opponent deletion observers resolved"
    );
}

#[test]
fn bt18_042_face_down_digimon_has_no_characteristics_and_cannot_pay_source_cost() {
    // CR4-7-9: face-down cards under cards have no card information, even
    // though their owner may inspect them (4-7-10). Digimon-only is a filter.
    let mut r = runner();
    let host = r.place_stack(0, &["M-L4", "BT18-042"]);
    r.game.players[0].battle_area[0].card_sources[0].face_down = true;
    fire(&mut r, host, EffectTiming::WhenDigivolving);
    assert!(
        r.pending_selection().is_none(),
        "unknown kind cannot satisfy Digimon-card cost"
    );
    assert_eq!(r.game.players[0].security.len(), 2);
}
#[test]
fn bt18_042_face_down_hybrid_does_not_complete_five_hybrid_named_evolution_route() {
    use digimon_engine::{action::space::encode_digivolve, enums::GamePhase};
    let mut koji = make_test_card("M-KOJI", "Koji Minamoto");
    koji.card_kind = CardKind::Tamer;
    koji.level = None;
    koji.dp = None;
    let mut hybrid = make_test_card("M-HYBRID", "Hybrid");
    hybrid.traits = vec!["Hybrid".into()];
    let mut r = DebugRunner::builder()
        .dsl_card("BT18-042")
        .unwrap()
        .add_card(koji)
        .add_card(hybrid)
        .hand(0, &["BT18-042"])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.place_stack(
        0,
        &[
            "M-HYBRID", "M-HYBRID", "M-HYBRID", "M-HYBRID", "M-HYBRID", "M-KOJI",
        ],
    );
    r.game.players[0].battle_area[0].card_sources[0].face_down = true;
    r.set_phase(GamePhase::Main);
    assert_eq!(
        build_action_mask(&r.game, 0)[encode_digivolve(0, 0) as usize],
        0.0,
        "only four sources have referenceable Hybrid trait under CR4-7-9"
    );
}

#[test]
fn bt18_042_actual_ad1_023_name_alias_satisfies_named_hybrid_evolution() {
    use digimon_engine::{action::space::encode_digivolve, enums::GamePhase};
    let mut hybrid = make_test_card("M-ALIAS-HYBRID", "Hybrid");
    hybrid.traits = vec!["Hybrid".into()];
    let mut r = DebugRunner::builder()
        .dsl_card("BT18-042")
        .unwrap()
        .dsl_card("AD1-023")
        .unwrap()
        .add_card(hybrid)
        .hand(0, &["BT18-042"])
        .deck(0, &["M-ALIAS-HYBRID"; 10])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.place_stack(
        0,
        &[
            "M-ALIAS-HYBRID",
            "M-ALIAS-HYBRID",
            "M-ALIAS-HYBRID",
            "M-ALIAS-HYBRID",
            "M-ALIAS-HYBRID",
            "AD1-023",
        ],
    );
    r.set_phase(GamePhase::Main);
    r.game.memory = 10;
    let a = encode_digivolve(0, 0);
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    r.game.decode_action(a, 0);
    if r.pending_selection().is_some() {
        action(&mut r, PASS);
    }
    assert_eq!(r.memory(), 5);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "BT18-042"
    );
}

#[test]
fn bt18_042_source_cost_payability_checks_controller_when_host_card_has_foreign_owner() {
    use digimon_engine::{
        enums::{Expiry, ModifierType},
        modifiers::PlayerModifierEntry,
    };
    for forbidden in [0, 1] {
        let mut r = runner();
        let host = r.place_stack(0, &["M-L4", "BT18-042"]);
        r.game.players[0].battle_area[0]
            .card_sources
            .last_mut()
            .unwrap()
            .owner = 1;
        r.game.modifiers.add_player_modifier(
            forbidden,
            PlayerModifierEntry::simple(
                ModifierType::CannotAddSecurityByEffect,
                1,
                Expiry::Permanent,
                None,
                1,
            ),
        );
        fire(&mut r, host, EffectTiming::WhenDigivolving);
        assert_eq!(
            r.pending_selection().is_some(),
            forbidden != 0,
            "effect controller pays; physical host-card owner does not control this effect"
        );
        if forbidden == 1 {
            accept(&mut r);
            choose_index(&mut r, 0);
            assert_eq!(r.game.players[0].security.len(), 3);
        }
    }
}
