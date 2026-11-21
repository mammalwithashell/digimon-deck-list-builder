//! EX11-047 Impmon — Digimon, Lv.3, Purple/Red, DP 1000, Cost 3.
//! Traits: Evil, LIBERATOR.
//!
//! # Card text (card image + official Bandai DB bundle `data/card_bundles/EX11-047.md`)
//!
//! - Evo boxes: Lv.2 purple/red split circle, cost 1; "[Digivolve] [Yaamon]: Cost 0".
//! - [Start of Your Main Phase] Trash 1 card in your hand. Then, gain 1 memory.
//! - Inherited: [Your Turn] This Digimon gets +2000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX11/Purple/EX11_047.cs — exact-name
//! `EqualsCardName("Yaamon")` alt digivolve; mandatory discard (canNoSelect
//! false) guarded by `HandCards.Count >= 1`; memory gain independent of the
//! discard; inherited `ChangeSelfDPStaticEffect(+2000)` on the owner's turn.
//!
//! # Replay regression (2026-11 DCGO PvP game)
//! A hatched [Yaamon] (EX11-005, cards.json `level: null`) digivolved into
//! this card from the BREEDING area through the cost-0 alt box.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_digivolve, BREEDING_TARGET, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};

const CARD_ID: &str = "EX11-047";

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A hatched [Yaamon] exactly as cards.json carries EX11-005 (`level: null`,
/// no `form_eng` — the API ingest drops both).
fn yaamon_from_cards_json(id: &str) -> CardData {
    let json = format!(
        r#"{{ "{id}": {{
            "index": 1, "norm_id": 0.00005, "card_id": "{id}", "card_index": 1,
            "card_name_eng": "Yaamon", "card_name_jpn": "",
            "card_effect_class_name": "X", "play_cost": 0, "dp": null,
            "level": null, "card_kind": 3, "rarity": 0, "card_colors": [6],
            "type_eng": ["Lesser", "LIBERATOR"], "form_eng": [], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        }} }}"#
    );
    CardData::load_from_str(&json)
        .expect("egg row parses")
        .remove(id)
        .expect("egg row present")
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-047 YAML loads from the embedded DSL pack")
        .add_card(filler("PAD-1"))
        .add_card(filler("PAD-2"))
        .add_card(filler("PAD-3"))
}

#[test]
fn ex11_047_registers_split_circle_and_yaamon_cost_0_paths() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-047 YAML loads")
        .build();
    let card = runner.compiled_card(CARD_ID).expect("compiled card present");
    let mut costs: Vec<i32> = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .map(|p| match p.cost.as_ref() {
            Some(CompiledCost::Literal(v)) => *v,
            other => panic!("literal costs expected, got {other:?}"),
        })
        .collect();
    costs.sort_unstable();
    assert_eq!(costs, vec![0, 1, 1], "purple /1 + red /1 split halves + [Yaamon] /0");
}

// ─── [Start of Your Main Phase] trash 1, then gain 1 memory ─────────────────

fn start_main_runner(hand: &[&str]) -> DebugRunner {
    let mut runner = builder()
        .hand(0, hand)
        .deck(0, &["PAD-1", "PAD-2", "PAD-3"])
        .deck(1, &["PAD-1", "PAD-2"])
        .memory(0)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.set_memory(2);
    runner
}

#[test]
fn ex11_047_start_of_main_trashes_one_card_then_gains_memory() {
    let mut runner = start_main_runner(&["PAD-1", "PAD-2"]);
    let before = runner.memory();
    runner.game.enter_main_phase();

    let view = runner.pending_selection_view().expect("discard pick pending");
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "the discard is mandatory (DCGO canNoSelect = false)"
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("discard one card");
    let _ = runner.auto_resolve();

    assert_eq!(runner.game.players[0].hand.len(), 1, "exactly one card trashed");
    assert_eq!(runner.game.players[0].trash.len(), 1);
    assert_eq!(runner.memory(), before + 1, "then gain 1 memory");
}

/// "Then, gain 1 memory" does not depend on a card having been trashed
/// (DCGO guards only the discard with `HandCards.Count >= 1`).
#[test]
fn ex11_047_start_of_main_gains_memory_even_with_an_empty_hand() {
    let mut runner = start_main_runner(&[]);
    let before = runner.memory();
    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].trash.len(), 0);
    assert_eq!(runner.memory(), before + 1, "empty hand still gains 1 memory");
}

// ─── Inherited +2000 DP on your turn ────────────────────────────────────────

fn carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.colors = vec![CardColor::Red];
    c
}

#[test]
fn ex11_047_inherited_gives_plus_2000_dp_on_your_turn_only() {
    let mut runner = builder()
        .add_card(carrier("CARRIER"))
        .deck(0, &["PAD-1", "PAD-2", "PAD-3"])
        .deck(1, &["PAD-1", "PAD-2", "PAD-3"])
        .memory(10)
        .start();
    let handle = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.game.effective_dp(handle),
        Some(7000),
        "[Your Turn] +2000 DP from the inherited effect"
    );
    runner.end_turn();
    let _ = runner.auto_resolve();
    runner.game.tick_declarative_effects();
    assert_eq!(runner.turn_player(), 1);
    assert_eq!(
        runner.game.effective_dp(handle),
        Some(5000),
        "no bonus on the opponent's turn"
    );
}

// ─── Alt digivolve [Yaamon]: Cost 0 from the breeding area ──────────────────

fn breeding_runner(base: CardData) -> DebugRunner {
    let base_id = base.card_id.clone();
    let mut r = builder()
        .add_card(base)
        .hand(0, &[CARD_ID])
        .deck(0, &["PAD-1", "PAD-2", "PAD-3"])
        .memory(5)
        .start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r.place_in_breeding(0, &base_id);
    r
}

/// A hatched [Yaamon] satisfies the cost-0 box by name even though cards.json
/// gives the egg no level and no form (name route needs neither); the digivolve
/// is offered and cost 0 pays nothing.
#[test]
fn ex11_047_breeding_yaamon_offers_cost_0_route() {
    let mut runner = breeding_runner(yaamon_from_cards_json("YAAMON"));

    let mask = build_action_mask(&runner.game, 0);
    assert!(
        mask[encode_digivolve(0, BREEDING_TARGET) as usize] > 0.0,
        "the breeding digivolve must be offered over a hatched [Yaamon]"
    );
    let before = runner.game.memory;
    let proceeded = runner
        .game
        .digivolve_from_hand_onto_breeding(0, 0, PlaySource::ByHand);
    if !proceeded {
        // The purple Lv.2 circle may also apply -> a cost choice.
        let view = runner.pending_selection_view().expect("cost choice pending");
        let choices = view.effect_choices.as_ref().expect("effect_choices");
        let cost0 = choices
            .iter()
            .find(|c| c.label.to_lowercase().contains("cost 0"))
            .expect("cost-0 option")
            .action_id;
        runner.execute_action(0, cost0).expect("pick cost 0");
    }
    assert_eq!(runner.game.memory, before, "cost-0 route pays nothing");
    let top = runner.game.players[0]
        .breeding_area
        .as_ref()
        .expect("breeding permanent remains")
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(top, CARD_ID);
}
