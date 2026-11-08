//! BT18-039 printed image: Barrier; optional security payment, ANY Digimon base DP;
//! inherited security removal MAY unsuspend, once per turn (API text loses may).
use digimon_engine::action::{
    build_action_mask,
    space::{encode_attack, encode_digivolve, PASS, PLAY_HAND_START},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Expiry, GamePhase, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;
const CARD: &str = "BT18-039";
fn setup(sec: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card(CARD)
        .expect("production Mistymon");
    for (id, kind, dp, lv) in [
        ("LOW", CardKind::Digimon, Some(4000), Some(4)),
        ("HIGH", CardKind::Digimon, Some(9000), Some(6)),
        ("NONE", CardKind::Digimon, None, Some(4)),
        ("TAMER", CardKind::Tamer, None, None),
        ("FILL", CardKind::Digimon, Some(3000), Some(3)),
    ] {
        let mut c = make_test_card(id, id);
        c.card_kind = kind;
        c.dp = dp;
        c.level = lv;
        c.colors = vec![CardColor::Yellow];
        c.play_cost = 3;
        b = b.add_card(c);
    }
    let mut r = b
        .security(0, sec)
        .security(1, &["FILL"; 3])
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r
}
fn fire(r: &mut DebugRunner, h: PermanentHandle, t: EffectTiming) {
    r.game.enqueue_triggered(t, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
fn mask(r: &DebugRunner) {
    let v = r.pending_selection_view().expect("player choice");
    let m = build_action_mask(&r.game, v.selecting_player);
    for a in v.valid_action_ids {
        assert_eq!(m[a as usize], 1.0);
    }
    if v.is_optional {
        assert_eq!(m[PASS as usize], 1.0);
    }
}
fn accept(r: &mut DebugRunner) {
    mask(r);
    r.accept_optional_trigger().unwrap();
}
fn decline(r: &mut DebugRunner) {
    mask(r);
    r.decline_optional_trigger().unwrap();
}
fn pick(r: &mut DebugRunner, h: PermanentHandle) {
    mask(r);
    let v = r.pending_selection_view().unwrap();
    let a = encode_attack(h.player as u16, h.index as u16);
    assert!(v.valid_action_ids.contains(&a), "{v:?}");
    r.execute_action(v.selecting_player, a).unwrap();
}
fn main_trigger(sec: &[&str]) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = setup(sec);
    let me = r.place_on_field(0, CARD, Some(0));
    let opp = r.place_on_field(1, "HIGH", Some(0));
    fire(&mut r, me, EffectTiming::OnPlay);
    (r, me, opp)
}
#[test]
fn bt18_039_decline_and_empty_security() {
    let (mut r, _, opp) = main_trigger(&["FILL"]);
    decline(&mut r);
    assert_eq!(r.security_count(0), 1);
    assert_eq!(r.effective_dp(opp), Some(9000));
    let (r, _, opp) = main_trigger(&[]);
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.effective_dp(opp), Some(9000));
}
#[test]
fn bt18_039_any_battle_digimon_mask_and_base_dp() {
    let mut r = setup(&["FILL", "HIGH"]);
    let me = r.place_on_field(0, CARD, Some(0));
    let own = r.place_on_field(0, "LOW", Some(0));
    r.place_on_field(0, "TAMER", Some(0));
    let opp = r.place_on_field(1, "HIGH", Some(0));
    r.place_on_field(1, "TAMER", Some(0));
    r.place_in_breeding(0, "LOW");
    r.place_in_breeding(1, "LOW");
    fire(&mut r, me, EffectTiming::OnPlay);
    accept(&mut r);
    let v = r.pending_selection_view().unwrap();
    assert!(!v.is_optional);
    let mut actual = v.valid_action_ids.clone();
    actual.sort();
    let mut expected = vec![
        encode_attack(0, me.index as u16),
        encode_attack(0, own.index as u16),
        encode_attack(1, opp.index as u16),
    ];
    expected.sort();
    assert_eq!(actual, expected);
    let mut other = setup(&[]);
    other.game = r.game.clone();
    pick(&mut other, own);
    assert_eq!(other.effective_dp(own), Some(6000));
    pick(&mut r, opp);
    assert_eq!(r.effective_dp(opp), Some(6000));
    assert_eq!(r.security_count(0), 1);
    assert_eq!(
        r.game.players[0]
            .trash
            .last()
            .unwrap()
            .card_id(&r.game.card_data),
        "HIGH"
    );
}
#[test]
fn bt18_039_additive_dp_and_opponent_turn_expiry() {
    let (mut r, _, opp) = main_trigger(&["FILL"]);
    r.game.modifiers.add(
        opp,
        ModifierEntry::simple(ModifierType::ChangeDp, 2000, Expiry::Permanent, 1),
    );
    accept(&mut r);
    pick(&mut r, opp);
    assert_eq!(r.effective_dp(opp), Some(8000));
    r.end_turn();
    assert_eq!(r.effective_dp(opp), Some(8000));
    r.end_turn();
    assert_eq!(r.effective_dp(opp), Some(11000));
}
#[test]
fn bt18_039_dpless_target_remains_without_dp() {
    let mut r = setup(&["FILL"]);
    let me = r.place_on_field(0, CARD, Some(0));
    let no = r.place_stack(1, &["TAMER", "HIGH"]);
    fire(&mut r, me, EffectTiming::OnPlay);
    accept(&mut r);
    pick(&mut r, no);
    let c = r.game.players[0].battle_area[me.index as usize]
        .top_card()
        .handle();
    let mut ctx = digimon_engine::effect_context::EffectContext::new(&mut r.game, c, Some(me), 0);
    assert_eq!(ctx.de_digivolve(no, Some(3), Some(1)), 1);
    assert_eq!(
        r.game.players[1].battle_area[no.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "TAMER"
    );
    assert_eq!(
        r.effective_dp(no),
        None,
        "base DP change must not manufacture DP"
    );
    assert_eq!(r.security_count(0), 0);
}
#[test]
fn bt18_039_real_play_and_digivolution_routes() {
    for evolve in [false, true] {
        let mut r = setup(&["FILL"]);
        let base = r.place_on_field(0, "LOW", Some(0));
        let opp = r.place_on_field(1, "HIGH", Some(0));
        let slot = r.add_to_hand(0, CARD);
        let a = if evolve {
            encode_digivolve(slot as u16, base.index as u16)
        } else {
            PLAY_HAND_START + slot as u16
        };
        assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
        r.game.decode_action(a, 0);
        accept(&mut r);
        pick(&mut r, opp);
        assert_eq!(r.effective_dp(opp), Some(6000));
    }
}
#[test]
fn bt18_039_inherited_optional_decline_opt_and_carrier_only() {
    let mut r = setup(&["FILL"; 4]);
    let h = r.place_stack(0, &[CARD, "HIGH"]);
    let other = r.place_on_field(0, "LOW", Some(0));
    r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    r.game.players[0].battle_area[other.index as usize].is_suspended = true;
    fire(&mut r, h, EffectTiming::OnOwnSecurityRemoved);
    decline(&mut r);
    assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
    fire(&mut r, h, EffectTiming::OnOwnSecurityRemoved);
    accept(&mut r);
    assert!(!r.game.players[0].battle_area[h.index as usize].is_suspended);
    assert!(r.game.players[0].battle_area[other.index as usize].is_suspended);
    r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    fire(&mut r, h, EffectTiming::OnOwnSecurityRemoved);
    assert!(r.pending_selection_view().is_none());
    assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
}
#[test]
fn bt18_039_inherited_wrong_owner_and_top_card_do_not_trigger() {
    let mut r = setup(&["FILL"]);
    let h = r.place_stack(0, &[CARD, "HIGH"]);
    r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    fire(&mut r, h, EffectTiming::OnOpponentSecurityRemoved);
    assert!(r.pending_selection_view().is_none());
    let top = r.place_on_field(0, CARD, Some(0));
    r.game.players[0].battle_area[top.index as usize].is_suspended = true;
    fire(&mut r, top, EffectTiming::OnOwnSecurityRemoved);
    assert!(r.pending_selection_view().is_none());
}

struct SecurityGuard {
    optional: bool,
    redirect: bool,
}
impl digimon_engine::effect::CardEffect for SecurityGuard {
    fn effects(
        &self,
        c: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        let redirect = self.redirect;
        let b = digimon_engine::effect::Effect::when_would_be_trashed(c)
            .replacement_condition(|_, s| {
                matches!(
                    s,
                    digimon_engine::replacement::ReplacementSubject::Card(
                        _,
                        digimon_engine::enums::Zone::Security
                    )
                )
            })
            .replacement_process(move |ctx| {
                if redirect {
                    ctx.redirect_to(digimon_engine::enums::Zone::Hand)
                } else {
                    ctx.cancel()
                }
            });
        vec![if self.optional {
            b.optional().build()
        } else {
            b.build()
        }]
    }
}
fn guarded(optional: bool, redirect: bool) -> (DebugRunner, PermanentHandle) {
    let mut r = setup(&["FILL"]);
    r.register_effect(
        "TAMER",
        std::sync::Arc::new(SecurityGuard { optional, redirect }),
    );
    r.place_on_field(0, "TAMER", Some(0));
    let me = r.place_on_field(0, CARD, Some(0));
    let opp = r.place_on_field(1, "HIGH", Some(0));
    fire(&mut r, me, EffectTiming::OnPlay);
    accept(&mut r);
    (r, opp)
}
#[test]
fn bt18_039_canceled_and_redirected_cost_never_change_dp() {
    for redirect in [false, true] {
        let (r, opp) = guarded(false, redirect);
        assert!(r.pending_selection_view().is_none());
        assert_eq!(r.effective_dp(opp), Some(9000));
        assert_eq!(r.security_count(0), if redirect { 0 } else { 1 });
    }
}
#[test]
fn bt18_039_nested_replacement_clone_waits_for_payment() {
    let (mut r, opp) = guarded(true, false);
    mask(&r);
    assert_eq!(r.effective_dp(opp), Some(9000));
    let mut canceled = setup(&[]);
    canceled.game = r.game.clone();
    accept(&mut canceled);
    assert!(canceled.pending_selection_view().is_none());
    assert_eq!(canceled.effective_dp(opp), Some(9000));
    assert_eq!(canceled.security_count(0), 1);
    decline(&mut r);
    assert_eq!(r.security_count(0), 0);
    pick(&mut r, opp);
    assert_eq!(r.effective_dp(opp), Some(6000));
}
struct RemoveSecurity {
    owner: u8,
}
impl digimon_engine::effect::CardEffect for RemoveSecurity {
    fn effects(
        &self,
        c: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        let owner = self.owner;
        vec![digimon_engine::effect::Effect::on_play(c)
            .process(move |ctx| {
                ctx.trash_top_security(owner);
            })
            .build()]
    }
}
#[test]
fn bt18_039_real_security_event_owner_gate_and_next_turn_reset() {
    let mut r = setup(&["FILL"; 4]);
    let h = r.place_stack(0, &[CARD, "HIGH"]);
    r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    r.register_effect("LOW", std::sync::Arc::new(RemoveSecurity { owner: 1 }));
    let actor = r.place_on_field(0, "LOW", Some(0));
    fire(&mut r, actor, EffectTiming::OnPlay);
    assert!(r.pending_selection_view().is_none());
    assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
    r.register_effect("TAMER", std::sync::Arc::new(RemoveSecurity { owner: 0 }));
    let actor = r.place_on_field(0, "TAMER", Some(0));
    fire(&mut r, actor, EffectTiming::OnPlay);
    accept(&mut r);
    assert!(!r.game.players[0].battle_area[h.index as usize].is_suspended);
    r.end_turn();
    r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    fire(&mut r, actor, EffectTiming::OnPlay);
    accept(&mut r);
    assert!(!r.game.players[0].battle_area[h.index as usize].is_suspended);
}

#[test]
fn bt18_039_barrier_is_printed_main_keyword() {
    let mut r = setup(&["FILL"]);
    let h = r.place_on_field(0, CARD, Some(0));
    assert!(r
        .game
        .has_keyword(h, digimon_engine::enums::Keyword::Barrier));
}

#[test]
fn bt18_039_inherited_requires_digimon_host() {
    let mut r = setup(&["FILL"]);
    let h = r.place_stack(0, &[CARD, "TAMER"]);
    r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    fire(&mut r, h, EffectTiming::OnOwnSecurityRemoved);
    assert!(
        r.pending_selection_view().is_none(),
        "this Digimon cannot unsuspend a Tamer host"
    );
    assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
}
