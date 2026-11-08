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
        .trigger_if
        .as_ref()
        .expect("All-Turns clause must carry a trigger qualifier");
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

/// Q1643's independent "Then" processing: declining the optional suspend
/// must still offer a mandatory lock (DCGO BT25_058 Shared OP / WD / WA).
#[test]
fn bt25_058_declined_suspend_still_requires_lock() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    let digimon = runner.place_on_field(1, "OPP-LV4", Some(0));
    let tamer = runner.place_on_field(1, "OPP-TAMER", Some(1));
    let play = digimon_engine::action::space::PLAY_HAND_START;
    assert_eq!(
        digimon_engine::action::build_action_mask(&runner.game, 0)[play as usize],
        1.0
    );
    runner.game.decode_action(play, 0);
    let pass = digimon_engine::action::space::PASS;
    let selection = runner.pending_selection().expect("optional suspend");
    assert!(selection.is_optional);
    assert_eq!(selection.valid_action_ids.len(), 2);
    assert!(digimon_engine::action::mask::build_action_mask(&runner.game, 0)[pass as usize] > 0.0);
    runner.execute_action(0, pass).expect("decline suspend");
    let selection = runner
        .pending_selection()
        .expect("mandatory Then lock after decline");
    assert!(!selection.is_optional);
    assert_eq!(selection.valid_action_ids.len(), 2);
    assert_eq!(
        digimon_engine::action::mask::build_action_mask(&runner.game, 0)[pass as usize],
        0.0
    );
    let action = selection.valid_action_ids[1];
    runner.execute_action(0, action).expect("lock tamer");
    assert!(!runner.game.players[1].battle_area[0].is_suspended);
    assert!(!runner.game.players[1].battle_area[1].is_suspended);
    assert!(!runner
        .modifiers()
        .has(digimon, ModifierType::CannotUnsuspend));
    assert!(runner.modifiers().has(tamer, ModifierType::CannotUnsuspend));
}

// ─── Section 4 — Behavior: [All Turns] effect-play observer ───────────────────

fn decline_suspend_and_lock_first(runner: &mut DebugRunner) {
    let pass = digimon_engine::action::space::PASS;
    let pick = runner.pending_selection().expect("suspend choice");
    assert!(pick.is_optional);
    assert!(pick.prompt.contains("suspend"), "{:?}", pick);
    runner.execute_action(0, pass).expect("decline suspend");
    let pick = runner
        .pending_selection()
        .expect("independent mandatory lock");
    assert!(!pick.is_optional);
    assert_eq!(
        digimon_engine::action::build_action_mask(&runner.game, 0)[pass as usize],
        0.0
    );
    let action = pick.valid_action_ids[0];
    runner.execute_action(0, action).expect("choose lock");
}

#[test]
fn bt25_058_selected_suspend_and_lock_can_choose_different_targets() {
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    let host = r.place_on_field(0, "TS-LV5", None);
    let digimon = r.place_on_field(1, "OPP-LV4", None);
    let tamer = r.place_on_field(1, "OPP-TAMER", None);
    let evolve = digimon_engine::action::space::encode_digivolve(0, host.index as u16);
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[evolve as usize],
        1.0
    );
    r.game.decode_action(evolve, 0);
    let suspend = r.pending_selection().unwrap().valid_action_ids[0];
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[suspend as usize],
        1.0
    );
    r.execute_action(0, suspend).unwrap();
    let lock = r
        .pending_selection()
        .expect("lock after selected suspension");
    assert!(!lock.is_optional);
    let action = lock.valid_action_ids[1];
    r.execute_action(0, action).unwrap();
    assert!(r.game.players[1].battle_area[0].is_suspended);
    assert!(!r.game.players[1].battle_area[1].is_suspended);
    assert!(!r.modifiers().has(digimon, ModifierType::CannotUnsuspend));
    assert!(r.modifiers().has(tamer, ModifierType::CannotUnsuspend));
}

#[test]
fn bt25_058_decline_lock_survives_clone_and_expires_after_opponent_turn() {
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    let host = r.place_on_field(0, "TS-LV5", None);
    let opp = r.place_on_field(1, "OPP-LV4", None);
    r.game.players[1].battle_area[0].is_suspended = true;
    let evolve = digimon_engine::action::space::encode_digivolve(0, host.index as u16);
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[evolve as usize],
        1.0
    );
    r.game.decode_action(evolve, 0);
    // Fork at the optional selection: the clone must own its continuation.
    let mut fork = DebugRunner::wrap(r.game.clone());
    decline_suspend_and_lock_first(&mut fork);
    assert!(r.pending_selection().unwrap().is_optional);
    assert!(!r.modifiers().has(opp, ModifierType::CannotUnsuspend));
    assert!(fork.modifiers().has(opp, ModifierType::CannotUnsuspend));
    fork.end_turn();
    assert_eq!(fork.game.turn_player(), 1);
    assert!(fork.game.players[1].battle_area[0].is_suspended);
    assert!(fork.modifiers().has(opp, ModifierType::CannotUnsuspend));
    fork.end_turn();
    assert!(!fork.modifiers().has(opp, ModifierType::CannotUnsuspend));
}

#[test]
fn bt25_058_already_suspended_pick_still_requires_lock() {
    let mut r = base().hand(0, &[CARD_ID]).memory(13).start();
    let opp = r.place_on_field(1, "OPP-LV4", None);
    r.game.players[1].battle_area[0].is_suspended = true;
    r.play(0, 0).unwrap();
    let pick = r.pending_selection().expect("optional suspend");
    assert!(pick.is_optional);
    assert_eq!(pick.valid_action_ids.len(), 1);
    let action = pick.valid_action_ids[0];
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[action as usize],
        1.0
    );
    r.execute_action(0, action).unwrap();
    let pick = r
        .pending_selection()
        .expect("mandatory lock despite no new suspension");
    assert!(!pick.is_optional);
    let action = pick.valid_action_ids[0];
    r.execute_action(0, action).unwrap();
    assert!(r.game.players[1].battle_area[0].is_suspended);
    assert!(r.modifiers().has(opp, ModifierType::CannotUnsuspend));
}

#[test]
fn bt25_058_when_digivolving_decline_still_locks() {
    use digimon_engine::action::{build_action_mask, space::encode_digivolve};
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    let host = r.place_on_field(0, "TS-LV5", None);
    let opp = r.place_on_field(1, "OPP-TAMER", None);
    let action = encode_digivolve(0, host.index as u16);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        CARD_ID
    );
    decline_suspend_and_lock_first(&mut r);
    assert!(r.modifiers().has(opp, ModifierType::CannotUnsuspend));
}

#[test]
fn bt25_058_when_attacking_decline_still_locks() {
    use digimon_engine::action::{
        build_action_mask,
        space::{encode_attack, SECURITY_TARGET},
    };
    let mut r = base().security(1, &["DECK-PAD"]).memory(5).start();
    let callismon = place_callismon(&mut r);
    let opp = r.place_on_field(1, "OPP-TAMER", None);
    r.game.players[0].battle_area[0].turn_played = 0;
    r.set_turn(2);
    let action = encode_attack(callismon.index as u16, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    decline_suspend_and_lock_first(&mut r);
    assert!(r.modifiers().has(opp, ModifierType::CannotUnsuspend));
}

/// CR 4.3 §16-27: deletion with sources replays Callismon as a new card;
/// its new On Play still has the mandatory lock after declining suspension.
#[test]
fn bt25_058_fortitude_replay_decline_still_locks() {
    let mut r = base().memory(5).start();
    let callismon = r.place_stack(0, &["TS-LV5", CARD_ID]);
    let opp = r.place_on_field(1, "OPP-TAMER", None);
    r.game.delete_permanent_with_effects(callismon);
    r.game.drain_effect_queue();
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        CARD_ID
    );
    let order = r.pending_selection().expect("replay trigger order");
    assert_eq!(order.kind, SelectionKind::TriggerOrder);
    let action = order
        .effect_choices
        .as_ref()
        .expect("named simultaneous effects")
        .iter()
        .find(|choice| choice.timing == Some(digimon_engine::enums::EffectTiming::OnPlay))
        .expect("choose the replay's On Play effect")
        .action_id;
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[action as usize],
        1.0
    );
    r.execute_action(0, action).unwrap();
    decline_suspend_and_lock_first(&mut r);
    assert!(r.modifiers().has(opp, ModifierType::CannotUnsuspend));
}

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
