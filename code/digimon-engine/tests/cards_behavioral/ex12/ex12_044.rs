use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

use super::support::{
    field_contains, hand_contains, plain_digimon, select_first_non_pass, select_hand_card,
    top_card_id, vb_digimon, with_evo_cost, DebugRunner,
};

const CARD_ID: &str = "EX12-044";

fn fire_when_attacking(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

#[test]
fn ex12_044_on_play_gives_one_opponent_digimon_minus_4000_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-044 YAML loads")
        .add_card(plain_digimon("OPP", CardColor::Purple, 5, 7000))
        .start();

    let angewomon = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));
    runner.fire_on_play(0, angewomon.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("Angewomon should select an opponent Digimon");
    assert_eq!(view.kind, SelectionKind::OppField);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve DP modifier");

    assert_eq!(runner.effective_dp(opp), Some(3000));
}

#[test]
fn ex12_044_when_attacking_with_same_level_source_pair_digivolves_from_hand_cost_reduced() {
    let evo = with_evo_cost(
        vb_digimon("VB-LV6", CardColor::Yellow, 6, 11000),
        CardColor::Yellow,
        5,
        3,
    );

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-044 YAML loads")
        .add_card(plain_digimon("SRC-L4A", CardColor::Yellow, 4, 4000))
        .add_card(plain_digimon("SRC-L4B", CardColor::Green, 4, 4000))
        .add_card(evo)
        .hand(0, &["VB-LV6"])
        .memory(5)
        .start();

    let angewomon = runner.place_stack(0, &["SRC-L4A", "SRC-L4B", CARD_ID]);
    let memory_before = runner.memory();

    fire_when_attacking(&mut runner, angewomon);
    let offer = runner
        .pending_selection_view()
        .expect("same-level source pair should offer hand digivolution");
    assert!(
        offer.is_optional,
        "printed 'may digivolve' must be optional"
    );
    select_first_non_pass(&mut runner);
    select_hand_card(&mut runner, 0, "VB-LV6");
    runner
        .auto_resolve()
        .expect("resolve Angewomon hand digivolution");

    assert_eq!(top_card_id(&runner, 0, angewomon.index as usize), "VB-LV6");
    assert_eq!(
        memory_before - runner.memory(),
        1,
        "cost 3 should be reduced by 2"
    );
}

#[test]
fn ex12_044_inherited_decode_grants_replacement_keyword() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-044 YAML loads")
        .add_card(vb_digimon("CARRIER", CardColor::Yellow, 6, 11000))
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-044");

    let decode_replacements = card
        .effects
        .iter()
        .filter(|clause| {
            matches!(
                clause,
                CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                    scope: CompiledScope::Inherited,
                    ..
                })
            )
        })
        .count();
    assert_eq!(
        decode_replacements, 1,
        "EX12-044 inherited Decode should compile as one inherited replacement clause"
    );
}

/// An INHERITED `<Decode>` is still SELF-scoped: it belongs to the host the
/// source card is buried under, not to every friendly permanent on the field.
///
/// `general_rule.pdf` 16-35-1 (p.39): "<Decode> is a keyword effect. When the
/// Digimon **with this effect** would leave the battle area other than by a
/// battle, you may play 1 Digimon card specified by this effect from **that
/// Digimon's** digivolution cards without paying the cost." 16-35-2 scopes the
/// trigger the same way. DCGO agrees:
/// `CardEffectFactory/KeyWordEffects/Decode.cs:51-56` gates on
/// `CanTriggerWhenRemoveField`, which
/// (`CardEffectCommons/CanUseEffects/WhenRemoveField.cs:11-14`) requires the
/// LEAVING permanent's `cardSources` to contain the Decode carrier itself --
/// for an INHERITED Decode that carrier is the source card, so the window
/// belongs to the host it is buried under and to no other permanent.
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
fn ex12_044_inherited_decode_does_not_open_a_window_for_another_permanents_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-044 YAML loads")
        .add_card(vb_digimon("VB-L4", CardColor::Yellow, 4, 4000))
        .add_card(plain_digimon("CARRIER-TOP", CardColor::Blue, 6, 11000))
        .add_card(plain_digimon("OTHER-TOP", CardColor::Blue, 6, 10000))
        .memory(10)
        .start();
    // The inherited Decode's host: EX12-044 sits UNDER "CARRIER-TOP" on slot 0,
    // and that host is NOT the permanent that leaves.
    let _carrier = runner.place_stack(0, &[CARD_ID, "CARRIER-TOP"]);
    // A SECOND P0 Digimon stacked over the exact payload EX12-044's Decode
    // names, so a cross-permanent window would have a legal candidate and
    // really park.
    let other = runner.place_stack(0, &["VB-L4", "OTHER-TOP"]);

    runner.game.return_to_hand_from_effect(other, 0);

    assert!(
        runner.game.pending_selection.is_none(),
        "16-35-1: <Decode> triggers only when the Digimon WITH THIS EFFECT \
         would leave the battle area -- the EX12-044 host is still on the field \
         and the leaving Digimon does not carry it as a source (got: {:?})",
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
        !field_contains(&runner, 0, "VB-L4"),
        "16-35-1: the Decode payload comes from THAT DIGIMON'S digivolution \
         cards -- an inherited Decode may not reach into another permanent's \
         stack"
    );
}

/// POSITIVE control -- the host carrying the inherited `<Decode>` leaving the
/// battle area other than by a battle must still open its window and play the
/// named source from its OWN digivolution cards.
///
/// `general_rule.pdf` 16-35-1 (p.39): "<Decode> is a keyword effect. When the
/// Digimon **with this effect** would leave the battle area other than by a
/// battle, you may play 1 Digimon card specified by this effect from **that
/// Digimon's** digivolution cards without paying the cost." 16-35-2 scopes the
/// trigger the same way. DCGO agrees:
/// `CardEffectFactory/KeyWordEffects/Decode.cs:51-56` gates on
/// `CanTriggerWhenRemoveField`, which
/// (`CardEffectCommons/CanUseEffects/WhenRemoveField.cs:11-14`) requires the
/// LEAVING permanent's `cardSources` to contain the Decode carrier itself --
/// for an INHERITED Decode that carrier is the source card, so the window
/// belongs to the host it is buried under and to no other permanent.
#[test]
fn ex12_044_inherited_decode_plays_matching_source_on_its_hosts_non_battle_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-044 YAML loads")
        .add_card(vb_digimon("VB-L4", CardColor::Yellow, 4, 4000))
        .add_card(plain_digimon("HOST-TOP", CardColor::Blue, 6, 11000))
        .memory(10)
        .start();
    let host = runner.place_stack(0, &["VB-L4", CARD_ID, "HOST-TOP"]);

    runner.game.return_to_hand_from_effect(host, 1);
    select_first_non_pass(&mut runner);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish Decode");

    assert!(
        field_contains(&runner, 0, "VB-L4"),
        "16-35-1: the host's own non-battle leave plays 1 named Digimon card \
         from THAT DIGIMON'S digivolution cards"
    );
    assert!(
        hand_contains(&runner, 0, "HOST-TOP"),
        "the original non-battle leave still returns the host to hand"
    );
}

/// NEGATIVE control on the other axis -- a leave BY A BATTLE opens no
/// `<Decode>` window at all.
///
/// `general_rule.pdf` 16-35-1 (p.39): "<Decode> is a keyword effect. When the
/// Digimon **with this effect** would leave the battle area other than by a
/// battle, you may play 1 Digimon card specified by this effect from **that
/// Digimon's** digivolution cards without paying the cost." 16-35-2 scopes the
/// trigger the same way. DCGO agrees:
/// `CardEffectFactory/KeyWordEffects/Decode.cs:51-56` gates on
/// `CanTriggerWhenRemoveField`, which
/// (`CardEffectCommons/CanUseEffects/WhenRemoveField.cs:11-14`) requires the
/// LEAVING permanent's `cardSources` to contain the Decode carrier itself --
/// for an INHERITED Decode that carrier is the source card, so the window
/// belongs to the host it is buried under and to no other permanent.
#[test]
fn ex12_044_inherited_decode_does_not_trigger_on_a_battle_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-044 YAML loads")
        .add_card(vb_digimon("VB-L4", CardColor::Yellow, 4, 4000))
        .add_card(plain_digimon("HOST-TOP", CardColor::Blue, 6, 11000))
        .memory(10)
        .start();
    let host = runner.place_stack(0, &["VB-L4", CARD_ID, "HOST-TOP"]);

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::Battle);

    assert!(
        runner.game.pending_selection.is_none(),
        "16-35-1: <Decode> fires only when the carrier would leave the battle \
         area OTHER THAN BY A BATTLE (got: {:?})",
        runner
            .game
            .pending_selection
            .as_ref()
            .map(|s| s.prompt.clone())
    );
    assert!(
        !field_contains(&runner, 0, "VB-L4"),
        "16-35-1: a battle leave Decodes nothing -- the source goes to trash \
         with the stack"
    );
}
