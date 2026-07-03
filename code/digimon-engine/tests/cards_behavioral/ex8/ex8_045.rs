//! EX8-045 Callismon — Digimon, Lv.6, Green/Purple, DP 12000, Cost 12.
//! Traits: Dark Animal, NSo. Attribute: Virus. Rarity: SP. Form: Mega.
//! Standard digivolve: Lv.5 Green / Cost 4 (printed evo box, cards.json evo_costs).
//!
//! # Card text (official Bandai DB bundle — data/card_bundles/EX8-045.md;
//! matches the card image)
//!
//! **Digivolve:** Lv.5 Green / Cost 4 (standard evo circle).
//! **Digivolve (alt box):** Lv.5 w/[NSo] trait: Cost 3.
//! **DNA Digivolve:** Green/Purple Lv.5 + Red/Yellow Lv.5: Cost 0.
//!
//! **Effect:**
//! [When Digivolving] Suspend 1 of your opponent's Digimon or Tamers. Then,
//! return 1 of their suspended Tamers to the bottom of the deck.
//! [Your Turn] For each color in this Digimon's digivolution cards, it gets
//! +1000 DP. While your opponent has no Digimon with as much or more DP as
//! this Digimon, this Digimon gains ＜Piercing＞ (When this Digimon attacks
//! and deletes an opponent's Digimon and survives the battle, it performs any
//! security checks it normally would.) and ＜Security A. +1＞ (This Digimon
//! checks 1 additional security card.)
//!
//! No separate "Inherited Effect" box on the printed card — every clause is
//! face-up card text (confirmed by the card image: all three effect blocks
//! sit in the single white text box, no gold/inherited box).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX8/Green/EX8_045.cs (base repo, resolved
//! via BASE_DCGO per rule 29 — NOT the worktree's empty placeholder)
//!   - Alternate Digivolution Requirement (None):
//!     AddSelfDigivolutionRequirementStaticEffect(permanentCondition:
//!     TopCard.IsLevel5 && TopCard.EqualsTraits("NSo"), digivolutionCost: 3).
//!   - DNA Digivolution (None): AddJogressConditionClass — element 1: own
//!     Green/Purple Lv.5 Digimon; element 2: own Red/Yellow Lv.5 Digimon;
//!     cost 0 (JogressCondition default index 0 = jogress digivolves
//!     unsuspended — `stacks_unsuspended: true`).
//!   - When Digivolving (OnEnterFieldAnyone): ActivateClass —
//!     CanActivateCondition: IsExistOnBattleAreaDigimon(card) &&
//!     HasMatchConditionPermanent(opp Digimon-or-Tamer). Coroutine step 1:
//!     mandatory SelectPermanentEffect (opp Digimon-or-Tamer, maxCount 1,
//!     canNoSelect: false, Mode.Tap) → suspend. Step 2 (runs independently,
//!     gated only on its own candidate existing): mandatory
//!     SelectPermanentEffect (opp Tamer AND IsSuspended, maxCount 1,
//!     canNoSelect: false, Mode.PutLibraryBottom) → return to bottom of deck.
//!   - Your Turn (None): ChangeSelfDPStaticEffect(changeValue: () => 1000 *
//!     SourceCount(), condition: IsOwnerTurn && IsExistOnBattleAreaDigimon &&
//!     SourceCount() >= 1) where SourceCount() =
//!     card.PermanentOfThisCard().DigivolutionCardsColors.Count (colors among
//!     the digivolution stack, i.e. cards BENEATH the top card).
//!     ChangeSelfSAttackStaticEffect(+1, condition: KeywordCondition) —
//!     KeywordCondition = IsExistOnBattleAreaDigimon(card) &&
//!     (opp has 0 Digimon || self.DP > max(opp Digimon DP)). NOTE: DCGO's
//!     KeywordCondition has NO IsOwnerTurn gate — the printed "While..."
//!     sentence is a separate, continuously-active clause; only the
//!     "+1000 DP" sentence carries the [Your Turn] header (confirmed by the
//!     official bundle's sentence boundary + DCGO's own lack of an
//!     IsOwnerTurn check on KeywordCondition).
//!   - Your Turn (OnDetermineDoSecurityCheck): PierceSelfEffect(condition:
//!     same KeywordCondition) — the same predicate, evaluated again at the
//!     security-check-decision timing so Piercing governs post-battle
//!     security checks even if the DP shifts mid-battle.
//!
//! # Patterns this test covers (docs/RUST_DSL_TEST_API.md §4.3)
//! - G2 DNA digivolve (multi-source materials, dna_costs) — printed
//!   Green/Purple Lv.5 + Red/Yellow Lv.5 / cost 0, stacks unsuspended.
//! - D1-adjacent: continuous formula-driven DP buff
//!   (`source_color_count` — "DistinctColorsCount formula" per the
//!   assessment brief) gated `[Your Turn]`.
//! - D4 declarative aura with DP-comparative `active_when` (`no_permanent`
//!   + `dp_gte: { formula: { source_dp: {} } }` — P-182/EX9-013 idiom)
//!   granting a bundled `<Piercing>` + `<Security A. +1>` keyword pair
//!   (EX8-023 idiom, but face-up not inherited, and no `your_turn` gate).
//! - F1-adjacent: [When Digivolving] two sequential mandatory selections,
//!   each independently gated on its own candidate set (AD1-014/BT16-028
//!   `any_of: [kind: digimon, kind: tamer]` idiom for step 1; `is_suspended`
//!   + `kind: tamer` idiom for step 2).
//! - H3 Piercing, H4 Security A. +N — DP-comparative keyword grant (unlike
//!   the static Examon/BT20-045 grant, this one is conditional).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, ModifierType};
use digimon_engine::selection::SelectionKind;
use digimon_engine::TriggerSource;

const CARD_ID: &str = "EX8-045";

// ─────────────────────────────────────────────────────────────────────────────
// Fixture builders
// ─────────────────────────────────────────────────────────────────────────────

fn opp_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, "Opp Digimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = level as u16;
    c.colors = vec![CardColor::Red];
    c
}

fn opp_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, "Opp Tamer");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c.colors = vec![CardColor::Red];
    c
}

fn source_filler(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, "Source Filler");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(1000);
    c.play_cost = 3;
    c.colors = vec![color];
    c
}

fn green_lv5(id: &str) -> CardData {
    let mut c = make_test_card(id, "Green Lv5");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 6;
    c.colors = vec![CardColor::Green];
    c
}

fn nso_lv5(id: &str) -> CardData {
    let mut c = make_test_card(id, "NSo Lv5");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 6;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["NSo".to_string()];
    c
}

fn purple_lv5(id: &str) -> CardData {
    let mut c = make_test_card(id, "Purple Lv5");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 6;
    c.colors = vec![CardColor::Purple];
    c
}

fn red_lv5(id: &str) -> CardData {
    let mut c = make_test_card(id, "Red Lv5");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 6;
    c.colors = vec![CardColor::Red];
    c
}

fn yellow_lv5(id: &str) -> CardData {
    let mut c = make_test_card(id, "Yellow Lv5");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 6;
    c.colors = vec![CardColor::Yellow];
    c
}

fn filler(id: &str) -> CardData {
    make_test_card(id, "Filler")
}

fn callismon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

/// Callismon compiles with its printed stats: Lv.6 green/purple Digimon,
/// cost 12, DP 12000, traits Dark Animal + NSo, and the standard
/// Lv.5-green cost-4 evo path (printed evo_costs).
#[test]
fn ex8_045_compiles_with_printed_stats_and_standard_digivolve() {
    let runner = callismon_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX8-045 in compiled cards");

    assert_eq!(compiled.card, "EX8-045");
    assert_eq!(compiled.name, "Callismon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(6));
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Green, CompiledColor::Purple],
        "cards.json card_colors [3, 6] = green + purple"
    );
    assert_eq!(compiled.cost, Some(12));
    assert_eq!(compiled.dp, Some(12000));
    for trait_name in ["Dark Animal", "NSo"] {
        assert!(
            compiled.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }

    // Standard printed evo box: Lv.5 green for cost 4.
    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(4))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(5)
                        || from.all_of.iter().any(|pred| pred.level_eq == Some(5)))
                        && (from.color_is == Some(CompiledColor::Green)
                            || from
                                .all_of
                                .iter()
                                .any(|pred| pred.color_is == Some(CompiledColor::Green)))
                })
        }),
        "Callismon must digivolve from a green Lv.5 for cost 4 (printed evo box)"
    );
}

/// Printed alt evo box: "Digivolve Lv.5 w/[NSo] trait: Cost 3".
#[test]
fn ex8_045_declares_nso_alt_digivolve_path() {
    let runner = callismon_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX8-045 in compiled cards");

    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(3))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(5)
                        || from.all_of.iter().any(|pred| pred.level_eq == Some(5)))
                        && (from.trait_has.as_deref() == Some("NSo")
                            || from
                                .all_of
                                .iter()
                                .any(|pred| pred.trait_has.as_deref() == Some("NSo")))
                })
        }),
        "Callismon must digivolve from any Lv.5 [NSo] Digimon for cost 3 (printed alt evo box)"
    );
}

/// Printed DNA Digivolve box: Green/Purple Lv.5 + Red/Yellow Lv.5, cost 0,
/// stacked unsuspended (DCGO JogressCondition default index 0).
#[test]
fn ex8_045_declares_dna_digivolve_path() {
    let runner = callismon_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX8-045 in compiled cards");

    let dna = compiled
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::DnaDigivolve)
        .expect("Callismon must declare a DnaDigivolve alt-path");

    assert_eq!(dna.cost, Some(CompiledCost::Literal(0)));
    assert_eq!(
        dna.materials.len(),
        2,
        "DNA digivolve needs exactly 2 material slots (Green/Purple Lv.5 + Red/Yellow Lv.5)"
    );
    assert!(
        dna.stacks_unsuspended,
        "DCGO JogressCondition default index 0 = stack and digivolve unsuspended"
    );
}

/// The [When Digivolving] clause is face-up (no inherited box on this card),
/// not optional (no printed "you may"), and not once-per-turn.
#[test]
fn ex8_045_when_digivolving_clause_structural_shape() {
    let runner = callismon_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX8-045 in compiled cards");

    let triggered = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving) => {
                Some(t)
            }
            _ => None,
        })
        .expect("EX8-045 must encode a [When Digivolving] clause");

    assert_eq!(
        triggered.scope,
        CompiledScope::FaceUp,
        "no separate inherited box on the printed card — all clauses are face-up"
    );
    assert!(
        !triggered.optional,
        "no printed 'you may' — the suspend/return-to-deck effect is mandatory"
    );
    assert!(
        !triggered.once_per_turn,
        "printed text carries no [Once Per Turn]"
    );
}

/// The [Your Turn] DP buff is a face-up declarative aura with a
/// `dp_modifier_fn` (not a flat literal) gated on `your_turn: true`.
#[test]
fn ex8_045_your_turn_dp_buff_is_formula_aura() {
    let runner = callismon_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX8-045 in compiled cards");

    let found = compiled.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope,
                active_when,
                dp_modifier_fn: Some(_),
                ..
            }) if *scope == CompiledScope::FaceUp && active_when.is_some()
        )
    });
    assert!(
        found,
        "expected a face-up formula-driven DP aura ([Your Turn] + source_color_count)"
    );
}

/// The DP-comparative Piercing + Security A. +1 grant is a face-up
/// declarative aura bundling both a `grant_keyword` and a flat
/// `security_attack: 1`.
#[test]
fn ex8_045_piercing_security_aura_structural_shape() {
    let runner = callismon_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX8-045 in compiled cards");

    let aura = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope,
                active_when,
                security_attack,
                grant_keyword,
                dp_modifier_fn,
                ..
            }) if *scope == CompiledScope::FaceUp
                && grant_keyword.as_ref().map(|g| g.keyword.as_str()) == Some("Piercing") =>
            {
                Some((active_when, security_attack, dp_modifier_fn))
            }
            _ => None,
        })
        .expect("EX8-045 must encode a face-up Piercing-granting aura");

    let (active_when, security_attack, dp_modifier_fn) = aura;
    assert!(
        active_when.is_some(),
        "the Piercing/Security A.+1 aura must carry a DP-comparative active_when gate"
    );
    assert_eq!(
        *security_attack,
        Some(1),
        "the aura grants <Security A. +1>"
    );
    assert!(
        dp_modifier_fn.is_none(),
        "the Piercing/Security A.+1 aura is a separate clause from the DP-buff aura"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — [When Digivolving]: suspend 1 opp Digimon/Tamer
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE: with an opponent Digimon on the field, [When Digivolving]
/// installs a mandatory selection over opponent Digimon-or-Tamer permanents.
#[test]
fn ex8_045_when_digivolving_offers_suspend_selection_over_digimon_or_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(opp_digimon("OPP-A", 4, 5000))
        .memory(20)
        .start();

    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-A", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("suspend selection must install with an opponent Digimon present");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(
        !runner.pending_is_optional(),
        "no printed 'you may' — the suspend pick is mandatory"
    );
}

/// NEGATIVE: with no opponent Digimon or Tamers at all, [When Digivolving]
/// installs no selection whatsoever (DCGO's whole-effect CanActivateCondition
/// gate — both steps are skipped).
#[test]
fn ex8_045_when_digivolving_with_no_opponent_permanents_installs_nothing() {
    let mut runner = callismon_runner().memory(20).start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon or Tamer exists — neither step should install"
    );
}

/// BEHAVIORAL: resolving the suspend selection over an opponent Tamer
/// suspends it. With no OTHER suspended Tamer present, the return-to-deck
/// follow-up must still offer the JUST-suspended Tamer (DCGO's second
/// selection is a fresh scan for ANY suspended opponent Tamer, which now
/// includes the one just suspended).
#[test]
fn ex8_045_when_digivolving_suspends_chosen_tamer_and_offers_it_for_return() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(opp_tamer("OPP-TAMER"))
        .memory(20)
        .start();

    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_tamer_handle = runner.place_on_field(1, "OPP-TAMER", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("suspend selection must install");
    assert_eq!(view.valid_action_ids.len(), 1, "only OPP-TAMER is a legal target");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("suspend OPP-TAMER");

    assert!(
        runner.game.players[1].battle_area[opp_tamer_handle.index as usize].is_suspended,
        "the chosen opponent Tamer must now be suspended"
    );

    // Follow-up: return-to-deck selection over suspended opponent Tamers.
    let view = runner
        .pending_selection_view()
        .expect("return-to-bottom-of-deck selection must install (the just-suspended Tamer qualifies)");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(!runner.pending_is_optional(), "mandatory — no printed 'you may'");
    assert_eq!(view.valid_action_ids.len(), 1);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("return the suspended Tamer to the bottom of the deck");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "OPP-TAMER"),
        "OPP-TAMER must leave the battle area"
    );
    assert_eq!(
        runner.game.players[1]
            .deck
            .last()
            .map(|c| c.card_id(&runner.game.card_data)),
        Some("OPP-TAMER"),
        "OPP-TAMER must land at the BOTTOM of the deck"
    );
}

/// NEGATIVE: suspending an opponent DIGIMON (not a Tamer) leaves no suspended
/// Tamer on the opponent's side — the return-to-deck follow-up must not
/// install (DCGO's step-2 candidate scan finds nothing).
#[test]
fn ex8_045_when_digivolving_no_return_step_when_no_suspended_tamer_exists() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(opp_digimon("OPP-A", 4, 5000))
        .memory(20)
        .start();

    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("suspend selection must install");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("suspend OPP-A (a Digimon, not a Tamer)");

    assert!(
        runner.game.players[1].battle_area[opp_a.index as usize].is_suspended,
        "OPP-A must be suspended"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no suspended opponent TAMER exists — the return-to-deck step must not install"
    );
}

/// POSITIVE (independent gating): an ALREADY-suspended opponent Tamer, sitting
/// alongside a separate opponent Digimon that gets chosen for the suspend
/// step, still qualifies for the return-to-deck follow-up — DCGO's step 2 is
/// a fresh scan, independent of which permanent step 1 targeted.
#[test]
fn ex8_045_when_digivolving_return_step_offers_pre_suspended_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(opp_digimon("OPP-A", 4, 5000))
        .add_card(opp_tamer("OPP-TAMER"))
        .memory(20)
        .start();

    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_tamer_handle = runner.place_on_field(1, "OPP-TAMER", Some(0));
    runner.game.players[1].battle_area[opp_tamer_handle.index as usize].is_suspended = true;

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    // Two candidates for step 1 (OPP-A and OPP-TAMER); choose OPP-A.
    let view = runner
        .pending_selection_view()
        .expect("suspend selection must install");
    assert_eq!(view.valid_action_ids.len(), 2);
    // `OppField` selections encode each candidate as `encode_attack(0, slot)`
    // (the side is implicit in the selection kind, not the action id).
    let opp_a_action = digimon_engine::action::space::encode_attack(0, opp_a.index.into());
    runner
        .execute_action(0, opp_a_action)
        .expect("suspend OPP-A specifically");

    // Step 2 must still offer the PRE-suspended OPP-TAMER.
    let view = runner
        .pending_selection_view()
        .expect("return-to-deck step must install for the pre-suspended Tamer");
    assert_eq!(view.valid_action_ids.len(), 1);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("return OPP-TAMER to the bottom of the deck");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "OPP-TAMER"),
        "OPP-TAMER must leave the battle area"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — [Your Turn] +1000 DP per digivolution-card color
// ─────────────────────────────────────────────────────────────────────────────

/// With ZERO digivolution cards (bare permanent), the DP buff contributes 0 —
/// effective DP stays at the printed 12000.
#[test]
fn ex8_045_dp_buff_zero_with_no_digivolution_cards() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .start();

    let carrier = r.place_on_field(0, CARD_ID, Some(0));
    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert_eq!(
        r.game.effective_dp(carrier),
        Some(12000),
        "no digivolution cards -> 0 distinct colors -> +0 DP"
    );
}

/// With a Green source beneath Callismon (1 distinct color: Green), the buff
/// is +1000. The TOP card itself does not count toward the color set.
#[test]
fn ex8_045_dp_buff_plus_1000_for_one_source_color() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(source_filler("SRC-GREEN", CardColor::Green))
        .start();

    let carrier = r.place_stack(0, &["SRC-GREEN", CARD_ID]);
    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert_eq!(
        r.game.effective_dp(carrier),
        Some(13000),
        "1 distinct digivolution-card color -> +1000 DP"
    );
}

/// With THREE distinct source colors (Green, Red, Blue) beneath the top card,
/// the buff is +3000. A duplicate color does not double-count.
#[test]
fn ex8_045_dp_buff_plus_3000_for_three_distinct_source_colors() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(source_filler("SRC-GREEN", CardColor::Green))
        .add_card(source_filler("SRC-RED", CardColor::Red))
        .add_card(source_filler("SRC-RED-2", CardColor::Red))
        .add_card(source_filler("SRC-BLUE", CardColor::Blue))
        .start();

    let carrier = r.place_stack(
        0,
        &["SRC-GREEN", "SRC-RED", "SRC-RED-2", "SRC-BLUE", CARD_ID],
    );
    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert_eq!(
        r.game.effective_dp(carrier),
        Some(15000),
        "3 distinct digivolution-card colors (Green, Red, Blue; Red counted once) -> +3000 DP"
    );
}

/// [Your Turn] gate: on the OPPONENT's turn the DP buff is absent even with
/// matching source colors on the stack.
#[test]
fn ex8_045_dp_buff_absent_on_opponents_turn() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(source_filler("SRC-GREEN", CardColor::Green))
        .start();

    let tp = r.game.turn_player();
    let non_tp = 1 - tp;
    let carrier = r.place_stack(non_tp, &["SRC-GREEN", CARD_ID]);
    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert_eq!(
        r.game.effective_dp(carrier),
        Some(12000),
        "[Your Turn] gate: no DP buff while it is not the carrier controller's turn"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — DP-comparative Piercing + Security A. +1
// ─────────────────────────────────────────────────────────────────────────────

/// AURA ON: opponent has ZERO Digimon on the field -> the "no Digimon with as
/// much or more DP" gate is vacuously satisfied -> Piercing + Security A. +1.
#[test]
fn ex8_045_piercing_security_on_when_opponent_has_no_digimon() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .start();

    let carrier = r.place_on_field(0, CARD_ID, Some(0));
    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert!(
        r.game.has_keyword(carrier, Keyword::Piercing),
        "opponent has no Digimon at all -> the DP-comparative gate is vacuously true"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        1,
        "Security A. +1 must accompany Piercing under the same gate"
    );
}

/// AURA ON: opponent's highest-DP Digimon has STRICTLY LESS DP than
/// Callismon's own effective DP.
#[test]
fn ex8_045_piercing_security_on_when_all_opponent_digimon_have_less_dp() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(opp_digimon("OPP-WEAK", 4, 11999))
        .start();

    let carrier = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP-WEAK", Some(0));
    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert!(
        r.game.has_keyword(carrier, Keyword::Piercing),
        "carrier DP 12000 > opponent's 11999 -> gate is satisfied"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        1
    );
}

/// AURA OFF: an opponent Digimon has DP EQUAL to Callismon's own DP — the
/// printed "as much OR MORE" threshold is inclusive of equality, so the gate
/// must be OFF (matches DCGO's `self.DP > opp.DP` failing at equality).
#[test]
fn ex8_045_piercing_security_off_when_opponent_digimon_has_equal_dp() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(opp_digimon("OPP-EQUAL", 4, 12000))
        .start();

    let carrier = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP-EQUAL", Some(0));
    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert!(
        !r.game.has_keyword(carrier, Keyword::Piercing),
        "opponent Digimon DP EQUALS carrier DP -> 'as much or more' gate is OFF"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        0
    );
}

/// AURA OFF: an opponent Digimon has STRICTLY MORE DP than Callismon.
#[test]
fn ex8_045_piercing_security_off_when_opponent_digimon_has_more_dp() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(opp_digimon("OPP-STRONG", 6, 15000))
        .start();

    let carrier = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP-STRONG", Some(0));
    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert!(
        !r.game.has_keyword(carrier, Keyword::Piercing),
        "opponent Digimon DP 15000 > carrier DP 12000 -> gate is OFF"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        0
    );
}

/// The DP-comparative gate reads Callismon's EFFECTIVE DP (including its own
/// [Your Turn] source-color buff), not just its printed 12000. With 1 source
/// color (+1000 -> 13000 effective), an opponent Digimon at 12500 DP must NOT
/// disable the gate (12500 < 13000), whereas without the buff it would
/// (12500 > 12000 printed).
#[test]
fn ex8_045_piercing_gate_uses_effective_dp_including_own_color_buff() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(source_filler("SRC-GREEN", CardColor::Green))
        .add_card(opp_digimon("OPP-MID", 6, 12500))
        .start();

    let carrier = r.place_stack(0, &["SRC-GREEN", CARD_ID]);
    r.place_on_field(1, "OPP-MID", Some(0));
    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert_eq!(
        r.game.effective_dp(carrier),
        Some(13000),
        "sanity: the color buff must be live"
    );
    assert!(
        r.game.has_keyword(carrier, Keyword::Piercing),
        "gate must compare against EFFECTIVE DP (13000), not printed DP (12000) -- \
         12500 < 13000 keeps the gate on"
    );
}

/// [Your Turn]-independence: unlike the DP-buff sentence, the DP-comparative
/// Piercing/Security A.+1 grant is NOT gated to the carrier's own turn (the
/// printed "While..." sentence carries no [Your Turn] header; DCGO's
/// KeywordCondition has no IsOwnerTurn check).
#[test]
fn ex8_045_piercing_security_active_on_opponents_turn_too() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .start();

    let tp = r.game.turn_player();
    let non_tp = 1 - tp;
    let carrier = r.place_on_field(non_tp, CARD_ID, Some(0));
    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert!(
        r.game.has_keyword(carrier, Keyword::Piercing),
        "the DP-comparative Piercing/Security A.+1 grant has no [Your Turn] gate"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        1
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — DNA digivolve execution
// ─────────────────────────────────────────────────────────────────────────────

/// BEHAVIORAL: DNA-digivolving via the printed Green/Purple Lv.5 + Red/Yellow
/// Lv.5 recipe stacks BOTH source Digimon under Callismon and charges its
/// printed cost 0.
#[test]
fn ex8_045_dna_digivolve_stacks_both_materials_for_cost_zero() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(green_lv5("GREEN-A"))
        .add_card(red_lv5("RED-A"))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(8)
        .start();

    let green = runner.place_on_field(0, "GREEN-A", Some(0));
    let red = runner.place_on_field(0, "RED-A", Some(0));
    runner.game.current_phase = digimon_engine::enums::GamePhase::Main;

    let memory_before = runner.game.memory;
    assert!(
        runner.game.initiate_dna_digivolve(0, 0),
        "DNA digivolve must initiate from hand index 0"
    );
    runner
        .game
        .resolve_selection(0, green.index as u16)
        .expect("pick the Green Lv.5 material");
    runner
        .game
        .resolve_selection(0, red.index as u16)
        .expect("pick the Red Lv.5 material");

    assert_eq!(
        runner.game.memory, memory_before,
        "printed DNA Digivolve cost is 0 -- memory must be unchanged"
    );
    assert_eq!(
        runner.game.n_dna_digivolutions[0], 1,
        "the DNA digivolution counter must increment"
    );
    // Exactly one permanent remains on the battle area (the two materials
    // stacked into one Callismon permanent).
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    let stacked = &runner.game.players[0].battle_area[0];
    assert_eq!(
        stacked.top_card().card_id(&runner.game.card_data),
        CARD_ID,
        "Callismon must be the new top card"
    );
    assert_eq!(
        stacked.card_sources.len(),
        3,
        "top card + 2 stacked materials = 3 total sources"
    );
}

/// DNA digivolve accepts the Purple/Yellow color-pair variant of the printed
/// recipe (Purple counts for slot 1's Green-OR-Purple; Yellow counts for slot
/// 2's Red-OR-Yellow).
#[test]
fn ex8_045_dna_digivolve_accepts_purple_and_yellow_variant() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(purple_lv5("PURPLE-A"))
        .add_card(yellow_lv5("YELLOW-A"))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(8)
        .start();

    let purple = runner.place_on_field(0, "PURPLE-A", Some(0));
    let yellow = runner.place_on_field(0, "YELLOW-A", Some(0));
    runner.game.current_phase = digimon_engine::enums::GamePhase::Main;

    assert!(runner.game.initiate_dna_digivolve(0, 0));
    runner
        .game
        .resolve_selection(0, purple.index as u16)
        .expect("pick the Purple Lv.5 material");
    runner
        .game
        .resolve_selection(0, yellow.index as u16)
        .expect("pick the Yellow Lv.5 material");

    assert_eq!(runner.game.n_dna_digivolutions[0], 1);
}

/// NEGATIVE: two Green Lv.5 Digimon do NOT satisfy the recipe — slot 2
/// requires Red or Yellow, which neither material provides. No valid first
/// target exists at all (every candidate first-material must pair with SOME
/// second-material to be offered), so `initiate_dna_digivolve` itself must
/// reject up front rather than opening a picker that can't be completed.
#[test]
fn ex8_045_dna_digivolve_rejects_two_green_materials() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(green_lv5("GREEN-A"))
        .add_card(green_lv5("GREEN-B"))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(8)
        .start();

    runner.place_on_field(0, "GREEN-A", Some(0));
    runner.place_on_field(0, "GREEN-B", Some(0));
    runner.game.current_phase = digimon_engine::enums::GamePhase::Main;

    assert!(
        !runner.game.initiate_dna_digivolve(0, 0),
        "two Green materials must not satisfy the Green/Purple + Red/Yellow recipe -- \
         no valid first-material target exists, so the picker must not open"
    );
    assert_eq!(
        runner.game.n_dna_digivolutions[0], 0,
        "no DNA digivolution should have occurred"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 6 — no PASS-driven auto-resolve through the two-selection [When
// Digivolving] cluster (anti-pattern guard — branch-specific driving only)
// ─────────────────────────────────────────────────────────────────────────────

/// PASS must not be a legal action for either mandatory [When Digivolving]
/// selection when a target exists (no printed "you may").
#[test]
fn ex8_045_when_digivolving_selections_do_not_expose_pass() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(opp_tamer("OPP-TAMER"))
        .memory(20)
        .start();

    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-TAMER", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    let view = runner.pending_selection_view().expect("suspend selection");
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "the suspend pick must not expose PASS (mandatory, no 'you may')"
    );
}
