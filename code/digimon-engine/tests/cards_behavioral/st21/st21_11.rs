//! ST21-11 MetalGarurumon — Digimon, Lv.6, Purple/Blue, DP 12000, Cost 7.
//! Traits: Cyborg / ADVENTURE. Attribute: Data.
//!
//! Printed (official observation / data/card_bundles/ST21-11.md):
//!   Standard digivolve: Purple Lv.5 / 4, Blue Lv.5 / 4  (from cards.json evo_costs)
//!   [Digivolve] Lv.5 w/[ADVENTURE] trait: Cost 3
//!   {Hand} [Counter] <Blast Digivolve>
//!   [On Play] [When Digivolving] Return 1 of your opponent's level 4 or lower
//!     Digimon to the bottom of the deck. For every 2 colors your Tamers have,
//!     add 1 to this effect's level maximum.
//!   [When Attacking] [Once Per Turn] You may play 1 level 4 or lower Digimon
//!     card from your trash without paying the cost.
//!   Special Rule: <Overflow (-4)>
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/ST21/Purple/ST21_11.cs — level max =
//! 4 + GetUniqueColourCountOnOwnerBattleArea(own Tamers) / 2; bottom-deck pick
//! canNoSelect false; WA OPT isOptional, trash pick canNoSelect true, filter
//! IsDigimon && Level <= 4. Sibling of ST20-11 WarGreymon.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledPredicate,
    CompiledTiming,
};
use digimon_engine::action::space::{ATTACK_START, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::TriggerSource;

const CARD_ID: &str = "ST21-11";

fn digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c.play_cost = level as u16;
    c
}

fn tamer(id: &str, colors: &[CardColor]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = colors.to_vec();
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-11 YAML must be present in the embedded DSL pack")
        .add_card(digimon("OPP-L4", 4))
        .add_card(digimon("OPP-L5", 5))
        .add_card(digimon("OPP-L6", 6))
        .add_card(digimon("TRASH-L4", 4))
        .add_card(digimon("TRASH-L5", 5))
        .add_card(tamer("T-RED", &[CardColor::Red]))
        .add_card(tamer("T-BLUE", &[CardColor::Blue]))
        .add_card(tamer("T-GY", &[CardColor::Green, CardColor::Yellow]))
        .add_card(make_test_card("PAD", "PAD"))
        .deck(0, &["PAD"; 5])
        .deck(1, &["PAD"; 5])
        .memory(10)
}

fn opp_slot(h: PermanentHandle) -> u16 {
    ATTACK_START + h.index as u16
}

fn flat(p: &CompiledPredicate) -> Vec<&CompiledPredicate> {
    let mut v = vec![p];
    for q in p.all_of.iter().chain(p.any_of.iter()) {
        v.extend(flat(q));
    }
    v
}

/// Fire [On Play] and return the set of offered opponent-field actions.
fn offered_after_on_play(r: &mut DebugRunner, me: PermanentHandle) -> Vec<u16> {
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    r.pending_selection_view()
        .map(|v| v.valid_action_ids.clone())
        .unwrap_or_default()
}

// ─── Structural ──────────────────────────────────────────────────────────────

#[test]
fn st21_11_stats_overflow_and_alt_paths() {
    let r = base().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(12000));
    assert!(card.color.contains(&CompiledColor::Purple) && card.color.contains(&CompiledColor::Blue));
    assert_eq!(card.ace_overflow, Some(-4));
    assert!(card
        .alt_paths
        .iter()
        .any(|p| p.kind == CompiledAltPathKind::BurstDigivolve));
    assert!(card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && matches!(p.cost, Some(CompiledCost::Literal(3)))
            && p.from.as_ref().is_some_and(|f| {
                let ps = flat(f);
                ps.iter().any(|q| q.level_eq == Some(5))
                    && ps.iter().any(|q| q.trait_has.as_deref() == Some("ADVENTURE"))
            })
    }));
    let t: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(t.iter().any(|t| t.when.contains(&CompiledTiming::OnPlay)
        && t.when.contains(&CompiledTiming::WhenDigivolving)
        && !t.optional));
    assert!(t
        .iter()
        .any(|t| t.when == vec![CompiledTiming::WhenAttacking] && t.once_per_turn));
}

// ─── [On Play][When Digivolving] level-max scaling bottom-deck ───────────────

#[test]
fn st21_11_no_tamers_level_max_is_4() {
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let l4 = r.place_on_field(1, "OPP-L4", Some(0));
    let l5 = r.place_on_field(1, "OPP-L5", Some(0));
    let offered = offered_after_on_play(&mut r, me);
    assert!(offered.contains(&opp_slot(l4)));
    assert!(!offered.contains(&opp_slot(l5)));
    assert!(!offered.contains(&PASS), "the bottom-deck pick is mandatory");
}

#[test]
fn st21_11_one_tamer_color_still_level_max_4() {
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "T-RED", Some(0));
    let l5 = r.place_on_field(1, "OPP-L5", Some(0));
    let offered = offered_after_on_play(&mut r, me);
    assert!(!offered.contains(&opp_slot(l5)), "1 color / 2 = 0 bonus");
}

#[test]
fn st21_11_two_tamer_colors_level_max_5() {
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "T-RED", Some(0));
    r.place_on_field(0, "T-BLUE", Some(0));
    let l5 = r.place_on_field(1, "OPP-L5", Some(0));
    let l6 = r.place_on_field(1, "OPP-L6", Some(0));
    let offered = offered_after_on_play(&mut r, me);
    assert!(offered.contains(&opp_slot(l5)));
    assert!(!offered.contains(&opp_slot(l6)));
}

#[test]
fn st21_11_four_tamer_colors_level_max_6() {
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "T-RED", Some(0));
    r.place_on_field(0, "T-BLUE", Some(0));
    r.place_on_field(0, "T-GY", Some(0));
    let l6 = r.place_on_field(1, "OPP-L6", Some(0));
    let offered = offered_after_on_play(&mut r, me);
    assert!(offered.contains(&opp_slot(l6)), "4 colors / 2 = +2 → level 6");
}

#[test]
fn st21_11_when_digivolving_returns_chosen_digimon_to_deck_bottom() {
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let l4 = r.place_on_field(1, "OPP-L4", Some(0));
    let deck_before = r.deck_size(1);
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    r.auto_resolve().expect("resolve");
    assert_eq!(r.battle_area_size(1), 0);
    assert_eq!(r.deck_size(1), deck_before + 1);
    assert!(
        r.game.players[1]
            .deck
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "OPP-L4"),
        "returned to the deck"
    );
}

// ─── [When Attacking][OPT] optional play Lv.4- Digimon from trash ────────────

fn push_trash(r: &mut DebugRunner, id: &str) {
    let idx = r.game.card_data.iter().position(|c| c.card_id == id).unwrap();
    let iid = r.game.next_card_index();
    r.game.players[0].trash.push(CardSource::new(idx, 0, iid));
}

fn on_field(r: &DebugRunner, id: &str) -> bool {
    r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == id)
}

fn accept_outer(r: &mut DebugRunner) {
    if r.pending_kind() == Some(digimon_engine::selection::SelectionKind::Replacement) {
        r.accept_optional_trigger().expect("accept outer optional prompt");
    }
}

#[test]
fn st21_11_when_attacking_plays_level_4_digimon_from_trash_free_once_per_turn() {
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, "TRASH-L4");
    push_trash(&mut r, "TRASH-L5");
    let mem = r.memory();

    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    accept_outer(&mut r);
    let view = r.pending_selection_view().expect("trash pick");
    let picks: Vec<_> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|a| *a != PASS)
        .collect();
    assert_eq!(picks.len(), 1, "only the level 4 Digimon is offered");
    r.execute_action(view.selecting_player, picks[0]).expect("pick");
    let _ = r.auto_resolve();
    assert!(on_field(&r, "TRASH-L4"));
    assert_eq!(r.memory(), mem, "without paying the cost");

    push_trash(&mut r, "TRASH-L4");
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_none(), "once per turn");
}

#[test]
fn st21_11_when_attacking_decline_plays_nothing() {
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, "TRASH-L4");
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_some(), "an optional prompt is offered");
    while let Some(sel) = r.pending_selection() {
        let p = sel.selecting_player;
        r.execute_action(p, PASS).expect("PASS legal ('you may')");
    }
    assert!(!on_field(&r, "TRASH-L4"));
}

/// Q1818: declining the "you may" does not use up the [Once Per Turn] — a
/// second attack the same turn offers the play again.
#[test]
fn st21_11_when_attacking_decline_keeps_once_per_turn() {
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, "TRASH-L4");
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    while let Some(sel) = r.pending_selection() {
        let p = sel.selecting_player;
        r.execute_action(p, PASS).expect("PASS legal ('you may')");
    }
    assert!(!on_field(&r, "TRASH-L4"));

    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    accept_outer(&mut r);
    assert!(
        r.pending_selection().is_some(),
        "a declined [Once Per Turn] 'you may' is offered again"
    );
}

// ─── {Hand}[Counter] <Blast Digivolve> ───────────────────────────────────────

/// When the opponent attacks, the Counter window offers digivolving ST21-11
/// from hand onto an eligible Lv.5 [ADVENTURE] Digimon; taking it digivolves
/// for free (no memory paid). DebugRunner DSL cards carry no printed
/// evo_costs, so the eligible route here is the Lv.5 [ADVENTURE] alt-path.
#[test]
fn st21_11_blast_digivolve_from_hand_in_counter_window_is_free() {
    use digimon_engine::action::space::encode_digivolve;
    use digimon_engine::combat::AttackResult;
    use digimon_engine::enums::GamePhase;

    let mut adv5 = digimon("ADV-L5", 5);
    adv5.traits = vec!["ADVENTURE".to_string()];
    let mut r = base()
        .add_card(adv5)
        .add_card(digimon("ATK", 6))
        .hand(1, &[CARD_ID])
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let target = r.place_on_field(1, "ADV-L5", Some(0));
    let mem_before = r.memory();

    let result = r.attack_digimon(atk, target, false);
    assert_eq!(result, AttackResult::InProgress);
    assert_eq!(r.current_phase(), GamePhase::CounterTiming);
    let view = r.pending_selection_view().expect("Counter window installs");
    assert_eq!(view.selecting_player, 1, "the defender decides");
    assert!(view.is_optional, "Blast Digivolve is a 'may'");
    let blast = encode_digivolve(0, target.index as u16);
    assert!(
        view.valid_action_ids.contains(&blast),
        "Blast Digivolve (hand 0 → ADV-L5) must be offered; got {:?}",
        view.valid_action_ids
    );
    r.execute_action(1, blast).expect("take Blast Digivolve");

    assert_eq!(
        r.game.players[1].battle_area[target.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        CARD_ID,
        "ST21-11 digivolved onto the attacked Digimon"
    );
    assert_eq!(r.memory(), mem_before, "Blast Digivolve pays no cost");
}
