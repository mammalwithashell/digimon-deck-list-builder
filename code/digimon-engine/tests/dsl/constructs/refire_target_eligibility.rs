//! `has_refireable_effect` (G-DSL-HAS-REFIREABLE-EFFECT): the permanent leaf that
//! offers only a Digimon whose top card carries an effect a refire could
//! activate.
//!
//! DCGO BT24_102.cs:93-98 (`CanSelectPermanentCondition`): Homeros's "activate 1
//! [On Play] or [When Digivolving] effect of 1 of your [Olympos XII] Digimon"
//! lets the player pick only a Digimon whose `EffectList(OnEnterFieldAnyone)`
//! holds a candidate; an all-ineligible board opens no prompt. The leaf names
//! the refire's timing (`on_play`, `when_digivolving`,
//! `on_play_or_when_digivolving`, the values `refire_effect` takes) and a timing
//! the Digimon lacks does not count: [When Attacking] / [On Deletion] effects are
//! not candidates, and a clause carrying both timings counts for either.
use digimon_engine::{
    debug_runner::DebugRunner,
    selection::{SelectionKind, TriggerSource},
    EffectTiming,
};

fn digimon(id: &str, when: &str) -> String {
    format!(
        "card: {id}\nname: {id}\nkind: digimon\nlevel: 5\ncolor: [red]\ncost: 5\ndp: 5000\neffects:\n  - when: {when}\n    summary: \"gain memory\"\n    process:\n      - gain_memory: 1\n"
    )
}

/// Offers the Digimon whose effects can be refired as ON PLAY.
const PICK_ON_PLAY: &str = r#"
card: PROBE-ON-PLAY
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_own_permanent:
          prompt: Pick a Digimon
          bind_as: pick
          optional: true
          filter:
            all_of:
              - kind: digimon
              - has_refireable_effect: on_play
"#;

/// ... as WHEN DIGIVOLVING.
const PICK_WHEN_DIGIVOLVING: &str = r#"
card: PROBE-WHEN-DIGIVOLVING
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_own_permanent:
          prompt: Pick a Digimon
          bind_as: pick
          optional: true
          filter:
            all_of:
              - kind: digimon
              - has_refireable_effect: when_digivolving
"#;

/// ... as either (Homeros's own timing).
const PICK_EITHER: &str = r#"
card: PROBE-EITHER
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_own_permanent:
          prompt: Pick a Digimon
          bind_as: pick
          optional: true
          filter:
            all_of:
              - kind: digimon
              - has_refireable_effect: on_play_or_when_digivolving
"#;

const BAD_TIMING: &str = r#"
card: PROBE-BAD
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_own_permanent:
          prompt: Pick a Digimon
          bind_as: pick
          filter: { has_refireable_effect: when_attacking }
"#;

fn offered(probe_yaml: &str, probe_id: &str, board: &[(&str, &str)]) -> Option<usize> {
    let mut b = DebugRunner::builder().from_dsl_yaml(probe_yaml).expect("probe YAML");
    for (id, when) in board {
        b = b.from_dsl_yaml(&digimon(id, when)).expect("board YAML");
    }
    let mut r = b.memory(0).start();
    let probe = r.place_on_field(0, probe_id, Some(0));
    for (id, _) in board {
        r.place_on_field(0, id, Some(0));
    }
    r.game.enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(probe));
    r.game.drain_effect_queue();
    r.pending_selection().map(|p| {
        assert_eq!(p.kind, SelectionKind::OwnField, "the pick is a field selection");
        p.valid_action_ids.len()
    })
}

const BOARD: &[(&str, &str)] = &[
    ("OP-ONLY", "on_play"),
    ("WD-ONLY", "when_digivolving"),
    ("BOTH", "[on_play, when_digivolving]"),
    ("ATTACK-ONLY", "when_attacking"),
    ("DELETION-ONLY", "on_deletion"),
];

#[test]
fn on_play_offers_digimon_with_an_on_play_effect() {
    assert_eq!(offered(PICK_ON_PLAY, "PROBE-ON-PLAY", BOARD), Some(2), "OP-ONLY and BOTH");
}

#[test]
fn when_digivolving_offers_digimon_with_a_when_digivolving_effect() {
    assert_eq!(
        offered(PICK_WHEN_DIGIVOLVING, "PROBE-WHEN-DIGIVOLVING", BOARD),
        Some(2),
        "WD-ONLY and BOTH"
    );
}

#[test]
fn either_timing_offers_every_digimon_with_one_of_them() {
    assert_eq!(
        offered(PICK_EITHER, "PROBE-EITHER", BOARD),
        Some(3),
        "OP-ONLY, WD-ONLY and BOTH; not the [When Attacking] or [On Deletion] ones"
    );
}

#[test]
fn an_all_ineligible_board_opens_no_prompt() {
    let board = [("ATTACK-ONLY", "when_attacking"), ("DELETION-ONLY", "on_deletion")];
    assert_eq!(offered(PICK_EITHER, "PROBE-EITHER", &board), None);
    assert_eq!(offered(PICK_ON_PLAY, "PROBE-ON-PLAY", &[("WD-ONLY", "when_digivolving")]), None);
}

#[test]
fn an_unknown_refire_timing_is_a_compile_error() {
    let err = match DebugRunner::builder().from_dsl_yaml(BAD_TIMING) {
        Ok(_) => panic!("`when_attacking` is not a refire timing"),
        Err(e) => e,
    };
    assert!(err.contains("unknown refire timing"), "{err}");
}
