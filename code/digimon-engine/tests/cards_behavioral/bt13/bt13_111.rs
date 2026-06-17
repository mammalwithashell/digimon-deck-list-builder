//! BT13-111 Gallantmon — Digimon, Lv.6, Red, Cost 13, DP 13000.
//! Traits: [Royal Knight, Holy Warrior]. Attribute: Virus. Form: Mega.
//!
//! # Card text (card image BT13-111 — authoritative)
//!
//! When you would play this card from the hand, if you have no Digimon, reduce
//! the play cost by 2 for every 5 total cards in both players' trashes.
//! ＜Rush＞ (This Digimon can attack the turn it was played.)
//! [On Play] [When Digivolving] [When Attacking] Delete 1 of your opponent's
//! Digimon with 6000 DP or less. If no opponent's Digimon was deleted by this
//! effect, delete 1 of their Digimon with 13000 DP or more.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT13/Red/BT13_111.cs
//!
//! - Region "Play Cost -" (None / ChangeCostClass): count() = 2*((own.Trash +
//!   enemy.Trash) / 5). Gates: card in HandCards (RootCondition == Hand),
//!   count() >= 1, GetBattleAreaDigimons().Count == 0.
//! - Region "Rush" (None): RushSelfStaticEffect.
//! - OnEnterFieldAnyone (On Play) / OnEnterFieldAnyone (When Digivolving) /
//!   OnAllyAttack (When Attacking): delete 1 opp Digimon DP <= 6000 (mandatory
//!   single pick), then FailureProcess (no deletion occurred) → delete 1 opp
//!   Digimon DP >= 13000.
//!
//! # DSL YAML
//! code/digimon-engine/cards/bt13/BT13-111.yaml
//!
//! # Patterns this test covers (RUST_DSL_TEST_API)
//! - cost_reduction when_playing_this gated on own battle-area Digimon == 0
//! - cost-reduction formula floor(combined_trash / 5) * 2 (shared_trash_count)
//! - cost reduction excludes the digivolve path (when_playing_this)
//! - Rush keyword installed on field entry
//! - delete-with-fallback via effect_deleted_any_opponent_digimon:false
//! - delete only targets dp_lte:6000 / fallback only dp_gte:13000 opp Digimon
//! - fallback does NOT fire when the <=6000 delete succeeded
//! - fires on On Play, When Digivolving, and When Attacking

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const YAML: &str = include_str!("../../../cards/bt13/BT13-111.yaml");

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Digimon with an explicit DP value (and level 5 so it can act as an
/// evolve source / generic field Digimon).
fn make_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

/// A Lv.5 Red Digimon eligible as a digivolution base for BT13-111.
fn make_lv5_red(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(8000);
    c
}

/// Push a card directly into a player's trash (without play/delete path).
fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {}", card_id));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, player, next_idx);
    runner.game.players[player as usize].trash.push(card);
}

/// Fire the [When Digivolving] timing for a permanent on the field.
fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 0 — load smoke
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt13_111_loads_rush_slice() {
    DebugRunner::builder()
        .dsl_card("BT13-111")
        .expect("BT13-111 must load from embedded DSL pack")
        .start();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt13_111_yaml_parses_and_compiles() {
    let _builder = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT13-111 YAML must parse and compile without errors");
}

#[test]
fn bt13_111_metadata_is_digimon_cost_13_dp_13000() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("BT13-111").expect("compiled present");
    assert_eq!(compiled.name, "Gallantmon");
    assert_eq!(compiled.cost, Some(13), "play cost 13");
    assert_eq!(compiled.dp, Some(13000), "13000 DP");
    assert_eq!(compiled.level, Some(6), "Level 6");
    for t in ["Royal Knight", "Holy Warrior"] {
        assert!(
            compiled.traits.contains(&t.to_string()),
            "trait {t} present"
        );
    }
}

/// The cost_reduction clause must be before_pay_cost + when_playing_this and
/// carry both a condition (no own Digimon) and an amount_fn (formula).
#[test]
fn bt13_111_has_cost_reduction_before_pay_cost_when_playing_this() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("BT13-111").expect("compiled");

    let cost_red = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
            reduction_timing,
            when_playing_this,
            condition,
            amount_fn,
            ..
        }) => Some((
            reduction_timing.clone(),
            *when_playing_this,
            condition.clone(),
            amount_fn.clone(),
        )),
        _ => None,
    });

    let (reduction_timing, when_playing_this, condition, amount_fn) =
        cost_red.expect("BT13-111 must have a CostReduction declarative clause");

    assert_eq!(
        reduction_timing.as_deref(),
        Some("before_pay_cost"),
        "reduction_timing must be before_pay_cost"
    );
    assert!(
        when_playing_this,
        "when_playing_this must be true (fires only on the hand-play path)"
    );
    assert!(
        condition.is_some(),
        "cost reduction must be gated on a condition (no own Digimon)"
    );
    assert!(
        amount_fn.is_some(),
        "cost reduction must use amount_fn (shared_trash_count formula)"
    );
}

#[test]
fn bt13_111_has_rush_grant_keyword() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("BT13-111").expect("compiled");
    let has_rush = compiled.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
            if keyword == "Rush"
    ));
    assert!(has_rush, "BT13-111 must grant Rush");
}

/// The delete clause fires on On Play, When Digivolving AND When Attacking and
/// is mandatory (the delete is not a 'may').
#[test]
fn bt13_111_delete_clause_on_play_digivolve_and_attack() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("BT13-111").expect("compiled");
    let triggered = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("BT13-111 must have a triggered delete clause");

    assert!(
        triggered.when.contains(&CompiledTiming::OnPlay),
        "fires on OnPlay; timings={:?}",
        triggered.when
    );
    assert!(
        triggered.when.contains(&CompiledTiming::WhenDigivolving),
        "fires on WhenDigivolving; timings={:?}",
        triggered.when
    );
    assert!(
        triggered.when.contains(&CompiledTiming::WhenAttacking),
        "fires on WhenAttacking; timings={:?}",
        triggered.when
    );
    assert!(
        !triggered.optional,
        "delete clause must be mandatory (the <=6000 delete is not a 'may')"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Rush keyword
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt13_111_on_field_has_rush() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    let handle = runner.place_on_field(0, "BT13-111", Some(0));
    runner.game.tick_declarative_effects();

    let via_modifier = runner.game.has_keyword(handle, Keyword::Rush);
    let via_card_data = runner
        .game
        .card_data
        .iter()
        .find(|c| c.card_id == "BT13-111")
        .map_or(false, |d| {
            d.keywords.iter().any(|k| matches!(k, Keyword::Rush))
        });

    assert!(
        via_modifier || via_card_data,
        "Rush must be present on BT13-111 after it enters the field"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Cost reduction (combined trash, no own Digimon gate)
// ═══════════════════════════════════════════════════════════════════════════════

/// With no own Digimon in play and exactly 5 combined trash cards, playing
/// BT13-111 (cost 13) costs 13 - floor(5/5)*2 = 11. memory delta = -11.
#[test]
fn bt13_111_cost_reduced_by_2_with_5_combined_trash_and_no_own_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["BT13-111"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(13)
        .start();

    // 5 combined trash cards (all P0).
    for _ in 0..5 {
        push_to_trash(&mut runner, 0, "FILL");
    }

    let mem_before = runner.memory();
    runner.play(0, 0);
    let _ = runner.auto_resolve();
    let mem_after = runner.memory();

    let paid = mem_before - mem_after;
    assert_eq!(
        paid, 11,
        "no own Digimon + 5 combined trash → cost 13 - 2 = 11; \
         before={mem_before}, after={mem_after}, paid={paid}"
    );
}

/// 10 combined trash (5 own + 5 opp) → floor(10/5)*2 = 4 reduction → cost 9.
#[test]
fn bt13_111_cost_reduced_by_4_with_10_combined_trash_across_players() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["BT13-111"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(13)
        .start();

    for _ in 0..5 {
        push_to_trash(&mut runner, 0, "FILL");
    }
    for _ in 0..5 {
        push_to_trash(&mut runner, 1, "FILL");
    }

    let mem_before = runner.memory();
    runner.play(0, 0);
    let _ = runner.auto_resolve();
    let mem_after = runner.memory();

    let paid = mem_before - mem_after;
    assert_eq!(
        paid, 9,
        "10 combined trash → floor(10/5)*2 = 4 → cost 13 - 4 = 9; \
         before={mem_before}, after={mem_after}, paid={paid}"
    );
}

/// Fewer than 5 combined trash cards → floor(4/5)*2 = 0 → no reduction (full 13).
#[test]
fn bt13_111_no_reduction_below_5_combined_trash() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["BT13-111"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(13)
        .start();

    for _ in 0..4 {
        push_to_trash(&mut runner, 0, "FILL");
    }

    let mem_before = runner.memory();
    runner.play(0, 0);
    let _ = runner.auto_resolve();
    let mem_after = runner.memory();

    let paid = mem_before - mem_after;
    assert_eq!(
        paid, 13,
        "fewer than 5 combined trash → no reduction → full cost 13; \
         before={mem_before}, after={mem_after}, paid={paid}"
    );
}

/// Gate is "you have no Digimon": with an own Digimon already in play, the
/// reduction does NOT apply even with abundant trash → full cost 13.
#[test]
fn bt13_111_no_reduction_when_own_digimon_in_play() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("OWN-DIG", 4, 4000))
        .add_card(filler("FILL"))
        .hand(0, &["BT13-111"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(13)
        .start();

    // An own Digimon already in play → gate fails.
    runner.place_on_field(0, "OWN-DIG", Some(0));

    // 10 combined trash would yield -4 if the gate were met.
    for _ in 0..10 {
        push_to_trash(&mut runner, 0, "FILL");
    }

    let mem_before = runner.memory();
    runner.play(0, 0);
    let _ = runner.auto_resolve();
    let mem_after = runner.memory();

    let paid = mem_before - mem_after;
    assert_eq!(
        paid, 13,
        "with an own Digimon in play the gate fails → full cost 13; \
         before={mem_before}, after={mem_after}, paid={paid}"
    );
}

/// An OPPONENT Digimon in play does NOT block the gate ("you have no Digimon"
/// is the controller's own field only). With an opp Digimon and 5 combined
/// trash, the reduction still applies → cost 11.
#[test]
fn bt13_111_reduction_applies_with_only_opponent_digimon_in_play() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("OPP-DIG", 4, 4000))
        .add_card(filler("FILL"))
        .hand(0, &["BT13-111"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(13)
        .start();

    // Opponent has a Digimon; controller's own field is empty.
    runner.place_on_field(1, "OPP-DIG", Some(0));

    for _ in 0..5 {
        push_to_trash(&mut runner, 0, "FILL");
    }

    let mem_before = runner.memory();
    runner.play(0, 0);
    let _ = runner.auto_resolve();
    let mem_after = runner.memory();

    // Paying 11 from a clamped starting memory of 10 overpays → memory goes
    // negative and the turn passes, flipping the stored sign. Compute the
    // actual amount paid in a sign-aware way (cf. P-186's test at p_186.rs).
    let paid = if runner.game.turn_player() == 0 {
        mem_before - mem_after
    } else {
        mem_before + mem_after
    };
    assert_eq!(
        paid, 11,
        "opponent Digimon does not count against 'you have no Digimon'; \
         5 trash → cost 11; before={mem_before}, after={mem_after}, paid={paid}"
    );
}

/// when_playing_this excludes the digivolve path: digivolving into BT13-111
/// from a Lv.5 Red base (cost 5) pays the digivolve cost, NOT a trash-reduced
/// play cost. With 10 combined trash, the digivolve cost stays 5 (no -4).
#[test]
fn bt13_111_cost_reduction_does_not_apply_on_digivolve_path() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_lv5_red("LV5-RED"))
        .add_card(filler("FILL"))
        .hand(0, &["BT13-111"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    // A Lv.5 Red Digimon on field to digivolve from.
    let base = runner.place_on_field(0, "LV5-RED", Some(0));

    // 10 combined trash — would be -4 if the reduction leaked onto digivolve.
    for _ in 0..10 {
        push_to_trash(&mut runner, 0, "FILL");
    }

    let mem_before = runner.memory();
    let hand_idx = runner
        .game
        .players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "BT13-111")
        .expect("BT13-111 in hand");
    let ok = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        base.index as usize,
        PlaySource::ByDigivolve,
    );
    assert!(ok, "digivolve_from_hand must succeed");
    let _ = runner.auto_resolve();
    let mem_after = runner.memory();

    let paid = mem_before - mem_after;
    assert_eq!(
        paid, 5,
        "digivolving pays the digivolve cost 5; the hand-play trash reduction must \
         not apply (when_playing_this); before={mem_before}, after={mem_after}, paid={paid}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — [On Play] delete-with-fallback
// ═══════════════════════════════════════════════════════════════════════════════

/// [On Play] with a <=6000 DP opponent Digimon: the delete prompt installs
/// over opponent-field targets.
#[test]
fn bt13_111_on_play_installs_delete_prompt_for_le_6000() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("OPP-SMALL", 4, 5000))
        .add_card(filler("FILL"))
        .hand(0, &["BT13-111"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    runner.place_on_field(1, "OPP-SMALL", Some(0));
    runner.play(0, 0);

    assert!(
        runner.game.pending_selection.is_some(),
        "[On Play] must install a delete prompt when a <=6000 DP opp Digimon exists"
    );
}

/// [On Play] deletes a <=6000 DP opponent Digimon.
#[test]
fn bt13_111_on_play_deletes_le_6000_opponent() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("OPP-SMALL", 4, 5000))
        .add_card(filler("FILL"))
        .hand(0, &["BT13-111"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    runner.place_on_field(1, "OPP-SMALL", Some(0));
    let opp_before = runner.battle_area_size(1);

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "[On Play] must delete the <=6000 DP opponent Digimon"
    );
}

/// 6000 DP exactly is deletable by the primary clause (dp_lte:6000 inclusive).
#[test]
fn bt13_111_on_play_deletes_exactly_6000() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("OPP-6K", 5, 6000))
        .add_card(filler("FILL"))
        .hand(0, &["BT13-111"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    runner.place_on_field(1, "OPP-6K", Some(0));
    let opp_before = runner.battle_area_size(1);

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "exactly 6000 DP is deletable (dp_lte:6000 inclusive)"
    );
}

/// Fallback: no <=6000 DP target but a >=13000 DP opponent Digimon → the
/// fallback deletes the >=13000 DP Digimon (effect_deleted_any:false branch).
#[test]
fn bt13_111_on_play_fallback_deletes_ge_13000_when_no_le_6000() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("OPP-HUGE", 7, 14000))
        .add_card(filler("FILL"))
        .hand(0, &["BT13-111"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    // Only a 14000 DP Digimon — no <=6000 target, but a >=13000 fallback target.
    runner.place_on_field(1, "OPP-HUGE", Some(0));
    let opp_before = runner.battle_area_size(1);

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "no <=6000 target → fallback deletes the >=13000 DP Digimon"
    );
}

/// 13000 DP exactly is deletable by the fallback (dp_gte:13000 inclusive).
#[test]
fn bt13_111_on_play_fallback_deletes_exactly_13000() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("OPP-13K", 6, 13000))
        .add_card(filler("FILL"))
        .hand(0, &["BT13-111"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    runner.place_on_field(1, "OPP-13K", Some(0));
    let opp_before = runner.battle_area_size(1);

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "exactly 13000 DP is deletable by the fallback (dp_gte:13000 inclusive)"
    );
}

/// Negative isolation: when a <=6000 target IS deleted, the >=13000 Digimon
/// survives (fallback did NOT fire). Stage both a 5000 and a 14000; only the
/// 5000 is deleted.
#[test]
fn bt13_111_on_play_fallback_does_not_fire_when_le_6000_deleted() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("OPP-SMALL", 4, 5000))
        .add_card(make_digimon("OPP-HUGE", 7, 14000))
        .add_card(filler("FILL"))
        .hand(0, &["BT13-111"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    runner.place_on_field(1, "OPP-SMALL", Some(0));
    runner.place_on_field(1, "OPP-HUGE", Some(0));
    let opp_before = runner.battle_area_size(1);

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    // Exactly one Digimon deleted (the <=6000 one); the 14000 survives because
    // the effect already deleted something, so the fallback branch is skipped.
    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "only the <=6000 DP Digimon is deleted; the >=13000 fallback must NOT fire"
    );
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-HUGE"),
        "the 14000 DP Digimon must survive (fallback skipped after a successful delete)"
    );
}

/// The primary delete only offers <=6000 DP opponent Digimon: an 8000 DP
/// opponent Digimon (>6000, <13000) is NOT a legal target for either branch,
/// so nothing is deleted and no prompt installs.
#[test]
fn bt13_111_on_play_does_nothing_against_mid_dp_only() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("OPP-MID", 5, 8000))
        .add_card(filler("FILL"))
        .hand(0, &["BT13-111"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    // 8000 DP: above the <=6000 cap and below the >=13000 fallback → untouchable.
    runner.place_on_field(1, "OPP-MID", Some(0));
    let opp_before = runner.battle_area_size(1);

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_before,
        "an 8000 DP Digimon is in neither delete window → nothing is deleted"
    );
}

/// The primary delete only offers <=6000 DP opponent Digimon; an own <=6000
/// Digimon is never a target (opponent-only).
#[test]
fn bt13_111_on_play_only_targets_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("OWN-SMALL", 4, 3000))
        .add_card(make_digimon("OPP-SMALL", 4, 5000))
        .add_card(filler("FILL"))
        .hand(0, &["BT13-111"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    runner.place_on_field(0, "OWN-SMALL", Some(0));
    runner.place_on_field(1, "OPP-SMALL", Some(0));

    runner.play(0, 0);

    // Only the opponent's <=6000 Digimon is a valid target (1 candidate).
    let view = runner
        .pending_selection_view()
        .expect("delete prompt must install");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the opponent's <=6000 DP Digimon is a valid target (own Digimon excluded); \
         got {} candidates",
        view.valid_action_ids.len()
    );

    let _ = runner.auto_resolve();
    assert!(
        runner.game.player(0).battle_area.iter().any(|p| {
            p.top_card().card_id(&runner.game.card_data) == "OWN-SMALL"
        }),
        "the controller's own 3000 DP Digimon must survive"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — [When Digivolving] delete-with-fallback
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt13_111_when_digivolving_deletes_le_6000_opponent() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_lv5_red("EVOLVE-SRC"))
        .add_card(make_digimon("OPP-SMALL", 4, 5000))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    runner.place_on_field(1, "OPP-SMALL", Some(0));
    let opp_before = runner.battle_area_size(1);

    let stack = runner.place_stack(0, &["EVOLVE-SRC", "BT13-111"]);
    fire_when_digivolving(&mut runner, stack);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "[When Digivolving] must delete the <=6000 DP opponent Digimon"
    );
}

#[test]
fn bt13_111_when_digivolving_fallback_deletes_ge_13000() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_lv5_red("EVOLVE-SRC"))
        .add_card(make_digimon("OPP-HUGE", 7, 14000))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    runner.place_on_field(1, "OPP-HUGE", Some(0));
    let opp_before = runner.battle_area_size(1);

    let stack = runner.place_stack(0, &["EVOLVE-SRC", "BT13-111"]);
    fire_when_digivolving(&mut runner, stack);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "[When Digivolving] no <=6000 target → fallback deletes the >=13000 DP Digimon"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — [When Attacking] delete-with-fallback
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt13_111_when_attacking_deletes_le_6000_opponent() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("OPP-SMALL", 4, 5000))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    // BT13-111 on the field as the attacker.
    let attacker = runner.place_on_field(0, "BT13-111", Some(0));
    runner.place_on_field(1, "OPP-SMALL", Some(0));
    let opp_before = runner.battle_area_size(1);

    // Attack the opponent player to open the When Attacking window.
    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    assert!(
        runner.battle_area_size(1) < opp_before,
        "[When Attacking] must delete the <=6000 DP opponent Digimon"
    );
}

#[test]
fn bt13_111_when_attacking_fallback_deletes_ge_13000() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("OPP-HUGE", 7, 14000))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    let attacker = runner.place_on_field(0, "BT13-111", Some(0));
    runner.place_on_field(1, "OPP-HUGE", Some(0));
    let opp_before = runner.battle_area_size(1);

    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    assert!(
        runner.battle_area_size(1) < opp_before,
        "[When Attacking] no <=6000 target → fallback deletes the >=13000 DP Digimon"
    );
}
