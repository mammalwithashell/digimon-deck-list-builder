//! BT23-101 Hudiemon — Digimon, Lv.4, Green/Yellow, DP 7000, Cost 7.
//! Traits: Insectoid, Hudie, CS.
//!
//! # Card text (BT23-101.json)
//! ＜Alliance＞ ...
//! [On Play] [When Digivolving] You may play 1 play cost 5 or lower [CS] trait
//! card from your hand without paying the cost. Then, to 1 of your opponent's
//! Digimon, give -3000 DP for the turn for each of your [Hudie] trait Digimon.
//! [When Attacking] [Once Per Turn] By returning 1 of your [CS] trait Tamers to
//! the hand, activate 1 of this Digimon's [On Play] effects.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT23/Green/BT23_101.cs
//!
//! # Patterns: H (Alliance keyword grant), C1 (free-play from hand), D
//! (formula DP debuff scaling with a trait count), E2/E3 (OPT activation cost +
//! On-Play re-activation via inline duplication).

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::PendingSelection;
use digimon_engine::{EffectTiming, TriggerSource};

fn hudiemon() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("BT23-101")
}

fn digimon_traits(
    id: &str,
    color: CardColor,
    level: u8,
    dp: i32,
    cost: u16,
    traits: &[&str],
) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = cost;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn tamer_traits(id: &str, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![color];
    c.dp = None;
    c.level = None;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn first_action(p: &PendingSelection) -> u16 {
    *p.valid_action_ids.first().expect("a legal action")
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, h: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(h));
    runner.game.drain_effect_queue();
}

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT23-101")
        .expect("BT23-101 in pack")
        .add_card(digimon_traits(
            "CS-5",
            CardColor::Green,
            4,
            4000,
            5,
            &["CS"],
        )) // legal free-play
        .add_card(digimon_traits(
            "CS-6",
            CardColor::Green,
            5,
            6000,
            6,
            &["CS"],
        )) // cost too high
        .add_card(digimon_traits("NONCS-3", CardColor::Green, 3, 2000, 3, &[])) // not CS
        .add_card(digimon_traits(
            "HUDIE-A",
            CardColor::Green,
            4,
            5000,
            5,
            &["Hudie"],
        )) // extra Hudie
        .add_card(digimon_traits("OPP", CardColor::Blue, 6, 11000, 11, &[])) // debuff target
        .add_card(tamer_traits("CS-TAMER", CardColor::Green, &["CS"]))
        .add_card(tamer_traits("NONCS-TAMER", CardColor::Green, &[]))
        .add_card(digimon_traits("FILLER", CardColor::Blue, 4, 3000, 4, &[]))
        .deck(0, &["FILLER"; 20])
        .deck(1, &["FILLER"; 20])
        .memory(10)
        .start()
}

// ─── SECTION 1 — Structural ─────────────────────────────────────────────────

#[test]
fn bt23_101_metadata_green_yellow_lv4_cs_hudie() {
    let card = hudiemon();
    assert_eq!(card.card, "BT23-101");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(7000));
    let colors: Vec<_> = card.color.iter().copied().collect();
    assert!(colors.contains(&digimon_dsl::compiled::CompiledColor::Green));
    assert!(colors.contains(&digimon_dsl::compiled::CompiledColor::Yellow));
    assert!(card.traits.iter().any(|t| t.eq_ignore_ascii_case("CS")));
    assert!(card.traits.iter().any(|t| t.eq_ignore_ascii_case("Hudie")));
}

#[test]
fn bt23_101_has_alliance_grant_and_two_triggered_clauses() {
    let card = hudiemon();
    // Declarative Alliance grant (a Declarative clause, not Triggered).
    let has_alliance = card.effects.iter().any(
        |c| matches!(c, CompiledClause::Declarative(d) if format!("{d:?}").contains("Alliance")),
    );
    assert!(
        has_alliance,
        "must declare an Alliance grant_keyword clause"
    );

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(
        triggered
            .iter()
            .any(|t| t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)),
        "On Play / When Digivolving clause"
    );
    let wa = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::WhenAttacking))
        .expect("When Attacking clause");
    assert!(
        wa.once_per_turn,
        "[Once Per Turn] on the When Attacking clause"
    );
}

#[test]
fn bt23_101_has_cs_and_conditional_erika_alt_paths() {
    let card = hudiemon();
    let cs4 = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(4))
    });
    let erika3 = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(3))
    });
    assert!(
        cs4,
        "Lv.3 [CS]-trait cost-4 digivolve; got {:?}",
        card.alt_paths
    );
    assert!(
        erika3,
        "[Erika Mishima] cost-3 conditional digivolve; got {:?}",
        card.alt_paths
    );
}

// ─── SECTION 2 — Alliance runtime-installed ─────────────────────────────────

#[test]
fn bt23_101_alliance_keyword_runtime_installed() {
    let mut runner = base_runner();
    let h = runner.place_on_field(0, "BT23-101", Some(0));
    runner.game.tick_declarative_effects();
    assert!(
        runner.game.has_keyword(h, Keyword::Alliance),
        "Alliance must be runtime-installed via the declarative tick"
    );
}

// ─── SECTION 3 — [On Play] play CS<=5 + formula debuff ──────────────────────

#[test]
fn bt23_101_on_play_offers_cs5_free_play() {
    let mut runner = base_runner();
    let h = runner.place_on_field(0, "BT23-101", Some(0));
    runner.game.set_memory(0);
    // Hand: one legal CS<=5, one CS-6 (too costly), one non-CS.
    // (Place into hand via the builder is simpler — rebuild with hand.)
    drop(h);
    let mut runner = base_runner_with_hand(&["CS-5", "CS-6", "NONCS-3"]);
    let h = runner.place_on_field(0, "BT23-101", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, h);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("optional CS free-play prompt installs");
    assert!(pending.is_optional, "'You may play' → optional");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "only the cost<=5 CS card is a legal free-play candidate (CS-6 too costly, NONCS-3 not CS)"
    );
}

#[test]
fn bt23_101_on_play_debuff_scales_with_one_hudie() {
    // Only Hudiemon on field → 1 [Hudie] Digimon → -3000.
    let mut runner = base_runner();
    let h = runner.place_on_field(0, "BT23-101", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0)); // 11000

    fire(&mut runner, EffectTiming::OnPlay, h);
    // Decline the optional CS play (no CS card in hand here anyway).
    if runner
        .game
        .pending_selection
        .as_ref()
        .map_or(false, |p| p.is_optional)
    {
        runner
            .execute_action(0, digimon_engine::action::space::PASS)
            .expect("decline optional play");
    }
    // Mandatory debuff target select.
    let (player, action) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("debuff target select");
        (p.selecting_player, first_action(p))
    };
    runner
        .execute_action(player, action)
        .expect("pick opp target");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.effective_dp(opp),
        Some(8000),
        "11000 - 3000 (1 Hudie = Hudiemon itself) = 8000"
    );
}

#[test]
fn bt23_101_on_play_debuff_scales_with_extra_hudie() {
    let mut runner = base_runner();
    let h = runner.place_on_field(0, "BT23-101", Some(0));
    runner.place_on_field(0, "HUDIE-A", Some(0)); // a 2nd Hudie Digimon
    let opp = runner.place_on_field(1, "OPP", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, h);
    if runner
        .game
        .pending_selection
        .as_ref()
        .map_or(false, |p| p.is_optional)
    {
        runner
            .execute_action(0, digimon_engine::action::space::PASS)
            .expect("decline optional play");
    }
    let (player, action) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("debuff target select");
        (p.selecting_player, first_action(p))
    };
    runner
        .execute_action(player, action)
        .expect("pick opp target");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.effective_dp(opp),
        Some(5000),
        "11000 - 6000 (2 Hudie Digimon) = 5000"
    );
}

#[test]
fn bt23_101_on_play_debuff_expires_end_of_turn() {
    let mut runner = base_runner();
    let h = runner.place_on_field(0, "BT23-101", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, h);
    if runner
        .game
        .pending_selection
        .as_ref()
        .map_or(false, |p| p.is_optional)
    {
        runner
            .execute_action(0, digimon_engine::action::space::PASS)
            .expect("decline optional play");
    }
    let (player, action) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("debuff target");
        (p.selecting_player, first_action(p))
    };
    runner
        .execute_action(player, action)
        .expect("pick opp target");
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(opp), Some(8000));

    runner.end_turn();
    let _ = runner.auto_resolve();
    let opp = runner.game.players[1]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "OPP")
        .map(|i| PermanentHandle {
            player: 1,
            index: i as u8,
        })
        .expect("OPP still on field");
    assert_eq!(
        runner.effective_dp(opp),
        Some(11000),
        "the -3000 debuff expires at end of turn"
    );
}

// ─── SECTION 4 — [When Attacking][OPT] return CS Tamer → re-activate On Play ─

#[test]
fn bt23_101_when_attacking_offers_reactivation_with_cs_tamer() {
    let mut runner = base_runner();
    let h = runner.place_on_field(0, "BT23-101", Some(0));
    runner.place_on_field(0, "CS-TAMER", Some(0));
    runner.place_on_field(1, "OPP", Some(0));

    fire(&mut runner, EffectTiming::WhenAttacking, h);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("When Attacking re-activation prompt installs when a CS Tamer is present");
    assert!(pending.is_optional, "'By returning ... you may' → optional");
}

#[test]
fn bt23_101_when_attacking_no_prompt_without_cs_tamer() {
    let mut runner = base_runner();
    let h = runner.place_on_field(0, "BT23-101", Some(0));
    runner.place_on_field(0, "NONCS-TAMER", Some(0)); // not a CS Tamer
    runner.place_on_field(1, "OPP", Some(0));

    fire(&mut runner, EffectTiming::WhenAttacking, h);

    assert!(
        runner.game.pending_selection.is_none(),
        "no CS Tamer → the When Attacking activation is unavailable"
    );
}

#[test]
fn bt23_101_when_attacking_returns_tamer_and_reactivates_debuff() {
    let mut runner = base_runner_with_hand(&["CS-5"]);
    let h = runner.place_on_field(0, "BT23-101", Some(0));
    runner.place_on_field(0, "CS-TAMER", Some(0));
    runner.place_on_field(1, "OPP", Some(0));

    fire(&mut runner, EffectTiming::WhenAttacking, h);
    // Drive the whole activation chain by accepting every prompt (first
    // non-PASS option): accept → return a CS Tamer (cost) → optional CS
    // free-play → debuff target.
    let mut guard = 0;
    while let Some(view) = runner.pending_selection_view() {
        let pick = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != digimon_engine::action::space::PASS)
            .unwrap_or(digimon_engine::action::space::PASS);
        runner
            .execute_action(view.selecting_player, pick)
            .expect("drive the re-activation chain");
        guard += 1;
        if guard > 12 {
            break;
        }
    }
    let _ = runner.auto_resolve();

    // The re-activated [On Play] applied its -3000 (>=1 Hudie = Hudiemon) debuff.
    let opp = runner.game.players[1]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "OPP")
        .map(|i| PermanentHandle {
            player: 1,
            index: i as u8,
        })
        .expect("OPP still on field");
    assert!(
        runner.effective_dp(opp).unwrap_or(11000) <= 8000,
        "the re-activated [On Play] applies at least -3000 to the opponent (got {:?})",
        runner.effective_dp(opp)
    );
}

#[test]
fn bt23_101_when_attacking_opt_lockout() {
    let mut runner = base_runner();
    let h = runner.place_on_field(0, "BT23-101", Some(0));
    runner.place_on_field(0, "CS-TAMER", Some(0));
    runner.place_on_field(0, "CS-TAMER", Some(0));
    runner.place_on_field(1, "OPP", Some(0));

    // First attack: fully ACTIVATE (declining would NOT consume the OPT slot —
    // only a real activation does). Drive the chain by accepting every prompt.
    fire(&mut runner, EffectTiming::WhenAttacking, h);
    assert!(
        runner.game.pending_selection.is_some(),
        "first When Attacking offers the activation"
    );
    let mut guard = 0;
    while let Some(view) = runner.pending_selection_view() {
        let pick = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != digimon_engine::action::space::PASS)
            .unwrap_or(digimon_engine::action::space::PASS);
        runner
            .execute_action(view.selecting_player, pick)
            .expect("drive first activation");
        guard += 1;
        if guard > 12 {
            break;
        }
    }
    let _ = runner.auto_resolve();

    // Second attack this turn → OPT lockout, no prompt.
    fire(&mut runner, EffectTiming::WhenAttacking, h);
    assert!(
        runner.game.pending_selection.is_none(),
        "[Once Per Turn] blocks a second When Attacking activation the same turn"
    );
}

// ─── helpers that need a custom hand ─────────────────────────────────────────

fn base_runner_with_hand(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT23-101")
        .expect("BT23-101 in pack")
        .add_card(digimon_traits(
            "CS-5",
            CardColor::Green,
            4,
            4000,
            5,
            &["CS"],
        ))
        .add_card(digimon_traits(
            "CS-6",
            CardColor::Green,
            5,
            6000,
            6,
            &["CS"],
        ))
        .add_card(digimon_traits("NONCS-3", CardColor::Green, 3, 2000, 3, &[]))
        .add_card(digimon_traits(
            "HUDIE-A",
            CardColor::Green,
            4,
            5000,
            5,
            &["Hudie"],
        ))
        .add_card(digimon_traits("OPP", CardColor::Blue, 6, 11000, 11, &[]))
        .add_card(tamer_traits("CS-TAMER", CardColor::Green, &["CS"]))
        .add_card(digimon_traits("FILLER", CardColor::Blue, 4, 3000, 4, &[]))
        .hand(0, hand)
        .deck(0, &["FILLER"; 20])
        .deck(1, &["FILLER"; 20])
        .memory(10)
        .start()
}

// ─── Hand-pick candidate legality (select-hand playability audit, 2026-11-13) ──
// "You may play 1 play cost 5 or lower [CS] trait card from your hand." DCGO
// BT23_101.cs:74-78 `CanSelectHandCardCondition` = `HasPlayCost && BasePlayCost
// <= 5 && HasCSTraits`, where `HasPlayCost` (CEntity_Base.cs:340) is false for an
// Option (and a DUAL, whose kinds include Option) and for a Digi-Egg (no play
// cost). CR 4.3 7-1-1: playing places "a new card with a play cost" onto the
// field — a Digi-Egg (Wanyamon BT22-004) has none and an Option (Kuremi
// Detective Agency BT22-099) is used, not played; a DUAL can't be played
// (4-6-2). DCGO does not re-check the free frame or play prohibitions here
// (`PlayPermanentCards` then refuses), which offers a choice that does nothing;
// CR 4.3 1-3-2 / 15-1-3 decide — those cards are not offered either.

fn cs_egg(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::DigiEgg;
    c.level = Some(2);
    c.dp = None;
    c.play_cost = 0;
    c.traits = vec!["CS".to_string()];
    c
}

fn cs_option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Green];
    c.traits = vec!["CS".to_string()];
    c
}

fn pick_runner(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-101")
        .expect("BT23-101 in pack")
        .add_card(digimon_traits("CS-5", CardColor::Green, 4, 4000, 5, &["CS"]))
        .add_card(digimon_traits("OPP", CardColor::Blue, 6, 11000, 11, &[]))
        .add_card(tamer_traits("CS-TAMER", CardColor::Green, &["CS"]))
        .add_card(cs_egg("CS-EGG"))
        .add_card(cs_option("CS-OPT"))
        .add_card(digimon_traits("FILLER", CardColor::Blue, 4, 3000, 4, &[]))
        .hand(0, hand)
        .deck(0, &["FILLER"; 20])
        .deck(1, &["FILLER"; 20])
        .memory(10)
        .start();
    let h = runner.place_on_field(0, "BT23-101", Some(0));
    runner.place_on_field(1, "OPP", Some(0));
    (runner, h)
}

/// Fire [On Play] and return the hand-pick candidates' card ids (`None` when the
/// first prompt is not the hand pick, i.e. no candidate exists).
fn offered_on_play(runner: &mut DebugRunner, h: PermanentHandle) -> Option<Vec<String>> {
    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    fire(runner, EffectTiming::OnPlay, h);
    let view = runner.pending_selection_view()?;
    if view.kind != digimon_engine::selection::SelectionKind::Hand {
        return None;
    }
    let hand: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let mut ids: Vec<String> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .map(|a| hand[(a - PLAY_HAND_START) as usize].clone())
        .collect();
    ids.sort();
    Some(ids)
}

/// Regression guard: a [CS] Digimon and a [CS] Tamer are both playable picks.
#[test]
fn bt23_101_on_play_offers_cs_digimon_and_cs_tamer() {
    let (mut runner, h) = pick_runner(&["CS-5", "CS-TAMER"]);
    assert_eq!(
        offered_on_play(&mut runner, h),
        Some(vec!["CS-5".to_string(), "CS-TAMER".to_string()])
    );
}

/// A [CS] Digi-Egg (e.g. Wanyamon BT22-004) has no play cost: never offered.
#[test]
fn bt23_101_on_play_excludes_cs_digi_egg() {
    let (mut runner, h) = pick_runner(&["CS-EGG", "CS-5"]);
    assert_eq!(offered_on_play(&mut runner, h), Some(vec!["CS-5".to_string()]));
}

/// A [CS] Option (e.g. Kuremi Detective Agency BT22-099) is used, not played:
/// never offered.
#[test]
fn bt23_101_on_play_excludes_cs_option() {
    let (mut runner, h) = pick_runner(&["CS-OPT", "CS-5"]);
    assert_eq!(offered_on_play(&mut runner, h), Some(vec!["CS-5".to_string()]));
}

/// "You can't play Digimon by effects": the [CS] Digimon is not offered, the
/// [CS] Tamer still is.
#[test]
fn bt23_101_on_play_excludes_digimon_when_effect_play_is_prohibited() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let (mut runner, h) = pick_runner(&["CS-5", "CS-TAMER"]);
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
    );
    assert_eq!(offered_on_play(&mut runner, h), Some(vec!["CS-TAMER".to_string()]));
}

/// A full battle area: nothing can be played, so no hand pick is offered and
/// the effect goes straight on to the -DP target.
#[test]
fn bt23_101_on_play_offers_no_hand_pick_when_battle_area_full() {
    let (mut runner, h) = pick_runner(&["CS-5", "CS-TAMER"]);
    runner.game.rules.field_slots = runner.game.players[0].battle_area.len() as u8;
    let offered = offered_on_play(&mut runner, h).unwrap_or_default();
    assert!(offered.is_empty(), "no playable candidate in a full battle area: {offered:?}");
}

/// The [When Attacking] re-activation runs its own inline copy of the [On Play]
/// body, so its hand pick needs the same filter: after returning the [CS] Tamer,
/// a [CS] Digi-Egg and a [CS] Option are still not offered (CR 4.3 7-1-1), while
/// the [CS] Digimon and the just-returned [CS] Tamer are.
#[test]
fn bt23_101_when_attacking_reactivated_pick_excludes_cs_egg_and_option() {
    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    use digimon_engine::selection::SelectionKind;
    let (mut runner, h) = pick_runner(&["CS-EGG", "CS-OPT", "CS-5"]);
    runner.place_on_field(0, "CS-TAMER", Some(0));
    fire(&mut runner, EffectTiming::WhenAttacking, h);
    // Accept the activation and pay its cost (return the [CS] Tamer) until the
    // re-activated body's hand pick is up.
    let mut guard = 0;
    let view = loop {
        let view = runner
            .pending_selection_view()
            .expect("the re-activated [On Play] reaches its hand pick");
        if view.kind == SelectionKind::Hand {
            break view;
        }
        let pick = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .expect("a non-pass option");
        runner
            .execute_action(view.selecting_player, pick)
            .expect("drive the activation cost");
        guard += 1;
        assert!(guard < 8, "never reached the hand pick");
    };
    let hand: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let mut ids: Vec<String> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .map(|a| hand[(a - PLAY_HAND_START) as usize].clone())
        .collect();
    ids.sort();
    assert_eq!(ids, vec!["CS-5".to_string(), "CS-TAMER".to_string()]);
}

// ─── -DP count: "each of your [Hudie] trait Digimon" (Tamers excluded) ───────
// DCGO BT23_101.cs `HudieDigimonAmount` filters
// `IsPermanentExistsOnOwnerBattleAreaDigimon && HasHudieTraits`, and the card
// prints "[Hudie] trait Digimon": a [Hudie] Tamer (the Erika Mishima line) is a
// permanent in the battle area but not a Digimon, so it adds nothing.

fn hudie_tamer_runner() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-101")
        .expect("BT23-101 in pack")
        .add_card(digimon_traits("OPP", CardColor::Blue, 6, 11000, 11, &[]))
        .add_card(tamer_traits("HUDIE-TAMER", CardColor::Green, &["Hudie"]))
        .add_card(digimon_traits("FILLER", CardColor::Blue, 4, 3000, 4, &[]))
        .deck(0, &["FILLER"; 20])
        .deck(1, &["FILLER"; 20])
        .memory(10)
        .start();
    let h = runner.place_on_field(0, "BT23-101", Some(0));
    runner.place_on_field(0, "HUDIE-TAMER", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));
    (runner, h, opp)
}

#[test]
fn bt23_101_on_play_debuff_does_not_count_hudie_tamers() {
    let (mut runner, h, opp) = hudie_tamer_runner();
    fire(&mut runner, EffectTiming::OnPlay, h);
    // No [CS] card in hand: the first prompt is the -DP target.
    let (player, action) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("debuff target select");
        assert!(!p.is_optional, "no hand pick without a [CS] candidate");
        (p.selecting_player, first_action(p))
    };
    runner
        .execute_action(player, action)
        .expect("pick opp target");
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.effective_dp(opp),
        Some(8000),
        "11000 - 3000 (Hudiemon only; the [Hudie] Tamer is not a Digimon) = 8000"
    );
}
