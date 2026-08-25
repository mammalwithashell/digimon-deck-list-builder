use digimon_engine::enums::{CardColor, Keyword, ModifierType};
use digimon_engine::replacement::ReplacementCause;

use super::support::{
    bottom_source_id, field_contains, hand_contains, hand_index, is_suspended, plain_digimon,
    push_to_trash, select_first_non_pass, select_hand_card, tb_digimon, DebugRunner,
};

const CARD_ID: &str = "EX12-036";

#[test]
fn ex12_036_has_barrier_and_evade() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-036 YAML loads")
        .start();
    let ryugumon = runner.place_on_field(0, CARD_ID, Some(0));

    assert!(runner.game.has_keyword(ryugumon, Keyword::Barrier));
    assert!(runner.game.has_keyword(ryugumon, Keyword::Evade));
}

#[test]
fn ex12_036_places_matching_hand_card_as_bottom_source_then_unsuspends_ally() {
    let aqua = super::support::digimon("AQUATIC-HAND", CardColor::Blue, 4, 4, 5000, &["Aquatic"]);
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-036 YAML loads")
        .add_card(aqua)
        .add_card(plain_digimon("ALLY", CardColor::Blue, 4, 5000))
        .hand(0, &[CARD_ID, "AQUATIC-HAND"])
        .memory(12)
        .start();
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    runner.game.players[0].battle_area[ally.index as usize].is_suspended = true;

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-036");
    select_hand_card(&mut runner, 0, "AQUATIC-HAND");
    assert_eq!(bottom_source_id(&runner, 0, 1), "AQUATIC-HAND");
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve unsuspend");

    assert!(!is_suspended(&runner, 0, ally.index as usize));
}

#[test]
fn ex12_036_ally_played_locks_opponent_when_digivolving_and_suspend() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-036 YAML loads")
        .add_card(tb_digimon("ALLY", CardColor::Blue, 4, 5000))
        .add_card(plain_digimon("OPP", CardColor::Blue, 4, 5000))
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.fire_play_event_triggers(ally.player, ally.index as usize, true, false);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve locks");

    assert!(runner
        .game
        .modifiers
        .has(opp, ModifierType::CannotActivateWhenDigivolvingEffects));
    assert!(runner.game.modifiers.has(opp, ModifierType::CannotSuspend));
}

#[test]
fn ex12_036_ally_played_observer_does_not_fire_from_trash() {
    // Regression for the EX12-063 exam phantom (qa/dcgo-exams/EX12/
    // EX12-063-inherited0.yaml): a DEAD EX12-036 sitting in the trash must
    // NOT enqueue its "[All Turns][Once Per Turn] when any of your Digimon
    // are played..." observer. The printed clause carries no [Trash] scope
    // (rule 15-14-3: only effects with the {Trash} icon activate from the
    // trash), and DCGO does not fire dead cards' observers. Before the fix,
    // the EnteredField
    // dispatch's trash scan enqueued this clause from the trash, parking a
    // phantom selection / TriggerOrder entry.
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-036 YAML loads")
        .add_card(tb_digimon("ALLY", CardColor::Blue, 4, 5000))
        .add_card(plain_digimon("OPP", CardColor::Blue, 4, 5000))
        .start();
    push_to_trash(&mut runner, 0, CARD_ID);
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.fire_play_event_triggers(ally.player, ally.index as usize, true, false);

    assert!(
        runner.game.pending_selection.is_none(),
        "dead EX12-036 in trash must not fire its on-ally-played observer \
         (got pending selection: {:?})",
        runner.game.pending_selection.as_ref().map(|s| &s.kind)
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(opp, ModifierType::CannotActivateWhenDigivolvingEffects),
        "phantom trash observer must not have locked the opponent Digimon"
    );
    assert!(!runner.game.modifiers.has(opp, ModifierType::CannotSuspend));
}

/// TDD RED — `<Decode>` is SELF-scoped on BOTH axes.
///
/// `general_rule.pdf` 16-35-1 (p.39): "<Decode> is a keyword effect. When the
/// Digimon **with this effect** would leave the battle area other than by a
/// battle, you may play 1 Digimon card specified by this effect from **that
/// Digimon's** digivolution cards without paying the cost." 16-35-2 repeats
/// the scoping for the trigger: it "triggers when the Digimon with this effect
/// would leave the battle area other than by a battle."
///
/// DCGO agrees: `CardEffectFactory/KeyWordEffects/Decode.cs:51-56` gates
/// `CanUseCondition` on `CardEffectCommons.CanTriggerWhenRemoveField(hashtable,
/// card)`, and `CardEffectCommons/CanUseEffects/WhenRemoveField.cs:11-14`
/// resolves that to `permanent.cardSources.Contains(card)` — the LEAVING
/// permanent must physically contain the Decode carrier.
///
/// EX12-036 authored its Decode clause's `active_when` as
/// `all_of: [replacement_subject_is_mine: true, none_of: [replacement_cause:
/// battle]]`. `lower_replacement.rs:108-125` only enforces
/// `replacement_subject_is_source` when the clause's `active_when` does NOT
/// read the replacement subject, and `predicate_reads_replacement_subject`
/// (`lower_replacement.rs:372-416`) counts `replacement_subject_is_mine` as
/// such a read — so the self-scope check is silently dropped and
/// `replacement.rs` `collect_candidates` step (2) offers the window for ANY
/// other battle-area permanent's non-battle leave.
#[test]
fn ex12_036_decode_does_not_open_a_window_for_another_permanents_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-036 YAML loads")
        .add_card(tb_digimon("TB-SRC", CardColor::Blue, 4, 5000))
        .add_card(plain_digimon("OTHER-TOP", CardColor::Blue, 5, 7000))
        .memory(10)
        .start();
    // Ryugumon sits on P0 slot 0 and is NOT the permanent that leaves.
    let _ryugumon = runner.place_on_field(0, CARD_ID, Some(0));
    // A SECOND P0 Digimon stacked over a Lv.4 [TB] source — exactly the payload
    // EX12-036's Decode names, so a (wrongly) cross-permanent window would have
    // a legal candidate and really park.
    let other = runner.place_stack(0, &["TB-SRC", "OTHER-TOP"]);

    // Non-battle leave of the OTHER permanent.
    runner.game.return_to_hand_from_effect(other, 0);

    assert!(
        runner.game.pending_selection.is_none(),
        "16-35-1: <Decode> triggers only when the Digimon WITH THIS EFFECT          would leave the battle area — Ryugumon is still on the field and the          leaving Digimon carries no Decode (got: {:?})",
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
        !field_contains(&runner, 0, "TB-SRC"),
        "16-35-1: the Decode payload comes from THAT DIGIMON'S digivolution          cards — Ryugumon's Decode may not reach into another permanent's stack"
    );
}

/// POSITIVE control for the self-scope test above — the carrier's OWN
/// non-battle leave must still open its `<Decode>` window and play the named
/// source.
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
fn ex12_036_decode_plays_matching_source_on_its_own_non_battle_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-036 YAML loads")
        .add_card(tb_digimon("TB-SRC", CardColor::Blue, 4, 5000))
        .memory(10)
        .start();
    let ryugumon = runner.place_stack(0, &["TB-SRC", CARD_ID]);

    // Non-battle leave of the carrier itself, caused by the opponent's effect.
    runner.game.return_to_hand_from_effect(ryugumon, 1);
    select_first_non_pass(&mut runner);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish Decode");

    assert!(
        field_contains(&runner, 0, "TB-SRC"),
        "16-35-1: the carrier's own non-battle leave plays 1 named Digimon          card from THAT DIGIMON'S digivolution cards"
    );
    assert!(
        hand_contains(&runner, 0, CARD_ID),
        "the original non-battle leave still returns Ryugumon to hand"
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
fn ex12_036_decode_does_not_trigger_on_a_battle_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-036 YAML loads")
        .add_card(tb_digimon("TB-SRC", CardColor::Blue, 4, 5000))
        .memory(10)
        .start();
    let ryugumon = runner.place_stack(0, &["TB-SRC", CARD_ID]);
    // Suspend the carrier so its granted `<Evade>` (a `WhenWouldBeDeleted`
    // replacement gated on being unsuspended) cannot park a window of its own
    // and mask what this test measures. `<Barrier>` is already inert: it needs
    // a non-empty security stack and this runner deals none.
    runner.game.players[0].battle_area[ryugumon.index as usize].is_suspended = true;

    runner
        .game
        .delete_permanent_with_cause(ryugumon, ReplacementCause::Battle);

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
        !field_contains(&runner, 0, "TB-SRC"),
        "16-35-1: a battle leave Decodes nothing — the source goes to trash          with the stack"
    );
}
