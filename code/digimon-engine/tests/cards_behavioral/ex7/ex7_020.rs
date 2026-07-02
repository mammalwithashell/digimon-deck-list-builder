//! EX7-020 Paledramon — Digimon, Lv.4, Blue, DP 5000, Cost 5.
//! Traits: Dragon. Attribute: Data. Rule box: "Trait: Has [Ice-Snow] Type."
//! Evo cost: from blue Lv.3 for 2 memory (cards.json evo_costs; matches the
//! card image's single digivolve circle — no alt evo box printed).
//!
//! # Card text (card image EX7-020.webp — authoritative; matches cards.json)
//!
//! **[When Digivolving]** Trash the bottom 2 digivolution cards of 1 of your
//! opponent's Digimon. Then, if your opponent has no Digimon with digivolution
//! cards, this Digimon gains <Jamming> and <Blocker> until the end of your
//! opponent's turn.
//!
//! **Inherited:** [When Attacking] [Once Per Turn] Trash the top digivolution
//! card of 1 of your opponent's Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Blue/EX7_020.cs
//!   - When Digivolving (OnEnterFieldAnyone): SelectPermanentEffect over ANY
//!     opponent battle-area Digimon (canNoSelect: false — MANDATORY;
//!     maxCount = min(1, count)) → TrashDigivolutionCardsFromTopOrBottom(
//!     trashCount: 2, isFromTop: false) — bottom 2, "up to" semantics.
//!     THEN, unconditionally after the select loop (CanActivateCondition only
//!     checks self-on-field — the grant check runs even when NO selection
//!     occurred because the opponent had no Digimon):
//!     `if (!HasMatchConditionOpponentsPermanent(p => p.IsDigimon &&
//!     !p.HasNoDigivolutionCards))` → GainJamming + GainBlocker on
//!     card.PermanentOfThisCard(), EffectDuration.UntilOpponentTurnEnd.
//!   - When Attacking ESS (OnAllyAttack, SetHashString "Attacking_EX7_020",
//!     maxUsableCount 1 = [Once Per Turn], SetIsInheritedEffect(true)):
//!     if any opponent battle-area Digimon exists, SelectPermanentEffect
//!     (canNoSelect: false — MANDATORY) → TrashDigivolutionCardsFromTopOrBottom(
//!     trashCount: 1, isFromTop: true) — the TOP digivolution card (the source
//!     immediately beneath the active top card) = `trash_top_stacked_sources`.
//!
//! # Faithfulness notes
//! - DCGO lets the When-Digivolving pick target ANY opponent Digimon, even one
//!   with no digivolution cards (the trash then no-ops, and the grant outcome
//!   is unchanged — it depends only on the post-trash board). The printed
//!   instruction is to trash digivolution cards, which only a stacked Digimon
//!   can satisfy, so this spec restricts the target set to opponent Digimon
//!   with stack_size >= 2 (EX7-016 sibling idiom) — no dominated no-op choice
//!   is exposed to the action space. End-state behavior is identical.
//! - The grant must fire even when the opponent ALREADY had no sourced Digimon
//!   (DCGO runs the check unconditionally) — so the trash half is gated
//!   per-step, NOT at clause level.
//!
//! # Patterns this test covers
//! - F: [When Digivolving] trash_bottom_sources(2) with up-to semantics
//! - Conditional self keyword grant (Jamming + Blocker) gated on a
//!   post-trash `no_permanent` existential
//! - Expiry: end_of_opponents_turn (persists through own turn end)
//! - G4-adjacent: inherited When Attacking + OPT source peel
//!   (`trash_top_stacked_sources`, count 1 — EX7-016 idiom)

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword, PlaySource};
use digimon_engine::selection::SelectionKind;

// ─── Fixture builders ─────────────────────────────────────────────────────────

/// A blue Lv.3 Digimon — the digivolution base for EX7-020 (Lv.3 blue / cost 2).
fn blue_lv3(id: &str) -> CardData {
    let mut c = make_test_card(id, "BlueRookie");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(3);
    c.dp = Some(2000);
    c
}

/// Generic opponent-side Digimon / stack filler.
fn junk(id: &str) -> CardData {
    let mut c = make_test_card(id, "Junkmon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c
}

/// High-DP carrier so the EX7-020 inherited effect can ride under a surviving
/// attacker across security checks.
fn strong_carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, "Carrier");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(9000);
    c
}

fn base_builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("EX7-020")
        .expect("EX7-020 YAML loads from the embedded pack")
}

fn zone_ids(
    cards: &[digimon_engine::card_source::CardSource],
    data: &[digimon_engine::card_data::CardData],
) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

/// Build a runner with EX7-020 in hand atop a blue Lv.3 base on field, plus
/// deck filler (the digivolve draw needs a non-empty deck).
fn wd_builder() -> DebugRunnerBuilder {
    base_builder()
        .add_card(blue_lv3("BASE"))
        .add_card(junk("FILL"))
        .add_card(junk("OPP-B1"))
        .add_card(junk("OPP-B2"))
        .add_card(junk("OPP-B3"))
        .add_card(junk("OPP-TOP"))
        .add_card(junk("OPP-C1"))
        .add_card(junk("OPP-TOP2"))
        .hand(0, &["EX7-020"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
}

/// Digivolve EX7-020 (hand index 0) onto the given base slot; the
/// [When Digivolving] trigger fires inside this call.
fn digivolve_ex7_020(runner: &mut DebugRunner, base: digimon_engine::permanent::PermanentHandle) {
    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "EX7-020 must digivolve from a blue Lv.3 base (cost 2)"
    );
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "EX7-020",
        "EX7-020 is now the top card of the digivolved stack"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_020_compiles_with_printed_stats_and_lv3_blue_path() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card("EX7-020")
        .expect("EX7-020 in compiled cards");

    assert_eq!(compiled.card, "EX7-020");
    assert_eq!(compiled.name, "Paledramon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(4));
    assert_eq!(compiled.color, vec![CompiledColor::Blue]);
    assert_eq!(compiled.cost, Some(5));
    assert_eq!(compiled.dp, Some(5000));
    assert!(
        compiled.traits.iter().any(|t| t == "Dragon"),
        "printed type [Dragon] missing"
    );
    assert!(
        compiled.traits.iter().any(|t| t == "Ice-Snow"),
        "rule box 'Trait: Has [Ice-Snow] Type.' must surface as an Ice-Snow trait"
    );

    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(2))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(3) || from.all_of.iter().any(|p| p.level_eq == Some(3)))
                        && (from.color_is == Some(CompiledColor::Blue)
                            || from
                                .all_of
                                .iter()
                                .any(|p| p.color_is == Some(CompiledColor::Blue)))
                })
        }),
        "Paledramon must digivolve from a blue Lv.3 for cost 2"
    );
}

#[test]
fn ex7_020_clause_shape_wd_mandatory_and_inherited_opt() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card("EX7-020")
        .expect("EX7-020 in compiled cards");

    let triggered: Vec<_> = compiled
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
        "EX7-020 has exactly two triggered clauses (When Digivolving + inherited When Attacking)"
    );

    let wd = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::WhenDigivolving])
        .expect("When Digivolving clause exists");
    assert_eq!(wd.scope, CompiledScope::FaceUp);
    assert!(
        !wd.optional,
        "the trash/grant has no 'you may' — mandatory (DCGO canNoSelect=false)"
    );
    assert!(!wd.once_per_turn);
    assert!(
        wd.condition.is_none(),
        "the When Digivolving clause must NOT be gated at clause level: DCGO fires it \
         (and grants Jamming/Blocker) even when the opponent already has no sourced Digimon"
    );

    let inherited = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::WhenAttacking])
        .expect("inherited When Attacking clause exists");
    assert_eq!(inherited.scope, CompiledScope::Inherited);
    assert!(
        inherited.once_per_turn,
        "printed [Once Per Turn] must compile to once_per_turn"
    );
    assert!(
        !inherited.optional,
        "the source trash has no 'you may' — mandatory when a target exists"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — [When Digivolving] trash bottom 2 + conditional Jamming/Blocker
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE (trash half): the chosen opponent Digimon loses its BOTTOM two
/// digivolution cards; the remaining source and the active top card stay. The
/// opponent still has a Digimon with digivolution cards afterwards, so the
/// grant half must NOT fire.
#[test]
fn ex7_020_wd_trashes_bottom_two_keeps_top_and_no_grant_while_sources_remain() {
    let mut runner = wd_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));
    // Opponent stack, bottom → top: OPP-B1 (bottom source), OPP-B2, OPP-B3,
    // OPP-TOP (active top card). 3 digivolution cards.
    runner.place_stack(1, &["OPP-B1", "OPP-B2", "OPP-B3", "OPP-TOP"]);

    digivolve_ex7_020(&mut runner, base);

    // Mandatory opponent-Digimon selection, no PASS, exactly one legal target.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "a mandatory opponent-Digimon selection must install"
    );
    let view = runner.pending_selection_view().expect("target prompt");
    assert!(
        !runner.pending_is_optional(),
        "the trash is not 'you may' — mandatory (DCGO canNoSelect=false)"
    );
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "PASS must not be exposed while a legal target exists"
    );
    assert_eq!(view.valid_action_ids.len(), 1, "exactly one legal target");

    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the stacked opponent Digimon");
    runner.auto_resolve().expect("finish When Digivolving");

    // Bottom two sources trashed, in bottom-to-top order; the rest stays.
    let trash_ids = zone_ids(&runner.game.players[1].trash, &runner.game.card_data);
    assert!(
        trash_ids.contains(&"OPP-B1".to_string()),
        "the bottom digivolution card is trashed; trash={trash_ids:?}"
    );
    assert!(
        trash_ids.contains(&"OPP-B2".to_string()),
        "the second-from-bottom digivolution card is trashed; trash={trash_ids:?}"
    );
    assert!(
        !trash_ids.contains(&"OPP-B3".to_string()),
        "the TOP digivolution card stays (bottom 2, not top 2)"
    );
    assert!(
        !trash_ids.contains(&"OPP-TOP".to_string()),
        "the active top card stays"
    );
    let opp = &runner.game.players[1].battle_area[0];
    assert_eq!(opp.card_sources.len(), 2, "stack shrinks 4 -> 2");
    assert_eq!(
        opp.top_card().card_id(&runner.game.card_data),
        "OPP-TOP",
        "top card unchanged"
    );

    // The opponent still has a Digimon WITH digivolution cards (OPP-B3 under
    // OPP-TOP) → no Jamming, no Blocker.
    let paledramon = runner.perm_handle(0, base.index as usize);
    assert!(
        !runner.game.has_keyword(paledramon, Keyword::Jamming),
        "Jamming must NOT be granted while a sourced opponent Digimon remains"
    );
    assert!(
        !runner.game.has_keyword(paledramon, Keyword::Blocker),
        "Blocker must NOT be granted while a sourced opponent Digimon remains"
    );
}

/// "Up to" semantics: a target with only ONE digivolution card loses that one
/// (DCGO TrashDigivolutionCardsFromTopOrBottom trashes min(2, available));
/// the opponent then has no sourced Digimon, so Jamming + Blocker are granted.
#[test]
fn ex7_020_wd_single_source_target_trashes_it_and_grants_jamming_blocker() {
    let mut runner = wd_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));
    // One digivolution card only: stack [OPP-B1, OPP-TOP].
    runner.place_stack(1, &["OPP-B1", "OPP-TOP"]);

    digivolve_ex7_020(&mut runner, base);

    let view = runner.pending_selection_view().expect("target prompt");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "a 1-source Digimon (stack size 2) is a legal target"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the 1-source opponent Digimon");
    runner.auto_resolve().expect("finish When Digivolving");

    let trash_ids = zone_ids(&runner.game.players[1].trash, &runner.game.card_data);
    assert!(
        trash_ids.contains(&"OPP-B1".to_string()),
        "the only digivolution card is trashed (up-to-2 semantics); trash={trash_ids:?}"
    );
    let opp = &runner.game.players[1].battle_area[0];
    assert_eq!(opp.card_sources.len(), 1, "stack shrinks 2 -> 1");
    assert_eq!(
        opp.top_card().card_id(&runner.game.card_data),
        "OPP-TOP",
        "top card unchanged"
    );

    // Post-trash, the opponent has NO Digimon with digivolution cards →
    // this Digimon gains <Jamming> and <Blocker>.
    let paledramon = runner.perm_handle(0, base.index as usize);
    assert!(
        runner.game.has_keyword(paledramon, Keyword::Jamming),
        "Jamming must be granted when no opponent Digimon has digivolution cards"
    );
    assert!(
        runner.game.has_keyword(paledramon, Keyword::Blocker),
        "Blocker must be granted when no opponent Digimon has digivolution cards"
    );
}

/// NEGATIVE (grant half): a SECOND sourced opponent Digimon (not the trash
/// target) blocks the grant — the check is over the whole board after the
/// trash, not just the chosen target.
#[test]
fn ex7_020_wd_second_sourced_opponent_digimon_blocks_grant() {
    let mut runner = wd_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));
    // Target A: 2 sources — fully emptied by the trash.
    runner.place_stack(1, &["OPP-B1", "OPP-B2", "OPP-TOP"]);
    // Bystander B: 1 source — keeps its digivolution card.
    runner.place_stack(1, &["OPP-C1", "OPP-TOP2"]);

    digivolve_ex7_020(&mut runner, base);

    let view = runner.pending_selection_view().expect("target prompt");
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both sourced opponent Digimon are legal targets"
    );
    // Pick stack A (slot 0) — the OppField action ids are ordered by slot, and
    // auto-picking the first valid id selects the first battle-area slot.
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose stack A");
    runner.auto_resolve().expect("finish When Digivolving");

    // A lost both sources; B is untouched.
    let trash_ids = zone_ids(&runner.game.players[1].trash, &runner.game.card_data);
    assert!(trash_ids.contains(&"OPP-B1".to_string()));
    assert!(trash_ids.contains(&"OPP-B2".to_string()));
    assert!(
        !trash_ids.contains(&"OPP-C1".to_string()),
        "the bystander's digivolution card must stay"
    );
    assert_eq!(runner.game.players[1].battle_area[0].card_sources.len(), 1);
    assert_eq!(runner.game.players[1].battle_area[1].card_sources.len(), 2);

    // B still has a digivolution card → no grant.
    let paledramon = runner.perm_handle(0, base.index as usize);
    assert!(
        !runner.game.has_keyword(paledramon, Keyword::Jamming),
        "a second sourced opponent Digimon must block the Jamming grant"
    );
    assert!(
        !runner.game.has_keyword(paledramon, Keyword::Blocker),
        "a second sourced opponent Digimon must block the Blocker grant"
    );
}

/// DCGO-verified: the grant half fires even when the opponent ALREADY has no
/// Digimon with digivolution cards — no selection prompt installs (nothing to
/// trash), but Jamming + Blocker are still granted.
#[test]
fn ex7_020_wd_grant_fires_when_opponent_has_no_sourced_digimon() {
    let mut runner = wd_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));
    // Bare opponent Digimon — no digivolution cards anywhere.
    runner.place_on_field(1, "OPP-TOP", Some(0));

    digivolve_ex7_020(&mut runner, base);

    assert!(
        runner.pending_selection().is_none(),
        "no trash target exists — no selection may install (no dominated no-op pick)"
    );
    runner.auto_resolve().ok();

    assert!(
        runner.game.players[1].trash.is_empty(),
        "nothing may be trashed"
    );
    let paledramon = runner.perm_handle(0, base.index as usize);
    assert!(
        runner.game.has_keyword(paledramon, Keyword::Jamming),
        "DCGO runs the grant check unconditionally — Jamming fires with no sourced opp Digimon"
    );
    assert!(
        runner.game.has_keyword(paledramon, Keyword::Blocker),
        "DCGO runs the grant check unconditionally — Blocker fires with no sourced opp Digimon"
    );
}

/// Same DCGO-verified shape with a fully empty opponent board: the clause
/// still fires and grants both keywords.
#[test]
fn ex7_020_wd_grant_fires_with_empty_opponent_board() {
    let mut runner = wd_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));

    digivolve_ex7_020(&mut runner, base);

    assert!(runner.pending_selection().is_none());
    runner.auto_resolve().ok();

    let paledramon = runner.perm_handle(0, base.index as usize);
    assert!(
        runner.game.has_keyword(paledramon, Keyword::Jamming),
        "Jamming must be granted when the opponent has no Digimon at all"
    );
    assert!(
        runner.game.has_keyword(paledramon, Keyword::Blocker),
        "Blocker must be granted when the opponent has no Digimon at all"
    );
}

/// Expiry: "until the end of your opponent's turn" — the keywords persist
/// through the controller's own turn end, survive the opponent's turn, and
/// expire when the opponent's turn ends.
#[test]
fn ex7_020_wd_grants_expire_at_end_of_opponents_turn() {
    let mut runner = wd_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.place_stack(1, &["OPP-B1", "OPP-TOP"]);

    digivolve_ex7_020(&mut runner, base);
    let view = runner.pending_selection_view().expect("target prompt");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the 1-source opponent Digimon");
    runner.auto_resolve().expect("finish When Digivolving");

    let paledramon = runner.perm_handle(0, base.index as usize);
    assert!(runner.game.has_keyword(paledramon, Keyword::Jamming));
    assert!(runner.game.has_keyword(paledramon, Keyword::Blocker));

    // End player 0's turn → keywords persist into the opponent's turn.
    runner.end_turn();
    assert!(
        runner.game.has_keyword(paledramon, Keyword::Jamming),
        "Jamming must persist through the controller's own turn end"
    );
    assert!(
        runner.game.has_keyword(paledramon, Keyword::Blocker),
        "Blocker must persist through the controller's own turn end"
    );

    // End the opponent's turn → keywords expire.
    runner.end_turn();
    assert!(
        !runner.game.has_keyword(paledramon, Keyword::Jamming),
        "Jamming must expire when the opponent's turn ends"
    );
    assert!(
        !runner.game.has_keyword(paledramon, Keyword::Blocker),
        "Blocker must expire when the opponent's turn ends"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Inherited [When Attacking][Once Per Turn] source peel
// ─────────────────────────────────────────────────────────────────────────────

/// Stack the carrier (with EX7-020 underneath) for player 0 and a 3-card
/// opponent stack; security for player 1 so the game continues.
fn inherited_setup() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut runner = base_builder()
        .add_card(strong_carrier("CARRIER"))
        .add_card(junk("OPP-BOTTOM"))
        .add_card(junk("OPP-MID"))
        .add_card(junk("OPP-TOP"))
        .add_card(junk("SEC1"))
        .add_card(junk("SEC2"))
        .security(1, &["SEC1", "SEC2"])
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &["EX7-020", "CARRIER"]);
    // Opponent: top card OPP-TOP, digivolution cards OPP-BOTTOM (bottom) and
    // OPP-MID (top digivolution card).
    runner.place_stack(1, &["OPP-BOTTOM", "OPP-MID", "OPP-TOP"]);
    (runner, carrier)
}

/// POSITIVE: attacking with EX7-020 in the stack prompts a mandatory opponent
/// Digimon selection; resolving it trashes the TOP digivolution card
/// (OPP-MID), leaving the active top card and the bottom source in place.
#[test]
fn ex7_020_inherited_when_attacking_trashes_top_digivolution_card() {
    let (mut runner, carrier) = inherited_setup();

    runner.attack_player(carrier, 1, false);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "a mandatory opponent-Digimon selection must install"
    );
    let view = runner.pending_selection_view().expect("target prompt");
    assert!(
        !runner.pending_is_optional(),
        "the trash is not 'you may' — no PASS while a legal target exists"
    );
    assert!(!view.valid_action_ids.contains(&PASS));
    assert_eq!(view.valid_action_ids.len(), 1, "exactly one legal target");

    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the stacked opponent Digimon");
    runner.auto_resolve().expect("finish attack");

    let trash_ids = zone_ids(&runner.game.players[1].trash, &runner.game.card_data);
    assert!(
        trash_ids.contains(&"OPP-MID".to_string()),
        "the TOP digivolution card (immediately beneath the top card) is trashed; trash={trash_ids:?}"
    );
    assert!(
        !trash_ids.contains(&"OPP-BOTTOM".to_string()),
        "the bottom digivolution card stays"
    );
    assert!(
        !trash_ids.contains(&"OPP-TOP".to_string()),
        "the active top card stays"
    );
    let opp = &runner.game.players[1].battle_area[0];
    assert_eq!(opp.card_sources.len(), 2, "stack shrinks 3 -> 2");
    assert_eq!(
        opp.top_card().card_id(&runner.game.card_data),
        "OPP-TOP",
        "top card unchanged"
    );
}

/// NEGATIVE: with no opponent Digimon holding digivolution cards, the clause
/// does not trigger at all (no prompt, no trash).
#[test]
fn ex7_020_inherited_no_trigger_without_stacked_opponent_digimon() {
    let mut runner = base_builder()
        .add_card(strong_carrier("CARRIER"))
        .add_card(junk("OPP-TOP"))
        .add_card(junk("SEC1"))
        .security(1, &["SEC1"])
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &["EX7-020", "CARRIER"]);
    // Bare opponent Digimon only — no digivolution cards anywhere.
    runner.place_on_field(1, "OPP-TOP", Some(0));

    runner.attack_player(carrier, 1, false);

    assert!(
        runner.pending_selection().is_none(),
        "no target selection may install when no opponent Digimon has digivolution cards"
    );
    let _ = runner.auto_resolve();
    // Only security-check fallout (SEC1) may reach the trash — the effect
    // itself must not have trashed anything.
    let trash_ids = zone_ids(&runner.game.players[1].trash, &runner.game.card_data);
    assert!(
        trash_ids.iter().all(|id| id == "SEC1"),
        "the effect must not trash anything (only security fallout allowed); trash={trash_ids:?}"
    );
}

/// OPT: the second attack in the same turn must not re-trigger the inherited
/// effect even though a legal target (OPP-BOTTOM still stacked) remains.
#[test]
fn ex7_020_inherited_opt_blocks_second_attack_same_turn() {
    let (mut runner, carrier) = inherited_setup();

    // First attack: fires, trashes OPP-MID. (Security fallout — SEC cards —
    // also reaches the trash; assertions are membership-based.)
    runner.attack_player(carrier, 1, false);
    let view = runner
        .pending_selection_view()
        .expect("first-attack prompt");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("resolve first trash");
    runner.auto_resolve().expect("finish first attack");
    let trash_ids = zone_ids(&runner.game.players[1].trash, &runner.game.card_data);
    assert!(
        trash_ids.contains(&"OPP-MID".to_string()),
        "first attack must trash the top digivolution card; trash={trash_ids:?}"
    );
    assert!(
        !trash_ids.contains(&"OPP-BOTTOM".to_string()),
        "first attack trashes exactly one digivolution card"
    );

    if runner.game_over() {
        return; // security exhausted unexpectedly; OPT cannot be probed further
    }

    // Second attack same turn: the opponent stack still has OPP-BOTTOM
    // (stack size 2 >= 2), so a target exists — but OPT must lock the clause.
    let carrier_again = runner.perm_handle(0, 0);
    runner.attack_player(carrier_again, 1, false);

    assert!(
        runner.pending_selection().is_none(),
        "[Once Per Turn] must block the second activation in the same turn"
    );
    let _ = runner.auto_resolve();
    let trash_ids = zone_ids(&runner.game.players[1].trash, &runner.game.card_data);
    assert!(
        !trash_ids.contains(&"OPP-BOTTOM".to_string()),
        "no additional digivolution card may be trashed by the second attack; trash={trash_ids:?}"
    );
    assert_eq!(
        runner.game.players[1].battle_area[0].card_sources.len(),
        2,
        "the opponent stack must stay at 2 cards after the locked second attack"
    );
}
