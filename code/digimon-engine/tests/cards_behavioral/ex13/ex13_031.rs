//! EX13-031 KingSukamon — Digimon, Lv.5, Yellow, DP 7000, Cost 7.
//! Traits: Mutant. Form: Ultimate. Attribute: Virus.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-031.md`)
//!
//! ```text
//! Digivolve: Yellow Lv.4 / 3; Black Lv.4 / 3
//! [On Play] [When Digivolving] [On Deletion] By trashing 1 card with [Chuumon]
//! or [Sukamon] in its name from your hand or your Digimon's digivolution
//! cards, you may change the base name, color and DP of 1 of your opponent's
//! Digimon to [Sukamon], white and 3000 until their turn ends.
//! Assembly -4: 3 Lv.4 or lower Digimon cards w/[Sukamon] in name
//! Inherited: [All Turns] [Once Per Turn] When any other Digimon with [Sukamon]
//! in their names are deleted, reveal the top 3 cards of your deck. You may
//! play 1 play cost 3 or lower Digimon card with [Chuumon] or [Sukamon] in its
//! name among them without paying the cost. Trash the rest.
//! ```
//!
//! Official Q&A: the target becomes a Digimon whose ORIGINAL name is
//! [Sukamon], original color white, original DP 3000.
//!
//! # DCGO C# reference
//! None (no `EX13_031.cs` at `b9a0638cd`). "Base DP" follows DCGO's engine
//! `ChangeBaseDPClass`: the base is replaced and +/- DP effects still apply.
//!
//! # Gap closed
//! G-DSL-ADD-MODIFIER-NAME-COLOR-PAYLOAD — typed `add_modifier` payloads
//! (`payload: { name | colors | dp | traits }`).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::{
    encode_source_select, ATTACK_START, PASS, PLAY_HAND_START, SEL_REVEAL_START,
    TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Expiry, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::{PermanentHandle, SynthIdentity};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX13-031";

fn digimon(id: &str, name: &str, level: u8, cost: u16) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = cost;
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-031 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("SUKA-H", "Sukamon", 4, 3))
        .add_card(digimon("CHUU-H", "Chuumon", 3, 3))
        .add_card(digimon("PLAIN-H", "Plainmon", 3, 3))
        .add_card(digimon("HOST", "Host", 5, 5))
        .add_card(digimon("CARRIER", "Carrier", 6, 8))
        .add_card(digimon("SUKA-L4", "Sukamon Four", 4, 3))
        .add_card(digimon("SUKA-L3", "Sukamon Three", 3, 3))
        .add_card(digimon("SUKA-L5", "Sukamon Five", 5, 5))
        .add_card(digimon("CHUU-L4", "Chuumon Four", 4, 3))
        .add_card(digimon("OTHER-SUKA", "Sukamon Ally", 4, 3))
        .add_card(digimon("OTHER-PLAIN", "Plain Ally", 4, 3))
        .add_card({
            let mut c = digimon("OPP", "Greymon", 4, 5);
            c.colors = vec![CardColor::Red];
            c.dp = Some(6000);
            c
        })
        .add_card({
            let mut c = digimon("OPP-SUKA", "Sukamon Rival", 4, 3);
            c.colors = vec![CardColor::Black];
            c
        })
        .add_card(digimon("FILL", "Filler", 3, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, source: PermanentHandle) {
    r.game.enqueue_triggered(timing, TriggerSource::Permanent(source));
    r.game.drain_effect_queue();
}

fn hand_slot(r: &DebugRunner, card_id: &str) -> u16 {
    r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} in hand")) as u16
}

fn pick_hand_cost(r: &mut DebugRunner, card_id: &str) {
    let view = r.pending_selection_view().expect("cost prompt");
    let id = PLAY_HAND_START + hand_slot(r, card_id);
    assert!(view.valid_action_ids.contains(&id), "{card_id} payable: {view:?}");
    r.execute_action(0, id).expect("pay cost");
}

fn pick_field(r: &mut DebugRunner, target: PermanentHandle) {
    let view = r.pending_selection_view().expect("field prompt");
    let id = ATTACK_START + target.index as u16;
    assert!(view.valid_action_ids.contains(&id), "{target:?} not selectable: {view:?}");
    r.execute_action(view.selecting_player, id).expect("pick");
}

fn identity(r: &DebugRunner, h: PermanentHandle) -> SynthIdentity {
    r.game.players[h.player as usize].battle_area[h.index as usize].synth_identity(
        &r.game.card_data,
        &r.game.modifiers,
        h,
    )
}

fn assert_sukamon(r: &DebugRunner, h: PermanentHandle) {
    let id = identity(r, h);
    assert_eq!(id.card_name, "Sukamon");
    assert_eq!(id.card_names, vec!["Sukamon".to_string()]);
    assert_eq!(id.colors, vec![CardColor::White]);
    assert_eq!(r.effective_dp(h), Some(3000));
}

fn assert_unchanged(r: &DebugRunner, h: PermanentHandle) {
    let id = identity(r, h);
    assert_eq!(id.card_name, "Greymon");
    assert_eq!(id.colors, vec![CardColor::Red]);
    assert_eq!(r.effective_dp(h), Some(6000));
}

fn field_ids(r: &DebugRunner, player: usize) -> Vec<String> {
    r.game.players[player]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn trash_ids(r: &DebugRunner, player: usize) -> Vec<String> {
    let mut v: Vec<String> = r.game.players[player]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    v.sort();
    v
}

/// KingSukamon on P0's field, Greymon on P1's, `hand` in P0's hand.
fn main_setup(hand: &[&str]) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = builder().hand(0, hand).start();
    r.skip_mulligan();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    (r, me, opp)
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_031_metadata_and_alt_paths() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(5), Some(7000), Some(7)));
    assert_eq!(c.color, vec![CompiledColor::Yellow]);
    assert_eq!(c.traits, vec!["Mutant".to_string()]);
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 2, "Yellow Lv.4 and Black Lv.4 circles");
    assert!(digi.iter().all(|p| p.cost == Some(CompiledCost::Literal(3))));
    let a = c
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Assembly)
        .expect("Assembly -4");
    assert_eq!(a.cost, Some(CompiledCost::Literal(4)));
    assert_eq!(a.materials.len(), 3);
}

#[test]
fn ex13_031_clause_shapes() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 2);
    let main = t.iter().find(|x| x.scope != CompiledScope::Inherited).expect("main");
    assert_eq!(
        main.when,
        vec![
            CompiledTiming::OnPlay,
            CompiledTiming::WhenDigivolving,
            CompiledTiming::OnDeletion
        ]
    );
    assert!(!main.once_per_turn);
    let inh = t.iter().find(|x| x.scope == CompiledScope::Inherited).expect("inherited");
    assert!(inh.once_per_turn);
}

// ─── Section 2/3 — [On Play] / [When Digivolving] / [On Deletion] ────────────

#[test]
fn ex13_031_on_play_trash_hand_sukamon_changes_target_to_white_3000_sukamon() {
    let (mut r, me, opp) = main_setup(&["SUKA-H"]);
    fire(&mut r, EffectTiming::OnPlay, me);
    assert!(r.pending_is_optional(), "\"By trashing ..., you may\" — the cost is optional");
    pick_hand_cost(&mut r, "SUKA-H");
    assert!(!r.pending_is_optional(), "after paying, the target pick is mandatory");
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    assert!(trash_ids(&r, 0).contains(&"SUKA-H".to_string()));
    assert_eq!(r.hand_size(0), 0);
    assert_sukamon(&r, opp);
}

#[test]
fn ex13_031_cost_can_be_a_chuumon_source_under_your_digimon() {
    let (mut r, me, opp) = main_setup(&[]);
    let host = r.place_stack(0, &["CHUU-L4", "HOST"]);
    fire(&mut r, EffectTiming::OnPlay, me);
    let view = r.pending_selection_view().expect("cost prompt");
    let src = encode_source_select(host.index as u16, 0).expect("source action");
    assert!(view.valid_action_ids.contains(&src), "Chuumon source payable: {view:?}");
    r.execute_action(0, src).expect("trash source");
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    assert!(trash_ids(&r, 0).contains(&"CHUU-L4".to_string()));
    assert_eq!(r.game.players[0].battle_area[host.index as usize].card_sources.len(), 1);
    assert_sukamon(&r, opp);
}

#[test]
fn ex13_031_non_matching_cards_are_not_payable() {
    let (mut r, me, _opp) = main_setup(&["PLAIN-H", "CHUU-H"]);
    fire(&mut r, EffectTiming::OnPlay, me);
    let view = r.pending_selection_view().expect("cost prompt");
    let plain = PLAY_HAND_START + hand_slot(&r, "PLAIN-H");
    let chuu = PLAY_HAND_START + hand_slot(&r, "CHUU-H");
    assert!(!view.valid_action_ids.contains(&plain), "no [Chuumon]/[Sukamon] in name");
    assert!(view.valid_action_ids.contains(&chuu));
}

#[test]
fn ex13_031_declining_the_cost_changes_nothing() {
    let (mut r, me, opp) = main_setup(&["SUKA-H"]);
    fire(&mut r, EffectTiming::OnPlay, me);
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 1, "cost not paid");
    assert!(r.pending_selection_view().is_none());
    assert_unchanged(&r, opp);
}

#[test]
fn ex13_031_no_payable_card_no_prompt() {
    let (mut r, me, opp) = main_setup(&["PLAIN-H"]);
    fire(&mut r, EffectTiming::OnPlay, me);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.hand_size(0), 1);
    assert_unchanged(&r, opp);
}

/// general_rule 15-7-5: the cost may be paid even if the result can't happen.
#[test]
fn ex13_031_cost_is_offered_even_with_no_opponent_digimon() {
    let mut r = builder().hand(0, &["SUKA-H"]).start();
    r.skip_mulligan();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, me);
    assert!(r.pending_is_optional(), "cost still offered");
    pick_hand_cost(&mut r, "SUKA-H");
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none(), "no target to pick");
    assert!(trash_ids(&r, 0).contains(&"SUKA-H".to_string()));
}

#[test]
fn ex13_031_when_digivolving_fires_the_same_effect() {
    let (mut r, me, opp) = main_setup(&["CHUU-H"]);
    fire(&mut r, EffectTiming::WhenDigivolving, me);
    pick_hand_cost(&mut r, "CHUU-H");
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    assert_sukamon(&r, opp);
}

#[test]
fn ex13_031_target_is_chosen_among_opponent_digimon() {
    let (mut r, me, opp) = main_setup(&["SUKA-H"]);
    let opp2 = r.place_on_field(1, "OPP-SUKA", Some(0));
    fire(&mut r, EffectTiming::OnPlay, me);
    pick_hand_cost(&mut r, "SUKA-H");
    let view = r.pending_selection_view().expect("target prompt");
    let offered: Vec<_> = view.valid_action_ids.iter().filter(|&&a| a != PASS).collect();
    assert_eq!(offered.len(), 2, "both opponent Digimon are choosable: {view:?}");
    pick_field(&mut r, opp2);
    let _ = r.auto_resolve();
    assert_unchanged(&r, opp);
    let id = identity(&r, opp2);
    assert_eq!(id.colors, vec![CardColor::White], "black replaced by white");
    assert_eq!(r.effective_dp(opp2), Some(3000), "4000 base → 3000 base");
}

/// "base DP": other DP effects still apply on top of the new 3000 base.
#[test]
fn ex13_031_base_dp_change_keeps_other_dp_modifiers_on_top() {
    let (mut r, me, opp) = main_setup(&["SUKA-H"]);
    r.game.modifiers.add(
        opp,
        ModifierEntry::simple(ModifierType::ChangeDp, 2000, Expiry::EndOfTurn, 1),
    );
    assert_eq!(r.effective_dp(opp), Some(8000));
    fire(&mut r, EffectTiming::OnPlay, me);
    pick_hand_cost(&mut r, "SUKA-H");
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    assert_eq!(r.effective_dp(opp), Some(5000), "3000 base + 2000");
}

/// From your turn, "until their turn ends" spans your turn end and their whole
/// next turn, ending when their turn ends.
#[test]
fn ex13_031_change_lasts_until_the_end_of_the_opponents_turn() {
    let (mut r, me, opp) = main_setup(&["SUKA-H"]);
    fire(&mut r, EffectTiming::OnPlay, me);
    pick_hand_cost(&mut r, "SUKA-H");
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    r.game.modifiers.expire_end_of_turn(0);
    assert_sukamon(&r, opp);
    r.game.modifiers.expire_end_of_turn(1);
    assert_unchanged(&r, opp);
}

#[test]
fn ex13_031_on_deletion_pays_from_hand_and_changes_target() {
    let (mut r, me, opp) = main_setup(&["SUKA-H"]);
    r.game.delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
    assert!(r.pending_is_optional(), "[On Deletion] cost prompt");
    pick_hand_cost(&mut r, "SUKA-H");
    // KingSukamon left the field, so the opponent's Digimon may have shifted.
    let opp_now = PermanentHandle { player: 1, index: 0 };
    pick_field(&mut r, opp_now);
    let _ = r.auto_resolve();
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_sukamon(&r, opp);
}

/// The renamed Digimon's rules names are exactly [Sukamon] (its old name is
/// gone), which is what every `name_contains` filter reads via the synth
/// overlay.
#[test]
fn ex13_031_renamed_target_matches_sukamon_name_filters() {
    let (mut r, me, opp) = main_setup(&["SUKA-H"]);
    fire(&mut r, EffectTiming::OnPlay, me);
    pick_hand_cost(&mut r, "SUKA-H");
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    assert!(identity(&r, opp)
        .card_names
        .iter()
        .any(|n| n.contains("Sukamon")));
    assert!(!identity(&r, opp).card_names.iter().any(|n| n == "Greymon"));
}

// ─── Section 3/5 — Inherited [All Turns][OPT] ────────────────────────────────

/// Carrier with KingSukamon as a source; deck top 3 = `top3` (last = top).
fn inherited_setup(top3_bottom_first: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut deck = vec!["FILL", "FILL"];
    deck.extend_from_slice(top3_bottom_first);
    let mut r = builder().deck(0, &deck).memory(0).start();
    r.skip_mulligan();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    (r, carrier)
}

fn offered_reveal_ids(r: &DebugRunner) -> Vec<String> {
    let view = r.pending_selection_view().expect("reveal prompt pending");
    let mut ids: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| a.checked_sub(SEL_REVEAL_START))
        .filter_map(|i| r.game.revealed_cards.get(i as usize))
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    ids.sort();
    ids
}

fn pick_revealed(r: &mut DebugRunner, card_id: &str) {
    let view = r.pending_selection_view().expect("reveal prompt pending");
    let want = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .find(|&a| {
            a.checked_sub(SEL_REVEAL_START)
                .and_then(|i| r.game.revealed_cards.get(i as usize))
                .is_some_and(|c| c.card_id(&r.game.card_data) == card_id)
        })
        .unwrap_or_else(|| panic!("{card_id} must be a legal pick: {view:?}"));
    r.execute_action(view.selecting_player, want).expect("pick revealed");
}

fn delete(r: &mut DebugRunner, h: PermanentHandle) {
    r.game.delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
}

#[test]
fn ex13_031_inherited_other_sukamon_deleted_reveals_and_plays_free() {
    let (mut r, carrier) = inherited_setup(&["PLAIN-H", "CHUU-H", "SUKA-L5"]);
    let ally = r.place_on_field(0, "OTHER-SUKA", Some(0));
    delete(&mut r, ally);
    assert_eq!(
        offered_reveal_ids(&r),
        vec!["CHUU-H".to_string()],
        "cost-5 Sukamon and non-matching names excluded"
    );
    pick_revealed(&mut r, "CHUU-H");
    let _ = r.auto_resolve();
    let field = field_ids(&r, 0);
    assert!(field.contains(&"CHUU-H".to_string()), "played: {field:?}");
    assert_eq!(r.memory(), 0, "without paying the cost");
    let trash = trash_ids(&r, 0);
    assert!(trash.contains(&"PLAIN-H".to_string()) && trash.contains(&"SUKA-L5".to_string()));
    assert_eq!(r.deck_size(0), 2);
    let _ = carrier;
}

#[test]
fn ex13_031_inherited_decline_trashes_all_three() {
    let (mut r, _carrier) = inherited_setup(&["PLAIN-H", "CHUU-H", "SUKA-H"]);
    let ally = r.place_on_field(0, "OTHER-SUKA", Some(0));
    delete(&mut r, ally);
    assert!(r.pending_is_optional(), "\"you may\" play");
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    let trash = trash_ids(&r, 0);
    for id in ["PLAIN-H", "CHUU-H", "SUKA-H", "OTHER-SUKA"] {
        assert!(trash.contains(&id.to_string()), "{id} trashed: {trash:?}");
    }
    assert_eq!(r.deck_size(0), 2);
}

#[test]
fn ex13_031_inherited_ignores_non_sukamon_deletions() {
    let (mut r, _carrier) = inherited_setup(&["PLAIN-H", "CHUU-H", "SUKA-H"]);
    let ally = r.place_on_field(0, "OTHER-PLAIN", Some(0));
    delete(&mut r, ally);
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.deck_size(0), 5);
}

#[test]
fn ex13_031_inherited_fires_on_an_opponents_sukamon_too() {
    let (mut r, _carrier) = inherited_setup(&["PLAIN-H", "CHUU-H", "SUKA-H"]);
    let rival = r.place_on_field(1, "OPP-SUKA", Some(0));
    delete(&mut r, rival);
    assert_eq!(
        offered_reveal_ids(&r),
        vec!["CHUU-H".to_string(), "SUKA-H".to_string()],
        "\"any other Digimon\" includes the opponent's"
    );
}

#[test]
fn ex13_031_inherited_is_once_per_turn() {
    let (mut r, _carrier) = inherited_setup(&["PLAIN-H", "CHUU-H", "SUKA-H"]);
    let a = r.place_on_field(0, "OTHER-SUKA", Some(0));
    r.place_on_field(0, "SUKA-L4", Some(0));
    delete(&mut r, a);
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    let deck_after_first = r.deck_size(0);
    let b_idx = field_ids(&r, 0).iter().position(|c| c == "SUKA-L4").expect("SUKA-L4");
    delete(
        &mut r,
        PermanentHandle {
            player: 0,
            index: b_idx as u8,
        },
    );
    assert!(r.pending_selection_view().is_none(), "already used this turn");
    assert_eq!(r.deck_size(0), deck_after_first);
}

/// The combo: the main effect renames an opponent's Digimon [Sukamon]; when it
/// is then deleted it IS a Digimon with [Sukamon] in its name (last-known
/// information), so the inherited effect fires.
#[test]
fn ex13_031_renamed_opponent_digimon_deletion_fires_the_inherited() {
    let mut r = builder()
        .hand(0, &["SUKA-H"])
        .deck(0, &["FILL", "FILL", "PLAIN-H", "CHUU-H", "SUKA-L4"])
        .start();
    r.skip_mulligan();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::OnPlay, me);
    pick_hand_cost(&mut r, "SUKA-H");
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    assert_sukamon(&r, opp);
    delete(&mut r, opp);
    assert_eq!(
        offered_reveal_ids(&r),
        vec!["CHUU-H".to_string(), "SUKA-L4".to_string()],
        "the renamed Greymon died as a [Sukamon]"
    );
    let _ = carrier;
}

#[test]
fn ex13_031_unrenamed_opponent_digimon_deletion_does_not_fire() {
    let (mut r, _carrier) = inherited_setup(&["PLAIN-H", "CHUU-H", "SUKA-H"]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    delete(&mut r, opp);
    assert!(r.pending_selection_view().is_none());
}

// ─── Section 3 — Assembly ────────────────────────────────────────────────────

fn trash_index(r: &DebugRunner, card_id: &str) -> u16 {
    r.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} in trash")) as u16
}

#[test]
fn ex13_031_assembly_three_lv4_or_lower_sukamon_for_3() {
    let mut r = builder().hand(0, &[CARD_ID]).memory(7).start();
    r.skip_mulligan();
    for id in ["SUKA-L4", "SUKA-L3", "OTHER-SUKA"] {
        r.inject_trash(0, id);
    }
    let mem0 = r.game.memory;
    r.game.decode_action(PLAY_HAND_START, 0);
    for id in ["SUKA-L4", "SUKA-L3", "OTHER-SUKA"] {
        let view = r.pending_selection_view().unwrap_or_else(|| panic!("Assembly element {id}"));
        let a = TRASH_EFFECT_START + trash_index(&r, id);
        assert!(view.valid_action_ids.contains(&a), "{id} pickable: {view:?}");
        r.game.decode_action(a, 0);
    }
    // KingSukamon's own [On Play] now offers its optional cost — the three
    // fresh [Sukamon] materials are payable sources. Decline it.
    assert!(r.pending_is_optional(), "[On Play] cost prompt after the Assembly play");
    r.decline_optional_trigger().expect("decline [On Play] cost");
    let _ = r.auto_resolve();
    let stack = &r.game.players[0].battle_area[0].card_sources;
    assert_eq!(stack.len(), 4, "3 materials under KingSukamon");
    assert_eq!(mem0 - r.game.memory, 3, "7 − 4 = 3");
}

#[test]
fn ex13_031_assembly_excludes_lv5_and_non_sukamon_materials() {
    let mut r = builder().hand(0, &[CARD_ID]).memory(7).start();
    r.skip_mulligan();
    for id in ["SUKA-L4", "SUKA-L5", "CHUU-L4", "SUKA-L3", "OTHER-SUKA"] {
        r.inject_trash(0, id);
    }
    r.game.decode_action(PLAY_HAND_START, 0);
    let view = r.pending_selection_view().expect("Assembly element prompt");
    for (id, legal) in [
        ("SUKA-L4", true),
        ("SUKA-L3", true),
        ("OTHER-SUKA", true),
        ("SUKA-L5", false),
        ("CHUU-L4", false),
    ] {
        let a = TRASH_EFFECT_START + trash_index(&r, id);
        assert_eq!(view.valid_action_ids.contains(&a), legal, "{id} legal={legal}: {view:?}");
    }
}
