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
    assert_eq!(rec.actions.len(), 64);
    assert_eq!(rec.digests.len(), 64);
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
        if i == 52 {
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
        if i == 55 {
            assert!(
                runner.game.pending_selection.is_some(),
                "clone actual pending step56"
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
