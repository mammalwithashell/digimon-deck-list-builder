//! §11-6-2 (Ver. 4.3): "For the end of the attack timing, the attack won't
//! end until all processing has been resolved."
//!
//! The rule's own case is pinned in
//! `rules_conformance::ch11_14_attack_procedure::attack_does_not_end_until_all_processing_resolved`.
//! These cover the parked `AttackState::EndOfAttackDrain` state itself
//! (G-ENGINE-ATTACK-ENDS-BEFORE-END-OF-ATTACK-EFFECTS-RESOLVE):
//!   - while the End of Attack pool waits on a decision the attacker is still
//!     attacking and the attack's outcome is carried in the state;
//!   - the parked state is plain data: a clone of the game parked there
//!     resolves the decision and ends the attack on its own, leaving the
//!     original parked (the resume is `advance_pending_attack`, not a
//!     closure; `Game: Clone` powers search).
//!
//! And the end of turn after an end-of-turn attack (§16-44-1,
//! G-ENGINE-ENGAGE-EOT-PASS-ONLY-STEP): with another end-of-turn action
//! still available the window stays open; once none is left the turn passes
//! with no PASS-only step.

use std::sync::{Arc, Mutex};

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::combat::AttackResult;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{Expiry, GamePhase, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{AttackState, AttackTarget};

const LINGER_YAML: &str = r#"
card: EOA-LINGER
name: Lingermon
kind: digimon
level: 4
color: [red]
cost: 5
dp: 6000
effects:
  - when: when_attacking
    summary: "[When Attacking] This Digimon gets +2000 DP for the attack."
    process:
      - add_dp_modifier:
          target: source
          value: 2000
          expiry: end_of_attack
  - when: end_of_attack
    optional: true
    summary: "[End of Attack] You may gain 1 memory."
    process:
      - gain_memory: 1
"#;

fn board() -> (DebugRunner, PermanentHandle) {
    let mut fill = make_test_card("FILL", "FILL");
    fill.dp = Some(1000);
    fill.level = Some(3);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(LINGER_YAML)
        .expect("attacker loads")
        .add_card(fill)
        .security(1, &["FILL", "FILL", "FILL"])
        .start();
    let atk = r.place_on_field(0, "EOA-LINGER", Some(0));
    (r, atk)
}

fn is_attacking(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_attacking
}

/// P0's `EOA-LINGER` attacks P1 and survives the security check; its optional
/// `[End of Attack]` waits on P0. The attack is parked in `EndOfAttackDrain`
/// carrying its outcome; the attacker is still attacking and keeps "+2000 DP
/// for the attack". The entry point reports `InProgress`.
#[test]
fn parked_end_of_attack_keeps_the_attack_in_progress() {
    let (mut r, atk) = board();
    let result = r.attack_player(atk, 1, false);
    assert_eq!(result, AttackResult::InProgress, "the attack has not ended");
    let sel = r.pending_selection().expect("the [End of Attack] decision");
    assert!(sel.is_optional && sel.selecting_player == 0, "{sel:?}");
    let pa = r.game.pending_attack.as_ref().expect("the attack is still going");
    assert_eq!(
        pa.state,
        AttackState::EndOfAttackDrain(AttackResult::SecurityCheckSurvived)
    );
    assert!(is_attacking(&r, atk), "the attacker is still attacking");
    assert_eq!(r.game.current_attacker(), Some(atk));
    assert_eq!(r.effective_dp(atk), Some(8000));

    r.decline_optional_trigger().expect("decline");
    assert!(r.game.pending_attack.is_none(), "now the attack has ended");
    assert!(!is_attacking(&r, atk));
    assert_eq!(r.effective_dp(atk), Some(6000));
}

/// Clone the game while it is parked in `EndOfAttackDrain` and resolve the
/// decision on the clone: the clone's attack ends (memory gained, DP back to
/// 6000), while the original stays parked and then ends on its own decision.
#[test]
fn clone_parked_in_end_of_attack_drain_resolves_independently() {
    let (mut r, atk) = board();
    r.attack_player(atk, 1, false);
    assert!(r.pending_selection().is_some(), "parked on the decision");
    let memory_before = r.game.memory;

    // Swap the clone in, resolve on it, swap the original back.
    let mut other = r.game.clone();
    std::mem::swap(&mut r.game, &mut other);
    r.accept_optional_trigger().expect("accept on the clone");
    assert!(r.pending_selection().is_none());
    assert!(r.game.pending_attack.is_none(), "the clone's attack ended");
    assert_eq!(r.game.memory, memory_before + 1, "the clone gained 1 memory");
    assert_eq!(r.effective_dp(atk), Some(6000));
    assert!(!is_attacking(&r, atk));
    std::mem::swap(&mut r.game, &mut other);

    assert!(
        matches!(
            r.game.pending_attack.as_ref().map(|pa| pa.state),
            Some(AttackState::EndOfAttackDrain(_))
        ),
        "the original is still parked"
    );
    assert_eq!(r.game.memory, memory_before);
    assert_eq!(r.effective_dp(atk), Some(8000));
    r.decline_optional_trigger().expect("decline on the original");
    assert!(r.game.pending_attack.is_none());
    assert_eq!(r.game.memory, memory_before, "declined: no memory");
    assert_eq!(r.effective_dp(atk), Some(6000));
}

/// P0 has two `<Engage>` Digimon. At the end of P0's turn both may attack.
/// After the first attacks, the `EndOfTurnAction` window stays open and the
/// second's attack is still offered (with PASS). After the second attacks,
/// nothing is left: the turn passes at once, with no PASS-only step.
#[test]
fn end_of_turn_window_stays_open_only_while_another_action_remains() {
    let mut eng = make_test_card("ENG", "ENG");
    eng.dp = Some(6000);
    eng.level = Some(4);
    let mut fill = make_test_card("FILL", "FILL");
    fill.dp = Some(1000);
    fill.level = Some(3);
    let mut r = DebugRunner::builder()
        .add_card(eng)
        .add_card(fill)
        .security(1, &["FILL", "FILL", "FILL"])
        .start();
    assert_eq!(r.game.turn_player(), 0);
    let a = r.place_on_field(0, "ENG", Some(0));
    let b = r.place_on_field(0, "ENG", Some(0));
    for h in [a, b] {
        r.game
            .modifiers
            .grant_keyword(h, Keyword::Engage, Expiry::EndOfTurn, 0);
    }
    r.game.end_turn();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.current_phase, GamePhase::EndOfTurnAction);

    r.game
        .decode_action(encode_attack(a.index as u16, SECURITY_TARGET), 0);
    assert!(r.pending_selection().is_none());
    assert!(r.game.pending_attack.is_none(), "the first attack ended");
    assert_eq!(r.security_count(1), 2, "it checked security");
    assert_eq!(
        r.game.current_phase,
        GamePhase::EndOfTurnAction,
        "the second <Engage> attack is still available"
    );
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[encode_attack(b.index as u16, SECURITY_TARGET) as usize],
        1.0
    );
    assert_eq!(mask[PASS as usize], 1.0);

    r.game
        .decode_action(encode_attack(b.index as u16, SECURITY_TARGET), 0);
    assert_eq!(r.security_count(1), 1, "the second attack checked security");
    assert_eq!(r.game.turn_player(), 1, "no PASS-only step: the turn passed");
}

/// An End of Attack effect that tries to declare a new attack with another
/// Digimon.
struct EndOfAttackDeclares(PermanentHandle, Arc<Mutex<Vec<AttackResult>>>);
impl CardEffect for EndOfAttackDeclares {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let other = self.0;
        let log = self.1.clone();
        vec![Effect::end_of_attack(c)
            .name("At the end of an attack, another Digimon attacks")
            .process(move |ctx| {
                let result = ctx.game.begin_attack(other, AttackTarget::Player(1), false);
                log.lock().unwrap().push(result);
            })
            .build()]
    }
}

/// While the End of Attack timing's pool is resolving the attack has not
/// ended (§11-6-2), so a new attack declared from it would overwrite the
/// attack in progress (its attacker would stay marked attacking and its end
/// would never run). No card does this; the declaration is refused and the
/// first attack ends normally.
#[test]
fn new_attack_declared_from_the_end_of_attack_pool_is_refused() {
    let mut atk = make_test_card("ATK", "ATK");
    atk.dp = Some(6000);
    atk.level = Some(4);
    let mut ally = make_test_card("ALLY", "ALLY");
    ally.dp = Some(5000);
    ally.level = Some(4);
    let mut fill = make_test_card("FILL", "FILL");
    fill.dp = Some(1000);
    fill.level = Some(3);
    let mut r = DebugRunner::builder()
        .add_card(atk)
        .add_card(ally)
        .add_card(fill)
        .security(1, &["FILL", "FILL", "FILL"])
        .start();
    let a = r.place_on_field(0, "ATK", Some(0));
    let b = r.place_on_field(0, "ALLY", Some(0));
    let log = Arc::new(Mutex::new(Vec::new()));
    r.register_effect("ATK", Arc::new(EndOfAttackDeclares(b, log.clone())));

    let result = r.attack_player(a, 1, false);
    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(
        *log.lock().unwrap(),
        vec![AttackResult::Invalid],
        "the declaration from the End of Attack pool is refused"
    );
    assert!(r.game.pending_attack.is_none(), "the first attack ended");
    assert!(!is_attacking(&r, a));
    assert!(!is_attacking(&r, b));
    assert!(
        !r.game.players[0].battle_area[b.index as usize].is_suspended,
        "ALLY did not attack"
    );
    assert_eq!(r.security_count(1), 2, "only the first attack checked security");
}
