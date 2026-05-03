use std::sync::{Arc, Mutex};

use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::{EffectContext, RevealBucketSelection};
use digimon_engine::selection::SelectionKind;

fn add_card_to_reveal_for_test(
    r: &mut DebugRunner,
    player: digimon_engine::enums::PlayerId,
    card_id: &str,
) -> CardHandle {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let card_index = r.game.next_card_index();
    let card = CardSource::new(data_idx, player, card_index);
    let handle = card.handle();
    r.game.revealed_cards.push(card);
    handle
}

#[test]
fn reveal_buckets_prevent_cross_bucket_duplicate_pick() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("BT17-009", "Hybrid Candidate"))
        .add_card(make_test_card("BT17-010", "Other Candidate"))
        .start();
    let p0 = 0;
    let source = CardHandle(0);
    let first = add_card_to_reveal_for_test(&mut r, p0, "BT17-009");
    let second = add_card_to_reveal_for_test(&mut r, p0, "BT17-010");

    let picked = Arc::new(Mutex::new(Vec::new()));
    let picked_slot = Arc::clone(&picked);

    {
        let mut ctx = EffectContext::new(&mut r.game, source, None, p0);
        ctx.select_reveal_buckets(
            vec![
                RevealBucketSelection {
                    bind_as: "hybrid".to_string(),
                    min: 0,
                    max: 1,
                    candidates: vec![first, second],
                },
                RevealBucketSelection {
                    bind_as: "tamer".to_string(),
                    min: 0,
                    max: 1,
                    candidates: vec![first],
                },
            ],
            "Choose cards",
            true,
            move |_ctx, buckets| {
                *picked_slot.lock().unwrap() = buckets
                    .into_iter()
                    .flat_map(|(_, cards)| cards)
                    .collect::<Vec<_>>();
            },
        );
    }

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("first bucket prompt");
    assert!(matches!(pending.kind, SelectionKind::RevealBucket { .. }));
    let first_action = pending.valid_action_ids[0];
    r.game
        .resolve_selection(p0, first_action)
        .expect("pick first card");

    assert!(
        r.game.pending_selection.is_none(),
        "second bucket should auto-finalize when duplicate prevention leaves no candidates"
    );

    assert_eq!(*picked.lock().unwrap(), vec![first]);
}

#[test]
fn reveal_bucket_finalizes_when_mandatory_min_exceeds_remaining_candidates_after_pick() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("BT17-009", "Hybrid Candidate"))
        .start();
    let p0 = 0;
    let source = CardHandle(0);
    let first = add_card_to_reveal_for_test(&mut r, p0, "BT17-009");

    let picked = Arc::new(Mutex::new(Vec::new()));
    let picked_slot = Arc::clone(&picked);

    {
        let mut ctx = EffectContext::new(&mut r.game, source, None, p0);
        ctx.select_reveal_buckets(
            vec![RevealBucketSelection {
                bind_as: "hybrid".to_string(),
                min: 2,
                max: 2,
                candidates: vec![first],
            }],
            "Choose cards",
            true,
            move |_ctx, buckets| {
                *picked_slot.lock().unwrap() = buckets;
            },
        );
    }

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("first bucket prompt");
    assert!(matches!(pending.kind, SelectionKind::RevealBucket { .. }));
    assert_eq!(pending.valid_action_ids.len(), 1);
    let first_action = pending.valid_action_ids[0];

    r.game
        .resolve_selection(p0, first_action)
        .expect("pick only available card");

    assert!(
        r.game.pending_selection.is_none(),
        "bucket should finalize instead of parking a mandatory empty prompt"
    );
    assert_eq!(
        *picked.lock().unwrap(),
        vec![("hybrid".to_string(), vec![first])]
    );
}

#[test]
fn reveal_bucket_finalizes_empty_when_mandatory_bucket_has_no_candidates() {
    let mut r = DebugRunner::builder().start();
    let p0 = 0;
    let source = CardHandle(0);

    let picked = Arc::new(Mutex::new(Vec::new()));
    let picked_slot = Arc::clone(&picked);

    {
        let mut ctx = EffectContext::new(&mut r.game, source, None, p0);
        ctx.select_reveal_buckets(
            vec![RevealBucketSelection {
                bind_as: "hybrid".to_string(),
                min: 1,
                max: 1,
                candidates: Vec::new(),
            }],
            "Choose cards",
            true,
            move |_ctx, buckets| {
                *picked_slot.lock().unwrap() = buckets;
            },
        );
    }

    assert!(
        r.game.pending_selection.is_none(),
        "bucket should finalize instead of parking a mandatory empty prompt"
    );
    assert_eq!(
        *picked.lock().unwrap(),
        vec![("hybrid".to_string(), vec![])]
    );
}
