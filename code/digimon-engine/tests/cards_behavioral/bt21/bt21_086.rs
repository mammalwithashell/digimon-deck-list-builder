//! BT21-086 Marcus Damon — Tamer, Yellow/Red, Cost 4.
//!
//! # Card text (cards.json — authoritative for printed text)
//!
//! ```text
//! [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
//! [On Play] 1 of your [Marcus Damon]s may suspend.
//! [All Turns] [Once Per Turn] When this Tamer suspends, 1 of your Digimon gains
//!   <Piercing> and +3000 DP for the turn. Then, 1 of your opponent's Digimon
//!   gets -3000 DP for the turn.
//! [Security] Play this card without paying the cost.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Red/BT21_086.cs
//!
//! # Self-scoped on_suspend
//! "When this Tamer suspends" gates the OnSuspend observer to the event
//! permanent being THIS exact Marcus, via `event_permanent_is_source: true`
//! (the BT23-077 Sistermon Ciel pattern). Another permanent suspending must
//! NOT fire it.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - G3 self-scoped OnSuspend observer (`event_permanent_is_source`)
//! - H3 Piercing grant (turn-scoped) + D1 +3000 DP buff
//! - Mandatory opponent Digimon -3000 DP debuff
//! - E2 OPT lockout (once per turn)

use digimon_dsl::compiled::{CompiledClause, CompiledPredicate, CompiledScope, CompiledTiming};
use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT21-086";

fn make_digimon(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.colors = vec![CardColor::Red];
    card
}

fn make_tamer(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Red];
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-086 must load from embedded DSL pack")
        .add_card(make_digimon("OWN-DIGI", "Agumon"))
        .add_card(make_digimon("OWN-DIGI2", "Gabumon"))
        .add_card(make_digimon("OPP-DIGI", "Numemon"))
        .add_card(make_digimon("OPP-DIGI2", "Goblimon"))
        .add_card(make_tamer("OTHER-TAMER", "Sora Takenouchi"))
        .memory(5)
}

fn encode_permanent(handle: PermanentHandle) -> u16 {
    encode_attack(0, handle.index as u16)
}

fn predicate_has_event_permanent_is_source(predicate: &CompiledPredicate) -> bool {
    predicate.event_permanent_is_source == Some(true)
        || predicate
            .all_of
            .iter()
            .any(predicate_has_event_permanent_is_source)
        || predicate
            .any_of
            .iter()
            .any(predicate_has_event_permanent_is_source)
        || predicate
            .none_of
            .iter()
            .any(predicate_has_event_permanent_is_source)
        || predicate
            .not
            .as_ref()
            .is_some_and(|p| predicate_has_event_permanent_is_source(p))
}

// ─── Structural ──────────────────────────────────────────────────────────────

#[test]
fn bt21_086_has_start_main_on_play_security_and_self_suspend_clauses() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");

    for timing in [
        CompiledTiming::StartOfYourMainPhase,
        CompiledTiming::OnPlay,
        CompiledTiming::OnSecurity,
    ] {
        assert!(
            card.effects.iter().any(|clause| matches!(
                clause,
                CompiledClause::Triggered(t) if t.when.contains(&timing)
            )),
            "BT21-086 must have {timing:?} clause"
        );
    }

    let on_suspend = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp
                    && t.when == vec![CompiledTiming::OnSuspend] =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("self-suspend buff/debuff clause should be authored");
    assert!(on_suspend.once_per_turn, "[Once Per Turn]");
    assert!(
        on_suspend
            .condition
            .as_ref()
            .is_some_and(predicate_has_event_permanent_is_source),
        "OnSuspend must be gated to the suspending event permanent being this Marcus"
    );
}

// ─── On Play (regression — existing behavior preserved) ──────────────────────

#[test]
fn bt21_086_on_play_can_suspend_a_marcus_damon() {
    let mut runner = base().hand(0, &[CARD_ID]).start();

    runner.play(0, 0).expect("play Marcus");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].battle_area[0].is_suspended,
        "BT21-086 On Play should suspend the selected Marcus Damon"
    );
}

// ─── Self-suspend: positive prompt + negative gating ─────────────────────────

#[test]
fn bt21_086_self_suspend_prompts_to_buff_own_digimon() {
    let mut runner = base().start();
    let marcus = runner.place_on_field(0, CARD_ID, Some(0));
    let own_a = runner.place_on_field(0, "OWN-DIGI", Some(0));
    let own_b = runner.place_on_field(0, "OWN-DIGI2", Some(0));
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    runner.game.suspend(marcus);

    let view = runner
        .pending_selection_view()
        .expect("self-suspend installs own Digimon buff selection");
    assert_eq!(view.kind, SelectionKind::OwnField);
    let mut expected = vec![encode_permanent(own_a), encode_permanent(own_b)];
    expected.sort();
    let mut got = view.valid_action_ids.clone();
    got.sort();
    assert_eq!(got, expected, "both own Digimon are legal Piercing/+3000 targets");
}

#[test]
fn bt21_086_other_permanent_suspending_does_not_fire_clause() {
    // NEGATIVE: a different permanent suspending must NOT trigger Marcus's
    // self-scoped OnSuspend observer.
    let mut runner = base().start();
    let _marcus = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "OWN-DIGI", Some(0));
    let other_tamer = runner.place_on_field(0, "OTHER-TAMER", Some(0));
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    runner.game.suspend(other_tamer);

    assert!(
        runner.pending_selection().is_none(),
        "another permanent suspending must not trigger BT21-086's self-suspend clause"
    );
}

#[test]
fn bt21_086_self_suspend_grants_piercing_plus_3000_then_debuffs_opponent() {
    let mut runner = base().start();
    let marcus = runner.place_on_field(0, CARD_ID, Some(0));
    let own = runner.place_on_field(0, "OWN-DIGI", Some(0));
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));

    let own_dp_before = runner.effective_dp(own).expect("own dp");
    let opp_dp_before = runner.effective_dp(opp).expect("opp dp");

    runner.game.suspend(marcus);

    // Step 1: buff own Digimon (Piercing + +3000).
    let buff_view = runner
        .pending_selection_view()
        .expect("own Digimon buff selection installs");
    assert_eq!(buff_view.kind, SelectionKind::OwnField);
    runner
        .execute_action(buff_view.selecting_player, encode_permanent(own))
        .expect("choose own Digimon");

    // Step 2: debuff opponent Digimon (-3000).
    let debuff_view = runner
        .pending_selection_view()
        .expect("opponent Digimon debuff selection installs");
    assert_eq!(debuff_view.kind, SelectionKind::OppField);
    runner
        .execute_action(debuff_view.selecting_player, encode_permanent(opp))
        .expect("choose opponent Digimon");
    runner.auto_resolve().expect("finish self-suspend effect");

    assert!(
        runner.game.has_keyword(own, Keyword::Piercing),
        "chosen own Digimon gains Piercing for the turn"
    );
    assert_eq!(
        runner.game.modifiers.sum(own, ModifierType::ChangeDp),
        3000,
        "chosen own Digimon gets +3000 DP"
    );
    assert_eq!(
        runner.effective_dp(own).expect("own dp"),
        own_dp_before + 3000
    );
    assert_eq!(
        runner.game.modifiers.sum(opp, ModifierType::ChangeDp),
        -3000,
        "chosen opponent Digimon gets -3000 DP"
    );
    assert_eq!(
        runner.effective_dp(opp).expect("opp dp"),
        opp_dp_before - 3000
    );
}

#[test]
fn bt21_086_self_suspend_is_once_per_turn() {
    // OPT lockout: a second suspend in the same turn does not re-install the
    // buff selection.
    let mut runner = base().start();
    let marcus = runner.place_on_field(0, CARD_ID, Some(0));
    let own = runner.place_on_field(0, "OWN-DIGI", Some(0));
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));

    runner.game.suspend(marcus);
    let buff_view = runner
        .pending_selection_view()
        .expect("first self-suspend installs buff selection");
    runner
        .execute_action(buff_view.selecting_player, encode_permanent(own))
        .expect("choose own Digimon");
    let debuff_view = runner
        .pending_selection_view()
        .expect("opponent debuff selection installs");
    runner
        .execute_action(debuff_view.selecting_player, encode_permanent(opp))
        .expect("choose opponent Digimon");
    runner.auto_resolve().expect("finish first activation");

    // Unsuspend then suspend again same turn → OPT must block.
    runner.game.unsuspend(marcus);
    runner.game.suspend(marcus);
    assert!(
        runner.pending_selection().is_none(),
        "[Once Per Turn] must block the second suspend in the same turn"
    );
}

#[test]
fn bt21_086_self_suspend_with_no_own_digimon_installs_no_prompt() {
    // NEGATIVE: this Marcus suspends but controls no Digimon to buff, so the
    // clause has no first target and installs no selection.
    let mut runner = base().start();
    let marcus = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    runner.game.suspend(marcus);

    assert!(
        runner.pending_selection().is_none(),
        "self-suspend with no own Digimon installs no buff prompt"
    );
}
