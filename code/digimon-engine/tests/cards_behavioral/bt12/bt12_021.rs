//! BT12-021 Veemon — Digimon, Lv.3, Blue, DP 1000, Cost 3.
//! Traits: Mini Dragon.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with
//! [Imperialdramon] in its name or the [Free] trait and 1 Tamer card with
//! [Davis Motomiya] in its name among them to your hand. Place the rest at
//! the bottom of your deck in any order.
//!
//! Inherited Effect:
//! [End of Your Turn] This Digimon and any of your other Digimon may DNA
//! digivolve into a Digimon card in the hand.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT12/Blue/BT12_021.cs
//!
//! Notable DCGO contracts:
//!   - Clause 1: `SimplifiedRevealDeckTopCardsAndSelect(revealCount: 3, ...)` with two
//!     buckets — (a) Digimon with `ContainsCardName("Imperialdramon") || CardTraits.Contains("Free")`,
//!     (b) Tamer with `ContainsCardName("Davis Motomiya")`. `mutualConditions: true`. Outer
//!     trigger: `isOptional: false` (mandatory). `remainingCardsPlace: DeckBottom`.
//!   - Clause 2: `SetIsInheritedEffect(true)`. `SetUpActivateClass(..., isOptional: true)`.
//!     `DNADigivolvePermanentsIntoHandOrTrashCard(... isHand: true ...)`. Registered as
//!     `alt_path_registration` declarative per RESOLVED-2026-05-02 in qa/dsl-vocab-gaps.md.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - A3 Reveal + select-multi (reveal 3, two named buckets)
//! - G2 Inherited end-of-turn DNA digivolve registration (alt_path_registration declarative)
//!
//! # Test layout
//!
//! Section 1 — Structural assertions on the compiled card (clause count, scope, timing)
//! Section 2 — Condition gating for clause 0 (per-branch by reveal contents)
//!   - both buckets hit (hand+2, bottom+1)
//!   - only Imperialdramon Digimon bucket hit (hand+1)
//!   - only Davis Motomiya Tamer bucket hit (hand+1)
//!   - neither bucket hit (all 3 to bottom)
//!   - Free-trait Digimon (not Imperialdramon-named) satisfies bucket 1
//! Section 3 — Behavioral: drive the both-buckets path, assert hand+deck state
//! Section 4 — Behavioral: alt_path_registration (structural; action-mask test for DNA
//!   digivolve is deferred to G2 mechanic tests)
//! Section 5 — Negative: clause 0 does not fire on opponent play

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

const CARD_ID: &str = "BT12-021";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn make_named_digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.traits = vec![];
    c
}

fn make_named_digimon_with_traits(id: &str, name: &str, traits: Vec<String>) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.traits = traits;
    c
}

fn make_named_tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.traits = vec![];
    c
}

fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec![];
    c
}

// ─── Runner builder ──────────────────────────────────────────────────────────

/// Base runner with BT12-021 loaded from the embedded DSL pack.
/// Decks are padded so draws during turn-begin don't exhaust them.
fn veemon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT12-021")
        .expect("BT12-021 in embedded DSL pack")
        .add_card(make_filler("PAD"))
        .deck(0, &["PAD", "PAD", "PAD", "PAD", "PAD", "PAD", "PAD", "PAD"])
        .deck(1, &["PAD", "PAD", "PAD", "PAD", "PAD"])
        .memory(10)
        .start()
}

// ─── Reveal-pick helpers ─────────────────────────────────────────────────────

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

fn pick_revealed_by_id(runner: &mut DebugRunner, id: &str, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = revealed_action_for_id(runner, id)
        .unwrap_or_else(|| panic!("{label}: revealed card {id} not present"));
    assert!(
        view.valid_action_ids.contains(&action),
        "{label}: action {action} for {id} not legal in current bucket"
    );
    runner.execute_action(0, action).expect(label);
}

fn assert_mandatory_pick(runner: &DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "{label}: PASS must not be legal for a mandatory bucket pick"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions on the compiled card
// ═══════════════════════════════════════════════════════════════════════════════

/// BT12-021 YAML must be present in the embedded pack and compile cleanly.
#[test]
fn bt12_021_yaml_compiles_and_metadata_matches_printed() {
    let runner = veemon_runner();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT12-021 in embedded DSL pack");
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.level, Some(3), "Lv.3");
    assert_eq!(card.cost, Some(3), "Cost 3");
    assert_eq!(card.dp, Some(1000), "DP 1000");
    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("Mini Dragon")),
        "BT12-021 must carry Mini Dragon trait; got {:?}",
        card.traits
    );
}

/// Exactly one OnPlay triggered clause exists, with own (FaceUp) scope.
/// Printed has no "you may" → mandatory (not optional).
/// Filter by OnPlay so we ignore the inherited EOT DNA-digivolve triggered
/// clause (G-DSL-EOT-DNA-INLINE, 2026-05-24).
#[test]
fn bt12_021_has_one_on_play_triggered_clause_mandatory() {
    let runner = veemon_runner();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT12-021 in embedded DSL pack");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1, "Exactly one OnPlay triggered clause");
    let clause = triggered[0];
    assert_eq!(clause.when, vec![CompiledTiming::OnPlay]);
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "OnPlay clause is FaceUp (own-play)"
    );
    assert!(!clause.optional, "No 'you may' in printed text — mandatory");
    assert!(!clause.once_per_turn, "No [Once Per Turn] on this clause");
}

/// G-DSL-EOT-DNA-INLINE (2026-05-24): inherited end-of-your-turn DNA
/// digivolve is now authored via the `may_dna_digivolve_now` step verb
/// inside a `Triggered` clause whose `when: end_of_your_turn, scope:
/// inherited, optional: true`. The body is a single `MayDnaDigivolveNow`
/// step that orchestrates the partner + target selections at trigger fire.
///
/// Predecessor authoring (`alt_path_registration { kind: dna_digivolve,
/// scope: inherited, trigger: end_of_your_turn }`) registered a next-turn
/// alt-path action and did not satisfy the printed "AT end of turn"
/// surface — see proposal `may-dna-digivolve-now-dsl-step`.
#[test]
fn bt12_021_has_inherited_dna_digivolve_may_step() {
    let runner = veemon_runner();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT12-021 in embedded DSL pack");
    let eot_dna_clause = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::EndOfYourTurn)
                && t.optional =>
        {
            Some(t)
        }
        _ => None,
    });
    let t = eot_dna_clause
        .expect("BT12-021 must carry an inherited optional EoT triggered clause for DNA digivolve");
    assert!(
        t.process.iter().any(|step| matches!(
            step,
            CompiledStep::MayDnaDigivolveNow {
                ignore_requirements: true,
                cost: 0,
                ..
            }
        )),
        "BT12-021 EoT inherited body must contain a `MayDnaDigivolveNow` step \
         with cost=0 and ignore_requirements=true"
    );
}

/// G-DSL-EOT-DNA-INLINE (2026-05-24): card now prints two triggered
/// clauses — (1) the [On Play] reveal-3 trigger and (2) the inherited
/// [End of Your Turn] DNA digivolve trigger (was previously an
/// alt_path_registration declarative — see proposal
/// `may-dna-digivolve-now-dsl-step`). No more alt_path_registration
/// clauses on this card.
#[test]
fn bt12_021_clause_count_matches_card_text() {
    let runner = veemon_runner();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT12-021 in embedded DSL pack");
    let triggered = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    let alt_path_regs = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::AltPathRegistration { .. })
            )
        })
        .count();
    assert_eq!(triggered, 2, "Exactly two triggered clauses expected");
    assert_eq!(
        alt_path_regs, 0,
        "Zero alt_path_registration clauses expected after EoT DNA migration"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating / per-branch reveal tests
// ═══════════════════════════════════════════════════════════════════════════════

/// POSITIVE (both buckets): Reveal 3 = [IMP-DIGI, DAVIS-TAMER, FILLER].
/// Pick Imperialdramon Digimon for bucket 1 and Davis Tamer for bucket 2.
/// Hand gains both; FILLER goes to deck bottom.
#[test]
fn bt12_021_on_play_both_buckets_hit_hand_gains_two() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-021")
        .expect("BT12-021 loads")
        .add_card(make_named_digimon("IMP-DIGI", "Imperialdramon Dragon Mode"))
        .add_card(make_named_tamer("DAVIS", "Davis Motomiya"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["IMP-DIGI", "DAVIS", "FILLER"])
        .hand(0, &["BT12-021"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Veemon BT12-021");

    // Bucket 1: Imperialdramon/Free Digimon — mandatory
    assert_mandatory_pick(&runner, "Imperialdramon/Free Digimon bucket");
    pick_revealed_by_id(&mut runner, "IMP-DIGI", "pick Imperialdramon Digimon");

    // Bucket 2: Davis Motomiya Tamer — mandatory
    assert_mandatory_pick(&runner, "Davis Motomiya Tamer bucket");
    pick_revealed_by_id(&mut runner, "DAVIS", "pick Davis Motomiya Tamer");

    runner.auto_resolve().expect("bottom remainder");

    let hand: Vec<_> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand.contains(&"IMP-DIGI".to_string()),
        "Imperialdramon Digimon must be in hand"
    );
    assert!(
        hand.contains(&"DAVIS".to_string()),
        "Davis Motomiya Tamer must be in hand"
    );

    // FILLER must be at deck bottom (Veemon was removed from hand to play; deck = [FILLER])
    let deck: Vec<_> = runner.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        deck.contains(&"FILLER".to_string()),
        "unchosen reveal card must be at deck bottom; deck={deck:?}"
    );
    // Trash must not gain cards from this effect
    assert!(
        runner.game.players[0].trash.is_empty(),
        "no cards should go to trash from clause 1"
    );
}

/// POSITIVE (Imperialdramon-only): Reveal = [IMP-DIGI, FILLER1, FILLER2].
/// Only bucket 1 has a candidate; bucket 2 silently skips.
/// Hand gains 1 (Imperialdramon); two fillers go to deck bottom.
#[test]
fn bt12_021_on_play_only_imperialdramon_found_skips_tamer_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-021")
        .expect("BT12-021 loads")
        .add_card(make_named_digimon(
            "IMP-DIGI",
            "Imperialdramon Fighter Mode",
        ))
        .add_card(make_filler("FILLER1"))
        .add_card(make_filler("FILLER2"))
        .deck(0, &["IMP-DIGI", "FILLER1", "FILLER2"])
        .hand(0, &["BT12-021"])
        .memory(10)
        .start();

    let hand_before = runner.hand_size(0); // =1 (just BT12-021)
    runner.play(0, 0).expect("play Veemon");

    pick_revealed_by_id(&mut runner, "IMP-DIGI", "pick Imperialdramon");
    runner
        .auto_resolve()
        .expect("resolve through empty tamer bucket + bottom placement");

    let hand_after = runner.hand_size(0);
    // Hand had 1 before (Veemon), played Veemon (-1), then gained IMP-DIGI (+1) → net 0
    // but hand_before was before the play so: hand_before=1, played -1, gained +1 = 1
    // Actually hand_before is before play. After play, hand was 0. After bucket1 add: hand=1.
    // The assertion should be that IMP-DIGI is in hand.
    let hand: Vec<_> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand.contains(&"IMP-DIGI".to_string()),
        "Imperialdramon must be added to hand"
    );
    assert!(
        !hand.contains(&"FILLER1".to_string()) && !hand.contains(&"FILLER2".to_string()),
        "fillers must not enter hand when no tamer bucket match"
    );
}

/// POSITIVE (Davis-only): Reveal = [DAVIS-TAMER, FILLER1, FILLER2].
/// Only bucket 2 has a candidate; bucket 1 silently skips.
/// Hand gains 1 (Davis Motomiya Tamer).
#[test]
fn bt12_021_on_play_only_davis_motomiya_found_skips_digimon_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-021")
        .expect("BT12-021 loads")
        .add_card(make_named_tamer("DAVIS", "Davis Motomiya"))
        .add_card(make_filler("FILLER1"))
        .add_card(make_filler("FILLER2"))
        .deck(0, &["DAVIS", "FILLER1", "FILLER2"])
        .hand(0, &["BT12-021"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Veemon");

    // Drive prompts — pick Davis when it appears as the tamer bucket candidate
    while let Some(view) = runner.pending_selection_view() {
        let davis_action = revealed_action_for_id(&runner, "DAVIS");
        if let Some(action) = davis_action {
            if view.valid_action_ids.contains(&action) {
                runner.execute_action(0, action).expect("pick Davis");
                continue;
            }
        }
        // advance through empty bucket or bottom-placement
        let next = view.valid_action_ids[0];
        runner.execute_action(0, next).expect("advance");
    }

    let hand: Vec<_> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand.contains(&"DAVIS".to_string()),
        "Davis Motomiya Tamer must be added to hand"
    );
    assert!(
        !hand.contains(&"FILLER1".to_string()) && !hand.contains(&"FILLER2".to_string()),
        "fillers must not enter hand when no Imperialdramon/Free Digimon found"
    );
}

/// NEGATIVE (neither bucket): Reveal = 3 fillers.
/// Both buckets have zero candidates; engine skips both silently.
/// All 3 fillers go to deck bottom. Hand stays filler-free.
#[test]
fn bt12_021_on_play_neither_bucket_found_bottoms_all_three() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-021")
        .expect("BT12-021 loads")
        .add_card(make_filler("FILLER1"))
        .add_card(make_filler("FILLER2"))
        .add_card(make_filler("FILLER3"))
        .deck(0, &["FILLER1", "FILLER2", "FILLER3"])
        .hand(0, &["BT12-021"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Veemon");
    runner
        .auto_resolve()
        .expect("resolve through both empty buckets + bottom placement");

    let hand: Vec<_> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    for filler in ["FILLER1", "FILLER2", "FILLER3"] {
        assert!(
            !hand.contains(&filler.to_string()),
            "{filler} must NOT enter hand when no bucket matches"
        );
    }
    let deck: Vec<_> = runner.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(deck.len(), 3, "all 3 reveal cards must be back in deck");
}

/// POSITIVE (Free-trait Digimon): Reveal = [FREE-DIGI, DAVIS, FILLER].
/// FREE-DIGI has the [Free] trait (not Imperialdramon name) — must satisfy bucket 1.
#[test]
fn bt12_021_on_play_free_trait_digimon_satisfies_imperialdramon_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-021")
        .expect("BT12-021 loads")
        .add_card(make_named_digimon_with_traits(
            "FREE-DIGI",
            "Veemon", // no Imperialdramon in name
            vec!["Free".to_string()],
        ))
        .add_card(make_named_tamer("DAVIS", "Davis Motomiya"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FREE-DIGI", "DAVIS", "FILLER"])
        .hand(0, &["BT12-021"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Veemon");

    // Bucket 1 (Imperialdramon-or-Free) should offer FREE-DIGI
    assert_mandatory_pick(
        &runner,
        "bucket 1 must have a candidate (Free-trait Digimon)",
    );
    pick_revealed_by_id(
        &mut runner,
        "FREE-DIGI",
        "pick Free-trait Digimon for bucket 1",
    );

    // Bucket 2: Davis Motomiya
    assert_mandatory_pick(&runner, "bucket 2 must have a candidate (Davis Tamer)");
    pick_revealed_by_id(&mut runner, "DAVIS", "pick Davis Motomiya Tamer");

    runner.auto_resolve().expect("bottom remainder");

    let hand: Vec<_> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand.contains(&"FREE-DIGI".to_string()),
        "Free-trait Digimon (non-Imperialdramon name) must satisfy bucket 1 and go to hand"
    );
    assert!(
        hand.contains(&"DAVIS".to_string()),
        "Davis Motomiya Tamer must go to hand"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: both-buckets path — deck-bottom contents
// ═══════════════════════════════════════════════════════════════════════════════

/// Both-buckets-hit: assert that the remaining card lands at deck BOTTOM
/// (position: bottom), and only there — not in trash, not in hand.
#[test]
fn bt12_021_on_play_remainder_goes_to_deck_bottom_not_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-021")
        .expect("BT12-021 loads")
        .add_card(make_named_digimon("IMP-DIGI", "Imperialdramon Dragon Mode"))
        .add_card(make_named_tamer("DAVIS", "Davis Motomiya"))
        .add_card(make_filler("REMAINDER"))
        .deck(0, &["IMP-DIGI", "DAVIS", "REMAINDER"])
        .hand(0, &["BT12-021"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Veemon");

    pick_revealed_by_id(&mut runner, "IMP-DIGI", "pick Imperialdramon");
    pick_revealed_by_id(&mut runner, "DAVIS", "pick Davis Tamer");
    runner.auto_resolve().expect("bottom remainder");

    // REMAINDER must be in deck (at bottom = end of deck vec)
    let deck_ids: Vec<_> = runner.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        deck_ids.contains(&"REMAINDER".to_string()),
        "remainder card must be placed back in deck; deck={deck_ids:?}"
    );
    // Must be at the bottom (last element)
    assert_eq!(
        deck_ids.last().unwrap().as_str(),
        "REMAINDER",
        "remainder must be at deck bottom (last element)"
    );
    // Must NOT be in trash
    let trash_ids: Vec<_> = runner.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        !trash_ids.contains(&"REMAINDER".to_string()),
        "remainder must NOT go to trash"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited EOYT alt_path_registration (structural)
// ═══════════════════════════════════════════════════════════════════════════════

/// G-DSL-EOT-DNA-INLINE (2026-05-24): the inherited EOT DNA digivolve is
/// authored as a `Triggered` clause with `scope: inherited, when:
/// end_of_your_turn, optional: true` and a body containing a single
/// `MayDnaDigivolveNow` step. This test confirms that shape (the same
/// integration crosscheck as `bt12_021_has_inherited_dna_digivolve_may_step`,
/// kept as a separate test for clear failure attribution).
#[test]
fn bt12_021_inherited_eot_dna_digivolve_has_correct_trigger_and_scope() {
    let runner = veemon_runner();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT12-021 in embedded DSL pack");

    let clause = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::EndOfYourTurn)
                && t.optional =>
        {
            Some(t)
        }
        _ => None,
    });
    let t = clause
        .expect("BT12-021 must have an inherited optional EoT triggered clause for DNA digivolve");
    assert!(
        t.process.iter().any(|step| matches!(
            step,
            CompiledStep::MayDnaDigivolveNow {
                ignore_requirements: true,
                cost: 0,
                ..
            }
        )),
        "process must contain a MayDnaDigivolveNow step with cost=0 and ignore_requirements=true"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Negative: clause 0 does not fire on opponent's play
// ═══════════════════════════════════════════════════════════════════════════════

/// When BT12-021 is played by the OPPONENT (player 1), player 0's hand
/// must not be affected. The OnPlay trigger fires for its controller (player 1),
/// installing a pending selection for player 1 to pick from the reveal.
/// Player 0's hand stays unchanged throughout.
///
/// Engine note: `place_on_field + fire_on_play` for player 1 DOES install
/// a pending selection (for player 1's reveal picks). We drive it to completion
/// and assert player 0's hand is untouched.
#[test]
fn bt12_021_on_play_by_opponent_does_not_affect_player_0_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-021")
        .expect("BT12-021 loads")
        .add_card(make_named_digimon("IMP-DIGI", "Imperialdramon Dragon Mode"))
        .add_card(make_named_tamer("DAVIS", "Davis Motomiya"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["IMP-DIGI", "DAVIS", "FILLER"])
        .memory(10)
        .start();

    let p0_hand_before = runner.hand_size(0);

    // Player 1 places BT12-021 on their field and fires OnPlay
    let perm = runner.place_on_field(1, "BT12-021", None);
    runner.fire_on_play(1, perm.index as usize);

    // Drive any pending selection (player 1's reveal picks) to completion
    runner.auto_resolve().ok();

    // Player 0's hand must be unchanged
    let p0_hand_after = runner.hand_size(0);
    assert_eq!(
        p0_hand_after, p0_hand_before,
        "Player 0's hand must not change when opponent plays BT12-021; \
         before={p0_hand_before} after={p0_hand_after}"
    );
}

/// G-DSL-EOT-DNA-INLINE (2026-05-24): the inherited EOT DNA digivolve
/// trigger fires on "end of YOUR turn" — scoped to the owner via the
/// `EndOfYourTurn` timing on the triggered clause. The behavioral test that
/// the action only appears for the owner (not the opponent) is handled by
/// the G2 mechanic-level test suite. Here we assert the timing is
/// `EndOfYourTurn` (not `EndOfTurn`), which encodes the owner-scoped trigger.
#[test]
fn bt12_021_inherited_eot_dna_digivolve_fires_on_owner_turn_only() {
    let runner = veemon_runner();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT12-021 in embedded DSL pack");

    let clause = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::EndOfYourTurn) =>
        {
            Some(t)
        }
        _ => None,
    });
    assert!(
        clause.is_some(),
        "inherited EoT DNA-digivolve clause must use EndOfYourTurn timing \
         (owner-scoped); a generic EndOfTurn timing would also fire on the \
         opponent's end step which the printed text forbids"
    );
}
