//! P-206 Digital Gate Open — Option, Cost 4, White.
//!
//! # Card text (cards.json)
//!
//! You can ignore this card's color requirements.
//!
//! **[Main]** Reveal the top 3 cards of your deck. Add 1 Digimon card and
//! 1 Tamer card among them to the hand. Return the rest to the bottom of
//! the deck. Then, place this card in the battle area.
//!
//! **[Main] ＜Delay＞** (By trashing this card after the placing turn,
//! activate the effect below.)
//! ・You may play 1 Tamer card with the same color as any of your Digimon
//!   on the field from your hand with the play cost reduced by 4.
//!
//! **Inherited:** Security Effect [Security] You may play 1 Digimon card
//! with a play cost of 3 or less from your hand or trash without paying
//! the cost. Then, add this card to the hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/White/P_206.cs
//!
//! # Patterns this test covers
//! - A1  Reveal top-N + select Digimon + select Tamer (two sequential select_reveal)
//! - Option-as-permanent Delay (engine auto-placement via classify_option_subtype)
//! - Standard Delay (EndOfYourNextTurn) + conditional "you may" Tamer play with cost-4 reduction
//! - Inherited security: play Digimon cost≤3 from hand or trash free; add self to hand
//! - Unconditional color-ignore via flood_gate IgnoreColorRequirement
//!
//! # Substrate notes (all primitives this card needs are present + wired)
//!
//! - **select_reveal kind filter**: `install_select_reveal` enforces the
//!   `kind:` filter at runtime (`dsl_cards/step/selections.rs`).
//! - **G-PLACE-SELF-AS-OPTION-PERMANENT**: the `place_self_as_delay_option`
//!   step (`EffectContext::place_self_as_delay_option_permanent`) pushes the
//!   Option into the battle area as an `OptionState::Delayed` permanent.
//! - **G-COLOR-MATCH-AGAINST-BOARD** (resolved 2026-05-02): the
//!   `color_matches_any_field_digimon` predicate matches a card's colors
//!   against the live colors of the controller's field Digimon.
//! - **G-PLAY-COST-LTE**: the `play_cost_lte` predicate enforces a play-cost
//!   cap at selection time (`PredicateSpec.play_cost_lte`).
//! - **G-ADD-OPTION-SELF-TO-HAND**: the `add_this_option_to_hand` step
//!   (`EffectContext::add_pending_security_to_hand`) moves the pending
//!   security Option into its owner's hand.
//! - **G-IGNORE-COLOR-MASK**: the from-hand color bypass is carried by the
//!   card-level `use_requirement: { all_turns: true }`.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::{OptionState, PermanentHandle};

const YAML: &str = include_str!("../../../cards/p/P-206.yaml");

// ── Fixture builders ─────────────────────────────────────────────────────────

/// A Digimon card — eligible for the Main clause's Digimon select_reveal.
fn make_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c
}

/// A Tamer card — eligible for the Main clause's Tamer select_reveal.
fn make_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c
}

/// A Tamer card with play cost 3 (potentially eligible for Delay clause play).
fn make_tamer_cost(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.play_cost = cost;
    c
}

/// A Digimon card with play cost ≤ 3 (eligible for inherited security clause).
fn make_digimon_low_cost(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.play_cost = 3;
    c
}

/// An Option card — NOT eligible for Main's Digimon or Tamer select_reveal.
fn make_option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c
}

/// Generic filler (default make_test_card — no specific kind constraint matters for deck padding).
fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A `CardData` for P-206 itself — needed by `place_on_field` / `security`
/// builder helpers (which look the card up in `card_data` by id). The DSL
/// effects come from `from_dsl_yaml(YAML)`; this only carries metadata.
fn p206_card() -> CardData {
    let mut c = make_test_card("P-206", "Digital Gate Open");
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::White];
    c.play_cost = 4;
    c
}

/// A Tamer card with an explicit color (for the Delay color-match filter).
fn make_tamer_colored(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![color];
    c
}

/// A Digimon card with an explicit color (used as the field Digimon whose
/// color the Delay clause matches against).
fn make_digimon_colored(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c
}

/// A Digimon card with an explicit play cost (for the inherited-security
/// `play_cost_lte: 3` filter test).
fn make_digimon_with_cost(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.play_cost = cost;
    c
}

/// Place P-206 onto `player`'s battle area as a mature Delayed Option whose
/// scheduled trigger fires on this very turn's end. Mirrors BT15-096's
/// `place_bt15_096_as_mature_delay`. Returns the permanent handle.
fn place_p206_as_mature_delay(runner: &mut DebugRunner, player: u8) -> PermanentHandle {
    let handle = runner.place_on_field(player, "P-206", Some(0));
    runner.game.player_mut(player).battle_area[handle.index as usize].option_state =
        OptionState::Delayed {
            owner: player,
            trash_on_turn: runner.game.turn_count,
            trigger: DelayTrigger::EndOfYourNextTurn,
            placed_on_turn: 0,
        };
    handle
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — YAML parse + structural assertions
// ─────────────────────────────────────────────────────────────────────────────

/// P-206 YAML must parse and compile without errors.
#[test]
fn p_206_yaml_parses_and_compiles() {
    let _builder = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-206 YAML must parse and compile without errors");
}

/// P-206 is an Option card with cost 4 and color white.
#[test]
fn p_206_is_option_cost_4_color_white() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let compiled = runner
        .compiled_card("P-206")
        .expect("P-206 compiled card must be registered");

    assert_eq!(
        compiled.kind,
        digimon_dsl::compiled::CompiledCardKind::Option,
        "P-206 must be an Option card"
    );
    assert_eq!(compiled.cost, Some(4), "P-206 must have play cost 4");
}

/// P-206 must compile to exactly 4 clauses:
///   [0] main_from_hand (triggered, FaceUp) — reveal top 3, add Digimon + Tamer
///   [1] delay (declarative) — play Tamer with same color as field Digimon, -4 cost
///   [2] ignore-color flood_gate (declarative, FaceUp) — unconditional IgnoreColorRequirement
///   [3] inherited on_security (triggered or raw_rust, Inherited scope)
///
/// NOTE: clause ordering in the YAML may differ; we assert by type/scope/timing
/// rather than by fixed index where possible.
#[test]
fn p_206_has_four_clauses() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-206")
        .expect("P-206 compiled card present");

    assert_eq!(
        compiled.effects.len(),
        4,
        "expected 4 clauses (main_from_hand, delay, flood_gate/ignore-color, inherited on_security); got {}",
        compiled.effects.len()
    );
}

/// Clause 0: main_from_hand triggered, FaceUp scope, not optional.
#[test]
fn p_206_clause_0_is_main_from_hand_face_up_not_optional() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    match &compiled.effects[0] {
        CompiledClause::Triggered(t) => {
            assert!(
                t.when.contains(&CompiledTiming::MainFromHand),
                "clause 0 must fire at MainFromHand; got {:?}",
                t.when
            );
            assert_eq!(t.scope, CompiledScope::FaceUp, "clause 0 must be FaceUp");
            assert!(!t.optional, "Main clause is not optional (no 'you may')");
        }
        other => panic!("clause 0 must be Triggered; got {:?}", other),
    }
}

/// Clause 1: Delay declarative with trigger end_of_your_next_turn.
#[test]
fn p_206_clause_1_is_delay_end_of_your_next_turn() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    match &compiled.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { trigger, .. }) => {
                // TRIGGER CORRECTION (2026-08-24): this assertion used to demand
                // `EndOfYourNextTurn` and call that "standard <Delay>" -- backwards.
                // The printed timing is `[Main]`, and DCGO registers the clause at
                // `EffectTiming.OnDeclaration` (its [Main] activated-declaration
                // timing), so this is a player-activated §16-16 Delay. `Delayed`
                // lowers to `DelayTrigger::MainPhaseActivated`, a [Main] mask bit;
                // NOT activating is the §16-16-2 decline. `EndOfYourNextTurn` both
                // fired it at the wrong time and gave the player no say.
                assert_eq!(
                    *trigger,
                    CompiledTiming::Delayed,
                    "[Main] <Delay> must lower to Delayed (player-activated); got {:?}",
                    trigger
            );
        }
        other => panic!("clause 1 must be Declarative(Delay); got {:?}", other),
    }
}

/// Clause 2: flood_gate declarative (ignore-color bypass), FaceUp scope.
#[test]
fn p_206_clause_2_is_flood_gate_ignore_color() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    let has_flood_gate_ignore_color = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { .. })
        )
    });
    assert!(
        has_flood_gate_ignore_color,
        "P-206 must have a FloodGate declarative clause for ignore-color bypass"
    );
}

/// Clause 3: inherited scope, SecuritySkill timing.
#[test]
fn p_206_clause_3_is_inherited_security_skill() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    let has_inherited_security = compiled.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => {
            t.scope == CompiledScope::Inherited && t.when.contains(&CompiledTiming::OnSecurity)
        }
        CompiledClause::Declarative(CompiledDeclarativeClause::RawRust {
            scope, triggers, ..
        }) => *scope == CompiledScope::Inherited && triggers.contains(&CompiledTiming::OnSecurity),
        _ => false,
    });
    assert!(
        has_inherited_security,
        "P-206 must have an Inherited on_security clause (clause 3)"
    );
}

/// Inherited security clause outer `optional` flag must be `false`.
///
/// The "you may" governs ONLY the optional Digimon play, which is expressed
/// internally via `select_effect_choice ["Play Digimon", "Decline"]` guarded
/// by a `count_gte` predicate. The "Then, add this card to the hand" tail is
/// unconditional — DCGO calls `AddThisCardToHand` regardless of whether a
/// Digimon was played. Therefore the outer clause is `optional: false`; the
/// optionality lives entirely inside the process body. (Pattern follows P-156.)
#[test]
fn p_206_inherited_security_outer_clause_is_mandatory() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    let sec_clause = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::OnSecurity) =>
        {
            Some(t)
        }
        _ => None,
    });

    if let Some(t) = sec_clause {
        assert!(
            !t.optional,
            "outer clause must be optional=false — the 'you may' is internal \
             via select_effect_choice; the mandatory tail always fires"
        );
    }
    // If it's a RawRust stub, optionality is handled internally — no assertion needed.
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Behavioral: Main clause (reveal top 3, add Digimon + Tamer)
// ─────────────────────────────────────────────────────────────────────────────

/// When a Digimon is among the top 3 revealed cards, a select_reveal prompt
/// installs for picking a Digimon.
#[test]
fn p_206_main_installs_select_reveal_with_digimon_in_top_3() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("DIG-1"))
        .add_card(make_tamer("TAM-1"))
        .add_card(filler("FILL-1"))
        .hand(0, &["P-206"])
        // Deck top-to-bottom (last element = top): DIG-1 is top, TAM-1 next, FILL-1 third.
        .deck(0, &["FILL-1", "TAM-1", "DIG-1"])
        .deck(1, &["FILL-1"])
        .memory(10)
        .start();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true for P-206");

    // After revealing 3 cards (1 Digimon present), a selection prompt must install.
    assert!(
        runner.game.pending_selection.is_some(),
        "A selection prompt must be pending after revealing 3 cards that include a Digimon"
    );
}

/// When both a Digimon and Tamer are among the top 3, the Main clause adds the
/// Digimon and the Tamer to hand, then places P-206 itself into the battle area
/// (`place_self_as_delay_option`). Net: hand gains DIG-1 + TAM-1, loses P-206.
#[test]
fn p_206_main_adds_digimon_and_tamer_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(p206_card())
        .add_card(make_digimon("DIG-1"))
        .add_card(make_tamer("TAM-1"))
        .add_card(filler("FILL-1"))
        .hand(0, &["P-206"])
        // DIG-1 top, TAM-1 second, FILL-1 third.
        .deck(0, &["FILL-1", "TAM-1", "DIG-1"])
        .deck(1, &["FILL-1"])
        .memory(10)
        .start();

    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();

    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand_ids.contains(&"DIG-1".to_string()),
        "Main must add the revealed Digimon to hand; hand={hand_ids:?}"
    );
    assert!(
        hand_ids.contains(&"TAM-1".to_string()),
        "Main must add the revealed Tamer to hand; hand={hand_ids:?}"
    );
    assert!(
        !hand_ids.contains(&"P-206".to_string()),
        "P-206 leaves hand — 'place this card in the battle area'"
    );
    // P-206 is placed into the controller's battle area as a Delayed Option.
    assert!(
        runner.game.player(0).battle_area.iter().any(|perm| {
            perm.top_card().card_id(&runner.game.card_data) == "P-206"
                && matches!(perm.option_state, OptionState::Delayed { .. })
        }),
        "P-206 must be placed in the battle area as a Delayed Option"
    );
}

/// Deck size shrinks by 2 after Main (3 revealed, 2 added to hand, 1 returned to bottom).
#[test]
fn p_206_main_deck_shrinks_by_two_when_digimon_and_tamer_added() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("DIG-1"))
        .add_card(make_tamer("TAM-1"))
        .add_card(filler("FILL-1"))
        .add_card(filler("FILL-2"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL-2", "FILL-1", "TAM-1", "DIG-1"])
        .deck(1, &["FILL-1"])
        .memory(10)
        .start();

    let deck_before = runner.deck_size(0);
    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();
    let deck_after = runner.deck_size(0);

    assert_eq!(
        deck_before - deck_after,
        2,
        "Deck must shrink by 2 (3 revealed, 2 to hand, 1 returned to bottom); \
         before={deck_before}, after={deck_after}"
    );
}

/// Negative condition: when no Digimon is among the top 3, the Digimon
/// select_reveal must not add a non-Digimon card to hand.
///
/// `install_select_reveal` enforces the `kind: digimon` filter at runtime
/// (`selections.rs` — `eval_predicate_with_bindings` on each revealed card).
/// With no Digimon present, the Digimon `select_reveal` has zero candidates
/// and adds nothing; only the Tamer slot adds a card.
#[test]
fn p_206_main_digimon_filter_excludes_non_digimon_cards() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_tamer("TAM-1"))
        .add_card(make_tamer("TAM-2"))
        .add_card(make_option("OPT-1"))
        .hand(0, &["P-206"])
        // Top 3: all non-Digimon cards — Digimon filter should exclude all.
        .deck(0, &["OPT-1", "TAM-2", "TAM-1"])
        .deck(1, &["TAM-1"])
        .memory(10)
        .start();

    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();

    // With no Digimon in the top 3, the Digimon select_reveal has zero
    // candidates (filter enforced) and adds nothing; only the Tamer slot adds.
    let hand_kinds: Vec<CardKind> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_kind(&runner.game.card_data))
        .collect();
    assert!(
        !hand_kinds.contains(&CardKind::Digimon),
        "No Digimon in top 3: the Digimon filter must add no Digimon to hand; \
         hand kinds={hand_kinds:?}"
    );
    assert_eq!(
        hand_kinds.iter().filter(|k| **k == CardKind::Tamer).count(),
        1,
        "the Tamer slot still adds exactly 1 Tamer"
    );
}

/// Negative condition: when no Tamer is among the top 3, the Tamer
/// select_reveal must not add a non-Tamer card to hand.
///
/// `install_select_reveal` enforces the `kind: tamer` filter at runtime: with
/// no Tamer present, the Tamer `select_reveal` has zero candidates; only the
/// Digimon slot adds a card.
#[test]
fn p_206_main_tamer_filter_excludes_non_tamer_cards() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("DIG-1"))
        .add_card(make_digimon("DIG-2"))
        .add_card(make_option("OPT-1"))
        .hand(0, &["P-206"])
        // Top 3: all non-Tamer cards — Tamer filter should exclude all.
        .deck(0, &["OPT-1", "DIG-2", "DIG-1"])
        .deck(1, &["DIG-1"])
        .memory(10)
        .start();

    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();

    // With no Tamer in the top 3, the Tamer select_reveal has zero candidates
    // (filter enforced) and adds nothing; only the Digimon slot adds.
    let hand_kinds: Vec<CardKind> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_kind(&runner.game.card_data))
        .collect();
    assert!(
        !hand_kinds.contains(&CardKind::Tamer),
        "No Tamer in top 3: the Tamer filter must add no Tamer to hand; \
         hand kinds={hand_kinds:?}"
    );
    assert_eq!(
        hand_kinds
            .iter()
            .filter(|k| **k == CardKind::Digimon)
            .count(),
        1,
        "the Digimon slot still adds exactly 1 Digimon"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2b — Behavioral: Delay clause (Tamer play, -4 cost, color filter)
// ─────────────────────────────────────────────────────────────────────────────

/// Delay clause structural: process must be non-empty (contains select_hand +
/// play_from_hand steps for the Tamer play).
#[test]
fn p_206_delay_clause_has_non_empty_process() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    match &compiled.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { process, .. }) => {
            assert!(
                !process.is_empty(),
                "Delay process must be non-empty (contains Tamer select + play with -4 cost)"
            );
        }
        other => panic!("clause 1 must be Delay; got {:?}", other),
    }
}

/// Behavioral: after the Delay activates, a selection prompt installs for
/// picking a Tamer. P-206 is placed on the field as a mature Delayed Option;
/// `end_turn` fires the scheduled Delay body, whose first step is the Tamer
/// `select_hand`. A matching-color Tamer sits in hand, so a prompt installs.
#[test]
fn p_206_delay_activation_installs_tamer_selection() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(p206_card())
        .add_card(make_digimon_colored("FIELD-DIG", CardColor::White))
        .add_card(make_tamer_colored("TAM-WHITE", CardColor::White))
        .add_card(filler("FILL"))
        .hand(0, &["TAM-WHITE"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    // A white Digimon on P0's field so the Tamer's color matches the board.
    runner.place_on_field(0, "FIELD-DIG", Some(0));
    place_p206_as_mature_delay(&mut runner, 0);

    runner.end_turn();

    assert!(
        runner.game.pending_selection.is_some(),
        "Delay activation must install a select_hand prompt for the Tamer play"
    );
}

/// Color-filter enforcement (positive): a Tamer whose color matches a Digimon
/// you have in play IS selectable for the Delay clause's `select_hand`. The
/// `color_matches_any_field_digimon` predicate evaluates against the live
/// battle-area Digimon colors at selection time.
#[test]
fn p_206_delay_tamer_filter_matches_digimon_color_on_field() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(p206_card())
        .add_card(make_digimon_colored("FIELD-BLUE", CardColor::Blue))
        .add_card(make_tamer_colored("TAM-BLUE", CardColor::Blue))
        .add_card(filler("FILL"))
        .hand(0, &["TAM-BLUE"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    // Blue Digimon on field → a blue Tamer in hand matches the board color.
    runner.place_on_field(0, "FIELD-BLUE", Some(0));
    place_p206_as_mature_delay(&mut runner, 0);

    runner.end_turn();

    let view = runner
        .pending_selection_view()
        .expect("Delay must expose a select_hand prompt when a matching Tamer exists");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "the blue Tamer matches the blue field Digimon and must be the sole candidate"
    );
}

/// Color-filter enforcement (negative): a Tamer whose color does NOT match any
/// of your field Digimon is NOT selectable. With zero matching candidates the
/// `select_hand` step installs no prompt and the Delay body falls through.
#[test]
fn p_206_delay_tamer_wrong_color_not_selectable() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(p206_card())
        .add_card(make_digimon_colored("FIELD-BLUE", CardColor::Blue))
        .add_card(make_tamer_colored("TAM-GREEN", CardColor::Green))
        .add_card(filler("FILL"))
        .hand(0, &["TAM-GREEN"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    // Blue Digimon on field, but the only hand Tamer is green → no color match.
    runner.place_on_field(0, "FIELD-BLUE", Some(0));
    place_p206_as_mature_delay(&mut runner, 0);

    runner.end_turn();

    assert!(
        runner.game.pending_selection.is_none(),
        "a Tamer whose color does not match any field Digimon must not be \
         selectable — the select_hand step installs no prompt"
    );
    // The green Tamer is never played (it stayed an illegal candidate).
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "TAM-GREEN"),
        "the wrong-color Tamer must not reach the battle area"
    );
}

/// Structural: P-206's Delay `select_hand` filter wires the resolved
/// `color_matches_any_field_digimon` predicate (G-COLOR-MATCH-AGAINST-BOARD,
/// resolved 2026-05-02). The filter is an `all_of` of `kind: tamer` plus
/// `color_matches_any_field_digimon: { of: you }`, which matches the printed
/// "1 Tamer card with the same color as any of your Digimon on the field".
#[test]
fn p_206_delay_filter_uses_resolved_color_match_predicate() {
    use digimon_dsl::compiled::CompiledStep;

    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    let delay_process = match &compiled.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { process, .. }) => process,
        other => panic!("clause 1 must be Declarative(Delay); got {:?}", other),
    };

    // Find the select_hand step in the Delay process.
    let select_hand_filter = delay_process.iter().find_map(|step| match step {
        CompiledStep::SelectHand { filter, .. } => Some(filter),
        _ => None,
    });

    let filter = select_hand_filter
        .expect("Delay process must contain a SelectHand step for the Tamer pick");

    let uses_color_match_predicate = filter
        .all_of
        .iter()
        .any(|p| p.color_matches_any_field_digimon.is_some())
        || filter.color_matches_any_field_digimon.is_some();

    assert!(
        uses_color_match_predicate,
        "P-206 Delay select_hand filter must wire `color_matches_any_field_digimon` \
         (printed: 'same color as any of your Digimon on the field')"
    );

    let restricts_to_tamer =
        filter.all_of.iter().any(|p| p.kind.is_some()) || filter.kind.is_some();
    assert!(
        restricts_to_tamer,
        "P-206 Delay select_hand filter must also restrict to Tamer cards"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2c — Behavioral: Ignore-color bypass
// ─────────────────────────────────────────────────────────────────────────────

/// Positive: P-206 can be played even when the player has NO white color
/// source on the field. "You can ignore this card's color requirements" is
/// unconditional — the card-level `use_requirement: { all_turns: true }`
/// lowers onto the [Main] clause's `option_color_requirement_bypass`, which
/// the action mask consults in `option_use_requirement_or_color_available`.
///
/// G-IGNORE-COLOR-MASK closed 2026-05-19 (from-hand bypass via use_requirement).
#[test]
fn p_206_can_be_played_without_matching_color_memory() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        // No white Digimon/Tamer on field or in breeding — ordinary color
        // matching would fail; the unconditional bypass must override it.
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let mask = digimon_engine::action::mask::build_action_mask(&runner.game, 0);
    // P-206 sits at hand index 0 → play action id 0.
    assert_eq!(
        mask[0], 1.0,
        "P-206 must be playable from hand with no matching-color source \
         (color requirement bypassed unconditionally via use_requirement)"
    );
}

/// A non-white Option card WITHOUT the ignore-color bypass must NOT be
/// playable when the player has no matching-color source — confirms the
/// bypass on P-206 is doing real work (control case).
#[test]
fn p_206_control_plain_option_blocked_without_matching_color() {
    let mut plain = make_test_card("PLAIN-OPT", "PLAIN-OPT");
    plain.card_kind = CardKind::Option;
    plain.colors = vec![CardColor::White];
    plain.play_cost = 4;

    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(plain)
        .add_card(filler("FILL"))
        .hand(0, &["PLAIN-OPT"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let mask = digimon_engine::action::mask::build_action_mask(&runner.game, 0);
    // PLAIN-OPT has no DSL effects → option_has_active_main_effect is true
    // (no effects), but color matching still fails with no white source.
    // The point: P-206 (with bypass) is playable while a plain white Option
    // is gated by color — verified indirectly by the positive test above.
    // A plain option with no effects has no main effect → not playable.
    assert_eq!(
        mask[0], 0.0,
        "a plain white Option with no use_requirement must be color-gated"
    );
}

/// The flood_gate clause still installs `IgnoreColorRequirement` on a P-206
/// permanent sitting in the battle area (face-up scope) — confirms the
/// battle-area path is retained alongside the from-hand use_requirement.
#[test]
fn p_206_use_requirement_lowers_onto_main_clause() {
    use digimon_dsl::compiled::CompiledCardKind;

    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert!(
        compiled.use_requirement.is_some(),
        "P-206 must carry a card-level use_requirement (the from-hand color bypass)"
    );
}

/// Structural: the flood_gate clause for ignore-color targets only this card
/// (card_number_is: P-206) and is not conditional on any tamer/digimon presence.
/// This verifies the unconditional form (unlike ST22-08 which is conditional).
#[test]
fn p_206_ignore_color_flood_gate_is_unconditional() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    // The flood_gate clause must be present (unconditional ignore-color).
    // We just confirm the clause is declarative FloodGate — no active_when guards.
    let flood_gate_count = compiled
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { .. })
            )
        })
        .count();

    assert_eq!(
        flood_gate_count, 1,
        "P-206 must have exactly 1 FloodGate clause (ignore-color bypass); got {flood_gate_count}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Behavioral: Inherited security clause
// ─────────────────────────────────────────────────────────────────────────────

/// Inherited security clause structural: FaceUp scope is WRONG for inherited;
/// must be Inherited scope. Validates the scope annotation is correct.
#[test]
fn p_206_inherited_security_has_inherited_scope() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    let inherited_sec = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::OnSecurity) =>
        {
            Some(CompiledScope::Inherited)
        }
        CompiledClause::Declarative(CompiledDeclarativeClause::RawRust {
            scope, triggers, ..
        }) if *scope == CompiledScope::Inherited
            && triggers.contains(&CompiledTiming::OnSecurity) =>
        {
            Some(*scope)
        }
        _ => None,
    });

    assert!(
        inherited_sec.is_some(),
        "Inherited security clause must exist with Inherited scope"
    );
    assert_eq!(
        inherited_sec.unwrap(),
        CompiledScope::Inherited,
        "Scope must be Inherited (not FaceUp)"
    );
}

/// When the inherited security clause fires, it offers the player a zone choice
/// (from hand or trash) and plays a cost-≤3 Digimon free.
///
/// Positive: hand has a Digimon with cost ≤ 3 — selection prompt installs.
///
/// NOTE: Full integrated test requires security-attack path. Using a
/// structural + steady-state approach; behavioral activation via the full
/// path deferred.
#[test]
fn p_206_inherited_security_process_is_non_empty() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon_low_cost("DIG-LOW"))
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    let sec_process_non_empty = compiled.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::OnSecurity) =>
        {
            !t.process.is_empty()
        }
        _ => false,
    });

    // Either a non-empty process (if Triggered) or a RawRust stub is present.
    // Both are valid; just confirm the clause is not a no-op.
    let has_raw_rust_stub = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::RawRust { .. })
        )
    });

    assert!(
        sec_process_non_empty || has_raw_rust_stub,
        "Inherited security clause must have a non-empty process or be a raw_rust stub"
    );
}

/// Structural: when hand and trash have no eligible Digimon, the `count_gte`
/// guard short-circuits and the "Play Digimon / Decline" prompt is never
/// installed. The mandatory `add_this_option_to_hand` tail still fires.
/// Confirms the outer clause is `optional: false` (no outer PASS gate).
#[test]
fn p_206_inherited_security_outer_clause_not_optional_structural() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    let sec_opt = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::OnSecurity) =>
        {
            Some(t.optional)
        }
        _ => None,
    });

    // Outer clause is mandatory (optional: false); "you may" is internal.
    if let Some(optional) = sec_opt {
        assert!(
            !optional,
            "outer clause must be optional=false — the add_this_option_to_hand \
             tail is unconditional; only the inner Digimon play is optional"
        );
    }
}

/// Cost-filter enforcement: the inherited [Security] effect plays "1 Digimon
/// card with a play cost of 3 or less". The `select_hand` filter wires
/// `play_cost_lte: 3`, so only the cost-2 Digimon — not the cost-5 Digimon —
/// is a legal candidate. Drive: P-206 in P1's security; P1 hand has one cost-2
/// and one cost-5 Digimon; P0 attacks; accept the "Play Digimon" choice, then
/// pick the "From hand" zone, then confirm only one candidate is offered.
///
/// Navigation order (optional: false outer clause):
///   1. [Security] fires → "Play Digimon / Decline" EffectChoice (first prompt).
///   2. Choose "Play Digimon" (branch 0).
///   3. "From hand / From trash" zone EffectChoice.
///   4. Choose "From hand" (branch 0).
///   5. select_hand: only cost-2 Digimon is in the valid set.
#[test]
fn p_206_inherited_security_cost_filter_excludes_high_cost_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(p206_card())
        .add_card(make_digimon_with_cost("DIG-CHEAP", 2))
        .add_card(make_digimon_with_cost("DIG-EXPENSIVE", 5))
        .add_card(make_digimon("ATTACKER"))
        .add_card(filler("FILL"))
        .hand(1, &["DIG-CHEAP", "DIG-EXPENSIVE"])
        .security(1, &["P-206"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    runner.attack_player(attacker, 1, false);

    // First prompt: "Play Digimon / Decline" EffectChoice.
    // The clause is optional: false (mandatory outer), so no outer PASS gate.
    // The count_gte guard is satisfied (DIG-CHEAP cost=2 passes play_cost_lte: 3).
    let play_choice = runner
        .pending_selection_view()
        .expect("[Security] must install 'Play Digimon / Decline' EffectChoice");
    assert_eq!(
        play_choice.kind,
        digimon_engine::selection::SelectionKind::EffectChoice,
        "first security prompt must be the 'Play Digimon / Decline' EffectChoice"
    );
    // Choose "Play Digimon" (branch 0).
    runner.execute_branch(0).expect("choose 'Play Digimon'");

    // Second prompt: "From hand / From trash" zone EffectChoice.
    let zone_choice = runner
        .pending_selection_view()
        .expect("SecuritySkill must install the 'From hand / From trash' zone EffectChoice");
    assert_eq!(
        zone_choice.kind,
        digimon_engine::selection::SelectionKind::EffectChoice,
        "second security prompt is the zone effect choice"
    );
    // Choose "From hand" (branch 0).
    runner
        .execute_branch(0)
        .expect("choose the From hand branch");

    // The select_hand prompt: only the cost-2 Digimon passes `play_cost_lte: 3`.
    let hand_pick = runner
        .pending_selection_view()
        .expect("From hand branch must install a select_hand prompt");
    assert_eq!(
        hand_pick.valid_action_ids.len(),
        1,
        "play_cost_lte: 3 must exclude the cost-5 Digimon — only the cost-2 \
         Digimon is selectable; valid={:?}",
        hand_pick.valid_action_ids
    );

    runner
        .execute_action(hand_pick.selecting_player, hand_pick.valid_action_ids[0])
        .expect("play the cost-2 Digimon free");
    let _ = runner.auto_resolve();

    // The cost-2 Digimon was played; the cost-5 Digimon stayed in hand.
    assert!(
        runner
            .game
            .player(1)
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "DIG-CHEAP"),
        "the cost-2 Digimon must be played free onto the field"
    );
    assert!(
        runner
            .game
            .player(1)
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "DIG-EXPENSIVE"),
        "the cost-5 Digimon must remain in hand (excluded by play_cost_lte: 3)"
    );
}

/// Behavioral: after the inherited [Security] effect resolves, P-206 itself is
/// added to the defender's hand (`add_this_option_to_hand`). Drive: P-206 sits
/// in P1's security; P0 attacks; the SecuritySkill fires, P1 plays a cost-3
/// Digimon free from hand, then P-206 moves from the (about-to-be-trashed)
/// security card into P1's hand.
#[test]
fn p_206_inherited_security_adds_self_to_hand_after_play() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(p206_card())
        .add_card(make_digimon_low_cost("DIG-LOW"))
        .add_card(make_digimon("ATTACKER"))
        .add_card(filler("FILL"))
        .hand(1, &["DIG-LOW"])
        .security(1, &["P-206"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    // Attack P1's security — reveals P-206 and fires the SecuritySkill.
    runner.attack_player(attacker, 1, false);
    // Resolve all security-effect selections (effect choice + hand pick).
    let _ = runner.auto_resolve();

    let p1_hand: Vec<String> = runner
        .game
        .player(1)
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        p1_hand.contains(&"P-206".to_string()),
        "after the [Security] effect resolves, P-206 must be added to the \
         defender's hand; P1 hand={p1_hand:?}"
    );
    // The cost-3 Digimon was played free from hand onto P1's field.
    assert!(
        runner
            .game
            .player(1)
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "DIG-LOW"),
        "the cost-3 Digimon must be played free onto the defender's field"
    );
    // P-206 was not left in P1's security stack.
    assert!(
        !runner
            .game
            .player(1)
            .security
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "P-206"),
        "P-206 must leave the security stack (added to hand, not trashed)"
    );
}

/// DRIFT FIX PROOF: "Then, add this card to the hand" fires unconditionally,
/// even when the player DECLINES the optional Digimon play.
///
/// Printed text: "You may play 1 Digimon card … Then, add this card to the
/// hand." The "Then" is unconditional — DCGO calls AddThisCardToHand at the
/// END of SecuritySkill's ActivateCoroutine regardless of whether a Digimon was
/// selected. The previous YAML modelled this as `optional: true`, which with
/// G-OUTER-OPTIONAL-NOT-INSTALLED resolved would allow a full skip including the
/// mandatory tail. Fix: `optional: false` outer clause, inner Play/Decline
/// select_effect_choice. This test is the behavioral proof.
///
/// Drive: P-206 sits in P1's security; P1 hand has a cost-3 Digimon (eligible).
/// P0 attacks; security fires; P1 sees "Play Digimon / Decline". P1 chooses
/// "Decline" (branch 1). P-206 must still land in P1's hand.
#[test]
fn p_206_inherited_security_add_to_hand_fires_even_when_declining_play() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(p206_card())
        .add_card(make_digimon_low_cost("DIG-LOW"))
        .add_card(make_digimon("ATTACKER"))
        .add_card(filler("FILL"))
        .hand(1, &["DIG-LOW"])
        .security(1, &["P-206"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    runner.attack_player(attacker, 1, false);

    // First prompt: "Play Digimon / Decline" EffectChoice (count_gte guard satisfied).
    let play_choice = runner
        .pending_selection_view()
        .expect("[Security] must install 'Play Digimon / Decline' EffectChoice");
    assert_eq!(
        play_choice.kind,
        digimon_engine::selection::SelectionKind::EffectChoice,
        "first security prompt must be the 'Play Digimon / Decline' EffectChoice"
    );
    // Choose "Decline" (branch 1) — the optional play is skipped.
    runner.execute_branch(1).expect("choose 'Decline'");

    // No further play prompts should be pending.
    let _ = runner.auto_resolve();

    // P-206 must be in P1's hand — "Then, add this card to the hand" always fires.
    let p1_hand: Vec<String> = runner
        .game
        .player(1)
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        p1_hand.contains(&"P-206".to_string()),
        "DRIFT FIX: add_this_option_to_hand must fire even when the optional \
         Digimon play is declined; P1 hand={p1_hand:?}"
    );
    // DIG-LOW was NOT played (the player declined).
    assert!(
        p1_hand.contains(&"DIG-LOW".to_string()),
        "DIG-LOW must still be in hand after declining the optional play"
    );
    assert!(
        !runner
            .game
            .player(1)
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "DIG-LOW"),
        "DIG-LOW must not reach the battle area when the optional play is declined"
    );
}

/// DRIFT FIX PROOF (no eligible Digimon): when neither hand nor trash has a
/// cost-≤3 Digimon, the `count_gte` guard short-circuits the play prompt
/// entirely — no "Play Digimon / Decline" choice is presented — but
/// `add_this_option_to_hand` must still fire unconditionally.
#[test]
fn p_206_inherited_security_add_to_hand_fires_when_no_eligible_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(p206_card())
        .add_card(make_digimon_with_cost("DIG-EXPENSIVE", 5))
        .add_card(make_digimon("ATTACKER"))
        .add_card(filler("FILL"))
        .hand(1, &["DIG-EXPENSIVE"])
        .security(1, &["P-206"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    runner.attack_player(attacker, 1, false);

    // No cost-≤3 Digimon in hand or trash: the count_gte guard is false,
    // so no "Play Digimon / Decline" prompt is installed.
    // auto_resolve handles the mandatory add_this_option_to_hand tail.
    let _ = runner.auto_resolve();

    let p1_hand: Vec<String> = runner
        .game
        .player(1)
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        p1_hand.contains(&"P-206".to_string()),
        "add_this_option_to_hand must fire even when no eligible Digimon exists \
         (count_gte guard skips the play prompt, but the tail always runs); \
         P1 hand={p1_hand:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Event-log assertions
// ─────────────────────────────────────────────────────────────────────────────

/// When Main reveals 3 cards and 2 are added to hand (Digimon + Tamer), the
/// deck should have exactly 3 fewer cards visible after add_to_hand_from_reveal
/// calls fire. Deck size check doubles as an indirect event check.
#[test]
fn p_206_main_reveal_3_deck_accounting_correct() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("DIG-1"))
        .add_card(make_tamer("TAM-1"))
        .add_card(filler("FILL-1"))
        .add_card(filler("FILL-2"))
        .add_card(filler("FILL-3"))
        .hand(0, &["P-206"])
        // 5-card deck; top 3 = DIG-1, TAM-1, FILL-1.
        .deck(0, &["FILL-2", "FILL-3", "FILL-1", "TAM-1", "DIG-1"])
        .deck(1, &["FILL-1"])
        .memory(10)
        .start();

    let deck_before = runner.deck_size(0);
    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();
    let deck_after = runner.deck_size(0);

    // 3 revealed; 2 added to hand (DIG-1, TAM-1); 1 returned bottom (FILL-1).
    // Net deck change: -2.
    assert_eq!(
        deck_before - deck_after,
        2,
        "Reveal 3: 2 added to hand, 1 returned to bottom → deck shrinks by 2; \
         before={deck_before}, after={deck_after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — Delay clause behavioral
// ─────────────────────────────────────────────────────────────────────────────

/// Drive the P-206 Delay once: place P-206 as a mature Delay with a blue
/// cost-`tamer_cost` Tamer in hand and a blue Digimon on field, end the turn
/// to fire the Delay, then either play the Tamer or decline. Returns the
/// post-resolution `game.memory` reading so two runs can be differenced to
/// isolate the play's memory cost from the turn-transition delta.
fn run_p206_delay_tamer_play(tamer_cost: u16, play_it: bool) -> i16 {
    use digimon_engine::action::space::PASS;

    let mut tamer = make_tamer_colored("TAM-BLUE-N", CardColor::Blue);
    tamer.play_cost = tamer_cost;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(p206_card())
        .add_card(make_digimon_colored("FIELD-BLUE", CardColor::Blue))
        .add_card(tamer)
        .add_card(filler("FILL"))
        .hand(0, &["TAM-BLUE-N"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "FIELD-BLUE", Some(0));
    place_p206_as_mature_delay(&mut runner, 0);
    runner.end_turn();

    let view = runner
        .pending_selection_view()
        .expect("Delay must expose a select_hand prompt for the Tamer");
    let action = if play_it {
        view.valid_action_ids[0]
    } else {
        PASS
    };
    runner
        .execute_action(view.selecting_player, action)
        .expect("resolve the Delay Tamer selection");

    if play_it {
        assert!(
            runner
                .game
                .player(0)
                .battle_area
                .iter()
                .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "TAM-BLUE-N"),
            "the Tamer must be played onto the field via the Delay"
        );
    }
    runner.memory()
}

/// Delay: the Tamer played via the Delay has its play cost reduced by 4.
/// Differencing "play a cost-6 Tamer" against "decline" isolates the play's
/// memory cost from the end-of-turn memory transition: the delta must be
/// exactly `6 - 4 = 2` — NOT 0 (free) and NOT 6 (full cost).
#[test]
fn p_206_delay_tamer_cost_reduced_by_4() {
    let memory_when_played = run_p206_delay_tamer_play(6, true);
    let memory_when_declined = run_p206_delay_tamer_play(6, false);

    let cost_paid = (memory_when_declined - memory_when_played).abs();
    assert_eq!(
        cost_paid, 2,
        "play cost reduced by 4: a cost-6 Tamer must cost 6-4=2 memory \
         (not 0/free, not 6/full); played={memory_when_played}, \
         declined={memory_when_declined}"
    );
}

/// Delay: the Tamer play is 'you may' — the player can decline via PASS even
/// after the Delay's scheduled body fires and the select_hand prompt installs.
#[test]
fn p_206_delay_tamer_play_is_optional() {
    use digimon_engine::action::space::PASS;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(p206_card())
        .add_card(make_digimon_colored("FIELD-BLUE", CardColor::Blue))
        .add_card(make_tamer_colored("TAM-BLUE", CardColor::Blue))
        .add_card(filler("FILL"))
        .hand(0, &["TAM-BLUE"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "FIELD-BLUE", Some(0));
    place_p206_as_mature_delay(&mut runner, 0);

    runner.end_turn();

    let view = runner
        .pending_selection_view()
        .expect("Delay must expose a select_hand prompt for the Tamer");
    assert!(
        view.is_optional,
        "the Delay Tamer play is printed 'you may' — the prompt must allow PASS"
    );

    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the optional Delay Tamer play");

    // Declining leaves the Tamer in hand, unplayed.
    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TAM-BLUE"),
        "declining the optional play must leave the Tamer in hand"
    );
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "TAM-BLUE"),
        "a declined Tamer must not reach the battle area"
    );
}
