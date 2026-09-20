//! BT25-091 Monica Simmons — Tamer, Purple, Cost 4, [TS].
//!
//! # Card text (cards.json)
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [On Play] You may return 1 [TS] trait Option card from your trash to the
//!   hand. If this effect didn't return, <Draw 1>.
//! [Your Turn] When you use [TS] trait Option cards, by suspending this Tamer,
//!   1 of your opponent's Digimon can't attack until their turn ends.
//! Inherited [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_091.cs
//!
//! # Patterns this test covers
//! - B1 start-of-turn tamer (memory swing — set to 3)
//! - A4 trash → hand recursion (optional) with else-draw branch
//! - Tamer [Security] play-self
//! - [Your Turn] `on_use_option` observer gated on WHO used WHAT
//!   (`TriggerSource::OptionUsed`, G-ENGINE-ON-USE-OPTION-EVENT-CARD): optional
//!   suspend cost, mandatory opponent-Digimon pick, CannotAttack until the
//!   opponent's turn ends (DCGO BT25_091.cs:99-174, WhenUseOption.cs:11-16)

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const YAML: &str = include_str!("../../../cards/bt25/BT25-091.yaml");

// ── Section 1: structural ────────────────────────────────────────────────

#[test]
fn bt25_091_structure_start_of_turn_on_play_and_security() {
    let runner = monica_runner().start();
    let compiled = runner
        .compiled_card("BT25-091")
        .expect("BT25-091 compiled card present");

    assert_eq!(compiled.card, "BT25-091");
    assert_eq!(compiled.kind, CompiledCardKind::Tamer);
    assert_eq!(compiled.cost, Some(4));
    assert!(compiled.traits.iter().any(|t| t == "TS"));

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::StartOfYourTurn]),
        "start-of-your-turn clause present"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnPlay]),
        "on-play clause present"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnSecurity]),
        "security play-self clause present"
    );
}

// ── Section 2: [Start of Your Turn] set memory to 3 ──────────────────────

#[test]
fn bt25_091_start_of_turn_sets_low_memory_to_three() {
    let mut runner = monica_runner()
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(1)
        .start();
    runner.place_on_field(0, "BT25-091", Some(0));
    // start() may already have fired StartOfYourTurn; reset to the pre-fire
    // value and cycle P0 → P1 → P0 so begin_turn fires it with Monica in play.
    runner.game.memory = 1;
    runner.end_turn();
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), 3, "memory <=2 is set to 3");
}

#[test]
fn bt25_091_start_of_turn_does_not_change_high_memory() {
    let mut runner = monica_runner()
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(5)
        .start();
    runner.place_on_field(0, "BT25-091", Some(0));
    runner.game.memory = 5;
    runner.end_turn();
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.memory(),
        5,
        "memory >2 is unchanged (condition gates)"
    );
}

// ── Section 3: [On Play] return-or-draw ──────────────────────────────────

#[test]
fn bt25_091_on_play_returns_ts_option_from_trash() {
    let mut runner = monica_runner()
        .hand(0, &["BT25-091"])
        .add_card(make_ts_option("TS-OPT"))
        .memory(8)
        .start();
    runner.inject_trash(0, "TS-OPT");

    let hand_before = runner.hand_size(0);
    let trash_before = runner.trash_size(0);
    let field_index = runner.play(0, 0).expect("Monica plays from hand");
    runner.fire_on_play(0, field_index);

    // Optional select_trash over the TS Option should install.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Trash),
        "[On Play] offers an optional trash return"
    );
    let view = runner.pending_selection_view().unwrap();
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("return the TS Option");
    runner.auto_resolve();

    // The TS Option moved trash → hand. (Net hand: -Monica +TS-OPT = same count
    // as before play; assert the option is in hand and trash shrank.)
    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TS-OPT"),
        "returned TS Option is in hand"
    );
    assert_eq!(runner.trash_size(0), trash_before - 1, "trash shrank by 1");
    let _ = hand_before;
}

/// "If this effect didn't return, <Draw 1>": a successful trash → hand return
/// must NOT draw. DCGO BT25_091.cs:52-93 draws only on `!returned`. The deck is
/// non-empty here so a spurious draw is observable (the test above runs with an
/// empty deck, where the draw is a silent no-op). DCGO exam
/// BT25-091#effect#1 led on exactly this: our p0.hand carried an extra card.
#[test]
fn bt25_091_on_play_return_does_not_draw() {
    let mut runner = monica_runner()
        .hand(0, &["BT25-091"])
        .add_card(make_ts_option("TS-OPT"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .memory(8)
        .start();
    runner.inject_trash(0, "TS-OPT");

    let field_index = runner.play(0, 0).expect("Monica plays from hand");
    let deck_before = runner.deck_size(0);
    let hand_before = runner.hand_size(0);
    // play_from_hand fires [On Play] itself (no extra fire_on_play here —
    // that would queue a second instance of the clause).
    let _ = field_index;
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Trash));
    let view = runner.pending_selection_view().unwrap();
    let pick = *view
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("a real trash pick");
    runner
        .execute_action(view.selecting_player, pick)
        .expect("return the TS Option");
    runner.auto_resolve();

    assert_eq!(runner.hand_size(0), hand_before + 1, "only the returned Option joins the hand");
    assert_eq!(runner.deck_size(0), deck_before, "a successful return must not <Draw 1>");
}

/// Declining the optional return (a legal target exists) takes the draw
/// branch — official Q&A on BT25-091 ("Yes ... if you choose to not return").
#[test]
fn bt25_091_on_play_declined_return_draws() {
    let mut runner = monica_runner()
        .hand(0, &["BT25-091"])
        .add_card(make_ts_option("TS-OPT"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .memory(8)
        .start();
    runner.inject_trash(0, "TS-OPT");

    let field_index = runner.play(0, 0).expect("Monica plays from hand");
    let deck_before = runner.deck_size(0);
    let _ = field_index;
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Trash));
    let view = runner.pending_selection_view().unwrap();
    assert!(runner.pending_is_optional(), "the return is declinable");
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the return");
    runner.auto_resolve();

    assert_eq!(runner.trash_size(0), 1, "TS Option stays in trash");
    assert_eq!(runner.deck_size(0), deck_before - 1, "declining draws 1");
}

#[test]
fn bt25_091_on_play_draws_when_no_return() {
    // No TS Option in trash → cannot return → fallback Draw 1.
    let mut runner = monica_runner()
        .hand(0, &["BT25-091"])
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .memory(8)
        .start();

    let field_index = runner.play(0, 0).expect("Monica plays from hand");
    let deck_before = runner.deck_size(0);
    runner.fire_on_play(0, field_index);
    let _ = runner.auto_resolve();

    // The optional return had no legal target, so the else-branch Draw 1 fires.
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "Draw 1 fires when nothing was returned"
    );
}

// ── Section 4: [Your Turn] When you use [TS] trait Option cards ──────────

/// Inline observer used ONLY to isolate the "you" half of the gate: no
/// [Your Turn] window, so the owner check is the only thing between P0's
/// Option use and P1's observer.
const OWNER_GATE_OBSERVER: &str = "
card: OBS-YOU
name: \"Owner Gate Observer\"
kind: tamer
color: [purple]
cost: 2
effects:
  - when: on_use_option
    active_when: { all_turns: true }
    summary: \"When YOU use an Option card, gain 1 memory\"
    condition:
      event_target_owner: you
    process:
      - gain_memory: 1
";

fn monica_handle(runner: &DebugRunner) -> PermanentHandle {
    let idx = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "BT25-091")
        .expect("Monica on P0's field");
    runner.perm_handle(0, idx)
}

fn is_suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn locked_count(runner: &DebugRunner, handles: &[PermanentHandle]) -> usize {
    handles
        .iter()
        .filter(|h| runner.game.modifiers.has(**h, ModifierType::CannotAttack))
        .count()
}

/// 9-1-5 + 18-1-2 -> 15-4-3-5-1: with the used Option's pending trash AND
/// Monica's own trigger due at the same timing, P0 orders them. These lines
/// take DCGO's order (trash first); the alternative is pinned in
/// `bt3_096.rs` (G-ENGINE-OPTION-TRASH-TURN-PLAYER-ORDER).
fn order_trash_first(runner: &mut DebugRunner) {
    let Some(view) = runner.pending_selection_view() else {
        return;
    };
    if !view.prompt.contains("pending items") {
        return;
    }
    let entry = view
        .effect_choices
        .as_ref()
        .and_then(|c| c.iter().find(|e| e.label.contains("Trash")))
        .expect("a trash-now entry");
    let aid = entry.action_id;
    runner
        .execute_action(view.selecting_player, aid)
        .expect("order the used Option's trash first");
}

fn accept(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("Monica's optional suspend-cost prompt must park");
    assert_eq!(view.selecting_player, 0);
    assert!(
        view.is_optional,
        "'by suspending this Tamer' is a 15-7-1 optional cost"
    );
    let a = *view
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("accept action");
    runner.execute_action(0, a).expect("accept");
}

#[test]
fn bt25_091_structure_on_use_option_clause() {
    let runner = monica_runner().start();
    let compiled = runner.compiled_card("BT25-091").unwrap();
    let t = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::OnUseOption] => Some(t),
            _ => None,
        })
        .expect("on_use_option clause present");
    assert!(t.optional, "DCGO SetUpActivateClass(.., -1, true, ..)");
    assert!(!t.once_per_turn, "no [Once Per Turn] printed");
}

#[test]
fn bt25_091_own_ts_option_use_suspends_and_locks_one_opponent_digimon() {
    let mut runner = monica_runner()
        .add_card(make_ts_option("TS-OPT"))
        .add_card(make_filler("OPP-A"))
        .add_card(make_filler("OPP-B"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .hand(0, &["TS-OPT"])
        .memory(8)
        .start();
    runner.place_on_field(0, "BT25-091", Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    let b = runner.place_on_field(1, "OPP-B", Some(0));
    runner.game.enter_main_phase();
    let monica = monica_handle(&runner);

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    order_trash_first(&mut runner);
    accept(&mut runner);
    assert!(is_suspended(&runner, monica), "Monica suspends as the cost");

    let view = runner
        .pending_selection_view()
        .expect("opponent-Digimon pick parks");
    assert_eq!(view.selecting_player, 0);
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(
        !view.is_optional,
        "DCGO canNoSelect:false — the pick is mandatory"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both opponent Digimon are candidates"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick one");
    let _ = runner.auto_resolve();
    assert!(runner.pending_selection().is_none());
    assert_eq!(
        locked_count(&runner, &[a, b]),
        1,
        "exactly 1 opponent Digimon can't attack"
    );

    // "until their turn ends": still locked through the opponent's turn,
    // gone once it ends.
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.turn_player(), 1);
    assert_eq!(
        locked_count(&runner, &[a, b]),
        1,
        "lock persists during the opponent's turn"
    );
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(
        locked_count(&runner, &[a, b]),
        0,
        "lock expires at the end of the opponent's turn"
    );
}

#[test]
fn bt25_091_declining_leaves_monica_unsuspended() {
    let mut runner = monica_runner()
        .add_card(make_ts_option("TS-OPT"))
        .add_card(make_filler("OPP-A"))
        .hand(0, &["TS-OPT"])
        .memory(8)
        .start();
    runner.place_on_field(0, "BT25-091", Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    runner.game.enter_main_phase();
    let monica = monica_handle(&runner);

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    order_trash_first(&mut runner);
    runner.execute_action(0, PASS).expect("decline");
    let _ = runner.auto_resolve();
    assert!(!is_suspended(&runner, monica));
    assert_eq!(locked_count(&runner, &[a]), 0);
}

#[test]
fn bt25_091_non_ts_option_does_not_trigger() {
    let mut plain = make_ts_option("PLAIN-OPT");
    plain.traits.clear();
    let mut runner = monica_runner()
        .add_card(plain)
        .add_card(make_filler("OPP-A"))
        .hand(0, &["PLAIN-OPT"])
        .memory(8)
        .start();
    runner.place_on_field(0, "BT25-091", Some(0));
    runner.place_on_field(1, "OPP-A", Some(0));
    runner.game.enter_main_phase();

    let _ = runner.game.play_option_from_hand(0, 0);
    assert!(
        runner.pending_selection().is_none(),
        "only [TS] trait Option cards trigger Monica (DCGO OptionTrigger: HasTSTraits)"
    );
}

#[test]
fn bt25_091_suspended_monica_is_not_offered() {
    let mut runner = monica_runner()
        .add_card(make_ts_option("TS-OPT"))
        .add_card(make_filler("OPP-A"))
        .hand(0, &["TS-OPT"])
        .memory(8)
        .start();
    let idx = runner.place_on_field(0, "BT25-091", Some(0)).index as usize;
    runner.place_on_field(1, "OPP-A", Some(0));
    runner.game.players[0].battle_area[idx].is_suspended = true;
    runner.game.enter_main_phase();

    let _ = runner.game.play_option_from_hand(0, 0);
    assert!(
        runner.pending_selection().is_none(),
        "DCGO CanActivateSuspendCostEffect: a suspended Monica cannot pay"
    );
}

#[test]
fn bt25_091_offered_even_when_opponent_has_no_digimon() {
    // DCGO's CanActivateCondition checks only the suspend cost; the pick is
    // skipped by `HasMatchConditionPermanent` AFTER Monica suspends.
    let mut runner = monica_runner()
        .add_card(make_ts_option("TS-OPT"))
        .hand(0, &["TS-OPT"])
        .memory(8)
        .start();
    runner.place_on_field(0, "BT25-091", Some(0));
    runner.game.enter_main_phase();
    let monica = monica_handle(&runner);

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    order_trash_first(&mut runner);
    accept(&mut runner);
    assert!(is_suspended(&runner, monica));
    let _ = runner.auto_resolve();
    assert!(runner.pending_selection().is_none(), "no target → no pick");
}

/// The "you" half of the gate, isolated from [Your Turn]: P1's observer must
/// NOT see P0's Option use as its own (DCGO `cardSource.Owner == card.Owner`,
/// WhenUseOption.cs:13), while P0's observer does.
#[test]
fn on_use_option_event_target_owner_reads_the_user_not_the_observer() {
    let mut runner = monica_runner()
        .from_dsl_yaml(OWNER_GATE_OBSERVER)
        .expect("observer YAML loads")
        .add_card(make_ts_option("TS-OPT"))
        .hand(0, &["TS-OPT"])
        .memory(8)
        .start();
    runner.place_on_field(1, "OBS-YOU", Some(0));
    // P0's own copy: also the purple permanent meeting the colour requirement.
    runner.place_on_field(0, "OBS-YOU", Some(0));
    runner.game.enter_main_phase();

    let before = runner.memory();
    let _ = runner.game.play_option_from_hand(0, 0);
    let _ = runner.auto_resolve();
    // Cost 3 paid, P0's observer +1, P1's observer silent (it would be -1).
    assert_eq!(
        runner.memory(),
        before - 3 + 1,
        "only the USER's observer fires on `event_target_owner: you`"
    );
}

// ── fixtures ─────────────────────────────────────────────────────────────

fn monica_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT25-091 YAML loads")
}

fn make_ts_option(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.colors = vec![CardColor::Purple];
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card.traits = vec!["TS".to_string()];
    card
}

fn make_filler(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card
}
