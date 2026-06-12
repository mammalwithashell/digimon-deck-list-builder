//! EX7-021 CrysPaledramon — Digimon, Lv.5, Blue, DP 7000, Cost 7.
//! Traits: Dragonkin. Attribute: Data. Form: Ultimate. Rarity: R.
//! Rule box (card image): "Rule: Trait: Has [Ice-Snow] Type." — the card
//! permanently has the [Ice-Snow] trait in addition to its printed type line
//! (cards.json type_eng carries only ["Dragonkin"]; same API-ingest gap as
//! siblings EX7-016 / EX7-020).
//! Standard digivolution: Lv.4 blue / cost 3 (cards.json evo_costs; the card
//! image shows a single digivolve circle — no alt evo box printed).
//!
//! # Card text (card image EX7-021.webp — authoritative; matches cards.json)
//!
//! **<Iceclad>** (Other than against Security Digimon, compare the number of
//! digivolution cards instead of DP in this Digimon's battles.)
//!
//! **[When Digivolving]** Trash any 2 digivolution cards under your
//! opponent's Digimon. Then, if your opponent has no Digimon with
//! digivolution cards, this Digimon unsuspends.
//!
//! **Inherited:** [Your Turn] While your opponent has no Digimon with
//! digivolution cards, this Digimon with the [Ice-Snow] trait gains
//! <Piercing> and <Security A. +1>.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Blue/EX7_021.cs
//!   - Iceclad (None): IcecladSelfStaticEffect(isInheritedEffect: false).
//!   - When Digivolving (OnEnterFieldAnyone): SelectTrashDigivolutionCards(
//!     permanentCondition: opponent battle-area Digimon, maxCount: 2,
//!     canNoTrash: FALSE — MANDATORY, isFromOnly1Permanent: FALSE —
//!     cross-permanent). The helper caps at
//!     maxDigivolutionDiscardCount = min(total selectable digivolution
//!     cards, 2) — "up to" semantics when fewer than 2 exist; with zero it
//!     no-ops entirely. THEN, unconditionally after the trash loop
//!     (CanActivateCondition only checks self-on-field): if
//!     !HasMatchConditionOpponentsPermanent(p => p.IsDigimon &&
//!     !p.HasNoDigivolutionCards) → IUnsuspendPermanents on
//!     card.PermanentOfThisCard().
//!   - Inherited (None + OnDetermineDoSecurityCheck, isInheritedEffect:
//!     true): ChangeSelfSAttackStaticEffect(+1) and PierceSelfEffect, both
//!     gated on IsOwnerTurn(card) && opponent has no Digimon with
//!     DigivolutionCards && carrier TopCard.EqualsTraits("Ice-Snow").
//!
//! # Faithfulness notes
//! - The unsuspend check must fire even when the opponent ALREADY had no
//!   sourced Digimon (DCGO runs it unconditionally after the trash loop) —
//!   so the trash half is gated per-step, NOT at clause level (EX7-020
//!   sibling idiom).
//! - "Trash any 2" is cross-permanent and mandatory: the player picks which
//!   digivolution cards from which opponent Digimon — exactly 2 when 2+
//!   exist (no PASS), the lone card when exactly 1 exists, nothing when none
//!   exist (DCGO min(total, 2) cap).
//! - The inherited grant gates on the INHERITING Digimon's [Ice-Snow] trait
//!   ("this Digimon with the [Ice-Snow] trait") — a carrier-trait gate, not
//!   a gate on EX7-021 itself.
//!
//! # Patterns this test covers
//! - Iceclad combat semantics (stack-count compare, not DP) — EX8-022 idiom.
//! - [When Digivolving] cross-permanent `select_opponent_sources` (no
//!   `target:`) + `trash_selected_sources`, with the DCGO up-to-2
//!   availability split.
//! - Post-trash unconditional `no_permanent` check + `unsuspend: source`
//!   (BT17-077 unsuspend shape).
//! - Inherited conditional self-aura granting a keyword (Piercing) AND a
//!   flat Security A. +1 under one active_when gate (EX11-002 + BT12-031
//!   idioms).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{encode_source_select, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX7-021";

// ─── Fixture builders ─────────────────────────────────────────────────────────

/// A blue Lv.4 Digimon — the digivolution base for EX7-021 (Lv.4 blue / cost 3).
fn blue_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, "BlueChampion");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(4);
    c.dp = Some(4000);
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

/// Opponent Digimon with parameterized DP for the Iceclad battle tests.
fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, "Opp Digimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(dp);
    c.colors = vec![CardColor::Red];
    c
}

/// An [Ice-Snow] trait carrier — EX7-021 sits underneath it so the inherited
/// [Your Turn] aura can ride.
fn ice_snow_carrier() -> CardData {
    let mut c = make_test_card("ICE-CARRIER", "IceCarrier");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.traits = vec!["Ice-Snow".to_string()];
    c
}

/// A carrier WITHOUT the Ice-Snow trait — the aura must stay off.
fn plain_carrier() -> CardData {
    let mut c = make_test_card("PLAIN-CARRIER", "PlainCarrier");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.traits = vec!["Beast".to_string()];
    c
}

fn base_builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX7-021 YAML loads from the embedded pack")
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

fn is_suspended(runner: &DebugRunner, handle: PermanentHandle) -> bool {
    runner.game.players[handle.player as usize]
        .battle_area
        .get(handle.index as usize)
        .map(|p| p.is_suspended)
        .unwrap_or(false)
}

/// Build a runner with EX7-021 in hand atop a blue Lv.4 base on field, plus
/// deck filler (the digivolve draw needs a non-empty deck).
fn wd_builder() -> DebugRunnerBuilder {
    base_builder()
        .add_card(blue_lv4("BASE"))
        .add_card(junk("FILL"))
        .add_card(junk("OPP-A1"))
        .add_card(junk("OPP-A2"))
        .add_card(junk("OPP-A3"))
        .add_card(junk("OPP-ATOP"))
        .add_card(junk("OPP-B1"))
        .add_card(junk("OPP-BTOP"))
        .hand(0, &["EX7-021"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
}

/// Digivolve EX7-021 (hand index 0) onto the given base slot; the
/// [When Digivolving] trigger fires inside this call.
fn digivolve_ex7_021(runner: &mut DebugRunner, base: PermanentHandle) {
    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "EX7-021 must digivolve from a blue Lv.4 base (cost 3)"
    );
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "EX7-021",
        "EX7-021 is now the top card of the digivolved stack"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_021_compiles_with_printed_stats_and_lv4_blue_path() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-021 in compiled cards");

    assert_eq!(compiled.card, "EX7-021");
    assert_eq!(compiled.name, "CrysPaledramon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(5));
    assert_eq!(compiled.color, vec![CompiledColor::Blue]);
    assert_eq!(compiled.cost, Some(7));
    assert_eq!(compiled.dp, Some(7000));
    assert!(
        compiled.traits.iter().any(|t| t == "Dragonkin"),
        "printed type [Dragonkin] missing"
    );
    assert!(
        compiled.traits.iter().any(|t| t == "Ice-Snow"),
        "rule box 'Trait: Has [Ice-Snow] Type.' must surface as an Ice-Snow trait"
    );

    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(3))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(4)
                        || from.all_of.iter().any(|p| p.level_eq == Some(4)))
                        && (from.color_is == Some(CompiledColor::Blue)
                            || from
                                .all_of
                                .iter()
                                .any(|p| p.color_is == Some(CompiledColor::Blue)))
                })
        }),
        "CrysPaledramon must digivolve from a blue Lv.4 for cost 3"
    );
}

/// The printed <Iceclad> keyword is encoded as a face-up grant_keyword clause
/// AND is live on the placed permanent via the canonical `Game::has_keyword`
/// query (the combat resolver's lookup site).
#[test]
fn ex7_021_iceclad_keyword_attached() {
    let mut runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-021 in compiled cards");

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
        "EX7-021 must encode <Iceclad> as a face-up grant_keyword clause"
    );

    let crys = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(
        runner.game.has_keyword(crys, Keyword::Iceclad),
        "the placed CrysPaledramon must answer has_keyword(Iceclad)"
    );
}

/// The [When Digivolving] clause is face-up, mandatory, not once-per-turn,
/// and NOT gated at clause level (the unsuspend check must run even when the
/// opponent already had no sourced Digimon — DCGO runs it unconditionally).
#[test]
fn ex7_021_wd_clause_shape_mandatory_and_ungated() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-021 in compiled cards");

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
        1,
        "EX7-021 has exactly one triggered clause (When Digivolving)"
    );

    let wd = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::WhenDigivolving])
        .expect("When Digivolving clause exists");
    assert_eq!(wd.scope, CompiledScope::FaceUp);
    assert!(
        !wd.optional,
        "the trash/unsuspend has no 'you may' — mandatory (DCGO canNoTrash=false)"
    );
    assert!(!wd.once_per_turn);
    assert!(
        wd.condition.is_none(),
        "the When Digivolving clause must NOT be gated at clause level: DCGO runs \
         the unsuspend check even when the opponent already has no sourced Digimon"
    );
}

/// The inherited [Your Turn] clause is a declarative Aura: inherited scope,
/// active_when gate, granting BOTH the Piercing keyword and Security A. +1.
#[test]
fn ex7_021_inherited_aura_shape_piercing_and_security_attack_plus_1() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-021 in compiled cards");

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
        .expect("EX7-021 must encode an inherited declarative Aura");

    let (active_when, security_attack, grant_keyword) = aura;
    assert!(
        active_when.is_some(),
        "the aura must carry an active_when gate ([Your Turn] + opponent stack \
         condition + Ice-Snow carrier-trait)"
    );
    assert_eq!(
        *security_attack,
        Some(1),
        "the aura grants a flat <Security A. +1>"
    );
    assert_eq!(
        grant_keyword.as_ref().map(|g| g.keyword.as_str()),
        Some("Piercing"),
        "the aura grants <Piercing>"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — <Iceclad> battle semantics (stack-count compare, not DP)
// ─────────────────────────────────────────────────────────────────────────────

/// Iceclad attacker wins on source count despite LOWER DP: CrysPaledramon
/// (7000 DP) with 1 source under it (2 cards total) beats a 10000-DP defender
/// with 1 card. Without Iceclad the DP compare (7000 < 10000) would delete it.
#[test]
fn ex7_021_iceclad_wins_battle_on_source_count_despite_lower_dp() {
    let mut runner = base_builder()
        .add_card(junk("SRC-A"))
        .add_card(opp_digimon("BIG-DEF", 10000))
        .start();

    let crys = runner.place_stack(0, &["SRC-A", CARD_ID]);
    let defender = runner.place_on_field(1, "BIG-DEF", Some(0));

    assert_eq!(
        runner.game.players[0].battle_area[crys.index as usize]
            .card_sources
            .len(),
        2,
        "CrysPaledramon stack: 1 source + top = 2 cards"
    );

    let result = runner.attack_digimon(crys, defender, false);

    assert_eq!(
        result,
        AttackResult::AttackerWins,
        "Iceclad compares stack sizes (2 > 1), not DP (7000 < 10000)"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "CrysPaledramon survives via source-count advantage"
    );
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        0,
        "the higher-DP defender is deleted — fewer sources loses under Iceclad"
    );
}

/// Equal source counts under Iceclad = mutual destruction, regardless of the
/// DP disparity (7000 vs 10000 would otherwise be DefenderWins).
#[test]
fn ex7_021_iceclad_equal_sources_mutual_destruction() {
    let mut runner = base_builder()
        .add_card(opp_digimon("BIG-DEF", 10000))
        .start();

    let crys = runner.place_on_field(0, CARD_ID, Some(0));
    let defender = runner.place_on_field(1, "BIG-DEF", Some(0));

    let result = runner.attack_digimon(crys, defender, false);

    assert_eq!(
        result,
        AttackResult::MutualDestruction,
        "Iceclad: equal stack sizes (1 == 1) → both deleted"
    );
    assert_eq!(runner.game.players[0].battle_area.len(), 0);
    assert_eq!(runner.game.players[1].battle_area.len(), 0);
}

/// Iceclad attacker LOSES with fewer sources despite higher DP: bare
/// CrysPaledramon (7000 DP, 1 card) attacks a 1000-DP defender with a 3-card
/// stack. Without Iceclad the DP compare (7000 > 1000) would delete the
/// defender.
#[test]
fn ex7_021_iceclad_loses_with_fewer_sources_despite_higher_dp() {
    let mut runner = base_builder()
        .add_card(junk("SRC-A"))
        .add_card(junk("SRC-B"))
        .add_card(opp_digimon("WEAK-DEF", 1000))
        .start();

    let crys = runner.place_on_field(0, CARD_ID, Some(0));
    let defender = runner.place_stack(1, &["SRC-A", "SRC-B", "WEAK-DEF"]);

    let result = runner.attack_digimon(crys, defender, false);

    assert_eq!(
        result,
        AttackResult::DefenderWins,
        "Iceclad compares stack sizes (1 < 3) — CrysPaledramon loses despite higher DP"
    );
    assert_eq!(runner.game.players[0].battle_area.len(), 0);
    assert_eq!(runner.game.players[1].battle_area.len(), 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — [When Digivolving] trash any 2 sources + conditional unsuspend
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE (cross-permanent + unsuspend): the opponent has TWO Digimon with
/// 1 digivolution card each. The player must pick exactly 2 sources (no
/// PASS) and may split the picks across stacks — one from each. Both stacks
/// shrink, the opponent is left with no sourced Digimon, and the suspended
/// CrysPaledramon unsuspends.
#[test]
fn ex7_021_wd_trashes_one_source_from_each_of_two_digimon_and_unsuspends() {
    let mut runner = wd_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));
    // Suspend the base so the post-trash unsuspend is observable; digivolving
    // preserves the permanent's suspension state.
    runner.game.players[0].battle_area[base.index as usize].is_suspended = true;
    // Two opponent stacks, 1 digivolution card each.
    runner.place_stack(1, &["OPP-A1", "OPP-ATOP"]);
    runner.place_stack(1, &["OPP-B1", "OPP-BTOP"]);

    digivolve_ex7_021(&mut runner, base);
    assert!(
        is_suspended(&runner, base),
        "digivolving must preserve the suspended state (pre-effect)"
    );

    // Mandatory cross-permanent source selection: 2 candidates, no PASS.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti { min: 2, max: 2, picked: 0 }),
        "a mandatory 2-source selection must install"
    );
    let view = runner.pending_selection_view().expect("source prompt");
    assert!(
        !runner.pending_is_optional(),
        "the trash is not 'you may' — mandatory (DCGO canNoTrash=false)"
    );
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "PASS must not be exposed before the minimum is met"
    );
    let pick_a = encode_source_select(0, 0).expect("source (slot 0, idx 0) encodes");
    let pick_b = encode_source_select(1, 0).expect("source (slot 1, idx 0) encodes");
    assert!(
        view.valid_action_ids.contains(&pick_a) && view.valid_action_ids.contains(&pick_b),
        "both opponent stacks' sources are candidates (cross-permanent); got {:?}",
        view.valid_action_ids
    );

    runner
        .execute_action(0, pick_a)
        .expect("pick the source under opponent stack A");
    // Second pick still mandatory — no PASS until min 2 is met.
    let view = runner.pending_selection_view().expect("second source prompt");
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "PASS must not appear after 1 of 2 mandatory picks"
    );
    runner
        .execute_action(0, pick_b)
        .expect("pick the source under opponent stack B");
    runner.auto_resolve().expect("finish When Digivolving");

    // Both stacks shrank to bare tops; both sources are in the trash.
    let trash_ids = zone_ids(&runner.game.players[1].trash, &runner.game.card_data);
    assert!(
        trash_ids.contains(&"OPP-A1".to_string()),
        "stack A's digivolution card is trashed; trash={trash_ids:?}"
    );
    assert!(
        trash_ids.contains(&"OPP-B1".to_string()),
        "stack B's digivolution card is trashed; trash={trash_ids:?}"
    );
    assert_eq!(
        runner.game.players[1].battle_area[0].card_sources.len(),
        1,
        "stack A shrinks 2 -> 1"
    );
    assert_eq!(
        runner.game.players[1].battle_area[1].card_sources.len(),
        1,
        "stack B shrinks 2 -> 1"
    );
    assert_eq!(
        runner.game.players[1].battle_area[0]
            .top_card()
            .card_id(&runner.game.card_data),
        "OPP-ATOP",
        "stack A's top card unchanged"
    );
    assert_eq!(
        runner.game.players[1].battle_area[1]
            .top_card()
            .card_id(&runner.game.card_data),
        "OPP-BTOP",
        "stack B's top card unchanged"
    );

    // Post-trash: no opponent Digimon has digivolution cards → unsuspend.
    assert!(
        !is_suspended(&runner, base),
        "CrysPaledramon must unsuspend when the opponent is left with no sourced Digimon"
    );
}

/// PLAYER CHOICE within one stack: the opponent's only Digimon has 3
/// digivolution cards; the player picks WHICH 2 to trash (here: bottom and
/// middle), the unchosen source and top card stay. A sourced opponent
/// Digimon remains afterwards, so the unsuspend must NOT fire.
#[test]
fn ex7_021_wd_picks_any_two_from_same_stack_and_no_unsuspend_while_sources_remain() {
    let mut runner = wd_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.game.players[0].battle_area[base.index as usize].is_suspended = true;
    // One opponent stack, bottom → top: OPP-A1, OPP-A2, OPP-A3, OPP-ATOP.
    runner.place_stack(1, &["OPP-A1", "OPP-A2", "OPP-A3", "OPP-ATOP"]);

    digivolve_ex7_021(&mut runner, base);

    let view = runner.pending_selection_view().expect("source prompt");
    assert_eq!(
        view.valid_action_ids.len(),
        3,
        "all 3 digivolution cards of the lone stack are candidates"
    );
    let pick_bottom = encode_source_select(0, 0).expect("bottom source encodes");
    let pick_middle = encode_source_select(0, 1).expect("middle source encodes");
    runner
        .execute_action(0, pick_bottom)
        .expect("pick the bottom digivolution card");
    runner
        .execute_action(0, pick_middle)
        .expect("pick the middle digivolution card");
    runner.auto_resolve().expect("finish When Digivolving");

    let trash_ids = zone_ids(&runner.game.players[1].trash, &runner.game.card_data);
    assert!(
        trash_ids.contains(&"OPP-A1".to_string()),
        "the chosen bottom card is trashed; trash={trash_ids:?}"
    );
    assert!(
        trash_ids.contains(&"OPP-A2".to_string()),
        "the chosen middle card is trashed; trash={trash_ids:?}"
    );
    assert!(
        !trash_ids.contains(&"OPP-A3".to_string()),
        "the unchosen digivolution card stays — the PLAYER picks which 2"
    );
    let opp = &runner.game.players[1].battle_area[0];
    assert_eq!(opp.card_sources.len(), 2, "stack shrinks 4 -> 2");
    assert_eq!(
        opp.top_card().card_id(&runner.game.card_data),
        "OPP-ATOP",
        "top card unchanged"
    );

    // OPP-A3 still rides under OPP-ATOP → a sourced opponent Digimon remains
    // → no unsuspend.
    assert!(
        is_suspended(&runner, base),
        "CrysPaledramon must stay suspended while a sourced opponent Digimon remains"
    );
}

/// DCGO up-to semantics: only ONE digivolution card exists in total
/// (maxDigivolutionDiscardCount = min(1, 2) = 1). The pick is still
/// mandatory (no PASS), trashes the lone source, and the opponent being left
/// sourceless unsuspends this Digimon.
#[test]
fn ex7_021_wd_single_available_source_trashes_it_and_unsuspends() {
    let mut runner = wd_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.game.players[0].battle_area[base.index as usize].is_suspended = true;
    // One opponent stack with exactly 1 digivolution card.
    runner.place_stack(1, &["OPP-A1", "OPP-ATOP"]);

    digivolve_ex7_021(&mut runner, base);

    let view = runner.pending_selection_view().expect("source prompt");
    assert!(
        !runner.pending_is_optional(),
        "the single available pick is still mandatory (DCGO canNoTrash=false)"
    );
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "PASS must not be exposed — DCGO trashes min(total, 2) mandatorily"
    );
    assert_eq!(view.valid_action_ids.len(), 1, "exactly one candidate");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick the lone digivolution card");
    runner.auto_resolve().expect("finish When Digivolving");

    let trash_ids = zone_ids(&runner.game.players[1].trash, &runner.game.card_data);
    assert!(
        trash_ids.contains(&"OPP-A1".to_string()),
        "the lone digivolution card is trashed (up-to-2 semantics); trash={trash_ids:?}"
    );
    assert_eq!(
        runner.game.players[1].battle_area[0].card_sources.len(),
        1,
        "stack shrinks 2 -> 1"
    );
    assert!(
        !is_suspended(&runner, base),
        "CrysPaledramon must unsuspend — the opponent has no sourced Digimon left"
    );
}

/// DCGO-verified: the unsuspend check runs even when the opponent ALREADY
/// had no Digimon with digivolution cards — no selection installs (nothing
/// to trash), but the suspended CrysPaledramon still unsuspends.
#[test]
fn ex7_021_wd_unsuspend_fires_when_opponent_already_sourceless() {
    let mut runner = wd_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.game.players[0].battle_area[base.index as usize].is_suspended = true;
    // Bare opponent Digimon — no digivolution cards anywhere.
    runner.place_on_field(1, "OPP-ATOP", Some(0));

    digivolve_ex7_021(&mut runner, base);

    assert!(
        runner.pending_selection().is_none(),
        "no digivolution card exists — no selection may install"
    );
    runner.auto_resolve().ok();

    assert!(
        runner.game.players[1].trash.is_empty(),
        "nothing may be trashed"
    );
    assert!(
        !is_suspended(&runner, base),
        "DCGO runs the unsuspend check unconditionally — it fires with no sourced opp Digimon"
    );
}

/// Same shape with a fully empty opponent board: the unsuspend half still
/// runs.
#[test]
fn ex7_021_wd_unsuspend_fires_with_empty_opponent_board() {
    let mut runner = wd_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.game.players[0].battle_area[base.index as usize].is_suspended = true;

    digivolve_ex7_021(&mut runner, base);

    assert!(runner.pending_selection().is_none());
    runner.auto_resolve().ok();

    assert!(
        !is_suspended(&runner, base),
        "the unsuspend check passes vacuously with an empty opponent board"
    );
}

/// An unsuspended CrysPaledramon stays unsuspended (the unsuspend is a
/// no-op) and the trash half still resolves normally.
#[test]
fn ex7_021_wd_unsuspend_noop_when_already_unsuspended() {
    let mut runner = wd_builder().start();
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.place_stack(1, &["OPP-A1", "OPP-ATOP"]);

    digivolve_ex7_021(&mut runner, base);

    let view = runner.pending_selection_view().expect("source prompt");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick the lone digivolution card");
    runner.auto_resolve().expect("finish When Digivolving");

    assert_eq!(
        runner.game.players[1].battle_area[0].card_sources.len(),
        1,
        "the trash half resolves normally"
    );
    assert!(
        !is_suspended(&runner, base),
        "an already-unsuspended Digimon simply stays unsuspended"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Inherited [Your Turn] aura: Piercing + Security A. +1
// ─────────────────────────────────────────────────────────────────────────────

fn aura_runner() -> DebugRunner {
    base_builder()
        .add_card(ice_snow_carrier())
        .add_card(plain_carrier())
        .add_card(junk("OPP-DEF"))
        .add_card(junk("OPP-SRC"))
        .start()
}

/// POSITIVE: carrier has Ice-Snow, it's the controller's turn, and the
/// opponent's only Digimon has no digivolution cards → the carrier gains
/// <Piercing> and <Security A. +1>.
#[test]
fn ex7_021_aura_on_grants_piercing_and_security_attack_plus_1() {
    let mut r = aura_runner();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let carrier = r.place_stack(tp, &[CARD_ID, "ICE-CARRIER"]);
    r.place_on_field(opp, "OPP-DEF", Some(0));

    r.game.enter_main_phase();
    r.game.set_memory(3);
    r.game.tick_declarative_effects();

    assert!(
        r.game.has_keyword(carrier, Keyword::Piercing),
        "carrier must gain <Piercing> while the gate is satisfied"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        1,
        "carrier must gain <Security A. +1> while the gate is satisfied"
    );
}

/// NEGATIVE (board): an opponent Digimon WITH a digivolution card turns the
/// aura off.
#[test]
fn ex7_021_aura_off_when_opp_digimon_has_digivolution_cards() {
    let mut r = aura_runner();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let carrier = r.place_stack(tp, &[CARD_ID, "ICE-CARRIER"]);
    r.place_stack(opp, &["OPP-SRC", "OPP-DEF"]);

    r.game.enter_main_phase();
    r.game.set_memory(3);
    r.game.tick_declarative_effects();

    assert!(
        !r.game.has_keyword(carrier, Keyword::Piercing),
        "Piercing must be absent while any opponent Digimon has digivolution cards"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        0,
        "Security A. +1 must be absent while any opponent Digimon has digivolution cards"
    );
}

/// NEGATIVE (timing): [Your Turn] — on the opponent's turn the carrier does
/// not get the grants even with the board condition satisfied.
#[test]
fn ex7_021_aura_off_on_opponents_turn() {
    let mut r = aura_runner();
    let tp = r.game.turn_player();
    let non_tp = 1 - tp;

    // Carrier stack belongs to the NON-turn player; the turn player has a
    // single sourceless Digimon (so the board condition holds for non_tp).
    let carrier = r.place_stack(non_tp, &[CARD_ID, "ICE-CARRIER"]);
    r.place_on_field(tp, "OPP-DEF", Some(0));

    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert!(
        !r.game.has_keyword(carrier, Keyword::Piercing),
        "[Your Turn] gate: Piercing must be absent on the opponent's turn"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        0,
        "[Your Turn] gate: Security A. +1 must be absent on the opponent's turn"
    );
}

/// NEGATIVE (trait): "this Digimon with the [Ice-Snow] trait" — a carrier
/// without the Ice-Snow trait does not get the grants.
#[test]
fn ex7_021_aura_off_when_carrier_lacks_ice_snow_trait() {
    let mut r = aura_runner();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let carrier = r.place_stack(tp, &[CARD_ID, "PLAIN-CARRIER"]);
    r.place_on_field(opp, "OPP-DEF", Some(0));

    r.game.enter_main_phase();
    r.game.set_memory(3);
    r.game.tick_declarative_effects();

    assert!(
        !r.game.has_keyword(carrier, Keyword::Piercing),
        "Piercing must be absent when the carrier lacks the Ice-Snow trait"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        0,
        "Security A. +1 must be absent when the carrier lacks the Ice-Snow trait"
    );
}
