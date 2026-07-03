//! BT12-038 GeoGreymon — Digimon, Lv.4, Yellow/Red, DP 5000, Cost 5.
//! Traits: Dinosaur. Form: Champion. Attribute: Vaccine.
//!
//! # Card text (BT12-038.json)
//!
//! `[When Digivolving] If you don't have [Marcus Damon] in play, you may`
//! `play 1 [Marcus Damon] from your hand without paying the cost.`
//!
//! Inherited: `[Your Turn] [Once Per Turn] When one of your yellow or red`
//! `Tamers becomes suspended, 1 of your opponent's Digimon gets -2000 DP for`
//! `the turn.`
//!
//! Digivolve: standard Yellow Lv.3 / cost 3 AND Red Lv.3 / cost 3 (BOTH
//! confirmed by the card image and the official Bandai DB bundle
//! data/card_bundles/BT12-038.md — cards.json evo_costs only carries the
//! Yellow circle, so the Red circle is authored as an explicit alt_path);
//! plus 2 from Lv.3 w/[Agumon] in name and [Dinosaur] trait (alt-path;
//! `xros_req`).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT12/Yellow/BT12_038.cs
//!   - `EffectTiming.None`: `AddSelfDigivolutionRequirementStaticEffect`
//!     (PermanentCondition: TopCard contains "Agumon" in name AND has
//!     [Dinosaur] trait AND Level == 3; digivolutionCost: 2).
//!   - `OnEnterFieldAnyone` (own, [When Digivolving]): `ActivateClass` with
//!     `CanActivateCondition` = IsExistOnBattleArea(card) && hand count >= 1
//!     && zero battle-area permanents whose TopCard.CardNames contains
//!     "Marcus Damon". `SelectHandEffect` (canNoSelect: true) filtered to
//!     hand cards named "Marcus Damon" belonging to the owner and playable
//!     as a new permanent → `PlayPermanentCards(payCost: false)`.
//!   - `OnTappedAnyone` (inherited): `ActivateClass`, `SetIsInheritedEffect`,
//!     `maxUsableCount: 1`. `CanUseCondition` = IsExistOnBattleArea(card) &&
//!     IsOwnerTurn(card) && the suspending permanent is on the OWNER's
//!     battle area, `IsTamer`, and its colors contain Red OR Yellow.
//!     `CanActivateCondition` = IsExistOnBattleArea(card) (unconditional —
//!     the mandatory select itself no-ops with zero opponent Digimon via
//!     `HasMatchConditionPermanent`). Body: mandatory
//!     (`canNoSelect: false`) select 1 opponent battle-area Digimon →
//!     `ChangeDigimonDP(-2000, UntilEachTurnEnd)`.
//!
//! # Patterns this test file covers (test API §4.3)
//! - When-Digivolving optional free-play of a named Tamer from hand, gated
//!   by a `no_permanent` existential ("if you don't have X in play") —
//!   twin shape to `BT13-020` clause 1 (`select_hand` optional +
//!   `play_from_hand_free`) plus the `no_permanent` clause-level `condition`
//!   idiom from `BT22-088` / `BT24-082`.
//! - B3-adjacent: inherited observer of own red/yellow Tamer suspend
//!   (`OnSuspend`, `[Your Turn][OPT]`) — twin shape to `BT21-004`
//!   (own yellow/red Tamer suspend, mandatory body, OPT).
//! - Mandatory opponent-Digimon target selection + `add_dp_modifier`
//!   (`-2000`, `end_of_turn`) — twin shape to `ST3-08` / `AD1-016` clause 3.
//! - E2 OPT lockout + clear-next-turn.
//! - Alt-path digivolve condition: Lv.3 w/"Agumon" in name AND [Dinosaur]
//!   trait, cost 2.
//! - Off-color standard-circle alt-path: Red Lv.3 / cost 3, authored to
//!   compensate for cards.json's lossy multi-colour-digivolve-cost drop
//!   (idiom: BT13-019 / BT13-030 / BT13-040). Behaviorally verified via
//!   `digivolve_from_hand` (Section 1b) — this is the only standard circle
//!   exercisable in the `DebugRunner` harness (G-DSL-FIXTURE-EVO-COSTS: DSL
//!   fixtures never see cards.json, so the Yellow circle can only be
//!   verified structurally/against the JSON, not behaviorally here).

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlayerId, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::PendingSelection;
use digimon_engine::{EffectTiming, TriggerSource};

// ─── helpers ──────────────────────────────────────────────────────────────

fn geogreymon() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("BT12-038")
}

fn make_tamer(card_id: &str, name: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![color];
    card.dp = None;
    card.level = None;
    card
}

fn make_marcus(card_id: &str, color: CardColor) -> CardData {
    // A Tamer named "Marcus Damon" — matches the DCGO CardNames.Contains
    // check and the DSL's `name_contains: "Marcus Damon"` filter.
    make_tamer(card_id, "Marcus Damon", color)
}

fn make_digimon(card_id: &str, dp: i32, color: CardColor) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.dp = Some(dp);
    card.level = Some(4);
    card
}

/// A plain Lv.3 Digimon base of the given color, for exercising the standard
/// digivolve circles (Yellow Lv.3/3 from evo_costs, Red Lv.3/3 from the
/// explicit alt_path added to compensate for cards.json's dropped circle).
fn make_lv3_base(card_id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.dp = Some(3000);
    card.level = Some(3);
    card
}

fn find_hand_index(runner: &DebugRunner, player: PlayerId, card_id: &str) -> Option<usize> {
    let data = &runner.game.card_data;
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(data) == card_id)
}

fn player_of(p: &PendingSelection) -> PlayerId {
    p.selecting_player
}

fn first_action(p: &PendingSelection) -> u16 {
    *p.valid_action_ids
        .first()
        .expect("at least one legal action")
}

fn perm_with_card_id(
    runner: &DebugRunner,
    player: PlayerId,
    card_id: &str,
) -> Option<PermanentHandle> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == card_id)
        .map(|i| PermanentHandle {
            player,
            index: i as u8,
        })
}

/// Place BT12-038 on the field (top card) and fire its own [When Digivolving]
/// clause, then drain the queue.
fn fire_when_digivolving(runner: &mut DebugRunner) -> PermanentHandle {
    let carrier = runner.place_on_field(0, "BT12-038", Some(0));
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();
    carrier
}

// ─────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn bt12_038_metadata_matches_printed() {
    let card = geogreymon();
    assert_eq!(card.card, "BT12-038");
    assert_eq!(card.name, "GeoGreymon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(5000));
    assert_eq!(card.cost, Some(5));

    let colors: Vec<_> = card.color.iter().copied().collect();
    assert!(
        colors.contains(&digimon_dsl::compiled::CompiledColor::Yellow),
        "must be Yellow"
    );
    assert!(
        colors.contains(&digimon_dsl::compiled::CompiledColor::Red),
        "must be Red"
    );
    assert!(
        card.traits.iter().any(|t| t.eq_ignore_ascii_case("Dinosaur")),
        "must have Dinosaur trait; got {:?}",
        card.traits
    );
}

#[test]
fn bt12_038_has_two_triggered_clauses() {
    let card = geogreymon();
    let triggered: Vec<_> = card
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
        "[When Digivolving] + inherited [Your Turn][OPT] on_suspend; got {:?}",
        triggered
            .iter()
            .map(|t| (t.scope, t.when.clone()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn bt12_038_clause1_is_own_when_digivolving() {
    let card = geogreymon();
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving) => {
                Some(t)
            }
            _ => None,
        })
        .expect("a [When Digivolving] clause must exist");
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "clause 1 is a own-card face-up effect, NOT inherited"
    );
}

#[test]
fn bt12_038_clause2_is_inherited_on_suspend_opt() {
    let card = geogreymon();
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSuspend) => {
                Some(t)
            }
            _ => None,
        })
        .expect("an OnSuspend clause must exist");
    assert_eq!(clause.scope, CompiledScope::Inherited, "must be inherited");
    assert!(clause.once_per_turn, "[Once Per Turn] flag must be set");
    assert!(
        !clause.optional,
        "the -2000 DP effect is mandatory once triggered — no 'may'"
    );
}

#[test]
fn bt12_038_has_alt_digivolve_lv3_agumon_dinosaur_cost2() {
    let card = geogreymon();
    let found = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(2))
    });
    assert!(
        found,
        "must declare a cost-2 Digivolve alt-path (Lv.3 w/[Agumon] in name \
         and [Dinosaur] trait); alt_paths={:?}",
        card.alt_paths
    );
}

#[test]
fn bt12_038_has_alt_digivolve_lv3_red_standard_circle_cost3() {
    // The printed Red Lv.3 / cost 3 standard digivolve circle (confirmed by
    // the card image and the official Bandai DB bundle) is dropped from
    // cards.json evo_costs (only the Yellow Lv.3/3 circle survives the
    // lossy multi-colour-digivolve-cost API ingest). This alt-path restores
    // it explicitly — mirrors the BT13-019 / BT13-030 / BT13-040 idiom.
    let card = geogreymon();
    let found = card.alt_paths.iter().any(|p| {
        let filter_debug = format!("{:?}", p.from);
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(3))
            && filter_debug.contains("level_eq: Some(3)")
            && filter_debug.contains("Red")
    });
    assert!(
        found,
        "must declare a cost-3 Digivolve alt-path gated on Lv.3 Red (the \
         standard Red circle dropped from cards.json evo_costs); alt_paths={:?}",
        card.alt_paths
    );
}

// ─────────────────────────────────────────────────────────────────────────
// SECTION 1b — Standard Red digivolve circle is actually playable in-engine
//
// NOTE: DSL-loaded `DebugRunner` fixtures never see `data/cards.json`
// (G-DSL-FIXTURE-EVO-COSTS, code/digimon-engine/src/debug_runner.rs) — the
// engine's evo-cost table for a `.dsl_card(...)` fixture is backfilled
// SOLELY from `kind: digivolve` alt-paths whose `from` carries a literal
// `level_eq` + `color_is`. The printed Yellow Lv.3/3 circle (loaded from
// cards.json evo_costs in production) is therefore NOT exercisable through
// this harness at all — there is no fixture-level way to prove it's wired,
// so this section only behaviorally verifies the Red circle, which is
// backfilled from the alt_path this fix adds and is the actual regression
// guard for the reported gap. The Yellow circle's presence in evo_costs is
// production data (data/cards.json), verified directly against the JSON
// and the card image/bundle in the YAML header comment.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn bt12_038_digivolves_from_red_lv3_base_for_cost_3() {
    // The Red Lv.3/3 circle is the one dropped by cards.json evo_costs and
    // restored via the explicit alt_path in this fix — this is the
    // regression guard for that gap. (Backfilled into the DSL fixture's
    // evo_costs per G-DSL-FIXTURE-EVO-COSTS, see the section note above.)
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-038")
        .expect("BT12-038 in pack")
        .add_card(make_lv3_base("RED-BASE", CardColor::Red))
        .hand(0, &["BT12-038"])
        .deck(0, &["RED-BASE"]) // rule draw on digivolve needs a non-empty deck
        .memory(10)
        .start();
    let base = runner.place_on_field(0, "RED-BASE", Some(0));

    let hand_idx = find_hand_index(&runner, 0, "BT12-038").expect("BT12-038 in hand");
    let mem_before = runner.memory();

    let ok =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByDigivolve);
    assert!(
        ok,
        "BT12-038 must digivolve from a Red Lv.3 base (standard circle restored via the \
         explicit alt_path, compensating for the circle cards.json evo_costs dropped)"
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BT12-038",
        "the base stack's top card must become BT12-038"
    );
    assert_eq!(
        mem_before - runner.memory(),
        3,
        "the standard Red Lv.3 circle costs 3 memory"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// SECTION 2 — Clause 1 [When Digivolving]: no_permanent-gated free Marcus play
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn bt12_038_when_digivolving_installs_optional_marcus_play_when_none_in_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-038")
        .expect("BT12-038 in pack")
        .add_card(make_marcus("MARCUS", CardColor::Yellow))
        .hand(0, &["MARCUS"])
        .memory(10)
        .start();

    fire_when_digivolving(&mut runner);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("the [When Digivolving] play prompt must install");
    assert!(
        pending.is_optional,
        "'you may play' is optional — PASS must be legal"
    );
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "exactly the Marcus Damon in hand is selectable; got {:?}",
        pending.valid_action_ids
    );
}

#[test]
fn bt12_038_when_digivolving_no_marcus_in_hand_installs_no_prompt() {
    // A non-Marcus Tamer in hand must NOT be offered (name filter).
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-038")
        .expect("BT12-038 in pack")
        .add_card(make_tamer("OTHER-TAMER", "Some Other Tamer", CardColor::Yellow))
        .hand(0, &["OTHER-TAMER"])
        .memory(10)
        .start();

    fire_when_digivolving(&mut runner);

    assert!(
        runner.game.pending_selection.is_none(),
        "no Marcus Damon in hand → no play prompt installs; got {:?}",
        runner.game.pending_selection
    );
}

#[test]
fn bt12_038_when_digivolving_no_prompt_when_marcus_already_in_play() {
    // The `no_permanent` gate: if a Marcus Damon is ALREADY on the battle
    // area, the whole clause must not activate — no prompt at all (matches
    // DCGO's CanActivateCondition battle-area count == 0 check, evaluated
    // BEFORE the SelectHandEffect is even constructed).
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-038")
        .expect("BT12-038 in pack")
        .add_card(make_marcus("MARCUS-FIELD", CardColor::Yellow))
        .add_card(make_marcus("MARCUS-HAND", CardColor::Red))
        .hand(0, &["MARCUS-HAND"])
        .memory(10)
        .start();
    runner.place_on_field(0, "MARCUS-FIELD", Some(0));

    fire_when_digivolving(&mut runner);

    assert!(
        runner.game.pending_selection.is_none(),
        "a Marcus Damon already in play must block the free-play prompt entirely; got {:?}",
        runner.game.pending_selection
    );
}

#[test]
fn bt12_038_when_digivolving_plays_marcus_free_and_reduces_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-038")
        .expect("BT12-038 in pack")
        .add_card(make_marcus("MARCUS", CardColor::Yellow))
        .hand(0, &["MARCUS"])
        .memory(10)
        .start();

    let battle_before = runner.battle_area_size(0);
    let hand_before = runner.game.players[0].hand.len();
    fire_when_digivolving(&mut runner);

    let (player, action_id) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("play prompt installed");
        (player_of(pending), first_action(pending))
    };
    runner
        .execute_action(player, action_id)
        .expect("Marcus play resolves");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        battle_before + 2,
        "carrier + played Marcus on field"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before - 1,
        "Marcus leaves the hand"
    );
    assert!(
        perm_with_card_id(&runner, 0, "MARCUS").is_some(),
        "Marcus Damon must be on the field"
    );
}

#[test]
fn bt12_038_when_digivolving_decline_plays_no_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-038")
        .expect("BT12-038 in pack")
        .add_card(make_marcus("MARCUS", CardColor::Yellow))
        .hand(0, &["MARCUS"])
        .memory(10)
        .start();

    let battle_before = runner.battle_area_size(0);
    let hand_before = runner.game.players[0].hand.len();
    fire_when_digivolving(&mut runner);

    let player = player_of(
        runner
            .game
            .pending_selection
            .as_ref()
            .expect("play prompt installed"),
    );
    runner
        .execute_action(player, PASS)
        .expect("decline resolves");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        battle_before + 1,
        "declining must not play Marcus (only the carrier is on field)"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "Marcus stays in hand on decline"
    );
    assert!(
        perm_with_card_id(&runner, 0, "MARCUS").is_none(),
        "no Marcus on field when the play was declined"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// SECTION 3 — Clause 2 [Your Turn][OPT] inherited: own red/yellow Tamer
// suspend → 1 opponent Digimon gets -2000 DP for the turn
// ─────────────────────────────────────────────────────────────────────────

/// Runner with BT12-038 as a digivolution source under a host Digimon, own
/// red/yellow/green Tamers, an opponent Digimon target, and stocked decks
/// (so turn rotation does not deck-out during OPT-clear tests).
fn suspend_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT12-038")
        .expect("BT12-038 in pack")
        .add_card(make_digimon("HOST", 4000, CardColor::Yellow))
        .add_card(make_tamer("RED-TAMER", "Red Tamer", CardColor::Red))
        .add_card(make_tamer("YEL-TAMER", "Yellow Tamer", CardColor::Yellow))
        .add_card(make_tamer("GRN-TAMER", "Green Tamer", CardColor::Green))
        .add_card(make_digimon("OPP1", 6000, CardColor::Blue))
        .add_card(make_digimon("OPP2", 3000, CardColor::Blue))
        .add_card(make_digimon("FILLER", 1000, CardColor::Blue))
        .deck(0, &["FILLER"; 20])
        .deck(1, &["FILLER"; 20])
        .memory(10)
        .start()
}

/// Place a host Digimon for `player` with BT12-038 underneath as a source.
fn host_with_geogreymon(runner: &mut DebugRunner, player: PlayerId) -> PermanentHandle {
    runner.place_stack(player, &["BT12-038", "HOST"])
}

#[test]
fn bt12_038_installs_mandatory_target_select_when_own_yellow_tamer_suspends() {
    let mut runner = suspend_runner();
    host_with_geogreymon(&mut runner, 0);
    let tamer = runner.place_on_field(0, "YEL-TAMER", Some(0));
    runner.place_on_field(1, "OPP1", Some(0));

    runner.game.suspend(tamer);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("own yellow Tamer suspend must install the mandatory DP-target prompt");
    assert!(
        !pending.is_optional,
        "the -2000 DP effect is mandatory — no PASS"
    );
}

#[test]
fn bt12_038_own_yellow_tamer_suspend_applies_minus_2000_dp_to_chosen_opponent() {
    let mut runner = suspend_runner();
    host_with_geogreymon(&mut runner, 0);
    let tamer = runner.place_on_field(0, "YEL-TAMER", Some(0));
    runner.place_on_field(1, "OPP1", Some(0));

    runner.game.suspend(tamer);

    let (player, action_id) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("DP-target prompt installed");
        (player_of(pending), first_action(pending))
    };
    runner
        .execute_action(player, action_id)
        .expect("target selection resolves");
    let _ = runner.auto_resolve();

    let opp1 = perm_with_card_id(&runner, 1, "OPP1").expect("OPP1 on field");
    assert_eq!(
        runner.effective_dp(opp1),
        Some(6000 - 2000),
        "chosen opponent Digimon must get -2000 DP for the turn"
    );
}

#[test]
fn bt12_038_own_red_tamer_suspend_also_triggers() {
    let mut runner = suspend_runner();
    host_with_geogreymon(&mut runner, 0);
    let tamer = runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.place_on_field(1, "OPP1", Some(0));

    runner.game.suspend(tamer);
    assert!(
        runner.game.pending_selection.is_some(),
        "a red Tamer suspending must also satisfy 'yellow or red'"
    );

    let (player, action_id) = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        (player_of(pending), first_action(pending))
    };
    runner.execute_action(player, action_id).unwrap();
    let _ = runner.auto_resolve();

    let opp1 = perm_with_card_id(&runner, 1, "OPP1").expect("OPP1 on field");
    assert_eq!(runner.effective_dp(opp1), Some(6000 - 2000));
}

#[test]
fn bt12_038_dp_penalty_reverts_at_end_of_turn() {
    let mut runner = suspend_runner();
    host_with_geogreymon(&mut runner, 0);
    let tamer = runner.place_on_field(0, "YEL-TAMER", Some(0));
    runner.place_on_field(1, "OPP1", Some(0));

    runner.game.suspend(tamer);
    let (player, action_id) = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        (player_of(pending), first_action(pending))
    };
    runner.execute_action(player, action_id).unwrap();
    let _ = runner.auto_resolve();

    let opp1 = perm_with_card_id(&runner, 1, "OPP1").expect("OPP1 on field");
    assert_eq!(runner.effective_dp(opp1), Some(6000 - 2000));

    runner.end_turn();
    let _ = runner.auto_resolve();

    let opp1_after = perm_with_card_id(&runner, 1, "OPP1").expect("OPP1 still on field");
    assert_eq!(
        runner.effective_dp(opp1_after),
        Some(6000),
        "the -2000 DP modifier must expire at end of turn"
    );
}

// ─── SECTION 3b — Negatives ─────────────────────────────────────────────────

#[test]
fn bt12_038_does_not_fire_on_green_tamer_suspend() {
    let mut runner = suspend_runner();
    host_with_geogreymon(&mut runner, 0);
    let tamer = runner.place_on_field(0, "GRN-TAMER", Some(0));
    runner.place_on_field(1, "OPP1", Some(0));

    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();

    let opp1 = perm_with_card_id(&runner, 1, "OPP1").expect("OPP1 on field");
    assert_eq!(
        runner.effective_dp(opp1),
        Some(6000),
        "a green Tamer is not yellow/red → no DP penalty"
    );
}

#[test]
fn bt12_038_does_not_fire_on_own_digimon_suspend() {
    let mut runner = suspend_runner();
    host_with_geogreymon(&mut runner, 0);
    let ally = runner.place_on_field(0, "FILLER", Some(0));
    runner.place_on_field(1, "OPP1", Some(0));

    runner.game.suspend(ally);
    let _ = runner.auto_resolve();

    let opp1 = perm_with_card_id(&runner, 1, "OPP1").expect("OPP1 on field");
    assert_eq!(
        runner.effective_dp(opp1),
        Some(6000),
        "a Digimon (not a Tamer) suspending must NOT trigger the clause"
    );
}

#[test]
fn bt12_038_does_not_fire_on_opponent_tamer_suspend() {
    let mut runner = suspend_runner();
    host_with_geogreymon(&mut runner, 0);
    let opp_tamer = runner.place_on_field(1, "YEL-TAMER", Some(0));
    runner.place_on_field(1, "OPP1", Some(0));

    runner.game.suspend(opp_tamer);
    let _ = runner.auto_resolve();

    let opp1 = perm_with_card_id(&runner, 1, "OPP1").expect("OPP1 on field");
    assert_eq!(
        runner.effective_dp(opp1),
        Some(6000),
        "the OPPONENT's Tamer suspending does not satisfy 'your' Tamers"
    );
}

#[test]
fn bt12_038_does_not_fire_on_opponents_turn() {
    let mut runner = suspend_runner();
    host_with_geogreymon(&mut runner, 0);
    let tamer = runner.place_on_field(0, "YEL-TAMER", Some(0));
    runner.place_on_field(1, "OPP1", Some(0));
    runner.game.turn_player_idx = 1; // opponent's turn

    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();

    let opp1 = perm_with_card_id(&runner, 1, "OPP1").expect("OPP1 on field");
    assert_eq!(
        runner.effective_dp(opp1),
        Some(6000),
        "[Your Turn] gate blocks the effect on the opponent's turn"
    );
}

#[test]
fn bt12_038_no_prompt_when_opponent_has_no_digimon() {
    // The mandatory select has zero candidates → the clause silently no-ops
    // (matches DCGO's HasMatchConditionPermanent guard).
    let mut runner = suspend_runner();
    host_with_geogreymon(&mut runner, 0);
    let tamer = runner.place_on_field(0, "YEL-TAMER", Some(0));
    // No opponent Digimon placed.

    runner.game.suspend(tamer);
    let resolved = runner.auto_resolve();
    assert!(
        resolved.is_ok(),
        "an empty mandatory select must not stall the queue"
    );
    assert!(
        runner.game.pending_selection.is_none(),
        "no legal target → no lingering prompt"
    );
}

// ─── SECTION 4 — OPT lockout + clear ────────────────────────────────────────

#[test]
fn bt12_038_opt_blocks_second_suspend_same_turn() {
    let mut runner = suspend_runner();
    host_with_geogreymon(&mut runner, 0);
    let t1 = runner.place_on_field(0, "YEL-TAMER", Some(0));
    let t2 = runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.place_on_field(1, "OPP1", Some(0));
    runner.place_on_field(1, "OPP2", Some(0));

    runner.game.suspend(t1);
    let (player, action_id) = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        (player_of(pending), first_action(pending))
    };
    runner.execute_action(player, action_id).unwrap();
    let _ = runner.auto_resolve();

    // First trigger applied -2000 to whichever target was selected. Confirm
    // exactly one of the two opponent Digimon was hit.
    let opp1 = perm_with_card_id(&runner, 1, "OPP1").expect("OPP1 on field");
    let opp2 = perm_with_card_id(&runner, 1, "OPP2").expect("OPP2 on field");
    let hit_count = [runner.effective_dp(opp1), runner.effective_dp(opp2)]
        .into_iter()
        .filter(|dp| *dp == Some(6000 - 2000) || *dp == Some(3000 - 2000))
        .count();
    assert_eq!(hit_count, 1, "exactly one opponent Digimon debuffed once");

    runner.game.suspend(t2);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.pending_selection.is_none(),
        "[Once Per Turn] must block a second suspend-trigger the same turn"
    );
    // No additional Digimon should have been debuffed by the second suspend.
    let opp1_dp = runner.effective_dp(opp1);
    let opp2_dp = runner.effective_dp(opp2);
    let still_hit_count = [opp1_dp, opp2_dp]
        .into_iter()
        .filter(|dp| *dp == Some(6000 - 2000) || *dp == Some(3000 - 2000))
        .count();
    assert_eq!(
        still_hit_count, 1,
        "OPT must prevent a second Digimon from being debuffed the same turn"
    );
}

#[test]
fn bt12_038_opt_clears_next_turn() {
    let mut runner = suspend_runner();
    host_with_geogreymon(&mut runner, 0);
    let tamer = runner.place_on_field(0, "YEL-TAMER", Some(0));
    runner.place_on_field(1, "OPP1", Some(0));

    runner.game.suspend(tamer);
    let (player, action_id) = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        (player_of(pending), first_action(pending))
    };
    runner.execute_action(player, action_id).unwrap();
    let _ = runner.auto_resolve();

    let opp1 = perm_with_card_id(&runner, 1, "OPP1").expect("OPP1 on field");
    assert_eq!(runner.effective_dp(opp1), Some(6000 - 2000));

    // Cycle turns until it's P0's turn again — the DP modifier reverts at
    // end of turn, so re-apply fresh from a full-DP baseline.
    for _ in 0..6 {
        runner.end_turn();
        let _ = runner.auto_resolve();
        if runner.game.turn_player() == 0 {
            break;
        }
    }
    assert_eq!(runner.game.turn_player(), 0, "back to P0's turn");

    let opp1_reset = perm_with_card_id(&runner, 1, "OPP1").expect("OPP1 still on field");
    assert_eq!(
        runner.effective_dp(opp1_reset),
        Some(6000),
        "DP modifier must have expired by the new turn"
    );

    let tamer_again =
        perm_with_card_id(&runner, 0, "YEL-TAMER").expect("yellow Tamer still on field");
    runner.game.suspend(tamer_again);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("OPT lockout must clear on the new turn");
    let (player2, action_id2) = (player_of(pending), first_action(pending));
    runner.execute_action(player2, action_id2).unwrap();
    let _ = runner.auto_resolve();

    let opp1_final = perm_with_card_id(&runner, 1, "OPP1").expect("OPP1 still on field");
    assert_eq!(
        runner.effective_dp(opp1_final),
        Some(6000 - 2000),
        "OPT lockout clears on the new turn — the debuff re-applies"
    );
}
