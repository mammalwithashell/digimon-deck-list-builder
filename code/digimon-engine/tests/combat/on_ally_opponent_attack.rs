//! Phase 9 Task 8 — OnAllyAttack + OnOpponentAttack dispatch.
//!
//! `OnAllyAttack` fires on every permanent in the attacker-controller's
//! battle area INCLUDING the attacker itself (DCGO `AttackProcess.cs`
//! stacks one `EffectTiming.OnAllyAttack` batch with no self exclusion;
//! "another Digimon" is a per-card predicate, `event_target_is_source:
//! false`); `OnOpponentAttack` fires on
//! every permanent in the opposing controller's battle area. Both fan
//! out after `OnAttack` + `WhenAttacking`. Spec §7.

use std::sync::{Arc, Mutex};

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::permanent::PermanentHandle;

fn card(id: &str) -> digimon_engine::CardData {
    make_test_card(id, id)
}

/// OnAllyAttack observer that records the `source_permanent` it fires on.
struct RecordAllyAttack(Arc<Mutex<Vec<PermanentHandle>>>);
impl CardEffect for RecordAllyAttack {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let log = self.0.clone();
        vec![Effect::on_ally_attack(c)
            .name("Record OnAllyAttack")
            .process(move |ctx| {
                if let Some(h) = ctx.source_permanent {
                    log.lock().unwrap().push(h);
                }
            })
            .build()]
    }
}

/// OnOpponentAttack observer that records the `source_permanent` it fires on.
struct RecordOpponentAttack(Arc<Mutex<Vec<PermanentHandle>>>);
impl CardEffect for RecordOpponentAttack {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let log = self.0.clone();
        vec![Effect::on_opponent_attack(c)
            .name("Record OnOpponentAttack")
            .process(move |ctx| {
                if let Some(h) = ctx.source_permanent {
                    log.lock().unwrap().push(h);
                }
            })
            .build()]
    }
}

/// OnAllyAttack observer that records `pending_attack.attacker` at fire time.
struct RecordAllyAttackAttacker(Arc<Mutex<Vec<PermanentHandle>>>);
impl CardEffect for RecordAllyAttackAttacker {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let log = self.0.clone();
        vec![Effect::on_ally_attack(c)
            .name("Record attacker via OnAllyAttack")
            .process(move |ctx| {
                if let Some(pa) = ctx.game.pending_attack.as_ref() {
                    log.lock().unwrap().push(pa.attacker);
                }
            })
            .build()]
    }
}

/// Test 1 — OnAllyAttack fires on every permanent of the attacker's
/// controller, the attacker itself included ("When a Digimon attacks" has no
/// "another"); the opposing controller's Digimon do NOT fire it.
///
/// To keep the controller's trigger bundle to a single entry (avoiding the
/// drainer's TriggerOrder prompt), only WIT1 carries the OnAllyAttack
/// observer; WIT2 is a non-observing permanent whose presence on the
/// attacker's side also asserts "ally without observer doesn't fire".
/// WIT3 is on the opponent's side; it also carries an OnAllyAttack
/// observer but MUST NOT fire because it's not an "ally" of the attacker.
#[test]
fn on_ally_attack_fires_on_same_controller_permanents_including_attacker() {
    let ally_fires: Arc<Mutex<Vec<PermanentHandle>>> = Arc::new(Mutex::new(Vec::new()));

    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("WIT1"))
        .add_card(card("WIT2"))
        .add_card(card("WIT3"))
        .add_card(card("DEF"))
        .start();

    r.register_effect("ATK", Arc::new(RecordAllyAttack(ally_fires.clone())));
    r.register_effect("WIT1", Arc::new(RecordAllyAttack(ally_fires.clone())));
    r.register_effect("WIT3", Arc::new(RecordAllyAttack(ally_fires.clone())));

    let atk = r.place_on_field(0, "ATK", Some(0));
    let wit1 = r.place_on_field(0, "WIT1", Some(0));
    let _wit2 = r.place_on_field(0, "WIT2", Some(0));
    let wit3 = r.place_on_field(1, "WIT3", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    r.attack_digimon(atk, def, false);
    // Two same-controller observers (WIT1 and the attacker) now pend together,
    // so the turn player orders them (CR 4.3 15-4-3-5-1).
    r.auto_resolve().expect("order and resolve both observers");

    let entries = ally_fires.lock().unwrap().clone();
    // WIT1 must fire (same controller as attacker, not attacker).
    assert!(
        entries.contains(&wit1),
        "WIT1 (P0 ally) should fire OnAllyAttack — got {:?}",
        entries
    );
    // The attacker observes its own attack too (no self exclusion).
    assert!(
        entries.contains(&atk),
        "ATK (attacker) must fire its own OnAllyAttack — got {:?}",
        entries
    );
    // Opposing P1 digimon must NOT fire OnAllyAttack.
    assert!(
        !entries.contains(&wit3),
        "WIT3 (opposing controller) must NOT fire OnAllyAttack — got {:?}",
        entries
    );
}

/// Test 2 — OnOpponentAttack fires on the opposing controller's Digimon.
#[test]
fn on_opponent_attack_fires_on_opposite_controller() {
    let opp_fires: Arc<Mutex<Vec<PermanentHandle>>> = Arc::new(Mutex::new(Vec::new()));

    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .add_card(card("WIT"))
        .start();

    r.register_effect("WIT", Arc::new(RecordOpponentAttack(opp_fires.clone())));

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let wit = r.place_on_field(1, "WIT", Some(0));

    r.attack_digimon(atk, def, false);

    let entries = opp_fires.lock().unwrap().clone();
    assert!(
        entries.contains(&wit),
        "WIT (on opposing field) should fire OnOpponentAttack — got {:?}",
        entries
    );
}

/// Test 3 — OnAllyAttack observers see the attacker via
/// `ctx.game.pending_attack.attacker` at fire time.
#[test]
fn observers_see_attacker_via_ctx_helper() {
    let seen_attackers: Arc<Mutex<Vec<PermanentHandle>>> = Arc::new(Mutex::new(Vec::new()));

    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("WIT"))
        .add_card(card("DEF"))
        .start();

    r.register_effect(
        "WIT",
        Arc::new(RecordAllyAttackAttacker(seen_attackers.clone())),
    );

    let atk = r.place_on_field(0, "ATK", Some(0));
    let _wit = r.place_on_field(0, "WIT", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    r.attack_digimon(atk, def, false);

    let entries = seen_attackers.lock().unwrap().clone();
    assert_eq!(entries.len(), 1, "WIT should fire exactly once");
    assert_eq!(
        entries[0], atk,
        "WIT observer should read attacker == ATK handle, got {:?}",
        entries[0]
    );
}

/// "When ANOTHER Digimon attacks" (BT23-069 Necromon, P-111 Knightmon) is a
/// per-card predicate, not a property of the timing: `OnAllyAttack` includes
/// the attacker, and `trigger_if: { event_target_is_source: false }` excludes
/// it. The self-exclusion that used to be hard-wired into the dispatch made
/// "When a Digimon attacks" cards (BT18-042) miss their own attack.
const ANOTHER_ATTACKS_YAML: &str = r#"
card: DSL-ANOTHER-OBSERVER
name: Another Observer
kind: digimon
level: 3
color: [red]
cost: 2
dp: 2000
effects:
  - when: on_ally_attack
    trigger_if: { event_target_is_source: false }
    process:
      - gain_memory: 1
"#;

#[test]
fn another_digimon_attacks_excludes_the_carrier_via_event_target_is_source() {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(ANOTHER_ATTACKS_YAML)
        .expect("register observer DSL")
        .add_card(card("ALLY"))
        .add_card(card("DEF"))
        .memory(3)
        .start();
    let observer = r.place_on_field(0, "DSL-ANOTHER-OBSERVER", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    // The ally attacks: the observer sees "another Digimon" attack.
    let before = r.memory();
    r.attack_digimon(ally, def, false);
    r.auto_resolve().ok();
    assert_eq!(r.memory(), before + 1, "another Digimon attacked");

    // The observer attacks: it is not "another".
    let mut r2 = DebugRunner::builder()
        .from_dsl_yaml(ANOTHER_ATTACKS_YAML)
        .expect("register observer DSL")
        .add_card(card("DEF"))
        .memory(3)
        .start();
    let observer2 = r2.place_on_field(0, "DSL-ANOTHER-OBSERVER", Some(0));
    let def2 = r2.place_on_field(1, "DEF", Some(0));
    let before2 = r2.memory();
    r2.attack_digimon(observer2, def2, false);
    r2.auto_resolve().ok();
    assert_eq!(r2.memory(), before2, "its own attack is not 'another'");
    let _ = observer;
}
