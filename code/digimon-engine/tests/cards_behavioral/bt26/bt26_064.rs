//! BT26-064 DemiDevimon — Digimon, Lv.3, Purple, DP 2000, Play cost 3.
//! Rookie / Virus / Evil, Iliad, ADAMAS, TS.
//! Digivolve: Purple Lv.2 / cost 0; [Digivolve] Lv.2 w/[TS] trait: Cost 0.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT26-064.md; card image)
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 card with the
//! [Fallen Angel], [Undead], [Wizard] or [Demon Lord] trait and 1 card with the
//! [TS] trait among them to the hand. Return the rest to the bottom of the deck.
//!
//! Inherited Effect: [When Attacking] [Once Per Turn] <Draw 1> and trash 1 card
//! in your hand.
//!
//! # DCGO C# reference
//! `Assets/Scripts/CardEffect/BT26/Purple/BT26_064.cs`:
//! - `AddSelfDigivolutionRequirementStaticEffect(level 2, TopCard.HasTSTraits
//!   (= EqualsTraits "TS"), cost 0)` → `alt_paths`.
//! - On Play: `SimplifiedRevealDeckTopCardsAndSelect(3)` with two ordered
//!   AddHand buckets (maxCount 1 each): `EqualsTraits` Fallen Angel / Undead /
//!   Wizard / Demon Lord, then `HasTSTraits`; remainder → deck bottom; not
//!   optional.
//! - Inherited (OnAllyAttack, maxPerTurn 1): `DrawClass(1)`, then if the hand
//!   is non-empty a `canNoSelect: false` Discard of 1.
//!
//! Exemplar: BT26-065 Falcomon (same two-bucket reveal + same inherited).
//!
//! # Patterns this test covers
//! - alt-path digivolve from a Lv.2 [TS] Digimon for cost 0 (+ negative)
//! - A2-style two-bucket reveal (mandatory, no duplicate card, trait gates)
//! - Inherited [When Attacking][OPT] draw-then-discard (+ lockout / reset)

use super::support::*;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, PlaySource};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-064";

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-064 in embedded DSL pack")
        .add_card(filler("FILLER"))
        // Bucket 1 only: each of the four printed traits.
        .add_card(digimon("ANGEL", "Angelmon", CardColor::Purple, 4, 5, &["Fallen Angel"]))
        .add_card(digimon("UNDEAD", "Undeadmon", CardColor::Black, 4, 5, &["Undead"]))
        .add_card(digimon("WIZARD", "Wizmon", CardColor::Blue, 4, 5, &["Wizard"]))
        .add_card(digimon("LORD", "Lordmon", CardColor::Purple, 6, 12, &["Demon Lord"]))
        // Bucket 2 only.
        .add_card(digimon("TS-CARD", "Tsmon", CardColor::Red, 3, 3, &["TS"]))
        // Legal for BOTH buckets.
        .add_card(digimon("BOTH", "Bothmon", CardColor::Purple, 5, 7, &["Fallen Angel", "TS"]))
        // Legal for neither: near-miss traits.
        .add_card(digimon("EVIL", "Evilmon", CardColor::Purple, 4, 5, &["Evil", "Demon", "Iliad"]))
        // Alt-path bases (red, so the printed Purple Lv.2 circle can't match).
        .add_card(digimon("TS-EGG", "Tsegg", CardColor::Red, 2, 0, &["TS"]))
        .add_card(digimon("PLAIN-EGG", "Plainegg", CardColor::Red, 2, 0, &["Iliad"]))
        .add_card(digimon("HOST", "Host", CardColor::Purple, 4, 5, &[]))
}

/// `deck_top[0]` is the top card of the deck; 4 FILLER sit below.
fn setup(deck_top: &[&str]) -> DebugRunner {
    let mut deck: Vec<&str> = vec!["FILLER"; 4];
    deck.extend(deck_top.iter().rev());
    let mut r = builder()
        .deck(0, &deck)
        .deck(1, &["FILLER"; 5])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

/// Play DemiDevimon from hand (real play path → On Play fires).
fn play_demidevimon(r: &mut DebugRunner) {
    let idx = r.add_to_hand(0, CARD_ID);
    r.play(0, idx).expect("BT26-064 played from hand");
}

// ─── Structure ──────────────────────────────────────────────────────────────

#[test]
fn bt26_064_structure_and_ts_alt_path() {
    let r = setup(&[]);
    let card = r.compiled_card(CARD_ID).unwrap();
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(2000));
    let alt = format!("{:?}", card.alt_paths);
    assert!(
        alt.contains("\"TS\"") && alt.contains("level_eq: Some(2)"),
        "alt path must gate on Lv.2 + [TS]: {alt}"
    );
}

// ─── Alt digivolve: Lv.2 w/[TS] trait, cost 0 ───────────────────────────────

fn try_digivolve_onto(base: &str) -> (DebugRunner, PermanentHandle, bool) {
    let mut r = setup(&[]);
    let h = r.place_on_field(0, base, Some(0));
    let idx = r.add_to_hand(0, CARD_ID);
    let ok = r
        .game
        .digivolve_from_hand(0, idx, h.index as usize, PlaySource::ByDigivolve);
    (r, h, ok)
}

#[test]
fn bt26_064_alt_path_digivolves_from_lv2_ts_for_zero() {
    let (mut r, h, ok) = try_digivolve_onto("TS-EGG");
    assert!(ok, "a red Lv.2 [TS] Digimon digivolves via the [TS] alt path");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, h), CARD_ID);
    assert_eq!(r.memory(), 3, "digivolution cost 0");
    // On Play is not When Digivolving: no reveal prompt.
    assert!(r.game.revealed_cards.is_empty());
    assert!(r.pending_selection_view().is_none());
}

#[test]
fn bt26_064_alt_path_rejects_lv2_without_ts() {
    let (r, h, ok) = try_digivolve_onto("PLAIN-EGG");
    assert!(!ok, "a red Lv.2 Digimon without [TS] has no legal route");
    assert_eq!(top_id(&r, h), "PLAIN-EGG");
}

// ─── [On Play] reveal 3: 1 Fallen Angel/Undead/Wizard/Demon Lord + 1 [TS] ───

#[test]
fn bt26_064_on_play_adds_one_per_bucket_and_bottoms_the_rest() {
    let mut r = setup(&["ANGEL", "TS-CARD", "EVIL"]);
    play_demidevimon(&mut r);
    let v = r.pending_selection_view().expect("bucket 1 prompt");
    assert!(!v.is_optional, "'Add 1' is not a 'may'");
    pick_reveal(&mut r, 0, "ANGEL");
    pick_reveal(&mut r, 0, "TS-CARD");
    let _ = r.auto_resolve();

    let hand = hand_ids(&r, 0);
    assert!(hand.contains(&"ANGEL".to_string()), "{hand:?}");
    assert!(hand.contains(&"TS-CARD".to_string()), "{hand:?}");
    assert_eq!(hand.len(), 2);
    // EVIL (no qualifying trait) went to the BOTTOM of the deck.
    assert_eq!(deck_ids(&r, 0)[0], "EVIL");
    assert_eq!(r.deck_size(0), 5);
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn bt26_064_on_play_first_bucket_accepts_each_printed_trait() {
    for id in ["ANGEL", "UNDEAD", "WIZARD", "LORD"] {
        let mut r = setup(&[id, "FILLER", "FILLER"]);
        play_demidevimon(&mut r);
        pick_reveal(&mut r, 0, id);
        let _ = r.auto_resolve();
        assert_eq!(hand_ids(&r, 0), vec![id.to_string()], "{id} qualifies");
    }
}

#[test]
fn bt26_064_on_play_near_miss_traits_do_not_qualify() {
    // [Evil] / [Demon] / [Iliad] are not [Fallen Angel]/[Undead]/[Wizard]/
    // [Demon Lord] and not [TS]: nothing is added, all 3 go to the bottom.
    let mut r = setup(&["EVIL", "FILLER", "FILLER"]);
    play_demidevimon(&mut r);
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).is_empty());
    assert_eq!(r.deck_size(0), 7);
    assert!(deck_ids(&r, 0)[..3].contains(&"EVIL".to_string()));
}

#[test]
fn bt26_064_on_play_ts_bucket_alone() {
    let mut r = setup(&["TS-CARD", "FILLER", "FILLER"]);
    play_demidevimon(&mut r);
    pick_reveal(&mut r, 0, "TS-CARD");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["TS-CARD".to_string()]);
}

#[test]
fn bt26_064_on_play_same_card_cannot_fill_both_buckets() {
    let mut r = setup(&["BOTH", "FILLER", "FILLER"]);
    play_demidevimon(&mut r);
    pick_reveal(&mut r, 0, "BOTH");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["BOTH".to_string()]);
    assert_eq!(r.deck_size(0), 6);
}

// ─── Inherited [When Attacking][OPT] <Draw 1> and trash 1 card ──────────────

#[test]
fn bt26_064_inherited_draws_then_trashes_on_attack() {
    let mut r = setup(&["FILLER"; 3]);
    let carrier = r.place_stack(0, &[CARD_ID, "HOST"]);
    push_hand(&mut r, 0, "ANGEL");
    let res = r.attack_player(carrier, 1, false);
    assert_eq!(res, AttackResult::InProgress);
    let v = r.pending_selection_view().expect("discard prompt");
    assert!(!v.is_optional, "the trash is mandatory");
    assert_eq!(r.hand_size(0), 2, "drew before the discard prompt");
    pick_hand(&mut r, 0, "ANGEL");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["FILLER".to_string()]);
    assert_eq!(trash_ids(&r, 0), vec!["ANGEL".to_string()]);
}

#[test]
fn bt26_064_inherited_not_active_while_demidevimon_is_the_top_card() {
    let mut r = setup(&["FILLER"; 3]);
    let solo = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, solo);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 0, "an inherited effect only works from under a Digimon");
}

#[test]
fn bt26_064_inherited_once_per_turn_and_resets_next_turn() {
    let mut r = setup(&["FILLER"; 3]);
    let carrier = r.place_stack(0, &[CARD_ID, "HOST"]);
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 0, "drew 1, trashed 1");
    assert_eq!(r.trash_size(0), 1);

    // Same turn: locked out.
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.trash_size(0), 1, "second activation this turn is gated");

    // Next own turn: the lockout has cleared.
    r.end_turn();
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.game.turn_player(), 0);
    let before_trash = r.trash_size(0);
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    assert!(
        r.pending_selection_view().is_some(),
        "discard prompt again on the next turn"
    );
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.trash_size(0), before_trash + 1);
}
