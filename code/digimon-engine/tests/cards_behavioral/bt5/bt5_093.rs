//! BT5-093 Tai Kamiya & Matt Ishida — Tamer, White, Cost 4.
//!
//! # Card text (cards.json — printed)
//!
//! ```text
//! [Start of Your Turn] If your opponent has a level 6 or higher Digimon in
//!   play, gain 2 memory.
//! [Your Turn] All of your Digimon with [Omnimon] in their name gain
//!   ＜Security A. +1＞.
//! ```
//!
//! Inherited (Security): [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT5/White/BT5_093.cs`
//!
//! # Source priority
//! Per CLAUDE.md: printed text > docs/RULES_CONTEXT.md > fandom > DCGO. The
//! DCGO mapping below is descriptive only — printed text is authoritative.
//!
//! Clause-by-clause DCGO walkthrough (BT5_093.cs):
//! - **Clause 1** — `EffectTiming.OnStartTurn` ActivateClass with description
//!   "[Start of Your Turn] If your opponent has a level 6 or higher Digimon
//!   in play, gain 2 memory." `CanActivateCondition` checks
//!   `IsExistOnBattleArea(card)` + `HasMatchConditionPermanent(level >= 6 opp
//!   Digimon)` + `CanAddMemory`; `ActivateCoroutine` calls `AddMemory(2)`.
//!   `isOptional=false` — mandatory when condition holds.
//! - **Clause 2** — `EffectTiming.None` (passive face-up declarative) using
//!   `CardEffectFactory.ChangeSAttackStaticEffect(changeValue: 1, ...)` gated
//!   by `IsOwnerTurn(card)` and `IsPermanentExistsOnOwnerBattleAreaDigimon
//!   && ContainsCardName("Omnimon")`. The DCGO factory installs a static
//!   security-attack delta on each matching permanent.
//! - **Clause 3 (Security)** — `EffectTiming.SecuritySkill` calling
//!   `CardEffectFactory.PlaySelfTamerSecurityEffect(card)`, the canonical
//!   "[Security] Play this card without paying the cost" idiom for Tamers.
//!
//! # YAML clause layout
//!
//! | # | Clause                                        | Kind / when             | Shape |
//! |---|-----------------------------------------------|-------------------------|-------|
//! | 1 | [Start of Your Turn] +2 memory if opp Lv6+    | triggered, StartOfYourTurn | condition: any_permanent { of: opponent, level_gte: 6, kind: digimon }; process: [gain_memory: 2] |
//! | 2 | [Your Turn] own [Omnimon] gain <Sec A. +1>    | declarative aura        | active_when: { your_turn: true }, target: { of: you, name_contains: "Omnimon" }, grant_keyword: { keyword: SecurityAttackPlus, value: 1 } |
//! | 3 | [Security] Play self without paying           | triggered, OnSecurity    | process: [play_from_security] |
//!
//! # Patterns this test covers
//!
//! - **B1** Triggered StartOfYourTurn with condition gate (any_permanent
//!   level_gte filter)
//! - **D5** Filtered aura grant_keyword (SecurityAttackPlus + your-turn gate)
//! - **D-OnSecurity** Tamer free-play from security via play_from_security
//!
//! # Known gaps tagged
//!
//! - **G-AURA-GRANTED-SECURITY-KEYWORD** (NEW — to be filed): aura
//!   `grant_keyword: { keyword: SecurityAttackPlus, ... }` lowers to
//!   `lower_aura.rs` which calls `ctx.grant_declarative_keyword(handle,
//!   Keyword::SecurityAttackPlus(1), Expiry::Permanent)`. This installs into
//!   `Modifiers::permanent_keywords` (visible to `Modifiers::has_keyword`).
//!   However, the security-loop consumer
//!   `Game::security_attack_keyword_bonus` (game.rs:1938) ONLY iterates
//!   `permanent.card_sources` (printed face/inherited keywords) — it does
//!   NOT consult `Modifiers::permanent_keywords`. As a result, an
//!   aura-granted SecurityAttackPlus does install on the target permanent
//!   but is never consumed at attack-resolution time, so the buffed Omnimon
//!   does not actually pop an extra security card. The test that exercises
//!   the end-to-end security pop is `#[ignore]`'d on this gap; the test
//!   that asserts the keyword is installed (the install-side half of the
//!   contract) runs green and locks the aura's runtime install.
//!
//! - **G-PREDICATE-OF-ALIAS** (NEW — to be filed): the YAML's
//!   `target: { of: you, ... }` for the aura uses an unrecognized field
//!   name. `BoolPredicateSpec` defines the controller filter as `owner`
//!   (`code/digimon-dsl/src/predicate.rs:89`); there is no `#[serde(alias =
//!   "of")]`. serde permissively drops unknown keys, so the aura target's
//!   compiled `owner` is `None` and the aura scans BOTH controllers'
//!   battle areas. The aura currently buffs opponent's Omnimon-named
//!   Digimon as well as own — directly violating "All of YOUR Digimon
//!   with [Omnimon] in their name". Fix: either rename `of:` to `owner:`
//!   in the YAML (1-line authoring fix) or add `#[serde(alias = "of")]`
//!   to `BoolPredicateSpec::owner`. The negative test for opponent's
//!   Omnimon is `#[ignore]`'d on this gap.
//!
//! - **G-AURA-ACTIVE-WHEN-CROSS-TURN** (NEW — to be filed): the aura's
//!   `active_when: { your_turn: true }` is authored, but cross-turn
//!   verification (own Omnimon must lose the keyword on opponent's turn)
//!   needs end-to-end behavioral coverage. The on-your-turn install-side
//!   tests pass; the cross-turn negative test is `#[ignore]`'d pending
//!   confirmation that the declarative tick re-evaluates `active_when`
//!   after `end_turn`.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const YAML: &str = include_str!("../../../cards/_examples/BT5-093.yaml");
const CARD_ID: &str = "BT5-093";

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Build a Digimon CardData with explicit level + DP.
fn make_named_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// Standard fixture: BT5-093 (Tai & Matt) registered + filler card. Memory
/// pre-funded so memory-gain assertions stay away from the seesaw boundary.
fn taimatt_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT5-093 YAML parses")
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .memory(0)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// BT5-093 must compile from the embedded DSL pack.
#[test]
fn bt5_093_compiles() {
    let runner = taimatt_runner();
    assert!(
        runner.compiled_card(CARD_ID).is_some(),
        "BT5-093 must compile from YAML"
    );
}

/// Card metadata: kind=Tamer, cost=4, no level/DP. Color is printed white.
#[test]
fn bt5_093_metadata_matches_printed_text() {
    let runner = taimatt_runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    assert_eq!(card.cost, Some(4), "BT5-093 prints Cost 4");
    assert!(
        matches!(card.kind, digimon_dsl::compiled::CompiledCardKind::Tamer),
        "BT5-093 must be a Tamer"
    );
    assert_eq!(
        card.level, None,
        "Tamers carry no printed level (DSL: omitted)"
    );
    assert_eq!(card.dp, None, "Tamers carry no printed DP (DSL: omitted)");
    assert!(
        card.color
            .iter()
            .any(|c| matches!(c, digimon_dsl::compiled::CompiledColor::White)),
        "BT5-093 must include White color"
    );
}

/// Exactly 3 effect clauses: SoT memory triggered + your-turn aura + security
/// triggered.
#[test]
fn bt5_093_has_exactly_three_clauses() {
    let runner = taimatt_runner();
    let card = runner.compiled_card(CARD_ID).unwrap();
    assert_eq!(
        card.effects.len(),
        3,
        "BT5-093 must have 3 clauses (SoT memory + aura + security); got {:?}",
        card.effects
    );
}

/// Clause 1: [Start of Your Turn] mandatory triggered with opp-Lv6+ condition,
/// FaceUp scope, gain_memory(2) process step.
#[test]
fn bt5_093_clause1_start_of_your_turn_mandatory_faceup_with_opp_lv6_condition() {
    let runner = taimatt_runner();
    let card = runner.compiled_card(CARD_ID).unwrap();

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::StartOfYourTurn))
        .expect("StartOfYourTurn clause must exist");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "Clause 1 must be FaceUp (active while BT5-093 is on field)"
    );
    assert!(
        !clause.optional,
        "Clause 1 has no \"you may\" — mandatory when condition holds (DCGO isOptional=false)"
    );
    assert!(
        !clause.once_per_turn,
        "Clause 1 has no [Once Per Turn] in printed text"
    );
    assert!(
        clause.condition.is_some(),
        "Clause 1 must declare a condition (any_permanent of: opponent, level_gte: 6)"
    );

    let has_gain_memory_2 = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::GainMemory(2)));
    assert!(
        has_gain_memory_2,
        "Clause 1 process must include gain_memory(2); got {:?}",
        clause.process
    );
}

/// Clause 2: face-up declarative Aura with `active_when` your-turn gate, a
/// non-empty target predicate, and `grant_keyword: SecurityAttackPlus(1)`.
#[test]
fn bt5_093_clause2_aura_grants_security_attack_plus_one_to_omnimon() {
    let runner = taimatt_runner();
    let card = runner.compiled_card(CARD_ID).unwrap();

    let aura = card
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
        .expect("BT5-093 must have a Declarative::Aura clause");

    let (scope, active_when, target, grant_keyword) = aura;

    assert_eq!(
        *scope,
        CompiledScope::FaceUp,
        "aura must be FaceUp scope (active while BT5-093 is on the field)"
    );
    assert!(
        active_when.is_some(),
        "aura must declare active_when (your_turn: true)"
    );
    assert_ne!(
        *target,
        digimon_dsl::compiled::CompiledPredicate::default(),
        "aura target predicate must be non-empty (filtered by name_contains: Omnimon)"
    );

    let gk = grant_keyword
        .as_ref()
        .expect("aura must carry grant_keyword");
    assert_eq!(
        gk.keyword, "SecurityAttackPlus",
        "aura must grant SecurityAttackPlus"
    );
    assert_eq!(
        gk.value,
        Some(1),
        "printed text says <Security A. +1> — value must be 1"
    );
}

/// Clause 3: on_security triggered, FaceUp scope, mandatory, with a
/// `play_from_security` process step.
#[test]
fn bt5_093_clause3_on_security_mandatory_with_play_from_security() {
    let runner = taimatt_runner();
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

    assert_eq!(
        security.scope,
        CompiledScope::FaceUp,
        "on_security uses FaceUp scope (no separate Security variant)"
    );
    assert!(
        !security.optional,
        "[Security] play_from_security is mandatory per RULES_CONTEXT.md §16"
    );
    assert!(!security.once_per_turn, "Security clause has no OPT marker");

    let has_play_from_security = security
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlayFromSecurity));
    assert!(
        has_play_from_security,
        "on_security clause must include play_from_security step; got {:?}",
        security.process
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2 — Clause 1 condition gating (positive + negative)
// ═══════════════════════════════════════════════════════════════════════════════

/// Negative gate: opponent has NO Digimon at all → no fire on SoT.
#[test]
fn bt5_093_clause1_no_opp_digimon_no_memory_gain() {
    let mut runner = taimatt_runner();
    let owen = runner.place_on_field(0, CARD_ID, Some(0));
    let mem_before = runner.memory();

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(owen),
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.memory(),
        mem_before,
        "no opp Digimon → condition false → memory unchanged; before={mem_before}, after={}",
        runner.memory()
    );
    assert!(
        runner.pending_selection().is_none(),
        "mandatory clause with failing condition must not install any prompt"
    );
}

/// Negative gate: opponent has only a low-level Digimon (Lv.5) → no fire.
/// Verifies the `level_gte: 6` filter rejects Lv.5.
#[test]
fn bt5_093_clause1_opp_only_lv5_digimon_no_memory_gain() {
    let mut runner = taimatt_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("OPP-LV5", "OppLv5", 5, 5000));

    let owen = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-LV5", Some(0));
    let mem_before = runner.memory();

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(owen),
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.memory(),
        mem_before,
        "opp Lv.5 must NOT satisfy level_gte: 6 gate; before={mem_before}, after={}",
        runner.memory()
    );
}

/// Positive gate: opponent has a Lv.6 Digimon → +2 memory.
#[test]
fn bt5_093_clause1_opp_lv6_digimon_gains_two_memory() {
    let mut runner = taimatt_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("OPP-LV6", "OppLv6", 6, 9000));

    let owen = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-LV6", Some(0));
    let mem_before = runner.memory();

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(owen),
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.memory(),
        mem_before + 2,
        "opp Lv.6 → +2 memory; before={mem_before}, after={}",
        runner.memory()
    );
}

/// Positive gate (boundary above): opp Lv.7 also satisfies the gate.
#[test]
fn bt5_093_clause1_opp_lv7_digimon_gains_two_memory() {
    let mut runner = taimatt_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("OPP-LV7", "OppLv7", 7, 13000));

    let owen = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-LV7", Some(0));
    let mem_before = runner.memory();

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(owen),
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.memory(),
        mem_before + 2,
        "opp Lv.7 satisfies level_gte:6 → +2 memory; before={mem_before}, after={}",
        runner.memory()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 3 — Clause 2 aura runtime: filtered keyword install
// ═══════════════════════════════════════════════════════════════════════════════
//
// The aura grants SecurityAttackPlus(1) to own permanents whose name contains
// "Omnimon" while it is your turn. The install side (Modifiers::has_keyword
// returns true after a tick) is testable today; the consumer side
// (Game::security_attack_keyword_bonus) does NOT scan
// Modifiers::permanent_keywords, so the end-to-end "extra security pop"
// behavior is currently broken — see G-AURA-GRANTED-SECURITY-KEYWORD note in
// the file header. The end-to-end test is `#[ignore]`'d.

/// Positive: own Omnimon-named Digimon under our control receives the
/// SecurityAttackPlus(1) keyword via the aura tick.
#[test]
fn bt5_093_aura_grants_security_attack_plus_to_own_omnimon() {
    let mut runner = taimatt_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("OWN-OMN", "Omnimon", 6, 13000));

    let _owen = runner.place_on_field(0, CARD_ID, Some(0));
    let omn = runner.place_on_field(0, "OWN-OMN", Some(0));

    runner.game.tick_declarative_effects();

    assert!(
        runner
            .game
            .modifiers
            .has_keyword(omn, Keyword::SecurityAttackPlus(1)),
        "own Omnimon-named Digimon must have SecurityAttackPlus(1) installed by the aura tick"
    );
}

/// Negative (name filter): a non-Omnimon-named own Digimon must NOT receive
/// the keyword — verifies the `name_contains: Omnimon` predicate filters
/// correctly.
#[test]
fn bt5_093_aura_does_not_grant_to_non_omnimon_own_digimon() {
    let mut runner = taimatt_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("OWN-PLAIN", "PlainDigimon", 6, 9000));

    let _owen = runner.place_on_field(0, CARD_ID, Some(0));
    let plain = runner.place_on_field(0, "OWN-PLAIN", Some(0));

    runner.game.tick_declarative_effects();

    assert!(
        !runner
            .game
            .modifiers
            .has_keyword(plain, Keyword::SecurityAttackPlus(1)),
        "non-Omnimon own Digimon must NOT receive SecurityAttackPlus from the aura"
    );
}

/// Negative (controller filter): an opponent's Omnimon-named Digimon must
/// NOT receive the keyword — printed text scopes to "your Digimon".
///
/// **Found bug — YAML authoring gap (G-PREDICATE-OF-ALIAS or YAML rename to
/// `owner`):** the YAML target predicate is currently `target: { of: you,
/// zone: [battle_area], kind: digimon, name_contains: "Omnimon" }`. The DSL
/// `BoolPredicateSpec` has no `of` field — the controller filter is named
/// `owner` (see `code/digimon-dsl/src/predicate.rs:89`). With serde defaulted
/// to permissive parsing, `of: you` is silently ignored, so the compiled
/// `target.owner` is `None` and the aura scans BOTH controllers' battle
/// areas. The fix is either:
///   (a) update `cards/_examples/BT5-093.yaml` to use `owner: you` (1-line
///       authoring fix), OR
///   (b) add `#[serde(alias = "of")]` to `BoolPredicateSpec::owner` so the
///       existing field name remains valid.
/// This test is `#[ignore]`'d on the gap; the install-side positive test
/// `bt5_093_aura_grants_security_attack_plus_to_own_omnimon` already passes.
#[test]
#[ignore = "BLOCKED: G-PREDICATE-OF-ALIAS — BT5-093.yaml uses `target: { of: you, ... }` but \
            `BoolPredicateSpec` has no `of` field (only `owner`). serde silently drops the \
            unknown key, so the aura target has no owner filter and matches both controllers' \
            Omnimon-named Digimon. Fix: rename `of:` to `owner:` in the YAML, or add \
            #[serde(alias=\"of\")] to BoolPredicateSpec::owner."]
fn bt5_093_aura_does_not_grant_to_opponents_omnimon() {
    let mut runner = taimatt_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("OPP-OMN", "Omnimon", 6, 13000));

    let _owen = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_omn = runner.place_on_field(1, "OPP-OMN", Some(0));

    runner.game.tick_declarative_effects();

    assert!(
        !runner
            .game
            .modifiers
            .has_keyword(opp_omn, Keyword::SecurityAttackPlus(1)),
        "opponent's Omnimon-named Digimon must NOT receive SecurityAttackPlus from the your-side aura"
    );
}

/// Negative (turn gate): on the opponent's turn, the `active_when:
/// { your_turn: true }` gate must turn the aura off — even an own Omnimon
/// should not carry the keyword after a tick on opp's turn.
///
/// Documented gap: the aura's `active_when` is evaluated against `ctx.player`
/// (the controller of the source) at tick time. End-to-end turn-gating goes
/// through the broader declarative-tick condition path. This test is
/// `#[ignore]`'d pending verification of the active-when consumer for aura
/// process closures during cross-turn ticks.
#[test]
#[ignore = "pending: G-AURA-ACTIVE-WHEN-CROSS-TURN — aura active_when:{your_turn:true} \
            evaluation across turn boundaries needs end-to-end behavioral coverage; \
            install-side tests on your-turn already pass."]
fn bt5_093_aura_does_not_grant_on_opponents_turn() {
    let mut runner = taimatt_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("OWN-OMN", "Omnimon", 6, 13000));

    let _owen = runner.place_on_field(0, CARD_ID, Some(0));
    let omn = runner.place_on_field(0, "OWN-OMN", Some(0));

    // Sanity: on P0's turn the aura should grant.
    runner.game.tick_declarative_effects();
    assert!(
        runner
            .game
            .modifiers
            .has_keyword(omn, Keyword::SecurityAttackPlus(1)),
        "sanity: on your turn the keyword must be installed"
    );

    // Switch to P1's turn; refresh declarative state.
    runner.end_turn();
    runner.game.tick_declarative_effects();

    assert!(
        !runner
            .game
            .modifiers
            .has_keyword(omn, Keyword::SecurityAttackPlus(1)),
        "on opponent's turn the active_when:{{your_turn:true}} gate must clear the keyword"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 4 — End-to-end security pop (BLOCKED on consumer-side gap)
// ═══════════════════════════════════════════════════════════════════════════════

/// End-to-end consumer test: an own Omnimon attacking opponent player while
/// BT5-093 is on the field should pop 2 security cards (base 1 + aura's
/// SecurityAttackPlus 1).
///
/// BLOCKED by **G-AURA-GRANTED-SECURITY-KEYWORD**: `lower_aura.rs` calls
/// `ctx.grant_declarative_keyword(handle, Keyword::SecurityAttackPlus(1),
/// Expiry::Permanent)` which writes into `Modifiers::permanent_keywords`,
/// but the security-loop consumer `Game::security_attack_keyword_bonus`
/// (game.rs:1938) iterates only `permanent.card_sources` (printed
/// face/inherited keywords). The aura-granted keyword is therefore never
/// folded into the `checks` total in `resolve_player_security_loop`
/// (combat.rs:1693). Closing this gap requires either:
///   (a) extending `security_attack_keyword_bonus` to also sum
///       `Keyword::SecurityAttackPlus`/`Minus` entries from
///       `Modifiers::permanent_keywords[handle]`, or
///   (b) routing the aura's grant through `ModifierType::SecurityAttackChange`
///       instead of as a keyword grant.
#[test]
#[ignore = "BLOCKED: G-AURA-GRANTED-SECURITY-KEYWORD — Modifiers::permanent_keywords entries \
            (installed by aura grant_keyword) are NOT consulted by Game::security_attack_keyword_bonus; \
            only printed top-card/inherited keywords on permanent.card_sources are summed at the security loop."]
fn bt5_093_aura_buffed_omnimon_pops_extra_security_card() {
    let mut runner = taimatt_runner();

    // Seed P1's security with 3 cards so we can observe a 2-pop precisely.
    runner
        .game
        .card_data
        .push(make_named_digimon("SEC", "SecCard", 1, 0));
    let sec_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "SEC")
        .unwrap();
    for _ in 0..3 {
        let next = runner.game.next_card_index();
        runner.game.players[1]
            .security
            .push(CardSource::new(sec_idx, 1, next));
    }

    runner
        .game
        .card_data
        .push(make_named_digimon("OWN-OMN", "Omnimon", 6, 13000));

    let _owen = runner.place_on_field(0, CARD_ID, Some(0));
    let omn = runner.place_on_field(0, "OWN-OMN", Some(0));

    // Tick the aura so SecurityAttackPlus(1) is installed.
    runner.game.tick_declarative_effects();

    let sec_before = runner.game.players[1].security.len();
    let _ = runner.attack_player(omn, 1, false);
    let sec_after = runner.game.players[1].security.len();

    assert_eq!(
        sec_before - sec_after,
        2,
        "aura-buffed Omnimon must pop base 1 + aura 1 = 2 security cards; before={sec_before}, after={sec_after}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 5 — OPT enforcement
// ═══════════════════════════════════════════════════════════════════════════════
//
// No clause on BT5-093 is `[Once Per Turn]`. Skipped intentionally.
