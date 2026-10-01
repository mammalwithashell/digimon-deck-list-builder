//! EX13-033 Mistymon — Digimon, Lv.5, Yellow/Red, DP 7000, Cost 7.
//! Traits: Magic Warrior / Witchelny. Form: Ultimate. Attribute: Virus.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-033.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-033.md` agrees)
//!
//! ```text
//! Digivolve: Yellow Lv.4 / cost 4; Red Lv.4 / cost 4;
//!            [Digivolve] Lv.4 w/[Witchelny] in text: Cost 3
//!
//! <Barrier> (When this Digimon would be deleted in battle, by trashing your
//! top security card, it isn't deleted.)
//! [On Play] [When Digivolving] You may place 1 [Witchelny] text card from
//! your hand as the bottom security card. Then, by trashing your top security
//! card, 1 of your Digimon may attack.
//! [All Turns] [Once Per Turn] When your security stack is removed from, 1 of
//! your opponent's Digimon gets -6000 DP for the turn. Then, if you have 3 or
//! fewer security cards, delete 1 of their 6000 DP or lower Digimon.
//!
//! Inherited Effect:
//! [All Turns] [Once Per Turn] When your security stack is removed from, this
//! Digimon may unsuspend.
//! ```
//!
//! # DCGO C# reference
//! None for EX13_033.cs (no file in the submodule).
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - Keyword grant (<Barrier>).
//! - Shared [On Play]/[When Digivolving]: optional hand → bottom security
//!   ([X]-in-text filter), then an optional "by" cost gating a may-attack.
//! - [All Turns][OPT] own-security-removed observer: -DP then a
//!   security-count-gated DP-threshold delete.
//! - Inherited [All Turns][OPT] own-security-removed optional self-unsuspend.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::{ATTACK_START, HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::trigger_context::EventCause;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX13-033";

fn digimon(id: &str, name: &str, level: u8, dp: i32, traits: &[&str], text: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = level as u16;
    c.traits = traits.iter().map(|s| s.to_string()).collect();
    c.effect_text = text.to_string();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-033 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("S1", "Sec One", 3, 1000, &[], ""))
        .add_card(digimon("S2", "Sec Two", 3, 1000, &[], ""))
        .add_card(digimon("S3", "Sec Three", 3, 1000, &[], ""))
        .add_card(digimon("S4", "Sec Four", 3, 1000, &[], ""))
        .add_card(digimon("S5", "Sec Five", 3, 1000, &[], ""))
        .add_card(digimon("OSEC", "Opp Sec", 3, 1000, &[], ""))
        .add_card(digimon("FILL", "Filler", 3, 1000, &[], ""))
        .add_card(digimon("W-TRAIT", "Witch Trait", 4, 5000, &["Witchelny"], ""))
        .add_card(digimon("W-TEXT", "Witch Text", 4, 5000, &[], "Lv.4 w/[Witchelny] in text"))
        .add_card(digimon("PLAIN", "Plainmon", 4, 5000, &[], ""))
        .add_card(digimon("ALLY", "Ally", 4, 6000, &[], ""))
        .add_card(digimon("CARRIER", "Carrier", 6, 11000, &[], ""))
        .add_card(digimon("OPP-12K", "Opp Twelve", 6, 12000, &[], ""))
        .add_card(digimon("OPP-6K", "Opp Six", 5, 6000, &[], ""))
        .add_card(digimon("OPP-7K", "Opp Seven", 5, 7000, &[], ""))
}

fn runner(security: &[&str], hand: &[&str]) -> DebugRunner {
    let mut r = builder()
        .security(0, security)
        .security(1, &["OSEC"; 5])
        .hand(0, hand)
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.skip_mulligan();
    r
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, h: PermanentHandle) {
    r.game.enqueue_triggered(timing, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

/// Fire the own-security-removed observer as the engine's fire-site builds it
/// (BT23-035 idiom): player 0 is the affected player and the observer.
fn fire_own_security_removed(r: &mut DebugRunner) {
    let card = r.game.players[0]
        .security
        .first()
        .map(|c| c.handle())
        .unwrap_or(CardHandle(0));
    r.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 0,
            observer_player: 0,
            source_player: 1,
            card,
            cause: EventCause::SecurityRemoval,
        },
    );
    r.game.drain_effect_queue();
}

fn fire_opponent_security_removed(r: &mut DebugRunner) {
    let card = r.game.players[1]
        .security
        .first()
        .map(|c| c.handle())
        .unwrap_or(CardHandle(0));
    r.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 1,
            observer_player: 0,
            source_player: 0,
            card,
            cause: EventCause::SecurityRemoval,
        },
    );
    r.game.drain_effect_queue();
}

fn security_ids(r: &DebugRunner, p: usize) -> Vec<String> {
    r.game.players[p]
        .security
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn opp_field_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn pick_hand(r: &mut DebugRunner, id: &str) {
    let v = r.pending_selection_view().expect("hand prompt");
    assert_eq!(v.kind, SelectionKind::Hand, "{v:?}");
    let idx = hand_ids(r).iter().position(|c| c == id).expect("in hand");
    let a = [HAND_EFFECT_START + idx as u16, PLAY_HAND_START + idx as u16]
        .into_iter()
        .find(|a| v.valid_action_ids.contains(a))
        .unwrap_or_else(|| panic!("{id} selectable: {v:?}"));
    r.execute_action(v.selecting_player, a).expect("pick hand");
}

fn pick_field(r: &mut DebugRunner, target: PermanentHandle) {
    let v = r.pending_selection_view().expect("field prompt");
    let id = ATTACK_START + target.index as u16;
    assert!(v.valid_action_ids.contains(&id), "{target:?} not selectable: {v:?}");
    r.execute_action(v.selecting_player, id).expect("pick");
}

fn offered_opp_field(r: &DebugRunner) -> Vec<u16> {
    let v = r.pending_selection_view().expect("field prompt");
    assert_eq!(v.kind, SelectionKind::OppField, "{v:?}");
    let mut ids: Vec<u16> = v
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .map(|a| a - ATTACK_START)
        .collect();
    ids.sort();
    ids
}

/// Answer the "trash top security so 1 of your Digimon may attack?" choice:
/// option 0 = pay, option 1 = decline.
fn answer_cost(r: &mut DebugRunner, pay: bool) {
    let v = r.pending_selection_view().expect("cost choice");
    assert_eq!(v.kind, SelectionKind::EffectChoice, "{v:?}");
    let opts: Vec<u16> = v.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    assert_eq!(opts.len(), 2, "pay / decline: {v:?}");
    r.execute_action(v.selecting_player, opts[if pay { 0 } else { 1 }]).expect("answer");
}

fn decline_pending(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("pending");
    r.execute_action(v.selecting_player, PASS).expect("decline");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_033_printed_metadata_and_digivolve_paths() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(5), Some(7000), Some(7)));
    assert_eq!(c.color, vec![CompiledColor::Yellow, CompiledColor::Red]);
    assert_eq!(c.traits, vec!["Magic Warrior".to_string(), "Witchelny".to_string()]);
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 3);
    assert_eq!(
        digi.iter().filter(|p| p.cost == Some(CompiledCost::Literal(4))).count(),
        2
    );
    assert_eq!(
        digi.iter().filter(|p| p.cost == Some(CompiledCost::Literal(3))).count(),
        1,
        "Lv.4 w/[Witchelny] in text: Cost 3"
    );
}

#[test]
fn ex13_033_clause_shape() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 3);
    let op = t
        .iter()
        .find(|x| x.when.contains(&CompiledTiming::OnPlay))
        .expect("OP/WD clause");
    assert!(op.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(!op.once_per_turn);
    let face_obs = t
        .iter()
        .find(|x| {
            x.scope == CompiledScope::FaceUp
                && x.when.contains(&CompiledTiming::OnOwnSecurityRemoved)
        })
        .expect("face-up security-removed observer");
    assert!(face_obs.once_per_turn);
    assert!(!face_obs.optional, "the -6000 is mandatory");
    let inh = t
        .iter()
        .find(|x| x.scope == CompiledScope::Inherited)
        .expect("inherited observer");
    assert_eq!(inh.when, vec![CompiledTiming::OnOwnSecurityRemoved]);
    assert!(inh.once_per_turn);
    assert!(inh.optional, "\"may unsuspend\"");
}

#[test]
fn ex13_033_has_barrier() {
    let mut r = runner(&["S1"], &[]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(me, Keyword::Barrier));
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2/3 — [On Play][When Digivolving]
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_033_on_play_places_witchelny_card_bottom_then_pays_and_attacks() {
    let mut r = runner(&["S1", "S2", "S3", "S4"], &[CARD_ID, "W-TRAIT", "W-TEXT", "PLAIN"]);
    r.place_on_field(0, "ALLY", Some(0));
    let opp_sec = r.security_count(1);
    r.play(0, 0).expect("Mistymon played");
    let v = r.pending_selection_view().expect("placement prompt");
    assert_eq!(v.kind, SelectionKind::Hand);
    assert!(v.is_optional, "\"You may place\"");
    assert_eq!(
        v.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        2,
        "only [Witchelny]-text cards offered"
    );
    pick_hand(&mut r, "W-TEXT");
    assert_eq!(security_ids(&r, 0)[0], "W-TEXT", "placed as the bottom security card");
    answer_cost(&mut r, true);
    assert_eq!(
        security_ids(&r, 0),
        vec!["W-TEXT", "S1", "S2", "S3"],
        "top security (S4) trashed as the cost"
    );
    // Choose the attacker (optional), then the attack target.
    let v = r.pending_selection_view().expect("attacker prompt");
    assert_eq!(v.kind, SelectionKind::OwnField);
    assert!(v.is_optional, "\"may attack\"");
    let a = v.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
    r.execute_action(0, a).expect("choose attacker");
    // Resolve the rest (target prompt, the [All Turns] observer with no
    // opponent Digimon, security check).
    for _ in 0..8 {
        let Some(v) = r.pending_selection_view() else { break };
        let a = v.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap_or(PASS);
        r.execute_action(v.selecting_player, a).unwrap();
    }
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), opp_sec - 1, "the attack checked 1 security");
}

#[test]
fn ex13_033_declining_placement_still_offers_the_cost() {
    let mut r = runner(&["S1", "S2", "S3", "S4"], &[CARD_ID, "W-TRAIT"]);
    r.play(0, 0).expect("played");
    decline_pending(&mut r);
    assert_eq!(security_ids(&r, 0), vec!["S1", "S2", "S3", "S4"]);
    assert_eq!(r.pending_kind(), Some(SelectionKind::EffectChoice));
}

#[test]
fn ex13_033_declining_the_cost_keeps_security_and_no_attack() {
    let mut r = runner(&["S1", "S2", "S3", "S4"], &[CARD_ID]);
    r.place_on_field(0, "ALLY", Some(0));
    r.play(0, 0).expect("played");
    answer_cost(&mut r, false);
    let _ = r.auto_resolve();
    assert_eq!(security_ids(&r, 0), vec!["S1", "S2", "S3", "S4"]);
    assert!(r.pending_selection_view().is_none(), "no attacker prompt");
    assert_eq!(r.security_count(1), 5);
}

#[test]
fn ex13_033_cost_payable_even_if_attack_declined() {
    let mut r = runner(&["S1", "S2", "S3", "S4"], &[CARD_ID]);
    r.play(0, 0).expect("played");
    answer_cost(&mut r, true);
    assert_eq!(r.security_count(0), 3);
    let v = r.pending_selection_view().expect("attacker prompt");
    assert_eq!(v.kind, SelectionKind::OwnField);
    r.execute_action(0, PASS).expect("decline attack");
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 5, "no attack");
}

#[test]
fn ex13_033_no_security_no_cost_offer() {
    let mut r = runner(&[], &[CARD_ID, "PLAIN"]);
    r.play(0, 0).expect("played");
    let _ = r.auto_resolve();
    assert!(
        r.pending_selection_view().is_none(),
        "no [Witchelny] card and no security → nothing offered"
    );
}

#[test]
fn ex13_033_placed_card_can_fund_the_cost_from_empty_security() {
    // With 0 security, placing a card makes the "by trashing" cost payable.
    let mut r = runner(&[], &[CARD_ID, "W-TRAIT"]);
    r.play(0, 0).expect("played");
    pick_hand(&mut r, "W-TRAIT");
    assert_eq!(r.security_count(0), 1);
    answer_cost(&mut r, true);
    assert_eq!(r.security_count(0), 0, "the placed card was the top (only) card");
}

#[test]
fn ex13_033_when_digivolving_fires() {
    let mut r = runner(&["S1", "S2"], &["W-TRAIT"]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, me);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2/3/5 — [All Turns][OPT] own security removed observer
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_033_observer_debuffs_and_with_4_security_does_not_delete() {
    let mut r = runner(&["S1", "S2", "S3", "S4"], &[]);
    r.place_on_field(0, CARD_ID, Some(0));
    let big = r.place_on_field(1, "OPP-12K", Some(0));
    let six = r.place_on_field(1, "OPP-6K", Some(0));
    fire_own_security_removed(&mut r);
    assert_eq!(offered_opp_field(&r), vec![big.index as u16, six.index as u16]);
    pick_field(&mut r, big);
    let _ = r.auto_resolve();
    assert_eq!(r.effective_dp(big), Some(6000), "-6000 for the turn");
    assert!(r.pending_selection_view().is_none(), "4 security → no delete");
    assert_eq!(opp_field_ids(&r).len(), 2);
}

#[test]
fn ex13_033_observer_with_3_security_deletes_6000_or_lower() {
    let mut r = runner(&["S1", "S2", "S3"], &[]);
    r.place_on_field(0, CARD_ID, Some(0));
    let big = r.place_on_field(1, "OPP-12K", Some(0));
    let seven = r.place_on_field(1, "OPP-7K", Some(0));
    let six = r.place_on_field(1, "OPP-6K", Some(0));
    fire_own_security_removed(&mut r);
    pick_field(&mut r, big);
    assert_eq!(
        offered_opp_field(&r),
        vec![big.index as u16, six.index as u16],
        "12000-6000 and 6000 are eligible; 7000 is not"
    );
    pick_field(&mut r, six);
    let _ = r.auto_resolve();
    let opp = opp_field_ids(&r);
    assert!(!opp.contains(&"OPP-6K".to_string()));
    assert!(opp.contains(&"OPP-7K".to_string()));
    assert!(opp.contains(&"OPP-12K".to_string()));
    let _ = seven;
}

#[test]
fn ex13_033_observer_ignores_opponent_security_removal() {
    let mut r = runner(&["S1", "S2"], &[]);
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP-12K", Some(0));
    fire_opponent_security_removed(&mut r);
    assert!(
        r.pending_selection_view().is_none(),
        "only YOUR security stack being removed from triggers it"
    );
}

#[test]
fn ex13_033_observer_once_per_turn_and_resets() {
    let mut r = runner(&["S1", "S2", "S3", "S4", "S5"], &[]);
    r.place_on_field(0, CARD_ID, Some(0));
    let big = r.place_on_field(1, "OPP-12K", Some(0));
    fire_own_security_removed(&mut r);
    pick_field(&mut r, big);
    let _ = r.auto_resolve();
    fire_own_security_removed(&mut r);
    assert!(r.pending_selection_view().is_none(), "OPT spent this turn");
    assert_eq!(r.effective_dp(big), Some(6000));
    r.end_turn();
    let _ = r.auto_resolve();
    fire_own_security_removed(&mut r);
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::OppField),
        "[All Turns] OPT resets on the opponent's turn"
    );
}

#[test]
fn ex13_033_own_cost_trash_triggers_observer_integrated() {
    let mut r = runner(&["S1", "S2", "S3", "S4"], &[CARD_ID]);
    let big = r.place_on_field(1, "OPP-12K", Some(0));
    r.play(0, 0).expect("played");
    answer_cost(&mut r, true);
    let mut debuffed = false;
    for _ in 0..8 {
        let Some(v) = r.pending_selection_view() else { break };
        if v.kind == SelectionKind::OppField && !debuffed {
            pick_field(&mut r, big);
            debuffed = true;
            continue;
        }
        let a = if v.valid_action_ids.contains(&PASS) {
            PASS
        } else {
            v.valid_action_ids[0]
        };
        r.execute_action(v.selecting_player, a).unwrap();
    }
    let _ = r.auto_resolve();
    assert!(debuffed, "trashing security as the cost fires the [All Turns] observer");
    assert_eq!(r.security_count(0), 3);
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2/3/5 — Inherited: may unsuspend
// ════════════════════════════════════════════════════════════════════════════

fn carrier_setup() -> (DebugRunner, PermanentHandle) {
    let mut r = runner(&["S1", "S2", "S3"], &[]);
    let h = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    (r, h)
}

#[test]
fn ex13_033_inherited_unsuspends_carrier() {
    let (mut r, h) = carrier_setup();
    fire_own_security_removed(&mut r);
    assert!(r.pending_is_optional(), "\"may unsuspend\"");
    r.accept_optional_trigger().expect("accept");
    let _ = r.auto_resolve();
    assert!(!r.game.players[0].battle_area[h.index as usize].is_suspended);
}

#[test]
fn ex13_033_inherited_may_be_declined() {
    let (mut r, h) = carrier_setup();
    fire_own_security_removed(&mut r);
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
}

#[test]
fn ex13_033_inherited_ignores_opponent_security_removal() {
    let (mut r, h) = carrier_setup();
    fire_opponent_security_removed(&mut r);
    assert!(r.pending_selection_view().is_none());
    assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
}

#[test]
fn ex13_033_inherited_once_per_turn_and_resets() {
    let (mut r, h) = carrier_setup();
    fire_own_security_removed(&mut r);
    r.accept_optional_trigger().expect("accept");
    let _ = r.auto_resolve();
    r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    fire_own_security_removed(&mut r);
    assert!(r.pending_selection_view().is_none(), "OPT spent");
    assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
    r.end_turn();
    let _ = r.auto_resolve();
    r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    fire_own_security_removed(&mut r);
    assert!(r.pending_is_optional(), "[All Turns] OPT resets next turn");
}
