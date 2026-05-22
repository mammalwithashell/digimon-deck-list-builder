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

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::effect_context::EffectContext;

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
