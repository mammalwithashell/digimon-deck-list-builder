//! BT12-011 Shoutmon (King Version) — Digimon, Lv.4, Red, DP 4000, Cost 5.
//! Traits: Mini Dragon, Xros Heart. Attribute: Data.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/BT12-011.md`)
//! Digivolve: Red Lv.3 / Cost 2. `[Digivolve] 2 from Lv.3 w/＜Save＞ in text`.
//! `[On Play][When Digivolving] You may play 1 [Taiki Kudo], [Yuu Amano], or`
//! `[Tagiru Akashi] from your hand without paying the cost.`
//! `[On Deletion] ＜Save＞. Then, place 1 Digimon card with ＜Save＞ in its`
//! `text from your trash under 1 of your Tamers.`
//! Special Play Condition: `DigiXros -2: 1 Digimon card w/＜Save＞ in text`.
//! Inherited: `[When Attacking][Once Per Turn] If this Digimon has ＜Save＞ in`
//! `its text, delete 1 of your opponent's Digimon with 4000 DP or less.`
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT12/Red/BT12_011.cs
//!   - Alt-digivolve: base TopCard.HasSaveText && Level == 3, cost 2.
//!   - [On Play]/[When Digivolving]: SelectHandEffect canNoSelect:true → free play.
//!   - [On Deletion]: CanActivateSave (own Tamer) → SaveProcess (optional) →
//!     trash pick canNoSelect:false (HasSaveText Digimon) → Tamer pick
//!     canNoSelect:false → AddDigivolutionCardsBottom.
//!   - Inherited [When Attacking] maxCount 1: carrier TopCard.HasSaveText →
//!     mandatory delete of an opponent Digimon with DP <= 4000.
//!   - DigiXros -2: 1 Digimon card with HasSaveText.
//!
//! # Patterns
//! - [On Play]+[When Digivolving] optional free play from hand by name
//! - On Deletion Save idiom (EX10-044) + sequenced trash→Tamer placement
//! - inherited [When Attacking][OPT] conditional on carrier text
//! - text-gated alt-digivolve + DigiXros

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT12-011";
const SAVE_TEXT: &str = "[On Deletion] ＜Save＞ (You may place this card under one of your Tamers.)";
const PRINTED_EFFECT: &str = "[On Play] [When Digivolving] You may play 1 [Taiki Kudo], [Yuu Amano] or [Tagiru Akashi] from your hand without paying its cost.\r\n[On Deletion] ＜Save＞. Then, place 1 Digimon card with ＜Save＞ in its text from your trash under 1 of your Tamers.";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn digimon(id: &str, name: &str, level: u8, dp: i32, color: CardColor, text: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c.effect_text = text.to_string();
    c
}

fn tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-011 loads")
        .add_card(tamer("TAIKI", "Taiki Kudo"))
        .add_card(tamer("YUU", "Yuu Amano"))
        .add_card(tamer("TAGIRU", "Tagiru Akashi"))
        .add_card(tamer("OTHER-TAMER", "Kiriha Aonuma"))
        .add_card(tamer("TAMER", "Plain Tamer"))
        .add_card(digimon("SAVE-DIGI", "Starmons", 3, 1000, CardColor::Yellow, SAVE_TEXT))
        .add_card(digimon("PLAIN-DIGI", "Agumon", 3, 2000, CardColor::Yellow, ""))
        .add_card(digimon("MSAVE-DIGI", "X4", 4, 8000, CardColor::Red, "＜Material Save 2＞"))
        .add_card(digimon("RED-LV3", "Shoutmon", 3, 2000, CardColor::Red, ""))
        .add_card(digimon("OPP-4000", "Opp4k", 4, 4000, CardColor::Blue, ""))
        .add_card(digimon("OPP-5000", "Opp5k", 4, 5000, CardColor::Blue, ""))
        .add_card(digimon("FILLER", "Filler", 3, 1000, CardColor::Red, ""))
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .memory(10)
}

fn base(hand: &[&str]) -> DebugRunner {
    let mut r = builder().hand(0, hand).start();
    r.skip_mulligan();
    r.set_first_player(0);
    r
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, h: PermanentHandle) {
    r.game.enqueue_triggered(timing, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

fn non_pass(r: &DebugRunner) -> Vec<u16> {
    r.pending_selection_view()
        .map(|v| v.valid_action_ids.into_iter().filter(|&a| a != PASS).collect())
        .unwrap_or_default()
}

fn seed_trash(r: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let next = r.game.next_card_index();
    r.game.players[p as usize].trash.push(CardSource::new(data_idx, p, next));
}

fn ids(r: &DebugRunner, cards: &[CardSource]) -> Vec<String> {
    cards.iter().map(|c| c.card_id(&r.game.card_data).to_string()).collect()
}

fn sources_of_field_card(r: &DebugRunner, p: u8, top_id: &str) -> Vec<String> {
    let perm = r.game.players[p as usize]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == top_id)
        .expect("permanent on field");
    ids(r, &perm.card_sources)
}

fn trash_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].trash)
}

fn on_field(r: &DebugRunner, p: u8, id: &str) -> bool {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .any(|x| x.top_card().card_id(&r.game.card_data) == id)
}

/// Mirror the live game, where `CardData` text is loaded from cards.json
/// (the embedded DSL pack leaves text fields empty).
fn set_printed_text(r: &mut DebugRunner) {
    for cd in r.game.card_data.iter_mut().filter(|c| c.card_id == CARD_ID) {
        cd.effect_text = PRINTED_EFFECT.to_string();
        cd.inherited_text = "[When Attacking] [Once Per Turn] If this Digimon has ＜Save＞ in its text, delete 1 of your opponent's Digimon with 4000 DP or less.".to_string();
    }
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn bt12_011_metadata_and_alt_paths() {
    let r = base(&[]);
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Shoutmon (King Version)");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(4000));
    let digivolves = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .count();
    assert_eq!(digivolves, 2, "Red Lv.3 circle + Lv.3 w/<Save> special");
    let xros = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::DigiXros)
        .expect("DigiXros -2 path");
    assert_eq!(xros.materials.len(), 1, "DigiXros: 1 material");
}

#[test]
fn bt12_011_clause_shape() {
    let r = base(&[]);
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let trig: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(trig.len(), 3);
    assert!(trig.iter().any(|t| t.scope != CompiledScope::Inherited
        && t.when.contains(&CompiledTiming::OnPlay)
        && t.when.contains(&CompiledTiming::WhenDigivolving)));
    assert!(trig.iter().any(|t| t.scope != CompiledScope::Inherited
        && t.when.contains(&CompiledTiming::OnDeletion)));
    assert!(trig.iter().any(|t| t.scope == CompiledScope::Inherited
        && t.when.contains(&CompiledTiming::WhenAttacking)
        && t.once_per_turn));
}

// ─── Section 2 — [On Play][When Digivolving] free-play named Tamer ──────────

#[test]
fn bt12_011_on_play_plays_named_tamer_free() {
    let mut r = base(&["TAIKI", "OTHER-TAMER"]);
    let shout = r.place_on_field(0, CARD_ID, Some(0));
    let mem = r.memory();

    fire(&mut r, EffectTiming::OnPlay, shout);
    let view = r.pending_selection_view().expect("hand pick");
    assert!(view.is_optional, "'You may play'");
    let picks = non_pass(&r);
    assert_eq!(picks.len(), 1, "only Taiki Kudo is eligible (not Kiriha)");
    r.execute_action(0, picks[0]).unwrap();
    let _ = r.auto_resolve();

    assert!(on_field(&r, 0, "TAIKI"), "Taiki Kudo played");
    assert!(!on_field(&r, 0, "OTHER-TAMER"));
    assert_eq!(r.memory(), mem, "without paying the cost");
}

#[test]
fn bt12_011_all_three_names_are_eligible() {
    let mut r = base(&["TAIKI", "YUU", "TAGIRU", "OTHER-TAMER"]);
    let shout = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, shout);
    assert_eq!(non_pass(&r).len(), 3, "Taiki, Yuu and Tagiru eligible");
}

#[test]
fn bt12_011_when_digivolving_plays_named_tamer_free() {
    let mut r = base(&["YUU"]);
    let shout = r.place_stack(0, &["RED-LV3", CARD_ID]);
    let mem = r.memory();

    fire(&mut r, EffectTiming::WhenDigivolving, shout);
    let picks = non_pass(&r);
    assert_eq!(picks.len(), 1);
    r.execute_action(0, picks[0]).unwrap();
    let _ = r.auto_resolve();

    assert!(on_field(&r, 0, "YUU"));
    assert_eq!(r.memory(), mem);
}

#[test]
fn bt12_011_on_play_decline_plays_nothing() {
    let mut r = base(&["TAGIRU"]);
    let shout = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, shout);
    assert!(r.pending_is_optional());
    r.execute_action(0, PASS).unwrap();
    let _ = r.auto_resolve();
    assert!(!on_field(&r, 0, "TAGIRU"));
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn bt12_011_on_play_no_named_tamer_no_play() {
    let mut r = base(&["OTHER-TAMER"]);
    let shout = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, shout);
    let _ = r.auto_resolve();
    assert!(!on_field(&r, 0, "OTHER-TAMER"));
    assert_eq!(r.hand_size(0), 1);
}

// ─── Section 3 — [On Deletion] <Save>. Then, trash → under Tamer ────────────

fn delete_shoutmon(r: &mut DebugRunner, shout: PermanentHandle) {
    let _ = r
        .game
        .delete_permanents_batch(vec![shout], ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
}

#[test]
fn bt12_011_on_deletion_save_then_places_save_text_card_from_trash() {
    let mut r = base(&[]);
    r.place_on_field(0, "TAMER", Some(0));
    seed_trash(&mut r, 0, "SAVE-DIGI");
    seed_trash(&mut r, 0, "PLAIN-DIGI");
    seed_trash(&mut r, 0, "MSAVE-DIGI");
    let shout = r.place_on_field(0, CARD_ID, Some(0));

    delete_shoutmon(&mut r, shout);

    // 1) <Save> — optional Tamer pick.
    let v = r.pending_selection_view().expect("Save prompt");
    assert!(v.is_optional, "<Save> is optional");
    let t = non_pass(&r);
    assert_eq!(t.len(), 1);
    r.execute_action(0, t[0]).unwrap();

    // 2) Then — trash pick: only the <Save>-text Digimon card (not the
    //    plain one, not the <Material Save> one).
    let v = r.pending_selection_view().expect("trash pick");
    assert!(!v.is_optional, "the trash placement is mandatory (canNoSelect:false)");
    let picks = non_pass(&r);
    assert_eq!(picks.len(), 1, "only the <Save>-text Digimon card is eligible");
    r.execute_action(0, picks[0]).unwrap();

    // 3) Tamer pick (mandatory).
    if let Some(v) = r.pending_selection_view() {
        assert!(!v.is_optional);
        let t = non_pass(&r);
        r.execute_action(0, t[0]).unwrap();
    }
    let _ = r.auto_resolve();

    let srcs = sources_of_field_card(&r, 0, "TAMER");
    assert!(srcs.iter().any(|c| c == CARD_ID), "Shoutmon saved under the Tamer");
    assert!(srcs.iter().any(|c| c == "SAVE-DIGI"), "<Save>-text card placed under the Tamer");
    let trash = trash_ids(&r, 0);
    assert!(!trash.iter().any(|c| c == CARD_ID || c == "SAVE-DIGI"));
    assert!(trash.iter().any(|c| c == "PLAIN-DIGI"));
}

#[test]
fn bt12_011_on_deletion_declined_save_still_places_and_self_is_eligible() {
    // In the live game Shoutmon (King Version) itself has <Save> in its text,
    // so with Save declined it is a legal trash pick for the "Then" step.
    let mut r = base(&[]);
    set_printed_text(&mut r);
    r.place_on_field(0, "TAMER", Some(0));
    seed_trash(&mut r, 0, "SAVE-DIGI");
    let shout = r.place_on_field(0, CARD_ID, Some(0));

    delete_shoutmon(&mut r, shout);

    let v = r.pending_selection_view().expect("Save prompt");
    assert!(v.is_optional);
    r.execute_action(0, PASS).expect("decline <Save>");

    let v = r.pending_selection_view().expect("the 'Then' trash pick still runs");
    assert!(!v.is_optional);
    assert_eq!(
        non_pass(&r).len(),
        2,
        "SAVE-DIGI and the deleted Shoutmon (King Version) itself are eligible"
    );
    // Pick both remaining steps and confirm a placement under the Tamer.
    let p = non_pass(&r);
    r.execute_action(0, p[0]).unwrap();
    if r.pending_selection_view().is_some() {
        let t = non_pass(&r);
        r.execute_action(0, t[0]).unwrap();
    }
    let _ = r.auto_resolve();
    let srcs = sources_of_field_card(&r, 0, "TAMER");
    assert_eq!(srcs.len(), 2, "Tamer + exactly one placed card");
}

#[test]
fn bt12_011_on_deletion_single_save_prompt_with_printed_text() {
    // With the printed text loaded (live game), the innate ＜Save＞ token must
    // not install a SECOND, independent <Save> trigger beside the authored
    // clause (DCGO registers one OnDestroyedAnyone effect).
    let mut r = base(&[]);
    set_printed_text(&mut r);
    r.place_on_field(0, "TAMER", Some(0));
    let shout = r.place_on_field(0, CARD_ID, Some(0));

    delete_shoutmon(&mut r, shout);

    let mut prompts = 0;
    for _ in 0..8 {
        let Some(v) = r.pending_selection_view() else { break };
        prompts += 1;
        let a = v.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap_or(PASS);
        if r.execute_action(v.selecting_player, a).is_err() {
            break;
        }
    }
    // Save (Tamer pick) → nothing left in trash with <Save> (self is saved)
    // ⇒ exactly one prompt.
    assert_eq!(prompts, 1, "exactly one <Save> decision, no duplicate keyword trigger");
    let srcs = sources_of_field_card(&r, 0, "TAMER");
    assert_eq!(srcs.iter().filter(|c| *c == CARD_ID).count(), 1);
}

#[test]
fn bt12_011_on_deletion_no_tamer_does_nothing() {
    let mut r = base(&[]);
    seed_trash(&mut r, 0, "SAVE-DIGI");
    let shout = r.place_on_field(0, CARD_ID, Some(0));
    delete_shoutmon(&mut r, shout);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    let trash = trash_ids(&r, 0);
    assert!(trash.iter().any(|c| c == CARD_ID));
    assert!(trash.iter().any(|c| c == "SAVE-DIGI"));
}

// ─── Section 4 — Inherited [When Attacking][OPT] ────────────────────────────

#[test]
fn bt12_011_inherited_deletes_4000_when_carrier_has_save_text() {
    let mut r = base(&[]);
    let carrier = r.place_stack(0, &[CARD_ID, "SAVE-DIGI"]);
    r.place_on_field(1, "OPP-4000", Some(0));
    r.place_on_field(1, "OPP-5000", Some(0));

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let picks = non_pass(&r);
    assert_eq!(picks.len(), 1, "only the 4000 DP Digimon is a legal target");
    assert!(!r.pending_is_optional(), "the delete is mandatory");
    r.execute_action(0, picks[0]).unwrap();
    let _ = r.auto_resolve();

    assert!(!on_field(&r, 1, "OPP-4000"), "4000 DP Digimon deleted");
    assert!(on_field(&r, 1, "OPP-5000"));
}

#[test]
fn bt12_011_inherited_no_save_text_no_delete() {
    let mut r = base(&[]);
    let carrier = r.place_stack(0, &[CARD_ID, "PLAIN-DIGI"]);
    r.place_on_field(1, "OPP-4000", Some(0));

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();

    assert!(r.game.pending_selection.is_none());
    assert!(on_field(&r, 1, "OPP-4000"), "carrier lacks <Save> in its text");
}

#[test]
fn bt12_011_inherited_material_save_is_not_save() {
    let mut r = base(&[]);
    let carrier = r.place_stack(0, &[CARD_ID, "MSAVE-DIGI"]);
    r.place_on_field(1, "OPP-4000", Some(0));

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();

    assert!(on_field(&r, 1, "OPP-4000"), "<Material Save> is not <Save>");
}

#[test]
fn bt12_011_inherited_once_per_turn() {
    let mut r = base(&[]);
    let carrier = r.place_stack(0, &[CARD_ID, "SAVE-DIGI"]);
    r.place_on_field(1, "OPP-4000", Some(0));
    r.place_on_field(1, "OPP-4000", Some(0));

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let picks = non_pass(&r);
    r.execute_action(0, picks[0]).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 1);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none(), "OPT lockout");
    assert_eq!(r.battle_area_size(1), 1, "no second delete this turn");

    // Lockout clears on player 0's next turn.
    r.end_turn();
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 0, "back on player 0's turn");
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let picks = non_pass(&r);
    assert_eq!(picks.len(), 1, "OPT lock cleared — delete offered again");
    r.execute_action(0, picks[0]).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0, "second 4000 DP Digimon deleted next turn");
}

// ─── Integrated — real play / real attack ───────────────────────────────────

#[test]
fn bt12_011_on_play_from_hand_plays_tamer_free() {
    let mut r = base(&[CARD_ID, "TAIKI"]);
    let mem = r.memory();
    let _ = r.play(0, 0);

    // Only the On Play hand pick should surface (no DigiXros material is
    // available, so the play is at full cost 5).
    let view = r.pending_selection_view().expect("On Play hand pick");
    assert!(view.is_optional);
    let picks = non_pass(&r);
    assert_eq!(picks.len(), 1, "Taiki Kudo offered");
    r.execute_action(0, picks[0]).unwrap();
    let _ = r.auto_resolve();

    assert!(on_field(&r, 0, CARD_ID), "Shoutmon (King Version) played");
    assert!(on_field(&r, 0, "TAIKI"), "Taiki Kudo played by the effect");
    assert_eq!(mem - r.memory(), 5, "only Shoutmon's cost 5 is paid; Taiki is free");
}

#[test]
fn bt12_011_attack_with_save_carrier_deletes_4000() {
    let mut r = builder().security(1, &["FILLER"; 3]).start();
    r.skip_mulligan();
    r.set_first_player(0);
    let carrier = r.place_stack(0, &[CARD_ID, "SAVE-DIGI"]);
    r.place_on_field(1, "OPP-4000", Some(0));
    r.place_on_field(1, "OPP-5000", Some(0));

    r.attack_player(carrier, 1, false);
    let picks = non_pass(&r);
    assert_eq!(picks.len(), 1, "only the 4000 DP Digimon is a legal target");
    assert!(!r.pending_is_optional(), "the delete is mandatory");
    r.execute_action(0, picks[0]).unwrap();
    let _ = r.auto_resolve();

    assert!(!on_field(&r, 1, "OPP-4000"), "4000 DP Digimon deleted on a real attack");
    assert!(on_field(&r, 1, "OPP-5000"));
}

// ─── Section 5 — Digivolution requirements ──────────────────────────────────

fn digivolve_runner(base_id: &str) -> (DebugRunner, PermanentHandle) {
    let mut r = builder().hand(0, &[CARD_ID]).memory(6).start();
    let b = r.place_on_field(0, base_id, Some(0));
    (r, b)
}

#[test]
fn bt12_011_digivolve_from_non_red_lv3_with_save_text_cost_2() {
    let (mut r, b) = digivolve_runner("SAVE-DIGI"); // yellow Lv.3 w/ <Save>
    let action = encode_digivolve(0, b.index as u16);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[action as usize], 1.0, "Lv.3 w/<Save> in text route is legal");
    let before = r.memory();
    r.game.decode_action(action, 0);
    assert_eq!(r.memory(), before - 2);
    assert_eq!(
        r.game.players[0].battle_area[b.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        CARD_ID
    );
}

#[test]
fn bt12_011_cannot_digivolve_from_non_red_lv3_without_save_text() {
    let (r, b) = digivolve_runner("PLAIN-DIGI"); // yellow Lv.3, no <Save>
    let action = encode_digivolve(0, b.index as u16);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[action as usize], 0.0);
}

#[test]
fn bt12_011_digivolve_from_red_lv3_cost_2() {
    let (r, b) = digivolve_runner("RED-LV3");
    let action = encode_digivolve(0, b.index as u16);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[action as usize], 1.0);
}

// ─── Section 6 — DigiXros -2 ────────────────────────────────────────────────

#[test]
fn bt12_011_digixros_with_save_text_material_costs_3() {
    let mut r = builder().hand(0, &[CARD_ID]).memory(10).start();
    r.place_on_field(0, "SAVE-DIGI", Some(0));
    r.place_on_field(0, "PLAIN-DIGI", Some(0));
    let mem = r.memory();
    let _ = r.play(0, 0);

    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Material),
        "DigiXros material prompt"
    );
    let ids = non_pass(&r);
    assert_eq!(
        ids.len(),
        1,
        "only the <Save>-text Digimon is offered (the plain Digimon is not a material)"
    );
    r.execute_action(0, ids[0]).unwrap();
    if r.pending_selection().is_some() {
        let _ = r.execute_action(0, PASS);
    }
    let _ = r.auto_resolve();

    assert_eq!(mem - r.memory(), 3, "cost 5 − 2");
    let srcs = sources_of_field_card(&r, 0, CARD_ID);
    assert!(srcs.iter().any(|c| c == "SAVE-DIGI"), "material placed under");
    assert!(on_field(&r, 0, "PLAIN-DIGI"), "the plain Digimon stays on the field");
}
