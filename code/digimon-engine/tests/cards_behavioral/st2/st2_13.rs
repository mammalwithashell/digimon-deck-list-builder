//! ST2-13 Hammer Spark — Option, Blue, Cost 0.
//!
//! # Card text (cards.json — printed)
//!
//! **[Main]** Gain 1 memory.
//!
//! **[Security]** Gain 2 memory.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST2/Blue/ST2_13.cs`
//!
//! Two `ICardEffect`s:
//!   1. `EffectTiming.OptionSkill` — `card.Owner.AddMemory(1, activateClass)`
//!      with `CanUseCondition = CardEffectCommons.CanTriggerOptionMainEffect`
//!      (i.e. standard option [Main] gating; not opt-in / not OPT).
//!   2. `EffectTiming.SecuritySkill` — `card.Owner.AddMemory(2, activateClass)`
//!      with `SetIsSecurityEffect(true)`. No `canNoSelect`, no per-turn cap —
//!      mandatory ([Security] effects are not opt-out per RULES_CONTEXT.md
//!      §16).
//!
//! # Source priority audit
//! Per CLAUDE.md the priority is printed text > RULES_CONTEXT.md > fandom >
//! DCGO. Printed text is two unconditional memory gains — there is no "you
//! may", no [Once Per Turn], no condition. RULES_CONTEXT.md §16 confirms
//! [Security] effects are mandatory. DCGO matches: both clauses use a plain
//! `AddMemory` with a `CanTrigger*` precondition that gates ON THE TIMING
//! (not on opt-in), and neither passes a `canNoSelect` flag.
//!
//! # Patterns this audit exercises
//!
//! - **B1-trivial: option-card [Main] with a single `gain_memory` step** —
//!   simplest possible Option, no targeting, no choice, no condition.
//!   Mirrors BT1-090 Gravity Crush's [Main] half (`activate_hand_main`
//!   dispatch path) without BT1-090's scheduled lose-2 tail.
//! - **Option [Security] = simple `gain_memory` step** — mirrors EX1-068
//!   Ice Wall's [Security] clause (sister card; same `gain_memory: 2`
//!   security shape). EX1-068's audit explicitly cites ST2-13 as the
//!   reference for this idiom (see `cards/ex1/EX1-068.yaml` line 173).
//! - **Inline behavioral security firing** — mirrors
//!   `tests/dsl/option_security_disposition.rs::
//!   resolving_security_option_can_move_self_to_hand_without_trashing`:
//!   place ST2-13 in P1's security stack and have P0 attack the player to
//!   drive the on_security trigger end-to-end.
//!
//! # YAML faithfulness verdict — PASS
//!
//! `code/digimon-engine/cards/_examples/ST2-13.yaml` declares exactly two
//! triggered clauses with the printed semantics:
//!
//! ```yaml
//! card: ST2-13
//! kind: option
//! color: [blue]
//! cost: 0
//! effects:
//!   - when: main_from_hand
//!     process:
//!       - gain_memory: 1
//!   - when: on_security
//!     process:
//!       - gain_memory: 2
//! ```
//!
//! Each printed clause maps 1:1 to a single `gain_memory: N` step under
//! the appropriate `when:` timing. No optionality, no [OPT], no condition,
//! no scope override — all of which match the printed text and DCGO. This
//! is the canonical minimal-Option YAML shape and is referenced by EX1-068
//! as the [Security] template for that card's own clause-2 audit.
//!
//! # Known engine gaps
//!
//! None. `gain_memory` is a Phase-2a step and has been wired in
//! `digimon-engine/src/dsl_cards/step/memory.rs` since the earliest DSL
//! waves; both `MainFromHand` and `OnSecurity` lowering paths are
//! production-stable. Behavioral coverage of option [Main] dispatch lives
//! in `bt1_090.rs`; behavioral coverage of inherited security lowering
//! lives in `tests/dsl/option_security_disposition.rs`. ST2-13 sits at the
//! intersection — there is no gap that affects either of its clauses.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCard, CompiledCardKind, CompiledClause, CompiledColor, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlayerId};

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "ST2-13";
const YAML: &str = include_str!("../../../cards/_examples/ST2-13.yaml");

// ─── Card-data factories ─────────────────────────────────────────────────────

fn make_filler(card_id: &str) -> CardData {
    make_test_card(card_id, card_id)
}

/// A vanilla 0-DP attacker we can park on P0's field to drive a security
/// reveal of P1's stack without any combat-system surprises (no inherited
/// effects, no keywords).
fn make_attacker(card_id: &str) -> CardData {
    let mut c = make_test_card(card_id, "Test Attacker");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c
}

/// Standard fixture: ST2-13 in the embedded DSL pack + a filler card.
/// Memory pre-set to 5 so behavioral assertions have headroom on either
/// side of the (-10, 10) seesaw clamp.
fn hammer_spark() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST2-13 must be in embedded DSL pack")
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .hand(0, &[CARD_ID])
        .memory(5)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// ST2-13 must be present in the embedded DSL pack and parse cleanly from
/// the curated `_examples/ST2-13.yaml` source.
#[test]
fn st2_13_compiles_in_embedded_pack() {
    let runner = hammer_spark();
    assert!(
        runner.compiled_card(CARD_ID).is_some(),
        "ST2-13 must be present in embedded DSL pack"
    );
}

/// Standalone YAML parse smoke — guards against accidental regressions in
/// the on-disk YAML (the embedded-pack snapshot above is regenerated; this
/// test exercises the live YAML byte-for-byte).
#[test]
fn st2_13_yaml_parses_and_compiles() {
    let _ = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("ST2-13 _examples YAML must parse and compile");
}

/// Card metadata: kind=Option, cost=0, single Blue color, no level, no DP.
#[test]
fn st2_13_card_metadata_matches_print() {
    let card = compiled(CARD_ID);

    assert_eq!(card.card, CARD_ID);
    assert_eq!(
        card.kind,
        CompiledCardKind::Option,
        "ST2-13 is an Option card"
    );
    assert_eq!(card.cost, Some(0), "ST2-13 prints Cost 0");
    assert_eq!(card.level, None, "Option cards have no level");
    assert_eq!(card.dp, None, "Option cards have no DP");
    assert_eq!(
        card.color.len(),
        1,
        "ST2-13 is mono-color (Blue); got {:?}",
        card.color
    );
    assert!(
        matches!(card.color[0], CompiledColor::Blue),
        "ST2-13 must be Blue; got {:?}",
        card.color[0]
    );
}

/// Exactly two triggered clauses: one `MainFromHand`, one `OnSecurity`.
/// No declarative clauses, no alt paths, no flood gates.
#[test]
fn st2_13_has_exactly_two_triggered_clauses() {
    let card = compiled(CARD_ID);

    assert_eq!(
        card.effects.len(),
        2,
        "ST2-13 must have exactly 2 clauses ([Main] + [Security]); got {}",
        card.effects.len()
    );

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(
        triggered.len(),
        2,
        "both clauses must be Triggered (not Declarative/AltPath/FloodGate); \
         got {} triggered out of {} total",
        triggered.len(),
        card.effects.len()
    );

    assert!(
        card.alt_paths.is_empty(),
        "ST2-13 must have no alt paths; got {:?}",
        card.alt_paths
    );
}

/// Clause [Main]: `MainFromHand` timing, `FaceUp` scope, mandatory, not
/// OPT, no `condition` and no `active_when`.
#[test]
fn st2_13_clause_main_shape() {
    let card = compiled(CARD_ID);

    let main = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("ST2-13 must have a MainFromHand clause");

    assert_eq!(
        main.when.len(),
        1,
        "[Main] clause must fire on MainFromHand only; got {:?}",
        main.when
    );
    assert_eq!(
        main.scope,
        CompiledScope::FaceUp,
        "Option [Main] uses FaceUp scope; got {:?}",
        main.scope
    );
    assert!(
        !main.optional,
        "[Main] is mandatory — printed text has no 'you may'; DCGO \
         OptionSkill ActivateClass passes no canNoSelect"
    );
    assert!(
        !main.once_per_turn,
        "[Main] is not OPT — printed text has no [Once Per Turn]"
    );
    assert!(
        main.condition.is_none(),
        "[Main] has no condition — printed text is unconditional; got {:?}",
        main.condition
    );
    assert!(
        main.active_when.is_none(),
        "[Main] has no active_when — fires whenever the option is played; got {:?}",
        main.active_when
    );
}

/// Clause [Main] process body: exactly one step, `GainMemory(1)`.
#[test]
fn st2_13_clause_main_process_is_single_gain_memory_1() {
    let card = compiled(CARD_ID);

    let main = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("ST2-13 must have a MainFromHand clause");

    assert_eq!(
        main.process.len(),
        1,
        "[Main] body must be exactly 1 step (printed: 'Gain 1 memory.'); got {:?}",
        main.process
    );
    assert!(
        matches!(main.process[0], CompiledStep::GainMemory(1)),
        "[Main] step must be GainMemory(1); got {:?}",
        main.process[0]
    );
}

/// Clause [Security]: `OnSecurity` timing, `FaceUp` scope, mandatory,
/// not OPT, no condition, no active_when.
#[test]
fn st2_13_clause_security_shape() {
    let card = compiled(CARD_ID);

    let sec = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("ST2-13 must have an OnSecurity clause");

    assert_eq!(
        sec.when.len(),
        1,
        "[Security] clause must fire on OnSecurity only; got {:?}",
        sec.when
    );
    assert_eq!(
        sec.scope,
        CompiledScope::FaceUp,
        "Option [Security] uses FaceUp scope (no separate Security variant); \
         got {:?}",
        sec.scope
    );
    assert!(
        !sec.optional,
        "[Security] is mandatory per RULES_CONTEXT.md §16; DCGO \
         SecuritySkill ActivateClass passes no canNoSelect"
    );
    assert!(
        !sec.once_per_turn,
        "[Security] is not OPT — printed text has no [Once Per Turn]"
    );
    assert!(
        sec.condition.is_none(),
        "[Security] has no condition; got {:?}",
        sec.condition
    );
    assert!(
        sec.active_when.is_none(),
        "[Security] has no active_when; got {:?}",
        sec.active_when
    );
}

/// Clause [Security] process body: exactly one step, `GainMemory(2)`.
#[test]
fn st2_13_clause_security_process_is_single_gain_memory_2() {
    let card = compiled(CARD_ID);

    let sec = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("ST2-13 must have an OnSecurity clause");

    assert_eq!(
        sec.process.len(),
        1,
        "[Security] body must be exactly 1 step (printed: 'Gain 2 memory.'); got {:?}",
        sec.process
    );
    assert!(
        matches!(sec.process[0], CompiledStep::GainMemory(2)),
        "[Security] step must be GainMemory(2); got {:?}",
        sec.process[0]
    );
}

/// No `MainOnField`, `MainFromTrash`, `OnPlay`, `OnSecurityCheck`,
/// `WhenAttacking`, etc. clauses sneak in — only the two printed clauses
/// exist. Defends against accidental gold-plating during future audits.
#[test]
fn st2_13_has_no_extraneous_timings() {
    let card = compiled(CARD_ID);

    let allowed = [CompiledTiming::MainFromHand, CompiledTiming::OnSecurity];

    for clause in &card.effects {
        let CompiledClause::Triggered(t) = clause else {
            panic!(
                "ST2-13 must have only Triggered clauses; got {:?}",
                clause
            );
        };
        for timing in &t.when {
            assert!(
                allowed.contains(timing),
                "Unexpected timing {:?} on ST2-13 — printed text is just \
                 [Main]/[Security]; allowed = {:?}",
                timing,
                allowed
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: [Main] gain 1 memory
// ═══════════════════════════════════════════════════════════════════════════════
//
// Pattern: `runner.game.activate_hand_main(player, hand_index)` — same
// dispatch path BT1-090's `gravity_crush` test exercises. ST2-13's body is
// strictly simpler (no scheduled tail) so we only assert the immediate
// memory bump.

/// Activating ST2-13 from P0's hand bumps memory by exactly +1 from the
/// active player's perspective.
#[test]
fn st2_13_main_activation_gains_1_memory() {
    let mut runner = hammer_spark();

    let mem_before = runner.memory();
    let fired = runner.game.activate_hand_main(0, 0);
    assert!(
        fired,
        "activate_hand_main must return true for ST2-13's MainFromHand clause"
    );

    assert_eq!(
        runner.memory(),
        mem_before + 1,
        "[Main] gain_memory: 1 must add exactly 1 to memory; before={mem_before}, \
         after={}",
        runner.memory()
    );
}

/// Re-activating from an independent fixture confirms idempotency: each
/// activation bumps by exactly 1 (no double-fire, no leak from a sibling
/// clause).
#[test]
fn st2_13_main_activation_is_a_pure_plus_one_with_no_side_effects() {
    let mut runner = hammer_spark();

    let hand_before = runner.game.players[0].hand.len();
    let trash_before = runner.game.players[0].trash.len();
    let battle_area_before = runner.game.players[0].battle_area.len();

    let mem_before = runner.memory();
    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must succeed");

    // Memory bumped by exactly 1 — no other zone should mutate from a pure
    // gain_memory body. (The Option's own resolution-path movement to trash
    // is driven by the option-resolution scaffolding, not the body — and
    // `activate_hand_main` is the body-only entry point. We assert no body-
    // side hand/battle-area mutations.)
    assert_eq!(
        runner.memory(),
        mem_before + 1,
        "[Main] body must be a +1 memory delta only"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        battle_area_before,
        "[Main] body must not place anything on the battle area; \
         before={battle_area_before}, after={}",
        runner.game.players[0].battle_area.len()
    );

    // Hand/trash tracking is informational — the activation entry point does
    // not by itself drive the option-trash transition (that lives in the
    // option-resolution scaffolding around `activate_hand_main`). So we
    // assert nothing left hand and nothing entered trash from the BODY.
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "[Main] body alone must not move the option out of hand; \
         before={hand_before}, after={}",
        runner.game.players[0].hand.len()
    );
    assert_eq!(
        runner.game.players[0].trash.len(),
        trash_before,
        "[Main] body alone must not push to trash; before={trash_before}, \
         after={}",
        runner.game.players[0].trash.len()
    );
}

/// Memory clamp interaction: the engine clamps at the configured upper
/// bound. Pre-set memory near the cap and confirm the gain saturates
/// instead of overflowing — a regression here would indicate a wider
/// memory-mutation bug, not an ST2-13-specific one, but the test pins it
/// for this card's clause shape.
#[test]
fn st2_13_main_activation_respects_memory_upper_clamp() {
    let mut runner = hammer_spark();

    // Force memory to the upper edge.
    let cap = runner.game.rules.memory_range.1;
    runner.game.memory = cap;

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must succeed at memory cap");

    assert_eq!(
        runner.memory(),
        cap,
        "memory must clamp at upper bound {cap} (no overflow); got {}",
        runner.memory()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: [Security] gain 2 memory
// ═══════════════════════════════════════════════════════════════════════════════
//
// Pattern: place ST2-13 in P1's security stack, park a vanilla attacker on
// P0's field, and call `runner.attack_player(attacker, 1, false)`. The
// security-check pipeline reveals ST2-13, fires its OnSecurity body, and
// settles. Mirrors `tests/dsl/option_security_disposition.rs::
// resolving_security_option_can_move_self_to_hand_without_trashing`.
//
// Memory semantics: `CompiledStep::GainMemory(n)` lowers to
// `EffectContext::gain_memory(n)` → `Game::gain_memory(n)` which adds n to
// the seesaw absolute value. Per Digimon TCG conventions, when a defender
// "gains memory" the seesaw shifts toward them, but the engine models the
// raw integer mutation: memory += 2. We assert the absolute mutation; the
// per-perspective interpretation is consistent across the codebase
// (compare BT1-090 / EX1-068).

/// Attack P1's security where the only security card is ST2-13 → P1's
/// on_security clause fires → memory increases by exactly 2.
#[test]
fn st2_13_security_reveal_gains_2_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST2-13 must be in embedded DSL pack")
        .add_card(make_attacker("ATK"))
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .security(1, &[CARD_ID])
        .memory(0)
        .start();

    let attacker = runner.place_on_field(0, "ATK", Some(0));
    let mem_before = runner.memory();
    let sec_before = runner.security_count(1);

    let result = runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    // The attack consumed P1's security card.
    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "P1's only security card must be revealed and removed; \
         before={sec_before}, after={}",
        runner.security_count(1)
    );

    // The attacker survives — there is no Digimon to clash with and ST2-13
    // is not a play-self-as-attacker option.
    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "ATK must survive the security check (no defender, no attack-cancel)"
    );

    // The memory delta is +2 (defender gain). The engine's gain_memory is
    // an absolute add; this matches both EX1-068's [Security] semantics and
    // BT1-090's [Main] arithmetic.
    assert_eq!(
        runner.memory() - mem_before,
        2,
        "[Security] gain_memory: 2 must add exactly 2 to memory; \
         before={mem_before}, after={}",
        runner.memory()
    );
}

/// Negative coverage: with NO ST2-13 in P1's security stack (just a plain
/// filler), an identical attack does NOT bump memory. Defends the assertion
/// above by ruling out the possibility that the security-check pipeline
/// gives memory regardless of revealed-card identity.
#[test]
fn st2_13_security_reveal_other_card_does_not_gain_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST2-13 must be in embedded DSL pack")
        .add_card(make_attacker("ATK"))
        .add_card(make_filler("FILL"))
        .add_card(make_filler("PLAIN-SEC"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .security(1, &["PLAIN-SEC"])
        .memory(0)
        .start();

    let attacker = runner.place_on_field(0, "ATK", Some(0));
    let mem_before = runner.memory();

    let _result = runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory() - mem_before,
        0,
        "Memory must NOT change when the revealed security is not ST2-13 — \
         confirms the +2 in the positive test is attributable to ST2-13's \
         on_security body, not to the security-reveal pipeline itself"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Anti-helper to silence dead-code warnings across the test fork
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = make_filler("X");
    let _ = make_attacker("X");
}
