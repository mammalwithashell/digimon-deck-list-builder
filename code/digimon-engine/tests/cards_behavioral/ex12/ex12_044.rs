use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_END, PLAY_HAND_START};
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

/// Walk the prompts (accepting each non-hand prompt with its first non-PASS
/// action) to the [When Attacking] hand pick and return the card ids it
/// offers. Empty when no hand pick installs.
fn offered_hand_ids(runner: &mut DebugRunner, player: u8) -> Vec<String> {
    for _ in 0..8 {
        let Some(view) = runner.pending_selection_view() else {
            return Vec::new();
        };
        if view.kind == SelectionKind::Hand {
            let hand: Vec<String> = runner.game.players[player as usize]
                .hand
                .iter()
                .map(|c| c.card_id(&runner.game.card_data).to_string())
                .collect();
            let mut ids: Vec<String> = view
                .valid_action_ids
                .iter()
                .copied()
                .filter(|&a| a != PASS)
                .map(|a| {
                    let slot = if a < PLAY_HAND_END { a - PLAY_HAND_START } else { a - HAND_EFFECT_START };
                    hand[slot as usize].clone()
                })
                .collect();
            ids.sort();
            return ids;
        }
        let Some(action) = view.valid_action_ids.iter().copied().find(|&a| a != PASS) else {
            return Vec::new();
        };
        runner.execute_action(view.selecting_player, action).expect("advance prompt");
    }
    panic!("never reached the hand pick");
}

/// Angewomon (EX12-044) stacked on two Lv.4 sources (the 2+ same-level gate),
/// `memory` memory, with `hand` in hand.
fn angewomon_runner(extra: Vec<digimon_engine::card_data::CardData>, hand: &[&str], memory: i16) -> (DebugRunner, PermanentHandle) {
    let mut builder = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-044 YAML loads")
        .add_card(plain_digimon("SRC-L4A", CardColor::Yellow, 4, 4000))
        .add_card(plain_digimon("SRC-L4B", CardColor::Green, 4, 4000));
    for card in extra {
        builder = builder.add_card(card);
    }
    let mut runner = builder.hand(0, hand).memory(memory).start();
    let angewomon = runner.place_stack(0, &["SRC-L4A", "SRC-L4B", CARD_ID]);
    (runner, angewomon)
}

// ── [When Attacking] hand-pick candidate legality ────────────────────────────
// DCGO EX12_044.cs:188-195 runs `DigivolveIntoHandOrTrashCard(payCost: true,
// reduceCostTuple: (2, null), isHand: true)`, whose `CanSelectCardCondition`
// (CardEffectCommons.cs:858-881) offers only a Digimon that
// `CanPlayCardTargetFrame(targetPermanent.PermanentFrame, PayCost: true, ...)`
// onto THIS Digimon with the reduction applied: a real digivolve route AND a
// payable cost. A card the digivolve would refuse is not a legal choice
// (CR 4.3 1-3-2; 8-1-3-2 pays the digivolution cost, which must be payable).

/// An [VB] Lv.6 whose only route costs 15 (13 after the -2) with 0 memory can't
/// be paid (memory would fall below -10): not offered; the payable one is.
#[test]
fn ex12_044_when_attacking_hand_pick_excludes_unaffordable_digivolve() {
    let cheap = with_evo_cost(vb_digimon("VB-LV6", CardColor::Yellow, 6, 11000), CardColor::Yellow, 5, 3);
    let pricey = with_evo_cost(vb_digimon("VB-PRICEY", CardColor::Yellow, 6, 11000), CardColor::Yellow, 5, 15);
    let (mut runner, angewomon) = angewomon_runner(vec![cheap, pricey], &["VB-LV6", "VB-PRICEY"], 0);
    fire_when_attacking(&mut runner, angewomon);
    assert_eq!(offered_hand_ids(&mut runner, 0), vec!["VB-LV6".to_string()]);
}

/// A [VB] Lv.6 with no digivolution route onto the Yellow Lv.5 Angewomon (its
/// only circle is from Purple) is not offered. Regression guard: the compile
/// pass adds the route check to requirement-honouring effect digivolves.
#[test]
fn ex12_044_when_attacking_hand_pick_excludes_card_with_no_route_onto_angewomon() {
    let cheap = with_evo_cost(vb_digimon("VB-LV6", CardColor::Yellow, 6, 11000), CardColor::Yellow, 5, 3);
    let no_route = with_evo_cost(vb_digimon("VB-PURPLE", CardColor::Purple, 6, 11000), CardColor::Purple, 5, 3);
    let (mut runner, angewomon) = angewomon_runner(vec![cheap, no_route], &["VB-LV6", "VB-PURPLE"], 5);
    fire_when_attacking(&mut runner, angewomon);
    assert_eq!(offered_hand_ids(&mut runner, 0), vec!["VB-LV6".to_string()]);
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
/// `general_rule.pdf` 16-36-1 (p.41): "<Decode> is a keyword effect. When the
/// Digimon **with this effect** would leave the battle area other than by a
/// battle, you may play 1 Digimon card specified by this effect from **that
/// Digimon's** digivolution cards without paying the cost." 16-36-2 scopes the
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
        "16-36-1: <Decode> triggers only when the Digimon WITH THIS EFFECT \
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
        "16-36-1: the Decode payload comes from THAT DIGIMON'S digivolution \
         cards -- an inherited Decode may not reach into another permanent's \
         stack"
    );
}

/// POSITIVE control -- the host carrying the inherited `<Decode>` leaving the
/// battle area other than by a battle must still open its window and play the
/// named source from its OWN digivolution cards.
///
/// `general_rule.pdf` 16-36-1 (p.41): "<Decode> is a keyword effect. When the
/// Digimon **with this effect** would leave the battle area other than by a
/// battle, you may play 1 Digimon card specified by this effect from **that
/// Digimon's** digivolution cards without paying the cost." 16-36-2 scopes the
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
        "16-36-1: the host's own non-battle leave plays 1 named Digimon card \
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
/// `general_rule.pdf` 16-36-1 (p.41): "<Decode> is a keyword effect. When the
/// Digimon **with this effect** would leave the battle area other than by a
/// battle, you may play 1 Digimon card specified by this effect from **that
/// Digimon's** digivolution cards without paying the cost." 16-36-2 scopes the
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
        "16-36-1: <Decode> fires only when the carrier would leave the battle \
         area OTHER THAN BY A BATTLE (got: {:?})",
        runner
            .game
            .pending_selection
            .as_ref()
            .map(|s| s.prompt.clone())
    );
    assert!(
        !field_contains(&runner, 0, "VB-L4"),
        "16-36-1: a battle leave Decodes nothing -- the source goes to trash \
         with the stack"
    );
}
