//! BT1-085 Tai Kamiya — Tamer, Red, Cost 4.
//!
//! # Card text (cards.json — printed)
//!
//! ```text
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [Your Turn] All of your red Digimon with 4 or more digivolution cards gain
//!   ＜Security A. +1＞ (This Digimon checks 1 additional security card.)
//! ```
//!
//! Inherited (Security): [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT1/Red/BT1_085.cs`
//!
//! Clause-by-clause DCGO walkthrough (BT1_085.cs):
//! - **Clause 1** — `EffectTiming.OnStartTurn` → `SetMemoryTo3TamerEffect`. The
//!   canonical "[Start of Your Turn] if memory <= 2, set it to 3" ramp idiom.
//! - **Clause 2** — `EffectTiming.None` passive declarative via
//!   `ChangeSAttackStaticEffect(changeValue: 1, isInheritedEffect: false)`.
//!   `condition` = `IsOwnerTurn(card)`; `permanentCondition` = own battle-area
//!   Digimon whose `TopCard.CardColors.Contains(Red)` AND
//!   `DigivolutionCards.Count >= 4`. This is a FLAT +1 delta (the count is a
//!   FILTER, not a formula value), so it SUMS onto the strike — distinct from
//!   WarGreymon's `ChangeSelfSAttack` whose changeValue IS a formula.
//! - **Clause 3 (Security)** — `EffectTiming.SecuritySkill` →
//!   `PlaySelfTamerSecurityEffect(card)`.
//!
//! # YAML clause layout
//!
//! | # | Clause                                            | Kind / when                | Shape |
//! |---|---------------------------------------------------|----------------------------|-------|
//! | 1 | [Start of Your Turn] if mem<=2 set to 3           | triggered, StartOfYourTurn | condition: { memory_lte: 2 }; process: [set_memory: 3] |
//! | 2 | [Your Turn] own red 4+source Digimon <Sec A. +1>  | declarative aura           | active_when: { your_turn: true }; target: { owner: you, kind: digimon, color_is: red, stack_size_gte: 5 }; grant_keyword: SecurityAttackPlus(1) |
//! | 3 | [Security] Play self without paying                | triggered, OnSecurity      | process: [play_from_security] |
//!
//! # The headline interaction (the reason this card was authored)
//!
//! BT1-085's flat `<Security A. +1>` delta SUMS with WarGreymon's (ST1-11)
//! base-inclusive `security_attack_fn` formula — they never collide on the
//! formula-aura MAX path. A 4-source WarGreymon + BT1-085 checks 4
//! (base 1 + formula floor(4/2)=2 + Tai +1); add Greymon's inherited
//! `<Security A. +1>` and it checks 5. Per DCGO `Permanent.Strike_AllowMinus`
//! (Strike = 1, then `Strike += changeValue` for each effect).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT1-085";

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// A plain Red Digimon (make_test_card already defaults to Red/Digimon).
fn make_red_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.colors = vec![CardColor::Red];
    c
}

/// A plain Blue Digimon — used to prove the `color_is: red` filter rejects it.
fn make_blue_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_red_digimon(id, level, dp);
    c.colors = vec![CardColor::Blue];
    c
}

/// Low-DP filler for the opponent's security stack so multi-strike checks have
/// cards to pop without decking the opponent out mid-test.
fn make_weak_opp(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(1000);
    c
}

/// Standard fixture: BT1-085 from the embedded pack + a handful of synthetic
/// Digimon for the aura filter tests.
fn tai_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT1-085 must load from the embedded DSL pack")
        .add_card(make_red_digimon("RED-D", 6, 12000))
        .add_card(make_blue_digimon("BLUE-D", 6, 12000))
        .memory(0)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1 — Structural
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt1_085_compiles() {
    let runner = tai_runner();
    assert!(
        runner.compiled_card(CARD_ID).is_some(),
        "BT1-085 must compile from the embedded DSL pack"
    );
}

#[test]
fn bt1_085_metadata_is_red_tamer_cost_four() {
    let runner = tai_runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    assert_eq!(card.cost, Some(4), "BT1-085 prints Cost 4");
    assert!(
        matches!(card.kind, digimon_dsl::compiled::CompiledCardKind::Tamer),
        "BT1-085 must be a Tamer"
    );
    assert_eq!(card.level, None, "Tamers carry no printed level");
    assert_eq!(card.dp, None, "Tamers carry no printed DP");
    assert!(
        card.color
            .iter()
            .any(|c| matches!(c, digimon_dsl::compiled::CompiledColor::Red)),
        "BT1-085 must include Red color"
    );
}

#[test]
fn bt1_085_has_exactly_three_clauses() {
    let runner = tai_runner();
    let card = runner.compiled_card(CARD_ID).unwrap();
    assert_eq!(
        card.effects.len(),
        3,
        "BT1-085 must have 3 clauses (SoT memory + aura + security); got {:?}",
        card.effects
    );
}

#[test]
fn bt1_085_clause2_aura_grants_security_attack_plus_one() {
    let runner = tai_runner();
    let card = runner.compiled_card(CARD_ID).unwrap();

    let (scope, active_when, target, grant_keyword) = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope,
                active_when,
                target,
                grant_keyword,
                ..
            }) => Some((scope, active_when, target, grant_keyword)),
            _ => None,
        })
        .expect("BT1-085 must have a Declarative::Aura clause");

    assert_eq!(*scope, CompiledScope::FaceUp, "aura must be FaceUp scope");
    assert!(
        active_when.is_some(),
        "aura must declare active_when (your_turn: true)"
    );
    assert_ne!(
        *target,
        digimon_dsl::compiled::CompiledPredicate::default(),
        "aura target must be filtered (red + 4-or-more digivolution cards)"
    );
    let gk = grant_keyword.as_ref().expect("aura must carry grant_keyword");
    assert_eq!(gk.keyword, "SecurityAttackPlus", "grants SecurityAttackPlus");
    assert_eq!(gk.value, Some(1), "<Security A. +1> — value must be 1");
}

#[test]
fn bt1_085_clause3_on_security_plays_self_without_cost() {
    let runner = tai_runner();
    let card = runner.compiled_card(CARD_ID).unwrap();

    let security = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("on_security clause must exist");

    assert!(
        !security.optional,
        "[Security] play_from_security is mandatory per RULES_CONTEXT.md §16"
    );
    assert!(
        security
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlayFromSecurity)),
        "on_security clause must include play_from_security; got {:?}",
        security.process
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2 — Clause 1: [Start of Your Turn] memory ramp (mem <= 2 -> 3)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt1_085_start_of_turn_sets_memory_to_three_when_two_or_less() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT1-085 loads")
        .memory(2)
        .start();
    let tai = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::StartOfYourTurn, TriggerSource::Permanent(tai));
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.memory(),
        3,
        "memory 2 (<= 2) must be set to 3; got {}",
        runner.memory()
    );
}

#[test]
fn bt1_085_start_of_turn_raises_zero_memory_to_three() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT1-085 loads")
        .memory(0)
        .start();
    let tai = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::StartOfYourTurn, TriggerSource::Permanent(tai));
    runner.game.drain_effect_queue();

    assert_eq!(runner.memory(), 3, "memory 0 (<= 2) must be set to 3");
}

#[test]
fn bt1_085_start_of_turn_does_not_change_memory_above_two() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT1-085 loads")
        .memory(4)
        .start();
    let tai = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::StartOfYourTurn, TriggerSource::Permanent(tai));
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.memory(),
        4,
        "memory 4 (> 2) fails the memory_lte:2 gate — must stay 4, not drop to 3"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 3 — Clause 2: [Your Turn] filtered <Security A. +1> aura
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt1_085_grants_security_attack_plus_to_red_digimon_with_four_sources() {
    let mut runner = tai_runner();
    runner.game.turn_count = 1; // your turn
    let _tai = runner.place_on_field(0, CARD_ID, Some(1));
    // 5-card red stack = 4 digivolution cards.
    let body = runner.place_stack(0, &["RED-D", "RED-D", "RED-D", "RED-D", "RED-D"]);

    runner.game.tick_declarative_effects();

    assert!(
        runner
            .game
            .modifiers
            .has_keyword(body, Keyword::SecurityAttackPlus(1)),
        "a red Digimon with 4 digivolution cards must receive SecurityAttackPlus(1)"
    );
}

#[test]
fn bt1_085_does_not_grant_to_red_digimon_with_three_sources() {
    let mut runner = tai_runner();
    runner.game.turn_count = 1;
    let _tai = runner.place_on_field(0, CARD_ID, Some(1));
    // 4-card red stack = only 3 digivolution cards (< 4).
    let body = runner.place_stack(0, &["RED-D", "RED-D", "RED-D", "RED-D"]);

    runner.game.tick_declarative_effects();

    assert!(
        !runner
            .game
            .modifiers
            .has_keyword(body, Keyword::SecurityAttackPlus(1)),
        "3 digivolution cards fails the stack_size_gte:5 (4+ sources) filter"
    );
}

#[test]
fn bt1_085_does_not_grant_to_non_red_digimon_with_four_sources() {
    let mut runner = tai_runner();
    runner.game.turn_count = 1;
    let _tai = runner.place_on_field(0, CARD_ID, Some(1));
    // Blue top card with 4 sources — fails the color_is:red filter.
    let body = runner.place_stack(0, &["BLUE-D", "BLUE-D", "BLUE-D", "BLUE-D", "BLUE-D"]);

    runner.game.tick_declarative_effects();

    assert!(
        !runner
            .game
            .modifiers
            .has_keyword(body, Keyword::SecurityAttackPlus(1)),
        "a non-red Digimon must NOT receive the grant (color_is:red filter)"
    );
}

#[test]
fn bt1_085_does_not_grant_on_opponents_turn() {
    let mut runner = tai_runner();
    runner.game.turn_count = 1;
    let _tai = runner.place_on_field(0, CARD_ID, Some(1));
    let body = runner.place_stack(0, &["RED-D", "RED-D", "RED-D", "RED-D", "RED-D"]);

    // Sanity: granted on your turn.
    runner.game.tick_declarative_effects();
    assert!(
        runner
            .game
            .modifiers
            .has_keyword(body, Keyword::SecurityAttackPlus(1)),
        "sanity: granted on your turn"
    );

    // Flip to the opponent's turn; the active_when:{your_turn:true} gate clears it.
    runner.end_turn();
    runner.game.tick_declarative_effects();
    assert!(
        !runner
            .game
            .modifiers
            .has_keyword(body, Keyword::SecurityAttackPlus(1)),
        "on the opponent's turn the your-turn gate must clear the grant"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 4 — Headline interaction: flat delta SUMS with WarGreymon's formula
// ═══════════════════════════════════════════════════════════════════════════════

/// 4-source WarGreymon (Greymon-free) + BT1-085 Tai checks **4** security:
/// base 1 + WarGreymon base-inclusive formula floor(4/2)=2 + Tai's flat +1.
/// Tai's grant routes through the SecurityAttackChange/keyword (summed) path,
/// NOT the formula-aura MAX path — so it adds, never collides.
#[test]
fn bt1_085_sums_additively_with_wargreymon_formula_to_check_four() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST1-01")
        .expect("ST1-01 (Koromon)")
        .dsl_card("ST1-03")
        .expect("ST1-03 (Agumon)")
        .dsl_card("ST1-05")
        .expect("ST1-05 (Birdramon)")
        .dsl_card("ST1-09")
        .expect("ST1-09 (MetalGreymon)")
        .dsl_card("ST1-11")
        .expect("ST1-11 (WarGreymon)")
        .dsl_card(CARD_ID)
        .expect("BT1-085 (Tai Kamiya)")
        .add_card(make_weak_opp("SEC-A", "SecA"))
        .add_card(make_weak_opp("SEC-B", "SecB"))
        .add_card(make_weak_opp("SEC-C", "SecC"))
        .add_card(make_weak_opp("SEC-D", "SecD"))
        .add_card(make_weak_opp("SEC-E", "SecE"))
        .add_card(make_weak_opp("SEC-F", "SecF"))
        .memory(3)
        .security(1, &["SEC-A", "SEC-B", "SEC-C", "SEC-D", "SEC-E", "SEC-F"])
        .start();
    runner.game.turn_count = 1;

    // 4 red sources [Koromon, Agumon, Birdramon, MetalGreymon] under WarGreymon.
    let stack = runner.place_stack(0, &["ST1-01", "ST1-03", "ST1-05", "ST1-09", "ST1-11"]);
    let _tai = runner.place_on_field(0, CARD_ID, Some(1));

    let before = runner.game.player(1).security.len();
    runner.attack_player(stack, 1, false);
    let _ = runner.auto_resolve();
    let after = runner.game.player(1).security.len();

    assert!(!runner.game_over(), "6 security vs a 4-check must not deck out");
    assert_eq!(
        before - after,
        4,
        "WarGreymon (4 red sources) + BT1-085 checks 4: base 1 + formula floor(4/2)=2 \
         + Tai's flat <Security A. +1>. Tai's grant is a summed delta, not a formula — \
         it adds to the formula's base-inclusive value, it does not MAX against it."
    );
}

/// Full ST-1 line (4 sources incl Greymon) + BT1-085 checks **5**:
/// base 1 + WarGreymon formula 2 + Greymon inherited +1 + Tai flat +1. Proves
/// three independent <Security A.> sources (formula + inherited keyword +
/// aura-granted keyword) all single-count and sum, matching DCGO's
/// `Strike_AllowMinus` (Strike=1 then += each).
#[test]
fn bt1_085_sums_with_wargreymon_and_greymon_to_check_five() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST1-01")
        .expect("ST1-01 (Koromon)")
        .dsl_card("ST1-03")
        .expect("ST1-03 (Agumon)")
        .dsl_card("ST1-07")
        .expect("ST1-07 (Greymon)")
        .dsl_card("ST1-09")
        .expect("ST1-09 (MetalGreymon)")
        .dsl_card("ST1-11")
        .expect("ST1-11 (WarGreymon)")
        .dsl_card(CARD_ID)
        .expect("BT1-085 (Tai Kamiya)")
        .add_card(make_weak_opp("SEC-A", "SecA"))
        .add_card(make_weak_opp("SEC-B", "SecB"))
        .add_card(make_weak_opp("SEC-C", "SecC"))
        .add_card(make_weak_opp("SEC-D", "SecD"))
        .add_card(make_weak_opp("SEC-E", "SecE"))
        .add_card(make_weak_opp("SEC-F", "SecF"))
        .add_card(make_weak_opp("SEC-G", "SecG"))
        .memory(3)
        .security(1, &["SEC-A", "SEC-B", "SEC-C", "SEC-D", "SEC-E", "SEC-F", "SEC-G"])
        .start();
    runner.game.turn_count = 1;

    // Full line: 4 sources [Koromon, Agumon, Greymon, MetalGreymon] under WarGreymon.
    let stack = runner.place_stack(0, &["ST1-01", "ST1-03", "ST1-07", "ST1-09", "ST1-11"]);
    let _tai = runner.place_on_field(0, CARD_ID, Some(1));

    let before = runner.game.player(1).security.len();
    runner.attack_player(stack, 1, false);
    let _ = runner.auto_resolve();
    let after = runner.game.player(1).security.len();

    assert!(!runner.game_over(), "7 security vs a 5-check must not deck out");
    assert_eq!(
        before - after,
        5,
        "full ST-1 line + BT1-085 checks 5: base 1 + WarGreymon formula 2 + Greymon \
         inherited <Security A. +1> + Tai aura <Security A. +1>. All three sources \
         single-count and sum."
    );
}
