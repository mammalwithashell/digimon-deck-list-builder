use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Expiry, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;
const ID: &str = "BT22-085";
fn base() -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder().dsl_card(ID).expect("BT22-085 loads");
    for (id, name, kind) in [
        ("VEED", "AeroVeedramon", CardKind::Digimon),
        ("OTHER", "Other", CardKind::Digimon),
        ("FAKE", "Veedramon Tamer", CardKind::Tamer),
    ] {
        let mut c = make_test_card(id, name);
        c.card_kind = kind;
        c.level = Some(5);
        c.dp = Some(5000);
        c.colors = vec![CardColor::Blue];
        b = b.add_card(c);
    }
    b.deck(0, &["OTHER"; 20]).deck(1, &["OTHER"; 20]).memory(10)
}
fn attack_event(r: &mut DebugRunner, h: PermanentHandle) {
    let card = r.top_card(h);
    r.game.enqueue_triggered(
        EffectTiming::OnAllyAttack,
        TriggerSource::PlayerBattleAreaAttack {
            player: h.player,
            attacker: h,
            card,
        },
    );
    r.game.drain_effect_queue();
}
fn choose(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("visible choice");
    let m = digimon_engine::action::build_action_mask(&r.game, v.selecting_player);
    assert!(v.valid_action_ids.iter().all(|a| m[*a as usize] > 0.0));
    r.execute_action(v.selecting_player, v.valid_action_ids[0])
        .unwrap();
}
#[test]
fn bt22_085_memory_setter_is_conditional() {
    for (before, after) in [(0, 3), (2, 3), (4, 4)] {
        let mut r = base().memory(before).start();
        let h = r.place_on_field(0, ID, Some(0));
        r.game
            .enqueue_triggered(EffectTiming::StartOfYourTurn, TriggerSource::Permanent(h));
        r.game.drain_effect_queue();
        assert_eq!(r.memory(), after);
    }
}
#[test]
fn bt22_085_on_play_filters_and_dp_expiry() {
    let mut r = base().hand(0, &[ID]).start();
    let h = r.place_on_field(0, "VEED", Some(0));
    r.place_on_field(0, "OTHER", Some(0));
    r.place_on_field(0, "FAKE", Some(0));
    r.place_on_field(1, "VEED", Some(0));
    r.play(0, 0).unwrap();
    assert_eq!(r.pending_action_count(), 1);
    choose(&mut r);
    assert_eq!(r.effective_dp(h), Some(8000));
    r.pass_turn();
    assert_eq!(r.effective_dp(h), Some(8000));
    r.pass_turn();
    r.auto_resolve().unwrap();
    assert_eq!(r.effective_dp(h), Some(5000));
}
#[test]
fn bt22_085_return_cost_preserves_shifted_attacker_and_expires() {
    let mut r = base().start();
    r.place_on_field(0, ID, Some(0));
    let h = r.place_on_field(0, "VEED", Some(0));
    r.place_on_field(0, "OTHER", Some(0));
    attack_event(&mut r, h);
    assert!(r.pending_is_optional());
    choose(&mut r);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.battle_area_size(0), 2);
    assert!(r.game.has_keyword(r.perm_handle(0, 0), Keyword::Jamming));
    assert!(!r.game.has_keyword(r.perm_handle(0, 1), Keyword::Jamming));
    r.pass_turn();
    assert!(!r.game.has_keyword(r.perm_handle(0, 0), Keyword::Jamming));
}
#[test]
fn bt22_085_decline_and_failed_payment_do_not_grant_jamming() {
    for prevent in [false, true] {
        let mut r = base().start();
        let t = r.place_on_field(0, ID, Some(0));
        let h = r.place_on_field(0, "VEED", Some(0));
        attack_event(&mut r, h);
        assert!(r.pending_is_optional());
        if prevent {
            r.game.modifiers.add(
                t,
                ModifierEntry::simple(
                    ModifierType::CannotBeReturnedToHand,
                    0,
                    Expiry::Permanent,
                    0,
                ),
            );
            choose(&mut r);
        } else {
            r.execute_action(0, digimon_engine::action::space::PASS)
                .unwrap();
        }
        assert_eq!(r.hand_size(0), 0);
        assert_eq!(r.battle_area_size(0), 2);
        assert!(!r.game.has_keyword(h, Keyword::Jamming));
    }
}
#[test]
fn bt22_085_attack_requires_own_veedramon_on_own_turn() {
    for (id, owner, turn) in [
        ("OTHER", 0, 0),
        ("FAKE", 0, 0),
        ("VEED", 1, 0),
        ("VEED", 0, 1),
    ] {
        let mut r = base().start();
        r.place_on_field(0, ID, Some(0));
        let h = r.place_on_field(owner, id, Some(0));
        r.game.turn_player_idx = turn;
        attack_event(&mut r, h);
        assert!(r.pending_selection().is_none());
        assert_eq!(r.hand_size(0), 0);
    }
}
#[test]
fn bt22_085_security_plays_without_cost() {
    let mut r = base().security(1, &[ID]).start();
    let h = r.place_on_field(0, "OTHER", Some(0));
    let before = r.memory();
    r.attack_player(h, 1, false);
    r.auto_resolve().unwrap();
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        ID
    );
    assert_eq!(r.memory(), before);
}
