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
//!   consume its `[Once Per Turn]`;
//! - a REFIRED effect ("activate … as an effect of this Digimon", §15-15-7-4)
//!   whose optional processing condition can't be executed is not offered
//!   either (G-ENGINE-REFIRED-EFFECT-SKIPS-OPTIONAL-DECISION);
//! - a condition paid by a `select_union_zone` pick that `trash_union_bound`
//!   trashes ("by trashing 1 Option card from any of your Digimon's
//!   digivolution cards", BT25-083) is offered only while that pick has a
//!   card, read like the pick itself reads it (a DUAL digivolution card is an
//!   Option card there, §4-6-6-2, official Q&A Q6395).

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, GamePhase};
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

/// "[On Play] By trashing your top security card, gain 1 memory." and the
/// mandatory "[When Attacking] Activate 1 of this Digimon's [On Play] effects."
const REFIRE_TRASH_SEC: &str = r#"
card: T-UNPAY-REFIRE
name: RefireTester
kind: digimon
level: 4
color: [red]
cost: 5
dp: 5000
effects:
  - when: on_play
    optional: true
    summary: "[On Play] By trashing your top security card, gain 1 memory."
    process:
      - trash_top_security: { of: you }
      - gain_memory: 1
  - when: when_attacking
    summary: "[When Attacking] Activate 1 of this Digimon's [On Play] effects."
    process:
      - refire_effect: { source: source, timing: on_play }
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

/// §15-15-7-4 with §15-7-4: the mandatory [When Attacking] activates the
/// [On Play] "By trashing your top security card, gain 1 memory". With 0
/// security cards the refired effect's condition can't be executed, so no
/// decision is offered and nothing is gained. Control: with 1 security card
/// the refired effect's own optional decision IS offered (the invoker being
/// mandatory does not force it), and accepting it pays and gains.
#[test]
fn unpayable_refired_optional_condition_is_not_offered() {
    let board = |security: &[&str]| {
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(REFIRE_TRASH_SEC)
            .expect("tester compiles")
            .add_card(make_test_card("FILL", "Filler"))
            .deck(0, &["FILL"; 4])
            .deck(1, &["FILL"; 4])
            .security(0, security)
            .memory(3)
            .start();
        let me = r.place_on_field(0, "T-UNPAY-REFIRE", Some(0));
        let memory_before = r.memory();
        r.game
            .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(me));
        r.game.drain_effect_queue();
        (r, memory_before)
    };

    let (r, memory_before) = board(&[]);
    assert!(
        r.pending_selection().is_none(),
        "0 security cards: the refired condition is not offered: {:?}",
        r.pending_selection()
    );
    assert_eq!(r.memory(), memory_before, "the payload did not run for free");

    let (mut r, memory_before) = board(&["FILL"]);
    let sel = r
        .pending_selection()
        .expect("1 security card: the refired effect's own decision is offered");
    assert_eq!(sel.selecting_player, 0);
    assert!(sel.is_optional, "the refired condition can be declined: {sel:?}");
    assert_eq!(r.security_count(0), 1, "nothing is paid before the decision");
    let accept = sel
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("an accept action");
    r.execute_action(0, accept).expect("accept the refired condition");
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 0, "the top security card was trashed");
    assert_eq!(r.memory(), memory_before + 1, "the refired effect gained 1 memory");
}

/// "[When Attacking] By trashing 1 Option card from any of your Digimon's
/// digivolution cards, gain 1 memory." -- LadyDevimon (BT25-083)'s condition
/// shape: a clause-level "you may", a mandatory material union pick, and
/// `trash_union_bound` paying it.
const WA_TRASH_SOURCE_OPTION: &str = r#"
card: T-UNPAY-UNION
name: UnionTester
kind: digimon
level: 5
color: [purple]
cost: 7
dp: 7000
effects:
  - when: when_attacking
    optional: true
    summary: "[When Attacking] By trashing 1 Option card from any of your Digimon's digivolution cards, gain 1 memory."
    process:
      - select_union_zone:
          of: you
          zones: [material]
          bind_as: paid
          filter: { kind: option }
          prompt: "Trash 1 Option card from any of your Digimon's digivolution cards"
      - trash_union_bound: { binding: paid }
      - gain_memory: 1
"#;

/// §15-7-4 for a union-zone trash condition. Under the tester (its only
/// digivolution card):
/// - `FILL` (a Digimon card): no Option card to trash, so the effect is not
///   offered and nothing is gained;
/// - `OPT` (an Option card): offered; executing it trashes `OPT` and gains 1;
/// - `BT25-085` (a DUAL card): offered -- the guard reads it as an Option card
///   exactly as the pick does (§4-6-6-2, Q6395) -- and executing it trashes it.
#[test]
fn unpayable_union_zone_trash_condition_is_not_offered() {
    let board = |source: &str| {
        let mut option = make_test_card("OPT", "Plain Option");
        option.card_kind = CardKind::Option;
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(WA_TRASH_SOURCE_OPTION)
            .expect("tester compiles")
            .dsl_card("BT25-085")
            .expect("BT25-085 compiles")
            .add_card(make_test_card("FILL", "Filler"))
            .add_card(option)
            .deck(0, &["FILL"; 4])
            .deck(1, &["FILL"; 4])
            .memory(3)
            .start();
        let me = r.place_stack(0, &[source, "T-UNPAY-UNION"]);
        let memory_before = r.memory();
        r.game
            .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(me));
        r.game.drain_effect_queue();
        (r, me, memory_before)
    };

    let (r, me, memory_before) = board("FILL");
    assert!(
        r.pending_selection().is_none(),
        "no Option card in any digivolution cards: the condition is not offered: {:?}",
        r.pending_selection()
    );
    assert_eq!(r.memory(), memory_before, "the payload did not run for free");
    assert_eq!(r.game.players[0].battle_area[me.index as usize].card_sources.len(), 2);

    for source in ["OPT", "BT25-085"] {
        let (mut r, me, memory_before) = board(source);
        let sel = r
            .pending_selection()
            .unwrap_or_else(|| panic!("{source}: the condition is payable and offered"));
        assert_eq!(sel.selecting_player, 0);
        assert!(sel.is_optional, "{source}: the condition can be declined: {sel:?}");
        r.accept_optional_trigger()
            .unwrap_or_else(|e| panic!("{source}: accept: {e:?}"));
        let sel = r
            .pending_selection()
            .unwrap_or_else(|| panic!("{source}: the pick has its card"));
        assert!(
            matches!(sel.kind, SelectionKind::UnionZone { .. }),
            "{source}: {sel:?}"
        );
        let pick = sel.valid_action_ids[0];
        r.execute_action(0, pick)
            .unwrap_or_else(|e| panic!("{source}: pay: {e:?}"));
        let _ = r.auto_resolve();
        assert_eq!(
            r.game.players[0].battle_area[me.index as usize].card_sources.len(),
            1,
            "{source} was trashed from the digivolution cards"
        );
        assert_eq!(r.memory(), memory_before + 1, "{source}: gained 1 memory");
    }
}
