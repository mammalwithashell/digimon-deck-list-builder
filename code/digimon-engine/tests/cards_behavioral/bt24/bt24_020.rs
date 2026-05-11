//! BT24-020 Gomamon — Digimon, Lv.3, Blue.
//!
//! # Card text
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with
//! the [Sea Beast] or [Shaman] trait or [Aqua] or [Sea Animal] in any of its
//! traits and 1 card with the [TS] trait among them to the hand. Return the
//! rest to the bottom of the deck.
//!
//! Inherited Effect [Your Turn] [Once Per Turn] When this Digimon unsuspends,
//! if you have 7 or fewer cards in your hand, <Draw 1>.
//!
//! # Patterns
//!
//! - A1/A2 reveal top-N, multi-bucket selection, bottom remainder
//! - select_reveal_buckets with no duplicate selected card
//! - inherited OnUnsuspend draw gated by hand size and [Once Per Turn]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledCost, CompiledCountAggregate,
    CompiledDpConstraint, CompiledPlayerRef, CompiledPredicate, CompiledScope, CompiledStep,
    CompiledTiming, CompiledZone,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

const CARD_ID: &str = "BT24-020";

#[test]
fn bt24_020_yaml_has_metadata_alt_path_on_play_and_inherited_unsuspend() {
    let runner = gomamon_runner().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT24-020 must be present in embedded DSL pack");

    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(3));
    assert_eq!(card.color, vec![CompiledColor::Blue]);
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(1000));
    for trait_name in ["Sea Beast", "Iliad", "ADAMAS", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT24-020 metadata must include trait {trait_name}"
        );
    }

    assert!(
        card.alt_paths.iter().any(|path| {
            path.cost == Some(CompiledCost::Literal(0))
                && path.from.as_ref().is_some_and(|from| {
                    from.all_of.iter().any(|p| p.level_eq == Some(2))
                        && from
                            .all_of
                            .iter()
                            .any(|p| p.color_is == Some(CompiledColor::Blue))
                })
        }),
        "BT24-020 must digivolve from blue level 2 for cost 0"
    );
    assert!(
        card.alt_paths.iter().any(|path| {
            path.cost == Some(CompiledCost::Literal(0))
                && path.from.as_ref().is_some_and(|from| {
                    from.all_of.iter().any(|p| p.level_eq == Some(2))
                        && from
                            .all_of
                            .iter()
                            .any(|p| p.trait_has.as_deref() == Some("TS"))
                })
        }),
        "BT24-020 must also digivolve from level 2 with [TS] trait for cost 0"
    );

    let on_play = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.scope == CompiledScope::FaceUp
                    && triggered.when == vec![CompiledTiming::OnPlay] =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("BT24-020 must have an On Play clause");
    assert!(
        on_play.process.iter().any(|step| matches!(
            step,
            CompiledStep::SelectRevealBuckets {
                no_duplicate_cards: true,
                ..
            }
        )),
        "On Play must use select_reveal_buckets with no duplicate cards"
    );
    assert!(
        on_play
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::PlaceRemainderOnDeck { .. })),
        "On Play must bottom the unchosen reveal remainder"
    );

    let inherited = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.scope == CompiledScope::Inherited
                    && triggered.when == vec![CompiledTiming::OnUnsuspend] =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("BT24-020 must have an inherited OnUnsuspend clause");
    assert!(inherited.once_per_turn);
    assert_eq!(
        inherited.condition,
        Some(CompiledPredicate {
            all_of: vec![
                CompiledPredicate {
                    your_turn: Some(true),
                    ..CompiledPredicate::default()
                },
                CompiledPredicate {
                    count_lte: Some(CompiledCountAggregate {
                        filter: Box::new(CompiledPredicate {
                            zone: vec![CompiledZone::Hand],
                            owner: Some(CompiledPlayerRef::You),
                            ..CompiledPredicate::default()
                        }),
                        n: CompiledDpConstraint::Literal(7),
                    }),
                    ..CompiledPredicate::default()
                }
            ],
            ..CompiledPredicate::default()
        }),
        "inherited clause must require your turn and hand size <= 7"
    );
    assert_eq!(
        inherited.process,
        vec![CompiledStep::Draw {
            of: CompiledPlayerRef::You,
            count: 1,
        }]
    );
}

#[test]
fn bt24_020_on_play_adds_sea_bucket_and_ts_bucket_without_double_picking() {
    let mut runner = gomamon_runner()
        .add_card(make_trait_digimon("DUAL", &["Sea Beast", "TS"]))
        .add_card(make_trait_digimon("SEA-ANIMAL", &["Sea Animal"]))
        .add_card(make_trait_tamer("TS-TAMER", &["TS"]))
        .add_card(make_trait_digimon("BOTTOM-PAD", &[]))
        .hand(0, &[CARD_ID])
        .deck(0, &["BOTTOM-PAD", "SEA-ANIMAL", "TS-TAMER", "DUAL"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play BT24-020");
    assert_required_pending_pick(&runner, "Sea/Shaman/Aqua/Sea Animal bucket");
    pick_revealed_by_id(&mut runner, "DUAL", "pick DUAL for first bucket");

    assert_required_pending_pick(&runner, "TS bucket");
    let second_view = runner.pending_selection_view().expect("TS bucket");
    let dual_action = revealed_action_for_id(&runner, "DUAL").expect("DUAL remains visible");
    assert!(
        !second_view.valid_action_ids.contains(&dual_action),
        "no_duplicate_cards must prevent DUAL from satisfying both buckets"
    );
    pick_revealed_by_id(&mut runner, "TS-TAMER", "pick TS-TAMER for TS bucket");
    runner.auto_resolve().expect("bottom reveal remainder");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert_eq!(hand_ids.iter().filter(|id| *id == "DUAL").count(), 1);
    assert_eq!(hand_ids.iter().filter(|id| *id == "TS-TAMER").count(), 1);
    assert!(
        !hand_ids.contains(&"SEA-ANIMAL".to_string()),
        "unchosen reveal card must not enter hand"
    );

    let deck_ids = zone_ids(&runner.game.players[0].deck, &runner.game.card_data);
    assert_eq!(
        deck_ids.first().map(String::as_str),
        Some("SEA-ANIMAL"),
        "unchosen reveal card must be returned to deck bottom; deck was {deck_ids:?}"
    );
}

#[test]
fn bt24_020_on_play_first_bucket_accepts_aqua_trait() {
    let mut runner = gomamon_runner()
        .add_card(make_trait_digimon("AQUA", &["Aqua"]))
        .add_card(make_trait_tamer("TS-TAMER", &["TS"]))
        .add_card(make_trait_digimon("FILLER", &[]))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILLER", "TS-TAMER", "AQUA"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play BT24-020");
    pick_revealed_by_id(&mut runner, "AQUA", "Aqua trait must be eligible");
    pick_revealed_by_id(&mut runner, "TS-TAMER", "pick TS card");
    runner.auto_resolve().expect("finish remainder");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"AQUA".to_string()));
    assert!(hand_ids.contains(&"TS-TAMER".to_string()));
}

#[test]
fn bt24_020_inherited_on_unsuspend_draws_when_hand_count_is_seven_or_less() {
    let mut runner = gomamon_runner()
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("DRAW", "Draw"))
        .add_card(make_test_card("HAND", "Hand"))
        .hand(0, &["HAND", "HAND", "HAND", "HAND", "HAND", "HAND", "HAND"])
        .deck(0, &["DRAW"])
        .memory(0)
        .start();

    let carrier = place_gomamon_as_source(&mut runner);
    runner.game_mut().players[0].battle_area[carrier.index as usize].is_suspended = true;

    runner.game_mut().unsuspend(carrier);
    let _ = runner.auto_resolve();

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert_eq!(
        hand_ids.iter().filter(|id| *id == "DRAW").count(),
        1,
        "inherited OnUnsuspend must draw 1 at hand size 7"
    );
    assert_eq!(runner.deck_size(0), 0);
}

#[test]
fn bt24_020_inherited_on_unsuspend_does_not_draw_when_hand_count_above_seven() {
    let mut runner = gomamon_runner()
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("DRAW", "Draw"))
        .add_card(make_test_card("HAND", "Hand"))
        .hand(
            0,
            &[
                "HAND", "HAND", "HAND", "HAND", "HAND", "HAND", "HAND", "HAND",
            ],
        )
        .deck(0, &["DRAW"])
        .memory(0)
        .start();

    let carrier = place_gomamon_as_source(&mut runner);
    runner.game_mut().players[0].battle_area[carrier.index as usize].is_suspended = true;

    runner.game_mut().unsuspend(carrier);
    let _ = runner.auto_resolve();

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        !hand_ids.contains(&"DRAW".to_string()),
        "hand size > 7 must suppress the inherited draw"
    );
    assert_eq!(runner.deck_size(0), 1);
}

#[test]
fn bt24_020_inherited_on_unsuspend_is_once_per_turn() {
    let mut runner = gomamon_runner()
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("DRAW-1", "Draw 1"))
        .add_card(make_test_card("DRAW-2", "Draw 2"))
        .memory(0)
        .deck(0, &["DRAW-2", "DRAW-1"])
        .start();

    let carrier = place_gomamon_as_source(&mut runner);

    for _ in 0..2 {
        runner.game_mut().players[0].battle_area[carrier.index as usize].is_suspended = true;
        runner.game_mut().unsuspend(carrier);
        let _ = runner.auto_resolve();
    }

    assert_eq!(
        runner.hand_size(0),
        1,
        "second unsuspend in the same turn must be blocked by [Once Per Turn]"
    );
    assert_eq!(runner.deck_size(0), 1);
}

fn gomamon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-020 YAML loads")
}

fn make_trait_digimon(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn make_trait_tamer(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn place_gomamon_as_source(runner: &mut DebugRunner) -> digimon_engine::PermanentHandle {
    runner.place_stack(0, &[CARD_ID, "CARRIER"])
}

fn assert_required_pending_pick(runner: &DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    assert!(
        !runner.pending_is_optional(),
        "{label}: reveal bucket should be mandatory when a matching card exists"
    );
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "{label}: PASS must not be legal before the required pick"
    );
}

fn pick_revealed_by_id(runner: &mut DebugRunner, id: &str, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = revealed_action_for_id(runner, id).expect("revealed card exists");
    assert!(
        view.valid_action_ids.contains(&action),
        "{label}: action {action} must be legal among {:?}",
        view.valid_action_ids
    );
    runner.execute_action(0, action).expect(label);
}

fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> Option<u16> {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
}

fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}
