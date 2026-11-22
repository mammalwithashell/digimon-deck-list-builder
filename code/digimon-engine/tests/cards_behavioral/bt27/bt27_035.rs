//! BT27-035: EN/JP printed images and JP Q7687 leave SEVEN. EN Q7687 alone
//! says eight; EN list expressly gives Japanese card list priority (2026-11-22).
//! Q7682/83 copied Option Main != Option use; Q7685 player orders both triggers;
//! Q7688 owner may inspect hand cards, activator cannot; Q7689 security stays hidden.
#[path = "../burst_digivolve_common.rs"]
mod burst;
#[path = "../bt26/support.rs"]
mod support;
use digimon_engine::{
    action::{
        mask::build_action_mask,
        space::{encode_digivolve, PASS},
    },
    debug_runner::{DebugRunner, DebugRunnerBuilder},
    enums::{CardColor, EffectSourceKind, EffectTiming},
    permanent::PermanentHandle,
    selection::{SelectionKind, TriggerSource},
};
use support::*;
const ID: &str = "BT27-035";
fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT27-035 YAML required")
        .add_card(filler("F"))
        .add_card(filler("HIDDEN-A"))
        .add_card(filler("HIDDEN-B"))
        .add_card(digimon("L3", "Three", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("L6", "Six", CardColor::Blue, 6, 10, &[]))
        .add_card(digimon("BLACK", "Black", CardColor::Black, 6, 10, &[]))
        .add_card(digimon(
            "SQUAD",
            "Squad",
            CardColor::Red,
            6,
            10,
            &["DATA SQUAD"],
        ))
        .add_card(tamer("T", "Observer", CardColor::Red, &[]))
        .deck(0, &["F"; 20])
        .deck(1, &["F"; 20])
        .security(1, &["HIDDEN-A", "HIDDEN-B"])
        .memory(10)
}
fn setup(n: usize) -> (DebugRunner, PermanentHandle) {
    let mut r = base().hand(1, &vec!["HIDDEN-A"; n]).start();
    r.skip_mulligan();
    r.set_first_player(0);
    let h = r.place_on_field(0, ID, Some(0));
    (r, h)
}
fn pick(r: &mut DebugRunner, a: u16) {
    let p = r.pending_selection_view().unwrap().selecting_player;
    assert_eq!(build_action_mask(&r.game, p)[a as usize], 1.0);
    r.execute_action(p, a).unwrap();
}
fn first(r: &mut DebugRunner) {
    let a = *r
        .pending_selection_view()
        .unwrap()
        .valid_action_ids
        .iter()
        .find(|a| **a != PASS)
        .unwrap();
    pick(r, a);
}
fn fire(r: &mut DebugRunner, h: PermanentHandle, t: EffectTiming) {
    r.game.enqueue_triggered(t, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
fn order(r: &mut DebugRunner, blind_first: bool) {
    let v = r.pending_selection_view().unwrap();
    assert_eq!(v.kind, SelectionKind::TriggerOrder);
    let c = v.effect_choices.unwrap();
    assert_eq!(c.len(), 2);
    pick(r, c[usize::from(blind_first)].action_id);
}
fn until_blind(r: &mut DebugRunner) {
    for _ in 0..4 {
        let v = r.pending_selection_view().expect("blind payment reachable");
        if matches!(v.kind, SelectionKind::CountCappedMultiSelect { .. }) {
            return;
        }
        assert!(v.is_optional, "{:?} {}", v.kind, v.prompt);
        first(r);
    }
    panic!("no blind selection");
}
fn blind(r: &mut DebugRunner, n: usize) -> Vec<digimon_engine::card_source::CardHandle> {
    let mut handles = vec![];
    for i in 0..n {
        let v = r.pending_selection_view().unwrap();
        assert!(matches!(
            v.kind,
            SelectionKind::CountCappedMultiSelect { .. }
        ));
        assert_eq!(v.selecting_player, 0);
        assert_eq!(v.zone_owner, Some(1));
        assert!(!v.valid_action_ids.contains(&PASS));
        let a = *v
            .valid_action_ids
            .iter()
            .filter(|a| **a != PASS)
            .nth(if i == 0 {
                v.valid_action_ids.len() - 1
            } else {
                0
            })
            .unwrap();
        // Legacy hand menu actions address shuffled slots; no card identity is shown.
        let index = (a - digimon_engine::action::space::PLAY_HAND_START) as usize;
        handles.push(r.game.players[1].hand[index].handle());
        pick(r, a);
    }
    if r.pending_selection_view()
        .is_some_and(|v| matches!(v.kind, SelectionKind::CountCappedMultiSelect { .. }))
    {
        pick(r, PASS);
    }
    handles
}
fn decline_rest(r: &mut DebugRunner) {
    for _ in 0..10 {
        let Some(v) = r.pending_selection_view() else {
            return;
        };
        if v.is_optional {
            pick(r, PASS)
        } else {
            first(r)
        }
    }
    panic!("unexpected selection loop");
}
#[test]
fn bt27_035_seven_has_no_blind_payment() {
    let (mut r, h) = setup(7);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    assert!(r.pending_is_optional());
    pick(&mut r, PASS);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.hand_size(1), 7);
    assert_eq!(r.game.players[1].security.len(), 2);
}
#[test]
fn bt27_035_eight_nine_leave_seven_and_security_bottom() {
    for n in [8, 9] {
        let (mut r, h) = setup(n);
        let security = r.game.players[1].security.last().unwrap().handle();
        let d = r.deck_size(1);
        fire(&mut r, h, EffectTiming::WhenDigivolving);
        order(&mut r, true);
        until_blind(&mut r);
        let chosen = blind(&mut r, n - 7);
        assert_eq!(r.hand_size(1), 7);
        assert_eq!(r.deck_size(1), d + n - 7 + 1);
        assert_eq!(r.game.players[1].deck[0].handle(), security);
        for (i, c) in chosen.iter().rev().enumerate() {
            assert_eq!(r.game.players[1].deck[i + 1].handle(), *c);
        }
        assert_eq!(r.game.players[1].security.len(), 1);
        assert!(r.game.players[1].trash.is_empty());
        assert!(r.game.revealed_cards.is_empty());
        decline_rest(&mut r);
    }
}
#[test]
fn bt27_035_declining_hand_payment_preserves_hand_security() {
    let (mut r, h) = setup(9);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    order(&mut r, true);
    assert!(r.pending_is_optional());
    pick(&mut r, PASS);
    decline_rest(&mut r);
    assert_eq!(r.hand_size(1), 9);
    assert_eq!(r.game.players[1].security.len(), 2);
}
#[test]
fn bt27_035_empty_security_still_pays_hand() {
    let (mut r, h) = setup(8);
    r.game.players[1].security.clear();
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    order(&mut r, true);
    until_blind(&mut r);
    blind(&mut r, 1);
    decline_rest(&mut r);
    assert_eq!(r.hand_size(1), 7);
}
#[test]
fn bt27_035_blind_choice_hides_identity_but_owner_observation_retains_it() {
    use digimon_engine::{
        card_registry::CardRegistry,
        observation::{build_observation_tensor, parse_observation_profile},
    };
    let (mut r, h) = setup(9);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    order(&mut r, true);
    until_blind(&mut r);
    let data = r
        .game
        .card_data
        .iter()
        .map(|c| (c.card_id.clone(), c.clone()))
        .collect();
    let reg = CardRegistry::from_cards(&data);
    let profile = parse_observation_profile("standard_lite_v2").unwrap();
    let a = build_observation_tensor(&r.game, 0, &reg, profile);
    let owner = build_observation_tensor(&r.game, 1, &reg, profile);
    let other = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "HIDDEN-B")
        .unwrap();
    r.game.players[1].hand[0].data_index = other;
    assert_eq!(a, build_observation_tensor(&r.game, 0, &reg, profile));
    assert_ne!(owner, build_observation_tensor(&r.game, 1, &reg, profile));
    let v = r.pending_selection_view().unwrap();
    if let Some(c) = v.card_choice {
        assert!(c.rows.iter().all(|row| row.card_id.is_none()));
    }
    for action in v.valid_action_ids {
        let e = digimon_engine::action::explain::explain_action(&r.game, 0, action);
        assert!(e.card_id.is_none(), "blind action exposes identity");
    }
}
#[test]
fn bt27_035_top_security_identity_never_revealed() {
    let (mut r, h) = setup(8);
    let top = r.game.players[1].security.last().unwrap().handle();
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    order(&mut r, true);
    until_blind(&mut r);
    blind(&mut r, 1);
    assert_eq!(r.game.players[1].deck[0].handle(), top);
    assert!(r.game.revealed_cards.is_empty());
    assert!(r.game.players[1].trash.is_empty());
}
#[test]
fn bt27_035_both_trigger_orders_have_distinct_legal_results() {
    for blind_first in [false, true] {
        let (mut r, h) = setup(8);
        let ally = r.place_on_field(0, "L3", Some(0));
        r.game.players[0].battle_area[ally.index as usize].is_suspended = true;
        r.place_on_field(1, "L3", None);
        fire(&mut r, h, EffectTiming::WhenDigivolving);
        order(&mut r, blind_first);
        if blind_first {
            until_blind(&mut r);
            blind(&mut r, 1);
            first(&mut r);
            first(&mut r);
            first(&mut r);
        } else {
            first(&mut r);
            first(&mut r);
            first(&mut r);
            until_blind(&mut r);
            blind(&mut r, 2);
        }
        assert!(!r.game.players[0].battle_area[ally.index as usize].is_suspended);
        assert_eq!(r.hand_size(1), if blind_first { 8 } else { 7 });
        assert_eq!(r.game.players[1].security.len(), 1);
    }
}
#[test]
fn bt27_035_main_bounce_seven_to_eight_then_optional_unsuspend() {
    for accept in [false, true] {
        let mut r = base().hand(0, &[ID]).hand(1, &["F"; 7]).start();
        let ally = r.place_on_field(0, "L6", Some(0));
        r.game.players[0].battle_area[ally.index as usize].is_suspended = true;
        r.place_on_field(1, "L3", None);
        r.game.play_option_from_hand(0, 0);
        assert!(!r.pending_is_optional());
        first(&mut r);
        assert_eq!(r.hand_size(1), 8);
        assert!(r.pending_is_optional());
        if accept {
            first(&mut r)
        } else {
            pick(&mut r, PASS)
        }
        assert_eq!(
            r.game.players[0].battle_area[ally.index as usize].is_suspended,
            !accept
        );
    }
}
#[test]
fn bt27_035_main_bounce_six_to_seven_does_not_unsuspend() {
    let mut r = base().hand(0, &[ID]).hand(1, &["F"; 6]).start();
    let ally = r.place_on_field(0, "L6", Some(0));
    r.game.players[0].battle_area[ally.index as usize].is_suspended = true;
    r.place_on_field(1, "L3", None);
    r.game.play_option_from_hand(0, 0);
    first(&mut r);
    assert_eq!(r.hand_size(1), 7);
    assert!(r.game.players[0].battle_area[ally.index as usize].is_suspended);
}
struct UseWitness(std::sync::Arc<std::sync::Mutex<u32>>);
impl digimon_engine::effect::CardEffect for UseWitness {
    fn effects(
        &self,
        c: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        let n = self.0.clone();
        vec![digimon_engine::effect::Effect::on_play(c)
            .timing(EffectTiming::OnUseOption)
            .process(move |_| {
                *n.lock().unwrap() += 1;
            })
            .build()]
    }
}
#[test]
fn bt27_035_direct_main_is_option_effect_not_use() {
    let (mut r, h) = setup(0);
    let n = std::sync::Arc::new(std::sync::Mutex::new(0));
    r.register_effect("T", std::sync::Arc::new(UseWitness(n.clone())));
    r.place_on_field(0, "T", None);
    r.place_on_field(1, "L3", None);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    first(&mut r);
    assert_eq!(
        r.pending_selection_view().unwrap().source_kind,
        EffectSourceKind::Option
    );
    first(&mut r);
    assert_eq!(*n.lock().unwrap(), 0);
    assert_eq!(r.memory(), 10);
    assert!(r.game.pending_option.is_none());
    assert_eq!(top_id(&r, h), ID);
}
#[test]
fn bt27_035_actual_use_pays_six_emits_use_and_offers_arts() {
    let mut r = base().hand(0, &[ID]).start();
    let n = std::sync::Arc::new(std::sync::Mutex::new(0));
    r.register_effect("T", std::sync::Arc::new(UseWitness(n.clone())));
    r.place_on_field(0, "T", None);
    r.place_on_field(0, "SQUAD", Some(0));
    r.place_on_field(1, "L3", None);
    r.game.play_option_from_hand(0, 0);
    first(&mut r);
    assert_eq!(r.memory(), 4);
    assert!(r.pending_selection().is_some());
    for _ in 0..8 {
        if r.pending_selection().is_none() {
            break;
        }
        first(&mut r);
    }
    assert_eq!(*n.lock().unwrap(), 1);
    assert!(field_ids(&r, 0).contains(&ID.into()));
}
#[test]
fn bt27_035_copied_main_shared_opt_but_reduction_not_opt() {
    let (mut r, h) = setup(8);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    order(&mut r, false);
    first(&mut r);
    until_blind(&mut r);
    blind(&mut r, 1);
    assert!(r.pending_selection().is_none());
    r.add_to_hand(1, "F");
    fire(&mut r, h, EffectTiming::WhenAttacking);
    assert_ne!(
        r.pending_selection_view().unwrap().kind,
        SelectionKind::TriggerOrder
    );
    until_blind(&mut r);
    blind(&mut r, 1);
    assert_eq!(r.hand_size(1), 7);
    assert_eq!(r.game.players[1].security.len(), 0);
}
#[test]
fn bt27_035_normal_and_trait_routes_and_cannot_play() {
    for host in ["L6", "BLACK", "SQUAD"] {
        let mut r = base().hand(0, &[ID]).start();
        assert!(r.play(0, 0).is_none());
        let h = r.place_on_field(0, host, Some(0));
        let a = encode_digivolve(0, h.index as u16);
        assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
        r.game.decode_action(a, 0);
        decline_rest(&mut r);
        assert_eq!(r.memory(), 5);
        assert_eq!(top_id(&r, h), ID);
    }
}
const SPEC: burst::BurstSpec = burst::BurstSpec {
    card: ID,
    host_name: "MirageGaogamon",
    host_color: CardColor::Blue,
    tamer_name: "Thomas H. Norstein",
};
#[test]
fn bt27_035_burst_returns_thomas_cost_zero_draws() {
    burst::burst_and_check(SPEC, burst::Host::BurstOnly);
}
#[test]
fn bt27_035_burst_requires_thomas() {
    burst::assert_not_offered_without_tamer(SPEC);
}
#[test]
fn bt27_035_burst_vs_normal_choice() {
    burst::assert_route_choice(SPEC);
}
#[test]
fn bt27_035_burst_turn_end_cleanup_after_host_shift() {
    burst::assert_turn_end_trash(SPEC, burst::Host::BurstOnly);
}
#[test]
fn bt27_035_q7686_option_tail_continues_after_source_departure() {
    let reactor = r#"
card: REACTOR
name: Reactor
kind: digimon
level: 4
color: [red]
cost: 4
dp: 4000
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    once_per_turn: true
    active_when: { is_source: true }
    process:
      - select_opponent_permanent:
          bind_as: activator
          filter: { name_is: 'MirageGaogamon: Burst Mode' }
          prompt: 'Remove activating Burst Mode'
      - delete_permanent: { target: activator }
"#;
    let mut r = base()
        .from_dsl_yaml(reactor)
        .unwrap()
        .hand(1, &["F"; 7])
        .start();
    let h = r.place_on_field(0, ID, Some(0));
    r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    let ally = r.place_on_field(0, "L3", Some(0));
    r.game.players[0].battle_area[ally.index as usize].is_suspended = true;
    r.place_on_field(1, "REACTOR", Some(0));
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    for _ in 0..10 {
        if r.pending_selection().is_none() {
            break;
        }
        first(&mut r);
    }
    assert!(!field_ids(&r, 0).contains(&ID.into()));
    assert_eq!(r.hand_size(1), 8);
    assert!(
        !r.game.players[0].battle_area[0].is_suspended,
        "After unsuspends another Digimon after activator leaves"
    );
}
#[test]
fn bt27_035_actual_attack_triggers_reduction() {
    let (mut r, h) = setup(8);
    let t = r.place_on_field(1, "L3", None);
    r.game.players[1].battle_area[t.index as usize].is_suspended = true;
    r.attack_digimon(h, t, false);
    order(&mut r, true);
    until_blind(&mut r);
    blind(&mut r, 1);
    decline_rest(&mut r);
    assert_eq!(r.hand_size(1), 7);
    assert_eq!(r.game.players[1].security.len(), 1);
}
#[test]
fn bt27_035_printed_keywords_and_reboot() {
    use digimon_engine::enums::Keyword;
    let (mut r, h) = setup(0);
    for k in [Keyword::Reboot, Keyword::Blocker, Keyword::Evade] {
        assert!(r.game.has_keyword(h, k));
    }
    r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    r.pass_turn();
    assert!(!r.game.players[0].battle_area[h.index as usize].is_suspended);
}

/// Q7681 / CR 8-3-2-1: Burst cleanup is a rule process and also applies to a
/// Digimon that Burst Digivolved in breeding. PR774 reports this substrate gap;
/// this witness must remain failing/blocked until an independent fix is verified.
#[test]
fn bt27_035_burst_in_breeding_trashes_top_at_end_of_turn() {
    use digimon_engine::action::space::BREEDING_TARGET;
    let mut r = base()
        .add_card(digimon(
            "BREED-MIRAGE",
            "MirageGaogamon",
            CardColor::Blue,
            6,
            12,
            &["DATA SQUAD"],
        ))
        .add_card(tamer(
            "THOMAS",
            "Thomas H. Norstein",
            CardColor::Blue,
            &["DATA SQUAD"],
        ))
        .hand(0, &[ID])
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.place_on_field(0, "THOMAS", Some(0));
    r.place_in_breeding(0, "BREED-MIRAGE");
    let action = encode_digivolve(0, BREEDING_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    let deck_before = r.deck_size(0);
    r.game.decode_action(action, 0);
    let view = r
        .pending_selection_view()
        .expect("normal versus Burst route choice");
    let burst = view
        .effect_choices
        .unwrap()
        .into_iter()
        .find(|choice| choice.label.starts_with("Burst Digivolve"))
        .expect("legal Burst route in breeding");
    pick(&mut r, burst.action_id);
    // The selected route includes the sole legal Thomas payment.
    decline_rest(&mut r);
    let breeding = r.game.players[0].breeding_area.as_ref().unwrap();
    assert_eq!(breeding.top_card().card_id(&r.game.card_data), ID);
    assert_eq!(breeding.card_sources.len(), 2);
    assert_eq!(r.memory(), 10, "Burst cost is zero");
    assert_eq!(r.deck_size(0), deck_before - 1, "normal evolution draw");
    assert!(hand_ids(&r, 0).contains(&"THOMAS".into()));
    r.pass_turn();
    decline_rest(&mut r);
    let breeding = r.game.players[0].breeding_area.as_ref().unwrap();
    assert_eq!(
        breeding.top_card().card_id(&r.game.card_data),
        "BREED-MIRAGE",
        "Burst rule cleanup must trash the top card even in breeding"
    );
    assert_eq!(breeding.card_sources.len(), 1);
    assert!(r.game.players[0]
        .trash
        .iter()
        .any(|card| card.card_id(&r.game.card_data) == ID));
}
