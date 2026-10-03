//! EX13-029 FlameWizardmon — Digimon, Lv.4, Yellow/Red, DP 5000, Cost 5.
//! Traits: Wizard / Witchelny. Form: Armor Form. Attribute: Virus.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-029.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-029.md` agrees)
//!
//! ```text
//! Digivolve: Yellow Lv.3 / cost 3; Red Lv.3 / cost 3;
//!            [Digivolve] Lv.3 w/[Witchelny] in text: Cost 2
//!
//! <Armor Purge> (When this Digimon would be deleted, you may trash the top
//! card of this Digimon to prevent that deletion.)
//! [When Digivolving] [When Attacking] [Once Per Turn] By trashing your top
//! security card, 1 of your opponent's Digimon gets -4000 DP for the turn.
//! After, if you have 3 or fewer security cards, delete 1 of your opponent's
//! Digimon with 4000 DP or less.
//! (Rule) Name: Also treated as [Wizardmon].
//!
//! Inherited Effect:
//! [All Turns] [Once Per Turn] When this Digimon with [Dynasmon] or
//! [Witchelny] in its text would leave the battle area by your opponent's
//! effects, by trashing your top security card, it doesn't leave.
//! ```
//!
//! # DCGO C# reference
//! None for EX13_029.cs. Inherited clause = BT18-030 Candlemon inherited
//! (`DCGO/Assets/Scripts/CardEffect/BT18/Yellow/BT18_030.cs`) with an
//! [X]-in-text carrier gate.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - Shared [When Digivolving]/[When Attacking] OPT, optional "by" cost
//!   (outer accept/decline before the cost moves).
//! - -DP debuff then a security-count-gated DP-threshold delete.
//! - Keyword grant (<Armor Purge>), also-treated-as name.
//! - F3 inherited leave-prevention replacement (security-trash cost).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{ATTACK_START, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX13-029";

fn digimon(id: &str, name: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = level as u16;
    c.traits = traits.iter().map(|s| s.to_string()).collect();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-029 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("SEC", "Sec", 3, 3000, &[]))
        .add_card(digimon("FILL", "Filler", 3, 3000, &[]))
        .add_card(digimon("OPP-8K", "Opp Eight", 5, 8000, &[]))
        .add_card(digimon("OPP-5K", "Opp Five", 4, 5000, &[]))
        .add_card(digimon("OPP-4K", "Opp Four", 4, 4000, &[]))
        .add_card(digimon("OPP-BIG", "Opp Big", 6, 13000, &[]))
        .add_card(digimon("CARRIER-W", "Witch Carrier", 5, 7000, &["Witchelny"]))
        .add_card(digimon("CARRIER-PLAIN", "Plain Carrier", 5, 7000, &["Beast"]))
}

fn runner(security: usize) -> DebugRunner {
    let mut r = builder()
        .security(0, &vec!["SEC"; security])
        .security(1, &["SEC"; 5])
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

fn opp_field_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn field_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn offered_field(r: &DebugRunner) -> Vec<u16> {
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

fn pick_field(r: &mut DebugRunner, target: PermanentHandle) {
    let v = r.pending_selection_view().expect("field prompt");
    let id = ATTACK_START + target.index as u16;
    assert!(v.valid_action_ids.contains(&id), "{target:?} not selectable: {v:?}");
    r.execute_action(v.selecting_player, id).expect("pick");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_029_printed_metadata_alias_and_digivolve_paths() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(4), Some(5000), Some(5)));
    assert_eq!(c.color, vec![CompiledColor::Yellow, CompiledColor::Red]);
    assert_eq!(c.traits, vec!["Wizard".to_string(), "Witchelny".to_string()]);
    assert_eq!(c.also_treated_as, vec!["Wizardmon".to_string()]);
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 3);
    assert_eq!(
        digi.iter()
            .filter(|p| p.cost == Some(CompiledCost::Literal(3)))
            .count(),
        2,
        "Yellow / Red Lv.3 circles"
    );
    assert_eq!(
        digi.iter()
            .filter(|p| p.cost == Some(CompiledCost::Literal(2)))
            .count(),
        1,
        "Lv.3 w/[Witchelny] in text: Cost 2"
    );
}

#[test]
fn ex13_029_clause_shape() {
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
    assert_eq!(t.len(), 1);
    assert!(t[0].when.contains(&CompiledTiming::WhenDigivolving));
    assert!(t[0].when.contains(&CompiledTiming::WhenAttacking));
    assert!(t[0].once_per_turn, "[Once Per Turn] shared by both timings");
    assert!(t[0].optional, "\"By trashing\" is an optional processing condition");

    let reps: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                scope,
                optional,
                once_per_turn,
                ..
            }) => Some((*scope, *optional, *once_per_turn)),
            _ => None,
        })
        .collect();
    assert_eq!(reps, vec![(CompiledScope::Inherited, true, true)]);
}

#[test]
fn ex13_029_has_armor_purge() {
    let mut r = runner(5);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(me, Keyword::ArmorPurge));
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2/3/4 — [When Digivolving][When Attacking][OPT]
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_029_with_five_security_debuffs_but_does_not_delete() {
    let mut r = runner(5);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let big = r.place_on_field(1, "OPP-8K", Some(0));
    let four = r.place_on_field(1, "OPP-4K", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, me);
    assert!(r.pending_is_optional(), "outer accept/decline before the cost");
    let trash_before = r.game.players[0].trash.len();
    r.accept_optional_trigger().expect("accept");
    assert_eq!(r.security_count(0), 4, "top security trashed as the cost");
    assert_eq!(
        r.game.players[0].trash.len(),
        trash_before + 1,
        "the cost moved the security card to the trash"
    );
    pick_field(&mut r, big);
    let _ = r.auto_resolve();
    assert_eq!(r.effective_dp(big), Some(4000), "-4000 for the turn");
    assert!(r.pending_selection_view().is_none(), "4 security (> 3) → no delete");
    assert!(opp_field_ids(&r).contains(&"OPP-4K".to_string()));
    let _ = four;
}

#[test]
fn ex13_029_with_four_security_debuffs_then_deletes_4000_or_less() {
    let mut r = runner(4);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let big = r.place_on_field(1, "OPP-8K", Some(0));
    let five = r.place_on_field(1, "OPP-5K", Some(0));
    let mut_big = r.place_on_field(1, "OPP-BIG", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, me);
    r.accept_optional_trigger().expect("accept");
    pick_field(&mut r, big);
    assert_eq!(r.security_count(0), 3);
    // Delete prompt: only Digimon with 4000 DP or less — the debuffed 8000
    // (now 4000) qualifies, the 5000 and 13000 do not.
    assert_eq!(offered_field(&r), vec![big.index as u16]);
    pick_field(&mut r, big);
    let _ = r.auto_resolve();
    let opp = opp_field_ids(&r);
    assert!(!opp.contains(&"OPP-8K".to_string()), "deleted");
    assert!(opp.contains(&"OPP-5K".to_string()));
    assert!(opp.contains(&"OPP-BIG".to_string()));
    let _ = (five, mut_big);
}

#[test]
fn ex13_029_delete_may_target_a_different_digimon() {
    let mut r = runner(3);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let big = r.place_on_field(1, "OPP-BIG", Some(0));
    let four = r.place_on_field(1, "OPP-4K", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, me);
    r.accept_optional_trigger().expect("accept");
    pick_field(&mut r, big);
    assert_eq!(offered_field(&r), vec![four.index as u16], "13000-4000 = 9000 is not eligible");
    pick_field(&mut r, four);
    let _ = r.auto_resolve();
    let opp = opp_field_ids(&r);
    assert!(!opp.contains(&"OPP-4K".to_string()));
    assert!(opp.contains(&"OPP-BIG".to_string()));
    assert_eq!(r.effective_dp(PermanentHandle { player: 1, index: 0 }), Some(9000));
}

#[test]
fn ex13_029_declining_the_cost_does_nothing() {
    let mut r = runner(3);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let four = r.place_on_field(1, "OPP-4K", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, me);
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 3);
    assert_eq!(r.effective_dp(four), Some(4000));
    assert!(opp_field_ids(&r).contains(&"OPP-4K".to_string()));
}

#[test]
fn ex13_029_no_security_no_offer() {
    let mut r = runner(0);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let four = r.place_on_field(1, "OPP-4K", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, me);
    assert!(r.pending_selection_view().is_none(), "unpayable cost → not offered");
    assert_eq!(r.effective_dp(four), Some(4000));
}

#[test]
fn ex13_029_when_attacking_fires_integrated() {
    let mut r = runner(4);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let big = r.place_on_field(1, "OPP-BIG", Some(0));
    r.attack_player(me, 1, false);
    assert!(r.pending_is_optional(), "[When Attacking] offers the cost");
    r.accept_optional_trigger().expect("accept");
    pick_field(&mut r, big);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 3);
    assert_eq!(r.effective_dp(big), Some(9000));
}

#[test]
fn ex13_029_when_digivolving_and_when_attacking_share_once_per_turn() {
    let mut r = runner(5);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let big = r.place_on_field(1, "OPP-BIG", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, me);
    r.accept_optional_trigger().expect("accept");
    pick_field(&mut r, big);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 4);
    r.attack_player(me, 1, false);
    for _ in 0..6 {
        let Some(v) = r.pending_selection_view() else { break };
        assert_ne!(v.kind, SelectionKind::OppField, "OPT spent — no second debuff");
        let a = v.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap_or(PASS);
        r.execute_action(v.selecting_player, a).unwrap();
    }
    assert_eq!(r.security_count(0), 4, "[When Attacking] locked out");
}

#[test]
fn ex13_029_once_per_turn_clears_next_turn() {
    let mut r = runner(5);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let big = r.place_on_field(1, "OPP-BIG", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, me);
    r.accept_optional_trigger().expect("accept");
    pick_field(&mut r, big);
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::WhenAttacking, me);
    assert!(r.pending_is_optional(), "OPT reset on the next own turn");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2/3/5 — Inherited leave prevention
// ════════════════════════════════════════════════════════════════════════════

fn inherited_setup(carrier: &str, security: usize) -> (DebugRunner, PermanentHandle) {
    let mut r = runner(security);
    let h = r.place_stack(0, &[CARD_ID, carrier]);
    (r, h)
}

#[test]
fn ex13_029_inherited_witchelny_carrier_trashes_top_security_and_stays() {
    let (mut r, carrier) = inherited_setup("CARRIER-W", 3);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
    assert!(r.pending_is_optional());
    r.accept_optional_trigger().expect("accept");
    let _ = r.auto_resolve();
    assert!(field_ids(&r).contains(&"CARRIER-W".to_string()));
    assert_eq!(r.security_count(0), 2);
}

#[test]
fn ex13_029_inherited_not_offered_for_carrier_without_text() {
    let (mut r, carrier) = inherited_setup("CARRIER-PLAIN", 3);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none());
    assert!(!field_ids(&r).contains(&"CARRIER-PLAIN".to_string()));
}

#[test]
fn ex13_029_inherited_not_offered_for_own_effect() {
    let (mut r, carrier) = inherited_setup("CARRIER-W", 3);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OwnEffect);
    assert!(r.pending_selection_view().is_none());
}

#[test]
fn ex13_029_inherited_not_offered_without_security() {
    let (mut r, carrier) = inherited_setup("CARRIER-W", 0);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none());
}

#[test]
fn ex13_029_inherited_once_per_turn() {
    let (mut r, carrier) = inherited_setup("CARRIER-W", 3);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    r.accept_optional_trigger().expect("accept");
    let _ = r.auto_resolve();
    let idx = field_ids(&r).iter().position(|c| c == "CARRIER-W").unwrap();
    r.game.delete_permanent_with_cause(
        PermanentHandle { player: 0, index: idx as u8 },
        ReplacementCause::OpponentEffect,
    );
    assert!(r.pending_selection_view().is_none(), "OPT spent");
    assert!(!field_ids(&r).contains(&"CARRIER-W".to_string()));
}
