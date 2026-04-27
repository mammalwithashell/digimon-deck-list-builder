//! Phase 9 Task 8 — OnAllyAttack + OnOpponentAttack dispatch.
//!
//! `OnAllyAttack` fires on every permanent in the attacker-controller's
//! battle area EXCEPT the attacker itself; `OnOpponentAttack` fires on
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

/// Test 1 — OnAllyAttack fires on attacker-controller's OTHER Digimon only.
/// The attacker itself does NOT fire its own OnAllyAttack; the opposing
/// controller's Digimon do NOT fire it either.
///
/// To keep the controller's trigger bundle to a single entry (avoiding the
/// drainer's TriggerOrder prompt), only WIT1 carries the OnAllyAttack
/// observer; WIT2 is a non-observing permanent whose presence on the
/// attacker's side also asserts "ally without observer doesn't fire".
/// WIT3 is on the opponent's side; it also carries an OnAllyAttack
/// observer but MUST NOT fire because it's not an "ally" of the attacker.
#[test]
fn on_ally_attack_fires_on_same_controller_permanents_except_attacker() {
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

    let entries = ally_fires.lock().unwrap().clone();
    // WIT1 must fire (same controller as attacker, not attacker).
    assert!(
        entries.contains(&wit1),
        "WIT1 (P0 ally) should fire OnAllyAttack — got {:?}",
        entries
    );
    // Attacker must NOT fire its own OnAllyAttack.
    assert!(
        !entries.contains(&atk),
        "ATK (attacker) must NOT fire its own OnAllyAttack — got {:?}",
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
