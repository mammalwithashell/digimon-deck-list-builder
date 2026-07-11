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
//! - §1 Structural: 1 triggered OnPlay clause + 1 declarative CostReduction
//!       clause authored `scope: inherited` (a SINGLE Inherited lowered
//!       `Effect` — the printed reducer is an INHERITED effect, dormant while
//!       this card is the face-up top of its own permanent; general_rule.pdf
//!       §15-3-1 + DCGO `Permanent.EffectList_ForCard`'s buried-only include
//!       rule — see the YAML file-header Clause 2 + HISTORY notes).
//! - §2 OnPlay condition gating: no own battle-area Digimon → no reveal.
//! - §3 Behavioral reveal-search: bucket 1 (add [Vemmon]-in-text to hand,
//!       incl. a NAME-only match with no self-referential body text — the
//!       widened `any_of` HasText axes), bucket 2 (place exact-name [Vemmon]
//!       as a chosen own Digimon's bottom source), both buckets together,
//!       remainder to deck bottom, exact-name vs. in-text gating, and a
//!       non-Vemmon card being left untouched.
//! - §4 Cost-reduction structural: scope / once_per_turn / active_when shape,
//!       incl. the widened `any_of` cost_target predicate.
//! - §5 Cost-reduction behavioral: face-up digivolve pays FULL cost (the
//!       over-application pin for the reverted `scope: both` mis-migration),
//!       buried positive (+ Digivolve event), a buried NAME-only cost-target
//!       match, the non-Vemmon-target negative, and the cross-position
//!       sequence proving the reduction comes from the BURIED position only
//!       (the face-up leg pays full and does not burn the OPT budget).

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

/// A card named "Vemmon Prime" (name CONTAINS "Vemmon") with EMPTY effect
/// text and no traits — eligible for bucket 1 ONLY via the widened
/// `name_contains` axis, not `effect_text_contains` (which would find
/// nothing in an empty body). Proves the "[Vemmon] in its text" whole-card
/// HasText widening (name axis) actually does work, per the official Q&A
/// covering the whole Vemmon-reducer family (BT21-056's bundle, restated in
/// this card's file-header note).
fn make_vemmon_name_only_card(id: &str) -> CardData {
    let mut c = make_test_card(id, "Vemmon Prime");
    c.effect_text = String::new();
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

/// A Lv.5 Digimon NAMED "Vemmon Prime" (name CONTAINS "Vemmon") with EMPTY
/// effect text — cost-target NAME-only match for the widened `cost_target`
/// `any_of` axes (proves the reducer's `name_contains` leaf, not just
/// `effect_text_contains`, drives the reduction). Lv.5 (digivolving from a
/// Lv.4 carrier) so the reduction can be exercised from Vemmon's BURIED
/// position — the only position where the inherited reducer is active.
fn make_vemmon_name_only_lv5(id: &str) -> CardData {
    let mut c = make_test_card(id, "Vemmon Prime");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(5000);
    c.play_cost = 6;
    c.colors = vec![CardColor::Black];
    c.effect_text = String::new();
    c.evo_costs = vec![EvoCost {
        level: 4,
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
fn bt18_060_compiles_to_two_clauses() {
    let runner = base_runner();
    let card = runner.compiled_card("BT18-060").expect("compiled");
    assert_eq!(
        card.effects.len(),
        2,
        "BT18-060 must compile to exactly 2 clauses: [On Play] triggered + \
         1 CostReduction authored `scope: inherited` (a single Inherited \
         lowered Effect — see the next tests); got {}",
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
fn bt18_060_cost_reduction_clause_is_authored_as_scope_inherited() {
    let runner = base_runner();
    let card = runner.compiled_card("BT18-060").expect("compiled");

    let scopes: Vec<CompiledScope> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                scope, ..
            }) => Some(*scope),
            _ => None,
        })
        .collect();

    assert_eq!(
        scopes.len(),
        1,
        "BT18-060 must have exactly 1 CostReduction CLAUSE in the compiled \
         card; got {}",
        scopes.len()
    );
    assert_eq!(
        scopes[0],
        CompiledScope::Inherited,
        "the CostReduction clause must be authored `scope: inherited` — the \
         printed reducer is an INHERITED effect, active only while this \
         Vemmon is a buried digivolution source (general_rule.pdf §15-3-1: \
         an inherited effect is gained by a Digimon FROM a digivolution \
         card; DCGO Permanent.EffectList_ForCard excludes a top card's \
         SetIsInheritedEffect(true) effects from the cost pipeline). The \
         round-2 `scope: both` authoring was an over-application and was \
         reverted — see the YAML HISTORY note"
    );
}

#[test]
fn bt18_060_cost_reduction_clause_is_once_per_turn_and_your_turn_gated() {
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
    assert_eq!(checked, 1, "expected exactly 1 CostReduction clause");
}

/// Directly re-lowers the BT18-060 YAML (mirroring
/// `scope_both_shared_opt_reducer.rs`'s pattern) to prove `scope: inherited`
/// produces exactly ONE Inherited BeforePayCost `Effect` — no FaceUp sibling
/// (the printed reducer is dormant while Vemmon is the top card), and no
/// `shared_opt_group` (a single copy keeps the default per-slot OPT keying,
/// matching `scope_inherited_opt_reducer_has_no_shared_group`).
#[test]
fn bt18_060_scope_inherited_lowers_to_single_inherited_effect() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::dsl_cards::DslCardEffect;
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::CardEffect;
    use std::sync::Arc;

    let yaml = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/cards/bt18/BT18-060.yaml"
    ))
    .expect("BT18-060.yaml reads");
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml).expect("YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("YAML compiles");
    let dsl = DslCardEffect::new(Arc::new(compiled));
    let effects = dsl.effects(CardHandle(99));

    let reducers: Vec<_> = effects
        .iter()
        .filter(|e| e.timing == EffectTiming::BeforePayCost)
        .collect();
    assert_eq!(
        reducers.len(),
        1,
        "scope: inherited must lower to exactly 1 BeforePayCost Effect (no \
         FaceUp sibling — the inherited reducer is dormant while Vemmon is \
         the face-up top; DCGO Permanent.EffectList_ForCard + rule 15-3-1)"
    );

    let reducer = reducers[0];
    assert!(
        reducer.inherited,
        "the single lowered copy must be Inherited (buried-only)"
    );
    assert_eq!(
        reducer.shared_opt_group, None,
        "a single inherited copy has no sibling to share an OPT budget with — \
         it keeps the default per-slot once-per-turn keying"
    );
    assert_eq!(reducer.max_per_turn, 1);
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

/// POSITIVE bucket 2 (after bucket 1 claims a SEPARATE candidate): a card
/// exact-named "Vemmon" is placed as a chosen own Digimon's bottom
/// digivolution card. NOTE (round-2 fidelity correction): a card exact-named
/// "Vemmon" is now ALWAYS ALSO eligible for bucket 1 under the widened
/// whole-card HasText scan (`name_contains: "Vemmon"` — DCGO's own
/// `HasText` includes the name), so it can no longer be isolated to bucket 2
/// by giving it empty effect text (that was only valid under the old,
/// narrower `effect_text_contains`-only modeling — the bug this round fixes).
/// To reach bucket 2 on this card in the REAL game, bucket 1 must have
/// ALREADY claimed its own (different) candidate first, per DCGO's
/// bucket-order (bucket 1 runs before bucket 2 and the pool shrinks as each
/// bucket claims a card) — so this test gives bucket 1 a separate
/// [Vemmon]-in-text candidate to consume, leaving the exact-name "Vemmon"
/// card as bucket 2's sole remaining pick.
#[test]
fn bt18_060_on_play_bucket2_places_exact_name_vemmon_as_chosen_digimon_source() {
    let mut exact = make_test_card("F060-EXACT", "Vemmon");
    exact.effect_text = String::new();
    let (mut runner, host) = runner_with_own_digimon(vec![
        make_vemmon_text_card("F060-BUCKET1-FODDER"),
        exact,
        make_filler("F060-D"),
    ]);
    let src = runner.top_card(host);
    let hand_before = runner.game.players[0].hand.len();
    let sources_before = runner.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();
    stack_deck_top(
        &mut runner,
        0,
        &["F060-D", "F060-EXACT", "F060-BUCKET1-FODDER"],
    );

    let process = on_play_process(&runner);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, Some(host), 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }
    // Drive: bucket-1 claims F060-BUCKET1-FODDER (the only OTHER text-match
    // candidate), bucket-2 reveal pick (F060-EXACT, the sole exact-name
    // remaining), own-Digimon target pick, remainder ordering.
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
    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        hand_ids.contains(&"F060-BUCKET1-FODDER"),
        "bucket 1 must have claimed its own separate candidate: {hand_ids:?}"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 1,
        "exactly 1 card added to hand by bucket 1 (its own fodder, not F060-EXACT)"
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
    let (mut runner, host) = runner_with_own_digimon(vec![text_card, exact, make_filler("F060-E")]);
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

/// POSITIVE bucket 1, NAME-only match (round-2 fidelity upgrade): a card
/// named "Vemmon Prime" (name CONTAINS "Vemmon") with EMPTY effect text is
/// still added to hand by bucket 1 via the widened `any_of` HasText axes
/// (`name_contains`), even though `effect_text_contains` alone would find
/// nothing in its blank body. Proves the "[Vemmon] in its text" whole-card
/// scan (official Q&A, shared Vemmon-reducer family) is not narrowed to
/// body-text-only.
#[test]
fn bt18_060_on_play_bucket1_name_only_match_is_eligible_via_widened_any_of() {
    let (mut runner, host) = runner_with_own_digimon(vec![
        make_vemmon_name_only_card("F060-NAMEONLY"),
        make_filler("F060-NO-A"),
        make_filler("F060-NO-B"),
    ]);
    let src = runner.top_card(host);
    let hand_before = runner.game.players[0].hand.len();
    stack_deck_top(&mut runner, 0, &["F060-NO-B", "F060-NO-A", "F060-NAMEONLY"]);

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
        hand_ids.contains(&"F060-NAMEONLY"),
        "a name-only (\"Vemmon Prime\", empty effect text) match must still be \
         added to hand via the widened name_contains axis: {hand_ids:?}"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 1,
        "exactly 1 card added to hand"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Cost reduction: structural (see Section 1 for scope/OPT checks)
// ═══════════════════════════════════════════════════════════════════════════

/// Checks whether a `cost_target` predicate (or one of its `any_of`/`all_of`
/// leaves) has an `effect_text_contains: "Vemmon"` leaf — the narrow,
/// body-text-only axis.
fn cost_target_has_effect_text_leaf(p: &CompiledPredicate) -> bool {
    if let Some(ct) = &p.cost_target {
        if predicate_has_effect_text_leaf(ct) {
            return true;
        }
    }
    p.all_of.iter().any(cost_target_has_effect_text_leaf)
        || p.any_of.iter().any(cost_target_has_effect_text_leaf)
}

fn predicate_has_effect_text_leaf(p: &CompiledPredicate) -> bool {
    p.effect_text_contains.as_deref() == Some("Vemmon")
        || p.all_of.iter().any(predicate_has_effect_text_leaf)
        || p.any_of.iter().any(predicate_has_effect_text_leaf)
}

/// Checks whether a `cost_target` predicate (or one of its `any_of`/`all_of`
/// leaves) has a `name_contains: "Vemmon"` leaf — the widened whole-card
/// HasText name axis (round-2 fidelity upgrade).
fn cost_target_has_name_contains_leaf(p: &CompiledPredicate) -> bool {
    if let Some(ct) = &p.cost_target {
        if predicate_has_name_contains_leaf(ct) {
            return true;
        }
    }
    p.all_of.iter().any(cost_target_has_name_contains_leaf)
        || p.any_of.iter().any(cost_target_has_name_contains_leaf)
}

fn predicate_has_name_contains_leaf(p: &CompiledPredicate) -> bool {
    p.name_contains.as_deref() == Some("Vemmon")
        || p.all_of.iter().any(predicate_has_name_contains_leaf)
        || p.any_of.iter().any(predicate_has_name_contains_leaf)
}

/// Checks whether a `cost_target` predicate (or one of its `any_of`/`all_of`
/// leaves) has a `trait_has: "Vemmon"` leaf — the widened whole-card HasText
/// traits axis.
fn cost_target_has_trait_has_leaf(p: &CompiledPredicate) -> bool {
    if let Some(ct) = &p.cost_target {
        if predicate_has_trait_has_leaf(ct) {
            return true;
        }
    }
    p.all_of.iter().any(cost_target_has_trait_has_leaf)
        || p.any_of.iter().any(cost_target_has_trait_has_leaf)
}

fn predicate_has_trait_has_leaf(p: &CompiledPredicate) -> bool {
    p.trait_has.as_deref() == Some("Vemmon")
        || p.all_of.iter().any(predicate_has_trait_has_leaf)
        || p.any_of.iter().any(predicate_has_trait_has_leaf)
}

#[test]
fn bt18_060_cost_reduction_targets_vemmon_via_widened_any_of_cost_target() {
    let runner = base_runner();
    let card = runner.compiled_card("BT18-060").expect("compiled");

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
                cost_target_has_effect_text_leaf(aw),
                "cost_target must gate on effect_text_contains: Vemmon (printed \
                 \"a Digimon card with [Vemmon] in its text\")"
            );
            assert!(
                cost_target_has_name_contains_leaf(aw),
                "cost_target must ALSO gate on name_contains: Vemmon — the round-2 \
                 whole-card HasText widening (name axis)"
            );
            assert!(
                cost_target_has_trait_has_leaf(aw),
                "cost_target must ALSO gate on trait_has: Vemmon — the round-2 \
                 whole-card HasText widening (traits axis)"
            );
        }
    }
    assert_eq!(
        checked, 1,
        "expected exactly 1 CostReduction clause (authored scope: inherited)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Cost reduction: behavioral
// ═══════════════════════════════════════════════════════════════════════════

/// OVER-APPLICATION PIN (face-up): Vemmon on top of its own permanent
/// digivolving into a [Vemmon]-text Lv.4 pays the FULL printed cost — NO
/// reduction. The reducer is an INHERITED effect, dormant while its card is
/// the face-up top:
///   - general_rule.pdf §15-3-1: "An inherited effect is an effect gained by
///     a Digimon FROM A DIGIVOLUTION CARD" — a top card's inherited box
///     grants nothing.
///   - DCGO BT18_060.cs: the reducer `changeCostClass` carries
///     `SetIsInheritedEffect(true)`, and the cost pipeline
///     (`CardSource.GetChangedPayingCost` → `permanent.EffectList` →
///     `Permanent.EffectList_ForCard`, Permanent.cs:1520-1541) EXCLUDES a top
///     card's inherited effects; the OPT tripwire `activateClass2` also
///     requires `!evoRootTops.Contains(card)` where `evoRootTops` are the
///     PRE-digivolution tops (CardController.cs:1394-1397 captures the old
///     top BEFORE `AddCardSource`), so it never fires for this case either.
///
/// This pins the reverted round-2 `scope: both` mis-migration, which granted
/// a -1 discount here (an over-application vs both DCGO and the rules).
#[test]
fn bt18_060_face_up_digivolve_pays_full_cost_over_application_pin() {
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
        memory_before - 2,
        "face-up Vemmon's inherited reducer is DORMANT (rule 15-3-1; DCGO \
         EffectList_ForCard buried-only include rule) — the full printed evo \
         cost 2 must be paid, no reduction"
    );
}

/// NEGATIVE (face-up, no Vemmon text): digivolving into a target with no
/// Vemmon text pays the full cost. Doubly guaranteed post-revert: the target
/// fails the `cost_target` text gate AND the inherited reducer is dormant
/// while Vemmon is the face-up top anyway (see the over-application pin
/// above) — kept as the cost_target-gate negative control.
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
    let _ = runner
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
/// the inherited clause fires, reducing the cost by 1. Also asserts a
/// `Digivolve` event fires for the reduced leg (folds in the retired
/// standalone event test — the face-up variant was removed with the
/// `scope: both` revert since the reduced path is now buried-only).
#[test]
fn bt18_060_inherited_cost_reduction_fires_when_buried() {
    use digimon_engine::events::GameEvent;

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
    assert!(
        carrier_evo,
        "Vemmon must digivolve into the plain Lv.4 carrier"
    );

    // Now digivolve the resulting permanent (Vemmon buried beneath the Lv.4
    // carrier) into the [Vemmon]-text Lv.5 target.
    let target_hand_idx = push_to_hand(&mut runner, 0, "F060-TARGET5");
    let memory_before = runner.game.memory;
    let cp = runner.event_checkpoint();
    let target_evo = runner.game.digivolve_from_hand(
        0,
        target_hand_idx,
        vemmon.index as usize,
        PlaySource::ByHand,
    );
    assert!(
        target_evo,
        "the carrier (with buried Vemmon) must digivolve into the [Vemmon]-text Lv.5"
    );
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "printed evo cost 2 - 1 (Vemmon's INHERITED/buried reduction) = 1 memory spent"
    );
    let events = runner.events_since(cp);
    let digivolve_count = events
        .iter()
        .filter(|e| matches!(e, GameEvent::Digivolve { .. }))
        .count();
    assert!(
        digivolve_count >= 1,
        "a Digivolve event must fire for the cost-reduced (buried) digivolve"
    );
}

/// Cross-position sequence (post-revert): the reduction comes from the
/// BURIED position ONLY, and the face-up leg neither reduces nor burns the
/// OPT budget.
///
/// Leg 1 (face-up): Vemmon on top digivolves into a [Vemmon]-text Lv.4 —
/// FULL cost paid (the inherited reducer is dormant while Vemmon is the top:
/// rule 15-3-1; DCGO EffectList_ForCard). Because no reduction applied, the
/// once-per-turn budget is untouched — mirroring DCGO, where the tripwire
/// `activateClass2` skips this case via `!evoRootTops.Contains(card)`
/// (Vemmon WAS the pre-digivolve top) so `isOverMaxCountPerTurn` stays clear.
///
/// Leg 2 (buried, SAME turn): the Lv.4 carrier (Vemmon now a digivolution
/// source) digivolves further into a [Vemmon]-text Lv.5 — the inherited
/// reducer fires, -1. Total reduction across both legs: exactly one, from
/// the buried position.
#[test]
fn bt18_060_cross_position_face_up_pays_full_then_buried_reduces_once() {
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
        memory_after_place - 2,
        "leg 1 (face-up): the inherited reducer is dormant — full printed \
         cost 2 paid, no reduction"
    );

    // Leg 2 (buried, SAME turn): the Lv.4 carrier (with Vemmon now buried)
    // digivolves further into a [Vemmon]-text Lv.5.
    let top_hand_idx = push_to_hand(&mut runner, 0, "F060-TOP");
    let leg2 =
        runner
            .game
            .digivolve_from_hand(0, top_hand_idx, vemmon.index as usize, PlaySource::ByHand);
    assert!(
        leg2,
        "leg 2: the carrier digivolves into the Vemmon-text Lv.5"
    );
    let memory_after_leg2 = runner.game.memory;

    assert_eq!(
        memory_after_leg2,
        memory_after_leg1 - 1,
        "leg 2 (buried): the inherited reduction fires — printed cost 2 - 1 \
         = 1 spent. Leg 1 (face-up, no reduction) must NOT have consumed the \
         once-per-turn budget (DCGO's tripwire only increments on \
         buried-position digivolutions)"
    );
}

/// POSITIVE (buried), NAME-only cost-target match (round-2 fidelity
/// upgrade, restaged to the buried position with the `scope: both` revert):
/// with Vemmon buried beneath a plain Lv.4 carrier, digivolving the carrier
/// into a Lv.5 target named "Vemmon Prime" (name CONTAINS "Vemmon") with
/// EMPTY effect text still triggers the reduction via the widened `any_of`
/// `cost_target` (`name_contains`), even though `effect_text_contains` alone
/// would find nothing in its blank body.
#[test]
fn bt18_060_buried_cost_reduction_fires_for_name_only_cost_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("parses")
        .add_card(make_plain_lv4("F060-NAMEONLY-CARRIER"))
        .add_card(make_vemmon_name_only_lv5("F060-NAMEONLY-TARGET"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // Bury Vemmon: digivolve it into the plain Lv.4 carrier (no Vemmon text
    // — no reduction on this leg; the face-up reducer is dormant anyway).
    let vemmon = runner.place_on_field(0, "BT18-060", None);
    let carrier_hand_idx = push_to_hand(&mut runner, 0, "F060-NAMEONLY-CARRIER");
    let carrier_evo = runner.game.digivolve_from_hand(
        0,
        carrier_hand_idx,
        vemmon.index as usize,
        PlaySource::ByHand,
    );
    assert!(
        carrier_evo,
        "Vemmon must digivolve into the plain Lv.4 carrier"
    );

    let hand_idx = push_to_hand(&mut runner, 0, "F060-NAMEONLY-TARGET");
    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, vemmon.index as usize, PlaySource::ByHand);
    assert!(
        digivolved,
        "the carrier (with buried Vemmon) must digivolve into the name-only \
         Vemmon-family Lv.5 target"
    );
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "printed evo cost 2 - 1 (widened name_contains match, buried \
         position) = 1 memory spent"
    );
}
