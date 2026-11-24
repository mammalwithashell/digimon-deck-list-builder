//! Task A2.2 — DSL `place_as_bottom_source` step can take a deck-top source.
//!
//! End-to-end coverage: a YAML `[On Play]` effect that tucks the controller's
//! deck-top card face-down under the source permanent, addressed via the
//! `{ deck_top: you }` structured binding ref.
//!
//! Pattern mirrors `place_as_bottom_source_face_down.rs` (Task A1.3). The ONLY
//! difference is the `source:` binding: instead of a `select_trash` named
//! binding, the source is the top of the controller's deck. No selection is
//! installed — `place_as_bottom_source` with a `deck_top` source runs
//! synchronously.

use std::sync::Arc;

use digimon_engine::action::{mask::build_action_mask, space::HAND_EFFECT_START};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::selection::SelectionKind;

const YAML: &str = r#"
card: TST-PABS-DT
name: PlaceDeckTopBottom
kind: digimon
level: 3
color: [red]
cost: 4
dp: 3000
effects:
  - when: on_play
    process:
      - place_as_bottom_source:
          source: { deck_top: you }
          target: source
          face_down: true
"#;

const KURATA_PLACEMENT: &str = r#"
card: BT27-098
name: Akihiro Kurata
kind: tamer
color: [purple]
cost: 4
effects:
  - text: "[All Turns] When any Digimon are deleted, by suspending this Tamer, you may place the top 2 cards of your deck face down under this Tamer."
    when: on_any_deletion
    trigger_if: { event_card_kind: digimon }
    optional: true
    process:
      - activation_cost: { suspend_self: true }
      - place_deck_top_under: { of: you, count: 2, target: source, face_down: true }
"#;

const WATCH_PLACEMENT: &str = r#"
card: TST-PABS-WATCH
name: Placement Observer
kind: tamer
color: [purple]
cost: 3
effects:
  - when: on_add_digivolution_cards
    trigger_if: { event_caused_by_own_effect: true }
    process:
      - gain_memory: 1
"#;

// BT27-098 (data/card_bundles/BT27-098.md), official Q&A:
// "The card is placed on the bottom of the cards under the Tamer."
#[test]
fn bt27_098_deletion_stashes_top_two_in_one_batch_or_does_nothing_on_decline() {
    for (deck_size, accept, order) in [
        (3, true, Some(0)),
        (3, true, Some(1)),
        (1, true, None),
        (0, true, None),
        (3, false, None),
    ] {
        let mut runner = DebugRunner::builder()
            .from_dsl_yaml(KURATA_PLACEMENT)
            .expect("Kurata placement compiles")
            .from_dsl_yaml(WATCH_PLACEMENT)
            .expect("observer compiles")
            .add_card(make_test_card("TST-PABS-OLD", "Existing Source"))
            .add_card(make_test_card("TST-PABS-VICTIM", "Deleted Digimon"))
            .add_card(make_test_card("TST-PABS-DECK", "Deck Card"))
            .deck(0, &["TST-PABS-DECK"; 3][..deck_size])
            .memory(5)
            .start();
        let kurata = runner.place_stack(0, &["TST-PABS-OLD", "BT27-098"]);
        runner.place_on_field(0, "TST-PABS-WATCH", None);
        let victim = runner.place_on_field(1, "TST-PABS-VICTIM", None);
        let deck_before: Vec<_> = runner.game.players[0].deck.iter().map(|c| c.handle()).collect();
        let memory = runner.game.memory;

        runner.game.delete_permanent_with_effects(victim);
        if accept {
            runner.accept_optional_trigger().unwrap();
        } else {
            runner.decline_optional_trigger().unwrap();
        }
        if let Some(order) = order {
            assert_two_card_order_menu(&runner, 0);
            assert_eq!(runner.game.players[0].deck.len(), deck_size);
            if order == 1 {
                runner.game = runner.game.clone();
            }
            runner.execute_action(0, HAND_EFFECT_START + order).unwrap();
        }
        assert!(runner.game.pending_selection.is_none());

        let moved = if accept { deck_size.min(2) } else { 0 };
        let (remaining, placed) = deck_before.split_at(deck_size - moved);
        let mut placed = placed.to_vec();
        if order == Some(1) {
            placed.reverse();
        }
        let player = &runner.game.players[0];
        assert_eq!(player.deck.iter().map(|c| c.handle()).collect::<Vec<_>>(), remaining);
        let tamer = &player.battle_area[kurata.index as usize];
        assert_eq!(tamer.is_suspended, accept);
        assert_eq!(tamer.card_sources.len(), moved + 2);
        assert!(tamer.card_sources[..moved].iter().all(|c| c.face_down));
        assert_eq!(
            tamer.card_sources[..moved].iter().map(|c| c.handle()).collect::<Vec<_>>(),
            placed
        );
        assert_eq!(tamer.card_sources[moved].card_id(&runner.game.card_data), "TST-PABS-OLD");
        assert!(!tamer.card_sources[moved].face_down);
        assert_eq!(runner.game.memory, memory + i16::from(moved > 0));
    }
}

fn assert_two_card_order_menu(runner: &DebugRunner, chooser: u8) {
    let pending = runner.game.pending_selection.as_ref().expect("order prompt");
    assert!(matches!(pending.kind, SelectionKind::EffectChoice));
    assert_eq!(pending.selecting_player, chooser);
    assert!(pending.card_choice.is_none());
    let choices = pending.effect_choices.as_ref().unwrap();
    assert_eq!(
        choices.iter().map(|choice| choice.label.as_str()).collect::<Vec<_>>(),
        ["1 → 2", "2 → 1"],
    );
    assert!(choices.iter().all(|choice| choice.source_card == Some(pending.source_card)));
    let legal: Vec<_> = build_action_mask(&runner.game, chooser)
        .iter()
        .enumerate()
        .filter_map(|(action, &legal)| (legal > 0.0).then_some(action as u16))
        .collect();
    assert_eq!(legal, [HAND_EFFECT_START, HAND_EFFECT_START + 1]);
    assert!(build_action_mask(&runner.game, 1 - chooser).iter().all(|&legal| legal == 0.0));
}

#[test]
fn place_deck_top_under_uses_the_chosen_digimon_and_the_named_deck() {
    let yaml = YAML.replace(
        "      - place_as_bottom_source:\n          source: { deck_top: you }\n          target: source\n          face_down: true",
        "      - select_own_permanent:\n          bind_as: recipient\n          filter: { kind: digimon }\n          prompt: Choose a Digimon to receive the cards\n      - place_deck_top_under: { of: opponent, count: 2, target: recipient, face_down: true }\n      - gain_memory: 1",
    );
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("targeted placement compiles")
        .add_card(make_test_card("TST-PABS-RECIPIENT", "Recipient"))
        .add_card(make_test_card("TST-PABS-DECK", "Deck Card"))
        .deck(0, &["TST-PABS-DECK"])
        .deck(1, &["TST-PABS-DECK"; 2])
        .start();
    let source = runner.place_on_field(0, "TST-PABS-DT", None);
    let recipient = runner.place_on_field(0, "TST-PABS-RECIPIENT", None);
    runner.game.fire_on_play(0, source.index as usize);
    assert_eq!(runner.game.players[1].deck.len(), 2);
    let memory = runner.game.memory;
    runner.choose_prompt_card("TST-PABS-RECIPIENT", None).unwrap();
    assert_two_card_order_menu(&runner, 0);
    assert_eq!(runner.game.memory, memory);
    runner.execute_action(0, HAND_EFFECT_START).unwrap();
    assert_eq!(runner.game.memory, memory + 1);
    let player = &runner.game.players[0];
    assert_eq!(player.deck.len(), 1);
    assert!(runner.game.players[1].deck.is_empty());
    assert_eq!(player.battle_area[source.index as usize].card_sources.len(), 1);
    let stack = &player.battle_area[recipient.index as usize].card_sources;
    assert_eq!(stack.len(), 3);
    assert!(stack[..2].iter().all(|c| c.face_down));
}

#[test]
fn deck_top_order_uses_the_effect_card_owner_even_with_a_different_controller() {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(KURATA_PLACEMENT).unwrap();
    let compiled = digimon_dsl::compile::compile(&spec).unwrap();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(KURATA_PLACEMENT)
        .unwrap()
        .add_card(make_test_card("TST-PABS-DECK", "Deck Card"))
        .deck(0, &["TST-PABS-DECK"; 2])
        .start();
    let kurata = runner.place_on_field(1, "BT27-098", None);
    let source = runner.game.players[1].battle_area[kurata.index as usize].top_card().handle();
    let effect = DslCardEffect::new(Arc::new(compiled)).effects(source).remove(0);
    (effect.process.unwrap())(&mut EffectContext::new(&mut runner.game, source, Some(kurata), 0));
    assert_two_card_order_menu(&runner, 1);
    runner.execute_action(1, HAND_EFFECT_START).unwrap();
    assert!(runner.game.players[0].deck.is_empty());
    assert_eq!(runner.game.players[1].battle_area[kurata.index as usize].card_sources.len(), 3);
}

#[test]
fn place_as_bottom_source_deck_top_stashes_deck_top_face_down() {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compiles cleanly");

    // Seed: TST-PABS-DT as a permanent on P0's battle area (the effect
    // source — `target: source` must resolve to a permanent), and TST-TOP
    // as the top card of P0's deck (the card to tuck under it).
    //
    // `DebugRunnerBuilder::deck` treats the LAST element as the top of the
    // deck, which is exactly what `CardSourceRef::DeckTop` pops.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-PABS-DT", "PlaceDeckTopBottom"))
        .add_card(make_test_card("TST-TOP", "DeckTopTarget"))
        .deck(0, &["TST-TOP"])
        .memory(5)
        .start();

    let source_perm = runner.place_on_field(0, "TST-PABS-DT", None);
    let src_card: CardHandle = runner.game.players[0].battle_area[source_perm.index as usize]
        .top_card()
        .handle();

    assert_eq!(
        runner.game.players[0].deck.len(),
        1,
        "deck should start with exactly the one seeded card"
    );
    let deck_top_handle: CardHandle = runner.game.players[0]
        .deck
        .last()
        .expect("deck top card")
        .handle();

    let dsl_effect = DslCardEffect::new(Arc::new(compiled));
    let effects = dsl_effect.effects(src_card);
    assert_eq!(effects.len(), 1, "exactly one OnPlay effect");
    let process = effects[0].process.as_ref().expect("OnPlay has a process");

    // Invoke the process. `place_as_bottom_source` with a deck-top source runs
    // synchronously — no `select_*` prompt is installed.
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, Some(source_perm), 0);
        process(&mut ctx);
    }
    assert!(
        runner.game.pending_selection.is_none(),
        "deck-top source needs no selection — the step is synchronous"
    );

    // The deck-top card was consumed: the deck shrank by 1.
    assert_eq!(
        runner.game.players[0].deck.len(),
        0,
        "deck-top card should have been consumed"
    );

    // The source permanent's stack grew by 1; the deck-top card now sits at
    // the BOTTOM (card_sources[0]) and is face-down.
    let perm = &runner.game.players[0].battle_area[source_perm.index as usize];
    assert_eq!(
        perm.card_sources.len(),
        2,
        "source permanent's stack should have grown by 1"
    );
    let bottom = &perm.card_sources[0];
    assert_eq!(
        bottom.handle(),
        deck_top_handle,
        "the deck-top card must be at the BOTTOM of the stack (card_sources[0])"
    );
    assert!(
        bottom.face_down,
        "face_down: true must place the tucked deck-top source face-down"
    );
}
