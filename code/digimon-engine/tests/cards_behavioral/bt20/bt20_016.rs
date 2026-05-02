//! BT20-016 Paildramon — Digimon | Lv5 | DP8000 | Cost8 | Red/Purple | Dragonkin
//!
//! # Card text (cards.json)
//!
//! [On Play] [When Digivolving]
//! 1 of your Digimon gains <Piercing> and gets +4000 DP for the turn. Then,
//! this Digimon may attack.
//!
//! [All Turns] When any of your [Paildramon] or [Dinobeemon] would be deleted,
//! 2 of your Digimon may DNA digivolve into [Imperialdramon: Dragon Mode] in
//! the hand.
//!
//! Inherited Effect:
//! <Security A. +1>
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT20/Red/BT20_016.cs
//!
//! # Implementation notes
//!
//! ## Clause 1 — [On Play][When Digivolving] — Fully DSL-implemented
//! Grants Piercing keyword + 4000 DP buff (end_of_turn expiry) to a selected
//! own Digimon. Mandatory selection (canNoSelect: false in DCGO).
//!
//! Sub-clause "Then, this Digimon may attack." is BLOCKED by G-MAY-ATTACK-NOW.
//! See qa/dsl-vocab-gaps.md. Omitted from YAML and tests until gap closes.
//!
//! ## Clause 2 — [All Turns] deletion observer — raw_rust declarative
//! The "when any of YOUR Paildramon or Dinobeemon would be deleted" clause is
//! a CROSS-PERMANENT deletion observer. DSL `kind: replacement` is blocked by
//! the `subject_matches` guard in `lower_replacement.rs` (lines 83–91), which
//! fires ONLY when the subject IS the carrier permanent. A cross-permanent
//! observer requires the `kind: raw_rust` declarative path, which produces a
//! hand-written `WhenWouldBeDeleted` effect that can inspect any subject.
//!
//! DCGO uses `ActivateClass` (not a replacement class), meaning deletion is NOT
//! cancelled — only an optional DNA digivolve is triggered. The implementation
//! mirrors this: the raw_rust function is a no-op (returns empty Vec<Effect>)
//! pending resolution of the following hybrid gap:
//!
//! **G-EVENT-TARGET-OWNER**: No predicate in `ReplacementContext` exposes which
//! player controls the subject permanent. Without it, a hand-written
//! `WhenWouldBeDeleted` effect can't filter for "your Paildramon/Dinobeemon"
//! (only controller-of-subject check is missing). Tracked in engine-gaps.md.
//!
//! **subject_matches gate**: `lower_replacement.rs` enforces self-only subjects;
//! cross-permanent deletion observers must bypass this gate via `raw_rust`.
//!
//! ## Clause 3 — Inherited <Security A. +1>
//! Uses `kind: grant_keyword` with `keyword: SecurityAttackPlus` and `value: 1`.
//! Blocked by G-DECLARATIVE-KEYWORD: declarative `grant_keyword` clauses compile
//! but are never fired at runtime (EffectTiming::Declarative not enqueued).
//! Tracked in qa/dsl-vocab-gaps.md.
//!
//! # Patterns this test covers
//! - H3 / Piercing keyword grant (end_of_turn expiry, on_play / when_digivolving)
//! - D1 / Temporary +4000 DP buff (end_of_turn expiry)
//! - G2-adjacent / DNA digivolve observer pattern (raw_rust declarative, PARTIAL)
//! - H11 / Security A.+1 inherited keyword (G-DECLARATIVE-KEYWORD gap noted)
//! - G-MAY-ATTACK-NOW blocked sub-clause (may-attack-now)
//! - G-EVENT-TARGET-OWNER blocked sub-clause (cross-permanent deletion observer)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

/// Production YAML for BT20-016, loaded at compile time.
const YAML: &str = include_str!("../../../cards/bt20/BT20-016.yaml");

/// Compile BT20-016 from production YAML.
fn compiled_bt20_016() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(YAML).expect("BT20-016.yaml parses without error");
    let registry = digimon_dsl::CardRegistry::from_specs("test", &[spec])
        .expect("BT20-016.yaml compiles without error");
    registry
        .lookup("BT20-016")
        .expect("BT20-016 in registry after compile")
        .clone()
}

/// Standard Digimon target for granting Piercing + DP.
fn ally_digimon() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("ALLY-DIG", "Ally Digimon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(6000);
    card
}

/// Paildramon-named card for deletion observer tests.
fn paildramon_card() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("TST-PAILDRAMON", "Paildramon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(8000);
    card
}

/// Dinobeemon-named card for deletion observer tests.
fn dinobeemon_card() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("TST-DINOBEEMON", "Dinobeemon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(6000);
    card
}

/// Imperialdramon Dragon Mode–named card for DNA hand-selection tests.
fn imperialdramon_dm_card() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("TST-IMP-DM", "Imperialdramon: Dragon Mode");
    card.card_kind = CardKind::Digimon;
    card.level = Some(6);
    card.dp = Some(11000);
    // DNA cost: Lv4 Red + Lv4 Purple / 0 memory — populated via DnaRequirement fields.
    use digimon_engine::card_data::{DnaCost, DnaRequirement};
    use digimon_engine::enums::CardColor;
    card.dna_costs = vec![DnaCost {
        requirement1: DnaRequirement {
            level: 4,
            card_colors: vec![CardColor::Red],
            name_contains: String::new(),
            text_contains: String::new(),
        },
        requirement2: DnaRequirement {
            level: 4,
            card_colors: vec![CardColor::Purple],
            name_contains: String::new(),
            text_contains: String::new(),
        },
        memory_cost: 0,
    }];
    card
}

/// Build a minimal runner with BT20-016 available via production YAML.
fn paildramon_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads into DebugRunnerBuilder")
        .memory(10)
        .build()
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

#[test]
fn bt20_016_yaml_parses_and_compiles() {
    // Smoke test: YAML round-trips through serde_yml and digimon_dsl compiler.
    let compiled = compiled_bt20_016();
    assert_eq!(compiled.card, "BT20-016", "card id must be BT20-016");
    assert_eq!(compiled.level, Some(5), "Paildramon is level 5");
    assert_eq!(compiled.dp, Some(8000), "DP is 8000");
    assert_eq!(compiled.cost, Some(8), "play cost is 8");
}

#[test]
fn bt20_016_has_on_play_when_digivolving_triggered_clause() {
    let compiled = compiled_bt20_016();
    let has_opwd = compiled.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        }
        _ => false,
    });
    assert!(
        has_opwd,
        "BT20-016 must have a triggered clause with [On Play] AND [When Digivolving] timings"
    );
}

#[test]
fn bt20_016_on_play_when_digivolving_clause_is_mandatory_and_not_opt() {
    let compiled = compiled_bt20_016();
    let clause = compiled
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
        .expect("must have [On Play][When Digivolving] clause");

    assert!(
        !clause.optional,
        "Clause 1 is mandatory — DCGO canNoSelect: false (printed text lacks 'may')"
    );
    assert!(
        !clause.once_per_turn,
        "Clause 1 has no [Once Per Turn] restriction"
    );
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "Clause 1 is own-scope (face-up)"
    );
}

#[test]
fn bt20_016_has_raw_rust_declarative_clause_for_all_turns() {
    // The All Turns deletion observer is registered as `kind: raw_rust` declarative
    // (cross-permanent deletion observer blocked by subject_matches gate + G-EVENT-TARGET-OWNER).
    let compiled = compiled_bt20_016();
    let has_raw_rust = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::RawRust { .. })
        )
    });
    assert!(
        has_raw_rust,
        "BT20-016 must have a RawRust declarative clause for the All Turns deletion observer"
    );
}

#[test]
fn bt20_016_has_inherited_grant_keyword_security_attack_plus() {
    // <Security A.+1> is expressed as a grant_keyword with keyword=SecurityAttackPlus.
    // Note: G-DECLARATIVE-KEYWORD gap means this never fires at runtime — but the
    // clause must be present in the compiled output.
    let compiled = compiled_bt20_016();
    let inherited_kw = compiled.effects.iter().any(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            scope,
            ..
        }) => keyword == "SecurityAttackPlus" && matches!(scope, CompiledScope::Inherited),
        _ => false,
    });
    assert!(
        inherited_kw,
        "BT20-016 must have an inherited GrantKeyword(SecurityAttackPlus) clause for <Security A.+1>"
    );
}

#[test]
fn bt20_016_has_digivolve_alt_path() {
    // Standard digivolve path: Lv4 Red / Cost 4.
    use digimon_dsl::compiled::CompiledAltPathKind;
    let compiled = compiled_bt20_016();
    let has_evo = compiled
        .alt_paths
        .iter()
        .any(|p| p.kind == CompiledAltPathKind::Digivolve);
    assert!(
        has_evo,
        "BT20-016 must have at least one Digivolve alt_path"
    );
}

#[test]
fn bt20_016_has_dna_digivolve_alt_path() {
    // DNA digivolve path: Lv4 Red + Lv4 Purple / Cost 0.
    use digimon_dsl::compiled::CompiledAltPathKind;
    let compiled = compiled_bt20_016();
    let has_dna = compiled
        .alt_paths
        .iter()
        .any(|p| p.kind == CompiledAltPathKind::DnaDigivolve);
    assert!(has_dna, "BT20-016 must have a DnaDigivolve alt_path");
}

// ─── Section 2: Condition gating — Clause 1 ──────────────────────────────────

/// Positive: with at least one own Digimon on the field, the on_play clause
/// installs a selection prompt.
#[test]
fn bt20_016_on_play_with_ally_on_field_prompts_selection() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(ally_digimon())
        .hand(0, &["BT20-016"])
        .memory(10)
        .build();

    // Pre-place an ally Digimon so the condition passes even before Paildramon lands.
    runner.place_on_field(0, "ALLY-DIG", Some(0));

    let _played = runner.play(0, 0).expect("Paildramon plays from hand");

    let kind = runner
        .pending_kind()
        .expect("a pending selection must install when at least one own Digimon is on the field");
    assert_eq!(
        kind,
        SelectionKind::OwnField,
        "selection kind must be OwnField (select_own_permanent)"
    );
}

/// Paildramon itself lands on the field; self-target is allowed (no exclusion in DCGO).
#[test]
fn bt20_016_on_play_self_target_allowed_when_only_own_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .hand(0, &["BT20-016"])
        .memory(10)
        .build();

    let _played = runner.play(0, 0).expect("Paildramon plays from hand");

    // Paildramon IS on the field. The condition must be satisfied.
    let kind = runner.pending_kind();
    assert!(
        kind.is_some(),
        "when Paildramon is the only own Digimon, it must still install a selection prompt"
    );
}

/// Negative: when no Digimon is on the field at the time of effect resolution,
/// no selection is installed (condition fails). This edge case can occur if the
/// play triggers into an empty board state (which cannot happen normally since
/// Paildramon itself lands first, but validates the condition binding).
#[test]
fn bt20_016_on_play_clause_condition_requires_own_digimon() {
    // Structural: verify the [On Play][When Digivolving] clause carries a condition.
    let compiled = compiled_bt20_016();
    let clause = compiled
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
        .expect("must have [On Play][When Digivolving] clause");

    assert!(
        clause.condition.is_some(),
        "Clause 1 must have a condition (any_permanent own digimon in battle_area)"
    );
}

// ─── Section 3: Behavioral outcomes — Clause 1 ───────────────────────────────

#[test]
fn bt20_016_on_play_grants_piercing_keyword_to_selected_ally() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(ally_digimon())
        .hand(0, &["BT20-016"])
        .memory(10)
        .build();

    let target_h = runner.place_on_field(0, "ALLY-DIG", Some(0));

    let _played = runner.play(0, 0).expect("Paildramon plays");
    runner.auto_resolve().expect("auto_resolve succeeds");

    let has_piercing = runner
        .game
        .modifiers
        .has_keyword(target_h, Keyword::Piercing);
    assert!(
        has_piercing,
        "selected ally Digimon must have Piercing keyword modifier after Clause 1 fires"
    );
}

#[test]
fn bt20_016_on_play_grants_4000_dp_buff_to_selected_ally() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(ally_digimon())
        .hand(0, &["BT20-016"])
        .memory(10)
        .build();

    let target_h = runner.place_on_field(0, "ALLY-DIG", Some(0));
    let base_dp = runner
        .effective_dp(target_h)
        .expect("ALLY-DIG must have DP");

    let _played = runner.play(0, 0).expect("Paildramon plays");
    runner.auto_resolve().expect("auto_resolve succeeds");

    let after_dp = runner
        .effective_dp(target_h)
        .expect("ALLY-DIG must still be on field");
    assert_eq!(
        after_dp,
        base_dp + 4000,
        "selected Digimon must have +4000 DP after Clause 1 fires"
    );
}

#[test]
fn bt20_016_when_digivolving_grants_piercing_keyword() {
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::TriggerSource;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(ally_digimon())
        .memory(10)
        .build();

    let ally_h = runner.place_on_field(0, "ALLY-DIG", Some(0));
    let pail_h = runner.place_on_field(0, "BT20-016", None);

    // Fire WhenDigivolving explicitly from the Paildramon permanent.
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(pail_h),
    );
    runner.game.drain_effect_queue();

    runner.auto_resolve().expect("auto_resolve succeeds");

    let has_piercing = runner.game.modifiers.has_keyword(ally_h, Keyword::Piercing);
    assert!(
        has_piercing,
        "ally must have Piercing after WhenDigivolving fires and selects it"
    );
}

#[test]
fn bt20_016_piercing_keyword_expires_at_end_of_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(ally_digimon())
        .hand(0, &["BT20-016"])
        .memory(10)
        .build();

    let target_h = runner.place_on_field(0, "ALLY-DIG", Some(0));

    let _played = runner.play(0, 0).expect("Paildramon plays");
    runner.auto_resolve().expect("auto_resolve succeeds");

    // Verify buff is active during the turn.
    assert!(
        runner
            .game
            .modifiers
            .has_keyword(target_h, Keyword::Piercing),
        "Piercing must be active during the turn it was granted"
    );

    // End P0's turn → expiry clears.
    runner.end_turn();

    assert!(
        !runner
            .game
            .modifiers
            .has_keyword(target_h, Keyword::Piercing),
        "Piercing must expire at end of the turn it was granted"
    );
}

#[test]
fn bt20_016_dp_buff_expires_at_end_of_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(ally_digimon())
        .hand(0, &["BT20-016"])
        .memory(10)
        .build();

    let target_h = runner.place_on_field(0, "ALLY-DIG", Some(0));
    let base_dp = runner
        .effective_dp(target_h)
        .expect("ALLY-DIG must have DP");

    let _played = runner.play(0, 0).expect("Paildramon plays");
    runner.auto_resolve().expect("auto_resolve succeeds");

    // Buffed during turn.
    let during_dp = runner
        .effective_dp(target_h)
        .expect("ALLY-DIG must be on field");
    assert_eq!(
        during_dp,
        base_dp + 4000,
        "+4000 DP active during play turn"
    );

    runner.end_turn();

    let after_dp = runner
        .effective_dp(target_h)
        .expect("ALLY-DIG must persist");
    assert_eq!(
        after_dp, base_dp,
        "+4000 DP must expire at end of the turn it was granted"
    );
}

#[test]
fn bt20_016_clause1_selection_is_mandatory() {
    // With an ally on field, the selection prompt must be mandatory (canNoSelect: false).
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(ally_digimon())
        .hand(0, &["BT20-016"])
        .memory(10)
        .build();

    runner.place_on_field(0, "ALLY-DIG", Some(0));
    let _played = runner.play(0, 0).expect("Paildramon plays");

    let view = runner
        .pending_selection_view()
        .expect("selection must be pending");
    assert!(
        !view.is_optional,
        "Clause 1 selection must be mandatory (DCGO canNoSelect: false)"
    );
}

// ─── Section 4: BLOCKED — may-attack-now sub-clause ─────────────────────────

#[test]
#[ignore = "pending: G-MAY-ATTACK-NOW — no DSL verb for optional mid-effect attack on a specific Digimon (qa/dsl-vocab-gaps.md)"]
fn bt20_016_after_on_play_this_digimon_may_attack() {
    // BLOCKED: card text says "Then, this Digimon may attack."
    // No DSL verb (may_attack_now) or engine primitive exists for optional
    // mid-effect attack on a specific Digimon. Omitted from YAML.
    // See qa/dsl-vocab-gaps.md — G-MAY-ATTACK-NOW.
    unimplemented!()
}

// ─── Section 5: All Turns clause — structural (raw_rust) ─────────────────────

#[test]
fn bt20_016_raw_rust_clause_has_fn_name_bt20_016_dna_on_deletion() {
    // Verify the raw_rust declarative clause names the correct registered function.
    let compiled = compiled_bt20_016();
    let fn_name = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::RawRust { fn_name, .. }) => {
            Some(fn_name.as_str())
        }
        _ => None,
    });
    assert_eq!(
        fn_name,
        Some("bt20_016_dna_on_deletion"),
        "raw_rust clause must reference 'bt20_016_dna_on_deletion' function"
    );
}

// ─── Section 6: BLOCKED — All Turns DNA digivolve observer ───────────────────

#[test]
#[ignore = "pending: G-EVENT-TARGET-OWNER — no predicate to filter which player controls an event-target permanent in replacement/trigger conditions (qa/archetype-qa/engine-gaps.md)"]
fn bt20_016_all_turns_fires_when_own_paildramon_would_be_deleted() {
    // BLOCKED: G-EVENT-TARGET-OWNER
    // The `bt20_016_dna_on_deletion` raw_rust function needs to filter for
    // "your Paildramon or Dinobeemon" — i.e., a WhenWouldBeDeleted effect where
    // the subject is controlled by the same player as the carrier. The engine
    // does not expose the subject's controller in the ReplacementContext, and
    // lower_replacement.rs's subject_matches gate prevents non-self subjects.
    // Until this gap closes, the raw_rust function returns an empty Vec<Effect>.
    // See qa/archetype-qa/engine-gaps.md — G-EVENT-TARGET-OWNER.
    unimplemented!()
}

#[test]
#[ignore = "pending: G-EVENT-TARGET-OWNER — cross-permanent deletion observer not implementable (qa/archetype-qa/engine-gaps.md)"]
fn bt20_016_all_turns_does_not_fire_for_opponent_paildramon_deletion() {
    // BLOCKED: same gap. The filter "any of YOUR [Paildramon] or [Dinobeemon]"
    // excludes the opponent's cards of the same name. Without the controller
    // predicate in ReplacementContext, accurate filtering is not possible.
    unimplemented!()
}

#[test]
#[ignore = "pending: G-EVENT-TARGET-OWNER + G-MAY-ATTACK-NOW — DNA digivolve triggered by deletion observer (qa/archetype-qa/engine-gaps.md)"]
fn bt20_016_all_turns_offers_dna_into_imperialdramon_from_hand_when_paildramon_deleted() {
    // BLOCKED: requires the deletion observer to fire (G-EVENT-TARGET-OWNER) and
    // the DNA selection + effect_initiated_dna_digivolve to work from that context.
    // After both gaps close, this test verifies:
    //   - When P0's Paildramon (TST-PAILDRAMON) would be deleted, the [All Turns]
    //     clause offers an optional prompt.
    //   - With "Imperialdramon: Dragon Mode" in hand and 2 own Digimon on field,
    //     the DNA digivolve completes successfully.
    unimplemented!()
}

// ─── Section 7: Inherited Security A.+1 ─────────────────────────────────────

#[test]
fn bt20_016_inherited_security_attack_plus_clause_is_inherited_scope() {
    let compiled = compiled_bt20_016();
    let clause = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            scope,
            ..
        }) if keyword == "SecurityAttackPlus" => Some(scope),
        _ => None,
    });
    assert!(
        clause.is_some(),
        "must have a GrantKeyword(SecurityAttackPlus) clause"
    );
    assert_eq!(
        *clause.unwrap(),
        CompiledScope::Inherited,
        "Security A.+1 must be inherited scope"
    );
}

#[test]
#[ignore = "pending: G-DECLARATIVE-KEYWORD — declarative grant_keyword clauses compile but are never fired at runtime (qa/dsl-vocab-gaps.md)"]
fn bt20_016_inherited_security_attack_plus_grants_modifier_at_runtime() {
    // BLOCKED: G-DECLARATIVE-KEYWORD
    // The EffectTiming::Declarative is not enqueued by the effect queue.
    // When this gap closes, verify that placing BT20-016 on the field as a
    // digivolution source installs a SecurityAttackPlus modifier on the carrier
    // permanent via the ModifierRegistry.
    // See qa/dsl-vocab-gaps.md — G-DECLARATIVE-KEYWORD.
    unimplemented!()
}
