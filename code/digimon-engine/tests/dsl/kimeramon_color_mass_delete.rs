//! Round-2 engine gap-closure — EX9-074 Kimeramon substrate (Gaps 1 + 2).
//!
//! Driver: EX9-074 Kimeramon (NOT authored here — these tests exercise the
//! *substrate* the card needs). DCGO `EX9_074.cs`.
//!
//! Gap 1 — `G-DSL-COLOR-MATCHES-OWN-SOURCE-STACK`: a permanent-subject predicate
//! leaf `color_matches_own_source_stack: { of: self }` that matches an opponent
//! Digimon sharing >=1 color with the CARRIER's NON-FLIPPED digivolution-source
//! color set (DCGO `DigivolutionCards.Filter(!IsFlipped).SelectMany(CardColors)
//! .Distinct()`).
//!
//! Gap 2 — `G-DSL-DELETE-ONE-PER-DISTINCT-OPPONENT-COLOR`: `delete_one_per_opponent_color`
//! iterates the 7 colors; for each color present among opponent Digimon it drives
//! a MANDATORY pick of 1 not-yet-chosen opponent Digimon of that color, accumulates
//! the picks, and batch-deletes after every color's pick resolves.
//!
//! Both branches gate on DistinctColorsCount of the carrier's non-flipped sources
//! (<=5 = single-target Branch A; >=6 = per-color Branch B). Proven with an inline
//! fixture that flips the branch by source-color count.

use digimon_dsl::compiled::{CompiledPredicate, CompiledSourceStackScope};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use digimon_engine::effect_context::EffectReadContext;
use digimon_engine::enums::{CardColor, CardKind, EffectSourceKind};
use digimon_engine::permanent::{Permanent, PermanentHandle};

/// Build a single-color Digimon card.
fn digimon(id: &str, name: &str, colors: &[CardColor]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = colors.to_vec();
    c.dp = Some(5000);
    c
}

/// Push a Digimon permanent (single card, no sources) onto `player`'s field and
/// return its handle.
fn place_digimon(runner: &mut DebugRunner, player: u8, id: &str) -> PermanentHandle {
    let g = runner.game_mut();
    let turn = g.turn_count;
    let idx = g.card_data.iter().position(|c| c.card_id == id).unwrap();
    let src = CardSource::new(idx, player, g.next_card_index());
    let perm = Permanent::new(src, turn);
    g.players[player as usize].battle_area.push(perm);
    PermanentHandle {
        player,
        index: (g.players[player as usize].battle_area.len() - 1) as u8,
    }
}

/// Build the carrier (player 0) permanent with a top card + the given
/// digivolution sources. `sources` is `(card_id, face_down)`; sources are pushed
/// bottom-first (the top card is pushed last so it is the effective top).
fn place_carrier_with_sources(
    runner: &mut DebugRunner,
    top_id: &str,
    sources: &[(&str, bool)],
) -> PermanentHandle {
    let g = runner.game_mut();
    let turn = g.turn_count;
    // Bottom source is the Permanent's base card.
    let (bottom_id, bottom_fd) = sources[0];
    let bottom_idx = g.card_data.iter().position(|c| c.card_id == bottom_id).unwrap();
    let mut bottom = CardSource::new(bottom_idx, 0, g.next_card_index());
    bottom.face_down = bottom_fd;
    let mut perm = Permanent::new(bottom, turn);
    // Remaining sources in the middle.
    for (sid, fd) in &sources[1..] {
        let sidx = g.card_data.iter().position(|c| c.card_id == *sid).unwrap();
        let mut s = CardSource::new(sidx, 0, g.next_card_index());
        s.face_down = *fd;
        perm.card_sources.push(s);
    }
    // Top card.
    let top_idx = g.card_data.iter().position(|c| c.card_id == top_id).unwrap();
    let top = CardSource::new(top_idx, 0, g.next_card_index());
    perm.card_sources.push(top);
    g.players[0].battle_area.push(perm);
    PermanentHandle {
        player: 0,
        index: (g.players[0].battle_area.len() - 1) as u8,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Gap 1 — color_matches_own_source_stack: { of: self }
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn color_matches_own_source_stack_matches_shared_non_flipped_source_color() {
    // Carrier (player 0) top = White Kimeramon; sources = [Red base, Blue mid].
    // Opponent (player 1) has a Blue Digimon (shares Blue with a source) and a
    // Green Digimon (shares nothing). The leaf must match the Blue one and reject
    // the Green one.
    let mut runner = DebugRunner::builder()
        .add_card(digimon("KIMERA", "Kimeramon", &[CardColor::White]))
        .add_card(digimon("RED-BASE", "Red Base", &[CardColor::Red]))
        .add_card(digimon("BLUE-MID", "Blue Mid", &[CardColor::Blue]))
        .add_card(digimon("OPP-BLUE", "Opp Blue", &[CardColor::Blue]))
        .add_card(digimon("OPP-GREEN", "Opp Green", &[CardColor::Green]))
        .add_card(make_test_card("SRC", "Src"))
        .hand(0, &["SRC"])
        .build();

    let carrier =
        place_carrier_with_sources(&mut runner, "KIMERA", &[("RED-BASE", false), ("BLUE-MID", false)]);
    let opp_blue = place_digimon(&mut runner, 1, "OPP-BLUE");
    let opp_green = place_digimon(&mut runner, 1, "OPP-GREEN");

    let src = runner.game.players[0].hand[0].handle();
    // source_permanent = the carrier — the leaf reads the carrier's sources.
    let rctx = EffectReadContext::new_with_source_kind(
        &runner.game,
        src,
        Some(carrier),
        EffectSourceKind::Digimon,
        0,
    );

    let pred = CompiledPredicate {
        color_matches_own_source_stack: Some(CompiledSourceStackScope::SelfCarrier),
        ..Default::default()
    };

    assert!(
        eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::Permanent(opp_blue), None),
        "opponent Blue Digimon shares Blue with a non-flipped source -> match"
    );
    assert!(
        !eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::Permanent(opp_green), None),
        "opponent Green Digimon shares no source color -> no match"
    );
}

#[test]
fn color_matches_own_source_stack_ignores_flipped_sources_and_top_card() {
    // Carrier top = White; sources = [Red (face-up), Blue (FACE-DOWN)]. The
    // non-flipped source color set is {Red} only (Blue is flipped; the White top
    // card is NOT a source). So an opponent Blue Digimon must NOT match (Blue is
    // flipped), an opponent White Digimon must NOT match (top card is excluded),
    // and an opponent Red Digimon MUST match.
    let mut runner = DebugRunner::builder()
        .add_card(digimon("KIMERA", "Kimeramon", &[CardColor::White]))
        .add_card(digimon("RED-BASE", "Red Base", &[CardColor::Red]))
        .add_card(digimon("BLUE-MID", "Blue Mid", &[CardColor::Blue]))
        .add_card(digimon("OPP-RED", "Opp Red", &[CardColor::Red]))
        .add_card(digimon("OPP-BLUE", "Opp Blue", &[CardColor::Blue]))
        .add_card(digimon("OPP-WHITE", "Opp White", &[CardColor::White]))
        .add_card(make_test_card("SRC", "Src"))
        .hand(0, &["SRC"])
        .build();

    let carrier = place_carrier_with_sources(
        &mut runner,
        "KIMERA",
        &[("RED-BASE", false), ("BLUE-MID", true)], // Blue source is flipped
    );
    let opp_red = place_digimon(&mut runner, 1, "OPP-RED");
    let opp_blue = place_digimon(&mut runner, 1, "OPP-BLUE");
    let opp_white = place_digimon(&mut runner, 1, "OPP-WHITE");

    let src = runner.game.players[0].hand[0].handle();
    let rctx = EffectReadContext::new_with_source_kind(
        &runner.game,
        src,
        Some(carrier),
        EffectSourceKind::Digimon,
        0,
    );
    let pred = CompiledPredicate {
        color_matches_own_source_stack: Some(CompiledSourceStackScope::SelfCarrier),
        ..Default::default()
    };

    assert!(
        eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::Permanent(opp_red), None),
        "Red source (face-up) -> Red opponent matches"
    );
    assert!(
        !eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::Permanent(opp_blue), None),
        "Blue source is FLIPPED -> Blue opponent must NOT match"
    );
    assert!(
        !eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::Permanent(opp_white), None),
        "White is the TOP card, not a source -> White opponent must NOT match"
    );
}

#[test]
fn color_matches_own_source_stack_no_sources_matches_nothing() {
    // A carrier with no digivolution sources (only a base card) has an empty
    // non-flipped source color set -> nothing matches, mirroring the empty-set
    // short-circuit in the sibling color leaves.
    let mut runner = DebugRunner::builder()
        .add_card(digimon("KIMERA", "Kimeramon", &[CardColor::White]))
        .add_card(digimon("OPP-WHITE", "Opp White", &[CardColor::White]))
        .add_card(make_test_card("SRC", "Src"))
        .hand(0, &["SRC"])
        .build();
    let carrier = place_digimon(&mut runner, 0, "KIMERA");
    let opp_white = place_digimon(&mut runner, 1, "OPP-WHITE");
    let src = runner.game.players[0].hand[0].handle();
    let rctx = EffectReadContext::new_with_source_kind(
        &runner.game,
        src,
        Some(carrier),
        EffectSourceKind::Digimon,
        0,
    );
    let pred = CompiledPredicate {
        color_matches_own_source_stack: Some(CompiledSourceStackScope::SelfCarrier),
        ..Default::default()
    };
    assert!(
        !eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::Permanent(opp_white), None),
        "carrier with no sources -> empty color set -> no match even for same top color"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Gap 2 — delete_one_per_opponent_color (per-color mandatory pick + batch delete)
// ─────────────────────────────────────────────────────────────────────────────

use digimon_engine::action::space::{decode_attack, encode_attack, PASS};
use digimon_engine::selection::SelectionKind;

/// Substrate-vehicle grantor: an On-Play `delete_one_per_opponent_color`
/// (EX9-074 Branch B reduced to the primitive under test).
const PER_COLOR_DELETE_YAML: &str = r#"
card: TEST-PER-COLOR-DELETE
name: Per Color Delete
kind: digimon
color: [white]
level: 6
cost: 12
dp: 12000
effects:
  - when: on_play
    process:
      - delete_one_per_opponent_color:
          prompt: "Delete 1 {color} Digimon"
"#;

fn per_color_setup() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(PER_COLOR_DELETE_YAML)
        .expect("per-color-delete YAML must parse + compile")
        .add_card(digimon("O-RED", "Opp Red", &[CardColor::Red]))
        .add_card(digimon("O-BLUE", "Opp Blue", &[CardColor::Blue]))
        .add_card(digimon("O-BLUE2", "Opp Blue 2", &[CardColor::Blue]))
        .add_card(digimon("O-GREEN", "Opp Green", &[CardColor::Green]))
        .add_card(digimon("O-RB", "Opp RedBlue", &[CardColor::Red, CardColor::Blue]))
        .add_card(make_test_card("FILL2", "Filler2"))
        .hand(0, &["TEST-PER-COLOR-DELETE"])
        .deck(0, &["FILL2", "FILL2", "FILL2"])
        .deck(1, &["FILL2", "FILL2", "FILL2"])
        .memory(20)
        .start()
}

/// Resolve the currently-pending mandatory per-color pick by choosing the
/// opponent battle-area permanent at `opp_index`. Asserts it is a mandatory
/// OppField pick (no PASS).
fn resolve_pick(runner: &mut DebugRunner, opp_index: u8) {
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("a per-color pick must be pending");
    assert_eq!(
        pending.kind,
        SelectionKind::OppField,
        "per-color pick targets the opponent's field"
    );
    assert!(
        !pending.is_optional,
        "each per-color pick is MANDATORY (DCGO canNoSelect: false)"
    );
    assert!(
        !pending.valid_action_ids.contains(&PASS),
        "mandatory pick must not offer PASS"
    );
    let action = encode_attack(1, opp_index as u16);
    assert!(
        pending.valid_action_ids.contains(&action),
        "opp index {opp_index} must be a legal pick; valid={:?}",
        pending.valid_action_ids
    );
    runner
        .game
        .resolve_selection(pending.selecting_player, action)
        .expect("per-color pick resolves");
}

#[test]
fn per_color_delete_one_per_distinct_opponent_color_batch_deletes() {
    // Opponent field: Red, Blue, Blue2, Green (4 Digimon, 3 distinct colors).
    // The step must ask for ONE pick per color present (Red, Blue, Green) — for
    // Blue there are two candidates and only ONE is deleted. So exactly 3 get
    // deleted; the un-picked Blue survives.
    let mut runner = per_color_setup();
    let _red = runner.place_on_field(1, "O-RED", None); // idx 0
    let _blue = runner.place_on_field(1, "O-BLUE", None); // idx 1
    let _blue2 = runner.place_on_field(1, "O-BLUE2", None); // idx 2
    let _green = runner.place_on_field(1, "O-GREEN", None); // idx 3
    assert_eq!(runner.game.player(1).battle_area.len(), 4);

    runner.play(0, 0).expect("play per-color-delete");

    // Color order is Red, Blue, Yellow, Green, White, Black, Purple.
    // Red: only O-RED (idx 0).
    resolve_pick(&mut runner, 0);
    // Blue: pick O-BLUE (idx 1); O-BLUE2 (idx 2) must survive.
    resolve_pick(&mut runner, 1);
    // Green: only O-GREEN. Its index may have shifted if earlier picks were
    // resolved before delete — but deletes are BATCHED at the terminal, so the
    // field is unchanged until every pick resolves; indices are stable here.
    resolve_pick(&mut runner, 3);

    // No more picks — the batch delete fires.
    assert!(
        runner.game.pending_selection.is_none(),
        "all colors resolved → batch delete, no further pending selection"
    );
    // Exactly 3 deleted (Red, one Blue, Green); the un-picked Blue2 survives.
    let survivors: Vec<String> = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(
        survivors,
        vec!["O-BLUE2".to_string()],
        "only the un-picked Blue Digimon survives; got {survivors:?}"
    );
}

#[test]
fn per_color_delete_skips_colors_with_no_target_and_excludes_picked() {
    // Opponent: one Red, one RedBlue (Red+Blue two-color). Colors present:
    // Red (2 candidates: O-RED, O-RB) and Blue (1 candidate: O-RB).
    // Red pick: choose O-RB (the two-color one). Blue then has NO remaining
    // candidate (O-RB already picked) → Blue is skipped. Only O-RB deleted; the
    // colors Yellow/Green/White/Black/Purple are skipped (no target).
    let mut runner = per_color_setup();
    let _red = runner.place_on_field(1, "O-RED", None); // idx 0
    let _rb = runner.place_on_field(1, "O-RB", None); // idx 1 (Red+Blue)
    assert_eq!(runner.game.player(1).battle_area.len(), 2);

    runner.play(0, 0).expect("play per-color-delete");

    // Red pick: choose the two-color O-RB (idx 1).
    resolve_pick(&mut runner, 1);
    // Blue would be next, but O-RB is already picked and O-RED is not Blue → Blue
    // has no candidate and is SKIPPED. No further pick surfaces.
    assert!(
        runner.game.pending_selection.is_none(),
        "Blue has no un-picked candidate → skipped; batch delete fires with just O-RB"
    );
    let survivors: Vec<String> = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(
        survivors,
        vec!["O-RED".to_string()],
        "the un-picked Red Digimon survives; the two-color pick counts once; got {survivors:?}"
    );
}

#[test]
fn per_color_delete_no_opponent_digimon_is_a_noop() {
    let mut runner = per_color_setup();
    // No opponent Digimon on the field.
    runner.play(0, 0).expect("play per-color-delete");
    assert!(
        runner.game.pending_selection.is_none(),
        "no opponent Digimon → no color has a target → no pick, no crash"
    );
    assert!(runner.game.player(1).battle_area.is_empty());
}

#[test]
fn per_color_delete_every_pick_surfaces_in_action_space() {
    // Two colors present (Red, Green) → exactly two mandatory picks surface, each
    // as an OppField selection with a decodable attack-encoded action.
    let mut runner = per_color_setup();
    let _red = runner.place_on_field(1, "O-RED", None);
    let _green = runner.place_on_field(1, "O-GREEN", None);
    runner.play(0, 0).expect("play per-color-delete");

    // First pick (Red).
    let pending = runner.game.pending_selection.as_ref().expect("first pick");
    assert_eq!(pending.kind, SelectionKind::OppField);
    let (p, _) = decode_attack(pending.valid_action_ids[0]);
    assert_eq!(p as u8, 1, "pick action decodes to an opponent-field target");
    resolve_pick(&mut runner, 0);

    // Second pick (Green).
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("second pick surfaces after the first resolves");
    assert_eq!(pending.kind, SelectionKind::OppField);
    resolve_pick(&mut runner, 1);

    assert!(runner.game.pending_selection.is_none());
    assert!(
        runner.game.player(1).battle_area.is_empty(),
        "both single-color Digimon deleted"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Branch switch — DistinctColorsCount(carrier sources) <=5 vs >=6 (inline fixture)
// ─────────────────────────────────────────────────────────────────────────────

/// Prove the branch gate: the shared extraction
/// `distinct_non_flipped_source_color_count` yields the DistinctColorsCount the
/// card switches on (<=5 = single-target Branch A; >=6 = per-color Branch B).
/// This is a direct test of the shared extraction the two branches gate on.
#[test]
fn branch_gate_distinct_source_color_count_switches_at_six() {
    use digimon_engine::dsl_cards::formula_eval::distinct_non_flipped_source_color_count;

    // Build a carrier whose sources cover exactly 5 distinct colors (Branch A).
    let mut runner = DebugRunner::builder()
        .add_card(digimon("TOP", "Top", &[CardColor::White]))
        .add_card(digimon("S-R", "SrcR", &[CardColor::Red]))
        .add_card(digimon("S-B", "SrcB", &[CardColor::Blue]))
        .add_card(digimon("S-Y", "SrcY", &[CardColor::Yellow]))
        .add_card(digimon("S-G", "SrcG", &[CardColor::Green]))
        .add_card(digimon("S-K", "SrcK", &[CardColor::Black]))
        .add_card(digimon("S-P", "SrcP", &[CardColor::Purple]))
        .add_card(make_test_card("SRC", "Src"))
        .hand(0, &["SRC"])
        .build();

    // 5 distinct source colors: Red, Blue, Yellow, Green, Black (top White excluded).
    let five = place_carrier_with_sources(
        &mut runner,
        "TOP",
        &[
            ("S-R", false),
            ("S-B", false),
            ("S-Y", false),
            ("S-G", false),
            ("S-K", false),
        ],
    );
    {
        let perm = &runner.game.player(0).battle_area[five.index as usize];
        assert_eq!(
            distinct_non_flipped_source_color_count(perm, &runner.game.card_data),
            5,
            "5 distinct non-flipped source colors → Branch A (<=5)"
        );
    }

    // Now build a 6-distinct-color carrier (Branch B). Add Purple to the sources.
    let six = place_carrier_with_sources(
        &mut runner,
        "TOP",
        &[
            ("S-R", false),
            ("S-B", false),
            ("S-Y", false),
            ("S-G", false),
            ("S-K", false),
            ("S-P", false),
        ],
    );
    {
        let perm = &runner.game.player(0).battle_area[six.index as usize];
        let n = distinct_non_flipped_source_color_count(perm, &runner.game.card_data);
        assert_eq!(n, 6, "6 distinct non-flipped source colors → Branch B (>=6)");
        assert!(n >= 6, "the >=6 branch predicate holds");
    }

    // Flipping (face-down) a source removes it from the count — proving the
    // branch gate shares the SAME non-flipped extraction as the Gap-1 leaf.
    let six_with_flip = place_carrier_with_sources(
        &mut runner,
        "TOP",
        &[
            ("S-R", false),
            ("S-B", false),
            ("S-Y", false),
            ("S-G", false),
            ("S-K", false),
            ("S-P", true), // Purple flipped face-down → excluded
        ],
    );
    let perm = &runner.game.player(0).battle_area[six_with_flip.index as usize];
    assert_eq!(
        distinct_non_flipped_source_color_count(perm, &runner.game.card_data),
        5,
        "a flipped Purple source drops the count to 5 → back to Branch A"
    );
}

/// Rule 28: the per-color-delete selection is CLONE-SAFE — a forked `Game`
/// mid-selection resolves via the resumable VM (`PerColorDeleteStep` data frame),
/// never the panic-stub closure. Clone after the FIRST pick parks, resolve the
/// remaining picks on the clone, and confirm the batch delete lands identically.
#[test]
fn per_color_delete_selection_is_clone_safe() {
    let mut runner = per_color_setup();
    runner.place_on_field(1, "O-RED", None); // idx 0
    runner.place_on_field(1, "O-GREEN", None); // idx 1
    runner.play(0, 0).expect("play per-color-delete");

    // First mandatory pick (Red) is parked with a resume frame.
    assert!(runner.game.pending_selection.is_some());
    assert!(
        runner.game.pending_selection_resume.is_some(),
        "the parked per-color pick must carry a resumable-VM stack (clone-safe data)"
    );

    // Clone the Game mid-selection; the closure becomes a panic-stub, the frame
    // carries the data. Resolve BOTH remaining picks on the CLONE.
    let mut cloned = runner.game.clone();
    // Red pick (idx 0) on the clone.
    cloned
        .resolve_selection(0, encode_attack(1, 0))
        .expect("clone: Red pick resolves via the data VM (no panic-stub)");
    // Green pick (idx 1) surfaces on the clone.
    assert!(cloned.pending_selection.is_some());
    cloned
        .resolve_selection(0, encode_attack(1, 1))
        .expect("clone: Green pick resolves via the data VM");
    // Both deleted on the clone.
    assert!(
        cloned.player(1).battle_area.is_empty(),
        "the batch delete landed on the CLONE (both opponent Digimon deleted)"
    );
    assert!(cloned.pending_selection.is_none());
}

// ─────────────────────────────────────────────────────────────────────────────
// Gap 3 — `own_source_stack_color_count_gte` (the YAML-reachable branch gate)
// ─────────────────────────────────────────────────────────────────────────────

/// The no-subject predicate leaf backing the EX9-074 branch discriminant:
/// `own_source_stack_color_count_gte: N` holds iff the CARRIER's non-flipped
/// source color count (the shared `distinct_non_flipped_source_color_count`
/// extraction proven above) is >= N. G-DSL-OWN-SOURCE-STACK-COLOR-COUNT-THRESHOLD.
#[test]
fn own_source_stack_color_count_gte_thresholds_on_non_flipped_source_colors() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("TOP", "Top", &[CardColor::White]))
        .add_card(digimon("S-R", "SrcR", &[CardColor::Red]))
        .add_card(digimon("S-B", "SrcB", &[CardColor::Blue]))
        .add_card(digimon("S-Y", "SrcY", &[CardColor::Yellow]))
        .add_card(digimon("S-G", "SrcG", &[CardColor::Green]))
        .add_card(digimon("S-K", "SrcK", &[CardColor::Black]))
        .add_card(digimon("S-P", "SrcP", &[CardColor::Purple]))
        .add_card(make_test_card("SRC", "Src"))
        .hand(0, &["SRC"])
        .build();

    // 6 distinct non-flipped source colors, with a 7th source FLIPPED — the
    // flipped Purple must not count (shared extraction with Gap 1).
    let carrier = place_carrier_with_sources(
        &mut runner,
        "TOP",
        &[
            ("S-R", false),
            ("S-B", false),
            ("S-Y", false),
            ("S-G", false),
            ("S-K", false),
            ("S-P", true), // flipped → excluded → count = 5
        ],
    );
    let src = runner.game.players[0].hand[0].handle();
    let rctx = EffectReadContext::new_with_source_kind(
        &runner.game,
        src,
        Some(carrier),
        EffectSourceKind::Digimon,
        0,
    );

    let gte = |n: u8| CompiledPredicate {
        own_source_stack_color_count_gte: Some(n),
        ..Default::default()
    };
    assert!(
        eval_predicate_with_bindings(&gte(5), &rctx, PredicateSubject::None, None),
        "5 non-flipped distinct colors -> gte(5) holds"
    );
    assert!(
        !eval_predicate_with_bindings(&gte(6), &rctx, PredicateSubject::None, None),
        "the flipped Purple source is excluded -> gte(6) fails at 5 live colors"
    );

    // Un-flipped sibling carrier: 6 distinct colors → gte(6) holds.
    let carrier6 = place_carrier_with_sources(
        &mut runner,
        "TOP",
        &[
            ("S-R", false),
            ("S-B", false),
            ("S-Y", false),
            ("S-G", false),
            ("S-K", false),
            ("S-P", false),
        ],
    );
    let rctx6 = EffectReadContext::new_with_source_kind(
        &runner.game,
        src,
        Some(carrier6),
        EffectSourceKind::Digimon,
        0,
    );
    assert!(
        eval_predicate_with_bindings(&gte(6), &rctx6, PredicateSubject::None, None),
        "6 non-flipped distinct colors -> gte(6) holds (Branch B)"
    );
}

/// No carrier (`source_permanent = None`) → the count is 0 → any floor >= 1
/// fails rather than panicking.
#[test]
fn own_source_stack_color_count_gte_without_carrier_fails_closed() {
    let runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Src"))
        .hand(0, &["SRC"])
        .build();
    let src = runner.game.players[0].hand[0].handle();
    let rctx = EffectReadContext::new_with_source_kind(
        &runner.game,
        src,
        None,
        EffectSourceKind::Digimon,
        0,
    );
    let pred = CompiledPredicate {
        own_source_stack_color_count_gte: Some(1),
        ..Default::default()
    };
    assert!(
        !eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::None, None),
        "no carrier -> empty color set -> the floor fails closed"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Gap 3 — YAML-reachable end-to-end: the EX9-074 branch shape via `if:`
// ─────────────────────────────────────────────────────────────────────────────

/// The EXACT EX9-074 Part-2 shape, authored in YAML: the `if:` gate reads the
/// new leaf; Branch B (>=6) is `delete_one_per_opponent_color`, Branch A
/// (else) is a mandatory same-color single delete filtered by
/// `color_matches_own_source_stack`.
const BRANCH_GATE_YAML: &str = r#"
card: TEST-COLOR-BRANCH-GATE
name: Color Branch Gate
kind: digimon
color: [white]
level: 6
cost: 12
dp: 12000
effects:
  - when: on_play
    process:
      - if:
          condition: { own_source_stack_color_count_gte: 6 }
          then:
            - delete_one_per_opponent_color:
                prompt: "Delete 1 {color} Digimon"
          else:
            - select_opponent_permanent:
                bind_as: same_color_victim
                filter:
                  all_of:
                    - kind: digimon
                    - color_matches_own_source_stack: { of: self }
                prompt: "Delete 1 same-color Digimon"
            - delete_permanent: { target: same_color_victim }
"#;

fn branch_gate_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(BRANCH_GATE_YAML)
        .expect("branch-gate YAML parses + compiles")
        .add_card(digimon("S-R", "SrcR", &[CardColor::Red]))
        .add_card(digimon("S-B", "SrcB", &[CardColor::Blue]))
        .add_card(digimon("S-Y", "SrcY", &[CardColor::Yellow]))
        .add_card(digimon("S-G", "SrcG", &[CardColor::Green]))
        .add_card(digimon("S-K", "SrcK", &[CardColor::Black]))
        .add_card(digimon("S-P", "SrcP", &[CardColor::Purple]))
        .add_card(digimon("O-RED", "Opp Red", &[CardColor::Red]))
        .add_card(digimon("O-GREEN", "Opp Green", &[CardColor::Green]))
        .add_card(make_test_card("FILL3", "Filler3"))
        .deck(0, &["FILL3", "FILL3"])
        .deck(1, &["FILL3", "FILL3"])
        .memory(20)
        .start()
}

fn fire_on_play(runner: &mut DebugRunner, source: PermanentHandle) {
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::TriggerSource;
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

/// Below-6 (5 distinct source colors) → the ELSE branch: ONE mandatory pick,
/// only same-color opponent Digimon offered; the off-color one survives.
#[test]
fn yaml_branch_gate_below_six_takes_single_same_color_delete() {
    let mut runner = branch_gate_runner();
    // 5 distinct source colors: Red, Blue, Yellow, Black, Purple (no Green).
    let carrier = place_carrier_with_sources(
        &mut runner,
        "TEST-COLOR-BRANCH-GATE",
        &[
            ("S-R", false),
            ("S-B", false),
            ("S-Y", false),
            ("S-K", false),
            ("S-P", false),
        ],
    );
    let _red = place_digimon(&mut runner, 1, "O-RED"); // shares Red
    let _green = place_digimon(&mut runner, 1, "O-GREEN"); // shares nothing

    fire_on_play(&mut runner, carrier);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("Branch A installs a single mandatory same-color pick");
    assert!(!pending.is_optional, "Branch A's pick is mandatory");
    // An `OppField` selection encodes candidates as `encode_attack(0, slot)`
    // (`100 + slot`) — the side is carried by the kind (selection.rs).
    assert_eq!(
        pending.valid_action_ids,
        vec![encode_attack(0, 0)],
        "only the Red opponent Digimon (shares a source color) is offered; Green is not"
    );
    runner
        .game
        .resolve_selection(pending.selecting_player, encode_attack(0, 0))
        .expect("Branch A pick resolves");
    runner.game.drain_effect_queue();

    assert!(runner.game.pending_selection.is_none(), "one pick only");
    let survivors: Vec<String> = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(
        survivors,
        vec!["O-GREEN".to_string()],
        "Branch A deletes exactly the picked same-color Digimon"
    );
}

/// 6+ distinct source colors → the THEN branch: one mandatory pick per color
/// present among opponent Digimon, batch-deleted.
#[test]
fn yaml_branch_gate_six_or_more_takes_per_color_mass_delete() {
    let mut runner = branch_gate_runner();
    // 6 distinct source colors.
    let carrier = place_carrier_with_sources(
        &mut runner,
        "TEST-COLOR-BRANCH-GATE",
        &[
            ("S-R", false),
            ("S-B", false),
            ("S-Y", false),
            ("S-G", false),
            ("S-K", false),
            ("S-P", false),
        ],
    );
    place_digimon(&mut runner, 1, "O-RED"); // idx 0
    place_digimon(&mut runner, 1, "O-GREEN"); // idx 1

    fire_on_play(&mut runner, carrier);

    // Per-color picks: Red first, then Green (both mandatory).
    resolve_pick(&mut runner, 0);
    resolve_pick(&mut runner, 1);

    assert!(runner.game.pending_selection.is_none());
    assert!(
        runner.game.player(1).battle_area.is_empty(),
        "Branch B deletes one opponent Digimon per distinct color present"
    );
}

#[allow(dead_code)]
fn _unused(_c: CardHandle) {}
