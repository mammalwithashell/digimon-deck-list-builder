//! BT12-034 Agumon — Digimon, Lv.3, Yellow/Red, DP 2000, Cost 3.
//! Traits: Dinosaur. Attribute: Vaccine.
//! Standard digivolve: Lv2 Yellow / Cost 1 AND Lv2 Red / Cost 1 (printed
//! evo_costs — a genuine multi-color cost, not modeled by this card's
//! `alt_paths`, which only cover the printed black-text alt-digivolve).
//!
//! # Card text (data/card_bundles/BT12-034.md — official Bandai DB)
//!
//! ```text
//! [Digivolve] 0 from Koromon
//!
//! [On Play] Reveal the top 4 cards of your deck. Add 1 Digimon card with
//! [Greymon] in its name and 1 [Marcus Damon] among them to your hand.
//! Place the rest at the bottom of your deck in any order.
//!
//! Inherited Effect:
//! [Your Turn][Once Per Turn] When one of your yellow or red Tamers becomes
//! suspended, 1 of your opponent's Digimon gets -2000 DP for the turn.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT12/Yellow/BT12_034.cs
//!   - `EffectTiming.None`: `AddSelfDigivolutionRequirementStaticEffect` —
//!     `PermanentCondition = TopCard.CardNames.Contains("Koromon")`,
//!     `digivolutionCost: 0`, `ignoreDigivolutionRequirement: false`.
//!   - `EffectTiming.OnEnterFieldAnyone`: `SimplifiedRevealDeckTopCardsAndSelect`
//!     (revealCount: 4) with two buckets — `CanSelectCardCondition` (IsDigimon
//!     && HasGreymonName) and `CanSelectCardCondition1` (CardNames contains
//!     "Marcus Damon" or "MarcusDamon"), `mutualConditions: true`,
//!     `remainingCardsPlace: DeckBottom`. Outer trigger NOT optional
//!     (`SetUpActivateClass(..., -1, false, ...)`).
//!   - `EffectTiming.OnTappedAnyone`, `SetIsInheritedEffect(true)`,
//!     `SetHashString("DP-2000_BT12_034")`: `CanUseCondition` gates on
//!     `IsExistOnBattleArea && IsOwnerTurn && CanTriggerWhenPermanentSuspends`
//!     where the suspending permanent is an owner Tamer with Red or Yellow in
//!     its colors. `ActivateCoroutine` mandatorily (`canNoSelect: false`)
//!     selects 1 opponent battle-area Digimon (when one exists) and applies
//!     `ChangeDigimonDP(changeValue: -2000, effectDuration: UntilEachTurnEnd)`.
//!
//! # Patterns this test covers (docs/RUST_DSL_TEST_API.md §4.3)
//! - A1/A3 — On-play reveal-4 + two-bucket selection (Greymon-name Digimon
//!   bucket, exact-name Marcus Damon bucket) via `select_reveal_buckets`.
//! - G4-adjacent — inherited [Your Turn][OPT] event observer keyed on own
//!   Tamer color (mirrors BT21-004's on_suspend + color-filtered event
//!   predicate, generalized to a mandatory opponent-target DP debuff instead
//!   of a draw).
//! - D1-adjacent — literal DP debuff (`add_dp_modifier: { value: -2000 }`)
//!   with `expiry: end_of_turn` ("for the turn" = DCGO's UntilEachTurnEnd,
//!   the current/your turn since the inherited clause only fires on
//!   [Your Turn]).
//! - E2 OPT lockout + turn-boundary clear.
//!
//! # Faithfulness notes (DCGO comparison)
//! - The digimon-only + Greymon-name bucket and the exact-name "Marcus Damon"
//!   bucket are mandatory once the reveal is populated (DCGO `maxCount: 1`
//!   with no `canNoSelect: true` override at the bucket level); resolves
//!   silently when no candidate matches (falls through to bottom-placement).
//! - The inherited DP-debuff clause's outer activation requires an opponent
//!   battle-area Digimon to exist (DCGO `HasMatchConditionPermanent`); if the
//!   opponent has none, the clause does not install a selection at all — it
//!   never surfaces an empty-target dead prompt.
//! - `[Once Per Turn]` is a SHARED lockout across both yellow-Tamer-suspend
//!   and red-Tamer-suspend triggers within the same turn (single DCGO hash
//!   `"DP-2000_BT12_034"`), not per-color.

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

// ═══════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════

fn compiled() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("BT12-034")
}

fn make_named_digimon(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card
}

fn make_named_tamer(id: &str, name: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![color];
    card.dp = None;
    card.level = None;
    card
}

fn make_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> Option<u16> {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
}

fn pick_revealed_by_id(runner: &mut DebugRunner, id: &str, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = revealed_action_for_id(runner, id)
        .unwrap_or_else(|| panic!("{label}: revealed card {id} not present"));
    assert!(
        view.valid_action_ids.contains(&action),
        "{label}: action {action} for {id} not legal in current bucket"
    );
    runner.execute_action(0, action).expect(label);
}

fn assert_required_pending_pick(runner: &DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    assert!(
        !runner.pending_is_optional(),
        "{label}: bucket should be mandatory when a matching card exists"
    );
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "{label}: PASS must not be legal before the required pick"
    );
}

/// Runner for the inherited on-suspend clause: BT12-034 as a digivolution
/// source under a host Digimon, plus Tamers to suspend, an opponent Digimon
/// target, and stocked decks so turn rotation doesn't deck out.
fn egg_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT12-034")
        .expect("BT12-034 in embedded DSL pack")
        .add_card(make_digimon("HOST", 4, 3000))
        .add_card(make_named_tamer("RED-TAMER", "Red Tamer", CardColor::Red))
        .add_card(make_named_tamer(
            "YEL-TAMER",
            "Yellow Tamer",
            CardColor::Yellow,
        ))
        .add_card(make_named_tamer(
            "GRN-TAMER",
            "Green Tamer",
            CardColor::Green,
        ))
        .add_card(make_digimon("FILLER", 3, 2000))
        .add_card(make_digimon("OPP", 5, 8000))
        .add_card(make_digimon("OPP2", 5, 6000))
        .deck(0, &["FILLER"; 20])
        .deck(1, &["FILLER"; 20])
        .memory(10)
        .start()
}

fn host_with_agumon(runner: &mut DebugRunner, player: u8) -> PermanentHandle {
    runner.place_stack(player, &["BT12-034", "HOST"])
}

/// Fully rotate the active player to `target`, flushing the EndOfTurnAction
/// phase each step (that's where `end_of_turn` modifier expiry and per-turn
/// OPT resets fire). Plain `end_turn` does not flush that step — see
/// AD1-016's `advance_to_turn_player` idiom.
fn advance_to_turn_player(runner: &mut DebugRunner, target: u8) {
    for _ in 0..8 {
        if runner.turn_player() == target {
            return;
        }
        let _ = runner.auto_resolve();
        runner.pass_turn();
        let _ = runner.auto_resolve();
        if runner.game.current_phase == digimon_engine::enums::GamePhase::EndOfTurnAction {
            runner.game.pass_end_of_turn_action();
            let _ = runner.auto_resolve();
        }
    }
    assert_eq!(
        runner.turn_player(),
        target,
        "failed to rotate to target turn player"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt12_034_yaml_parses_and_compiles() {
    let spec: CardSpec = serde_yml::from_str(include_str!("../../../cards/bt12/BT12-034.yaml"))
        .expect("BT12-034 YAML parses");
    let _compiled = compile(&spec).expect("BT12-034 YAML compiles");
}

#[test]
fn bt12_034_metadata_is_yellow_red_lv3_2000dp() {
    let card = compiled();
    assert_eq!(card.card, "BT12-034");
    assert_eq!(card.name, "Agumon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(2000));
    assert!(card
        .color
        .contains(&digimon_dsl::compiled::CompiledColor::Yellow));
    assert!(card
        .color
        .contains(&digimon_dsl::compiled::CompiledColor::Red));
}

/// Printed alt-digivolve: "[Digivolve] 0 from Koromon".
#[test]
fn bt12_034_has_koromon_named_alt_digivolve_path_at_cost_zero() {
    let card = compiled();
    let koromon_alt = card.alt_paths.iter().any(|path| {
        let Some(from) = path.from.as_ref() else {
            return false;
        };
        let name_match = from.name_contains.as_deref() == Some("Koromon")
            || from
                .all_of
                .iter()
                .any(|p| p.name_contains.as_deref() == Some("Koromon"));
        name_match
            && path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(0))
    });
    assert!(
        koromon_alt,
        "BT12-034 must declare the Koromon-named alt-digivolve path at cost 0 \
         (DCGO AddSelfDigivolutionRequirementStaticEffect)"
    );
}

/// Exactly one FaceUp OnPlay triggered clause and one Inherited triggered
/// clause (on_suspend). No aura / declarative clauses on this card.
#[test]
fn bt12_034_has_one_on_play_and_one_inherited_on_suspend_clause() {
    let card = compiled();
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
        "exactly two triggered clauses: OnPlay reveal + inherited on_suspend debuff"
    );

    let on_play = triggered
        .iter()
        .find(|c| c.scope == CompiledScope::FaceUp)
        .expect("a FaceUp OnPlay clause must exist");
    assert_eq!(on_play.when, vec![CompiledTiming::OnPlay]);
    assert!(!on_play.optional, "printed text has no 'you may' on OnPlay");

    let inherited = triggered
        .iter()
        .find(|c| c.scope == CompiledScope::Inherited)
        .expect("an Inherited on_suspend clause must exist");
    assert!(inherited.when.contains(&CompiledTiming::OnSuspend));
    assert!(inherited.once_per_turn, "[Once Per Turn]");
    assert!(
        !inherited.optional,
        "mandatory DP debuff once triggered — no 'may'"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — OnPlay behavioral: reveal 4, two buckets, bottom remainder
// ═══════════════════════════════════════════════════════════════════════════

/// Positive path — both buckets satisfied. Reveal 4 = [GREYMON, MARCUS,
/// FILLER1, FILLER2]. GREYMON goes to the Greymon-name bucket, MARCUS goes to
/// the Marcus Damon bucket. The two fillers go to deck bottom.
#[test]
fn bt12_034_on_play_adds_one_greymon_digimon_and_marcus_damon_bottoms_rest() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-034")
        .expect("BT12-034 YAML loads")
        .add_card(make_named_digimon("GREYMON", "Greymon"))
        .add_card(make_named_digimon("MARCUS", "Marcus Damon"))
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .deck(0, &["GREYMON", "MARCUS", "FILLER1", "FILLER2"])
        .hand(0, &["BT12-034"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Agumon BT12-034");

    assert_required_pending_pick(&runner, "Greymon-name Digimon bucket");
    pick_revealed_by_id(&mut runner, "GREYMON", "pick Greymon");

    assert_required_pending_pick(&runner, "Marcus Damon bucket");
    pick_revealed_by_id(&mut runner, "MARCUS", "pick Marcus Damon");

    runner.auto_resolve().expect("bottom remainder");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"GREYMON".to_string()),
        "Greymon-name Digimon must land in hand"
    );
    assert!(
        hand_ids.contains(&"MARCUS".to_string()),
        "Marcus Damon must land in hand"
    );
    let deck_ids: Vec<String> = runner.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let last_two: Vec<String> = deck_ids.iter().rev().take(2).cloned().collect();
    assert!(
        last_two.contains(&"FILLER1".to_string()) && last_two.contains(&"FILLER2".to_string()),
        "unchosen reveal cards return to deck bottom; deck end was {last_two:?}"
    );
}

/// Digimon bucket only: reveal contains a Greymon-name Digimon but no Marcus
/// Damon. Marcus-bucket has zero candidates and falls through silently.
#[test]
fn bt12_034_on_play_only_greymon_found_skips_marcus_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-034")
        .expect("BT12-034 YAML loads")
        .add_card(make_named_digimon("GREYMON", "Greymon"))
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .add_card(make_test_card("FILLER3", "Filler3"))
        .deck(0, &["GREYMON", "FILLER1", "FILLER2", "FILLER3"])
        .hand(0, &["BT12-034"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Agumon BT12-034");
    pick_revealed_by_id(&mut runner, "GREYMON", "pick Greymon");
    runner
        .auto_resolve()
        .expect("resolve through empty Marcus Damon bucket");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"GREYMON".to_string()),
        "Greymon-name Digimon must be added to hand"
    );
    assert!(
        !hand_ids
            .iter()
            .any(|id| id == "FILLER1" || id == "FILLER2" || id == "FILLER3"),
        "no Marcus Damon was found, no extra add"
    );
}

/// Marcus Damon bucket only: reveal contains Marcus Damon but no Greymon-name
/// Digimon. Digimon bucket has zero candidates and falls through silently.
#[test]
fn bt12_034_on_play_only_marcus_damon_found_skips_greymon_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-034")
        .expect("BT12-034 YAML loads")
        .add_card(make_named_digimon("MARCUS", "Marcus Damon"))
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .add_card(make_test_card("FILLER3", "Filler3"))
        .deck(0, &["MARCUS", "FILLER1", "FILLER2", "FILLER3"])
        .hand(0, &["BT12-034"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Agumon BT12-034");

    while let Some(view) = runner.pending_selection_view() {
        let marcus_action = revealed_action_for_id(&runner, "MARCUS");
        if let Some(action) = marcus_action {
            if view.valid_action_ids.contains(&action) {
                runner.execute_action(0, action).expect("pick Marcus Damon");
                continue;
            }
        }
        let next = view.valid_action_ids[0];
        runner.execute_action(0, next).expect("advance");
    }

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"MARCUS".to_string()),
        "Marcus Damon must land in hand"
    );
}

/// Neither bucket has a candidate — reveal 4 fillers, both buckets resolve
/// without adds, all 4 cards hit deck bottom.
#[test]
fn bt12_034_on_play_neither_bucket_found_bottoms_all_four() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-034")
        .expect("BT12-034 YAML loads")
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .add_card(make_test_card("FILLER3", "Filler3"))
        .add_card(make_test_card("FILLER4", "Filler4"))
        .deck(0, &["FILLER1", "FILLER2", "FILLER3", "FILLER4"])
        .hand(0, &["BT12-034"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Agumon BT12-034");
    runner
        .auto_resolve()
        .expect("resolve through both empty buckets");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    for filler in ["FILLER1", "FILLER2", "FILLER3", "FILLER4"] {
        assert!(
            !hand_ids.contains(&filler.to_string()),
            "{filler} must NOT enter hand when no bucket matches"
        );
    }
    let deck_ids: Vec<String> = runner.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(deck_ids.len(), 4, "all 4 reveal cards back in deck");
}

/// The Digimon bucket requires `kind: digimon` — a Tamer named "Greymon" must
/// NOT satisfy it (DCGO `CanSelectCardCondition`: IsDigimon && HasGreymonName).
#[test]
fn bt12_034_greymon_bucket_rejects_tamer_with_greymon_name() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-034")
        .expect("BT12-034 YAML loads")
        .add_card(make_named_tamer(
            "FAKE-GREYMON-TAMER",
            "Greymon",
            CardColor::Yellow,
        ))
        .add_card(make_named_digimon("MARCUS", "Marcus Damon"))
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .deck(0, &["FAKE-GREYMON-TAMER", "MARCUS", "FILLER1", "FILLER2"])
        .hand(0, &["BT12-034"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play BT12-034");

    // Walk the RevealBucket prompts ONLY; the fake Greymon Tamer must never
    // be accepted by any bucket in that phase (only the Marcus Damon Digimon
    // may be picked there, but since it's not named Greymon it also should
    // not be offered in the Greymon bucket — that bucket should have ZERO
    // candidates here). Once bucket selection is done, the engine moves on
    // to an `OrderedPermutation` prompt (bottom-placement order for the
    // unchosen reveals) which legitimately reuses the same
    // `SEL_REVEAL_START`-based action-id range for a different purpose — do
    // not apply the "never legal" assertion there.
    while let Some(view) = runner.pending_selection_view() {
        let in_bucket_phase = matches!(view.kind, SelectionKind::RevealBucket { .. });
        if in_bucket_phase {
            let fake_action = revealed_action_for_id(&runner, "FAKE-GREYMON-TAMER");
            if let Some(fake) = fake_action {
                assert!(
                    !view.valid_action_ids.contains(&fake),
                    "a bucket wrongly accepted Tamer-kind 'Greymon': \
                     valid_action_ids = {:?}",
                    view.valid_action_ids
                );
            }
        }
        let marcus_action = revealed_action_for_id(&runner, "MARCUS");
        if in_bucket_phase {
            if let Some(marcus) = marcus_action {
                if view.valid_action_ids.contains(&marcus) {
                    runner.execute_action(0, marcus).expect("pick Marcus Damon");
                    continue;
                }
            }
        }
        let next = view.valid_action_ids[0];
        runner.execute_action(0, next).expect("advance");
    }

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"MARCUS".to_string()),
        "Marcus Damon must be picked for its bucket"
    );
    assert!(
        !hand_ids.contains(&"FAKE-GREYMON-TAMER".to_string()),
        "Tamer-kind 'Greymon' must never enter hand via either bucket"
    );
}

/// The Marcus Damon bucket requires an EXACT name match — a card merely
/// containing "Marcus Damon" as a substring of a longer name is fine (DCGO
/// uses `ContainsCardName`), but a Digimon simply named "Marcus" (not
/// "Marcus Damon") must NOT satisfy it.
#[test]
fn bt12_034_marcus_bucket_rejects_non_matching_name() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-034")
        .expect("BT12-034 YAML loads")
        .add_card(make_named_tamer("NOT-MARCUS", "Marcus", CardColor::Yellow))
        .add_card(make_named_digimon("GREYMON", "Greymon"))
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .deck(0, &["NOT-MARCUS", "GREYMON", "FILLER1", "FILLER2"])
        .hand(0, &["BT12-034"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play BT12-034");

    // Same RevealBucket-phase scoping as
    // `bt12_034_greymon_bucket_rejects_tamer_with_greymon_name` — the later
    // `OrderedPermutation` bottom-placement prompt legitimately reuses the
    // reveal action-id range for every unchosen card, including NOT-MARCUS.
    while let Some(view) = runner.pending_selection_view() {
        let in_bucket_phase = matches!(view.kind, SelectionKind::RevealBucket { .. });
        if in_bucket_phase {
            let not_marcus_action = revealed_action_for_id(&runner, "NOT-MARCUS");
            if let Some(bad) = not_marcus_action {
                assert!(
                    !view.valid_action_ids.contains(&bad),
                    "a Tamer named merely 'Marcus' (not 'Marcus Damon') must never \
                     be a legal pick in a bucket: valid_action_ids = {:?}",
                    view.valid_action_ids
                );
            }
        }
        let greymon_action = revealed_action_for_id(&runner, "GREYMON");
        if in_bucket_phase {
            if let Some(greymon) = greymon_action {
                if view.valid_action_ids.contains(&greymon) {
                    runner.execute_action(0, greymon).expect("pick Greymon");
                    continue;
                }
            }
        }
        let next = view.valid_action_ids[0];
        runner.execute_action(0, next).expect("advance");
    }

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"GREYMON".to_string()),
        "Greymon must be picked for its bucket"
    );
    assert!(
        !hand_ids.contains(&"NOT-MARCUS".to_string()),
        "'Marcus' alone must not satisfy the [Marcus Damon] bucket"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Inherited [Your Turn][OPT] on_suspend DP debuff: positive
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt12_034_debuffs_opp_digimon_when_own_yellow_tamer_suspends() {
    let mut runner = egg_runner();
    host_with_agumon(&mut runner, 0);
    let tamer = runner.place_on_field(0, "YEL-TAMER", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    assert_eq!(runner.dp_of(opp), Some(8000));

    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();

    // Mandatory single target: only one opponent Digimon exists, so the
    // selection (if any) auto-resolves to it via auto_resolve's first-legal
    // pick, which is unambiguous here.
    assert_eq!(
        runner.dp_of(opp),
        Some(6000),
        "own yellow Tamer suspend → 1 opponent Digimon gets -2000 DP for the turn"
    );
}

#[test]
fn bt12_034_debuffs_opp_digimon_when_own_red_tamer_suspends() {
    let mut runner = egg_runner();
    host_with_agumon(&mut runner, 0);
    let tamer = runner.place_on_field(0, "RED-TAMER", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.dp_of(opp),
        Some(6000),
        "own red Tamer suspend → 1 opponent Digimon gets -2000 DP for the turn"
    );
}

/// When the opponent has 2+ Digimon, the controller must explicitly choose
/// which one takes the debuff — the choice surfaces through pending_selection
/// (no-approximations: no auto-pick of "the first" or "the strongest").
#[test]
fn bt12_034_debuff_target_is_a_real_selection_among_multiple_opp_digimon() {
    let mut runner = egg_runner();
    host_with_agumon(&mut runner, 0);
    let tamer = runner.place_on_field(0, "YEL-TAMER", Some(0));
    let opp1 = runner.place_on_field(1, "OPP", Some(0));
    let opp2 = runner.place_on_field(1, "OPP2", Some(0));

    runner.game.suspend(tamer);

    let view = runner
        .pending_selection_view()
        .expect("a DP-debuff target selection must install");
    assert!(
        !runner.pending_is_optional(),
        "the debuff target pick is mandatory once the clause fires"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both opponent Digimon must be legal targets"
    );

    // Drive the explicit pick — choose the second opponent Digimon (opp2).
    let action = view.valid_action_ids[1];
    runner
        .execute_action(view.selecting_player, action)
        .expect("resolve DP-debuff target selection");
    let _ = runner.auto_resolve();

    let opp1_dp = runner.dp_of(opp1);
    let opp2_dp = runner.dp_of(opp2);
    let hit_opp1 = opp1_dp == Some(6000);
    let hit_opp2 = opp2_dp == Some(4000);
    assert!(
        hit_opp1 ^ hit_opp2,
        "exactly one opponent Digimon should have taken the -2000 debuff; \
         opp1={opp1_dp:?} opp2={opp2_dp:?}"
    );
}

#[test]
fn bt12_034_debuff_expires_at_end_of_turn() {
    let mut runner = egg_runner();
    host_with_agumon(&mut runner, 0);
    let tamer = runner.place_on_field(0, "YEL-TAMER", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();
    assert_eq!(runner.dp_of(opp), Some(6000), "debuff applied (8000-2000)");

    // Rotate to the opponent's turn and back — "for the turn" (the current,
    // own turn) must have cleared by the time it's this player's turn again.
    advance_to_turn_player(&mut runner, 1);
    advance_to_turn_player(&mut runner, 0);
    assert_eq!(
        runner.dp_of(opp),
        Some(8000),
        "debuff must expire once the turn it was applied in ends"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited on_suspend DP debuff: negatives
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt12_034_does_not_debuff_on_green_tamer_suspend() {
    let mut runner = egg_runner();
    host_with_agumon(&mut runner, 0);
    let tamer = runner.place_on_field(0, "GRN-TAMER", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.dp_of(opp),
        Some(8000),
        "a green Tamer is not yellow/red → no debuff"
    );
}

#[test]
fn bt12_034_does_not_debuff_on_own_digimon_suspend() {
    let mut runner = egg_runner();
    host_with_agumon(&mut runner, 0);
    let ally = runner.place_on_field(0, "FILLER", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.suspend(ally);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.dp_of(opp),
        Some(8000),
        "a Digimon (not a Tamer) suspending → no debuff"
    );
}

#[test]
fn bt12_034_does_not_debuff_on_opponent_tamer_suspend() {
    let mut runner = egg_runner();
    host_with_agumon(&mut runner, 0);
    let opp_tamer = runner.place_on_field(1, "YEL-TAMER", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.suspend(opp_tamer);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.dp_of(opp),
        Some(8000),
        "the OPPONENT's Tamer suspending does not satisfy 'your' Tamers"
    );
}

#[test]
fn bt12_034_does_not_debuff_on_opponents_turn() {
    let mut runner = egg_runner();
    host_with_agumon(&mut runner, 0);
    let tamer = runner.place_on_field(0, "YEL-TAMER", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));
    runner.game.turn_player_idx = 1; // opponent's turn

    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.dp_of(opp),
        Some(8000),
        "[Your Turn] gate blocks the debuff on the opponent's turn"
    );
}

/// DCGO's `HasMatchConditionPermanent` gate: with no opponent battle-area
/// Digimon at all, the clause must not install a dead/empty selection.
#[test]
fn bt12_034_no_selection_installs_when_opponent_has_no_digimon() {
    let mut runner = egg_runner();
    host_with_agumon(&mut runner, 0);
    let tamer = runner.place_on_field(0, "YEL-TAMER", Some(0));

    runner.game.suspend(tamer);

    assert!(
        runner.pending_selection_view().is_none(),
        "no opponent Digimon exists — no debuff-target selection should install"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — OPT lockout + clear
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt12_034_opt_blocks_second_suspend_debuff_same_turn() {
    let mut runner = egg_runner();
    host_with_agumon(&mut runner, 0);
    let t1 = runner.place_on_field(0, "YEL-TAMER", Some(0));
    let t2 = runner.place_on_field(0, "RED-TAMER", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.suspend(t1);
    let _ = runner.auto_resolve();
    assert_eq!(runner.dp_of(opp), Some(6000), "first debuff applied");

    runner.game.suspend(t2);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.dp_of(opp),
        Some(6000),
        "[Once Per Turn] blocks a second suspend-debuff the same turn, even \
         from a different (red) Tamer — shared OPT hash across both colors"
    );
}

#[test]
fn bt12_034_opt_clears_next_turn() {
    let mut runner = egg_runner();
    host_with_agumon(&mut runner, 0);
    let tamer = runner.place_on_field(0, "YEL-TAMER", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();
    assert_eq!(runner.dp_of(opp), Some(6000), "first debuff applied");

    // Rotate all the way back around to player 0's turn.
    for _ in 0..6 {
        runner.end_turn();
        let _ = runner.auto_resolve();
        if runner.game.turn_player() == 0 {
            break;
        }
    }
    assert_eq!(runner.game.turn_player(), 0, "back to P0's turn");
    // The previous debuff expired at end of the turn it was applied in.
    assert_eq!(runner.dp_of(opp), Some(8000), "prior debuff has expired");

    // Re-suspend the (now unsuspended, new-turn) yellow Tamer.
    let tamer = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "YEL-TAMER")
        .map(|i| PermanentHandle {
            player: 0,
            index: i as u8,
        })
        .expect("yellow Tamer still on field");
    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.dp_of(opp),
        Some(6000),
        "OPT lockout clears on the new turn — a fresh debuff applies"
    );
}
