//! Engine guards from the 2026-11 matrix bug scan, on synthetic inline-YAML
//! cards so each guards the SUBSTRATE and not one printed card.
//!
//! - {Trash} triggers and CR 4.3 15-4-4-3 / 15-14-3-1 (source left the trash).
//! - A mandatory "by trashing ..." union-zone pick with no candidate and
//!   CR 4.3 15-7-2 (the text after "by" cannot run).
//! - `select_union_zone` over `material` and a non-`Digimon`-kind top card
//!   that is still a Digimon on the field (DUAL card, CR 4.3 4-6-3).
//! - An Into-direction alt-digivolve path ("this Digimon can digivolve into
//!   [X] in the hand ...") and CR 4.3 3-4-7-4 (off in the breeding area).
//! - The per-effect result log across a prompt nested in an `if` body, read by
//!   a trailing result predicate (CR 4.3 15-14-1-2 through `refund_opt`).

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_digivolve, BREEDING_TARGET, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const TRASH_OBS: &str = r#"
card: GUARD-TRASH-OBS
name: "Trash Observer"
kind: digimon
level: 3
color: [red]
cost: 3
dp: 1000
effects:
  - when: on_ally_played
    scope: trash
    active_when: { your_turn: true }
    trigger_if:
      all_of:
        - event_target_owner: you
        - event_target_kind: digimon
    summary: "[Trash] [Your Turn] When any of your Digimon are played, gain 1 memory."
    process:
      - gain_memory: 1
"#;

/// "[Main] By suspending this Tamer and trashing 1 Option card from your hand
/// or your Digimon's digivolution cards, gain 1 memory." WITHOUT the
/// activation gate a real card carries, so the cost pick can be reached with
/// no candidate.
const UNGATED_UNION_COST: &str = r#"
card: GUARD-UNION-COST
name: "Ungated Union Cost"
kind: tamer
color: [purple]
cost: 3
effects:
  - when: main_on_field
    summary: "[Main] By suspending this Tamer and trashing 1 Option card from your hand or your Digimon's digivolution cards, gain 1 memory."
    process:
      - suspend: { target: source }
      - select_union_zone:
          of: you
          zones: [hand, material]
          filter: { kind: option }
          bind_as: cost_option
          prompt: "Trash 1 Option card from your hand or your Digimon's digivolution cards"
      - trash_union_bound:
          binding: cost_option
      - gain_memory: 1
"#;

/// The same cost as the clause's FIRST step: the activation itself is gated
/// on the pick having a candidate (CR 4.3 15-7-4).
const LEADING_UNION_COST: &str = r#"
card: GUARD-UNION-LEAD
name: "Leading Union Cost"
kind: tamer
color: [purple]
cost: 3
effects:
  - when: main_on_field
    summary: "[Main] By trashing 1 Option card from your Digimon's digivolution cards, gain 1 memory."
    process:
      - select_union_zone:
          of: you
          zones: [material]
          filter: { kind: option }
          bind_as: cost_option
          prompt: "Trash 1 Option card from your Digimon's digivolution cards"
      - trash_union_bound:
          binding: cost_option
      - gain_memory: 1
"#;

fn card(id: &str, kind: CardKind, level: Option<u8>) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = kind;
    c.colors = vec![CardColor::Purple];
    c.level = level;
    c.dp = level.map(|l| 1000 * l as i32);
    c.play_cost = 3;
    c
}

fn builder(yaml: &str) -> DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline guard card compiles")
        .add_card(card("G-FILL", CardKind::Digimon, Some(3)))
        .add_card(card("G-LV4", CardKind::Digimon, Some(4)))
        .add_card(card("G-DUAL", CardKind::Dual, Some(6)))
        .add_card(card("G-OPTION", CardKind::Option, None))
        .deck(0, &["G-FILL"; 6])
        .deck(1, &["G-FILL"; 6])
        .security(0, &["G-FILL"; 3])
        .security(1, &["G-FILL"; 3])
        .memory(3)
}

fn push_trash(r: &mut DebugRunner, id: &str) -> digimon_engine::card_source::CardHandle {
    let idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == id)
        .unwrap_or_else(|| panic!("unknown card {id}"));
    let next = r.game.next_card_index();
    let source = CardSource::new(idx, 0, next);
    let handle = source.handle();
    r.game.players[0].put_in_trash(source);
    handle
}

fn settle_first_choice(r: &mut DebugRunner) {
    for _ in 0..16 {
        let Some(sel) = r.pending_selection() else {
            return;
        };
        let (player, action) = (
            sel.selecting_player,
            sel.valid_action_ids
                .iter()
                .copied()
                .find(|&a| a != PASS)
                .unwrap_or(PASS),
        );
        r.execute_action(player, action).unwrap();
    }
    panic!("prompts did not settle");
}

fn trash_obs_board() -> (DebugRunner, Vec<digimon_engine::card_source::CardHandle>) {
    let mut r = builder(TRASH_OBS).start();
    r.set_first_player(0);
    let copies = vec![
        push_trash(&mut r, "GUARD-TRASH-OBS"),
        push_trash(&mut r, "GUARD-TRASH-OBS"),
    ];
    let played = r.place_on_field(0, "G-LV4", None);
    r.fire_play_event_triggers(0, played.index as usize, false, false);
    (r, copies)
}

/// Control: both copies are still in the trash, both activate.
#[test]
fn trash_zone_triggers_activate_while_their_cards_stay_in_the_trash() {
    let (mut r, _) = trash_obs_board();
    let before = r.memory();
    settle_first_choice(&mut r);
    assert_eq!(r.memory() - before, 2, "one activation per copy in the trash");
}

/// CR 4.3 15-4-4-3: a card that changes areas is a new card (3-1-3-1), so its
/// pending {Trash} effect can no longer be activated. 15-14-3-1: a {Trash}
/// effect activates "while the card with the effect is in your trash".
#[test]
fn pending_trash_zone_trigger_is_lost_when_its_card_leaves_the_trash() {
    let (mut r, copies) = trash_obs_board();
    assert!(r.pending_selection().is_some(), "two simultaneous triggers pend");
    let at = r.game.players[0]
        .trash
        .iter()
        .position(|c| c.handle() == copies[0])
        .unwrap();
    let moved = r.game.players[0].trash.remove(at);
    r.game.players[0].deck.insert(0, moved);
    let before = r.memory();
    settle_first_choice(&mut r);
    assert_eq!(
        r.memory() - before,
        1,
        "only the copy still in the trash activates"
    );
}

/// The card left the trash and came back before its pending effect activated:
/// it is a new card in the same area, and the pending effect stays lost.
#[test]
fn pending_trash_zone_trigger_is_lost_when_its_card_leaves_and_re_enters_the_trash() {
    let (mut r, copies) = trash_obs_board();
    assert!(r.pending_selection().is_some(), "two simultaneous triggers pend");
    let at = r.game.players[0]
        .trash
        .iter()
        .position(|c| c.handle() == copies[0])
        .unwrap();
    let moved = r.game.players[0].trash.remove(at);
    r.game.players[0].put_in_trash(moved);
    let before = r.memory();
    settle_first_choice(&mut r);
    assert_eq!(
        r.memory() - before,
        1,
        "the re-entered card is a new card; its old pending effect is gone"
    );
}

/// CR 4.3 15-7-2: "If the content of the optional processing conditions isn't
/// executed, the processing after the conditions can't be executed." A
/// mandatory "by trashing 1 ..." pick that finds no candidate did not execute.
#[test]
fn mandatory_union_cost_pick_without_candidates_does_not_run_the_tail() {
    let mut r = builder(UNGATED_UNION_COST).start();
    r.set_first_player(0);
    let tamer = r.place_on_field(0, "GUARD-UNION-COST", Some(0));
    r.place_on_field(0, "G-LV4", Some(0));
    r.game.enter_main_phase();
    let before = r.memory();
    let _ = r.game.activate_field_main(0, tamer.index as usize);
    assert!(r.pending_selection().is_none(), "nothing to pick");
    assert_eq!(r.memory(), before, "the cost was not paid, so no memory is gained");
}

/// Control for the test above: with an Option under a Digimon the pick is
/// offered and paying it runs the tail.
#[test]
fn mandatory_union_cost_pick_with_a_candidate_runs_the_tail() {
    let mut r = builder(UNGATED_UNION_COST).start();
    r.set_first_player(0);
    let tamer = r.place_on_field(0, "GUARD-UNION-COST", Some(0));
    let host = r.place_on_field(0, "G-LV4", Some(0));
    r.push_source(host, "G-OPTION");
    r.game.enter_main_phase();
    let before = r.memory();
    assert!(r.game.activate_field_main(0, tamer.index as usize));
    assert!(r.pending_selection().is_some(), "the Option is a real pick");
    settle_first_choice(&mut r);
    assert_eq!(r.memory() - before, 1);
    assert_eq!(r.game.players[0].trash.len(), 1, "the Option was trashed");
}

/// CR 4.3 4-6-3: "A DUAL card placed on the field is treated as a Digimon",
/// so the cards under it are a Digimon's digivolution cards. The leading-cost
/// activation gate (15-7-4) and the pick both scan them.
#[test]
fn union_material_scan_reads_the_stack_under_a_dual_top_card() {
    let mut r = builder(LEADING_UNION_COST).start();
    r.set_first_player(0);
    let tamer = r.place_on_field(0, "GUARD-UNION-LEAD", Some(0));
    let host = r.place_on_field(0, "G-DUAL", Some(0));
    r.push_source(host, "G-OPTION");
    r.game.enter_main_phase();
    let before = r.memory();
    assert!(
        r.game.activate_field_main(0, tamer.index as usize),
        "an Option under a DUAL top card can pay the cost"
    );
    assert!(r.pending_selection().is_some(), "the Option is a real pick");
    settle_first_choice(&mut r);
    assert_eq!(r.memory() - before, 1);
    assert_eq!(r.game.players[0].trash.len(), 1, "the Option was trashed");
}

// ─── Into-direction alt-digivolve paths and the breeding area ────────────────

/// "[Your Turn] This Digimon can digivolve into [Warp Target] in the hand for
/// a digivolution cost of 2, ignoring digivolution requirements."
const INTO_WARP: &str = r#"
card: GUARD-INTO-WARP
name: "Into Warp"
kind: digimon
level: 3
color: [red]
cost: 3
dp: 1000
alt_paths:
  - kind: digivolve
    direction: into
    from:
      zone: [hand]
      of: you
      name_is: "Warp Target"
    cost: 2
    ignore_requirements: true
    condition: { your_turn: true }
"#;

fn warp_board() -> DebugRunner {
    let mut target = make_test_card("G-WARP-TARGET", "Warp Target");
    target.level = Some(6);
    target.dp = Some(11000);
    target.evo_costs = Vec::new();
    let mut r = builder(INTO_WARP)
        .add_card(target)
        .hand(0, &["G-WARP-TARGET"])
        .start();
    r.set_first_player(0);
    r.game.current_phase = GamePhase::Main;
    r
}

/// Control: in the battle area the clause is live.
#[test]
fn into_direction_warp_is_offered_in_the_battle_area() {
    let mut r = warp_board();
    let base = r.place_on_field(0, "GUARD-INTO-WARP", Some(0));
    let action = encode_digivolve(0, base.index as u16);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    let before = r.memory();
    assert!(r.game.digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand));
    assert_eq!(before - r.memory(), 2);
}

/// CR 4.3 3-4-7-4: "Effects on cards in breeding areas can't trigger or
/// activate, except for effects that explicitly specify or reference breeding
/// areas." Official Q&A Q6388 (BlackGatomon (BT25-082)): "The effect doesn't
/// activate in the breeding area."
#[test]
fn into_direction_warp_is_off_in_the_breeding_area() {
    let mut r = warp_board();
    r.place_in_breeding(0, "GUARD-INTO-WARP");
    let action = encode_digivolve(0, BREEDING_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 0.0);
    let before = r.memory();
    assert!(!r.game.digivolve_from_hand_onto_breeding(0, 0, PlaySource::ByHand));
    assert_eq!(r.memory(), before);
    let breeding = r.game.players[0].breeding_area.as_ref().expect("still there");
    assert_eq!(breeding.card_sources.len(), 1);
}

// ─── Result log across a prompt nested in an `if` body ───────────────────────

/// "[On Play] [When Attacking] [Once Per Turn] You may return 1 card in your
/// opponent's trash to the bottom of the deck." with the DCGO-style
/// `RemoveUse` when nothing was returned (the Chronomon: Holy Mode (BT26-016)
/// shape: the prompt sits inside an `if`, the guard after it).
const NESTED_RESULT_LOG: &str = r#"
card: GUARD-NESTED-LOG
name: "Nested Log"
kind: digimon
level: 5
color: [red]
cost: 5
dp: 5000
effects:
  - when: [on_play, when_attacking]
    once_per_turn: true
    summary: "[On Play] [When Attacking] [Once Per Turn] You may return 1 card in your opponent's trash to the bottom of the deck."
    process:
      - if:
          condition:
            count_gte: { filter: { of: opponent, zone: [trash] }, n: 1 }
          then:
            - select_count_capped_multi:
                of: opponent
                zone: trash
                min: 1
                max: 1
                optional_zero: true
                filter: {}
                bind_as: back
                prompt: "Return 1 card in your opponent's trash to the bottom of the deck"
            - if:
                condition: { binding_count_eq: { binding: back, n: 1 } }
                then:
                  - return_trash_list_to_deck_bottom: { of: opponent, cards: back }
      - if:
          condition: { effect_returned_any_card: false }
          then:
            - refund_opt: {}
"#;

fn fire(r: &mut DebugRunner, timing: EffectTiming, h: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

fn nested_log_board() -> (DebugRunner, PermanentHandle) {
    let mut r = builder(NESTED_RESULT_LOG).start();
    r.set_first_player(0);
    for _ in 0..2 {
        let idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "G-FILL")
            .unwrap();
        let next = r.game.next_card_index();
        r.game.players[1].put_in_trash(CardSource::new(idx, 1, next));
    }
    let h = r.place_on_field(0, "GUARD-NESTED-LOG", Some(0));
    (r, h)
}

/// CR 4.3 15-14-1-2: an [X Per Turn] effect used X times "won't trigger again
/// during that turn". The return happened inside the nested prompt's tail; the
/// trailing guard must see it.
#[test]
fn result_log_from_a_nested_prompt_reaches_the_trailing_guard() {
    let (mut r, h) = nested_log_board();
    fire(&mut r, EffectTiming::OnPlay, h);
    assert!(r.pending_selection().is_some(), "the return prompt");
    settle_first_choice(&mut r);
    assert_eq!(r.game.players[1].trash.len(), 1, "one card was returned");

    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert!(
        r.pending_selection().is_none(),
        "a card was returned: the [Once Per Turn] is spent"
    );
    assert_eq!(r.game.players[1].trash.len(), 1);
}

/// Control: declining returns nothing, the guard refunds the use, and the
/// next trigger the same turn offers the effect again.
#[test]
fn nested_prompt_declined_keeps_the_once_per_turn() {
    let (mut r, h) = nested_log_board();
    fire(&mut r, EffectTiming::OnPlay, h);
    let sel = r.pending_selection().expect("the return prompt");
    let player = sel.selecting_player;
    r.execute_action(player, PASS).unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[1].trash.len(), 2, "nothing was returned");

    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert!(
        r.pending_selection().is_some(),
        "nothing was returned: the use was refunded"
    );
}

/// Unit level: overlaying a resolved selection's bindings onto the snapshot
/// its outer tail parked with carries the newer result-log entries over,
/// once, and does not splice in an unrelated history.
#[test]
fn merge_slots_from_adopts_newer_result_log_entries_of_its_own_lineage() {
    use digimon_engine::card_source::CardHandle;
    let h0 = PermanentHandle { player: 0, index: 0 };
    let h1 = PermanentHandle { player: 1, index: 2 };

    let mut parked = Bindings::new();
    parked.record_suspended(h0);
    let mut resolved = parked.clone();
    resolved.record_returned_to_deck(CardHandle(7));
    resolved.record_deleted(h1, Some(3000), true);

    parked.merge_slots_from(&resolved);
    parked.merge_slots_from(&resolved);
    let log = parked.result_log();
    assert_eq!(log.suspended, vec![h0], "the shared prefix is not duplicated");
    assert_eq!(log.returned_to_deck, vec![CardHandle(7)]);
    assert_eq!(log.returned_cards, vec![CardHandle(7)]);
    assert_eq!(log.deleted, vec![(h1, Some(3000))]);
    assert_eq!(log.deleted_is_digimon, vec![true]);

    let mut unrelated = Bindings::new();
    unrelated.record_returned_to_deck(CardHandle(9));
    unrelated.record_returned_to_deck(CardHandle(10));
    parked.merge_slots_from(&unrelated);
    assert_eq!(
        parked.result_log().returned_to_deck,
        vec![CardHandle(7)],
        "a log that does not extend this one is another history"
    );
}
