//! P-228 Unique Emblem: Frozen Crown — Option, Cost 2, Blue, [LIBERATOR].
//!
//! # Card text (`data/cards.json`, verified against the card image)
//! - [Main] Reveal the top 3 cards of your deck. Add 1 [Ice-Snow] trait Digimon
//!   card and 1 [LIBERATOR] trait card among them to the hand. Return the rest
//!   to the bottom of the deck. Then, place this card in the battle area.
//! - [Your Turn] When any of your [Suzune Kazuki]s are played, <Delay> (By
//!   trashing this card after the placing turn, activate the effect below.)
//!   1 of your Digimon may digivolve into a level 6 or lower [LIBERATOR] trait
//!   card in the hand with the digivolution cost reduced by 3.
//! - Security Effect [Security] Activate this card's [Main] effects.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Blue/P_228.cs
//! Note: DCGO's Ice-Snow bucket condition checks only `EqualsTraits("Ice-Snow")`
//! without `IsDigimon` (same simplification as its P_229 sibling), but the
//! printed text says "[Ice-Snow] trait **Digimon** card" — the printed text
//! governs, so the first bucket requires Digimon + Ice-Snow.
//!
//! # Patterns this test covers (sibling of P-229 "Unique Emblem: Narrative
//! Ronde" — same template card with Ice-Snow / [Suzune Kazuki] / blue)
//! - Option [Main] two-pass reveal search: `select_reveal_buckets` dual bucket
//!   ([Ice-Snow] Digimon + [LIBERATOR]) with `no_duplicate_cards`, bottom the
//!   rest.
//! - Inherited [Security] mirrors the [Main] search body, then places self.
//! - Battle-area placement of the resolving Option: the `kind: delay` clause
//!   makes the Option pipeline auto-seat P-228 as a Delayed Option after the
//!   [Main] body resolves.
//! - Event-gated <Delay> on a matching [Suzune Kazuki] play after the placing
//!   turn: trash-this-card cost, then optional digivolve of one of your Digimon
//!   into a level <= 6 [LIBERATOR] card from hand at cost -3.
//! - Negative gates: placing turn (16-16-3), non-Suzune plays, and an
//!   opponent-owned Suzune play on the opponent's turn ([Your Turn] + "any of
//!   YOUR [Suzune Kazuki]s") must all leave the Delay parked.
//! - Behavioral [Security] drive: a real security check runs the [Main] search
//!   for the DEFENDER and places P-228 in the defender's battle area via the
//!   security clause's explicit `place_self_as_delay_option`.

use std::path::Path;

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledCostDelta, CompiledDeclarativeClause,
    CompiledPlayerRef, CompiledPredicate, CompiledScope, CompiledStackPosition, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

fn p_228_yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/p/P-228.yaml"))
        .expect("P-228 YAML must exist at cards/p/P-228.yaml")
}

fn p_228_runner() -> DebugRunner {
    let yaml = p_228_yaml();
    DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-228 YAML must parse and compile")
        .memory(10)
        .start()
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn ice_snow_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Blue];
    card.traits = vec!["Ice-Snow".to_string()];
    card
}

fn liberator_option(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.colors = vec![CardColor::Blue];
    card.traits = vec!["LIBERATOR".to_string()];
    card
}

fn ice_snow_liberator_digimon(id: &str) -> CardData {
    let mut card = ice_snow_digimon(id);
    card.traits.push("LIBERATOR".to_string());
    card
}

/// A non-Digimon [Ice-Snow] trait card — must be rejected by the first bucket
/// (printed text requires an "[Ice-Snow] trait **Digimon** card").
fn ice_snow_option(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.colors = vec![CardColor::Blue];
    card.traits = vec!["Ice-Snow".to_string()];
    card
}

/// A Suzune Kazuki Tamer — the event card that gates P-228's <Delay>.
fn suzune(id: &str) -> CardData {
    let mut card = make_test_card(id, "Suzune Kazuki");
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 0;
    card.colors = vec![CardColor::Blue];
    card.traits = vec!["LIBERATOR".to_string()];
    card
}

/// A non-Suzune Tamer — used to prove the event predicate gates the <Delay>.
fn other_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, "Other Tamer");
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 0;
    card.colors = vec![CardColor::Blue];
    card
}

/// A level-3 blue Digimon — the digivolve base for the <Delay> body.
fn base_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Base Digimon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.colors = vec![CardColor::Blue];
    card
}

/// A level-4 [LIBERATOR] Digimon — a legal <Delay> digivolve target
/// (level <= 6, [LIBERATOR] trait, digivolves from a level-3 blue base).
fn liberator_evo(id: &str) -> CardData {
    let mut card = make_test_card(id, "Liberator Evo");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 5;
    card.colors = vec![CardColor::Blue];
    card.traits = vec!["LIBERATOR".to_string()];
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Blue as u8,
        level: 3,
        memory_cost: 3,
    }];
    card
}

/// A level-7 [LIBERATOR] Digimon — must be excluded by `level_lte: 6`.
fn high_liberator_evo(id: &str) -> CardData {
    let mut card = make_test_card(id, "High Liberator Evo");
    card.card_kind = CardKind::Digimon;
    card.level = Some(7);
    card.dp = Some(12000);
    card.play_cost = 12;
    card.colors = vec![CardColor::Blue];
    card.traits = vec!["LIBERATOR".to_string()];
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Blue as u8,
        level: 3,
        memory_cost: 3,
    }];
    card
}

fn predicate_has_ice_snow_digimon_filter(predicate: &CompiledPredicate) -> bool {
    predicate.all_of.iter().any(|part| {
        part.kind == Some(CompiledCardKind::Digimon)
            || part
                .all_of
                .iter()
                .any(|nested| nested.kind == Some(CompiledCardKind::Digimon))
    }) && predicate.all_of.iter().any(|part| {
        part.trait_has.as_deref() == Some("Ice-Snow")
            || part
                .all_of
                .iter()
                .any(|nested| nested.trait_has.as_deref() == Some("Ice-Snow"))
    })
}

fn predicate_contains_kind(predicate: &CompiledPredicate, kind: CompiledCardKind) -> bool {
    predicate.kind == Some(kind)
        || predicate
            .all_of
            .iter()
            .any(|part| predicate_contains_kind(part, kind))
        || predicate
            .any_of
            .iter()
            .any(|part| predicate_contains_kind(part, kind))
}

fn predicate_contains_trait(predicate: &CompiledPredicate, needle: &str) -> bool {
    predicate.trait_has.as_deref() == Some(needle)
        || predicate
            .all_of
            .iter()
            .any(|part| predicate_contains_trait(part, needle))
        || predicate
            .any_of
            .iter()
            .any(|part| predicate_contains_trait(part, needle))
}

fn predicate_has_level_lte(predicate: &CompiledPredicate) -> bool {
    predicate.level_lte.is_some()
        || predicate.all_of.iter().any(predicate_has_level_lte)
        || predicate.any_of.iter().any(predicate_has_level_lte)
}

fn battle_area_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.player(player).battle_area.iter().any(|perm| {
        perm.card_sources
            .iter()
            .any(|src| src.card_id(&runner.game.card_data) == card_id)
    })
}

fn hand_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner
        .game
        .player(player)
        .hand
        .iter()
        .any(|src| src.card_id(&runner.game.card_data) == card_id)
}

fn trash_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner
        .game
        .player(player)
        .trash
        .iter()
        .any(|src| src.card_id(&runner.game.card_data) == card_id)
}

fn hand_index_of(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner
        .game
        .player(player)
        .hand
        .iter()
        .position(|src| src.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

fn p_228_delayed_option(runner: &DebugRunner, player: u8) -> OptionState {
    runner
        .game
        .player(player)
        .battle_area
        .iter()
        .find(|perm| perm.top_card().card_id(&runner.game.card_data) == "P-228")
        .map(|perm| perm.option_state)
        .expect("P-228 should be parked in the battle area as a Delayed Option")
}

#[test]
fn p_228_yaml_parses_and_compiles() {
    let _runner = p_228_runner();
}

#[test]
fn p_228_is_blue_liberator_option_cost_2() {
    let runner = p_228_runner();
    let compiled = runner.compiled_card("P-228").expect("P-228 compiled");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.color, vec![CompiledColor::Blue]);
    assert_eq!(compiled.cost, Some(2));
    assert_eq!(compiled.traits, vec!["LIBERATOR".to_string()]);
}

#[test]
fn p_228_has_main_security_and_event_gated_delay_clauses() {
    let runner = p_228_runner();
    let compiled = runner.compiled_card("P-228").expect("P-228 compiled");

    assert_eq!(
        compiled.effects.len(),
        3,
        "Main search + inherited Security + event-gated Delay clauses"
    );

    let main = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("a MainFromHand search clause must exist");
    assert_eq!(main.scope, CompiledScope::FaceUp);
    assert!(!main.optional);

    let security = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("an inherited OnSecurity clause must exist");
    assert_eq!(security.scope, CompiledScope::Inherited);
    assert!(!security.optional);

    assert!(
        compiled.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay { .. })
        )),
        "an event-gated Delay clause must exist"
    );
}

#[test]
fn p_228_main_uses_dual_reveal_buckets_for_ice_snow_digimon_and_liberator() {
    let runner = p_228_runner();
    let compiled = runner.compiled_card("P-228").expect("P-228 compiled");
    let main = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("MainFromHand clause");

    assert!(matches!(
        main.process.first(),
        Some(CompiledStep::RevealTopDeck { count: 3, .. })
    ));

    let buckets = main
        .process
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectRevealBuckets {
                buckets,
                no_duplicate_cards,
                ..
            } => Some((buckets, no_duplicate_cards)),
            _ => None,
        })
        .expect("Main must use select_reveal_buckets");

    assert!(
        *buckets.1,
        "the same revealed card cannot satisfy both buckets"
    );
    assert_eq!(buckets.0.len(), 2);
    assert_eq!(buckets.0[0].bind_as, "ice_snow_pick");
    assert_eq!(buckets.0[0].min, 1);
    assert_eq!(buckets.0[0].max, 1);
    assert!(
        predicate_has_ice_snow_digimon_filter(
            buckets.0[0]
                .filter
                .as_ref()
                .expect("Ice-Snow bucket filter")
        ),
        "first bucket must require an [Ice-Snow] trait Digimon"
    );
    assert_eq!(buckets.0[1].bind_as, "liberator_pick");
    assert_eq!(buckets.0[1].min, 1);
    assert_eq!(buckets.0[1].max, 1);
    assert_eq!(
        buckets.0[1]
            .filter
            .as_ref()
            .and_then(|filter| filter.trait_has.as_deref()),
        Some("LIBERATOR"),
        "second bucket must accept [LIBERATOR] trait cards of any kind"
    );

    assert_eq!(
        main.process
            .iter()
            .filter(|step| matches!(step, CompiledStep::AddToHandFromReveal { .. }))
            .count(),
        2,
        "both bucket bindings must be added to hand"
    );
    assert!(main.process.iter().any(|step| matches!(
        step,
        CompiledStep::PlaceRemainderOnDeck {
            position: CompiledStackPosition::Bottom,
            ..
        }
    )));
}

#[test]
fn p_228_delay_is_event_gated_on_own_suzune_played() {
    let runner = p_228_runner();
    let compiled = runner.compiled_card("P-228").expect("P-228 compiled");
    let delay = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
                trigger,
                active_when,
                process,
                ..
            }) => Some((trigger, active_when, process)),
            _ => None,
        })
        .expect("Delay clause");

    // The <Delay> watches the OnAllyPlayed event, not a turn-keyed schedule.
    assert_eq!(*delay.0, CompiledTiming::OnAllyPlayed);
    let active_when = delay
        .1
        .as_ref()
        .expect("Delay must be gated on the Suzune Kazuki play event");
    assert_eq!(active_when.your_turn, Some(true));
    assert_eq!(active_when.event_target_owner, Some(CompiledPlayerRef::You));
    assert_eq!(
        active_when.event_card_name_contains.as_deref(),
        Some("Suzune Kazuki")
    );

    let own_perm = delay
        .2
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectOwnPermanent {
                filter, optional, ..
            } => Some((filter, optional)),
            _ => None,
        })
        .expect("Delay body must select a Digimon base");
    assert!(
        *own_perm.1,
        "printed Delay digivolve says '1 of your Digimon may'"
    );
    assert!(predicate_contains_kind(
        own_perm.0,
        CompiledCardKind::Digimon
    ));

    let evo = delay
        .2
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectHand { filter, .. } => Some(filter),
            _ => None,
        })
        .expect("Delay body must select the evolution card from hand");
    assert!(predicate_contains_kind(evo, CompiledCardKind::Digimon));
    assert!(predicate_contains_trait(evo, "LIBERATOR"));
    assert!(
        predicate_has_level_lte(evo),
        "the hand evolution card must be filtered to level 6 or lower"
    );

    assert!(delay.2.iter().any(|step| {
        matches!(
            step,
            CompiledStep::EffectInitiatedDigivolve {
                cost: CompiledCostDelta::Reduce(3),
                ignore_requirements: false,
                ..
            }
        )
    }));
}

#[test]
fn p_228_security_clause_mirrors_main_search_then_places_self() {
    let runner = p_228_runner();
    let compiled = runner.compiled_card("P-228").expect("P-228 compiled");
    let main = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("MainFromHand clause");
    let security = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("inherited OnSecurity clause");

    // Security "Activate this card's [Main] effects" re-runs the supported Main
    // search body, then appends the explicit battle-area placement (the
    // security pipeline does not auto-place Delay Options).
    assert_eq!(
        security.process.last(),
        Some(&CompiledStep::PlaceSelfAsDelayOption),
        "the security clause must place P-228 in the battle area"
    );
    assert_eq!(
        &security.process[..security.process.len() - 1],
        main.process.as_slice(),
        "the security search body must mirror the supported [Main] search"
    );
}

/// Behavioral [Security] drive — printed text: "[Security] Activate this
/// card's [Main] effects." When P-228 is checked from security, the DEFENDER
/// (the card's owner) reveals THEIR top 3, adds the [Ice-Snow] Digimon and
/// [LIBERATOR] picks to hand, bottoms the rest, and P-228 is placed in the
/// defender's battle area as the same OnAllyPlayed event-gated Delayed Option
/// (via the security clause's explicit `place_self_as_delay_option` — the
/// security pipeline does not auto-place Delay Options).
#[test]
fn p_228_security_check_activates_main_search_and_places_self_as_delay_option() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&p_228_yaml())
        .expect("P-228 YAML parses")
        .add_card(ice_snow_digimon("ICE"))
        .add_card(liberator_option("LIB"))
        .add_card(base_digimon("BASE"))
        .add_card(filler("FILL"))
        .security(1, &["P-228"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL", "LIB", "ICE"])
        .memory(0)
        .start();

    let attacker = runner.place_on_field(0, "BASE", Some(0));
    let hand_before = runner.game.player(1).hand.len();

    runner.attack_player(attacker, 1, false);
    runner
        .auto_resolve()
        .expect("resolve the security reveal buckets and battle-area placement");

    assert_eq!(
        runner.security_count(1),
        0,
        "P-228 must be checked and leave the security stack"
    );
    assert!(
        hand_contains(&runner, 1, "ICE"),
        "the defender adds the revealed [Ice-Snow] Digimon card to hand"
    );
    assert!(
        hand_contains(&runner, 1, "LIB"),
        "the defender adds the revealed [LIBERATOR] card to hand"
    );
    assert_eq!(
        runner.game.player(1).hand.len(),
        hand_before + 2,
        "exactly the two bucket picks are added; the rest returns to the deck"
    );
    assert!(
        !trash_contains(&runner, 1, "P-228"),
        "[Security] 'place this card in the battle area' — P-228 must not be trashed"
    );
    assert!(
        matches!(
            p_228_delayed_option(&runner, 1),
            OptionState::Delayed {
                trigger: DelayTrigger::OnEvent(EffectTiming::OnAllyPlayed),
                ..
            }
        ),
        "the security placement parks P-228 as the same OnAllyPlayed event-gated Delayed Option"
    );
}

#[test]
fn p_228_main_reveal_buckets_expose_mandatory_masked_choices() {
    let yaml = p_228_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-228 YAML parses")
        .add_card(ice_snow_liberator_digimon("BOTH"))
        .add_card(liberator_option("LIB-OPT"))
        .add_card(filler("FILL"))
        .hand(0, &["P-228"])
        .deck(0, &["FILL", "LIB-OPT", "BOTH"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    assert!(runner.game.activate_hand_main(0, 0));

    let first = runner
        .pending_selection_view()
        .expect("Ice-Snow bucket selection should be pending");
    assert_eq!(first.valid_action_ids.len(), 1);
    assert!(
        !first.valid_action_ids.contains(&PASS),
        "Ice-Snow bucket is mandatory when an eligible revealed card exists"
    );
    let first_action = first.valid_action_ids[0];
    runner
        .execute_action(first.selecting_player, first_action)
        .expect("choose Ice-Snow bucket card");

    let second = runner
        .pending_selection_view()
        .expect("LIBERATOR bucket selection should be pending");
    assert_eq!(
        second.valid_action_ids.len(),
        1,
        "the already-picked card must be excluded from the LIBERATOR bucket"
    );
    assert!(
        !second.valid_action_ids.contains(&PASS),
        "LIBERATOR bucket is mandatory when an eligible revealed card exists"
    );
}

#[test]
fn p_228_main_adds_distinct_ice_snow_digimon_and_liberator_card_to_hand() {
    let yaml = p_228_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-228 YAML parses")
        .add_card(ice_snow_liberator_digimon("BOTH"))
        .add_card(liberator_option("LIB-OPT"))
        .add_card(filler("FILL"))
        .hand(0, &["P-228"])
        .deck(0, &["FILL", "LIB-OPT", "BOTH"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let hand_before = runner.game.player(0).hand.len();
    let deck_before = runner.deck_size(0);

    assert!(runner.game.activate_hand_main(0, 0));
    runner
        .auto_resolve()
        .expect("resolve both reveal bucket choices");

    let hand_ids: Vec<_> = runner
        .game
        .player(0)
        .hand
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(hand_ids.iter().any(|id| id == "BOTH"), "hand={hand_ids:?}");
    assert!(
        hand_ids.iter().any(|id| id == "LIB-OPT"),
        "hand={hand_ids:?}"
    );
    assert_eq!(runner.game.player(0).hand.len(), hand_before + 2);
    assert_eq!(
        deck_before - runner.deck_size(0),
        2,
        "two revealed cards move to hand and the third returns to deck bottom"
    );
}

/// Printed-text fidelity vs DCGO: the first bucket requires an [Ice-Snow]
/// trait **Digimon** card. A revealed non-Digimon [Ice-Snow] card must not be
/// selectable for the first bucket (it may still satisfy the [LIBERATOR]
/// bucket only if it carries that trait — here it does not).
#[test]
fn p_228_main_ice_snow_bucket_rejects_non_digimon_ice_snow_cards() {
    let yaml = p_228_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-228 YAML parses")
        .add_card(ice_snow_option("ICE-OPT"))
        .add_card(liberator_option("LIB-OPT"))
        .add_card(filler("FILL"))
        .hand(0, &["P-228"])
        .deck(0, &["FILL", "LIB-OPT", "ICE-OPT"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let hand_before = runner.game.player(0).hand.len();

    assert!(runner.game.activate_hand_main(0, 0));
    runner
        .auto_resolve()
        .expect("resolve the reveal (Ice-Snow bucket empty, LIBERATOR bucket has one hit)");

    let hand_ids: Vec<_> = runner
        .game
        .player(0)
        .hand
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        !hand_ids.iter().any(|id| id == "ICE-OPT"),
        "a non-Digimon [Ice-Snow] card must not satisfy the Digimon bucket; hand={hand_ids:?}"
    );
    assert!(
        hand_ids.iter().any(|id| id == "LIB-OPT"),
        "the [LIBERATOR] card is still added; hand={hand_ids:?}"
    );
    assert_eq!(
        runner.game.player(0).hand.len(),
        hand_before + 1,
        "only the LIBERATOR pick is added when no [Ice-Snow] Digimon is revealed"
    );
}

/// "Then, place this card in the battle area" — after the [Main] reveal-search
/// resolves through the Option pipeline, P-228 is seated in the battle area as
/// an event-gated Delayed Option (`OnEvent(OnAllyPlayed)`), parked indefinitely
/// until a [Suzune Kazuki] is played.
#[test]
fn p_228_main_places_this_card_in_battle_area_as_event_gated_delay_option() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&p_228_yaml())
        .expect("P-228 YAML parses")
        .add_card(ice_snow_digimon("ICE"))
        .add_card(liberator_option("LIB"))
        .add_card(base_digimon("BASE"))
        .add_card(filler("FILL"))
        .hand(0, &["P-228"])
        .deck(0, &["FILL", "LIB", "ICE"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    // A blue permanent satisfies the Option color requirement for P-228.
    runner.place_on_field(0, "BASE", Some(0));
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending,
        "the [Main] reveal-search must park the Option pipeline"
    );
    runner
        .auto_resolve()
        .expect("resolve Main reveal search and battle-area placement");

    assert!(
        battle_area_contains(&runner, 0, "P-228"),
        "P-228 must be placed in the battle area as a delayed Option"
    );
    assert!(
        matches!(
            p_228_delayed_option(&runner, 0),
            OptionState::Delayed {
                trigger: DelayTrigger::OnEvent(EffectTiming::OnAllyPlayed),
                ..
            }
        ),
        "P-228 parks as an OnAllyPlayed event-gated Delayed Option"
    );
}

/// Drive the full event-gated <Delay>: place P-228, advance past the placing
/// turn, play a [Suzune Kazuki], and resolve the masked, optional digivolve —
/// trash P-228 as the activation cost, then digivolve one of your Digimon into
/// a level <= 6 [LIBERATOR] card from hand at digivolution cost -3.
#[test]
fn p_228_delay_triggers_after_suzune_is_played_and_offers_reduced_cost_digivolve() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&p_228_yaml())
        .expect("P-228 YAML parses")
        .add_card(ice_snow_digimon("ICE"))
        .add_card(liberator_option("LIB"))
        .add_card(suzune("SUZUNE"))
        .add_card(base_digimon("BASE"))
        .add_card(liberator_evo("EVO"))
        .add_card(high_liberator_evo("BIGEVO"))
        .add_card(filler("FILL"))
        .hand(0, &["P-228", "SUZUNE", "EVO", "BIGEVO"])
        .deck(
            0,
            &["FILL", "FILL", "FILL", "FILL", "FILL", "ICE", "LIB", "FILL"],
        )
        .deck(1, &["FILL"; 10])
        .memory(10)
        .start();

    runner.place_on_field(0, "BASE", Some(0));

    // [Main]: play P-228, resolve the reveal-search, auto-place as a Delay Option.
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner
        .auto_resolve()
        .expect("resolve Main reveal search and placement");
    assert!(
        matches!(
            p_228_delayed_option(&runner, 0),
            OptionState::Delayed {
                trigger: DelayTrigger::OnEvent(EffectTiming::OnAllyPlayed),
                ..
            }
        ),
        "P-228 parks as an OnAllyPlayed event-gated Delayed Option"
    );

    // Advance past the placing turn.
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();

    // Play a [Suzune Kazuki] — the gating OnAllyPlayed event.
    let suzune_idx = hand_index_of(&runner, 0, "SUZUNE");
    assert!(
        runner.game.play_from_hand(0, suzune_idx).is_some(),
        "Suzune Kazuki must be playable"
    );

    // The <Delay> timing is reached: first the optional ACTIVATION choice
    // (16-16-2) — accept it to activate the Delay.
    {
        let view = runner
            .pending_selection_view()
            .expect("playing Suzune Kazuki should offer P-228's Delay activation");
        assert_eq!(view.kind, SelectionKind::Replacement);
        assert!(
            runner.pending_is_optional(),
            "the <Delay> activation itself is optional (16-16-2)"
        );
        runner
            .execute_action(view.selecting_player, REPLACEMENT_ACCEPT)
            .expect("accept the Delay activation");
    }

    // Then the body: choose the digivolve base (only BASE is a Digimon).
    {
        let view = runner
            .pending_selection_view()
            .expect("accepting the Delay should expose the delayed digivolve");
        assert_eq!(view.kind, SelectionKind::OwnField);
        assert!(
            runner.pending_is_optional(),
            "the printed Delay digivolve is optional ('may')"
        );
        assert_eq!(
            view.valid_action_ids.len(),
            1,
            "only the Digimon base should be a legal digivolve source"
        );
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("choose the digivolve base");
    }

    // Then the hand evolution card — only the level-4 [LIBERATOR] Digimon
    // qualifies (ICE lacks [LIBERATOR], LIB is not a Digimon, BIGEVO is level 7).
    {
        let view = runner
            .pending_selection_view()
            .expect("choosing the base should expose the hand evolution selection");
        assert_eq!(view.kind, SelectionKind::Hand);
        assert_eq!(
            view.valid_action_ids.len(),
            1,
            "only the level <= 6 [LIBERATOR] Digimon card should be legal"
        );
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("choose the [LIBERATOR] evolution card");
    }

    runner.auto_resolve().expect("complete the Delay digivolve");

    assert!(
        !hand_contains(&runner, 0, "EVO"),
        "the selected evolution card should leave hand"
    );
    assert!(
        battle_area_contains(&runner, 0, "EVO"),
        "the selected evolution card should become the top of the field stack"
    );
    assert!(
        battle_area_contains(&runner, 0, "BASE"),
        "the base remains under the evolution card"
    );
    assert!(
        trash_contains(&runner, 0, "P-228"),
        "the <Delay> activation cost trashes P-228"
    );
    assert!(
        !battle_area_contains(&runner, 0, "P-228"),
        "P-228 leaves the battle area once the Delay activation cost is paid"
    );
}

/// Negative gate (placing turn): the `<Delay>` is only activatable "after the
/// placing turn" (rules manual 16-16-3). A [Suzune Kazuki] played on the same
/// turn P-228 was placed must NOT fire the Delay body.
#[test]
fn p_228_delay_does_not_fire_on_the_placing_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&p_228_yaml())
        .expect("P-228 YAML parses")
        .add_card(ice_snow_digimon("ICE"))
        .add_card(liberator_option("LIB"))
        .add_card(suzune("SUZUNE"))
        .add_card(base_digimon("BASE"))
        .add_card(liberator_evo("EVO"))
        .add_card(filler("FILL"))
        .hand(0, &["P-228", "SUZUNE", "EVO"])
        .deck(0, &["FILL", "FILL", "FILL", "ICE", "LIB", "FILL"])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    runner.place_on_field(0, "BASE", Some(0));
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner.auto_resolve().expect("resolve Main and placement");

    // Play Suzune Kazuki on the SAME turn P-228 was placed.
    let suzune_idx = hand_index_of(&runner, 0, "SUZUNE");
    assert!(runner.game.play_from_hand(0, suzune_idx).is_some());

    assert!(
        runner.pending_selection().is_none(),
        "the Delay must not fire on its placing turn"
    );
    assert!(
        battle_area_contains(&runner, 0, "P-228"),
        "P-228 stays parked when the Delay does not fire"
    );
    assert!(
        hand_contains(&runner, 0, "EVO"),
        "the evolution card stays in hand because the Delay body never ran"
    );
}

/// Negative gate (event predicate): the Delay is gated by
/// `event_card_name_contains: "Suzune Kazuki"`. Playing a non-Suzune card after
/// the placing turn must NOT fire the Delay body.
#[test]
fn p_228_delay_does_not_fire_when_a_non_suzune_is_played() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&p_228_yaml())
        .expect("P-228 YAML parses")
        .add_card(ice_snow_digimon("ICE"))
        .add_card(liberator_option("LIB"))
        .add_card(other_tamer("OTHER"))
        .add_card(base_digimon("BASE"))
        .add_card(liberator_evo("EVO"))
        .add_card(filler("FILL"))
        .hand(0, &["P-228", "OTHER", "EVO"])
        .deck(
            0,
            &["FILL", "FILL", "FILL", "FILL", "FILL", "ICE", "LIB", "FILL"],
        )
        .deck(1, &["FILL"; 10])
        .memory(10)
        .start();

    runner.place_on_field(0, "BASE", Some(0));
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner.auto_resolve().expect("resolve Main and placement");

    // Advance past the placing turn.
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();

    // Play a non-Suzune Tamer — the event predicate must reject it.
    let other_idx = hand_index_of(&runner, 0, "OTHER");
    assert!(runner.game.play_from_hand(0, other_idx).is_some());

    assert!(
        runner.pending_selection().is_none(),
        "the Delay must not fire when a non-Suzune card is played"
    );
    assert!(
        battle_area_contains(&runner, 0, "P-228"),
        "P-228 stays parked when the play event does not match"
    );
    assert!(
        hand_contains(&runner, 0, "EVO"),
        "the evolution card stays in hand because the Delay body never ran"
    );
}

/// Negative gate ([Your Turn] + ownership): the trigger is printed "[Your
/// Turn] When any of YOUR [Suzune Kazuki]s are played". The OPPONENT playing
/// their own [Suzune Kazuki] — necessarily on the opponent's turn, after the
/// placing turn — fails both the `your_turn` tag and the `event_target_owner:
/// you` gate, so P-228's Delay must NOT fire.
/// DCGO crosscheck: P_228.cs gates on `IsOwnerTurn(card)` and
/// `IsPermanentExistsOnOwnerBattleArea(permanent, card)`.
#[test]
fn p_228_delay_does_not_fire_when_opponent_plays_their_own_suzune() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&p_228_yaml())
        .expect("P-228 YAML parses")
        .add_card(ice_snow_digimon("ICE"))
        .add_card(liberator_option("LIB"))
        .add_card(suzune("SUZUNE"))
        .add_card(base_digimon("BASE"))
        .add_card(liberator_evo("EVO"))
        .add_card(filler("FILL"))
        .hand(0, &["P-228", "EVO"])
        .hand(1, &["SUZUNE"])
        .deck(0, &["FILL", "FILL", "FILL", "ICE", "LIB", "FILL"])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    runner.place_on_field(0, "BASE", Some(0));
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner.auto_resolve().expect("resolve Main and placement");

    // Pass the turn: it is now AFTER the placing turn (the 16-16-3 gate
    // passes), so only the [Your Turn] / event-owner gates can reject the
    // opponent's play.
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 1);
    runner.game.enter_main_phase();

    // The OPPONENT plays their own [Suzune Kazuki].
    let suzune_idx = hand_index_of(&runner, 1, "SUZUNE");
    assert!(
        runner.game.play_from_hand(1, suzune_idx).is_some(),
        "the opponent's Suzune Kazuki must be playable"
    );

    assert!(
        runner.pending_selection().is_none(),
        "an opponent-owned Suzune play on the opponent's turn must not fire the Delay"
    );
    assert!(
        battle_area_contains(&runner, 0, "P-228"),
        "P-228 stays parked when the play event fails the [Your Turn]/ownership gates"
    );
    assert!(
        !trash_contains(&runner, 0, "P-228"),
        "the Delay activation cost must not be paid"
    );
    assert!(
        hand_contains(&runner, 0, "EVO"),
        "the evolution card stays in hand because the Delay body never ran"
    );
}

/// Rules manual 16-16-2: "The processing from <Delay> is optional." When a
/// [Suzune Kazuki] play reaches the <Delay> timing, the controller must be
/// offered an ACTIVATION choice (activate = trash P-228 + run the body /
/// decline = do nothing). Declining must leave P-228 parked in the battle
/// area, un-trashed, and still able to fire on a LATER matching event.
/// DCGO crosscheck: P_228.cs registers the Delay trigger with
/// `SetUpActivateClass(..., isOptional: true, ...)` — the player is asked.
#[test]
fn p_228_delay_activation_is_optional_and_declining_leaves_it_parked() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&p_228_yaml())
        .expect("P-228 YAML parses")
        .add_card(ice_snow_digimon("ICE"))
        .add_card(liberator_option("LIB"))
        .add_card(suzune("SUZUNE"))
        .add_card(suzune("SUZUNE2"))
        .add_card(base_digimon("BASE"))
        .add_card(liberator_evo("EVO"))
        .add_card(filler("FILL"))
        .hand(0, &["P-228", "SUZUNE", "SUZUNE2", "EVO"])
        .deck(
            0,
            &["FILL", "FILL", "FILL", "FILL", "FILL", "ICE", "LIB", "FILL"],
        )
        .deck(1, &["FILL"; 10])
        .memory(10)
        .start();

    runner.place_on_field(0, "BASE", Some(0));
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner.auto_resolve().expect("resolve Main and placement");

    // Advance past the placing turn.
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();

    // Play a [Suzune Kazuki]. The <Delay> timing is reached — the engine must
    // ask "activate P-228's <Delay>?" rather than force the activation.
    let suzune_idx = hand_index_of(&runner, 0, "SUZUNE");
    assert!(runner.game.play_from_hand(0, suzune_idx).is_some());

    let view = runner
        .pending_selection_view()
        .expect("reaching the Delay timing must surface an activation choice");
    assert_eq!(
        view.kind,
        SelectionKind::Replacement,
        "the FIRST choice must be the outer <Delay> activation accept/decline, \
         not the body's digivolve pick"
    );
    assert!(
        runner.pending_is_optional(),
        "16-16-2: the <Delay> activation is optional — PASS must be legal"
    );
    assert_eq!(
        view.valid_action_ids,
        vec![REPLACEMENT_ACCEPT],
        "accept is the single non-PASS action"
    );

    // DECLINE the activation.
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("declining the Delay activation must be a legal action");
    runner.auto_resolve().expect("settle the declined Delay");

    // Declining leaves everything untouched: P-228 parked, nothing trashed,
    // no digivolve.
    assert!(
        battle_area_contains(&runner, 0, "P-228"),
        "declining the <Delay> must leave P-228 in the battle area"
    );
    assert!(
        !trash_contains(&runner, 0, "P-228"),
        "declining the <Delay> must not trash P-228"
    );
    assert!(
        matches!(
            p_228_delayed_option(&runner, 0),
            OptionState::Delayed {
                trigger: DelayTrigger::OnEvent(EffectTiming::OnAllyPlayed),
                ..
            }
        ),
        "P-228 stays parked as an event-gated Delayed Option after declining"
    );
    assert!(
        hand_contains(&runner, 0, "EVO"),
        "the evolution card stays in hand — the declined body never ran"
    );

    // The declined <Delay> re-arms: a LATER matching event offers the choice
    // again, and accepting drives the full activation (trash + digivolve).
    let suzune2_idx = hand_index_of(&runner, 0, "SUZUNE2");
    assert!(runner.game.play_from_hand(0, suzune2_idx).is_some());

    let view = runner
        .pending_selection_view()
        .expect("a later Suzune play must re-offer the declined Delay");
    assert_eq!(view.kind, SelectionKind::Replacement);
    runner
        .execute_action(view.selecting_player, REPLACEMENT_ACCEPT)
        .expect("accept the Delay activation");

    runner
        .auto_resolve()
        .expect("drive the accepted Delay body to completion");
    assert!(
        trash_contains(&runner, 0, "P-228"),
        "accepting the <Delay> trashes P-228 as the activation cost"
    );
    assert!(
        battle_area_contains(&runner, 0, "EVO"),
        "accepting the <Delay> lets the digivolve resolve"
    );
}

/// Cost-firing clause: the `<Delay>` activation cost is "by trashing this
/// card". When a [Suzune Kazuki] play fires the Delay after the placing turn,
/// P-228 is trashed even if the optional digivolve body is declined — the
/// trash is the activation cost, not part of the optional "may digivolve".
#[test]
fn p_228_delay_trashes_self_as_cost_even_when_digivolve_is_declined() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&p_228_yaml())
        .expect("P-228 YAML parses")
        .add_card(ice_snow_digimon("ICE"))
        .add_card(liberator_option("LIB"))
        .add_card(suzune("SUZUNE"))
        .add_card(base_digimon("BASE"))
        .add_card(liberator_evo("EVO"))
        .add_card(filler("FILL"))
        .hand(0, &["P-228", "SUZUNE", "EVO"])
        .deck(
            0,
            &["FILL", "FILL", "FILL", "FILL", "FILL", "ICE", "LIB", "FILL"],
        )
        .deck(1, &["FILL"; 10])
        .memory(10)
        .start();

    runner.place_on_field(0, "BASE", Some(0));
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner.auto_resolve().expect("resolve Main and placement");

    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();

    let suzune_idx = hand_index_of(&runner, 0, "SUZUNE");
    assert!(runner.game.play_from_hand(0, suzune_idx).is_some());

    // ACCEPT the (optional, 16-16-2) Delay activation itself...
    let view = runner
        .pending_selection_view()
        .expect("playing Suzune after the placing turn offers the Delay activation");
    assert_eq!(view.kind, SelectionKind::Replacement);
    runner
        .execute_action(view.selecting_player, REPLACEMENT_ACCEPT)
        .expect("accept the Delay activation");

    // ...then decline the optional digivolve body with PASS.
    let view = runner
        .pending_selection_view()
        .expect("accepting the Delay activation exposes the digivolve body");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(
        runner.pending_is_optional(),
        "the Delay digivolve is the printed 'may', so PASS is legal"
    );
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the Delay digivolve");
    runner.auto_resolve().expect("settle the declined Delay");

    // Body declined — no digivolve happened.
    assert!(
        hand_contains(&runner, 0, "EVO"),
        "declining the Delay leaves the evolution card in hand"
    );
    // ...but the activation cost still trashed P-228.
    assert!(
        trash_contains(&runner, 0, "P-228"),
        "the <Delay> cost trashes P-228 even when the digivolve is declined"
    );
    assert!(
        !battle_area_contains(&runner, 0, "P-228"),
        "P-228 must leave the battle area once the Delay activation cost is paid"
    );
}
