use digimon_engine::enums::{CardColor, ModifierType};
use digimon_engine::selection::{SelectionKind, TriggerSource};
use digimon_engine::{enums::EffectTiming, permanent::PermanentHandle};

use super::support::{
    plain_digimon, push_to_trash, select_first_non_pass, tamer, vb_digimon, DebugRunner,
};

const CARD_ID: &str = "EX12-032";

#[test]
fn ex12_032_dna_materials_are_printed_level4_pairs() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-032 YAML loads")
        .start();
    let card = runner
        .game
        .card_data
        .iter()
        .find(|card| card.card_id == CARD_ID)
        .expect("EX12-032 card data registered");
    let dna = card
        .dna_costs
        .first()
        .expect("EX12-032 should expose its DNA digivolve route");

    assert_eq!(dna.requirement1.level, 4);
    assert_eq!(dna.requirement2.level, 4);
    assert_eq!(dna.memory_cost, 0);
    assert!(dna.requirement1.card_colors.contains(&CardColor::Blue));
    assert!(dna.requirement1.card_colors.contains(&CardColor::Yellow));
    assert!(dna.requirement2.card_colors.contains(&CardColor::Purple));
    assert!(dna.requirement2.card_colors.contains(&CardColor::Red));
}

fn fire_when_attacking(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

#[test]
fn ex12_032_on_play_locks_opponent_tamer_from_suspending() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-032 YAML loads")
        .add_card(tamer("OPP-TAMER", CardColor::Blue))
        .start();

    let weregarurumon = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER", Some(0));
    runner.fire_on_play(0, weregarurumon.index as usize);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve EX12-032 lock");

    assert!(
        runner
            .game
            .modifiers
            .has(opp_tamer, ModifierType::CannotSuspend),
        "chosen opponent Tamer should receive CannotSuspend"
    );
}

#[test]
fn ex12_032_when_attacking_requires_same_level_source_pair() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-032 YAML loads")
        .add_card(plain_digimon("SRC-L3", CardColor::Blue, 3, 2000))
        .add_card(plain_digimon("SRC-L4", CardColor::Blue, 4, 4000))
        .add_card(vb_digimon("TRASH-GARURUMON", CardColor::Blue, 6, 12000))
        .start();

    let weregarurumon = runner.place_stack(0, &["SRC-L3", "SRC-L4", CARD_ID]);
    push_to_trash(&mut runner, 0, "TRASH-GARURUMON");

    fire_when_attacking(&mut runner, weregarurumon);

    assert!(
        runner.pending_selection_view().is_none(),
        "no same-level source pair means the trash-digivolve prompt must not install"
    );
}

#[test]
fn ex12_032_when_attacking_prompts_with_same_level_source_pair() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-032 YAML loads")
        .add_card(plain_digimon("SRC-L4A", CardColor::Blue, 4, 4000))
        .add_card(plain_digimon("SRC-L4B", CardColor::Purple, 4, 4000))
        .add_card(vb_digimon("TRASH-GARURUMON", CardColor::Blue, 6, 12000))
        .start();

    let weregarurumon = runner.place_stack(0, &["SRC-L4A", "SRC-L4B", CARD_ID]);
    push_to_trash(&mut runner, 0, "TRASH-GARURUMON");

    fire_when_attacking(&mut runner, weregarurumon);

    let offer = runner
        .pending_selection_view()
        .expect("same-level source pair should offer trash digivolution");
    assert_eq!(offer.kind, SelectionKind::Replacement);
    assert!(
        offer.is_optional,
        "printed 'may digivolve' must be declinable"
    );
    select_first_non_pass(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("accepting the optional trigger should expose trash candidates");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert!(
        !view.valid_action_ids.is_empty(),
        "matching trash Digimon should be selectable"
    );
}
