//! BT20-016 Paildramon — Digimon | Lv5 | DP8000 | Cost8 | Red/Purple | Dragonkin
//!
//! # Card text (cards.json)
//!
//! [On Play] [When Digivolving]
//! For the turn, 1 of your Digimon gains <Piercing> and gets +4000 DP. Then,
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
//! ## Clause 1 — [On Play][When Digivolving] — pure DSL
//! Grants Piercing keyword + 4000 DP buff (end_of_turn expiry) to a selected
//! own Digimon. Mandatory selection (canNoSelect: false in DCGO). The
//! sub-clause "Then, this Digimon may attack." routes through `may_attack_now`
//! on the source permanent (printed text omits "without suspending", so the
//! attack suspends BT20-016 normally).
//!
//! ## Clause 2 — [All Turns] deletion observer → DNA digivolve — pure DSL
//! "When any of your [Paildramon] or [Dinobeemon] would be deleted, 2 of your
//! Digimon may DNA digivolve into [Imperialdramon: Dragon Mode] in the hand."
//!
//! Split into two triggered clauses:
//!   - Clause 2a — `when: on_deletion` (self-deletion). BT20-016 is itself named
//!     [Paildramon]; `on_any_deletion` only scans LIVE permanents so the
//!     just-deleted carrier never observes its own deletion there. The dedicated
//!     `on_deletion` self-timing fires while the carrier is still on field.
//!   - Clause 2b — `when: on_any_deletion` + `event_target_owner: you` +
//!     `event_target_kind: digimon` + name `any_of` (Paildramon/Dinobeemon).
//!     Fires when ANOTHER of the controller's Paildramon/Dinobeemon is deleted.
//!     G-EVENT-TARGET-OWNER closed 2026-05-02 — same substrate as BT21-026.
//!
//! Each clause's body is the optional DNA digivolve: pick the Imperialdramon:
//! Dragon Mode in hand (`select_hand`), pick 2 own Digimon as DNA materials
//! (`select_dna_pair`), then `effect_initiated_dna_digivolve`. The whole body
//! is wrapped in `- optional:` to mirror the printed "may".
//!
//! ## Clause 3 — Inherited <Security A. +1>
//! `kind: grant_keyword`, `keyword: SecurityAttackPlus`, `value: 1`,
//! `scope: inherited`. G-DECLARATIVE-KEYWORD closed 2026-05-02 — the modifier
//! installs at runtime and is visible to `game.has_keyword()` via the
//! digivolution-stack walk (ST1-07 / AD1-009 idiom).
//!
//! # Patterns this test covers
//! - H3 / Piercing keyword grant (end_of_turn expiry, on_play / when_digivolving)
//! - D1 / Temporary +4000 DP buff (end_of_turn expiry)
//! - Effect-created `may_attack_now` sub-clause
//! - G3 / DNA-digivolve-on-deletion (self via on_deletion + cross-permanent via
//!   on_any_deletion + event_target_owner)
//! - H11 / Security A.+1 inherited keyword (runtime stack-walk)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

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

/// Lv4 Red Digimon — legal DNA material element 1.
fn red_lv4_material() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("RED-LV4", "Red Material");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.colors = vec![CardColor::Red];
    card.dp = Some(5000);
    card
}

/// Lv4 Purple Digimon — legal DNA material element 2.
fn purple_lv4_material() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("PURPLE-LV4", "Purple Material");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.colors = vec![CardColor::Purple];
    card.dp = Some(5000);
    card
}

/// Imperialdramon Dragon Mode–named card — the DNA digivolve result card.
/// Carries the printed Lv4 Red + Lv4 Purple / 0 memory DNA recipe.
fn imperialdramon_dm_card() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("TST-IMP-DM", "Imperialdramon: Dragon Mode");
    card.card_kind = CardKind::Digimon;
    card.level = Some(6);
    card.dp = Some(11000);
    use digimon_engine::card_data::{DnaCost, DnaRequirement};
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

/// Clause 2 is now pure DSL — two triggered clauses, no raw_rust placeholder.
#[test]
fn bt20_016_has_no_raw_rust_clause() {
    let compiled = compiled_bt20_016();
    let has_raw_rust = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::RawRust { .. })
        )
    });
    assert!(
        !has_raw_rust,
        "BT20-016 clause 2 is pure DSL — no raw_rust declarative clause should remain"
    );
}

/// Clause 2a — the self-deletion arm uses `when: on_deletion`.
#[test]
fn bt20_016_has_on_deletion_triggered_clause() {
    let compiled = compiled_bt20_016();
    let has_on_deletion = compiled.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::OnDeletion),
        _ => false,
    });
    assert!(
        has_on_deletion,
        "BT20-016 must have an on_deletion triggered clause (self-deletion DNA arm)"
    );
}

/// Clause 2b — the cross-permanent arm uses `when: on_any_deletion`.
#[test]
fn bt20_016_has_on_any_deletion_triggered_clause() {
    let compiled = compiled_bt20_016();
    let has_on_any_deletion = compiled.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::OnAnyDeletion),
        _ => false,
    });
    assert!(
        has_on_any_deletion,
        "BT20-016 must have an on_any_deletion triggered clause (cross-permanent DNA arm)"
    );
}

/// Both deletion arms are optional ("may DNA digivolve").
#[test]
fn bt20_016_deletion_arms_are_optional() {
    let compiled = compiled_bt20_016();
    for c in &compiled.effects {
        if let CompiledClause::Triggered(t) = c {
            if t.when.contains(&CompiledTiming::OnDeletion)
                || t.when.contains(&CompiledTiming::OnAnyDeletion)
            {
                assert!(
                    t.optional,
                    "deletion-arm DNA digivolve clauses must be optional (printed 'may')"
                );
            }
        }
    }
}

/// The on_any_deletion arm carries a condition gating on event-target owner,
/// kind, and Paildramon/Dinobeemon name.
#[test]
fn bt20_016_on_any_deletion_clause_has_condition() {
    let compiled = compiled_bt20_016();
    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnAnyDeletion) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("must have on_any_deletion clause");
    assert!(
        clause.condition.is_some(),
        "on_any_deletion clause must have a condition (event_target_owner + kind + name)"
    );
}

#[test]
fn bt20_016_has_inherited_grant_keyword_security_attack_plus() {
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
    use digimon_dsl::compiled::CompiledAltPathKind;
    let compiled = compiled_bt20_016();
    let has_evo = compiled
        .alt_paths
        .iter()
        .any(|p| p.kind == CompiledAltPathKind::Digivolve);
    assert!(has_evo, "BT20-016 must have at least one Digivolve alt_path");
}

#[test]
fn bt20_016_has_dna_digivolve_alt_path() {
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

    let kind = runner.pending_kind();
    assert!(
        kind.is_some(),
        "when Paildramon is the only own Digimon, it must still install a selection prompt"
    );
}

/// Structural: the [On Play][When Digivolving] clause carries a condition.
#[test]
fn bt20_016_on_play_clause_condition_requires_own_digimon() {
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
    let base_dp = runner.effective_dp(target_h).expect("ALLY-DIG must have DP");

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
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(ally_digimon())
        .memory(10)
        .build();

    let ally_h = runner.place_on_field(0, "ALLY-DIG", Some(0));
    let pail_h = runner.place_on_field(0, "BT20-016", None);

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

    assert!(
        runner
            .game
            .modifiers
            .has_keyword(target_h, Keyword::Piercing),
        "Piercing must be active during the turn it was granted"
    );

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
    let base_dp = runner.effective_dp(target_h).expect("ALLY-DIG must have DP");

    let _played = runner.play(0, 0).expect("Paildramon plays");
    runner.auto_resolve().expect("auto_resolve succeeds");

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

#[test]
fn bt20_016_may_attack_can_be_declined() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(ally_digimon())
        .memory(10)
        .build();
    runner.game.turn_count = 1;

    let ally_h = runner.place_on_field(0, "ALLY-DIG", Some(0));
    let pail_h = runner.place_on_field(0, "BT20-016", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(pail_h),
    );
    runner.game.drain_effect_queue();

    // Resolve the mandatory buff target.
    let view = runner
        .pending_selection_view()
        .expect("buff target selection must install");
    runner
        .game
        .resolve_selection(view.selecting_player, view.valid_action_ids[0])
        .expect("choose buff target");

    // The may_attack_now prompt must be optional and PASS-able.
    let view = runner
        .pending_selection_view()
        .expect("may_attack_now prompt must install");
    assert!(view.is_optional, "may-attack prompt must be optional");
    runner
        .game
        .resolve_selection(view.selecting_player, PASS)
        .expect("PASS must be legal on the optional may-attack prompt");

    assert!(
        runner.pending_selection().is_none(),
        "after declining may-attack, no pending selection remains"
    );
    assert!(
        !runner.game.players[0].battle_area[pail_h.index as usize].is_suspended,
        "declining the attack must leave BT20-016 unsuspended"
    );
}

// ─── Section 5: All Turns clause — self-deletion arm (Clause 2a) ──────────────

/// Positive: when BT20-016 itself (a Paildramon) is deleted, the on_deletion
/// arm offers the optional DNA digivolve. With an Imperialdramon: Dragon Mode
/// in hand and 2 own Digimon on field, the DNA digivolve completes.
///
/// The clause is optional ("may"): the first prompt is the optional
/// `select_hand` for the result card (PASS would decline the whole DNA
/// digivolve), then the `select_dna_pair`.
#[test]
fn bt20_016_self_deletion_offers_dna_into_imperialdramon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(red_lv4_material())
        .add_card(purple_lv4_material())
        .add_card(imperialdramon_dm_card())
        .hand(0, &["TST-IMP-DM"])
        .memory(10)
        .build();

    let red_h = runner.place_on_field(0, "RED-LV4", Some(0));
    let purple_h = runner.place_on_field(0, "PURPLE-LV4", Some(0));
    let pail_h = runner.place_on_field(0, "BT20-016", Some(0));

    // Delete BT20-016 — its own on_deletion arm should fire.
    runner.game.delete_permanent_with_effects(pail_h);
    runner.game.drain_effect_queue();

    // First prompt: the optional select_hand for Imperialdramon: Dragon Mode.
    let view = runner
        .pending_selection_view()
        .expect("on_deletion DNA arm must install a hand-select prompt");
    assert_eq!(
        view.kind,
        SelectionKind::Hand,
        "first step is the result-card hand selection"
    );
    assert!(view.is_optional, "DNA digivolve is optional ('may')");
    runner
        .game
        .resolve_selection(view.selecting_player, view.valid_action_ids[0])
        .expect("pick Imperialdramon: Dragon Mode from hand");

    // Next: select_dna_pair — pick first material.
    let view = runner
        .pending_selection_view()
        .expect("first DNA material select must install");
    let pick_red = encode_attack(red_h.player as u16, red_h.index as u16);
    runner
        .game
        .resolve_selection(view.selecting_player, pick_red)
        .expect("pick Red Lv4 material");

    // select_dna_pair — pick second material.
    let view = runner
        .pending_selection_view()
        .expect("second DNA material select must install");
    let pick_purple = encode_attack(purple_h.player as u16, purple_h.index as u16);
    runner
        .game
        .resolve_selection(view.selecting_player, pick_purple)
        .expect("pick Purple Lv4 material");

    runner.auto_resolve().expect("finish DNA digivolve flow");

    // Imperialdramon: Dragon Mode must now be on the field as the merged top card.
    let imp_on_field = runner.game.players[0].battle_area.iter().any(|perm| {
        perm.card_sources
            .last()
            .and_then(|cs| runner.game.card_data_for_handle(cs.handle()))
            .map(|d| d.card_name == "Imperialdramon: Dragon Mode")
            .unwrap_or(false)
    });
    assert!(
        imp_on_field,
        "after a successful DNA digivolve, Imperialdramon: Dragon Mode must be on the field"
    );
    // The Imperialdramon: Dragon Mode card left the hand.
    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|cs| runner
                .game
                .card_data_for_handle(cs.handle())
                .map(|d| d.card_name == "Imperialdramon: Dragon Mode")
                .unwrap_or(false)),
        "Imperialdramon: Dragon Mode must leave the hand once it digivolves"
    );
}

/// Negative: the self-deletion DNA arm is optional — declining it leaves the
/// board unchanged (no DNA digivolve).
#[test]
fn bt20_016_self_deletion_dna_arm_can_be_declined() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(red_lv4_material())
        .add_card(purple_lv4_material())
        .add_card(imperialdramon_dm_card())
        .hand(0, &["TST-IMP-DM"])
        .memory(10)
        .build();

    let red_h = runner.place_on_field(0, "RED-LV4", Some(0));
    let purple_h = runner.place_on_field(0, "PURPLE-LV4", Some(0));
    let pail_h = runner.place_on_field(0, "BT20-016", Some(0));

    runner.game.delete_permanent_with_effects(pail_h);
    runner.game.drain_effect_queue();

    // First prompt is the optional hand-select; PASS declines the whole
    // DNA digivolve.
    let view = runner
        .pending_selection_view()
        .expect("on_deletion DNA arm must install a hand-select prompt");
    assert!(view.is_optional, "DNA digivolve is optional");
    runner
        .game
        .resolve_selection(view.selecting_player, PASS)
        .expect("PASS must be legal on the optional DNA prompt");

    runner.auto_resolve().expect("finish declined flow");

    // Imperialdramon: Dragon Mode stays in hand; the 2 materials stay on field.
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|cs| runner
                .game
                .card_data_for_handle(cs.handle())
                .map(|d| d.card_name == "Imperialdramon: Dragon Mode")
                .unwrap_or(false)),
        "declining the DNA digivolve leaves Imperialdramon: Dragon Mode in hand"
    );
    let imp_on_field = runner.game.players[0].battle_area.iter().any(|perm| {
        perm.card_sources
            .last()
            .and_then(|cs| runner.game.card_data_for_handle(cs.handle()))
            .map(|d| d.card_name == "Imperialdramon: Dragon Mode")
            .unwrap_or(false)
    });
    assert!(
        !imp_on_field,
        "no DNA digivolve happens when the optional arm is declined"
    );
}

// ─── Section 6: All Turns clause — cross-permanent arm (Clause 2b) ────────────

/// Positive: when ANOTHER of P0's Paildramon-named Digimon is deleted, the
/// carrier BT20-016's on_any_deletion arm offers the optional DNA digivolve.
/// The first prompt is the optional result-card hand-select.
#[test]
fn bt20_016_cross_permanent_deletion_of_own_paildramon_offers_dna() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(paildramon_card())
        .add_card(red_lv4_material())
        .add_card(purple_lv4_material())
        .add_card(imperialdramon_dm_card())
        .hand(0, &["TST-IMP-DM"])
        .memory(10)
        .build();

    // The carrier BT20-016 stays on field to observe the deletion.
    let carrier_h = runner.place_on_field(0, "BT20-016", Some(0));
    let other_pail_h = runner.place_on_field(0, "TST-PAILDRAMON", Some(0));
    let red_h = runner.place_on_field(0, "RED-LV4", Some(0));
    let purple_h = runner.place_on_field(0, "PURPLE-LV4", Some(0));

    // Delete the OTHER Paildramon — the carrier's on_any_deletion arm fires.
    runner.game.delete_permanent_with_effects(other_pail_h);
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("on_any_deletion DNA arm must install for the carrier");
    assert_eq!(
        view.kind,
        SelectionKind::Hand,
        "first step is the result-card hand selection"
    );
    assert!(view.is_optional, "cross-permanent DNA digivolve is optional");
    assert_eq!(
        view.selecting_player, 0,
        "the carrier's controller resolves the optional DNA prompt"
    );
}

/// Positive: a Dinobeemon-named deletion also triggers the cross-permanent arm.
#[test]
fn bt20_016_cross_permanent_deletion_of_own_dinobeemon_offers_dna() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(dinobeemon_card())
        .add_card(imperialdramon_dm_card())
        .hand(0, &["TST-IMP-DM"])
        .memory(10)
        .build();

    let carrier_h = runner.place_on_field(0, "BT20-016", Some(0));
    let dino_h = runner.place_on_field(0, "TST-DINOBEEMON", Some(0));

    runner.game.delete_permanent_with_effects(dino_h);
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("Dinobeemon deletion must trigger the carrier's on_any_deletion arm");
    assert_eq!(
        view.kind,
        SelectionKind::Hand,
        "the cross-permanent arm installs the same DNA hand-select"
    );
    assert!(view.is_optional, "DNA digivolve is optional");
}

/// Negative: deleting a non-Paildramon/Dinobeemon own Digimon must NOT
/// trigger the cross-permanent arm.
#[test]
fn bt20_016_cross_permanent_arm_ignores_unrelated_own_digimon_deletion() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(ally_digimon())
        .memory(10)
        .build();

    let carrier_h = runner.place_on_field(0, "BT20-016", Some(0));
    let ally_h = runner.place_on_field(0, "ALLY-DIG", Some(0));

    runner.game.delete_permanent_with_effects(ally_h);
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "deleting a non-Paildramon/Dinobeemon own Digimon must not trigger the DNA arm"
    );
}

/// Negative: deleting the OPPONENT's Paildramon must NOT trigger the
/// cross-permanent arm — the clause is gated by `event_target_owner: you`.
#[test]
fn bt20_016_cross_permanent_arm_ignores_opponent_paildramon_deletion() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(paildramon_card())
        .memory(10)
        .build();

    let carrier_h = runner.place_on_field(0, "BT20-016", Some(0));
    // The opponent (P1) owns a Paildramon-named Digimon.
    let opp_pail_h = runner.place_on_field(1, "TST-PAILDRAMON", Some(0));

    runner.game.delete_permanent_with_effects(opp_pail_h);
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "deleting the OPPONENT's Paildramon must not trigger the carrier's DNA arm \
         (event_target_owner: you gate)"
    );
}

/// Integrated: the cross-permanent arm completes a DNA digivolve into
/// Imperialdramon: Dragon Mode using 2 of the controller's other Digimon.
#[test]
fn bt20_016_cross_permanent_arm_completes_dna_digivolve() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(paildramon_card())
        .add_card(red_lv4_material())
        .add_card(purple_lv4_material())
        .add_card(imperialdramon_dm_card())
        .hand(0, &["TST-IMP-DM"])
        .memory(10)
        .build();

    // Place the OTHER Paildramon LAST so deleting it does not shift the
    // material slot indices captured below.
    let carrier_h = runner.place_on_field(0, "BT20-016", Some(0));
    let red_h = runner.place_on_field(0, "RED-LV4", Some(0));
    let purple_h = runner.place_on_field(0, "PURPLE-LV4", Some(0));
    let other_pail_h = runner.place_on_field(0, "TST-PAILDRAMON", Some(0));

    runner.game.delete_permanent_with_effects(other_pail_h);
    runner.game.drain_effect_queue();

    // First prompt: the optional result-card hand-select.
    let view = runner
        .pending_selection_view()
        .expect("on_any_deletion DNA arm must install a hand-select prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    runner
        .game
        .resolve_selection(view.selecting_player, view.valid_action_ids[0])
        .expect("pick Imperialdramon: Dragon Mode");

    // Pick the 2 DNA materials (slot indices unchanged — the deleted Paildramon
    // was the highest-index slot).
    let view = runner
        .pending_selection_view()
        .expect("first DNA material select");
    let pick_red = encode_attack(red_h.player as u16, red_h.index as u16);
    runner
        .game
        .resolve_selection(view.selecting_player, pick_red)
        .expect("pick Red Lv4");

    let view = runner
        .pending_selection_view()
        .expect("second DNA material select");
    let pick_purple = encode_attack(purple_h.player as u16, purple_h.index as u16);
    runner
        .game
        .resolve_selection(view.selecting_player, pick_purple)
        .expect("pick Purple Lv4");

    runner.auto_resolve().expect("finish DNA digivolve");

    let imp_on_field = runner.game.players[0].battle_area.iter().any(|perm| {
        perm.card_sources
            .last()
            .and_then(|cs| runner.game.card_data_for_handle(cs.handle()))
            .map(|d| d.card_name == "Imperialdramon: Dragon Mode")
            .unwrap_or(false)
    });
    assert!(
        imp_on_field,
        "cross-permanent DNA arm must put Imperialdramon: Dragon Mode on the field"
    );
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
fn bt20_016_inherited_security_attack_plus_value_is_one() {
    let compiled = compiled_bt20_016();
    let value = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            value,
            ..
        }) if keyword == "SecurityAttackPlus" => Some(*value),
        _ => None,
    });
    assert_eq!(
        value,
        Some(Some(1)),
        "SecurityAttackPlus value must be 1 (card text says '+1')"
    );
}

/// Positive runtime: when BT20-016 is a digivolution source under a carrier
/// Digimon, the carrier inherits SecurityAttackPlus(1) via the stack-walk.
/// G-DECLARATIVE-KEYWORD closed 2026-05-02 — the inherited modifier installs.
#[test]
fn bt20_016_inherited_security_attack_plus_grants_modifier_at_runtime() {
    let carrier_card = make_test_card("PLAIN-LV6", "PlainLv6");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(carrier_card)
        .memory(10)
        .build();

    // Place BT20-016 at slot 0 first to capture its card source.
    runner.place_on_field(0, "BT20-016", Some(0));
    let paildramon_source = {
        let game = runner.game_mut();
        game.players[0].battle_area[0].top_card().clone()
    };

    // Replace slot 0 with the carrier.
    let carrier = runner.place_on_field(0, "PLAIN-LV6", Some(0));

    // Push BT20-016's card source under the carrier (simulates a digivolve stack).
    {
        let game = runner.game_mut();
        game.players[0].battle_area[carrier.index as usize]
            .card_sources
            .insert(0, paildramon_source);
    }

    assert!(
        runner
            .game
            .has_keyword(carrier, Keyword::SecurityAttackPlus(1)),
        "a carrier with BT20-016 as a digivolution source must inherit SecurityAttackPlus(1)"
    );
}

/// Negative runtime: a carrier without BT20-016 in its stack does NOT have
/// SecurityAttackPlus(1).
#[test]
fn bt20_016_inherited_security_attack_plus_absent_without_source() {
    let carrier_card = make_test_card("PLAIN-LV6", "PlainLv6");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT20-016 YAML loads")
        .add_card(carrier_card)
        .memory(10)
        .build();

    let carrier = runner.place_on_field(0, "PLAIN-LV6", Some(0));

    assert!(
        !runner
            .game
            .has_keyword(carrier, Keyword::SecurityAttackPlus(1)),
        "a carrier with no BT20-016 in its digivolution stack must NOT have SecurityAttackPlus(1)"
    );
}
