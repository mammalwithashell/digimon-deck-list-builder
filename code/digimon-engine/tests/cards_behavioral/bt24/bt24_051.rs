//! BT24-051 Merukimon.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{decode_attack, encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT24-051";

#[test]
fn bt24_051_metadata_cost_reduction_triggers_and_auras_match_printed_card() {
    let runner = merukimon_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT24-051");

    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    assert!(card
        .traits
        .iter()
        .any(|trait_name| trait_name == "Olympos XII"));
    assert!(card.traits.iter().any(|trait_name| trait_name == "TS"));
    assert!(card.alt_paths.iter().any(|path| path.cost.is_some()
        && path.from.as_ref().is_some_and(|from| {
            from.all_of.iter().any(|pred| pred.level_eq == Some(5))
                && from
                    .all_of
                    .iter()
                    .flat_map(|pred| pred.any_of.iter())
                    .any(|pred| pred.trait_has.as_deref() == Some("TS"))
        })));

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
            amount: Some(5),
            ..
        })
    )));

    let suspend_clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(trigger)
                if trigger.when.contains(&CompiledTiming::OnPlay)
                    && trigger.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(trigger)
            }
            _ => None,
        })
        .expect("shared suspend/attack clause");
    assert!(suspend_clause.process.iter().any(|step| matches!(
        step,
        CompiledStep::SelectCountCappedMulti { max, .. }
            if matches!(max, digimon_dsl::compiled::CompiledCountBound::Literal(2))
    )));
    assert!(suspend_clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::Optional(_))));

    let opt = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(trigger)
                if trigger.when.contains(&CompiledTiming::WhenAttacking)
                    && trigger.when.contains(&CompiledTiming::WhenDigivolving)
                    && trigger.once_per_turn =>
            {
                Some(trigger)
            }
            _ => None,
        })
        .expect("OPT unsuspend clause");
    assert!(opt.optional);

    assert_eq!(
        card.effects
            .iter()
            .filter(|clause| matches!(
                clause,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                    grant_keyword: Some(_),
                    ..
                })
            ))
            .count(),
        2,
        "Rush and Piercing should be separate Iliad auras"
    );
}

#[test]
fn bt24_051_suspends_two_then_boosted_digimon_attacks_opponent_digimon() {
    let mut runner = merukimon_runner()
        .add_card(own_iliad("ALLY", CardColor::Blue))
        .add_card(opponent_digimon("OPP-A", 7000))
        .add_card(opponent_tamer("OPP-TAMER"))
        .add_card(opponent_digimon("DEFENDER", 3000))
        .memory(20)
        .start();
    let meruki = runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER", Some(0));
    let _defender = runner.place_on_field(1, "DEFENDER", Some(0));
    runner.game.turn_count = 1;

    fire_on_play(&mut runner, meruki);
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::OppField)
    ));
    runner
        .execute_action(0, encode_attack(0, opp_a.index as u16))
        .expect("select first suspend target");
    runner
        .execute_action(0, encode_attack(0, opp_tamer.index as u16))
        .expect("select second suspend target");
    assert!(runner.game.players[1].battle_area[opp_a.index as usize].is_suspended);
    assert!(runner.game.players[1].battle_area[opp_tamer.index as usize].is_suspended);

    let boost_prompt = runner
        .pending_selection_view()
        .expect("boost attacker selection");
    assert_eq!(boost_prompt.kind, SelectionKind::OwnField);
    assert!(boost_prompt.is_optional);
    runner
        .execute_action(
            boost_prompt.selecting_player,
            encode_attack(0, ally.index as u16),
        )
        .expect("select boosted attacker");
    assert_eq!(runner.effective_dp(ally), Some(9000));

    let attack_prompt = runner
        .pending_selection_view()
        .expect("attack target selection");
    assert_eq!(attack_prompt.kind, SelectionKind::Target);
    assert!(attack_prompt.is_optional);
    assert!(
        attack_prompt.valid_action_ids.iter().all(
            |action| decode_attack(*action).1 != digimon_engine::action::space::SECURITY_TARGET
        ),
        "Merukimon's rider may attack opponent Digimon only"
    );
    let digimon_attack = attack_prompt
        .valid_action_ids
        .iter()
        .copied()
        .find(|action| decode_attack(*action).1 != digimon_engine::action::space::SECURITY_TARGET)
        .expect("a suspended opponent Digimon should be an attack target");
    runner
        .execute_action(attack_prompt.selecting_player, digimon_attack)
        .expect("attack opposing Digimon");
}

#[test]
fn bt24_051_opt_unsuspends_one_own_digimon_and_auras_grant_rush_piercing() {
    let mut runner = merukimon_runner()
        .add_card(own_iliad("ALLY", CardColor::Blue))
        .start();
    let meruki = runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    runner.game.players[0].battle_area[ally.index as usize].is_suspended = true;
    runner.game.turn_count = 1;
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(ally, Keyword::Rush));
    assert!(runner.game.has_keyword(ally, Keyword::Piercing));

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(meruki),
    );
    runner.game.drain_effect_queue();
    if runner.pending_kind() != Some(SelectionKind::OwnField) {
        let accept_prompt = runner
            .pending_selection_view()
            .expect("optional OPT unsuspend prompt");
        let accept_action = accept_prompt
            .valid_action_ids
            .iter()
            .copied()
            .find(|action| *action != PASS)
            .expect("optional trigger should expose an accept action");
        runner
            .execute_action(accept_prompt.selecting_player, accept_action)
            .expect("accept OPT unsuspend");
    }
    if let Some(prompt) = runner.pending_selection_view() {
        assert_eq!(prompt.kind, SelectionKind::OwnField);
        assert!(prompt
            .valid_action_ids
            .contains(&encode_attack(0, ally.index as u16)));
        runner
            .execute_action(prompt.selecting_player, encode_attack(0, ally.index as u16))
            .expect("select suspended Digimon");
    }

    assert!(!runner.game.players[0].battle_area[ally.index as usize].is_suspended);
}

fn merukimon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-051 YAML loads")
}

fn fire_on_play(
    runner: &mut digimon_engine::debug_runner::DebugRunner,
    source: digimon_engine::PermanentHandle,
) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn own_iliad(id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.traits = vec!["Iliad".to_string(), "TS".to_string()];
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
    card
}

fn opponent_digimon(id: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.level = Some(4);
    card.dp = Some(dp);
    card.play_cost = 4;
    card
}

fn opponent_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Purple];
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card
}

// ─── G-ENGINE-PLAY-MASK-IGNORES-WHEN-PLAYING-REDUCTION ──────────────────────
// The RL mask must judge hand-play affordability against the cost AFTER the
// automatic `when_playing_this` reduction (declare-then-pay, rule 1-3-11-1:
// the play is declarable iff the REDUCED cost is payable within the -10 floor).

fn mask_runner(memory: i16, own_digimon: usize) -> DebugRunner {
    let mut runner = merukimon_runner()
        .add_card(own_iliad("ALLY", CardColor::Blue))
        .hand(0, &[CARD_ID])
        .memory(memory)
        .start();
    for _ in 0..own_digimon {
        runner.place_on_field(0, "ALLY", Some(0));
    }
    runner
}

fn hand_play_offered(runner: &DebugRunner) -> bool {
    let mask = digimon_engine::action::mask::build_action_mask(&runner.game, 0);
    mask[digimon_engine::action::space::PLAY_HAND_START as usize] == 1.0
}

#[test]
fn bt24_051_mask_offers_play_at_zero_memory_with_three_digimon_and_resolves_at_minus_seven() {
    let mut runner = mask_runner(0, 3);
    assert_eq!(runner.game.players[0].hand.len(), 1);
    assert!(
        hand_play_offered(&runner),
        "12 - 5 = 7 is payable from 0 memory (0 - 7 >= -10)"
    );
    runner
        .play(0, 0)
        .expect("Merukimon plays at the reduced cost");
    assert_eq!(runner.memory(), -7, "paid the reduced cost 7");
    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == CARD_ID));
}

#[test]
fn bt24_051_mask_hides_play_when_even_the_reduced_cost_overdraws() {
    // -4 - 7 = -11 < -10: unpayable even after the reduction.
    let runner = mask_runner(-4, 3);
    assert!(!hand_play_offered(&runner));
    // -3 - 7 = -10: exactly the floor — payable.
    let runner = mask_runner(-3, 3);
    assert!(hand_play_offered(&runner));
}

#[test]
fn bt24_051_mask_keeps_printed_cost_when_condition_is_false() {
    // Only 2 Digimon: no reduction, printed 12 from 0 → -12 < -10.
    let runner = mask_runner(0, 2);
    assert!(!hand_play_offered(&runner));
    // 2 memory → 2 - 12 = -10: payable at the printed cost.
    let mut runner = mask_runner(2, 2);
    assert!(hand_play_offered(&runner));
    runner.play(0, 0).expect("plays at printed cost");
    assert_eq!(runner.memory(), -10, "paid the printed 12");
}

// ─── Cost reduction counts BOTH players' Digimon ─────────────────────────────

/// "When this card would be played, if there are 3 or more Digimon, reduce the
/// play cost by 5." The count is board-wide (BT25-059 / BT25-050 / BT25-055
/// read the same wording with `owner: any`; DCGO counts both players): 1 own +
/// 2 opponent Digimon = 3 → play cost 12 - 5 = 7.
#[test]
fn bt24_051_cost_reduction_counts_opponent_digimon() {
    let mut runner = merukimon_runner()
        .add_card(own_iliad("OWN-ILIAD", CardColor::Green))
        .add_card(opponent_digimon("OPP-A", 4000))
        .add_card(opponent_digimon("OPP-B", 4000))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    runner.place_on_field(0, "OWN-ILIAD", Some(0));
    runner.place_on_field(1, "OPP-A", Some(0));
    runner.place_on_field(1, "OPP-B", Some(0));
    runner.play(0, 0).expect("play Merukimon");
    assert_eq!(
        runner.memory(),
        3,
        "1 own + 2 opponent Digimon = 3 Digimon: paid 7 (12 - 5) from 10"
    );
}

/// Negative: 2 Digimon on the board (1 own + 1 opponent) → full cost 12.
#[test]
fn bt24_051_cost_not_reduced_with_two_digimon() {
    let mut runner = merukimon_runner()
        .add_card(own_iliad("OWN-ILIAD", CardColor::Green))
        .add_card(opponent_digimon("OPP-A", 4000))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    runner.place_on_field(0, "OWN-ILIAD", Some(0));
    runner.place_on_field(1, "OPP-A", Some(0));
    runner.play(0, 0).expect("play Merukimon");
    assert_eq!(
        runner.memory(),
        -2,
        "only 2 Digimon: full cost 12 paid from 10"
    );
}
