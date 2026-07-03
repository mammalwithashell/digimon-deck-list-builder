//! LM-056 Image Training - Option, Cost 2, Purple/Blue.
//!
//! # Card text (per-card JSON: code/digimon-engine/cards/lm/LM-056.json)
//!
//! While you don't have [Image Training] in the battle area, you can ignore
//! this card's color requirements.
//!
//! [Main] Reveal the top 2 cards of your deck. Add 1 blue or purple card among
//! them to the hand. Return the rest to the bottom of deck. Then, place this
//! card in the battle area.
//!
//! [Main] <Delay> (By trashing this card after the placing turn, activate the
//! effect below.) 1 of your Digimon may digivolve into a blue or purple
//! Digimon card in your hand for its digivolution cost. When it would
//! digivolve by this effect, reduce the cost by 2.
//!
//! Inherited: Security Effect [Security] Reveal the top 2 cards of your deck.
//! Add 1 blue or purple card among them to the hand. Return the rest to the
//! bottom of deck. Then, place this card in the battle area.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/LM/Purple/LM_056.cs (base repo — resolved via
//! BASE_DCGO per rule 29). `ActivateClass` (OptionSkill) for the reveal/add/
//! bottom Main body + `PlaceDelayOptionCards`; `ActivateClass` (OnDeclaration)
//! for the standard `<Delay>` body — `DeletePeremanentAndProcessAccordingToResult`
//! trashes the card first, then `SelectPermanentEffect` (canNoSelect: true) then
//! `DigivolveIntoHandOrTrashCard` with `reduceCostTuple: (2, null)`. Security
//! mirrors Main via `AddActivateMainOptionSecurityEffect`. `IgnoreColorConditionClass`
//! gated on "no [Image Training] in the battle area" (`EffectTiming.None`).
//!
//! # Patterns this test covers (mirrors LM-054 Treadmill Training exactly,
//! purple/blue instead of yellow/black — same YAML idiom per the newer
//! `use_requirement` + `trigger: delayed` pattern, PUPPETS-G009)
//! - D3 conditional Option color-requirement bypass
//! - A2 reveal 2, add 1 matching card, bottom the rest
//! - Standard Delay body with optional own-Digimon selection
//! - Effect-initiated digivolve with cost reduction
//! - Inherited security mirror plus battle-area placement
//!
//! # Known gap
//! Standard Delay main-phase activation action is still partial: the engine can
//! park and automatically fire scheduled Delay effects, but it does not expose
//! standard later-main-phase Delay activation through the action mask yet.
//! (Same known-gap note as LM-054; not LM-056-specific.)

use std::path::Path;

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledPredicate,
    CompiledScope, CompiledStackPosition, CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS, PLAY_HAND_START,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::{OptionPlayResult, SelectionKind, TriggerSource};

fn lm_056_yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/lm/LM-056.yaml"))
        .expect("LM-056 YAML must exist at cards/lm/LM-056.yaml")
}

fn lm_056_runner() -> DebugRunner {
    let yaml = lm_056_yaml();
    DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-056 YAML must parse and compile")
        .memory(10)
        .start()
}

fn card(id: &str, kind: CardKind, colors: Vec<CardColor>) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = kind;
    card.colors = colors;
    card
}

fn digimon(id: &str, color: CardColor, level: u8) -> CardData {
    let mut card = card(id, CardKind::Digimon, vec![color]);
    card.level = Some(level);
    card.dp = Some(3000);
    card.play_cost = 4;
    card
}

fn tamer(id: &str, color: CardColor) -> CardData {
    card(id, CardKind::Tamer, vec![color])
}

fn option(id: &str, color: CardColor) -> CardData {
    card(id, CardKind::Option, vec![color])
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn lv4_evo(id: &str, color: CardColor, base_color: CardColor, cost: u16) -> CardData {
    let mut card = digimon(id, color, 4);
    card.evo_costs = vec![EvoCost {
        card_color: match base_color {
            CardColor::Red => 0,
            CardColor::Blue => 1,
            CardColor::Yellow => 2,
            CardColor::Green => 3,
            CardColor::White => 4,
            CardColor::Black => 5,
            CardColor::Purple => 6,
        },
        level: 3,
        memory_cost: cost,
    }];
    card
}

fn predicate_allows_color(predicate: &CompiledPredicate, color: CompiledColor) -> bool {
    predicate.color_is == Some(color)
        || predicate
            .any_of
            .iter()
            .any(|child| predicate_allows_color(child, color))
        || predicate
            .all_of
            .iter()
            .any(|child| predicate_allows_color(child, color))
}

#[test]
fn lm_056_yaml_parses_without_error() {
    let _runner = lm_056_runner();
}

#[test]
fn lm_056_is_purple_blue_option_cost_2() {
    let runner = lm_056_runner();
    let compiled = runner
        .compiled_card("LM-056")
        .expect("LM-056 must be registered");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Purple, CompiledColor::Blue]
    );
    assert_eq!(compiled.cost, Some(2));
}

#[test]
fn lm_056_color_bypass_is_conditioned_on_no_image_training_in_battle_area() {
    let runner = lm_056_runner();
    let compiled = runner
        .compiled_card("LM-056")
        .expect("LM-056 must be registered");

    let use_requirement = compiled
        .use_requirement
        .as_ref()
        .expect("no-Image-Training color bypass must compile as use_requirement");
    let no_image_training = use_requirement
        .no_permanent
        .as_ref()
        .expect("color bypass must be disabled by your battle-area Image Training");

    assert_eq!(
        no_image_training.of,
        digimon_dsl::compiled::CompiledPlayerRef::You
    );
    assert_eq!(
        no_image_training.predicate.card_number_is.as_deref(),
        Some("LM-056"),
        "bypass must check for the same named card in your battle area"
    );
}

#[test]
fn lm_056_action_mask_allows_color_bypass_only_without_battle_area_copy() {
    let yaml = lm_056_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-056 YAML parses")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(filler("FILL"))
        .hand(0, &["LM-056"])
        .deck(0, &["FILL"; 4])
        .memory(10)
        .start();
    runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        1.0,
        "LM-056 should be usable despite no purple/blue permanent when no LM-056 is in battle area"
    );

    runner.place_on_field(0, "LM-056", Some(0));
    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        0.0,
        "an existing battle-area LM-056 must disable the color-requirement bypass"
    );
}

#[test]
fn lm_056_has_main_delay_and_inherited_security_clauses() {
    let runner = lm_056_runner();
    let compiled = runner
        .compiled_card("LM-056")
        .expect("LM-056 must be registered");

    assert_eq!(
        compiled.effects.len(),
        3,
        "LM-056 has Main, Delay, and inherited Security effects"
    );

    match &compiled.effects[0] {
        CompiledClause::Triggered(triggered) => {
            assert!(triggered.when.contains(&CompiledTiming::MainFromHand));
            assert!(
                !triggered.optional,
                "the reveal/add Main effect is mandatory"
            );
        }
        other => panic!("clause 0 must be main_from_hand; got {other:?}"),
    }

    match &compiled.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
            trigger, process, ..
        }) => {
            assert_eq!(
                *trigger,
                CompiledTiming::Delayed,
                "standard <Delay> compiles to the player-activated Delayed timing (PUPPETS-G009)"
            );
            assert!(
                process
                    .iter()
                    .any(|step| matches!(step, CompiledStep::EffectInitiatedDigivolve { .. })),
                "Delay body must perform effect-initiated digivolution"
            );
        }
        other => panic!("clause 1 must be a Delay declarative; got {other:?}"),
    }

    match &compiled.effects[2] {
        CompiledClause::Triggered(triggered) => {
            assert_eq!(triggered.scope, CompiledScope::Inherited);
            assert!(triggered.when.contains(&CompiledTiming::OnSecurity));
            assert!(triggered
                .process
                .iter()
                .any(|step| matches!(step, CompiledStep::RevealTopDeck { count: 2, .. })));
            assert!(triggered
                .process
                .iter()
                .any(|step| matches!(step, CompiledStep::SelectReveal { .. })));
            assert!(triggered.process.iter().any(|step| matches!(
                step,
                CompiledStep::PlaceRemainderOnDeck {
                    position: CompiledStackPosition::Bottom,
                    ..
                }
            )));
            assert!(triggered
                .process
                .iter()
                .any(|step| matches!(step, CompiledStep::PlaceSelfAsDelayOption)));
        }
        other => panic!("clause 2 must be inherited on_security; got {other:?}"),
    }
}

#[test]
fn lm_056_main_reveals_top_2_and_selects_purple_or_blue_card() {
    let runner = lm_056_runner();
    let compiled = runner
        .compiled_card("LM-056")
        .expect("LM-056 must be registered");
    let main = match &compiled.effects[0] {
        CompiledClause::Triggered(triggered) => triggered,
        other => panic!("clause 0 must be triggered; got {other:?}"),
    };

    assert!(main
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::RevealTopDeck { count: 2, .. })));

    let select_filter = main
        .process
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectReveal {
                filter, optional, ..
            } => {
                assert!(
                    !*optional,
                    "PASS must not be legal when a purple/blue revealed card exists"
                );
                Some(filter)
            }
            _ => None,
        })
        .expect("Main must select from revealed cards");

    assert_eq!(
        select_filter.kind, None,
        "LM-056 adds a purple or blue card, not only a Digimon card"
    );
    assert!(predicate_allows_color(select_filter, CompiledColor::Purple));
    assert!(predicate_allows_color(select_filter, CompiledColor::Blue));
    assert!(main
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::AddToHandFromReveal { .. })));
    assert!(main.process.iter().any(|step| matches!(
        step,
        CompiledStep::PlaceRemainderOnDeck {
            position: CompiledStackPosition::Bottom,
            ..
        }
    )));
}

#[test]
fn lm_056_main_selection_filters_to_purple_or_blue_and_pass_is_not_legal() {
    let yaml = lm_056_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-056 YAML parses")
        .add_card(option("PURPLE-OPT", CardColor::Purple))
        .add_card(tamer("BLUE-TAMER", CardColor::Blue))
        .add_card(digimon("RED-DIGI", CardColor::Red, 3))
        .add_card(filler("FILL"))
        .hand(0, &["LM-056"])
        .deck(0, &["FILL", "FILL", "RED-DIGI", "PURPLE-OPT"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();

    assert!(runner.game.activate_hand_main(0, 0));
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Reveal));
    let reveal = runner.pending_selection_view().unwrap();
    assert_eq!(
        reveal.valid_action_ids.len(),
        1,
        "only the purple card among the top 2 should be selectable"
    );
    assert!(
        !reveal.is_optional,
        "PASS/decline must not be legal when an eligible card is revealed"
    );
    assert_eq!(
        build_action_mask(&runner.game, 0)[PASS as usize],
        0.0,
        "action mask must not expose PASS for the mandatory reveal pick"
    );

    runner
        .execute_action(0, reveal.valid_action_ids[0])
        .expect("purple reveal choice resolves");
    let _ = runner.auto_resolve();

    assert!(runner
        .game
        .player(0)
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "PURPLE-OPT"));
    assert!(!runner
        .game
        .player(0)
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "RED-DIGI"));
}

#[test]
fn lm_056_playing_main_places_it_as_delayed_option() {
    let yaml = lm_056_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-056 YAML parses")
        .add_card(option("PURPLE-OPT", CardColor::Purple))
        .add_card(filler("FILL"))
        .hand(0, &["LM-056"])
        .deck(0, &["FILL", "PURPLE-OPT"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner
        .auto_resolve()
        .expect("resolve LM-056 placement as a delayed Option");

    assert!(runner.game.player(0).battle_area.iter().any(|permanent| {
        permanent.top_card().card_id(&runner.game.card_data) == "LM-056"
            && matches!(
                permanent.option_state,
                OptionState::Delayed {
                    trigger: DelayTrigger::MainPhaseActivated,
                    ..
                }
            )
    }));
}

#[test]
fn lm_056_security_reveals_adds_matching_card_and_places_self_as_delay_option() {
    let yaml = lm_056_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-056 YAML parses")
        .add_card(digimon("ATTACKER", CardColor::Purple, 3))
        .add_card(option("PURPLE-OPT", CardColor::Purple))
        .add_card(digimon("RED-DIGI", CardColor::Red, 3))
        .add_card(filler("FILL"))
        .security(1, &["LM-056"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL", "RED-DIGI", "PURPLE-OPT"])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let result = runner.attack_player(attacker, 1, false);
    assert_eq!(
        result,
        AttackResult::InProgress,
        "security combat pauses while the mandatory reveal choice is pending"
    );
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Reveal));
    let reveal = runner
        .pending_selection_view()
        .expect("security reveal selection should be pending");
    assert_eq!(
        reveal.valid_action_ids.len(),
        1,
        "only the purple card among the revealed pair should be selectable"
    );
    runner
        .execute_action(reveal.selecting_player, reveal.valid_action_ids[0])
        .expect("choose the purple security reveal card");
    runner
        .auto_resolve()
        .expect("resolve bottom placement and security option placement");

    assert!(runner
        .game
        .player(1)
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "PURPLE-OPT"));
    assert_eq!(runner.security_count(1), 0, "LM-056 should leave security");
    assert_eq!(
        runner.trash_size(1),
        0,
        "LM-056 should be placed in battle area instead of trashed"
    );
    let placed = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "LM-056")
        .expect("LM-056 should be placed as a battle-area delayed Option");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
}

#[test]
fn lm_056_delay_body_selects_target_then_purple_or_blue_evolution_card() {
    let yaml = lm_056_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-056 YAML parses")
        .add_card(digimon("BASE", CardColor::Purple, 3))
        .add_card(lv4_evo(
            "PURPLE-EVO",
            CardColor::Purple,
            CardColor::Purple,
            3,
        ))
        .add_card(lv4_evo("RED-EVO", CardColor::Red, CardColor::Purple, 3))
        .hand(0, &["PURPLE-EVO", "RED-EVO"])
        .memory(5)
        .start();
    let delay_perm = runner.place_on_field(0, "LM-056", Some(0));
    runner.place_on_field(0, "BASE", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::DelayEffect,
        TriggerSource::Permanent(delay_perm),
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let target_pick = runner.pending_selection_view().unwrap();
    assert!(
        target_pick.is_optional,
        "Delay digivolve is optional at the target selection"
    );
    assert_eq!(
        build_action_mask(&runner.game, 0)[PASS as usize],
        1.0,
        "optional Delay target selection must expose PASS"
    );
    assert_eq!(
        target_pick.valid_action_ids.len(),
        1,
        "the Option permanent itself is not a Digimon target"
    );

    runner
        .execute_action(0, target_pick.valid_action_ids[0])
        .expect("target selection resolves");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    let evo_pick = runner.pending_selection_view().unwrap();
    assert!(
        !evo_pick.is_optional,
        "after choosing a target, choosing a legal evolution card is mandatory"
    );
    assert_eq!(
        evo_pick.valid_action_ids.len(),
        1,
        "only the purple evolution card should be selectable"
    );

    runner
        .execute_action(0, evo_pick.valid_action_ids[0])
        .expect("evolution selection resolves");
    runner.game.drain_effect_queue();

    let base = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|permanent| permanent.is_digimon(&runner.game.card_data))
        .expect("base Digimon remains in battle area");
    assert_eq!(base.stack_size(), 2);
    assert_eq!(
        base.top_card().card_id(&runner.game.card_data),
        "PURPLE-EVO"
    );
    assert_eq!(
        runner.memory(),
        4,
        "printed evo cost 3 reduced by 2 should pay only 1 memory"
    );
}

/// PUPPETS-G009 — LM-056's standard `<Delay>` is a player-visible
/// `[Main]`-phase activation. No activation on the placing turn; on a later
/// own main phase the mask exposes the `FIELD_EFFECT` action. Taking it
/// trashes LM-056 as the cost and installs the optional own-Digimon digivolve
/// selection.
#[test]
fn lm_056_delay_is_main_phase_action_after_placing_turn() {
    let yaml = lm_056_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-056 YAML parses")
        .add_card(digimon("BASE", CardColor::Purple, 3))
        .add_card(lv4_evo(
            "PURPLE-EVO",
            CardColor::Purple,
            CardColor::Purple,
            3,
        ))
        .add_card(filler("FILL"))
        .hand(0, &["PURPLE-EVO"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    let delay_perm = runner.place_on_field(0, "LM-056", Some(0));
    runner.place_on_field(0, "BASE", Some(0));
    let placing_turn = runner.game.turn_count;
    runner.game.player_mut(0).battle_area[delay_perm.index as usize].option_state =
        OptionState::Delayed {
            owner: 0,
            trash_on_turn: u16::MAX,
            trigger: DelayTrigger::MainPhaseActivated,
            placed_on_turn: placing_turn,
        };
    let delay_idx = delay_perm.index as usize;
    let bit = (FIELD_EFFECT_START
        + delay_idx as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN) as usize;

    // Same turn it was placed: the Delay activation is NOT legal (16-16-3).
    runner.game.enter_main_phase();
    assert_eq!(
        build_action_mask(&runner.game, 0)[bit],
        0.0,
        "LM-056 <Delay> must not be activatable on the placing turn"
    );

    // Advance to the controller's next main phase.
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
    runner.game.set_memory(5);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[bit], 1.0,
        "LM-056 <Delay> activation is a legal action"
    );
    assert_eq!(mask[PASS as usize], 1.0, "declining stays legal");

    // Take the activation — trashes LM-056 as cost, installs the digivolve.
    runner.game.decode_action(bit as u16, 0);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField),
        "the <Delay> body installs the optional own-Digimon digivolve target pick"
    );
    let target_pick = runner.pending_selection_view().unwrap();
    runner
        .execute_action(0, target_pick.valid_action_ids[0])
        .expect("target selection resolves");
    let evo_pick = runner.pending_selection_view().unwrap();
    runner
        .execute_action(0, evo_pick.valid_action_ids[0])
        .expect("evolution selection resolves");
    runner.game.drain_effect_queue();

    let base = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|permanent| permanent.is_digimon(&runner.game.card_data))
        .expect("base Digimon remains in battle area");
    assert_eq!(base.stack_size(), 2, "LM-056 <Delay> digivolved the base");
    assert_eq!(
        base.top_card().card_id(&runner.game.card_data),
        "PURPLE-EVO"
    );
    assert_eq!(
        runner.memory(),
        4,
        "printed evo cost 3 reduced by 2 pays only 1 memory"
    );
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| matches!(p.option_state, OptionState::Delayed { .. })),
        "LM-056 is trashed as the <Delay> activation cost"
    );
}

/// PUPPETS-G009 — declining (PASS) leaves LM-056's `<Delay>` Option parked on
/// the battle area for a future legal activation.
#[test]
fn lm_056_declining_delay_leaves_option_in_battle_area() {
    let yaml = lm_056_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-056 YAML parses")
        .add_card(digimon("BASE", CardColor::Purple, 3))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    let delay_perm = runner.place_on_field(0, "LM-056", Some(0));
    runner.place_on_field(0, "BASE", Some(0));
    let placing_turn = runner.game.turn_count;
    runner.game.player_mut(0).battle_area[delay_perm.index as usize].option_state =
        OptionState::Delayed {
            owner: 0,
            trash_on_turn: u16::MAX,
            trigger: DelayTrigger::MainPhaseActivated,
            placed_on_turn: placing_turn,
        };

    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();

    // Decline — pass instead of activating the Delay.
    runner.game.decode_action(PASS, 0);

    assert!(
        runner.game.player(0).battle_area.iter().any(|p| matches!(
            p.option_state,
            OptionState::Delayed {
                trigger: DelayTrigger::MainPhaseActivated,
                ..
            }
        )),
        "declined LM-056 <Delay> stays parked for a future activation"
    );
    assert_eq!(runner.trash_size(0), 0, "declined LM-056 is not trashed");
}
