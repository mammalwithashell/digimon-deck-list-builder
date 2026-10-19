//! EX8-002 Bukamon: inherited [When Attacking] [Once Per Turn]
//! If you have 0 memory, gain 1 memory.
//! Official observation b771257ceef1736e7b8143729b10e3a5d7fac07ecda08c36420edcc8eaba2486
//! and actual card image verified; DCGO EX8/Blue/EX8_002.cs.
//! Patterns: inherited carrier-local trigger, exact memory condition, mandatory gain, OPT.
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;
const ID: &str = "EX8-002";
fn base() -> DebugRunner {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/ex8/EX8-002.yaml");
    let yaml = std::fs::read_to_string(path).expect("production EX8-002 must exist");
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .unwrap()
        .from_dsl_yaml(include_str!("../../../cards/st2/ST2-02.yaml"))
        .unwrap()
        .from_dsl_yaml(include_str!("../../../cards/st2/ST2-05.yaml"))
        .unwrap()
        .add_card(make_test_card("HOST", "Carrier"))
        .deck(0, &["HOST"; 20])
        .deck(1, &["HOST"; 20])
        .memory(0)
        .start();
    r.skip_mulligan();
    r
}
fn fire(r: &mut DebugRunner, h: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    assert!(
        r.pending_selection().is_none(),
        "gain is mandatory with no choice"
    );
}
#[test]
fn ex8_002_printed_metadata() {
    let r = base();
    let d = r.game.card_data.iter().find(|d| d.card_id == ID).unwrap();
    assert_eq!(d.card_name, "Bukamon");
    assert_eq!(d.card_kind, CardKind::DigiEgg);
    assert_eq!(d.level, Some(2));
    assert_eq!(d.colors, vec![CardColor::Blue]);
    for t in ["Lesser", "DS"] {
        assert!(d.traits.contains(&t.into()));
    }
    assert_eq!(r.compiled_card(ID).unwrap().effects.len(), 1);
}
#[test]
fn ex8_002_exact_zero_memory_both_controllers() {
    for player in [0, 1] {
        for memory in [-1, 0, 1] {
            let mut r = base();
            r.game.turn_player_idx = player;
            r.game.set_memory(memory);
            let h = r.place_stack(player as u8, &[ID, "HOST"]);
            fire(&mut r, h);
            assert_eq!(r.memory(), if memory == 0 { 1 } else { memory });
        }
    }
}
#[test]
fn ex8_002_inherited_only_and_attacking_carrier_only() {
    let mut r = base();
    let host = r.place_stack(0, &[ID, "HOST"]);
    let ally = r.place_on_field(0, "HOST", Some(0));
    let opponent = r.place_on_field(1, "HOST", Some(0));
    let face = r.place_on_field(0, ID, Some(0));
    fire(&mut r, ally);
    fire(&mut r, opponent);
    fire(&mut r, face);
    assert_eq!(r.memory(), 0);
    fire(&mut r, host);
    assert_eq!(r.memory(), 1);
}
#[test]
fn ex8_002_failed_condition_does_not_spend_opt() {
    let mut r = base();
    let h = r.place_stack(0, &[ID, "HOST"]);
    r.game.set_memory(1);
    fire(&mut r, h);
    r.game.set_memory(0);
    fire(&mut r, h);
    assert_eq!(r.memory(), 1);
}
#[test]
fn ex8_002_opt_per_copy_clone_and_turn_reset() {
    let mut r = base();
    let a = r.place_stack(0, &[ID, "HOST"]);
    let b = r.place_stack(0, &[ID, "HOST"]);
    fire(&mut r, a);
    assert_eq!(r.memory(), 1);
    r.game = r.game.clone();
    r.game.set_memory(0);
    fire(&mut r, a);
    assert_eq!(r.memory(), 0);
    fire(&mut r, b);
    assert_eq!(r.memory(), 1);
    r.end_turn();
    r.auto_resolve().unwrap();
    r.end_turn();
    r.auto_resolve().unwrap();
    r.game.set_memory(0);
    fire(&mut r, a);
    assert_eq!(r.memory(), 1);
}
#[test]
fn ex8_002_real_attack_mask_reaches_memory_gain() {
    use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
    let mut r = base();
    let h = r.place_stack(0, &[ID, "HOST"]);
    let card = r.game.players[1].deck.pop().unwrap();
    r.game.players[1].security.push(card);
    let action = encode_attack(h.index as u16, SECURITY_TARGET);
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[action as usize],
        1.0
    );
    r.game.decode_action(action, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 1);
    assert!(r.pending_selection().is_none());
}

#[test]
fn ex8_002_opt_stays_spent_after_real_carrier_evolution_and_clone() {
    use digimon_engine::action::{build_action_mask, space::{encode_attack, encode_digivolve, SECURITY_TARGET}};
    let mut r = base();
    let h = r.place_stack(0, &[ID, "ST2-02"]);
    for _ in 0..2 {
        let card = r.game.players[1].deck.pop().unwrap();
        r.game.players[1].security.push(card);
    }
    fire(&mut r, h);
    assert_eq!(r.memory(), 1);
    // Evolving the carrier must not create a new use for the same source card.
    r.game.set_memory(2);
    let hand = r.add_to_hand(0, "ST2-05");
    let evolve = encode_digivolve(hand as u16, h.index as u16);
    assert_eq!(build_action_mask(&r.game, 0)[evolve as usize], 1.0);
    r.game.decode_action(evolve, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 0);
    assert_eq!(r.game.players[0].battle_area[h.index as usize].top_card().card_id(&r.game.card_data), "ST2-05");
    r.game = r.game.clone();
    let attack = encode_attack(h.index as u16, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[attack as usize], 1.0);
    r.game.decode_action(attack, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 0, "same inherited source remains spent after evolution");
    r.end_turn();
    r.auto_resolve().unwrap();
    r.end_turn();
    r.auto_resolve().unwrap();
    r.game.set_memory(0);
    assert_eq!(build_action_mask(&r.game, 0)[attack as usize], 1.0);
    r.game.decode_action(attack, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 1, "next turn restores the evolved carrier's inherited use");
}
