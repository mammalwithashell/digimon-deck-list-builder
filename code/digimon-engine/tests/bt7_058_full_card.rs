//! Full printed BT7-058 behavior. SKULL_TEST_YAML / AXE_TEST_YAML select
//! root-controlled declaration baselines before production implementation.
use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_attack, PASS, PLAY_HAND_START, SECURITY_TARGET},
    },
    debug_runner::{make_test_card, DebugRunner},
    permanent::PermanentHandle,
};
use std::sync::Arc;
fn yaml(var: &str, id: &str) -> String {
    std::fs::read_to_string(
        std::env::var(var)
            .unwrap_or_else(|_| format!("{}/cards/bt7/{id}.yaml", env!("CARGO_MANIFEST_DIR"))),
    )
    .unwrap()
}
fn setup(donor_sources: bool, with_target: bool) -> DebugRunner {
    setup_with_protection(donor_sources, with_target, false)
}
fn setup_with_protection(donor_sources: bool, with_target: bool, protect: bool) -> DebugRunner {
    let mut alias = make_test_card("HOST-ALIAS", "Other");
    alias.also_treated_as.push("Knightmon".into());
    alias.also_treated_as.push("DeadlyAxemon".into());
    let mut r = DebugRunner::builder()
        .add_card(alias)
        .add_card(make_test_card("HOST-K", "DarkKnightmon"))
        .add_card(make_test_card("HOST-B", "Bagramon"))
        .add_card(make_test_card("HOST-N", "Other"))
        .from_dsl_yaml(&yaml("SKULL_TEST_YAML", "BT7-058"))
        .unwrap()
        .from_dsl_yaml(&yaml("AXE_TEST_YAML", "BT7-059"))
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/bt7/BT7-063.yaml"))
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/bt9/BT9-109.yaml"))
        .unwrap()
        .add_card(make_test_card("FILL", "Ordinary filler"))
        .hand(
            0,
            if with_target {
                &["BT7-063", "FILL"][..]
            } else {
                &["FILL"][..]
            },
        )
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .security(1, &["FILL"; 5])
        .memory(10)
        .start();
    if protect {
        let original = r.game.effect_registry.get("BT7-059").unwrap();
        r.register_effect("BT7-059", Arc::new(Protect(original)));
    }
    r.skip_mulligan();
    r.place_on_field(0, "BT7-058", Some(0));
    if donor_sources {
        r.place_stack(0, &["BT9-109", "FILL", "BT7-059"]);
    } else {
        r.place_on_field(0, "BT7-059", Some(0));
    }
    r.game.enter_main_phase();
    r
}
fn act(r: &mut DebugRunner, a: u16) {
    assert_eq!(
        build_action_mask(&r.game, 0)[a as usize],
        1.0,
        "actual legal action {a}"
    );
    r.execute_action(0, a).unwrap();
}
fn attack(r: &mut DebugRunner) {
    let a = encode_attack(0, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    r.game.decode_action(a, 0);
}
fn donor(r: &mut DebugRunner, index: usize) {
    act(r, encode_attack(0, index as u16));
}
fn top(r: &DebugRunner) -> String {
    r.game.players[0].battle_area[0]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

#[test]
fn skull_donor_name_only_in_lower_source_does_not_qualify() {
    let mut r = setup(false, true);
    r.game.players[0].battle_area.remove(1);
    r.place_stack(0, &["BT7-059", "FILL"]);
    let before_hand = r.game.players[0].hand.len();
    attack(&mut r);
    assert!(r.game.pending_selection.is_none(), "a lower-source Axemon is not the donor's name");
    assert_eq!(r.game.players[0].hand.len(), before_hand);
    assert_eq!(r.game.players[0].battle_area[1].card_sources.len(), 2);
    assert_eq!(top(&r), "BT7-058");
}

#[test]
fn skull_as_lower_source_does_not_gain_its_face_attack_clause() {
    let mut r = setup(false, true);
    r.force_base_dp("HOST-K", 12000);
    r.game.players[0].battle_area.clear();
    r.place_stack(0, &["BT7-058", "HOST-K"]);
    r.place_on_field(0, "BT7-059", Some(0));
    r.game.players[0].battle_area[0].turn_played = 0;
    let before_hand = r.game.players[0].hand.len();
    attack(&mut r);
    assert!(r.game.pending_selection.is_none(), "inherited Skull must not offer its face placement effect");
    assert_eq!(r.game.players[0].hand.len(), before_hand);
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert_eq!(top(&r), "HOST-K");
}
#[test]
fn skull_places_only_donor_top_trashes_even_protected_sources_then_free_evolves() {
    let mut r = setup(true, true);
    let donor_host = PermanentHandle { player: 0, index: 1 };
    let protected = r.game.players[0].battle_area[1].card_sources[0].handle();
    assert!(r.game.source_card_trash_blocked(donor_host, protected, 0),
        "X Antibody supplies actual inherited source-trash protection");
    let h = r.game.players[0].battle_area[1].top_card().handle();
    attack(&mut r);
    assert!(r.game.pending_selection.is_some());
    donor(&mut r, 1);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.game.players[0].battle_area[0].card_sources[0].handle(), h);
    let mut trash: Vec<_> = r.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data))
        .collect();
    trash.sort();
    assert_eq!(trash, vec!["BT9-109", "FILL"]);
    assert_eq!(
        build_action_mask(&r.game, 0)[PLAY_HAND_START as usize + 1],
        0.0,
        "unrelated hand card excluded"
    );
    act(&mut r, PLAY_HAND_START);
    assert_eq!(top(&r), "BT7-063");
    assert_eq!(r.memory(), 10);
    assert_eq!(
        r.game.players[0].hand.len(),
        2,
        "free evolution still draws one"
    );
    assert_eq!(
        r.game.players[1].security.len(),
        3,
        "inherited Security A +1 applies during ongoing attack"
    );
}

#[test]
fn skull_deep_donor_stack_still_exposes_the_permanent_choice() {
    let mut r = setup(false, true);
    r.game.players[0].battle_area.pop();
    let mut stack = vec!["FILL"; 12];
    stack.push("BT7-059");
    r.place_stack(0, &stack);
    let physical = r.game.players[0].battle_area[1].top_card().handle();
    attack(&mut r);
    let pending = r.game.pending_selection.as_ref().expect("deep donor is a legal permanent choice");
    let choices: Vec<_> = pending.valid_action_ids.iter().copied().filter(|a| *a != PASS).collect();
    assert_eq!(choices.len(), 1);
    act(&mut r, choices[0]);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.game.players[0].battle_area[0].card_sources[0].handle(), physical);
    assert_eq!(r.game.players[0].trash.len(), 12);
    act(&mut r, PLAY_HAND_START);
    assert_eq!(top(&r), "BT7-063");
}

#[test]
fn skull_all_legal_darkknightmon_copies_are_offered_but_invalid_route_is_not() {
    let mut r = setup(false, true);
    r.add_to_hand(0, "BT7-063");
    r.add_to_hand(0, "BT7-063");
    r.add_to_hand(0, "HOST-K"); // Correct name, no legal evolution route.
    attack(&mut r);
    donor(&mut r, 1);
    let mask = build_action_mask(&r.game, 0);
    for (index, expected) in [(0, 1.0), (1, 0.0), (2, 1.0), (3, 1.0), (4, 0.0)] {
        assert_eq!(mask[PLAY_HAND_START as usize + index], expected);
    }
    let saved = r.game.clone();
    for index in [0, 2, 3] {
        r.game = saved.clone();
        let card = r.game.players[0].hand[index].handle();
        act(&mut r, PLAY_HAND_START + index as u16);
        assert_eq!(r.game.players[0].battle_area[0].top_card().handle(), card);
        assert_eq!(r.memory(), 10);
    }
}

#[test]
fn skull_donor_sources_go_to_their_physical_owners_trash() {
    let mut r = setup(true, false);
    let foreign = r.game.players[0].battle_area[1].card_sources[1].handle();
    r.game.players[0].battle_area[1].card_sources[1].owner = 1;
    attack(&mut r);
    donor(&mut r, 1);
    assert!(r.game.players[1].trash.iter().any(|c| c.handle() == foreign));
    assert!(!r.game.players[0].trash.iter().any(|c| c.handle() == foreign));
    assert_eq!(r.game.players[0].battle_area.len(), 1);
}

#[test]
fn skull_donor_uses_rule_names_and_effective_permanent_name() {
    use digimon_engine::{enums::{Expiry, ModifierType}, modifiers::{ModifierEntry, ModifierPayload}};
    for printed_alias in [true, false] {
        let mut r = setup(false, true);
        r.game.players[0].battle_area.pop();
        let h = r.place_on_field(0, if printed_alias { "HOST-ALIAS" } else { "FILL" }, Some(0));
        if !printed_alias {
            r.game.modifiers.add(h, ModifierEntry::simple(ModifierType::ChangeBaseCardName, 0, Expiry::Permanent, 0)
                .with_payload(ModifierPayload::Name { value: "DeadlyAxemon".into(), base: true }));
        }
        let physical = r.game.players[0].battle_area[1].top_card().handle();
        attack(&mut r);
        donor(&mut r, 1);
        assert_eq!(r.game.players[0].battle_area[0].card_sources[0].handle(), physical);
        act(&mut r, PLAY_HAND_START);
        assert_eq!(top(&r), "BT7-063");
    }
}
#[test]
fn skull_payment_is_optional_and_clone_independent() {
    let mut r = setup(false, true);
    attack(&mut r);
    let saved = r.game.clone();
    act(&mut r, PASS);
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert_eq!(top(&r), "BT7-058");
    r.game = saved;
    donor(&mut r, 1);
    act(&mut r, PLAY_HAND_START);
    assert_eq!(top(&r), "BT7-063");
    assert_eq!(r.game.players[0].battle_area.len(), 1);
}
#[test]
fn skull_can_pay_without_darkknightmon_and_can_decline_evolution_after_payment() {
    for target in [false, true] {
        let mut r = setup(false, target);
        attack(&mut r);
        donor(&mut r, 1);
        if target {
            act(&mut r, PASS);
        }
        assert_eq!(top(&r), "BT7-058");
        assert_eq!(r.game.players[0].battle_area.len(), 1);
        assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 2);
        assert_eq!(r.memory(), 10);
    }
}
#[test]
fn skull_exposes_both_own_donors_but_not_opponent_or_wrong_name() {
    let mut r = setup(false, true);
    r.place_on_field(0, "BT7-059", Some(0));
    r.place_on_field(0, "FILL", Some(0));
    r.place_on_field(1, "BT7-059", Some(0));
    attack(&mut r);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[encode_attack(0, 1) as usize], 1.0);
    assert_eq!(mask[encode_attack(0, 2) as usize], 1.0);
    assert_eq!(mask[encode_attack(0, 0) as usize], 0.0);
    assert_eq!(mask[encode_attack(0, 3) as usize], 0.0);
    let saved = r.game.clone();
    for pick in [1, 2] {
        r.game = saved.clone();
        let selected = r.game.players[0].battle_area[pick].top_card().handle();
        donor(&mut r, pick);
        assert_eq!(
            r.game.players[0].battle_area[0].card_sources[0].handle(),
            selected
        );
        act(&mut r, PLAY_HAND_START);
        assert_eq!(r.game.players[1].battle_area.len(), 1);
    }
}
struct Protect(Arc<dyn digimon_engine::effect::CardEffect>);
impl digimon_engine::effect::CardEffect for Protect {
    fn effects(
        &self,
        c: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        let mut effects = self.0.effects(c);
        effects.push(
            digimon_engine::effect::Effect::when_would_leave_battle_area(c)
                .name("Protect donor")
                .optional()
                .replacement_process(|ctx| ctx.cancel())
                .build(),
        );
        effects
    }
}
#[test]
fn skull_parked_departure_accept_cannot_grant_evolution_decline_can_in_clone() {
    let mut r = setup_with_protection(true, true, true);
    attack(&mut r);
    donor(&mut r, 1);
    assert!(r.game.pending_selection.is_some());
    let accept = *r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids
        .iter()
        .find(|a| **a != PASS)
        .unwrap();
    let saved = r.game.clone();
    act(&mut r, accept);
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert_eq!(top(&r), "BT7-058");
    assert!(r.game.pending_selection.is_none());
    r.game = saved;
    act(&mut r, PASS);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert!(
        r.game.pending_selection.is_some(),
        "successful parked placement resumes actual free evolution choice"
    );
    act(&mut r, PLAY_HAND_START);
    assert_eq!(top(&r), "BT7-063");
}
#[test]
fn skull_inherited_bonus_tracks_turn_name_and_alias() {
    for (id, expected) in [
        ("HOST-K", 1),
        ("HOST-B", 1),
        ("HOST-ALIAS", 1),
        ("HOST-N", 0),
    ] {
        let mut r = setup(false, false);
        r.game.players[0].battle_area.clear();
        r.place_stack(0, &["BT7-058", id]);
        let h = PermanentHandle {
            player: 0,
            index: 0,
        };
        r.game.tick_declarative_effects();
        assert_eq!(
            r.game.security_attack_keyword_bonus(h),
            expected,
            "host {id}"
        );
        r.game.turn_player_idx = 1;
        r.game.tick_declarative_effects();
        assert_eq!(r.game.security_attack_keyword_bonus(h), 0);
    }
}

#[test]
fn skull_parked_payment_rebinds_surviving_attacker_after_earlier_donor_slot_disappears() {
    let mut r = setup_with_protection(true, true, true);
    r.game.players[0].battle_area.swap(0, 1);
    let attacker = r.game.players[0].battle_area[1].top_card().handle();
    let paid = r.game.players[0].battle_area[0].top_card().handle();
    let attack = encode_attack(1, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[attack as usize], 1.0);
    r.game.decode_action(attack, 0);
    donor(&mut r, 0);
    let saved = r.game.clone();
    for _ in 0..2 {
        r.game = saved.clone();
        act(&mut r, PASS);
        assert_eq!(r.game.players[0].battle_area.len(), 1);
        assert_eq!(
            r.game.players[0].battle_area[0].top_card().handle(),
            attacker
        );
        assert_eq!(
            r.game.players[0].battle_area[0].card_sources[0].handle(),
            paid
        );
        assert!(
            r.game.pending_selection.is_some(),
            "successful physical move must resume effect on compacted original attacker"
        );
        act(&mut r, PLAY_HAND_START);
        assert_eq!(top(&r), "BT7-063");
    }
}

#[test]
fn skull_synchronous_compaction_keeps_original_attacker_and_attack() {
    let mut r = setup(false, true);
    r.game.players[0].battle_area.swap(0, 1);
    let attacker = r.game.players[0].battle_area[1].top_card().handle();
    let paid = r.game.players[0].battle_area[0].top_card().handle();
    let a = encode_attack(1, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    r.game.decode_action(a, 0);
    donor(&mut r, 0);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(
        r.game.players[0].battle_area[0].top_card().handle(),
        attacker
    );
    assert_eq!(
        r.game.players[0].battle_area[0].card_sources[0].handle(),
        paid
    );
    act(&mut r, PLAY_HAND_START);
    assert_eq!(top(&r), "BT7-063");
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.players[1].security.len(), 3);
}
#[test]
fn skull_declining_donor_or_paying_without_hand_target_leaves_no_hidden_prompt() {
    let mut decline = setup(false, true);
    attack(&mut decline);
    act(&mut decline, PASS);
    assert!(decline.game.pending_selection.is_none());
    assert_eq!(decline.game.players[0].battle_area.len(), 2);
    assert_eq!(decline.game.players[0].hand.len(), 2);
    assert_eq!(decline.game.players[1].security.len(), 4);
    let mut absent = setup(false, false);
    attack(&mut absent);
    donor(&mut absent, 1);
    assert!(absent.game.pending_selection.is_none());
    assert_eq!(absent.game.players[0].battle_area.len(), 1);
    assert_eq!(top(&absent), "BT7-058");
    assert_eq!(absent.game.players[0].hand.len(), 1);
    assert_eq!(absent.game.players[1].security.len(), 4);
}
#[test]
fn skull_unaffected_donor_attempt_cannot_pay_or_grant_evolution() {
    use digimon_engine::{
        enums::{Expiry, ModifierType},
        modifiers::ModifierEntry,
    };
    let mut r = setup(true, true);
    let h = PermanentHandle {
        player: 0,
        index: 1,
    };
    let cards: Vec<_> = r.game.players[0].battle_area[1]
        .card_sources
        .iter()
        .map(|c| c.handle())
        .collect();
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotBeAffected, 0, Expiry::Permanent, 0),
    );
    attack(&mut r);
    donor(&mut r, 1);
    assert!(
        r.game.pending_selection.is_none(),
        "failed By placement must not open evolution"
    );
    assert_eq!(top(&r), "BT7-058");
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert_eq!(
        r.game.players[0].battle_area[1]
            .card_sources
            .iter()
            .map(|c| c.handle())
            .collect::<Vec<_>>(),
        cards
    );
    assert!(r.game.players[0].trash.is_empty());
    assert_eq!(r.game.players[0].hand.len(), 2);
    assert_eq!(r.memory(), 10);
}
#[test]
fn skull_pays_one_physical_donor_and_leaves_the_other_unchanged() {
    let mut r = setup(false, true);
    r.place_on_field(0, "BT7-059", Some(0));
    let untouched = r.game.players[0].battle_area[2].top_card().handle();
    let saved = r.game.players[0].battle_area[2].clone();
    attack(&mut r);
    donor(&mut r, 1);
    act(&mut r, PLAY_HAND_START);
    let p = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().handle() == untouched)
        .unwrap();
    assert_eq!(p.card_sources.len(), saved.card_sources.len());
    assert_eq!(p.is_suspended, saved.is_suspended);
    assert_eq!(p.linked_cards.len(), saved.linked_cards.len());
    assert_eq!(r.game.players[0].battle_area.len(), 2);
}
#[test]
fn skull_attack_on_digimon_continues_with_evolved_attacker() {
    let mut r = setup(false, true);
    r.place_on_field(1, "FILL", Some(0));
    r.game.players[1].battle_area[0].is_suspended = true;
    let victim = r.game.players[1].battle_area[0].top_card().handle();
    let a = encode_attack(0, 0);
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    r.game.decode_action(a, 0);
    donor(&mut r, 1);
    act(&mut r, PLAY_HAND_START);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(top(&r), "BT7-063");
    assert!(r.game.players[1].battle_area.is_empty());
    assert!(r.game.players[1].trash.iter().any(|c| c.handle() == victim));
    assert_eq!(r.game.players[1].security.len(), 5);
}
#[test]
fn skull_places_donor_top_and_rule_trashes_its_link_cards() {
    let mut r = setup(false, true);
    let linked = r.game.players[0].hand.pop().unwrap();
    let link = linked.handle();
    let paid = r.game.players[0].battle_area[1].top_card().handle();
    r.game.players[0].battle_area[1].linked_cards.push(linked);
    attack(&mut r);
    donor(&mut r, 1);
    assert!(r.game.players[0].trash.iter().any(|c| c.handle() == link));
    assert_eq!(
        r.game.players[0].battle_area[0].card_sources[0].handle(),
        paid
    );
    assert!(r.game.players[0].battle_area[0].linked_cards.is_empty());
    act(&mut r, PLAY_HAND_START);
    assert_eq!(top(&r), "BT7-063");
}
