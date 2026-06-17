//! `[When Attacking]` must be scoped to the ATTACKER's stack only.
//!
//! Regression: an inherited `[When Attacking]` effect on a digivolution
//! source under a NON-attacking permanent (e.g. Gabumon ST6-03 beneath
//! Garurumon in slot 0) wrongly fired when a DIFFERENT Digimon (DemiDevimon
//! in another slot) attacked. Root cause: `fire_on_attack` dispatched the
//! `WhenAttacking` timing battle-area-wide (`PlayerBattleArea`) instead of
//! attacker-scoped, so every permanent's own + inherited `WhenAttacking`
//! clauses fired on any friendly attack. `[When Attacking]` is carrier-scoped
//! ("when THIS Digimon attacks"); cross-Digimon observers use `OnAllyAttack`.

use std::sync::{Arc, Mutex};

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::permanent::PermanentHandle;

fn card(id: &str) -> digimon_engine::CardData {
    make_test_card(id, id)
}

/// An INHERITED `[When Attacking]` clause (like Gabumon's) that records the
/// carrier permanent it fires on. Active only while this card is a
/// digivolution source beneath another.
struct InheritedWhenAttacking(Arc<Mutex<Vec<PermanentHandle>>>);
impl CardEffect for InheritedWhenAttacking {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let log = self.0.clone();
        vec![Effect::when_attacking(c)
            .inherited()
            .name("Inherited When Attacking")
            .process(move |ctx| {
                if let Some(h) = ctx.source_permanent {
                    log.lock().unwrap().push(h);
                }
            })
            .build()]
    }
}

/// Bug repro: SRC's inherited `[When Attacking]` must NOT fire when a
/// different Digimon (ATK, in another slot) attacks.
#[test]
fn inherited_when_attacking_does_not_fire_for_other_attacker() {
    let fired: Arc<Mutex<Vec<PermanentHandle>>> = Arc::new(Mutex::new(Vec::new()));

    let mut r = DebugRunner::builder()
        .add_card(card("ATK"))
        .add_card(card("TOP"))
        .add_card(card("SRC"))
        .add_card(card("DEF"))
        .start();

    // SRC carries the inherited When Attacking clause.
    r.register_effect("SRC", Arc::new(InheritedWhenAttacking(fired.clone())));

    // Slot 0: stack [SRC (source) -> TOP (top)] — SRC is NOT the attacker.
    let _stack = r.place_stack(0, &["SRC", "TOP"]);
    // Another slot: the actual attacker, with no When Attacking effect.
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    r.attack_digimon(atk, def, false);

    let entries = fired.lock().unwrap().clone();
    assert!(
        entries.is_empty(),
        "SRC's inherited [When Attacking] must NOT fire when ATK (a different \
         Digimon) attacks — fired on {:?}",
        entries
    );
}

/// Positive control: SRC's inherited `[When Attacking]` DOES fire when its
/// own carrier stack attacks. Guards against the fix over-scoping.
#[test]
fn inherited_when_attacking_fires_when_own_stack_attacks() {
    let fired: Arc<Mutex<Vec<PermanentHandle>>> = Arc::new(Mutex::new(Vec::new()));

    let mut r = DebugRunner::builder()
        .add_card(card("TOP"))
        .add_card(card("SRC"))
        .add_card(card("DEF"))
        .start();

    r.register_effect("SRC", Arc::new(InheritedWhenAttacking(fired.clone())));

    let stack = r.place_stack(0, &["SRC", "TOP"]);
    let def = r.place_on_field(1, "DEF", Some(0));

    r.attack_digimon(stack, def, false);

    let entries = fired.lock().unwrap().clone();
    assert_eq!(
        entries.len(),
        1,
        "SRC's inherited [When Attacking] should fire exactly once when its own \
         stack attacks — fired on {:?}",
        entries
    );
    assert_eq!(
        entries[0], stack,
        "inherited clause should fire on its carrier (the attacking stack)"
    );
}
