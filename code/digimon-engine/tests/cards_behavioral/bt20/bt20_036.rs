//! BT20-036 BanchoLeomon — Lv.6 Yellow/Black, DP 12000, Cost 12.
//! Full clause inventory in the YAML header. The Q30 judge-quiz pin
//! (`judge_quiz/c_declare_then_pay.rs`) exercises this card as a Partition
//! source played mid-deletion.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

#[test]
fn bt20_036_compiles_with_redirect_and_eot_dna_clauses() {
    let r = DebugRunner::builder()
        .dsl_card("BT20-036")
        .expect("BT20-036 YAML parses and compiles")
        .build();
    let card = r.compiled_card("BT20-036").expect("present in pack");
    assert_eq!(card.alt_paths.len(), 2, "Lv5 yellow/4 + Lv5 ACCEL/3");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::OnOpponentAttack)
        )),
        "inherited [Opp Turn][OPT] attack redirect compiles"
    );
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp
                    && t.when.contains(&CompiledTiming::EndOfYourTurn)
        )),
        "[End of Your Turn] DNA-into-[Chaosmon] clause compiles"
    );
}

/// [On Play]: De-Digivolve 2 on one opponent Digimon, then −5000 DP on one.
#[test]
fn bt20_036_on_play_de_digivolves_2_then_minus_5000() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-036")
        .expect("BT20-036 loads")
        .add_card({
            let mut c = make_test_card("BL-OPP-T", "Opp Top");
            c.card_kind = CardKind::Digimon;
            c.level = Some(5);
            c.dp = Some(7000);
            c
        })
        .add_card({
            let mut c = make_test_card("BL-OPP-S", "Opp Src");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(8000);
            c
        })
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);

    let opp = r.place_stack(1, &["BL-OPP-S", "BL-OPP-S", "BL-OPP-T"]);
    let bancho = r.place_on_field(0, "BT20-036", Some(0));
    r.fire_on_play(0, bancho.index as usize);

    // De-Digivolve target pick, then the −5000 target pick.
    r.auto_resolve().expect("both opponent picks resolve");

    assert_eq!(
        r.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        1,
        "De-Digivolve 2 trashed 2 cards off the stack"
    );
    // The De-Digivolve exposed the bottom BL-OPP-S (base 8000); the second
    // pick applied -5000 until the end of their turn → 3000.
    assert_eq!(
        r.dp_of(opp),
        Some(3000),
        "the -5000 DP modifier applies to the surviving Digimon"
    );
}
