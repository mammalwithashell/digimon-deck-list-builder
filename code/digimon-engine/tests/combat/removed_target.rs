//! §11-2-6 (Ver. 4.3): "Even if the attack target Digimon is removed during an
//! attack, that Digimon remains the attack target, but the attack fails."
//! §11-5-1-4-1: an unsuccessful attack takes no other action and goes to the
//! end of attack timing. §12-1-5: the attack target can't block.
//!
//! The rule's own example is pinned in
//! `rules_conformance::ch11_14_attack_procedure::removed_target_stays_the_target_and_attack_fails`.
//! These cover the identity-keyed bookkeeping around it
//! (G-ENGINE-REMOVED-ATTACK-TARGET-SLOT-REUSED):
//!   - attacker AND target leave in one attack: one unsuccessful end, no
//!     battle, no security check, one end of attack timing;
//!   - the Digimon that compacted into the removed target's slot is not the
//!     attack target: it may block (and is then battled, §12-1-7), and if it
//!     doesn't block it is not battled; the UI shows no target slot;
//!   - an effect that switches the attack target after the attacker left
//!     switches nothing (DCGO `SwitchDefender` yields without an attacker)
//!     and does not read the Digimon in the attacker's old slot.

use std::sync::{Arc, Mutex};

use digimon_engine::action::space::encode_attack;
use digimon_engine::card_source::CardHandle;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{Expiry, GamePhase, Keyword};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::AttackTarget;

const DEL4K_THEN_SELF_YAML: &str = r#"
card: RT-ATK-BOTH
name: Recklessmon
kind: digimon
level: 4
color: [red]
cost: 5
dp: 6000
effects:
  - when: when_attacking
    summary: "[When Attacking] Delete 1 of your opponent's Digimon with 4000 DP or less. Then, delete this Digimon."
    process:
      - select_opponent_permanent:
          bind_as: victim
          filter:
            all_of:
              - kind: digimon
              - dp_lte: 4000
          prompt: "Delete 1 of your opponent's Digimon with 4000 DP or less"
      - delete_permanent: { target: victim }
      - delete_permanent: { target: source }
"#;

const DEL4K_YAML: &str = r#"
card: RT-ATK
name: Strikeattackmon
kind: digimon
level: 4
color: [red]
cost: 5
dp: 6000
effects:
  - when: when_attacking
    summary: "[When Attacking] Delete 1 of your opponent's Digimon with 4000 DP or less."
    process:
      - select_opponent_permanent:
          bind_as: victim
          filter:
            all_of:
              - kind: digimon
              - dp_lte: 4000
          prompt: "Delete 1 of your opponent's Digimon with 4000 DP or less"
      - delete_permanent: { target: victim }
"#;

/// A broad timing witness, distinct from a printed carrier-scoped
/// `[End of Attack]` DSL clause. It observes an attack even after its
/// attacker has left, so it can prove that the timing occurred once.
struct BroadEndOfAttackWitness;
impl CardEffect for BroadEndOfAttackWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::end_of_attack(card)
            .name("At the end of an attack, gain 1 memory")
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

fn digimon(id: &str, dp: i32) -> digimon_engine::CardData {
    let mut c = make_test_card(id, id);
    c.dp = Some(dp);
    c.level = Some(4);
    c
}

fn suspend(r: &mut DebugRunner, h: PermanentHandle) {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = true;
}

fn memory_gains_from(events: &[GameEvent], id: &str) -> usize {
    events
        .iter()
        .filter(|e| {
            matches!(e, GameEvent::MemoryChange { source_card_id: Some(src), .. } if src == id)
        })
        .count()
}

fn security_reveals(events: &[GameEvent]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, GameEvent::SecurityReveal { .. }))
        .count()
}

/// Accept the first offered option of every prompt, never a block.
fn settle(r: &mut DebugRunner) {
    for _ in 0..16 {
        let Some(sel) = r.game.pending_selection.as_ref() else {
            return;
        };
        assert_ne!(r.current_phase(), GamePhase::BlockTiming, "no block expected: {sel:?}");
        let (player, action) = (sel.selecting_player, sel.valid_action_ids[0]);
        r.game.resolve_selection(player, action).expect("settle prompt");
    }
    panic!("prompts did not settle");
}

/// The attacker deletes the attack target and then itself in one
/// [When Attacking]. Both §11-1-5 (the attacker left) and §11-2-6 (the target
/// was removed) make the attack not successful: it ends ONCE (§11-5-1-4-1) --
/// no battle with `BIG`, which compacted into the target's slot, no security
/// check, and one end of attack timing (`RT-EOA`, which compacted into the
/// attacker's slot, gains memory exactly once).
#[test]
fn attacker_and_target_both_leave_one_unsuccessful_end() {
    let mut witness_card = digimon("RT-EOA", 1000);
    witness_card.card_name = "Afterattackmon".to_string();
    witness_card.level = Some(3);
    witness_card.play_cost = 2;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(DEL4K_THEN_SELF_YAML)
        .expect("attacker loads")
        .add_card(witness_card)
        .add_card(digimon("FILL", 1000))
        .add_card(digimon("BIG", 7000))
        .security(1, &["FILL", "FILL", "FILL"])
        .start();
    r.register_effect("RT-EOA", Arc::new(BroadEndOfAttackWitness));
    let atk = r.place_on_field(0, "RT-ATK-BOTH", Some(0));
    let ally = r.place_on_field(0, "RT-EOA", Some(0));
    assert_eq!((atk.index, ally.index), (0, 1));
    let target = r.place_on_field(1, "FILL", Some(0));
    let big = r.place_on_field(1, "BIG", Some(0));
    suspend(&mut r, target);
    suspend(&mut r, big);
    let cp = r.event_checkpoint();

    r.attack_digimon(atk, target, false);
    settle(&mut r);

    let top_ids = |r: &DebugRunner, p: usize| -> Vec<String> {
        r.game.players[p]
            .battle_area
            .iter()
            .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
            .collect()
    };
    assert_eq!(top_ids(&r, 1), vec!["BIG".to_string()], "BIG was never battled");
    assert_eq!(top_ids(&r, 0), vec!["RT-EOA".to_string()], "the attacker deleted itself");
    let events = r.events_since(cp);
    assert_eq!(security_reveals(events), 0, "no security check");
    assert_eq!(r.security_count(1), 3);
    assert_eq!(
        memory_gains_from(events, "RT-EOA"),
        1,
        "exactly one end of attack timing"
    );
    assert!(r.game.pending_attack.is_none(), "the attack ended");
    assert!(r.game.pending_selection.is_none());
}

/// Board for the block cases: P0's `RT-ATK` (6000) attacks P1's suspended
/// `FILL` (slot 0) and its [When Attacking] deletes it; P1's unsuspended
/// <Blocker> `BLK` (3000) compacts from slot 1 into slot 0. Returns the runner
/// parked at the block timing.
fn removed_target_block_board() -> (DebugRunner, PermanentHandle, usize) {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(DEL4K_YAML)
        .expect("attacker loads")
        .add_card(digimon("FILL", 1000))
        .add_card(digimon("BLK", 3000))
        .security(1, &["FILL", "FILL", "FILL"])
        .start();
    let atk = r.place_on_field(0, "RT-ATK", Some(0));
    let target = r.place_on_field(1, "FILL", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    assert_eq!((target.index, blk.index), (0, 1));
    suspend(&mut r, target);
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);
    let cp = r.event_checkpoint();

    r.attack_digimon(atk, target, false);
    // The [When Attacking] pick (FILL is the only Digimon with 4000 DP or
    // less), then any counter timing, up to the block timing.
    for _ in 0..8 {
        if r.current_phase() == GamePhase::BlockTiming {
            break;
        }
        let Some(sel) = r.game.pending_selection.as_ref() else {
            break;
        };
        let (player, action) = (sel.selecting_player, sel.valid_action_ids[0]);
        r.game.resolve_selection(player, action).expect("settle prompt");
    }
    assert_eq!(
        r.game.players[1].battle_area.len(),
        1,
        "the target was deleted; BLK compacted into its slot"
    );
    (r, atk, cp)
}

/// §12-1-5 excludes the attack target from blocking, but the Digimon in the
/// removed target's old slot is not the attack target (§11-2-6): `BLK` is
/// offered. The UI shows the attacker's slot and no target slot. Blocking
/// switches the attack target to `BLK` (§12-1-7), which is then battled and
/// deleted (6000 vs 3000); no security is checked.
#[test]
fn removed_target_slot_occupant_may_block_and_is_then_battled() {
    let (mut r, atk, cp) = removed_target_block_board();
    assert_eq!(r.current_phase(), GamePhase::BlockTiming, "BLK is a block candidate");
    let sel = r.game.pending_selection.as_ref().expect("block timing");
    let block = encode_attack(0, 0);
    assert_eq!(sel.valid_action_ids, vec![block], "BLK, now in slot 0, may block");

    let ui = digimon_engine::serialization::to_ui_json(&r.game);
    assert_eq!(ui["pendingAttack"]["attackerSlot"], atk.index as i64);
    assert_eq!(
        ui["pendingAttack"]["targetSlot"], -1,
        "the removed target has no slot; BLK is not the target"
    );

    r.game.resolve_selection(1, block).expect("BLK blocks");
    settle(&mut r);

    assert!(
        r.game.players[1].battle_area.is_empty(),
        "BLK blocked, became the attack target and lost the battle"
    );
    assert_eq!(r.battle_area_size(0), 1, "the attacker won");
    assert_eq!(security_reveals(r.events_since(cp)), 0, "a blocked attack checks no security");
    assert!(r.game.pending_attack.is_none());
}

/// Same board, P1 declines the block: the removed `FILL` is still the attack
/// target, so the attack fails (§11-2-6) -- `BLK` in its old slot is not
/// battled and no security is checked.
#[test]
fn removed_target_slot_occupant_not_battled_when_it_does_not_block() {
    let (mut r, _atk, cp) = removed_target_block_board();
    assert_eq!(r.current_phase(), GamePhase::BlockTiming);
    r.game
        .resolve_selection(1, digimon_engine::action::space::PASS)
        .expect("P1 declines the block");
    settle(&mut r);

    assert_eq!(r.game.players[1].battle_area.len(), 1, "BLK was not battled");
    assert!(!r.game.players[1].battle_area[0].is_suspended, "BLK did not block");
    assert_eq!(r.battle_area_size(0), 1, "the attacker battled nothing");
    assert_eq!(security_reveals(r.events_since(cp)), 0, "no security check");
    assert_eq!(r.security_count(1), 3);
    assert!(r.game.pending_attack.is_none());
}

/// [When Attacking] that deletes its own Digimon and then tries to switch the
/// attack target to the opponent.
struct LeaveThenRedirect(PermanentHandle, Arc<Mutex<Vec<String>>>);
impl CardEffect for LeaveThenRedirect {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let me = self.0;
        let log = self.1.clone();
        vec![Effect::on_attack(c)
            .name("Delete this Digimon, then switch the attack target")
            .process(move |ctx| {
                ctx.delete_permanent(me);
                let result = ctx.redirect_attack(AttackTarget::Player(1));
                log.lock().unwrap().push(format!("redirect {result:?}"));
            })
            .build()]
    }
}

/// `OnAttackTargetChange` witness.
struct WitnessTargetChange(Arc<Mutex<Vec<String>>>);
impl CardEffect for WitnessTargetChange {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let log = self.0.clone();
        vec![Effect::on_attack_target_change(c)
            .name("Witness target change")
            .process(move |_ctx| {
                log.lock().unwrap().push("target changed".to_string());
            })
            .build()]
    }
}

/// After the attacking Digimon left, the attack target can't switch: there is
/// no attacking Digimon (DCGO `SwitchDefender` yields when
/// `AttackingPermanent.TopCard == null`). The switch is a no-op -- it neither
/// validates against nor names `ALLY`, which compacted into the attacker's
/// slot, and no `OnAttackTargetChange` fires. The attack then ends as not
/// successful (§11-5-1-4-1): no battle with `DEF`.
#[test]
fn target_switch_after_attacker_left_switches_nothing() {
    let mut r = DebugRunner::builder()
        .add_card(digimon("ATK", 6000))
        .add_card(digimon("ALLY", 1000))
        .add_card(digimon("DEF", 2000))
        .security(1, &["DEF", "DEF", "DEF"])
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let _ally = r.place_on_field(0, "ALLY", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    suspend(&mut r, def);
    let log = Arc::new(Mutex::new(Vec::new()));
    r.register_effect("ATK", Arc::new(LeaveThenRedirect(atk, log.clone())));
    r.register_effect("DEF", Arc::new(WitnessTargetChange(log.clone())));
    let cp = r.event_checkpoint();

    let result = r.attack_digimon(atk, def, false);
    settle(&mut r);

    assert_eq!(
        *log.lock().unwrap(),
        vec!["redirect Ok(())".to_string()],
        "the switch is a no-op: no target change observed"
    );
    assert_ne!(result, AttackResult::InProgress);
    assert_eq!(r.game.players[1].battle_area.len(), 1, "DEF was not battled");
    assert_eq!(security_reveals(r.events_since(cp)), 0, "the attack did not reach the player");
    assert_eq!(r.security_count(1), 3);
    assert!(r.game.pending_attack.is_none());
}
