//! Phase 9 Task 8 — OnBlock observer dispatch.
//!
//! OnBlock fires globally (both players' battle areas) after a blocker is
//! declared. Observers can read `ctx.game.pending_attack.effective_target`
//! to see the post-declare blocker handle. Spec §7.

use std::sync::{Arc, Mutex};

use digimon_engine::action::space::encode_attack;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{Expiry, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::AttackTarget;

fn card(id: &str) -> digimon_engine::CardData {
    make_test_card(id, id)
}

/// OnBlock observer that increments a shared counter when it fires.
struct WitnessOnBlock(Arc<Mutex<u32>>);
impl CardEffect for WitnessOnBlock {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let counter = self.0.clone();
        vec![Effect::on_block(c)
            .name("Witness OnBlock")
            .process(move |_ctx| {
                *counter.lock().unwrap() += 1;
            })
            .build()]
    }
}

/// OnBlock observer that records the `effective_target` it sees when it fires.
struct RecordTargetOnBlock(Arc<Mutex<Vec<AttackTarget>>>);
impl CardEffect for RecordTargetOnBlock {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let log = self.0.clone();
        vec![Effect::on_block(c)
            .name("Record target OnBlock")
            .process(move |ctx| {
                if let Some(pa) = ctx.game.pending_attack.as_ref() {
                    log.lock().unwrap().push(pa.effective_target);
                }
            })
            .build()]
    }
}

/// Test 1 — OnBlock fires on observers from BOTH players' battle areas.
#[test]
fn on_block_fires_globally_when_blocker_declared() {
    let p0_count: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
    let p1_count: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));

    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .add_card(card("BLK"))
        .add_card(card("W0"))
        .add_card(card("W1"))
        .start();

    r.register_effect("W0", Arc::new(WitnessOnBlock(p0_count.clone())));
    r.register_effect("W1", Arc::new(WitnessOnBlock(p1_count.clone())));

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    let _w0 = r.place_on_field(0, "W0", Some(0));
    let _w1 = r.place_on_field(1, "W1", Some(0));

    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    r.attack_digimon(atk, def, false);
    // BLK is at slot 2 on P1 (after DEF@0, BLK@1, W1@2)? Let me recompute:
    // place order: DEF@0, BLK@1, W1@2 on P1. BLK index = 1.
    let block_action = encode_attack(0, blk.index as u16);
    r.game
        .resolve_selection(1, block_action)
        .expect("declare blocker");

    assert_eq!(*p0_count.lock().unwrap(), 1, "P0 witness saw OnBlock fire");
    assert_eq!(*p1_count.lock().unwrap(), 1, "P1 witness saw OnBlock fire");
}

/// Test 2 — OnBlock observer reads `effective_target` post-declare: it
/// resolves to the blocker handle, not the original target.
#[test]
fn on_block_sees_post_declare_state() {
    let log: Arc<Mutex<Vec<AttackTarget>>> = Arc::new(Mutex::new(Vec::new()));

    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .add_card(card("BLK"))
        .add_card(card("W"))
        .start();

    r.register_effect("W", Arc::new(RecordTargetOnBlock(log.clone())));

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    let _w = r.place_on_field(0, "W", Some(0));

    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    r.attack_digimon(atk, def, false);
    let block_action = encode_attack(0, blk.index as u16);
    r.game
        .resolve_selection(1, block_action)
        .expect("declare blocker");

    let entries = log.lock().unwrap().clone();
    assert!(!entries.is_empty(), "OnBlock observer should fire");
    // Every observation should see the blocker as effective_target, not DEF.
    for t in &entries {
        match t {
            AttackTarget::Digimon(h) => {
                assert_eq!(
                    *h, blk,
                    "effective_target at OnBlock fire time must be blocker (got {:?})",
                    h
                );
                assert_ne!(*h, def, "must not still be the original target");
            }
            AttackTarget::Player(p) => {
                panic!("effective_target unexpectedly a Player: {}", p);
            }
        }
    }
}

/// Test 3 — Collision forces a mandatory block; OnBlock still fires.
#[test]
fn on_block_fires_on_collision_forced_block() {
    let count: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));

    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("DEF"))
        .add_card(card("W"))
        .start();

    r.register_effect("W", Arc::new(WitnessOnBlock(count.clone())));

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let _w = r.place_on_field(0, "W", Some(0));

    // No Blocker keyword anywhere. Collision grants virtual Blocker to
    // every opponent Digimon and makes the block selection mandatory.
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Collision, Expiry::Permanent, 0);

    r.attack_digimon(atk, def, false);
    // Forced block — pick DEF (slot 0).
    let block_action = encode_attack(0, 0);
    r.game
        .resolve_selection(1, block_action)
        .expect("Collision block selection resolves");

    assert_eq!(
        *count.lock().unwrap(),
        1,
        "OnBlock must fire even on Collision-forced block"
    );
    let _ = def; // quiet unused-warning noise on some toolchains
    let _: PermanentHandle = atk;
}
