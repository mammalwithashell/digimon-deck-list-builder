//! BT22-026 MetalGarurumon — Digimon, Lv.6, Blue, DP 12000, Cost 12.
//! Traits: Cyborg, CS.
//! Attribute: Data
//! Evo: Lv.5 Blue / cost 4
//!
//! # Card text (cards.json — verbatim)
//!
//! [Hand] [Main] If you have [Nokia Shiramine], 1 of your [Gabumon]
//! digivolves into this card for a digivolution cost of 6, ignoring
//! digivolution requirements.
//!
//! [When Digivolving] Activate 1 of the effects below:
//!   ・1 of your [Agumon] may digivolve into [WarGreymon] in the hand,
//!     ignoring digivolution requirements and without paying the cost.
//!   ・Return 1 of your opponent's Digimon with the lowest level to the hand.
//!
//! Inherited Effect [When Attacking] [Once Per Turn]
//!   If this Digimon has [Omnimon] in its name, it unsuspends.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT22/Blue/BT22_026.cs
//!
//! # Patterns this test covers
//! - Structural: 1 alt_path (Digivolve Lv.5/cost-4) + 3 triggered clauses
//!   (main_from_hand Nokia jump + [When Digivolving] branch-choice + inherited
//!   [When Attacking][OPT]).
//! - [Hand][Main] Nokia jump (`when: main_from_hand`): the Nokia Shiramine +
//!   Gabumon-target `condition:` gate → select Gabumon → effect_initiated_
//!   digivolve (cost 6, ignore reqs). Re-modelled off the retired
//!   `activated_digivolve` alt-path (G-ACTIVATED-DIGIVOLVE-EXECUTION). Mirrors
//!   BT24-016 Lamiamon clause 1 / BT22-013. Driven through `activate_hand_main`.
//! - Branch-choice [When Digivolving] (2-way EffectChoice, mandatory).
//! - Lowest-level bounce branch (`level_matches_aggregate { selector:
//!   lowest_level, of: opponent }` — same shape as AD1-012 / BT24-040).
//!   This predicate IS evaluated by the engine — NOT blocked.
//! - Effect-initiated digivolve from hand (Agumon → WarGreymon)
//!   — same shape as BT17-015 branch 1 / BT22-013 branch 0.
//!   G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET is RESOLVED:
//!   the full chain runs, test is active and green.
//! - Inherited [When Attacking][OPT] gated on top-card name ("Omnimon")
//!   unsuspending self — same shape as BT22-013 / BT17-015 inherited but
//!   body swapped from trash-security to unsuspend-self.
//!   G-DSL-SOURCE-NAME-CONTAINS is RESOLVED: name-gate negative case
//!   exercised, test is active and green.
//!
//! # Sister cards (cross-reference)
//! - BT22-013 WarGreymon — direct Red mirror; same overall layout, different
//!   color/trait/name swaps. Branch 1 differs: BT22-013 deletes lowest DP
//!   (G-PRED-DP-LTE RESOLVED), BT22-026 bounces lowest level (also active).
//!   Inherited body differs: BT22-013 trashes opp top security; BT22-026
//!   unsuspends self.
//! - BT17-015 WarGreymon (the "old" WarGreymon DSL test) — same branch-choice
//!   shape, same inherited shape; differs in cost reduction (Tai Kamiya
//!   tamer, -3 cost) vs the [Hand][Main] jump from Gabumon at cost 6.
//! - BT24-016 Lamiamon — the canonical precedent for re-modelling a
//!   `[Hand][Main]` "If you have <Tamer>, … digivolves into this card"
//!   jump as a `when: main_from_hand` clause whose `condition:` enforces the
//!   tamer precondition (Owen Dreadnought there; Nokia Shiramine here) and
//!   whose body runs `select_own_permanent → effect_initiated_digivolve
//!   { from_hand: self, ignore_requirements }`. BT22-026's [Hand][Main] Nokia
//!   jump is modelled identically (Gabumon target here vs Agumon on BT22-013).
//! - AD1-012 CresGarurumon — first/canonical user of `level_matches_aggregate
//!   { selector: lowest_level, of: opponent }`. BT22-026 branch 1 reuses
//!   this exact pattern.
//!
//! # Faithfulness diff vs. card text
//!
//! | Card-text element                                                    | YAML clause                                                                          | Status         |
//! |----------------------------------------------------------------------|--------------------------------------------------------------------------------------|----------------|
//! | "Standard Lv.5 Blue digivolve cost 4"                                  | `alt_paths: { kind: digivolve, from: { level_eq: 5 }, cost: 4 }`                       | OK             |
//! | "[Hand][Main] if Nokia Shiramine, 1 Gabumon digivolves at cost 6, ignore reqs" | `when: main_from_hand` + `condition: { Nokia on field, Gabumon target }` + `select_own_permanent { name_contains: "Gabumon" }` + `effect_initiated_digivolve { from_hand: self, cost: 6, ignore_requirements: true }` | OK (re-modelled off retired activated_digivolve — G-ACTIVATED-DIGIVOLVE-EXECUTION) |
//! | "[When Digivolving] Activate 1 of the effects below: …"               | `when: when_digivolving` + `select_effect_choice { 2 labels }`                         | OK             |
//! | "1 of your [Agumon] may digivolve into [WarGreymon] in hand free, ignore reqs" | `select_own_permanent { name_contains: "Agumon" }` + `select_hand { name_contains: "WarGreymon" }` + `effect_initiated_digivolve { ignore_requirements, cost: 0 }` | OK (G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET RESOLVED) |
//! | "Return 1 of your opponent's Digimon with the lowest level to the hand" | `select_opponent_permanent { kind: digimon, level_matches_aggregate { lowest_level/opponent } }` + `return_to_hand` | OK |
//! | Inherited "[When Attacking][OPT] If [Omnimon] in name, it unsuspends" | `scope: inherited`, `when: when_attacking`, `once_per_turn: true`, `condition: { source_name_contains: "Omnimon" }`, `unsuspend { target: source }` | OK (G-DSL-SOURCE-NAME-CONTAINS RESOLVED — name gate enforced) |
//!
//! # Gap status for this card (all resolved)
//!
//! - **G-ACTIVATED-DIGIVOLVE-EXECUTION** — CLOSED for this card by re-model.
//!   The `kind: activated_digivolve` alt-path had no engine execution route,
//!   so the [Hand][Main] Nokia jump is now a `when: main_from_hand` clause
//!   (see §4). The Nokia precondition — which an alt-path `condition:` could
//!   not enforce on the mask — is now the clause `condition:` (Nokia Shiramine
//!   on field AND a [Gabumon] target present).
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

/// The production YAML for BT22-026, inlined at compile time from the
/// canonical location under `cards/bt22/`.
const YAML: &str = include_str!("../../../cards/bt22/BT22-026.yaml");

/// Compile BT22-026 from the production YAML and return the CompiledCard.
fn compiled_bt22_026() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("BT22-026.yaml parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("BT22-026.yaml compiles");
    registry
        .lookup("BT22-026")
        .expect("BT22-026 in registry")
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

fn make_wargreymon(id: &str) -> CardData {
    let mut card = make_test_card(id, "WarGreymon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(6);
    card.dp = Some(12000);
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

fn make_opp_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
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
    // BT22-026 source so the inherited [When Attacking] clause's name gate
    // (printed text) sees an Omnimon-named top card.
    let mut card = make_test_card(id, "Omnimon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(7);
    card.dp = Some(13000);
    card
}

// ─── Section 1 — Structural assertions ──────────────────────────────────────

#[test]
fn bt22_026_compiles_with_three_alt_paths_and_three_triggered_clauses() {
    let card = compiled_bt22_026();

    // Printed alt-digivolutions (confirmed on the card image + official Bandai DB,
    // promote-official-bandai-card-source): the standard Lv.5/cost-4 Digivolve PLUS the
    // printed "[Digivolve] Lv.5 w/[Garurumon] in name or w/[CS] trait: Cost 3" alt, encoded
    // as a name route + a trait route. (The earlier "cost-3 is unprinted DCGO-only"
    // reasoning was the lossy-cards.json error this change corrects.) The [Hand][Main]
    // Nokia jump remains a separate main_from_hand clause.
    assert_eq!(
        card.alt_paths.len(),
        3,
        "expected 3 alt_paths (standard Digivolve + Garurumon-name + CS-trait), got {}",
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
fn bt22_026_first_alt_path_is_digivolve_lv5_cost4() {
    let card = compiled_bt22_026();
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
/// Shiramine + Gabumon-target gate) and is a face-up own effect. Mirrors
/// BT24-016 Lamiamon clause 1 / BT22-013.
#[test]
fn bt22_026_has_main_from_hand_nokia_jump_clause() {
    let card = compiled_bt22_026();
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
        "the Nokia jump must carry a condition (the Nokia Shiramine + Gabumon gate)"
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
fn bt22_026_when_digivolving_clause_is_mandatory_and_face_up() {
    let card = compiled_bt22_026();
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
fn bt22_026_inherited_when_attacking_clause_is_opt() {
    let card = compiled_bt22_026();
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
// The pattern follows BT17-015 / BT22-013 branch-choice tests. We use
// direct enqueue_triggered to fire the clause without needing a full
// digivolve chain (which would require setting up the Gabumon-as-base).

fn place_bt22_on_field(runner: &mut DebugRunner, player: PlayerId) -> PermanentHandle {
    runner.place_on_field(player, "BT22-026", Some(0))
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
fn bt22_026_when_digivolving_installs_branch_choice_prompt() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-026 YAML loads")
        .add_card(make_opp_digimon("OPP-DIGI", "OppDigi", 5, 5000))
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

/// Branch 1 (Return lowest-level): selecting branch 1 then a single low-level
/// opp Digimon returns it to the opponent's hand.
#[test]
fn bt22_026_when_digivolving_branch_1_returns_opp_digimon_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-026 YAML loads")
        .add_card(make_opp_digimon("OPP-LOW", "OppLow", 3, 3000))
        .memory(15)
        .start();

    let bt22 = place_bt22_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-LOW", None);
    let opp_field_before = runner.battle_area_size(1);
    let opp_hand_before = runner.game.players[1].hand.len();

    trigger_when_digivolving(&mut runner, bt22);

    let kind = runner.pending_kind().expect("branch prompt installs");
    assert_eq!(kind, SelectionKind::EffectChoice);
    runner.execute_branch(1).expect("pick Bounce branch");

    // Auto-resolve picks the (only) legal target.
    let _ = runner.auto_resolve();

    let opp_field_after = runner.battle_area_size(1);
    let opp_hand_after = runner.game.players[1].hand.len();
    assert_eq!(
        opp_field_after,
        opp_field_before - 1,
        "branch 1 must remove the (only / lowest-level) opp Digimon from field"
    );
    assert_eq!(
        opp_hand_after,
        opp_hand_before + 1,
        "branch 1 must return the picked Digimon to the opponent's hand"
    );
}

/// Branch 1 with multiple opp Digimon: only the LOWEST-LEVEL one should be
/// a valid bounce target. This uses `level_matches_aggregate { selector:
/// lowest_level, of: opponent }` which IS evaluated by the engine — same
/// pattern as AD1-012 and BT24-040.
#[test]
fn bt22_026_when_digivolving_branch_1_only_lowest_level_is_a_legal_target() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-026 YAML loads")
        .add_card(make_opp_digimon("OPP-LOW", "OppLow", 3, 3000))
        .add_card(make_opp_digimon("OPP-HIGH", "OppHigh", 6, 11000))
        .memory(15)
        .start();

    let bt22 = place_bt22_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-LOW", None);
    runner.place_on_field(1, "OPP-HIGH", None);

    trigger_when_digivolving(&mut runner, bt22);
    let _ = runner.execute_branch(1);

    let view = runner
        .pending_selection_view()
        .expect("bounce-target selection installs");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the lowest-level (Lv.3) opp Digimon should be a legal bounce target"
    );
}

/// Branch 0 (Digivolve Agumon → WarGreymon free): selecting branch 0 chains
/// through select_own_permanent → select_hand → effect_initiated_digivolve.
/// G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET is RESOLVED
/// (qa/resolved-gaps.md): the full chain runs to completion. Same gap pattern
/// as BT17-015 branch 1 / BT22-013 branch 0.
#[test]
fn bt22_026_when_digivolving_branch_0_digivolves_agumon_into_wargreymon_free() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-026 YAML loads")
        .add_card(make_agumon("MY-AGU"))
        .add_card(make_wargreymon("MY-WG"))
        .hand(0, &["MY-WG"])
        .memory(15)
        .start();

    let agu_handle = runner.place_on_field(0, "MY-AGU", None);
    let bt22 = place_bt22_on_field(&mut runner, 0);

    trigger_when_digivolving(&mut runner, bt22);

    let kind = runner.pending_kind().expect("branch prompt installs");
    assert_eq!(kind, SelectionKind::EffectChoice);
    runner.execute_branch(0).expect("pick Digivolve branch");
    let _ = runner.auto_resolve();

    // If the chain runs to completion, WarGreymon ends up as the top card
    // of the (former) Agumon stack.
    let agu_perm = &runner.game.players[0].battle_area[agu_handle.index as usize];
    let top = agu_perm.top_card();
    assert_eq!(
        top.card_id(&runner.game.card_data),
        "MY-WG",
        "WarGreymon must be top card of the Agumon stack after digivolve branch"
    );
}

/// Negative: with NO opp Digimon and NO own Agumon, branch-choice still
/// installs but neither sub-branch has a legal target. The game must not
/// panic.
#[test]
fn bt22_026_when_digivolving_no_targets_does_not_panic() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-026 YAML loads")
        .memory(15)
        .start();

    let bt22 = place_bt22_on_field(&mut runner, 0);

    // Should not panic even with no valid targets in either branch.
    trigger_when_digivolving(&mut runner, bt22);
    // No assertion required — absence of panic is the acceptance criterion.
}

// ─── Section 3 — Inherited [When Attacking][OPT] behavioral ────────────────

/// Stack BT22-026 under an Omnimon-named top card so the printed name gate
/// "[Omnimon] in name" matches the actual top card.
fn place_omnimon_over_bt22_026(runner: &mut DebugRunner, player: PlayerId) -> PermanentHandle {
    runner.place_stack(player, &["BT22-026", "OMNI-TOP"])
}

/// Positive condition: Omnimon at top of stack + BT22-026 below — inherited
/// trigger fires on attack and unsuspends the carrier permanent.
///
/// To observe an unsuspend, we suspend the attacker just before the attack;
/// the inherited [When Attacking] clause should re-unsuspend it during/after
/// the attack resolution.
#[test]
fn bt22_026_inherited_when_attacking_omnimon_top_unsuspends_self() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-026 YAML loads")
        .add_card(make_omnimon_top("OMNI-TOP"))
        .add_card(make_filler_digimon("DEF-FILLER"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let attacker = place_omnimon_over_bt22_026(&mut runner, 0);
    let defender = runner.place_on_field(1, "DEF-FILLER", Some(0));

    runner.attack_digimon(attacker, defender, false);
    let _ = runner.auto_resolve();

    // After the attack resolution, the attacker (which would normally be
    // suspended after attacking) should be unsuspended by the inherited
    // [When Attacking] clause.
    let attacker_perm = &runner.game.players[0].battle_area[attacker.index as usize];
    assert!(
        !attacker_perm.is_suspended,
        "Omnimon-named attacker must be unsuspended via inherited [When Attacking]"
    );
}

/// Negative condition: stack has only MetalGarurumon (BT22-026) at top —
/// name gate must block the unsuspend. `source_name_contains` is evaluated
/// by the engine predicate path (`predicate.rs:284`), so the inherited
/// [When Attacking] gate correctly blocks when the top card is not
/// Omnimon-named.
#[test]
fn bt22_026_inherited_when_attacking_metalgarurumon_top_does_not_unsuspend() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-026 YAML loads")
        .add_card(make_filler_digimon("DEF-FILLER"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // BT22-026 alone at top — printed name "MetalGarurumon" does NOT contain "Omnimon".
    let attacker = runner.place_on_field(0, "BT22-026", Some(0));
    let defender = runner.place_on_field(1, "DEF-FILLER", Some(0));

    runner.attack_digimon(attacker, defender, false);
    let _ = runner.auto_resolve();

    // Without the Omnimon-name gate honored, the attacker should remain
    // suspended after attacking (since the inherited clause should NOT fire).
    let attacker_perm = &runner.game.players[0].battle_area[attacker.index as usize];
    assert!(
        attacker_perm.is_suspended,
        "MetalGarurumon-only top card must NOT trigger the [Omnimon]-name inherited \
         (attacker must remain suspended after attacking)"
    );
}

/// OPT: a second attack in the same turn must NOT unsuspend the carrier
/// a second time. Once the first attack uses the OPT slot, the second
/// attack's inherited cannot fire.
#[test]
fn bt22_026_inherited_when_attacking_opt_blocks_second_activation() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-026 YAML loads")
        .add_card(make_omnimon_top("OMNI-TOP"))
        .add_card(make_filler_digimon("DEF-1"))
        .add_card(make_filler_digimon("DEF-2"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let attacker = place_omnimon_over_bt22_026(&mut runner, 0);
    let def1 = runner.place_on_field(1, "DEF-1", Some(0));

    runner.attack_digimon(attacker, def1, false);
    let _ = runner.auto_resolve();
    // After first attack: inherited fires, unsuspends self.
    let after_first = {
        let p = &runner.game.players[0].battle_area[attacker.index as usize];
        p.is_suspended
    };
    assert!(
        !after_first,
        "first attack must unsuspend the Omnimon attacker via inherited"
    );

    // Second attack same turn — OPT must lock out the second unsuspend.
    if runner.battle_area_size(0) <= attacker.index as usize {
        return;
    }

    let def2 = runner.place_on_field(1, "DEF-2", Some(0));
    runner.attack_digimon(attacker, def2, false);
    let _ = runner.auto_resolve();

    // After second attack the OPT lockout means the inherited does NOT fire,
    // so the attacker should be suspended (suspended by the act of attacking,
    // not unsuspended afterward).
    let attacker_perm = &runner.game.players[0].battle_area[attacker.index as usize];
    assert!(
        attacker_perm.is_suspended,
        "OPT must prevent the inherited from firing twice in one turn \
         (attacker must remain suspended after second attack)"
    );
}

// ─── Section 4 — [Hand][Main] Nokia jump (main_from_hand) behavioral ─────────
//
// Printed Clause 1: "[Hand][Main] If you have [Nokia Shiramine], 1 of your
// [Gabumon] digivolves into this card for a digivolution cost of 6, ignoring
// digivolution requirements."
//
// Re-modelled (Task A2, gap-closure plan) from the retired `kind:
// activated_digivolve` alt-path (which had no engine execution route —
// G-ACTIVATED-DIGIVOLVE-EXECUTION) onto a `when: main_from_hand` triggered
// clause, EXACTLY mirroring BT24-016 Lamiamon clause 1 / BT22-013. The engine
// masks a Hand [Main] action for any hand card with a `MainFromHand` effect
// whose `condition:` passes; `activate_hand_main` runs it. The Nokia "If you
// have [Nokia Shiramine]" precondition — which could NOT be expressed on an
// alt-path — IS expressible as the `main_from_hand` `condition:` gate.
//
// Uses a REAL implemented Gabumon (BT22-017, same set/color) as the digivolve
// base so the body exercises `effect_initiated_digivolve { from_hand: self }`
// against a production card, not a synthetic stand-in. (BT22-017's own
// `[On Play]` / inherited clauses do not fire here — the Gabumon is placed
// directly on the field, not played, so neither interferes.)

/// Positive: Nokia Shiramine + a real [Gabumon] (BT22-017) on field, BT22-026
/// in hand → activating the [Hand][Main] effect digivolves MetalGarurumon onto
/// the Gabumon at cost 6, ignoring requirements. The Gabumon stack's top card
/// becomes BT22-026 and MetalGarurumon leaves the hand.
#[test]
fn bt22_026_hand_main_jump_digivolves_gabumon_at_cost6_with_nokia() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-026 YAML loads")
        .dsl_card("BT22-017")
        .expect("BT22-017 Gabumon in embedded DSL pack")
        .add_card(make_nokia_tamer("MY-NOKIA"))
        .add_card(make_filler_digimon("FILL"))
        .hand(0, &["BT22-026"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .memory(15)
        .start();

    // Nokia Shiramine (Tamer) + a real Gabumon (BT22-017) on player 0's field.
    runner.place_on_field(0, "MY-NOKIA", Some(0));
    runner.place_on_field(0, "BT22-017", Some(0));

    let mem_before = runner.memory();
    assert!(
        runner.game.activate_hand_main(0, 0),
        "the [Hand][Main] Nokia jump must fire (Nokia + Gabumon present)"
    );

    // The body selects which Gabumon to digivolve into.
    let view = runner
        .pending_selection_view()
        .expect("the Gabumon select prompt must install");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("select the Gabumon");
    let _ = runner.auto_resolve();

    // MetalGarurumon (BT22-026) digivolved onto the Gabumon permanent.
    let gabumon_perm = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| {
            p.card_sources
                .iter()
                .any(|s| s.card_id(&runner.game.card_data) == "BT22-017")
        })
        .expect("the Gabumon permanent must still be on the field");
    assert_eq!(
        gabumon_perm.top_card().card_id(&runner.game.card_data),
        "BT22-026",
        "MetalGarurumon must be the top card of the Gabumon stack after the [Hand][Main] jump"
    );
    assert_eq!(
        runner.hand_size(0),
        0,
        "MetalGarurumon must leave the hand after digivolving onto the Gabumon"
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

/// Condition gate (Nokia absent): with a real [Gabumon] on field but NO Nokia
/// Shiramine, the masked [Hand][Main] action is NOT offered —
/// `activate_hand_main` does not fire, and no digivolve happens.
#[test]
fn bt22_026_hand_main_jump_not_offered_without_nokia() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-026 YAML loads")
        .dsl_card("BT22-017")
        .expect("BT22-017 Gabumon in embedded DSL pack")
        .add_card(make_filler_digimon("FILL"))
        .hand(0, &["BT22-026"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .memory(15)
        .start();

    // Gabumon on field, but NO Nokia Shiramine — the Nokia gate must block the
    // masked Hand [Main] action.
    runner.place_on_field(0, "BT22-017", Some(0));

    assert!(
        !runner.game.activate_hand_main(0, 0),
        "without Nokia Shiramine the [Hand][Main] condition fails — the jump must not fire"
    );
    assert!(
        runner.game.pending_selection.is_none(),
        "no selection installs when the Nokia-gated [Hand][Main] jump is not offered"
    );

    // The Gabumon stack must be untouched (MetalGarurumon never digivolved onto it).
    let gabumon_perm = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| {
            p.card_sources
                .iter()
                .any(|s| s.card_id(&runner.game.card_data) == "BT22-017")
        })
        .expect("the Gabumon permanent must still be on the field");
    assert_eq!(
        gabumon_perm.top_card().card_id(&runner.game.card_data),
        "BT22-017",
        "no digivolve — the Gabumon must still be the top card of its own stack"
    );
    assert_eq!(
        runner.hand_size(0),
        1,
        "MetalGarurumon must remain in hand — the jump was not offered"
    );
}
