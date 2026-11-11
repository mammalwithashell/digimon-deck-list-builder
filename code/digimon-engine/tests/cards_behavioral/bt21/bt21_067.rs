//! BT21-067 Garurumon — Digimon, Lv.4, Purple, DP 4000, Cost 4.
//! Traits: Beast / ADVENTURE. Attribute: Vaccine.
//!
//! Printed (official observation / data/card_bundles/BT21-067.md):
//!   Standard digivolve: Purple Lv.3 / cost 2   (cards.json ALSO carries a stale cost-3 circle — open data drift)
//!   [Digivolve] Lv.3 w/[Gabumon] in name or w/[ADVENTURE] trait: Cost 2
//!   [Security] At the end of the battle, play this card without paying the cost.
//!   [On Play] [When Digivolving] You may return 1 Digimon card with the
//!     [ADVENTURE] trait from your trash to the hand.
//!   Inherited: [When Attacking] [Once Per Turn] <Draw 1> and trash 1 card in
//!     your hand.
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/BT21/Purple/BT21_067.cs

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledPredicate,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::SelectionKind;
use digimon_engine::TriggerSource;

const CARD_ID: &str = "BT21-067";

fn digimon(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-067 YAML must be present in the embedded DSL pack")
        .add_card(digimon("ADV-DIGI", &["ADVENTURE"]))
        .add_card(digimon("NON-ADV", &["Beast"]))
        .add_card({
            let mut c = make_test_card("ADV-OPT", "AdvOption");
            c.card_kind = CardKind::Option;
            c.traits = vec!["ADVENTURE".to_string()];
            c
        })
        .add_card(digimon("HOST", &["Beast"]))
        .add_card(make_test_card("PAD", "PAD"))
        .deck(0, &["PAD"; 5])
        .deck(1, &["PAD"; 5])
        .memory(10)
}

fn push_trash(r: &mut DebugRunner, id: &str) {
    let idx = r.game.card_data.iter().position(|c| c.card_id == id).unwrap();
    let iid = r.game.next_card_index();
    r.game.players[0].trash.push(CardSource::new(idx, 0, iid));
}

fn flat(p: &CompiledPredicate) -> Vec<&CompiledPredicate> {
    let mut v = vec![p];
    for q in &p.all_of {
        v.extend(flat(q));
    }
    for q in &p.any_of {
        v.extend(flat(q));
    }
    v
}

// ─── Structural ──────────────────────────────────────────────────────────────

#[test]
fn bt21_067_alt_path_gabumon_or_adventure_lv3_cost_2() {
    let r = base().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.color, vec![CompiledColor::Purple]);
    // Special condition: Lv.3 with [Gabumon] in name OR [ADVENTURE] trait.
    assert!(card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && matches!(p.cost, Some(CompiledCost::Literal(2)))
            && p.from.as_ref().is_some_and(|f| {
                let ps = flat(f);
                ps.iter().any(|q| q.level_eq == Some(3))
                    && ps
                        .iter()
                        .any(|q| q.name_contains.as_deref() == Some("Gabumon"))
                    && ps
                        .iter()
                        .any(|q| q.trait_has.as_deref() == Some("ADVENTURE"))
            })
    }));
}

#[test]
fn bt21_067_declares_security_optional_op_wd_and_inherited_draw_discard() {
    let r = base().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let t: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(t
        .iter()
        .any(|t| t.when == vec![CompiledTiming::OnSecurity] && !t.optional));
    assert!(t.iter().any(|t| t.when.contains(&CompiledTiming::OnPlay)
        && t.when.contains(&CompiledTiming::WhenDigivolving)));
    assert!(t.iter().any(|t| t.scope == CompiledScope::Inherited
        && t.when == vec![CompiledTiming::WhenAttacking]
        && t.once_per_turn));
}

// ─── [On Play] optional return ───────────────────────────────────────────────

#[test]
fn bt21_067_on_play_returns_adventure_digimon_from_trash() {
    let mut r = base().hand(0, &[CARD_ID]).start();
    push_trash(&mut r, "ADV-DIGI");
    let hand_before = r.hand_size(0);
    r.play(0, 0).expect("play Garurumon");
    assert_eq!(r.pending_kind(), Some(SelectionKind::Trash));
    assert!(r.pending_is_optional(), "'You may return' must be declinable");
    r.auto_resolve().expect("pick");
    assert_eq!(r.hand_size(0), hand_before, "-1 played +1 returned");
    assert_eq!(r.trash_size(0), 0);
}

#[test]
fn bt21_067_on_play_decline_leaves_trash_untouched() {
    let mut r = base().hand(0, &[CARD_ID]).start();
    push_trash(&mut r, "ADV-DIGI");
    r.play(0, 0).expect("play");
    let p = r.pending_selection().expect("pick").selecting_player;
    r.execute_action(p, PASS).expect("decline");
    assert!(r.pending_selection().is_none());
    assert_eq!(r.trash_size(0), 1);
}

#[test]
fn bt21_067_on_play_non_adventure_or_non_digimon_not_selectable() {
    let mut r = base().hand(0, &[CARD_ID]).start();
    push_trash(&mut r, "NON-ADV");
    push_trash(&mut r, "ADV-OPT");
    push_trash(&mut r, "ADV-DIGI");
    r.play(0, 0).expect("play");
    let view = r
        .pending_selection_view()
        .expect("the trash pick must install (ADV-DIGI is eligible)");
    let picks: Vec<_> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|a| *a != PASS)
        .collect();
    assert_eq!(
        picks.len(),
        1,
        "only the ADVENTURE Digimon is offered; the non-ADVENTURE Digimon and the ADVENTURE Option are not"
    );
    r.execute_action(view.selecting_player, picks[0]).expect("pick");
    r.auto_resolve().expect("finish");
    let in_hand = |id: &str| {
        r.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == id)
    };
    assert!(in_hand("ADV-DIGI"));
    assert!(!in_hand("NON-ADV") && !in_hand("ADV-OPT"));
}

#[test]
fn bt21_067_when_digivolving_fires_return() {
    let mut r = base().start();
    push_trash(&mut r, "ADV-DIGI");
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    assert_eq!(r.pending_kind(), Some(SelectionKind::Trash));
}

// ─── [Security] ──────────────────────────────────────────────────────────────

#[test]
fn bt21_067_security_plays_card_onto_field() {
    let mut r = base().add_card(digimon("ATTACKER", &[])).start();
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
    assert!(r.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == CARD_ID));
}

// ─── Inherited [When Attacking][OPT] draw 1 + trash 1 ────────────────────────

#[test]
fn bt21_067_inherited_when_attacking_draws_then_trashes_one() {
    let mut r = base().hand(0, &["PAD"; 2]).start();
    let host = r.place_stack(0, &[CARD_ID, "HOST"]);
    let hand_before = r.hand_size(0);
    let deck_before = r.deck_size(0);
    let trash_before = r.trash_size(0);
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(host));
    r.game.drain_effect_queue();
    let view = r.pending_selection_view().expect("hand-trash pick installs");
    assert!(!view.valid_action_ids.contains(&PASS), "the trash is mandatory");
    r.auto_resolve().expect("resolve");
    assert_eq!(r.deck_size(0), deck_before - 1, "drew 1");
    assert_eq!(r.hand_size(0), hand_before, "+1 drawn -1 trashed");
    assert_eq!(r.trash_size(0), trash_before + 1);

    // Once per turn.
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(host));
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck_before - 1, "only once per turn");
}

// ─── Production evo_costs regression (BLOCKED on data correction) ────────────

/// Using PRODUCTION card data (data/cards.json), digivolving BT21-067 from a
/// plain Purple Lv.3 base must cost exactly 2 with no cost-choice prompt: the
/// official DB / card image print a single Purple Lv.3 circle, cost 2.
///
/// BLOCKED: data/card_overrides.json (frozen legacy input) adds a second
/// Purple Lv.3 / cost-3 circle that flows into data/cards.json, so the engine
/// installs a 2-vs-3 cost-choice prompt. The fix is a data/card_corrections.json
/// evo_costs entry citing an official observation for BT21-067, which is not
/// committed under data/card_sources/observations/official/ yet. Un-ignore
/// once the corrected cards.json lands.
#[test]
#[ignore = "BLOCKED: stale Purple Lv.3 cost-3 circle in data/cards.json (card_overrides.json); needs card_corrections entry + official observation"]
fn bt21_067_production_purple_lv3_route_costs_exactly_2_with_no_choice() {
    use digimon_engine::enums::{CardColor, PlaySource};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/cards.json");
    let prod = CardData::load_from_file(&path).expect("load data/cards.json");
    let card = prod.get(CARD_ID).expect("BT21-067 in cards.json").clone();
    let purple_lv3_circles: Vec<_> = card
        .evo_costs
        .iter()
        .filter(|e| e.level == 3 && e.card_color == CardColor::Purple as u8)
        .map(|e| e.memory_cost)
        .collect();
    assert_eq!(
        purple_lv3_circles,
        vec![2],
        "production data must carry exactly one Purple Lv.3 circle, cost 2"
    );

    let mut purple_base = digimon("PURPLE-L3", &["Beast"]);
    purple_base.colors = vec![CardColor::Purple];
    let mut r = DebugRunner::builder()
        .add_card(card) // production CardData wins over the compiled-card default
        .dsl_card(CARD_ID)
        .expect("BT21-067 YAML in embedded pack")
        .add_card(purple_base)
        .add_card(make_test_card("PAD", "PAD"))
        .hand(0, &[CARD_ID])
        .deck(0, &["PAD"; 5])
        .deck(1, &["PAD"; 5])
        .memory(10)
        .start();
    let base = r.place_on_field(0, "PURPLE-L3", Some(0));
    r.game.enter_main_phase();
    let mem = r.memory();
    let ok = r
        .game
        .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByDigivolve);
    assert!(
        r.pending_selection().is_none(),
        "no digivolution cost-choice prompt may be offered (single printed cost)"
    );
    assert!(ok, "digivolve succeeds");
    assert_eq!(mem - r.memory(), 2, "pays the printed cost 2");
}
