//! EX10-059 DarknessBagramon — Digimon, Lv.7, Purple/Black, DP 16000, Cost 16.
//! Traits: [Composite, Bagra Army].
//!
//! # Card text (card image — verified)
//!
//! [On Play][When Digivolving] Choose 1 card in your opponent's hand without
//! looking and place it as any of their Digimon's bottom digivolution card or
//! under any of their Tamers. Then, by placing 3 [Bagra Army] trait Digimon
//! cards from your trash as this Digimon's top digivolution cards, delete 1
//! of their Digimon or Tamers with cards under it.
//! — ***BOTH SENTENCES OMITTED (PARTIAL)***: blind opponent-hand pick +
//! cross-player tuck (G-DSL-BLIND-OPP-HAND-PLACE); sentence 2's
//! place-as-TOP-source verb now exists (`place_as_top_source`,
//! G-DSL-PLACE-AS-TOP-SOURCE resolved 2026-07-05) but the clause still
//! needs the 3-pick trash-cost flow + the "with cards under it" target
//! filter. See qa/dsl-vocab-gaps.md. NOT stubbed.
//!
//! [All Turns] This Digimon gains all [All Turns] effects on all level 6
//! [Bagra Army] trait Digimon cards in its digivolution cards.
//! — ***OMITTED (PARTIAL)***: no source-card effect-adoption machinery
//! (G-DSL-GAIN-ALL-TURNS-FROM-SOURCES, qa/dsl-vocab-gaps.md).
//!
//! DigiXros -3: [Bagramon] × [DarkKnightmon]
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX10/Purple/EX10_059.cs
//!
//! The full DigiXros behavioral coverage — material commit order, Yuu Amano
//! pre-attach interaction, and the 16 −3 −3 −2×N cost stack — is the
//! judge-quiz Q29 pin (`tests/judge_quiz/e_partition_digixros.rs`).

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledZone};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn ex10_059_compiles_with_named_two_slot_digixros_path() {
    let r = DebugRunner::builder()
        .dsl_card("EX10-059")
        .expect("EX10-059 YAML parses and compiles")
        .build();
    let card = r.compiled_card("EX10-059").expect("present in pack");

    let xros = card
        .alt_paths
        .iter()
        .find(|p| matches!(p.kind, CompiledAltPathKind::DigiXros))
        .expect("the [DigiXros -3] path must compile");
    assert_eq!(
        xros.materials.len(),
        2,
        "[Bagramon] × [DarkKnightmon] slots"
    );
    for slot in &xros.materials {
        assert_eq!(
            slot.cost_delta,
            Some(-3),
            "each used material reduces the cost by 3"
        );
        assert_eq!(
            slot.zones,
            vec![CompiledZone::Hand],
            "the printed DigiXros materials come from hand"
        );
    }
}

/// The DigiXros transaction computes 16 −3 −3 = 10 with both named materials
/// (judge-quiz Q29's cost base, before Yuu Amano's −2s).
#[test]
fn ex10_059_digixros_with_both_materials_costs_10() {
    use digimon_engine::action::space::PASS;
    use digimon_engine::selection::SelectionKind;

    let mut r = DebugRunner::builder()
        .dsl_card("EX10-059")
        .expect("EX10-059 loads")
        .dsl_card("EX10-056")
        .expect("EX10-056 Bagramon loads")
        .dsl_card("EX10-031")
        .expect("EX10-031 DarkKnightmon loads")
        .memory(10)
        .hand(0, &["EX10-059", "EX10-056", "EX10-031"])
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    let memory_before = r.memory();

    let _ = r.play(0, 0);
    // Fill both named material slots from hand, then commit.
    for _ in 0..2 {
        let view = r
            .pending_selection_view()
            .expect("a DigiXros material slot must surface");
        assert_eq!(view.kind, SelectionKind::Material);
        let pick = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .expect("a hand material candidate");
        r.execute_action(0, pick).expect("select material");
    }
    if r.pending_selection_view()
        .is_some_and(|v| v.kind == SelectionKind::Material)
    {
        let _ = r.execute_action(0, PASS);
    }
    let _ = r.auto_resolve();

    assert_eq!(
        r.memory(),
        memory_before - 10,
        "DarknessBagramon via DigiXros with both materials must cost 16 −3 −3 = 10"
    );
    let db = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "EX10-059")
        .expect("DarknessBagramon on field");
    // Materials commit bottom→top in requirement order: DarkKnightmon at the
    // bottom, Bagramon above it, the top card last.
    let ids: Vec<&str> = db
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data))
        .collect();
    assert_eq!(
        ids,
        vec!["EX10-031", "EX10-056", "EX10-059"],
        "materials must sit at the bottom in requirement-spec order"
    );
}
