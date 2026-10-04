//! BT21-082 Takuya Kanbara — Tamer, Red, cost 3, Hero.
//!
//! Official text (data/card_bundles/BT21-082.md):
//!   [Security] Play this card without paying the cost.
//!   [Start of Your Main Phase] 1 of your Digimon or Tamers may digivolve into
//!   a Digimon card with the [Hybrid] or [Hero] trait in the hand. For each of
//!   your red Tamers with different names, reduce this effect's digivolution
//!   cost by 1.
//!   Inherited: [Your Turn] [Once Per Turn] When your opponent's security
//!   stack is removed from, you may play 1 red Tamer card from your hand
//!   without paying the cost.
//!
//! DCGO: BT21/Red/BT21_082.cs.
//!
//! Pattern tags: tamer-security-play-self, start-of-main-optional-digivolve,
//! digimon-or-tamer-digivolve-base, cost-reduce-distinct-names-count,
//! inherited-on-opponent-security-removed-opt, play-from-hand-free.
#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{self, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT21-082";

/// Red [Hybrid] Lv.4 that can digivolve onto a red Tamer for 4.
const HYBRID_EVO_YAML: &str = r#"
card: T-HYB-EVO
name: "Hybrid Evo"
kind: digimon
level: 4
color: [red]
cost: 6
dp: 6000
traits: [Hybrid]
alt_paths:
  - kind: digivolve
    from: { kind: tamer, color_is: red }
    cost: 4
effects: []
"#;

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(filler("FILLER"))
        .add_card(digimon("BASE3", "Base", CardColor::Red, 3, 3, &[], None))
        .add_card(digimon("HERO4", "Hero Four", CardColor::Red, 4, 5, &["Hero"], Some(4)))
        .add_card(digimon("PLAIN4", "Plain Four", CardColor::Red, 4, 5, &["Beast"], Some(4)))
        .add_card(digimon("ATK", "Attacker", CardColor::Blue, 3, 3, &[], None))
        .add_card(tamer("RED-T2", "Other Red Tamer", CardColor::Red))
        .add_card(tamer("BLUE-T", "Blue Tamer", CardColor::Blue))
        .from_dsl_yaml(HYBRID_EVO_YAML)
        .expect("hybrid fixture")
        .dsl_card(CARD_ID)
        .expect("BT21-082")
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        // Non-Digimon security so the security check is battle-free and the
        // attacking carrier survives.
        .security(1, &["BLUE-T"; 4])
        .memory(10)
        .start();
    r.set_first_player(0);
    r.game.enter_main_phase();
    r
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, h: PermanentHandle) {
    r.game.enqueue_triggered(timing, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

// ─── Structure ──────────────────────────────────────────────────────────────

#[test]
fn bt21_082_structure() {
    let r = setup();
    let c = r.compiled_card(CARD_ID).unwrap();
    assert_eq!(c.cost, Some(3));
    assert_eq!(c.traits, vec!["Hero".to_string()]);
    let triggered: Vec<_> = c
        .effects
        .iter()
        .filter_map(|cl| match cl {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 3);
    assert!(triggered[0].when.contains(&CompiledTiming::OnSecurity));
    assert!(triggered[1].when.contains(&CompiledTiming::StartOfYourMainPhase));
    assert!(!triggered[1].optional, "choice lives in the optional pick");
    let inh = triggered[2];
    assert_eq!(inh.scope, CompiledScope::Inherited);
    assert!(inh.when.contains(&CompiledTiming::OnOpponentSecurityRemoved));
    assert!(inh.optional, "'you may play'");
    assert!(inh.once_per_turn, "[Once Per Turn]");
}

// ─── [Start of Your Main Phase] ─────────────────────────────────────────────

#[test]
fn bt21_082_somp_digimon_digivolves_with_one_red_tamer_reduction() {
    let mut r = setup();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    let base = r.place_on_field(0, "BASE3", Some(0));
    push_hand(&mut r, 0, "HERO4");
    fire(&mut r, EffectTiming::StartOfYourMainPhase, t);
    let v = r.pending_selection_view().expect("optional base pick");
    assert!(v.is_optional, "'may digivolve'");
    pick_own_field(&mut r, 0, base);
    pick_hand(&mut r, 0, "HERO4");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, base), "HERO4");
    assert_eq!(r.memory(), 10 - 3, "cost 4 − 1 (Takuya is a red Tamer)");
}

#[test]
fn bt21_082_somp_reduction_counts_red_tamers_with_different_names() {
    let mut r = setup();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, CARD_ID, Some(0)); // same name — no extra reduction
    r.place_on_field(0, "RED-T2", Some(0)); // different name — −1
    r.place_on_field(0, "BLUE-T", Some(0)); // not red — no reduction
    let base = r.place_on_field(0, "BASE3", Some(0));
    push_hand(&mut r, 0, "HERO4");
    fire(&mut r, EffectTiming::StartOfYourMainPhase, t);
    pick_own_field(&mut r, 0, base);
    pick_hand(&mut r, 0, "HERO4");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, base), "HERO4");
    assert_eq!(r.memory(), 10 - 2, "cost 4 − 2 distinct red Tamer names");
}

#[test]
fn bt21_082_somp_tamer_digivolves_into_hybrid() {
    let mut r = setup();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "T-HYB-EVO");
    fire(&mut r, EffectTiming::StartOfYourMainPhase, t);
    pick_own_field(&mut r, 0, t);
    pick_hand(&mut r, 0, "T-HYB-EVO");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, t), "T-HYB-EVO", "a Tamer is a legal base");
    assert_eq!(r.memory(), 10 - 3, "Tamer route cost 4 − 1");
}

#[test]
fn bt21_082_somp_only_hybrid_or_hero_cards() {
    let mut r = setup();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "BASE3", Some(0));
    push_hand(&mut r, 0, "PLAIN4");
    fire(&mut r, EffectTiming::StartOfYourMainPhase, t);
    let _ = r.auto_resolve();
    assert!(
        r.pending_selection_view().is_none(),
        "no [Hybrid]/[Hero] candidate → no prompt"
    );
}

#[test]
fn bt21_082_somp_plain_card_not_offered_alongside_hero() {
    let mut r = setup();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    let base = r.place_on_field(0, "BASE3", Some(0));
    push_hand(&mut r, 0, "PLAIN4");
    push_hand(&mut r, 0, "HERO4");
    fire(&mut r, EffectTiming::StartOfYourMainPhase, t);
    pick_own_field(&mut r, 0, base);
    assert!(hand_pickable(&r, 0, "HERO4"));
    assert!(!hand_pickable(&r, 0, "PLAIN4"));
}

#[test]
fn bt21_082_somp_decline_does_nothing() {
    let mut r = setup();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    let base = r.place_on_field(0, "BASE3", Some(0));
    push_hand(&mut r, 0, "HERO4");
    fire(&mut r, EffectTiming::StartOfYourMainPhase, t);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, base), "BASE3");
    assert_eq!(r.memory(), 10);
    assert!(hand_ids(&r, 0).contains(&"HERO4".to_string()));
}

#[test]
fn bt21_082_somp_fires_through_real_turn_flow() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "BASE3", Some(0));
    push_hand(&mut r, 0, "HERO4");
    r.end_turn();
    r.game.enter_main_phase();
    assert!(
        r.pending_selection_view().is_none(),
        "[Start of YOUR Main Phase] — not on the opponent's"
    );
    r.end_turn();
    assert_eq!(r.game.turn_player(), 0);
    r.game.enter_main_phase();
    let v = r.pending_selection_view().expect("Takuya's start-of-main prompt");
    assert!(v.is_optional);
}

// ─── [Security] ─────────────────────────────────────────────────────────────

#[test]
fn bt21_082_security_plays_self() {
    let mut r = DebugRunner::builder()
        .add_card(filler("FILLER"))
        .add_card(digimon("ATK", "Attacker", CardColor::Blue, 3, 3, &[], None))
        .dsl_card(CARD_ID)
        .expect("BT21-082")
        .deck(0, &["FILLER"; 4])
        .deck(1, &["FILLER"; 4])
        .security(1, &[CARD_ID])
        .memory(3)
        .start();
    r.set_first_player(0);
    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&CARD_ID.to_string()));
}

// ─── Inherited ──────────────────────────────────────────────────────────────

fn carrier(r: &mut DebugRunner) -> PermanentHandle {
    r.place_stack(0, &[CARD_ID, "ATK"])
}

#[test]
fn bt21_082_inherited_plays_red_tamer_on_security_removal() {
    let mut r = setup();
    let c = carrier(&mut r);
    push_hand(&mut r, 0, "RED-T2");
    push_hand(&mut r, 0, "BLUE-T");
    let mem = r.memory();
    r.attack_player(c, 1, false);
    // The optional red-Tamer hand pick IS the "you may" prompt.
    let v = r.pending_selection_view().expect("red Tamer pick");
    assert!(v.is_optional);
    assert!(hand_pickable(&r, 0, "RED-T2"));
    assert!(!hand_pickable(&r, 0, "BLUE-T"), "red Tamers only");
    pick_hand(&mut r, 0, "RED-T2");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"RED-T2".to_string()));
    assert_eq!(r.memory(), mem, "without paying the cost");
}

#[test]
fn bt21_082_inherited_decline_trigger() {
    let mut r = setup();
    let c = carrier(&mut r);
    push_hand(&mut r, 0, "RED-T2");
    r.attack_player(c, 1, false);
    r.decline_optional_trigger().expect("optional trigger");
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"RED-T2".to_string()));
    assert!(hand_ids(&r, 0).contains(&"RED-T2".to_string()));
}

#[test]
fn bt21_082_inherited_once_per_turn() {
    let mut r = setup();
    let c = carrier(&mut r);
    push_hand(&mut r, 0, "RED-T2");
    push_hand(&mut r, 0, "RED-T2");
    r.attack_player(c, 1, false);
    pick_hand(&mut r, 0, "RED-T2");
    let _ = r.auto_resolve();
    // Ready the carrier and attack again the same turn.
    r.game.players[0].battle_area[c.index as usize].is_suspended = false;
    r.attack_player(c, 1, false);
    let _ = r.auto_resolve();
    assert!(
        r.pending_selection_view().is_none(),
        "[Once Per Turn] — no second trigger"
    );
    assert_eq!(
        field_ids(&r, 0).iter().filter(|id| *id == "RED-T2").count(),
        1
    );
}

#[test]
fn bt21_082_inherited_ignores_own_security_removal_on_opponents_turn() {
    let mut r = setup();
    // On the opponent's turn, OUR security being removed (they attack us)
    // is neither [Your Turn] nor "your opponent's security stack".
    let _c = carrier(&mut r);
    push_hand(&mut r, 0, "RED-T2");
    r.end_turn();
    assert_eq!(r.game.turn_player(), 1);
    r.game.enter_main_phase();
    let idx = data_idx(&r, "BLUE-T");
    let next = r.game.next_card_index();
    r.game.players[0].security.push(CardSource::new(idx, 0, next));
    let atk = r.place_on_field(1, "ATK", Some(0));
    r.attack_player(atk, 0, false);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert!(hand_ids(&r, 0).contains(&"RED-T2".to_string()));
}

#[test]
fn bt21_082_inherited_inactive_while_takuya_is_top_card() {
    let mut r = setup();
    // Takuya itself on the field (not as a digivolution card) — inherited
    // text is not active; the attacker is a plain Digimon.
    r.place_on_field(0, CARD_ID, Some(0));
    let atk = r.place_on_field(0, "ATK", Some(0));
    push_hand(&mut r, 0, "RED-T2");
    r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert!(hand_ids(&r, 0).contains(&"RED-T2".to_string()));
}

// ─── Local fixtures ─────────────────────────────────────────────────────────

fn digimon(
    id: &str,
    name: &str,
    color: CardColor,
    level: u8,
    cost: u16,
    traits: &[&str],
    evo_from_lv3: Option<u8>,
) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = cost;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    if let Some(memory_cost) = evo_from_lv3 {
        c.evo_costs = vec![EvoCost {
            card_color: color as u8,
            level: 3,
            memory_cost: memory_cost.into(),
        }];
    }
    c
}

fn tamer(id: &str, name: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![color];
    c
}

fn filler(id: &str) -> CardData {
    digimon(id, id, CardColor::Red, 3, 3, &[], None)
}

fn data_idx(r: &DebugRunner, card_id: &str) -> usize {
    r.game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"))
}

fn push_hand(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = data_idx(r, card_id);
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .hand
        .push(CardSource::new(idx, p, next));
}

fn ids(r: &DebugRunner, cards: &[CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].hand)
}

fn pass(r: &mut DebugRunner, p: u8) {
    r.execute_action(p, PASS).expect("PASS legal");
}

fn non_pass(r: &DebugRunner) -> Vec<u16> {
    r.pending_selection_view()
        .map(|v| {
            v.valid_action_ids
                .into_iter()
                .filter(|&a| a != PASS)
                .collect()
        })
        .unwrap_or_default()
}

fn hand_pickable(r: &DebugRunner, p: u8, card_id: &str) -> bool {
    let Some(idx) = r.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
    else {
        return false;
    };
    non_pass(r).contains(&(space::PLAY_HAND_START + idx as u16))
}

fn pick_hand(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = r.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in hand"));
    let action = space::PLAY_HAND_START + idx as u16;
    let v = r.pending_selection_view().expect("hand selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{card_id} not a legal hand pick (valid {:?})",
        v.valid_action_ids
    );
    r.execute_action(p, action).unwrap();
}

fn pick_own_field(r: &mut DebugRunner, selector: u8, h: PermanentHandle) {
    let action = space::encode_attack(0, h.index as u16);
    let v = r.pending_selection_view().expect("field selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{h:?} not a legal pick (valid {:?})",
        v.valid_action_ids
    );
    r.execute_action(selector, action).unwrap();
}
