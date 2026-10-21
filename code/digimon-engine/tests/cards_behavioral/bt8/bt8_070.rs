//! BT8-070 BlackWarGreymon — Digimon, Lv.6, Black/Red, DP 12000, Cost 12.
//! Traits: Dragonkin. Form: Mega. Attribute: Virus.
//! Digivolve: Black Lv.5 / cost 4; Red Lv.5 / cost 4.
//!
//! # Printed text (official Bandai DB — data/card_bundles/BT8-070.md)
//!
//! ```text
//! [When Digivolving] If this Digimon has a red digivolution card, choose any
//! number of your opponent's Digimon. If this Digimon has a black digivolution
//! card, choose any number of your opponent's Tamers. The chosen cards' play
//! costs must add up to 6 or less. Delete the chosen cards.
//! [All Turns][Once Per Turn] When an opponent's Digimon is deleted, you may
//! unsuspend this Digimon.
//! ```
//!
//! Official Q&A: Q1751 with red AND black digivolution cards it can delete
//! both Digimon and Tamers; Q1752 the combined play costs of the chosen
//! Digimon and Tamers must add up to 6 or less (one budget, not 6 each);
//! Q1753 a single red-and-black multicolour digivolution card enables both.
//! Rule 1-3-6: "choose any number" means at least 1 when any can be chosen.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT8/Black/BT8_070.cs
//!   - OnEnterFieldAnyone (WD): candidates = opponent battle-area permanents
//!     with HasPlayCost && cost <= 6 that are Digimon (needs a red source) or
//!     Tamers (needs a black source); SelectPermanentEffect canNoSelect:false,
//!     canEndNotMax:true, running sum <= 6, at least 1 → Mode.Destroy.
//!   - OnDestroyedAnyone: opponent battle-area Digimon deleted, maxCount 1,
//!     optional; CanActivate = CanUnsuspend(this) → unsuspend this.
//!
//! # Patterns
//! - play-cost budget multi-select (`select_opponent_play_cost_budget`) with
//!   a per-kind candidate filter gated on this Digimon's source colours
//!   (`self_source_count`)
//! - [All Turns][OPT] `on_any_deletion` opponent-Digimon observer → optional
//!   unsuspend self (RB1-010 idiom)

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_attack, encode_digivolve, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT8-070";

fn card(id: &str, kind: CardKind, colors: &[CardColor], level: Option<u8>, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = kind;
    c.colors = colors.to_vec();
    c.level = level;
    c.dp = level.map(|l| 1000 * l as i32);
    c.play_cost = cost;
    c
}

fn setup() -> DebugRunner {
    use CardColor::*;
    use CardKind::*;
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT8-070 in the embedded DSL pack")
        .add_card(card("FILL", Digimon, &[Green], Some(3), 3))
        .add_card(card("RED5", Digimon, &[Red], Some(5), 7))
        .add_card(card("BLACK5", Digimon, &[Black], Some(5), 7))
        .add_card(card("YEL5", Digimon, &[Yellow], Some(5), 7))
        .add_card(card("REDSRC", Digimon, &[Red], Some(4), 5))
        .add_card(card("BLACKSRC", Digimon, &[Black], Some(4), 5))
        .add_card(card("RBSRC", Digimon, &[Red, Black], Some(4), 5))
        .add_card(card("YELSRC", Digimon, &[Yellow], Some(4), 5))
        .add_card(card("OPP-D2", Digimon, &[Blue], Some(3), 2))
        .add_card(card("OPP-D3", Digimon, &[Blue], Some(3), 3))
        .add_card(card("OPP-D4", Digimon, &[Blue], Some(4), 4))
        .add_card(card("OPP-D7", Digimon, &[Blue], Some(5), 7))
        .add_card(card("OPP-T2", Tamer, &[Blue], None, 2))
        .add_card(card("OPP-T3", Tamer, &[Blue], None, 3))
        .add_card(card("OPP-TOKEN", Token, &[Blue], Some(1), 0))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();
    r.set_first_player(0);
    // Printed circles (Black Lv.5 / 4, Red Lv.5 / 4) — production reads them
    // from cards.json evo_costs; the test card store has none.
    let idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == CARD_ID)
        .unwrap();
    r.game.card_data[idx].evo_costs = vec![
        EvoCost { card_color: CardColor::Black as u8, level: 5, memory_cost: 4 },
        EvoCost { card_color: CardColor::Red as u8, level: 5, memory_cost: 4 },
    ];
    r
}

fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn opp(r: &DebugRunner, id: &str) -> u16 {
    let idx = field_ids(r, 1)
        .iter()
        .position(|x| x == id)
        .unwrap_or_else(|| panic!("{id} not on the opponent's field"));
    encode_attack(0, idx as u16)
}

fn offered(r: &DebugRunner, id: &str) -> bool {
    let a = opp(r, id);
    r.pending_selection_view()
        .is_some_and(|v| v.valid_action_ids.contains(&a))
}

fn pick(r: &mut DebugRunner, id: &str) {
    let a = opp(r, id);
    assert!(offered(r, id), "{id} must be a legal pick");
    r.execute_action(0, a).unwrap();
}

/// BlackWarGreymon on top of `sources` (bottom first), its [When Digivolving]
/// fired.
fn wd_with_sources(r: &mut DebugRunner, sources: &[&str]) -> PermanentHandle {
    let mut ids = sources.to_vec();
    ids.push(CARD_ID);
    let h = r.place_stack(0, &ids);
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    h
}

fn suspend(r: &mut DebugRunner, h: PermanentHandle) {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = true;
}

fn is_suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn opp_handle(r: &DebugRunner, id: &str) -> PermanentHandle {
    let idx = field_ids(r, 1).iter().position(|x| x == id).unwrap();
    PermanentHandle { player: 1, index: idx as u8 }
}

// ─── Section 1: structural ──────────────────────────────────────────────────

#[test]
fn bt8_070_structure() {
    let r = setup();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.level, Some(6));
    assert_eq!(c.dp, Some(12000));
    let trig: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(trig.len(), 2);
    let wd = trig
        .iter()
        .find(|t| t.when == vec![CompiledTiming::WhenDigivolving])
        .expect("[When Digivolving]");
    assert!(!wd.optional, "mandatory");
    let del = trig
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnAnyDeletion])
        .expect("[All Turns] deletion observer");
    assert!(del.optional, "'you may unsuspend'");
    assert!(del.once_per_turn);
    assert!(del.active_when.is_some(), "[All Turns]");
}

// ─── Section 3: [When Digivolving] ──────────────────────────────────────────

#[test]
fn bt8_070_digivolves_from_red_lv5_and_deletes_opponent_digimon() {
    let mut r = setup();
    r.game.enter_main_phase();
    let base = r.place_stack(0, &["BLACKSRC", "RED5"]);
    r.place_on_field(1, "OPP-D3", Some(0));
    r.place_on_field(1, "OPP-T2", Some(0));
    let slot = r.game.players[0].hand.len();
    let data = r.game.card_data.iter().position(|c| c.card_id == CARD_ID).unwrap();
    let next = r.game.next_card_index();
    r.game.players[0]
        .hand
        .push(digimon_engine::card_source::CardSource::new(data, 0, next));
    let action = encode_digivolve(slot as u16, base.index as u16);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0, "Red Lv.5 circle");
    let mem = r.memory();
    r.game.decode_action(action, 0);
    assert_eq!(r.memory(), mem - 4, "digivolve cost 4");
    // Red (RED5) and black (BLACKSRC) sources → Digimon and Tamers.
    assert!(offered(&r, "OPP-D3"));
    assert!(offered(&r, "OPP-T2"));
    pick(&mut r, "OPP-D3");
    pick(&mut r, "OPP-T2");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).is_empty(), "both chosen cards deleted");
}

#[test]
fn bt8_070_red_source_only_offers_digimon() {
    let mut r = setup();
    r.place_on_field(1, "OPP-D3", Some(0));
    r.place_on_field(1, "OPP-T2", Some(0));
    wd_with_sources(&mut r, &["REDSRC"]);
    assert!(offered(&r, "OPP-D3"));
    assert!(!offered(&r, "OPP-T2"), "no black source → Tamers can't be chosen");
    pick(&mut r, "OPP-D3");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP-T2".to_string()]);
}

#[test]
fn bt8_070_black_source_only_offers_tamers() {
    let mut r = setup();
    r.place_on_field(1, "OPP-D3", Some(0));
    r.place_on_field(1, "OPP-T2", Some(0));
    wd_with_sources(&mut r, &["BLACKSRC"]);
    assert!(offered(&r, "OPP-T2"));
    assert!(!offered(&r, "OPP-D3"), "no red source → Digimon can't be chosen");
    pick(&mut r, "OPP-T2");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP-D3".to_string()]);
}

#[test]
fn bt8_070_no_red_or_black_source_does_nothing() {
    let mut r = setup();
    r.place_on_field(1, "OPP-D3", Some(0));
    r.place_on_field(1, "OPP-T2", Some(0));
    wd_with_sources(&mut r, &["YELSRC"]);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(field_ids(&r, 1).len(), 2);
}

#[test]
fn bt8_070_one_red_black_multicolour_source_enables_both() {
    // Q1753.
    let mut r = setup();
    r.place_on_field(1, "OPP-D2", Some(0));
    r.place_on_field(1, "OPP-T2", Some(0));
    wd_with_sources(&mut r, &["RBSRC"]);
    assert!(offered(&r, "OPP-D2"));
    assert!(offered(&r, "OPP-T2"));
}

#[test]
fn bt8_070_digimon_and_tamers_share_one_6_cost_budget() {
    // Q1751 + Q1752: both kinds, but the total is 6 or less.
    let mut r = setup();
    r.place_on_field(1, "OPP-D4", Some(0));
    r.place_on_field(1, "OPP-T2", Some(0));
    r.place_on_field(1, "OPP-T3", Some(0));
    wd_with_sources(&mut r, &["REDSRC", "BLACKSRC"]);
    pick(&mut r, "OPP-D4");
    assert!(offered(&r, "OPP-T2"), "4 + 2 = 6 fits");
    assert!(!offered(&r, "OPP-T3"), "4 + 3 = 7 exceeds the shared budget");
    pick(&mut r, "OPP-T2");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP-T3".to_string()]);
}

#[test]
fn bt8_070_a_card_costing_more_than_6_is_never_offered() {
    let mut r = setup();
    r.place_on_field(1, "OPP-D7", Some(0));
    r.place_on_field(1, "OPP-D2", Some(0));
    wd_with_sources(&mut r, &["REDSRC"]);
    assert!(!offered(&r, "OPP-D7"));
    assert!(offered(&r, "OPP-D2"));
}

#[test]
fn bt8_070_only_over_budget_targets_means_no_prompt() {
    let mut r = setup();
    r.place_on_field(1, "OPP-D7", Some(0));
    wd_with_sources(&mut r, &["REDSRC", "BLACKSRC"]);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(field_ids(&r, 1), vec!["OPP-D7".to_string()]);
}

#[test]
fn bt8_070_must_choose_at_least_one_then_may_stop() {
    // Rule 1-3-6: "any number" is 1 or more when a card can be chosen.
    let mut r = setup();
    r.place_on_field(1, "OPP-D2", Some(0));
    r.place_on_field(1, "OPP-D3", Some(0));
    wd_with_sources(&mut r, &["REDSRC"]);
    let v = r.pending_selection_view().unwrap();
    assert!(!v.valid_action_ids.contains(&PASS), "can't choose none");
    pick(&mut r, "OPP-D2");
    let v = r.pending_selection_view().expect("may add OPP-D3 (2 + 3 = 5)");
    assert!(v.valid_action_ids.contains(&PASS), "may stop after 1");
    r.execute_action(0, PASS).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP-D3".to_string()]);
}

#[test]
fn bt8_070_a_token_has_no_play_cost_and_is_not_offered() {
    // DCGO HasPlayCost: a token has no printed play cost.
    let mut r = setup();
    r.place_on_field(1, "OPP-TOKEN", Some(0));
    r.place_on_field(1, "OPP-D2", Some(0));
    wd_with_sources(&mut r, &["REDSRC"]);
    assert!(!offered(&r, "OPP-TOKEN"));
    assert!(offered(&r, "OPP-D2"));
}

/// Setup with an opposing DUAL card (Bearcatmon BT26-057, "Play cost: D") on
/// top of a permanent. Its cards.json `play_cost` (4) is restored so the test
/// discriminates: a raw-play-cost read would offer it under the 6 budget.
fn setup_with_opposing_dual() -> DebugRunner {
    use CardColor::*;
    use CardKind::*;
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT8-070 in the embedded DSL pack")
        .dsl_card("BT26-057")
        .expect("BT26-057 in the embedded DSL pack")
        .add_card(card("FILL", Digimon, &[Green], Some(3), 3))
        .add_card(card("REDSRC", Digimon, &[Red], Some(4), 5))
        .add_card(card("BLACKSRC", Digimon, &[Black], Some(4), 5))
        .add_card(card("OPP-D2", Digimon, &[Blue], Some(3), 2))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();
    r.set_first_player(0);
    let dual = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "BT26-057")
        .unwrap();
    assert_eq!(r.game.card_data[dual].card_kind, Dual);
    r.game.card_data[dual].play_cost = 4;
    r
}

#[test]
fn bt8_070_a_dual_topped_digimon_has_no_play_cost_and_is_not_offered() {
    // DCGO CEntity_Base.HasPlayCost = !cardKind.Contains(Option) && PlayCost
    // >= 0 — a DUAL's kinds contain Option, so a DUAL-topped Digimon (§4-6-3)
    // is never a play-cost candidate; a normal Digimon still is.
    let mut r = setup_with_opposing_dual();
    r.place_on_field(1, "BT26-057", Some(0));
    r.place_on_field(1, "OPP-D2", Some(0));
    wd_with_sources(&mut r, &["REDSRC", "BLACKSRC"]);
    assert!(!offered(&r, "BT26-057"), "a DUAL top has no play cost");
    assert!(offered(&r, "OPP-D2"));
    pick(&mut r, "OPP-D2");
    // 4 of the budget remains, and the DUAL is still not offered on the
    // re-derived (resume) pick: the step ends with OPP-D2 as the only pick.
    assert!(!offered(&r, "BT26-057"), "still no play cost after the first pick");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["BT26-057".to_string()]);
}

#[test]
fn bt8_070_only_a_dual_topped_digimon_means_no_prompt() {
    let mut r = setup_with_opposing_dual();
    r.place_on_field(1, "BT26-057", Some(0));
    wd_with_sources(&mut r, &["REDSRC", "BLACKSRC"]);
    assert!(r.game.pending_selection.is_none(), "no candidate → nothing to choose");
    assert_eq!(field_ids(&r, 1), vec!["BT26-057".to_string()]);
}

// ─── Section 3: [All Turns][Once Per Turn] unsuspend ────────────────────────

#[test]
fn bt8_070_unsuspends_when_an_opponent_digimon_is_deleted() {
    let mut r = setup();
    let b = r.place_on_field(0, CARD_ID, Some(0));
    suspend(&mut r, b);
    let o = r.place_on_field(1, "OPP-D3", Some(0));
    r.game.delete_permanent_with_effects(o);
    assert!(r.pending_is_optional(), "'you may'");
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert!(!is_suspended(&r, b));
}

#[test]
fn bt8_070_unsuspend_can_be_declined() {
    let mut r = setup();
    let b = r.place_on_field(0, CARD_ID, Some(0));
    suspend(&mut r, b);
    let o = r.place_on_field(1, "OPP-D3", Some(0));
    r.game.delete_permanent_with_effects(o);
    r.decline_optional_trigger().unwrap();
    assert!(is_suspended(&r, b));
}

#[test]
fn bt8_070_own_digimon_or_opponent_tamer_deleted_does_not_trigger() {
    let mut r = setup();
    let b = r.place_on_field(0, CARD_ID, Some(0));
    suspend(&mut r, b);
    let own = r.place_on_field(0, "FILL", Some(0));
    r.game.delete_permanent_with_effects(own);
    assert!(r.game.pending_selection.is_none(), "own Digimon");
    let t = r.place_on_field(1, "OPP-T2", Some(0));
    r.game.delete_permanent_with_effects(t);
    assert!(r.game.pending_selection.is_none(), "a Tamer is not a Digimon");
    assert!(is_suspended(&r, b));
}

#[test]
fn bt8_070_unsuspend_triggers_in_the_opponents_turn_too() {
    let mut r = setup();
    let b = r.place_on_field(0, CARD_ID, Some(0));
    r.pass_turn();
    let _ = r.auto_resolve();
    suspend(&mut r, b);
    let o = r.place_on_field(1, "OPP-D3", Some(0));
    r.game.delete_permanent_with_effects(o);
    assert!(r.pending_is_optional(), "[All Turns]");
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert!(!is_suspended(&r, b));
}

#[test]
fn bt8_070_unsuspend_is_once_per_turn_and_resets() {
    let mut r = setup();
    let b = r.place_on_field(0, CARD_ID, Some(0));
    suspend(&mut r, b);
    r.place_on_field(1, "OPP-D2", Some(0));
    r.place_on_field(1, "OPP-D3", Some(0));
    r.place_on_field(1, "OPP-D4", Some(0));
    let o = opp_handle(&r, "OPP-D2");
    r.game.delete_permanent_with_effects(o);
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert!(!is_suspended(&r, b));
    suspend(&mut r, b);
    let o = opp_handle(&r, "OPP-D3");
    r.game.delete_permanent_with_effects(o);
    assert!(r.game.pending_selection.is_none(), "[Once Per Turn] spent");
    assert!(is_suspended(&r, b));
    r.pass_turn();
    let _ = r.auto_resolve();
    suspend(&mut r, b);
    let o = opp_handle(&r, "OPP-D4");
    r.game.delete_permanent_with_effects(o);
    assert!(r.pending_is_optional(), "a new turn resets it");
}

#[test]
fn bt8_070_unsuspended_blackwargreymon_is_not_offered_and_keeps_its_opt() {
    // DCGO CanActivate: CanUnsuspend(this).
    let mut r = setup();
    let b = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP-D2", Some(0));
    r.place_on_field(1, "OPP-D3", Some(0));
    let o = opp_handle(&r, "OPP-D2");
    r.game.delete_permanent_with_effects(o);
    assert!(r.game.pending_selection.is_none());
    suspend(&mut r, b);
    let o = opp_handle(&r, "OPP-D3");
    r.game.delete_permanent_with_effects(o);
    assert!(r.pending_is_optional(), "OPT still available");
}

#[test]
fn bt8_070_its_own_wd_deletion_lets_it_unsuspend() {
    let mut r = setup();
    r.place_on_field(1, "OPP-D3", Some(0));
    r.place_on_field(1, "OPP-D2", Some(0));
    let mut ids = vec!["REDSRC"];
    ids.push(CARD_ID);
    let b = r.place_stack(0, &ids);
    suspend(&mut r, b);
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(b));
    r.game.drain_effect_queue();
    pick(&mut r, "OPP-D3");
    pick(&mut r, "OPP-D2");
    assert!(field_ids(&r, 1).is_empty());
    assert!(
        r.pending_is_optional(),
        "two Digimon deleted at once → one optional unsuspend (§15-5-2)"
    );
    r.accept_optional_trigger().unwrap();
    assert!(r.game.pending_selection.is_none(), "triggered once");
    assert!(!is_suspended(&r, b));
}
