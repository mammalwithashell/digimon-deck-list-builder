//! Mammal (BT25 slice) — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/mammal-bt25-model.md`. Slice cards (all IMPLEMENTED,
//! per-card tests green): BT25-022 Lunamon, BT25-030 Elecmon, BT25-031 Patamon,
//! BT25-078 Gazimon — four Lv.3 `Mammal`/`Iliad`/`TS` rookies forming the
//! searchable rookie engine of the BT25 "TS" crossover decks.
//!
//! Per Phase-4 precondition gating, every authored combo names only implemented
//! slice cards (no blocked pieces). Cross-set cards are pulled ONLY if a combo
//! fires their *printed effect* — none of these do (they run on the four slice
//! cards plus synthetic neutral fixtures whose only relevant properties are
//! trait/level/text), so no cross-set implementation is pulled (lazy closure).
//!
//! Each `#[test]` maps 1:1 to a named combo in the model and asserts the
//! combo's claimed *mechanical* outcome — the system-level fact the per-card
//! tests cannot express (one mammal searching up another; an off-color Lv.2
//! source digivolving into a mammal; a tuck under an existing multi-card stack).
//!
//! Sources cited inline per combo: card text (cards.json / the YAML specs),
//! DCGO C# (`$BASE_DCGO/Assets/Scripts/CardEffect/BT25/<Color>/BT25_0xx.cs`),
//! and `general_rule.pdf` for digivolution / reveal-return timing.

#![allow(dead_code)]

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::selection::SelectionKind;

const LUNAMON: &str = "BT25-022";
const ELECMON: &str = "BT25-030";
const PATAMON: &str = "BT25-031";
const GAZIMON: &str = "BT25-078";

// ─── Shared fixtures ─────────────────────────────────────────────────────────

/// Push the given card IDs onto player 0's deck top; the LAST id ends up on top
/// (revealed first). Mirrors the per-card-test helper so the reveal order is
/// deterministic.
fn stack_deck_top(runner: &mut DebugRunner, ids: &[&str]) {
    for id in ids {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .unwrap_or_else(|| panic!("card {id} registered in the test pool"));
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .deck
            .push(CardSource::new(data_idx, 0, card_index));
    }
}

/// Pull the `[On Play]` (or shared) reveal process out of a compiled slice card.
fn on_play_process(runner: &DebugRunner, card_id: &str) -> Vec<CompiledStep> {
    runner
        .compiled_card(card_id)
        .unwrap_or_else(|| panic!("{card_id} compiled card present"))
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => {
                Some(t.process.clone())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("{card_id} must have an [On Play] reveal clause"))
}

/// A neutral Lv.2 Digimon carrying the `TS` trait, in the given color — the
/// off-color digivolution *source* for the mammals' cost-0 TS alt-path. The
/// color is chosen to *differ* from the mammal it sits under, to prove the
/// splash. No printed effect — only its level/trait/color matter.
fn make_ts_lv2(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(2);
    c.dp = Some(1000);
    c.play_cost = 2;
    c.traits = vec!["TS".to_string()];
    c
}

/// A neutral off-color Lv.2 Digimon WITHOUT the `TS` trait — the unhappy-path
/// source. It must be rejected as a digivolution base for a mammal, because the
/// alt-path gates on the TS trait and the printed path gates on matching color.
fn make_plain_lv2(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(2);
    c.dp = Some(1000);
    c.play_cost = 2;
    c.traits = vec!["Beast".to_string()];
    c
}

/// Find a card's index in player 0's hand (the `hand_index` arg of
/// `Game::digivolve_from_hand`).
fn hand_index(runner: &DebugRunner, card_id: &str) -> usize {
    runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} in P0 hand"))
}

// ═════════════════════════════════════════════════════════════════════════════
// Combo MA1 — "Lunamon search adds a second mammal" (self-feeding engine)
// ═════════════════════════════════════════════════════════════════════════════
//
// Cards: BT25-022 Lunamon (search) + BT25-031 Patamon (the [Iliad] pick — a
// mammal, so it carries Iliad) + BT25-030 Elecmon (the [TS] pick).
//
// Claimed outcome: Lunamon's [On Play] reveals the top 3; the Iliad bucket takes
// Patamon and the TS bucket takes Elecmon — BOTH to hand; the remaining filler
// returns to the deck bottom. The *system* fact a per-card test cannot show: a
// mammal (Patamon) is added to hand AS ANOTHER MAMMAL'S Iliad pick, refueling
// the next rookie play — the engine is self-feeding.
//
// Sources: DCGO BT25_022.cs `SimplifiedRevealDeckTopCardsAndSelect`, buckets
// `HasIliadTraits` / `HasTSTraits`, `mutualConditions: true`; general_rule.pdf
// reveal/return-to-bottom timing.

/// Happy path: a second mammal (Patamon) is searched to hand as Lunamon's Iliad
/// pick, alongside Elecmon as the TS pick.
#[test]
fn ma1_lunamon_search_adds_a_second_mammal_to_hand() {
    let filler = make_test_card("FILL-MA1", "Filler MA1");
    let holder = make_test_card("MA1-HOLDER", "Holder");

    let mut runner = DebugRunner::builder()
        .dsl_card(LUNAMON)
        .expect("BT25-022 (Lunamon) in embedded DSL pack")
        .dsl_card(PATAMON)
        .expect("BT25-031 (Patamon) in embedded DSL pack")
        .dsl_card(ELECMON)
        .expect("BT25-030 (Elecmon) in embedded DSL pack")
        .add_card(filler)
        .add_card(holder)
        .build();

    // Lunamon stands on the field as the [On Play] source carrier.
    let holder_perm = runner.place_on_field(0, "MA1-HOLDER", None);
    let src = runner.top_card(holder_perm);

    let hand_before = runner.game.players[0].hand.len();
    let deck_before = runner.game.players[0].deck.len();
    // Top-3 (last pushed = revealed first): FILL, ELECMON, PATAMON.
    stack_deck_top(&mut runner, &["FILL-MA1", ELECMON, PATAMON]);

    let process = on_play_process(&runner, LUNAMON);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, Some(holder_perm), 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }
    runner.auto_resolve().expect("both bucket picks resolve");

    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();

    // ── Claimed mechanical outcome ───────────────────────────────────────────
    // 1. Patamon — a fellow mammal — was added to hand as the [Iliad] pick.
    assert!(
        hand_ids.contains(&PATAMON),
        "the self-feeding engine: Patamon (a mammal) is searched up as Lunamon's [Iliad] pick: {hand_ids:?}"
    );
    // 2. Elecmon was added as the [TS] pick.
    assert!(
        hand_ids.contains(&ELECMON),
        "Elecmon added to hand as the [TS] pick: {hand_ids:?}"
    );
    // 3. Exactly two cards entered the hand.
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 2,
        "exactly the two bucket picks entered hand"
    );
    // 4. Net deck: 3 revealed, 2 picks left for hand, 1 filler returned to
    //    bottom. Relative to the pre-stack baseline that is +1 (3 stacked − 2
    //    taken). Matches the per-card BT25-022 reveal accounting.
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before + 1,
        "of the 3 stacked + revealed cards, 2 went to hand and 1 returned to bottom"
    );
    assert!(
        runner.pending_selection().is_none(),
        "the mandatory two-bucket pick fully resolves"
    );
}

/// Unhappy path: only ONE mammal in the top-3 and no other Iliad/TS partner.
/// `no_duplicate_cards` (DCGO `mutualConditions: true`) forbids that single
/// mammal filling BOTH the Iliad and the TS bucket, so only one bucket can fill
/// — the self-feeding chain stalls when a distinct second target is absent.
#[test]
fn ma1_no_duplicate_blocks_one_mammal_filling_both_buckets() {
    // Two non-Iliad / non-TS fillers + one mammal (Patamon, has both Iliad+TS).
    let fa = make_test_card("FA-MA1B", "fa");
    let fb = make_test_card("FB-MA1B", "fb");
    let holder = make_test_card("MA1B-HOLDER", "Holder");

    let mut runner = DebugRunner::builder()
        .dsl_card(LUNAMON)
        .expect("BT25-022 (Lunamon)")
        .dsl_card(PATAMON)
        .expect("BT25-031 (Patamon)")
        .add_card(fa)
        .add_card(fb)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "MA1B-HOLDER", None);
    let src = runner.top_card(holder_perm);

    let hand_before = runner.game.players[0].hand.len();
    // Top-3: FB, FA, PATAMON (Patamon revealed first/on top).
    stack_deck_top(&mut runner, &["FB-MA1B", "FA-MA1B", PATAMON]);

    let process = on_play_process(&runner, LUNAMON);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, Some(holder_perm), 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }
    let _ = runner.auto_resolve();

    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();

    // Patamon satisfies BOTH bucket filters by trait, but no_duplicate_cards
    // means it can fill only ONE bucket → exactly ONE card enters hand, and the
    // other bucket fizzles (no distinct second candidate).
    assert!(
        hand_ids.contains(&PATAMON),
        "the lone mammal fills one bucket: {hand_ids:?}"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 1,
        "no_duplicate_cards: one card cannot fill both buckets → only one pick enters hand"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no pending selection lingers after the single-candidate fizzle"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Combo MA2 — "off-color TS Lv.2 free-digivolves into a mammal"
// ═════════════════════════════════════════════════════════════════════════════
//
// Cards: a synthetic Lv.2 [TS] source in a DIFFERENT color (Blue) + BT25-031
// Patamon (a Yellow rookie) in hand.
//
// Claimed outcome: the TS-trait Lv.2 — though Blue, not Yellow — is a legal
// cost-0 digivolution source for (Yellow) Patamon via the mammals' shared
// `trait_has: TS / cost: 0` alt-path. The digivolution succeeds at memory cost
// 0 across the color line. This is the deck's color-splash identity, exercised
// at the *digivolution recipe* level and cross-card (the source is a different
// card than the mammal) — beyond the per-card `alt_paths.len()==2` structural
// check.
//
// Sources: each mammal YAML `alt_paths` (`trait_has: TS`, `cost: 0`,
// `level_eq: 2`); DCGO BT25_031.cs `AddSelfDigivolutionRequirementStaticEffect(
// level:2)` gated on `TopCard.HasTSTraits`; general_rule.pdf digivolution.

/// Happy path: an off-color (Blue) Lv.2 TS source digivolves into Patamon
/// (Yellow) for cost 0, proving the cross-color splash recipe.
#[test]
fn ma2_off_color_ts_lv2_digivolves_into_mammal_for_zero() {
    let mut runner = DebugRunner::builder()
        .dsl_card(PATAMON)
        .expect("BT25-031 (Patamon) in embedded DSL pack")
        .add_card(make_ts_lv2("TS-LV2-BLUE", CardColor::Blue))
        // deck pad so the digivolve's mandatory rule-draw has a card to take.
        .add_card(make_test_card("MA2-PAD", "pad"))
        .deck(0, &["MA2-PAD", "MA2-PAD", "MA2-PAD", "MA2-PAD"])
        .deck(1, &["MA2-PAD"])
        .hand(0, &[PATAMON])
        .memory(10)
        .start();

    // The off-color Blue Lv.2 TS source stands on P0's field.
    let base = runner.place_on_field(0, "TS-LV2-BLUE", Some(0));
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "TS-LV2-BLUE",
        "Blue Lv.2 TS source on field"
    );
    let memory_before = runner.memory();
    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    // Drive the real digivolution of Patamon (from hand) onto the off-color
    // Blue Lv.2. `digivolve_from_hand` resolves the route via the registered
    // alt-paths — it must accept the off-color source because Patamon's TS
    // alt-path gates on the TS trait, NOT color.
    let h = hand_index(&runner, PATAMON);
    let ok = runner
        .game
        .digivolve_from_hand(0, h, base.index as usize, PlaySource::ByDigivolve);
    assert!(
        ok,
        "off-color (Blue) Lv.2 TS source must be a legal cost-0 digivolution \
         source for (Yellow) Patamon via the TS alt-path"
    );
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    // ── Claimed mechanical outcome ───────────────────────────────────────────
    // 1. Patamon is now the top of the stack, over the Blue Lv.2 source.
    let perm = &runner.game.players[0].battle_area[base.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        PATAMON,
        "Patamon digivolved onto the off-color Lv.2 TS source"
    );
    assert!(
        perm.card_sources.len() >= 2,
        "the Blue Lv.2 remains under Patamon as a digivolution source"
    );
    assert_eq!(
        perm.card_sources[0].card_id(&runner.game.card_data),
        "TS-LV2-BLUE",
        "the off-color Lv.2 is the bottom source under the mammal"
    );
    // 2. It was a cost-0 alt-digivolve: memory is unchanged by the digivolve
    //    itself (the standard digivolve rule-draw does not touch memory).
    assert_eq!(
        runner.memory(),
        memory_before,
        "the TS alt-path is cost 0 → no memory paid for the cross-color digivolve"
    );
    // 3. Digivolving draws 1 by rule; Patamon left hand to evolve;
    //    net hand = (−1 Patamon stacked) + (+1 rule draw) = unchanged.
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "Patamon left hand to evolve (−1) and the digivolve drew 1 (+1) → net hand unchanged"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "the digivolve rule-draw removed one card from the deck"
    );
}

/// Unhappy path: an off-color (Blue) Lv.2 WITHOUT the TS trait must be rejected
/// as a base for (Yellow) Patamon. Neither recipe matches: the printed path
/// needs matching color (Yellow), and the alt-path needs the TS trait — the
/// plain Blue Beast Lv.2 has neither. This pins that the splash is gated on the
/// TS trait, not on "any Lv.2".
#[test]
fn ma2_off_color_non_ts_lv2_is_rejected_as_base() {
    let mut runner = DebugRunner::builder()
        .dsl_card(PATAMON)
        .expect("BT25-031 (Patamon) in embedded DSL pack")
        .add_card(make_plain_lv2("PLAIN-LV2-BLUE", CardColor::Blue))
        .add_card(make_test_card("MA2B-PAD", "pad"))
        .deck(0, &["MA2B-PAD", "MA2B-PAD", "MA2B-PAD", "MA2B-PAD"])
        .deck(1, &["MA2B-PAD"])
        .hand(0, &[PATAMON])
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "PLAIN-LV2-BLUE", Some(0));
    let hand_before = runner.hand_size(0);

    let h = hand_index(&runner, PATAMON);
    let ok = runner
        .game
        .digivolve_from_hand(0, h, base.index as usize, PlaySource::ByDigivolve);
    assert!(
        !ok,
        "an off-color Lv.2 without the TS trait must NOT be a legal base — the \
         splash recipe is gated on the TS trait, not on any Lv.2"
    );
    // The board is untouched: Patamon stayed in hand, the base is unchanged.
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "PLAIN-LV2-BLUE",
        "the rejected base remains a plain Lv.2"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "the rejected digivolve leaves Patamon in hand"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Combo MA3 — "Gazimon tucks a [Three Musketeers] source under an existing stack"
// ═════════════════════════════════════════════════════════════════════════════
//
// Cards: BT25-078 Gazimon standing on a multi-card stack + a revealed
// [Three Musketeers]-TRAIT card in the deck top-3.
//
// Claimed outcome: choosing Gazimon's *place* branch tucks the
// [Three Musketeers]-trait card as Gazimon's BOTTOM digivolution card — i.e.
// underneath the existing source(s) of the stack, NOT to hand; the remaining
// revealed cards return to the deck bottom. The system fact a per-card test
// only partly shows: the placement is at the BOTTOM of an *already multi-card*
// stack (the per-card test placed under a single-card holder).
//
// Sources: DCGO BT25_078.cs — `AddDigivolutionCardsBottom` on the place branch
// vs `AddHandCards` on the add branch, `canNoSelect: true` (optional);
// general_rule.pdf digivolution-card placement (bottom).

/// Happy path: the place branch tucks a [Three Musketeers]-trait card as the
/// new BOTTOM source of Gazimon's existing two-card stack.
#[test]
fn ma3_gazimon_place_branch_tucks_under_existing_stack() {
    let mut tm_trait = make_test_card("TM-TRAIT-MA3", "TM Trait MA3");
    tm_trait.traits.push("Three Musketeers".to_string());
    let under = make_test_card("UNDER-MA3", "Under source");
    let fa = make_test_card("FA-MA3", "fa");
    let fb = make_test_card("FB-MA3", "fb");

    let mut runner = DebugRunner::builder()
        .dsl_card(GAZIMON)
        .expect("BT25-078 (Gazimon) in embedded DSL pack")
        .add_card(tm_trait)
        .add_card(under)
        .add_card(fa)
        .add_card(fb)
        .build();

    // A genuine multi-card stack: [UNDER-MA3 (bottom), Gazimon (top)].
    let stack = runner.place_stack(0, &["UNDER-MA3", GAZIMON]);
    let src = runner.top_card(stack);
    let sources_before = runner.game.players[0].battle_area[stack.index as usize]
        .card_sources
        .len();
    assert_eq!(sources_before, 2, "stack starts as [UNDER-MA3, Gazimon]");

    let deck_before = runner.game.players[0].deck.len();
    // Top-3: FB, FA, TM-TRAIT (TM-TRAIT revealed on top).
    stack_deck_top(&mut runner, &["FB-MA3", "FA-MA3", "TM-TRAIT-MA3"]);

    let process = on_play_process(&runner, GAZIMON);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, Some(stack), 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }

    // First prompt: the hand-vs-source effect choice → pick branch 1 (place).
    let choice = runner
        .pending_selection()
        .expect("Gazimon's effect-choice prompt surfaces");
    assert_eq!(choice.kind, SelectionKind::EffectChoice);
    let place_action = *choice
        .valid_action_ids
        .get(1)
        .expect("branch 1 (place-as-source) action present");
    runner
        .execute_action(0, place_action)
        .expect("choose the place-as-bottom-source branch");
    runner.auto_resolve().expect("reveal pick + remainder resolve");

    // ── Claimed mechanical outcome ───────────────────────────────────────────
    let perm = &runner.game.players[0].battle_area[stack.index as usize];
    let source_ids: Vec<&str> = perm
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    // 1. The TM-trait card joined the stack as a source (one more than before).
    assert_eq!(
        perm.card_sources.len(),
        sources_before + 1,
        "exactly one card was tucked under the existing stack"
    );
    // 2. It is the BOTTOM source — beneath the prior UNDER-MA3 base.
    assert_eq!(
        source_ids.first().copied(),
        Some("TM-TRAIT-MA3"),
        "the [Three Musketeers] card is the new BOTTOM digivolution card of the stack: {source_ids:?}"
    );
    assert_eq!(
        source_ids.get(1).copied(),
        Some("UNDER-MA3"),
        "the original base source is now ABOVE the tucked card"
    );
    // 3. Gazimon remains the top of the stack (unchanged).
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        GAZIMON,
        "Gazimon is still the top of the stack after the tuck"
    );
    // 4. The tucked card did NOT go to hand.
    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        !hand_ids.contains(&"TM-TRAIT-MA3"),
        "the place branch must NOT add the card to hand: {hand_ids:?}"
    );
    // 5. Of the 3 stacked + revealed cards, 1 was tucked under the stack and 2
    //    fillers returned to bottom → relative to the pre-stack baseline, +2.
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before + 2,
        "one card was tucked (left deck), the two fillers returned to bottom"
    );
}

/// Unhappy path: Gazimon's whole clause is optional ("...or you may place...").
/// Declining at the optional/effect-choice layer leaves the stack and hand
/// untouched — the tuck is never forced.
#[test]
fn ma3_gazimon_optional_decline_leaves_stack_untouched() {
    let mut tm_trait = make_test_card("TM-TRAIT-MA3B", "TM Trait MA3B");
    tm_trait.traits.push("Three Musketeers".to_string());
    let under = make_test_card("UNDER-MA3B", "Under source");

    let mut runner = DebugRunner::builder()
        .dsl_card(GAZIMON)
        .expect("BT25-078 (Gazimon)")
        .add_card(tm_trait)
        .add_card(under)
        .build();

    let stack = runner.place_stack(0, &["UNDER-MA3B", GAZIMON]);
    let src = runner.top_card(stack);
    let sources_before = runner.game.players[0].battle_area[stack.index as usize]
        .card_sources
        .len();
    let hand_before = runner.game.players[0].hand.len();
    stack_deck_top(&mut runner, &["TM-TRAIT-MA3B"]);

    // Confirm the clause is optional at the structural level (no-approx: the
    // PASS is exposed because the printed text says "you may").
    let clause_optional = runner
        .compiled_card(GAZIMON)
        .expect("compiled")
        .effects
        .iter()
        .any(|c| matches!(c, CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::OnPlay) && t.optional));
    assert!(clause_optional, "Gazimon's shared clause is optional");

    let process = on_play_process(&runner, GAZIMON);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, Some(stack), 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }
    // Decline at whatever optional gate surfaces.
    if let Some(sel) = runner.pending_selection() {
        if sel.is_optional || sel.valid_action_ids.contains(&digimon_engine::action::space::PASS) {
            let _ = runner
                .game
                .resolve_selection(0, digimon_engine::action::space::PASS);
        }
    }
    let _ = runner.auto_resolve();

    // Stack source count unchanged; nothing tucked, nothing added to hand.
    let perm = &runner.game.players[0].battle_area[stack.index as usize];
    assert_eq!(
        perm.card_sources.len(),
        sources_before,
        "declining the optional clause tucks nothing under the stack"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "declining adds nothing to hand"
    );
}
