//! BT16-027 Imperialdramon: Fighter Mode — Digimon, Lv.6, Blue/Green, DP 13000, Cost 8.
//! Traits: Ancient Dragonkin.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [Hand] [Counter] <Blast Digivolve>
//!   (Your Digimon may digivolve into this card without paying the cost.)
//!
//! [On Play] [When Digivolving]
//!   Return 1 of your opponent's Digimon with as many or fewer digivolution
//!   cards as this Digimon to the bottom of the deck.
//!
//! [End of Attack] [Once Per Turn]
//!   Unsuspend this Digimon. Then, if [Imperialdramon: Dragon Mode] is in this
//!   Digimon's digivolution cards, return 1 of your opponent's suspended Digimon
//!   to the bottom of the deck.
//! ```
//!
//! Inherited: `Ace Overflow <-4>` — ACE keyword with -4 memory penalty.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT16/Blue/BT16_027.cs
//!
//! # Clause analysis
//!
//! - Clause 0 (Blast Digivolve): Supported via `kind: burst_digivolve` alt-path
//!   + `kind: grant_keyword, keyword: BlastDigivolve`. The activated digivolve from
//!   [Imperialdramon: Dragon Mode] is modelled as `kind: activated_digivolve`.
//!
//! - Clause 1 ([On Play][When Digivolving] return-to-bottom):
//!   IMPLEMENTED — `select_opponent_permanent` filtered by
//!   `materials_count_lte: { formula: { source_material_count: {} } }`, which
//!   compares each candidate's digivolution-card count against this Digimon's at
//!   runtime. AD1-025 uses the identical pattern faithfully.
//!
//! - Clause 2 ([End of Attack][OPT] unsuspend + conditional return):
//!   IMPLEMENTED for the Dragon Mode source predicate slice. The source-relative
//!   `self_digivolution_contains_name` predicate gates the opponent suspended
//!   Digimon return prompt.
//!
//! - Inherited (Ace Overflow -4): Fully implemented via top-level `ace_overflow: -4`.
//!
//! # Patterns
//! - H12: Blast Digivolve / burst_digivolve alt-path + BlastDigivolve keyword grant
//! - H13: ACE Overflow -4 via `ace_overflow` top-level field
//! - `materials_count_lte` + `source_material_count`: dynamic digivolution-card
//!   count comparison for the [On Play][When Digivolving] return clause
//! - `self_digivolution_contains_name`: gates the Dragon Mode End of Attack rider

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::{SelectionKind, TriggerSource};
use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledTiming,
};
use digimon_engine::permanent::PermanentHandle;

// ─── Fixture ─────────────────────────────────────────────────────────────────

fn fighter_mode_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT16-027")
        .expect("BT16-027 YAML loads")
}

// ─── Structural tests ────────────────────────────────────────────────────────

/// Verify basic card metadata matches cards.json.
#[test]
fn bt16_027_metadata_level_cost_dp_color() {
    let runner = fighter_mode_runner().start();
    let compiled = runner.compiled_card("BT16-027").expect("BT16-027 compiles");

    assert_eq!(compiled.name, "Imperialdramon: Fighter Mode");
    assert_eq!(compiled.level, Some(6));
    assert_eq!(compiled.cost, Some(8));
    assert_eq!(compiled.dp, Some(13000));
    assert!(
        compiled.color.contains(&CompiledColor::Blue),
        "must be blue; color={:?}",
        compiled.color
    );
    assert!(
        compiled.color.contains(&CompiledColor::Green),
        "must be green; color={:?}",
        compiled.color
    );
}

/// Verify Ace Overflow -4 is recorded on the compiled card.
#[test]
fn bt16_027_ace_overflow_is_minus_4() {
    let runner = fighter_mode_runner().start();
    let compiled = runner.compiled_card("BT16-027").expect("BT16-027 compiles");

    let ace = compiled.ace_overflow.expect("ace_overflow must be present");
    assert_eq!(ace, -4, "Ace Overflow must be -4 per printed text");
}

/// Verify the standard digivolve path (Lv.5 Blue / Cost 5) is present.
#[test]
fn bt16_027_has_standard_lv5_blue_digivolve_path() {
    let runner = fighter_mode_runner().start();
    let compiled = runner.compiled_card("BT16-027").expect("BT16-027 compiles");

    let has_standard = compiled.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(5))
            && p.from
                .as_ref()
                .is_some_and(|f| f.level_eq == Some(5) && f.color_is == Some(CompiledColor::Blue))
    });
    assert!(
        has_standard,
        "must have Lv.5 Blue cost-5 digivolve path; alt_paths={:?}",
        compiled.alt_paths
    );
}

/// Verify the activated digivolve path (from Dragon Mode on own field / Cost 2).
#[test]
fn bt16_027_has_activated_digivolve_from_dragon_mode() {
    let runner = fighter_mode_runner().start();
    let compiled = runner.compiled_card("BT16-027").expect("BT16-027 compiles");

    let has_activated = compiled.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::ActivatedDigivolve
            && p.cost == Some(CompiledCost::Literal(2))
    });
    assert!(
        has_activated,
        "must have ActivatedDigivolve path with cost 2 for Imperialdramon: Dragon Mode; alt_paths={:?}",
        compiled.alt_paths
    );
}

/// Verify the burst digivolve (Blast Digivolve) alt-path is present.
#[test]
fn bt16_027_has_burst_digivolve_alt_path() {
    let runner = fighter_mode_runner().start();
    let compiled = runner.compiled_card("BT16-027").expect("BT16-027 compiles");

    let has_burst = compiled.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::BurstDigivolve && p.cost == Some(CompiledCost::Literal(0))
    });
    assert!(
        has_burst,
        "must have BurstDigivolve path with cost 0; alt_paths={:?}",
        compiled.alt_paths
    );
}

/// Verify the BlastDigivolve keyword grant declarative clause is present.
#[test]
fn bt16_027_blast_digivolve_keyword_grant_is_declared() {
    let runner = fighter_mode_runner().start();
    let compiled = runner.compiled_card("BT16-027").expect("BT16-027 compiles");

    let has_blast_keyword = compiled.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == "BlastDigivolve"
        )
    });
    assert!(
        has_blast_keyword,
        "must have a GrantKeyword clause for BlastDigivolve"
    );
}

/// Confirm Clause 2 (on_play/when_digivolving return) is authored now that the
/// `materials_count_lte` + `source_material_count` predicate pair is available.
#[test]
fn bt16_027_on_play_clause_is_authored() {
    let runner = fighter_mode_runner().start();
    let compiled = runner.compiled_card("BT16-027").expect("BT16-027 compiles");

    let on_play_triggered = compiled.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(t) => {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        }
        _ => false,
    });
    assert!(
        on_play_triggered,
        "on_play / when_digivolving return clause must be authored"
    );
}

/// Confirm Clause 3 (end_of_attack unsuspend + conditional) is authored now that
/// the source-stack name predicate is available.
#[test]
fn bt16_027_end_of_attack_clause_is_once_per_turn() {
    let runner = fighter_mode_runner().start();
    let compiled = runner.compiled_card("BT16-027").expect("BT16-027 compiles");

    let end_of_attack = compiled.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(t) => {
            t.when.contains(&CompiledTiming::EndOfAttack).then_some(t)
        }
        _ => None,
    });
    let clause = end_of_attack.expect("end_of_attack clause must be authored");
    assert!(clause.once_per_turn, "printed End of Attack effect is OPT");
    assert!(!clause.optional, "printed End of Attack effect is mandatory");
}

// ─── [On Play][When Digivolving] behavioural tests ───────────────────────────

fn fire_on_play(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

/// [On Play][When Digivolving]: return 1 opp Digimon with ≤ digi-cards as this.
///
/// The Fighter is placed on a stack of 3 cards (2 digivolution cards). An
/// opponent Digimon with 2 digivolution cards (equal) and one with 1 (fewer)
/// must both be selectable; the selected one returns to the deck bottom.
#[test]
fn bt16_027_on_play_returns_opp_digimon_with_lte_digi_cards() {
    let mut runner = end_of_attack_runner();
    // Fighter on a 3-card stack → 2 digivolution cards.
    let fighter = runner.place_stack(0, &["PLAIN-SOURCE", "DRAGON-MODE", "BT16-027"]);
    // Opponent Digimon with equal digivolution-card count (2).
    let opp_equal = runner.place_on_field(1, "OPP-SUSPENDED", Some(0));
    runner.push_source(opp_equal, "PLAIN-SOURCE");
    runner.push_source(opp_equal, "PLAIN-SOURCE");
    // Opponent Digimon with fewer digivolution cards (1).
    let opp_fewer = runner.place_on_field(1, "OPP-UNSUSPENDED", Some(0));
    runner.push_source(opp_fewer, "PLAIN-SOURCE");

    fire_on_play(&mut runner, fighter);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    let view = runner
        .pending_selection_view()
        .expect("on-play return prompt should install");
    let equal_action = encode_attack(0, opp_equal.index as u16);
    let fewer_action = encode_attack(0, opp_fewer.index as u16);
    assert!(
        view.valid_action_ids.contains(&equal_action),
        "opponent Digimon with EQUAL digivolution cards must be selectable"
    );
    assert!(
        view.valid_action_ids.contains(&fewer_action),
        "opponent Digimon with FEWER digivolution cards must be selectable"
    );

    let deck_before = runner.deck_size(1);
    let battle_before = runner.battle_area_size(1);
    runner
        .execute_action(0, equal_action)
        .expect("select the equal-count opponent Digimon");
    assert_eq!(
        runner.battle_area_size(1),
        battle_before - 1,
        "selected opponent Digimon must leave the battle area"
    );
    assert_eq!(
        runner.deck_size(1),
        deck_before + 1,
        "selected opponent Digimon must return to the bottom of the deck"
    );
}

/// [On Play][When Digivolving]: opponent Digimon with MORE digi-cards is excluded.
#[test]
fn bt16_027_on_play_excludes_opp_digimon_with_more_digi_cards() {
    let mut runner = end_of_attack_runner();
    // Fighter played from hand → 0 digivolution cards.
    let fighter = runner.place_on_field(0, "BT16-027", Some(0));
    // Opponent Digimon with MORE digivolution cards (1 > 0).
    let opp_more = runner.place_on_field(1, "OPP-SUSPENDED", Some(0));
    runner.push_source(opp_more, "PLAIN-SOURCE");
    // Opponent Digimon with EQUAL count (0) — the only legal target.
    let opp_equal = runner.place_on_field(1, "OPP-UNSUSPENDED", Some(0));

    fire_on_play(&mut runner, fighter);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    let view = runner
        .pending_selection_view()
        .expect("on-play return prompt should install");
    let more_action = encode_attack(0, opp_more.index as u16);
    let equal_action = encode_attack(0, opp_equal.index as u16);
    assert!(
        !view.valid_action_ids.contains(&more_action),
        "opponent Digimon with MORE digivolution cards must be excluded"
    );
    assert_eq!(
        view.valid_action_ids,
        vec![equal_action],
        "only the equal-count opponent Digimon should be selectable"
    );

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[more_action as usize], 0.0,
        "action mask must not expose the more-digi-cards opponent Digimon"
    );
}

fn fire_end_of_attack(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfAttack, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

fn set_suspended(runner: &mut DebugRunner, handle: PermanentHandle, suspended: bool) {
    runner.game.players[handle.player as usize].battle_area[handle.index as usize].is_suspended =
        suspended;
}

fn is_suspended(runner: &DebugRunner, handle: PermanentHandle) -> bool {
    runner.game.players[handle.player as usize].battle_area[handle.index as usize].is_suspended
}

fn end_of_attack_runner() -> DebugRunner {
    fighter_mode_runner()
        .add_card(make_test_card(
            "DRAGON-MODE",
            "Imperialdramon: Dragon Mode",
        ))
        .add_card(make_test_card("PLAIN-SOURCE", "Plain Source"))
        .add_card(make_test_card("OPP-SUSPENDED", "Opponent Suspended"))
        .add_card(make_test_card("OPP-UNSUSPENDED", "Opponent Unsuspended"))
        .memory(10)
        .start()
}

/// [End of Attack][OPT] always unsuspends this Digimon.
#[test]
fn bt16_027_end_of_attack_unsuspends_self() {
    let mut runner = end_of_attack_runner();
    let fighter = runner.place_on_field(0, "BT16-027", Some(0));
    set_suspended(&mut runner, fighter, true);

    fire_end_of_attack(&mut runner, fighter);

    assert!(
        !is_suspended(&runner, fighter),
        "End of Attack must unsuspend BT16-027"
    );
}

/// [End of Attack][OPT][Once Per Turn] — conditional return fires when Dragon Mode is in digi-cards.
#[test]
fn bt16_027_end_of_attack_returns_opp_suspended_when_dragon_mode_in_digi_cards() {
    let mut runner = end_of_attack_runner();
    let fighter = runner.place_stack(0, &["DRAGON-MODE", "BT16-027"]);
    let suspended = runner.place_on_field(1, "OPP-SUSPENDED", Some(0));
    let unsuspended = runner.place_on_field(1, "OPP-UNSUSPENDED", Some(0));
    set_suspended(&mut runner, fighter, true);
    set_suspended(&mut runner, suspended, true);

    fire_end_of_attack(&mut runner, fighter);

    assert!(
        !is_suspended(&runner, fighter),
        "BT16-027 must unsuspend before the conditional return prompt"
    );
    assert!(
        !is_suspended(&runner, unsuspended),
        "control target starts unsuspended"
    );
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    assert!(
        !runner.pending_is_optional(),
        "printed return rider is mandatory when a suspended target exists"
    );

    let view = runner
        .pending_selection_view()
        .expect("Dragon Mode source should install opponent selection");
    let suspended_action = encode_attack(0, suspended.index as u16);
    let unsuspended_action = encode_attack(0, unsuspended.index as u16);
    assert_eq!(
        view.valid_action_ids,
        vec![suspended_action],
        "only suspended opponent Digimon should be selectable"
    );
    assert!(
        !view.valid_action_ids.contains(&unsuspended_action),
        "unsuspended opponent Digimon must be excluded from action ids"
    );

    let mask = build_action_mask(&runner.game, 0);
    assert!(mask[suspended_action as usize] > 0.0);
    assert_eq!(
        mask[unsuspended_action as usize], 0.0,
        "action mask must not expose the unsuspended opponent Digimon"
    );

    let deck_before = runner.deck_size(1);
    runner
        .execute_action(0, suspended_action)
        .expect("select suspended opponent Digimon");
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "selected suspended opponent Digimon must leave the battle area"
    );
    assert_eq!(
        runner.deck_size(1),
        deck_before + 1,
        "selected suspended opponent Digimon must return to deck bottom"
    );
    assert!(
        runner.pending_selection().is_none(),
        "selection should be fully resolved"
    );
}

/// [End of Attack][OPT][Once Per Turn] — conditional return does NOT fire when Dragon Mode is absent.
#[test]
fn bt16_027_end_of_attack_no_selection_when_dragon_mode_absent_from_digi_cards() {
    let mut runner = end_of_attack_runner();
    let fighter = runner.place_stack(0, &["PLAIN-SOURCE", "BT16-027"]);
    let suspended = runner.place_on_field(1, "OPP-SUSPENDED", Some(0));
    set_suspended(&mut runner, fighter, true);
    set_suspended(&mut runner, suspended, true);

    fire_end_of_attack(&mut runner, fighter);

    assert!(
        !is_suspended(&runner, fighter),
        "BT16-027 must still unsuspend without Dragon Mode in sources"
    );
    assert!(
        runner.pending_selection().is_none(),
        "without Dragon Mode in digivolution cards, the return prompt must not install"
    );
}

/// [End of Attack][OPT][Once Per Turn] OPT enforcement.
#[test]
fn bt16_027_end_of_attack_is_once_per_turn() {
    let mut runner = end_of_attack_runner();
    let fighter = runner.place_stack(0, &["DRAGON-MODE", "BT16-027"]);
    let first = runner.place_on_field(1, "OPP-SUSPENDED", Some(0));
    set_suspended(&mut runner, first, true);
    set_suspended(&mut runner, fighter, true);

    fire_end_of_attack(&mut runner, fighter);
    let action = runner
        .pending_selection_view()
        .expect("first End of Attack should prompt")
        .valid_action_ids[0];
    runner
        .execute_action(0, action)
        .expect("resolve first End of Attack prompt");

    let second = runner.place_on_field(1, "OPP-SUSPENDED", Some(0));
    set_suspended(&mut runner, second, true);
    set_suspended(&mut runner, fighter, true);

    fire_end_of_attack(&mut runner, fighter);

    assert!(
        is_suspended(&runner, fighter),
        "OPT lockout must prevent a second same-turn unsuspend"
    );
    assert!(
        runner.pending_selection().is_none(),
        "OPT lockout must prevent a second same-turn return prompt"
    );
}
