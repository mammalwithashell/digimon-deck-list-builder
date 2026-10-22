//! BT23-076: card image, official bundle, and DCGO BT23_076.cs agree.
use digimon_engine::action::space::{ATTACK_START, PASS, TRASH_EFFECT_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
const ID: &str = "BT23-076";
fn card(id: &str, name: &str, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(level);
    c.dp = Some(3000);
    c.traits = traits.iter().map(|s| s.to_string()).collect();
    c.evo_costs = vec![EvoCost {
        card_color: 0,
        level: level - 1,
        memory_cost: 3,
    }];
    c
}
fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT23-076 YAML loads")
        .add_card(card("BASE", "Base", 3, &[]))
        .add_card(card("WRONG-BASE", "Wrong", 5, &[]))
        .add_card(card("HUCK", "BaoHuckmon", 4, &[]))
        .add_card(card("RK", "Knight", 4, &["Royal Knight"]))
        .add_card(card("CS", "Cyber", 4, &["CS"]))
        .add_card(card("PLAIN", "Plain", 4, &[]))
        .add_card(card("WRONG-EVO", "Huckmon", 6, &[]))
        .add_card({
            let mut c = card("TAMER", "Huckmon", 4, &["CS"]);
            c.card_kind = CardKind::Tamer;
            c.level = None;
            c
        })
        .add_card(card("FILL", "Filler", 3, &[]))
}
fn setup(hand: &[&str], trash: &[&str]) -> DebugRunner {
    let mut r = builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .memory(7)
        .start();
    for id in trash {
        r.inject_trash(0, id);
    }
    r.place_stack(0, &[ID]);
    r.place_stack(0, &["BASE"]);
    r.place_stack(0, &["WRONG-BASE"]);
    r.place_stack(1, &["BASE"]);
    r
}
fn suspend(r: &mut DebugRunner, player: u8, index: usize) {
    let h = digimon_engine::permanent::PermanentHandle {
        player,
        index: index as u8,
    };
    r.game.suspend(h);
    r.game.drain_effect_queue();
}
fn choices(r: &DebugRunner, expected: &[u16]) {
    let v = r.pending_selection_view().expect("selection");
    let mut got = v.valid_action_ids.clone();
    got.sort();
    let mut want = expected.to_vec();
    want.sort();
    assert_eq!(got, want);
    let m = digimon_engine::action::mask::build_action_mask(&r.game, v.selecting_player);
    for a in got {
        assert!(m[a as usize] > 0.5);
    }
    assert!(v.is_optional);
    assert!(m[PASS as usize] > 0.5);
}
fn pick(r: &mut DebugRunner, a: u16) {
    r.execute_action(0, a).unwrap();
}
fn top(r: &DebugRunner, index: usize) -> &str {
    r.game.players[0].battle_area[index]
        .top_card()
        .card_id(&r.game.card_data)
}
#[test]
fn bt23_076_on_play_exchanges_top_security_then_recovers() {
    let mut r = builder()
        .hand(0, &[ID])
        .security(0, &["BASE", "HUCK"])
        .deck(0, &["FILL", "CS"])
        .deck(1, &["FILL"; 6])
        .memory(7)
        .start();
    assert!(r.play(0, 0).is_some());
    r.auto_resolve().unwrap();
    assert_eq!(r.security_count(0), 2);
    assert_eq!(r.game.players[0].hand[0].card_id(&r.game.card_data), "HUCK");
    assert_eq!(
        r.game.players[0]
            .security
            .last()
            .unwrap()
            .card_id(&r.game.card_data),
        "CS"
    );
    assert_eq!(r.deck_size(0), 1);
}
#[test]
fn bt23_076_empty_security_still_recovers() {
    let mut r = builder()
        .hand(0, &[ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 6])
        .memory(7)
        .start();
    assert!(r.play(0, 0).is_some());
    r.auto_resolve().unwrap();
    assert_eq!(r.security_count(0), 1);
    assert!(r.game.players[0].hand.is_empty());
}
#[test]
fn bt23_076_union_mask_includes_names_and_traits_excludes_illegal_routes_and_self() {
    let mut r = setup(
        &["HUCK", "CS", "PLAIN", "TAMER", "WRONG-EVO"],
        &["RK", "PLAIN"],
    );
    suspend(&mut r, 0, 0);
    // Wrong-base can evolve to WRONG-EVO, so both legal other Digimon appear.
    choices(&r, &[ATTACK_START + 1, ATTACK_START + 2]);
    pick(&mut r, ATTACK_START + 1);
    choices(&r, &[0, 1, TRASH_EFFECT_START]);
    let before = r.memory();
    let deck = r.deck_size(0);
    pick(&mut r, 0);
    r.auto_resolve().unwrap();
    assert_eq!(top(&r, 1), "HUCK");
    assert_eq!(top(&r, 0), ID);
    assert_eq!(r.memory(), before - 2);
    assert_eq!(r.deck_size(0), deck - 1);
}
#[test]
fn bt23_076_evolves_from_trash_pays_reduced_cost() {
    let mut r = setup(&["PLAIN"], &["CS"]);
    suspend(&mut r, 0, 0);
    choices(&r, &[ATTACK_START + 1]);
    pick(&mut r, ATTACK_START + 1);
    choices(&r, &[TRASH_EFFECT_START]);
    let before = r.memory();
    pick(&mut r, TRASH_EFFECT_START);
    r.auto_resolve().unwrap();
    assert_eq!(top(&r, 1), "CS");
    assert_eq!(r.memory(), before - 2);
    assert!(r.game.players[0].trash.is_empty());
}
#[test]
fn bt23_076_can_decline_either_base_or_result() {
    for decline_result in [false, true] {
        let mut r = setup(&["HUCK"], &[]);
        suspend(&mut r, 0, 0);
        if decline_result {
            pick(&mut r, ATTACK_START + 1);
        }
        pick(&mut r, PASS);
        r.auto_resolve().unwrap();
        assert_eq!(top(&r, 1), "BASE");
        assert_eq!(r.memory(), 7);
        assert_eq!(r.game.players[0].hand.len(), 1);
    }
}
#[test]
fn bt23_076_other_suspend_does_not_trigger() {
    for p in [0, 1] {
        let mut r = setup(&["HUCK"], &[]);
        suspend(&mut r, p, if p == 0 { 1 } else { 0 });
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn bt23_076_opponent_turn_does_not_trigger() {
    let mut r = setup(&["HUCK"], &[]);
    r.end_turn();
    r.auto_resolve().unwrap();
    assert_eq!(r.turn_player(), 1);
    suspend(&mut r, 0, 0);
    assert!(r.pending_selection().is_none());
}
#[test]
fn bt23_076_no_eligible_evolution_ends_without_prompt() {
    let mut r = setup(&["PLAIN", "TAMER"], &[]);
    suspend(&mut r, 0, 0);
    assert!(r.pending_selection().is_none());
    assert_eq!(top(&r, 1), "BASE");
}
#[test]
fn bt23_076_suspend_trigger_has_no_once_per_turn_limit() {
    let mut r = setup(&["HUCK"], &[]);
    for _ in 0..2 {
        suspend(&mut r, 0, 0);
        choices(&r, &[ATTACK_START + 1]);
        pick(&mut r, PASS);
        r.auto_resolve().unwrap();
        r.game.players[0].battle_area[0].is_suspended = false;
    }
}

#[test]
fn bt23_076_self_is_excluded_even_when_a_legal_evolution_exists_for_it() {
    let mut white = card("WHITE-EVO", "White", 4, &["CS"]);
    white.evo_costs[0].card_color = CardColor::White as u8;
    let mut r = builder()
        .add_card(white)
        .hand(0, &["WHITE-EVO", "HUCK"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(7)
        .start();
    r.place_stack(0, &[ID]);
    r.place_stack(0, &["BASE"]);
    suspend(&mut r, 0, 0);
    choices(&r, &[ATTACK_START + 1]);
    pick(&mut r, ATTACK_START + 1);
    choices(&r, &[1]);
}
#[test]
fn bt23_076_reduction_does_not_generate_memory_from_zero_cost() {
    let mut free = card("FREE", "Huckmon", 4, &[]);
    free.evo_costs[0].memory_cost = 0;
    let mut r = builder()
        .add_card(free)
        .hand(0, &["FREE"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(7)
        .start();
    r.place_stack(0, &[ID]);
    r.place_stack(0, &["BASE"]);
    suspend(&mut r, 0, 0);
    pick(&mut r, ATTACK_START + 1);
    pick(&mut r, 0);
    r.auto_resolve().unwrap();
    assert_eq!(top(&r, 1), "FREE");
    assert_eq!(r.memory(), 7);
}
#[test]
fn bt23_076_empty_deck_still_adds_top_security_to_hand() {
    let mut r = builder()
        .hand(0, &[ID])
        .security(0, &["CS"])
        .deck(1, &["FILL"; 6])
        .memory(7)
        .start();
    assert!(r.play(0, 0).is_some());
    r.auto_resolve().unwrap();
    assert_eq!(r.security_count(0), 0);
    assert_eq!(r.game.players[0].hand[0].card_id(&r.game.card_data), "CS");
}
#[test]
fn bt23_076_printed_metadata() {
    let r = builder().start();
    let c = r.compiled_card(ID).unwrap();
    assert_eq!(c.form.as_deref(), Some("Rookie"));
    let data = digimon_engine::debug_runner::card_data_for_test_from_compiled(c);
    for t in ["Puppet", "CS", "Rookie", "Vaccine"] {
        assert!(data.traits.iter().any(|s| s == t), "missing {t}");
    }
    assert_eq!(data.play_cost, 3);
    assert_eq!(data.dp, Some(3000));
    assert_eq!(data.level, Some(3));
    assert!(data.evo_costs.is_empty());
}
