//! EX8-028 Skadimon — Digimon, Lv.6, Blue/Yellow, DP 12000, Cost 12.
//! Traits: Ice-Snow, LIBERATOR. Form: Mega. Attribute: Vaccine. Rarity: SR.
//! Standard digivolution: Lv.5 blue for 4 memory (printed evo box).
//! Alt digivolution (printed evo box): "Digivolve Lv.5 w/[Ice-Snow] trait: Cost 3".
//!
//! # Card text (card image EX8-028.webp — authoritative; cards.json agrees)
//!
//! **Effect:**
//! <Iceclad> (Other than against Security Digimon, compare the number of
//! digivolution cards instead of DP in this Digimon's battles.)
//! <Barrier> (When this Digimon would be deleted in battle, by trashing the
//! top card of your security stack, prevent that deletion.)
//! [When Digivolving] You may play 1 level 4 or lower [Ice-Snow] trait Digimon
//! card from your hand without paying the cost. For each of your opponent's
//! Digimon with no digivolution cards, add 1 to this effect's level maximum.
//! [When Digivolving] [When Attacking] [Once Per Turn] By placing 1 Digimon
//! with no digivolution cards as the bottom security card, this Digimon
//! unsuspends.
//!
//! **Inherited:** (none)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX8/Blue/EX8_028.cs
//!   - Alternate Digivolution Requirement (None):
//!     AddSelfDigivolutionRequirementStaticEffect(permanentCondition:
//!     EqualsTraits("Ice-Snow") && HasLevel && Level == 5, digivolutionCost: 3).
//!   - Iceclad (None): IcecladSelfStaticEffect(isInheritedEffect: false).
//!   - Barrier (WhenPermanentWouldBeDeleted): BarrierSelfEffect(false).
//!   - When Digivolving (OnEnterFieldAnyone, ActivateClass, maxCount -1,
//!     isOptional true): MaxLevel() = 4 + Enemy.GetBattleAreaDigimons()
//!     .Count(p => p.DigivolutionCards.Count == 0) — the level cap is computed
//!     LIVE at candidate-filter time. CanSelectCardCondition: IsDigimon &&
//!     CanPlayAsNewPermanent(payCost: false) && EqualsTraits("Ice-Snow") &&
//!     Level <= MaxLevel(). SelectHandEffect(maxCount: 1, canNoSelect: TRUE —
//!     printed "you may", decline legal) → PlayPermanentCards(payCost: false,
//!     isTapped: false, activateETB: true).
//!   - When Digivolving — OPT (OnEnterFieldAnyone, ActivateClass, maxCount 1,
//!     isOptional true, SetHashString("Unsuspend_EX8_028")) AND
//!     When Attacking — OPT (OnAllyAttack + CanTriggerOnAttack, ActivateClass,
//!     maxCount 1, isOptional true, SAME hash "Unsuspend_EX8_028" → the two
//!     timings share ONE once-per-turn counter):
//!     CanSelectPermanentCondition: IsPermanentExistsOnBattleAreaDigimon
//!     (EITHER side's battle area — HasMatchConditionPermanent scans
//!     gameContext.Players.Map(GetBattleAreaPermanents)) &&
//!     DigivolutionCards.Count == 0. SelectPermanentEffect(maxCount: 1,
//!     canNoSelect: FALSE — mandatory once activated). On pick:
//!     IPutSecurityPermanent(selectedCard, …, toTop: false, isFaceup: false)
//!     → the placed TOP CARD lands at the BOTTOM of topCard.OWNER's security
//!     stack, face down. Then, if the placed top card actually left the
//!     battle area, IUnsuspendPermanents([card.PermanentOfThisCard()]).
//!
//! # Patterns this test covers
//! - Iceclad combat semantics (EX8-022/EX8-023 idiom).
//! - Barrier battle-deletion replacement (EX11-019 / BT23-102 idiom).
//! - Formula-valued level cap on a select_hand filter (PR #470 DpConstraint
//!   formula; base 4 + 1 per sourceless opponent Digimon).
//! - Free play from hand (EX11-015 play_from_hand_free idiom), decline legal.
//! - By-cost optional clause (BT17-077 grammar) with outer accept/decline
//!   prompt + mandatory inner select over BOTH battle areas
//!   (select_any_permanent → SelectionKind::AnyField).
//! - place_permanent_on_security bottom, destination = target OWNER's stack
//!   (binding_owner branch; DCGO topCard.Owner.SecurityCards).
//! - OPT shared across [When Digivolving] and [When Attacking] (one clause,
//!   two timings — DCGO shared hash "Unsuspend_EX8_028"); decline does not
//!   consume the OPT.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX8-028";

// ─── Fixture builders ─────────────────────────────────────────────────────────

/// An [Ice-Snow] trait Digimon of the given level — the free-play candidate.
fn ice_snow_digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, &format!("IceSnow Lv{level}"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Ice-Snow".to_string()];
    c
}

/// A non-[Ice-Snow] Digimon — must never be a free-play candidate.
fn plain_digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, &format!("Plain Lv{level}"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Beast".to_string()];
    c
}

/// A generic opponent/ally board Digimon (Lv.4).
fn board_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, "Board Digimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c
}

/// An opponent TAMER — sourceless but must count for NEITHER the level-cap
/// formula NOR the pay-target set (both are Digimon-only).
fn opp_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, "Opp Tamer");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c.colors = vec![CardColor::Red];
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

/// A RED Lv.5 [Ice-Snow] Digimon — matches ONLY Skadimon's printed alt evo box
/// ("Lv.5 w/[Ice-Snow] trait: Cost 3"), not the standard Lv.5-blue path.
fn red_ice_snow_lv5(id: &str) -> CardData {
    let mut c = make_test_card(id, "Red IceSnow Lv5");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(7000);
    c.play_cost = 7;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Ice-Snow".to_string()];
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

/// Fire [When Digivolving] for `handle` without a full digivolve action.
fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

/// Fire [When Attacking] for `handle` without a full attack flow.
fn fire_when_attacking(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

/// Drive pending prompts until one of kind `want` is on top, resolving
/// TriggerOrder prompts (pick the first entry) and DECLINING any optional
/// Replacement accept-prompts that belong to OTHER clauses on the way.
/// Panics if `want` never surfaces.
fn drive_to_kind(runner: &mut DebugRunner, player: u8, want: SelectionKind) {
    for _ in 0..8 {
        let Some(view) = runner.pending_selection_view() else {
            panic!("no pending selection while seeking {want:?}");
        };
        if view.kind == want {
            return;
        }
        match view.kind {
            SelectionKind::TriggerOrder => {
                runner
                    .execute_action(player, view.valid_action_ids[0])
                    .expect("resolve trigger-order prompt");
            }
            SelectionKind::Replacement => {
                runner
                    .execute_action(player, PASS)
                    .expect("decline foreign optional-trigger prompt");
            }
            other => panic!("unexpected selection kind {other:?} while seeking {want:?}"),
        }
    }
    panic!("selection of kind {want:?} never surfaced");
}

/// Drain any remaining prompts WITHOUT activating optional clauses: resolve
/// TriggerOrder picks and DECLINE optional Replacement accept-prompts. Used
/// after a branch-specific pick so the [WD][WA] by-cost clause is not
/// auto-accepted (a blanket `auto_resolve` would accept it and feed a board
/// Digimon into security).
fn decline_rest(runner: &mut DebugRunner, player: u8) {
    for _ in 0..8 {
        let Some(view) = runner.pending_selection_view() else {
            return;
        };
        match view.kind {
            SelectionKind::TriggerOrder => {
                runner
                    .execute_action(player, view.valid_action_ids[0])
                    .expect("resolve trigger-order prompt");
            }
            SelectionKind::Replacement => {
                runner
                    .execute_action(player, PASS)
                    .expect("decline optional trigger");
            }
            other => panic!("unexpected selection kind {other:?} while declining the rest"),
        }
    }
    panic!("prompts never drained");
}

/// Re-resolve a permanent's handle by top-card id (battle-area indices shift
/// when a lower-indexed permanent leaves the field).
fn find_permanent(runner: &DebugRunner, player: u8, card_id: &str) -> PermanentHandle {
    let index = runner.game.players[player as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not on player {player}'s battle area"));
    PermanentHandle {
        player,
        index: index as u8,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

/// EX8-028 compiles with its printed stats: Lv.6 blue/yellow Digimon, cost 12,
/// DP 12000, traits Ice-Snow + LIBERATOR, the standard Lv.5-blue cost-4 evo
/// path, and the printed "Digivolve Lv.5 w/[Ice-Snow] trait: Cost 3" alt path
/// (DCGO gates on EqualsTraits("Ice-Snow") && Level == 5).
#[test]
fn ex8_028_compiles_with_printed_stats_and_digivolve_paths() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .start();

    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX8-028 in compiled cards");

    assert_eq!(compiled.card, "EX8-028");
    assert_eq!(compiled.name, "Skadimon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(6));
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Blue, CompiledColor::Yellow],
        "cards.json card_colors [1, 2] = blue + yellow (multi-color)"
    );
    assert_eq!(compiled.cost, Some(12));
    assert_eq!(compiled.dp, Some(12000));
    for trait_name in ["Ice-Snow", "LIBERATOR"] {
        assert!(
            compiled.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }

    // Standard printed evo box: Lv.5 blue for 4 memory.
    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(4))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(5)
                        || from.all_of.iter().any(|pred| pred.level_eq == Some(5)))
                        && (from.color_is == Some(CompiledColor::Blue)
                            || from
                                .all_of
                                .iter()
                                .any(|pred| pred.color_is == Some(CompiledColor::Blue)))
                })
        }),
        "Skadimon must digivolve from a blue Lv.5 for cost 4 (printed evo box)"
    );

    // Printed alt evo box: Lv.5 with [Ice-Snow] trait for 3 memory.
    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(3))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(5)
                        || from.all_of.iter().any(|pred| pred.level_eq == Some(5)))
                        && (from.trait_has.as_deref() == Some("Ice-Snow")
                            || from
                                .all_of
                                .iter()
                                .any(|pred| pred.trait_has.as_deref() == Some("Ice-Snow")))
                })
        }),
        "Skadimon must digivolve from a Lv.5 [Ice-Snow] Digimon for cost 3 (printed alt evo box)"
    );
}

/// The printed <Iceclad> and <Barrier> keywords are encoded as face-up
/// grant_keyword clauses AND are live on the placed permanent through the
/// canonical `Game::has_keyword` query.
#[test]
fn ex8_028_iceclad_and_barrier_keywords_attached() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .start();

    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX8-028 in compiled cards");
    for keyword in ["Iceclad", "Barrier"] {
        assert!(
            compiled.effects.iter().any(|clause| {
                matches!(
                    clause,
                    CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                        keyword: k,
                        scope,
                        ..
                    }) if k == keyword && *scope != CompiledScope::Inherited
                )
            }),
            "EX8-028 must encode <{keyword}> as a face-up grant_keyword clause"
        );
    }

    let skadimon = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(
        runner.game.has_keyword(skadimon, Keyword::Iceclad),
        "the placed Skadimon must answer has_keyword(Iceclad)"
    );
    assert!(
        runner.game.has_keyword(skadimon, Keyword::Barrier),
        "the placed Skadimon must answer has_keyword(Barrier)"
    );
}

/// The [When Digivolving] free-play clause is face-up, fires only at
/// WhenDigivolving, is optional ("You may play"), and is NOT once-per-turn.
#[test]
fn ex8_028_wd_free_play_clause_structural_shape() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX8-028 in compiled cards");

    let triggered = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::WhenDigivolving] => {
                Some(t)
            }
            _ => None,
        })
        .expect("EX8-028 must encode a [When Digivolving]-only clause (the free play)");

    assert_eq!(triggered.scope, CompiledScope::FaceUp);
    assert!(triggered.optional, "printed 'You may play 1 …'");
    assert!(
        !triggered.once_per_turn,
        "the free-play clause has no printed [Once Per Turn] (DCGO maxCount -1)"
    );
}

/// The pay-unsuspend clause fires at BOTH WhenDigivolving and WhenAttacking,
/// is optional (by-cost idiom), and IS once-per-turn — one clause, two
/// timings, sharing one OPT counter (DCGO shared hash "Unsuspend_EX8_028").
#[test]
fn ex8_028_pay_unsuspend_clause_structural_shape() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX8-028 in compiled cards");

    let triggered = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("EX8-028 must encode ONE clause at [When Digivolving][When Attacking]");

    assert_eq!(triggered.scope, CompiledScope::FaceUp);
    assert!(
        triggered.optional,
        "'By placing … unsuspends' is the by-cost optional idiom (DCGO isOptional true)"
    );
    assert!(
        triggered.once_per_turn,
        "printed [Once Per Turn]; one clause at both timings = DCGO's shared hash"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — <Iceclad> battle semantics
// ─────────────────────────────────────────────────────────────────────────────

/// Iceclad attacker wins on source count despite LOWER stack DP impact:
/// Skadimon (12000 DP) with 1 source (2 cards) beats a 15000-DP defender with
/// 1 card. Without Iceclad the DP compare (12000 < 15000) would delete
/// Skadimon.
#[test]
fn ex8_028_iceclad_wins_battle_on_source_count_despite_lower_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .add_card(source_filler("SRC-A"))
        .add_card(board_digimon("BIG-DEF", 15000))
        .start();

    let skadimon = runner.place_stack(0, &["SRC-A", CARD_ID]);
    let defender = runner.place_on_field(1, "BIG-DEF", Some(0));

    // The sourceless defender makes Skadimon's own [When Attacking] by-cost
    // clause prompt mid-attack — decline it so the plain battle resolves.
    let result = runner.attack_digimon(skadimon, defender, false);
    if result == AttackResult::InProgress {
        drive_to_kind(&mut runner, 0, SelectionKind::Replacement);
        runner
            .execute_action(0, PASS)
            .expect("decline the pay-unsuspend clause");
        runner.auto_resolve().expect("finish the attack");
    }

    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "Iceclad compares stack sizes (2 > 1), not DP (12000 < 15000) — Skadimon survives"
    );
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        0,
        "the higher-DP single-card defender is deleted under Iceclad"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — <Barrier> battle-deletion replacement
// ─────────────────────────────────────────────────────────────────────────────

/// ACCEPT: when Skadimon would be deleted in battle, accepting Barrier
/// trashes the controller's top security card and prevents the deletion.
#[test]
fn ex8_028_barrier_accept_trashes_top_security_and_prevents_battle_deletion() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .add_card(make_test_card("SEC", "SEC"))
        .security(0, &["SEC", "SEC"])
        .start();
    let skadimon = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .delete_permanent_with_cause(skadimon, ReplacementCause::Battle);

    let view = runner
        .pending_selection_view()
        .expect("Barrier accept prompt must install for a battle deletion");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(runner.pending_is_optional(), "Barrier may be declined");
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept Barrier");
    runner.auto_resolve().expect("finish Barrier");

    assert_eq!(
        runner.battle_area_size(0),
        1,
        "Barrier prevents Skadimon's battle deletion"
    );
    assert_eq!(
        runner.game.players[0].security.len(),
        1,
        "Barrier's cost trashes the top security card"
    );
}

/// DECLINE: refusing Barrier lets the battle deletion proceed and pays no
/// security cost.
#[test]
fn ex8_028_barrier_declined_battle_deletion_proceeds() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .add_card(make_test_card("SEC", "SEC"))
        .security(0, &["SEC", "SEC"])
        .start();
    let skadimon = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .delete_permanent_with_cause(skadimon, ReplacementCause::Battle);

    let view = runner
        .pending_selection_view()
        .expect("Barrier accept prompt must install for a battle deletion");
    assert_eq!(view.kind, SelectionKind::Replacement);
    runner.execute_action(0, PASS).expect("decline Barrier");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        0,
        "declined Barrier: the battle deletion proceeds"
    );
    assert_eq!(
        runner.game.players[0].security.len(),
        2,
        "declined Barrier: no security card is trashed"
    );
}

/// Barrier only covers BATTLE deletion — an effect deletion offers no prompt
/// and resolves normally.
#[test]
fn ex8_028_barrier_ignores_effect_deletion() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .add_card(make_test_card("SEC", "SEC"))
        .security(0, &["SEC", "SEC"])
        .start();
    let skadimon = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .delete_permanent_with_cause(skadimon, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "Barrier must not fire for non-battle deletion"
    );
    assert_eq!(runner.battle_area_size(0), 0, "effect deletion proceeds");
    assert_eq!(
        runner.game.players[0].security.len(),
        2,
        "no Barrier cost is paid"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — [When Digivolving] free play with the dynamic level cap
// ─────────────────────────────────────────────────────────────────────────────

/// CAP = 4 with ZERO sourceless opponent Digimon: the opponent has one SOURCED
/// Digimon and one sourceless TAMER (neither raises the cap — the formula
/// counts sourceless opponent DIGIMON only). Hand: Ice-Snow Lv.4, Ice-Snow
/// Lv.5, non-Ice-Snow Lv.4. Only the Ice-Snow Lv.4 is a legal candidate
/// (Lv.5 over the cap; non-Ice-Snow trait-rejected); picking it plays it
/// without paying its cost.
#[test]
fn ex8_028_wd_free_play_cap_is_4_with_no_sourceless_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .add_card(ice_snow_digimon("ICE-4", 4))
        .add_card(ice_snow_digimon("ICE-5", 5))
        .add_card(plain_digimon("PLAIN-4", 4))
        .add_card(source_filler("SRC-A"))
        .add_card(board_digimon("OPP-SOURCED", 5000))
        .add_card(opp_tamer("OPP-TAMER"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    runner.place_stack(1, &["SRC-A", "OPP-SOURCED"]);
    runner.place_on_field(1, "OPP-TAMER", Some(0));

    let skadimon = runner.place_stack(0, &["SRC-A", CARD_ID]);
    put_in_hand(&mut runner, 0, "ICE-4");
    put_in_hand(&mut runner, 0, "ICE-5");
    put_in_hand(&mut runner, 0, "PLAIN-4");

    let memory_before = runner.game.memory;
    let field_before = runner.battle_area_size(0);
    fire_when_digivolving(&mut runner, skadimon);

    drive_to_kind(&mut runner, 0, SelectionKind::Hand);
    let view = runner.pending_selection_view().unwrap();
    assert!(
        runner.pending_is_optional(),
        "printed 'you may' — PASS must be legal"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "cap 4 + trait gate: only the Ice-Snow Lv.4 may be played \
         (Lv.5 exceeds the cap; the non-Ice-Snow Lv.4 is trait-rejected)"
    );

    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("play the Ice-Snow Lv.4 free");
    decline_rest(&mut runner, 0);

    assert_eq!(
        runner.battle_area_size(0),
        field_before + 1,
        "the Ice-Snow Lv.4 was played to the battle area"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "ICE-4"),
        "the played permanent is the Ice-Snow Lv.4"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "the play is free — no memory paid"
    );
}

/// CAP = 5 with EXACTLY ONE sourceless opponent Digimon — full integration:
/// Skadimon digivolves over a red Lv.5 [Ice-Snow] base via the printed alt
/// evo box (cost 3), and the [When Digivolving] hand pick offers the Ice-Snow
/// Lv.5 but NOT the Ice-Snow Lv.6 (cap 4+1 = 5).
#[test]
fn ex8_028_wd_free_play_cap_scales_to_5_with_one_sourceless_opponent() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .add_card(red_ice_snow_lv5("ICEBASE"))
        .add_card(ice_snow_digimon("ICE-5", 5))
        .add_card(ice_snow_digimon("ICE-6", 6))
        .add_card(board_digimon("OPP-BARE", 5000))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(8)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(1, "OPP-BARE", Some(0));
    let base = runner.place_on_field(0, "ICEBASE", Some(0));
    let hand_idx = put_in_hand(&mut runner, 0, CARD_ID);
    put_in_hand(&mut runner, 0, "ICE-5");
    put_in_hand(&mut runner, 0, "ICE-6");

    let memory_before = runner.game.memory;
    assert!(
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByHand),
        "Skadimon must digivolve over the red Lv.5 [Ice-Snow] base via the \
         printed alt evo box"
    );
    assert_eq!(
        runner.game.memory,
        memory_before - 3,
        "the alt path charges its printed cost 3 (the Lv.5-blue path does \
         not match a red base)"
    );

    runner.game.drain_effect_queue();
    drive_to_kind(&mut runner, 0, SelectionKind::Hand);
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "cap 4+1 = 5: the Ice-Snow Lv.5 is legal, the Lv.6 is not"
    );

    let memory_at_pick = runner.game.memory;
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("play the Ice-Snow Lv.5 free");
    decline_rest(&mut runner, 0);

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "ICE-5"),
        "the Ice-Snow Lv.5 was played"
    );
    assert_eq!(
        runner.game.memory, memory_at_pick,
        "the play is free — no memory paid"
    );
}

/// CAP = 6 with TWO sourceless opponent Digimon: the Ice-Snow Lv.6 becomes a
/// legal candidate (cap 4+2 = 6).
#[test]
fn ex8_028_wd_free_play_cap_scales_to_6_with_two_sourceless_opponents() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .add_card(ice_snow_digimon("ICE-6", 6))
        .add_card(board_digimon("OPP-A", 5000))
        .add_card(board_digimon("OPP-B", 5000))
        .add_card(source_filler("SRC-A"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(1, "OPP-A", Some(0));
    runner.place_on_field(1, "OPP-B", Some(0));

    let skadimon = runner.place_stack(0, &["SRC-A", CARD_ID]);
    put_in_hand(&mut runner, 0, "ICE-6");

    fire_when_digivolving(&mut runner, skadimon);
    drive_to_kind(&mut runner, 0, SelectionKind::Hand);
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "cap 4+2 = 6: the Ice-Snow Lv.6 is now a legal candidate"
    );

    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("play the Ice-Snow Lv.6 free");
    decline_rest(&mut runner, 0);

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "ICE-6"),
        "the Ice-Snow Lv.6 was played"
    );
}

/// DECLINE: the free play is optional — PASS at the hand pick plays nothing
/// and pays nothing.
#[test]
fn ex8_028_wd_free_play_decline_is_legal() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .add_card(ice_snow_digimon("ICE-4", 4))
        .add_card(source_filler("SRC-A"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let skadimon = runner.place_stack(0, &["SRC-A", CARD_ID]);
    put_in_hand(&mut runner, 0, "ICE-4");

    let field_before = runner.battle_area_size(0);
    let hand_before = runner.game.players[0].hand.len();
    fire_when_digivolving(&mut runner, skadimon);

    drive_to_kind(&mut runner, 0, SelectionKind::Hand);
    assert!(runner.pending_is_optional(), "PASS must be legal");
    runner
        .execute_action(0, PASS)
        .expect("decline the free play");
    decline_rest(&mut runner, 0);

    assert_eq!(runner.battle_area_size(0), field_before, "nothing played");
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "the hand is untouched"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — [WD][WA][Once Per Turn] pay-unsuspend
// ─────────────────────────────────────────────────────────────────────────────

/// TARGET SET: the pay selection offers sourceless Digimon on BOTH battle
/// areas (DCGO HasMatchConditionPermanent scans every player) and excludes
/// sourced Digimon. Skadimon itself carries a source here, so the two legal
/// targets are exactly the controller's bare ally and the opponent's bare
/// Digimon.
#[test]
fn ex8_028_pay_unsuspend_offers_sourceless_digimon_on_both_sides() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .add_card(board_digimon("ALLY-BARE", 4000))
        .add_card(board_digimon("OPP-BARE", 4000))
        .add_card(board_digimon("OPP-SOURCED", 4000))
        .add_card(source_filler("SRC-A"))
        .add_card(source_filler("SRC-B"))
        .start();
    runner.game.turn_count = 1;

    let ally = runner.place_on_field(0, "ALLY-BARE", Some(0));
    let opp_bare = runner.place_on_field(1, "OPP-BARE", Some(0));
    runner.place_stack(1, &["SRC-B", "OPP-SOURCED"]);
    let skadimon = runner.place_stack(0, &["SRC-A", CARD_ID]);

    fire_when_attacking(&mut runner, skadimon);

    drive_to_kind(&mut runner, 0, SelectionKind::Replacement);
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept the optional pay-unsuspend clause");

    let view = runner
        .pending_selection_view()
        .expect("the pay-target selection must install");
    // AnyField (NOT Target): the pick spans BOTH battle areas and encodes the
    // engine player in each id (encode_attack(player, index)). The UI's
    // board-click router recognizes AnyField and decodes the side; routing this
    // as `Target` left every target unclickable AND — the inner pick being
    // mandatory — uncancellable: the EX8-028 "place 1 Digimon as the bottom
    // security card" softlock. This assertion is the engine-side regression guard.
    assert_eq!(view.kind, SelectionKind::AnyField);
    assert!(
        !runner.pending_is_optional(),
        "once activated, the pick is mandatory (DCGO canNoSelect: false)"
    );
    let mut ids = view.valid_action_ids.clone();
    ids.sort_unstable();
    let mut expected = vec![
        encode_attack(u16::from(ally.player), u16::from(ally.index)),
        encode_attack(u16::from(opp_bare.player), u16::from(opp_bare.index)),
    ];
    expected.sort_unstable();
    assert_eq!(
        ids, expected,
        "exactly the two sourceless Digimon — one per side — are legal; \
         sourced Digimon (incl. Skadimon itself) are excluded"
    );
}

/// OWN-SIDE PAY (full attack integration): Skadimon attacks the opponent
/// player, suspending itself; paying the cost with the controller's bare ally
/// places that ally's card at the BOTTOM of the CONTROLLER's security stack
/// face down, and Skadimon unsuspends.
#[test]
fn ex8_028_pay_own_digimon_goes_to_own_security_bottom_and_unsuspends() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .add_card(board_digimon("ALLY-BARE", 4000))
        .add_card(source_filler("SRC-A"))
        .add_card(make_test_card("SEC", "SEC"))
        .security(0, &["SEC"])
        .security(1, &["SEC"])
        .start();
    runner.game.turn_count = 2;

    let ally = runner.place_on_field(0, "ALLY-BARE", Some(0));
    let skadimon = runner.place_stack(0, &["SRC-A", CARD_ID]);

    let ally_card_index = runner.game.players[0].battle_area[ally.index as usize]
        .top_card()
        .card_index;
    let own_security_before = runner.game.players[0].security.len();
    let field_before = runner.battle_area_size(0);

    let _ = runner.attack_player(skadimon, 1, false);

    drive_to_kind(&mut runner, 0, SelectionKind::Replacement);
    assert!(
        runner.game.players[0].battle_area[skadimon.index as usize].is_suspended,
        "the attack declaration suspended Skadimon"
    );
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept the pay-unsuspend clause");

    drive_to_kind(&mut runner, 0, SelectionKind::AnyField);
    let view = runner.pending_selection_view().unwrap();
    let ally_action = encode_attack(u16::from(ally.player), u16::from(ally.index));
    assert!(view.valid_action_ids.contains(&ally_action));
    runner
        .execute_action(0, ally_action)
        .expect("place the bare ally as the bottom security card");
    runner.auto_resolve().expect("finish the attack flow");

    assert_eq!(
        runner.battle_area_size(0),
        field_before - 1,
        "the paid ally left the battle area"
    );
    assert_eq!(
        runner.game.players[0].security.len(),
        own_security_before + 1,
        "the ally's card joins the CONTROLLER's security stack (its owner)"
    );
    assert_eq!(
        runner.game.players[0].security[0].card_index, ally_card_index,
        "the placed card sits at the BOTTOM of the stack"
    );
    assert!(
        !runner.game.players[0]
            .face_up_security
            .contains(&ally_card_index),
        "the placed security card is face down"
    );
    // Skadimon survived the security check and is unsuspended by the effect.
    let skadimon_perm = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("Skadimon survives the security check");
    assert!(
        !skadimon_perm.is_suspended,
        "this Digimon unsuspends after paying the cost"
    );
}

/// NO-WEDGE regression (user report: "EX8-028 breaks the game due to the
/// when attacking effect"; resolved in 0.4.0 — the pay pick was routed with
/// the wrong SelectionKind, leaving a mandatory selection unanswerable).
/// Drives the FULL attack with the [When Attacking] effect resolving through
/// real selections and asserts the attack completes, the security check on
/// the defending player still happens, no selection is left dangling, and
/// the turn can end normally afterwards.
#[test]
fn ex8_028_when_attacking_effect_resolves_and_attack_completes_without_wedge() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .add_card(board_digimon("ALLY-BARE", 4000))
        .add_card(source_filler("SRC-A"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(0, &["FILL"])
        .security(1, &["FILL", "FILL"])
        .start();
    runner.game.turn_count = 2;

    let ally = runner.place_on_field(0, "ALLY-BARE", Some(0));
    let skadimon = runner.place_stack(0, &["SRC-A", CARD_ID]);
    let opp_security_before = runner.game.players[1].security.len();

    let _ = runner.attack_player(skadimon, 1, false);

    // [When Attacking] by-cost clause: accept, then pay with the bare ally.
    drive_to_kind(&mut runner, 0, SelectionKind::Replacement);
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept the pay-unsuspend clause");
    drive_to_kind(&mut runner, 0, SelectionKind::AnyField);
    let pay_action = encode_attack(u16::from(ally.player), u16::from(ally.index));
    runner
        .execute_action(0, pay_action)
        .expect("the pay target must be selectable (0.4.0 softlock regression)");
    runner
        .auto_resolve()
        .expect("the attack must resolve to completion");

    // The attack was NOT wedged: it ran through to the security check and
    // fully cleaned up.
    assert_eq!(
        runner.game.players[1].security.len(),
        opp_security_before - 1,
        "the player attack must continue into the security check after the \
         [When Attacking] effect resolves"
    );
    assert!(
        runner.game.pending_attack.is_none(),
        "no attack left in flight"
    );
    assert!(
        runner.game.pending_selection.is_none(),
        "no selection left dangling"
    );
    assert!(!runner.game_over(), "the game continues");

    // Skadimon unsuspended via the effect and the game can proceed normally.
    let skadimon_perm = &runner.game.players[0].battle_area
        [find_permanent(&runner, 0, CARD_ID).index as usize];
    assert!(
        !skadimon_perm.is_suspended,
        "Skadimon unsuspends after paying the cost"
    );
    runner.end_turn();
    assert_eq!(
        runner.game.turn_player(),
        1,
        "the turn passes normally after the attack"
    );
}

/// OPPONENT-SIDE PAY: choosing the OPPONENT's bare Digimon places its card at
/// the bottom of the OPPONENT's security stack (DCGO routes the card to
/// topCard.Owner's stack), not the controller's.
#[test]
fn ex8_028_pay_opponent_digimon_goes_to_opponent_security_bottom() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .add_card(board_digimon("OPP-BARE", 4000))
        .add_card(source_filler("SRC-A"))
        .add_card(make_test_card("SEC", "SEC"))
        .security(0, &["SEC"])
        .security(1, &["SEC"])
        .start();
    runner.game.turn_count = 1;

    let opp_bare = runner.place_on_field(1, "OPP-BARE", Some(0));
    let skadimon = runner.place_stack(0, &["SRC-A", CARD_ID]);
    runner.game.players[0].battle_area[skadimon.index as usize].is_suspended = true;

    let opp_card_index = runner.game.players[1].battle_area[opp_bare.index as usize]
        .top_card()
        .card_index;
    let own_security_before = runner.game.players[0].security.len();
    let opp_security_before = runner.game.players[1].security.len();

    fire_when_attacking(&mut runner, skadimon);
    drive_to_kind(&mut runner, 0, SelectionKind::Replacement);
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept the pay-unsuspend clause");

    drive_to_kind(&mut runner, 0, SelectionKind::AnyField);
    let opp_action = encode_attack(u16::from(opp_bare.player), u16::from(opp_bare.index));
    runner
        .execute_action(0, opp_action)
        .expect("place the opponent's bare Digimon as a bottom security card");
    runner.auto_resolve().expect("finish the effect");

    assert_eq!(
        runner.game.players[1].battle_area.len(),
        0,
        "the opponent's Digimon left the battle area"
    );
    assert_eq!(
        runner.game.players[1].security.len(),
        opp_security_before + 1,
        "the card joins the OPPONENT's security stack (its owner's), not the controller's"
    );
    assert_eq!(
        runner.game.players[0].security.len(),
        own_security_before,
        "the controller's security stack is untouched"
    );
    assert_eq!(
        runner.game.players[1].security[0].card_index, opp_card_index,
        "the placed card sits at the BOTTOM of the opponent's stack"
    );
    assert!(
        !runner.game.players[0].battle_area[skadimon.index as usize].is_suspended,
        "Skadimon unsuspends after paying the cost"
    );
}

/// SELF-PAY: a sourceless Skadimon is itself a legal target (DCGO's filter is
/// just "battle-area Digimon with no digivolution cards"). Placing itself
/// moves it to its owner's security bottom; the unsuspend tail then no-ops on
/// the departed permanent without panicking.
#[test]
fn ex8_028_pay_can_place_self_when_sourceless() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .add_card(make_test_card("SEC", "SEC"))
        .security(0, &["SEC"])
        .start();
    runner.game.turn_count = 1;

    let skadimon = runner.place_on_field(0, CARD_ID, Some(0));
    let own_security_before = runner.game.players[0].security.len();

    fire_when_attacking(&mut runner, skadimon);
    drive_to_kind(&mut runner, 0, SelectionKind::Replacement);
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept the pay-unsuspend clause");

    drive_to_kind(&mut runner, 0, SelectionKind::AnyField);
    let view = runner.pending_selection_view().unwrap();
    let self_action = encode_attack(u16::from(skadimon.player), u16::from(skadimon.index));
    assert_eq!(
        view.valid_action_ids,
        vec![self_action],
        "the sourceless Skadimon is itself the only legal target"
    );
    runner
        .execute_action(0, self_action)
        .expect("Skadimon pays with itself");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        0,
        "Skadimon left the battle area"
    );
    assert_eq!(
        runner.game.players[0].security.len(),
        own_security_before + 1,
        "Skadimon's card joins its owner's security stack"
    );
}

/// OPT IS SHARED ACROSS TIMINGS: using the clause at [When Digivolving] locks
/// it for [When Attacking] in the same turn (DCGO shared hash
/// "Unsuspend_EX8_028"); the lock clears on the next own turn.
#[test]
fn ex8_028_pay_unsuspend_opt_shared_across_wd_and_wa() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .add_card(board_digimon("ALLY-A", 4000))
        .add_card(board_digimon("ALLY-B", 4000))
        .add_card(source_filler("SRC-A"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .start();
    runner.game.turn_count = 1;

    // Skadimon FIRST (index 0) so its handle survives an ally leaving the
    // field. Two bare allies: paying one leaves the other eligible, so a
    // missing second prompt proves the OPT lock, not an empty target set.
    let skadimon = runner.place_stack(0, &["SRC-A", CARD_ID]);
    let ally_a = runner.place_on_field(0, "ALLY-A", Some(0));
    runner.place_on_field(0, "ALLY-B", Some(0));

    // Use the clause at WD.
    fire_when_digivolving(&mut runner, skadimon);
    drive_to_kind(&mut runner, 0, SelectionKind::Replacement);
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept at [When Digivolving]");
    drive_to_kind(&mut runner, 0, SelectionKind::AnyField);
    let pay_action = encode_attack(u16::from(ally_a.player), u16::from(ally_a.index));
    runner
        .execute_action(0, pay_action)
        .expect("pay with ALLY-A");
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.game.players[0].security.len(),
        1,
        "the WD activation placed ALLY-A into security"
    );

    // Same turn, [When Attacking]: the shared OPT must suppress the prompt.
    // Re-resolve Skadimon's handle — an ally left the field above.
    let skadimon = find_permanent(&runner, 0, CARD_ID);
    fire_when_attacking(&mut runner, skadimon);
    assert!(
        runner.pending_selection_view().is_none(),
        "[Once Per Turn] is shared across WD and WA — no second prompt this turn"
    );

    // Next own turn: the OPT clears and the clause offers again.
    runner.end_turn();
    runner.end_turn();
    fire_when_attacking(&mut runner, skadimon);
    drive_to_kind(&mut runner, 0, SelectionKind::Replacement);
    assert!(
        runner.pending_selection_view().is_some(),
        "the OPT lock clears on the next turn"
    );
}

/// DECLINING DOES NOT CONSUME THE OPT: passing on the outer accept prompt
/// changes nothing, and the clause offers again later in the same turn (DCGO
/// decrements its counter only on activation).
#[test]
fn ex8_028_pay_unsuspend_decline_changes_nothing_and_keeps_opt() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .add_card(board_digimon("ALLY-BARE", 4000))
        .add_card(source_filler("SRC-A"))
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, "ALLY-BARE", Some(0));
    let skadimon = runner.place_stack(0, &["SRC-A", CARD_ID]);
    runner.game.players[0].battle_area[skadimon.index as usize].is_suspended = true;

    fire_when_attacking(&mut runner, skadimon);
    drive_to_kind(&mut runner, 0, SelectionKind::Replacement);
    runner.execute_action(0, PASS).expect("decline the clause");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        2,
        "decline: nothing left the battle area"
    );
    assert_eq!(
        runner.game.players[0].security.len(),
        0,
        "decline: nothing was placed into security"
    );
    assert!(
        runner.game.players[0].battle_area[skadimon.index as usize].is_suspended,
        "decline: Skadimon stays suspended"
    );

    // Same turn, second [When Attacking]: the prompt must reappear.
    fire_when_attacking(&mut runner, skadimon);
    drive_to_kind(&mut runner, 0, SelectionKind::Replacement);
    assert!(
        runner.pending_selection_view().is_some(),
        "declining must not consume the [Once Per Turn]"
    );
}

/// NO ELIGIBLE DIGIMON → NO PROMPT: with every battle-area Digimon carrying
/// digivolution cards, the optional clause must not even offer its accept
/// prompt (DCGO CanActivateCondition gate).
#[test]
fn ex8_028_pay_unsuspend_no_prompt_without_eligible_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-028 found in embedded DSL pack")
        .add_card(board_digimon("OPP-SOURCED", 4000))
        .add_card(source_filler("SRC-A"))
        .add_card(source_filler("SRC-B"))
        .start();
    runner.game.turn_count = 1;

    runner.place_stack(1, &["SRC-B", "OPP-SOURCED"]);
    let skadimon = runner.place_stack(0, &["SRC-A", CARD_ID]);

    fire_when_attacking(&mut runner, skadimon);

    assert!(
        runner.pending_selection_view().is_none(),
        "every Digimon has digivolution cards — the by-cost clause must not prompt"
    );
}
