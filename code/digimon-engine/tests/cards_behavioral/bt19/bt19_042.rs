//! BT19-042, image and official bundle checked against DCGO BT19_042.cs.
use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};
const ID: &str = "BT19-042";
fn setup(source: Option<&str>, own: usize, opponent: usize) -> (DebugRunner, PermanentHandle) {
    let mut b = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT19-042 production YAML");
    for (id, name) in [
        ("DYN", "Dynasmon"),
        ("X", "X Antibody"),
        ("NEAR", "Dynasmon (X Antibody)"),
        ("FILL", "Filler"),
    ] {
        let mut c = make_test_card(id, name);
        c.card_kind = CardKind::Digimon;
        c.colors = vec![CardColor::Yellow];
        c.level = Some(5);
        c.dp = Some(6000);
        b = b.add_card(c);
    }
    let own_cards = vec!["FILL"; own];
    let opp_cards = vec!["FILL"; opponent];
    let mut r = b
        .security(0, &own_cards)
        .security(1, &opp_cards)
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .memory(3)
        .start();
    r.skip_mulligan();
    let h = if let Some(s) = source {
        r.place_stack(0, &[s, ID])
    } else {
        r.place_on_field(0, ID, Some(0))
    };
    (r, h)
}
fn fire(r: &mut DebugRunner, h: PermanentHandle, t: EffectTiming) {
    r.game.enqueue_triggered(t, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
fn payment_mask(r: &DebugRunner) {
    let p = r
        .pending_selection_view()
        .expect("optional security payment");
    assert_eq!(p.kind, SelectionKind::Replacement);
    assert!(p.is_optional);
    let mask = build_action_mask(&r.game, p.selecting_player);
    assert_eq!(mask[PASS as usize], 1.0);
    assert!(p.valid_action_ids.iter().any(|a| *a != PASS));
    for a in p.valid_action_ids {
        assert_eq!(mask[a as usize], 1.0);
    }
}
#[test]
fn bt19_042_keywords_and_digivolution_circles() {
    let (r, h) = setup(None, 3, 3);
    assert!(r.game.has_keyword(h, Keyword::Raid));
    assert!(r.game.has_keyword(h, Keyword::Blocker));
    let pack = digimon_engine::dsl_registry::from_embedded_cached().unwrap();
    let card = pack.lookup(ID).unwrap();
    assert_eq!(card.alt_paths.len(), 3);
    assert_eq!(r.effective_dp(h), Some(12000));
}
#[test]
fn bt19_042_digivolving_pays_security_then_trashes_opponent_and_buffs() {
    let (mut r, h) = setup(Some("DYN"), 3, 3);
    let own = r.game.players[0].security.last().unwrap().handle();
    let opp = r.game.players[1].security.last().unwrap().handle();
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    payment_mask(&r);
    assert_eq!(r.game.players[0].security.len(), 3);
    assert_eq!(r.effective_dp(h), Some(12000));
    r.accept_optional_trigger().unwrap();
    assert_eq!(r.game.players[0].security.len(), 2);
    assert_eq!(r.game.players[1].security.len(), 2);
    assert_eq!(r.game.players[0].trash.last().unwrap().handle(), own);
    assert_eq!(r.game.players[1].trash.last().unwrap().handle(), opp);
    assert_eq!(r.effective_dp(h), Some(18000));
}
#[test]
fn bt19_042_attacking_with_x_antibody_source_can_pay() {
    let (mut r, h) = setup(Some("X"), 1, 1);
    fire(&mut r, h, EffectTiming::WhenAttacking);
    payment_mask(&r);
    r.accept_optional_trigger().unwrap();
    assert!(r.game.players[0].security.is_empty());
    assert!(r.game.players[1].security.is_empty());
    assert_eq!(r.effective_dp(h), Some(18000));
}
#[test]
fn bt19_042_decline_keeps_security_and_does_not_spend_shared_opt() {
    let (mut r, h) = setup(Some("DYN"), 3, 3);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    payment_mask(&r);
    r.decline_optional_trigger().unwrap();
    assert_eq!(r.game.players[0].security.len(), 3);
    assert_eq!(r.game.players[1].security.len(), 3);
    assert_eq!(r.effective_dp(h), Some(12000));
    fire(&mut r, h, EffectTiming::WhenAttacking);
    payment_mask(&r);
}
#[test]
fn bt19_042_digivolving_and_attacking_share_once_per_turn() {
    for (first, second) in [
        (EffectTiming::WhenDigivolving, EffectTiming::WhenAttacking),
        (EffectTiming::WhenAttacking, EffectTiming::WhenDigivolving),
    ] {
        let (mut r, h) = setup(Some("DYN"), 3, 3);
        fire(&mut r, h, first);
        r.accept_optional_trigger().unwrap();
        fire(&mut r, h, second);
        assert!(r.pending_selection_view().is_none());
        assert_eq!(r.game.players[0].security.len(), 2);
        assert_eq!(r.game.players[1].security.len(), 2);
        assert_eq!(r.effective_dp(h), Some(18000));
    }
}
#[test]
fn bt19_042_requires_exact_source_name_and_payable_security() {
    for (source, security) in [
        (None, 3),
        (Some("NEAR"), 3),
        (Some("FILL"), 3),
        (Some("DYN"), 0),
    ] {
        let (mut r, h) = setup(source, security, 3);
        fire(&mut r, h, EffectTiming::WhenAttacking);
        assert!(r.pending_selection_view().is_none());
        assert_eq!(r.effective_dp(h), Some(12000));
        assert_eq!(r.game.players[0].security.len(), security);
        assert_eq!(r.game.players[1].security.len(), 3);
    }
}
#[test]
fn bt19_042_empty_opponent_security_does_not_prevent_paid_buff() {
    let (mut r, h) = setup(Some("DYN"), 1, 0);
    fire(&mut r, h, EffectTiming::WhenAttacking);
    r.accept_optional_trigger().unwrap();
    assert_eq!(r.effective_dp(h), Some(18000));
    assert!(r.game.players[0].security.is_empty());
}
#[test]
fn bt19_042_end_of_your_turn_recovers_only_at_two_or_fewer() {
    for n in [0, 1, 2, 3] {
        let (mut r, h) = setup(None, n, 3);
        let deck = r.game.players[0].deck.len();
        fire(&mut r, h, EffectTiming::EndOfYourTurn);
        assert!(r.pending_selection_view().is_none());
        assert_eq!(r.game.players[0].security.len(), n + usize::from(n <= 2));
        assert_eq!(r.game.players[0].deck.len(), deck - usize::from(n <= 2));
    }
}
#[test]
fn bt19_042_buff_lasts_until_opponent_turn_ends() {
    let (mut r, h) = setup(Some("DYN"), 5, 3);
    fire(&mut r, h, EffectTiming::WhenAttacking);
    r.accept_optional_trigger().unwrap();
    r.end_turn();
    assert_eq!(r.effective_dp(h), Some(18000));
    r.end_turn();
    assert_eq!(r.effective_dp(h), Some(12000));
}

#[test]
fn bt19_042_each_printed_digivolution_route_is_legal_and_pays_its_cost() {
    for (name, color, level, cost) in [
        ("Yellow ultimate", CardColor::Yellow, 5, 4),
        ("Red ultimate", CardColor::Red, 5, 4),
        ("Dynasmon", CardColor::Yellow, 6, 1),
    ] {
        let mut base = make_test_card("BASE", name);
        base.card_kind = CardKind::Digimon;
        base.colors = vec![color];
        base.level = Some(level);
        base.dp = Some(6000);
        let mut r = DebugRunner::builder()
            .dsl_card(ID)
            .unwrap()
            .add_card(base)
            .hand(0, &[ID])
            .deck(0, &["BASE"; 10])
            .deck(1, &["BASE"; 10])
            .memory(10)
            .start();
        r.skip_mulligan();
        r.game.current_phase = digimon_engine::enums::GamePhase::Main;
        let h = r.place_on_field(0, "BASE", Some(0));
        let action = digimon_engine::action::space::encode_digivolve(0, h.index as u16);
        let mask = build_action_mask(&r.game, 0);
        assert_eq!(mask[action as usize], 1.0);
        let before = r.game.memory;
        r.game.decode_action(action, 0);
        assert_eq!(r.game.memory, before - cost);
        assert_eq!(
            r.game.players[0].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            ID
        );
        assert!(
            r.pending_selection_view().is_none(),
            "empty security makes by-cost unavailable"
        );
    }
}
