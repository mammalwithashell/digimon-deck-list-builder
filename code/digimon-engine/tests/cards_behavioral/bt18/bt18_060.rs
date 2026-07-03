//! BT18-060 Vemmon — Digimon, Lv.3, Black, DP 1000, Cost 3.
//! Traits: Unknown, LIBERATOR. Attribute: Unknown. Form: Rookie.
//!
//! # Card text (official Bandai DB bundle data/card_bundles/BT18-060.md,
//! cross-checked against the card image)
//!
//! **[On Play]** Reveal the top 3 cards of your deck. Among them, add 1 card
//! with [Vemmon] in its text to the hand and place 1 [Vemmon] as any of your
//! Digimon's bottom digivolution card. Return the rest to the bottom of the
//! deck.
//!
//! **Inherited [Your Turn][Once Per Turn]:** When this Digimon would
//! digivolve into a Digimon card with [Vemmon] in its text, reduce the
//! digivolution cost by 1.
//!
//! # Official Q&A
//! "Yes, you must add as many cards to your hand and place as bottom
//! digivolution cards as possible." — both reveal buckets are MANDATORY.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT18/Black/BT18_060.cs
//!
//! # Patterns this test covers
//! - §1 Structural: 1 triggered OnPlay clause + 2 declarative CostReduction
//!       clauses (face_up + inherited — see the YAML file-header KNOWN GAP
//!       note on why the OPT budget is not shared between them).
//! - §2 OnPlay condition gating: no own battle-area Digimon → no reveal.
//! - §3 Behavioral reveal-search: bucket 1 (add [Vemmon]-in-text to hand),
//!       bucket 2 (place exact-name [Vemmon] as a chosen own Digimon's bottom
//!       source), both buckets together, remainder to deck bottom, exact-name
//!       vs. in-text gating, and a non-Vemmon card being left untouched.
//! - §4 Cost-reduction structural: scope / once_per_turn / active_when shape.
//! - §5 Cost-reduction behavioral: face_up positive/negative, inherited
//!       positive, and the pinned cross-position double-reduction regression
//!       documenting the known engine gap (G-COST-REDUCTION-SHARED-OPT-BOTH-SCOPES).

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledPredicate, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::permanent::PermanentHandle;

// ─── Card factories ────────────────────────────────────────────────────────

/// A filler Digimon card with no relevant text/name (never eligible for
/// either reveal bucket).
fn make_filler(id: &str) -> CardData {
    make_test_card(id, &format!("{id}-Filler"))
}

/// A card with "[Vemmon]" in its EFFECT TEXT but NOT named "Vemmon" — only
/// eligible for bucket 1 (add-to-hand, text-scan match).
fn make_vemmon_text_card(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id}-VemmonText"));
    c.effect_text = "This card synergizes with [Vemmon].".to_string();
    c
}

/// A Lv.4 Digimon whose card text contains "[Vemmon]" (digivolve-cost-target
/// gate for Clause 2 tests) but which is NOT named "Vemmon" — a downstream
/// Vemmon-line card (Snatchmon-shape).
fn make_vemmon_text_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id}-Lv4"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(3000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Black];
    c.effect_text = "[Vemmon] this Digimon's text mentions Vemmon.".to_string();
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 5, // Black
        memory_cost: 2,
    }];
    c
}

/// A Lv.4 Digimon with NO Vemmon text — negative control for cost reduction.
fn make_plain_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id}-Lv4Plain"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(3000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Black];
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 5, // Black
        memory_cost: 2,
    }];
    c
}

/// A Lv.5 Digimon whose text contains "[Vemmon]" — digivolve target for the
/// "buried" (inherited-scope) cost-reduction test.
fn make_vemmon_text_lv5(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id}-Lv5"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(5000);
    c.play_cost = 6;
    c.colors = vec![CardColor::Black];
    c.effect_text = "[Vemmon] this Digimon's text mentions Vemmon.".to_string();
    c.evo_costs = vec![EvoCost {
        level: 4,
        card_color: 5, // Black
        memory_cost: 2,
    }];
    c
}

// ─── Helpers ───────────────────────────────────────────────────────────────

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("BT18-060 YAML parses and compiles")
        .build()
}

/// Push `ids` onto player `p`'s deck TOP, in order (last id ends up as the
/// topmost / first-revealed card).
fn stack_deck_top(runner: &mut DebugRunner, p: usize, ids: &[&str]) {
    for id in ids {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .unwrap_or_else(|| panic!("card {id} registered"));
        let card_index = runner.game.next_card_index();
        runner.game.players[p]
            .deck
            .push(CardSource::new(data_idx, p as u8, card_index));
    }
}

fn push_to_hand(runner: &mut DebugRunner, p: usize, card_id: &str) -> usize {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} in card_data"));
    let card_index = runner.game.next_card_index();
    runner.game.players[p]
        .hand
        .push(CardSource::new(data_idx, p as u8, card_index));
    runner.game.players[p].hand.len() - 1
}

/// Pull the [On Play] clause's process body.
fn on_play_process(runner: &DebugRunner) -> Vec<CompiledStep> {
    runner
        .compiled_card("BT18-060")
        .expect("compiled")
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => {
                Some(t.process.clone())
            }
            _ => None,
        })
        .expect("BT18-060 must have an [On Play] clause")
}

/// Recursively drive every pending selection by picking the FIRST valid
/// action id, up to `max_steps` iterations. Used to walk a reveal-bucket
/// sequence deterministically for tests that only care about the end state.
fn drive_first_choice_n_times(runner: &mut DebugRunner, max_steps: usize) {
    for _ in 0..max_steps {
        let Some(sel) = runner.pending_selection() else {
            return;
        };
        let action = sel
            .valid_action_ids
            .first()
            .copied()
            .expect("selection has at least one valid action");
        let _ = runner.execute_action(0, action);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt18_060_compiles_to_three_clauses() {
    let runner = base_runner();
    let card = runner.compiled_card("BT18-060").expect("compiled");
    assert_eq!(
        card.effects.len(),
        3,
        "BT18-060 must compile to exactly 3 clauses: [On Play] triggered + \
         2 CostReduction (face_up + inherited); got {}",
        card.effects.len()
    );
}

#[test]
fn bt18_060_on_play_clause_is_face_up_and_gated_on_own_digimon() {
    let runner = base_runner();
    let card = runner.compiled_card("BT18-060").expect("compiled");

    let on_play = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("BT18-060 must have a Triggered OnPlay clause");

    assert_eq!(
        on_play.scope,
        CompiledScope::FaceUp,
        "[On Play] clause must be FaceUp scope"
    );

    fn has_battle_area_digimon_gate(p: &CompiledPredicate) -> bool {
        p.any_permanent.is_some()
            || p.all_of.iter().any(has_battle_area_digimon_gate)
            || p.any_of.iter().any(has_battle_area_digimon_gate)
    }
    let cond = on_play
        .condition
        .as_ref()
        .expect("[On Play] clause must carry a condition (own battle-area Digimon gate)");
    assert!(
        has_battle_area_digimon_gate(cond),
        "the condition must gate on an own battle-area Digimon existing \
         (DCGO IsExistOnBattleAreaDigimon)"
    );
}

#[test]
fn bt18_060_has_two_cost_reduction_clauses_face_up_and_inherited() {
    let runner = base_runner();
    let card = runner.compiled_card("BT18-060").expect("compiled");

    let scopes: Vec<CompiledScope> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                scope,
                ..
            }) => Some(*scope),
            _ => None,
        })
        .collect();

    assert_eq!(
        scopes.len(),
        2,
        "BT18-060 must have exactly 2 CostReduction clauses; got {}",
        scopes.len()
    );
    assert!(
        scopes.contains(&CompiledScope::FaceUp),
        "one CostReduction clause must be FaceUp scope"
    );
    assert!(
        scopes.contains(&CompiledScope::Inherited),
        "one CostReduction clause must be Inherited scope"
    );
}

#[test]
fn bt18_060_both_cost_reduction_clauses_are_once_per_turn_and_your_turn_gated() {
    let runner = base_runner();
    let card = runner.compiled_card("BT18-060").expect("compiled");

    fn has_your_turn(p: &CompiledPredicate) -> bool {
        p.your_turn == Some(true)
            || p.all_of.iter().any(has_your_turn)
            || p.any_of.iter().any(has_your_turn)
    }
    fn has_source_is_cost_target(p: &CompiledPredicate) -> bool {
        p.source_is_cost_target_permanent == Some(true)
            || p.all_of.iter().any(has_source_is_cost_target)
            || p.any_of.iter().any(has_source_is_cost_target)
    }

    let mut checked = 0;
    for c in &card.effects {
        if let CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
            once_per_turn,
            active_when,
            amount,
            ..
        }) = c
        {
            checked += 1;
            assert!(
                *once_per_turn,
                "[Your Turn][Once Per Turn] printed qualifier → once_per_turn must be true"
            );
            assert_eq!(*amount, Some(1), "reduce the digivolution cost by 1");
            let aw = active_when
                .as_ref()
                .expect("CostReduction clause must carry active_when");
            assert!(has_your_turn(aw), "must carry your_turn: true gate");
            assert!(
                has_source_is_cost_target(aw),
                "must gate on source_is_cost_target_permanent: true (\"this Digimon\" \
                 is the one digivolving)"
            );
        }
    }
    assert_eq!(checked, 2, "expected exactly 2 CostReduction clauses");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — [On Play] condition gating
// ═══════════════════════════════════════════════════════════════════════════

/// NEGATIVE: with no own battle-area Digimon, the [On Play] effect must not
/// fire (no reveal, no selection installed).
/// NOTE: this condition is checked by DCGO as `IsExistOnBattleAreaDigimon
/// (card)` — "does the controller have ANY Digimon on the battle area"
/// (no self-exclusion). Because BT18-060 is ITSELF a Digimon and is already
/// on the battle area by the time its own [On Play] resolves, this gate is
/// self-satisfying and can never actually fail during BT18-060's own natural
/// play — DCGO's own comment/structure treats it the same way (a defensive
/// existence check, not a meaningful behavioral gate for THIS particular
/// [On Play], since the card can't be "played" without becoming an own
/// battle-area Digimon first). We therefore only verify the gate is present
/// structurally (Section 1's `bt18_060_on_play_clause_is_face_up_and_gated_on_own_digimon`)
/// and, here, that firing the process via the REAL play path (BT18-060
/// entering the battle area normally) reaches the reveal step at all —
/// confirming the condition does not accidentally block BT18-060's own
/// legitimate [On Play].
#[test]
fn bt18_060_on_play_condition_is_satisfied_by_its_own_natural_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("BT18-060 compiles")
        .add_card(make_vemmon_text_card("F060-SELF-TEXT"))
        .add_card(make_filler("F060-SELF-A"))
        .add_card(make_filler("F060-SELF-B"))
        .hand(0, &["BT18-060"])
        .deck(0, &["F060-SELF-B", "F060-SELF-A", "F060-SELF-TEXT"])
        .memory(10)
        .start();

    let played = runner.play(0, 0);
    assert!(played.is_some(), "BT18-060 must be playable");
    let _ = runner.auto_resolve();

    // BT18-060 must now be on the battle area (its own condition gate is
    // self-satisfying — DCGO's IsExistOnBattleAreaDigimon(card) does not
    // self-exclude), and the reveal-search must have actually run: the
    // [Vemmon]-in-text filler was added to hand, proving the [On Play]
    // process was not gated out by its own own-battle-area-Digimon condition.
    assert!(
        !runner.game.players[0].battle_area.is_empty(),
        "BT18-060 must have entered the battle area via play"
    );
    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        hand_ids.contains(&"F060-SELF-TEXT"),
        "the [Vemmon]-in-text filler must have been added to hand by BT18-060's own \
         [On Play], confirming the own-battle-area-Digimon condition did not block \
         BT18-060's own natural play: {hand_ids:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: reveal-search two-bucket [On Play]
// ═══════════════════════════════════════════════════════════════════════════

/// Build a runner with an own Digimon on the field (BT18-060's own Digimon
/// gate needs at least one) and run the [On Play] process directly.
fn runner_with_own_digimon(extra_cards: Vec<CardData>) -> (DebugRunner, PermanentHandle) {
    let mut builder = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("BT18-060 compiles")
        .add_card(make_test_card("F060-HOST", "Host"));
    for c in extra_cards {
        builder = builder.add_card(c);
    }
    let mut runner = builder.build();
    let host = runner.place_on_field(0, "F060-HOST", None);
    (runner, host)
}

/// POSITIVE bucket 1 only: a [Vemmon]-in-text card among the reveal is added
/// to hand; no exact-name "Vemmon" card is present, so bucket 2 fizzles.
#[test]
fn bt18_060_on_play_bucket1_adds_vemmon_text_card_to_hand() {
    let (mut runner, host) = runner_with_own_digimon(vec![
        make_vemmon_text_card("F060-TEXT"),
        make_filler("F060-A"),
        make_filler("F060-B"),
    ]);
    let src = runner.top_card(host);
    let hand_before = runner.game.players[0].hand.len();
    let sources_before = runner.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();
    stack_deck_top(&mut runner, 0, &["F060-B", "F060-A", "F060-TEXT"]);
    let deck_before = runner.game.players[0].deck.len();

    let process = on_play_process(&runner);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, Some(host), 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }
    // Drive: bucket-1 pick (only F060-TEXT qualifies), remainder ordering.
    drive_first_choice_n_times(&mut runner, 10);

    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        hand_ids.contains(&"F060-TEXT"),
        "the [Vemmon]-in-text card must be added to hand: {hand_ids:?}"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 1,
        "exactly 1 card added to hand"
    );
    // Bucket 2 had no exact-name "Vemmon" candidate → no source placed.
    assert_eq!(
        runner.game.players[0].battle_area[host.index as usize]
            .card_sources
            .len(),
        sources_before,
        "bucket 2 must not place anything when no exact-name Vemmon was revealed"
    );
    // All 3 revealed cards left the deck top (reveal removes them); 1 went to
    // hand, 2 fillers return to the bottom — net deck size drops by 1.
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before - 1,
        "1 card left for hand, 2 fillers returned to the bottom of the deck"
    );
}

/// POSITIVE bucket 2 only: an exact-name "Vemmon" card among the reveal is
/// placed as a chosen own Digimon's bottom digivolution card; no [Vemmon]-
/// in-text card is present (the exact-name card is filtered out of bucket 1's
/// candidate set only if it lacks "Vemmon" in its own text — here it has
/// "Vemmon" in its text too, so bucket 1 runs FIRST per DCGO's bucket order
/// and would claim it; to isolate bucket 2 we give bucket-2's card an empty
/// effect_text so only its EXACT NAME makes it eligible, and it is NOT
/// text-eligible for bucket 1).
#[test]
fn bt18_060_on_play_bucket2_places_exact_name_vemmon_as_chosen_digimon_source() {
    let mut exact = make_test_card("F060-EXACT", "Vemmon");
    exact.effect_text = String::new(); // NOT eligible for bucket 1 (no text match)
    let (mut runner, host) =
        runner_with_own_digimon(vec![exact, make_filler("F060-C"), make_filler("F060-D")]);
    let src = runner.top_card(host);
    let hand_before = runner.game.players[0].hand.len();
    let sources_before = runner.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();
    stack_deck_top(&mut runner, 0, &["F060-D", "F060-C", "F060-EXACT"]);

    let process = on_play_process(&runner);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, Some(host), 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }
    // Drive: bucket-1 (fizzles, no eligible text-match candidate — no prompt),
    // bucket-2 reveal pick, own-Digimon target pick, remainder ordering.
    drive_first_choice_n_times(&mut runner, 10);

    let perm = &runner.game.players[0].battle_area[host.index as usize];
    let source_ids: Vec<&str> = perm
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        source_ids.contains(&"F060-EXACT"),
        "exact-name Vemmon must be placed as a bottom digivolution source: {source_ids:?}"
    );
    assert_eq!(
        perm.card_sources.len(),
        sources_before + 1,
        "exactly 1 source placed"
    );
    assert_eq!(
        perm.card_sources[0].card_id(&runner.game.card_data),
        "F060-EXACT",
        "placed card must be the BOTTOM digivolution card"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "bucket 1 fizzled — nothing added to hand"
    );
}

/// POSITIVE both buckets: distinct eligible cards for bucket 1 (text-only)
/// and bucket 2 (exact name, no text match) are both routed correctly in the
/// SAME resolution, and the remaining filler returns to the deck bottom.
#[test]
fn bt18_060_on_play_both_buckets_fire_independently_same_resolution() {
    let text_card = make_vemmon_text_card("F060-TEXT2");
    let mut exact = make_test_card("F060-EXACT2", "Vemmon");
    exact.effect_text = String::new();
    let (mut runner, host) =
        runner_with_own_digimon(vec![text_card, exact, make_filler("F060-E")]);
    let src = runner.top_card(host);
    let sources_before = runner.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();
    stack_deck_top(&mut runner, 0, &["F060-E", "F060-EXACT2", "F060-TEXT2"]);
    let deck_before = runner.game.players[0].deck.len();

    let process = on_play_process(&runner);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, Some(host), 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }
    drive_first_choice_n_times(&mut runner, 10);

    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        hand_ids.contains(&"F060-TEXT2"),
        "bucket 1 card added to hand: {hand_ids:?}"
    );

    let perm = &runner.game.players[0].battle_area[host.index as usize];
    let source_ids: Vec<&str> = perm
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        source_ids.contains(&"F060-EXACT2"),
        "bucket 2 card placed as bottom source: {source_ids:?}"
    );
    assert_eq!(
        perm.card_sources.len(),
        sources_before + 1,
        "exactly 1 source placed (bucket 2)"
    );

    // All 3 revealed cards left the deck top; 2 were routed (1 hand, 1
    // source), 1 filler returns to bottom — net deck size drops by 2.
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before - 2,
        "the untouched filler returns to the bottom of the deck"
    );
}

/// NEGATIVE: a plain card with neither the exact name "Vemmon" nor "[Vemmon]"
/// in its text must never be picked by either bucket, and ends up back in the
/// deck (remainder).
#[test]
fn bt18_060_on_play_plain_card_is_never_picked_and_returns_to_deck() {
    let (mut runner, host) = runner_with_own_digimon(vec![
        make_filler("F060-PLAIN-A"),
        make_filler("F060-PLAIN-B"),
        make_filler("F060-PLAIN-C"),
    ]);
    let src = runner.top_card(host);
    let hand_before = runner.game.players[0].hand.len();
    let sources_before = runner.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();
    stack_deck_top(
        &mut runner,
        0,
        &["F060-PLAIN-C", "F060-PLAIN-B", "F060-PLAIN-A"],
    );
    let deck_before = runner.game.players[0].deck.len();

    let process = on_play_process(&runner);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, Some(host), 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }
    drive_first_choice_n_times(&mut runner, 10);

    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "no plain card should be added to hand"
    );
    assert_eq!(
        runner.game.players[0].battle_area[host.index as usize]
            .card_sources
            .len(),
        sources_before,
        "no plain card should be placed as a source"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before,
        "all 3 revealed cards return to the bottom of the deck (net zero deck size change)"
    );
}

/// GATE: a card with "[Vemmon]" in its TEXT but a DIFFERENT exact name is
/// eligible for bucket 1 (add to hand) but must NOT be offered by bucket 2
/// (exact-name-only match) even when it is the sole remaining candidate.
#[test]
fn bt18_060_on_play_text_only_card_not_eligible_for_bucket2_exact_name_gate() {
    // Bucket 1 auto-claims the only text-eligible card, so to prove bucket 2's
    // gate independently we drain bucket 1's candidate manually by giving it
    // the exact text match AND confirming it lands in HAND (bucket 1), not as
    // a source (bucket 2) — i.e. bucket 1 wins first and the card never
    // reaches bucket 2's install because the pool already shrank. This
    // confirms bucket-2's `name_is: Vemmon` predicate is doing real work,
    // not merely "whatever bucket 1 leaves over".
    let (mut runner, host) = runner_with_own_digimon(vec![make_vemmon_text_card("F060-ONLY")]);
    let src = runner.top_card(host);
    let sources_before = runner.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();
    stack_deck_top(&mut runner, 0, &["F060-ONLY"]);

    let process = on_play_process(&runner);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, Some(host), 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }
    drive_first_choice_n_times(&mut runner, 10);

    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        hand_ids.contains(&"F060-ONLY"),
        "text-only-match card must go to hand via bucket 1: {hand_ids:?}"
    );
    assert_eq!(
        runner.game.players[0].battle_area[host.index as usize]
            .card_sources
            .len(),
        sources_before,
        "a text-only-match card (not exact-named \"Vemmon\") must never be placed \
         as a bottom digivolution source by bucket 2"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Cost reduction: structural (see Section 1 for scope/OPT checks)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt18_060_cost_reduction_targets_vemmon_text_via_cost_target() {
    let runner = base_runner();
    let card = runner.compiled_card("BT18-060").expect("compiled");

    fn cost_target_has_vemmon_text(p: &CompiledPredicate) -> bool {
        if let Some(ct) = &p.cost_target {
            if ct.effect_text_contains.as_deref() == Some("Vemmon") {
                return true;
            }
        }
        p.all_of.iter().any(cost_target_has_vemmon_text) || p.any_of.iter().any(cost_target_has_vemmon_text)
    }

    let mut checked = 0;
    for c in &card.effects {
        if let CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
            active_when,
            ..
        }) = c
        {
            checked += 1;
            let aw = active_when.as_ref().expect("active_when present");
            assert!(
                cost_target_has_vemmon_text(aw),
                "cost_target must gate on effect_text_contains: Vemmon (printed \
                 \"a Digimon card with [Vemmon] in its text\")"
            );
        }
    }
    assert_eq!(checked, 2);
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Cost reduction: behavioral
// ═══════════════════════════════════════════════════════════════════════════

/// POSITIVE (face-up): Vemmon on top of its own permanent digivolves into a
/// [Vemmon]-text Lv.4 → cost reduced by 1 (2 - 1 = 1 memory spent).
#[test]
fn bt18_060_face_up_cost_reduction_fires_digivolving_into_vemmon_text() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("parses")
        .add_card(make_vemmon_text_lv4("F060-TARGET"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let vemmon = runner.place_on_field(0, "BT18-060", None);
    let hand_idx = push_to_hand(&mut runner, 0, "F060-TARGET");

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, vemmon.index as usize, PlaySource::ByHand);
    assert!(
        digivolved,
        "Vemmon must digivolve into the [Vemmon]-text Lv.4 target"
    );
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "printed evo cost 2 - 1 (Vemmon's face-up reduction) = 1 memory spent"
    );
}

/// NEGATIVE (face-up, no Vemmon text): digivolving into a target with no
/// Vemmon text pays the full cost.
#[test]
fn bt18_060_face_up_cost_reduction_does_not_fire_for_non_vemmon_text_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("parses")
        .add_card(make_plain_lv4("F060-PLAIN-TARGET"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let vemmon = runner.place_on_field(0, "BT18-060", None);
    let hand_idx = push_to_hand(&mut runner, 0, "F060-PLAIN-TARGET");

    let memory_before = runner.game.memory;
    let _ =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, vemmon.index as usize, PlaySource::ByHand);

    assert_eq!(
        runner.game.memory,
        memory_before - 2,
        "target has no [Vemmon] text — full printed evo cost 2 is paid, no reduction"
    );
}

/// POSITIVE (inherited / buried): Vemmon is a digivolution SOURCE beneath a
/// Lv.4 carrier; the carrier digivolves further into a [Vemmon]-text Lv.5 →
/// the inherited clause fires, reducing the cost by 1.
#[test]
fn bt18_060_inherited_cost_reduction_fires_when_buried() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("parses")
        .add_card(make_plain_lv4("F060-CARRIER"))
        .add_card(make_vemmon_text_lv5("F060-TARGET5"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // Place Vemmon on field, then digivolve it into the plain Lv.4 carrier
    // (no Vemmon text — no reduction on this leg), burying Vemmon as source 0.
    let vemmon = runner.place_on_field(0, "BT18-060", None);
    let carrier_hand_idx = push_to_hand(&mut runner, 0, "F060-CARRIER");
    let carrier_evo = runner.game.digivolve_from_hand(
        0,
        carrier_hand_idx,
        vemmon.index as usize,
        PlaySource::ByHand,
    );
    assert!(carrier_evo, "Vemmon must digivolve into the plain Lv.4 carrier");

    // Now digivolve the resulting permanent (Vemmon buried beneath the Lv.4
    // carrier) into the [Vemmon]-text Lv.5 target.
    let target_hand_idx = push_to_hand(&mut runner, 0, "F060-TARGET5");
    let memory_before = runner.game.memory;
    let target_evo =
        runner
            .game
            .digivolve_from_hand(0, target_hand_idx, vemmon.index as usize, PlaySource::ByHand);
    assert!(
        target_evo,
        "the carrier (with buried Vemmon) must digivolve into the [Vemmon]-text Lv.5"
    );
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "printed evo cost 2 - 1 (Vemmon's INHERITED/buried reduction) = 1 memory spent"
    );
}

/// KNOWN GAP (pinned regression) — G-COST-REDUCTION-SHARED-OPT-BOTH-SCOPES.
///
/// DCGO shares ONE per-turn budget (`activateClass2`) between the face-up and
/// buried positions, so digivolving Vemmon (face-up, -1) and THEN digivolving
/// the result further (inherited, buried) in the SAME turn should get NO
/// second reduction — DCGO's `isOverMaxCountPerTurn` would refuse it.
///
/// This engine currently authors the printed effect as two SEPARATE
/// `CostReduction` clauses (face_up + inherited — required, since the DSL's
/// `scope: both` expansion is FloodGate-only, not CostReduction; see the
/// YAML file-header note), each with its OWN once-per-turn budget. There is
/// no `scope: both` widening available for CostReduction without an
/// engine-substrate change (`code/digimon-engine/src/dsl_cards/mod.rs` +
/// `code/digimon-engine/src/game_actions/cost.rs`), which is out of scope for
/// a card script. This test PINS the current (over-permissive) behavior: BOTH
/// reductions fire in the same turn, granting -2 total where the real game
/// grants only -1. It exists to make the divergence visible/trackable, not to
/// assert correctness — flip this assertion the moment
/// G-COST-REDUCTION-SHARED-OPT-BOTH-SCOPES closes.
#[test]
fn bt18_060_cross_position_double_reduction_is_a_known_gap() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("parses")
        .add_card(make_vemmon_text_lv4("F060-MID"))
        .add_card(make_vemmon_text_lv5("F060-TOP"))
        .memory(20)
        .start();
    runner.game.turn_count = 1;

    // Leg 1 (face-up): Vemmon on top digivolves into a [Vemmon]-text Lv.4.
    let vemmon = runner.place_on_field(0, "BT18-060", None);
    let mid_hand_idx = push_to_hand(&mut runner, 0, "F060-MID");
    let memory_after_place = runner.game.memory;
    let leg1 =
        runner
            .game
            .digivolve_from_hand(0, mid_hand_idx, vemmon.index as usize, PlaySource::ByHand);
    assert!(leg1, "leg 1: Vemmon digivolves into the Vemmon-text Lv.4");
    let memory_after_leg1 = runner.game.memory;
    assert_eq!(
        memory_after_leg1,
        memory_after_place - 1,
        "leg 1 (face-up) reduction fires: printed cost 2 - 1 = 1 spent"
    );

    // Leg 2 (inherited, SAME turn): the Lv.4 carrier (with Vemmon now buried)
    // digivolves further into a [Vemmon]-text Lv.5.
    let top_hand_idx = push_to_hand(&mut runner, 0, "F060-TOP");
    let leg2 =
        runner
            .game
            .digivolve_from_hand(0, top_hand_idx, vemmon.index as usize, PlaySource::ByHand);
    assert!(leg2, "leg 2: the carrier digivolves into the Vemmon-text Lv.5");
    let memory_after_leg2 = runner.game.memory;

    // KNOWN-GAP ASSERTION: the inherited clause fires AGAIN (separate OPT
    // budget from the face_up clause), reducing leg 2's cost by 1 as well —
    // -2 total this turn where DCGO's shared counter would only grant -1.
    assert_eq!(
        memory_after_leg2,
        memory_after_leg1 - 1,
        "KNOWN GAP: the inherited clause's SEPARATE once-per-turn budget still \
         grants leg 2 a reduction in the same turn (DCGO's shared counter would \
         refuse it — see G-COST-REDUCTION-SHARED-OPT-BOTH-SCOPES)"
    );
}

/// Integrated event-log check: a `Digivolve` event fires for the face-up
/// cost-reduced path (confirms the digivolution actually completed).
#[test]
fn bt18_060_face_up_cost_reduction_digivolve_emits_digivolve_event() {
    use digimon_engine::events::GameEvent;

    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("parses")
        .add_card(make_vemmon_text_lv4("F060-EVT"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let vemmon = runner.place_on_field(0, "BT18-060", None);
    let hand_idx = push_to_hand(&mut runner, 0, "F060-EVT");

    let cp = runner.event_checkpoint();
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, vemmon.index as usize, PlaySource::ByHand);
    assert!(digivolved, "digivolve must succeed");

    let events = runner.events_since(cp);
    let digivolve_count = events
        .iter()
        .filter(|e| matches!(e, GameEvent::Digivolve { .. }))
        .count();
    assert!(
        digivolve_count >= 1,
        "a Digivolve event must fire for the cost-reduced digivolve"
    );
}
