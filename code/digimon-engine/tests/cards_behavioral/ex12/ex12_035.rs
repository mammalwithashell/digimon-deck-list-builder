use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost, CompiledPredicate};
use digimon_engine::action::space::PASS;
use digimon_engine::enums::CardColor;
use digimon_engine::replacement::ReplacementCause;

use super::support::{
    field_contains, hand_contains, plain_digimon, select_first_non_pass, vb_digimon, DebugRunner,
};

const CARD_ID: &str = "EX12-035";

fn pred_any<F: Fn(&CompiledPredicate) -> bool + Copy>(pred: &CompiledPredicate, f: F) -> bool {
    f(pred)
        || pred.all_of.iter().any(|child| pred_any(child, f))
        || pred.any_of.iter().any(|child| pred_any(child, f))
        || pred.none_of.iter().any(|child| pred_any(child, f))
        || pred.not.as_deref().is_some_and(|child| pred_any(child, f))
}

#[test]
fn ex12_035_has_printed_three_material_assembly_route() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-035 YAML loads")
        .start();

    let card = runner
        .compiled_card(CARD_ID)
        .expect("EX12-035 should be compiled");
    let assembly = card
        .alt_paths
        .iter()
        .find(|path| path.kind == CompiledAltPathKind::Assembly)
        .expect("EX12-035 prints Assembly -6 and must compile an assembly route");

    assert_eq!(assembly.cost, Some(CompiledCost::Literal(6)));
    assert_eq!(
        assembly.materials.len(),
        3,
        "Assembly -6 requires Lv.5 x Lv.4 x Lv.3 materials"
    );

    for (material, level) in assembly.materials.iter().zip([5, 4, 3]) {
        assert!(material.stack_under, "assembly material should stack under");
        assert!(
            pred_any(&material.filter, |pred| pred.level_eq == Some(level)),
            "assembly material should require level {level}: {:?}",
            material.filter
        );
        assert!(
            pred_any(&material.filter, |pred| {
                pred.name_contains.as_deref() == Some("Gabumon")
                    || pred.name_contains.as_deref() == Some("Garurumon")
                    || pred.trait_has.as_deref() == Some("ME")
                    || pred.trait_has.as_deref() == Some("VB")
            }),
            "assembly material should require [Gabumon]/[Garurumon]/[ME]/[VB]: {:?}",
            material.filter
        );
    }
}

#[test]
fn ex12_035_on_play_bottom_decks_opponent_with_lte_source_count() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-035 YAML loads")
        .add_card(plain_digimon("OWN-MAT1", CardColor::Blue, 3, 2000))
        .add_card(plain_digimon("OWN-MAT2", CardColor::Blue, 4, 4000))
        .add_card(plain_digimon("OPP-A1", CardColor::Blue, 3, 2000))
        .add_card(plain_digimon("OPP-A2", CardColor::Blue, 4, 4000))
        .add_card(plain_digimon("OPP-A3", CardColor::Blue, 5, 7000))
        .add_card(plain_digimon("OPP-HIGH", CardColor::Blue, 6, 10000))
        .add_card(plain_digimon("OPP-B1", CardColor::Blue, 3, 2000))
        .add_card(plain_digimon("OPP-LOW", CardColor::Blue, 4, 4000))
        .start();

    let metalgarurumon = runner.place_stack(0, &["OWN-MAT1", "OWN-MAT2", CARD_ID]);
    runner.place_stack(1, &["OPP-A1", "OPP-A2", "OPP-A3", "OPP-HIGH"]);
    runner.place_stack(1, &["OPP-B1", "OPP-LOW"]);

    runner.fire_on_play(0, metalgarurumon.index as usize);
    let view = runner
        .pending_selection_view()
        .expect("source-trash prompt should require 4 sources");
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "printed 'Trash any 4 digivolution cards' should not expose a zero-trash PASS"
    );
    for _ in 0..4 {
        select_first_non_pass(&mut runner);
    }
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve EX12-035 on-play");

    assert_eq!(
        runner.battle_area_size(1),
        1,
        "one eligible opponent Digimon should be bottom-decked after trashing 4 sources"
    );
    assert_eq!(
        runner.game.players[1].battle_area[0].card_sources.len(),
        1,
        "all selected opponent sources should be trashed before the bottom-deck selection"
    );
    let bottomed = runner.game.players[1]
        .deck
        .last()
        .expect("bottom-decked opponent Digimon")
        .card_id(&runner.game.card_data)
        .to_string();
    assert!(
        bottomed == "OPP-HIGH" || bottomed == "OPP-LOW",
        "bottom-decked card should be one of the eligible opponent Digimon, got {bottomed}"
    );
}

/// `<Decode>` is SELF-scoped: another permanent leaving is not this Digimon's
/// trigger, and its digivolution cards are not this Digimon's payload.
///
/// `general_rule.pdf` 16-35-1 (p.39): "<Decode> is a keyword effect. When the
/// Digimon **with this effect** would leave the battle area other than by a
/// battle, you may play 1 Digimon card specified by this effect from **that
/// Digimon's** digivolution cards without paying the cost." 16-35-2 scopes the
/// trigger the same way. DCGO agrees:
/// `CardEffectFactory/KeyWordEffects/Decode.cs:51-56` gates on
/// `CanTriggerWhenRemoveField`, which
/// (`CardEffectCommons/CanUseEffects/WhenRemoveField.cs:11-14`) requires the
/// LEAVING permanent's `cardSources` to contain the Decode carrier itself.
///
/// The over-offer this pins down: `lower_replacement.rs:108-125` only enforces
/// `replacement_subject_is_source` when the clause's `active_when` does NOT
/// read the replacement subject, and `predicate_reads_replacement_subject`
/// (`lower_replacement.rs:372-416`) counts `replacement_subject_is_mine` as
/// such a read. This card's Decode clause carried
/// `replacement_subject_is_mine: true`, which silently dropped the self-scope
/// check, so `replacement.rs` `collect_candidates` step (2) offered the window
/// for ANY other friendly battle-area permanent's non-battle leave.
#[test]
fn ex12_035_decode_does_not_open_a_window_for_another_permanents_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-035 YAML loads")
        .add_card(vb_digimon("VB-SRC", CardColor::Blue, 5, 7000))
        .add_card(plain_digimon("OTHER-TOP", CardColor::Blue, 6, 10000))
        .memory(10)
        .start();
    // MetalGarurumon sits on P0 slot 0 and is NOT the permanent that leaves.
    let _metalgarurumon = runner.place_on_field(0, CARD_ID, Some(0));
    // A SECOND P0 Digimon stacked over a Lv.5 [VB] source — exactly the payload
    // EX12-035's Decode names, so a cross-permanent window would have a legal
    // candidate and really park.
    let other = runner.place_stack(0, &["VB-SRC", "OTHER-TOP"]);

    runner.game.return_to_hand_from_effect(other, 0);

    assert!(
        runner.game.pending_selection.is_none(),
        "16-35-1: <Decode> triggers only when the Digimon WITH THIS EFFECT          would leave the battle area — MetalGarurumon is still on the field          and the leaving Digimon carries no Decode (got: {:?})",
        runner
            .game
            .pending_selection
            .as_ref()
            .map(|s| s.prompt.clone())
    );
    assert!(
        hand_contains(&runner, 0, "OTHER-TOP"),
        "the bounce itself must have committed"
    );
    assert!(
        !field_contains(&runner, 0, "VB-SRC"),
        "16-35-1: the Decode payload comes from THAT DIGIMON'S digivolution          cards — MetalGarurumon's Decode may not reach into another          permanent's stack"
    );
}

/// POSITIVE control — the carrier's OWN non-battle leave must still open its
/// `<Decode>` window and play the named source.
///
/// `general_rule.pdf` 16-35-1 (p.39): "<Decode> is a keyword effect. When the
/// Digimon **with this effect** would leave the battle area other than by a
/// battle, you may play 1 Digimon card specified by this effect from **that
/// Digimon's** digivolution cards without paying the cost." 16-35-2 scopes the
/// trigger the same way. DCGO agrees:
/// `CardEffectFactory/KeyWordEffects/Decode.cs:51-56` gates on
/// `CanTriggerWhenRemoveField`, which
/// (`CardEffectCommons/CanUseEffects/WhenRemoveField.cs:11-14`) requires the
/// LEAVING permanent's `cardSources` to contain the Decode carrier itself.
#[test]
fn ex12_035_decode_plays_matching_source_on_its_own_non_battle_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-035 YAML loads")
        .add_card(vb_digimon("VB-SRC", CardColor::Blue, 5, 7000))
        .memory(10)
        .start();
    let metalgarurumon = runner.place_stack(0, &["VB-SRC", CARD_ID]);

    runner.game.return_to_hand_from_effect(metalgarurumon, 1);
    select_first_non_pass(&mut runner);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish Decode");

    assert!(
        field_contains(&runner, 0, "VB-SRC"),
        "16-35-1: the carrier's own non-battle leave plays 1 named Digimon          card from THAT DIGIMON'S digivolution cards"
    );
    assert!(
        hand_contains(&runner, 0, CARD_ID),
        "the original non-battle leave still returns MetalGarurumon to hand"
    );
}

/// NEGATIVE control on the other axis — a leave BY A BATTLE opens no `<Decode>`
/// window at all.
///
/// `general_rule.pdf` 16-35-1 (p.39): "<Decode> is a keyword effect. When the
/// Digimon **with this effect** would leave the battle area other than by a
/// battle, you may play 1 Digimon card specified by this effect from **that
/// Digimon's** digivolution cards without paying the cost." 16-35-2 scopes the
/// trigger the same way. DCGO agrees:
/// `CardEffectFactory/KeyWordEffects/Decode.cs:51-56` gates on
/// `CanTriggerWhenRemoveField`, which
/// (`CardEffectCommons/CanUseEffects/WhenRemoveField.cs:11-14`) requires the
/// LEAVING permanent's `cardSources` to contain the Decode carrier itself.
#[test]
fn ex12_035_decode_does_not_trigger_on_a_battle_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-035 YAML loads")
        .add_card(vb_digimon("VB-SRC", CardColor::Blue, 5, 7000))
        .memory(10)
        .start();
    let metalgarurumon = runner.place_stack(0, &["VB-SRC", CARD_ID]);
    // Suspend the carrier so its printed `<Evade>` (a `WhenWouldBeDeleted`
    // replacement gated on being unsuspended) cannot park a window of its own
    // and mask what this test measures. `<Barrier>` is not printed here and
    // this runner deals no security anyway.
    runner.game.players[0].battle_area[metalgarurumon.index as usize].is_suspended = true;

    runner
        .game
        .delete_permanent_with_cause(metalgarurumon, ReplacementCause::Battle);

    assert!(
        runner.game.pending_selection.is_none(),
        "16-35-1: <Decode> fires only when the carrier would leave the battle          area OTHER THAN BY A BATTLE (got: {:?})",
        runner
            .game
            .pending_selection
            .as_ref()
            .map(|s| s.prompt.clone())
    );
    assert!(
        !field_contains(&runner, 0, "VB-SRC"),
        "16-35-1: a battle leave Decodes nothing — the source goes to trash          with the stack"
    );
}
