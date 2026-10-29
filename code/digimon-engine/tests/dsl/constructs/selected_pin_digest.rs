//! A selected permanent's stable identity is parked gameplay state. Two
//! continuations can have identical field slots and visible choices while
//! committing their eventual effect to different permanents.

use digimon_engine::action::build_action_mask;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::recorder::VerificationDigestVersion;
use digimon_engine::resume::{OuterContinuation, ResumeFrame, ResumeStack};
use digimon_engine::{EffectTiming, Game, TriggerSource};

fn fixture(selector: &str, bind_as: &str, target_owner: u8) -> DebugRunner {
    fixture_with_schedule(selector, bind_as, target_owner, false)
}

fn fixture_with_schedule(
    selector: &str,
    bind_as: &str,
    target_owner: u8,
    scheduled: bool,
) -> DebugRunner {
    let mut yaml = format!(
        r#"
card: PIN-DIGEST
name: Pin Digest
kind: tamer
color: [black]
cost: 2
effects:
  - when: on_play
    process:
      - {selector}: {{ bind_as: {bind_as}, filter: {{ kind: digimon }}, prompt: Choose target }}
      - select_hand: {{ of: you, bind_as: hand_card, filter: {{ kind: digimon }}, prompt: Choose hand card }}
      - suspend: {{ target: {bind_as} }}
"#
    );
    if scheduled {
        yaml = yaml.replace(
            &format!("      - suspend: {{ target: {bind_as} }}"),
            &format!("      - schedule_delayed:\n          when: end_of_your_turn\n          body:\n            - suspend: {{ target: {bind_as} }}"),
        );
    }
    let mut target = make_test_card("PIN-TARGET", "Pin Target");
    target.level = Some(3);
    target.dp = Some(3000);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .unwrap()
        .add_card(target)
        .hand(0, &["PIN-TARGET", "PIN-TARGET"])
        .deck(0, &["PIN-TARGET"; 10])
        .deck(1, &["PIN-TARGET"; 10])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    r.place_on_field(target_owner, "PIN-TARGET", None);
    r.place_on_field(target_owner, "PIN-TARGET", None);
    let actor = r.place_on_field(0, "PIN-DIGEST", None);
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(actor));
    r.game.drain_effect_queue();
    r
}

fn pick_first(r: &mut DebugRunner, prompt: &str) {
    let pending = r.game.pending_selection.as_ref().expect("real choice");
    assert_eq!(pending.prompt, prompt);
    assert_eq!(pending.selecting_player, 0);
    assert!(!pending.is_optional);
    assert_eq!(pending.valid_action_ids.len(), 2);
    let action = pending.valid_action_ids[0];
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.execute_action(0, action).unwrap();
}

fn bindings(game: &Game) -> &Bindings {
    match &game.pending_selection_resume.as_ref().unwrap().frames[0] {
        ResumeFrame::RunTail { bindings, .. } => bindings,
        other => panic!("expected hand-choice tail, got {other:?}"),
    }
}

fn bindings_mut(game: &mut Game) -> &mut Bindings {
    match &mut game.pending_selection_resume.as_mut().unwrap().frames[0] {
        ResumeFrame::RunTail { bindings, .. } => bindings,
        other => panic!("expected hand-choice tail, got {other:?}"),
    }
}

fn distinct_parked_pins(selector: &str, owner: u8) -> (DebugRunner, Game) {
    park_alternatives(fixture(selector, "target", owner), owner, true)
}

fn park_alternatives(mut r: DebugRunner, owner: u8, preallocated: bool) -> (DebugRunner, Game) {
    if preallocated {
        // Allocate both identities through actual DSL choices before forking.
        // This avoids private-field fixture writes and makes exchanged frame
        // payloads genuinely distinct. The lazy-allocation case skips this.
        let actor = r
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .source_permanent
            .unwrap();
        for index in 0..2 {
            let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[index];
            assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
            r.execute_action(0, action).unwrap();
            pick_first(&mut r, "Choose hand card");
            for permanent in &mut r.game.players[owner as usize].battle_area {
                permanent.is_suspended = false;
            }
            r.game.scheduled_effects.clear();
            r.game.dsl_resolved_tail_bindings = None;
            r.game
                .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(actor));
            r.game.drain_effect_queue();
        }
    }
    let initial = r.game.clone();
    pick_first(&mut r, "Choose target");
    let first = r.game.clone();

    // Both choices bind the exact same public field slot. The second branch
    // chooses the other physical permanent, then restores the identical board.
    r.game = initial;
    r.game.players[owner as usize].battle_area.swap(0, 1);
    pick_first(&mut r, "Choose target");
    r.game.players[owner as usize].battle_area.swap(0, 1);
    assert_eq!(
        bindings(&first).get("target"),
        bindings(&r.game).get("target"),
        "the raw PermanentHandle is identical; only the identity pin differs"
    );
    assert_eq!(
        r.game.pending_selection.as_ref().unwrap().prompt,
        "Choose hand card"
    );
    (r, first)
}

#[test]
fn current_digest_distinguishes_selected_identity_behind_identical_slot_and_hand_choice() {
    for (selector, owner) in [
        ("select_own_permanent", 0),
        ("select_opponent_permanent", 1),
        ("select_any_permanent", 0),
        ("select_any_permanent", 1),
    ] {
        let (r, first) = distinct_parked_pins(selector, owner);
        assert_eq!(
            first.verification_digest_with_version(VerificationDigestVersion::LegacyV2),
            r.game
                .verification_digest_with_version(VerificationDigestVersion::LegacyV2),
            "archival digest projection must stay unchanged for {selector}"
        );
        assert_ne!(
            first.verification_digest(),
            r.game.verification_digest(),
            "different selected identities are different gameplay states: {selector}, owner {owner}"
        );
    }
}

#[test]
fn parked_pin_changes_eventual_target_and_clone_preserves_each_branch() {
    for (selector, owner) in [
        ("select_own_permanent", 0),
        ("select_opponent_permanent", 1),
        ("select_any_permanent", 0),
        ("select_any_permanent", 1),
    ] {
        let (mut r, first) = distinct_parked_pins(selector, owner);
        let second = r.game.clone();
        for (parked, target) in [(first, 0), (second, 1)] {
            assert_eq!(
                parked.verification_digest(),
                parked.clone().verification_digest()
            );
            let mut expected = None;
            for _ in 0..2 {
                r.game = parked.clone();
                pick_first(&mut r, "Choose hand card");
                assert!(r.game.pending_selection.is_none());
                let field = &r.game.players[owner as usize].battle_area;
                assert!(field[target].is_suspended);
                assert!(!field[1 - target].is_suspended);
                let digest = r.game.verification_digest();
                if let Some(previous) = expected {
                    assert_eq!(digest, previous);
                }
                expected = Some(digest);
            }
        }
    }
}

#[test]
fn pin_map_insertion_order_does_not_change_current_or_archival_digest() {
    let mut alpha = fixture("select_own_permanent", "alpha", 0);
    pick_first(&mut alpha, "Choose target");
    let a = bindings(&alpha.game).clone();
    let mut beta = fixture("select_own_permanent", "beta", 0);
    pick_first(&mut beta, "Choose target");
    let b = bindings(&beta.game).clone();
    let mut left = Bindings::new();
    left.merge_slots_from(&a);
    left.merge_slots_from(&b);
    let mut right = Bindings::new();
    right.merge_slots_from(&b);
    right.merge_slots_from(&a);
    let mut reordered = alpha.game.clone();
    *bindings_mut(&mut alpha.game) = left;
    *bindings_mut(&mut reordered) = right;
    assert_eq!(
        alpha.game.verification_digest(),
        reordered.verification_digest()
    );
    assert_eq!(
        alpha
            .game
            .verification_digest_with_version(VerificationDigestVersion::LegacyV2),
        reordered.verification_digest_with_version(VerificationDigestVersion::LegacyV2)
    );
}

#[test]
fn pin_frame_path_is_significant_when_the_same_pins_exchange_continuation_frames() {
    let (r, first) = distinct_parked_pins("select_own_permanent", 0);
    let first_frame = first.pending_selection_resume.as_ref().unwrap().frames[0].clone();
    let second_frame = r.game.pending_selection_resume.as_ref().unwrap().frames[0].clone();
    let mut forward = first.clone();
    forward.pending_selection_resume.as_mut().unwrap().frames =
        vec![first_frame.clone(), second_frame.clone()];
    let mut reverse = first;
    reverse.pending_selection_resume.as_mut().unwrap().frames = vec![second_frame, first_frame];
    assert_eq!(
        forward.verification_digest_with_version(VerificationDigestVersion::LegacyV2),
        reverse.verification_digest_with_version(VerificationDigestVersion::LegacyV2)
    );
    assert_ne!(forward.verification_digest(), reverse.verification_digest());
}

#[test]
fn legacy_binding_debug_projection_excludes_only_the_new_selected_pin_metadata() {
    let (_, first) = distinct_parked_pins("select_own_permanent", 0);
    let pinned = bindings(&first);
    let mut without_pin = pinned.clone();
    // A normal insert deliberately clears the selected identity pin while
    // retaining the same legacy binding slot. Legacy digest paths already use
    // Debug for some enclosing declarations, so they must retain this view.
    without_pin.insert("target", pinned.get("target").unwrap());
    assert_eq!(format!("{pinned:?}"), format!("{without_pin:?}"));
}

#[test]
fn lazy_identity_allocation_hashes_which_live_host_the_same_number_resolves_to() {
    let (r, first) = park_alternatives(fixture("select_own_permanent", "target", 0), 0, false);
    // No selection happened before the fork, so each branch's first capture
    // allocates the same next identity. Its live host differs after restoration.
    assert_eq!(
        first.verification_digest_with_version(VerificationDigestVersion::LegacyV2),
        r.game
            .verification_digest_with_version(VerificationDigestVersion::LegacyV2)
    );
    assert_ne!(first.verification_digest(), r.game.verification_digest());
}

fn outer_from_frame(frame: &ResumeFrame) -> OuterContinuation {
    match frame {
        ResumeFrame::RunTail {
            prov,
            inner_tail,
            bindings,
            runtime,
            trigger_context,
            ..
        } => OuterContinuation {
            tail: inner_tail.clone(),
            bindings: bindings.clone(),
            runtime: runtime.clone(),
            trigger_context: trigger_context.clone(),
            source_card: prov.source_card,
            source_permanent: prov.source_permanent,
            player: prov.controller,
        },
        other => panic!("expected hand-choice tail, got {other:?}"),
    }
}

#[test]
fn nested_frames_and_outer_continuations_keep_distinct_pin_scopes() {
    let (r, first) = distinct_parked_pins("select_own_permanent", 0);
    let first_frame = first.pending_selection_resume.as_ref().unwrap().frames[0].clone();
    let second_frame = r.game.pending_selection_resume.as_ref().unwrap().frames[0].clone();
    for carrier in ["nested", "outer"] {
        let mut a = first.clone();
        let mut b = first.clone();
        for (game, frame) in [(&mut a, &first_frame), (&mut b, &second_frame)] {
            let inner = if carrier == "nested" {
                ResumeStack {
                    frames: vec![frame.clone()],
                }
            } else {
                // The identical inner continuation carries no selected pin;
                // the changed binding exists only in the enclosing outer tail.
                let mut neutral = first_frame.clone();
                if let ResumeFrame::RunTail { bindings, .. } = &mut neutral {
                    let raw = bindings.get("target").unwrap();
                    bindings.insert("target", raw);
                }
                ResumeStack {
                    frames: vec![neutral],
                }
            };
            game.pending_selection_resume = Some(ResumeStack {
                frames: vec![ResumeFrame::ReplacementBody {
                    inner: Box::new(inner),
                    outer_conts: if carrier == "outer" {
                        vec![outer_from_frame(frame)]
                    } else {
                        vec![]
                    },
                }],
            });
        }
        assert_eq!(
            a.verification_digest_with_version(VerificationDigestVersion::LegacyV2),
            b.verification_digest_with_version(VerificationDigestVersion::LegacyV2),
            "legacy {carrier}"
        );
        assert_ne!(
            a.verification_digest(),
            b.verification_digest(),
            "current {carrier}"
        );
    }
}

#[test]
fn scheduled_and_queued_processing_hash_captured_pin_and_resolve_the_selected_host() {
    let fixture = fixture_with_schedule("select_own_permanent", "target", 0, true);
    let (mut r, first) = park_alternatives(fixture, 0, true);
    let second = r.game.clone();
    let mut captures = Vec::new();
    for parked in [first, second] {
        r.game = parked;
        pick_first(&mut r, "Choose hand card");
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.game.scheduled_effects.len(), 1);
        assert!(r.game.players[0]
            .battle_area
            .iter()
            .all(|p| !p.is_suspended));
        // This short-lived publication is consumed by selection wrappers. It
        // must not serve as a second witness for the scheduled capture below.
        r.game.dsl_resolved_tail_bindings = None;
        captures.push(r.game.clone());
    }
    for queued in [false, true] {
        let mut games = captures.clone();
        if queued {
            for game in &mut games {
                assert_eq!(
                    game.enqueue_scheduled_for_timing(EffectTiming::EndOfYourTurn),
                    1
                );
                assert!(game.scheduled_effects.is_empty());
            }
        }
        assert_eq!(
            games[0].verification_digest_with_version(VerificationDigestVersion::LegacyV2),
            games[1].verification_digest_with_version(VerificationDigestVersion::LegacyV2),
            "legacy queued={queued}"
        );
        assert_ne!(
            games[0].verification_digest(),
            games[1].verification_digest(),
            "current queued={queued}"
        );
        for (target, game) in games.into_iter().enumerate() {
            r.game = game;
            if queued {
                r.game.drain_effect_queue();
            } else {
                digimon_engine::scheduled_effects::fire_scheduled_for_timing(
                    &mut r.game,
                    EffectTiming::EndOfYourTurn,
                );
            }
            assert!(r.game.players[0].battle_area[target].is_suspended);
            assert!(!r.game.players[0].battle_area[1 - target].is_suspended);
        }
    }
}
