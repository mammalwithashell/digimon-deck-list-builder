//! ST20-02 Biyomon — Digimon, Lv.3, Red, DP 1000, play cost 3.
//! Traits: Bird, ADVENTURE. Form: Rookie. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB, data/card_bundles/ST20-02.md)
//!
//! ```text
//! [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with the
//! [ADVENTURE] trait and 1 such Tamer card or Option card among them to the
//! hand. Return the rest to the bottom of the deck.
//!
//! Inherited Effect
//! [Your Turn] This Digimon gets +2000 DP.
//!
//! evo_costs:  Red Lv.2: Cost 0
//! xros_req:   [Digivolve] Lv.2 w/[ADVENTURE] trait: Cost 0
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST20/Red/ST20_02.cs
//!   - Alternate Digivolution Requirement: `HasAdventureTraits && Level == 2`,
//!     cost 0, requirements not ignored.
//!   - [On Play]: `SimplifiedRevealDeckTopCardsAndSelect(3, [Digimon+ADVENTURE
//!     max 1, (Option|Tamer)+ADVENTURE max 1], DeckBottom)`; `canNoSelect` is
//!     false, so each bucket is MANDATORY whenever a legal card was revealed.
//!   - Your Turn ESS: `ChangeSelfDPStaticEffect(+2000, inherited)`.
//!
//! # Patterns (docs/RUST_DSL_TEST_API.md §4.3 / §14.6)
//! - A  reveal-N + multi-bucket add to hand + bottom remainder (BT24-031 mold)
//! - D4 inherited [Your Turn] DP aura (stack-carrier scoped)
//! - Alt-path digivolve (xros_req) exercised through `digivolve_from_hand`
//!
//! There is no cost to pay and no once-per-turn clause on this card, so no
//! cost-firing or OPT-lockout tests apply. The bucket prompts are mandatory
//! (no PASS), which the tests assert.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "ST20-02";

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// A deck/reveal fixture of the given kind. `adventure` toggles the [ADVENTURE]
/// trait; Tamers and Options carry no level or DP.
fn fixture(id: &str, kind: CardKind, adventure: bool) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = kind;
    if kind != CardKind::Digimon {
        card.level = None;
        card.dp = None;
    }
    if adventure {
        card.traits = vec!["ADVENTURE".to_string()];
    }
    card
}

/// Biyomon in P0's hand; `deck` is listed bottom -> top (last = top of deck),
/// so the revealed top 3 are the last three entries.
fn biyomon_runner(deck: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-02 in embedded DSL pack")
        .add_card(fixture("ADV-DIGI", CardKind::Digimon, true))
        .add_card(fixture("ADV-DIGI-2", CardKind::Digimon, true))
        .add_card(fixture("ADV-DIGI-DEEP", CardKind::Digimon, true))
        .add_card(fixture("ADV-TAMER", CardKind::Tamer, true))
        .add_card(fixture("ADV-OPTION", CardKind::Option, true))
        .add_card(fixture("PLAIN-DIGI", CardKind::Digimon, false))
        .add_card(fixture("PLAIN-TAMER", CardKind::Tamer, false))
        .add_card(fixture("PLAIN-OPTION", CardKind::Option, false))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &[CARD_ID])
        .deck(0, deck)
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start()
}

fn zone_ids(cards: &[digimon_engine::card_source::CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

fn hand_ids(runner: &DebugRunner) -> Vec<String> {
    let mut ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    ids.sort();
    ids
}

fn deck_ids(runner: &DebugRunner) -> Vec<String> {
    zone_ids(&runner.game.players[0].deck, &runner.game.card_data)
}

/// Card ids the pending reveal-bucket prompt currently allows picking.
fn legal_reveal_ids(runner: &DebugRunner) -> Vec<String> {
    let view = runner
        .pending_selection_view()
        .expect("a reveal prompt is pending");
    let mut ids: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|a| **a != PASS)
        .map(|a| {
            let idx = (*a - SEL_REVEAL_START) as usize;
            runner.game.revealed_cards[idx]
                .card_id(&runner.game.card_data)
                .to_string()
        })
        .collect();
    ids.sort();
    ids
}

fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> u16 {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
        .expect("revealed card exists")
}

fn pick_revealed_by_id(runner: &mut DebugRunner, id: &str, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = revealed_action_for_id(runner, id);
    assert!(view.valid_action_ids.contains(&action), "{label}");
    runner.execute_action(0, action).expect(label);
}

fn assert_required_pending_pick(runner: &DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    assert!(
        !runner.pending_is_optional(),
        "{label}: the bucket pick must be mandatory when a legal card was revealed"
    );
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "{label}: PASS must not be legal before the required pick"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st20_02_identity_matches_printed_card() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("compiles")
        .start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.name, "Biyomon");
    assert_eq!(c.level, Some(3));
    assert_eq!(c.dp, Some(1000));
    assert_eq!(c.cost, Some(3));
    let traits: Vec<String> = c.all_traits().iter().map(|t| t.to_lowercase()).collect();
    for want in ["bird", "adventure", "vaccine"] {
        assert!(
            traits.contains(&want.to_string()),
            "missing trait {want}: {traits:?}"
        );
    }
}

/// Two requirement-respecting zero-cost digivolve paths (standard Red Lv.2 +
/// the xros_req Lv.2 [ADVENTURE] box).
#[test]
fn st20_02_declares_standard_and_xros_req_alt_paths() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("compiles")
        .start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.alt_paths.len(), 2, "standard circle + xros_req box");
    for path in &c.alt_paths {
        assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
        assert_eq!(path.cost, Some(CompiledCost::Literal(0)));
        assert!(
            !path.ignore_requirements,
            "DCGO: ignoreDigivolutionRequirement false"
        );
        assert_eq!(path.from.as_ref().expect("from").level_eq, Some(2));
    }
    assert!(
        c.alt_paths
            .iter()
            .any(|p| p.from.as_ref().unwrap().trait_has.as_deref() == Some("ADVENTURE")),
        "an [ADVENTURE] source path must exist"
    );
}

/// One [On Play] triggered clause (mandatory, not OPT) + one inherited aura.
#[test]
fn st20_02_has_on_play_clause_and_inherited_aura() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("compiles")
        .start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.effects.len(), 2);
    let on_play = c
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("[On Play] triggered clause");
    assert_eq!(on_play.when, vec![CompiledTiming::OnPlay]);
    assert_eq!(on_play.scope, CompiledScope::FaceUp);
    assert!(!on_play.optional, "printed text has no \"may\"");
    assert!(!on_play.once_per_turn);
    let aura = c
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope,
                dp_modifier,
                ..
            }) => Some((*scope, *dp_modifier)),
            _ => None,
        })
        .expect("inherited aura");
    assert_eq!(aura, (CompiledScope::Inherited, Some(2000)));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — [On Play] reveal 3: ADVENTURE Digimon + ADVENTURE Tamer/Option
// ═══════════════════════════════════════════════════════════════════════════

/// POSITIVE: one ADVENTURE Digimon and one ADVENTURE Tamer are offered in turn,
/// each pick is mandatory, only the TOP 3 cards are revealed (the 4th ADVENTURE
/// Digimon is out of reach), and the unpicked card goes to the deck bottom.
#[test]
fn st20_02_on_play_adds_adventure_digimon_then_adventure_tamer() {
    // bottom -> top: DEEP, FILLER, TAMER, DIGI  => revealed = DIGI, TAMER, FILLER
    let mut runner = biyomon_runner(&["ADV-DIGI-DEEP", "FILLER", "ADV-TAMER", "ADV-DIGI"]);
    let mem_before = runner.memory();
    runner.play(0, 0).expect("play Biyomon");
    assert_eq!(mem_before - runner.memory(), 3, "play cost 3 paid");

    assert_required_pending_pick(&runner, "Digimon bucket");
    assert_eq!(
        legal_reveal_ids(&runner),
        vec!["ADV-DIGI".to_string()],
        "only the revealed [ADVENTURE] Digimon is legal in the Digimon bucket"
    );
    pick_revealed_by_id(&mut runner, "ADV-DIGI", "pick the ADVENTURE Digimon");

    assert_required_pending_pick(&runner, "Tamer/Option bucket");
    assert_eq!(
        legal_reveal_ids(&runner),
        vec!["ADV-TAMER".to_string()],
        "only the revealed [ADVENTURE] Tamer is legal in the Tamer/Option bucket"
    );
    pick_revealed_by_id(&mut runner, "ADV-TAMER", "pick the ADVENTURE Tamer");
    runner.auto_resolve().expect("finish the reveal");

    assert_eq!(
        hand_ids(&runner),
        vec!["ADV-DIGI".to_string(), "ADV-TAMER".to_string()]
    );
    let deck = deck_ids(&runner);
    assert_eq!(deck.len(), 2, "unrevealed 4th card + 1 returned card");
    assert_eq!(
        deck.first().map(String::as_str),
        Some("FILLER"),
        "returned to the BOTTOM"
    );
    assert_eq!(
        deck.last().map(String::as_str),
        Some("ADV-DIGI-DEEP"),
        "the 4th card was never revealed and is still the top card"
    );
    assert!(
        !hand_ids(&runner).contains(&"ADV-DIGI-DEEP".to_string()),
        "only the top 3 are revealed"
    );
}

/// POSITIVE: an [ADVENTURE] OPTION satisfies the second bucket ("Tamer card or
/// Option card"); with both a Tamer and an Option revealed the player picks
/// exactly ONE of them and the other returns to the deck bottom.
#[test]
fn st20_02_second_bucket_accepts_an_adventure_option_and_takes_only_one() {
    // revealed = DIGI, OPTION, TAMER (top 3)
    let mut runner = biyomon_runner(&["FILLER", "ADV-TAMER", "ADV-OPTION", "ADV-DIGI"]);
    runner.play(0, 0).expect("play Biyomon");

    pick_revealed_by_id(&mut runner, "ADV-DIGI", "pick the ADVENTURE Digimon");
    assert_required_pending_pick(&runner, "Tamer/Option bucket");
    assert_eq!(
        legal_reveal_ids(&runner),
        vec!["ADV-OPTION".to_string(), "ADV-TAMER".to_string()],
        "both the Tamer and the Option are legal here"
    );
    pick_revealed_by_id(&mut runner, "ADV-OPTION", "pick the ADVENTURE Option");
    runner.auto_resolve().expect("finish the reveal");

    assert_eq!(
        hand_ids(&runner),
        vec!["ADV-DIGI".to_string(), "ADV-OPTION".to_string()],
        "exactly 1 Digimon and 1 Tamer-or-Option are added; the Tamer is NOT"
    );
    let deck = deck_ids(&runner);
    assert!(
        deck.contains(&"ADV-TAMER".to_string()),
        "unpicked Tamer returns to the deck"
    );
    assert_eq!(deck.len(), 2);
}

/// NEGATIVE: non-[ADVENTURE] Digimon / Tamers / Options are never legal, and
/// when NOTHING qualifies no prompt installs and nothing is added.
#[test]
fn st20_02_non_adventure_cards_are_never_offered() {
    let mut runner = biyomon_runner(&["FILLER", "PLAIN-OPTION", "PLAIN-TAMER", "PLAIN-DIGI"]);
    runner.play(0, 0).expect("play Biyomon");
    // Only the remainder-ordering prompt (if any) may remain; no reveal-bucket
    // pick is offered because nothing revealed is a legal candidate.
    assert!(
        !matches!(
            runner.pending_kind(),
            Some(SelectionKind::RevealBucket { .. })
        ),
        "no [ADVENTURE] card was revealed, so no bucket prompt may install"
    );
    runner
        .auto_resolve()
        .expect("resolve any remainder ordering");
    assert!(runner.pending_selection().is_none());
    assert!(hand_ids(&runner).is_empty(), "nothing is added to the hand");
    assert_eq!(
        deck_ids(&runner).len(),
        4,
        "all four deck cards are still in the deck"
    );
}

/// NEGATIVE: a revealed non-[ADVENTURE] Digimon is skipped but the ADVENTURE
/// Tamer is still offered (the buckets resolve independently).
#[test]
fn st20_02_missing_digimon_bucket_does_not_block_the_tamer_bucket() {
    let mut runner = biyomon_runner(&["FILLER", "ADV-TAMER", "PLAIN-DIGI"]);
    runner.play(0, 0).expect("play Biyomon");
    assert_required_pending_pick(&runner, "Tamer/Option bucket");
    assert_eq!(legal_reveal_ids(&runner), vec!["ADV-TAMER".to_string()]);
    pick_revealed_by_id(&mut runner, "ADV-TAMER", "pick the ADVENTURE Tamer");
    runner.auto_resolve().expect("finish the reveal");
    assert_eq!(hand_ids(&runner), vec!["ADV-TAMER".to_string()]);
}

/// NEGATIVE: the Tamer/Option bucket never takes a Digimon. With two
/// [ADVENTURE] Digimon revealed the player takes ONE (the Digimon bucket is
/// max 1); the other cannot be added through the second bucket.
#[test]
fn st20_02_second_bucket_never_takes_a_digimon() {
    let mut runner = biyomon_runner(&["FILLER", "ADV-DIGI-2", "ADV-DIGI"]);
    runner.play(0, 0).expect("play Biyomon");
    assert_required_pending_pick(&runner, "Digimon bucket");
    assert_eq!(
        legal_reveal_ids(&runner),
        vec!["ADV-DIGI".to_string(), "ADV-DIGI-2".to_string()]
    );
    pick_revealed_by_id(&mut runner, "ADV-DIGI-2", "pick one Digimon");
    runner.auto_resolve().expect("finish the reveal");
    assert_eq!(
        hand_ids(&runner),
        vec!["ADV-DIGI-2".to_string()],
        "only one Digimon is added; the second bucket is Tamer/Option only"
    );
    assert!(deck_ids(&runner).contains(&"ADV-DIGI".to_string()));
}

/// The On Play prompt is a real choice among legal cards: with two ADVENTURE
/// Digimon revealed both are offered, so the pick is not auto-resolved.
#[test]
fn st20_02_digimon_pick_offers_every_legal_candidate() {
    let mut runner = biyomon_runner(&["FILLER", "ADV-DIGI-2", "ADV-DIGI"]);
    runner.play(0, 0).expect("play Biyomon");
    let view = runner
        .pending_selection_view()
        .expect("Digimon bucket prompt");
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both ADVENTURE Digimon are selectable (no auto-pick)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Inherited [Your Turn] +2000 DP
// ═══════════════════════════════════════════════════════════════════════════

fn carrier_runner() -> DebugRunner {
    let mut carrier = make_test_card_with_level("CARRIER", "Carrier", 4);
    carrier.dp = Some(3000);
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-02 compiles")
        .add_card(carrier)
        .memory(10)
        .start()
}

/// POSITIVE: Biyomon beneath a carrier gives it +2000 DP on your turn.
#[test]
fn st20_02_inherited_plus_2000_on_your_turn() {
    let mut runner = carrier_runner();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.game.tick_declarative_effects();
    assert_eq!(runner.turn_player(), 0);
    assert_eq!(runner.effective_dp(carrier), Some(5000));
}

/// NEGATIVE: [Your Turn] only — no bonus on the opponent's turn.
#[test]
fn st20_02_inherited_bonus_absent_on_opponents_turn() {
    let mut runner = carrier_runner();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1);
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(carrier), Some(3000));
}

/// NEGATIVE: an inherited effect does not apply to Biyomon itself while it is
/// the face-up top card (rules: inherited effects live on digivolution cards).
#[test]
fn st20_02_face_up_biyomon_does_not_buff_itself() {
    let mut runner = carrier_runner();
    let biyomon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(biyomon), Some(1000));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Alternate digivolution (evo_costs circle + xros_req box)
// ═══════════════════════════════════════════════════════════════════════════

fn base(id: &str, level: u8, color: CardColor, adventure: bool) -> CardData {
    let mut c = make_test_card_with_level(id, id, level);
    c.colors = vec![color];
    c.dp = Some(2000);
    c.play_cost = 2;
    if adventure {
        c.traits = vec!["ADVENTURE".to_string()];
    }
    c
}

/// Try to digivolve Biyomon (in hand) onto `b`; returns (succeeded, memory spent).
fn try_digivolve(b: CardData) -> (bool, i16) {
    let base_id = b.card_id.clone();
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-02 in embedded DSL pack")
        .add_card(b)
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 3])
        .memory(10)
        .start();
    let handle = r.place_on_field(0, &base_id, Some(0));
    let before = r.memory();
    let ok = r
        .game
        .digivolve_from_hand(0, 0, handle.index as usize, PlaySource::ByDigivolve);
    let _ = r.auto_resolve();
    (ok, before - r.memory())
}

/// Standard circle: Red Lv.2 -> cost 0.
#[test]
fn st20_02_digivolves_from_red_level_2_for_free() {
    assert_eq!(
        try_digivolve(base("RED-LV2", 2, CardColor::Red, false)),
        (true, 0)
    );
}

/// xros_req box: a non-red Lv.2 [ADVENTURE] Digimon -> cost 0.
#[test]
fn st20_02_digivolves_from_non_red_level_2_adventure_for_free() {
    assert_eq!(
        try_digivolve(base("BLUE-LV2-ADV", 2, CardColor::Blue, true)),
        (true, 0)
    );
}

/// NEGATIVE: a non-red Lv.2 without [ADVENTURE] is not a legal base.
#[test]
fn st20_02_rejects_non_red_level_2_without_adventure() {
    assert!(!try_digivolve(base("BLUE-LV2", 2, CardColor::Blue, false)).0);
}

/// NEGATIVE: the level still has to be 2 — a Lv.3 [ADVENTURE] base is rejected.
#[test]
fn st20_02_rejects_level_3_adventure_base() {
    assert!(!try_digivolve(base("RED-LV3-ADV", 3, CardColor::Red, true)).0);
}
