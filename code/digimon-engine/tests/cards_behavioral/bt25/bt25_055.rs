//! BT25-055 Deramon — Digimon, Lv.5, Green DP 6000, Cost 6.
//! Traits: Avian, Iliad, TS. Standard evo: Lv.4 green / cost 3.
//!
//! # Card text (cards.json — verbatim)
//!
//! [On Play] [When Digivolving] You may suspend 1 Digimon. Then, if there are
//! 2 or more suspended Digimon, 1 of your Digimon may unsuspend.
//! [All Turns] [Once Per Turn] When this Digimon suspends, you may play 1
//! 4000 DP or lower Digimon card with [Vegetation], [Plant], [Avian] or [Bird]
//! in any of its traits or the [TS] trait from your hand without paying the cost.
//!
//! Inherited: [Opponent's Turn] [Once Per Turn] When one of your opponent's
//! Digimon attacks, you may change the attack target to 1 of your suspended
//! Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_055.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - C (OnPlay/WhenDigivolving optional suspend + conditional unsuspend)
//! - E (OnSuspend triggered free-play from hand, OPT)
//! - G (inherited opponent-attack redirect)
//! - H (alt digivolution path)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-055";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn own_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(3000);
    card.traits = vec!["Beast".to_string()];
    card
}

fn opp_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(7000);
    card.traits = vec!["Beast".to_string()];
    card
}

/// A 4000-DP-or-lower [Avian] Digimon — a valid OPT free-play target.
fn avian_target(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.traits = vec!["Avian".to_string()];
    card
}

/// Over-DP, non-matching trait — must NOT be a legal OPT free-play target.
fn big_nonmatch(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(9000);
    card.play_cost = 7;
    card.traits = vec!["Machine".to_string()];
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-055 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(own_digimon("OWN-A"))
        .add_card(own_digimon("OWN-B"))
        .add_card(opp_digimon("OPP-A"))
        .add_card(avian_target("AVIAN"))
        .add_card(big_nonmatch("BIG"))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_055_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "Deramon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(6000));
    assert_eq!(card.cost, Some(6));
    // Card image + official bundle (data/card_bundles/BT25-055.md):
    // "Form: Ultimate  Attribute: Data".
    assert_eq!(card.attribute, ["Data"]);
}

#[test]
fn bt25_055_has_alt_digivolve_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert!(
        !card.alt_paths.is_empty(),
        "BT25-055 must have a TS-trait alt-digivolve path"
    );
}

#[test]
fn bt25_055_op_wd_clause_has_suspend_and_unsuspend() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("must have OnPlay/WhenDigivolving suspend clause");
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::Suspend { .. })),
        "OP/WD clause must contain a suspend step"
    );
}

#[test]
fn bt25_055_opt_suspend_clause_is_once_per_turn_and_optional() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSuspend) => Some(t),
            _ => None,
        })
        .expect("must have OnSuspend free-play clause");
    assert!(clause.once_per_turn, "OnSuspend clause is [Once Per Turn]");
    assert!(clause.optional, "OnSuspend clause is 'you may'");
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlayFromHandFree { .. })),
        "OnSuspend clause must free-play the selected card"
    );
}

#[test]
fn bt25_055_inherited_redirect_clause_present() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnOpponentAttack) => {
                Some(t)
            }
            _ => None,
        })
        .expect("must have inherited OnOpponentAttack redirect clause");
    assert_eq!(
        clause.scope,
        CompiledScope::Inherited,
        "redirect is inherited"
    );
    assert!(clause.once_per_turn, "redirect clause is OPT");
    assert!(clause.optional, "redirect clause is 'you may'");
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::RedirectAttackTarget { .. })),
        "redirect clause must contain a redirect_attack_target step"
    );
}

// ─── Section 2 — Behavioral ──────────────────────────────────────────────────

/// On Play: accepting the suspend pick suspends a chosen Digimon.
#[test]
fn bt25_055_on_play_suspends_chosen_digimon() {
    let mut runner = base().start();
    let victim = runner.place_on_field(0, "OWN-A", Some(1));
    let deramon = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(
        !runner.game.players[0].battle_area[victim.index as usize].is_suspended,
        "precondition: victim unsuspended"
    );

    runner.fire_on_play(0, deramon.index as usize);
    // The optional suspend pick installs a selection; suspend the victim.
    assert!(
        runner.pending_selection().is_some(),
        "On Play must install the suspend-target selection"
    );
    runner
        .auto_resolve()
        .expect("resolve suspend (and any unsuspend guard)");

    assert!(
        runner.game.players[0].battle_area[victim.index as usize].is_suspended
            || runner.game.players[0].battle_area[deramon.index as usize].is_suspended,
        "On Play must have suspended a Digimon"
    );
}

/// OnSuspend OPT: when Deramon itself suspends, accepting the trigger lets you
/// free-play a qualifying hand Digimon.
#[test]
fn bt25_055_self_suspend_allows_freeplay() {
    let mut runner = base().hand(0, &["AVIAN"]).start();
    let deramon = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    // Suspend Deramon itself → fires the OnSuspend OPT clause.
    runner.game.suspend(deramon);

    assert!(
        runner.pending_selection().is_some(),
        "self-suspend must install the optional free-play prompt"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the optional free-play");
    runner.auto_resolve().expect("resolve the hand free-play");

    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "the AVIAN card left the hand (played)"
    );
    assert_eq!(
        runner.battle_area_size(0),
        field_before + 1,
        "the AVIAN card was played to the battle area for free"
    );
}

/// OnSuspend OPT — negative: declining the optional trigger plays nothing.
#[test]
fn bt25_055_self_suspend_decline_plays_nothing() {
    let mut runner = base().hand(0, &["AVIAN"]).start();
    let deramon = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    runner.game.suspend(deramon);
    assert!(runner.pending_selection().is_some(), "prompt installed");
    runner
        .decline_optional_trigger()
        .expect("declining is legal");

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "no card played on decline"
    );
    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "no new permanent on decline"
    );
}

/// OnSuspend OPT — negative: another Digimon (not Deramon) suspending must NOT
/// fire Deramon's self-suspend clause (event_target_is_source gate).
#[test]
fn bt25_055_other_suspend_does_not_fire() {
    let mut runner = base().hand(0, &["AVIAN"]).start();
    let deramon = runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_on_field(0, "OWN-A", Some(1));

    // Suspend the OTHER Digimon, not Deramon.
    runner.game.suspend(other);

    assert!(
        runner.pending_selection().is_none(),
        "Deramon's self-suspend clause must NOT fire when a different Digimon suspends"
    );
}

// ─── Section 3 — the free-play pick offers only playable cards ───────────────
//
// DCGO BT25_055.cs:134-144 `CanSelectCardCondition` is `IsDigimon && HasDP &&
// CardDP <= 4000 && traits` -- it does NOT call `CanPlayAsNewPermanent`, so
// DCGO offers a card the play then refuses (no empty frame, or a "can't play
// Digimon by effects" lock). The rules decide that case: CR 4.3 1-3-2 (an
// impossible action is not carried out) and 7-1-1 (playing places a card on
// the field) -- a card that can't be played is not a "play" choice, and
// choosing it would only be a dead pick in the RL action space. We filter by
// playability (`can_play_from_effect_without_cost`), and gate the trigger on a
// playable candidate (DCGO `CanActivateCondition`, :128-132, with the same
// filter), which is DCGO's own "select nothing -> RemoveUse" outcome.

/// An [Avian] Digi-Egg (no DP, no play cost).
fn avian_egg(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::DigiEgg;
    card.level = Some(2);
    card.dp = None;
    card.play_cost = 0;
    card.traits = vec!["Avian".to_string()];
    card
}

/// Suspend Deramon (BT25-055) and return the hand-pick candidates, accepting
/// an outer "you may" prompt if one opens. `None` = no prompt at all.
fn offered_after_self_suspend(hand: &[&str], setup: impl FnOnce(&mut DebugRunner)) -> Option<Vec<String>> {
    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    use digimon_engine::selection::SelectionKind;
    let mut runner = base().add_card(avian_egg("AVIAN-EGG")).hand(0, hand).start();
    let deramon = runner.place_on_field(0, CARD_ID, Some(0));
    setup(&mut runner);
    runner.game.suspend(deramon);
    runner.pending_selection()?;
    if runner.pending_kind() != Some(SelectionKind::Hand) {
        runner.accept_optional_trigger().expect("accept the optional free-play");
    }
    let Some(view) = runner.pending_selection_view() else {
        return Some(Vec::new());
    };
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    Some(
        view.valid_action_ids
            .iter()
            .copied()
            .filter(|&a| a != PASS)
            .map(|a| hand_ids[(a - PLAY_HAND_START) as usize].clone())
            .collect(),
    )
}

#[test]
fn bt25_055_self_suspend_not_offered_when_battle_area_full() {
    assert_eq!(
        offered_after_self_suspend(&["AVIAN"], |r| {
            r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
        }),
        None,
        "no empty frame: the [Avian] Digimon can't be played, so the effect is not offered"
    );
}

#[test]
fn bt25_055_self_suspend_not_offered_under_effect_play_prohibition() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    assert_eq!(
        offered_after_self_suspend(&["AVIAN"], |r| {
            r.game.modifiers.add_player_modifier(
                0,
                PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
            );
        }),
        None,
        "\"you can't play Digimon by effects\": nothing to offer"
    );
}

/// "with [Bird] in any of its traits" is a substring match (DCGO
/// BT25_055.cs:142 `ContainsTraits("Bird")`): a [Sky Bird] Digimon qualifies.
#[test]
fn bt25_055_self_suspend_pick_matches_bird_inside_a_longer_trait() {
    let mut sky_bird = avian_target("SKY-BIRD");
    sky_bird.traits = vec!["Sky Bird".to_string()];
    let mut runner = base().add_card(sky_bird).hand(0, &["SKY-BIRD"]).start();
    let deramon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.suspend(deramon);
    assert!(runner.pending_selection().is_some(), "a [Sky Bird] Digimon is a candidate");
    runner.accept_optional_trigger().ok();
    runner.auto_resolve().expect("resolve the free play");
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "SKY-BIRD"),
        "SKY-BIRD was played"
    );
}

/// Playing nothing does not spend the [Once Per Turn]: DCGO `BT25_055.cs`
/// `if (!Used) activateClass.RemoveUse()` (the pick selected no card), so a
/// second self-suspend the same turn offers the free play again. Our YAML
/// needs no `refund_opt` for this: the clause is `optional: true` and its
/// first step is the optional pick, so the "you may" folds into that pick
/// and PASS is the activation decline (no use spent). Regression guard.
#[test]
fn bt25_055_self_suspend_declined_pick_does_not_spend_once_per_turn() {
    use digimon_engine::action::space::PASS;
    use digimon_engine::selection::SelectionKind;
    let mut runner = base().hand(0, &["AVIAN"]).start();
    let deramon = runner.place_on_field(0, CARD_ID, Some(0));

    // First self-suspend: open the hand pick and select nothing.
    runner.game.suspend(deramon);
    assert!(runner.pending_selection().is_some(), "first suspend fires");
    if runner.pending_kind() != Some(SelectionKind::Hand) {
        runner.accept_optional_trigger().expect("accept the outer you-may");
    }
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand), "the hand pick is open");
    runner.execute_action(0, PASS).expect("select no card");
    runner.auto_resolve().expect("settle");
    assert_eq!(runner.hand_size(0), 1, "nothing played");

    // Unsuspend and suspend again the same turn.
    runner.game.players[0].battle_area[deramon.index as usize].is_suspended = false;
    runner.game.suspend(deramon);
    assert!(
        runner.pending_selection().is_some(),
        "the empty pick refunded the [Once Per Turn]: the second suspend offers the play again"
    );
    if runner.pending_kind() != Some(SelectionKind::Hand) {
        runner.accept_optional_trigger().expect("accept the outer you-may");
    }
    runner.auto_resolve().expect("resolve the free play");
    assert_eq!(runner.hand_size(0), 0, "AVIAN played on the second suspend");
}

/// Regression guard: an [Avian] Digi-Egg is never a "Digimon card with 4000
/// DP or lower" play candidate (no DP, no play cost; CR 4.3 7-1-1).
#[test]
fn bt25_055_self_suspend_pick_excludes_avian_digi_egg() {
    assert_eq!(
        offered_after_self_suspend(&["AVIAN-EGG", "AVIAN"], |_| {}),
        Some(vec!["AVIAN".to_string()])
    );
}
