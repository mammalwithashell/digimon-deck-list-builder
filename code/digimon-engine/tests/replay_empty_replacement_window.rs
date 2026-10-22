//! Preserve the recorded legal path across empty replacement windows.
//! This test consumes the committed recording; it never regenerates digests.
use digimon_engine::{
    card_data::CardData,
    recorder::{ReplayDeckRef, VerificationReplayRecording},
    runners::HeadlessRunner,
};
#[test]
fn empty_replacement_window_preserves_recorded_path_and_pending_clone() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cards = CardData::load_from_file(&root.join("data/cards.json")).unwrap();
    let path=root.join("qa/replay-goldens/refresh_v2_generated_greedy_001_digilab_0286bd08d2eb_vs_digilab_03052bcf3884_seed_40521419.replay.json");
    let rec: VerificationReplayRecording =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(rec.engine_schema_version, 1, "schema retained; only reviewed source-attribution step53 migrated");
    let digest_version = digimon_engine::recorder::VerificationDigestVersion::from_engine_schema(rec.engine_schema_version).unwrap();
    // Migrated 2026-10-17 for the §15-7-1 optional-condition prompt (da93979e8): one accept
    // action (59) inserted after step 44, so the recorded path now has 65 actions and digests.
    // Migrated 2026-10-18 for trigger qualifiers decided when the event triggers (§15-8-3-1,
    // §15-8-3-4; c917f8cdb): the opponent's Owen Dreadnought printings (EX11-054, BT24-082) observe
    // only THEIR controller's Digimon being played / digivolving, so P0's plays no longer queue
    // them. Digests 6-8, 19-21, 27-29, 36, 50, 52-55 and 57 lost only those never-activating
    // queue entries (every other hashed token identical); actions and the rest are unchanged.
    // Migrated 2026-10-21 for a declined folded "may" keeping its [Once Per Turn] (§15-14-1-4,
    // Q1818; 9888747b0): P0's breeding-area BT21-001 pick declined at step 61 is refunded and
    // re-offered on the next two attacks' security removals, so two declines (62) are inserted
    // (now steps 63 and 65; 67 actions). Steps 1-59 are unchanged; digests 60-62 change.
    assert_eq!(rec.actions.len(), 67);
    assert_eq!(rec.digests.len(), 67);
    let deck = |d: &ReplayDeckRef| match d {
        ReplayDeckRef::Inline { cards } => cards.clone(),
        _ => panic!("regression fixture requires inline decks"),
    };
    let make = || {
        HeadlessRunner::new(
            deck(&rec.deck_refs.player1),
            deck(&rec.deck_refs.player2),
            &cards,
            false,
            false,
            false,
            rec.seed,
        )
        .unwrap()
    };
    let mut runner = make();
    let mut clone: Option<HeadlessRunner> = None;
    for (i, action) in rec.actions.iter().enumerate() {
        assert_eq!(
            runner.current_decision_player(),
            action.seat,
            "actor at {}",
            i + 1
        );
        assert!(
            runner.get_action_mask()[action.action_id as usize] > 0.0,
            "legal action at {}",
            i + 1
        );
        runner.step(action.action_id);
        assert_eq!(
            runner.game.verification_digest_with_version(digest_version),
            rec.digests[i],
            "recorded digest at {}",
            i + 1
        );
        if let Some(branch) = clone.as_mut() {
            assert_eq!(
                branch.current_decision_player(),
                action.seat,
                "clone actor at {}",
                i + 1
            );
            assert!(
                branch.get_action_mask()[action.action_id as usize] > 0.0,
                "clone legal action at {}",
                i + 1
            );
            branch.step(action.action_id);
            assert_eq!(
                branch.game.verification_digest_with_version(digest_version),
                rec.digests[i],
                "clone digest at {}",
                i + 1
            );
        }
        if i == 53 {
            use digimon_engine::enums::EffectTiming;
            use digimon_engine::trigger_context::EventCause;
            let affected: Vec<_> = runner.game.effect_queue.iter()
                .filter(|q| q.timing == EffectTiming::OnOpponentSecurityRemoved).collect();
            assert_eq!(affected.len(), 2, "both inherited security-removal observers are queued");
            for queued in affected {
                let context = queued.trigger_context.as_ref().unwrap();
                assert_eq!(context.affected_player, Some(1));
                assert_eq!(context.cause, Some(EventCause::OpponentEffect), "P0's effect removes P1 security");
                let source = context.source_effect.expect("resumed body retains effect provenance");
                assert_eq!(source.controller, 0);
                assert_eq!(source.source_card, Some(digimon_engine::card_source::CardHandle(27)));
                assert_eq!(source.source_permanent, Some(digimon_engine::permanent::PermanentHandle {player:0,index:0}));
            }
        }
        if i == 56 {
            assert!(
                runner.game.pending_selection.is_some(),
                "clone actual pending step57"
            );
            let mut branch = make();
            branch.game = runner.game.clone();
            assert_eq!(branch.get_action_mask(), runner.get_action_mask());
            assert_eq!(
                branch.game.verification_digest(),
                runner.game.verification_digest()
            );
            clone = Some(branch);
        }
    }
    assert!(clone.is_some());
}
