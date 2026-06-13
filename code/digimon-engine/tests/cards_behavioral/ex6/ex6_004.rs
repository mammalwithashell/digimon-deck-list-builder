//! EX6-004 Kokomon — Digi-Egg (In-Training), Lv.2, Green, no DP.
//! Traits: Lesser. Form: In-Training.
//!
//! # Card text (card image — verified) — INHERITED ONLY
//! `Inherited: [Your Turn] [Once Per Turn] When an effect suspends one of`
//! `your Digimon, 1 of your Digimon gets +2000 DP for the turn.`
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX6/EX6_004.cs
//!   - OnTappedAnyone, IsInheritedEffect, IsOwnerTurn, maxUsableCount: 1,
//!     gated `CardEffectCommons.IsByEffect(hashtable, null)` — the suspension
//!     must come FROM AN EFFECT (an attack-declaration / cost suspend does not
//!     qualify). Body: mandatory select 1 own battle-area Digimon
//!     (canNoSelect: false) → ChangeDigimonDP +2000 UntilEachTurnEnd.
//!
//! # Engine gate
//! The "by an effect" qualifier is `event_is_effect_initiated: true`, fed by
//! the `effect_initiated` bit on the suspend event
//! (G-SUSPEND-EFFECT-INITIATED): `EffectContext::suspend` tags the OnSuspend
//! trigger `effect_initiated: true`; the raw `Game::suspend` (attack
//! declaration, costs paid outside an effect context) tags `false`.
//!
//! Judge-quiz consumer: Q24 (Hudiemon `<Alliance>` suspends Tentomon carrying
//! this egg; the trigger fires but Tentomon dies to the 0-DP rules check
//! before the inherited effect activates).

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, ModifierType, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

fn egg() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("EX6-004")
}

fn make_digimon(card_id: &str, color: CardColor, dp: i32) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.dp = Some(dp);
    card.level = Some(4);
    card
}

/// Test effect whose body suspends a fixed target THROUGH the effect context
/// (the effect-initiated suspension path Kokomon's clause gates on).
struct SuspendByEffect {
    target: PermanentHandle,
}
impl CardEffect for SuspendByEffect {
    fn effects(&self, card: digimon_engine::card_source::CardHandle) -> Vec<Effect> {
        let target = self.target;
        vec![Effect::when_attacking(card)
            .name("suspend by effect")
            .process(move |ctx| {
                ctx.suspend(target);
            })
            .build()]
    }
}

/// Runner with EX6-004 + a host, an ally to suspend, and an effect source.
fn egg_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX6-004")
        .expect("EX6-004 in pack")
        .add_card(make_digimon("HOST", CardColor::Green, 4000))
        .add_card(make_digimon("ALLY", CardColor::Green, 3000))
        .add_card(make_test_card("SRC", "Src"))
        .add_card(make_digimon("FILLER", CardColor::Blue, 3000))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .memory(10)
        .start()
}

/// Place a host Digimon for `player` with EX6-004 underneath as a source.
fn host_with_egg(runner: &mut DebugRunner, player: PlayerId) -> PermanentHandle {
    runner.place_stack(player, &["EX6-004", "HOST"])
}

/// Fire `SuspendByEffect { target }` from SRC (owned by `player`).
fn suspend_via_effect(runner: &mut DebugRunner, player: PlayerId, target: PermanentHandle) {
    let src = runner.place_on_field(player, "SRC", None);
    runner.register_effect("SRC", Arc::new(SuspendByEffect { target }));
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(src));
    runner.game.drain_effect_queue();
}

/// Sum of ChangeDp modifier values on `handle`.
fn dp_mod_sum(runner: &DebugRunner, handle: PermanentHandle) -> i32 {
    runner
        .game
        .modifiers
        .get(handle, ModifierType::ChangeDp)
        .iter()
        .map(|e| e.value)
        .sum()
}

// ─── SECTION 1 — Structural ─────────────────────────────────────────────────

#[test]
fn ex6_004_metadata_is_green_lv2_egg_no_dp() {
    let card = egg();
    assert_eq!(card.card, "EX6-004");
    assert_eq!(card.name, "Kokomon");
    assert_eq!(card.level, Some(2));
    assert_eq!(card.dp, None, "an In-Training egg has no DP");
    assert!(card
        .color
        .contains(&digimon_dsl::compiled::CompiledColor::Green));
}

#[test]
fn ex6_004_has_single_inherited_opt_suspend_clause_no_main_effect() {
    let card = egg();
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1, "exactly one clause (the inherited buff)");
    let clause = triggered[0];
    assert_eq!(clause.scope, CompiledScope::Inherited, "must be inherited");
    assert!(clause.when.contains(&CompiledTiming::OnSuspend));
    assert!(clause.once_per_turn, "[Once Per Turn]");
    assert!(!clause.optional, "mandatory — no 'may'");
}

// ─── SECTION 2 — Behavioral (effect-initiated suspend → +2000) ──────────────

#[test]
fn ex6_004_effect_suspend_grants_2000_to_selected_digimon() {
    let mut runner = egg_runner();
    let host = host_with_egg(&mut runner, 0);
    let ally = runner.place_on_field(0, "ALLY", Some(0));

    suspend_via_effect(&mut runner, 0, ally);

    // Kokomon's mandatory "1 of your Digimon gets +2000" selection must be
    // pending (the suspension came from an effect, on the owner's turn).
    assert!(
        runner.game.pending_selection.is_some(),
        "effect-initiated suspend of an own Digimon must surface Kokomon's \
         +2000 target selection"
    );
    // Resolve it (whichever own Digimon the picker lands on gets +2000).
    let _ = runner.auto_resolve();

    let total = dp_mod_sum(&runner, host) + dp_mod_sum(&runner, ally);
    assert_eq!(
        total, 2000,
        "exactly one +2000 ChangeDp modifier lands on one of P0's Digimon"
    );
}

// ─── SECTION 2b — Negatives ─────────────────────────────────────────────────

#[test]
fn ex6_004_raw_suspend_does_not_trigger() {
    let mut runner = egg_runner();
    let host = host_with_egg(&mut runner, 0);
    let ally = runner.place_on_field(0, "ALLY", Some(0));

    // A non-effect suspension (attack declaration / cost path).
    runner.game.suspend(ally);
    assert!(
        runner.game.pending_selection.is_none(),
        "a NON-effect suspension must not fire 'when an effect suspends'"
    );
    let _ = runner.auto_resolve();
    assert_eq!(dp_mod_sum(&runner, host) + dp_mod_sum(&runner, ally), 0);
}

#[test]
fn ex6_004_opponent_digimon_suspend_does_not_trigger() {
    let mut runner = egg_runner();
    host_with_egg(&mut runner, 0);
    let opp = runner.place_on_field(1, "ALLY", Some(0));

    suspend_via_effect(&mut runner, 0, opp);
    assert!(
        runner.game.pending_selection.is_none(),
        "the OPPONENT's Digimon suspending does not satisfy 'one of your Digimon'"
    );
}

#[test]
fn ex6_004_does_not_trigger_on_opponents_turn() {
    let mut runner = egg_runner();
    host_with_egg(&mut runner, 0);
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    runner.game.turn_player_idx = 1; // opponent's turn

    suspend_via_effect(&mut runner, 1, ally);
    assert!(
        runner.game.pending_selection.is_none(),
        "[Your Turn] gate blocks the trigger on the opponent's turn"
    );
}

// ─── SECTION 3 — OPT lockout ────────────────────────────────────────────────

#[test]
fn ex6_004_opt_blocks_second_effect_suspend_same_turn() {
    let mut runner = egg_runner();
    let host = host_with_egg(&mut runner, 0);
    let a1 = runner.place_on_field(0, "ALLY", Some(0));
    let a2 = runner.place_on_field(0, "FILLER", Some(0));

    suspend_via_effect(&mut runner, 0, a1);
    assert!(runner.game.pending_selection.is_some());
    let _ = runner.auto_resolve();
    let total_after_first =
        dp_mod_sum(&runner, host) + dp_mod_sum(&runner, a1) + dp_mod_sum(&runner, a2);
    assert_eq!(total_after_first, 2000);

    suspend_via_effect(&mut runner, 0, a2);
    assert!(
        runner.game.pending_selection.is_none(),
        "[Once Per Turn] blocks a second effect-suspend trigger the same turn"
    );
    let _ = runner.auto_resolve();
    let total_after_second =
        dp_mod_sum(&runner, host) + dp_mod_sum(&runner, a1) + dp_mod_sum(&runner, a2);
    assert_eq!(total_after_second, 2000, "no second +2000");
}
