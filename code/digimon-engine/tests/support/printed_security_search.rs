//! Shared printed-search probe. Navigation never resolves the card effect.
use digimon_engine::{action::{build_action_mask,space::PASS},card_choice_page::CARD_CHOICE_NEXT,
    debug_runner::DebugRunner,owned_card_choice::{CardChoiceOperation,CardChoiceZone}};
use rand::RngCore;
pub fn inspect_all_then_decline(r:&mut DebugRunner,actor:u8,eligible:&[usize]) {
    let handles:Vec<_>=r.game.player(actor).security.iter().map(|c|c.handle()).collect();
    let rng=r.game.rng.clone().next_u64();
    let mut seen=Vec::new();
    loop {
        let pending=r.game.pending_selection.as_ref().expect("a nonempty printed search must be inspectable even with no matches");
        assert_eq!(pending.selecting_player,actor);
        let owner=pending.card_choice.as_ref().expect("owned security search");
        let menu=owner.view(&r.game,actor);
        let mask=build_action_mask(&r.game,actor);
        for row in menu.rows.iter().filter(|row|matches!(row.zone,Some(CardChoiceZone::Security))) {
            let index:usize=row.position.as_deref().unwrap().parse().unwrap();
            assert_eq!(row.owner,Some(actor));
            assert!(row.identity_visible,"printed search grants inspection of every card, including ineligible cards");
            assert_eq!(row.card_id.as_deref(),Some(r.game.player(actor).security[index].card_id(&r.game.card_data)));
            assert_eq!(matches!(row.operation,CardChoiceOperation::CommitCard),eligible.contains(&index));
            assert_eq!(mask[row.action_id as usize],if eligible.contains(&index){1.0}else{0.0});
            seen.push(index);
        }
        let peer=owner.view(&r.game,1-actor);
        assert!(peer.rows.iter().all(|row|row.card_id.is_none()&&row.card_registry_index.is_none()&&!row.identity_visible));
        assert!(build_action_mask(&r.game,1-actor).iter().all(|&v|v==0.0));
        if mask[CARD_CHOICE_NEXT as usize]==0.0 {break;}
        r.execute_action(actor,CARD_CHOICE_NEXT).unwrap();
        assert_eq!(r.game.player(actor).security.iter().map(|c|c.handle()).collect::<Vec<_>>(),handles);
        assert_eq!(r.game.rng.clone().next_u64(),rng);
    }
    assert_eq!(seen,(0..handles.len()).collect::<Vec<_>>(),"the entire security stack must be inspected");
    let mut expected=r.game.clone();expected.shuffle_security(actor);
    assert_eq!(build_action_mask(&r.game,actor)[PASS as usize],1.0);
    r.execute_action(actor,PASS).unwrap();r.auto_resolve().unwrap();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.player(actor).security.iter().map(|c|c.handle()).collect::<Vec<_>>(),expected.player(actor).security.iter().map(|c|c.handle()).collect::<Vec<_>>());
    assert_eq!(r.game.rng.clone().next_u64(),expected.rng.next_u64(),"decline shuffles exactly once without earlier navigation RNG");
}
