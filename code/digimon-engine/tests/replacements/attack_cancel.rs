use std::sync::{Arc, Mutex};

use digimon_engine::action::space::encode_source_select;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::CardData;

struct TumblemonCancelAttack(Arc<Mutex<u32>>);

impl CardEffect for TumblemonCancelAttack {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let fired = self.0.clone();
        vec![Effect::on_opponent_attack(card)
            .name("EX10-003 Tumblemon attack cancel")
            .pay_cost(move |ctx| {
                let fired = fired.clone();
                ctx.select_own_sources(
                    "Trash 3 Mineral/Rock sources to end the attack",
                    3,
                    3,
                    |game, source_ref| {
                        game.card(source_ref.card)
                            .traits
                            .iter()
                            .any(|trait_name| trait_name == "Mineral" || trait_name == "Rock")
                    },
                    move |ctx, refs| {
                        for source_ref in refs {
                            assert!(ctx.game.trash_source_ref(source_ref));
                        }
                        *fired.lock().unwrap() += 1;
                    },
                );
                true
            })
            .process(|ctx| {
                ctx.cancel_pending_attack();
            })
            .build()]
    }
}

struct CountEndOfAttack(Arc<Mutex<u32>>);

impl CardEffect for CountEndOfAttack {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let fired = self.0.clone();
        vec![Effect::end_of_attack(card)
            .name("Count EndOfAttack")
            .process(move |_ctx| {
                *fired.lock().unwrap() += 1;
            })
            .build()]
    }
}

fn card_with_traits(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, name);
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

#[test]
fn cancel_pending_attack_outside_combat_is_noop() {
    let end_of_attack = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("WITNESS", "Witness"))
        .memory(2)
        .start();
    r.register_effect("WITNESS", Arc::new(CountEndOfAttack(end_of_attack.clone())));
    r.place_on_field(0, "WITNESS", Some(0));

    r.game.cancel_pending_attack_from_effect();

    assert!(r.game.pending_attack.is_none());
    assert_eq!(*end_of_attack.lock().unwrap(), 0);
    assert_eq!(r.memory(), 2, "unrelated state was not mutated");
}

#[test]
fn ex10_003_pay_cost_can_end_pending_attack() {
    let fired = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("EX10-003", "Tumblemon"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(card_with_traits("SRC1", "Mineral Source 1", &["Mineral"]))
        .add_card(card_with_traits("SRC2", "Mineral Source 2", &["Mineral"]))
        .add_card(card_with_traits("SRC3", "Rock Source 3", &["Rock"]))
        .add_card(make_test_card("SECURITY", "Security"))
        .security(0, &["SECURITY"])
        .start();
    r.register_effect("EX10-003", Arc::new(TumblemonCancelAttack(fired.clone())));

    let blocker = r.place_stack(0, &["SRC1", "SRC2", "SRC3", "EX10-003"]);
    let attacker = r.place_on_field(1, "ATTACKER", Some(0));

    r.attack_player(attacker, 0, false);

    assert!(
        r.game.pending_selection.is_some(),
        "Tumblemon pay-cost prompt is exposed"
    );
    r.game
        .resolve_selection(0, encode_source_select(blocker.index as u16, 0).unwrap())
        .unwrap();
    r.game
        .resolve_selection(0, encode_source_select(blocker.index as u16, 1).unwrap())
        .unwrap();
    r.game
        .resolve_selection(0, encode_source_select(blocker.index as u16, 2).unwrap())
        .unwrap();

    assert_eq!(*fired.lock().unwrap(), 1);
    assert!(
        r.game.pending_attack.is_none(),
        "attack state is fully cleared"
    );
    assert_eq!(r.security_count(0), 1, "security was not checked");
    assert_eq!(r.trash_size(0), 3, "three sources were paid");
}

#[test]
fn dsl_end_attack_step_cancels_pending_attack() {
    let yaml = r#"
card: DSL-END-ATTACK
name: End Attack
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_attack
    process:
      - end_attack: true
  - when: end_of_attack
    process:
      - gain_memory: 1
"#;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(make_test_card("SECURITY", "Security"))
        .security(1, &["SECURITY"])
        .start();

    let attacker = r.place_on_field(0, "DSL-END-ATTACK", Some(0));

    r.attack_player(attacker, 1, false);

    assert!(
        r.game.pending_attack.is_none(),
        "attack state is fully cleared"
    );
    assert_eq!(r.security_count(1), 1, "security was not checked");
    assert_eq!(r.memory(), 1, "EndOfAttack fired exactly once");
}
