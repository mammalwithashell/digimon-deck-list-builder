//! EX7-002 Hiyarimon: official observation ebd670529b9e530a6b9adaabbddc5c94fcfb29717da299ff2409daee05fcc01d.
//! CR 4.3: 3-4-7 (breeding isolation), 15-14-1 (once-per-turn).
use digimon_engine::debug_runner::DebugRunner;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX7-002")
        .expect("EX7-002 production YAML")
        .dsl_card("BT16-082")
        .expect("Ukkomon carrier")
        .dsl_card("BT16-085")
        .expect("Tamer control")
        .deck(0, &["BT16-082"; 20])
        .deck(1, &["BT16-082"; 20])
        .security(1, &["BT16-085"; 5])
        .memory(10)
        .start()
}

fn attack(r: &mut DebugRunner, h: digimon_engine::permanent::PermanentHandle) {
    r.game_mut().current_phase = digimon_engine::enums::GamePhase::Main;
    let action = digimon_engine::action::space::encode_attack(h.index as u16, 14);
    let mask = digimon_engine::action::mask::build_action_mask(&r.game, 0);
    assert!(mask[action as usize] > 0.5, "security attack must be legal");
    assert_ne!(
        r.attack_player(h, 1, false),
        digimon_engine::combat::AttackResult::Invalid
    );
    r.auto_resolve().expect("resolve attack");
}

#[test]
fn ex7_002_draws_with_empty_opponent_board() {
    let mut r = runner();
    let h = r.place_stack(0, &["EX7-002", "BT16-082"]);
    let before = r.hand_size(0);
    attack(&mut r, h);
    assert_eq!(r.hand_size(0), before + 1);
}

#[test]
fn ex7_002_draws_when_opponent_has_only_sourceless_digimon() {
    let mut r = runner();
    r.place_stack(1, &["BT16-082"]);
    let h = r.place_stack(0, &["EX7-002", "BT16-082"]);
    let before = r.hand_size(0);
    attack(&mut r, h);
    assert_eq!(r.hand_size(0), before + 1);
}

#[test]
fn ex7_002_does_not_draw_if_any_opponent_digimon_has_sources() {
    let mut r = runner();
    r.place_stack(1, &["BT16-082"]);
    r.place_stack(1, &["EX7-002", "BT16-082"]);
    let h = r.place_stack(0, &["EX7-002", "BT16-082"]);
    let before = r.hand_size(0);
    attack(&mut r, h);
    assert_eq!(r.hand_size(0), before);
}

#[test]
fn ex7_002_inherited_effect_is_once_per_turn() {
    let mut r = runner();
    let h = r.place_stack(0, &["EX7-002", "BT16-082"]);
    let before = r.hand_size(0);
    attack(&mut r, h);
    r.game_mut().players[0].battle_area[h.index as usize].is_suspended = false;
    attack(&mut r, h);
    assert_eq!(r.hand_size(0), before + 1);
}

#[test]
fn ex7_002_opponent_breeding_sources_do_not_prevent_draw() {
    let mut r = runner();
    r.place_stack(1, &["EX7-002", "BT16-082"]);
    let breeding = r.game_mut().players[1].battle_area.remove(0);
    r.game_mut().players[1].breeding_area = Some(breeding);
    let h = r.place_stack(0, &["EX7-002", "BT16-082"]);
    let before = r.hand_size(0);
    attack(&mut r, h);
    assert_eq!(r.hand_size(0), before + 1);
}

#[test]
fn ex7_002_does_not_trigger_for_another_digimons_attack() {
    let mut r = runner();
    r.place_stack(0, &["EX7-002", "BT16-082"]);
    let other = r.place_stack(0, &["BT16-082"]);
    let before = r.hand_size(0);
    attack(&mut r, other);
    assert_eq!(r.hand_size(0), before);
}

#[test]
fn ex7_002_failed_condition_does_not_spend_once_per_turn() {
    let mut r = runner();
    r.place_stack(1, &["EX7-002", "BT16-082"]);
    let h = r.place_stack(0, &["EX7-002", "BT16-082"]);
    let before = r.hand_size(0);
    attack(&mut r, h);
    assert_eq!(r.hand_size(0), before);
    r.game_mut().players[1].battle_area[0]
        .card_sources
        .remove(0);
    r.game_mut().players[0].battle_area[h.index as usize].is_suspended = false;
    attack(&mut r, h);
    assert_eq!(r.hand_size(0), before + 1);
}

#[test]
fn ex7_002_opponent_tamer_sources_do_not_prevent_draw() {
    let mut r = runner();
    r.place_stack(1, &["EX7-002", "BT16-085"]);
    let h = r.place_stack(0, &["EX7-002", "BT16-082"]);
    let before = r.hand_size(0);
    attack(&mut r, h);
    assert_eq!(r.hand_size(0), before + 1);
}
