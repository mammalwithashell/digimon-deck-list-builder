//! BT17-095 Miraculous Mega Knight — Option, Cost 2, Red+Blue.
//!
//! # Card text (cards.json)
//!
//! [Main] You may play 1 [Agumon] or [Gabumon] from your hand or trash without
//! paying the cost. Then, place this card in the battle area.
//!
//! [All Turns] When one of your level 6 Digimon with [Greymon] or [Garurumon]
//! in its name would leave the battle area outside of a battle, ＜Delay＞ (By
//! trashing this card after the placing turn, activate the effect below.)
//! ・That Digimon and a card in the hand may DNA digivolve into a Digimon card
//! with [Omnimon] in its name in the hand.
//!
//! Inherited (Security): [Security] You may play 1 card with [Tai Kamiya] or
//! [Matt Ishida] in its name from your hand or trash without paying the cost.
//! Then, add this card to the hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT17/Red/BT17_095.cs
//!
//! # Patterns this test file covers
//!
//! - Clause A (Main): `select_effect_choice` (From hand / From trash) +
//!   branching `select_hand`/`select_trash` + `play_from_*_free` (the
//!   G-DSL-UNION-PLAY-FREE workaround for hand-or-trash optional play).
//!   Final step: `place_self_as_delay_option` to seat this Option as a
//!   Delay-Option permanent.
//! - Clause B (All Turns leave-watcher): `kind: replacement` with
//!   `trigger: when_would_leave_battle_area` filtered by
//!   `replacement_subject_is_mine: true` + `level_eq: 6` +
//!   `name_contains: Greymon|Garurumon` + `none_of: [replacement_cause: battle]`.
//!   Process pays the Delay cost (`delete_permanent: { target: source }`) but
//!   the DNA-digivolve-into-Omnimon body is omitted (BLOCKED on
//!   G-DSL-DNA-FROM-HAND-PARTNER).
//! - Clause C (Security inherited): same `select_effect_choice` workaround +
//!   `add_this_option_to_hand` tail.
//!
//! # Known gaps
//!
//! - **G-DSL-UNION-PLAY-FREE**: `select_union_zone` binds a Card-typed
//!   binding, but `play_from_hand_free` / `play_from_trash_free` need
//!   HandIndex / TrashIndex bindings. Workaround: `select_effect_choice` +
//!   branching. Tests that would assert auto-collapse to the populated zone
//!   (the DCGO `SetBool(...)` shortcut) are #[ignore]'d.
//! - **G-DSL-DNA-FROM-HAND-PARTNER**: `effect_initiated_dna_digivolve`
//!   requires both DNA materials to be on-field permanents. BT17-095's second
//!   material lives in the hand. The DNA-into-Omnimon sub-clause is omitted
//!   from Clause B's process; behavioral tests for the DNA branch are
//!   #[ignore]'d.

#![allow(unused_imports, dead_code)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

const BT17_095_YAML: &str = include_str!("../../../cards/bt17/BT17-095.yaml");

// ─── Helper cards ────────────────────────────────────────────────────────────

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A red Digimon used for [Main] selection: name carries either Agumon or
/// Gabumon. Level 3 / DP 2000 / Cost 3.
fn make_named_red_digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c
}

/// A level-6 Digimon with the given name (intended to carry "Greymon" or
/// "Garurumon"). Used for the Clause B leave-watcher tests.
fn make_l6_digimon_named(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 11;
    c.colors = vec![CardColor::Red];
    c
}

/// A non-matching Digimon (for negative tests of Clause B subject filter).
fn make_l6_digimon_unmatched(id: &str) -> CardData {
    let mut c = make_test_card(id, "OtherL6");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 11;
    c.colors = vec![CardColor::Red];
    c
}

/// A Tamer with a Tai Kamiya / Matt Ishida name for the Security clause.
fn make_named_tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c
}

fn bt17_095_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(BT17_095_YAML)
        .expect("BT17-095 YAML must parse")
        .memory(10)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

/// YAML must parse and compile without error.
#[test]
fn bt17_095_yaml_parses_without_error() {
    let _runner = bt17_095_runner();
}

/// Three compiled clauses: Main (triggered), All-Turns (declarative
/// replacement), Security (triggered, inherited scope).
#[test]
fn bt17_095_has_three_clauses() {
    let runner = bt17_095_runner();
    let compiled = runner
        .compiled_card("BT17-095")
        .expect("BT17-095 must be in compiled_cards");

    assert_eq!(
        compiled.effects.len(),
        3,
        "BT17-095 must have exactly 3 compiled clauses (Main, Replacement, Security); got {}",
        compiled.effects.len()
    );
}

/// Clause A: triggered with `main_from_hand` timing, optional, FaceUp scope.
#[test]
fn bt17_095_main_clause_is_optional_face_up() {
    let runner = bt17_095_runner();
    let compiled = runner
        .compiled_card("BT17-095")
        .expect("BT17-095 must be in compiled_cards");

    let main_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("Main clause with MainFromHand timing must exist");

    assert!(
        main_clause.optional,
        "BT17-095 Main clause must be optional (printed: 'You may play')"
    );
    assert_eq!(
        main_clause.scope,
        CompiledScope::FaceUp,
        "BT17-095 Main clause must have FaceUp scope (Option played from hand)"
    );
}

/// Clause B is a declarative replacement (NOT a triggered clause), targeting
/// `WhenWouldLeaveBattleArea` timing.
#[test]
fn bt17_095_has_replacement_clause_for_when_would_leave_battle_area() {
    let runner = bt17_095_runner();
    let compiled = runner
        .compiled_card("BT17-095")
        .expect("BT17-095 must be in compiled_cards");

    let has_leave_replacement = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                trigger,
                ..
            }) if trigger == "when_would_leave_battle_area"
        )
    });

    assert!(
        has_leave_replacement,
        "BT17-095 must have a declarative `when_would_leave_battle_area` replacement clause"
    );
}

/// Clause C: triggered with `on_security` timing, optional, INHERITED scope.
#[test]
fn bt17_095_security_clause_is_optional_inherited() {
    let runner = bt17_095_runner();
    let compiled = runner
        .compiled_card("BT17-095")
        .expect("BT17-095 must be in compiled_cards");

    let sec_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("Security clause with OnSecurity timing must exist");

    assert!(
        sec_clause.optional,
        "BT17-095 Security clause must be optional (printed: 'You may play')"
    );
    assert_eq!(
        sec_clause.scope,
        CompiledScope::Inherited,
        "BT17-095 Security clause must have Inherited scope (rides on the option's inherited surface)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Clause A: [Main] play [Agumon]/[Gabumon] from hand or trash
// ═══════════════════════════════════════════════════════════════════════════

/// Activating [Main] from hand installs a pending selection (the From-hand /
/// From-trash effect choice). Smoke test: the activation does not panic and
/// puts the engine into a SelectionKind::EffectChoice state.
#[test]
fn bt17_095_main_installs_zone_choice_selection() {
    let agumon = make_named_red_digimon("BT17095-AGU", "Agumon");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_095_YAML)
        .expect("BT17-095 YAML parses")
        .add_card(agumon.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT17-095", "BT17095-AGU"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(
        fired,
        "activate_hand_main must return true for BT17-095 at hand index 0"
    );

    assert!(
        runner.game.pending_selection.is_some(),
        "BT17-095 Main must install a pending zone-choice selection"
    );
}

/// Even with no eligible Agumon/Gabumon in hand or trash, the [Main] effect's
/// outer zone-choice prompt still installs (the player may select either
/// branch and resolve to a no-op selection). The whole flow must not panic.
#[test]
fn bt17_095_main_no_panic_when_no_eligible_named_digimon_anywhere() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_095_YAML)
        .expect("BT17-095 YAML parses")
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["BT17-095"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired);

    // Drain any installed selections; with no eligible [Agumon]/[Gabumon],
    // each branch's optional select_hand / select_trash must be a no-op.
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 30 {
        let player = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        let _ = runner.game.resolve_selection(player, action);
        runner.game.drain_effect_queue();
        steps += 1;
    }
    // Primary assertion: no panic.
}

/// G-DSL-UNION-PLAY-FREE: with the workaround in place, the engine cannot
/// auto-collapse the From-hand / From-trash decision to the populated zone
/// (DCGO does this via `SetBool(...)` when only one zone is non-empty). The
/// player must always pick one of the two branches.
///
/// When the gap closes (`select_union_zone` widened to bind the picked card
/// as a HandIndex/TrashIndex), this card's Main clause should be rewritten
/// to use a single `select_union_zone` step and this test should be enabled
/// to assert the auto-collapse behavior.
#[test]
#[ignore = "pending: G-DSL-UNION-PLAY-FREE — select_union_zone binds Card, not HandIndex/TrashIndex"]
fn bt17_095_main_auto_collapses_zone_choice_when_only_one_zone_eligible() {
    // Placeholder test. When G-DSL-UNION-PLAY-FREE closes:
    //   1. Seed P0 with BT17-095 in hand and an Agumon ONLY in trash.
    //   2. Activate Main.
    //   3. Assert that the FIRST pending selection is the trash-card pick
    //      (engine collapsed the empty hand branch).
    unimplemented!("blocked on G-DSL-UNION-PLAY-FREE");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Clause B: [All Turns] cross-permanent leave watcher
// ═══════════════════════════════════════════════════════════════════════════

/// Smoke test: with BT17-095 placed on field as an Option (not yet seated
/// through the full Main flow), and a non-matching opponent Digimon being
/// deleted, the replacement clause must NOT fire (subject filter rejects
/// non-mine subjects).
#[test]
fn bt17_095_replacement_does_not_fire_for_opponent_digimon_leaving() {
    let opp_l6 = make_l6_digimon_named("BT17095-OPP-GREY", "Greymon");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_095_YAML)
        .expect("BT17-095 YAML parses")
        .add_card(opp_l6.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let _bt17_095 = runner.place_on_field(0, "BT17-095", Some(0));
    let opp_grey = runner.place_on_field(1, "BT17095-OPP-GREY", None);

    // Snapshot field counts.
    let bt17_095_before = runner.game.players[0].battle_area.len();
    let _ = bt17_095_before;

    // Delete the opponent's L6 Greymon — should NOT trigger BT17-095's
    // replacement (subject_is_mine filter rejects opponent subjects). The
    // Greymon should leave normally and BT17-095 should remain on the field.
    runner.game.delete_permanent_with_cause(
        opp_grey,
        digimon_engine::replacement::ReplacementCause::OwnEffect,
    );
    runner.game.drain_effect_queue();

    // BT17-095 should still be on P0's field (no Delay cost was triggered).
    let bt17_095_still_present = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT17-095");
    assert!(
        bt17_095_still_present,
        "BT17-095 must remain on field; replacement should not fire for opponent's leaving Digimon"
    );
}

/// Smoke test: when an OWN level-3 (NOT level 6) Greymon-named Digimon leaves,
/// the replacement should NOT fire (level filter rejects).
#[test]
fn bt17_095_replacement_does_not_fire_for_own_low_level_greymon() {
    let l3_grey = make_named_red_digimon("BT17095-L3-GREY", "Greymon");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_095_YAML)
        .expect("BT17-095 YAML parses")
        .add_card(l3_grey.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let _bt17_095 = runner.place_on_field(0, "BT17-095", Some(0));
    let own_l3 = runner.place_on_field(0, "BT17095-L3-GREY", None);

    runner.game.delete_permanent_with_cause(
        own_l3,
        digimon_engine::replacement::ReplacementCause::OwnEffect,
    );
    runner.game.drain_effect_queue();

    let bt17_095_still_present = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT17-095");
    assert!(
        bt17_095_still_present,
        "BT17-095 must remain on field; level_eq:6 filter rejects level-3 subject"
    );
}

/// Smoke test: when an OWN level-6 Digimon WITHOUT Greymon/Garurumon in name
/// leaves, the replacement should NOT fire (name filter rejects).
#[test]
fn bt17_095_replacement_does_not_fire_for_own_l6_unmatched_name() {
    let l6_other = make_l6_digimon_unmatched("BT17095-L6-OTHER");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_095_YAML)
        .expect("BT17-095 YAML parses")
        .add_card(l6_other.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let _bt17_095 = runner.place_on_field(0, "BT17-095", Some(0));
    let own_l6 = runner.place_on_field(0, "BT17095-L6-OTHER", None);

    runner.game.delete_permanent_with_cause(
        own_l6,
        digimon_engine::replacement::ReplacementCause::OwnEffect,
    );
    runner.game.drain_effect_queue();

    let bt17_095_still_present = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT17-095");
    assert!(
        bt17_095_still_present,
        "BT17-095 must remain on field; name filter rejects non-Greymon/non-Garurumon subject"
    );
}

/// G-DSL-DNA-FROM-HAND-PARTNER: the printed body's DNA-digivolve sub-clause
/// (where the second material lives in the controller's hand) cannot be
/// expressed with the current `effect_initiated_dna_digivolve` verb. Until
/// the engine grows a hand-partner variant, the DNA reward is omitted from
/// Clause B and the leaving Digimon proceeds without merging into Omnimon.
///
/// This test is the reminder that the DNA leg exists in the printed text but
/// is not asserted positively in the current implementation. When the gap
/// closes:
///   1. Seed P0 with BT17-095 on field as a Delay-Option.
///   2. Seed P0 with a level-6 Greymon-named Digimon on field.
///   3. Seed P0 with an Omnimon-name level-7 hand card and a matching DNA
///      partner card in hand.
///   4. Trigger leave (own-effect deletion of the Greymon).
///   5. Drive the DNA prompts; assert the merged Omnimon permanent appears
///      with the leaving Greymon's stack underneath.
#[test]
#[ignore = "pending: G-DSL-DNA-FROM-HAND-PARTNER — effect_initiated_dna_digivolve cannot accept hand-card target_b"]
fn bt17_095_replacement_dna_digivolves_into_omnimon_when_eligible() {
    unimplemented!("blocked on G-DSL-DNA-FROM-HAND-PARTNER");
}

/// G-DSL-DNA-FROM-HAND-PARTNER: when the DNA leg is implemented, fires that
/// the leaving subject does NOT proceed to the trash if a successful DNA
/// merge consumes it (it migrates into the merged Omnimon permanent's stack).
/// Until then, this assertion is unreachable — the leaving subject always
/// proceeds (default Proceed outcome).
#[test]
#[ignore = "pending: G-DSL-DNA-FROM-HAND-PARTNER — leaving subject always proceeds with DNA omitted"]
fn bt17_095_replacement_dna_consumes_leaving_subject_into_merged_permanent() {
    unimplemented!("blocked on G-DSL-DNA-FROM-HAND-PARTNER");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Clause C: [Security] (inherited) Tai Kamiya / Matt Ishida
// ═══════════════════════════════════════════════════════════════════════════

/// Smoke test: firing the Security clause on a placed BT17-095 with no
/// eligible Tai/Matt cards anywhere completes without panic.
#[test]
fn bt17_095_security_no_panic_with_empty_hand_and_trash() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_095_YAML)
        .expect("BT17-095 YAML parses")
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let field_handle = runner.place_on_field(0, "BT17-095", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::Permanent(field_handle),
    );
    runner.game.drain_effect_queue();

    // Drain any selections — the workaround installs a zone choice prompt.
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 30 {
        let player = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        let _ = runner.game.resolve_selection(player, action);
        runner.game.drain_effect_queue();
        steps += 1;
    }
    // Primary assertion: no panic.
}

/// Positive: when defender's Security has BT17-095 and they have a Tai Kamiya
/// Tamer in their trash, the attack flow resolves the security and the printed
/// "Then, add this card to the hand" lands BT17-095 in the defender's hand.
///
/// This test exercises the `add_this_option_to_hand` tail and the full
/// Security flow end-to-end. It does NOT positively assert that the Tamer was
/// played from trash — that requires walking each prompt down a specific
/// branch, which `auto_resolve` may resolve in either order. The primary
/// assertion is on the post-state shape (BT17-095 in defender's hand).
#[test]
fn bt17_095_security_adds_card_to_hand_after_play() {
    let mut attacker = make_filler("BT17095-ATK");
    attacker.dp = Some(6000);
    let tamer = make_named_tamer("BT17095-TAI", "Tai Kamiya");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_095_YAML)
        .expect("BT17-095 YAML parses")
        .add_card(attacker.clone())
        .add_card(tamer.clone())
        .memory(10)
        .deck(0, &["BT17095-TAI"; 2])
        .deck(1, &["BT17095-TAI"; 2])
        .security(1, &["BT17-095"])
        .start();

    // Seed defender's trash with the Tai Kamiya Tamer.
    let trash_seed = runner.game.players[1]
        .deck
        .pop()
        .expect("Tamer seed in deck");
    runner.game.players[1].trash.push(trash_seed);

    let attacker = runner.place_on_field(0, "BT17095-ATK", Some(0));
    assert_eq!(runner.hand_size(1), 0, "precondition: defender hand empty");

    let _ = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    assert_eq!(runner.security_count(1), 0, "BT17-095 left security");
    assert_eq!(
        runner.hand_size(1),
        1,
        "BT17-095 must have been routed to defender's hand by add_this_option_to_hand"
    );
    let hand_id = runner.game.players[1].hand[0]
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(
        hand_id, "BT17-095",
        "the card landed in defender's hand must be BT17-095 itself"
    );
}

/// G-DSL-UNION-PLAY-FREE: same as Clause A — the security clause cannot
/// auto-collapse the From-hand / From-trash branch. Disabled until the gap
/// closes.
#[test]
#[ignore = "pending: G-DSL-UNION-PLAY-FREE — select_union_zone binding type"]
fn bt17_095_security_auto_collapses_zone_when_only_trash_eligible() {
    unimplemented!("blocked on G-DSL-UNION-PLAY-FREE");
}
