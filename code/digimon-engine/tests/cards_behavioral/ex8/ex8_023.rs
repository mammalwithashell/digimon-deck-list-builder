//! EX8-023 PolarBearmon — Digimon, Lv.5, Blue/Yellow, DP 7000, Cost 7.
//! Traits: Ice-Snow, LIBERATOR. Attribute: Vaccine. Rarity: U.
//! Standard digivolution: Lv.4 blue for 4 memory (printed evo box).
//! Alt digivolution (printed evo box): "Digivolve Lv.4 w/[Ice-Snow] trait: Cost 3".
//!
//! # Card text (card image EX8-023 — authoritative)
//!
//! **Effect:**
//! <Iceclad> (Other than against Security Digimon, compare the number of
//! digivolution cards instead of DP in this Digimon's battles.)
//! [On Play] [When Digivolving] Trash any 2 digivolution cards from your
//! opponent's Digimon. Then, 1 of your opponent's Digimon with no
//! digivolution cards can't suspend or activate [When Digivolving] effects
//! until the end of their turn.
//!
//! **Inherited:**
//! [Your Turn] While your opponent has no Digimon with digivolution cards,
//! this Digimon with the [Ice-Snow] trait gains <Piercing> (When this Digimon
//! attacks and deletes an opponent's Digimon and survives the battle, it
//! performs any security checks it normally would.) and <Security A. +1>
//! (This Digimon checks 1 additional security card.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX8/Blue/EX8_023.cs
//!   - Alternate Digivolution Requirement (None):
//!     AddSelfDigivolutionRequirementStaticEffect(permanentCondition:
//!     EqualsTraits("Ice-Snow") && Level == 4, digivolutionCost: 3).
//!   - Iceclad (None): IcecladSelfStaticEffect(isInheritedEffect: false).
//!   - On Play + When Digivolving (OnEnterFieldAnyone, two ActivateClass with
//!     identical coroutines): CanActivateCondition gates on ≥1 opponent
//!     battle-area Digimon. SelectTrashDigivolutionCards(maxCount: 2,
//!     canNoTrash: false, isFromOnly1Permanent: false) — a MANDATORY
//!     cross-permanent pick of exactly min(2, available) opponent
//!     digivolution cards. THEN, if any opponent battle-area Digimon with
//!     DigivolutionCards.Count == 0 exists, a MANDATORY SelectPermanentEffect
//!     (maxCount: 1, canNoSelect: false) marks it with CanNotSuspendClass AND
//!     a DisableEffectClass over IsWhenDigivolving effects, both added to
//!     selectedPermanent.UntilOwnerTurnEndEffects (= until the end of the
//!     OPPONENT's turn). The "Then" half is NOT gated on the trash trashing
//!     anything — only on a sourceless opponent Digimon existing.
//!   - Your Turn (None + OnDetermineDoSecurityCheck, isInheritedEffect: true):
//!     ChangeSelfSAttackStaticEffect(+1) and PierceSelfEffect, both gated on
//!     IsOwnerTurn + carrier EqualsTraits("Ice-Snow") + opponent has NO
//!     Digimon with DigivolutionCards.
//!
//! # Patterns this test covers
//! - Iceclad combat semantics (EX8-022 idiom).
//! - Cross-permanent mandatory source trash clamped to min(2, available)
//!   (`select_opponent_sources` without `target:` + if/else count ladder).
//! - Post-trash mandatory lock select (a Digimon stripped bare by the trash
//!   IS a legal target) → CannotSuspend + CannotActivateWhenDigivolvingEffects,
//!   expiry end_of_opponents_turn (BT25-026 / BT20-084 idiom).
//! - WD-suppression: the locked Digimon's [When Digivolving] effects do not
//!   fire (effect_queue::permanent_activation_blocked_for_timing).
//! - Inherited conditional self-aura granting <Piercing> + Security A. +1
//!   (BT12-050 grant_keyword + EX11-002 active_when idioms).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{
    CardColor, CardKind, EffectTiming, Expiry, Keyword, ModifierType, PlaySource,
};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX8-023";

// ─── Fixture builders ─────────────────────────────────────────────────────────

/// A plain Lv.4 opponent Digimon. DP parameterized so the Iceclad battle tests
/// can prove DP is NOT the comparator.
fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, "Opp Digimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c
}

/// A BLUE Lv.4 opponent Digimon — a legal base for the STANDARD evo path
/// (Lv.4 blue: cost 4) so the opponent can digivolve it into EX8-023 in the
/// WD-suppression test.
fn blue_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, "Blue Lv4");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Blue];
    c
}

/// A generic digivolution-source filler card (stacked under tops).
fn source_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, "Source Filler");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(1000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c
}

/// A RED Lv.4 Digimon with the [Ice-Snow] trait — matches ONLY PolarBearmon's
/// printed alt evo box ("Lv.4 w/[Ice-Snow] trait: Cost 3"), not the standard
/// Lv.4-blue path, so digivolving over it proves the alt path works and
/// charges its printed cost 3.
fn red_ice_snow_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, "Red IceSnow Lv4");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Ice-Snow".to_string()];
    c
}

/// A carrier Digimon WITH the [Ice-Snow] trait — EX8-023 sits underneath it
/// as a digivolution source so its inherited [Your Turn] aura applies.
fn ice_snow_carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, "IceSnow Carrier");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 12;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Ice-Snow".to_string()];
    c
}

/// A carrier WITHOUT the [Ice-Snow] trait — the inherited aura must stay off.
fn plain_carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, "Plain Carrier");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 12;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Beast".to_string()];
    c
}

fn filler(id: &str) -> CardData {
    make_test_card(id, "Filler")
}

/// Push a registered card into a player's hand; returns the hand index.
fn put_in_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("put_in_hand: card {card_id} not registered"));
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_idx, player, card_index));
    runner.game.player(player).hand.len() - 1
}

/// True when `handle` carries a modifier entry of `ty` with the given expiry.
fn has_entry_with_expiry(
    runner: &DebugRunner,
    handle: PermanentHandle,
    ty: ModifierType,
    expiry: Expiry,
) -> bool {
    runner
        .game
        .modifiers
        .get(handle, ty)
        .iter()
        .any(|entry| entry.modifier == ty && entry.expiry == expiry)
}

/// Assert the full lock landed: CannotSuspend AND
/// CannotActivateWhenDigivolvingEffects, both until end of opponent's turn.
fn assert_locked(runner: &DebugRunner, handle: PermanentHandle) {
    assert!(
        runner.game.modifiers.has(handle, ModifierType::CannotSuspend),
        "locked Digimon must hold CannotSuspend"
    );
    assert!(
        runner
            .game
            .modifiers
            .has(handle, ModifierType::CannotActivateWhenDigivolvingEffects),
        "locked Digimon must hold CannotActivateWhenDigivolvingEffects"
    );
    assert!(
        has_entry_with_expiry(
            runner,
            handle,
            ModifierType::CannotSuspend,
            Expiry::EndOfOpponentsTurn
        ),
        "CannotSuspend must expire at the end of the opponent's turn"
    );
    assert!(
        has_entry_with_expiry(
            runner,
            handle,
            ModifierType::CannotActivateWhenDigivolvingEffects,
            Expiry::EndOfOpponentsTurn
        ),
        "CannotActivateWhenDigivolvingEffects must expire at the end of the opponent's turn"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

/// EX8-023 compiles with its printed stats: Lv.5 blue/yellow Digimon, cost 7,
/// DP 7000, traits Ice-Snow + LIBERATOR, the standard Lv.4-blue cost-4 evo
/// path, and the printed "Digivolve Lv.4 w/[Ice-Snow] trait: Cost 3" alt path
/// (DCGO gates on EqualsTraits("Ice-Snow") && Level == 4).
#[test]
fn ex8_023_compiles_with_printed_stats_and_digivolve_paths() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-023 found in embedded DSL pack")
        .start();

    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX8-023 in compiled cards");

    assert_eq!(compiled.card, "EX8-023");
    assert_eq!(compiled.name, "PolarBearmon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(5));
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Blue, CompiledColor::Yellow],
        "cards.json card_colors [1, 2] = blue + yellow (multi-color)"
    );
    assert_eq!(compiled.cost, Some(7));
    assert_eq!(compiled.dp, Some(7000));
    for trait_name in ["Ice-Snow", "LIBERATOR"] {
        assert!(
            compiled.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }

    // Standard printed evo box: Lv.4 blue for 4 memory.
    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(4))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(4)
                        || from.all_of.iter().any(|pred| pred.level_eq == Some(4)))
                        && (from.color_is == Some(CompiledColor::Blue)
                            || from
                                .all_of
                                .iter()
                                .any(|pred| pred.color_is == Some(CompiledColor::Blue)))
                })
        }),
        "PolarBearmon must digivolve from a blue Lv.4 for cost 4 (printed evo box)"
    );

    // Printed alt evo box: Lv.4 with [Ice-Snow] trait for 3 memory.
    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(3))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(4)
                        || from.all_of.iter().any(|pred| pred.level_eq == Some(4)))
                        && (from.trait_has.as_deref() == Some("Ice-Snow")
                            || from
                                .all_of
                                .iter()
                                .any(|pred| pred.trait_has.as_deref() == Some("Ice-Snow")))
                })
        }),
        "PolarBearmon must digivolve from a Lv.4 [Ice-Snow] Digimon for cost 3 (printed alt evo box)"
    );
}

/// The printed <Iceclad> keyword is encoded as a face-up grant_keyword clause
/// AND is live on the placed permanent via the canonical `Game::has_keyword`
/// query (the combat resolver's lookup site).
#[test]
fn ex8_023_iceclad_keyword_attached() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-023 found in embedded DSL pack")
        .start();

    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX8-023 in compiled cards");
    let has_iceclad_clause = compiled.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope,
                ..
            }) if keyword == "Iceclad" && *scope != CompiledScope::Inherited
        )
    });
    assert!(
        has_iceclad_clause,
        "EX8-023 must encode <Iceclad> as a face-up grant_keyword clause"
    );

    let bearmon = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(
        runner.game.has_keyword(bearmon, Keyword::Iceclad),
        "the placed PolarBearmon must answer has_keyword(Iceclad) — the combat \
         resolver consumes Iceclad through this exact query"
    );
}

/// The [On Play][When Digivolving] clause is face-up, fires at both timings,
/// and is NOT optional and NOT once-per-turn (no printed qualifiers).
#[test]
fn ex8_023_on_play_wd_clause_structural_shape() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-023 found in embedded DSL pack")
        .start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX8-023 in compiled cards");

    let triggered = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("EX8-023 must encode an [On Play][When Digivolving] clause");

    assert_eq!(
        triggered.scope,
        CompiledScope::FaceUp,
        "the trash-2-sources clause is face-up card text"
    );
    assert!(
        !triggered.optional,
        "no printed 'you may' — the effect (and its selections) is mandatory"
    );
    assert!(
        !triggered.once_per_turn,
        "printed text carries no [Once Per Turn]"
    );
}

/// The inherited [Your Turn] clause is an Inherited-scoped declarative aura
/// granting <Piercing> AND Security A. +1 behind an active_when gate.
#[test]
fn ex8_023_inherited_aura_structural_shape() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-023 found in embedded DSL pack")
        .start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX8-023 in compiled cards");

    let aura = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope,
                active_when,
                security_attack,
                grant_keyword,
                ..
            }) if *scope == CompiledScope::Inherited => {
                Some((active_when, security_attack, grant_keyword))
            }
            _ => None,
        })
        .expect("EX8-023 must encode an inherited declarative aura");

    let (active_when, security_attack, grant_keyword) = aura;
    assert!(
        active_when.is_some(),
        "the inherited aura must carry an active_when gate ([Your Turn] + \
         opponent-no-sourced-Digimon + Ice-Snow carrier trait)"
    );
    assert_eq!(
        *security_attack,
        Some(1),
        "the inherited aura grants <Security A. +1>"
    );
    assert_eq!(
        grant_keyword.as_ref().map(|g| g.keyword.as_str()),
        Some("Piercing"),
        "the inherited aura grants <Piercing>"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — <Iceclad> battle semantics (stack-count compare, not DP)
// ─────────────────────────────────────────────────────────────────────────────

/// Iceclad attacker wins on source count despite LOWER DP: PolarBearmon
/// (7000 DP) with 1 source under it (2 cards total) beats a 10000-DP defender
/// with 1 card. Without Iceclad the DP compare (7000 < 10000) would delete
/// PolarBearmon.
#[test]
fn ex8_023_iceclad_wins_battle_on_source_count_despite_lower_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-023 found in embedded DSL pack")
        .add_card(source_filler("SRC-A"))
        .add_card(opp_digimon("BIG-DEF", 10000))
        .start();

    let bearmon = runner.place_stack(0, &["SRC-A", CARD_ID]);
    let defender = runner.place_on_field(1, "BIG-DEF", Some(0));

    let result = runner.attack_digimon(bearmon, defender, false);

    assert_eq!(
        result,
        AttackResult::AttackerWins,
        "Iceclad compares stack sizes (2 > 1), not DP (7000 < 10000)"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "PolarBearmon survives via source-count advantage"
    );
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        0,
        "the higher-DP defender is deleted — fewer sources loses under Iceclad"
    );
}

/// Iceclad attacker LOSES with fewer sources despite higher DP: bare
/// PolarBearmon (7000 DP, 1 card) attacks a 1000-DP defender with a 3-card
/// stack. Without Iceclad the DP compare (7000 > 1000) would delete the
/// defender.
#[test]
fn ex8_023_iceclad_loses_with_fewer_sources_despite_higher_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-023 found in embedded DSL pack")
        .add_card(source_filler("SRC-A"))
        .add_card(source_filler("SRC-B"))
        .add_card(opp_digimon("WEAK-DEF", 1000))
        .start();

    let bearmon = runner.place_on_field(0, CARD_ID, Some(0));
    let defender = runner.place_stack(1, &["SRC-A", "SRC-B", "WEAK-DEF"]);

    let result = runner.attack_digimon(bearmon, defender, false);

    assert_eq!(
        result,
        AttackResult::DefenderWins,
        "Iceclad compares stack sizes (1 < 3) — PolarBearmon loses despite 7000 vs 1000 DP"
    );
    assert_eq!(runner.game.players[0].battle_area.len(), 0);
    assert_eq!(runner.game.players[1].battle_area.len(), 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — [On Play][When Digivolving] trash any 2 cross-permanent + lock
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE (cross-permanent): the opponent has TWO Digimon, each carrying 1
/// digivolution card. On Play installs a MANDATORY 2-pick cross-permanent
/// source selection (no PASS before the minimum); picking one source from
/// EACH stack trashes both. Both Digimon are now sourceless, so the lock
/// select installs; the chosen one gets CannotSuspend AND
/// CannotActivateWhenDigivolvingEffects until the end of the opponent's turn.
#[test]
fn ex8_023_on_play_trashes_two_sources_across_permanents_then_locks() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-023 found in embedded DSL pack")
        .add_card(source_filler("SRC-A"))
        .add_card(source_filler("SRC-B"))
        .add_card(opp_digimon("OPP-A", 5000))
        .add_card(opp_digimon("OPP-B", 5000))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let opp_a = runner.place_stack(1, &["SRC-A", "OPP-A"]);
    let opp_b = runner.place_stack(1, &["SRC-B", "OPP-B"]);

    let bearmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, bearmon.index as usize);

    // First selection: the cross-permanent source pick, exactly 2, mandatory.
    let view = runner
        .pending_selection_view()
        .expect("[On Play] must install the cross-permanent source selection");
    assert!(
        matches!(
            view.kind,
            SelectionKind::SourceMulti {
                min: 2,
                max: 2,
                picked: 0
            }
        ),
        "expected a mandatory exactly-2 source pick; got {:?}",
        view.kind
    );
    assert!(
        !runner.pending_is_optional(),
        "printed text has no 'you may' — no PASS before the minimum is met"
    );
    // Pick one source from each opponent stack (first candidate twice — the
    // pickers re-enumerate live candidates between picks).
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick the first opponent digivolution card");
    let view = runner
        .pending_selection_view()
        .expect("the second pick of the mandatory 2 must still be pending");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick the second opponent digivolution card");

    // Both sources trashed, both stacks stripped to their top card.
    assert_eq!(
        runner.game.players[1].battle_area[opp_a.index as usize]
            .card_sources
            .len(),
        1,
        "OPP-A's digivolution card was trashed"
    );
    assert_eq!(
        runner.game.players[1].battle_area[opp_b.index as usize]
            .card_sources
            .len(),
        1,
        "OPP-B's digivolution card was trashed"
    );
    assert_eq!(
        runner.game.players[1].trash.len(),
        2,
        "both trashed sources land in the opponent's trash"
    );

    // Then: the lock select over sourceless opponent Digimon (both qualify).
    let view = runner
        .pending_selection_view()
        .expect("the 'Then' lock selection must install after the trash");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(
        !runner.pending_is_optional(),
        "the lock selection is mandatory (no printed 'you may')"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both (now sourceless) opponent Digimon are legal lock targets"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the first opponent Digimon as the lock target");
    runner.auto_resolve().expect("finish the On Play effect");

    assert_locked(&runner, opp_a);
    assert!(
        !runner
            .game
            .modifiers
            .has(opp_b, ModifierType::CannotSuspend),
        "only the CHOSEN Digimon is locked"
    );
}

/// Up-to clamp (DCGO maxDigivolutionDiscardCount = min(available, 2)): with
/// only ONE digivolution card on the opponent's side, the pick is a mandatory
/// exactly-1 selection — the player must trash that 1, then the lock half
/// still runs.
#[test]
fn ex8_023_on_play_trash_clamps_to_single_available_source() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-023 found in embedded DSL pack")
        .add_card(source_filler("SRC-A"))
        .add_card(opp_digimon("OPP-A", 5000))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let opp_a = runner.place_stack(1, &["SRC-A", "OPP-A"]);

    let bearmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, bearmon.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("[On Play] must install the source selection clamped to 1");
    assert!(
        matches!(
            view.kind,
            SelectionKind::SourceMulti {
                min: 1,
                max: 1,
                picked: 0
            }
        ),
        "with 1 available source the pick clamps to a mandatory exactly-1; got {:?}",
        view.kind
    );
    assert!(!runner.pending_is_optional(), "mandatory — no PASS");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("trash the only opponent digivolution card");

    assert_eq!(
        runner.game.players[1].battle_area[opp_a.index as usize]
            .card_sources
            .len(),
        1,
        "the single digivolution card was trashed"
    );
    assert_eq!(runner.game.players[1].trash.len(), 1);

    // The "Then" lock half still runs after the clamped trash.
    let view = runner
        .pending_selection_view()
        .expect("the lock selection must still install after a clamped 1-card trash");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the (now sourceless) opponent Digimon");
    runner.auto_resolve().expect("finish the On Play effect");

    assert_locked(&runner, opp_a);
}

/// Zero opponent digivolution cards: the trash half is skipped entirely (no
/// source selection), but the "Then" half still runs — DCGO's lock guard only
/// requires a sourceless opponent Digimon to exist.
#[test]
fn ex8_023_on_play_skips_trash_with_no_sources_but_still_locks() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-023 found in embedded DSL pack")
        .add_card(opp_digimon("OPP-A", 5000))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));

    let bearmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, bearmon.index as usize);

    // No sources anywhere → the first pending selection is the LOCK select.
    let view = runner
        .pending_selection_view()
        .expect("with no opponent sources, the lock selection installs directly");
    assert_eq!(
        view.kind,
        SelectionKind::OppField,
        "no source pick should appear — the opponent has no digivolution cards"
    );
    assert!(!runner.pending_is_optional(), "mandatory — no PASS");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the sourceless opponent Digimon");
    runner.auto_resolve().expect("finish the On Play effect");

    assert_eq!(runner.game.players[1].trash.len(), 0, "nothing was trashed");
    assert_locked(&runner, opp_a);
}

/// NEGATIVE (lock gate): when every opponent Digimon still has digivolution
/// cards after the trash, the lock selection must NOT install and no modifier
/// lands anywhere.
#[test]
fn ex8_023_on_play_lock_skipped_when_no_sourceless_digimon_remains() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-023 found in embedded DSL pack")
        .add_card(source_filler("SRC-A"))
        .add_card(source_filler("SRC-B"))
        .add_card(source_filler("SRC-C"))
        .add_card(source_filler("SRC-D"))
        .add_card(source_filler("SRC-E"))
        .add_card(source_filler("SRC-F"))
        .add_card(opp_digimon("OPP-A", 5000))
        .add_card(opp_digimon("OPP-B", 5000))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // Both opponent stacks carry 3 sources — trashing any 2 cannot strip
    // either below 1 source.
    let opp_a = runner.place_stack(1, &["SRC-A", "SRC-B", "SRC-C", "OPP-A"]);
    let opp_b = runner.place_stack(1, &["SRC-D", "SRC-E", "SRC-F", "OPP-B"]);

    let bearmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, bearmon.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("[On Play] must install the cross-permanent source selection");
    assert!(matches!(
        view.kind,
        SelectionKind::SourceMulti {
            min: 2,
            max: 2,
            picked: 0
        }
    ));
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("first source pick");
    let view = runner.pending_selection_view().expect("second pick pending");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("second source pick");

    assert_eq!(runner.game.players[1].trash.len(), 2, "2 sources trashed");
    assert!(
        runner.pending_selection().is_none(),
        "every opponent Digimon still has digivolution cards — the lock \
         selection must be silently skipped"
    );
    for handle in [opp_a, opp_b] {
        assert!(
            !runner
                .game
                .modifiers
                .has(handle, ModifierType::CannotSuspend),
            "no lock may land when no sourceless opponent Digimon exists"
        );
    }
}

/// A Digimon stripped bare BY THE TRASH is a legal lock target: OPP-A carries
/// 2 sources, OPP-B carries 1. Picking both of OPP-A's sources leaves OPP-A
/// sourceless and OPP-B still sourced — the lock select must offer EXACTLY
/// OPP-A.
#[test]
fn ex8_023_on_play_stripped_digimon_is_legal_lock_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-023 found in embedded DSL pack")
        .add_card(source_filler("SRC-A"))
        .add_card(source_filler("SRC-B"))
        .add_card(source_filler("SRC-C"))
        .add_card(opp_digimon("OPP-A", 5000))
        .add_card(opp_digimon("OPP-B", 5000))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // OPP-A first (field index 0) so its sources enumerate first.
    let opp_a = runner.place_stack(1, &["SRC-A", "SRC-B", "OPP-A"]);
    let opp_b = runner.place_stack(1, &["SRC-C", "OPP-B"]);

    let bearmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, bearmon.index as usize);

    // Pick both of OPP-A's sources (its candidates enumerate first).
    let view = runner
        .pending_selection_view()
        .expect("source selection installs");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick OPP-A's first source");
    let view = runner.pending_selection_view().expect("second pick pending");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick OPP-A's second source");

    assert_eq!(
        runner.game.players[1].battle_area[opp_a.index as usize]
            .card_sources
            .len(),
        1,
        "OPP-A was stripped bare by the trash"
    );
    assert_eq!(
        runner.game.players[1].battle_area[opp_b.index as usize]
            .card_sources
            .len(),
        2,
        "OPP-B still carries its digivolution card"
    );

    let view = runner
        .pending_selection_view()
        .expect("the lock selection must install — OPP-A is now sourceless");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the freshly-stripped OPP-A is a legal lock target (OPP-B is sourced)"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("lock OPP-A");
    runner.auto_resolve().expect("finish the On Play effect");

    assert_locked(&runner, opp_a);
    assert!(
        !runner
            .game
            .modifiers
            .has(opp_b, ModifierType::CannotSuspend),
        "the sourced OPP-B must not be locked"
    );
}

/// NEGATIVE (clause gate): with NO opponent Digimon at all, nothing happens —
/// no source selection, no lock selection (DCGO CanActivateCondition requires
/// an opponent battle-area Digimon).
#[test]
fn ex8_023_on_play_with_no_opponent_digimon_does_nothing() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-023 found in embedded DSL pack")
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let bearmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, bearmon.index as usize);

    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon — neither the trash nor the lock selection installs"
    );
}

/// WHEN DIGIVOLVING via the printed alt evo box: PolarBearmon digivolves over
/// a RED Lv.4 [Ice-Snow] base (matching ONLY the alt path) for its printed
/// cost 3, then the [When Digivolving] effect fires the same trash + lock
/// pipeline.
#[test]
fn ex8_023_when_digivolving_via_ice_snow_alt_path_costs_3_and_fires_effect() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-023 found in embedded DSL pack")
        .add_card(red_ice_snow_lv4("ICEBASE"))
        .add_card(source_filler("SRC-A"))
        .add_card(source_filler("SRC-B"))
        .add_card(opp_digimon("OPP-A", 5000))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(8)
        .start();
    runner.game.turn_count = 1;

    let opp_a = runner.place_stack(1, &["SRC-A", "SRC-B", "OPP-A"]);
    let base = runner.place_on_field(0, "ICEBASE", Some(0));
    let hand_idx = put_in_hand(&mut runner, 0, CARD_ID);

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByHand);
    assert!(
        digivolved,
        "PolarBearmon must digivolve over the red Lv.4 [Ice-Snow] base via \
         the printed alt evo box (Lv.4 w/[Ice-Snow]: cost 3)"
    );
    assert_eq!(
        runner.game.memory,
        memory_before - 3,
        "the alt path charges its printed cost 3 (the standard Lv.4-blue \
         path does not match a red base)"
    );

    runner.game.drain_effect_queue();
    let view = runner
        .pending_selection_view()
        .expect("[When Digivolving] must install the source selection");
    assert!(matches!(
        view.kind,
        SelectionKind::SourceMulti {
            min: 2,
            max: 2,
            picked: 0
        }
    ));
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("first source pick");
    let view = runner.pending_selection_view().expect("second pick pending");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("second source pick");

    assert_eq!(
        runner.game.players[1].battle_area[opp_a.index as usize]
            .card_sources
            .len(),
        1,
        "both of the opponent's digivolution cards are trashed"
    );

    // OPP-A is now sourceless → the lock select installs.
    let view = runner
        .pending_selection_view()
        .expect("the lock selection must install after the WD trash");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("lock OPP-A");
    runner.auto_resolve().expect("finish the WD effect");

    assert_locked(&runner, opp_a);
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Lock semantics: expiry + [When Digivolving] suppression
// ─────────────────────────────────────────────────────────────────────────────

/// "Until the end of THEIR turn": the lock survives the end of the granting
/// player's turn and expires at the end of the opponent's (the target
/// controller's) turn — bt20_084 expire_end_of_turn idiom.
#[test]
fn ex8_023_lock_expires_at_end_of_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-023 found in embedded DSL pack")
        .add_card(opp_digimon("OPP-A", 5000))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let bearmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, bearmon.index as usize);

    let view = runner.pending_selection_view().expect("lock selection");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("lock the opponent Digimon");
    runner.auto_resolve().expect("finish the On Play effect");
    assert_locked(&runner, opp_a);

    // End of the CONTROLLER's turn: the lock must persist.
    runner.game.modifiers.expire_end_of_turn(0);
    assert!(
        runner
            .game
            .modifiers
            .has(opp_a, ModifierType::CannotSuspend),
        "the lock must survive the granting player's own turn end"
    );
    assert!(
        runner
            .game
            .modifiers
            .has(opp_a, ModifierType::CannotActivateWhenDigivolvingEffects),
        "the WD lock must survive the granting player's own turn end"
    );

    // End of the OPPONENT's turn: both halves of the lock expire.
    runner.game.modifiers.expire_end_of_turn(1);
    assert!(
        !runner
            .game
            .modifiers
            .has(opp_a, ModifierType::CannotSuspend),
        "CannotSuspend expires at the end of the opponent's turn"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(opp_a, ModifierType::CannotActivateWhenDigivolvingEffects),
        "CannotActivateWhenDigivolvingEffects expires at the end of the opponent's turn"
    );
}

/// BEHAVIORAL (WD suppression): the locked Digimon's [When Digivolving]
/// effects do not fire when it digivolves during the lock window. Control:
/// the opponent first digivolves an UNLOCKED sibling into EX8-023 and its
/// [When Digivolving] selection installs; then digivolves the LOCKED Digimon
/// into EX8-023 and nothing installs.
#[test]
fn ex8_023_locked_digimon_when_digivolving_effects_suppressed() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-023 found in embedded DSL pack")
        .add_card(blue_lv4("BASE-A"))
        .add_card(blue_lv4("BASE-B"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .memory(20)
        .start();
    runner.game.turn_count = 1;

    // Two sourceless opponent Lv.4 blue Digimon. BASE-A (index 0) will be
    // locked; BASE-B is the unlocked control.
    let base_a = runner.place_on_field(1, "BASE-A", Some(0));
    let base_b = runner.place_on_field(1, "BASE-B", Some(0));

    // P0 plays PolarBearmon: no sources to trash, lock select over both
    // sourceless Digimon — pick BASE-A (first candidate).
    let bearmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, bearmon.index as usize);
    let view = runner.pending_selection_view().expect("lock selection");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("lock BASE-A");
    runner.auto_resolve().expect("finish the On Play effect");
    assert_locked(&runner, base_a);
    assert!(
        !runner
            .game
            .modifiers
            .has(base_b, ModifierType::CannotSuspend),
        "BASE-B (control) must remain unlocked"
    );

    // Move to the opponent's turn — the lock lasts until THEIR turn ends.
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.game.set_memory(10);
    assert!(
        runner
            .game
            .modifiers
            .has(base_a, ModifierType::CannotActivateWhenDigivolvingEffects),
        "the WD lock must still be live on the opponent's own turn"
    );

    // CONTROL: the UNLOCKED BASE-B digivolves into EX8-023 → its
    // [When Digivolving] fires. P0 has a sourceless Digimon (PolarBearmon
    // itself), so the lock select installs for player 1.
    let hand_b = put_in_hand(&mut runner, 1, CARD_ID);
    assert!(
        runner
            .game
            .digivolve_from_hand(1, hand_b, base_b.index as usize, PlaySource::ByHand),
        "the unlocked BASE-B must digivolve into PolarBearmon (Lv.4 blue, cost 4)"
    );
    runner.game.drain_effect_queue();
    let view = runner.pending_selection_view().expect(
        "control: the unlocked Digimon's [When Digivolving] must install its \
         lock selection (P0's PolarBearmon is a sourceless target)",
    );
    assert_eq!(view.selecting_player, 1, "the digivolving player selects");
    runner
        .execute_action(1, view.valid_action_ids[0])
        .expect("resolve the control WD lock selection");
    runner.auto_resolve().expect("finish the control WD effect");

    // SUPPRESSED: the LOCKED BASE-A digivolves into EX8-023 → its
    // [When Digivolving] effects must NOT fire (no selection installs).
    let hand_a = put_in_hand(&mut runner, 1, CARD_ID);
    assert!(
        runner
            .game
            .digivolve_from_hand(1, hand_a, base_a.index as usize, PlaySource::ByHand),
        "the locked BASE-A can still digivolve — only its WD effects are suppressed"
    );
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_selection().is_none(),
        "the locked Digimon's [When Digivolving] effects must not activate"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — Inherited [Your Turn] aura: <Piercing> + <Security A. +1>
// ─────────────────────────────────────────────────────────────────────────────

fn aura_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-023 found in embedded DSL pack")
        .add_card(ice_snow_carrier("ICE-CARRIER"))
        .add_card(plain_carrier("PLAIN-CARRIER"))
        .add_card(opp_digimon("OPP-DEF", 4000))
        .add_card(source_filler("OPP-SRC"))
        .start()
}

/// AURA ON: carrier has [Ice-Snow], it's the controller's turn, and the
/// opponent has no Digimon with digivolution cards → the carrier gains
/// <Piercing> and Security Attack +1.
#[test]
fn ex8_023_inherited_aura_grants_piercing_and_security_attack_plus_1() {
    let mut r = aura_runner();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let carrier = r.place_stack(tp, &[CARD_ID, "ICE-CARRIER"]);
    // Opponent: a single sourceless Digimon (condition satisfied).
    r.place_on_field(opp, "OPP-DEF", Some(0));

    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert!(
        r.game.has_keyword(carrier, Keyword::Piercing),
        "carrier must gain <Piercing> while the inherited gate is satisfied"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        1,
        "carrier must gain <Security A. +1> while the inherited gate is satisfied"
    );
}

/// AURA OFF: an opponent Digimon has a digivolution card → no grants, even
/// though every other condition is satisfied.
#[test]
fn ex8_023_inherited_aura_off_when_opponent_has_sourced_digimon() {
    let mut r = aura_runner();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let carrier = r.place_stack(tp, &[CARD_ID, "ICE-CARRIER"]);
    // Opponent Digimon WITH a digivolution card under it.
    r.place_stack(opp, &["OPP-SRC", "OPP-DEF"]);

    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert!(
        !r.game.has_keyword(carrier, Keyword::Piercing),
        "<Piercing> must be absent while any opponent Digimon has digivolution cards"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        0,
        "Security A. +1 must be absent while any opponent Digimon has digivolution cards"
    );
}

/// AURA OFF: the carrier lacks the [Ice-Snow] trait ("this Digimon with the
/// [Ice-Snow] trait") → no grants.
#[test]
fn ex8_023_inherited_aura_off_when_carrier_lacks_ice_snow_trait() {
    let mut r = aura_runner();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let carrier = r.place_stack(tp, &[CARD_ID, "PLAIN-CARRIER"]);
    r.place_on_field(opp, "OPP-DEF", Some(0));

    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert!(
        !r.game.has_keyword(carrier, Keyword::Piercing),
        "<Piercing> must be absent when the carrier lacks the Ice-Snow trait"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        0,
        "Security A. +1 must be absent when the carrier lacks the Ice-Snow trait"
    );
}

/// AURA OFF: [Your Turn] — on the opponent's turn the grants are absent even
/// with the board condition satisfied.
#[test]
fn ex8_023_inherited_aura_off_on_opponents_turn() {
    let mut r = aura_runner();
    let tp = r.game.turn_player();
    let non_tp = 1 - tp;

    // The carrier stack belongs to the NON-turn player; the turn player has
    // a single sourceless Digimon (the board condition holds for non_tp).
    let carrier = r.place_stack(non_tp, &[CARD_ID, "ICE-CARRIER"]);
    r.place_on_field(tp, "OPP-DEF", Some(0));

    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert!(
        !r.game.has_keyword(carrier, Keyword::Piercing),
        "[Your Turn] gate: <Piercing> must be absent on the opponent's turn"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        0,
        "[Your Turn] gate: Security A. +1 must be absent on the opponent's turn"
    );
}
