//! BT13-100 Yoshino Fujieda — Tamer, Green, Cost 4.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT13-100.md)
//!   [Start of Your Turn] If you have 2 or less memory, set it to 3.
//!   [Your Turn] When any of your Digimon digivolve into Digimon with
//!   [Vegetation], [Plant] or [Fairy] in any of their traits, by suspending
//!   this Tamer, gain 1 memory.
//!   [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//!   DCGO/Assets/Scripts/CardEffect/BT13/Green/BT13_100.cs —
//!   SetMemoryTo3TamerEffect; OnEnterFieldAnyone optional ActivateClass
//!   (IsOwnerTurn, CanTriggerWhenPermanentDigivolving(own Digimon, TopCard
//!   HasPlantTraits || HasFairyTraits), CanActivateSuspendCostEffect) → tap
//!   self → AddMemory(1); PlaySelfTamerSecurityEffect.
//!
//! # Patterns
//! - Start-of-your-turn memory floor (set to 3) — positive / negative
//! - `on_digivolve` observer, optional suspend-self activation cost —
//!   accept / decline / trait-gate negative / opponent-turn negative /
//!   already-suspended negative
//! - [Security] play self free (real attack → security check)

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "BT13-100";

fn make_base(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(level);
    c.dp = Some(3000);
    c
}

/// A Lv.(base+1) green Digimon with `traits`, digivolvable from a green
/// Lv.`base_level` Digimon at cost 0.
fn make_evo(id: &str, level: u8, base_level: u8, traits: &[&str]) -> CardData {
    let mut c = make_base(id, level);
    c.dp = Some(5000);
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c.evo_costs = vec![EvoCost {
        card_color: 3, // Green
        level: base_level,
        memory_cost: 0,
    }];
    c
}

fn runner_with_evo(evo_traits: &[&str], memory: i16) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-100 in embedded DSL pack")
        .add_card(make_base("BASE-L3", 3))
        .add_card(make_evo("EVO-L4", 4, 3, evo_traits))
        .add_card(make_test_card("DRAW", "Draw"))
        .deck(0, &["DRAW", "DRAW", "DRAW"])
        .deck(1, &["DRAW", "DRAW", "DRAW"])
        .hand(0, &["EVO-L4"])
        .memory(memory)
        .start()
}

/// Digivolve P0's BASE-L3 into EVO-L4 through the real digivolve flow.
fn digivolve(runner: &mut DebugRunner, base: PermanentHandle) {
    let evo_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "EVO-L4")
        .expect("EVO-L4 in hand");
    assert!(
        runner
            .game
            .digivolve_from_hand(0, evo_idx, base.index as usize, PlaySource::ByDigivolve),
        "digivolve must succeed"
    );
}

fn is_suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.player(h.player).battle_area[h.index as usize].is_suspended
}

// ── Structural ───────────────────────────────────────────────────────────────

#[test]
fn bt13_100_has_three_clauses_with_expected_shapes() {
    let card = compiled(CARD_ID);
    assert_eq!(card.kind, digimon_dsl::compiled::CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(4));
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 3);

    let sot = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::StartOfYourTurn))
        .expect("start-of-your-turn clause");
    assert!(sot.process.iter().any(|s| matches!(s, CompiledStep::SetMemory(3))));

    let digi = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnDigivolve))
        .expect("on_digivolve observer");
    assert!(digi.optional, "'by suspending this Tamer' is an optional cost");
    assert!(!digi.once_per_turn, "no [Once Per Turn] printed");
    assert!(digi
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::GainMemory(1))));

    let sec = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("security clause");
    assert!(sec
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlayFromSecurity)));
}

// ── Clause 1: [Start of Your Turn] memory floor ──────────────────────────────

#[test]
fn bt13_100_start_of_turn_sets_two_memory_to_three() {
    let mut runner = runner_with_evo(&[], 2);
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    runner
        .game
        .enqueue_triggered(EffectTiming::StartOfYourTurn, TriggerSource::Permanent(tamer));
    runner.game.drain_effect_queue();
    assert_eq!(runner.memory(), 3);
}

#[test]
fn bt13_100_start_of_turn_leaves_three_or_more_memory() {
    let mut runner = runner_with_evo(&[], 4);
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    runner
        .game
        .enqueue_triggered(EffectTiming::StartOfYourTurn, TriggerSource::Permanent(tamer));
    runner.game.drain_effect_queue();
    assert_eq!(runner.memory(), 4, "memory > 2 fails the gate");
}

// ── Clause 2: [Your Turn] digivolve observer ─────────────────────────────────

#[test]
fn bt13_100_digivolve_into_vegetation_accept_suspends_and_gains_one() {
    let mut runner = runner_with_evo(&["Vegetation"], 5);
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE-L3", Some(0));
    digivolve(&mut runner, base);
    let mem_before = runner.memory();

    assert!(
        runner.pending_is_optional(),
        "'by suspending this Tamer' must surface an optional prompt"
    );
    runner.accept_optional_trigger().expect("accept");
    runner.auto_resolve().expect("resolve");

    assert!(is_suspended(&runner, tamer), "Tamer suspended as cost");
    assert_eq!(runner.memory(), mem_before + 1);
}

#[test]
fn bt13_100_digivolve_into_compound_plant_trait_fires() {
    let mut runner = runner_with_evo(&["Carnivorous Plant"], 5);
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE-L3", Some(0));
    digivolve(&mut runner, base);
    let mem_before = runner.memory();
    assert!(runner.pending_is_optional(), "[Plant] in any trait qualifies");
    runner.accept_optional_trigger().expect("accept");
    runner.auto_resolve().expect("resolve");
    assert!(is_suspended(&runner, tamer));
    assert_eq!(runner.memory(), mem_before + 1);
}

#[test]
fn bt13_100_digivolve_into_fairy_fires() {
    let mut runner = runner_with_evo(&["Fairy"], 5);
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE-L3", Some(0));
    digivolve(&mut runner, base);
    let mem_before = runner.memory();
    assert!(runner.pending_is_optional(), "[Fairy] qualifies");
    runner.accept_optional_trigger().expect("accept");
    runner.auto_resolve().expect("resolve");
    assert!(is_suspended(&runner, tamer));
    assert_eq!(runner.memory(), mem_before + 1);
}

#[test]
fn bt13_100_decline_leaves_tamer_unsuspended_and_memory_unchanged() {
    let mut runner = runner_with_evo(&["Fairy"], 5);
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE-L3", Some(0));
    digivolve(&mut runner, base);
    let mem_before = runner.memory();
    assert!(runner.pending_is_optional());
    runner.decline_optional_trigger().expect("decline");
    runner.auto_resolve().expect("settle");
    assert!(!is_suspended(&runner, tamer));
    assert_eq!(runner.memory(), mem_before);
}

#[test]
fn bt13_100_non_matching_trait_digivolve_does_not_fire() {
    let mut runner = runner_with_evo(&["Beast"], 5);
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE-L3", Some(0));
    digivolve(&mut runner, base);
    let mem_before = runner.memory();
    assert!(runner.pending_selection().is_none(), "no prompt for a [Beast] result");
    let _ = runner.auto_resolve();
    assert!(!is_suspended(&runner, tamer));
    assert_eq!(runner.memory(), mem_before);
}

#[test]
fn bt13_100_already_suspended_tamer_cannot_pay_cost() {
    let mut runner = runner_with_evo(&["Vegetation"], 5);
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].battle_area[tamer.index as usize].is_suspended = true;
    let base = runner.place_on_field(0, "BASE-L3", Some(0));
    digivolve(&mut runner, base);
    let mem_before = runner.memory();
    assert!(
        runner.pending_selection().is_none(),
        "suspended Tamer cannot pay the suspend cost → no prompt"
    );
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), mem_before);
}

#[test]
fn bt13_100_opponents_digivolve_on_their_turn_does_not_fire() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-100 in embedded DSL pack")
        .add_card(make_base("BASE-L3", 3))
        .add_card(make_evo("EVO-L4", 4, 3, &["Vegetation"]))
        .add_card(make_test_card("DRAW", "Draw"))
        .deck(0, &["DRAW", "DRAW", "DRAW"])
        .deck(1, &["DRAW", "DRAW", "DRAW"])
        .hand(1, &["EVO-L4"])
        .memory(5)
        .start();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 1);
    let base = runner.place_on_field(1, "BASE-L3", Some(1));
    let evo_idx = runner.game.players[1]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "EVO-L4")
        .expect("EVO-L4 in P1 hand");
    let _ = runner
        .game
        .digivolve_from_hand(1, evo_idx, base.index as usize, PlaySource::ByDigivolve);
    if let Some(v) = runner.pending_selection_view() {
        assert_ne!(
            v.selecting_player, 0,
            "Yoshino must not prompt P0 for an opponent's digivolve"
        );
    }
    let _ = runner.auto_resolve();
    assert!(!is_suspended(&runner, tamer), "Yoshino stays unsuspended");
}

// ── Clause 3: [Security] play self free ──────────────────────────────────────

#[test]
fn bt13_100_security_check_plays_self_for_free() {
    let mut attacker = make_test_card("ATTACKER", "Attacker");
    attacker.card_kind = CardKind::Digimon;
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-100 in embedded DSL pack")
        .add_card(attacker)
        .add_card(make_test_card("DRAW", "Draw"))
        .security(1, &[CARD_ID])
        .deck(0, &["DRAW"])
        .deck(1, &["DRAW"])
        .memory(0)
        .start();
    let atk = runner.place_on_field(0, "ATTACKER", Some(0));
    let mem_before = runner.memory();

    let result = runner.attack_player(atk, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert!(
        runner
            .game
            .player(1)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "Yoshino must be played to P1's battle area"
    );
    assert_eq!(runner.memory(), mem_before, "played without paying the cost");
}
