//! EX7-010 Deputymon — Digimon, Lv.4, Red, DP 6000, Cost 6.
//! Traits: Mutant. Form: Champion. Attribute: Vaccine.
//!
//! # Card text (data/card_bundles/EX7-010.md — official Bandai DB, verbatim;
//! cross-checked against the card image EX7-010.webp)
//!
//! [When Digivolving] [When Attacking] [Once Per Turn] You may trash any 1
//! Option card from 1 Digimon's digivolution cards.
//! [Your Turn] This Digimon gains the [Three Musketeers] trait.
//!
//! Inherited Effect: [Your Turn] This Digimon gets +2000 DP.
//!
//! Official Q&A: "Yes, you can choose either your Digimon or an opponent's
//! Digimon."
//!
//! Digivolution requirements (official Bandai DB):
//!   - Standard circle: Red Lv.3 / cost 2
//!   - xros_req: "[Digivolve] Lv.3 w/[Three Musketeers] in text: Cost 2"
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Red/EX7_010.cs
//!
//! # DCGO crosscheck
//! - Alt-digivolve: `HasText("Three Musketeers") && Level == 3`, cost 2.
//! - Shared WD/WA body: TWO ActivateClass entries (OnEnterFieldAnyone /
//!   OnAllyAttack) sharing hash `TrashOption_EX7_010`, timesPerTurn 1,
//!   isOptional TRUE. Body: `SelectTrashDigivolutionCards(permanentCondition:
//!   ANY battle-area Digimon, cardCondition: IsOption &&
//!   !CanNotTrashFromDigivolutionCards, maxCount 1, canNoTrash: false,
//!   isFromOnly1Permanent: true)`.
//! - [Your Turn] trait grant: ChangeTraitsClass gated IsOwnerTurn &&
//!   IsExistOnBattleAreaDigimon && `PermanentOfThisCard().TopCard == card`
//!   (face-up only) → appends "Three Musketeers".
//! - ESS: ChangeSelfDPStaticEffect(+2000, isInherited) gated IsOwnerTurn.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - E2/E3: optional OPT trigger with a shared hash across two timings
//! - Source-stack pick on ANY Digimon (own or opponent) → trash
//! - D4: [Your Turn] self-aura trait grant (G-DSL-AURA-GRANT-TRAITS)
//! - D1: inherited [Your Turn] +2000 DP

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, ModifierType, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX7-010";

fn digimon(id: &str, level: u8, color: CardColor, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 4;
    c
}

fn option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::Red];
    c.level = None;
    c.dp = None;
    c.play_cost = 4;
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Green];
    c
}

fn zone_ids(cards: &[digimon_engine::card_source::CardSource], data: &[CardData]) -> Vec<String> {
    cards.iter().map(|c| c.card_id(data).to_string()).collect()
}

fn sources_of(runner: &DebugRunner, h: PermanentHandle) -> usize {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .len()
}

fn has_tm_trait(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].has_trait_for_rules(
        "Three Musketeers",
        &runner.game.card_data,
        &runner.game.modifiers,
        h,
    )
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX7-010 YAML loads from the embedded pack")
        .add_card(digimon("RED-LV3", 3, CardColor::Red, 3000))
        .add_card(digimon("BLK-LV3", 3, CardColor::Black, 3000))
        .add_card(digimon("OPP", 4, CardColor::Blue, 4000))
        .add_card(digimon("OPP-2", 4, CardColor::Blue, 4000))
        .add_card(digimon("OPP-BIG", 4, CardColor::Blue, 8000))
        .add_card(digimon("ALLY", 4, CardColor::Red, 4000))
        .add_card(option("OPT-A"))
        .add_card(option("OPT-B"))
        .add_card(digimon("DIGI-SRC", 3, CardColor::Red, 2000))
        .add_card(filler("FILL"))
        .add_card(digimon("CARRIER", 5, CardColor::Red, 7000))
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(10)
}

/// Accept the outer optional prompt if one is pending (the WA/WD clause is
/// "You may"), returning whether a prompt was present at all.
fn accept_if_optional(runner: &mut DebugRunner) -> bool {
    if runner.pending_kind() == Some(SelectionKind::Replacement) {
        runner.accept_optional_trigger().expect("accept optional trigger");
        return true;
    }
    runner.pending_selection().is_some()
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_010_structure_matches_printed_text() {
    let runner = base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1, "ONE shared WD/WA clause (DCGO shares the OPT hash)");
    let t = triggered[0];
    assert_eq!(t.when, vec![CompiledTiming::WhenDigivolving, CompiledTiming::WhenAttacking]);
    assert!(t.optional, "'You may trash'");
    assert!(t.once_per_turn, "[Once Per Turn]");

    let trait_aura = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, active_when, grant_traits, .. })
                if *scope == CompiledScope::FaceUp
                    && active_when.is_some()
                    && grant_traits == &vec!["Three Musketeers".to_string()]
        )
    });
    assert!(trait_aura, "[Your Turn] face-up trait-grant aura must be present");

    let dp_aura = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, active_when, dp_modifier, .. })
                if *scope == CompiledScope::Inherited && active_when.is_some() && *dp_modifier == Some(2000)
        )
    });
    assert!(dp_aura, "inherited [Your Turn] +2000 DP aura must be present");

    let paths = compiled
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .count();
    assert_eq!(paths, 2, "standard Red Lv.3/2 + special TM-text Lv.3/2");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2/3 — [When Attacking] trash an Option from ANY Digimon's sources
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_010_wa_trashes_option_from_opponent_digimon_sources() {
    let mut runner = base().start();
    let deputy = runner.place_on_field(0, CARD_ID, Some(0));
    // OPP-BIG (8000) beats Deputymon (6000) so the opponent's stack survives
    // the battle and its digivolution cards can be inspected afterwards.
    let opp = runner.place_stack(1, &["OPT-A", "DIGI-SRC", "OPP-BIG"]);
    let opp_trash_before = runner.trash_size(1);

    runner.attack_digimon(deputy, opp, false);
    assert!(accept_if_optional(&mut runner), "the optional WA trigger must surface");

    let view = runner.pending_selection_view().expect("host Digimon prompt");
    assert_eq!(view.kind, SelectionKind::AnyField, "any Digimon (own or opponent) may be chosen");
    assert_eq!(view.valid_action_ids.len(), 1, "only OPP carries an Option source");
    runner.execute_action(0, view.valid_action_ids[0]).expect("choose OPP");

    let view = runner.pending_selection_view().expect("source prompt");
    assert!(matches!(view.kind, SelectionKind::SourceMulti { min: 1, max: 1, .. }));
    assert_eq!(view.valid_action_ids.len(), 1, "only the Option source (not DIGI-SRC) is selectable");
    runner.execute_action(0, view.valid_action_ids[0]).expect("pick OPT-A");
    runner.auto_resolve().expect("finish");

    assert_eq!(sources_of(&runner, opp), 2, "OPT-A left OPP's stack");
    assert_eq!(runner.trash_size(1), opp_trash_before + 1, "OPT-A went to its owner's trash");
    assert!(zone_ids(&runner.game.players[1].trash, &runner.game.card_data).contains(&"OPT-A".to_string()));
}

#[test]
fn ex7_010_wa_trashes_option_from_own_digimon_sources() {
    let mut runner = base().start();
    let deputy = runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_stack(0, &["OPT-B", "ALLY"]);
    let opp = runner.place_on_field(1, "OPP", Some(0));
    let own_trash_before = runner.trash_size(0);

    runner.attack_digimon(deputy, opp, false);
    assert!(accept_if_optional(&mut runner));
    let view = runner.pending_selection_view().expect("host prompt");
    assert_eq!(view.valid_action_ids.len(), 1, "only ALLY carries an Option source");
    runner.execute_action(0, view.valid_action_ids[0]).expect("choose ALLY");
    let view = runner.pending_selection_view().expect("source prompt");
    runner.execute_action(0, view.valid_action_ids[0]).expect("pick OPT-B");
    runner.auto_resolve().expect("finish");

    assert_eq!(sources_of(&runner, ally), 1, "OPT-B trashed from ALLY");
    assert_eq!(runner.trash_size(0), own_trash_before + 1);
}

#[test]
fn ex7_010_wa_no_prompt_when_no_digimon_has_an_option_source() {
    let mut runner = base().start();
    let deputy = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_stack(1, &["DIGI-SRC", "OPP"]);
    runner.attack_digimon(deputy, opp, false);
    assert!(
        runner.pending_selection().is_none(),
        "no Option in any digivolution cards → nothing to trash, no prompt"
    );
}

#[test]
fn ex7_010_wa_decline_trashes_nothing() {
    let mut runner = base().start();
    let deputy = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_stack(1, &["OPT-A", "OPP-BIG"]);
    runner.attack_digimon(deputy, opp, false);
    assert!(runner.pending_is_optional());
    runner.execute_action(0, PASS).expect("decline");
    runner.auto_resolve().expect("battle resolves");
    assert_eq!(sources_of(&runner, opp), 2, "OPT-A stays");
}

#[test]
fn ex7_010_opt_locks_second_activation_same_turn_and_clears_next_turn() {
    let mut runner = base().start();
    let deputy = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_stack(1, &["OPT-A", "OPT-B", "OPP"]);
    runner.attack_digimon(deputy, opp, false);
    assert!(accept_if_optional(&mut runner));
    let view = runner.pending_selection_view().expect("host prompt");
    runner.execute_action(0, view.valid_action_ids[0]).expect("choose OPP");
    let view = runner.pending_selection_view().expect("source prompt");
    runner.execute_action(0, view.valid_action_ids[0]).expect("pick");
    runner.auto_resolve().expect("battle: Deputymon 6000 beats OPP 4000");
    assert_eq!(runner.battle_area_size(1), 0, "OPP deleted in battle");

    // Second attack this turn (unsuspend it for the test) → OPT lockout.
    runner.game.players[0].battle_area[deputy.index as usize].is_suspended = false;
    let opp2 = runner.place_stack(1, &["OPT-A", "OPP-2"]);
    runner.attack_digimon(deputy, opp2, false);
    assert!(
        runner.pending_selection().is_none(),
        "[Once Per Turn]: the second attack in the same turn must not re-offer the trash"
    );
    runner.auto_resolve().ok();

    // Next own turn: the lockout clears.
    runner.end_turn();
    runner.end_turn();
    runner.game.players[0].battle_area[deputy.index as usize].is_suspended = false;
    let opp3 = runner.place_stack(1, &["OPT-B", "OPP-2"]);
    runner.attack_digimon(deputy, opp3, false);
    assert!(runner.pending_selection().is_some(), "OPT lockout clears after the turn ends");
}

#[test]
fn ex7_010_wd_offers_trash_when_digivolving() {
    let mut runner = base().hand(0, &[CARD_ID]).start();
    let base_perm = runner.place_on_field(0, "RED-LV3", Some(0));
    let opp = runner.place_stack(1, &["OPT-A", "OPP"]);
    let memory_before = runner.memory();
    assert!(
        runner.game.digivolve_from_hand(0, 0, base_perm.index as usize, PlaySource::ByHand),
        "Red Lv.3 base, cost 2"
    );
    assert_eq!(runner.memory(), memory_before - 2);
    assert!(accept_if_optional(&mut runner), "[When Digivolving] offers the trash");
    let view = runner.pending_selection_view().expect("host prompt");
    runner.execute_action(0, view.valid_action_ids[0]).expect("choose OPP");
    let view = runner.pending_selection_view().expect("source prompt");
    runner.execute_action(0, view.valid_action_ids[0]).expect("pick");
    runner.auto_resolve().expect("finish");
    assert_eq!(sources_of(&runner, opp), 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — [Your Turn] trait grant + inherited DP
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_010_gains_three_musketeers_trait_on_own_turn_only() {
    let mut runner = base().start();
    let deputy = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.tick_declarative_effects();
    assert!(has_tm_trait(&runner, deputy), "[Your Turn] Deputymon has the [Three Musketeers] trait");
    assert!(
        runner
            .game
            .players[0]
            .battle_area[deputy.index as usize]
            .has_trait_for_rules("Mutant", &runner.game.card_data, &runner.game.modifiers, deputy),
        "printed traits are kept (add, not replace)"
    );

    runner.end_turn();
    runner.game.tick_declarative_effects();
    assert!(!has_tm_trait(&runner, deputy), "no trait on the opponent's turn");
}

#[test]
fn ex7_010_trait_grant_is_face_up_only() {
    let mut runner = base().start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.game.tick_declarative_effects();
    assert!(
        !has_tm_trait(&runner, stack),
        "as a digivolution card Deputymon grants no trait (DCGO TopCard == card gate)"
    );
    assert_eq!(runner.effective_dp(stack), Some(7000 + 2000), "...but the inherited +2000 DP applies");
}

#[test]
fn ex7_010_inherited_dp_bonus_only_on_own_turn() {
    let mut runner = base().start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.end_turn();
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(stack), Some(7000), "no bonus on the opponent's turn");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — Digivolution paths
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_010_digivolves_from_black_lv3_with_tm_in_text_for_two() {
    let mut blk = digimon("BLK-TM-LV3", 3, CardColor::Black, 3000);
    blk.effect_text = "[On Play] ... [Three Musketeers] ...".to_string();
    let mut runner = base().add_card(blk).hand(0, &[CARD_ID]).start();
    let base_perm = runner.place_on_field(0, "BLK-TM-LV3", Some(0));
    let memory_before = runner.memory();
    assert!(runner.game.digivolve_from_hand(0, 0, base_perm.index as usize, PlaySource::ByHand));
    assert_eq!(runner.memory(), memory_before - 2, "special path costs 2");
}

#[test]
fn ex7_010_cannot_digivolve_from_plain_black_lv3() {
    let mut runner = base().hand(0, &[CARD_ID]).start();
    let base_perm = runner.place_on_field(0, "BLK-LV3", Some(0));
    assert!(!runner.game.digivolve_from_hand(0, 0, base_perm.index as usize, PlaySource::ByHand));
}
