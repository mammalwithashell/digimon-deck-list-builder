//! LM-049 Midnight Memory Boost! - Option, Cost 3, Black.
//!
//! # Card text (per-card JSON: code/digimon-engine/cards/lm/LM-049.json)
//!
//! Blue also meets this card's color requirements.
//! [Main] Reveal the top 3 cards of your deck. Add 1 black or blue Digimon
//! card among them to the hand. Return the rest to the bottom of deck. Then,
//! place this card in the battle area.
//! [Main] <Delay> (By trashing this card after the placing turn, activate the
//! effect below.)
//! - Gain 2 memory.
//!
//! Inherited: Security Effect [Security] Place this card in the battle area.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/LM/Black/LM_049.cs (base repo) —
//! `IgnoreColorConditionClass` gates the blue color-requirement bypass on
//! having a blue Digimon/Tamer permanent; `ActivateClass` runs
//! `RevealDeckTopCardsAndSelect(revealCount: 3, ..., mode: AddHand)` filtered
//! to `IsDigimon && (Black || Blue)`, remainder to `DeckBottom`, then
//! `PlaceDelayOptionCards`; `EffectTiming.OnDeclaration` fires
//! `Gain2MemoryOptionDelayEffect`; `EffectTiming.SecuritySkill` fires
//! `PlaceSelfDelayOptionSecurityEffect`.
//!
//! # Template note
//! LM-049 is the structural twin of LM-037 Sepia Memory Boost! (same body,
//! yellow -> blue substitution) and LM-054 Treadmill Training (same Delay /
//! inherited-security scaffolding). This test mirrors their coverage shape.
//!
//! # Patterns this test covers
//! - D3-style always-on color-requirement bypass (LM-037 template; unlike
//!   LM-054, LM-049's bypass is unconditional, matching the printed text
//!   "Blue also meets this card's color requirements.")
//! - A2 reveal top 3, add 1 matching Digimon, bottom the rest
//! - Standard player-activated `<Delay>` body (PUPPETS-G009) gaining 2 memory
//! - Inherited [Security] placement as a delayed Option via
//!   `place_self_as_delay_option`

use std::path::Path;

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope,
    CompiledStackPosition, CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS, PLAY_HAND_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::OptionPlayResult;

fn lm_049_yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/lm/LM-049.yaml"))
        .expect("LM-049 YAML must exist at cards/lm/LM-049.yaml")
}

fn lm_049_runner() -> DebugRunner {
    let yaml = lm_049_yaml();
    DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-049 YAML must parse and compile")
        .memory(10)
        .start()
}

fn make_digimon(id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card
}

fn make_tamer(id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![color];
    card
}

fn make_option(id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.colors = vec![color];
    card
}

fn black_digimon(id: &str) -> CardData {
    make_digimon(id, CardColor::Black)
}

fn blue_digimon(id: &str) -> CardData {
    make_digimon(id, CardColor::Blue)
}

fn red_digimon(id: &str) -> CardData {
    make_digimon(id, CardColor::Red)
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

#[test]
fn lm_049_yaml_parses_without_error() {
    let _runner = lm_049_runner();
}

#[test]
fn lm_049_is_black_option_cost_3() {
    let runner = lm_049_runner();
    let compiled = runner
        .compiled_card("LM-049")
        .expect("LM-049 must be registered");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.color, vec![CompiledColor::Black]);
    assert_eq!(compiled.cost, Some(3));
}

#[test]
fn lm_049_use_requirement_allows_blue_to_meet_option_color_requirement() {
    let runner = lm_049_runner();
    let compiled = runner
        .compiled_card("LM-049")
        .expect("LM-049 must be registered");

    let use_requirement = compiled
        .use_requirement
        .as_ref()
        .expect("blue color-requirement clause must be represented");
    let field_req = use_requirement
        .any_field_permanent
        .as_ref()
        .expect("use_requirement should scan your field");

    assert_eq!(field_req.of, digimon_dsl::compiled::CompiledPlayerRef::You);
    assert_eq!(
        field_req.predicate.color_is,
        Some(CompiledColor::Blue),
        "blue permanents must satisfy LM-049 option use requirements"
    );
    assert!(
        field_req
            .predicate
            .any_of
            .iter()
            .any(|p| p.kind == Some(CompiledCardKind::Digimon)),
        "blue Digimon must satisfy the alternate color requirement"
    );
    assert!(
        field_req
            .predicate
            .any_of
            .iter()
            .any(|p| p.kind == Some(CompiledCardKind::Tamer)),
        "blue Tamers must satisfy the alternate color requirement"
    );
}

#[test]
fn lm_049_action_mask_allows_use_with_blue_tamer_but_not_unrelated_color() {
    let yaml = lm_049_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-049 YAML parses")
        .add_card(make_tamer("BLUE-TAMER", CardColor::Blue))
        .add_card(make_tamer("RED-TAMER", CardColor::Red))
        .add_card(filler("FILL"))
        .hand(0, &["LM-049"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();

    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        0.0,
        "LM-049 should not be usable with no black or blue Digimon/Tamer"
    );

    runner.place_on_field(0, "RED-TAMER", Some(0));
    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        0.0,
        "red permanents must not satisfy LM-049 color requirements"
    );

    runner.place_on_field(0, "BLUE-TAMER", Some(0));
    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        1.0,
        "blue permanents must satisfy LM-049 color requirements"
    );
    assert_ne!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid,
        "execution legality must match the action mask"
    );
}

#[test]
fn lm_049_has_main_delay_and_inherited_security_clauses() {
    let runner = lm_049_runner();
    let compiled = runner
        .compiled_card("LM-049")
        .expect("LM-049 must be registered");

    assert_eq!(
        compiled.effects.len(),
        3,
        "LM-049 should have Main, Delay, and inherited Security clauses"
    );

    match &compiled.effects[0] {
        CompiledClause::Triggered(t) => {
            assert!(t.when.contains(&CompiledTiming::MainFromHand));
            assert_eq!(t.scope, CompiledScope::FaceUp);
            assert!(!t.optional, "[Main] search is mandatory");
        }
        other => panic!("clause 0 must be main_from_hand triggered; got {other:?}"),
    }

    match &compiled.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
            trigger, process, ..
        }) => {
            assert_eq!(*trigger, CompiledTiming::Delayed);
            assert!(
                process
                    .iter()
                    .any(|s| matches!(s, CompiledStep::GainMemory(2))),
                "Delay process must gain 2 memory; got {process:?}"
            );
        }
        other => panic!("clause 1 must be a standard Delay; got {other:?}"),
    }

    match &compiled.effects[2] {
        CompiledClause::Triggered(t) => {
            assert!(t.when.contains(&CompiledTiming::OnSecurity));
            assert_eq!(t.scope, CompiledScope::Inherited);
            assert_eq!(
                t.process,
                vec![CompiledStep::PlaceSelfAsDelayOption],
                "inherited Security placement must use the native delay-option placement step"
            );
        }
        other => panic!("clause 2 must be inherited on_security; got {other:?}"),
    }
}

#[test]
fn lm_049_main_reveals_top_3_and_filters_black_or_blue_digimon() {
    let runner = lm_049_runner();
    let compiled = runner
        .compiled_card("LM-049")
        .expect("LM-049 must be registered");

    let main = match &compiled.effects[0] {
        CompiledClause::Triggered(t) => t,
        other => panic!("clause 0 must be triggered; got {other:?}"),
    };

    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::RevealTopDeck { count: 3, .. })),
        "Main process must reveal top 3 cards; got {:?}",
        main.process
    );

    let select_filter = main
        .process
        .iter()
        .find_map(|s| match s {
            CompiledStep::SelectReveal {
                filter, optional, ..
            } => {
                assert!(
                    !*optional,
                    "printed text says add 1 matching card; PASS must not be legal when a candidate exists"
                );
                Some(filter)
            }
            _ => None,
        })
        .expect("Main process must select from reveal");

    assert_eq!(select_filter.kind, Some(CompiledCardKind::Digimon));
    assert!(
        select_filter
            .any_of
            .iter()
            .any(|p| p.color_is == Some(CompiledColor::Black)),
        "select_reveal must allow black Digimon"
    );
    assert!(
        select_filter
            .any_of
            .iter()
            .any(|p| p.color_is == Some(CompiledColor::Blue)),
        "select_reveal must allow blue Digimon"
    );
    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::AddToHandFromReveal { .. })),
        "Main process must add the selected revealed card to hand"
    );
    assert!(
        main.process.iter().any(|s| matches!(
            s,
            CompiledStep::PlaceRemainderOnDeck {
                position: CompiledStackPosition::Bottom,
                ..
            }
        )),
        "Main process must return the rest to deck bottom"
    );
}

#[test]
fn lm_049_main_adds_blue_digimon_from_top_3_to_hand() {
    let yaml = lm_049_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-049 YAML parses")
        .add_card(blue_digimon("BLUE-DIGI"))
        .add_card(black_digimon("BLACK-DIGI"))
        .add_card(red_digimon("RED-DIGI"))
        .add_card(make_option("BLACK-OPT", CardColor::Black))
        .add_card(filler("FILL"))
        .hand(0, &["LM-049"])
        .deck(
            0,
            &[
                "FILL",
                "FILL",
                "FILL",
                "BLACK-OPT",
                "RED-DIGI",
                "BLACK-DIGI",
                "BLUE-DIGI",
            ],
        )
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let hand_before = runner.game.players[0].hand.len();
    let deck_before = runner.deck_size(0);

    assert!(runner.game.activate_hand_main(0, 0));
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 1,
        "LM-049 Main should add exactly one eligible Digimon from reveal"
    );
    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BLUE-DIGI"),
        "the isolated eligible blue Digimon should be selected and added to hand"
    );
    assert_eq!(
        deck_before - runner.deck_size(0),
        1,
        "Reveal 3, add 1, and return 2 should shrink deck by exactly 1"
    );
}

#[test]
fn lm_049_main_no_eligible_digimon_among_reveal_adds_nothing() {
    let yaml = lm_049_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-049 YAML parses")
        .add_card(black_digimon("BLACK-FIELD"))
        .add_card(red_digimon("RED-DIGI-1"))
        .add_card(red_digimon("RED-DIGI-2"))
        .add_card(make_option("BLACK-OPT", CardColor::Black))
        .add_card(filler("FILL"))
        .hand(0, &["LM-049"])
        .deck(
            0,
            &[
                "FILL", "FILL", "FILL", "BLACK-OPT", "RED-DIGI-2", "RED-DIGI-1",
            ],
        )
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "BLACK-FIELD", Some(0));
    runner.game.enter_main_phase();

    let hand_before = runner.game.players[0].hand.len();
    let deck_before = runner.deck_size(0);

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner
        .auto_resolve()
        .expect("resolve LM-049 with no eligible reveal candidate");

    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before - 1,
        "no black/blue Digimon among the top 3 (a black Option and two red Digimon) means nothing is added; \
         hand only shrinks by the LM-049 card itself leaving hand to be played"
    );
    assert_eq!(
        deck_before - runner.deck_size(0),
        0,
        "all 3 revealed cards should return to the bottom of the deck when none are eligible"
    );
    // LM-049 itself should still be placed as a delayed Option permanent even
    // when the reveal yields no eligible addition.
    assert!(runner.game.player(0).battle_area.iter().any(|permanent| {
        permanent.top_card().card_id(&runner.game.card_data) == "LM-049"
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
fn lm_049_security_check_places_self_in_battle_area_as_delay_option() {
    let yaml = lm_049_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-049 YAML parses")
        .add_card(red_digimon("ATTACKER"))
        .add_card(filler("FILL"))
        .security(1, &["LM-049"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let result = runner.attack_player(attacker, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(runner.security_count(1), 0, "LM-049 should leave security");
    assert_eq!(
        runner.trash_size(1),
        0,
        "LM-049 should be placed in battle area instead of trashed"
    );
    let placed = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "LM-049")
        .expect("LM-049 should be placed as a battle-area Option permanent");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
}

/// PUPPETS-G009 — LM-049's standard `<Delay>` is a player-visible
/// `[Main]`-phase activation. Playing it parks a `MainPhaseActivated`
/// delayed Option; on a later main phase the controller activates it,
/// trashing the Option as cost and running the body (gain 2 memory).
#[test]
fn lm_049_delay_activation_gains_2_memory_via_main_phase_action() {
    let yaml = lm_049_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-049 YAML parses")
        .add_card(make_tamer("BLUE-TAMER", CardColor::Blue))
        .add_card(red_digimon("FILL"))
        .hand(0, &["LM-049"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    runner.place_on_field(0, "BLUE-TAMER", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner
        .auto_resolve()
        .expect("resolve LM-049 reveal selection and delay placement");

    let delay_idx = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|permanent| {
            matches!(
                permanent.option_state,
                OptionState::Delayed {
                    trigger: DelayTrigger::MainPhaseActivated,
                    ..
                }
            )
        })
        .expect("playing LM-049 should park it as a MainPhaseActivated delayed option");

    let bit = (FIELD_EFFECT_START
        + delay_idx as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN) as usize;
    assert_eq!(
        build_action_mask(&runner.game, 0)[bit],
        0.0,
        "LM-049 <Delay> must not be activatable on the placing turn"
    );

    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
    runner.game.set_memory(0);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[bit], 1.0,
        "LM-049 <Delay> activation is a legal action"
    );
    assert_eq!(mask[PASS as usize], 1.0, "declining stays legal");

    runner.game.decode_action(bit as u16, 0);

    assert_eq!(
        runner.memory(),
        2,
        "LM-049 <Delay> body gains 2 memory when the player activates it"
    );
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|permanent| matches!(permanent.option_state, OptionState::Delayed { .. })),
        "LM-049 is trashed as the <Delay> activation cost"
    );
}

/// PUPPETS-G009 — declining (PASS) the `<Delay>` activation leaves LM-049
/// parked on the battle area for a later legal activation instead of forcing
/// it (negative counterpart to the previous positive test).
#[test]
fn lm_049_declining_delay_leaves_option_in_battle_area() {
    let yaml = lm_049_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("LM-049 YAML parses")
        .add_card(black_digimon("BLACK-FIELD"))
        .add_card(red_digimon("FILL"))
        .hand(0, &["LM-049"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    // A black permanent on the field satisfies LM-049's own (black) color
    // requirement directly -- the "blue also meets this requirement" clause
    // is only needed when no black permanent is present (covered by the
    // dedicated color-bypass tests above).
    runner.place_on_field(0, "BLACK-FIELD", Some(0));
    runner.game.enter_main_phase();
    runner.game.set_memory(10);

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner
        .auto_resolve()
        .expect("resolve LM-049 reveal selection and delay placement");

    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();

    let delay_idx = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|permanent| {
            matches!(
                permanent.option_state,
                OptionState::Delayed {
                    trigger: DelayTrigger::MainPhaseActivated,
                    ..
                }
            )
        })
        .expect("LM-049 should still be parked as a delayed Option");
    let bit = (FIELD_EFFECT_START
        + delay_idx as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN) as usize;
    assert_eq!(
        build_action_mask(&runner.game, 0)[bit],
        1.0,
        "LM-049 <Delay> activation is legal after the placing turn"
    );

    // Decline -- pass instead of activating the Delay (PASS at Main phase
    // ends the turn per `decode_main`, matching LM-054's equivalent test;
    // the assertion of interest is that LM-049 stays parked and untrashed
    // rather than being force-activated).
    runner.game.decode_action(PASS, 0);

    assert!(
        runner.game.player(0).battle_area.iter().any(|p| matches!(
            p.option_state,
            OptionState::Delayed {
                trigger: DelayTrigger::MainPhaseActivated,
                ..
            }
        )),
        "declined LM-049 <Delay> stays parked for a future activation"
    );
    assert_eq!(runner.trash_size(0), 0, "declined LM-049 is not trashed");
}
