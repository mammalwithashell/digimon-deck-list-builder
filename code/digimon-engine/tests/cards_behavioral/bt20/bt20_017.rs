//! BT20-017 Jesmon.

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{decode_attack, encode_attack, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn digimon(id: &str, name: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT20-017")
        .expect("BT20-017 YAML loads")
        .memory(20)
        .start()
}

fn token_count(runner: &DebugRunner) -> usize {
    runner
        .game
        .player(0)
        .battle_area
        .iter()
        .filter(|perm| perm.top_card().card_name(&runner.game.card_data) == "Atho, René & Por")
        .count()
}

#[test]
fn bt20_017_has_printed_token_and_other_played_observer_shape() {
    let runner = runner();
    let card = runner
        .compiled_card("BT20-017")
        .expect("BT20-017 compiled card present");

    assert_eq!(card.name, "Jesmon");
    assert_eq!(card.kind, digimon_dsl::compiled::CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(11));
    assert_eq!(card.dp, Some(11000));
    assert!(card.traits.iter().any(|t| t == "Holy Warrior"));
    assert!(card.traits.iter().any(|t| t == "Royal Knight"));

    let token_clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnPlay)
                    && triggered.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("shared On Play / When Digivolving token clause exists");
    assert!(token_clause.optional, "printed token play says 'may'");
    assert!(
        token_clause.process.iter().any(|step| matches!(
            step,
            CompiledStep::PlayToken { token_name, .. } if token_name == "atho_rene_por"
        )),
        "token clause must play the registered Atho, René & Por token"
    );

    let observer = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnAllyPlayed) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("other-Digimon-play observer exists");
    assert!(observer.once_per_turn);
    assert!(observer
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::SelectOpponentPermanent { .. })));
    assert!(
        observer
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::DeletePermanent { .. })),
        "observer must delete the selected opponent Digimon"
    );
    assert!(
        observer.process.iter().any(|step| matches!(
            step,
            CompiledStep::Optional(steps)
                if steps.iter().any(|inner| matches!(inner, CompiledStep::MayAttackNow { .. }))
        )),
        "observer must offer a player-visible may-attack choice"
    );
}

#[test]
fn bt20_017_on_play_can_decline_token() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-017")
        .expect("BT20-017 YAML loads")
        .hand(0, &["BT20-017"])
        .memory(20)
        .start();

    runner.play(0, 0).expect("Jesmon plays");
    assert!(runner.pending_is_optional());
    assert_eq!(
        build_action_mask(&runner.game, 0)[PASS as usize],
        1.0,
        "optional token play must expose PASS"
    );
    runner
        .decline_optional_trigger()
        .expect("decline optional token play");

    assert_eq!(token_count(&runner), 0, "decline leaves no token in battle");
}

#[test]
fn bt20_017_on_play_may_play_atho_rene_por_token() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-017")
        .expect("BT20-017 YAML loads")
        .hand(0, &["BT20-017"])
        .memory(20)
        .start();

    runner.play(0, 0).expect("Jesmon plays");
    runner
        .accept_optional_trigger()
        .expect("accept optional token play");

    assert_eq!(token_count(&runner), 1);
}

#[test]
fn bt20_017_own_play_observer_ignores_jesmons_own_play_event() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-017")
        .expect("BT20-017 YAML loads")
        .add_card(digimon("OPP-LOW", "Opponent low DP", 7000))
        .memory(20)
        .start();
    let jesmon = runner.place_on_field(0, "BT20-017", Some(0));
    runner.place_on_field(1, "OPP-LOW", Some(0));

    let card = runner.top_card(jesmon);
    runner.game.enqueue_triggered(
        EffectTiming::OnAllyPlayed,
        TriggerSource::EnteredField {
            player: 0,
            permanent: jesmon,
            card,
            effect_initiated: false,
        },
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "event_target_is_source: false keeps Jesmon from reacting to its own play"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "self-play event must not delete an opponent Digimon"
    );
}

#[test]
fn bt20_017_other_digimon_play_deletes_low_dp_then_offers_may_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-017")
        .expect("BT20-017 YAML loads")
        .add_card(digimon("ALLY", "Other ally", 3000))
        .add_card(digimon("OPP-LOW", "Opponent low DP", 7000))
        .add_card(digimon("OPP-HIGH", "Opponent high DP", 9000))
        .hand(0, &["ALLY"])
        .security(1, &["OPP-HIGH"])
        .memory(20)
        .start();
    runner.game.turn_count = 5;
    let jesmon = runner.place_on_field(0, "BT20-017", Some(0));
    runner.place_on_field(1, "OPP-LOW", Some(0));
    runner.place_on_field(1, "OPP-HIGH", Some(0));

    runner.play(0, 0).expect("other ally plays");

    let delete_prompt = runner
        .pending_selection_view()
        .expect("delete prompt should open");
    assert_eq!(delete_prompt.kind, SelectionKind::OppField);
    assert_eq!(
        delete_prompt.valid_action_ids,
        vec![encode_attack(0, 0)],
        "only the 8000-DP-or-less opponent Digimon is legal"
    );
    runner
        .execute_action(0, encode_attack(0, 0))
        .expect("delete the low-DP opponent");
    assert_eq!(runner.battle_area_size(1), 1);
    assert_eq!(
        runner.game.player(1).battle_area[0]
            .top_card()
            .card_name(&runner.game.card_data),
        "Opponent high DP",
        "9000-DP opponent Digimon remains"
    );

    let pick_attacker = runner
        .pending_selection_view()
        .expect("own Digimon may attack selection opens");
    assert_eq!(pick_attacker.kind, SelectionKind::OwnField);
    assert!(pick_attacker.is_optional);
    assert_eq!(
        build_action_mask(&runner.game, 0)[PASS as usize],
        1.0,
        "own-attacker pick can be declined"
    );
    runner
        .execute_action(0, encode_attack(0, jesmon.index as u16))
        .expect("choose Jesmon as the Digimon that may attack");

    let attack_prompt = runner
        .pending_selection_view()
        .expect("may_attack_now opens an attack target prompt");
    assert!(
        attack_prompt
            .valid_action_ids
            .iter()
            .copied()
            .filter(|action| *action != PASS)
            .all(|action| decode_attack(action).0 as u8 == jesmon.index),
        "the attack prompt must be for the selected Jesmon"
    );
}
