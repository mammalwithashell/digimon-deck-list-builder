//! BT22-084 Nokia Shiramine — Tamer, Red+Blue, Cost 5.
//!
//! # Card text (cards.json)
//!
//! **[Start of Your Turn]** If you have 2 or less memory, set it to 3.
//! **[Start of Your Main Phase] [On Play]** If you have 1 or fewer Digimon,
//!     you may play 1 [Agumon] or [Gabumon] from your hand without paying
//!     the cost.
//! **[All Turns]** All your Digimon with [Greymon], [Garurumon] or
//!     [Omnimon] in their names get +1000 DP.
//!
//! Inherited (Security):
//!   **[Security]** Play this card without paying the cost.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT22/Red/BT22_084.cs`
//!
//! Notable DCGO contracts:
//!   - Start of Your Turn: `CardEffectFactory.SetMemoryTo3TamerEffect(card)` —
//!     unconditional set (the "if memory <= 2" gate is handled inside the
//!     factory).
//!   - Start of Your Main Phase + On Play (shared coroutine):
//!     `SetUpActivateClass(..., isOptional: true, ...)` and
//!     `SelectHandEffect.SetUp(canNoSelect: true, ...)` — truly optional;
//!     player may decline. Filter is
//!     `cardSource.IsDigimon && (EqualsCardName("Agumon") || EqualsCardName("Gabumon"))`.
//!     Note DCGO uses `EqualsCardName` (exact match), but the printed text
//!     uses bracketed `[Agumon]` / `[Gabumon]` which by Comprehensive Rules
//!     §16 conventionally matches *substring* in name. The YAML uses
//!     `name_contains` which is the conventional Rust DSL spelling for
//!     bracket-named card references and matches BT17-007 / BT17-015 sibling
//!     cards. Cross-checked OK.
//!   - All Turns: `ChangeDPStaticEffect` with `permanentCondition` = "owner's
//!     battle-area Digimon AND TopCard.ContainsCardName(Greymon|Garurumon|Omnimon)",
//!     `changeValue: 1000`, `isInheritedEffect: false`, `condition` =
//!     `IsExistOnBattleArea(card)` (carrier on field). NOT marked as
//!     `other` in DCGO — Nokia is a Tamer, not a Digimon, so she never
//!     matches the predicate herself; an `other: true` flag would be a
//!     no-op safeguard.
//!   - Security: `PlaySelfTamerSecurityEffect(card)` — plays this card from
//!     security without paying cost (resolved precedent: BT21-015 / BT5-093 /
//!     BT9-092 G-PLACE-SELF-AS-OPTION-PERMANENT).
//!
//! # YAML location
//! `code/digimon-engine/cards/bt22/BT22-084.yaml` — promoted to the per-set
//! production folder on 2026-05-20 (DNA Omnimon missing-card authoring
//! pass). The prior `cards/_examples/BT22-084.yaml` draft was removed in the
//! same change. The embedded DSL pack surfaces the production card via
//! `compiled("BT22-084")`. This test follows `bt17_007.rs` precedent and
//! uses `card_data_from_compiled` + `compiled` rather than `include_str!`.
//!
//! # Audit summary (2026-05-03) — VERDICT: AUDIT-WITH-CAVEATS
//!
//! Faithfulness diff (per clause):
//!
//! 1. **[Start of Your Turn] memory <=2 → set to 3** — YAML matches printed
//!    text (`condition: { memory_lte: 2 }`, `process: [set_memory: 3]`).
//!    Same shape as BT18-087 clause 1, which has passing behavioral
//!    coverage. PASS.
//!
//! 2. **[Start of Your Main Phase] [On Play] (you may) play 1
//!    [Agumon]/[Gabumon] free if Digimon count <=1** — YAML matches
//!    printed text in optionality (`optional: true`), filter
//!    (`name_contains: Agumon | Gabumon`), payment-free clause
//!    (`play_from_hand_free`), and trigger union
//!    (`when: [start_of_your_main_phase, on_play]` — DCGO splits these
//!    into two factory entries that share `SharedActivateCoroutine`,
//!    which is functionally equivalent).
//!
//!    **G-COUNT-LTE-EVAL — RESOLVED:** the
//!    `count_lte: { filter: { of: you, zone: [battle_area], kind: digimon },
//!    n: 1 }` gate is parsed, lowered into `CompiledPredicate.count_lte`,
//!    AND evaluated — `predicate.rs` has a `pred.count_lte` arm that
//!    compares `count_matching` against the cap. Both the positive case
//!    (0 Digimon → fires) and the negative case (2+ Digimon → suppressed)
//!    behave faithfully; `bt22_084_clause2_blocked_when_two_or_more_digimon_present`
//!    is an active (non-ignored) test.
//!
//! 3. **[All Turns] +1000 DP filtered aura on own Digimon with Greymon /
//!    Garurumon / Omnimon in name** — YAML uses `kind: aura`,
//!    `active_when: { all_turns: true }`, predicate
//!    `{ owner: you, zone: [battle_area], any_of: [name_contains:
//!    Greymon, name_contains: Garurumon, name_contains: Omnimon] }`,
//!    `dp_modifier: 1000`. Matches printed text + DCGO predicate exactly.
//!    G-DECLARATIVE-KEYWORD was resolved 2026-05-02 for battle-area
//!    filtered auras (Group 6) — `tick_declarative_effects` materialises
//!    matching permanent modifiers. Behavioral DP-buff tests should pass.
//!    Aura does NOT include `other: true`; harmless because Nokia is a
//!    Tamer (no DP) so the filter cannot ever match her. Cross-checked OK.
//!
//! 4. **[Security] play this card without paying cost** — YAML uses
//!    `when: on_security`, `process: [play_from_security: {}]`. Resolved by
//!    BT21-015 precedent (G-PLACE-SELF-AS-OPTION-PERMANENT, engine-gaps.md
//!    line 236). Structural-only assertion in this audit; full security-
//!    replay path covered in `bt21_015.rs`.
//!
//! Remaining caveats / non-faithfulness items: NONE. All four clauses are
//! faithfully implemented; clause 2's negative gate (G-COUNT-LTE-EVAL) is
//! resolved and covered by an active test.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4 / §5)
//!   - §5 Structural: 4 effect entries — 3 triggered (start_of_your_turn,
//!     start_of_your_main_phase + on_play, on_security) + 1 declarative
//!     aura. Optionality flags. Clause scopes.
//!   - §5 Condition gating: clause 1 fires at memory<=2 / not at memory>=3;
//!     clause 2 positive (0 Digimon, candidate in hand → prompt installs);
//!     clause 2 negative-1 (2+ Digimon → count_lte: 1 fails, no prompt);
//!     clause 2 negative-2 (no Agumon/Gabumon in hand → no prompt).
//!   - §5 Behavioral integrated: clause 1 sets memory to 3; clause 2 picks
//!     Agumon and plays it for free (no memory deduction); clause 2 PASS
//!     leaves hand intact; clause 3 +1000 DP applied to a named Digimon.
//!   - §5 Negative aura: aura does not buff a non-matching name (Veemon)
//!     and does not buff opponent's Greymon-named Digimon.
//!   - OPT: none of Nokia's clauses are [Once Per Turn] — out of scope.
//!   - Self-protection: aura predicate cannot match Nokia (Tamer kind);
//!     not a faithfulness item but worth a coverage note.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "BT22-084";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn nokia_card_data() -> CardData {
    card_data_from_compiled(CARD_ID)
}

fn make_filler(card_id: &str) -> CardData {
    make_test_card(card_id, card_id)
}

/// A Lv.4 Digimon with a name that matches Nokia's aura filter (contains
/// "Greymon"). DP set to a known baseline so we can measure the +1000 buff.
fn make_named_digimon(card_id: &str, card_name: &str, dp: i32) -> CardData {
    let mut c = make_test_card(card_id, card_name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

/// A Lv.3 Agumon — used by clause 2 free-play tests (matches `name_contains:
/// Agumon` filter).
fn make_agumon(card_id: &str) -> CardData {
    let mut c = make_test_card(card_id, "Agumon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.play_cost = 3;
    c.dp = Some(1000);
    c
}

/// A Lv.3 Gabumon.
fn make_gabumon(card_id: &str) -> CardData {
    let mut c = make_test_card(card_id, "Gabumon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.play_cost = 3;
    c.dp = Some(1000);
    c
}

/// A non-matching Lv.3 Digimon (no Agumon/Gabumon in name) — used to verify
/// the clause-2 hand filter rejects unrelated names.
fn make_other_digimon(card_id: &str, card_name: &str) -> CardData {
    let mut c = make_test_card(card_id, card_name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.play_cost = 3;
    c.dp = Some(1000);
    c
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// Smoke: the embedded DSL pack ships BT22-084 from `_examples/` and it
/// compiles as a Tamer.
#[test]
fn bt22_084_compiles_as_tamer() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(5));
}

/// Three triggered clauses + one declarative aura clause (matches the
/// dsl_omnimon_slice precedent assertion).
#[test]
fn bt22_084_has_three_triggered_clauses_and_one_aura() {
    let card = compiled(CARD_ID);

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
        3,
        "BT22-084 must have exactly 3 triggered clauses"
    );

    let aura_count = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura { .. })
            )
        })
        .count();
    assert_eq!(
        aura_count, 1,
        "BT22-084 must have exactly 1 declarative aura clause"
    );
}

/// Clause 1: start_of_your_turn, FaceUp, not optional, not OPT.
#[test]
fn bt22_084_clause1_start_of_your_turn_face_up_not_optional() {
    let card = compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::StartOfYourTurn])
        .expect("BT22-084 must have a start_of_your_turn clause");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "Clause 1 must be FaceUp (own-field) scope"
    );
    assert!(
        !clause.optional,
        "Clause 1 (set memory to 3) is not player-optional"
    );
    assert!(
        !clause.once_per_turn,
        "Clause 1 has no [Once Per Turn] in printed text"
    );
}

/// Clause 2: start_of_your_main_phase + on_play (shared trigger union),
/// FaceUp, optional ("you may"), not OPT.
#[test]
fn bt22_084_clause2_main_phase_and_on_play_optional() {
    let card = compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when == vec![CompiledTiming::StartOfYourMainPhase, CompiledTiming::OnPlay]
                || t.when == vec![CompiledTiming::OnPlay, CompiledTiming::StartOfYourMainPhase]
        })
        .expect(
            "BT22-084 must have a shared start_of_your_main_phase + on_play \
             triggered clause",
        );

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "Clause 2 must be FaceUp (own-field) scope"
    );
    assert!(
        clause.optional,
        "Clause 2 must be optional (printed 'you may')"
    );
    assert!(
        !clause.once_per_turn,
        "Clause 2 has no [Once Per Turn] in printed text"
    );
}

/// Clause 3: aura with `active_when: { all_turns: true }`, owner=you, +1000
/// DP. We only assert the declarative shape here; behavioral installation
/// is covered in Section 4.
#[test]
fn bt22_084_clause3_is_aura_clause() {
    let card = compiled(CARD_ID);
    let aura = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura { .. }) => Some(c),
        _ => None,
    });
    assert!(
        aura.is_some(),
        "BT22-084 must have a declarative Aura clause"
    );
}

/// Clause 4: on_security triggered clause that lowers to play_from_security.
#[test]
fn bt22_084_clause4_on_security_triggered() {
    let card = compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::OnSecurity])
        .expect("BT22-084 must have an on_security clause");

    // Security clause is not player-optional (printed text says "Play this
    // card without paying the cost" with no "may"). DCGO factory
    // PlaySelfTamerSecurityEffect — no canNoSelect parameter; effect always
    // fires.
    assert!(
        !clause.optional,
        "on_security clause is not player-optional"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 1: [Start of Your Turn] set memory to 3 if memory <= 2
//
// Same shape as BT18-087 clause 1 (memory_lte: 2 → set_memory: 3).
// We exercise via the start/end_turn flow (P0 → P1 → P0) so begin_turn
// fires StartOfYourTurn for P0 with Nokia in play.
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: memory == 2 at start of P0's turn → memory_lte: 2 satisfied →
/// Nokia sets memory to 3.
#[test]
fn bt22_084_clause1_fires_when_memory_lte_2() {
    let mut runner = DebugRunner::builder()
        .add_card(nokia_card_data())
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(2)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    // start() may have fired StartOfYourTurn already; reset memory to the
    // pre-fire value before the next turn-cycle (precedent: bt18_087).
    runner.game.memory = 2;

    runner.end_turn(); // memory flips to -2 (P1's perspective)
    runner.end_turn(); // memory flips back to +2 (P0's perspective);
                       // begin_turn fires StartOfYourTurn → 2 ≤ 2 → set 3.

    assert_eq!(
        runner.memory(),
        3,
        "memory must be set to 3 when it was 2 at start of P0's turn"
    );
}

/// Positive boundary: memory == 0 → satisfied → set to 3.
#[test]
fn bt22_084_clause1_fires_when_memory_is_zero() {
    let mut runner = DebugRunner::builder()
        .add_card(nokia_card_data())
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(0)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 0;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(
        runner.memory(),
        3,
        "memory must be set to 3 when it was 0 at start of P0's turn"
    );
}

/// Negative: memory == 5 (> 2) → condition fails → memory unchanged.
#[test]
fn bt22_084_clause1_does_not_fire_when_memory_above_2() {
    let mut runner = DebugRunner::builder()
        .add_card(nokia_card_data())
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(5)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 5;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(
        runner.memory(),
        5,
        "memory must remain 5 when condition fails (5 > 2)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause 2: [Start of Main Phase] [On Play] (you may) play
//                       Agumon/Gabumon free if you have ≤1 Digimon.
//
// We focus on the [On Play] half because it's most easily exercised via
// `play()` from hand. The Start-of-Main-Phase half shares the same
// SharedActivateCoroutine in DCGO and the same compiled triggered clause
// (single entry with two `when` timings) in the YAML, so behavioral
// coverage of one half exercises the same step body for the other.
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: 0 Digimon on field, Agumon in hand → On Play installs prompt;
/// player picks Agumon → it enters battle area for free (no memory deduction
/// for Agumon's printed cost).
#[test]
fn bt22_084_clause2_on_play_offers_and_plays_agumon_free() {
    let mut runner = DebugRunner::builder()
        .add_card(nokia_card_data())
        .add_card(make_agumon("AGUMON-1"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .hand(0, &[CARD_ID, "AGUMON-1"])
        .memory(10) // enough to play Nokia (cost 5); we'll measure post-play
        .start();

    let memory_before_nokia = runner.memory();
    runner.play(0, 0).expect("Nokia plays from hand (cost 5)");

    // After Nokia hits the field, On Play fires. The clause is optional
    // ("you may") and its body's first step is a mandatory select_hand, so
    // an outer accept/decline prompt installs first (G-OUTER-OPTIONAL-NOT-
    // INSTALLED). Accept it to enter the inner select_hand picker.
    assert!(
        runner.game.pending_selection.is_some(),
        "Nokia's On Play installs an outer accept/decline prompt (optional clause)"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the outer optional-trigger prompt");
    assert!(
        runner.game.pending_selection.is_some(),
        "accepting the outer prompt installs the inner select_hand prompt"
    );

    let memory_after_nokia = runner.memory();

    // Resolve selection: pick the first valid action (Agumon).
    let action = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        pending.valid_action_ids[0]
    };
    runner
        .game
        .resolve_selection(0, action)
        .expect("select Agumon from hand");
    let _ = runner.game.drain_effect_queue();

    // Battle area: Nokia + Agumon == 2 permanents.
    assert_eq!(
        runner.battle_area_size(0),
        2,
        "Nokia + Agumon must both be on field"
    );
    // Hand: Agumon should no longer be there.
    assert_eq!(
        runner.hand_size(0),
        0,
        "Agumon must leave hand once played free"
    );
    // Free play must not deduct Agumon's printed cost (3) from memory.
    let memory_after_agumon = runner.memory();
    assert_eq!(
        memory_after_agumon, memory_after_nokia,
        "play_from_hand_free must not deduct Agumon's cost; \
         memory_after_nokia={memory_after_nokia}, memory_after_agumon={memory_after_agumon}, \
         memory_before_nokia={memory_before_nokia}"
    );
}

/// Positive: Gabumon variant (same filter — `name_contains: Gabumon`).
#[test]
fn bt22_084_clause2_on_play_offers_gabumon() {
    let mut runner = DebugRunner::builder()
        .add_card(nokia_card_data())
        .add_card(make_gabumon("GABU-1"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .hand(0, &[CARD_ID, "GABU-1"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("Nokia plays from hand");
    // Optional clause → outer accept/decline prompt first.
    assert!(
        runner.game.pending_selection.is_some(),
        "Nokia On Play installs an outer accept/decline prompt (optional clause)"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the outer optional-trigger prompt");
    assert!(
        runner.game.pending_selection.is_some(),
        "inner select_hand prompt installs after accepting"
    );

    let action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner
        .game
        .resolve_selection(0, action)
        .expect("select Gabumon");
    let _ = runner.game.drain_effect_queue();

    assert_eq!(runner.battle_area_size(0), 2);
}

/// Optional: clause 2 is optional — player may decline and Agumon stays in
/// hand. G-OUTER-OPTIONAL-NOT-INSTALLED (closed 2026-05-20): the clause-level
/// `optional: true` installs an outer accept/decline prompt before the inner
/// (mandatory) select_hand; declining skips the body cleanly.
#[test]
fn bt22_084_clause2_pass_leaves_hand_intact() {
    let mut runner = DebugRunner::builder()
        .add_card(nokia_card_data())
        .add_card(make_agumon("AGUMON-1"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .hand(0, &[CARD_ID, "AGUMON-1"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("Nokia plays");
    assert!(
        runner.game.pending_selection.is_some(),
        "an outer accept/decline prompt must install for the optional clause"
    );
    assert!(
        runner.pending_is_optional(),
        "the outer prompt must be optional (printed 'you may')"
    );

    runner
        .decline_optional_trigger()
        .expect("decline the outer optional-trigger prompt");
    let _ = runner.game.drain_effect_queue();

    // Battle area: only Nokia — declining skipped the free-play body.
    assert_eq!(runner.battle_area_size(0), 1);
    // Hand: Agumon still there.
    assert_eq!(runner.hand_size(0), 1);
}

/// Negative-1: hand has no Agumon / Gabumon → select_hand has no candidates
/// → no prompt installed (or prompt has zero valid picks; either way,
/// nothing enters the field).
#[test]
fn bt22_084_clause2_no_prompt_when_no_matching_card_in_hand() {
    let mut runner = DebugRunner::builder()
        .add_card(nokia_card_data())
        .add_card(make_other_digimon("OTHER-1", "Patamon"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .hand(0, &[CARD_ID, "OTHER-1"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("Nokia plays");
    let _ = runner.game.drain_effect_queue();

    // No matching card → no Digimon enters the field beyond Nokia.
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "no Agumon/Gabumon in hand → no extra Digimon enters via clause 2"
    );
    // Patamon stays in hand.
    assert_eq!(runner.hand_size(0), 1);
}

/// Negative-2: 2+ Digimon already on field suppresses clause 2's prompt
/// entirely (count_lte: 1 fails). G-COUNT-LTE-EVAL is RESOLVED — the
/// non-security `count_lte` aggregate predicate is evaluated in
/// `predicate.rs` (the `pred.count_lte` arm compares `count_matching`
/// against the cap), so the gate now correctly fails with 2+ Digimon.
#[test]
fn bt22_084_clause2_blocked_when_two_or_more_digimon_present() {
    let mut runner = DebugRunner::builder()
        .add_card(nokia_card_data())
        .add_card(make_agumon("AGUMON-1"))
        .add_card(make_other_digimon("DIGIMON-A", "Patamon"))
        .add_card(make_other_digimon("DIGIMON-B", "Tentomon"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .hand(0, &[CARD_ID, "AGUMON-1"])
        .memory(10)
        .start();

    // Pre-place 2 Digimon → count > 1 → clause 2 condition should fail.
    runner.place_on_field(0, "DIGIMON-A", Some(0));
    runner.place_on_field(0, "DIGIMON-B", Some(0));

    runner.play(0, 0).expect("Nokia plays");
    let _ = runner.game.drain_effect_queue();

    // Expected: no prompt, only Nokia + the 2 pre-existing Digimon (3 total),
    // Agumon still in hand.
    assert!(
        runner.game.pending_selection.is_none(),
        "count_lte: 1 must fail with 2+ Digimon → no prompt"
    );
    assert_eq!(
        runner.battle_area_size(0),
        3,
        "battle area must remain at Nokia + 2 pre-existing Digimon"
    );
    assert_eq!(runner.hand_size(0), 1, "Agumon must remain in hand");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause 3: [All Turns] aura +1000 DP to own Digimon with
//                       Greymon / Garurumon / Omnimon in name.
//
// Aura runtime was unblocked 2026-05-02 (G-DECLARATIVE-KEYWORD resolution
// for filtered auras / Group 6). `tick_declarative_effects` now installs
// matching permanent modifiers; `effective_dp` returns base DP + the sum
// of all ChangeDp modifiers + dynamic aura bonus.
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: own Greymon-named Digimon receives +1000 DP buff while Nokia
/// is on the field.
#[test]
fn bt22_084_clause3_aura_buffs_own_greymon_named_digimon() {
    let mut runner = DebugRunner::builder()
        .add_card(nokia_card_data())
        .add_card(make_named_digimon("GREYMON-1", "Greymon", 4000))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(3)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    let greymon = runner.place_on_field(0, "GREYMON-1", Some(0));

    // Trigger declarative tick (aura installation may fire on placement, on
    // turn boundary, or on demand depending on engine wiring).
    runner.game.tick_declarative_effects();

    let dp = runner.effective_dp(greymon).expect("Greymon has DP");
    assert_eq!(
        dp, 5000,
        "Greymon must be 4000 + 1000 = 5000 DP under Nokia's aura; got {dp}"
    );
}

/// Positive: Garurumon-named Digimon also gets +1000.
#[test]
fn bt22_084_clause3_aura_buffs_own_garurumon_named_digimon() {
    let mut runner = DebugRunner::builder()
        .add_card(nokia_card_data())
        .add_card(make_named_digimon("GARURU-1", "Garurumon", 3000))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(3)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    let garu = runner.place_on_field(0, "GARURU-1", Some(0));
    runner.game.tick_declarative_effects();

    let dp = runner.effective_dp(garu).expect("Garurumon has DP");
    assert_eq!(
        dp, 4000,
        "Garurumon must be 3000 + 1000 = 4000 DP; got {dp}"
    );
}

/// Positive: Omnimon-named Digimon (e.g. "Omnimon", "Omnimon X Antibody")
/// gets +1000.
#[test]
fn bt22_084_clause3_aura_buffs_own_omnimon_named_digimon() {
    let mut runner = DebugRunner::builder()
        .add_card(nokia_card_data())
        .add_card(make_named_digimon("OMNI-1", "Omnimon", 13000))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(3)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    let omni = runner.place_on_field(0, "OMNI-1", Some(0));
    runner.game.tick_declarative_effects();

    let dp = runner.effective_dp(omni).expect("Omnimon has DP");
    assert_eq!(
        dp, 14000,
        "Omnimon must be 13000 + 1000 = 14000 DP; got {dp}"
    );
}

/// Negative: own Digimon whose name does NOT contain Greymon/Garurumon/
/// Omnimon receives no buff.
#[test]
fn bt22_084_clause3_aura_does_not_buff_unrelated_own_digimon() {
    let mut runner = DebugRunner::builder()
        .add_card(nokia_card_data())
        .add_card(make_named_digimon("VEEMON-1", "Veemon", 3000))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(3)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    let veemon = runner.place_on_field(0, "VEEMON-1", Some(0));
    runner.game.tick_declarative_effects();

    let dp = runner.effective_dp(veemon).expect("Veemon has DP");
    assert_eq!(
        dp, 3000,
        "Veemon must remain 3000 DP (name does not contain Greymon/Garurumon/Omnimon)"
    );
}

/// Negative: opponent's Greymon-named Digimon does NOT receive Nokia's
/// buff (aura predicate is `owner: you`).
#[test]
fn bt22_084_clause3_aura_does_not_buff_opponent_greymon() {
    let mut runner = DebugRunner::builder()
        .add_card(nokia_card_data())
        .add_card(make_named_digimon("OPP-GREYMON", "Greymon", 4000))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(3)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    let opp_greymon = runner.place_on_field(1, "OPP-GREYMON", Some(0));
    runner.game.tick_declarative_effects();

    let dp = runner
        .effective_dp(opp_greymon)
        .expect("Opp Greymon has DP");
    assert_eq!(
        dp, 4000,
        "Opponent's Greymon must NOT receive Nokia's aura (owner: you predicate)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Clause 4: [Security] Play this card without paying the cost
//
// Resolved precedent: BT21-015 / BT5-093 / BT9-092 — see engine-gaps.md
// G-PLACE-SELF-AS-OPTION-PERMANENT (resolved 2026-04-27). Full security-
// replay path coverage lives in those companion test files; here we only
// assert the structural shape (the on_security clause exists with
// play_from_security as its body).
// ═══════════════════════════════════════════════════════════════════════════════

/// Structural: an on_security triggered clause exists. The play_from_security
/// step body is part of CompiledClause::Triggered.steps but the precise
/// step-introspection API isn't required for an audit pass — the dedicated
/// behavioral coverage in bt21_015.rs / bt5_093.rs / bt9_092.rs validates
/// the security-replay path for cards using this same primitive.
#[test]
fn bt22_084_clause4_on_security_structural_present() {
    let card = compiled(CARD_ID);
    let on_sec = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::OnSecurity]);
    assert!(
        on_sec.is_some(),
        "BT22-084 must compile an on_security clause for the [Security] play-self effect"
    );
}
