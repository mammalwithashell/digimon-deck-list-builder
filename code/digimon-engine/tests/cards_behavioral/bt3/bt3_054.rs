//! BT3-054 Blossomon — Digimon, Lv.5, Green, DP 7000, Cost 7.
//! Traits: Vegetation. Form: Ultimate. Attribute: Data. Digivolve: green Lv.4, cost 3.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT3-054.md)
//!
//! ＜Digisorption -3＞ (When one of your Digimon digivolves into this card from
//! your hand, you may suspend 1 of your Digimon to reduce the memory cost of the
//! digivolution by 3.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT3/Green/BT3_054.cs — BeforePayCost
//! ActivateClass (isOptional) gated on `Card == this && isEvolution`; selects 1
//! unsuspended own battle-area Digimon (`CanTapWhenAbsorbEvolution`, the
//! digivolving Digimon included), suspends it, then "Digivolution Cost -3".
//!
//! # Rules
//! 16-9 ＜Digisorption＞ — Opt-cost→Mand: suspending is optional; once paid the
//! reduction is mandatory; the cost floors at 0.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - D2 BeforePayCost digivolve cost reduction with an INTERACTIVE pay_cost
//!   hosted by the hand card being digivolved into (G-ENGINE-DIGISORPTION).

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledStep};
use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};

const CARD_ID: &str = "BT3-054";
// Evo-color byte for Green (matches `card_data::parse_card_color`).
const EVO_GREEN: u8 = 3;

fn green_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

/// A plain green Lv.5 card with the same printed digivolve cost (3) but no
/// Digisorption — the control target.
fn plain_green_lv5() -> CardData {
    let mut c = make_test_card("PLAIN-LV5", "PlainLv5");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(5);
    c.dp = Some(7000);
    c.play_cost = 7;
    c.evo_costs = vec![EvoCost {
        level: 4,
        card_color: EVO_GREEN,
        memory_cost: 3,
    }];
    c
}

/// BASE (green Lv.4, the digivolving Digimon) + OTHER (green Lv.4 bystander) on
/// P0's field, Blossomon in hand. Returns (runner, base index, other index).
fn runner_with(hand: &[&str]) -> (DebugRunner, usize, usize) {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT3-054 in embedded DSL pack")
        .add_card(green_lv4("BASE"))
        .add_card(green_lv4("OTHER"))
        .add_card(plain_green_lv5())
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .hand(0, hand)
        .memory(5)
        .start();
    runner.game.turn_count = 1;
    let base = runner.place_on_field(0, "BASE", Some(0));
    let other = runner.place_on_field(0, "OTHER", Some(0));
    (runner, base.index as usize, other.index as usize)
}

fn top_id(runner: &DebugRunner, idx: usize) -> String {
    runner.game.players[0].battle_area[idx]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn bt3_054_compiles_digisorption_as_hosted_optional_interactive_reducer() {
    let (runner, _, _) = runner_with(&[CARD_ID]);
    let card = runner.compiled_card(CARD_ID).expect("compiled BT3-054");
    assert_eq!(card.name, "Blossomon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.traits, vec!["Vegetation"]);
    assert_eq!(card.alt_paths.len(), 1, "green Lv.4 digivolve path");

    let reducers: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                when_digivolving_into_this,
                when_playing_this,
                optional,
                amount,
                pay_cost,
                ..
            }) => Some((
                *when_digivolving_into_this,
                *when_playing_this,
                *optional,
                *amount,
                pay_cost.clone(),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(reducers.len(), 1, "exactly one Digisorption reducer");
    let (into_this, playing_this, optional, amount, pay_cost) = &reducers[0];
    assert!(*into_this, "hosted by the card being digivolved into");
    assert!(!*playing_this, "not a play-cost reducer");
    assert!(*optional, "'you may suspend' — optional");
    assert_eq!(*amount, Some(3));
    assert!(matches!(
        pay_cost.as_slice(),
        [
            CompiledStep::SelectOwnPermanent { .. },
            CompiledStep::Suspend { .. }
        ]
    ));
}

// ─── Section 2 — Behavior ────────────────────────────────────────────────────

/// Accept, choose the OTHER Digimon: it is suspended and the digivolve cost
/// 3 is reduced to 0.
#[test]
fn bt3_054_accept_suspend_other_digimon_reduces_digivolve_cost_by_three() {
    let (mut runner, base, other) = runner_with(&[CARD_ID]);
    let mem_before = runner.memory();

    let done = runner
        .game
        .digivolve_from_hand(0, 0, base, PlaySource::ByHand);
    assert!(!done, "digivolve parks on the Digisorption accept gate");
    assert!(runner.pending_is_optional(), "Digisorption is optional");
    assert_eq!(runner.memory(), mem_before, "nothing paid yet");

    runner.accept_optional_trigger().expect("accept Digisorption");
    let pick = runner
        .pending_selection_view()
        .expect("choose which Digimon to suspend");
    assert!(!pick.is_optional, "the pick is mandatory once accepted");
    let base_action = encode_attack(0, base as u16);
    let other_action = encode_attack(0, other as u16);
    assert!(pick.valid_action_ids.contains(&base_action));
    assert!(pick.valid_action_ids.contains(&other_action));
    runner
        .execute_action(0, other_action)
        .expect("suspend OTHER as the cost");
    runner.auto_resolve().expect("settle");

    assert_eq!(top_id(&runner, base), CARD_ID, "BASE digivolved into Blossomon");
    assert!(runner.game.players[0].battle_area[other].is_suspended);
    assert!(!runner.game.players[0].battle_area[base].is_suspended);
    assert_eq!(
        runner.memory(),
        mem_before,
        "digivolve cost 3 - 3 = 0 memory paid"
    );
}

/// The digivolving Digimon itself is a legal suspend target (DCGO
/// `CanTapWhenAbsorbEvolution` admits any own unsuspended battle-area Digimon).
#[test]
fn bt3_054_accept_can_suspend_the_digivolving_digimon_itself() {
    let (mut runner, base, other) = runner_with(&[CARD_ID]);
    let mem_before = runner.memory();
    let _ = runner
        .game
        .digivolve_from_hand(0, 0, base, PlaySource::ByHand);
    runner.accept_optional_trigger().expect("accept");
    runner
        .execute_action(0, encode_attack(0, base as u16))
        .expect("suspend the digivolving Digimon");
    runner.auto_resolve().expect("settle");

    assert_eq!(top_id(&runner, base), CARD_ID);
    assert!(
        runner.game.players[0].battle_area[base].is_suspended,
        "Blossomon stays suspended (it digivolved from a suspended Digimon)"
    );
    assert!(!runner.game.players[0].battle_area[other].is_suspended);
    assert_eq!(runner.memory(), mem_before);
}

/// Decline: full cost (3), nothing suspended.
#[test]
fn bt3_054_decline_pays_full_digivolve_cost() {
    let (mut runner, base, other) = runner_with(&[CARD_ID]);
    let mem_before = runner.memory();
    let _ = runner
        .game
        .digivolve_from_hand(0, 0, base, PlaySource::ByHand);
    runner.decline_optional_trigger().expect("decline");
    runner.auto_resolve().expect("settle");

    assert_eq!(top_id(&runner, base), CARD_ID);
    assert!(!runner.game.players[0].battle_area[base].is_suspended);
    assert!(!runner.game.players[0].battle_area[other].is_suspended);
    assert_eq!(mem_before - runner.memory(), 3, "full digivolve cost 3");
}

/// Negative: with no unsuspended Digimon the cost is unpayable — no prompt,
/// full cost.
#[test]
fn bt3_054_not_offered_when_no_digimon_can_be_suspended() {
    let (mut runner, base, other) = runner_with(&[CARD_ID]);
    let b = runner.perm_handle(0, base);
    let o = runner.perm_handle(0, other);
    runner.game.suspend(b);
    runner.game.suspend(o);
    let mem_before = runner.memory();

    let done = runner
        .game
        .digivolve_from_hand(0, 0, base, PlaySource::ByHand);
    assert!(done, "no Digisorption prompt — digivolve completes");
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(top_id(&runner, base), CARD_ID);
    assert_eq!(mem_before - runner.memory(), 3);
}

/// Negative: Digisorption only reduces a digivolution INTO THIS card. A
/// Blossomon already on the field does not reduce another card's digivolve.
#[test]
fn bt3_054_field_blossomon_does_not_reduce_other_digivolutions() {
    let (mut runner, base, _other) = runner_with(&["PLAIN-LV5"]);
    runner.place_on_field(0, CARD_ID, Some(0));
    let mem_before = runner.memory();

    let done = runner
        .game
        .digivolve_from_hand(0, 0, base, PlaySource::ByHand);
    assert!(done, "no reducer offered for a different digivolve target");
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(top_id(&runner, base), "PLAIN-LV5");
    assert_eq!(mem_before - runner.memory(), 3);
}

/// Negative: playing Blossomon from hand (not digivolving) is not Digisorption.
#[test]
fn bt3_054_not_offered_when_played_from_hand() {
    let (mut runner, _base, _other) = runner_with(&[CARD_ID]);
    runner.game.enter_main_phase();
    let mem_before = runner.memory();
    runner
        .game
        .play_from_hand(0, 0)
        .expect("play Blossomon from hand");
    assert!(
        runner.game.pending_selection.is_none(),
        "no Digisorption prompt on a play"
    );
    assert_eq!(mem_before - runner.memory(), 7, "full play cost 7");
}
