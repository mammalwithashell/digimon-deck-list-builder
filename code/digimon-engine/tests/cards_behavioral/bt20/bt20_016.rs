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
//! Sub-clause "Then, this Digimon may attack." is implemented through
//! `may_attack_now` on the source permanent.
//!
//! ## Clause 2 — [All Turns] deletion observer — non-cancelling replacement
//! The "when any of YOUR Paildramon or Dinobeemon would be deleted" clause is
//! a CROSS-PERMANENT deletion observer authored as `kind: replacement` on the
//! `when_would_leave_battle_area` trigger. G-EVENT-TARGET-OWNER is resolved:
//! the `replacement_subject_is_mine` predicate exposes the deletion subject's
//! controller, so the clause filters for "your [Paildramon]/[Dinobeemon]"
//! without the carrier-only `subject_matches` gate.
//!
//! The clause carries no cancel/prevent step — the optional DNA digivolve runs
//! and the original deletion still commits (non-cancelling observer, mirroring
//! DCGO's `ActivateClass` and the BT20-091 Cool Boy / RK-G004 idiom).
//!
//! ## Clause 3 — Inherited <Security A. +1>
//! Uses `kind: grant_keyword` with `keyword: SecurityAttackPlus` and `value: 1`.
//! G-DECLARATIVE-KEYWORD does NOT block this: inherited declarative
//! `grant_keyword` clauses materialize at runtime via
//! `Game::tick_declarative_effects`, which runs inherited declarative effects
//! from buried digivolution sources. Verified behaviorally below.
//!
//! # Patterns this test covers
//! - H3 / Piercing keyword grant (end_of_turn expiry, on_play / when_digivolving)
//! - D1 / Temporary +4000 DP buff (end_of_turn expiry)
//! - G2 / DNA digivolve non-cancelling deletion observer (kind: replacement)
//! - H11 / Security A.+1 inherited keyword (declarative materialization)
//! - Effect-created `may_attack_now` sub-clause

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
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
fn bt20_016_has_replacement_clause_for_all_turns_deletion_observer() {
    // The [All Turns] "when own Paildramon/Dinobeemon would be deleted"
    // observer is a non-cancelling `kind: replacement` clause on the
    // when_would_leave_battle_area trigger (G-EVENT-TARGET-OWNER resolved).
    let compiled = compiled_bt20_016();
    let has_replacement = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { trigger, .. })
                if trigger == "when_would_leave_battle_area"
        )
    });
    assert!(
        has_replacement,
        "BT20-016 must have a when_would_leave_battle_area replacement clause for the All Turns deletion observer"
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

// ─── Section 4: may-attack-now sub-clause ───────────────────────────────────

#[test]
fn bt20_016_after_when_digivolving_this_digimon_may_attack() {
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::TriggerSource;

    let mut security_card = make_test_card("SEC", "Security");
    security_card.dp = Some(2000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(ally_digimon())
        .add_card(security_card)
        .security(1, &["SEC"])
        .memory(10)
        .build();
    runner.game.turn_count = 1;

    let ally_h = runner.place_on_field(0, "ALLY-DIG", Some(0));
    let pail_h = runner.place_on_field(0, "BT20-016", Some(0));
    let security_before = runner.game.players[1].security.len();

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(pail_h),
    );
    runner.game.drain_effect_queue();

    let choose_ally = encode_attack(0, ally_h.index as u16);
    let select = runner
        .game
        .pending_selection
        .as_ref()
        .expect("mandatory buff target selection should install");
    assert!(
        select.valid_action_ids.contains(&choose_ally),
        "ally should be a legal Piercing/+4000 target"
    );
    runner
        .game
        .resolve_selection(0, choose_ally)
        .expect("choose buff target");

    assert!(
        runner.game.modifiers.has_keyword(ally_h, Keyword::Piercing),
        "selected ally should receive Piercing before the attack prompt"
    );

    let attack_player = encode_attack(pail_h.index as u16, SECURITY_TARGET);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("may_attack_now should install an attack prompt for BT20-016");
    assert!(pending.is_optional, "printed 'may attack' must be optional");
    assert!(
        build_action_mask(&runner.game, 0)[PASS as usize] > 0.0,
        "optional may_attack_now must expose PASS through the action mask"
    );
    assert!(
        pending.valid_action_ids.contains(&attack_player),
        "BT20-016 should be able to attack the opponent player"
    );

    runner
        .game
        .resolve_selection(0, attack_player)
        .expect("resolve BT20-016 effect-created attack");

    assert_eq!(
        runner.game.players[1].security.len(),
        security_before - 1,
        "effect-created attack should resolve through the normal security flow"
    );
    assert!(
        runner.game.players[0].battle_area[pail_h.index as usize].is_suspended,
        "BT20-016 does not say without suspending, so the attack must suspend it"
    );
}

// ─── Section 5: All Turns clause — structural (replacement) ──────────────────

#[test]
fn bt20_016_deletion_replacement_clause_is_optional_and_non_cancelling() {
    // The deletion observer is `optional: true` (printed "may DNA digivolve").
    // It is a non-cancelling observer: the compiled Replacement clause carries
    // no cancel/prevent step in its process body — the lowered effect leaves
    // the replacement outcome as Proceed, so the deletion still commits.
    // Mirrors DCGO's ActivateClass and BT20-091 Cool Boy.
    let compiled = compiled_bt20_016();
    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                trigger,
                optional,
                process,
                ..
            }) if trigger == "when_would_leave_battle_area" => {
                Some((*optional, process.clone()))
            }
            _ => None,
        })
        .expect("BT20-016 must have a when_would_leave_battle_area replacement clause");

    assert!(
        clause.0,
        "deletion observer must be optional (printed 'may DNA digivolve')"
    );

    // No cancel/prevent step anywhere in the body → the deletion proceeds.
    fn has_cancel(steps: &[CompiledStep]) -> bool {
        steps.iter().any(|s| {
            let name = format!("{s:?}");
            name.contains("CancelReplacement")
                || name.contains("PreventDeletion")
                || matches!(
                    s,
                    CompiledStep::If { then, else_branch, .. }
                        if has_cancel(then) || has_cancel(else_branch)
                )
                || matches!(s, CompiledStep::Optional(steps) if has_cancel(steps))
        })
    }
    assert!(
        !has_cancel(&clause.1),
        "deletion observer must NOT cancel the deletion (non-cancelling observer); \
         process={:?}",
        clause.1
    );
}

// ─── Section 6: All Turns DNA digivolve observer (replacement) ───────────────
//
// G-EVENT-TARGET-OWNER resolved: `replacement_subject_is_mine` exposes the
// deletion subject's controller, so the cross-permanent observer can filter
// for "your [Paildramon]/[Dinobeemon]".

use digimon_engine::replacement::ReplacementCause;

/// When P0's [Paildramon] would be deleted by an opponent effect, the [All
/// Turns] deletion observer fires and offers the optional DNA-digivolve prompt.
#[test]
fn bt20_016_all_turns_fires_when_own_paildramon_would_be_deleted() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(paildramon_card())
        .add_card(imperialdramon_dm_card())
        .add_card(ally_digimon())
        .memory(10)
        .hand(0, &["TST-IMP-DM"])
        .build();

    // Carrier BT20-016 on field (the observer source) + a separate own
    // [Paildramon] permanent that will be deleted + a second own Digimon so
    // the DNA-digivolve has two materials available.
    let _carrier = runner.place_on_field(0, "BT20-016", Some(0));
    let victim = runner.place_on_field(0, "TST-PAILDRAMON", Some(0));
    runner.place_on_field(0, "ALLY-DIG", Some(0));

    runner
        .game
        .delete_permanent_with_cause(victim, ReplacementCause::OpponentEffect);

    assert!(
        runner.game.pending_selection.is_some(),
        "deletion observer must fire when own [Paildramon] would be deleted"
    );
}

/// When the OPPONENT's [Paildramon] would be deleted, the observer must NOT
/// fire (`replacement_subject_is_mine` rejects opponent-controlled subjects).
#[test]
fn bt20_016_all_turns_does_not_fire_for_opponent_paildramon_deletion() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(paildramon_card())
        .add_card(imperialdramon_dm_card())
        .memory(10)
        .hand(0, &["TST-IMP-DM"])
        .build();

    let _carrier = runner.place_on_field(0, "BT20-016", Some(0));
    // The deletion victim is the OPPONENT's [Paildramon].
    let opp_victim = runner.place_on_field(1, "TST-PAILDRAMON", Some(0));

    runner
        .game
        .delete_permanent_with_cause(opp_victim, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "deletion observer must NOT fire for the opponent's [Paildramon] \
         (replacement_subject_is_mine gate)"
    );
}

/// When a non-Paildramon/Dinobeemon own Digimon would be deleted, the observer
/// must NOT fire (name filter rejects it).
#[test]
fn bt20_016_all_turns_does_not_fire_for_non_named_own_digimon_deletion() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(ally_digimon())
        .add_card(imperialdramon_dm_card())
        .memory(10)
        .hand(0, &["TST-IMP-DM"])
        .build();

    let _carrier = runner.place_on_field(0, "BT20-016", Some(0));
    // ALLY-DIG is named "Ally Digimon" — not Paildramon/Dinobeemon.
    let victim = runner.place_on_field(0, "ALLY-DIG", Some(0));

    runner
        .game
        .delete_permanent_with_cause(victim, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "deletion observer must NOT fire for a non-[Paildramon]/[Dinobeemon] own Digimon"
    );
}

/// Full flow: when own [Paildramon] would be deleted, with Imperialdramon:
/// Dragon Mode in hand and 2 own Digimon available as DNA materials, the
/// player may DNA digivolve them into Imperialdramon: Dragon Mode. The
/// original deletion still commits (non-cancelling observer).
#[test]
fn bt20_016_all_turns_offers_dna_into_imperialdramon_and_deletion_still_commits() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(paildramon_card())
        .add_card(dinobeemon_card())
        .add_card(imperialdramon_dm_card())
        .add_card(ally_digimon())
        .memory(10)
        .hand(0, &["TST-IMP-DM"])
        .build();

    let _carrier = runner.place_on_field(0, "BT20-016", Some(0));
    let victim = runner.place_on_field(0, "TST-PAILDRAMON", Some(0));
    // Two other own Digimon to serve as DNA materials.
    runner.place_on_field(0, "TST-DINOBEEMON", Some(0));
    runner.place_on_field(0, "ALLY-DIG", Some(0));

    let victim_index = victim.index;

    runner
        .game
        .delete_permanent_with_cause(victim, ReplacementCause::OpponentEffect);

    // The observer offers an optional accept/decline prompt; drive through
    // every selection accepting the DNA digivolve.
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 40 {
        let sel = runner.game.pending_selection.as_ref().unwrap();
        let player = sel.selecting_player;
        // Prefer a non-PASS action so the DNA digivolve is actually taken.
        let action = sel
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .unwrap_or(PASS);
        let _ = runner.game.resolve_selection(player, action);
        runner.game.drain_effect_queue();
        steps += 1;
    }

    // The victim [Paildramon] is gone — the non-cancelling observer let the
    // deletion commit.
    let victim_still_present = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "TST-PAILDRAMON");
    assert!(
        !victim_still_present,
        "the original deletion must still commit (non-cancelling observer)"
    );

    // Imperialdramon: Dragon Mode left the hand (the DNA digivolve consumed it).
    let imp_in_hand = runner
        .game
        .player(0)
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "TST-IMP-DM");
    assert!(
        !imp_in_hand,
        "Imperialdramon: Dragon Mode must leave hand when the DNA digivolve resolves"
    );
    // Imperialdramon: Dragon Mode is now on the field as the DNA-digivolved top card.
    let imp_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "TST-IMP-DM");
    assert!(
        imp_on_field,
        "Imperialdramon: Dragon Mode must be on the field after the DNA digivolve"
    );
    let _ = victim_index;
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

/// Behavioral: BT20-016's inherited `<Security A. +1>` must install at runtime.
/// When BT20-016 sits as a digivolution source UNDER another Digimon, the
/// inherited declarative `grant_keyword: SecurityAttackPlus` materializes via
/// `tick_declarative_effects`, so the carrier permanent's security-attack
/// keyword bonus is +1.
#[test]
fn bt20_016_inherited_security_attack_plus_grants_modifier_at_runtime() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(ally_digimon())
        .memory(10)
        .build();

    // Stack: BT20-016 (buried digivolution source) under ALLY-DIG (top card).
    let carrier = runner.place_stack(0, &["BT20-016", "ALLY-DIG"]);

    // Materialize declarative state from on-field sources.
    runner.game.tick_declarative_effects();

    let bonus = runner.game.security_attack_keyword_bonus(carrier);
    assert_eq!(
        bonus, 1,
        "BT20-016's inherited <Security A. +1> must grant +1 security-attack \
         keyword bonus to its carrier; got {bonus}"
    );
}
