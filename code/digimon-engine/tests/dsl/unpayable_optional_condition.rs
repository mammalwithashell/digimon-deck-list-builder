//! Engine pins for the optional-processing-condition payability guard
//! (`dsl_cards::lower_triggered::leading_cost_guard` / `CostGuard`,
//! G-ENGINE-UNPAYABLE-OPTIONAL-CONDITION-OFFERED).
//!
//! Comprehensive Rules Ver. 4.3 §15-7-4: "If executing an optional processing
//! condition's contents isn't possible, then the player can't choose to
//! execute them." §15-8-4-4-1: an activation-type effect with an optional
//! processing condition can only be declared while the condition can be
//! performed. Owner ruling 2026-10-10 #6: a trigger whose condition can't be
//! paid is not offered.
//!
//! The rule-level conformance tests live in
//! `tests/rules_conformance/ch15_07_optional_conditions.rs`; these pin two
//! engine paths the guard also drives:
//! - a `TriggerOrder` bundle excludes an unpayable optional entry only
//!   transiently, and RE-OFFERS it once a sibling has made it payable;
//! - an unpayable `[Main]` can't be declared, so attempting it does not
//!   consume its `[Once Per Turn]`.

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, GamePhase};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

/// "[On Play] <Draw 1>." and "[On Play] By trashing 1 card in your hand,
/// gain 1 memory." -- two simultaneous triggers of one player; the second's
/// condition is payable only after the first has drawn.
const DRAW_THEN_PAY: &str = r#"
card: T-UNPAY-ORDER
name: OrderTester
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_play
    summary: "[On Play] <Draw 1>"
    process:
      - draw: { of: you, count: 1 }
  - when: on_play
    optional: true
    summary: "[On Play] By trashing 1 card in your hand, gain 1 memory."
    process:
      - select_hand:
          of: you
          bind_as: paid
          optional: true
          cost: true
          filter: {}
          prompt: "By trashing 1 card in your hand, gain 1 memory"
      - trash_from_hand_by_index: { of: you, hand_index: paid }
      - gain_memory: 1
"#;

/// "[Main] [Once Per Turn] By trashing your top security card, <Draw 1>."
const MAIN_OPT_TRASH_SEC: &str = r#"
card: T-UNPAY-MAIN-OPT
name: MainOptTester
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: main_on_field
    once_per_turn: true
    summary: "[Main] [Once Per Turn] By trashing your top security card, <Draw 1>"
    process:
      - trash_top_security: { of: you }
      - draw: { of: you, count: 1 }
"#;

fn field_main_action(h: PermanentHandle) -> u16 {
    FIELD_EFFECT_START + h.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}

/// With an empty hand the second trigger's "by trashing 1 card in your hand"
/// can't be executed, so it is not an orderable entry: no `TriggerOrder`
/// prompt is shown and the <Draw 1> resolves alone. It stays pending, and
/// once the draw has put a card in the hand it IS offered (its optional hand
/// pick), and paying it gains the memory.
#[test]
fn unpayable_trigger_order_entry_is_excluded_then_reoffered_when_payable() {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(DRAW_THEN_PAY)
        .expect("tester compiles")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(3)
        .start();
    assert_eq!(r.hand_size(0), 0, "the hand starts empty");
    let me = r.place_on_field(0, "T-UNPAY-ORDER", Some(0));
    let memory_before = r.memory();

    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();

    let sel = r
        .pending_selection()
        .expect("the hand-trash condition is offered once the draw made it payable");
    assert_ne!(
        sel.kind,
        SelectionKind::TriggerOrder,
        "the unpayable entry was not orderable beside <Draw 1>: {sel:?}"
    );
    assert_eq!(sel.selecting_player, 0);
    assert!(sel.is_optional, "the condition is a \"you may\": {sel:?}");
    assert_eq!(r.hand_size(0), 1, "<Draw 1> resolved first");

    let pick = sel
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("the drawn card can be trashed");
    r.execute_action(0, pick).expect("pay the condition");
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 0, "the drawn card was trashed as the condition");
    assert_eq!(
        r.memory(),
        memory_before + 1,
        "the processing after the condition was performed"
    );
}

/// §15-8-4-4-1: with 0 security cards "[Main] [Once Per Turn] By trashing your
/// top security card, <Draw 1>" can't be declared. Forcing the action anyway
/// activates nothing and does NOT spend the [Once Per Turn]: once a security
/// card exists the same turn, the [Main] is declarable and resolves; after
/// that it is spent.
#[test]
fn unpayable_main_does_not_consume_its_once_per_turn() {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(MAIN_OPT_TRASH_SEC)
        .expect("tester compiles")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(3)
        .start();
    let main = r.place_on_field(0, "T-UNPAY-MAIN-OPT", Some(0));
    if r.current_phase() != GamePhase::Main {
        r.game.enter_main_phase();
    }
    let action = field_main_action(main);
    assert_eq!(r.security_count(0), 0);
    assert_eq!(
        build_action_mask(&r.game, 0)[action as usize],
        0.0,
        "0 security cards: the [Main] can't be declared"
    );

    let hand_before = r.hand_size(0);
    assert!(
        !r.game.activate_field_main(0, main.index as usize),
        "the unpayable [Main] does not activate"
    );
    assert_eq!(r.hand_size(0), hand_before, "nothing was drawn for free");

    r.game
        .setup_security_for_player(0, 1)
        .expect("place 1 security card");
    assert_eq!(r.security_count(0), 1);
    assert_eq!(
        build_action_mask(&r.game, 0)[action as usize],
        1.0,
        "payable now, and its [Once Per Turn] was not spent by the refused attempt"
    );
    r.game.decode_action(action, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 0, "the top security card was trashed");
    assert_eq!(r.hand_size(0), hand_before + 1, "<Draw 1> resolved");

    r.game
        .setup_security_for_player(0, 1)
        .expect("place another security card");
    assert_eq!(
        build_action_mask(&r.game, 0)[action as usize],
        0.0,
        "control: after a real activation the [Once Per Turn] is spent"
    );
}
