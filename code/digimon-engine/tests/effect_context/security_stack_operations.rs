use std::sync::Arc;

use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardSourceRef, StackPosition};

struct OppSecurityRemovedGainOne;

impl CardEffect for OppSecurityRemovedGainOne {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_opponent_security_removed(card)
            .name("gain 1 on opponent security removed")
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

struct LoseSecurityGainTwo;

impl CardEffect for LoseSecurityGainTwo {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_lose_security(card)
            .name("gain 2 on lose security")
            .process(|ctx| ctx.gain_memory(2))
            .build()]
    }
}

struct LoseSecurityAddPendingToHand;

impl CardEffect for LoseSecurityAddPendingToHand {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_lose_security(card)
            .name("add removed security to hand")
            .process(|ctx| {
                assert!(ctx.add_pending_security_to_hand());
            })
            .build()]
    }
}

struct LoseSecuritySelectThenAddPendingToHand;

impl CardEffect for LoseSecuritySelectThenAddPendingToHand {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_lose_security(card)
            .name("select then add removed security to hand")
            .process(|ctx| {
                ctx.select_hand(
                    0,
                    "Choose a card",
                    false,
                    |_game, _idx| true,
                    |cb_ctx, _idx| {
                        assert!(cb_ctx.add_pending_security_to_hand());
                    },
                );
            })
            .build()]
    }
}

struct LoseSecuritySelectThenTrashNextSecurity;

impl CardEffect for LoseSecuritySelectThenTrashNextSecurity {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_lose_security(card)
            .name("select then trash next security")
            .process(|ctx| {
                ctx.select_hand(
                    0,
                    "Choose a card",
                    false,
                    |_game, _idx| true,
                    |cb_ctx, _idx| {
                        assert!(cb_ctx.trash_top_security(0));
                    },
                );
            })
            .build()]
    }
}

fn add_card_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("test card exists");
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, player, card_index);
    let handle = card.handle();
    runner.game.players[player as usize].trash.push(card);
    handle
}

fn add_card_to_security_owned(
    runner: &mut DebugRunner,
    security_player: u8,
    owner: u8,
    card_id: &str,
) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("test card exists");
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, owner, card_index);
    let handle = card.handle();
    runner.game.players[security_player as usize]
        .security
        .push(card);
    handle
}

#[test]
fn selected_security_card_moves_to_hand_by_exact_identity() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SEC-A", "Security A"))
        .add_card(make_test_card("SEC-B", "Security B"))
        .add_card(make_test_card("SEC-C", "Security C"))
        .security(0, &["SEC-A", "SEC-B", "SEC-C"])
        .start();

    let source_card = runner.game.players[0].security[0].handle();
    let picked = runner.game.players[0].security[1].handle();
    let not_picked_low = runner.game.players[0].security[0].handle();
    let not_picked_high = runner.game.players[0].security[2].handle();

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        ctx.select_security(
            0,
            "Choose a security card",
            false,
            |_game, _idx| true,
            |cb_ctx, idx| {
                let card = cb_ctx.game.player(0).security[idx].handle();
                assert!(cb_ctx.add_to_hand_from_security(0, card));
            },
        );
    }

    let action = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("security selection installed");
        pending.valid_action_ids[1]
    };
    runner
        .game
        .resolve_selection(0, action)
        .expect("selected security action resolves");

    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.handle() == picked),
        "the selected security card moves to hand"
    );
    assert!(
        !runner.game.players[0]
            .security
            .iter()
            .any(|c| c.handle() == picked),
        "the selected card leaves security"
    );
    assert!(
        runner.game.players[0]
            .security
            .iter()
            .any(|c| c.handle() == not_picked_low),
        "the lower non-picked security card stays put"
    );
    assert!(
        runner.game.players[0]
            .security
            .iter()
            .any(|c| c.handle() == not_picked_high),
        "the upper non-picked security card stays put"
    );
}

#[test]
fn selected_security_card_moves_to_owners_hand() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BORROWED", "Borrowed"))
        .start();
    let borrowed = add_card_to_security_owned(&mut runner, 0, 1, "BORROWED");

    {
        let mut ctx = EffectContext::new(&mut runner.game, borrowed, None, 0);
        assert!(ctx.add_to_hand_from_security(0, borrowed));
    }

    assert_eq!(runner.game.players[0].hand.len(), 0);
    assert_eq!(runner.game.players[1].hand.len(), 1);
    assert_eq!(runner.game.players[1].hand[0].handle(), borrowed);
    assert_eq!(runner.game.players[1].hand[0].owner, 1);
}

#[test]
fn trash_top_security_fires_opponent_security_removed_once() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("OBS", "Observer"))
        .add_card(make_test_card("SEC", "Security"))
        .security(1, &["SEC"])
        .memory(0)
        .start();
    runner.register_effect("OBS", Arc::new(OppSecurityRemovedGainOne));
    runner.place_on_field(0, "OBS", Some(0));

    let source_card = runner.game.players[0].battle_area[0].top_card().handle();
    let before = runner.game.memory;

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        assert!(ctx.trash_top_security(1));
    }

    assert_eq!(
        runner.game.memory,
        before + 1,
        "OnOpponentSecurityRemoved observer should fire exactly once"
    );
    assert_eq!(runner.game.players[1].security.len(), 0);
    assert_eq!(runner.game.players[1].trash.len(), 1);
}

#[test]
fn trash_top_security_fires_removed_cards_on_lose_security_once() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("OBS", "Observer"))
        .add_card(make_test_card("SEC", "Security"))
        .security(1, &["SEC"])
        .memory(0)
        .start();
    runner.register_effect("OBS", Arc::new(OppSecurityRemovedGainOne));
    runner.register_effect("SEC", Arc::new(LoseSecurityGainTwo));
    runner.place_on_field(0, "OBS", Some(0));

    let source_card = runner.game.players[0].battle_area[0].top_card().handle();

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        assert!(ctx.trash_top_security(1));
    }

    assert_eq!(
        runner.game.memory, 3,
        "OnLoseSecurity (+2) and OnOpponentSecurityRemoved (+1) should each fire once"
    );
}

#[test]
fn trash_top_security_pending_to_hand_does_not_duplicate_removed_card() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SEC", "Security"))
        .security(0, &["SEC"])
        .start();
    runner.register_effect("SEC", Arc::new(LoseSecurityAddPendingToHand));

    let source_card = runner.game.players[0].security[0].handle();

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        assert!(ctx.trash_top_security(0));
    }

    assert_eq!(runner.game.players[0].security.len(), 0);
    assert_eq!(runner.game.players[0].trash.len(), 0);
    assert_eq!(runner.game.players[0].hand.len(), 1);
    assert_eq!(
        runner.game.players[0].hand[0].card_id(&runner.game.card_data),
        "SEC"
    );
    assert!(runner.game.pending_security.is_none());
}

#[test]
fn trash_top_security_resumes_after_lose_security_selection_without_duplication() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SEC", "Security"))
        .add_card(make_test_card("CHOICE", "Choice"))
        .security(0, &["SEC"])
        .hand(0, &["CHOICE"])
        .start();
    runner.register_effect("SEC", Arc::new(LoseSecuritySelectThenAddPendingToHand));

    let source_card = runner.game.players[0].security[0].handle();

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        assert!(ctx.trash_top_security(0));
    }

    assert!(runner.game.pending_selection.is_some());
    assert!(!runner.game.pending_effect_security_removal.is_empty());
    assert!(runner.game.pending_security.is_some());

    let action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner
        .game
        .resolve_selection(0, action)
        .expect("hand selection resolves");

    let security_cards_in_hand = runner.game.players[0]
        .hand
        .iter()
        .filter(|card| card.card_id(&runner.game.card_data) == "SEC")
        .count();
    assert_eq!(security_cards_in_hand, 1);
    assert_eq!(runner.game.players[0].trash.len(), 0);
    assert!(runner.game.pending_selection.is_none());
    assert!(runner.game.pending_security.is_none());
    assert!(runner.game.pending_effect_security_removal.is_empty());
}

#[test]
fn nested_deferred_security_removals_resume_outer_after_inner_selection() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("INNER", "Inner"))
        .add_card(make_test_card("OUTER", "Outer"))
        .add_card(make_test_card("CHOICE-A", "Choice A"))
        .add_card(make_test_card("CHOICE-B", "Choice B"))
        .security(0, &["INNER", "OUTER"])
        .hand(0, &["CHOICE-A", "CHOICE-B"])
        .start();
    runner.register_effect("OUTER", Arc::new(LoseSecuritySelectThenTrashNextSecurity));
    runner.register_effect("INNER", Arc::new(LoseSecuritySelectThenAddPendingToHand));

    let source_card = runner.game.players[0].security[1].handle();

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        assert!(ctx.trash_top_security(0));
    }

    assert!(runner.game.pending_selection.is_some());
    assert_eq!(runner.game.pending_effect_security_removal.len(), 1);

    let outer_action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner
        .game
        .resolve_selection(0, outer_action)
        .expect("outer hand selection resolves");

    assert!(runner.game.pending_selection.is_some());
    assert_eq!(
        runner.game.pending_effect_security_removal.len(),
        2,
        "inner direct removal should stack on top of the outer continuation"
    );

    let inner_action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner
        .game
        .resolve_selection(0, inner_action)
        .expect("inner hand selection resolves");

    assert!(runner.game.pending_selection.is_none());
    assert!(runner.game.pending_effect_security_removal.is_empty());
    assert!(runner.game.pending_security.is_none());
    assert_eq!(runner.game.players[0].security.len(), 0);
    assert_eq!(
        runner.game.players[0]
            .hand
            .iter()
            .filter(|card| card.card_id(&runner.game.card_data) == "INNER")
            .count(),
        1
    );
    assert_eq!(
        runner.game.players[0]
            .trash
            .iter()
            .filter(|card| card.card_id(&runner.game.card_data) == "OUTER")
            .count(),
        1
    );
}

#[test]
fn place_on_security_bottom_from_trash_preserves_owner_and_does_not_fire_loss_observer() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("OBS", "Observer"))
        .add_card(make_test_card("RECOVER", "Recover"))
        .add_card(make_test_card("EXISTING", "Existing"))
        .security(0, &["EXISTING"])
        .memory(0)
        .start();
    runner.register_effect("OBS", Arc::new(OppSecurityRemovedGainOne));
    runner.place_on_field(1, "OBS", Some(0));

    let recover_handle = add_card_to_trash(&mut runner, 1, "RECOVER");
    let source_card = runner.game.players[1].battle_area[0].top_card().handle();
    let before = runner.game.memory;

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 1);
        assert!(ctx.place_on_security(0, CardSourceRef::Trash(1, 0), StackPosition::Bottom, false,));
    }

    assert_eq!(
        runner.game.memory, before,
        "recovering to security must not fire security-loss observers"
    );
    assert_eq!(runner.game.players[1].trash.len(), 0);
    assert_eq!(
        runner.game.players[0].security[0].handle(),
        recover_handle,
        "bottom placement inserts at index 0"
    );
    assert_eq!(
        runner.game.players[0].security[0].owner, 1,
        "the recovered card keeps its original owner"
    );
}
