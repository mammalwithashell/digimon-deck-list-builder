//! EX13-043 Leopardmon — Digimon, Lv.6, Green, DP 12000, Cost 12.
//! Traits: Holy Warrior / Royal Knight. Attribute: Data.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-043.md`)
//!
//! [On Play] [When Digivolving] You may suspend 1 Digimon. Then, you may
//! return 1 of your opponent's lowest DP Digimon to the bottom of the deck.
//! [When Digivolving] [When Attacking] [Once Per Turn] You may play or use 1
//! [Mammal], [Beast], [Beastkin] or [Royal Knight] trait card from your hand
//! with the cost reduced by 4. For each suspended Digimon, further reduce it
//! by 1.
//! [All Turns] [Once Per Turn] When any of your suspended Digimon would leave
//! the battle area other than by your effects, by unsuspending 1 of your
//! Digimon, they don't leave.
//! Assembly -5: Lv.5 × Lv.4 × Lv.3, all green w/[Mammal]/[Beast]/[Beastkin] trait
//! Digivolve: Green Lv.5 / cost 3; [Leopardmon: Leopard Mode]: cost 1.
//!
//! # DCGO C# reference
//! None at the submodule (no EX13_043.cs); printed text governs.
//!
//! # Patterns this test covers
//! - Optional any-side suspend + optional lowest-DP bottom-deck (independent).
//! - D2-adjacent formula cost reduction (4 + suspended Digimon count) on a
//!   play-or-use from hand, OPT refunded on decline.
//! - F3 leave-replacement over own suspended Digimon paid by an unsuspend.
//! - Assembly with colour + trait per-level materials.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledCost, CompiledTiming};
use digimon_engine::action::space::{
    encode_attack, ATTACK_START, HAND_EFFECT_START, PASS, PLAY_HAND_START, TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX13-043";

fn digimon(id: &str, level: u8, dp: i32, cost: u16, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = cost;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-043 YAML loads")
        .add_card(make_test_card("PAD", "PAD"))
        .add_card(digimon("BEAST7", 5, 7000, 7, CardColor::Green, &["Beast"]))
        .add_card(digimon("RK9", 6, 11000, 9, CardColor::Yellow, &["Royal Knight"]))
        .add_card(digimon("PLAIN", 4, 5000, 4, CardColor::Green, &["Dragon"]))
        .add_card(digimon("G5", 5, 7000, 7, CardColor::Green, &["Mammal"]))
        .add_card(digimon("G4", 4, 5000, 5, CardColor::Green, &["Beastkin"]))
        .add_card(digimon("G3", 3, 3000, 3, CardColor::Green, &["Beast"]))
        .add_card(digimon("OPP-LOW", 3, 2000, 3, CardColor::Red, &[]))
        .add_card(digimon("OPP-HIGH", 6, 11000, 9, CardColor::Red, &[]))
        .add_card(digimon("ALLY", 4, 5000, 4, CardColor::Green, &[]))
        .deck(0, &["PAD"; 8])
        .deck(1, &["PAD"; 8])
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, perm: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();
}

fn field_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn is_suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} in hand"))
}

fn hand_action(runner: &DebugRunner, card_id: &str) -> Option<u16> {
    let view = runner.pending_selection_view()?;
    let slot = hand_index(runner, 0, card_id) as u16;
    [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
}

fn trash_index(runner: &DebugRunner, player: u8, card_id: &str) -> u16 {
    runner.game.players[player as usize]
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} in trash")) as u16
}

// ─── Section 1 — Structural ───────────────────────────────────────────────────

#[test]
fn ex13_043_metadata_and_alt_paths() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(6), Some(12000), Some(12)));
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 2);
    assert!(digi.iter().any(|p| p.cost == Some(CompiledCost::Literal(1))
        && p.from.as_ref().and_then(|f| f.name_is.as_deref()) == Some("Leopardmon: Leopard Mode")));
    let a = c
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Assembly)
        .expect("Assembly");
    assert_eq!(a.materials.len(), 3);
}

#[test]
fn ex13_043_clause_shapes() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 2);
    let play = t
        .iter()
        .find(|x| x.when == vec![CompiledTiming::WhenDigivolving, CompiledTiming::WhenAttacking])
        .expect("WD/WA");
    assert!(play.once_per_turn);
    assert_eq!(c.effects.len(), 3, "+ the leave-replacement");
}

// ─── Section 2/3 — OP/WD suspend + bottom-deck ───────────────────────────────

#[test]
fn ex13_043_on_play_may_suspend_any_digimon_then_bottom_deck_the_lowest() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(12).start();
    runner.skip_mulligan();
    let low = runner.place_on_field(1, "OPP-LOW", Some(0));
    let high = runner.place_on_field(1, "OPP-HIGH", Some(0));
    runner.play(0, 0).expect("plays");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::AnyField), "either side (Q&A)");
    assert!(runner.pending_is_optional());
    runner
        .execute_action(0, encode_attack(1, high.index as u16))
        .expect("suspend OPP-HIGH");
    assert!(is_suspended(&runner, high));
    let v = runner.pending_selection_view().expect("bottom-deck prompt");
    assert!(v.is_optional);
    assert!(
        !v.valid_action_ids.contains(&(ATTACK_START + high.index as u16)),
        "only the lowest DP"
    );
    runner
        .execute_action(0, ATTACK_START + low.index as u16)
        .expect("bottom-deck OPP-LOW");
    let _ = runner.auto_resolve();
    assert_eq!(field_ids(&runner, 1), vec!["OPP-HIGH".to_string()]);
    assert_eq!(
        runner.game.players[1].deck.first().map(|c| c.card_id(&runner.game.card_data).to_string()),
        Some("OPP-LOW".to_string()),
        "returned to the bottom of its owner's deck"
    );
}

#[test]
fn ex13_043_declining_the_suspend_still_offers_the_bottom_deck() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-LOW", Some(0));
    fire(&mut runner, EffectTiming::OnPlay, me);
    runner.execute_action(0, PASS).expect("decline suspend");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    runner.execute_action(0, PASS).expect("decline bottom");
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(1), 1);
}

// ─── Section 2/3/5 — WD/WA play-or-use with reduction ────────────────────────

#[test]
fn ex13_043_wd_plays_a_beast_card_with_4_plus_suspended_count_reduction() {
    let mut runner = builder().hand(0, &["BEAST7"]).memory(10).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-HIGH", Some(0));
    runner.game.suspend(opp);
    // OnPlay/WD shared clause would also fire; drive only the WD/WA one via WA.
    let mem0 = runner.memory();
    fire(&mut runner, EffectTiming::WhenAttacking, me);
    let a = hand_action(&runner, "BEAST7").expect("BEAST7 selectable");
    runner.execute_action(0, a).expect("play");
    let _ = runner.auto_resolve();
    assert!(field_ids(&runner, 0).contains(&"BEAST7".to_string()));
    assert_eq!(mem0 - runner.memory(), 7 - 4 - 1, "1 suspended Digimon → reduce by 5");
}

#[test]
fn ex13_043_royal_knight_trait_qualifies_and_plain_does_not() {
    let mut runner = builder().hand(0, &["RK9", "PLAIN"]).memory(10).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    fire(&mut runner, EffectTiming::WhenAttacking, me);
    assert!(hand_action(&runner, "RK9").is_some());
    assert!(hand_action(&runner, "PLAIN").is_none());
    let mem0 = runner.memory();
    let a = hand_action(&runner, "RK9").unwrap();
    runner.execute_action(0, a).expect("play");
    let _ = runner.auto_resolve();
    assert_eq!(mem0 - runner.memory(), 9 - 4, "no suspended Digimon → reduce by 4");
}

#[test]
fn ex13_043_play_decline_keeps_the_opt_and_use_spends_it() {
    let mut runner = builder().hand(0, &["BEAST7", "RK9"]).memory(10).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    fire(&mut runner, EffectTiming::WhenAttacking, me);
    runner.execute_action(0, PASS).expect("decline");
    let _ = runner.auto_resolve();
    fire(&mut runner, EffectTiming::WhenAttacking, me);
    let a = hand_action(&runner, "BEAST7").expect("offered again (OPT refunded)");
    runner.execute_action(0, a).expect("play");
    let _ = runner.auto_resolve();
    fire(&mut runner, EffectTiming::WhenAttacking, me);
    assert!(runner.pending_selection().is_none(), "OPT spent");
}

// ─── Section 2/3/5 — leave-replacement ───────────────────────────────────────

#[test]
fn ex13_043_suspended_ally_removed_by_opponent_is_saved_by_unsuspending() {
    let mut runner = builder().start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    runner.game.suspend(ally);
    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    assert!(runner.pending_is_optional(), "the replacement may be declined");
    let _ = runner.auto_resolve();
    assert!(field_ids(&runner, 0).contains(&"ALLY".to_string()), "it doesn't leave");
    assert!(!is_suspended(&runner, ally), "the unsuspend cost was paid on it");
}

#[test]
fn ex13_043_unsuspended_ally_is_not_covered() {
    let mut runner = builder().start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    assert!(runner.pending_selection().is_none());
    assert!(!field_ids(&runner, 0).contains(&"ALLY".to_string()));
}

#[test]
fn ex13_043_own_effect_removal_is_not_covered() {
    let mut runner = builder().start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    runner.game.suspend(ally);
    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::OwnEffect);
    assert!(runner.pending_selection().is_none());
    assert!(!field_ids(&runner, 0).contains(&"ALLY".to_string()));
}

#[test]
fn ex13_043_declining_the_replacement_lets_it_leave() {
    let mut runner = builder().start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    runner.game.suspend(ally);
    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::Battle);
    let p = runner.pending_selection().expect("prompt").selecting_player;
    runner.execute_action(p, PASS).expect("decline");
    let _ = runner.auto_resolve();
    assert!(!field_ids(&runner, 0).contains(&"ALLY".to_string()));
}

#[test]
fn ex13_043_replacement_is_once_per_turn() {
    let mut runner = builder().start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    runner.game.suspend(ally);
    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    let _ = runner.auto_resolve();
    let ally = runner.perm_handle(0, 1);
    runner.game.suspend(ally);
    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    let _ = runner.auto_resolve();
    assert!(!field_ids(&runner, 0).contains(&"ALLY".to_string()), "OPT spent");
}

// ─── Section 3 — Assembly ────────────────────────────────────────────────────

#[test]
fn ex13_043_assembly_green_mammal_beast_beastkin_line_for_7() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(7).start();
    runner.skip_mulligan();
    for id in ["G5", "G4", "G3"] {
        runner.inject_trash(0, id);
    }
    let mem0 = runner.game.memory;
    runner.game.decode_action(PLAY_HAND_START, 0);
    for id in ["G5", "G4", "G3"] {
        assert!(runner.game.pending_selection.is_some(), "Assembly element {id}");
        let idx = trash_index(&runner, 0, id);
        runner.game.decode_action(TRASH_EFFECT_START + idx, 0);
    }
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].battle_area[0].card_sources.len(), 4);
    assert_eq!(mem0 - runner.game.memory, 7);
}
