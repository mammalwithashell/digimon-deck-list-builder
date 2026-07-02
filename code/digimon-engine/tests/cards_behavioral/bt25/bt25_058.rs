//! BT25-058 Callismon — Digimon, Lv.6, Green/Black, DP 13000, Cost 13.
//! Traits: Dark Animal, Iliad, TS. Attribute: Virus.
//! Evo (printed): Lv.5 Green / Cost 5 (+ DCGO self-digivolution-requirement on
//! a Lv.5 [TS]-trait base, cost 4, ignoring requirements — printed xros_req box
//! "[Digivolve] Lv.5 w/[TS] trait: Cost 4").
//!
//! # Card text (cards.json — verbatim)
//!
//! ＜Reboot＞ (This Digimon also unsuspends in your opponent's unsuspend phase.)
//! ＜Blocker＞ (This Digimon can block in the blocker timing.)
//! ＜Fortitude＞ (When this Digimon with digivolution cards is deleted, play this
//! card without paying the cost.)
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] You may suspend
//! 1 of your opponent's Digimon or Tamers. Then, 1 of their Digimon or Tamers
//! can't unsuspend until their turn ends.
//! [All Turns] [Once Per Turn] When effects play or digivolve any Digimon,
//! ＜De-Digivolve 1＞ 1 of your opponent's Digimon. (Trash the top card. You
//! can't trash past level 3 cards.) Then, this Digimon may battle 1 of your
//! opponent's Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_058.cs
//!
//! - Region "Alt Digivolution" (None): AddSelfDigivolutionRequirementStatic-
//!   Effect(permanentCondition: TopCard.HasTSTraits, digivolutionCost: 4,
//!   ignoreDigivolutionRequirement: true, level: 5).
//! - Region "Reboot"/"Blocker": Reboot/BlockerSelfStaticEffect (own keywords).
//! - Region "Fortitude" (OnDestroyedAnyone): FortitudeSelfEffect.
//! - Region "Shared OP/WD/WA" (onPlay+whenDigivolving+whenAttacking,
//!   maxCountPerTurn: 1, optional: false outer): body picks an OPTIONAL opp
//!   Digimon/Tamer to suspend (canNoSelect: true), then a MANDATORY opp
//!   Digimon/Tamer to gain CannotUnsuspend until end of their turn.
//! - Region "All Turns" (OnEnterFieldAnyone, oncePerTurn 1): CanUseCondition
//!   gates on (play OR digivolve of a battle-area Digimon) && IsByEffect. Body:
//!   De-Digivolve 1 a MANDATORY opp Digimon (canNoSelect: false), then an
//!   OPTIONAL battle: this Digimon battles 1 opp Digimon (canNoSelect: true).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - H5/H7 Blocker + Reboot keywords; F-keyword Fortitude
//! - E2/E3 OPT shared clause across OnPlay/WhenDigivolving/WhenAttacking
//! - F7 cannot-unsuspend lock
//! - on-effect-play/digivolve observer + De-Digivolve + effect battle

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, Keyword, ModifierType, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT25-058";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn make_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card.evo_costs = vec![EvoCost {
        card_color: 0,
        level: level.saturating_sub(1),
        memory_cost: 2,
    }];
    card
}

fn make_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-058 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("OPP-LV4", 4, 5000, &["Beast"]))
        .add_card(make_digimon("OPP-LV3", 3, 3000, &["Beast"]))
        .add_card(make_digimon("OWN-PLAY", 4, 4000, &["Beast"]))
        .add_card(make_tamer("OPP-TAMER"))
        .add_card(make_digimon("TS-LV5", 5, 9000, &["TS"]))
        .deck(0, &["DECK-PAD"; 10])
        .deck(1, &["DECK-PAD"; 10])
}

fn place_callismon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_on_field(0, CARD_ID, Some(0))
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_058_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-058 must be present in embedded DSL pack");

    assert_eq!(card.name, "Callismon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(13));
    assert_eq!(card.dp, Some(13000));
    for trait_name in ["Dark Animal", "Iliad", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-058 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_058_grants_reboot_blocker_fortitude() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let granted: Vec<String> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) => Some(keyword.clone()),
            _ => None,
        })
        .collect();

    for kw in ["Reboot", "Blocker", "Fortitude"] {
        assert!(
            granted.iter().any(|g| g.eq_ignore_ascii_case(kw)),
            "BT25-058 must grant <{kw}> (granted: {granted:?})"
        );
    }
}

#[test]
fn bt25_058_has_ts_lv5_cost4_ignore_alt_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let path = card
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(4))
        })
        .expect("BT25-058 must include a cost-4 digivolve alt-path");

    assert!(
        path.ignore_requirements,
        "the Lv.5 [TS] alt-path ignores digivolution requirements"
    );
    let from = path.from.as_ref().expect("alt-path has a from: filter");
    assert_eq!(from.level_eq, Some(5), "alt-path base must be Lv.5");
    assert_eq!(
        from.trait_has.as_deref(),
        Some("TS"),
        "alt-path base must carry the [TS] trait"
    );
}

#[test]
fn bt25_058_shared_clause_covers_op_wd_wa_and_is_opt() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT25-058 must have a shared OnPlay/WhenDigivolving/WhenAttacking clause");

    assert!(
        clause.once_per_turn,
        "printed [Once Per Turn] → once_per_turn"
    );
    let has_lock = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::AddModifier { .. }));
    assert!(
        has_lock,
        "shared clause must add a CannotUnsuspend modifier to an opp permanent"
    );
}

#[test]
fn bt25_058_has_all_turns_dedigivolve_battle_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.process
                    .iter()
                    .any(|s| matches!(s, CompiledStep::DeDigivolve { .. })) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT25-058 must have an [All Turns] de-digivolve clause");

    assert!(clause.once_per_turn, "[Once Per Turn]");
    assert!(
        clause.when.contains(&CompiledTiming::OnEnterFieldAnyone)
            || clause.when.contains(&CompiledTiming::OnDigivolve),
        "[All Turns] clause must observe play/digivolve timings"
    );
    let has_battle = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::Battle { .. }));
    assert!(
        has_battle,
        "[All Turns] clause must end with a may-battle step"
    );
    // The effect-initiated gate is the structural correctness point.
    let condition = clause
        .active_when
        .as_ref()
        .or(clause.condition.as_ref())
        .expect("All-Turns clause must carry a gate");
    let mentions_effect_gate = condition.event_is_effect_initiated == Some(true)
        || condition
            .all_of
            .iter()
            .any(|p| p.event_is_effect_initiated == Some(true));
    assert!(
        mentions_effect_gate,
        "[All Turns] clause must gate on event_is_effect_initiated: true (DCGO IsByEffect)"
    );
}

// ─── Section 2 — Behavior: keywords ──────────────────────────────────────────

#[test]
fn bt25_058_has_blocker_and_reboot_keywords_on_field() {
    let mut runner = base().memory(13).start();
    let callismon = place_callismon(&mut runner);

    assert!(
        runner.game.has_keyword(callismon, Keyword::Blocker),
        "Callismon must have <Blocker> on the field"
    );
    assert!(
        runner.game.has_keyword(callismon, Keyword::Reboot),
        "Callismon must have <Reboot> on the field"
    );
}

// ─── Section 3 — Behavior: shared OP/WD/WA suspend + cant-unsuspend ───────────

/// Positive: with an opponent Digimon present, playing Callismon installs the
/// shared clause prompt (suspend / lock). This is the OnPlay arm.
#[test]
fn bt25_058_on_play_installs_prompt() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(13).start();
    runner.place_on_field(1, "OPP-LV4", Some(0));

    let _c = runner.play(0, 0).expect("play Callismon");

    assert!(
        runner.pending_selection().is_some(),
        "OnPlay arm of the shared clause must install a prompt (suspend / lock opp permanent)"
    );
}

/// Negative: no opponent Digimon or Tamer → the shared clause has no legal
/// target and installs no prompt.
#[test]
fn bt25_058_on_play_no_prompt_without_opponent_permanent() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(13).start();

    let _c = runner.play(0, 0).expect("play Callismon");

    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon/Tamer → shared clause is a no-op"
    );
}

/// The lock target gains CannotUnsuspend.
#[test]
fn bt25_058_locks_an_opponent_permanent_from_unsuspending() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(13).start();
    let opp = runner.place_on_field(1, "OPP-LV4", Some(0));

    let _c = runner.play(0, 0).expect("play Callismon");
    assert!(runner.pending_selection().is_some(), "prompt installed");
    runner.auto_resolve().expect("resolve suspend + lock picks");

    assert!(
        runner.modifiers().has(opp, ModifierType::CannotUnsuspend),
        "the chosen opponent permanent must gain CannotUnsuspend"
    );
}

// ─── Section 4 — Behavior: [All Turns] effect-play observer ───────────────────

/// Positive: while Callismon is on the field, when a Digimon is played BY EFFECT,
/// the [All Turns] clause installs the De-Digivolve prompt over opponent Digimon.
#[test]
fn bt25_058_all_turns_fires_on_effect_play_of_a_digimon() {
    let mut runner = base().hand(0, &["OWN-PLAY"]).memory(13).start();
    let _c = place_callismon(&mut runner);
    // De-Digivolve target must exist (opp Digimon with digivolution cards).
    let _opp = runner.place_stack(1, &["OPP-LV3", "OPP-LV4"]);

    let played = runner.game.players[0].hand[0].handle();
    runner.game.play_card_from_effect_without_cost(0, played);

    assert!(
        runner.pending_selection().is_some(),
        "[All Turns] clause must install a De-Digivolve prompt when a Digimon is played by effect"
    );
}

/// Negative: a normal (hand) play of a Digimon does NOT trigger the [All Turns]
/// clause — it is gated on `event_is_effect_initiated`.
#[test]
fn bt25_058_all_turns_no_fire_on_normal_hand_play() {
    use digimon_engine::enums::{CostDelta, PlaySource};

    let mut runner = base().hand(0, &["OWN-PLAY"]).memory(13).start();
    let _c = place_callismon(&mut runner);
    let _opp = runner.place_stack(1, &["OPP-LV3", "OPP-LV4"]);

    runner
        .game
        .play_from_hand_with_cost(0, 0, CostDelta::Free, PlaySource::ByHand);

    assert!(
        runner.pending_selection().is_none(),
        "normal hand play is not effect-initiated → [All Turns] clause must not fire"
    );
}
