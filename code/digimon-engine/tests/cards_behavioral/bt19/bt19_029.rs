//! BT19-029 Tapirmon: printed image and official bundle agree; DCGO BT19_029.cs.
//! Optional security payment on on_play and inherited self-only leave prevention.
use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD: &str = "BT19-029";

struct OpponentDeletesCarrier;
impl digimon_engine::effect::CardEffect for OpponentDeletesCarrier {
    fn effects(
        &self,
        card: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        vec![digimon_engine::effect::Effect::on_play(card)
            .process(|ctx| {
                ctx.delete_permanent(PermanentHandle {
                    player: 0,
                    index: 0,
                });
            })
            .build()]
    }
}
struct RejectOpponentSecurityTrash;
impl digimon_engine::effect::CardEffect for RejectOpponentSecurityTrash {
    fn effects(
        &self,
        card: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        vec![digimon_engine::effect::Effect::when_would_be_trashed(card)
            .replacement_condition(|ctx, subject| {
                matches!(
                    subject,
                    digimon_engine::replacement::ReplacementSubject::Card(
                        _,
                        digimon_engine::enums::Zone::Security
                    )
                ) && ctx.replacement_cause() == Some(ReplacementCause::OpponentEffect)
            })
            .replacement_process(|ctx| ctx.cancel())
            .build()]
    }
}
#[test]
fn bt19_029_security_payment_is_owners_effect_inside_opponents_deletion() {
    let (mut r, carrier) = inherited("DATA", &["SEC"]);
    r.register_effect("PLAIN", std::sync::Arc::new(RejectOpponentSecurityTrash));
    r.place_on_field(0, "PLAIN", Some(0));
    r.register_effect("RED", std::sync::Arc::new(OpponentDeletesCarrier));
    let opponent = r.place_on_field(1, "RED", Some(0));
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(opponent));
    r.game.drain_effect_queue();
    assert_optional_mask(&r);
    r.accept_optional_trigger().unwrap();
    r.auto_resolve().unwrap();
    assert!(
        r.game.players[0].security.is_empty(),
        "own payment cannot be canceled by an opponent-effect-only shield"
    );
    assert_eq!(
        r.game.players[0].battle_area[carrier.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "DATA"
    );
}

fn setup(security: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card(CARD)
        .expect("production Tapirmon YAML");
    for (id, color, traits) in [
        ("DATA", CardColor::Yellow, vec!["Data"]),
        ("WITCH", CardColor::Yellow, vec!["Witchelny"]),
        ("RED", CardColor::Red, vec!["Data", "Witchelny"]),
        ("PLAIN", CardColor::Yellow, vec!["Beast"]),
        ("FILL", CardColor::Yellow, vec!["Data"]),
        ("SEC", CardColor::Yellow, vec!["Data"]),
    ] {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Digimon;
        c.colors = vec![color];
        c.traits = traits.into_iter().map(String::from).collect();
        c.level = Some(5);
        c.dp = Some(6000);
        b = b.add_card(c);
    }
    let mut egg = make_test_card("EGG", "Yellow egg");
    egg.card_kind = CardKind::DigiEgg;
    egg.colors = vec![CardColor::Yellow];
    egg.level = Some(2);
    b = b.add_card(egg);
    let mut tamer = make_test_card("TAMER", "Witchelny Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.colors = vec![CardColor::Yellow];
    tamer.traits = vec!["Witchelny".into(), "Data".into()];
    tamer.level = None;
    tamer.dp = None;
    b = b.add_card(tamer);
    let mut r = b
        .security(0, security)
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .memory(3)
        .start();
    r.skip_mulligan();
    r
}

fn play_trigger(r: &mut DebugRunner) {
    let h = r.place_on_field(0, CARD, Some(0));
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

fn assert_optional_mask(r: &DebugRunner) {
    let v = r
        .pending_selection_view()
        .expect("explicit payment decision");
    assert_eq!(v.kind, SelectionKind::Replacement);
    assert!(v.is_optional);
    let mask = build_action_mask(&r.game, v.selecting_player);
    assert_eq!(mask[PASS as usize], 1.0);
    assert!(v.valid_action_ids.iter().any(|a| *a != PASS));
    for a in v.valid_action_ids {
        assert_eq!(mask[a as usize], 1.0);
    }
}

#[test]
fn bt19_029_on_play_accept_pays_top_then_gains_memory() {
    let mut r = setup(&["FILL", "SEC"]);
    let hand = r.game.players[0].hand.len();
    let memory = r.game.memory;
    play_trigger(&mut r);
    assert_optional_mask(&r);
    assert_eq!(r.game.players[0].security.len(), 2, "cost waits for choice");
    r.accept_optional_trigger().unwrap();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.game.players[0].security.len(), 1);
    assert_eq!(
        r.game.players[0].security[0].card_id(&r.game.card_data),
        "FILL"
    );
    assert_eq!(
        r.game.players[0]
            .trash
            .last()
            .unwrap()
            .card_id(&r.game.card_data),
        "SEC"
    );
    assert_eq!(r.game.players[0].hand.len(), hand);
    assert_eq!(r.game.memory, memory + 1);
}

#[test]
fn bt19_029_on_play_decline_preserves_security_hand_and_memory() {
    let mut r = setup(&["SEC"]);
    let hand = r.game.players[0].hand.len();
    let memory = r.game.memory;
    play_trigger(&mut r);
    assert_optional_mask(&r);
    r.decline_optional_trigger().unwrap();
    assert_eq!(r.game.players[0].security.len(), 1);
    assert_eq!(r.game.players[0].hand.len(), hand);
    assert_eq!(r.game.memory, memory);
}

#[test]
fn bt19_029_empty_security_cannot_pay_or_receive_benefits() {
    let mut r = setup(&[]);
    let hand = r.game.players[0].hand.len();
    let memory = r.game.memory;
    play_trigger(&mut r);
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.game.players[0].hand.len(), hand);
    assert_eq!(r.game.memory, memory);
}

fn inherited(carrier: &str, security: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = setup(security);
    let h = r.place_stack(0, &[CARD, carrier]);
    (r, h)
}

#[test]
fn bt19_029_inherited_yellow_data_or_witchelny_pays_security_to_stay() {
    for carrier in ["DATA", "WITCH"] {
        let (mut r, h) = inherited(carrier, &["FILL", "SEC"]);
        r.game
            .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
        assert_optional_mask(&r);
        r.accept_optional_trigger().unwrap();
        assert_eq!(r.game.players[0].battle_area.len(), 1);
        assert_eq!(r.game.players[0].security.len(), 1);
        assert_eq!(
            r.game.players[0]
                .trash
                .last()
                .unwrap()
                .card_id(&r.game.card_data),
            "SEC"
        );
    }
}

#[test]
fn bt19_029_inherited_rejects_wrong_color_or_trait() {
    for carrier in ["RED", "PLAIN"] {
        let (mut r, h) = inherited(carrier, &["SEC"]);
        r.game
            .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
        assert!(r.pending_selection_view().is_none());
        assert!(r.game.players[0].battle_area.is_empty());
        assert_eq!(r.game.players[0].security.len(), 1);
    }
}

#[test]
fn bt19_029_inherited_rejects_battle_own_effect_and_unpayable_cost() {
    for cause in [ReplacementCause::Battle, ReplacementCause::OwnEffect] {
        let (mut r, h) = inherited("DATA", &["SEC"]);
        r.game.delete_permanent_with_cause(h, cause);
        assert!(r.pending_selection_view().is_none());
        assert!(r.game.players[0].battle_area.is_empty());
        assert_eq!(r.game.players[0].security.len(), 1);
    }
    let (mut r, h) = inherited("DATA", &[]);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none());
    assert!(r.game.players[0].battle_area.is_empty());
}

#[test]
fn bt19_029_inherited_decline_allows_deletion_without_cost() {
    let (mut r, h) = inherited("DATA", &["SEC"]);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    assert_optional_mask(&r);
    r.decline_optional_trigger().unwrap();
    assert!(r.game.players[0].battle_area.is_empty());
    assert_eq!(r.game.players[0].security.len(), 1);
}

#[test]
fn bt19_029_inherited_only_protects_its_carrier() {
    let (mut r, _) = inherited("DATA", &["SEC"]);
    let other = r.place_on_field(0, "WITCH", Some(0));
    r.game
        .delete_permanent_with_cause(other, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.game.players[0].security.len(), 1);
}

#[test]
fn bt19_029_inherited_once_per_turn_and_resets_on_next_turn() {
    let (mut r, h) = inherited("DATA", &["SEC", "SEC", "SEC"]);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    r.accept_optional_trigger().unwrap();
    let mut next_turn = setup(&[]);
    next_turn.game = r.game.clone();
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none());
    assert!(r.game.players[0].battle_area.is_empty());
    next_turn.end_turn();
    next_turn
        .game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    assert_optional_mask(&next_turn);
}

// A controlled security replacement isolates payment success and parked continuations.
struct CancelSecurityLoss {
    optional: bool,
}
impl digimon_engine::effect::CardEffect for CancelSecurityLoss {
    fn effects(
        &self,
        card: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        let b = digimon_engine::effect::Effect::when_would_be_trashed(card)
            .replacement_process(|rctx| rctx.cancel());
        vec![if self.optional {
            b.optional().build()
        } else {
            b.build()
        }]
    }
}
fn install_security_guard(r: &mut DebugRunner, optional: bool) {
    r.register_effect(
        "PLAIN",
        std::sync::Arc::new(CancelSecurityLoss { optional }),
    );
    r.place_on_field(0, "PLAIN", Some(0));
}

#[test]
fn bt19_029_canceled_security_payment_gives_no_memory() {
    let mut r = setup(&["SEC"]);
    install_security_guard(&mut r, false);
    let hand = r.game.players[0].hand.len();
    let memory = r.game.memory;
    play_trigger(&mut r);
    r.accept_optional_trigger().unwrap();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.game.players[0].security.len(), 1);
    assert_eq!(
        r.game.players[0].hand.len(),
        hand,
        "unpaid condition leaves hand unchanged"
    );
    assert_eq!(r.game.memory, memory, "unpaid by-condition gives no memory");
}

#[test]
fn bt19_029_parked_security_payment_waits_for_result_before_benefits() {
    let mut r = setup(&["SEC"]);
    install_security_guard(&mut r, true);
    let hand = r.game.players[0].hand.len();
    let memory = r.game.memory;
    play_trigger(&mut r);
    r.accept_optional_trigger().unwrap();
    assert_optional_mask(&r);
    assert_eq!(
        r.game.players[0].hand.len(),
        hand,
        "no hand change while payment is parked"
    );
    assert_eq!(r.game.memory, memory, "no memory while payment is parked");
    let mut canceled = setup(&[]);
    canceled.game = r.game.clone();
    canceled.accept_optional_trigger().unwrap();
    assert_eq!(canceled.game.players[0].security.len(), 1);
    assert_eq!(canceled.game.players[0].hand.len(), hand);
    assert_eq!(canceled.game.memory, memory);
    r.decline_optional_trigger().unwrap();
    assert!(r.pending_selection_view().is_none());
    assert!(r.game.players[0].security.is_empty());
    assert_eq!(r.game.players[0].hand.len(), hand);
    assert_eq!(r.game.memory, memory + 1);
}

#[test]
fn bt19_029_inherited_canceled_security_cost_does_not_save_carrier() {
    let (mut r, h) = inherited("DATA", &["SEC"]);
    install_security_guard(&mut r, false);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    r.accept_optional_trigger().unwrap();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.game.players[0].security.len(), 1);
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "only the security guard remains"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "PLAIN"
    );
}

#[test]
fn bt19_029_inherited_parked_security_cost_saves_only_after_payment() {
    let (mut r, h) = inherited("DATA", &["SEC"]);
    install_security_guard(&mut r, true);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    r.accept_optional_trigger().unwrap();
    assert_optional_mask(&r);
    let mut canceled = setup(&[]);
    canceled.game = r.game.clone();
    canceled.accept_optional_trigger().unwrap();
    assert!(canceled.pending_selection_view().is_none());
    assert_eq!(canceled.game.players[0].security.len(), 1);
    assert_eq!(canceled.game.players[0].battle_area.len(), 1);
    assert_eq!(
        canceled.game.players[0].battle_area[0]
            .top_card()
            .card_id(&canceled.game.card_data),
        "PLAIN"
    );
    r.decline_optional_trigger().unwrap();
    assert!(r.pending_selection_view().is_none());
    assert!(r.game.players[0].security.is_empty());
    assert_eq!(
        r.game.players[0].battle_area.len(),
        2,
        "paid save leaves carrier and guard"
    );
}

struct RecoverDiscardedSecurity;
impl digimon_engine::effect::CardEffect for RecoverDiscardedSecurity {
    fn effects(
        &self,
        card: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        vec![digimon_engine::effect::Effect::on_discard_security(card)
            .process(|ctx| {
                ctx.recover_from_deck(ctx.player, 1);
            })
            .build()]
    }
}
#[test]
fn bt19_029_security_recovery_observer_does_not_erase_successful_payment() {
    let mut r = setup(&["SEC"]);
    r.register_effect("SEC", std::sync::Arc::new(RecoverDiscardedSecurity));
    let hand = r.game.players[0].hand.len();
    let memory = r.game.memory;
    play_trigger(&mut r);
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(
        r.game.players[0].security.len(),
        1,
        "discarded security recovers a replacement card"
    );
    assert_eq!(
        r.game.players[0].hand.len(),
        hand,
        "committed trash paid even if security count recovered"
    );
    assert_eq!(r.game.memory, memory + 1);
    assert!(r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "SEC"));
}

#[test]
fn bt19_029_inherited_does_not_protect_a_yellow_witchelny_tamer() {
    let (mut r, h) = inherited("TAMER", &["SEC"]);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    assert!(
        r.pending_selection_view().is_none(),
        "printed carrier must be a Digimon"
    );
    assert!(r.game.players[0].battle_area.is_empty());
    assert_eq!(r.game.players[0].security.len(), 1);
}

#[test]
fn bt19_029_normal_play_uses_printed_three_cost_and_optional_payment() {
    let mut r = setup(&["SEC"]);
    r.game.current_phase = digimon_engine::enums::GamePhase::Main;
    r.game.memory = 10;
    let index = r.add_to_hand(0, CARD);
    let a = digimon_engine::action::space::PLAY_HAND_START + index as u16;
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    r.game.decode_action(a, 0);
    assert_optional_mask(&r);
    assert_eq!(r.game.memory, 7);
    r.accept_optional_trigger().unwrap();
    assert_eq!(r.game.memory, 8);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        CARD
    );
}

#[test]
fn bt19_029_yellow_level_two_evolution_is_zero_cost_and_does_not_trigger_on_play() {
    let mut r = setup(&["SEC"]);
    r.game.current_phase = digimon_engine::enums::GamePhase::Main;
    let h = r.place_on_field(0, "EGG", Some(0));
    let index = r.add_to_hand(0, CARD);
    let a = digimon_engine::action::space::encode_digivolve(index as u16, h.index as u16);
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    let memory = r.game.memory;
    let hand = r.game.players[0].hand.len();
    r.game.decode_action(a, 0);
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.game.memory, memory);
    assert_eq!(
        r.game.players[0].hand.len(),
        hand,
        "one card evolved and one evolution draw"
    );
    assert_eq!(r.game.players[0].security.len(), 1);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        CARD
    );
}

#[test]
fn bt19_029_payable_condition_can_be_chosen_when_memory_gain_is_prohibited() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut r = setup(&["SEC"]);
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::CannotGainMemoryByEffect,
            0,
            Expiry::Permanent,
            None,
            1,
        ),
    );
    let memory = r.game.memory;
    play_trigger(&mut r);
    assert_optional_mask(&r);
    r.accept_optional_trigger().unwrap();
    assert!(r.game.players[0].security.is_empty());
    assert_eq!(
        r.game.memory, memory,
        "CR4.3 15-7-5 permits payment without downstream benefit"
    );
}

#[test]
fn bt19_029_inherited_stops_opponent_hand_and_deck_returns() {
    for deck in [false, true] {
        let (mut r, h) = inherited("DATA", &["SEC"]);
        if deck {
            r.game.return_to_deck_from_effect(h, 1);
        } else {
            r.game.return_to_hand_from_effect(h, 1);
        }
        assert_optional_mask(&r);
        r.accept_optional_trigger().unwrap();
        assert_eq!(r.game.players[0].battle_area.len(), 1);
        assert!(r.game.players[0].security.is_empty());
        assert_eq!(
            r.game.players[0].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            "DATA"
        );
    }
}

#[test]
fn bt19_029_inherited_decline_does_not_spend_another_carriers_opt() {
    let (mut r, h) = inherited("DATA", &["SEC"]);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    assert_optional_mask(&r);
    r.decline_optional_trigger().unwrap();
    let other = r.place_stack(0, &[CARD, "DATA"]);
    r.game
        .delete_permanent_with_cause(other, ReplacementCause::OpponentEffect);
    assert_optional_mask(&r);
    r.accept_optional_trigger().unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert!(r.game.players[0].security.is_empty());
}
