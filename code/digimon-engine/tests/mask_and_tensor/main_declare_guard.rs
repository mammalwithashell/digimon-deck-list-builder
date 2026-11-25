//! Loop guard for a kept `[Main]` use (owner ruling 2026-11-24, "Match dcgo
//! on the main use"; the guard design is the orchestrator's, see
//! docs/RUST_ENGINE_API.md "Main-declaration guard").
//!
//! A `[Main] [Once Per Turn]` clause that gives its use back when its pick is
//! passed (DSL `refund_opt`, DCGO `RemoveUse()`) makes "declare, then pass" a
//! decision that returns the game to the state it started from. Offered again,
//! that pair is a free no-op action an agent can repeat forever. The guard
//! withholds exactly that declaration while nothing has changed since the
//! pass, and keeps the use: any real change offers it again.
//!
//! The fixture card is synthetic (`KEEP-MAIN`): "[Main] [Once Per Turn] You
//! may play 1 Digimon card from your hand." with `refund_opt` on the passed
//! pick, so the guard is exercised before any production card refunds.

use digimon_engine::action::explain::explain_action;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS, PLAY_HAND_START,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
use digimon_engine::Game;

const KEEP: &str = r#"
card: KEEP-MAIN
name: Keeper
kind: digimon
level: 4
color: [red]
cost: 3
dp: 5000
effects:
  - when: main_on_field
    once_per_turn: true
    optional: true
    summary: "[Main] [Once Per Turn] You may play 1 Digimon card from your hand."
    process:
      - select_hand:
          of: you
          bind_as: pick
          optional: true
          filter: { kind: digimon }
          prompt: "Play 1 Digimon card from your hand"
      - if:
          condition: { binding_present: pick }
          then:
            - play_or_use_from_hand: { of: you, hand_index: pick }
          else:
            - refund_opt: {}
"#;

/// The same clause with nothing it could ever pick (no Option cards in the
/// fixture hands): the declaration refunds without a prompt.
const EMPTY: &str = r#"
card: EMPTY-MAIN
name: Emptier
kind: digimon
level: 4
color: [red]
cost: 3
dp: 5000
effects:
  - when: main_on_field
    once_per_turn: true
    optional: true
    summary: "[Main] [Once Per Turn] You may play 1 Option card from your hand."
    process:
      - select_hand:
          of: you
          bind_as: pick
          optional: true
          filter: { kind: option }
          prompt: "Play 1 Option card from your hand"
      - if:
          condition: { binding_present: pick }
          then:
            - play_or_use_from_hand: { of: you, hand_index: pick }
          else:
            - refund_opt: {}
"#;

/// An inherited refund-capable `[Main]` (bottom of the stack, matched first).
const INH_KEEP: &str = r#"
card: INH-KEEP
name: Inherited Keeper
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - scope: inherited
    when: main_on_field
    once_per_turn: true
    optional: true
    summary: "[Main] [Once Per Turn] You may play 1 Digimon card from your hand."
    process:
      - select_hand:
          of: you
          bind_as: pick
          optional: true
          filter: { kind: digimon }
          prompt: "Play 1 Digimon card from your hand"
      - if:
          condition: { binding_present: pick }
          then:
            - play_or_use_from_hand: { of: you, hand_index: pick }
          else:
            - refund_opt: {}
"#;

/// An inherited `[Main]` whose "by suspending this Digimon" cost is unpayable
/// while its carrier is suspended.
const INH_SUSPEND: &str = r#"
card: INH-SUSP
name: Inherited Suspender
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - scope: inherited
    when: main_on_field
    once_per_turn: true
    summary: "[Main] [Once Per Turn] By suspending this Digimon, gain 2 memory."
    process:
      - activation_cost: { suspend_self: true }
      - gain_memory: 2
"#;

/// The top card's own `[Main]`: gain 1 memory.
const TOP_GAIN: &str = r#"
card: TOP-GAIN
name: Top Gainer
kind: digimon
level: 4
color: [red]
cost: 3
dp: 5000
effects:
  - when: main_on_field
    once_per_turn: true
    summary: "[Main] [Once Per Turn] Gain 1 memory."
    process:
      - gain_memory: 1
"#;

/// A refund-capable `[Main]` whose pick spans hand and trash: the union pick
/// is a card choice, which allocates a card-choice id even when passed (the
/// Rie Kishibe (EX13-074) shape).
const UNION_KEEP: &str = r#"
card: UNION-MAIN
name: Union Keeper
kind: digimon
level: 4
color: [red]
cost: 3
dp: 5000
effects:
  - when: main_on_field
    once_per_turn: true
    summary: "[Main] [Once Per Turn] You may play 1 Digimon card from your hand or trash."
    process:
      - select_union_zone:
          of: you
          zones: [hand, trash]
          optional: true
          bind_as: pick
          filter: { kind: digimon }
          prompt: "Play 1 Digimon card from your hand or trash"
      - if:
          condition: { binding_present: pick }
          then:
            - draw: { of: you, count: 1 }
          else:
            - refund_opt: {}
"#;

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 2;
    c
}

fn builder() -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder();
    for yaml in [KEEP, EMPTY, INH_KEEP, INH_SUSPEND, TOP_GAIN, UNION_KEEP] {
        b = b.from_dsl_yaml(yaml).expect("fixture card compiles");
    }
    b.add_card(filler("G-LV3"))
        .deck(0, &["G-LV3"; 8])
        .deck(1, &["G-LV3"; 8])
        .security(0, &["G-LV3"; 3])
        .security(1, &["G-LV3"; 3])
        .hand(0, &["G-LV3", "G-LV3"])
        .hand(1, &["G-LV3", "G-LV3"])
        .memory(6)
}

fn main_bit(perm: usize) -> u16 {
    FIELD_EFFECT_START + perm as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}

fn offered(g: &Game, bit: u16) -> bool {
    build_action_mask(g, 0)[bit as usize] > 0.5
}

/// One KEEP-MAIN on player 0's field, in player 0's main phase.
fn keeper_board(copies: usize) -> DebugRunner {
    let mut r = builder().start();
    r.set_first_player(0);
    for _ in 0..copies {
        r.place_on_field(0, "KEEP-MAIN", Some(0));
    }
    r.game.enter_main_phase();
    assert_eq!(r.game.current_phase, GamePhase::Main);
    r
}

fn declare(g: &mut Game, perm: usize) {
    let bit = main_bit(perm);
    assert!(offered(g, bit), "the [Main] on permanent {perm} is offered");
    g.decode_action(bit, 0);
}

fn pass_pick(g: &mut Game) {
    assert!(g.pending_selection.is_some(), "the hand pick is pending");
    assert!(offered(g, PASS), "the pick can be passed");
    g.decode_action(PASS, 0);
    assert!(g.pending_selection.is_none(), "passing the pick ends the effect");
}

fn pick_first(g: &mut Game) {
    let sel = g.pending_selection.as_ref().expect("the hand pick is pending");
    let a = sel
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("a card to pick");
    g.decode_action(a, 0);
}

fn uses_spent(g: &Game, perm: usize) -> u32 {
    g.players[0].battle_area[perm]
        .effect_activations
        .values()
        .map(|&n| n as u32)
        .sum()
}

#[test]
fn declare_then_pass_keeps_the_use_and_withholds_the_identical_declare() {
    let mut r = keeper_board(1);
    let g = &mut r.game;
    let before_full = g.verification_digest();
    let before_free = g.guard_free_verification_digest();
    let hand_before = g.players[0].hand.len();

    declare(g, 0);
    let mid = g.verification_digest();
    assert_ne!(mid, before_full, "declaring changes the state (a prompt is open)");
    pass_pick(g);

    assert_eq!(uses_spent(g, 0), 0, "the passed pick gives the use back");
    assert!(
        g.players[0].battle_area[0].effect_activations.is_empty(),
        "a refunded use leaves no zero-count entry behind"
    );
    assert_eq!(g.players[0].hand.len(), hand_before);
    assert_eq!(
        g.guard_free_verification_digest(),
        before_free,
        "declare-then-pass is a gameplay no-op"
    );
    let after_full = g.verification_digest();
    assert_ne!(after_full, before_full, "the pass changes the full digest (the guard parks)");
    assert_ne!(after_full, mid);
    assert!(
        !offered(g, main_bit(0)),
        "the identical declaration is not offered again while nothing changed"
    );
    assert!(offered(g, PASS), "ending the turn is still legal");
    assert!(offered(g, PLAY_HAND_START), "playing a card is still legal");
}

#[test]
fn a_real_state_change_offers_the_declare_again_and_it_can_complete() {
    let mut r = keeper_board(1);
    let g = &mut r.game;
    declare(g, 0);
    pass_pick(g);
    assert!(!offered(g, main_bit(0)));

    // Play a card from hand: a turn action.
    g.decode_action(PLAY_HAND_START, 0);
    while g.pending_selection.is_some() {
        let a = g.pending_selection.as_ref().unwrap().valid_action_ids[0];
        g.decode_action(a, 0);
    }
    assert!(offered(g, main_bit(0)), "the kept use is offered after a real change");

    let hand_before = g.players[0].hand.len();
    let field_before = g.players[0].battle_area.len();
    declare(g, 0);
    pick_first(g);
    assert_eq!(g.players[0].hand.len(), hand_before - 1, "the pick was played");
    assert_eq!(g.players[0].battle_area.len(), field_before + 1);
    assert_eq!(uses_spent(g, 0), 1, "a completed pick spends the use");
    assert!(!offered(g, main_bit(0)), "the spent [Once Per Turn] is not offered");
}

#[test]
fn a_second_copy_is_independent() {
    let mut r = keeper_board(2);
    let g = &mut r.game;
    declare(g, 0);
    pass_pick(g);
    assert!(!offered(g, main_bit(0)));
    assert!(offered(g, main_bit(1)), "the other copy's declaration is a different action");

    declare(g, 1);
    pass_pick(g);
    assert!(!offered(g, main_bit(0)), "both declare-and-pass pairs are parked");
    assert!(!offered(g, main_bit(1)));
    assert_eq!(uses_spent(g, 0) + uses_spent(g, 1), 0);

    // A real change frees both.
    g.decode_action(PLAY_HAND_START, 0);
    assert!(offered(g, main_bit(0)));
    assert!(offered(g, main_bit(1)));
}

#[test]
fn declaring_the_other_copy_and_picking_clears_the_park() {
    let mut r = keeper_board(2);
    let g = &mut r.game;
    declare(g, 0);
    pass_pick(g);
    declare(g, 1);
    pick_first(g);
    assert_eq!(uses_spent(g, 1), 1, "copy 1 spent its use");
    assert!(offered(g, main_bit(0)), "the state changed, so copy 0 is offered again");
    assert!(!offered(g, main_bit(1)));
}

#[test]
fn zero_candidate_declaration_refunds_without_a_prompt_and_is_withheld() {
    let mut r = builder().start();
    r.set_first_player(0);
    r.place_on_field(0, "EMPTY-MAIN", Some(0));
    r.game.enter_main_phase();
    let g = &mut r.game;
    // The clause has no `condition:`, so the [Main] is offered with nothing
    // to pick (the Gekomon (BT26-021) shape).
    assert!(offered(g, main_bit(0)), "offered with no candidate");
    let before_free = g.guard_free_verification_digest();
    let before_full = g.verification_digest();
    g.decode_action(main_bit(0), 0);
    assert!(g.pending_selection.is_none(), "nothing to pick, no prompt");
    assert_eq!(uses_spent(g, 0), 0);
    assert_eq!(g.guard_free_verification_digest(), before_free);
    assert_ne!(g.verification_digest(), before_full, "every legal action changes the state");
    assert!(!offered(g, main_bit(0)), "the no-op declaration is not offered again");
}

/// A passed card-choice pick bumps the card-choice id allocator and changes
/// nothing else: it must still be proven a no-op, or the declaration could
/// be repeated forever (each pass a "new" state).
#[test]
fn a_passed_card_choice_pick_is_still_a_no_op() {
    let mut r = builder().start();
    r.set_first_player(0);
    r.place_on_field(0, "UNION-MAIN", Some(0));
    r.game.enter_main_phase();
    let g = &mut r.game;
    let before_free = g.guard_free_verification_digest();
    declare(g, 0);
    let id_before = g.pending_selection.as_ref().map(|s| s.card_choice.is_some());
    assert_eq!(id_before, Some(true), "the union pick is a card choice");
    pass_pick(g);
    assert_eq!(uses_spent(g, 0), 0, "the passed pick gives the use back");
    assert_eq!(
        g.guard_free_verification_digest(),
        before_free,
        "the card-choice id allocator is not gameplay state"
    );
    assert!(!offered(g, main_bit(0)), "the identical declaration is withheld");
}

/// Drive both seats with PASS (or the first legal action) until player 0's
/// next main phase.
fn advance_to_next_own_main(g: &mut Game) {
    let start_turn = g.turn_count;
    for _ in 0..400 {
        if g.turn_count > start_turn
            && g.turn_player() == 0
            && g.current_phase == GamePhase::Main
            && g.pending_selection.is_none()
        {
            return;
        }
        let who = g
            .pending_selection
            .as_ref()
            .map(|s| s.selecting_player)
            .unwrap_or_else(|| g.turn_player());
        let mask = build_action_mask(g, who);
        let a = if mask[PASS as usize] > 0.5 {
            PASS
        } else {
            mask.iter().position(|v| *v > 0.5).expect("some legal action") as u16
        };
        g.decode_action(a, who);
    }
    panic!("player 0's next main phase was not reached");
}

#[test]
fn the_park_does_not_cross_a_turn_boundary() {
    let mut r = keeper_board(1);
    let g = &mut r.game;
    declare(g, 0);
    pass_pick(g);
    assert!(!offered(g, main_bit(0)));
    advance_to_next_own_main(g);
    assert!(offered(g, main_bit(0)), "a new turn offers the [Main] again");
    declare(g, 0);
    pick_first(g);
    assert_eq!(uses_spent(g, 0), 1);
}

#[test]
fn the_card_leaving_and_re_entering_offers_it_again() {
    let mut r = keeper_board(1);
    declare(&mut r.game, 0);
    pass_pick(&mut r.game);
    assert!(!offered(&r.game, main_bit(0)));
    // Off the field, then a new KEEP-MAIN in its place, through direct calls
    // that bypass the action decoder.
    let gone = r.game.players[0].battle_area.remove(0);
    r.game.players[0].trash.extend(gone.card_sources);
    r.place_on_field(0, "KEEP-MAIN", Some(0));
    assert!(offered(&r.game, main_bit(0)), "the new card is a new object");
}

#[test]
fn a_stale_park_after_a_direct_state_change_does_not_apply() {
    let mut r = keeper_board(1);
    declare(&mut r.game, 0);
    pass_pick(&mut r.game);
    assert!(!offered(&r.game, main_bit(0)));
    // A direct Game call that never goes through `decode_action`.
    r.place_on_field(0, "G-LV3", Some(0));
    assert!(
        offered(&r.game, main_bit(0)),
        "the park applies only while the state is the one it was proven in"
    );
}

/// Clone at every stage of declare / pass / change / declare / pick; each
/// clone has the original's digest and, driven with the same actions, the
/// same masks and digests to the end.
#[test]
fn clones_at_every_stage_behave_like_the_original() {
    let mut r = keeper_board(2);
    let script: Vec<Box<dyn Fn(&mut Game)>> = vec![
        Box::new(|g| declare(g, 0)),
        Box::new(pass_pick),
        Box::new(|g| declare(g, 1)),
        Box::new(pass_pick),
        Box::new(|g| g.decode_action(PLAY_HAND_START, 0)),
        Box::new(|g| declare(g, 0)),
        Box::new(pick_first),
    ];
    for stage in 0..=script.len() {
        let mut original = r.game.clone();
        for step in &script[..stage] {
            step(&mut original);
        }
        let mut clone = original.clone();
        assert_eq!(clone.verification_digest(), original.verification_digest());
        for step in &script[stage..] {
            step(&mut original);
            step(&mut clone);
            assert_eq!(clone.verification_digest(), original.verification_digest());
            assert_eq!(build_action_mask(&clone, 0), build_action_mask(&original, 0));
        }
    }
    r.game.decode_action(PASS, 0);
}

/// A parked effect lower in the stack falls through to the next live `[Main]`
/// on the same permanent: the mask, the label and what the decoder fires are
/// the same effect.
#[test]
fn a_parked_inherited_main_falls_through_to_the_top_main_on_both_sides() {
    let mut r = builder().start();
    r.set_first_player(0);
    let host = r.place_on_field(0, "TOP-GAIN", Some(0));
    r.push_source(host, "INH-KEEP");
    r.game.enter_main_phase();
    let g = &mut r.game;
    let bit = main_bit(0);
    let before_label = explain_action(g, 0, bit).effect_name;
    declare(g, 0);
    assert!(g.pending_selection.is_some(), "the inherited [Main] matched first");
    pass_pick(g);
    assert!(offered(g, bit), "the top's own [Main] is live under the same bit");
    let after_label = explain_action(g, 0, bit).effect_name;
    assert_ne!(after_label, before_label, "the label names the top's [Main] now");
    let memory = g.memory;
    g.decode_action(bit, 0);
    assert!(g.pending_selection.is_none());
    assert_eq!(g.memory, memory + 1, "the decoder fired the top's [Main], as offered");
}

/// Regression for the mask/decoder asymmetry: a `[Main]` whose "by
/// suspending" cost is unpayable is not offered (15-7-1, 15-7-2), so the
/// bit names the next one; the decoder must fire that one, not the
/// unpayable one.
#[test]
fn an_unpayable_activation_cost_is_skipped_on_both_sides() {
    let mut r = builder().start();
    r.set_first_player(0);
    let host = r.place_on_field(0, "TOP-GAIN", Some(0));
    r.push_source(host, "INH-SUSP");
    r.game.enter_main_phase();
    r.game.players[0].battle_area[0].is_suspended = true;
    let g = &mut r.game;
    let bit = main_bit(0);
    assert!(offered(g, bit), "the top's [Main] is offered");
    let memory = g.memory;
    g.decode_action(bit, 0);
    assert_eq!(g.memory, memory + 1, "the offered [Main] (gain 1) fired");
}

/// Determinization: a park made in the true game holds in every world of the
/// same infoset, for either viewer; a world cut mid-prompt cannot prove the
/// pre-declare state, so it does not park on that pass, in every world alike,
/// and parks after its own declare and pass.
#[test]
fn determinized_worlds_agree_on_the_park() {
    use digimon_engine::determinize::materialize;
    let mut r = keeper_board(1);
    let mut mid = r.game.clone();
    declare(&mut mid, 0);

    declare(&mut r.game, 0);
    pass_pick(&mut r.game);
    let parked = r.game.clone();
    assert!(!offered(&parked, main_bit(0)));
    for viewer in [0u8, 1u8] {
        for seed in 0..8u64 {
            let world = materialize(&parked, viewer, seed);
            assert!(
                !offered(&world, main_bit(0)),
                "viewer {viewer} seed {seed}: the park holds in the world"
            );
        }
    }
    for seed in 0..8u64 {
        let mut world = materialize(&mid, 0, seed);
        pass_pick(&mut world);
        assert!(
            offered(&world, main_bit(0)),
            "seed {seed}: a world cut mid-prompt does not park on that pass"
        );
        declare(&mut world, 0);
        pass_pick(&mut world);
        assert!(!offered(&world, main_bit(0)), "seed {seed}: its own declare and pass parks");
    }
}
