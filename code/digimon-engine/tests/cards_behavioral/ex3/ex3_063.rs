//! EX3-063 Imperialdramon: Dragon Mode — Lv.6 Purple/Red, DP 12000, Cost 12.
//! The full DNA-digivolve + opponent-keeps-1 + Partition interaction is the
//! judge-quiz Q30 pin (`judge_quiz/c_declare_then_pay.rs`), which drives
//! this card end-to-end from Flamedramon's inherited [EoT] DNA digivolve.

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn ex3_063_compiles_with_dna_path_and_both_clauses() {
    let r = DebugRunner::builder()
        .dsl_card("EX3-063")
        .expect("EX3-063 YAML parses and compiles")
        .build();
    let card = r.compiled_card("EX3-063").expect("present in pack");
    assert!(
        card.alt_paths
            .iter()
            .any(|p| matches!(p.kind, CompiledAltPathKind::DnaDigivolve)),
        "the DNA Digivolution (purple Lv.5 + red Lv.5 / 0) path compiles"
    );
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving)
        )),
        "[When Digivolving] opponent-keeps-1 clause compiles"
    );
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenAttacking) && t.once_per_turn
        )),
        "[When Attacking][Once Per Turn] +2000/Fighter Mode clause compiles"
    );
}

// ─── [When Attacking] Fighter Mode pick: only cards that can digivolve ──────
// DCGO EX3_063.cs:227-230 names the card and passes it to
// `DigivolveIntoHandOrTrashCard(payCost: true)`, whose `CanSelectCardCondition`
// (CardEffectCommons.cs:858-880) also requires `IsDigimon`, `!CanNotEvolve(this)`
// and `CanPlayCardTargetFrame(this.PermanentFrame, ...)` (a digivolution route
// onto THIS Digimon). That frame check does not test memory when the frame
// holds a permanent (CardSource.cs:1150-1161), but "for the digivolution cost"
// must be paid: CR 4.3 1-3-11-1 (can't declare a digivolution whose cost
// can't be paid; the gauge floor is 10 on the opponent's side, 1-4-2-2) and
// 8-1-2-6 (an unpayable digivolution is returned), so an unpayable card is a
// dead choice too (1-3-2).

use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::make_test_card;
use digimon_engine::enums::{CardColor, CardKind};

/// An "Imperialdramon: Fighter Mode" Lv.7 test card with one red circle.
fn fighter_mode(id: &str, from_level: u8, cost: u16) -> CardData {
    let mut c = make_test_card(id, "Imperialdramon: Fighter Mode");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(7);
    c.dp = Some(13000);
    c.play_cost = 13;
    c.evo_costs = vec![EvoCost {
        card_color: 0,
        level: from_level,
        memory_cost: cost,
    }];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(3);
    c.dp = Some(1000);
    c
}

/// Hand candidates offered by the Fighter Mode pick after Dragon Mode attacks
/// (memory 2), or none when no pick installs.
fn fighter_mode_offered(hand: &[&str]) -> Vec<String> {
    let mut r = DebugRunner::builder()
        .dsl_card("EX3-063")
        .expect("EX3-063 YAML parses and compiles")
        .add_card(fighter_mode("FM-OK", 6, 3))
        .add_card(fighter_mode("FM-NOROUTE", 5, 3))
        .add_card(fighter_mode("FM-PRICEY", 6, 14))
        .add_card(filler("FILL"))
        .hand(0, hand)
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(2)
        .start();
    r.skip_mulligan();
    let dragon = r.place_on_field(0, "EX3-063", Some(0));
    r.attack_player(dragon, 1, false);
    for _ in 0..6 {
        let Some(v) = r.pending_selection_view() else { return Vec::new() };
        if v.prompt.contains("Fighter Mode") {
            let mut ids: Vec<String> = v
                .valid_action_ids
                .iter()
                .filter(|&&a| a != PASS)
                .filter_map(|&a| {
                    let slot = if a >= HAND_EFFECT_START { a - HAND_EFFECT_START } else { a - PLAY_HAND_START };
                    r.game.players[0].hand.get(slot as usize)
                })
                .map(|c| c.card_id(&r.game.card_data).to_string())
                .collect();
            ids.sort();
            return ids;
        }
        let a = v.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap_or(PASS);
        r.execute_action(v.selecting_player, a).expect("unrelated prompt");
    }
    Vec::new()
}

/// A Fighter Mode with no route onto this Lv.6 (its circle is Lv.5) is not
/// offered (regression guard: the compile pass adds the route check).
#[test]
fn ex3_063_fighter_mode_pick_excludes_a_card_with_no_route_onto_this_digimon() {
    assert_eq!(
        fighter_mode_offered(&["FM-OK", "FM-NOROUTE"]),
        vec!["FM-OK".to_string()]
    );
}

/// Memory 2: the cost-14 route would take the gauge to 12 on the opponent's
/// side, so that Fighter Mode can't be digivolved into and is not offered.
#[test]
fn ex3_063_fighter_mode_pick_excludes_an_unpayable_card() {
    assert_eq!(
        fighter_mode_offered(&["FM-OK", "FM-PRICEY"]),
        vec!["FM-OK".to_string()]
    );
}

/// Only an unpayable Fighter Mode in hand: nothing is offered.
#[test]
fn ex3_063_only_unpayable_fighter_mode_offers_nothing() {
    assert_eq!(fighter_mode_offered(&["FM-PRICEY"]), Vec::<String>::new());
}
