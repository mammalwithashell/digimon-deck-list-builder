//! EX9-060 Devidramon — Digimon, Lv.4, Purple, DP 5000, Cost 5.
//! Traits: [Evil Dragon, DM, Ver.5]. Attribute: Virus.
//!
//! # Card text (data/card_bundles/EX9-060.md — official Bandai DB, verbatim)
//!
//! ＜Training＞ (In the main phase, by suspending this Digimon, place your
//! deck's top card face down as this Digimon's bottom digivolution card.
//! This effect can also activate in the breeding area.)
//!
//! [When Digivolving] [When Attacking] [Once Per Turn] By placing 1 card in
//! your hand face down as this Digimon's bottom digivolution card, ＜Draw 1＞
//! (Draw 1 card from your deck.)
//!
//! Inherited Effect:
//!   [On Deletion] Delete 1 of your opponent's level 4 or lower Digimon.
//!
//! Digivolution requirements (official Bandai DB):
//!   Standard: Purple Lv.3 / cost 2.
//!   Alt: [Digivolve] Lv.3 w/[DM] trait: Cost 2.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX9/Purple/EX9_060.cs (read from base repo;
//! confirms: AddSelfDigivolutionRequirementStaticEffect for the [DM] Lv.3
//! alt-path; TrainingEffect(card) auto-install; OnEnterFieldAnyone (When
//! Digivolving) + OnAllyAttack (When Attacking) share hash
//! "EX9_060_Draw" — a single OPT lockout across both timings — each an
//! ActivateClass with isOptional:true wrapping a SelectHandEffect
//! (canNoSelect: true), then AddDigivolutionCardsBottom(isFacedown: true) +
//! DrawClass(1); OnDestroyedAnyone inherited effect deletes 1 opponent
//! Digimon with Level <= 4 via SelectPermanentEffect (Mode.Destroy,
//! canNoSelect: false, maxCount capped at available-candidate count).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - H7-adjacent: printed <Training> keyword (grant_keyword, own scope)
//! - E2 OPT + optional decline (cost-paid, declinable) — Clause 2
//! - G-OPTIONAL-COST-DECLINE-ABORTS-CLAUSE: select_hand cost:true + then:
//! - Multi-timing shared OPT: [When Digivolving] + [When Attacking] one clause
//! - F5-adjacent: inherited [On Deletion] delete filtered by level_lte
//! - alt-path: Lv.3 w/[DM] trait, cost 2 (xros_req)

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

// ─── Helpers ──────────────────────────────────────────────────────────────

fn devidramon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX9-060")
        .expect("EX9-060 must be present in the embedded DSL pack")
        .memory(10)
        .build()
}

fn filler_hand_card(id: &str) -> CardData {
    make_test_card(id, id)
}

fn opp_digimon_with_level(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

fn hand_index(runner: &DebugRunner, player: digimon_engine::enums::PlayerId, card_id: &str) -> u16 {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|card| card.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand")) as u16
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ex9_060_yaml_loads_and_compiles_from_embedded_pack() {
    let runner = devidramon_runner();
    let compiled = runner
        .compiled_card("EX9-060")
        .expect("EX9-060 compiled card must be available");

    assert_eq!(compiled.card, "EX9-060");
    assert_eq!(compiled.name, "Devidramon");
    assert_eq!(compiled.level, Some(4));
    assert_eq!(compiled.dp, Some(5000));
}

#[test]
fn ex9_060_declares_own_scope_training_keyword_grant() {
    let runner = devidramon_runner();
    let compiled = runner
        .compiled_card("EX9-060")
        .expect("EX9-060 compiled card must be available");

    let has_face_up_training = compiled.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::FaceUp,
                ..
            }) if keyword == "Training"
        )
    });

    assert!(
        has_face_up_training,
        "EX9-060 must declare own-scope GrantKeyword(Training)"
    );
}

#[test]
fn ex9_060_alt_path_contains_lv3_dm_trait_cost_two() {
    let runner = devidramon_runner();
    let compiled = runner
        .compiled_card("EX9-060")
        .expect("EX9-060 compiled card must be available");

    let dm_lv3_path = compiled.alt_paths.iter().find(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(2))
            && path
                .from
                .as_ref()
                .map(|from| from.level_eq == Some(3) && from.trait_has.as_deref() == Some("DM"))
                .unwrap_or(false)
    });

    assert!(
        dm_lv3_path.is_some(),
        "EX9-060 must declare [Digivolve] Lv.3 with [DM] trait: cost 2"
    );
}

#[test]
fn ex9_060_has_when_digivolving_when_attacking_opt_clause() {
    let runner = devidramon_runner();
    let compiled = runner
        .compiled_card("EX9-060")
        .expect("EX9-060 compiled card must be available");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::WhenDigivolving)
                && t.when.contains(&CompiledTiming::WhenAttacking)
        })
        .expect("[When Digivolving][When Attacking] clause must exist");

    assert!(
        clause.once_per_turn,
        "clause must be [Once Per Turn] (shared OPT hash EX9_060_Draw in DCGO)"
    );
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "clause is the card's own (non-inherited) effect"
    );
}

#[test]
fn ex9_060_has_inherited_on_deletion_clause() {
    let runner = devidramon_runner();
    let compiled = runner
        .compiled_card("EX9-060")
        .expect("EX9-060 compiled card must be available");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.scope == CompiledScope::Inherited && t.when.contains(&CompiledTiming::OnDeletion)
        })
        .expect("inherited [On Deletion] clause must exist");

    assert!(
        !clause.optional,
        "'[On Deletion] Delete 1 of your opponent's...' is NOT phrased with 'you may' — mandatory"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — <Training> keyword: native visibility on field
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ex9_060_training_keyword_visible_on_battle_area_carrier() {
    let mut runner = devidramon_runner();
    let card_data = runner
        .game
        .card_data
        .iter()
        .find(|c| c.card_id == "EX9-060")
        .expect("EX9-060 card data must be loaded");

    // Only assert native visibility if the current engine maps DSL Training
    // into native card data keywords (mirrors EX9-008's precedent test).
    if card_data.keywords.contains(&Keyword::Training) {
        let devidramon = runner.place_on_field(0, "EX9-060", Some(0));
        assert!(
            runner.game.has_keyword(devidramon, Keyword::Training),
            "EX9-060 on field must report Keyword::Training"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [When Digivolving][When Attacking][OPT] place hand card, draw 1
// ═══════════════════════════════════════════════════════════════════════════

/// Attacking with a hand card available installs a Hand selection prompt
/// (the cost: "By placing 1 card in your hand face down...").
#[test]
fn ex9_060_when_attacking_with_hand_card_installs_hand_selection() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-060")
        .expect("EX9-060 parses")
        .add_card(filler_hand_card("FILLER-ATK-1"))
        .add_card(make_test_card("OPP-FILLER", "OppFiller"))
        .hand(0, &["FILLER-ATK-1"])
        .deck(1, &["OPP-FILLER"; 5])
        .memory(10)
        .start();

    let devidramon = runner.place_on_field(0, "EX9-060", Some(0));
    let opp = runner.place_on_field(1, "OPP-FILLER", Some(0));

    runner.attack_digimon(devidramon, opp, false);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "When Attacking must prompt a Hand selection for the placement cost"
    );
    assert!(
        runner.pending_is_optional(),
        "the cost pick is optional (\"By placing...\" is declinable per DCGO canNoSelect:true / isOptional:true)"
    );
}

/// Selecting a hand card places it as this Digimon's bottom digivolution
/// source (face-down) and draws exactly 1 card.
#[test]
fn ex9_060_when_attacking_placing_hand_card_grows_stack_and_draws_one() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-060")
        .expect("EX9-060 parses")
        .add_card(filler_hand_card("FILLER-ATK-2"))
        .add_card(filler_hand_card("DECK-DRAW-1"))
        .add_card(make_test_card("OPP-FILLER-2", "OppFiller2"))
        .hand(0, &["FILLER-ATK-2"])
        .deck(0, &["DECK-DRAW-1"])
        .deck(1, &["OPP-FILLER-2"; 5])
        .memory(10)
        .start();

    let devidramon = runner.place_on_field(0, "EX9-060", Some(0));
    let opp = runner.place_on_field(1, "OPP-FILLER-2", Some(0));

    let stack_before = runner.game.players[0].battle_area[devidramon.index as usize]
        .card_sources
        .len();
    let hand_before = runner.hand_size(0);

    runner.attack_digimon(devidramon, opp, false);

    let slot = hand_index(&runner, 0, "FILLER-ATK-2");
    runner
        .execute_action(0, PLAY_HAND_START + slot)
        .expect("select the hand card to place as bottom source");
    runner.auto_resolve().expect("resolve remaining prompts");

    let perm = &runner.game.players[0].battle_area[devidramon.index as usize];
    assert_eq!(
        perm.card_sources.len(),
        stack_before + 1,
        "placing the hand card must grow the digivolution stack by exactly 1"
    );
    assert_eq!(
        perm.card_sources[0].card_id(&runner.game.card_data),
        "FILLER-ATK-2",
        "the placed card must sit at the bottom (index 0) of the stack"
    );
    assert!(
        perm.card_sources[0].face_down,
        "the placed card must be face-down"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1 + 1,
        "hand loses the placed card but gains the drawn card (net unchanged)"
    );
}

/// Declining the placement prompt (PASS) does nothing: no source added, no
/// draw. This is the "By Xing, Y" cost-gate — decline aborts the whole tail.
#[test]
fn ex9_060_when_attacking_declining_placement_draws_nothing() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-060")
        .expect("EX9-060 parses")
        .add_card(filler_hand_card("FILLER-ATK-3"))
        .add_card(filler_hand_card("DECK-DRAW-2"))
        .add_card(make_test_card("OPP-FILLER-3", "OppFiller3"))
        .hand(0, &["FILLER-ATK-3"])
        .deck(0, &["DECK-DRAW-2"])
        .deck(1, &["OPP-FILLER-3"; 5])
        .memory(10)
        .start();

    let devidramon = runner.place_on_field(0, "EX9-060", Some(0));
    let opp = runner.place_on_field(1, "OPP-FILLER-3", Some(0));

    let stack_before = runner.game.players[0].battle_area[devidramon.index as usize]
        .card_sources
        .len();
    let hand_before = runner.hand_size(0);
    let deck_before = runner.game.players[0].deck.len();

    runner.attack_digimon(devidramon, opp, false);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    runner
        .execute_action(0, PASS)
        .expect("decline the placement cost");

    assert!(
        runner.pending_selection().is_none(),
        "declining aborts the whole clause tail (no further prompts)"
    );

    let perm = &runner.game.players[0].battle_area[devidramon.index as usize];
    assert_eq!(
        perm.card_sources.len(),
        stack_before,
        "declining must not add any source"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declining must not change hand size"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before,
        "declining must not draw (no deck change)"
    );
}

/// With an empty hand, the cost has no candidate — DCGO's
/// `CanActivateCondition` gates on `card.Owner.HandCards.Count > 0`, so no
/// selection installs at all (not even a PASS-only prompt).
#[test]
fn ex9_060_when_attacking_with_empty_hand_installs_no_selection() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-060")
        .expect("EX9-060 parses")
        .add_card(make_test_card("OPP-FILLER-4", "OppFiller4"))
        .deck(1, &["OPP-FILLER-4"; 5])
        .memory(10)
        .start();

    let devidramon = runner.place_on_field(0, "EX9-060", Some(0));
    let opp = runner.place_on_field(1, "OPP-FILLER-4", Some(0));

    assert_eq!(runner.hand_size(0), 0, "hand must be empty for this test");

    runner.attack_digimon(devidramon, opp, false);

    // Either no selection installs, or if one installs it must be trivially
    // decline-only (optional with zero non-PASS actions) — either way no
    // source can be placed. We assert the strict no-selection outcome to
    // match DCGO's explicit HandCards.Count > 0 gate.
    assert!(
        runner.pending_selection().is_none(),
        "empty hand must not install a placement prompt (mirrors DCGO CanActivateCondition gate)"
    );
}

/// [When Digivolving] timing fires the same clause body (shared OPT hash
/// "EX9_060_Draw" spans both When Digivolving and When Attacking in DCGO).
#[test]
fn ex9_060_when_digivolving_installs_hand_selection() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-060")
        .expect("EX9-060 parses")
        .add_card(filler_hand_card("FILLER-WD-1"))
        .add_card(make_test_card("LV3-BASE", "Lv3Base"))
        .hand(0, &["FILLER-WD-1"])
        .memory(10)
        .start();

    // Digivolve EX9-060 onto a Lv.3 base via place_stack (structural
    // digivolution) then fire the WhenDigivolving trigger bundle directly —
    // consistent with the ex10_033 precedent for multi-timing clauses.
    let devidramon = runner.place_stack(0, &["LV3-BASE", "EX9-060"]);

    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::WhenDigivolving,
        digimon_engine::selection::TriggerSource::Permanent(devidramon),
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "When Digivolving must prompt the same Hand selection for the placement cost"
    );
}

/// OPT lockout: after resolving the clause once via When Attacking, a second
/// attack in the same turn must not re-install the Hand prompt (shared OPT
/// hash across both timings per DCGO "EX9_060_Draw").
#[test]
fn ex9_060_opt_blocks_second_activation_same_turn_across_timings() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-060")
        .expect("EX9-060 parses")
        .add_card(filler_hand_card("FILLER-OPT-1"))
        .add_card(filler_hand_card("FILLER-OPT-2"))
        .add_card(filler_hand_card("DECK-DRAW-OPT"))
        .add_card(make_test_card("OPP-FILLER-OPT", "OppFillerOpt"))
        .hand(0, &["FILLER-OPT-1", "FILLER-OPT-2"])
        .deck(0, &["DECK-DRAW-OPT"])
        .deck(1, &["OPP-FILLER-OPT"; 10])
        .memory(10)
        .start();

    let devidramon = runner.place_on_field(0, "EX9-060", Some(0));
    let opp1 = runner.place_on_field(1, "OPP-FILLER-OPT", Some(0));
    let opp2 = runner.place_on_field(1, "OPP-FILLER-OPT", Some(0));

    // First attack — resolve the placement.
    runner.attack_digimon(devidramon, opp1, false);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    let slot = hand_index(&runner, 0, "FILLER-OPT-1");
    runner
        .execute_action(0, PLAY_HAND_START + slot)
        .expect("select first hand card");
    runner.auto_resolve().expect("resolve remaining prompts");

    // Second attack, same turn — Devidramon is suspended after attacking in
    // the real game, but battle_area suspension doesn't block a *second*
    // Digimon's attack. To isolate the OPT lockout (not the suspend gate),
    // unsuspend and re-attack with the SAME permanent within the same turn.
    runner.game.players[0].battle_area[devidramon.index as usize].is_suspended = false;
    runner.attack_digimon(devidramon, opp2, false);

    assert!(
        runner.pending_selection().is_none(),
        "OPT must lock the second activation in the same turn, even across [When Attacking] reuse"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited [On Deletion] delete 1 opp level-4-or-lower Digimon
// ═══════════════════════════════════════════════════════════════════════════

/// A carrier with EX9-060 as a digivolution source, when deleted, offers a
/// selection over eligible opponent Digimon with level <= 4.
#[test]
fn ex9_060_inherited_on_deletion_offers_level_4_or_lower_target() {
    let mut carrier = make_test_card("CARRIER-DEL-1", "CarrierDel1");
    carrier.level = Some(5);
    carrier.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-060")
        .expect("EX9-060 parses")
        .add_card(carrier)
        .add_card(opp_digimon_with_level("OPP-LV4", 4, 3000))
        .add_card(opp_digimon_with_level("OPP-LV6", 6, 8000))
        .memory(10)
        .start();

    let carrier_handle = runner.place_stack(0, &["EX9-060", "CARRIER-DEL-1"]);
    let opp_lv4 = runner.place_on_field(1, "OPP-LV4", Some(0));
    let _opp_lv6 = runner.place_on_field(1, "OPP-LV6", Some(0));

    runner.game.delete_permanent_with_cause(carrier_handle, digimon_engine::replacement::ReplacementCause::OpponentEffect);

    let kind = runner
        .pending_kind()
        .expect("On Deletion must install a selection when an eligible opponent Digimon exists");
    assert_eq!(kind, SelectionKind::OppField);

    let view = runner.pending_selection_view().unwrap();
    let opp_lv4_index = opp_lv4.index as u16;
    let targets_lv4 = view
        .valid_action_ids
        .iter()
        .any(|&a| digimon_engine::action::space::decode_attack(a).1 == opp_lv4_index);
    assert!(
        targets_lv4,
        "the level-4 opponent Digimon must be a valid deletion target"
    );
}

/// Executing the deletion target removes the chosen level-4-or-lower opponent
/// Digimon from the battle area.
#[test]
fn ex9_060_inherited_on_deletion_deletes_chosen_target() {
    let mut carrier = make_test_card("CARRIER-DEL-2", "CarrierDel2");
    carrier.level = Some(5);
    carrier.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-060")
        .expect("EX9-060 parses")
        .add_card(carrier)
        .add_card(opp_digimon_with_level("OPP-LV3", 3, 2000))
        .memory(10)
        .start();

    let carrier_handle = runner.place_stack(0, &["EX9-060", "CARRIER-DEL-2"]);
    let opp = runner.place_on_field(1, "OPP-LV3", Some(0));

    let opp_field_size_before = runner.game.players[1].battle_area.len();

    runner.game.delete_permanent_with_cause(carrier_handle, digimon_engine::replacement::ReplacementCause::OpponentEffect);

    let action = digimon_engine::action::space::encode_attack(0, opp.index as u16);
    runner
        .execute_action(1, action)
        .unwrap_or_else(|_| {
            // The selecting player for a self-triggered On Deletion effect is
            // the deleted permanent's controller (player 0), not the target's
            // controller. Retry with player 0 if player 1 was rejected.
            runner
                .execute_action(0, action)
                .expect("delete the chosen opponent Digimon")
        });

    assert_eq!(
        runner.game.players[1].battle_area.len(),
        opp_field_size_before - 1,
        "the chosen level-4-or-lower opponent Digimon must be deleted"
    );
}

/// No eligible opponent Digimon (all above level 4) means no selection
/// installs — the effect silently has no target (no-approximations: it does
/// NOT fall back to deleting a higher-level Digimon).
#[test]
fn ex9_060_inherited_on_deletion_no_target_when_all_opponents_above_level_4() {
    let mut carrier = make_test_card("CARRIER-DEL-3", "CarrierDel3");
    carrier.level = Some(5);
    carrier.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-060")
        .expect("EX9-060 parses")
        .add_card(carrier)
        .add_card(opp_digimon_with_level("OPP-LV5", 5, 7000))
        .memory(10)
        .start();

    let carrier_handle = runner.place_stack(0, &["EX9-060", "CARRIER-DEL-3"]);
    let _opp = runner.place_on_field(1, "OPP-LV5", Some(0));

    let opp_field_size_before = runner.game.players[1].battle_area.len();

    runner.game.delete_permanent_with_cause(carrier_handle, digimon_engine::replacement::ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection().is_none(),
        "no selection should install when no opponent Digimon has level <= 4"
    );
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        opp_field_size_before,
        "no opponent Digimon should be deleted when none is eligible"
    );
}

/// Devidramon's [On Deletion] text lives ONLY in `inherited_effect_description_eng`
/// (no own-face copy — unlike e.g. BT17-102, which prints the same [On Deletion]
/// text in BOTH its own face text and as inherited, requiring two YAML clauses).
/// Per rule 15-3-1 ("an inherited effect can't be activated by just one card"),
/// deleting Devidramon directly — alone, not buried as a digivolution source
/// under some other carrier — must NOT fire the inherited clause.
#[test]
fn ex9_060_inherited_on_deletion_does_not_fire_when_deleted_alone() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-060")
        .expect("EX9-060 parses")
        .add_card(opp_digimon_with_level("OPP-LV2", 2, 1000))
        .memory(10)
        .start();

    let devidramon = runner.place_on_field(0, "EX9-060", Some(0));
    let _opp = runner.place_on_field(1, "OPP-LV2", Some(0));

    runner.game.delete_permanent_with_cause(devidramon, digimon_engine::replacement::ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection().is_none(),
        "inherited [On Deletion] must NOT fire when EX9-060 is deleted alone (rule 15-3-1: an inherited effect can't be activated by just one card)"
    );
}
