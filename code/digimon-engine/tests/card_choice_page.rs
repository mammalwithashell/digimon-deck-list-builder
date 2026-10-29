//! Pure paging-owner regressions. Integration into game selection is separately gated.
use digimon_engine::card_choice_page::{
    CardChoiceError, CardChoicePage, CardChoiceResult, CARD_CHOICE_NEXT, CARD_CHOICE_PREVIOUS,
    CARD_CHOICE_START,
};

#[test]
fn every_physical_candidate_is_reachable_without_a_depth_cap() {
    let mut page = CardChoicePage::new(17, (0usize..1001).collect(), false).unwrap();
    let mut seen = Vec::new();
    loop {
        let token = page.token();
        for row in page.rows() {
            seen.push(match page.resolve(token, row.action_id).unwrap() {
                CardChoiceResult::Selected(pin) => pin,
                other => panic!("unexpected result: {other:?}"),
            });
        }
        if !page.valid_action_ids().contains(&CARD_CHOICE_NEXT) {
            break;
        }
        assert_eq!(
            page.resolve(token, CARD_CHOICE_NEXT),
            Ok(CardChoiceResult::Navigated)
        );
    }
    assert_eq!(seen, (0..1001).collect::<Vec<_>>());
    assert_eq!(page.page_count(), 36);
}

#[test]
fn stale_click_cannot_select_a_different_card_using_the_same_action() {
    let mut page = CardChoicePage::new(9, (0usize..60).collect(), false).unwrap();
    let old = page.token();
    page.resolve(old, CARD_CHOICE_NEXT).unwrap();
    let snapshot = page.clone();
    assert_eq!(
        page.resolve(old, CARD_CHOICE_START),
        Err(CardChoiceError::StaleToken)
    );
    assert_eq!(page, snapshot);
    assert_eq!(
        page.resolve(page.token(), CARD_CHOICE_START),
        Ok(CardChoiceResult::Selected(28))
    );
}

#[test]
fn clone_and_back_navigation_preserve_physical_pins_but_invalidate_tokens() {
    let mut page = CardChoicePage::new(
        30,
        vec![11usize, 12, 999]
            .into_iter()
            .chain(1000..1030)
            .collect(),
        false,
    )
    .unwrap();
    let initial = page.token();
    let fork = page.clone();
    page.resolve(initial, CARD_CHOICE_NEXT).unwrap();
    page.resolve(page.token(), CARD_CHOICE_PREVIOUS).unwrap();
    assert_eq!(page.rows(), fork.rows());
    assert_ne!(page.token(), fork.token());
    assert_eq!(
        fork.clone().resolve(initial, CARD_CHOICE_START + 2),
        Ok(CardChoiceResult::Selected(999))
    );
    assert_eq!(
        page.resolve(initial, CARD_CHOICE_START),
        Err(CardChoiceError::StaleToken)
    );
}

#[test]
fn invalid_navigation_decline_and_cross_prompt_tokens_leave_owner_unchanged() {
    let mut page = CardChoicePage::new(3, vec![42usize], false).unwrap();
    let other = CardChoicePage::new(4, vec![42usize], true).unwrap();
    let snapshot = page.clone();
    for action in [
        CARD_CHOICE_PREVIOUS,
        CARD_CHOICE_NEXT,
        CARD_CHOICE_START + 1,
        62,
    ] {
        assert_eq!(
            page.resolve(page.token(), action),
            Err(CardChoiceError::InvalidAction)
        );
        assert_eq!(page, snapshot);
    }
    assert_eq!(
        page.resolve(other.token(), CARD_CHOICE_START),
        Err(CardChoiceError::StaleToken)
    );
    assert_eq!(page, snapshot);
    assert_eq!(
        other.clone().resolve(other.token(), 62),
        Ok(CardChoiceResult::Declined)
    );
}

#[test]
fn empty_and_duplicate_physical_candidates_are_rejected() {
    assert_eq!(
        CardChoicePage::<usize>::new(1, vec![], false),
        Err(CardChoiceError::EmptyCandidates)
    );
    assert_eq!(
        CardChoicePage::new(1, vec![3usize, 3], false),
        Err(CardChoiceError::DuplicateCandidate)
    );
}
