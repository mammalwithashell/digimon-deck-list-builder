//! General permanent-to-security payment receipts: physical movement and parked outcomes.
use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardKind, EffectTiming, GamePhase, Zone};
use digimon_engine::selection::TriggerSource;
use std::sync::Arc;
const LAUNCH: &str = r#"
card: LAUNCH
name: Launch
kind: tamer
color: [yellow]
cost: 0
effects:
  - when: on_play
    process:
      - select_any_permanent:
          bind_as: chosen
          filter: {kind: digimon}
          prompt: Choose a Digimon to place in its owner's security
      - place_on_security:
          source: {permanent: chosen}
          to_owner: true
          position: top
          bind_placed_as: paid
      - if:
          condition: {binding_present: paid}
          then:
            - gain_memory: 2
"#;
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
fn setup(owner: u8, kind: CardKind) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    setup_yaml(owner, kind, LAUNCH)
}
fn setup_yaml(
    owner: u8,
    kind: CardKind,
    yaml: &str,
) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut c = make_test_card("PAY", "Pay");
    c.card_kind = kind;
    let mut pad = make_test_card("PAD", "Pad");
    pad.card_kind = CardKind::Tamer;
    pad.dp = None;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(c)
        .add_card(pad.clone())
        .add_card({
            let mut park = pad;
            park.card_id = "PARK".into();
            park
        })
        .memory(5)
        .deck(0, &["PAD"; 4])
        .deck(1, &["PAD"; 4])
        .start();
    r.game.current_phase = GamePhase::Main;
    let pay = r.place_on_field(owner, "PAY", Some(0));
    (r, pay)
}
fn begin(r: &mut DebugRunner) {
    let e = r.place_on_field(0, "LAUNCH", Some(0));
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(e));
    r.game.drain_effect_queue();
    let v = r.pending_selection_view().unwrap();
    assert!(!v.is_optional);
    assert_eq!(v.valid_action_ids.len(), 1);
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
}
#[test]
fn normal_owner_routing_both_sides_pays_once() {
    for owner in [0, 1] {
        let (mut r, h) = setup(owner, CardKind::Digimon);
        let top = r.game.players[owner as usize].battle_area[h.index as usize]
            .top_card()
            .handle();
        begin(&mut r);
        assert!(r.pending_selection().is_none());
        assert_eq!(r.memory(), 7);
        assert_eq!(r.security_count(owner), 1);
        assert_eq!(
            r.game.players[owner as usize]
                .security
                .last()
                .unwrap()
                .handle(),
            top
        );
        assert_eq!(r.security_count(1 - owner), 0);
    }
}
#[test]
fn canceled_and_redirected_costs_do_not_pay() {
    for redirect in [None, Some(Zone::Hand), Some(Zone::Deck)] {
        let (mut r, _) = setup(0, CardKind::Digimon);
        r.register_effect(
            "PAD",
            Arc::new(Guard {
                optional: false,
                redirect,
            }),
        );
        r.place_on_field(0, "PAD", Some(0));
        begin(&mut r);
        r.auto_resolve().unwrap();
        assert_eq!(r.memory(), 5);
        assert_eq!(r.security_count(0), 0);
    }
}
#[test]
fn parked_payment_waits_and_cloned_accept_decline_have_separate_receipts() {
    let (mut r, _) = setup(0, CardKind::Digimon);
    r.register_effect(
        "PAD",
        Arc::new(Guard {
            optional: true,
            redirect: None,
        }),
    );
    r.place_on_field(0, "PAD", Some(0));
    begin(&mut r);
    assert!(r.pending_selection_view().unwrap().is_optional);
    assert_eq!(r.memory(), 5);
    let (mut other, _) = setup(0, CardKind::Digimon);
    other.game = r.game.clone();
    r.accept_optional_trigger().unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 5);
    assert_eq!(r.security_count(0), 0);
    other.decline_optional_trigger().unwrap();
    other.auto_resolve().unwrap();
    assert_eq!(other.memory(), 7);
    assert_eq!(other.security_count(0), 1);
}
#[test]
fn token_and_egg_rule_routing_meets_condition_without_adding_security() {
    for kind in [CardKind::Token, CardKind::DigiEgg] {
        let (mut r, _) = setup(0, kind);
        begin(&mut r);
        assert_eq!(r.security_count(0), 0);
        assert_eq!(
            r.memory(),
            7,
            "{kind:?} processed placement qualifies per Q4008/Q4009"
        );
    }
}

#[test]
fn failed_reuse_clears_old_payment_and_cannot_retarget_a_compacted_slot() {
    let yaml = format!(
        "{}{}",
        LAUNCH,
        r#"
      - place_on_security:
          source: {permanent: chosen}
          to_owner: true
          position: top
          bind_placed_as: paid
      - if:
          condition: {binding_present: paid}
          then:
            - gain_memory: 4
"#
    );
    let (mut r, _) = setup_yaml(0, CardKind::Digimon, &yaml);
    begin(&mut r);
    assert!(r.pending_selection().is_none());
    assert_eq!(
        r.memory(),
        7,
        "old success cannot revive after the selected physical permanent departed"
    );
    assert_eq!(r.security_count(0), 1);
    assert_eq!(
        r.battle_area_size(0),
        1,
        "the newly compacted Launch slot was not the selected payer"
    );
}

/// The replacement places the same physical card itself and cancels the original
/// move. That other effect cannot pay Launch's optional processing condition.
struct IndependentlyPlaceSameCard;
impl CardEffect for IndependentlyPlaceSameCard {
    fn effects(&self, c: digimon_engine::card_source::CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_leave_battle_area(c)
            .replacement_process(|ctx| {
                if ctx.original_destination != Some(Zone::Security) {
                    return;
                }
                if let digimon_engine::replacement::ReplacementSubject::Permanent(h) = ctx.subject {
                    ctx.effect.place_permanent_on_security(
                        h.player,
                        h,
                        digimon_engine::enums::StackPosition::Top,
                        false,
                    );
                    ctx.cancel();
                }
            })
            .once_per_turn()
            .build()]
    }
}
#[test]
fn nested_other_effect_placing_same_physical_card_does_not_pay_canceled_original_move() {
    let (mut r, h) = setup(0, CardKind::Digimon);
    let card = r.game.players[0].battle_area[h.index as usize]
        .top_card()
        .handle();
    r.register_effect("PAD", Arc::new(IndependentlyPlaceSameCard));
    r.place_on_field(0, "PAD", Some(0));
    begin(&mut r);
    r.auto_resolve().unwrap();
    assert!(
        r.game.players[0]
            .security
            .iter()
            .any(|c| c.handle() == card),
        "the independent replacement effect really moved the selected physical card"
    );
    assert_eq!(
        r.memory(),
        5,
        "replacement's separate placement cannot credit the canceled optional payment"
    );
}

/// The dedicated helper deliberately performs the original requested movement.
struct HandleOriginalPlacement;
impl CardEffect for HandleOriginalPlacement {
    fn effects(&self, c: digimon_engine::card_source::CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_leave_battle_area(c)
            .replacement_process(|ctx| {
                if ctx.original_destination != Some(Zone::Security) {
                    return;
                }
                if let digimon_engine::replacement::ReplacementSubject::Permanent(h) = ctx.subject {
                    ctx.effect
                        .place_permanent_on_security_and_handle_current_replacement(
                            h.player,
                            h,
                            digimon_engine::enums::StackPosition::Top,
                            false,
                        );
                    ctx.handled();
                }
            })
            .build()]
    }
}
#[test]
fn dedicated_handle_current_replacement_processing_pays_the_original_operation() {
    let (mut r, h) = setup(0, CardKind::Digimon);
    let card = r.game.players[0].battle_area[h.index as usize]
        .top_card()
        .handle();
    r.register_effect("PAD", Arc::new(HandleOriginalPlacement));
    r.place_on_field(0, "PAD", Some(0));
    begin(&mut r);
    r.auto_resolve().unwrap();
    assert!(r.game.players[0]
        .security
        .iter()
        .any(|c| c.handle() == card));
    assert_eq!(
        r.memory(),
        7,
        "explicit original-operation handling remains a paid processing condition"
    );
}

struct ParkIndependentPlacement;
impl CardEffect for ParkIndependentPlacement {
    fn effects(&self, c: digimon_engine::card_source::CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_place_in_security(c)
            .optional()
            .replacement_process(|ctx| ctx.cancel())
            .build()]
    }
}
#[test]
fn nested_independent_parked_placement_in_clones_never_credits_canceled_parent() {
    let (mut r, h) = setup(0, CardKind::Digimon);
    let card = r.game.players[0].battle_area[h.index as usize]
        .top_card()
        .handle();
    r.register_effect("PAD", Arc::new(IndependentlyPlaceSameCard));
    r.place_on_field(0, "PAD", Some(0));
    r.register_effect("PARK", Arc::new(ParkIndependentPlacement));
    r.place_on_field(0, "PARK", Some(0));
    begin(&mut r);
    assert!(r.pending_selection_view().unwrap().is_optional);
    assert_eq!(r.memory(), 5);
    let (mut clone, _) = setup(0, CardKind::Digimon);
    clone.game = r.game.clone();
    r.accept_optional_trigger().unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 5);
    assert_eq!(r.security_count(0), 0);
    clone.decline_optional_trigger().unwrap();
    clone.auto_resolve().unwrap();
    assert!(clone.game.players[0]
        .security
        .iter()
        .any(|c| c.handle() == card));
    assert_eq!(
        clone.memory(),
        5,
        "separate child operation cannot credit canceled parent even after parked clone resume"
    );
}

struct InlineCompactPayer {
    at_placement: bool,
    payer: digimon_engine::card_source::CardHandle,
}
impl CardEffect for InlineCompactPayer {
    fn effects(&self, c: digimon_engine::card_source::CardHandle) -> Vec<Effect> {
        let payer = self.payer;
        let e = if self.at_placement {
            Effect::when_would_place_in_security(c)
        } else {
            Effect::when_would_leave_battle_area(c)
        };
        vec![e
            .replacement_condition(move |ctx, subject| match subject {
                digimon_engine::replacement::ReplacementSubject::Permanent(h) => ctx.game.players
                    [h.player as usize]
                    .battle_area
                    .get(h.index as usize)
                    .is_some_and(|p| p.top_card().handle() == payer),
                digimon_engine::replacement::ReplacementSubject::Card(card, _) => *card == payer,
                _ => false,
            })
            .replacement_process(|ctx| {
                ctx.effect
                    .delete_permanent(digimon_engine::permanent::PermanentHandle {
                        player: 0,
                        index: 0,
                    })
            })
            .build()]
    }
}
fn inline_compaction(at_placement: bool) {
    let (mut r, _) = setup(0, CardKind::Digimon);
    r.place_on_field(0, "PAD", Some(0));
    r.game.players[0].battle_area.swap(0, 1);
    let payer = r.game.players[0].battle_area[1].top_card().handle();
    let sentinel = r.place_on_field(0, "PAD", Some(0));
    let sentinel_card = r.game.players[0].battle_area[sentinel.index as usize]
        .top_card()
        .handle();
    r.register_effect(
        "PARK",
        Arc::new(InlineCompactPayer {
            at_placement,
            payer,
        }),
    );
    r.place_on_field(0, "PARK", Some(0));
    begin(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(
        r.game.players[0].security.last().unwrap().handle(),
        payer,
        "the same selected physical payer must move after lower-slot removal"
    );
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().handle() == sentinel_card),
        "adjacent permanent must remain"
    );
    assert_eq!(r.memory(), 7);
}
#[test]
fn inline_would_leave_compaction_keeps_exact_selected_payer() {
    inline_compaction(false);
}
#[test]
fn inline_would_place_compaction_keeps_exact_selected_payer() {
    inline_compaction(true);
}
