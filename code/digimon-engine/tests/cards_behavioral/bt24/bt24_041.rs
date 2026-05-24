//! BT24-041 Minervamon.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledFormula, CompiledPerSelector, CompiledPlayerRef,
    CompiledStep, CompiledTiming, CompiledZone,
};
use digimon_engine::action::space::{encode_attack, PASS, PLAY_HAND_START};
use digimon_engine::debug_runner::{make_test_card, make_test_egg, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::selection::{SelectionKind, TriggerSource};
use digimon_engine::PermanentHandle;

const YAML: &str = include_str!("../../../cards/bt24/BT24-041.yaml");

#[test]
fn bt24_041_has_printed_metadata_alt_paths_cost_reduction_formula_and_auras() {
    let runner = minervamon_runner().start();
    let compiled = runner
        .compiled_card("BT24-041")
        .expect("BT24-041 compiled card present");

    assert_eq!(compiled.card, "BT24-041");
    assert_eq!(compiled.name, "Minervamon");
    assert_eq!(compiled.level, Some(6));
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Yellow, CompiledColor::Purple]
    );
    assert_eq!(compiled.cost, Some(12));
    assert_eq!(compiled.dp, Some(12000));
    assert_eq!(compiled.attribute.as_deref(), Some("Virus"));
    for trait_name in ["Shaman", "Olympos XII", "Iliad", "TS"] {
        assert!(
            compiled.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }

    assert!(compiled.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(4))
            && path.from.as_ref().is_some_and(|from| {
                from.level_eq == Some(5) && from.color_is == Some(CompiledColor::Yellow)
            })
    }));
    assert!(compiled.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(3))
            && path.from.as_ref().is_some_and(|from| {
                from.all_of.iter().any(|p| p.level_eq == Some(5))
                    && from.all_of.iter().any(|p| {
                        p.any_of.iter().any(|branch| {
                            matches!(
                                branch.trait_has.as_deref(),
                                Some("Beastkin" | "Dark Dragon" | "TS")
                            )
                        })
                    })
            })
    }));

    assert!(
        compiled.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                when_playing_this: true,
                amount: Some(5),
                ..
            })
        )),
        "Minervamon must reduce its own play cost by 5 with an Iliad Digimon/Tamer"
    );

    let triggered = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::OnDeletion) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("shared On Play / When Digivolving / On Deletion clause");
    assert!(triggered.process.iter().any(|step| matches!(
        step,
        CompiledStep::SelectHand { optional: true, filter, .. }
            if filter.trait_has.as_deref() == Some("Iliad")
                || filter.all_of.iter().any(|inner| inner.trait_has.as_deref() == Some("Iliad"))
    )));
    assert!(triggered
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::PlayFromHandFree { .. })));
    assert!(triggered.process.iter().any(|step| matches!(
        step,
        CompiledStep::DeDigivolve {
            amount: None,
            amount_fn: Some(CompiledFormula::BasePerDelta { per, .. }),
            stop_at_level: Some(3),
            ..
        } if matches!(
            per,
            CompiledPerSelector::FilteredCardCountInZoneScoped {
                zone: CompiledZone::BattleArea,
                of: CompiledPlayerRef::You,
                ..
            }
        )
    )));

    for keyword in ["Reboot", "Blocker"] {
        assert!(
            compiled.effects.iter().any(|clause| matches!(
                clause,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                    target,
                    grant_keyword: Some(grant),
                    ..
                }) if target.kind == Some(CompiledCardKind::Digimon)
                    && target.trait_has.as_deref() == Some("Iliad")
                    && grant.keyword == keyword
            )),
            "missing opponent-turn Iliad aura for {keyword}"
        );
    }
}

#[test]
fn bt24_041_play_cost_reduces_when_you_have_iliad_digimon_or_tamer() {
    let mut with_iliad = minervamon_runner()
        .hand(0, &["BT24-041"])
        .memory(20)
        .start();
    with_iliad.place_on_field(0, "ILIAD-TAMER", Some(0));

    let memory_before = with_iliad.memory();
    with_iliad.play(0, 0).expect("play Minervamon reduced");
    let paid_cost = memory_before - with_iliad.memory();
    assert_eq!(paid_cost, 7);

    let mut without_iliad = minervamon_runner()
        .hand(0, &["BT24-041"])
        .memory(20)
        .start();

    let memory_before = without_iliad.memory();
    without_iliad.play(0, 0).expect("play Minervamon unreduced");
    let paid_cost = memory_before - without_iliad.memory();
    assert_eq!(paid_cost, 12);
}

#[test]
fn bt24_041_optional_hand_play_filters_iliad_cost_and_then_formula_de_digivolves() {
    let mut runner = minervamon_runner()
        .hand(0, &["ILIAD-CHEAP", "ILIAD-EXPENSIVE", "PLAIN-CHEAP"])
        .memory(20)
        .start();
    let minervamon = runner.place_on_field(0, "BT24-041", Some(0));
    runner.place_on_field(0, "ALLY-DIGIMON", Some(0));
    let opponent = runner.place_stack(1, &["OPP-BASE", "OPP-MID-A", "OPP-MID-B", "OPP-TOP"]);

    fire_on_play(&mut runner, minervamon);

    let hand_prompt = runner
        .pending_selection_view()
        .expect("optional cost-5 Iliad hand play prompt");
    assert_eq!(hand_prompt.kind, SelectionKind::Hand);
    assert!(hand_prompt.is_optional);
    assert_eq!(
        hand_prompt.valid_action_ids,
        vec![PLAY_HAND_START],
        "only the cost-5 Iliad card should be selectable"
    );

    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("play the eligible Iliad card");
    assert!(
        battle_area_contains(&runner, 0, "ILIAD-CHEAP"),
        "selected Iliad card should be played for free"
    );
    assert!(
        hand_contains(&runner, "ILIAD-EXPENSIVE") && hand_contains(&runner, "PLAIN-CHEAP"),
        "expensive Iliad and cheap non-Iliad cards should remain in hand"
    );

    let target_prompt = runner
        .pending_selection_view()
        .expect("De-Digivolve target prompt follows optional hand play");
    assert_eq!(target_prompt.kind, SelectionKind::OppField);
    assert_eq!(
        target_prompt.valid_action_ids,
        vec![target_action(opponent)]
    );
    runner
        .execute_action(0, target_action(opponent))
        .expect("select opponent Digimon for De-Digivolve");

    assert_eq!(
        runner.game.players[1].battle_area[opponent.index as usize].stack_size(),
        1,
        "Minervamon plus ally plus played Iliad Digimon means De-Digivolve 3, capped by sources"
    );
}

#[test]
fn bt24_041_formula_de_digivolve_stops_at_lvl3_above_digi_egg() {
    let mut runner = minervamon_runner()
        .hand(0, &["ILIAD-CHEAP"])
        .memory(20)
        .start();
    let minervamon = runner.place_on_field(0, "BT24-041", Some(0));
    runner.place_on_field(0, "ALLY-DIGIMON", Some(0));
    let opponent = runner.place_stack(1, &["TEST-EGG", "OPP-LV3", "OPP-LV4"]);

    fire_on_play(&mut runner, minervamon);
    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("play eligible Iliad card so formula amount is 3");
    runner
        .execute_action(0, target_action(opponent))
        .expect("select opponent Digimon");

    let perm = &runner.game.players[1].battle_area[opponent.index as usize];
    assert_eq!(
        perm.stack_size(),
        2,
        "standard De-Digivolve must stop at Lv3 even when amount_fn evaluates above 1"
    );
    assert_eq!(perm.top_card().card_id(&runner.game.card_data), "OPP-LV3");
    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "TEST-EGG"),
        "Minervamon must not expose a bare Digi-Egg in battle area"
    );
}

#[test]
fn bt24_041_declining_hand_play_still_runs_de_digivolve_tail() {
    let mut runner = minervamon_runner()
        .hand(0, &["ILIAD-CHEAP"])
        .memory(20)
        .start();
    let minervamon = runner.place_on_field(0, "BT24-041", Some(0));
    let opponent = runner.place_stack(1, &["OPP-BASE", "OPP-MID-A", "OPP-TOP"]);

    fire_on_play(&mut runner, minervamon);

    let hand_prompt = runner
        .pending_selection_view()
        .expect("optional hand play prompt");
    assert!(hand_prompt.is_optional);
    runner
        .execute_action(0, PASS)
        .expect("decline optional hand play");

    let target_prompt = runner
        .pending_selection_view()
        .expect("De-Digivolve target prompt should still run after decline");
    assert_eq!(
        target_prompt.valid_action_ids,
        vec![target_action(opponent)]
    );
    runner
        .execute_action(0, target_action(opponent))
        .expect("select opponent Digimon");

    assert_eq!(
        runner.game.players[1].battle_area[opponent.index as usize].stack_size(),
        2,
        "with only Minervamon as your Digimon, the formula should De-Digivolve 1"
    );
}

#[test]
fn bt24_041_opponent_turn_aura_grants_reboot_and_blocker_to_own_iliad_digimon_only() {
    let mut runner = minervamon_runner().start();
    runner.place_on_field(0, "BT24-041", Some(0));
    let own_iliad = runner.place_on_field(0, "ALLY-ILIAD", Some(0));
    let own_plain = runner.place_on_field(0, "ALLY-DIGIMON", Some(0));
    let opponent_iliad = runner.place_on_field(1, "OPP-ILIAD", Some(0));

    runner.game.tick_declarative_effects();
    assert!(!runner.game.has_keyword(own_iliad, Keyword::Reboot));
    assert!(!runner.game.has_keyword(own_iliad, Keyword::Blocker));

    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "must be opponent's turn");
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(own_iliad, Keyword::Reboot));
    assert!(runner.game.has_keyword(own_iliad, Keyword::Blocker));
    assert!(!runner.game.has_keyword(own_plain, Keyword::Reboot));
    assert!(!runner.game.has_keyword(own_plain, Keyword::Blocker));
    assert!(!runner.game.has_keyword(opponent_iliad, Keyword::Reboot));
    assert!(!runner.game.has_keyword(opponent_iliad, Keyword::Blocker));
}

fn minervamon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-041 YAML loads")
        .add_card(make_digimon("ALLY-DIGIMON", &[]))
        .add_card(make_digimon("ALLY-ILIAD", &["Iliad"]))
        .add_card(make_tamer("ILIAD-TAMER", &["Iliad"]))
        .add_card(make_digimon_with_cost("ILIAD-CHEAP", &["Iliad"], 5))
        .add_card(make_digimon_with_cost("ILIAD-EXPENSIVE", &["Iliad"], 6))
        .add_card(make_digimon_with_cost("PLAIN-CHEAP", &[], 5))
        .add_card(make_digimon("OPP-ILIAD", &["Iliad"]))
        .add_card(make_digimon("OPP-BASE", &[]))
        .add_card(make_digimon("OPP-MID-A", &[]))
        .add_card(make_digimon("OPP-MID-B", &[]))
        .add_card(make_digimon("OPP-TOP", &[]))
        .add_card(make_test_egg("TEST-EGG", "Test Egg"))
        .add_card(make_level_digimon("OPP-LV3", 3))
        .add_card(make_level_digimon("OPP-LV4", 4))
}

fn make_digimon(id: &str, traits: &[&str]) -> digimon_engine::CardData {
    make_digimon_with_cost(id, traits, 3)
}

fn make_digimon_with_cost(id: &str, traits: &[&str], play_cost: u16) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.play_cost = play_cost;
    card.level = Some(4);
    card.dp = Some(5000);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn make_tamer(id: &str, traits: &[&str]) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::White];
    card.play_cost = 3;
    card.level = None;
    card.dp = None;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn make_level_digimon(id: &str, level: u8) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Blue];
    card.play_cost = 3;
    card.level = Some(level);
    card.dp = Some(i32::from(level) * 1000);
    card
}

fn fire_on_play(runner: &mut DebugRunner, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn target_action(handle: PermanentHandle) -> u16 {
    encode_attack(0, handle.index as u16)
}

fn battle_area_contains(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
}

fn hand_contains(runner: &DebugRunner, card_id: &str) -> bool {
    runner.game.players[0]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == card_id)
}
