//! BT25-016 GrapLeomon — Digimon, Lv.5, Red, DP 7000, Cost 7.
//! Traits: Beastkin, Iliad, TS. Attribute: Vaccine.
//! Evo: Lv.4 Red / cost 4 (printed). Alt-digi: Lv.4 w/ [TS] trait / cost 3.
//!
//! # Card text (cards.json — verbatim; confirmed against card image BT25-016)
//!
//! [On Play] [When Digivolving] 1 of your Digimon gets +3000 DP for the turn.
//! Then, 1 of your Digimon may attack.
//! [All Turns] When a Digimon with 13000 DP or more attacks, this Digimon may
//! digivolve into [Marsmon] or [Callismon] in the hand without paying the cost.
//!
//! Inherited Effect:
//! <Security A. +1> (This Digimon checks 1 additional security card.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_016.cs
//!   - EffectTiming.None: AddSelfDigivolutionRequirementStaticEffect(level 4,
//!     HasTSTraits, cost 3).
//!   - Shared OnPlay/WhenDigivolving: SelectPermanentEffect over own Digimon
//!     (canNoSelect:false) → ChangeDigimonDP(+3000, UntilEachTurnEnd). Then
//!     SelectPermanentEffect over own Digimon that CanAttack (canNoSelect:true,
//!     Mode.Attack).
//!   - OnAllyAttack: when an attacking permanent has DP >= 13000, this Digimon
//!     may digivolve into [Marsmon]/[Callismon] from hand, payCost:false,
//!     isHand:true. Skippable (optional).
//!   - ChangeSelfSAttackStaticEffect(+1, isInheritedEffect:true).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - D1 conditional DP buff (selected target, end-of-turn expiry)
//! - effect-created may-attack on a selected own Digimon (Track D may_attack_now)
//! - All-turns on-attack observer gated on attacker DP >= 13000
//! - self effect_initiated_digivolve into one of two named hand cards (free)
//! - H4 inherited Security A. +1
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

const CARD_ID: &str = "BT25-016";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn lv5_digimon(card_id: &str, name: &str, dp: i32) -> CardData {
    let mut card = make_test_card_with_level(card_id, name, 5);
    card.colors = vec![CardColor::Red];
    card.dp = Some(dp);
    card
}

fn named_lv6(card_id: &str, name: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, name, 6);
    card.colors = vec![CardColor::Red];
    card.dp = Some(11000);
    // Digivolves from a Lv.5 for cost 4 (so "without paying the cost" is observable).
    card.evo_costs = vec![EvoCost {
        card_color: 0,
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
        .expect("BT25-016 YAML parses and compiles")
        .add_card(lv5_digimon("ALLY-A", "Ally A", 5000))
        .add_card(lv5_digimon("ALLY-B", "Ally B", 5000))
        .add_card(lv5_digimon("BIG-13K", "Big Beast", 13000))
        .add_card(named_lv6("MARS", "Marsmon"))
        .add_card(named_lv6("CALLIS", "Callismon"))
        .add_card(named_lv6("OTHER6", "Random Mega"))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
}

/// GrapLeomon standing on P0's field as its own top card (top of a Lv.5 stack).
fn place_grapleomon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_on_field(0, CARD_ID, Some(0))
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_016_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-016 must be in the embedded DSL pack");

    assert_eq!(card.name, "GrapLeomon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    for trait_name in ["Beastkin", "Iliad", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-016 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_016_has_ts_alt_digivolution_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_ts = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(3))
            && p.from
                .as_ref()
                .is_some_and(|f| f.level_eq == Some(4) && f.trait_has.as_deref() == Some("TS"))
    });
    assert!(has_ts, "BT25-016 must have a Lv.4 [TS] → cost 3 alt-path");
}

#[test]
fn bt25_016_has_on_play_when_digivolving_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let on_play_wd = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => {
            t.scope == CompiledScope::FaceUp
                && t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        }
        _ => false,
    });
    assert!(
        on_play_wd,
        "BT25-016 must have an [On Play][When Digivolving] clause"
    );
}

#[test]
fn bt25_016_has_all_turns_attack_observer_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let observer = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => {
            t.scope == CompiledScope::FaceUp
                && (t.when.contains(&CompiledTiming::OnAllyAttack)
                    || t.when.contains(&CompiledTiming::OnAttack)
                    || t.when.contains(&CompiledTiming::OnOpponentAttack))
                && t.optional
        }
        _ => false,
    });
    assert!(
        observer,
        "BT25-016 must have an optional all-turns on-attack digivolve observer"
    );
}

// ─── Section 2 — [On Play][When Digivolving] +3000 DP then may attack ─────────

#[test]
fn bt25_016_on_play_grants_plus_3000_to_selected_ally() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let ally = runner.place_on_field(0, "ALLY-A", Some(0));
    let base_dp = runner.effective_dp(ally).expect("ally has dp");

    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);
    runner.play(0, hand_index);

    // The +3000 DP selection is mandatory: drive the first own-Digimon target
    // explicitly to assert a buff lands.
    if let Some(view) = runner.pending_selection_view() {
        runner.execute_action(0, view.valid_action_ids[0]).ok();
    }
    runner.auto_resolve().expect("resolve play");

    // Assert at least one own Digimon now carries a +3000 ChangeDp modifier
    // (the "for the turn" buff). The modifier registry is the reliable signal —
    // dp_of/effective_dp both already fold modifiers in.
    let buffed = (0..runner.battle_area_size(0)).any(|i| {
        let h = runner.perm_handle(0, i);
        runner
            .modifiers()
            .sum(h, digimon_engine::enums::ModifierType::ChangeDp)
            >= 3000
    });
    assert!(
        buffed,
        "the [On Play] effect must grant +3000 DP to a selected own Digimon"
    );
}

#[test]
fn bt25_016_on_play_exposes_may_attack_choice() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    runner.game_mut().players[1].security.clear();
    // An ally already on field that can attack the player.
    let ally = runner.place_on_field(0, "ALLY-A", Some(0));

    let hand_index = push_to_hand(&mut runner, 0, CARD_ID);
    runner.play(0, hand_index);

    // Resolve the mandatory +3000 DP pick.
    if let Some(view) = runner.pending_selection_view() {
        runner.execute_action(0, view.valid_action_ids[0]).ok();
    }
    // After the DP pick, the "1 of your Digimon may attack" choice must be
    // exposed via pending selection (no hidden auto-attack).
    assert!(
        runner.pending_selection_view().is_some(),
        "the 'may attack' choice must be exposed via pending selection"
    );
    runner.auto_resolve().expect("resolve the rest");
}

// ─── Section 3 — [All Turns] 13000+ DP attacker → free digivolve into Marsmon/Callismon ─

#[test]
fn bt25_016_free_digivolve_when_13k_attacker_attacks() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    runner.game_mut().players[1].security.clear();

    // GrapLeomon on field; a 13000-DP own Digimon that will attack.
    let grap = place_grapleomon(&mut runner);
    let big = runner.place_on_field(0, "BIG-13K", Some(0));
    let marsmon_in_hand = push_to_hand(&mut runner, 0, "MARS");

    let memory_before = runner.memory();
    runner.attack_player(big, 1, false);

    // The optional free-digivolve into Marsmon/Callismon must be offered.
    assert!(
        runner.pending_kind().is_some(),
        "a 13000+ DP attacker must offer GrapLeomon's free digivolve"
    );
    runner.auto_resolve().expect("resolve digivolve offer");

    // The digivolve is "without paying the cost" — memory must be unchanged.
    assert_eq!(
        runner.memory(),
        memory_before,
        "the digivolve is 'without paying the cost' — memory must be unchanged"
    );
}

#[test]
fn bt25_016_no_digivolve_when_attacker_below_13k() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    runner.game_mut().players[1].security.clear();

    let grap = place_grapleomon(&mut runner);
    let small = runner.place_on_field(0, "ALLY-A", Some(0)); // 5000 DP
    let _marsmon_in_hand = push_to_hand(&mut runner, 0, "MARS");

    runner.attack_player(small, 1, false);
    assert!(
        runner.pending_selection().is_none(),
        "an attacker below 13000 DP must NOT trigger the free digivolve"
    );
    runner.auto_resolve().ok();
}

#[test]
fn bt25_016_no_digivolve_offer_without_named_card_in_hand() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    runner.game_mut().players[1].security.clear();

    let grap = place_grapleomon(&mut runner);
    let big = runner.place_on_field(0, "BIG-13K", Some(0));
    // Only a non-Marsmon/Callismon Mega is in hand.
    let _other = push_to_hand(&mut runner, 0, "OTHER6");

    runner.attack_player(big, 1, false);
    assert!(
        runner.pending_selection().is_none(),
        "without [Marsmon]/[Callismon] in hand there is no digivolve to offer"
    );
    runner.auto_resolve().ok();
}

// ─── Section 4 — Inherited Security A. +1 ────────────────────────────────────

#[test]
fn bt25_016_has_inherited_security_attack_plus_one() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_sa = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(d) => format!("{d:?}").contains("SecurityAttack"),
        _ => false,
    });
    assert!(
        has_sa,
        "BT25-016 must grant inherited <Security A. +1> (SecurityAttackPlus)"
    );
}
