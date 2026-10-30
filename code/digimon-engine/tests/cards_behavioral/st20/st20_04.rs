//! ST20-04 Garudamon — Lv.5, Red, DP 7000, Cost 7.
//! Traits: Birdkin, ADVENTURE. Form: Ultimate. Attribute: Vaccine.
//!
//! # Printed text (official Bandai DB — data/card_bundles/ST20-04.md)
//!
//! Digivolve: Red Lv.4 / Cost 3; [Digivolve] Lv.4 w/[ADVENTURE] trait: Cost 3
//!
//! [On Play] [When Digivolving] For the turn, 1 of your Digimon gains
//!   <Security A. +1>, and for every 2 colors your Tamers have, gets +2000 DP.
//! [Your Turn] [Once Per Turn] When your other Digimon are played or
//!   digivolve, if any of them have the [ADVENTURE] trait, 1 of your Digimon
//!   gains <Alliance> for the turn. Then, 1 of your Digimon may attack.
//! Inherited: <Alliance>
//!
//! # DCGO reference — DCGO/Assets/Scripts/CardEffect/ST20/Red/ST20_04.cs
//! - OP / WD: mandatory SelectPermanentEffect (own Digimon, canNoSelect
//!   false) → ChangeDigimonSAttack(+1) + ChangeDigimonDP(2000 * colors/2),
//!   both UntilEachTurnEnd.
//! - Your Turn OPT (hash "alliance-attack-ST20_04"): OnEnterFieldAnyone,
//!   owner turn, own OTHER Digimon played / digivolving; IsSkippable unless
//!   one of the batch has [ADVENTURE] → mandatory Alliance pick (any own
//!   Digimon, self included — official Q&A "You must give to 1 of your
//!   Digimon"), then optional attacker pick (canNoSelect true) →
//!   SelectAttackEffect (normal attack flow).
//! - Inherited: AllianceSelfEffect(isInheritedEffect: true).
//!
//! The shared "[Your Turn][OPT] ... gains <Alliance> ... may attack" clause is
//! exercised exhaustively here; the other six ADVENTURE Lv.5s (ST20-06/09,
//! ST21-04/09, BT21-061/078) reuse the identical YAML idiom.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "ST20-04";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, color: CardColor, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn tamer(id: &str, colors: &[CardColor]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c.colors = colors.to_vec();
    c
}

/// Lv.5 ADVENTURE evolver with an explicit blue Lv.4 / cost 0 circle so the
/// DebugRunner can drive a real digivolve (DSL cards carry empty evo_costs).
fn adventure_evolver(id: &str) -> CardData {
    let mut c = digimon(id, CardColor::Blue, 5, 6000, &["ADVENTURE"]);
    c.evo_costs = vec![EvoCost {
        card_color: 1,
        level: 4,
        memory_cost: 0,
    }];
    c
}

fn plain_evolver(id: &str) -> CardData {
    let mut c = digimon(id, CardColor::Blue, 5, 6000, &["Beast"]);
    c.evo_costs = vec![EvoCost {
        card_color: 1,
        level: 4,
        memory_cost: 0,
    }];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-04 YAML in embedded pack")
        .add_card(digimon("ADV", CardColor::Red, 4, 4000, &["ADVENTURE"]))
        .add_card(digimon("ADV2", CardColor::Red, 4, 4000, &["ADVENTURE"]))
        .add_card(digimon("PLAIN", CardColor::Red, 4, 4000, &["Beast"]))
        .add_card(digimon("ALLY", CardColor::Red, 4, 5000, &["Beast"]))
        .add_card(digimon("BASE4", CardColor::Blue, 4, 4000, &["Beast"]))
        .add_card(adventure_evolver("ADV-EVO"))
        .add_card(plain_evolver("PLAIN-EVO"))
        .add_card(digimon("OPP", CardColor::Blue, 4, 3000, &[]))
        .add_card(digimon("SEC", CardColor::Blue, 3, 1000, &[]))
        .add_card(tamer("T-RB", &[CardColor::Red, CardColor::Blue]))
        .add_card(tamer("T-YG", &[CardColor::Yellow, CardColor::Green]))
        .add_card(tamer("T-R", &[CardColor::Red]))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(10)
}

fn put_in_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("card {card_id} not registered"));
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_idx, player, card_index));
    runner.game.player(player).hand.len() - 1
}

fn field_action(index: u8) -> u16 {
    encode_attack(0, index as u16)
}

fn play_other(runner: &mut DebugRunner, player: u8, id: &str) -> PermanentHandle {
    let h = runner.place_on_field(player, id, None);
    runner.fire_play_event_triggers(player, h.index as usize, false, false);
    h
}

fn is_alliance_prompt(runner: &DebugRunner) -> bool {
    runner
        .pending_selection_view()
        .map(|v| v.kind == SelectionKind::OwnField && v.prompt.contains("Alliance"))
        .unwrap_or(false)
}

// ─── Section 1: structure ────────────────────────────────────────────────────

#[test]
fn st20_04_printed_stats_and_digivolve_paths() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.name, "Garudamon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(7000));
    for t in ["Birdkin", "ADVENTURE"] {
        assert!(card.traits.iter().any(|x| x == t), "missing trait {t}");
    }
    let digi: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 2, "red Lv.4 circle + Lv.4 [ADVENTURE] box");
    assert!(digi.iter().all(|p| p.cost == Some(CompiledCost::Literal(3))));
}

/// Alt box: a non-red Lv.4 [ADVENTURE] Digimon digivolves into Garudamon for 3.
#[test]
fn st20_04_digivolves_from_lv4_adventure_for_cost_3() {
    let mut runner = builder()
        .add_card(digimon("BLUE-ADV4", CardColor::Blue, 4, 4000, &["ADVENTURE"]))
        .start();
    runner.game.turn_count = 1;
    let base = runner.place_on_field(0, "BLUE-ADV4", Some(0));
    let idx = put_in_hand(&mut runner, 0, CARD_ID);
    let mem = runner.game.memory;
    assert!(runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByHand));
    assert_eq!(runner.game.memory, mem - 3);
}

#[test]
fn st20_04_inherited_alliance() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    let host = runner.place_stack(0, &[CARD_ID, "ALLY"]);
    assert!(
        runner.game.has_keyword(host, Keyword::Alliance),
        "carrier inherits <Alliance>"
    );
}

// ─── Section 2: [On Play] / [When Digivolving] ───────────────────────────────

/// 4 tamer colors (2 tamers × 2 colors) → +4000 DP and Security A. +1 on the
/// CHOSEN Digimon; the pick is mandatory and offers every own Digimon.
#[test]
fn st20_04_on_play_grants_sa_plus_one_and_dp_per_two_tamer_colors() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, "T-RB", Some(0));
    runner.place_on_field(0, "T-YG", Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    let garuda = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, garuda.index as usize);

    let view = runner.pending_selection_view().expect("OP pick installs");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(!view.is_optional, "1 of your Digimon — mandatory");
    assert_eq!(view.valid_action_ids.len(), 2, "ALLY + Garudamon (Tamers excluded)");
    runner
        .execute_action(0, field_action(ally.index))
        .expect("pick ALLY");
    runner.auto_resolve().unwrap();

    assert_eq!(runner.effective_dp(ally), Some(9000), "5000 + 2×2000");
    assert_eq!(runner.game.security_attack_keyword_bonus(ally), 1);
    assert_eq!(runner.effective_dp(garuda), Some(7000), "unchosen untouched");
    assert_eq!(runner.game.security_attack_keyword_bonus(garuda), 0);

    // "For the turn": both expire at end of turn.
    runner.end_turn();
    assert_eq!(runner.effective_dp(ally), Some(5000));
    assert_eq!(runner.game.security_attack_keyword_bonus(ally), 0);
}

/// 1 tamer color → floor(1/2) = 0 → no DP, but Security A. +1 still applies.
#[test]
fn st20_04_on_play_one_tamer_color_gives_no_dp() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, "T-R", Some(0));
    let garuda = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, garuda.index as usize);
    runner
        .execute_action(0, field_action(garuda.index))
        .expect("pick self");
    runner.auto_resolve().unwrap();
    assert_eq!(runner.effective_dp(garuda), Some(7000));
    assert_eq!(runner.game.security_attack_keyword_bonus(garuda), 1);
}

#[test]
fn st20_04_when_digivolving_fires_same_body() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, "T-RB", Some(0));
    let garuda = runner.place_stack(0, &["ALLY", CARD_ID]);
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(garuda));
    runner.game.drain_effect_queue();
    let view = runner.pending_selection_view().expect("WD pick installs");
    assert_eq!(view.kind, SelectionKind::OwnField);
    runner.execute_action(0, field_action(garuda.index)).unwrap();
    runner.auto_resolve().unwrap();
    assert_eq!(runner.effective_dp(garuda), Some(9000));
    assert_eq!(runner.game.security_attack_keyword_bonus(garuda), 1);
}

// ─── Section 3: [Your Turn][OPT] ADVENTURE observer ──────────────────────────

/// Own other [ADVENTURE] Digimon played → mandatory Alliance pick (player
/// choice over all own Digimon, self included) → optional attacker pick;
/// declining the attack leaves Alliance on the CHOSEN Digimon only.
#[test]
fn st20_04_adventure_play_grants_chosen_alliance_then_attack_is_optional() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    let garuda = runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    let adv = play_other(&mut runner, 0, "ADV");

    assert!(is_alliance_prompt(&runner), "Alliance pick must install");
    let view = runner.pending_selection_view().unwrap();
    assert!(!view.is_optional, "the Alliance grant is mandatory");
    assert_eq!(view.valid_action_ids.len(), 3, "Garudamon, ALLY, ADV");
    runner.execute_action(0, field_action(ally.index)).unwrap();
    assert!(runner.game.has_keyword(ally, Keyword::Alliance));
    assert!(!runner.game.has_keyword(adv, Keyword::Alliance));

    // Then: optional attacker pick. Only Digimon able to attack are offered
    // (the just-played ADV is summoning-sick).
    let view = runner
        .pending_selection_view()
        .expect("may-attack attacker pick installs");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(view.is_optional, "'may attack' — PASS legal");
    assert!(view.valid_action_ids.contains(&field_action(garuda.index)));
    assert!(view.valid_action_ids.contains(&field_action(ally.index)));
    assert!(!view.valid_action_ids.contains(&field_action(adv.index)));
    runner.execute_action(0, PASS).unwrap();
    assert!(runner.pending_selection_view().is_none());
    assert_eq!(runner.security_count(1), 3, "declined — no attack happened");

    runner.end_turn();
    assert!(
        !runner.game.has_keyword(ally, Keyword::Alliance),
        "Alliance is granted for the turn only"
    );
}

/// Accepting the attack runs the normal attack flow (security check).
#[test]
fn st20_04_accepted_attack_goes_through_normal_attack_flow() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    let garuda = runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    play_other(&mut runner, 0, "ADV");

    runner.execute_action(0, field_action(garuda.index)).unwrap(); // Alliance → Garudamon
    runner.execute_action(0, field_action(ally.index)).unwrap(); // attacker = ALLY
    let view = runner
        .pending_selection_view()
        .expect("attack-target prompt installs");
    let to_player = encode_attack(ally.index as u16, SECURITY_TARGET);
    assert!(
        view.valid_action_ids.contains(&to_player),
        "the opponent player is a legal attack target"
    );
    runner.execute_action(0, to_player).unwrap();
    runner.auto_resolve().unwrap();
    assert_eq!(runner.security_count(1), 2, "ALLY's attack checked 1 security");
    assert!(
        runner.game.player(0).battle_area[ally.index as usize].is_suspended,
        "the attacker suspends to attack"
    );
}

/// [Once Per Turn]: a second ADVENTURE play the same turn does not fire.
#[test]
fn st20_04_observer_once_per_turn() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, CARD_ID, Some(0));
    play_other(&mut runner, 0, "ADV");
    assert!(is_alliance_prompt(&runner));
    while runner.pending_selection_view().is_some() {
        let v = runner.pending_selection_view().unwrap();
        let a = if v.is_optional { PASS } else { v.valid_action_ids[0] };
        runner.execute_action(v.selecting_player, a).unwrap();
    }
    play_other(&mut runner, 0, "ADV2");
    assert!(
        runner.pending_selection_view().is_none(),
        "second qualifying play in the same turn must not fire"
    );
}

/// A non-[ADVENTURE] own play neither fires nor burns the once-per-turn; a
/// later [ADVENTURE] play the same turn still fires.
#[test]
fn st20_04_non_adventure_play_does_not_fire_or_consume_opt() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, CARD_ID, Some(0));
    play_other(&mut runner, 0, "PLAIN");
    assert!(runner.pending_selection_view().is_none());
    play_other(&mut runner, 0, "ADV");
    assert!(is_alliance_prompt(&runner), "ADVENTURE play still fires");
}

/// Garudamon's own play is not "your other Digimon": only its [On Play]
/// fires; after it resolves nothing else is pending.
#[test]
fn st20_04_own_play_does_not_fire_observer() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    let garuda = runner.place_on_field(0, CARD_ID, None);
    runner.fire_play_event_triggers(0, garuda.index as usize, false, false);
    let view = runner.pending_selection_view().expect("On Play pick");
    assert!(!view.prompt.contains("Alliance"), "OP pick, not the observer");
    runner.execute_action(0, field_action(garuda.index)).unwrap();
    assert!(
        runner.pending_selection_view().is_none(),
        "observer must not fire on Garudamon's own play"
    );
}

/// Opponent's [ADVENTURE] play on your turn is not "your" Digimon.
#[test]
fn st20_04_opponent_adventure_play_does_not_fire() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, CARD_ID, Some(0));
    play_other(&mut runner, 1, "ADV");
    assert!(runner.pending_selection_view().is_none());
}

/// [Your Turn]: an own [ADVENTURE] Digimon played during the opponent's turn
/// does not fire.
#[test]
fn st20_04_does_not_fire_on_opponents_turn() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1);
    runner.game.memory = 5;
    play_other(&mut runner, 0, "ADV");
    assert!(
        runner.pending_selection_view().is_none(),
        "[Your Turn] gate — no trigger on the opponent's turn"
    );
}

/// Digivolve trigger: another own Digimon digivolving into an [ADVENTURE]
/// card fires the observer.
#[test]
fn st20_04_other_digimon_digivolving_into_adventure_fires() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE4", Some(0));
    let idx = put_in_hand(&mut runner, 0, "ADV-EVO");
    assert!(runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByHand));
    runner.game.drain_effect_queue();
    assert!(is_alliance_prompt(&runner), "digivolve into ADVENTURE fires");
}

/// Digivolving into a non-[ADVENTURE] card does not fire.
#[test]
fn st20_04_other_digimon_digivolving_into_non_adventure_does_not_fire() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE4", Some(0));
    let idx = put_in_hand(&mut runner, 0, "PLAIN-EVO");
    assert!(runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByHand));
    runner.game.drain_effect_queue();
    assert!(runner.pending_selection_view().is_none());
}
