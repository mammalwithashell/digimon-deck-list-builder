//! BT7-032 Pulsemon — Digimon, Lv.3, Yellow, DP 2000, Cost 3.
//! Traits: Beastkin. Form: Rookie. Attribute: Vaccine.
//! Standard digivolve (official Bandai DB bundle / cards.json evo_costs):
//! Yellow Lv.2 / cost 0.
//!
//! # Card text (card image + data/card_bundles/BT7-032.md — verbatim)
//!
//! ```text
//! Inherited Effect [When Attacking][Once Per Turn] If you have 3 security
//! cards, gain 2 memory.
//! ```
//!
//! No [On Play]/[When Digivolving]/[Security] text, no DigiXros/DNA/Assembly,
//! no Rule-granted traits. Single inherited clause.
//!
//! # DCGO C# reference
//!
//! DCGO/Assets/Scripts/CardEffect/BT7/Yellow/BT7_032.cs
//!
//! `EffectTiming.OnAllyAttack` ActivateClass, `SetIsInheritedEffect(true)`,
//! `SetUpActivateClass(CanActivateCondition, ActivateCoroutine, maxUsableCount:
//! 1, isOptional:false, ...)`.
//!
//! - `CanUseCondition`: `IsExistOnBattleArea(card) && CanTriggerOnAttack(hashtable,
//!   card)`. `CanTriggerOnAttack` -> `CanTriggerOnPermanentAttack(hashtable,
//!   permanent => permanent.cardSources.Contains(card))` -- true only when the
//!   ATTACKING permanent's own card-source stack contains this card, i.e. this
//!   card's stack IS the attacker. This is carrier-scoped ("when THIS Digimon
//!   attacks"), matching the engine's `WhenAttacking` timing (combat.rs lines
//!   3750-3758), not the ally-observer `OnAllyAttack` fan-out. Authored with
//!   `when: when_attacking`.
//! - `CanActivateCondition`: `IsExistOnBattleArea(card)` -- no extra gate beyond
//!   still being on the field at activation time.
//! - `ActivateCoroutine`: `if (card.Owner.SecurityCards.Count == 3) -> AddMemory(2)
//!   else -> activateClass.RemoveUse()` -- the RemoveUse() rollback matches the
//!   engine's OPT-count-before-condition ordering (a turn where the security
//!   count never equals exactly 3 never consumes the [Once Per Turn] slot).
//!
//! # Engine note: [When Attacking] on an INHERITED clause is dormant on the
//! carrier's own top-card position (rule 15-3-1: an inherited effect is
//! active only while its card is a digivolution source BENEATH another
//! card). `effect_queue.rs`'s top-card scan explicitly skips
//! `effect.inherited` clauses (see the comment at `enqueue_from_permanent`,
//! "An inherited effect ... is active ONLY while that card is a digivolution
//! source beneath another card"); the dedicated below-top `inherited_sources`
//! scan is what dispatches them. So this card's [When Attacking][OPT] clause
//! never fires while Pulsemon itself is the lone/top permanent that
//! declares the attack -- only when Pulsemon is buried under a higher-level
//! carrier that attacks. This is real TCG rules behavior, not an engine gap,
//! and is pinned by a dedicated negative test below.
//!
//! # Patterns this test covers
//!
//! - Structural: metadata, the Yellow Lv.2/cost 0 digivolve alt-path, and the
//!   inherited WhenAttacking clause shape (inherited scope, once_per_turn,
//!   mandatory, `security_count_eq == 3`).
//! - Negative: Pulsemon alone/on top of its own stack does NOT fire its
//!   inherited clause when it attacks (rule 15-3-1 -- inherited effects are
//!   dormant on the card's own top position).
//! - Positive: Pulsemon buried under a carrier, carrier attacks with exactly
//!   3 security cards -> gains 2 memory.
//! - Negative: buried Pulsemon's carrier attacks with fewer than 3 security
//!   cards -> no gain.
//! - Negative: buried Pulsemon's carrier attacks with more than 3 security
//!   cards -> no gain (DCGO's condition is an exact `== 3`, not `>= 3`).
//! - Carrier-scoping: a DIFFERENT ally Digimon attacking (not the carrier
//!   whose stack contains Pulsemon) does NOT fire the inherited clause
//!   (WhenAttacking is carrier-scoped, not an ally-observer fan-out).
//! - Once-per-turn lockout: after firing once this turn (at exactly 3
//!   security), a genuine SECOND attack in the same turn (carrier manually
//!   unsuspended between attacks, since attacking suspends the attacker) does
//!   NOT gain memory again, even though the security count is still exactly 3.
//! - OPT clears after end_turn: on the next turn, at exactly 3 security again,
//!   the trigger fires and gains memory again.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDpConstraint,
    CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::permanent::PermanentHandle;

// -- Helpers ------------------------------------------------------------

/// Base runner with BT7-032 loaded from the embedded DSL pack, plus a filler
/// card for padding hands/decks/security stacks, and a higher-level CARRIER
/// Digimon so Pulsemon can be placed as a buried digivolution source (the
/// only field position where its inherited clause is live).
fn pulsemon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT7-032")
        .expect("BT7-032 in embedded DSL pack")
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card({
            let mut c = make_test_card("CARRIER", "Carrier");
            c.card_kind = digimon_engine::enums::CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(5000);
            c
        })
        .add_card({
            let mut c = make_test_card("ALLY", "Ally");
            c.card_kind = digimon_engine::enums::CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(4000);
            c
        })
}

/// Memory starts at 0 (well within the standard `(-10, 10)` `memory_range`)
/// so a +2 gain is observable and not clamped at the ceiling.
const START_MEMORY: i16 = 0;

// =============================================================================
// Section 1 - Structural assertions
// =============================================================================

#[test]
fn bt7_032_yaml_compiles_and_is_in_pack() {
    let runner = pulsemon_runner().start();
    assert!(
        runner.compiled_card("BT7-032").is_some(),
        "BT7-032 must be present in the embedded DSL pack"
    );
}

#[test]
fn bt7_032_metadata_and_digivolve_path_match_printed_text() {
    let runner = pulsemon_runner().start();
    let compiled = runner.compiled_card("BT7-032").expect("BT7-032 compiles");

    assert_eq!(compiled.name, "Pulsemon");
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.dp, Some(2000));
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert!(
        compiled.traits.iter().any(|t| t == "Beastkin"),
        "Beastkin trait must be present; traits={:?}",
        compiled.traits
    );

    let digivolve_paths: Vec<_> = compiled
        .alt_paths
        .iter()
        .filter(|path| path.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert!(
        digivolve_paths.iter().any(|path| {
            path.cost == Some(CompiledCost::Literal(0))
                && path.from.as_ref().is_some_and(|from| {
                    from.level_eq == Some(2) && from.color_is == Some(CompiledColor::Yellow)
                })
        }),
        "standard Yellow Lv.2/cost 0 digivolve path must be present; paths={digivolve_paths:?}"
    );
}

/// The inherited clause must be WhenAttacking, Inherited scope, mandatory,
/// once_per_turn, and gated on security_count_eq == 3 (not gte/lte).
#[test]
fn bt7_032_inherited_when_attacking_clause_shape() {
    let runner = pulsemon_runner().start();
    let card = runner.compiled_card("BT7-032").expect("BT7-032 in pack");

    let clause: &CompiledTriggeredClause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if matches!(t.scope, CompiledScope::Inherited)
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("must have an inherited WhenAttacking triggered clause");

    assert_eq!(
        clause.scope,
        CompiledScope::Inherited,
        "clause must be Inherited scope"
    );
    assert!(
        !clause.optional,
        "gain 2 memory is mandatory per printed text (no 'you may')"
    );
    assert!(clause.once_per_turn, "printed text carries [Once Per Turn]");
    assert_eq!(
        clause
            .condition
            .as_ref()
            .and_then(|p| p.security_count_eq.clone()),
        Some(CompiledDpConstraint::Literal(3)),
        "condition must be an EXACT security_count == 3 gate, not gte/lte"
    );
}

// =============================================================================
// Section 2 - Behavioral: carrier-position scoping (rule 15-3-1)
// =============================================================================

/// NEGATIVE: Pulsemon alone on the field (its own top-card position) does
/// NOT fire its inherited clause when it attacks. Inherited effects are
/// active only while the card is a digivolution source BENEATH another
/// card (rule 15-3-1); the engine's top-card scan explicitly skips
/// `effect.inherited` clauses.
#[test]
fn bt7_032_inherited_does_not_fire_from_its_own_top_position() {
    let mut runner = pulsemon_runner()
        .security(0, &["FILLER", "FILLER", "FILLER"])
        .memory(START_MEMORY)
        .start();
    runner.game.turn_count = 3;
    runner.game.turn_player_idx = 0;

    let carrier = runner.place_on_field(0, "BT7-032", Some(0));
    let memory_before = runner.memory();

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before,
        "Pulsemon on its own top position must NOT fire the inherited [When Attacking] clause, \
         even with exactly 3 security cards (rule 15-3-1: inherited effects are dormant on top)"
    );
}

// =============================================================================
// Section 3 - Behavioral: condition gating (exactly 3 security cards),
// firing from a buried source under a carrier (the clause's only live
// position)
// =============================================================================

/// POSITIVE: Pulsemon buried under a carrier; carrier attacks with exactly
/// 3 security cards -> gains 2 memory.
#[test]
fn bt7_032_when_attacking_gains_2_memory_at_exactly_3_security() {
    let mut runner = pulsemon_runner()
        .security(0, &["FILLER", "FILLER", "FILLER"])
        .memory(START_MEMORY)
        .start();
    runner.game.turn_count = 3;
    runner.game.turn_player_idx = 0;

    let carrier = runner.place_stack(0, &["BT7-032", "CARRIER"]);
    let memory_before = runner.memory();

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before + 2,
        "carrier attacking with Pulsemon buried and exactly 3 security cards must gain 2 memory"
    );
}

/// NEGATIVE: buried Pulsemon's carrier attacks with fewer than 3 security
/// cards -> no gain.
#[test]
fn bt7_032_when_attacking_no_gain_below_3_security() {
    let mut runner = pulsemon_runner()
        .security(0, &["FILLER", "FILLER"])
        .memory(START_MEMORY)
        .start();
    runner.game.turn_count = 3;
    runner.game.turn_player_idx = 0;

    let carrier = runner.place_stack(0, &["BT7-032", "CARRIER"]);
    let memory_before = runner.memory();

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before,
        "attacking with fewer than 3 security cards must NOT gain memory"
    );
}

/// NEGATIVE: buried Pulsemon's carrier attacks with more than 3 security
/// cards -> no gain (DCGO's condition is an exact `== 3`, not `>= 3`).
#[test]
fn bt7_032_when_attacking_no_gain_above_3_security() {
    let mut runner = pulsemon_runner()
        .security(0, &["FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(START_MEMORY)
        .start();
    runner.game.turn_count = 3;
    runner.game.turn_player_idx = 0;

    let carrier = runner.place_stack(0, &["BT7-032", "CARRIER"]);
    let memory_before = runner.memory();

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before,
        "attacking with more than 3 security cards must NOT gain memory (exact-3 gate, not >=3)"
    );
}

// =============================================================================
// Section 4 - Behavioral: carrier-scoping (WhenAttacking, not ally-observer)
// =============================================================================

/// NEGATIVE: a DIFFERENT ally Digimon attacking (not the carrier whose
/// stack contains Pulsemon) must NOT fire Pulsemon's inherited clause --
/// WhenAttacking is carrier-scoped, not an ally-observer fan-out.
#[test]
fn bt7_032_inherited_does_not_fire_when_a_different_ally_attacks() {
    let mut runner = pulsemon_runner()
        .security(0, &["FILLER", "FILLER", "FILLER"])
        .memory(START_MEMORY)
        .start();
    runner.game.turn_count = 3;
    runner.game.turn_player_idx = 0;

    // Pulsemon sits buried under CARRIER (idle); a separate ALLY Digimon
    // attacks instead.
    let _carrier = runner.place_stack(0, &["BT7-032", "CARRIER"]);
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    let memory_before = runner.memory();

    runner.attack_player(ally, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before,
        "a different ally's attack must NOT fire Pulsemon's carrier-scoped inherited clause"
    );
}

// =============================================================================
// Section 5 - Behavioral: [Once Per Turn] lockout + clearing
// =============================================================================

/// The trigger fires at most once per turn even if the carrier genuinely
/// attacks a second time in the same turn while the condition still holds.
///
/// Attacking suspends the attacker, so a naive "declare a second attack in
/// the same turn" without manually unsuspending would have the SECOND
/// `attack_player` call rejected by `can_attack` (suspended attackers can't
/// attack) -- that would make the trigger's absence on the second attack
/// trivially true regardless of whether [Once Per Turn] actually gates it
/// (no attack = no trigger = no memory change either way). To make this a
/// real assertion of the OPT lockout, the carrier is manually unsuspended
/// between the two attacks (mirrors the idiom in
/// tests/archetypes/thomas_data_squad_bt25.rs:203 and
/// tests/cost_hooks/activation_cost.rs:247), so the second `attack_player`
/// genuinely declares and fires the [When Attacking] trigger a second time --
/// and the assertion that memory does not change again proves the OPT gate
/// itself blocks the repeat gain.
#[test]
fn bt7_032_opt_locks_out_second_attack_same_turn() {
    let mut runner = pulsemon_runner()
        .security(0, &["FILLER", "FILLER", "FILLER"])
        .memory(START_MEMORY)
        .start();
    runner.game.turn_count = 3;
    runner.game.turn_player_idx = 0;

    let carrier = runner.place_stack(0, &["BT7-032", "CARRIER"]);
    let memory_before_first = runner.memory();

    // First attack: condition holds (exactly 3 security) -> gains 2 memory.
    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();
    if runner.game_over() {
        return;
    }
    let memory_after_first = runner.memory();
    assert_eq!(
        memory_after_first,
        memory_before_first + 2,
        "first attack at exactly 3 security must gain 2 memory"
    );

    // Manually unsuspend the carrier so the second attack genuinely declares
    // (attacking re-suspends the attacker) -- the OPT gate is then the ONLY
    // thing that can block the second memory gain.
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = false;

    let carrier_again = runner.perm_handle(0, 0);
    assert!(
        !runner.game.players[0].battle_area[carrier_again.index as usize].is_suspended,
        "carrier must be unsuspended before the second attack, or the second \
         attack_player call would be rejected outright and this test would \
         pass trivially regardless of whether [Once Per Turn] gates anything"
    );

    // Second attack, same turn: [Once Per Turn] must block the repeat gain
    // even though the security-count condition still holds.
    let second_attack_result = runner.attack_player(carrier_again, 1, false);
    assert!(
        !matches!(
            second_attack_result,
            digimon_engine::combat::AttackResult::Invalid
        ),
        "the second attack must genuinely declare (not be rejected as Invalid) \
         so the OPT lockout assertion below is a real test of the gate, not a \
         trivial no-attack-means-no-trigger pass; got {second_attack_result:?}"
    );
    let _ = runner.auto_resolve();
    if runner.game_over() {
        return;
    }
    let memory_after_second = runner.memory();

    assert_eq!(
        memory_after_second, memory_after_first,
        "[Once Per Turn] must block a second memory gain from a genuine second attack in the same turn"
    );
}

/// OPT clears after end_turn: on the next turn, at exactly 3 security again,
/// the trigger fires and gains memory again.
#[test]
fn bt7_032_opt_clears_after_end_turn() {
    let mut runner = pulsemon_runner()
        .security(0, &["FILLER", "FILLER", "FILLER"])
        .deck(0, &["FILLER", "FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(START_MEMORY)
        .start();
    runner.game.turn_count = 3;
    runner.game.turn_player_idx = 0;

    let carrier = runner.place_stack(0, &["BT7-032", "CARRIER"]);
    let memory_before_first = runner.memory();

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();
    if runner.game_over() {
        return;
    }
    let memory_after_first = runner.memory();
    assert_eq!(
        memory_after_first,
        memory_before_first + 2,
        "first attack at exactly 3 security must gain 2 memory"
    );

    // End turns P0 -> P1 -> P0 so it is P0's turn again, with a fresh OPT slot.
    runner.end_turn();
    runner.end_turn();
    if runner.game_over() {
        return;
    }

    // Carrier is unsuspended by the Unsuspend phase on P0's new turn.
    let carrier_again = runner.perm_handle(0, 0);
    let memory_before_second = runner.memory();

    runner.attack_player(carrier_again, 1, false);
    let _ = runner.auto_resolve();
    if runner.game_over() {
        return;
    }

    assert_eq!(
        runner.memory(),
        memory_before_second + 2,
        "on a new turn the [Once Per Turn] slot must have cleared, so a fresh attack at exactly 3 security gains 2 memory again"
    );
}
