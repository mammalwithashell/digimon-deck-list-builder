//! LM-020: official revised text exempts this Digimon's own/inherited effects.
//! https://world.digimoncard.com/rule/revised/ ; current official bundle.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_attack, encode_digivolve, PASS},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{
    CardColor, CardKind, EffectSourceKind, EffectTiming, Expiry, GamePhase, Zone,
};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;
use std::sync::Arc;
const ID: &str = "LM-020";
fn base() -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder()
        .dsl_card(ID)
        .expect("production Quantumon");
    b = b
        .from_dsl_yaml(
            r#"
card: INH
name: Inherited Self Buff
kind: digimon
level: 3
color: [yellow]
cost: 0
dp: 1000
effects:
  - when: on_attack
    scope: inherited
    process:
      - add_dp_modifier: {target: source, value: 1000, expiry: end_of_turn}
"#,
        )
        .unwrap();
    for (id, kind, color, level) in [
        ("Y5", CardKind::Digimon, CardColor::Yellow, 5),
        ("G5", CardKind::Digimon, CardColor::Green, 5),
        ("B5", CardKind::Digimon, CardColor::Blue, 5),
        ("Y4", CardKind::Digimon, CardColor::Yellow, 4),
        ("D", CardKind::Digimon, CardColor::Yellow, 3),
        ("T", CardKind::Tamer, CardColor::Yellow, 0),
        ("O", CardKind::Option, CardColor::Yellow, 0),
        ("EGG", CardKind::DigiEgg, CardColor::Yellow, 2),
        ("TOKEN", CardKind::Token, CardColor::Yellow, 3),
    ] {
        let mut c = make_test_card(id, id);
        c.card_kind = kind;
        c.colors = vec![color];
        c.level = if level == 0 { None } else { Some(level) };
        c.dp = Some(9000);
        b = b.add_card(c);
    }
    b.deck(0, &["D"; 20])
        .deck(1, &["D"; 20])
        .security(0, &["D"])
        .security(1, &["D", "T", "O"])
        .memory(10)
}
fn phase(r: &mut DebugRunner) {
    r.game.current_phase = GamePhase::Main;
    r.game.turn_count = 2;
}
fn fire(r: &mut DebugRunner, h: PermanentHandle, t: EffectTiming) {
    r.game.enqueue_triggered(t, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
fn accept(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("optional by cost");
    assert!(v.is_optional);
    assert_eq!(
        build_action_mask(&r.game, v.selecting_player)[PASS as usize],
        1.0
    );
    r.accept_optional_trigger().unwrap();
}
fn choose(r: &mut DebugRunner, a: u16) {
    let v = r.pending_selection_view().unwrap();
    assert!(v.valid_action_ids.contains(&a));
    assert_eq!(
        build_action_mask(&r.game, v.selecting_player)[a as usize],
        1.0
    );
    r.execute_action(v.selecting_player, a).unwrap();
}
fn pay(r: &mut DebugRunner, h: PermanentHandle) {
    accept(r);
    let v = r.pending_selection_view().unwrap();
    assert!(!v.is_optional);
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
    choose(r, encode_attack(h.player as u16, h.index as u16));
}
fn finish_security(r: &mut DebugRunner, index: usize) {
    let v = r
        .pending_selection_view()
        .expect("all opponent security physical choices");
    assert!(!v.is_optional);
    assert_eq!(
        build_action_mask(&r.game, v.selecting_player)[PASS as usize],
        0.0
    );
    choose(r, v.valid_action_ids[index]);
    assert!(r.pending_selection().is_none());
}
#[test]
fn both_printed_normal_evolution_routes_pay_five_and_offer_decline() {
    for src in ["Y5", "G5"] {
        let mut r = base().hand(0, &[ID]).start();
        phase(&mut r);
        let h = r.place_on_field(0, src, Some(0));
        let a = encode_digivolve(0, h.index as u16);
        assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0, "{src}");
        r.game.decode_action(a, 0);
        assert_eq!(r.memory(), 5);
        accept(&mut r);
        assert!(r.pending_selection().is_some());
    }
}
#[test]
fn invalid_color_and_level_evolution_routes_are_masked() {
    for src in ["B5", "Y4"] {
        let mut r = base().hand(0, &[ID]).start();
        phase(&mut r);
        let h = r.place_on_field(0, src, Some(0));
        assert_eq!(
            build_action_mask(&r.game, 0)[encode_digivolve(0, h.index as u16) as usize],
            0.0
        );
    }
}
#[test]
fn placement_masks_both_sides_and_selected_physical_owner_routes_security() {
    for owner in [0u8, 1] {
        for controlled_by in [0u8, 1] {
            let mut r = base().start();
            let q = r.place_on_field(0, ID, Some(0));
            let payee = r.place_on_field(controlled_by, "D", Some(0));
            r.game.players[controlled_by as usize].battle_area[payee.index as usize]
                .card_sources
                .last_mut()
                .unwrap()
                .owner = owner;
            r.place_on_field(0, "T", Some(0));
            r.place_on_field(1, "T", Some(0));
            r.place_in_breeding(1, "D");
            let top = r.game.players[controlled_by as usize].battle_area[payee.index as usize]
                .top_card()
                .handle();
            let before = r.security_count(owner);
            fire(&mut r, q, EffectTiming::WhenDigivolving);
            accept(&mut r);
            let v = r.pending_selection_view().unwrap();
            assert_eq!(v.valid_action_ids.len(), 2);
            assert!(v
                .valid_action_ids
                .contains(&encode_attack(0, q.index as u16)));
            assert!(v
                .valid_action_ids
                .contains(&encode_attack(controlled_by as u16, payee.index as u16)));
            choose(
                &mut r,
                encode_attack(controlled_by as u16, payee.index as u16),
            );
            assert_eq!(r.security_count(owner), before + 1);
            assert_eq!(
                r.game.players[owner as usize]
                    .security
                    .last()
                    .unwrap()
                    .handle(),
                top
            );
            let sec = r.game.players[1].security.len();
            let selected = r.game.players[1].security[0].handle();
            assert_eq!(
                r.pending_selection_view().unwrap().valid_action_ids.len(),
                sec
            );
            finish_security(&mut r, 0);
            assert_eq!(r.game.players[1].deck.last().unwrap().handle(), selected);
            assert_eq!(r.game.players[1].security.len(), sec - 1);
        }
    }
}
#[test]
fn declining_cost_preserves_every_physical_zone() {
    let mut r = base().start();
    let q = r.place_on_field(0, ID, Some(0));
    let before = (r.security_count(0), r.security_count(1), r.deck_size(1));
    fire(&mut r, q, EffectTiming::WhenDigivolving);
    r.decline_optional_trigger().unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(
        (r.security_count(0), r.security_count(1), r.deck_size(1)),
        before
    );
    assert_eq!(r.battle_area_size(0), 1);
}
struct Guard {
    optional: bool,
    redirect: Option<Zone>,
}
impl CardEffect for Guard {
    fn effects(&self, c: digimon_engine::card_source::CardHandle) -> Vec<Effect> {
        let redirect = self.redirect;
        let e = Effect::when_would_leave_battle_area(c).replacement_process(move |ctx| {
            if ctx.original_destination == Some(Zone::Security) {
                match redirect {
                    Some(z) => ctx.redirect_to(z),
                    None => ctx.cancel(),
                }
            }
        });
        vec![if self.optional {
            e.optional().build()
        } else {
            e.build()
        }]
    }
}
#[test]
fn canceled_redirected_or_parked_canceled_cost_never_touches_opponent_security() {
    for optional in [false, true] {
        for redirect in [None, Some(Zone::Hand), Some(Zone::Deck)] {
            let mut r = base().start();
            r.register_effect("T", Arc::new(Guard { optional, redirect }));
            r.place_on_field(0, "T", Some(0));
            let q = r.place_on_field(0, ID, Some(0));
            let h = r.place_on_field(0, "D", Some(0));
            fire(&mut r, q, EffectTiming::WhenDigivolving);
            pay(&mut r, h);
            if optional {
                assert_eq!(r.security_count(1), 3);
                r.accept_optional_trigger().unwrap();
            }
            r.auto_resolve().unwrap();
            assert_eq!(r.security_count(1), 3);
            assert_eq!(r.deck_size(1), 20);
        }
    }
}
#[test]
fn parked_declined_replacement_pays_and_self_placement_tail_survives_departure() {
    let mut r = base().start();
    r.register_effect(
        "T",
        Arc::new(Guard {
            optional: true,
            redirect: None,
        }),
    );
    r.place_on_field(0, "T", Some(0));
    let q = r.place_on_field(0, ID, Some(0));
    let top = r.game.players[0].battle_area[q.index as usize]
        .top_card()
        .handle();
    fire(&mut r, q, EffectTiming::WhenDigivolving);
    pay(&mut r, q);
    assert!(r.pending_selection_view().unwrap().is_optional);
    assert_eq!(r.security_count(1), 3);
    r.decline_optional_trigger().unwrap();
    assert_eq!(r.game.players[0].security.last().unwrap().handle(), top);
    finish_security(&mut r, 2);
    assert_eq!(r.security_count(1), 2);
}
#[test]
fn processed_token_or_egg_cost_qualifies_without_security_addition() {
    for id in ["TOKEN", "EGG"] {
        let mut r = base().start();
        let q = r.place_on_field(0, ID, Some(0));
        let h = r.place_on_field(0, id, Some(0));
        fire(&mut r, q, EffectTiming::WhenDigivolving);
        pay(&mut r, h);
        assert_eq!(r.security_count(0), 1);
        finish_security(&mut r, 0);
        assert_eq!(r.security_count(1), 2);
        assert_eq!(r.battle_area_size(0), 1);
    }
}
fn category(r: &mut DebugRunner, q: PermanentHandle, declared: usize, return_pos: usize) {
    fire(r, q, EffectTiming::StartOfOpponentsTurn);
    let v = r.pending_selection_view().unwrap();
    assert_eq!(v.valid_action_ids.len(), 4);
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
    r.execute_branch(declared).unwrap();
    let v = r
        .pending_selection_view()
        .expect("choose return top or bottom");
    assert_eq!(v.valid_action_ids.len(), 2);
    r.execute_branch(return_pos).unwrap();
    assert!(r.pending_selection().is_none());
}
#[test]
fn all_declared_categories_and_top_bottom_returns_match_exact_card() {
    for (declared, id, kind) in [
        (0, "D", EffectSourceKind::Digimon),
        (1, "T", EffectSourceKind::Tamer),
        (2, "O", EffectSourceKind::Option),
    ] {
        for pos in [0, 1] {
            let mut r = base().deck(1, &["Y5", id]).start();
            let q = r.place_on_field(0, ID, Some(0));
            let card = r.game.players[1].deck.last().unwrap().handle();
            category(&mut r, q, declared, pos);
            assert!(r.game.permanent_is_unaffected_by_effect(q, 1, kind));
            assert_eq!(
                if pos == 0 {
                    r.game.players[1].deck.last().unwrap().handle()
                } else {
                    r.game.players[1].deck.first().unwrap().handle()
                },
                card
            );
        }
    }
}
#[test]
fn mismatched_category_does_not_grant_immunity_but_still_returns_reveal() {
    let mut r = base().deck(1, &["D", "T"]).start();
    let q = r.place_on_field(0, ID, Some(0));
    category(&mut r, q, 0, 1);
    for k in [
        EffectSourceKind::Digimon,
        EffectSourceKind::Tamer,
        EffectSourceKind::Option,
    ] {
        assert!(!r.game.permanent_is_unaffected_by_effect(q, 1, k));
    }
}
#[test]
fn digi_egg_declaration_never_becomes_digimon_category_immunity() {
    let mut r = base().deck(1, &["D", "EGG"]).start();
    let q = r.place_on_field(0, ID, Some(0));
    category(&mut r, q, 3, 0);
    assert!(r
        .game
        .permanent_is_unaffected_by_effect(q, 1, EffectSourceKind::DigiEgg));
    assert!(!r
        .game
        .permanent_is_unaffected_by_effect(q, 1, EffectSourceKind::Digimon));
}

struct TargetDelta {
    target: PermanentHandle,
}
impl CardEffect for TargetDelta {
    fn effects(&self, c: digimon_engine::card_source::CardHandle) -> Vec<Effect> {
        let target = self.target;
        vec![Effect::on_play(c)
            .process(move |ctx| {
                ctx.add_dp_modifier(target, -2000, Expiry::EndOfTurn);
                ctx.suspend(target);
            })
            .build()]
    }
}
#[test]
fn current_errata_allows_own_inherited_effect_but_blocks_other_friendly_and_opposing_digimon() {
    let mut r = base().deck(1, &["D"]).start();
    let q = r.place_stack(0, &["INH", ID]);
    category(&mut r, q, 0, 0);
    fire(&mut r, q, EffectTiming::OnAttack);
    assert_eq!(
        r.effective_dp(q),
        Some(14000),
        "this Digimon's inherited effect remains effective"
    );
    r.register_effect("D", Arc::new(TargetDelta { target: q }));
    for owner in [0, 1] {
        let e = r.place_on_field(owner, "D", Some(0));
        fire(&mut r, e, EffectTiming::OnPlay);
        assert_eq!(r.effective_dp(q), Some(14000));
        assert!(!r.game.players[0].battle_area[0].is_suspended);
    }
    r.register_effect("T", Arc::new(TargetDelta { target: q }));
    let e = r.place_on_field(1, "T", Some(0));
    fire(&mut r, e, EffectTiming::OnPlay);
    assert_eq!(r.effective_dp(q), Some(12000));
    assert!(r.game.players[0].battle_area[0].is_suspended);
}
#[test]
fn immunity_expires_at_that_opponent_turn_end_and_empty_deck_returns_no_nonexistent_card() {
    let mut r = base().deck(1, &["D"]).start();
    r.game.turn_player_idx = 1;
    let q = r.place_on_field(0, ID, Some(0));
    category(&mut r, q, 0, 0);
    assert!(r
        .game
        .permanent_is_unaffected_by_effect(q, 1, EffectSourceKind::Digimon));
    r.end_turn();
    assert!(!r
        .game
        .permanent_is_unaffected_by_effect(q, 1, EffectSourceKind::Digimon));
    let mut r = base().deck(1, &[]).start();
    let q = r.place_on_field(0, ID, Some(0));
    fire(&mut r, q, EffectTiming::StartOfOpponentsTurn);
    r.execute_branch(0).unwrap();
    assert!(r.pending_selection().is_none());
    assert!(r.game.revealed_cards.is_empty());
}
#[test]
fn successful_payment_publicly_reveals_all_security_without_flipping_then_forgets_order_on_shuffle()
{
    let mut r = base().start();
    let q = r.place_on_field(0, ID, Some(0));
    let h = r.place_on_field(0, "D", Some(0));
    let cards: Vec<_> = r.game.players[1]
        .security
        .iter()
        .map(|c| c.handle())
        .collect();
    fire(&mut r, q, EffectTiming::WhenDigivolving);
    pay(&mut r, h);
    assert_eq!(r.security_count(1), 3);
    assert!(r.game.players[1].face_up_security.is_empty());
    for h in &cards {
        assert!(r.game.security_card_is_public(1, *h));
    }
    let reveals: Vec<_> = r
        .game
        .events
        .iter()
        .filter_map(|e| {
            if let digimon_engine::events::GameEvent::Reveal {
                source_zone: digimon_engine::events::RevealZone::SecurityStack,
                card_id,
                ..
            } = e
            {
                Some(card_id.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(reveals.len(), 3);
    assert!(reveals.contains(&"D") && reveals.contains(&"T") && reveals.contains(&"O"));
    assert!(
        !r.game
            .events
            .iter()
            .any(|e| matches!(e, digimon_engine::events::GameEvent::SecurityReveal { .. })),
        "effect reveal is not a security check"
    );
    let ui = digimon_engine::serialization::to_ui_json(&r.game);
    assert!(ui["player2"]["securityIds"]
        .as_array()
        .unwrap()
        .iter()
        .all(|x| x.is_string()));
    assert!(ui["player2"]["securityFaceUp"]
        .as_array()
        .unwrap()
        .iter()
        .all(|x| x.as_bool() == Some(false)));
    finish_security(&mut r, 0);
    for c in &r.game.players[1].security {
        assert!(!r.game.security_card_is_public(1, c.handle()));
    }
    assert!(r.game.players[1].face_up_security.is_empty());
}
