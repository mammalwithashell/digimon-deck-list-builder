//! BT24-061 Vademon — Digimon, Lv.5, Black, DP 7000, Play cost 6.
//! Form: Ultimate. Attribute: Virus. Traits: [Alien], [Iliad], [TS].
//!
//! # Card text (code/digimon-engine/cards/bt24/BT24-061.json — verbatim;
//! cross-checked against data/card_bundles/BT24-061.md and the card image)
//!
//! Digivolve: Lv.4 black circle, cost 3.
//! [Digivolve] Lv.4 w/[TS] trait: Cost 3
//!
//! [On Play] [When Digivolving] Return 1 of your opponent's play cost 3 or
//!   lower Digimon or Tamers to the top of the deck.
//!
//! Inherited Effect:
//! [When Attacking] [Once Per Turn] ＜De-Digivolve 1＞ 1 of your opponent's
//!   Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Black/BT24_061.cs
//!   - Alt digivolve: AddSelfDigivolutionRequirementStaticEffect,
//!     `TopCard.IsLevel4 && TopCard.HasTSTraits`, cost 3.
//!   - Shared OP/WD: SelectPermanentEffect(opponent battle area, IsDigimon ||
//!     IsTamer, HasPlayCost && GetCostItself <= 3), maxCount 1,
//!     canNoSelect: false (mandatory), Mode.PutLibraryTop.
//!   - Inherited [When Attacking] OPT (hash "BT24_061_Inherited"):
//!     SelectPermanent(opponent Digimon), canNoSelect: false →
//!     IDegeneration(selected, 1).
//!
//! # Rules
//! - §4-7-8: a Digimon/Tamer removed from the battle area takes its stacked
//!   cards to the trash (only the top card goes to the deck — the printed text
//!   has no "and all of its digivolution cards").
//! - §16-12: <De-Digivolve> is mandatory and can't trash Lv.3 or lower cards.
//!
//! # Patterns covered (RUST_DSL_TEST_API.md §4.3: G4)
//! - Standard + special alt-digivolve (Lv.4 black / Lv.4 [TS], both cost 3)
//! - Shared [On Play]/[When Digivolving] mandatory opponent Digimon-or-Tamer
//!   pick filtered by play cost, returned to the top of its owner's deck
//! - Inherited [When Attacking][OPT] mandatory De-Digivolve 1 on an opponent
//!   Digimon, driven through a real attack, with OPT lockout + turn reset

use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledStackPosition, CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_attack, encode_digivolve, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};
use digimon_engine::selection::SelectionKind;
use digimon_engine::PermanentHandle;

const CARD_ID: &str = "BT24-061";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, level: u8, play_cost: u16, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.play_cost = play_cost;
    c.dp = Some(4000);
    c.colors = vec![color];
    c.traits = traits.iter().map(|s| s.to_string()).collect();
    c
}

fn tamer(id: &str, play_cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = play_cost;
    c.colors = vec![CardColor::Red];
    c
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn vademon_builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-061 YAML loads")
        .add_card(filler("FILL"))
        .add_card(filler("SEC"))
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .security(1, &["SEC", "SEC", "SEC"])
}

fn main_phase(runner: &mut DebugRunner) {
    runner.game.turn_count = 1;
    runner.game.current_phase = GamePhase::Main;
}

/// OppField selections encode the chosen opponent slot as `encode_attack(0, slot)`.
fn target_action(h: PermanentHandle) -> u16 {
    encode_attack(0, h.index as u16)
}

fn handle_of(runner: &DebugRunner, player: u8, top_id: &str) -> PermanentHandle {
    let idx = runner.game.players[player as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == top_id)
        .unwrap_or_else(|| panic!("{top_id} on player {player}'s field"));
    PermanentHandle {
        player,
        index: idx as u8,
    }
}

fn on_field(runner: &DebugRunner, player: usize, top_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == top_id)
}

fn deck_top_id(runner: &DebugRunner, player: usize) -> String {
    runner.game.players[player]
        .deck
        .last()
        .expect("deck not empty")
        .card_id(&runner.game.card_data)
        .to_string()
}

fn trash_has(runner: &DebugRunner, player: usize, id: &str) -> bool {
    runner.game.players[player]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == id)
}

fn shared_clause(
    runner: &DebugRunner,
) -> digimon_dsl::compiled::CompiledTriggeredClause {
    let compiled = runner.compiled_card(CARD_ID).expect("present");
    compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t.clone())
            }
            _ => None,
        })
        .expect("shared On Play / When Digivolving clause")
}

fn inherited_clause(
    runner: &DebugRunner,
) -> digimon_dsl::compiled::CompiledTriggeredClause {
    let compiled = runner.compiled_card(CARD_ID).expect("present");
    compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t.clone())
            }
            _ => None,
        })
        .expect("inherited When Attacking clause")
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt24_061_yaml_compiles_with_printed_metadata() {
    let runner = vademon_builder().start();
    let compiled = runner.compiled_card(CARD_ID).expect("BT24-061 in pack");
    assert_eq!(compiled.card, CARD_ID);
    assert_eq!(compiled.level, Some(5));
    assert_eq!(compiled.cost, Some(6));
    assert_eq!(compiled.dp, Some(7000));
    for t in ["Alien", "Iliad", "TS"] {
        assert!(compiled.traits.iter().any(|x| x == t), "missing trait {t}");
    }
}

/// The shared OP/WD clause is mandatory (no "you may"), not once-per-turn, and
/// returns the pick to the TOP of the deck WITHOUT its digivolution cards.
#[test]
fn bt24_061_shared_clause_is_mandatory_top_of_deck_return() {
    let runner = vademon_builder().start();
    let clause = shared_clause(&runner);
    assert!(!clause.optional, "printed text has no 'you may'");
    assert!(!clause.once_per_turn, "OP/WD clause is not OPT");
    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(
        clause.process.iter().any(|s| matches!(
            s,
            CompiledStep::ReturnToDeck {
                position: CompiledStackPosition::Top,
                include_sources: false,
                ..
            }
        )),
        "must return the chosen card to the top of the deck (sources trashed, §4-7-8)"
    );
}

/// The inherited clause is [When Attacking][Once Per Turn], mandatory, and
/// De-Digivolves 1 (stopping at Lv.3, §16-12-4).
#[test]
fn bt24_061_inherited_clause_is_opt_mandatory_de_digivolve_1() {
    let runner = vademon_builder().start();
    let clause = inherited_clause(&runner);
    assert!(clause.once_per_turn, "[Once Per Turn]");
    assert!(!clause.optional, "<De-Digivolve> is mandatory (§16-12-3)");
    assert!(
        clause.process.iter().any(|s| matches!(
            s,
            CompiledStep::DeDigivolve {
                amount: Some(1),
                stop_at_level: Some(3),
                ..
            }
        )),
        "inherited clause must De-Digivolve 1"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Digivolution requirements
// ═══════════════════════════════════════════════════════════════════════════════

fn digivolve_runner(base: CardData) -> DebugRunner {
    let id = base.card_id.clone();
    let mut r = vademon_builder()
        .add_card(base)
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    main_phase(&mut r);
    r.place_on_field(0, &id, Some(0));
    r
}

/// Special requirement: a NON-black Lv.4 with the [TS] trait digivolves into
/// Vademon for 3.
#[test]
fn bt24_061_digivolves_from_non_black_ts_level4_for_3() {
    let mut runner = digivolve_runner(digimon("TS-LV4", 4, 4, CardColor::Red, &["TS"]));
    let mask = build_action_mask(&runner.game, 0);
    assert!(
        mask[encode_digivolve(0, 0) as usize] > 0.0,
        "Lv.4 w/[TS] route must be offered"
    );
    let mem = runner.game.memory;
    assert!(runner.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand));
    assert_eq!(top_id(&runner, 0, 0), CARD_ID);
    assert_eq!(mem - runner.game.memory, 3, "the [TS] route costs 3");
}

/// Standard requirement: a black Lv.4 (no [TS]) digivolves into Vademon for 3.
#[test]
fn bt24_061_digivolves_from_black_level4_for_3() {
    let mut runner = digivolve_runner(digimon("BLK-LV4", 4, 4, CardColor::Black, &[]));
    let mem = runner.game.memory;
    assert!(runner.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand));
    assert_eq!(top_id(&runner, 0, 0), CARD_ID);
    assert_eq!(mem - runner.game.memory, 3, "the black Lv.4 circle costs 3");
}

/// Negative: a non-black Lv.4 without [TS] has no route.
#[test]
fn bt24_061_not_offered_from_non_black_non_ts_level4() {
    let runner = digivolve_runner(digimon("RED-LV4", 4, 4, CardColor::Red, &["Dragon"]));
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[encode_digivolve(0, 0) as usize],
        0.0,
        "neither the black circle nor the [TS] condition is met"
    );
}

/// Negative: a [TS] Digimon that is NOT Lv.4 has no route.
#[test]
fn bt24_061_not_offered_from_ts_level3() {
    let runner = digivolve_runner(digimon("TS-LV3", 3, 3, CardColor::Red, &["TS"]));
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[encode_digivolve(0, 0) as usize],
        0.0,
        "the [TS] route requires a Lv.4"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [On Play] / [When Digivolving] return to top of deck
// ═══════════════════════════════════════════════════════════════════════════════

/// Board: opponent has a cost-3 Digimon stacked on a source, a cost-3 Tamer, a
/// cost-4 Digimon; you have your own cost-2 Digimon.
fn op_runner() -> DebugRunner {
    let mut r = vademon_builder()
        .add_card(digimon("OPP-SRC", 3, 2, CardColor::Blue, &[]))
        .add_card(digimon("OPP-C3", 4, 3, CardColor::Blue, &[]))
        .add_card(digimon("OPP-C4", 4, 4, CardColor::Blue, &[]))
        .add_card(tamer("OPP-TAMER", 3))
        .add_card(digimon("OWN-C2", 3, 2, CardColor::Black, &[]))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    main_phase(&mut r);
    r.place_stack(1, &["OPP-SRC", "OPP-C3"]);
    r.place_on_field(1, "OPP-C4", Some(0));
    r.place_on_field(1, "OPP-TAMER", None);
    r.place_on_field(0, "OWN-C2", Some(0));
    r
}

/// Playing Vademon installs a MANDATORY opponent-field pick whose legal set is
/// exactly the opponent's play-cost<=3 Digimon and Tamers (not the cost-4
/// Digimon, not your own cost-2 Digimon).
#[test]
fn bt24_061_on_play_offers_only_opponent_cost3_or_lower_digimon_and_tamers() {
    let mut runner = op_runner();
    runner.play(0, 0).expect("Vademon played");

    let view = runner.pending_selection_view().expect("return target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(!runner.pending_is_optional(), "mandatory — no 'you may'");
    assert!(!view.valid_action_ids.contains(&PASS), "can't decline");

    let c3 = handle_of(&runner, 1, "OPP-C3");
    let tamer = handle_of(&runner, 1, "OPP-TAMER");
    let mut expected = vec![target_action(c3), target_action(tamer)];
    expected.sort();
    let mut got = view.valid_action_ids.clone();
    got.sort();
    assert_eq!(got, expected, "only cost<=3 opponent Digimon/Tamers");
}

/// Picking the cost-3 Digimon puts its top card on top of the opponent's deck
/// and trashes the card under it (§4-7-8).
#[test]
fn bt24_061_on_play_returns_digimon_to_top_of_owners_deck() {
    let mut runner = op_runner();
    let deck_before = runner.deck_size(1);
    runner.play(0, 0).expect("Vademon played");
    let c3 = handle_of(&runner, 1, "OPP-C3");
    runner.execute_action(0, target_action(c3)).expect("pick OPP-C3");
    let _ = runner.auto_resolve();

    assert!(!on_field(&runner, 1, "OPP-C3"), "OPP-C3 left the battle area");
    assert_eq!(runner.deck_size(1), deck_before + 1);
    assert_eq!(deck_top_id(&runner, 1), "OPP-C3", "placed on TOP of the deck");
    assert!(trash_has(&runner, 1, "OPP-SRC"), "its digivolution card is trashed");
    assert!(on_field(&runner, 1, "OPP-C4"), "cost-4 Digimon untouched");
    assert!(on_field(&runner, 1, "OPP-TAMER"), "Tamer untouched");
}

/// A Tamer is a legal pick and goes to the top of its owner's deck.
#[test]
fn bt24_061_on_play_returns_tamer_to_top_of_deck() {
    let mut runner = op_runner();
    runner.play(0, 0).expect("Vademon played");
    let tamer = handle_of(&runner, 1, "OPP-TAMER");
    runner.execute_action(0, target_action(tamer)).expect("pick Tamer");
    let _ = runner.auto_resolve();

    assert!(!on_field(&runner, 1, "OPP-TAMER"));
    assert_eq!(deck_top_id(&runner, 1), "OPP-TAMER");
    assert!(on_field(&runner, 1, "OPP-C3"));
}

/// Negative: with no opponent Digimon/Tamer at play cost 3 or lower, nothing
/// is offered and the board is unchanged.
#[test]
fn bt24_061_on_play_no_eligible_target_no_selection() {
    let mut runner = vademon_builder()
        .add_card(digimon("OPP-C4", 4, 4, CardColor::Blue, &[]))
        .add_card(tamer("OPP-TAMER4", 4))
        .add_card(digimon("OWN-C2", 3, 2, CardColor::Black, &[]))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    main_phase(&mut runner);
    runner.place_on_field(1, "OPP-C4", Some(0));
    runner.place_on_field(1, "OPP-TAMER4", None);
    runner.place_on_field(0, "OWN-C2", Some(0));
    let deck_before = runner.deck_size(1);

    runner.play(0, 0).expect("Vademon played");
    assert!(
        runner.pending_selection_view().is_none(),
        "no cost<=3 opponent Digimon/Tamer → no prompt"
    );
    assert!(on_field(&runner, 1, "OPP-C4"));
    assert!(on_field(&runner, 1, "OPP-TAMER4"));
    assert!(on_field(&runner, 0, "OWN-C2"), "own Digimon never a target");
    assert_eq!(runner.deck_size(1), deck_before);
}

/// [When Digivolving]: digivolving onto a [TS] Lv.4 fires the same return.
#[test]
fn bt24_061_when_digivolving_returns_opponent_card_to_top_of_deck() {
    let mut runner = vademon_builder()
        .add_card(digimon("TS-LV4", 4, 4, CardColor::Red, &["TS"]))
        .add_card(digimon("OPP-C3", 4, 3, CardColor::Blue, &[]))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    main_phase(&mut runner);
    runner.place_on_field(0, "TS-LV4", Some(0));
    runner.place_on_field(1, "OPP-C3", Some(0));

    assert!(runner.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand));
    let view = runner
        .pending_selection_view()
        .expect("When Digivolving return prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    let c3 = handle_of(&runner, 1, "OPP-C3");
    runner.execute_action(0, target_action(c3)).expect("pick OPP-C3");
    let _ = runner.auto_resolve();

    assert!(!on_field(&runner, 1, "OPP-C3"));
    assert_eq!(deck_top_id(&runner, 1), "OPP-C3");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited [When Attacking][Once Per Turn] <De-Digivolve 1>
// ═══════════════════════════════════════════════════════════════════════════════

/// Vademon as a digivolution source under CARRIER; opponent has a 3-card
/// Digimon (Lv.3 base, Lv.4, Lv.5 top) and a Tamer.
fn inherited_runner() -> (DebugRunner, PermanentHandle) {
    let mut r = vademon_builder()
        .add_card(digimon("CARRIER", 6, 7, CardColor::Black, &[]))
        .add_card(digimon("OPP-L3", 3, 3, CardColor::Blue, &[]))
        .add_card(digimon("OPP-L4", 4, 4, CardColor::Blue, &[]))
        .add_card(digimon("OPP-L5", 5, 5, CardColor::Blue, &[]))
        .add_card(tamer("OPP-TAMER", 3))
        .memory(5)
        .start();
    main_phase(&mut r);
    r.place_stack(0, &[CARD_ID, "CARRIER"]);
    r.place_stack(1, &["OPP-L3", "OPP-L4", "OPP-L5"]);
    r.place_on_field(1, "OPP-TAMER", None);
    let carrier = handle_of(&r, 0, "CARRIER");
    (r, carrier)
}

/// Attacking with the carrier installs a mandatory pick of an opponent DIGIMON
/// (Tamers excluded); the chosen Digimon loses its top card.
#[test]
fn bt24_061_inherited_when_attacking_de_digivolves_opponent_digimon() {
    let (mut runner, carrier) = inherited_runner();
    runner.attack_player(carrier, 1, false);

    let view = runner
        .pending_selection_view()
        .expect("De-Digivolve target prompt on attack");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(!runner.pending_is_optional(), "<De-Digivolve> is mandatory");
    let opp = handle_of(&runner, 1, "OPP-L5");
    assert_eq!(
        view.valid_action_ids,
        vec![target_action(opp)],
        "only the opponent's Digimon — never the Tamer"
    );

    runner.execute_action(0, target_action(opp)).expect("pick OPP-L5");
    let _ = runner.auto_resolve();

    let opp = handle_of(&runner, 1, "OPP-L4");
    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize].stack_size(),
        2,
        "De-Digivolve 1 trashed exactly the top card"
    );
    assert!(trash_has(&runner, 1, "OPP-L5"));
}

/// Negative: Vademon's inherited effect is inactive while Vademon itself is the
/// top card (it only applies as a digivolution card).
#[test]
fn bt24_061_inherited_does_not_fire_from_top_card() {
    let mut runner = vademon_builder()
        .add_card(digimon("OPP-L3", 3, 3, CardColor::Blue, &[]))
        .add_card(digimon("OPP-L4", 4, 4, CardColor::Blue, &[]))
        .memory(5)
        .start();
    main_phase(&mut runner);
    let vade = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_stack(1, &["OPP-L3", "OPP-L4"]);

    let sec_before = runner.security_count(1);
    runner.attack_player(vade, 1, false);
    assert!(
        runner.pending_selection_view().is_none(),
        "inherited text must not fire from the top card"
    );
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "the attack itself resolved (checks one security card)"
    );
    let opp = handle_of(&runner, 1, "OPP-L4");
    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize].stack_size(),
        2
    );
}

/// Negative: with no opponent Digimon (only a Tamer) nothing is offered.
#[test]
fn bt24_061_inherited_no_opponent_digimon_no_selection() {
    let mut runner = vademon_builder()
        .add_card(digimon("CARRIER", 6, 7, CardColor::Black, &[]))
        .add_card(tamer("OPP-TAMER", 3))
        .memory(5)
        .start();
    main_phase(&mut runner);
    runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.place_on_field(1, "OPP-TAMER", None);
    let carrier = handle_of(&runner, 0, "CARRIER");

    let sec_before = runner.security_count(1);
    runner.attack_player(carrier, 1, false);
    assert!(
        runner.pending_selection_view().is_none(),
        "a Tamer is not a De-Digivolve target"
    );
    let _ = runner.auto_resolve();
    assert_eq!(runner.security_count(1), sec_before - 1, "the attack resolved");
    assert!(on_field(&runner, 1, "OPP-TAMER"));
}

/// OPT: a second attack in the same turn does not re-fire; after a full turn
/// cycle it fires again.
#[test]
fn bt24_061_inherited_is_once_per_turn_and_resets_next_turn() {
    let (mut runner, carrier) = inherited_runner();

    runner.attack_player(carrier, 1, false);
    let opp = handle_of(&runner, 1, "OPP-L5");
    runner.execute_action(0, target_action(opp)).expect("first activation");
    let _ = runner.auto_resolve();

    // Second attack this turn.
    let carrier = handle_of(&runner, 0, "CARRIER");
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = false;
    let sec_before = runner.security_count(1);
    runner.attack_player(carrier, 1, false);
    assert!(
        runner.pending_selection_view().is_none(),
        "[Once Per Turn]: no second activation this turn"
    );
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "the second attack itself resolved"
    );
    let opp = handle_of(&runner, 1, "OPP-L4");
    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize].stack_size(),
        2,
        "still only one card trashed"
    );

    // Opponent's turn, then back to player 0.
    runner.end_turn();
    let _ = runner.auto_resolve();
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 0);
    runner.game.current_phase = GamePhase::Main;

    let carrier = handle_of(&runner, 0, "CARRIER");
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = false;
    runner.attack_player(carrier, 1, false);
    let view = runner
        .pending_selection_view()
        .expect("OPT lockout cleared on the next turn");
    let opp = handle_of(&runner, 1, "OPP-L4");
    assert_eq!(view.valid_action_ids, vec![target_action(opp)]);
}

fn top_id(runner: &DebugRunner, player: usize, index: usize) -> String {
    runner.game.players[player].battle_area[index]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}
