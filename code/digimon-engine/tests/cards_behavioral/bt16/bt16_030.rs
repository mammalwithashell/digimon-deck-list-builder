//! BT16-030 Salamon — Digimon, Lv.3, Yellow/Purple, DP 1000, Cost 3.
//! Traits: Mammal. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB / data/card_bundles/BT16-030.md)
//!
//! Digivolve: Purple Lv.2 / Cost 1, Red Lv.2 / Cost 1
//! [Digivolve] [Nyaromon]: Cost 0
//!
//! [Start of Your Main Phase] [On Play] If it's your turn, 1 of your Digimon
//! may digivolve into a level 4 Digimon card with the [Holy Beast] or [Free]
//! trait in the trash with the digivolution cost reduced by 1.
//!
//! Inherited Effect: [Your Turn] All of your opponent's Security Digimon get
//! -3000 DP.
//!
//! Official Q&A: the effect does not ignore digivolution requirements.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT16/Yellow/BT16_030.cs
//!
//! # Patterns this test covers
//! - B-family effect-initiated digivolve from TRASH onto a chosen own Digimon
//!   (`has_digivolve_candidate` + `can_digivolve_onto` + cost reduce)
//! - Two-timing clause (`[Start of Your Main Phase]` + `[On Play]`) with an
//!   "If it's your turn" gate
//! - `[Free]` matched as attribute-folded trait
//! - D4 declarative aura (inherited security DP debuff, PUPPETS-G008 idiom)
//! - Name-gated alt-path ([Nyaromon]: Cost 0)

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledCost, CompiledTiming};
use digimon_engine::action::space::{encode_attack, PASS, TRASH_EFFECT_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT16-030";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, color: CardColor, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = level as u16 + 1;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// Lv.4 card with one printed circle `from_color` Lv.3 / cost 3.
fn lv4(id: &str, from_color: CardColor, traits: &[&str]) -> CardData {
    let mut c = digimon(id, from_color, 4, traits);
    c.evo_costs = vec![EvoCost {
        card_color: from_color as u8,
        level: 3,
        memory_cost: 3,
    }];
    c
}

fn runner_with(hand: &[&str], memory: i16) -> DebugRunner {
    let mut lv5 = digimon("HOLY5", CardColor::Yellow, 5, &["Holy Beast"]);
    lv5.evo_costs = vec![EvoCost {
        card_color: CardColor::Yellow as u8,
        level: 3,
        memory_cost: 3,
    }];
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT16-030 YAML loads")
        .add_card(lv4("HOLY4", CardColor::Yellow, &["Holy Beast"]))
        // Attribute folded into traits (card_data.rs merges attribute_eng).
        .add_card(lv4("FREE4", CardColor::Yellow, &["Beastkin", "Free"]))
        .add_card(lv4("PLAIN4", CardColor::Yellow, &["Beastkin"]))
        .add_card(lv4("REDHOLY4", CardColor::Red, &["Holy Beast"]))
        .add_card(lv5)
        .add_card(digimon("RED3", CardColor::Red, 3, &[]))
        .add_card({
            let mut c = digimon("NYAROMON", CardColor::Yellow, 2, &[]);
            c.card_name = "Nyaromon".to_string();
            c
        })
        .add_card(make_test_card("FILL", "Fill"))
        .hand(0, hand)
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(memory)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.game.current_phase = GamePhase::Main;
    r
}

fn push_trash(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .trash
        .push(CardSource::new(idx, p, next));
}

fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

fn find(r: &DebugRunner, p: u8, id: &str) -> PermanentHandle {
    let index = r.game.players[p as usize]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} on field"));
    PermanentHandle {
        player: p,
        index: index as u8,
    }
}

fn pick_own(r: &mut DebugRunner, h: PermanentHandle) {
    let v = r.pending_selection_view().expect("own-Digimon pick");
    assert_eq!(v.kind, SelectionKind::OwnField, "{v:?}");
    assert!(v.is_optional, "\"may digivolve\" — optional");
    let action = encode_attack(0, h.index as u16);
    assert!(v.valid_action_ids.contains(&action), "{v:?}");
    r.execute_action(0, action).unwrap();
}

fn trash_action(r: &DebugRunner, id: &str) -> u16 {
    let idx = r.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} in trash"));
    TRASH_EFFECT_START + idx as u16
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

fn fire_start_of_main(r: &mut DebugRunner, h: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

// ─── Structural ──────────────────────────────────────────────────────────────

#[test]
fn bt16_030_clause_shapes() {
    let r = runner_with(&[], 5);
    let card = r.compiled_card(CARD_ID).expect("compiles");
    assert_eq!(card.name, "Salamon");
    assert_eq!(card.level, Some(3));
    let trig = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause");
    assert!(trig.when.contains(&CompiledTiming::StartOfYourMainPhase));
    assert!(trig.when.contains(&CompiledTiming::OnPlay));
    assert!(!trig.once_per_turn, "not [Once Per Turn]");
    assert!(
        card.effects
            .iter()
            .any(|c| matches!(c, CompiledClause::Declarative(_))),
        "inherited security-DP aura"
    );
}

#[test]
fn bt16_030_nyaromon_alt_path_costs_zero() {
    let r = runner_with(&[], 5);
    let card = r.compiled_card(CARD_ID).expect("compiles");
    let nyaro = card.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.from
                .as_ref()
                .is_some_and(|f| f.name_is.as_deref() == Some("Nyaromon"))
    });
    assert_eq!(nyaro.expect("[Nyaromon] path").cost, Some(CompiledCost::Literal(0)));
}

#[test]
fn bt16_030_digivolves_from_nyaromon_for_zero() {
    let mut r = runner_with(&[CARD_ID], 5);
    let ny = r.place_on_field(0, "NYAROMON", Some(0));
    r.game
        .decode_action(digimon_engine::action::space::encode_digivolve(0, ny.index as u16), 0);
    assert_eq!(top_id(&r, ny), CARD_ID);
    assert_eq!(r.memory(), 5, "[Digivolve] [Nyaromon]: Cost 0");
}

// ─── [On Play] ───────────────────────────────────────────────────────────────

#[test]
fn bt16_030_on_play_digivolves_salamon_into_holy_beast_from_trash_cost_minus_1() {
    let mut r = runner_with(&[CARD_ID], 10);
    push_trash(&mut r, 0, "HOLY4");
    push_trash(&mut r, 0, "PLAIN4");
    push_trash(&mut r, 0, "HOLY5");
    r.play(0, 0).expect("Salamon plays");
    assert_eq!(r.memory(), 7);
    let sal = find(&r, 0, CARD_ID);
    pick_own(&mut r, sal);
    let v = r.pending_selection_view().expect("trash pick");
    assert!(v.is_optional);
    assert_eq!(
        non_pass(&r),
        vec![trash_action(&r, "HOLY4")],
        "only the Lv.4 [Holy Beast] card (not trait-less Lv.4, not Lv.5)"
    );
    let a = trash_action(&r, "HOLY4");
    r.execute_action(0, a).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, sal), "HOLY4");
    assert_eq!(r.memory(), 7 - (3 - 1), "digivolution cost reduced by 1");
    assert!(!r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "HOLY4"));
}

#[test]
fn bt16_030_on_play_free_attribute_card_qualifies() {
    let mut r = runner_with(&[CARD_ID], 10);
    push_trash(&mut r, 0, "FREE4");
    r.play(0, 0).expect("plays");
    let sal = find(&r, 0, CARD_ID);
    pick_own(&mut r, sal);
    let a = trash_action(&r, "FREE4");
    assert_eq!(non_pass(&r), vec![a]);
    r.execute_action(0, a).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, sal), "FREE4");
}

#[test]
fn bt16_030_on_play_can_digivolve_another_digimon() {
    let mut r = runner_with(&[CARD_ID], 10);
    let red = r.place_on_field(0, "RED3", Some(0));
    push_trash(&mut r, 0, "REDHOLY4");
    r.play(0, 0).expect("plays");
    let sal = find(&r, 0, CARD_ID);
    // Only RED3 can take the red-circle card; Salamon (yellow/purple) can't.
    let v = r.pending_selection_view().expect("pick");
    assert!(v.valid_action_ids.contains(&encode_attack(0, red.index as u16)));
    assert!(!v.valid_action_ids.contains(&encode_attack(0, sal.index as u16)));
    pick_own(&mut r, red);
    let a = trash_action(&r, "REDHOLY4");
    r.execute_action(0, a).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, red), "REDHOLY4");
    assert_eq!(top_id(&r, sal), CARD_ID);
}

#[test]
fn bt16_030_on_play_respects_digivolution_requirements() {
    // Official Q&A: requirements still apply. A red-circle card can't go on
    // a yellow/purple Salamon, so with no other Digimon nothing is offered.
    let mut r = runner_with(&[CARD_ID], 10);
    push_trash(&mut r, 0, "REDHOLY4");
    r.play(0, 0).expect("plays");
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(top_id(&r, find(&r, 0, CARD_ID)), CARD_ID);
}

#[test]
fn bt16_030_on_play_decline_does_nothing() {
    let mut r = runner_with(&[CARD_ID], 10);
    push_trash(&mut r, 0, "HOLY4");
    r.play(0, 0).expect("plays");
    let v = r.pending_selection_view().expect("optional pick");
    assert!(v.is_optional);
    r.execute_action(0, PASS).unwrap();
    let _ = r.auto_resolve();
    let sal = find(&r, 0, CARD_ID);
    assert_eq!(top_id(&r, sal), CARD_ID);
    assert_eq!(r.memory(), 7);
}

#[test]
fn bt16_030_on_play_without_trash_candidate_is_noop() {
    let mut r = runner_with(&[CARD_ID], 10);
    push_trash(&mut r, 0, "PLAIN4");
    r.play(0, 0).expect("plays");
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
}

#[test]
fn bt16_030_on_play_not_your_turn_does_not_fire() {
    // "If it's your turn": an [On Play] resolving on the opponent's turn.
    let mut r = runner_with(&[], 10);
    push_trash(&mut r, 0, "HOLY4");
    let sal = r.place_on_field(0, CARD_ID, Some(0));
    r.set_first_player(1);
    assert_eq!(r.turn_player(), 1);
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(sal));
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(top_id(&r, sal), CARD_ID);
}

// ─── [Start of Your Main Phase] ──────────────────────────────────────────────

#[test]
fn bt16_030_start_of_main_phase_digivolves_from_trash() {
    let mut r = runner_with(&[], 10);
    push_trash(&mut r, 0, "HOLY4");
    let sal = r.place_on_field(0, CARD_ID, Some(0));
    fire_start_of_main(&mut r, sal);
    pick_own(&mut r, sal);
    let a = trash_action(&r, "HOLY4");
    r.execute_action(0, a).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, sal), "HOLY4");
    assert_eq!(r.memory(), 10 - 2);
}

#[test]
fn bt16_030_start_of_main_phase_needs_lv4_holy_beast_or_free() {
    let mut r = runner_with(&[], 10);
    push_trash(&mut r, 0, "PLAIN4");
    push_trash(&mut r, 0, "HOLY5");
    let sal = r.place_on_field(0, CARD_ID, Some(0));
    fire_start_of_main(&mut r, sal);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(top_id(&r, sal), CARD_ID);
}

// ─── Inherited [Your Turn] security DP debuff ───────────────────────────────

fn sec_runner(sec_player: u8) -> DebugRunner {
    let mut atk = digimon("ATK", CardColor::Red, 4, &[]);
    atk.dp = Some(8000);
    let mut sec = digimon("SEC", CardColor::Red, 4, &[]);
    sec.dp = Some(9000);
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("loads")
        .add_card(atk)
        .add_card(sec)
        .security(sec_player, &["SEC"])
        .start()
}

#[test]
fn bt16_030_inherited_security_dp_debuff_applies_on_your_turn() {
    let mut r = sec_runner(1);
    let atk = r.place_stack(0, &[CARD_ID, "ATK"]);
    let result = r.attack_player(atk, 1, false);
    assert_eq!(result, AttackResult::SecurityCheckSurvived, "9000 - 3000 < 8000");
}

#[test]
fn bt16_030_inherited_security_dp_debuff_absent_without_salamon() {
    let mut r = sec_runner(1);
    let atk = r.place_on_field(0, "ATK", Some(0));
    let result = r.attack_player(atk, 1, false);
    assert_eq!(result, AttackResult::AttackerDeletedBySecurity);
}

#[test]
fn bt16_030_inherited_security_dp_debuff_not_on_opponents_turn() {
    let mut r = sec_runner(0);
    assert_eq!(r.game.turn_player(), 0);
    let atk = r.place_stack(1, &[CARD_ID, "ATK"]);
    let result = r.attack_player(atk, 0, false);
    assert_eq!(result, AttackResult::AttackerDeletedBySecurity);
}
