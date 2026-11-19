//! EX11-013 Sangomon — Lv.3 Blue, Mollusk / LIBERATOR / Aquatic ((Rule) trait).
//!
//! [When Moving] [On Play] If you have 7 or fewer cards in your hand, <Draw 1>.
//! Inherited: [End of Attack] [Once Per Turn] Gain 1 memory.
//!
//! DCGO: EX11/Blue/EX11_013.cs. Synthetic boards only (no recorded games).

#[path = "../bt26/support.rs"]
mod support;

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, GamePhase, PlaySource};
use support::*;

const CARD_ID: &str = "EX11-013";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-013 in embedded DSL pack")
        .add_card(filler("FILL"))
        .add_card(digimon("BLUE2", "Blue Two", CardColor::Blue, 2, 0, &[]))
        .add_card(digimon("RED2", "Red Two", CardColor::Red, 2, 0, &[]))
        .add_card(digimon("CARRIER", "Carrier", CardColor::Blue, 4, 5, &[]))
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .security(1, &["FILL"; 5])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.game.turn_player_idx = 0;
    r.game.current_phase = GamePhase::Main;
    r.game.turn_count = 3;
    r
}

#[test]
fn ex11_013_metadata_and_clause_shape() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Sangomon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(1000));
    for t in ["Mollusk", "LIBERATOR", "Aquatic"] {
        assert!(card.traits.iter().any(|x| x == t), "trait {t}");
    }
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 2);
    let draw = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("WM/OP clause");
    assert_eq!(draw.when.len(), 2);
    assert!(!draw.optional && !draw.once_per_turn);
    let inherited = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::EndOfAttack))
        .expect("inherited End of Attack");
    assert!(inherited.once_per_turn);
}

#[test]
fn ex11_013_digivolves_from_blue_lv2_for_0_only() {
    for (base_id, ok) in [("BLUE2", true), ("RED2", false)] {
        let mut r = setup();
        let base = r.place_on_field(0, base_id, Some(0));
        let idx = r.add_to_hand(0, CARD_ID);
        let done = r.game.digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByDigivolve);
        assert_eq!(done, ok, "{base_id}");
        if ok {
            assert_eq!(top_id(&r, base), CARD_ID);
            assert_eq!(r.memory(), 5, "cost 0");
        }
    }
}

#[test]
fn ex11_013_on_play_draws_one_with_seven_or_fewer_cards_in_hand() {
    for (hand, expected) in [(0usize, 1usize), (7, 8), (8, 8), (9, 9)] {
        let mut r = setup();
        for _ in 0..hand {
            push_hand(&mut r, 0, "FILL");
        }
        let h = r.place_on_field(0, CARD_ID, Some(0));
        fire(&mut r, EffectTiming::OnPlay, h);
        let _ = r.auto_resolve();
        assert_eq!(hand_ids(&r, 0).len(), hand + (expected - hand), "hand {hand}");
        assert_eq!(hand_ids(&r, 0).len(), expected, "hand {hand}");
    }
}

#[test]
fn ex11_013_when_moving_draws_one_only_for_itself() {
    // Itself moving from the breeding area to the battle area draws 1.
    let mut r = setup();
    r.place_in_breeding(0, CARD_ID);
    let before = hand_ids(&r, 0).len();
    assert!(r.move_from_breeding(0));
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0).len(), before + 1, "[When Moving] fired for itself");

    // Another Digimon moving does not trigger it.
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_in_breeding(0, "CARRIER");
    let before = hand_ids(&r, 0).len();
    assert!(r.move_from_breeding(0));
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0).len(), before, "[When Moving] is self-scoped");
}

#[test]
fn ex11_013_inherited_end_of_attack_gains_one_memory_once_per_turn() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    fire(&mut r, EffectTiming::EndOfAttack, host);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 6);
    fire(&mut r, EffectTiming::EndOfAttack, host);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 6, "[Once Per Turn]");
}

#[test]
fn ex11_013_face_up_has_no_inherited_end_of_attack() {
    let mut r = setup();
    let host = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::EndOfAttack, host);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 5, "inherited effects don't work face-up");
}
