//! Test-only physical union rows. Action numbers never identify a zone.
use digimon_engine::{
    action::build_action_mask,
    game::Game,
    owned_card_choice::{CardChoiceOperation, CardChoiceZone},
};
pub fn action(game: &Game, owner: u8, id: &str, trash: bool) -> Option<u16> {
    let pending = game
        .pending_selection
        .as_ref()
        .expect("pending union choice");
    assert_eq!(pending.selecting_player, owner);
    let page = pending
        .card_choice
        .as_ref()
        .expect("physical union owner")
        .view(game, owner);

    let row = page.rows.iter().find(|row| {
        matches!(row.operation, CardChoiceOperation::CommitCard)
            && matches!(
                (&row.zone, trash),
                (Some(CardChoiceZone::Trash), true) | (Some(CardChoiceZone::Hand), false)
            )
            && row.owner == Some(owner)
            && row.card_id.as_deref() == Some(id)
    });
    assert!(build_action_mask(game, 1 - owner).iter().all(|v| *v == 0.0));
    row.map(|row| {
        assert!(row.identity_visible);
        let index: usize = row
            .position
            .as_deref()
            .expect("physical zone position")
            .parse()
            .unwrap();
        let physical = if trash {
            &game.player(owner).trash[index]
        } else {
            &game.player(owner).hand[index]
        };
        assert_eq!(physical.card_id(&game.card_data), id);
        assert_eq!(build_action_mask(game, owner)[row.action_id as usize], 1.0);
        let peer = pending.card_choice.as_ref().unwrap().view(game, 1 - owner);
        if !trash {
            let peer_row = peer
                .rows
                .iter()
                .find(|r| r.action_id == row.action_id)
                .unwrap();
            assert!(!peer_row.identity_visible);
            assert!(peer_row.card_id.is_none());
        }
        row.action_id
    })
}
pub fn is_zone(game: &Game, action: u16, trash: bool) -> bool {
    let p = game.pending_selection.as_ref().unwrap();
    let page = p
        .card_choice
        .as_ref()
        .unwrap()
        .view(game, p.selecting_player);
    page.rows.iter().any(|r| {
        r.action_id == action
            && matches!(r.operation, CardChoiceOperation::CommitCard)
            && matches!(
                (&r.zone, trash),
                (Some(CardChoiceZone::Trash), true) | (Some(CardChoiceZone::Hand), false)
            )
    })
}
