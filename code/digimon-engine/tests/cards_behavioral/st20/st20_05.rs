//! ST20-05 Gatomon — Digimon, Lv.4, Yellow, DP 4000, Cost 4.
//! Traits: Holy Beast / ADVENTURE. Attribute: Vaccine.
//!
//! # Printed text (official observation, card_official.json)
//!
//! [Digivolve] Lv.3 w/[ADVENTURE] trait: Cost 2 (standard: Lv.3 yellow / Cost 2)
//! [Security] At the end of the battle, play this card without paying the cost.
//! [On Play] [When Digivolving] Give 2 of your opponent's Digimon
//!   <Security A. -1> until their turn ends.
//! Inherited: [When Attacking] [Once Per Turn] 1 of your opponent's Digimon
//!   gets -2000 DP for the turn.
//!
//! # DCGO reference
//! DCGO/Assets/Scripts/CardEffect/ST20/Yellow/ST20_05.cs — OP/WD body is a
//! SelectPermanentEffect maxCount = min(2, n), canNoSelect false,
//! canEndNotMax false, ChangeDigimonSAttack(-1, UntilOpponentTurnEnd).
//! ESS: OnAllyAttack, maxUsableCount 1, ChangeDigimonDP(-2000, UntilEachTurnEnd).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::TriggerSource;

const CARD_ID: &str = "ST20-05";

fn make_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-05 YAML must be present in the embedded DSL pack")
        .add_card(make_digimon("OPP-A", 4, 6000))
        .add_card(make_digimon("OPP-B", 4, 6000))
        .add_card(make_digimon("OPP-C", 4, 6000))
        .add_card(make_digimon("HOST", 5, 7000))
        .add_card(make_test_card("PAD", "PAD"))
        .deck(0, &["PAD"; 5])
        .deck(1, &["PAD"; 5])
        .memory(10)
}

fn sa_debuffed(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.modifiers.has(h, ModifierType::SecurityAttackChange)
}

// ─── Structural ──────────────────────────────────────────────────────────────

#[test]
fn st20_05_has_printed_stats_and_adventure_alt_digivolve() {
    let r = base().start();
    let card = r.compiled_card(CARD_ID).expect("ST20-05 compiled");
    assert_eq!(card.name, "Gatomon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.dp, Some(4000));
    assert_eq!(card.color, vec![CompiledColor::Yellow]);
    assert!(card.traits.iter().any(|t| t == "ADVENTURE"));
    let has_adv_path = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && matches!(p.cost, Some(CompiledCost::Literal(2)))
            && p.from.as_ref().is_some_and(|f| {
                let preds: Vec<_> = std::iter::once(&**f).chain(f.all_of.iter()).collect();
                preds.iter().any(|q| q.level_eq == Some(3))
                    && preds
                        .iter()
                        .any(|q| q.trait_has.as_deref() == Some("ADVENTURE"))
            })
    });
    assert!(has_adv_path, "must digivolve from Lv.3 [ADVENTURE] for 2");
}

#[test]
fn st20_05_declares_security_op_wd_and_inherited_clauses() {
    let r = base().start();
    let card = r.compiled_card(CARD_ID).expect("ST20-05 compiled");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(triggered
        .iter()
        .any(|t| t.when == vec![CompiledTiming::OnSecurity] && !t.optional));
    assert!(triggered.iter().any(|t| t.when.contains(&CompiledTiming::OnPlay)
        && t.when.contains(&CompiledTiming::WhenDigivolving)
        && !t.optional));
    assert!(triggered.iter().any(|t| t.scope == CompiledScope::Inherited
        && t.when == vec![CompiledTiming::WhenAttacking]
        && t.once_per_turn
        && !t.optional));
}

// ─── [On Play] Security A. -1 to 2 opponent Digimon ─────────────────────────

#[test]
fn st20_05_on_play_gives_exactly_two_of_three_opponent_digimon_security_a_minus_1() {
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(1, "OPP-A", Some(0));
    let b = r.place_on_field(1, "OPP-B", Some(0));
    let c = r.place_on_field(1, "OPP-C", Some(0));

    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();

    let view = r.pending_selection_view().expect("2-target pick installs");
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "the Security A. -1 pick is mandatory (no PASS)"
    );
    assert!(!r.pending_is_optional());
    r.auto_resolve().expect("resolve both picks");

    let debuffed = [a, b, c].iter().filter(|h| sa_debuffed(&r, **h)).count();
    assert_eq!(debuffed, 2, "exactly 2 opponent Digimon get <Security A. -1>");
    assert!(
        !sa_debuffed(&r, me),
        "Gatomon itself must not be debuffed (opponent's Digimon only)"
    );
}

#[test]
fn st20_05_on_play_with_one_opponent_digimon_debuffs_it() {
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(1, "OPP-A", Some(0));

    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    r.auto_resolve().expect("resolve");

    assert!(sa_debuffed(&r, a), "the lone opponent Digimon is debuffed");
}

#[test]
fn st20_05_security_a_debuff_value_is_minus_one_until_opponents_turn_end() {
    use digimon_engine::enums::Expiry;
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(1, "OPP-A", Some(0));

    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    r.auto_resolve().expect("resolve");

    let entries = r.game.modifiers.get(a, ModifierType::SecurityAttackChange);
    assert_eq!(entries.len(), 1, "[When Digivolving] also debuffs");
    assert_eq!(entries[0].value, -1);
    assert_eq!(entries[0].expiry, Expiry::EndOfOpponentsTurn);
}

#[test]
fn st20_05_on_play_with_no_opponent_digimon_installs_nothing() {
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_none());
}

// ─── [Security] play self after battle ───────────────────────────────────────

#[test]
fn st20_05_security_plays_card_onto_field() {
    let mut r = base().add_card(make_digimon("ATTACKER", 4, 3000)).start();
    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    let idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == CARD_ID)
        .unwrap();
    let iid = r.game.next_card_index();
    r.game.players[1].security.push(CardSource::new(idx, 1, iid));

    r.attack_player(attacker, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(r.security_count(1), 0);
    assert!(
        r.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == CARD_ID),
        "Gatomon must be played onto its owner's battle area by its [Security] effect"
    );
}

// ─── Inherited [When Attacking][OPT] -2000 DP ────────────────────────────────

#[test]
fn st20_05_inherited_when_attacking_gives_minus_2000_dp() {
    let mut r = base().start();
    let host = r.place_stack(0, &[CARD_ID, "HOST"]);
    let a = r.place_on_field(1, "OPP-A", Some(0));
    let before = r.effective_dp(a).unwrap();

    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(host));
    r.game.drain_effect_queue();
    let view = r.pending_selection_view().expect("-2000 target pick installs");
    assert!(!view.valid_action_ids.contains(&PASS), "mandatory pick");
    r.auto_resolve().expect("resolve");

    assert_eq!(r.effective_dp(a), Some(before - 2000));
}

#[test]
fn st20_05_inherited_when_attacking_is_once_per_turn() {
    let mut r = base().start();
    let host = r.place_stack(0, &[CARD_ID, "HOST"]);
    let a = r.place_on_field(1, "OPP-A", Some(0));
    let before = r.effective_dp(a).unwrap();

    for _ in 0..2 {
        r.game
            .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(host));
        r.game.drain_effect_queue();
        r.auto_resolve().expect("resolve");
    }
    assert_eq!(
        r.effective_dp(a),
        Some(before - 2000),
        "the inherited -2000 applies only once per turn"
    );
}
