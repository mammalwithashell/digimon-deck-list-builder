//! Declined optional replacements must not consume the event.
//!
//! Repro (found authoring EX12-036-effect1): EX12-036 Ryugumon prints
//! `<Barrier>`, `<Evade>`, `<Decode (...)>`. When an opponent's effect
//! deletes it (EX12-047 On Play), the engine's deletion batch fires
//! `WhenWouldLeaveBattleArea` (Decode) first, then `WhenWouldBeDeleted`
//! (Evade; Barrier's `replacement_condition` is battle-only per §16-24-1 so
//! it doesn't apply to effect deletions). Declining Decode's prompt resolved
//! the deletion straight to trash — Evade's yes/no was never offered.
//!
//! Rules basis (general_rule.pdf Ver.3.6):
//! - §15-8-5-3: "Immediate-type effects will only trigger simultaneously
//!   with other immediate-type effects." Evade (§16-21-2, would be deleted)
//!   and Decode (§16-35-2, would leave other than by battle) both trigger
//!   on the same effect-deletion.
//! - §15-8-5-4: "Each immediate-type effect can be activated one at a time
//!   until the cause that first interrupted the immediate-type effect is
//!   resolved." Only "the already activated" effect can't activate again —
//!   an effect that was offered and DECLINED was never activated, so the
//!   remaining simultaneous immediate effects must still get their window.
//! - §16-35-1 Decode plays 1 specified card from the leaver's digivolution
//!   cards; it does NOT prevent the leave — so even an ACCEPTED Decode
//!   leaves the deletion in progress and Evade must still be offered.
//!
//! DCGO cross-check (`DestroyPermanentsClass.Destroy()`, CardController.cs):
//! both cut-in timings (`WhenPermanentWouldBeDeleted` then `WhenRemoveField`)
//! are stacked into ONE `autoProcessing_CutIn` queue and drained by a single
//! `TriggeredSkillProcess` → `MultipleSkills.ActivateMultipleSkills` loop.
//! A declined optional is simply removed from the stack and the loop
//! continues with the remaining stacked effects; only effects that actually
//! processed enter `SkillInfos_used` (via `SetOnProcessCallbuck`). Declined
//! never blocks the others.
//!
//! The engine bug was in `replacement.rs::try_replace_impl`'s
//! commit-continuation guard: during `in_replacement_commit` it blocked ANY
//! prior fire for the subject regardless of timing — conflating a
//! replacement that FIRED (accept + outcome committed, where the broadened
//! block is the documented double-prompt protection, see
//! `dispatcher_guard.rs::commit_continuation_broadening_blocks_different_timing_v1_known_limitation`)
//! with one that was OFFERED AND DECLINED (outcome `None` — the event
//! proceeds unchanged, so the not-yet-offered windows must still dispatch).
//! The deletion decline path additionally bypassed the remaining window
//! stages entirely (`commit_permanent_deletion_no_replace`).

use digimon_engine::action::space::{encode_source_select, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

/// Inline DSL mirror of EX12-036's replacement-relevant clauses: a granted
/// `<Evade>` plus the per-card `<Decode>` replacement at
/// `when_would_leave_battle_area` (optional, non-battle causes only).
/// `<Barrier>` is intentionally omitted: under an effect-deletion cause its
/// battle-only `replacement_condition` (§16-24-1) filters it out anyway, and
/// keeping Evade as the sole `WhenWouldBeDeleted` candidate mirrors the
/// EX12-047-deletes-EX12-036 repro exactly.
const DECODE_EVADE_CARRIER: &str = r#"
card: DSL-DECODE-EVADE
name: Decode Evade Carrier
kind: digimon
level: 6
color: [blue]
cost: 12
dp: 12000
effects:
  - kind: grant_keyword
    keyword: Evade
    summary: "<Evade>"
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - none_of:
            - replacement_cause: battle
    summary: "<Decode (Lv.5 or lower TB)> play 1 matching source when this would leave outside battle"
    process:
      - select_material:
          of_permanent: replacement_subject
          bind_as: decode_source
          optional: true
          filter:
            all_of:
              - kind: digimon
              - level_lte: 5
              - trait_has: TB
          prompt: "Decode: play 1 level 5 or lower [TB] source"
      - play_from_materials:
          target: replacement_subject
          source_index: decode_source
          cost_delta: free
"#;

/// Level-5 [TB] Digimon used as the carrier's digivolution source — the
/// Decode target.
const TB_SOURCE: &str = r#"
card: DSL-TB-SOURCE
name: TB Source
kind: digimon
level: 5
color: [blue]
cost: 8
dp: 7000
traits: [TB]
"#;

fn start_carrier_over_tb_source() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(DECODE_EVADE_CARRIER)
        .expect("carrier DSL compiles")
        .from_dsl_yaml(TB_SOURCE)
        .expect("source DSL compiles")
        .start();
    let carrier = r.place_stack(0, &["DSL-TB-SOURCE", "DSL-DECODE-EVADE"]);
    (r, carrier)
}

fn expect_decode_prompt(r: &DebugRunner) {
    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("effect deletion must first park the Decode (would-leave) accept dialog");
    assert_eq!(sel.kind, SelectionKind::Replacement);
    assert_eq!(sel.selecting_player, 0);
    assert!(
        sel.prompt.contains("when_would_leave_battle_area"),
        "first prompt is the Decode leave-replacement (stage 1); got: {}",
        sel.prompt
    );
}

fn expect_evade_prompt(r: &DebugRunner, context: &str) {
    let sel = r.game.pending_selection.as_ref().unwrap_or_else(|| {
        panic!(
            "{context}: <Evade>'s WhenWouldBeDeleted window must still be \
             offered (§15-8-5-4 — declining/using another simultaneous \
             immediate effect does not consume the event)"
        )
    });
    assert_eq!(sel.kind, SelectionKind::Replacement, "{context}");
    assert_eq!(sel.selecting_player, 0, "{context}");
    assert!(
        sel.prompt.contains("<Evade>"),
        "{context}: expected the <Evade> accept dialog, got: {}",
        sel.prompt
    );
    assert!(sel.valid_action_ids.contains(&REPLACEMENT_ACCEPT));
}

/// THE repro — declined `<Decode>` must not eat `<Evade>`.
///
/// Decline Decode's would-leave dialog, then accept Evade: the carrier
/// survives suspended, nothing is trashed.
#[test]
fn declined_decode_still_offers_evade_then_evade_saves() {
    let (mut r, carrier) = start_carrier_over_tb_source();

    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    expect_decode_prompt(&r);
    r.game.resolve_selection(0, PASS).expect("decline Decode");

    expect_evade_prompt(&r, "after declining Decode");
    assert_eq!(
        r.battle_area_size(0),
        1,
        "carrier must still be on field while Evade's window is open"
    );

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Evade");

    assert_eq!(
        r.battle_area_size(0),
        1,
        "Evade prevents the deletion — carrier survives"
    );
    let perm = &r.game.player(0).battle_area[carrier.index as usize];
    assert!(perm.is_suspended, "Evade's cost suspends the carrier");
    assert_eq!(
        perm.card_sources.len(),
        2,
        "declined Decode extracted nothing — the [TB] source stays stacked"
    );
    assert!(
        r.game.player(0).trash.is_empty(),
        "nothing goes to trash when Evade prevents the deletion"
    );
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.replacement_depth, 0);
}

/// Declining BOTH windows lets the original deletion proceed exactly once.
#[test]
fn declining_decode_then_evade_deletes_exactly_once() {
    let (mut r, carrier) = start_carrier_over_tb_source();

    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    expect_decode_prompt(&r);
    r.game.resolve_selection(0, PASS).expect("decline Decode");

    expect_evade_prompt(&r, "after declining Decode");
    r.game.resolve_selection(0, PASS).expect("decline Evade");

    assert_eq!(
        r.battle_area_size(0),
        0,
        "with both replacements declined the deletion proceeds"
    );
    let trash_ids: Vec<String> = r
        .game
        .player(0)
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    assert_eq!(
        trash_ids.len(),
        2,
        "carrier top card + its source trash exactly once; got {trash_ids:?}"
    );
    assert!(trash_ids.contains(&"DSL-DECODE-EVADE".to_string()));
    assert!(trash_ids.contains(&"DSL-TB-SOURCE".to_string()));
    assert!(
        r.game.pending_selection.is_none(),
        "no further prompt — the declined windows must not re-offer"
    );
    assert_eq!(r.game.replacement_depth, 0);
}

/// Accepting Decode's dialog but then declining its nested material pick is
/// still a non-replacement (outcome `None`) — Evade must still be offered.
#[test]
fn accepted_decode_with_declined_material_pick_still_offers_evade() {
    let (mut r, carrier) = start_carrier_over_tb_source();

    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    expect_decode_prompt(&r);
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Decode");

    // Nested cancelable material pick parks.
    {
        let sel = r
            .game
            .pending_selection
            .as_ref()
            .expect("accepted Decode parks its material pick");
        assert!(
            sel.is_optional,
            "the Decode material pick is cancelable (optional select_material)"
        );
    }
    r.game
        .resolve_selection(0, PASS)
        .expect("decline the Decode material pick");

    expect_evade_prompt(&r, "after declining Decode's material pick");
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Evade");

    assert_eq!(r.battle_area_size(0), 1, "Evade saves the carrier");
    assert!(r.game.player(0).battle_area[carrier.index as usize].is_suspended);
    assert!(r.game.player(0).trash.is_empty());
    assert_eq!(r.game.replacement_depth, 0);
}

/// §16-35 Decode does not prevent the leave: even a fully USED Decode (source
/// extracted and played) leaves the deletion in progress, so Evade's window
/// still opens (§15-8-5-4 — the deletion cause is not yet resolved). Declining
/// Evade then trashes the carrier while the Decode-played Digimon stays.
#[test]
fn used_decode_still_offers_evade_and_decline_trashes_carrier() {
    let (mut r, carrier) = start_carrier_over_tb_source();

    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    expect_decode_prompt(&r);
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Decode");

    // Pick the [TB] source (permanent 0, source index 0) — Decode plays it.
    let pick = encode_source_select(carrier.index as u16, 0).expect("encode source pick");
    {
        let sel = r
            .game
            .pending_selection
            .as_ref()
            .expect("accepted Decode parks its material pick");
        assert!(
            sel.valid_action_ids.contains(&pick),
            "the stacked [TB] source is offered; got {:?}",
            sel.valid_action_ids
        );
    }
    r.game
        .resolve_selection(0, pick)
        .expect("pick the [TB] source");

    // The source is played as its own permanent...
    assert_eq!(
        r.battle_area_size(0),
        2,
        "Decode plays the picked source as a new permanent"
    );

    // ...and the deletion is still unresolved: Evade's window must open.
    expect_evade_prompt(&r, "after a used (non-preventing) Decode");
    r.game.resolve_selection(0, PASS).expect("decline Evade");

    assert_eq!(
        r.battle_area_size(0),
        1,
        "carrier trashed; the Decode-played Digimon remains"
    );
    let survivor = &r.game.player(0).battle_area[0];
    assert_eq!(
        survivor.top_card().card_id(&r.game.card_data),
        "DSL-TB-SOURCE"
    );
    let trash_ids: Vec<String> = r
        .game
        .player(0)
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    assert_eq!(
        trash_ids,
        vec!["DSL-DECODE-EVADE".to_string()],
        "only the carrier's top card is trashed (its source was extracted by Decode)"
    );
    assert_eq!(r.game.replacement_depth, 0);
}
