//! BT25-054 GreatGrizzlymon — Digimon, Lv.5, Green, DP 7000, Cost 8.
//! Traits: Beastkin, Iliad, TS. Attribute: Vaccine.
//! Evo: Lv.4 Green / cost 4 (printed). Alt-digi: Lv.4 w/ [TS] trait / cost 3.
//!
//! # Card text (cards.json — verbatim; confirmed against card image BT25-054)
//!
//! <Blocker> (This Digimon can block in the blocker timing.)
//! [On Play] [When Digivolving] Give 1 of your opponent's Digimon "[Start of
//! Your Main Phase] This Digimon attacks." until their turn ends.
//! [All Turns] When this Digimon wins a battle, it may digivolve into
//! [Callismon] or [Marsmon] in the hand without paying the cost.
//!
//! Inherited Effect:
//! [All Turns] [Once Per Turn] When this Digimon deletes your opponent's Digimon
//! in battle, trash their top security card.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_054.cs
//!   - EffectTiming.None: AddSelfDigivolutionRequirementStaticEffect(level 4,
//!     HasTSTraits, cost 3) + BlockerSelfStaticEffect.
//!   - Shared OnPlay/WhenDigivolving: SelectPermanentEffect over opp Digimon →
//!     grant "[Start of Your Main Phase] This Digimon attacks." (UntilOwnerTurnEnd
//!     i.e. until the opponent's turn ends).
//!   - OnEndBattle: when this Digimon wins (CanTriggerWhenWinBattle / winner
//!     contains card), may digivolve into [Callismon]/[Marsmon] from hand,
//!     payCost:false. Skippable.
//!   - OnEndBattle inherited: OPT, when this Digimon deletes opp's Digimon in
//!     battle (CanTriggerWhenDeleteOpponentDigimonByBattle, winner = self),
//!     IDestroySecurity(enemy, 1, fromTop:true). isInheritedEffect:true.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - H5 <Blocker>
//! - grant_triggered_effect: taunt opp Digimon a Start-of-Main forced attack
//! - win-battle (source_deleted_battle_opponent) → self free digivolve
//! - inherited win-battle (source_deleted_battle_opponent) OPT → trash top security
//! - alt-digivolution Lv.4 [TS] → cost 3

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-054";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn opp_digimon(card_id: &str, dp: i32) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.colors = vec![CardColor::Purple];
    card.dp = Some(dp);
    card
}

fn named_lv6(card_id: &str, name: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, name, 6);
    card.colors = vec![CardColor::Green];
    card.dp = Some(11000);
    card.evo_costs = vec![EvoCost {
        card_color: 3,
        level: 5,
        memory_cost: 4,
    }];
    card
}

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_index, player, instance_id));
    runner.game.players[player as usize].hand.len() - 1
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-054 YAML parses and compiles")
        .add_card(opp_digimon("OPP-3K", 3000))
        .add_card(opp_digimon("OPP-WEAK", 1000))
        .add_card(named_lv6("CALLIS", "Callismon"))
        .add_card(named_lv6("MARS", "Marsmon"))
        .add_card(named_lv6("OTHER6", "Random Mega"))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
}

/// GreatGrizzlymon (DP 7000) on P0's field as its own top card.
fn place_grizzly(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_on_field(0, CARD_ID, Some(0))
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_054_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-054 must be in the embedded DSL pack");

    assert_eq!(card.name, "GreatGrizzlymon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    for trait_name in ["Beastkin", "Iliad", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-054 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_054_has_ts_alt_digivolution_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_ts = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(3))
            && p.from
                .as_ref()
                .is_some_and(|f| f.level_eq == Some(4) && f.trait_has.as_deref() == Some("TS"))
    });
    assert!(has_ts, "BT25-054 must have a Lv.4 [TS] → cost 3 alt-path");
}

#[test]
fn bt25_054_has_blocker_keyword() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_blocker = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(d) => format!("{d:?}").contains("Blocker"),
        _ => false,
    });
    assert!(has_blocker, "BT25-054 must grant <Blocker>");
}

#[test]
fn bt25_054_has_op_taunt_and_two_endbattle_clauses() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let has_op = triggered.iter().any(|t| {
        t.when.contains(&CompiledTiming::OnPlay)
            && t.when.contains(&CompiledTiming::WhenDigivolving)
    });
    // Win-battle clauses lower onto a deletion/end-battle timing.
    let has_win_digivolve = triggered.iter().any(|t| {
        t.scope == CompiledScope::FaceUp
            && (t.when.contains(&CompiledTiming::OnAnyDeletion)
                || t.when.contains(&CompiledTiming::EndOfBattle))
            && t.optional
    });
    let has_inherited_win_security = triggered.iter().any(|t| {
        t.scope == CompiledScope::Inherited
            && (t.when.contains(&CompiledTiming::OnAnyDeletion)
                || t.when.contains(&CompiledTiming::EndOfBattle))
            && t.once_per_turn
    });
    assert!(has_op, "must have [On Play][When Digivolving] taunt clause");
    assert!(
        has_win_digivolve,
        "must have the optional win-battle self-digivolve clause"
    );
    assert!(
        has_inherited_win_security,
        "must have the inherited OPT win-battle trash-security clause"
    );
}

// ─── Section 2 — [On Play] taunt: grant opp Digimon a start-of-main attack ────

#[test]
fn bt25_054_on_play_taunts_one_opponent_digimon() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let opp = runner.place_on_field(1, "OPP-3K", Some(0));

    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);
    runner.play(0, hand_index);

    // The taunt requires selecting exactly 1 opponent Digimon — a selection
    // must install (mandatory pick when a target exists).
    assert!(
        runner.pending_kind().is_some(),
        "the [On Play] taunt must install an opponent-Digimon selection"
    );
    runner.auto_resolve().expect("resolve taunt");
}

#[test]
fn bt25_054_on_play_no_taunt_when_no_opponent_digimon() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    // Opponent has no Digimon to taunt.

    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);
    runner.play(0, hand_index);

    assert!(
        runner.pending_selection().is_none(),
        "with no opponent Digimon there is nothing to taunt"
    );
    runner.auto_resolve().ok();
}

// ─── Section 3 — [All Turns] win battle → free digivolve into Callismon/Marsmon ─

#[test]
fn bt25_054_free_digivolve_when_grizzly_wins_a_battle() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    runner.game_mut().players[1].security.clear();

    // GreatGrizzlymon (7000) attacks a weak opponent (1000) it will delete and
    // survive — i.e. it "wins the battle".
    let grizzly = place_grizzly(&mut runner);
    let weak = runner.place_on_field(1, "OPP-WEAK", Some(0));
    let _callis_in_hand = push_to_hand(&mut runner, 0, "CALLIS");

    let memory_before = runner.memory();
    runner.attack_digimon(grizzly, weak, false);

    // The optional free digivolve into Callismon/Marsmon must be offered.
    assert!(
        runner.pending_kind().is_some(),
        "winning a battle must offer GreatGrizzlymon's free digivolve"
    );
    runner.auto_resolve().expect("resolve digivolve offer");

    assert_eq!(
        runner.memory(),
        memory_before,
        "the digivolve is 'without paying the cost' — memory unchanged"
    );
}

#[test]
fn bt25_054_no_digivolve_when_no_named_card_in_hand() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    runner.game_mut().players[1].security.clear();

    let grizzly = place_grizzly(&mut runner);
    let weak = runner.place_on_field(1, "OPP-WEAK", Some(0));
    // Only a non-Callismon/Marsmon Mega in hand.
    let _other = push_to_hand(&mut runner, 0, "OTHER6");

    runner.attack_digimon(grizzly, weak, false);
    assert!(
        runner.pending_selection().is_none(),
        "without [Callismon]/[Marsmon] in hand there is no digivolve to offer"
    );
    runner.auto_resolve().ok();
}

// ─── Section 4 — Inherited [All Turns][OPT] win battle → trash top security ───

#[test]
fn bt25_054_inherited_trashes_top_security_on_battle_win() {
    // GreatGrizzlymon buried under a Lv.6 carrier; the inherited fires from the
    // stack when the carrier deletes an opponent in battle and survives.
    let mut carrier = named_lv6("CARRIER6", "Carrier Mega");
    carrier.dp = Some(9000);

    let mut runner = base().add_card(carrier).memory(10).start();
    runner.game.turn_count = 1;

    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER6"]); // GreatGrizzlymon is a source
    let weak = runner.place_on_field(1, "OPP-WEAK", Some(0)); // 1000 DP — gets deleted
                                                              // Opponent security to trash.
    runner.game_mut().players[1].security.clear();
    let fill_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "FILL")
        .unwrap();
    for _ in 0..2 {
        let iid = runner.game.next_card_index();
        runner.game.players[1]
            .security
            .push(CardSource::new(fill_idx, 1, iid));
    }

    let sec_before = runner.security_count(1);
    runner.attack_digimon(stack, weak, false);
    runner
        .auto_resolve()
        .expect("resolve inherited security trash");

    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "the inherited effect trashes the opponent's top security card on a battle win"
    );
}

#[test]
fn bt25_054_inherited_does_not_fire_when_no_battle_win() {
    // GreatGrizzlymon as a source but no battle occurs → no security trash.
    let mut carrier = named_lv6("CARRIER6", "Carrier Mega");
    carrier.dp = Some(9000);

    let mut runner = base().add_card(carrier).memory(10).start();
    runner.game.turn_count = 1;

    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER6"]);
    runner.game_mut().players[1].security.clear();
    let fill_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "FILL")
        .unwrap();
    for _ in 0..2 {
        let iid = runner.game.next_card_index();
        runner.game.players[1]
            .security
            .push(CardSource::new(fill_idx, 1, iid));
    }

    let sec_before = runner.security_count(1);
    // Attack directly into the (now non-empty) player security — this is a
    // security check, NOT a Digimon-vs-Digimon battle win, so the inherited
    // (source_deleted_battle_opponent) must NOT fire.
    runner.attack_player(stack, 1, false);
    runner.auto_resolve().ok();

    assert!(
        runner.security_count(1) >= sec_before - 1,
        "a normal security check is not a battle-win; the inherited must not \
         additionally trash a card via source_deleted_battle_opponent"
    );
}
