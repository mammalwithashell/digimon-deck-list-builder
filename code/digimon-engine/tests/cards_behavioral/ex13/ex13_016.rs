//! EX13-016 Omnimon — Digimon, Lv.7, Red/White/Blue, DP 15000, Cost 15.
//! Traits: Holy Warrior / Royal Knight / CS. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-016.md`)
//!
//! ```text
//! Digivolve: Red Lv.6 / 5; Blue Lv.6 / 5; [Digivolve] Lv.6 w/[CS] trait: Cost 5
//! [DNA Digivolve] Lv.6 w/[Greymon] in name + Lv.6 w/[Garurumon] in name: Cost 0
//! Assembly -7: [WarGreymon]×[MetalGarurumon]×[Agumon]×[Gabumon]
//! <Raid> <Blocker>
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] 2 of your
//! opponent's Digimon or Tamers can't suspend until their turn ends.
//! [On Play] [When Digivolving] [Counter] [Once Per Turn] You may delete 1 of
//! your opponent's Digimon with as many digivolution cards as this Digimon or
//! fewer.
//! [All Turns] When this Digimon would leave the battle area, by trashing 2
//! same-level cards from its digivolution cards, it doesn't leave.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Red/EX13_016.cs (delete clause L129-183).
//!
//! # Verdict — IMPLEMENTED
//! The would-leave save uses `select_materials { min: 2, max: 2, same_by:
//! level }` + `trash_selected_materials` (G-ENGINE-SAME-LEVEL-SOURCE-PAIR-
//! SELECTION, RESOLVED 2026-10-01); the replacement preflight only offers it
//! when a same-level pair exists.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledDeclarativeClause,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

use super::orphan_support::{assert_replacement_prompt, delete_with};

const CARD_ID: &str = "EX13-016";

fn digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = 3;
    c
}

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-016 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("L4A", 4))
        .add_card(digimon("L4B", 4))
        .add_card(digimon("L5", 5))
        .add_card(digimon("L6", 6))
        .add_card(digimon("L6B", 6))
        .add_card(digimon("OPP", 5))
        .add_card(tamer("TAMER"))
        .add_card(digimon("FILL", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["FILL"; 3])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start()
}

fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

fn source_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    let p = &r.game.players[h.player as usize].battle_area[h.index as usize];
    let n = p.card_sources.len() - 1;
    p.card_sources[..n]
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn cannot_suspend(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.modifiers.has(h, ModifierType::CannotSuspend)
}

/// [On Play] queues the lock AND the delete offer together: resolve the
/// simultaneous-trigger order prompt (first entry = the lock clause).
fn ex13_016_order_first(r: &mut DebugRunner) {
    while r.pending_kind() == Some(SelectionKind::TriggerOrder) {
        let v = r.pending_selection_view().unwrap();
        r.execute_action(v.selecting_player, v.valid_action_ids[0]).unwrap();
    }
}

fn ex13_016_pick(r: &mut DebugRunner, idx: usize) {
    let v = r.pending_selection_view().expect("pending");
    r.execute_action(v.selecting_player, v.valid_action_ids[idx])
        .expect("pick");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_016_identity_alt_paths_and_keywords() {
    let r = runner();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(7));
    assert_eq!(card.dp, Some(15000));
    let count = |k: CompiledAltPathKind| card.alt_paths.iter().filter(|p| p.kind == k).count();
    assert_eq!(count(CompiledAltPathKind::Digivolve), 3, "red Lv.6, blue Lv.6, Lv.6 [CS]");
    assert_eq!(count(CompiledAltPathKind::DnaDigivolve), 1);
    assert_eq!(count(CompiledAltPathKind::Assembly), 1);
    let keywords: Vec<&str> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword, ..
            }) => Some(keyword.as_str()),
            _ => None,
        })
        .collect();
    assert!(keywords.contains(&"Raid") && keywords.contains(&"Blocker"), "{keywords:?}");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — [OP][WD][WA][OPT] 2 opponent Digimon/Tamers can't suspend
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_016_on_play_locks_two_of_three_opponent_digimon_or_tamers() {
    let mut r = runner();
    let o1 = r.place_on_field(1, "OPP", Some(0));
    let o2 = r.place_on_field(1, "TAMER", Some(0));
    let o3 = r.place_on_field(1, "OPP", Some(0));
    let omni = r.place_stack(0, &["L6", CARD_ID]);
    r.fire_on_play(0, omni.index as usize);
    ex13_016_order_first(&mut r);
    // The lock is mandatory; with 3 candidates the controller picks 2.
    let v = r.pending_selection_view().expect("lock pick");
    assert_eq!(v.valid_action_ids.len(), 3, "2 Digimon + 1 Tamer offered");
    assert_eq!(v.selecting_player, 0);
    ex13_016_pick(&mut r, 0);
    ex13_016_pick(&mut r, 0);
    // Resolve whatever follows (the optional delete offer: decline).
    while let Some(v) = r.pending_selection_view() {
        if v.is_optional {
            r.decline_optional_trigger().unwrap();
        } else {
            ex13_016_pick(&mut r, 0);
        }
    }
    let locked = [o1, o2, o3].iter().filter(|h| cannot_suspend(&r, **h)).count();
    assert_eq!(locked, 2, "exactly 2 of the opponent's permanents can't suspend");
}

#[test]
fn ex13_016_lock_takes_every_target_when_fewer_than_two() {
    let mut r = runner();
    let o1 = r.place_on_field(1, "TAMER", Some(0));
    let omni = r.place_stack(0, &["L6", CARD_ID]);
    r.fire_on_play(0, omni.index as usize);
    while let Some(v) = r.pending_selection_view() {
        if v.is_optional {
            r.decline_optional_trigger().unwrap();
        } else {
            ex13_016_pick(&mut r, 0);
        }
    }
    assert!(cannot_suspend(&r, o1), "the only opponent Tamer can't suspend");
}

#[test]
fn ex13_016_lock_is_once_per_turn_across_on_play_and_when_attacking() {
    let mut r = runner();
    let o1 = r.place_on_field(1, "OPP", Some(0));
    let omni = r.place_stack(0, &["L6", CARD_ID]);
    r.fire_on_play(0, omni.index as usize);
    while let Some(v) = r.pending_selection_view() {
        if v.is_optional {
            r.decline_optional_trigger().unwrap();
        } else {
            ex13_016_pick(&mut r, 0);
        }
    }
    assert!(cannot_suspend(&r, o1));
    // A fresh opponent Digimon arrives; a re-fired lock would catch it.
    let o2 = r.place_on_field(1, "OPP", Some(0));
    r.attack_player(omni, 1, false);
    // [When Attacking]: no second lock this turn (OPT). Decline anything else.
    while let Some(v) = r.pending_selection_view() {
        if v.is_optional {
            r.decline_optional_trigger().unwrap();
        } else {
            ex13_016_pick(&mut r, 0);
        }
    }
    assert!(!cannot_suspend(&r, o2), "[Once Per Turn] — the lock didn't re-fire");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — [OP][WD][Counter][OPT] may delete ≤ sources
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_016_on_play_may_delete_opponent_digimon_with_no_more_sources() {
    let mut r = runner();
    // Omnimon has 2 digivolution cards; the opponent Digimon with 3 can't be
    // chosen, the one with 2 can.
    let big = r.place_stack(1, &["FILL", "FILL", "FILL", "OPP"]);
    let small = r.place_stack(1, &["FILL", "FILL", "OPP"]);
    let omni = r.place_stack(0, &["L5", "L6", CARD_ID]);
    // Lock resolves first (mandatory); decline nothing there.
    r.fire_on_play(0, omni.index as usize);
    ex13_016_order_first(&mut r);
    // Lock pick (2 opp Digimon): take both.
    let v = r.pending_selection_view().expect("lock pick");
    assert!(!v.is_optional);
    ex13_016_pick(&mut r, 0);
    // (second pick, if still pending as a lock pick)
    if r
        .pending_selection_view()
        .is_some_and(|v| !v.is_optional && v.valid_action_ids.len() == 1)
    {
        ex13_016_pick(&mut r, 0);
    }
    // "You may delete": DCGO EX13_016.cs has no Yes/No at [On Play] (optional:
    // false); the canNoSelect delete pick itself is the decline.
    let v = r.pending_selection_view().expect("delete target pick");
    assert_ne!(v.kind, SelectionKind::Replacement, "no outer accept prompt");
    assert!(v.is_optional, "'You may delete' is a declinable pick");
    let targets = non_pass(&v.valid_action_ids);
    assert_eq!(targets.len(), 1, "only the ≤2-source Digimon is eligible");
    r.execute_action(v.selecting_player, targets[0]).expect("delete it");
    let _ = r.auto_resolve();
    let ids: Vec<String> = r.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.card_sources.len().to_string())
        .collect();
    assert_eq!(ids, vec!["4"], "the 3-source Digimon stays; the 2-source one was deleted");
    let _ = (big, small);
}

fn non_pass(ids: &[u16]) -> Vec<u16> {
    ids.iter().copied().filter(|&a| a != PASS).collect()
}

/// Fire `timing` for Omnimon and resolve the lock clause (if it is offered)
/// until the delete pick is pending; returns whether it is.
fn reach_delete_pick(r: &mut DebugRunner, omni: PermanentHandle, timing: EffectTiming) -> bool {
    r.game.enqueue_triggered(timing, TriggerSource::Permanent(omni));
    r.game.drain_effect_queue();
    for _ in 0..8 {
        ex13_016_order_first(r);
        let Some(v) = r.pending_selection_view() else { return false };
        if v.is_optional {
            return true;
        }
        ex13_016_pick(r, 0); // the mandatory lock pick
    }
    false
}

/// Accepting is choosing a target: passing the delete pick while a target
/// was possible keeps the [Once Per Turn] (DCGO EX13_016.cs: canNoSelect
/// pick, `if (!isUsed) activateClass.RemoveUse()`; owner ruling 2026-11-24,
/// "Generally let's look to match dcgo"; W14 row (b), Q1818). The next
/// [When Digivolving] the same turn offers the delete again.
#[test]
fn ex13_016_delete_pick_passed_keeps_the_once_per_turn() {
    let mut r = runner();
    let target = r.place_stack(1, &["FILL", "OPP"]);
    let omni = r.place_stack(0, &["L5", "L6", CARD_ID]);
    assert!(reach_delete_pick(&mut r, omni, EffectTiming::OnPlay), "[On Play] delete pick");
    assert_ne!(r.pending_kind(), Some(SelectionKind::Replacement), "no outer accept prompt");
    r.decline_optional_trigger().expect("pass the delete pick");
    assert!(r.pending_selection().is_none(), "no decision is left");
    assert_eq!(r.game.players[1].battle_area.len(), 1, "nothing deleted");

    assert!(
        reach_delete_pick(&mut r, omni, EffectTiming::WhenDigivolving),
        "the passed pick kept the use: [When Digivolving] offers the delete again"
    );
    let v = r.pending_selection_view().unwrap();
    r.execute_action(v.selecting_player, non_pass(&v.valid_action_ids)[0]).unwrap();
    assert!(r.pending_selection().is_none(), "no decision is left");
    assert!(r.game.players[1].battle_area.is_empty(), "deleted at [When Digivolving]");
    let _ = target;
}

/// Choosing a target spends the [Once Per Turn] (row c): a later [When
/// Digivolving] the same turn offers no delete.
#[test]
fn ex13_016_delete_chosen_spends_the_once_per_turn() {
    let mut r = runner();
    r.place_stack(1, &["FILL", "OPP"]);
    r.place_stack(1, &["FILL", "OPP"]);
    let omni = r.place_stack(0, &["L5", "L6", CARD_ID]);
    assert!(reach_delete_pick(&mut r, omni, EffectTiming::OnPlay));
    assert_ne!(r.pending_kind(), Some(SelectionKind::Replacement), "no outer accept prompt");
    let v = r.pending_selection_view().unwrap();
    r.execute_action(v.selecting_player, non_pass(&v.valid_action_ids)[0]).unwrap();
    assert!(r.pending_selection().is_none(), "no decision is left");
    assert_eq!(r.game.players[1].battle_area.len(), 1, "one deleted");
    assert!(
        !reach_delete_pick(&mut r, omni, EffectTiming::WhenDigivolving),
        "the chosen delete spent the use"
    );
}

/// [Counter] in real combat. A refunded use is not offered again inside the
/// same attack (one Counter window per attack: `try_enter_counter`'s
/// `counter_depth` guard; DCGO runs CounterTiming once per attack and keeps
/// the accepted effect in SkillInfos_used, Q7253), but it is offered at the
/// next attack the same turn; choosing a target there spends it, so a third
/// attack offers nothing.
#[test]
fn ex13_016_counter_passed_pick_refunds_for_a_later_attack_only() {
    use digimon_engine::action::space::{encode_attack, ATTACK_START};
    use digimon_engine::enums::GamePhase;

    let mut r = runner();
    let atk1 = r.place_on_field(0, "OPP", Some(0));
    let atk2 = r.place_on_field(0, "OPP", Some(0));
    let _third = r.place_on_field(0, "OPP", Some(0));
    let omni = r.place_stack(1, &["L6", CARD_ID]);
    // Suspended: no <Blocker> prompt in these attacks.
    r.game.players[1].battle_area[omni.index as usize].is_suspended = true;
    let counter_action = encode_attack(0, omni.index as u16);

    // Resolve every remaining prompt of the attack, failing if Omnimon's
    // [Counter] is offered again.
    let finish_attack = |r: &mut DebugRunner| {
        for _ in 0..32 {
            let Some(v) = r.pending_selection_view() else { return };
            assert!(
                !(r.current_phase() == GamePhase::CounterTiming
                    && v.valid_action_ids.contains(&counter_action)),
                "Omnimon's [Counter] is not offered twice in one attack"
            );
            let id = if v.is_optional { PASS } else { v.valid_action_ids[0] };
            r.execute_action(v.selecting_player, id).unwrap();
        }
        panic!("attack did not finish");
    };

    // Attack 1: use the [Counter], then pass the delete pick.
    r.attack_player(atk1, 1, false);
    assert_eq!(r.current_phase(), GamePhase::CounterTiming);
    let v = r.pending_selection_view().expect("counter window");
    assert!(v.valid_action_ids.contains(&counter_action), "Omnimon's [Counter] is offered");
    r.execute_action(1, counter_action).unwrap();
    let v = r.pending_selection_view().expect("delete pick");
    assert_ne!(v.kind, SelectionKind::Replacement, "the window pick is the accept");
    assert!(v.is_optional, "the delete pick is declinable (PASS): {v:?}");
    r.execute_action(1, PASS).unwrap();
    finish_attack(&mut r);
    assert_eq!(r.game.players[0].battle_area.len(), 3, "nothing deleted");

    // Attack 2: the refunded use is back; delete the attacker.
    r.attack_player(atk2, 1, false);
    assert_eq!(r.current_phase(), GamePhase::CounterTiming);
    let v = r.pending_selection_view().expect("counter window");
    assert!(
        v.valid_action_ids.contains(&counter_action),
        "the passed pick kept the use for a later attack this turn"
    );
    r.execute_action(1, counter_action).unwrap();
    r.execute_action(1, ATTACK_START + atk2.index as u16).expect("delete the attacker");
    finish_attack(&mut r);
    assert_eq!(r.game.players[0].battle_area.len(), 2, "attacker 2 deleted");

    // Attack 3: the use is spent.
    let atk3 = PermanentHandle { player: 0, index: r.game.players[0].battle_area.len() as u8 - 1 };
    r.attack_player(atk3, 1, false);
    let offered = r.current_phase() == GamePhase::CounterTiming
        && r.pending_selection_view().is_some_and(|v| v.valid_action_ids.contains(&counter_action));
    assert!(!offered, "the chosen delete spent the [Once Per Turn]");
}

#[test]
fn ex13_016_delete_not_offered_without_an_eligible_target() {
    let mut r = runner();
    let _big = r.place_stack(1, &["FILL", "FILL", "OPP"]);
    let omni = r.place_stack(0, &["L6", CARD_ID]);
    r.fire_on_play(0, omni.index as usize);
    // The lock picks its single target; no optional delete prompt follows.
    while let Some(v) = r.pending_selection_view() {
        assert!(
            !v.is_optional,
            "no 'may delete' offer: the opponent Digimon has more sources than Omnimon"
        );
        ex13_016_pick(&mut r, 0);
    }
    assert_eq!(r.game.players[1].battle_area.len(), 1);
}

// ════════════════════════════════════════════════════════════════════════════
// Section 4 — [All Turns] would-leave save (same-level pair)
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_016_save_trashes_a_same_level_pair_and_stays() {
    let mut r = runner();
    // Sources (bottom→top): L4A, L5, L4B — the only same-level pair is 4/4.
    let omni = r.place_stack(0, &["L4A", "L5", "L4B", CARD_ID]);
    delete_with(&mut r, omni, ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
    r.accept_optional_trigger().expect("accept the save");
    let v = r.pending_selection_view().expect("first pair pick");
    assert_eq!(v.valid_action_ids.len(), 2, "only the two level-4 cards start a pair");
    assert!(!v.is_optional, "exactly 2 must be trashed");
    ex13_016_pick(&mut r, 0);
    let v = r.pending_selection_view().expect("second pair pick");
    assert_eq!(v.valid_action_ids.len(), 1, "only the other level-4 card");
    ex13_016_pick(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.game.players[0].battle_area.len(), 1, "it doesn't leave");
    assert_eq!(top_id(&r, omni), CARD_ID);
    assert_eq!(source_ids(&r, omni), vec!["L5"], "the level-4 pair was trashed");
    let trash: Vec<String> = r.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    assert!(trash.contains(&"L4A".to_string()) && trash.contains(&"L4B".to_string()));
}

#[test]
fn ex13_016_save_not_offered_without_a_same_level_pair() {
    let mut r = runner();
    // Levels 4, 5, 6 + a level-less Tamer card: no pair → no offer.
    let omni = r.place_stack(0, &["L4A", "TAMER", "L5", "L6", CARD_ID]);
    delete_with(&mut r, omni, ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area.is_empty(), "deleted — the save was unpayable");
}

#[test]
fn ex13_016_save_decline_lets_it_leave() {
    let mut r = runner();
    let omni = r.place_stack(0, &["L6", "L6B", CARD_ID]);
    delete_with(&mut r, omni, ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
    r.decline_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area.is_empty());
}

#[test]
fn ex13_016_save_is_not_once_per_turn() {
    let mut r = runner();
    let omni = r.place_stack(0, &["L4A", "L4B", "L6", "L6B", CARD_ID]);
    for round in 0..2 {
        delete_with(&mut r, omni, ReplacementCause::OpponentEffect);
        assert_replacement_prompt(&r);
        r.accept_optional_trigger().unwrap();
        ex13_016_pick(&mut r, 0);
        ex13_016_pick(&mut r, 0);
        let _ = r.auto_resolve();
        assert_eq!(r.game.players[0].battle_area.len(), 1, "saved in round {round}");
    }
    assert!(source_ids(&r, omni).is_empty(), "both pairs were spent");
    // No pair left → the third removal goes through.
    delete_with(&mut r, omni, ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area.is_empty());
}
