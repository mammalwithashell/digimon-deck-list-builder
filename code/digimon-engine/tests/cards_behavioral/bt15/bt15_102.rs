//! BT15-102 Apocalymon — Digimon, Lv.7, White, cost 15, DP 15000,
//! [Unidentified].
//!
//! Printed clauses (official Bandai DB bundle; DCGO authority BT15_102.cs):
//!  (a) "When this card would be played, by placing up to 3 [Dark Masters]
//!      trait cards with different names from your battle area or trash
//!      under it, reduce the play cost by 4 for each one."
//!  (b) "[End of Your Turn] [Once Per Turn] By placing 1 level 6 or lower
//!      card from your trash as this Digimon's bottom digivolution card,
//!      activate 1 [On Play] effect on that card as an effect of this
//!      Digimon. Then, trash the top 2 cards of your opponent's deck for
//!      each of this Digimon's level 6 digivolution cards."
//!
//! Clause (a) rides the `cast_time_assembly` alt-path (substrate tests:
//! tests/cast_time_assembly.rs — optional-zero, distinct-name masking, the
//! DCGO CanNoSelect payability gate, clone-safety). The tests here pin the
//! REAL card's numbers and the DCGO resolution order.
//!
//! Clause (b) uses `select_trash` → `place_as_bottom_source.bind_placed_as`
//! → `refire_card_effect` (foreign-card refire; engine tests:
//! tests/effect_context/effect_refiring.rs) → formula `trash_from_top`.
//! DCGO facts pinned below: the trigger is MANDATORY (`isOptional: false`)
//! with a DECLINABLE placement pick (`canNoSelect: () => true`), and the
//! ". Then, trash…" mill runs even when the placement is declined.

use std::sync::{Arc, Mutex};

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledColor, CompiledDistinctBy, CompiledRepeat, CompiledZone,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, GamePhase};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;
use digimon_engine::{CardEffect, CardHandle, Effect};

const YAML: &str = include_str!("../../../cards/bt15/BT15-102.yaml");

fn dark_master(card_id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card_with_level(card_id, name, 6);
    card.traits = vec!["Dark Masters".to_string()];
    card
}

fn runner_builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT15-102 YAML compiles")
        .add_card(dark_master("DM-PIED", "Piedmon"))
        .add_card(dark_master("DM-METAL", "MetalSeadramon"))
        .add_card(dark_master("DM-MACHINE", "Machinedramon"))
        .add_card(dark_master("DM-PUPPET", "Puppetmon"))
        .add_card(dark_master("DM-PUPPET-ALT", "Puppetmon"))
        .add_card(make_test_card_with_level("VANILLA-3", "Vanilla Three", 3))
        .add_card(make_test_card_with_level("LEVEL-7", "Level Seven", 7))
        .add_card(deckfill("DECK-FILL"))
}

fn deckfill(card_id: &str) -> digimon_engine::card_data::CardData {
    make_test_card_with_level(card_id, "Deck Filler", 3)
}

fn tamer(card_id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}

fn apocalymon_perm(runner: &DebugRunner, player: u8) -> PermanentHandle {
    let idx = runner.game.players[player as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "BT15-102")
        .expect("Apocalymon on the field");
    PermanentHandle {
        player,
        index: idx as u8,
    }
}

fn source_ids_bottom_up(runner: &DebugRunner, handle: PermanentHandle) -> Vec<String> {
    runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn fire_end_of_your_turn(runner: &mut DebugRunner, carrier: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();
}

fn play_cost_paid(runner: &DebugRunner) -> Vec<i16> {
    runner
        .game
        .events()
        .iter()
        .filter_map(|e| match e {
            GameEvent::Play {
                card_id, cost_paid, ..
            } if card_id == "BT15-102" => Some(*cost_paid),
            _ => None,
        })
        .collect()
}

/// A level-6 card with one [On Play] effect that records its carrier.
#[derive(Clone)]
struct RecordingOnPlay {
    seen: Arc<Mutex<Vec<(CardHandle, Option<PermanentHandle>)>>>,
}

impl CardEffect for RecordingOnPlay {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = Arc::clone(&self.seen);
        vec![Effect::on_play(card)
            .name("recording on play")
            .process(move |ctx| {
                ctx.gain_memory(1);
                seen.lock()
                    .unwrap()
                    .push((ctx.source_card, ctx.source_permanent));
            })
            .build()]
    }
}

/// A level-3 card with TWO [On Play] effects (drives the EffectChoice arm).
#[derive(Clone)]
struct TwoOnPlayEffects;

impl CardEffect for TwoOnPlayEffects {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            Effect::on_play(card)
                .name("gain one")
                .process(|ctx| ctx.gain_memory(1))
                .build(),
            Effect::on_play(card)
                .name("gain three")
                .process(|ctx| ctx.gain_memory(3))
                .build(),
        ]
    }
}

// ─── Structure: the YAML compiles to the printed shape ──────────────────────

#[test]
fn bt15_102_yaml_compiles_with_cast_time_assembly_and_eot_clause() {
    let runner = runner_builder().start();
    let compiled = runner.compiled_card("BT15-102").expect("compiled card");

    assert_eq!(compiled.card, "BT15-102");
    assert_eq!(compiled.name, "Apocalymon");
    assert_eq!(compiled.cost, Some(15));
    assert_eq!(compiled.level, Some(7));
    assert_eq!(compiled.color, vec![CompiledColor::White]);
    assert!(compiled.traits.iter().any(|t| t == "Unidentified"));
    assert!(compiled.dp == Some(15000));

    let path = compiled
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::CastTimeAssembly)
        .expect("cast_time_assembly alt-path present");
    assert_eq!(path.materials.len(), 1);
    let material = &path.materials[0];
    assert_eq!(material.filter.trait_has.as_deref(), Some("Dark Masters"));
    assert_eq!(
        material.repeat,
        Some(CompiledRepeat::Range { min: 0, max: 3 }),
        "up to 3 — zero placements legal"
    );
    assert_eq!(material.distinct_by, Some(CompiledDistinctBy::Name));
    assert_eq!(material.cost_delta, Some(-4));
    assert!(material.zones.contains(&CompiledZone::BattleArea));
    assert!(material.zones.contains(&CompiledZone::Trash));
}

// ─── Clause (a): cast-time stack construction ───────────────────────────────

#[test]
fn bt15_102_play_with_three_dark_masters_pays_three_and_stacks_them_under() {
    let mut runner = runner_builder().hand(0, &["BT15-102"]).memory(10).start();
    runner.inject_trash(0, "DM-PIED");
    runner.inject_trash(0, "DM-METAL");
    let machine = runner.place_stack(0, &["VANILLA-3", "DM-MACHINE"]);

    assert_eq!(runner.game.play_from_hand(0, 0), None, "parks on assembly");
    assert_eq!(runner.current_phase(), GamePhase::SelectMaterial);

    runner
        .execute_action(0, machine.index as u16)
        .expect("place the battle-area Machinedramon (top card only)");
    runner
        .execute_action(0, TRASH_EFFECT_START)
        .expect("place Piedmon from trash");
    runner
        .execute_action(0, TRASH_EFFECT_START + 1)
        .expect("place MetalSeadramon from trash");
    runner.execute_action(0, PASS).expect("finish at the cap");

    assert_eq!(runner.memory(), 7, "cost 15 - 3×4 = 3 paid from memory 10");
    assert_eq!(play_cost_paid(&runner), vec![3]);

    let apoc = apocalymon_perm(&runner, 0);
    let sources = source_ids_bottom_up(&runner, apoc);
    assert_eq!(sources.last().map(String::as_str), Some("BT15-102"));
    assert_eq!(sources.len(), 4, "three placed cards under the top");
    for id in ["DM-MACHINE", "DM-PIED", "DM-METAL"] {
        assert!(sources.contains(&id.to_string()), "{id} under Apocalymon");
    }
    // The consumed permanent's own digivolution card goes to trash
    // (general_rule.pdf leave-field handling; DCGO material take).
    assert_eq!(
        runner.game.players[0]
            .trash
            .iter()
            .map(|c| c.card_id(&runner.game.card_data).to_string())
            .collect::<Vec<_>>(),
        vec!["VANILLA-3".to_string()]
    );
}

#[test]
fn bt15_102_with_no_dark_masters_plays_synchronously_at_full_cost() {
    let mut runner = runner_builder().hand(0, &["BT15-102"]).memory(10).start();
    runner.inject_trash(0, "VANILLA-3"); // not a Dark Masters — no candidate

    let played = runner.game.play_from_hand(0, 0);
    assert!(
        played.is_some(),
        "no candidates → no assembly selection → synchronous play"
    );
    // Synchronous `play_from_hand` pays the full 15 (10 → -5); the runner
    // loop's check_turn_end (not the raw play API) owns the turn handoff.
    assert_eq!(runner.memory(), -5, "full cost 15 paid from memory 10");
    assert_eq!(play_cost_paid(&runner), vec![15]);
    let apoc = apocalymon_perm(&runner, 0);
    assert_eq!(source_ids_bottom_up(&runner, apoc).len(), 1);
}

#[test]
fn bt15_102_decline_plays_at_full_cost_and_touches_nothing() {
    let mut runner = runner_builder().hand(0, &["BT15-102"]).memory(10).start();
    runner.inject_trash(0, "DM-PIED");

    assert_eq!(runner.game.play_from_hand(0, 0), None);
    let sel = runner.pending_selection().expect("assembly selection");
    assert!(sel.is_optional, "payable at 10 → placing zero is legal");
    runner.execute_action(0, PASS).expect("decline");

    assert_eq!(play_cost_paid(&runner), vec![15]);
    assert_eq!(runner.trash_size(0), 1, "Piedmon stays in trash");
    let apoc = apocalymon_perm(&runner, 0);
    assert_eq!(source_ids_bottom_up(&runner, apoc).len(), 1);
}

#[test]
fn bt15_102_same_named_dark_masters_cannot_both_be_placed() {
    let mut runner = runner_builder().hand(0, &["BT15-102"]).memory(10).start();
    runner.inject_trash(0, "DM-PUPPET");
    runner.inject_trash(0, "DM-PUPPET-ALT");

    assert_eq!(runner.game.play_from_hand(0, 0), None);
    runner
        .execute_action(0, TRASH_EFFECT_START)
        .expect("place the first Puppetmon");
    let sel = runner.pending_selection().expect("selection re-installs");
    assert!(
        sel.valid_action_ids.is_empty(),
        "the second same-named Puppetmon must be masked out (different names): {:?}",
        sel.valid_action_ids
    );
    runner.execute_action(0, PASS).expect("finish");
    assert_eq!(play_cost_paid(&runner), vec![11], "only one -4 applied");
}

#[test]
fn bt15_102_mask_offers_play_at_memory_zero_only_via_potential_reduction() {
    // 15 is unpayable from memory 0 (0-15 < -10). With 3 distinct Dark
    // Masters the potential -12 makes it offerable (declare-then-pay).
    let mut runner = runner_builder().hand(0, &["BT15-102"]).memory(0).start();
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 0.0,
        "no reduction available"
    );

    runner.inject_trash(0, "DM-PIED");
    runner.inject_trash(0, "DM-METAL");
    runner.inject_trash(0, "DM-MACHINE");
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[PLAY_HAND_START as usize], 1.0);

    // DCGO CanNoSelect: cannot stop while the remaining cost is unpayable.
    assert_eq!(runner.game.play_from_hand(0, 0), None);
    assert!(!runner.pending_selection().unwrap().is_optional);
    runner.execute_action(0, TRASH_EFFECT_START).expect("1st");
    assert!(!runner.pending_selection().unwrap().is_optional, "cost 11");
    let next = *runner
        .pending_selection()
        .unwrap()
        .valid_action_ids
        .first()
        .unwrap();
    runner.execute_action(0, next).expect("2nd");
    assert!(
        runner.pending_selection().unwrap().is_optional,
        "cost 7 payable from 0"
    );
    runner.execute_action(0, PASS).expect("stop at two");
    assert_eq!(play_cost_paid(&runner), vec![7]);
}

// ─── Clause (b): [End of Your Turn] [Once Per Turn] ─────────────────────────

#[test]
fn bt15_102_eot_places_bottom_refires_on_play_as_apocalymon_and_mills_per_lv6() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut runner = runner_builder()
        .add_card(dark_master("EOT-ONPLAY", "Refire Fodder"))
        .deck(
            1,
            &[
                "DECK-FILL",
                "DECK-FILL",
                "DECK-FILL",
                "DECK-FILL",
                "DECK-FILL",
                "DECK-FILL",
                "DECK-FILL",
                "DECK-FILL",
            ],
        )
        .memory(0)
        .start();
    runner.register_effect(
        "EOT-ONPLAY",
        Arc::new(RecordingOnPlay {
            seen: Arc::clone(&seen),
        }),
    );
    // Apocalymon with two level-6 digivolution cards already under it.
    let apoc = runner.place_stack(0, &["DM-PIED", "DM-METAL", "BT15-102"]);
    let apoc_top = runner.top_card(apoc);
    runner.inject_trash(0, "EOT-ONPLAY");
    let opp_deck_before = runner.game.players[1].deck.len();
    let opp_trash_before = runner.game.players[1].trash.len();

    fire_end_of_your_turn(&mut runner, apoc);

    let sel = runner.pending_selection().expect("trash placement pick");
    assert!(
        sel.is_optional,
        "DCGO canNoSelect: () => true — the placement pick is declinable"
    );
    let pick = *sel.valid_action_ids.first().expect("EOT-ONPLAY selectable");
    runner.execute_action(0, pick).expect("place from trash");

    // Placed as the BOTTOM digivolution card.
    let sources = source_ids_bottom_up(&runner, apoc);
    assert_eq!(
        sources.first().map(String::as_str),
        Some("EOT-ONPLAY"),
        "placed card must be the BOTTOM source: {sources:?}"
    );
    assert_eq!(runner.trash_size(0), 0, "the card left the trash");

    // Its [On Play] ran as an effect of THIS Digimon: the body saw
    // Apocalymon as source card + permanent.
    assert_eq!(
        *seen.lock().unwrap(),
        vec![(apoc_top, Some(apoc))],
        "the refired body reads Apocalymon as 'this Digimon'"
    );

    // Then: 3 level-6 digivolution cards (Piedmon, MetalSeadramon, and the
    // just-placed level-6 fodder) → trash 6 from the opponent's deck top.
    assert_eq!(
        runner.game.players[1].deck.len(),
        opp_deck_before - 6,
        "2 × 3 lv6 digivolution cards milled"
    );
    assert_eq!(runner.game.players[1].trash.len(), opp_trash_before + 6);
    assert_eq!(runner.memory(), 1, "the recorder's gain_memory(1) applied");
}

#[test]
fn bt15_102_eot_decline_still_mills_per_printed_then_clause() {
    // DCGO: the deck-trash runs after the `if (selectedCard != null)` block —
    // declining the placement does NOT skip the ". Then, trash…" clause.
    let mut runner = runner_builder()
        .deck(
            1,
            &[
                "DECK-FILL",
                "DECK-FILL",
                "DECK-FILL",
                "DECK-FILL",
                "DECK-FILL",
            ],
        )
        .memory(0)
        .start();
    let apoc = runner.place_stack(0, &["DM-PIED", "DM-METAL", "BT15-102"]);
    runner.inject_trash(0, "VANILLA-3"); // a lv3 candidate exists → clause fires
    let opp_deck_before = runner.game.players[1].deck.len();

    fire_end_of_your_turn(&mut runner, apoc);
    let sel = runner.pending_selection().expect("placement pick");
    assert!(sel.is_optional);
    runner
        .execute_action(0, PASS)
        .expect("decline the placement");

    assert_eq!(runner.trash_size(0), 1, "nothing placed");
    assert_eq!(
        source_ids_bottom_up(&runner, apoc).len(),
        3,
        "no new digivolution card"
    );
    assert_eq!(
        runner.game.players[1].deck.len(),
        opp_deck_before - 4,
        "2 × 2 lv6 digivolution cards still milled on decline"
    );
}

#[test]
fn bt15_102_eot_does_not_fire_without_a_level_six_or_lower_trash_card() {
    let mut runner = runner_builder()
        .add_card(tamer("TAMER-NL", "Levelless Tamer"))
        .deck(1, &["DECK-FILL", "DECK-FILL", "DECK-FILL", "DECK-FILL"])
        .memory(0)
        .start();
    let apoc = runner.place_stack(0, &["DM-PIED", "DM-METAL", "BT15-102"]);
    // Trash holds only a level 7 card and a level-less Tamer — no candidate.
    runner.inject_trash(0, "LEVEL-7");
    runner.inject_trash(0, "TAMER-NL");
    let opp_deck_before = runner.game.players[1].deck.len();

    fire_end_of_your_turn(&mut runner, apoc);

    assert!(
        runner.pending_selection().is_none(),
        "no eligible trash card → the clause must not fire at all"
    );
    assert_eq!(
        runner.game.players[1].deck.len(),
        opp_deck_before,
        "no mill either — the whole effect is gated on a trash candidate \
         (DCGO CanActivateCondition)"
    );
}

#[test]
fn bt15_102_eot_pick_excludes_level_seven_and_levelless_cards() {
    let mut runner = runner_builder()
        .add_card(tamer("TAMER-NL", "Levelless Tamer"))
        .deck(1, &["DECK-FILL", "DECK-FILL"])
        .memory(0)
        .start();
    let apoc = runner.place_stack(0, &["DM-PIED", "BT15-102"]);
    runner.inject_trash(0, "LEVEL-7");
    runner.inject_trash(0, "TAMER-NL");
    runner.inject_trash(0, "VANILLA-3");

    fire_end_of_your_turn(&mut runner, apoc);
    let sel = runner.pending_selection().expect("placement pick");
    assert_eq!(
        sel.valid_action_ids,
        vec![TRASH_EFFECT_START + 2],
        "only the level-3 card qualifies ('level 6 or lower' requires a level)"
    );
}

#[test]
fn bt15_102_eot_is_once_per_turn() {
    let mut runner = runner_builder()
        .deck(
            1,
            &[
                "DECK-FILL",
                "DECK-FILL",
                "DECK-FILL",
                "DECK-FILL",
                "DECK-FILL",
            ],
        )
        .memory(0)
        .start();
    let apoc = runner.place_stack(0, &["DM-PIED", "BT15-102"]);
    runner.inject_trash(0, "VANILLA-3");
    runner.inject_trash(0, "DM-MACHINE");

    fire_end_of_your_turn(&mut runner, apoc);
    let pick = *runner
        .pending_selection()
        .expect("first activation")
        .valid_action_ids
        .first()
        .unwrap();
    runner.execute_action(0, pick).expect("resolve first");
    let deck_after_first = runner.game.players[1].deck.len();

    // Same turn, a second EndOfYourTurn dispatch must be OPT-locked.
    fire_end_of_your_turn(&mut runner, apoc);
    assert!(
        runner.pending_selection().is_none(),
        "[Once Per Turn] — no second activation this turn"
    );
    assert_eq!(
        runner.game.players[1].deck.len(),
        deck_after_first,
        "no second mill"
    );
}

#[test]
fn bt15_102_eot_two_on_play_effects_surface_mandatory_choice_then_mill_runs() {
    let mut two_onplay = make_test_card_with_level("EOT-TWO", "Two Effect Fodder", 3);
    two_onplay.traits = vec![];
    let mut runner = runner_builder()
        .add_card(two_onplay)
        .deck(
            1,
            &[
                "DECK-FILL",
                "DECK-FILL",
                "DECK-FILL",
                "DECK-FILL",
                "DECK-FILL",
            ],
        )
        .memory(0)
        .start();
    runner.register_effect("EOT-TWO", Arc::new(TwoOnPlayEffects));
    let apoc = runner.place_stack(0, &["DM-PIED", "DM-METAL", "BT15-102"]);
    runner.inject_trash(0, "EOT-TWO");
    let opp_deck_before = runner.game.players[1].deck.len();

    fire_end_of_your_turn(&mut runner, apoc);
    let pick = *runner
        .pending_selection()
        .expect("placement pick")
        .valid_action_ids
        .first()
        .unwrap();
    runner.execute_action(0, pick).expect("place EOT-TWO");

    let sel = runner.pending_selection().expect("effect choice");
    assert!(
        !sel.is_optional,
        "choosing which [On Play] to activate is mandatory once placed \
         (DCGO canNoSelect: () => false)"
    );
    let choices = sel.effect_choices.as_ref().expect("choice entries");
    assert_eq!(choices.len(), 2);
    runner
        .execute_action(0, HAND_EFFECT_START + 1)
        .expect("activate the second effect");
    assert_eq!(runner.memory(), 3, "'gain three' executed");

    // The ". Then," mill composes AFTER the choice resolves; the placed
    // fodder is level 3, so only Piedmon + MetalSeadramon count → 4 cards.
    assert_eq!(
        runner.game.players[1].deck.len(),
        opp_deck_before - 4,
        "2 × 2 lv6 digivolution cards (the lv3 placed card does not count)"
    );
}

#[test]
fn bt15_102_eot_clone_mid_placement_pick_resolves_on_the_clone() {
    let mut runner = runner_builder()
        .deck(
            1,
            &[
                "DECK-FILL",
                "DECK-FILL",
                "DECK-FILL",
                "DECK-FILL",
                "DECK-FILL",
            ],
        )
        .memory(0)
        .start();
    let apoc = runner.place_stack(0, &["DM-PIED", "DM-METAL", "BT15-102"]);
    runner.inject_trash(0, "VANILLA-3");

    fire_end_of_your_turn(&mut runner, apoc);
    let pick = *runner
        .pending_selection()
        .expect("placement pick")
        .valid_action_ids
        .first()
        .unwrap();

    let mut cloned = runner.game.clone();
    cloned
        .resolve_selection(0, pick)
        .expect("clone resolves the placement via the resumable VM");
    let clone_apoc = &cloned.players[0].battle_area[apoc.index as usize];
    assert_eq!(
        clone_apoc.card_sources[0].card_id(&cloned.card_data),
        "VANILLA-3",
        "clone placed the bottom source"
    );
    assert_eq!(cloned.players[1].deck.len(), 1, "clone milled 2 × 2 lv6");

    // The original is untouched and still resolvable.
    assert_eq!(runner.trash_size(0), 1);
    assert_eq!(runner.game.players[1].deck.len(), 5);
    runner.execute_action(0, pick).expect("original still live");
    assert_eq!(runner.game.players[1].deck.len(), 1);
}
