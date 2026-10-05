//! BT8-054 Pistmon — Digimon, Lv.5, Green, DP 7000, Cost 7.
//! Traits: Vegetation. Form: Ultimate. Attribute: Virus. Digivolve: green Lv.4, cost 3.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT8-054.md)
//!
//! ＜Digisorption -2＞ (When one of your Digimon digivolves into this card from
//! your hand, you may suspend 1 of your Digimon to reduce the memory cost of the
//! digivolution by 2.)
//!
//! Inherited: [All Turns] This Digimon gets +1000 DP for each of your other
//! suspended Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT8/Green/BT8_054.cs — Digisorption -2 (same
//! shape as BT3-054) + inherited ChangeDP `+1000 * count(owner battle-area
//! Digimon != this permanent && IsSuspended)`.
//!
//! # Rules
//! 16-9 ＜Digisorption＞ — Opt-cost→Mand: suspending is optional; once paid the
//! reduction is mandatory; the cost floors at 0.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - D2 BeforePayCost digivolve cost reduction with an INTERACTIVE pay_cost
//!   hosted by the hand card being digivolved into (G-ENGINE-DIGISORPTION).
//! - E-family inherited self DP aura with a count formula ("other" suspended).

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledStep};
use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};

const CARD_ID: &str = "BT8-054";
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
/// P0's field, Pistmon in hand. Returns (runner, base index, other index).
fn runner_with(hand: &[&str]) -> (DebugRunner, usize, usize) {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT8-054 in embedded DSL pack")
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
fn bt8_054_compiles_digisorption_as_hosted_optional_interactive_reducer() {
    let (runner, _, _) = runner_with(&[CARD_ID]);
    let card = runner.compiled_card(CARD_ID).expect("compiled BT8-054");
    assert_eq!(card.name, "Pistmon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.traits, vec!["Vegetation"]);
    assert_eq!(card.alt_paths.len(), 1, "green Lv.4 digivolve path");

    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: digimon_dsl::compiled::CompiledScope::Inherited,
                ..
            })
        )),
        "inherited +1000-per-other-suspended DP aura"
    );

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
    assert_eq!(*amount, Some(2));
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
fn bt8_054_accept_suspend_other_digimon_reduces_digivolve_cost_by_two() {
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

    assert_eq!(top_id(&runner, base), CARD_ID, "BASE digivolved into Pistmon");
    assert!(runner.game.players[0].battle_area[other].is_suspended);
    assert!(!runner.game.players[0].battle_area[base].is_suspended);
    assert_eq!(
        mem_before - runner.memory(),
        1,
        "digivolve cost 3 - 2 = 1 memory paid"
    );
}

/// The digivolving Digimon itself is a legal suspend target (DCGO
/// `CanTapWhenAbsorbEvolution` admits any own unsuspended battle-area Digimon).
#[test]
fn bt8_054_accept_can_suspend_the_digivolving_digimon_itself() {
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
        "Pistmon stays suspended (it digivolved from a suspended Digimon)"
    );
    assert!(!runner.game.players[0].battle_area[other].is_suspended);
    assert_eq!(mem_before - runner.memory(), 1);
}

/// Decline: full cost (3), nothing suspended.
#[test]
fn bt8_054_decline_pays_full_digivolve_cost() {
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
fn bt8_054_not_offered_when_no_digimon_can_be_suspended() {
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
/// Pistmon already on the field does not reduce another card's digivolve.
#[test]
fn bt8_054_field_blossomon_does_not_reduce_other_digivolutions() {
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

/// Negative: playing Pistmon from hand (not digivolving) is not Digisorption.
#[test]
fn bt8_054_not_offered_when_played_from_hand() {
    let (mut runner, _base, _other) = runner_with(&[CARD_ID]);
    runner.game.enter_main_phase();
    let mem_before = runner.memory();
    runner
        .game
        .play_from_hand(0, 0)
        .expect("play Pistmon from hand");
    assert!(
        runner.game.pending_selection.is_none(),
        "no Digisorption prompt on a play"
    );
    assert_eq!(mem_before - runner.memory(), 7, "full play cost 7");
}

// ─── Section 3 — Inherited DP aura ───────────────────────────────────────────

/// Pistmon as a digivolution source: the host gets +1000 DP per OTHER own
/// suspended Digimon. The host's own suspension and opponent's suspended
/// Digimon do not count.
#[test]
fn bt8_054_inherited_plus_1000_per_other_own_suspended_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT8-054 in embedded DSL pack")
        .add_card(green_lv4("TOP"))
        .add_card(green_lv4("ALLY-A"))
        .add_card(green_lv4("ALLY-B"))
        .add_card(green_lv4("FOE"))
        .start();
    let host = runner.place_stack(0, &[CARD_ID, "TOP"]);
    let a = runner.place_on_field(0, "ALLY-A", None);
    let b = runner.place_on_field(0, "ALLY-B", None);
    let foe = runner.place_on_field(1, "FOE", None);
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(host), Some(4000), "no other suspended Digimon");

    runner.game.suspend(a);
    runner.game.suspend(foe);
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(host),
        Some(5000),
        "one other OWN suspended Digimon (+1000); the opponent's does not count"
    );

    runner.game.suspend(b);
    runner.game.suspend(host);
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(host),
        Some(6000),
        "two other own suspended (+2000); the host's own suspension is excluded"
    );
}

/// Negative: Pistmon as the TOP card does not apply its inherited aura to itself.
#[test]
fn bt8_054_inherited_aura_inactive_when_pistmon_is_top_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT8-054 in embedded DSL pack")
        .add_card(green_lv4("ALLY-A"))
        .start();
    let pist = runner.place_on_field(0, CARD_ID, None);
    let a = runner.place_on_field(0, "ALLY-A", None);
    runner.game.suspend(a);
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(pist), Some(7000));
}
