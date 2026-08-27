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
//! - Structural: 1 alt_path (Digivolve Lv.5/cost-4) + 3 triggered clauses
//!   (main_from_hand Nokia jump + [When Digivolving] branch-choice + inherited
//!   [When Attacking][OPT]).
//! - [Hand][Main] Nokia jump (`when: main_from_hand`): the Nokia Shiramine +
//!   Agumon-target `condition:` gate → select Agumon → effect_initiated_
//!   digivolve (cost 6, ignore reqs). Re-modelled off the retired
//!   `activated_digivolve` alt-path (G-ACTIVATED-DIGIVOLVE-EXECUTION). Mirrors
//!   BT24-016 Lamiamon clause 1. Driven through `activate_hand_main`.
//! - E1 Branch-choice [When Digivolving] (2-way EffectChoice, mandatory).
//! - F1-adjacent Lowest-DP delete branch (`dp_lte: { aggregate: { selector:
//!   lowest_dp, scope: opponent } }` — same shape as BT24-017 / BT24-040).
//! - F2 Effect-initiated DNA-free digivolve from hand (Gabumon → MetalGarurumon)
//!   — same shape as BT17-015 branch 1. G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-
//!   WITH-PERMANENT-TARGET is RESOLVED: the full chain runs, test is active and green.
//! - G4 Inherited [When Attacking][OPT] gated on top-card name ("Omnimon")
//!   trashing top opp security — same shape as BT17-015 inherited.
//!   G-DSL-SOURCE-NAME-CONTAINS is RESOLVED: name-gate negative case exercised,
//!   test is active and green.
//!
//! # Sister cards (cross-reference)
//! - BT17-015 WarGreymon (the "old" WarGreymon DSL test) — same branch-choice
//!   shape, same inherited shape; differs in the cost reduction (Tai Kamiya
//!   tamer, -3 cost) vs the activated_digivolve from Agumon at cost 6.
//! - BT24-016 Lamiamon — the canonical precedent for re-modelling a
//!   `[Hand][Main]` "If you have <Tamer>, … digivolves into this card"
//!   jump as a `when: main_from_hand` clause whose `condition:` enforces the
//!   tamer precondition (Owen Dreadnought there; Nokia Shiramine here) and
//!   whose body runs `select_own_permanent → effect_initiated_digivolve
//!   { from_hand: self, ignore_requirements }`. BT22-013's [Hand][Main] Nokia
//!   jump is modelled identically.
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
//! | "[Hand][Main] if Nokia Shiramine, 1 Agumon digivolves at cost 6, ignore reqs" | `when: main_from_hand` + `condition: { Nokia on field, Agumon target }` + `select_own_permanent { name_contains: "Agumon" }` + `effect_initiated_digivolve { from_hand: self, cost: 6, ignore_requirements: true }` | OK (re-modelled off retired activated_digivolve — G-ACTIVATED-DIGIVOLVE-EXECUTION) |
//! | "[When Digivolving] Activate 1 of the effects below: …"               | `when: when_digivolving` + `select_effect_choice { 2 labels }`                         | OK             |
//! | "1 of your [Gabumon] may digivolve into [MetalGarurumon] in hand free, ignore reqs" | `select_own_permanent { name_contains: "Gabumon" }` + `select_hand { name_contains: "MetalGarurumon" }` + `effect_initiated_digivolve { ignore_requirements, cost: 0 }` | OK (G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET RESOLVED) |
//! | "Delete 1 of your opponent's Digimon with the lowest DP"              | `select_opponent_permanent { kind: digimon, dp_lte: { aggregate: lowest_dp/opponent } }` + `delete_permanent` | OK (G-PRED-DP-LTE RESOLVED — lowest-DP filter honored) |
//! | Inherited "[When Attacking][OPT] If [Omnimon] in name, trash top opp sec" | `scope: inherited`, `when: when_attacking`, `once_per_turn: true`, `condition: { source_name_contains: "Omnimon" }`, `trash_top_security { of: opponent }` | OK (G-DSL-SOURCE-NAME-CONTAINS RESOLVED — name gate enforced) |
//!
//! # Gap status for this card (all resolved)
//!
//! - **G-ACTIVATED-DIGIVOLVE-EXECUTION** — CLOSED for this card by re-model.
//!   The `kind: activated_digivolve` alt-path had no engine execution route,
//!   so the [Hand][Main] Nokia jump is now a `when: main_from_hand` clause
//!   (see §5). The Nokia precondition — which an alt-path `condition:` could
//!   not enforce on the mask — is now the clause `condition:`.
//! - **G-PRED-DP-LTE** — RESOLVED (qa/resolved-gaps.md). `dp_lte` on
//!   permanents is now evaluated by the engine predicate evaluator. The
//!   branch 1 delete filters to only the lowest-DP opponent Digimon, and the
//!   test asserting this behavior is active and green. First flagged on
//!   BT24-017 (Medusamon).
//! - **G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET** —
//!   RESOLVED (qa/resolved-gaps.md). The `select_own_permanent → select_hand
//!   → effect_initiated_digivolve` chain now runs to completion; the hand-pick
//!   prompt installs after the permanent pick and the digivolve executes.
//!   Branch 0 test is active and green. First flagged on BT17-015.
//! - **G-DSL-SOURCE-NAME-CONTAINS** — RESOLVED (qa/resolved-gaps.md).
//!   `source_name_contains` is evaluated by the engine predicate path
//!   (predicate.rs, via subject_or_source_permanent). The inherited [When
//!   Attacking] clause's "[Omnimon] in name" gate correctly blocks when the
//!   top card is not Omnimon-named. Negative test is active and green. First
//!   flagged on BT17-015.

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
fn bt22_013_compiles_with_three_alt_paths_and_three_triggered_clauses() {
    let card = compiled_bt22_013();

    // Printed alt-digivolutions (confirmed on the card image + official Bandai DB,
    // promote-official-bandai-card-source): the standard Lv.5/cost-4 Digivolve PLUS the
    // printed "[Digivolve] Lv.5 w/[Greymon] in name or w/[CS] trait: Cost 3" alt, encoded
    // as a name route + a trait route. (The earlier "cost-3 is unprinted DCGO-only"
    // reasoning was the lossy-cards.json error this change corrects — the card face shows
    // the cost-3 box.) The [Hand][Main] Nokia jump remains a separate main_from_hand clause.
    assert_eq!(
        card.alt_paths.len(),
        3,
        "expected 3 alt_paths (standard Digivolve + Greymon-name + CS-trait), got {}",
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
        3,
        "expected exactly 3 triggered clauses (main_from_hand Nokia jump + [When Digivolving] branch-choice + inherited [When Attacking])"
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

/// The [Hand][Main] Nokia jump is now a `main_from_hand` triggered clause (NOT
/// an `activated_digivolve` alt-path — that kind had no engine execution route,
/// G-ACTIVATED-DIGIVOLVE-EXECUTION). It carries a `condition:` (the Nokia
/// Shiramine + Agumon-target gate) and is a face-up own effect. Mirrors
/// BT24-016 Lamiamon clause 1.
#[test]
fn bt22_013_has_main_from_hand_nokia_jump_clause() {
    let card = compiled_bt22_013();
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("must have a main_from_hand Nokia jump clause");
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "the Nokia jump is a face-up own effect"
    );
    assert!(
        clause.condition.is_some(),
        "the Nokia jump must carry a condition (the Nokia Shiramine + Agumon gate)"
    );

    // The retired `activated_digivolve` alt-path must be gone — only the
    // standard Digivolve alt-path remains.
    assert!(
        card.alt_paths
            .iter()
            .all(|p| p.kind == CompiledAltPathKind::Digivolve),
        "no ActivatedDigivolve alt-path may remain after the re-model"
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

/// Branch 1 with multiple opp Digimon: only the LOWEST-DP one is a valid
/// delete target. G-PRED-DP-LTE is RESOLVED (qa/resolved-gaps.md): `dp_lte`
/// is now evaluated by the engine, so the prompt correctly filters to only the
/// lowest-DP Digimon. Same gap pattern as BT24-017's lowest-DP delete test.
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
/// chains through select_own_permanent → select_hand →
/// effect_initiated_digivolve. G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-
/// PERMANENT-TARGET is RESOLVED (qa/resolved-gaps.md): the full chain runs to
/// completion. Same gap pattern as BT17-015 branch 1.
#[test]
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

// ─── Section 4 — [Hand][Main] Nokia jump (main_from_hand) behavioral ─────────
//
// Printed Clause 1: "[Hand][Main] If you have [Nokia Shiramine], 1 of your
// [Agumon] digivolves into this card for a digivolution cost of 6, ignoring
// digivolution requirements."
//
// Re-modelled (Task A1, gap-closure plan) from the retired `kind:
// activated_digivolve` alt-path (which had no engine execution route —
// G-ACTIVATED-DIGIVOLVE-EXECUTION) onto a `when: main_from_hand` triggered
// clause, EXACTLY mirroring BT24-016 Lamiamon clause 1. The engine masks a
// Hand [Main] action for any hand card with a `MainFromHand` effect whose
// `condition:` passes; `activate_hand_main` runs it. The Nokia "If you have
// [Nokia Shiramine]" precondition — which could NOT be expressed on an
// alt-path — IS expressible as the `main_from_hand` `condition:` gate.
//
// Uses a REAL implemented Agumon (BT22-008, same set/color) as the digivolve
// base so the body exercises `effect_initiated_digivolve { from_hand: self }`
// against a production card, not a synthetic stand-in. (BT22-008's own
// `[On Play]` / inherited clauses do not fire here — the Agumon is placed
// directly on the field, not played, so neither interferes.)

/// Positive: Nokia Shiramine + a real [Agumon] (BT22-008) on field, BT22-013
/// in hand → activating the [Hand][Main] effect digivolves WarGreymon onto the
/// Agumon at cost 6, ignoring requirements. The Agumon stack's top card becomes
/// BT22-013 and WarGreymon leaves the hand.
#[test]
fn bt22_013_hand_main_jump_digivolves_agumon_at_cost6_with_nokia() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-013 YAML loads")
        .dsl_card("BT22-008")
        .expect("BT22-008 Agumon in embedded DSL pack")
        .add_card(make_nokia_tamer("MY-NOKIA"))
        .add_card(make_filler_digimon("FILL"))
        .hand(0, &["BT22-013"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .memory(15)
        .start();

    // Nokia Shiramine (Tamer) + a real Agumon (BT22-008) on player 0's field.
    runner.place_on_field(0, "MY-NOKIA", Some(0));
    runner.place_on_field(0, "BT22-008", Some(0));

    let mem_before = runner.memory();
    assert!(
        runner.game.activate_hand_main(0, 0),
        "the [Hand][Main] Nokia jump must fire (Nokia + Agumon present)"
    );

    // The body selects which Agumon to digivolve into.
    let view = runner
        .pending_selection_view()
        .expect("the Agumon select prompt must install");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("select the Agumon");
    let _ = runner.auto_resolve();

    // WarGreymon (BT22-013) digivolved onto the Agumon permanent.
    let agumon_perm = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| {
            p.card_sources
                .iter()
                .any(|s| s.card_id(&runner.game.card_data) == "BT22-008")
        })
        .expect("the Agumon permanent must still be on the field");
    assert_eq!(
        agumon_perm.top_card().card_id(&runner.game.card_data),
        "BT22-013",
        "WarGreymon must be the top card of the Agumon stack after the [Hand][Main] jump"
    );
    // §8-1-3-3: digivolving draws 1 card, so the hand SIZE is unchanged --
    // assert WarGreymon's departure by identity instead of by arithmetic.
    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "BT22-013"),
        "WarGreymon must leave the hand after digivolving onto the Agumon"
    );
    assert_eq!(
        runner.hand_size(0),
        1,
        "the digivolve draw replaced it (§8-1-3-3)"
    );
    // The cost-6 digivolve must actually deduct 6 memory — assert the delta so
    // the test is robust to any starting-memory clamp. A silently-ignored cost
    // would leave the delta at 0.
    assert_eq!(
        mem_before - runner.memory(),
        6,
        "the [Hand][Main] jump must pay digivolution cost 6 (before={}, after={})",
        mem_before,
        runner.memory(),
    );
}

/// Condition gate (Nokia absent): with a real [Agumon] on field but NO Nokia
/// Shiramine, the masked [Hand][Main] action is NOT offered —
/// `activate_hand_main` does not fire, and no digivolve happens.
#[test]
fn bt22_013_hand_main_jump_not_offered_without_nokia() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-013 YAML loads")
        .dsl_card("BT22-008")
        .expect("BT22-008 Agumon in embedded DSL pack")
        .add_card(make_filler_digimon("FILL"))
        .hand(0, &["BT22-013"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .memory(15)
        .start();

    // Agumon on field, but NO Nokia Shiramine — the Nokia gate must block the
    // masked Hand [Main] action.
    runner.place_on_field(0, "BT22-008", Some(0));

    assert!(
        !runner.game.activate_hand_main(0, 0),
        "without Nokia Shiramine the [Hand][Main] condition fails — the jump must not fire"
    );
    assert!(
        runner.game.pending_selection.is_none(),
        "no selection installs when the Nokia-gated [Hand][Main] jump is not offered"
    );

    // The Agumon stack must be untouched (WarGreymon never digivolved onto it).
    let agumon_perm = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| {
            p.card_sources
                .iter()
                .any(|s| s.card_id(&runner.game.card_data) == "BT22-008")
        })
        .expect("the Agumon permanent must still be on the field");
    assert_eq!(
        agumon_perm.top_card().card_id(&runner.game.card_data),
        "BT22-008",
        "no digivolve — the Agumon must still be the top card of its own stack"
    );
    assert_eq!(
        runner.hand_size(0),
        1,
        "WarGreymon must remain in hand — the jump was not offered"
    );
}
