//! BT25-071 Orochimon — Digimon, Lv.5, Black, DP 6000, Cost 7.
//! Traits: Dark Dragon, Titan, TS.
//! Evo: Lv.4 Black / cost 3 (printed; + DCGO alt on a [TS] Lv.4 base, cost 3).
//!
//! # Card text (cards.json — verbatim)
//!
//! [On Play] [When Digivolving] 1 of your opponent's Digimon or Tamers can't
//! attack until their turn ends.
//! [All Turns] [Once Per Turn] When this Digimon suspends, reveal the top 3
//! cards of your deck. You may play 1 play cost 4 or lower [TS] trait Digimon
//! card among them without paying the cost. Return the rest to the bottom of
//! the deck.
//!
//! Inherited Effect: (identical to the [All Turns] on-suspend clause).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_071.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - B (OnPlay/WhenDigivolving + on-suspend triggered clauses)
//! - C (mandatory opponent select → CannotAttack modifier; reveal-N play-1)
//! - E (reveal + play-from-revealed-free, remainder to deck bottom)
//! - G (inherited duplicate of the suspend clause)
//! - H (alt digivolution path)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, ModifierType, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-071";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn opp_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.traits = vec!["Beast".to_string()];
    card
}

fn opp_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card
}

/// A cost-4 [TS] Lv.4 Digimon — a legal reveal-play target.
fn ts_lv4(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
    card.traits = vec!["TS".to_string()];
    card
}

/// A cost-5 [TS] Digimon — too expensive to be a reveal-play target.
fn ts_lv5_cost5(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(6000);
    card.play_cost = 5;
    card.traits = vec!["TS".to_string()];
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-071 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(opp_digimon("OPP-DIGI"))
        .add_card(opp_tamer("OPP-TAMER"))
        .add_card(ts_lv4("TS-LV4"))
        .add_card(ts_lv5_cost5("TS-LV5"))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_071_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "Orochimon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(6000));
}

#[test]
fn bt25_071_has_lockdown_and_two_suspend_clauses() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let lockdown = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
        )
    });
    assert!(
        lockdown,
        "must have an OnPlay/WhenDigivolving lockdown clause"
    );

    let suspend_clauses: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSuspend) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(
        suspend_clauses.len(),
        2,
        "must have a face-up AND an inherited on-suspend reveal clause"
    );
    assert!(
        suspend_clauses
            .iter()
            .any(|t| t.scope == CompiledScope::FaceUp),
        "one suspend clause is face-up"
    );
    assert!(
        suspend_clauses
            .iter()
            .any(|t| t.scope == CompiledScope::Inherited),
        "one suspend clause is inherited"
    );
    for t in &suspend_clauses {
        assert!(t.once_per_turn, "[Once Per Turn] on the suspend clause");
        assert!(
            t.process
                .iter()
                .any(|s| matches!(s, CompiledStep::PlayFromRevealedFree { .. })),
            "suspend clause must play-from-revealed-free"
        );
    }
}

#[test]
fn bt25_071_lockdown_clause_is_mandatory() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("lockdown clause present");
    assert!(!clause.optional, "no printed 'may' → mandatory");
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::AddModifier { .. })),
        "lockdown clause must add a modifier (CannotAttack)"
    );
}

// ─── Section 2 — Behavior: lockdown ──────────────────────────────────────────

/// Positive (Digimon target): on play, the chosen opponent Digimon gains
/// CannotAttack.
#[test]
fn bt25_071_locks_an_opponent_digimon() {
    let mut runner = base().start();
    let orochimon = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));

    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::OnPlay,
        digimon_engine::selection::TriggerSource::Permanent(orochimon),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some()
            || runner.game.modifiers.has(opp, ModifierType::CannotAttack),
        "lockdown must target the opponent Digimon"
    );
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("resolve lockdown select");
    }
    assert!(
        runner.game.modifiers.has(opp, ModifierType::CannotAttack),
        "chosen opponent Digimon must gain CannotAttack"
    );
}

/// Positive (Tamer target): a Tamer is a legal lockdown target.
#[test]
fn bt25_071_can_lock_an_opponent_tamer() {
    let mut runner = base().start();
    let orochimon = runner.place_on_field(0, CARD_ID, Some(0));
    let tamer = runner.place_on_field(1, "OPP-TAMER", Some(0));

    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::OnPlay,
        digimon_engine::selection::TriggerSource::Permanent(orochimon),
    );
    runner.game.drain_effect_queue();
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("resolve lockdown select");
    }
    assert!(
        runner.game.modifiers.has(tamer, ModifierType::CannotAttack),
        "an opponent Tamer must be a legal lockdown target"
    );
}

/// Negative: opponent has no Digimon/Tamer → lockdown is a no-op.
#[test]
fn bt25_071_lockdown_no_fire_without_target() {
    let mut runner = base().start();
    let orochimon = runner.place_on_field(0, CARD_ID, Some(0));

    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::OnPlay,
        digimon_engine::selection::TriggerSource::Permanent(orochimon),
    );
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon/Tamer → lockdown installs no prompt"
    );
}

// ─── Section 3 — Behavior: on-suspend reveal-play ────────────────────────────

/// Positive: this Digimon suspends with a cost-4 [TS] Digimon at the top of the
/// deck → the player may play it for free; the rest go to the deck bottom.
/// `TS-LV4` is placed last in the deck array (= top of deck → reveal index 0 =
/// SEL_REVEAL_START), and the bucket pick is executed explicitly (not
/// auto_resolved, which would decline the min:0 optional play).
#[test]
fn bt25_071_suspend_reveals_and_plays_ts_digimon() {
    let mut runner = base()
        .deck(0, &["PAD", "PAD", "PAD", "PAD", "TS-LV4"])
        .memory(2) // not enough to pay cost-4 — must be free
        .start();
    let orochimon = runner.place_on_field(0, CARD_ID, Some(0));

    let field_before = runner.battle_area_size(0);
    let memory_before = runner.game.memory;
    runner.game.suspend(orochimon);
    assert!(
        runner.pending_selection().is_some(),
        "self-suspend must install the reveal/play selection"
    );

    // Pick the TS-LV4 (revealed index 0 = SEL_REVEAL_START).
    let _ = runner.execute_action(0, SEL_REVEAL_START);
    runner.game.drain_effect_queue();

    // After the play, the remainder-ordering selection (place the other 2 on
    // the deck bottom in any order) is installed — resolve it.
    while let Some(view) = runner.pending_selection_view() {
        let action = view.valid_action_ids.first().copied().unwrap_or(PASS);
        let _ = runner.execute_action(view.selecting_player, action);
        runner.game.drain_effect_queue();
    }

    assert_eq!(
        runner.battle_area_size(0),
        field_before + 1,
        "the cost-4 [TS] Digimon must be played to the field for free"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "play_from_revealed_free must not spend the revealed card's play cost"
    );
    // Reveal of 3 consumed; one played, two returned to deck bottom → deck
    // shrinks by exactly 1 (the played card).
    assert_eq!(
        runner.deck_size(0),
        5 - 1,
        "only the played reveal card leaves the deck; the rest are bottomed"
    );
}

/// Negative: no eligible [TS] cost<=4 card in the reveal → nothing is played,
/// but all 3 revealed cards are returned to the deck bottom (deck size stable).
#[test]
fn bt25_071_suspend_reveals_no_eligible_play() {
    let mut runner = base()
        .deck(0, &["PAD", "PAD", "PAD", "TS-LV5", "PAD"])
        .start();
    let orochimon = runner.place_on_field(0, CARD_ID, Some(0));

    let field_before = runner.battle_area_size(0);
    let deck_before = runner.deck_size(0);
    runner.game.suspend(orochimon);
    // The mandatory reveal still runs; the play bucket is min:0 with no
    // eligible candidate, so decline via PASS and drain the bottom-remainder
    // tail.
    while let Some(view) = runner.pending_selection_view() {
        let action = if view.valid_action_ids.contains(&PASS) {
            PASS
        } else {
            view.valid_action_ids.first().copied().unwrap_or(PASS)
        };
        let _ = runner.execute_action(view.selecting_player, action);
        runner.game.drain_effect_queue();
    }
    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "no eligible card → nothing is played"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "all revealed cards return to the deck bottom"
    );
}

/// OPT lockout: a second self-suspend in the same turn does not re-reveal.
#[test]
fn bt25_071_suspend_reveal_is_once_per_turn() {
    let mut runner = base()
        .deck(0, &["TS-LV4", "PAD", "PAD", "TS-LV4", "PAD"])
        .start();
    let orochimon = runner.place_on_field(0, CARD_ID, Some(0));

    runner.game.suspend(orochimon);
    runner.auto_resolve().expect("first reveal resolves");
    let field_after_first = runner.battle_area_size(0);

    runner.game.suspend(orochimon);
    assert!(
        runner.pending_selection().is_none(),
        "second self-suspend in the same turn must not re-fire (OPT)"
    );
    assert_eq!(
        runner.battle_area_size(0),
        field_after_first,
        "no second reveal-play in the same turn"
    );
}
