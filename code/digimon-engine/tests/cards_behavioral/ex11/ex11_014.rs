//! EX11-014 Penguinmon — Digimon, Lv.3, Blue/Yellow, DP 2000, Cost 3.
//! Traits: Avian, LIBERATOR (+ rule-box [Ice-Snow]). Attribute: Vaccine.
//! Form: Rookie. Rarity: C.
//!
//! # Card text (card image EX11-014.webp + official Bandai DB bundle
//! # data/card_bundles/EX11-014.md, authoritative)
//!
//! Evo box: Lv.2 from blue OR yellow: Cost 1 — a blue/yellow SPLIT circle
//! (official DB lists both "Blue Lv.2 / cost 1" and "Yellow Lv.2 / cost 1";
//! cards.json `evo_costs` lossily carries only the blue half).
//! Evo box: "Digivolve [Hiyarimon]: Cost 0" (cards.json `xros_req`).
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 [Suzune Kazuki] and
//! 1 [Ice-Snow] trait Digimon card among them to the hand. Return the rest
//! to the bottom of the deck.
//!
//! Rule: Trait: Has [Ice-Snow] Type.
//!
//! Inherited Effect: <Jamming> (This Digimon can't be deleted in battles
//! against Security Digimon.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX11/Blue/EX11_014.cs
//!   - Alt digivolution requirement (None): AddSelfDigivolutionRequirement-
//!     StaticEffect(permanentCondition: TopCard.EqualsCardName("Hiyarimon"),
//!     digivolutionCost: 0) — EXACT name gate (`name_is`), no level/color.
//!   - On Play (OnEnterFieldAnyone): SimplifiedRevealDeckTopCardsAndSelect(
//!     revealCount: 3,
//!     conditions[0]: EqualsCardName("Suzune Kazuki"), AddHand, maxCount 1;
//!     conditions[1]: EqualsTraits("Ice-Snow") && IsDigimon, AddHand,
//!       maxCount 1;
//!     remainingCardsPlace: DeckBottom). canNoSelect defaults to FALSE: each
//!     add is MANDATORY whenever a matching card was revealed (no printed
//!     "you may"). Selections run sequentially and a picked card leaves the
//!     pool — one card can satisfy only one bucket (`no_duplicate_cards`).
//!   - Inherited <Jamming> (None): JammingSelfStaticEffect(
//!     isInheritedEffect: true).
//!
//! # Patterns this test file covers
//! - Reveal-3 two-bucket pick (Group E): exact-name bucket + kind+trait
//!   bucket, `no_duplicate_cards`, remainder to deck bottom — BT25-022 /
//!   P-228 idioms.
//! - Mandatory bucket picks (no PASS in the mask) — P-228 §mandatory idiom.
//! - Inherited static keyword grant (<Jamming>) verified in a real losing
//!   security-Digimon battle — security_effects.rs combat idiom.
//! - Alt digivolve recipe registration (standard Lv.2 blue + Lv.2 yellow
//!   split halves + [Hiyarimon]) and the yellow-half battle-area digivolve.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_digivolve, BREEDING_TARGET, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword, PlaySource};

const CARD_ID: &str = "EX11-014";

// ─── Card factories ───────────────────────────────────────────────────────────

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A [Suzune Kazuki] Tamer — bucket 0's exact-name target.
fn suzune(id: &str) -> CardData {
    let mut c = make_test_card(id, "Suzune Kazuki");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["LIBERATOR".to_string()];
    c
}

/// A card whose name merely CONTAINS "Suzune Kazuki" — must NOT satisfy
/// bucket 0 (DCGO uses EqualsCardName, an exact-name match).
fn near_name_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, "Suzune Kazuki Drone");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c.colors = vec![CardColor::Blue];
    c
}

/// An [Ice-Snow] trait Digimon — bucket 1's target.
fn ice_snow_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Ice-Snow".to_string()];
    c
}

/// A non-Digimon [Ice-Snow] card — must NOT satisfy bucket 1 (printed text
/// requires an "[Ice-Snow] trait **Digimon** card"; DCGO checks IsDigimon).
fn ice_snow_option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Ice-Snow".to_string()];
    c
}

/// A synthetic card matching BOTH buckets (exact name "Suzune Kazuki" AND an
/// [Ice-Snow] Digimon) — proves `no_duplicate_cards`: one revealed card can
/// fill only one bucket.
fn both_buckets(id: &str) -> CardData {
    let mut c = make_test_card(id, "Suzune Kazuki");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Ice-Snow".to_string()];
    c
}

/// A plain carrier Digimon to stack EX11-014 under (inherited Jamming test).
fn carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(3000);
    c.colors = vec![CardColor::Blue];
    c
}

/// A high-DP security Digimon the carrier loses the DP battle against.
fn digimon_security(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(dp);
    c.colors = vec![CardColor::Red];
    c
}

// ─── Section 1: Structural ──────────────────────────────────────────────────

#[test]
fn ex11_014_has_on_play_reveal_two_mandatory_buckets_rest_to_bottom() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-014 YAML loads from embedded DSL pack")
        .build();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("EX11-014 compiled card present");

    let on_play = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("EX11-014 must have an [On Play] clause");

    let has_reveal = on_play
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::RevealTopDeck { .. }));
    assert!(has_reveal, "must reveal the top of the deck");

    let buckets: Vec<_> = on_play
        .process
        .iter()
        .filter_map(|s| match s {
            CompiledStep::SelectRevealBuckets {
                buckets,
                no_duplicate_cards,
                ..
            } => Some((buckets, no_duplicate_cards)),
            _ => None,
        })
        .collect();
    assert_eq!(buckets.len(), 1, "exactly one select_reveal_buckets step");
    let (bucket_list, no_dup) = &buckets[0];
    assert_eq!(
        bucket_list.len(),
        2,
        "two buckets: [Suzune Kazuki] name + [Ice-Snow] Digimon"
    );
    for b in bucket_list.iter() {
        assert_eq!(
            (b.min, b.max),
            (1, 1),
            "each add is mandatory when candidates exist (DCGO canNoSelect=false): \
             bucket {} must be min 1 / max 1",
            b.bind_as
        );
    }
    assert!(
        **no_dup,
        "select_reveal_buckets must set no_duplicate_cards (sequential DCGO picks \
         remove the card from the pool)"
    );

    let add_to_hand_count = on_play
        .process
        .iter()
        .filter(|s| matches!(s, CompiledStep::AddToHandFromReveal { .. }))
        .count();
    assert_eq!(add_to_hand_count, 2, "must add both picks to hand");

    let has_place_remainder = on_play
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlaceRemainderOnDeck { .. }));
    assert!(
        has_place_remainder,
        "rest returns to the bottom of the deck"
    );
}

#[test]
fn ex11_014_has_inherited_jamming_grant() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-014 YAML loads from embedded DSL pack")
        .build();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("EX11-014 compiled card present");

    let jamming = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            scope,
            ..
        }) => *scope == CompiledScope::Inherited && keyword.eq_ignore_ascii_case("Jamming"),
        _ => false,
    });
    assert!(jamming, "EX11-014 must grant inherited <Jamming>");
}

#[test]
fn ex11_014_registers_standard_and_hiyarimon_digivolve_paths() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-014 YAML loads from embedded DSL pack")
        .build();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("EX11-014 compiled card present");

    let digivolve_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .collect();
    assert_eq!(
        digivolve_paths.len(),
        3,
        "EX11-014 must register BOTH halves of the printed blue/yellow split \
         circle (official DB: Blue Lv.2/1 + Yellow Lv.2/1) plus the \
         [Hiyarimon] cost-0 alt path"
    );

    // Cost table: two cost-1 circles (the split halves) + one cost-0 alt.
    let mut costs: Vec<i32> = digivolve_paths
        .iter()
        .map(|p| match p.cost.as_ref() {
            Some(CompiledCost::Literal(v)) => *v,
            other => panic!("EX11-014 digivolve paths must carry literal costs, got {other:?}"),
        })
        .collect();
    costs.sort_unstable();
    assert_eq!(
        costs,
        vec![0, 1, 1],
        "printed cost table: [Hiyarimon] cost 0 + blue Lv.2 cost 1 + yellow \
         Lv.2 cost 1"
    );
}

/// Rule box: "Trait: Has [Ice-Snow] Type." — the spec's `traits:` must carry
/// Ice-Snow in addition to the printed Avian / LIBERATOR type line.
#[test]
fn ex11_014_traits_include_rule_box_ice_snow() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-014 YAML loads from embedded DSL pack")
        .build();
    let data = runner
        .game
        .card_data
        .iter()
        .find(|c| c.card_id == CARD_ID)
        .expect("EX11-014 registered in card_data");

    for t in ["Avian", "LIBERATOR", "Ice-Snow"] {
        assert!(
            data.traits.iter().any(|x| x == t),
            "EX11-014 traits must include {t}; got {:?}",
            data.traits
        );
    }
}

// ─── Section 2: Behavioral — [On Play] reveal + buckets ────────────────────

/// Helper: a runner with EX11-014 in hand and a controlled deck top.
/// `deck`: last element = top of deck.
fn on_play_runner(extra: Vec<CardData>, deck_top_down: &[&str]) -> DebugRunner {
    // .deck() wants bottom-first; reverse the top-down listing.
    let mut deck: Vec<&str> = vec!["PAD-1", "PAD-2", "PAD-3"];
    deck.extend(deck_top_down.iter().rev());

    let mut b = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-014 YAML loads from embedded DSL pack")
        .add_card(filler("PAD-1"))
        .add_card(filler("PAD-2"))
        .add_card(filler("PAD-3"));
    for c in extra {
        b = b.add_card(c);
    }
    b.hand(0, &[CARD_ID]).deck(0, &deck).memory(10).start()
}

fn hand_ids(runner: &DebugRunner) -> Vec<String> {
    runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

/// Mixed reveal — one [Suzune Kazuki], one [Ice-Snow] Digimon, one filler:
/// both matches are added to hand, the filler returns to the deck bottom.
#[test]
fn ex11_014_on_play_adds_suzune_and_ice_snow_digimon_rest_to_bottom() {
    let mut runner = on_play_runner(
        vec![suzune("SUZ"), ice_snow_digimon("ICE"), filler("FILL")],
        &["SUZ", "ICE", "FILL"],
    );
    let deck_before = runner.game.players[0].deck.len();

    assert!(
        runner.game.play_from_hand(0, 0).is_some(),
        "EX11-014 must be playable from hand"
    );
    runner.auto_resolve().expect("resolve both bucket picks");

    let hand = hand_ids(&runner);
    assert!(
        hand.iter().any(|id| id == "SUZ"),
        "Suzune to hand: {hand:?}"
    );
    assert!(
        hand.iter().any(|id| id == "ICE"),
        "Ice-Snow Digimon to hand: {hand:?}"
    );
    assert!(
        !hand.iter().any(|id| id == "FILL"),
        "non-matching filler must NOT be added: {hand:?}"
    );
    // 3 revealed, 2 to hand, 1 back to the bottom.
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before - 2,
        "two picks leave the deck, the filler returns to the bottom"
    );
    // The filler is the new bottom card (index 0).
    assert_eq!(
        runner.game.players[0].deck[0].card_id(&runner.game.card_data),
        "FILL",
        "the unpicked card returns to the BOTTOM of the deck"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no selection remains after resolution"
    );
}

/// Both bucket selections are MANDATORY when candidates exist: PASS must not
/// be in the legal action mask for either pick (DCGO canNoSelect = false; no
/// printed "you may").
#[test]
fn ex11_014_on_play_bucket_picks_are_mandatory_no_pass() {
    let mut runner = on_play_runner(
        vec![suzune("SUZ"), ice_snow_digimon("ICE"), filler("FILL")],
        &["SUZ", "ICE", "FILL"],
    );

    assert!(
        runner.game.play_from_hand(0, 0).is_some(),
        "EX11-014 must be playable from hand"
    );

    let first = runner
        .pending_selection_view()
        .expect("first bucket selection pending");
    assert!(
        !first.valid_action_ids.contains(&PASS),
        "the [Suzune Kazuki] pick is mandatory — no PASS"
    );
    assert_eq!(
        first.valid_action_ids.len(),
        1,
        "only the Suzune card satisfies bucket 0"
    );
    runner
        .execute_action(first.selecting_player, first.valid_action_ids[0])
        .expect("pick the Suzune card");

    let second = runner
        .pending_selection_view()
        .expect("second bucket selection pending");
    assert!(
        !second.valid_action_ids.contains(&PASS),
        "the [Ice-Snow] Digimon pick is mandatory — no PASS"
    );
    assert_eq!(
        second.valid_action_ids.len(),
        1,
        "only the Ice-Snow Digimon satisfies bucket 1"
    );
    runner
        .execute_action(second.selecting_player, second.valid_action_ids[0])
        .expect("pick the Ice-Snow Digimon");
}

/// A non-Digimon [Ice-Snow] card must NOT satisfy bucket 1 (printed text:
/// "[Ice-Snow] trait **Digimon** card"; DCGO adds IsDigimon).
#[test]
fn ex11_014_ice_snow_bucket_rejects_non_digimon_ice_snow_card() {
    let mut runner = on_play_runner(
        vec![suzune("SUZ"), ice_snow_option("ICE-OPT"), filler("FILL")],
        &["SUZ", "ICE-OPT", "FILL"],
    );
    let hand_before = runner.game.players[0].hand.len();

    assert!(runner.game.play_from_hand(0, 0).is_some());
    runner
        .auto_resolve()
        .expect("resolve (Ice-Snow bucket empty, Suzune bucket has one hit)");

    let hand = hand_ids(&runner);
    assert!(
        !hand.iter().any(|id| id == "ICE-OPT"),
        "a non-Digimon [Ice-Snow] card must not satisfy the Digimon bucket: {hand:?}"
    );
    assert!(
        hand.iter().any(|id| id == "SUZ"),
        "the Suzune pick still resolves: {hand:?}"
    );
    // hand: -1 (played EX11-014) +1 (SUZ).
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "only the Suzune pick is added when no [Ice-Snow] Digimon is revealed"
    );
}

/// Exact-name fidelity: DCGO gates bucket 0 with EqualsCardName("Suzune
/// Kazuki"). A revealed card whose name merely CONTAINS the phrase must not
/// satisfy the bucket.
#[test]
fn ex11_014_suzune_bucket_requires_exact_name() {
    let mut runner = on_play_runner(
        vec![
            near_name_tamer("NEAR"),
            ice_snow_digimon("ICE"),
            filler("FILL"),
        ],
        &["NEAR", "ICE", "FILL"],
    );

    assert!(runner.game.play_from_hand(0, 0).is_some());
    runner
        .auto_resolve()
        .expect("resolve (Suzune bucket empty, Ice-Snow bucket has one hit)");

    let hand = hand_ids(&runner);
    assert!(
        !hand.iter().any(|id| id == "NEAR"),
        "\"Suzune Kazuki Drone\" must NOT satisfy the exact-name bucket: {hand:?}"
    );
    assert!(
        hand.iter().any(|id| id == "ICE"),
        "the Ice-Snow pick still resolves: {hand:?}"
    );
}

/// `no_duplicate_cards`: a single revealed card that satisfies BOTH filters
/// (named "Suzune Kazuki" and an [Ice-Snow] Digimon) fills only ONE bucket —
/// exactly one card is added to hand.
#[test]
fn ex11_014_one_card_cannot_fill_both_buckets() {
    let mut runner = on_play_runner(
        vec![both_buckets("BOTH"), filler("FILL-A"), filler("FILL-B")],
        &["BOTH", "FILL-A", "FILL-B"],
    );
    let hand_before = runner.game.players[0].hand.len();

    assert!(runner.game.play_from_hand(0, 0).is_some());
    runner
        .auto_resolve()
        .expect("resolve the single shared pick");

    let hand = hand_ids(&runner);
    assert!(
        hand.iter().any(|id| id == "BOTH"),
        "the double-matching card is added once: {hand:?}"
    );
    // hand: -1 (played EX11-014) +1 (BOTH) — NOT +2.
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "one revealed card must fill at most one bucket"
    );
}

/// Fizzle: with zero eligible candidates both buckets are skipped silently
/// (no pending selection) and all 3 revealed cards return to the deck bottom.
#[test]
fn ex11_014_on_play_fizzles_when_no_candidates() {
    let mut runner = on_play_runner(
        vec![filler("F-A"), filler("F-B"), filler("F-C")],
        &["F-A", "F-B", "F-C"],
    );
    let hand_before = runner.game.players[0].hand.len();
    let deck_before = runner.game.players[0].deck.len();

    assert!(runner.game.play_from_hand(0, 0).is_some());
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before - 1,
        "nothing is added to hand (only EX11-014 left it when played)"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before,
        "all 3 revealed cards return to the deck"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no pending selection remains on the fizzle path"
    );
    assert_eq!(
        runner.game.revealed_cards.len(),
        0,
        "reveal pool empty after the fizzle"
    );
}

// ─── Section 3: Behavioral — inherited <Jamming> ────────────────────────────

/// Stacked under a carrier, EX11-014's inherited <Jamming> keeps the carrier
/// alive through a LOSING security-Digimon battle (the defining Jamming
/// behavior, rules manual §16: not deleted in battles against Security
/// Digimon).
#[test]
fn ex11_014_inherited_jamming_keeps_carrier_alive_vs_security_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-014 YAML loads from embedded DSL pack")
        .add_card(carrier("CARRIER"))
        .add_card(digimon_security("SEC", 9000))
        .security(1, &["SEC"])
        .start();

    // EX11-014 as a digivolution source under the carrier.
    let atk = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(atk, Keyword::Jamming),
        "the carrier must hold <Jamming> from EX11-014's inherited grant"
    );

    let result = runner.attack_player(atk, 1, false);
    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "Jamming carrier (3000 DP) must survive the losing battle vs the \
         9000 DP security Digimon"
    );
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "the carrier stays on the field"
    );
    assert_eq!(runner.trash_size(1), 1, "the security card still trashes");
}

// ─── Section 4: Behavioral — alt digivolve "[Hiyarimon]: Cost 0" ───────────
//
// Hiyarimon (BT8-002 / EX7-002 / EX11-002) is a Lv.2 Digi-Egg card — in a
// real game it exists ONLY in the breeding area (hatched from the egg deck).
// The printed alt evo box "Digivolve [Hiyarimon]: Cost 0" must therefore be
// honored on a BREEDING-AREA base, alongside the standard "Lv.2 from blue:
// Cost 1" circle — surfacing a rule-17 cost choice {0, 1}, exactly as the
// battle-area path does (digivolve_cost_choice_archetypes.rs). DCGO applies
// `AddSelfDigivolutionRequirementStaticEffect` costs to ANY target permanent
// (`CardSource.CostList(targetPermanent, …)`) — no battle-area-only gate.

/// A hatched Hiyarimon: Lv.2 blue Digi-Egg-kind card named exactly
/// "Hiyarimon" (DCGO EX11_014.cs gates on EqualsCardName("Hiyarimon")).
fn hiyarimon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Hiyarimon");
    c.card_kind = CardKind::DigiEgg;
    c.level = Some(2);
    c.dp = Some(1000);
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Lesser".to_string()];
    c
}

/// A Lv.2 blue in-training whose name merely CONTAINS "Hiyarimon" — must NOT
/// satisfy EX11-014's exact-name alt path (DCGO uses EqualsCardName here,
/// unlike sibling EX8-019's ContainsCardName).
fn near_hiyarimon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Hiyarimon X");
    c.card_kind = CardKind::DigiEgg;
    c.level = Some(2);
    c.dp = Some(1000);
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Lesser".to_string()];
    c
}

/// Runner with `base` hatched in the breeding area and EX11-014 in hand.
fn breeding_runner(base: CardData) -> DebugRunner {
    let base_id = base.card_id.clone();
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-014 YAML loads from embedded DSL pack")
        .add_card(base)
        .add_card(filler("PAD-A"))
        .add_card(filler("PAD-B"))
        .hand(0, &[CARD_ID])
        .deck(0, &["PAD-A", "PAD-B"])
        .memory(5)
        .start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r.place_in_breeding(0, &base_id);
    r
}

/// The mask offers the breeding digivolve, and executing it surfaces the
/// rule-17 cost CHOICE between the [Hiyarimon] cost-0 alt path and the
/// standard Lv.2-blue cost-1 circle; picking the cost-0 option pays 0.
#[test]
fn ex11_014_breeding_hiyarimon_offers_cost_0_alt_route() {
    let mut runner = breeding_runner(hiyarimon("HIYARI"));

    let mask = build_action_mask(&runner.game, 0);
    assert!(
        mask[encode_digivolve(0, BREEDING_TARGET) as usize] > 0.0,
        "the breeding digivolve action must be offered"
    );

    let mem_before = runner.game.memory;
    let proceeded = runner
        .game
        .digivolve_from_hand_onto_breeding(0, 0, PlaySource::ByHand);
    assert!(
        !proceeded,
        "breeding Hiyarimon satisfies BOTH the [Hiyarimon] cost-0 alt path and \
         the standard Lv.2-blue cost-1 circle — the digivolve must PAUSE for a \
         rule-17 cost choice, not auto-pay the printed circle"
    );
    let view = runner
        .pending_selection_view()
        .expect("a cost-choice selection must be pending");
    let choices = view
        .effect_choices
        .as_ref()
        .expect("the cost choice must carry effect_choices");
    let labels: Vec<String> = choices.iter().map(|c| c.label.to_lowercase()).collect();
    assert!(
        labels.iter().any(|l| l.contains("cost 0")),
        "an option for the [Hiyarimon] cost-0 route (labels {labels:?})"
    );
    assert!(
        labels.iter().any(|l| l.contains("cost 1")),
        "an option for the standard cost-1 circle (labels {labels:?})"
    );

    let cost0 = choices
        .iter()
        .find(|c| c.label.to_lowercase().contains("cost 0"))
        .expect("cost-0 option present")
        .action_id;
    runner
        .execute_action(0, cost0)
        .expect("resolve the cost-0 choice");

    let breeding = runner.game.players[0]
        .breeding_area
        .as_ref()
        .expect("breeding permanent remains");
    assert_eq!(
        breeding.top_card().card_id(&runner.game.card_data),
        CARD_ID,
        "EX11-014 must be stacked on the breeding Hiyarimon"
    );
    assert_eq!(
        runner.game.memory, mem_before,
        "the [Hiyarimon] alt route costs 0 memory"
    );
}

/// Exact-name fidelity at runtime: a breeding base named "Hiyarimon X" does
/// NOT satisfy the exact-name alt path — only the standard Lv.2-blue circle
/// applies, so the digivolve completes immediately at cost 1 (no prompt).
#[test]
fn ex11_014_breeding_near_name_pays_standard_cost_1_no_prompt() {
    let mut runner = breeding_runner(near_hiyarimon("NEAR-HIYA"));

    let mem_before = runner.game.memory;
    let proceeded = runner
        .game
        .digivolve_from_hand_onto_breeding(0, 0, PlaySource::ByHand);
    assert!(
        proceeded,
        "only the standard circle applies — the digivolve completes immediately"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no cost choice for a single applicable route"
    );
    assert_eq!(
        mem_before - runner.game.memory,
        1,
        "\"Hiyarimon X\" must NOT satisfy the exact-name cost-0 path (DCGO \
         EqualsCardName) — the standard circle costs 1"
    );
}

// ─── Section 5: Behavioral — the YELLOW half of the split circle ────────────
//
// The printed evo circle is a blue/yellow SPLIT (official Bandai DB bundle:
// "Blue Lv.2 / cost 1" AND "Yellow Lv.2 / cost 1"). cards.json's evo_costs
// lossily carries only the blue half, so the yellow half exists ONLY via the
// YAML alt_path — these tests pin it.

/// A hatched yellow Lv.2 — matches ONLY the yellow half of the split circle
/// (not blue, not named "Hiyarimon").
fn yellow_lv2(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::DigiEgg;
    c.level = Some(2);
    c.dp = Some(1000);
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["Lesser".to_string()];
    c
}

/// A hatched red Lv.2 — matches NO route (off-colour for both split halves,
/// wrong name for the [Hiyarimon] path).
fn red_lv2(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::DigiEgg;
    c.level = Some(2);
    c.dp = Some(1000);
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Lesser".to_string()];
    c
}

/// The yellow half of the printed split circle: a yellow Lv.2 base is a
/// legal digivolve target at cost 1. Only one route applies (blue half fails
/// colour, [Hiyarimon] path fails the exact name), so the digivolve completes
/// immediately with no rule-17 cost prompt.
#[test]
fn ex11_014_yellow_lv2_base_digivolves_at_cost_1() {
    let mut runner = breeding_runner(yellow_lv2("YEL-BASE"));

    let mask = build_action_mask(&runner.game, 0);
    assert!(
        mask[encode_digivolve(0, BREEDING_TARGET) as usize] > 0.0,
        "the yellow half of the split circle must offer the breeding digivolve"
    );

    let mem_before = runner.game.memory;
    let proceeded = runner
        .game
        .digivolve_from_hand_onto_breeding(0, 0, PlaySource::ByHand);
    assert!(
        proceeded,
        "only the yellow circle applies — the digivolve completes immediately"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no cost choice for a single applicable route"
    );
    assert_eq!(
        mem_before - runner.game.memory,
        1,
        "the yellow Lv.2 half of the split circle costs 1 (official DB: \
         Yellow Lv.2 / cost 1)"
    );
    let breeding = runner.game.players[0]
        .breeding_area
        .as_ref()
        .expect("breeding permanent remains");
    assert_eq!(
        breeding.top_card().card_id(&runner.game.card_data),
        CARD_ID,
        "EX11-014 must be stacked on the yellow Lv.2 base"
    );
}

/// Off-colour negative: a red Lv.2 base satisfies NO route (neither split
/// half nor the [Hiyarimon] name gate) — the breeding digivolve is not
/// offered at all.
#[test]
fn ex11_014_red_lv2_base_has_no_digivolve_route() {
    let runner = breeding_runner(red_lv2("RED-BASE"));

    let mask = build_action_mask(&runner.game, 0);
    assert!(
        !(mask[encode_digivolve(0, BREEDING_TARGET) as usize] > 0.0),
        "a red Lv.2 base matches neither the blue nor the yellow half of the \
         split circle, nor the [Hiyarimon] path — no digivolve action"
    );
}

/// Baseline: the same carrier WITHOUT EX11-014 stacked dies to the same
/// security Digimon — proving the inherited grant is what does the work.
#[test]
fn ex11_014_baseline_carrier_without_penguinmon_dies_vs_security_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-014 YAML loads from embedded DSL pack")
        .add_card(carrier("CARRIER"))
        .add_card(digimon_security("SEC", 9000))
        .security(1, &["SEC"])
        .start();

    let atk = runner.place_on_field(0, "CARRIER", Some(0));
    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(atk, Keyword::Jamming),
        "no Jamming without the EX11-014 source"
    );

    let result = runner.attack_player(atk, 1, false);
    assert_eq!(
        result,
        AttackResult::AttackerDeletedBySecurity,
        "without Jamming the 3000 DP carrier loses to the 9000 DP security \
         Digimon"
    );
    assert_eq!(runner.battle_area_size(0), 0, "the carrier is deleted");
}
