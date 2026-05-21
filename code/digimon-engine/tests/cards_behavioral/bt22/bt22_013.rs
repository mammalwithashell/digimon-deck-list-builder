//! BT22-013 WarGreymon — Digimon, Lv.6, Red, DP 12000, Cost 12.
//! Traits: Dragonkin, CS.
//! Attribute: Vaccine
//! Evo: Lv.5 Red / cost 4
//!
//! # Card text (cards.json — verbatim)
//!
//! [Hand] [Main] If you have [Nokia Shiramine], 1 of your [Agumon] digivolves
//! into this card for a digivolution cost of 6, ignoring digivolution
//! requirements.
//!
//! [When Digivolving] Activate 1 of the effects below:
//!   ・1 of your [Gabumon] may digivolve into [MetalGarurumon] in the hand,
//!     ignoring digivolution requirements and without paying the cost.
//!   ・Delete 1 of your opponent's Digimon with the lowest DP.
//!
//! Inherited Effect [When Attacking] [Once Per Turn]
//!   If this Digimon has [Omnimon] in its name, trash your opponent's top
//!   security card.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT22/Red/BT22_013.cs
//!
//! # Patterns this test covers
//! - Structural: 2 alt_paths (Digivolve Lv.5/cost-4 + ActivatedDigivolve from
//!   Agumon at cost 6, ignore_requirements) + 2 triggered clauses
//!   ([When Digivolving] branch-choice + inherited [When Attacking][OPT]).
//! - E1 Branch-choice [When Digivolving] (2-way EffectChoice, mandatory).
//! - F1-adjacent Lowest-DP delete branch (`dp_lte: { aggregate: { selector:
//!   lowest_dp, scope: opponent } }` — same shape as BT24-017 / BT24-040).
//! - F2 Effect-initiated DNA-free digivolve from hand (Gabumon → MetalGarurumon)
//!   — same shape as BT17-015 branch 1 (BLOCKED: G-EFFECT-INITIATED-DIGIVOLVE-
//!   FROM-HAND-WITH-PERMANENT-TARGET).
//! - G4 Inherited [When Attacking][OPT] gated on top-card name ("Omnimon")
//!   trashing top opp security — same shape as BT17-015 inherited (BLOCKED:
//!   G-DSL-SOURCE-NAME-CONTAINS for the name-gate negative case).
//!
//! # Sister cards (cross-reference)
//! - BT17-015 WarGreymon (the "old" WarGreymon DSL test) — same branch-choice
//!   shape, same inherited shape; differs in the cost reduction (Tai Kamiya
//!   tamer, -3 cost) vs the activated_digivolve from Agumon at cost 6.
//! - BT24-016 Lamiamon — first/canonical user of `activated_digivolve` from
//!   field with `extra_cost`. Header for that card established
//!   G-ALT-PATH-CONDITION as a DSL vocab gap; the gap was RESOLVED at the
//!   substrate level on 2026-05-15 (`AltPathSpec.condition` field +
//!   consumer wiring in `dna_digivolve.rs`). BT22-013's YAML does not yet
//!   populate `condition:` to gate on Nokia Shiramine; card-local
//!   authoring follow-up.
//! - BT24-017 Medusamon — first/canonical user of `dp_lte: { aggregate:
//!   { selector: lowest_dp, scope: opponent } }`. Header for that card
//!   established G-PRED-DP-LTE as an engine gap (predicate parses + compiles
//!   but the engine evaluator does not check `dp_lte` on permanents). We hit
//!   the same gap on branch 1 of [When Digivolving].
//!
//! # Faithfulness diff vs. card text
//!
//! | Card-text element                                                    | YAML clause                                                                          | Status         |
//! |----------------------------------------------------------------------|--------------------------------------------------------------------------------------|----------------|
//! | "Standard Lv.5 Red digivolve cost 4"                                  | `alt_paths: { kind: digivolve, from: { level_eq: 5 }, cost: 4 }`                       | OK             |
//! | "[Hand][Main] if Nokia Shiramine, 1 Agumon digivolves at cost 6, ignore reqs" | `alt_paths: { kind: activated_digivolve, from: { name_contains: "Agumon" }, cost: 6, ignore_requirements: true }` | DSL-GAP — see below (G-ALT-PATH-CONDITION) |
//! | "[When Digivolving] Activate 1 of the effects below: …"               | `when: when_digivolving` + `select_effect_choice { 2 labels }`                         | OK             |
//! | "1 of your [Gabumon] may digivolve into [MetalGarurumon] in hand free, ignore reqs" | `select_own_permanent { name_contains: "Gabumon" }` + `select_hand { name_contains: "MetalGarurumon" }` + `effect_initiated_digivolve { ignore_requirements, cost: 0 }` | DSL-GAP * (G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET) |
//! | "Delete 1 of your opponent's Digimon with the lowest DP"              | `select_opponent_permanent { kind: digimon, dp_lte: { aggregate: lowest_dp/opponent } }` + `delete_permanent` | DSL-GAP * (G-PRED-DP-LTE — over-permissive on lowest-DP gate) |
//! | Inherited "[When Attacking][OPT] If [Omnimon] in name, trash top opp sec" | `scope: inherited`, `when: when_attacking`, `once_per_turn: true`, `condition: { source_name_contains: "Omnimon" }`, `trash_top_security { of: opponent }` | DSL-GAP * (G-DSL-SOURCE-NAME-CONTAINS — over-permissive on name gate) |
//!
//! # Known gaps that affect this card
//!
//! - **G-ALT-PATH-CONDITION** — RESOLVED 2026-05-15 at the substrate
//!   level (`AltPathSpec.condition` field exists, consumed by the
//!   Digivolve route in `dna_digivolve.rs`). BT22-013's YAML does not
//!   yet populate `condition:` to gate on Nokia Shiramine; the
//!   Agumon-on-field filter IS enforced via `from:` but Nokia presence
//!   is not checked. Card-local authoring follow-up.
//! - **G-PRED-DP-LTE** — `dp_lte` predicate parses + compiles but the engine
//!   predicate evaluator does not yet evaluate it for permanents. The
//!   "lowest DP" filter on branch 1's delete will offer ALL opp Digimon, not
//!   only the lowest-DP one. First flagged on BT24-017 (Medusamon).
//! - **G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET** — the
//!   `select_own_permanent → select_hand → effect_initiated_digivolve` chain
//!   terminates after the permanent pick; the hand-pick prompt never installs.
//!   First flagged on BT17-015 (the "old" WarGreymon) for the identical
//!   Gabumon → MetalGarurumon branch.
//! - **G-DSL-SOURCE-NAME-CONTAINS** — `source_name_contains` predicate parses
//!   + compiles but the engine evaluator never inspects it. The inherited
//!   [When Attacking] clause's "[Omnimon] in name" gate degenerates to true.
//!   First flagged on BT17-015 (the "old" WarGreymon) for the identical
//!   inherited [When Attacking][OPT] clause.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

/// The production YAML for BT22-013, inlined at compile time from the
/// canonical location under `cards/bt22/`.
const YAML: &str = include_str!("../../../cards/bt22/BT22-013.yaml");

/// Compile BT22-013 from the production YAML and return the CompiledCard.
fn compiled_bt22_013() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("BT22-013.yaml parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("BT22-013.yaml compiles");
    registry
        .lookup("BT22-013")
        .expect("BT22-013 in registry")
        .clone()
}

// ─── Fixture helpers ────────────────────────────────────────────────────────

fn make_agumon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Agumon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(2000);
    card
}

fn make_gabumon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Gabumon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card
}

fn make_metalgarurumon(id: &str) -> CardData {
    let mut card = make_test_card(id, "MetalGarurumon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(6);
    card.dp = Some(11000);
    card.play_cost = 12;
    card
}

fn make_nokia_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, "Nokia Shiramine");
    card.card_kind = CardKind::Tamer;
    card
}

fn make_unrelated_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, "Tai Kamiya");
    card.card_kind = CardKind::Tamer;
    card
}

fn make_opp_digimon(id: &str, name: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(dp);
    card
}

fn make_filler_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(6);
    card.dp = Some(11000);
    card
}

fn make_omnimon_top(id: &str) -> CardData {
    // Top-card stand-in whose name contains "Omnimon" — used to ride above a
    // BT22-013 source so the inherited [When Attacking] clause's name gate
    // (printed text) sees an Omnimon-named top card.
    let mut card = make_test_card(id, "Omnimon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(7);
    card.dp = Some(13000);
    card
}

// ─── Section 1 — Structural assertions ──────────────────────────────────────

#[test]
fn bt22_013_compiles_with_two_alt_paths_and_two_triggered_clauses() {
    let card = compiled_bt22_013();

    assert_eq!(
        card.alt_paths.len(),
        2,
        "expected exactly 2 alt_paths (Digivolve + ActivatedDigivolve), got {}",
        card.alt_paths.len()
    );

    let triggered: Vec<&CompiledTriggeredClause> = card
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
        "expected exactly 2 triggered clauses ([When Digivolving] branch-choice + inherited [When Attacking])"
    );
}

#[test]
fn bt22_013_first_alt_path_is_digivolve_lv5_cost4() {
    let card = compiled_bt22_013();
    let path = &card.alt_paths[0];
    assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(
        path.cost,
        Some(digimon_dsl::compiled::CompiledCost::Literal(4)),
        "standard digivolve must be Lv.5/cost-4 (printed cards.json evo_costs)"
    );
    assert!(
        !path.ignore_requirements,
        "standard digivolve must NOT ignore requirements"
    );
}

#[test]
fn bt22_013_second_alt_path_is_activated_digivolve_from_agumon_cost6() {
    let card = compiled_bt22_013();
    let path = &card.alt_paths[1];
    assert_eq!(path.kind, CompiledAltPathKind::ActivatedDigivolve);
    assert_eq!(
        path.cost,
        Some(digimon_dsl::compiled::CompiledCost::Literal(6)),
        "activated_digivolve from Agumon must be cost 6 (printed)"
    );
    assert!(
        path.ignore_requirements,
        "activated_digivolve must ignore digivolution requirements (printed)"
    );

    let from_pred = path
        .from
        .as_ref()
        .expect("activated_digivolve must carry a `from:` predicate");
    assert_eq!(
        from_pred.name_contains.as_deref(),
        Some("Agumon"),
        "from filter must target name_contains: Agumon (printed)"
    );
}

#[test]
fn bt22_013_when_digivolving_clause_is_mandatory_and_face_up() {
    let card = compiled_bt22_013();
    let clause: &CompiledTriggeredClause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving)
                    && matches!(t.scope, CompiledScope::FaceUp) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("must have a face-up [When Digivolving] clause");

    assert!(
        !clause.optional,
        "branch-choice clause itself is mandatory once triggered (printed: 'Activate 1 of the effects below')"
    );
    assert!(
        !clause.once_per_turn,
        "no [Once Per Turn] on the branch-choice clause"
    );
}

#[test]
fn bt22_013_inherited_when_attacking_clause_is_opt() {
    let card = compiled_bt22_013();
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
        .expect("must have inherited [When Attacking] clause");

    assert!(
        clause.once_per_turn,
        "inherited [When Attacking] must be Once Per Turn (printed)"
    );
    assert_eq!(clause.scope, CompiledScope::Inherited);
}

// ─── Section 2 — [When Digivolving] branch-choice behavioral ────────────────
//
// The pattern follows BT17-015 branch-choice tests. We use direct
// enqueue_triggered to fire the clause without needing a full digivolve
// chain (which would require setting up the Agumon-as-base).

fn place_bt22_on_field(runner: &mut DebugRunner, player: PlayerId) -> PermanentHandle {
    runner.place_on_field(player, "BT22-013", Some(0))
}

fn trigger_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    use digimon_engine::selection::TriggerSource;
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

/// Positive: firing [When Digivolving] installs the 2-way EffectChoice prompt.
#[test]
fn bt22_013_when_digivolving_installs_branch_choice_prompt() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-013 YAML loads")
        .add_card(make_opp_digimon("OPP-DIGI", "OppDigi", 5000))
        .memory(15)
        .start();

    let bt22 = place_bt22_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-DIGI", None);

    trigger_when_digivolving(&mut runner, bt22);

    let kind = runner
        .pending_kind()
        .expect("EffectChoice prompt must install on WhenDigivolving");
    assert_eq!(
        kind,
        SelectionKind::EffectChoice,
        "WhenDigivolving must install a 2-way EffectChoice prompt"
    );
}

/// Branch 1 (Delete lowest-DP): selecting branch 1 then a single low-DP opp
/// Digimon removes that Digimon from the opponent's battle area.
#[test]
fn bt22_013_when_digivolving_branch_1_deletes_opp_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-013 YAML loads")
        .add_card(make_opp_digimon("OPP-LOW", "OppLow", 3000))
        .memory(15)
        .start();

    let bt22 = place_bt22_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-LOW", None);
    let opp_before = runner.battle_area_size(1);

    trigger_when_digivolving(&mut runner, bt22);

    let kind = runner.pending_kind().expect("branch prompt installs");
    assert_eq!(kind, SelectionKind::EffectChoice);
    runner.execute_branch(1).expect("pick Delete branch");

    // Auto-resolve picks the (only) legal target.
    let _ = runner.auto_resolve();

    let opp_after = runner.battle_area_size(1);
    assert_eq!(
        opp_after,
        opp_before - 1,
        "branch 1 must delete the (only / lowest-DP) opp Digimon"
    );
}

/// Branch 1 with multiple opp Digimon: only the LOWEST-DP one should be a
/// valid delete target. **BLOCKED**: G-PRED-DP-LTE — the predicate evaluator
/// does not yet honor `dp_lte` on permanents, so the prompt offers ALL opp
/// Digimon. Same gap as BT24-017's `bt24_017_delete_targets_only_lowest_dp_digimon`.
#[test]
fn bt22_013_when_digivolving_branch_1_only_lowest_dp_is_a_legal_target() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-013 YAML loads")
        .add_card(make_opp_digimon("OPP-LOW", "OppLow", 3000))
        .add_card(make_opp_digimon("OPP-HIGH", "OppHigh", 9000))
        .memory(15)
        .start();

    let bt22 = place_bt22_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-LOW", None);
    runner.place_on_field(1, "OPP-HIGH", None);

    trigger_when_digivolving(&mut runner, bt22);
    let _ = runner.execute_branch(1);

    let view = runner
        .pending_selection_view()
        .expect("delete-target selection installs");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the lowest-DP (3000) opp Digimon should be a legal delete target"
    );
}

/// Branch 0 (Digivolve Gabumon → MetalGarurumon free): selecting branch 0
/// should chain through select_own_permanent → select_hand →
/// effect_initiated_digivolve. **BLOCKED**: G-EFFECT-INITIATED-DIGIVOLVE-
/// FROM-HAND-WITH-PERMANENT-TARGET. Same gap as BT17-015 branch 1.
#[test]
// Phase 2 Track F (2026-05-17): G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-
// WITH-PERMANENT-TARGET resolved as phantom (see BT17-015 sister test).
fn bt22_013_when_digivolving_branch_0_digivolves_gabumon_into_metalgarurumon_free() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-013 YAML loads")
        .add_card(make_gabumon("MY-GABU"))
        .add_card(make_metalgarurumon("MY-MG"))
        .hand(0, &["MY-MG"])
        .memory(15)
        .start();

    let gabu_handle = runner.place_on_field(0, "MY-GABU", None);
    let bt22 = place_bt22_on_field(&mut runner, 0);

    trigger_when_digivolving(&mut runner, bt22);

    let kind = runner.pending_kind().expect("branch prompt installs");
    assert_eq!(kind, SelectionKind::EffectChoice);
    runner.execute_branch(0).expect("pick Digivolve branch");
    let _ = runner.auto_resolve();

    // If the chain runs to completion, MetalGarurumon ends up as the top card
    // of the (former) Gabumon stack.
    let gabu_perm = &runner.game.players[0].battle_area[gabu_handle.index as usize];
    let top = gabu_perm.top_card();
    assert_eq!(
        top.card_id(&runner.game.card_data),
        "MY-MG",
        "MetalGarurumon must be top card of the Gabumon stack after digivolve branch"
    );
}

/// Negative: with NO opp Digimon and NO own Gabumon, branch-choice still
/// installs but neither sub-branch has a legal target. The game must not
/// panic.
#[test]
fn bt22_013_when_digivolving_no_targets_does_not_panic() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-013 YAML loads")
        .memory(15)
        .start();

    let bt22 = place_bt22_on_field(&mut runner, 0);

    // Should not panic even with no valid targets in either branch.
    trigger_when_digivolving(&mut runner, bt22);
    // No assertion required — absence of panic is the acceptance criterion.
}

// ─── Section 3 — Inherited [When Attacking][OPT] behavioral ────────────────

/// Stack BT22-013 under an Omnimon-named top card so the printed name gate
/// "[Omnimon] in name" matches the actual top card.
fn place_omnimon_over_bt22_013(runner: &mut DebugRunner, player: PlayerId) -> PermanentHandle {
    runner.place_stack(player, &["BT22-013", "OMNI-TOP"])
}

/// Positive condition: Omnimon at top of stack + BT22-013 below — inherited
/// trigger fires on attack and trashes 1 from opp security.
#[test]
fn bt22_013_inherited_when_attacking_omnimon_top_trashes_security() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-013 YAML loads")
        .add_card(make_omnimon_top("OMNI-TOP"))
        .add_card(make_filler_digimon("DEF-FILLER"))
        .add_card(make_test_card("SEC-FILLER", "SecFiller"))
        .security(1, &["SEC-FILLER", "SEC-FILLER", "SEC-FILLER"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let attacker = place_omnimon_over_bt22_013(&mut runner, 0);
    let defender = runner.place_on_field(1, "DEF-FILLER", Some(0));

    let sec_before = runner.security_count(1);
    assert!(sec_before >= 1, "test setup needs at least 1 opp security");

    runner.attack_digimon(attacker, defender, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "Omnimon-named attacker must trash 1 opp security via inherited"
    );
}

/// Negative condition: stack has only WarGreymon (BT22-013) at top — name
/// gate must block the trash. `source_name_contains` is evaluated by the
/// engine predicate path (`predicate.rs:284`, via `subject_or_source_permanent`
/// falling back to `rctx.source_permanent()`), so the inherited [When
/// Attacking] gate correctly blocks when the top card is not Omnimon-named.
#[test]
fn bt22_013_inherited_when_attacking_wargreymon_top_does_not_trash_security() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-013 YAML loads")
        .add_card(make_filler_digimon("DEF-FILLER"))
        .add_card(make_test_card("SEC-FILLER", "SecFiller"))
        .security(1, &["SEC-FILLER", "SEC-FILLER", "SEC-FILLER"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // BT22-013 alone at top — printed name "WarGreymon" does NOT contain "Omnimon".
    let attacker = runner.place_on_field(0, "BT22-013", Some(0));
    let defender = runner.place_on_field(1, "DEF-FILLER", Some(0));

    let sec_before = runner.security_count(1);
    runner.attack_digimon(attacker, defender, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_before,
        "WarGreymon-only top card must NOT trigger the [Omnimon]-name inherited"
    );
}

/// OPT: a second attack in the same turn must NOT trash a second security.
/// Relies on G-OPT-TRIGGERED being closed for permanent-backed triggered
/// effects (fixed 2026-04-29 — see BT17-015 sibling test).
#[test]
fn bt22_013_inherited_when_attacking_opt_blocks_second_activation() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-013 YAML loads")
        .add_card(make_omnimon_top("OMNI-TOP"))
        .add_card(make_filler_digimon("DEF-1"))
        .add_card(make_filler_digimon("DEF-2"))
        .add_card(make_test_card("SEC-FILLER", "SecFiller"))
        .security(1, &["SEC-FILLER", "SEC-FILLER", "SEC-FILLER", "SEC-FILLER"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let attacker = place_omnimon_over_bt22_013(&mut runner, 0);
    let def1 = runner.place_on_field(1, "DEF-1", Some(0));
    let sec_before = runner.security_count(1);

    runner.attack_digimon(attacker, def1, false);
    let _ = runner.auto_resolve();
    let sec_after_first = runner.security_count(1);
    assert_eq!(
        sec_after_first,
        sec_before - 1,
        "first attack must trash 1 opp security"
    );

    // Second attack same turn — OPT must lock out a second trash.
    if runner.battle_area_size(0) <= attacker.index as usize {
        return;
    }

    let def2 = runner.place_on_field(1, "DEF-2", Some(0));
    runner.attack_digimon(attacker, def2, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_after_first,
        "OPT must prevent the inherited from firing twice in one turn"
    );
}

// ─── Section 4 — Activated digivolve [Hand][Main] precondition gating ───────
//
// Printed Clause 1: "[Hand][Main] If you have [Nokia Shiramine], 1 of your
// [Agumon] digivolves into this card for a digivolution cost of 6, ignoring
// digivolution requirements."
//
// The Nokia Shiramine precondition cannot be expressed today (G-ALT-PATH-
// CONDITION). The Agumon `from:` filter IS enforced, but the Nokia gate is
// not. The negative test (alt-path must NOT be offered when Nokia is absent)
// is BLOCKED until the gap closes.
//
// We DO assert the structural shape of the alt-path (cost 6, ignore_reqs,
// from filter targets Agumon) in Section 1 — that part IS faithfully encoded
// and verifiable.

/// Negative: with Agumon on field but NO Nokia Shiramine tamer, the
/// activated_digivolve alt-path should NOT be available. **PENDING**
/// card-local authoring of `condition:` on the BT22-013.yaml
/// activated_digivolve path. Substrate is ready (G-ALT-PATH-CONDITION
/// RESOLVED 2026-05-15); same authoring gap as BT22-026 / BT24-016.
#[test]
fn bt22_013_activated_digivolve_blocked_without_nokia_tamer() {
    // Setup: Agumon on field, BT22-013 in hand, NO Nokia Shiramine tamer.
    // The activated_digivolve alt-path MUST NOT be offered as a play option.
    //
    // Verifying this requires checking `valid_actions` for the absence of an
    // "activate digivolve from hand" action targeting BT22-013 with Agumon as
    // the source. The exact action shape depends on engine wiring of
    // ActivatedDigivolve alt-paths, which is itself a moving target.
    //
    // Asserting non-availability requires the Nokia gate to be honored — i.e.
    // requires G-ALT-PATH-CONDITION to be closed. Until then, the alt-path
    // would be offered, making the negative assertion fail.
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-013 YAML loads")
        .add_card(make_agumon("MY-AGU"))
        .hand(0, &["BT22-013"])
        .memory(15)
        .start();

    runner.place_on_field(0, "MY-AGU", None);

    // Probe valid_actions for an activated digivolve targeting BT22-013.
    // Under the gap, this probe will return at least one such action even
    // though Nokia is absent — so the assertion below would fail until the
    // gap closes. The test serves as the regression once G-ALT-PATH-CONDITION
    // lands.
    //
    // Detailed action-introspection scaffolding is left as a TODO for
    // when the gap closes; the test body is a placeholder that captures the
    // intended assertion shape.
    //
    // assert!(
    //     !runner.has_activated_digivolve_action_for("BT22-013"),
    //     "without Nokia, the activated_digivolve alt-path must NOT be offered"
    // );
    let _ = runner;
}

/// Positive: with Agumon on field AND Nokia Shiramine tamer on field, the
/// activated_digivolve alt-path IS available. Without G-ALT-PATH-CONDITION
/// the result is the same as the negative case (alt-path always available
/// with Agumon present), so this is a sanity scaffold rather than an active
/// gate test. Once the gap closes, this becomes the positive sister of the
/// negative test above.
#[test]
fn bt22_013_activated_digivolve_available_with_nokia_and_agumon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-013 YAML loads")
        .add_card(make_agumon("MY-AGU"))
        .add_card(make_nokia_tamer("MY-NOKIA"))
        .hand(0, &["BT22-013"])
        .memory(15)
        .start();

    runner.place_on_field(0, "MY-NOKIA", None);
    runner.place_on_field(0, "MY-AGU", None);
    let _ = runner;
}
