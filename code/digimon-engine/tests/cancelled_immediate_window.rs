//! Q7274; general_rule.pdf 15-8-5-4, 16-24-4 [3.6 numbering; deleted in 4.3 — see changelog-3.6-to-4.3.md] and 16-29.
use digimon_engine::action::{
    mask::build_action_mask,
    space::{PASS, REPLACEMENT_ACCEPT},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, EffectTiming, Zone};
use digimon_engine::replacement::{ReplacementCause, ReplacementSubject};
use digimon_engine::selection::SelectionKind;
fn hydrate_printed(r: &mut DebugRunner) {
    for c in r
        .game
        .card_data
        .iter_mut()
        .filter(|c| c.card_id == "EX13-024")
    {
        c.effect_text = r###"＜Raid＞ ＜Blocker＞
[On Play] [When Digivolving] For each of this Digimon's digivolution cards, trash any 1 digivolution card from your opponent's Digimon. Then, you may return all of their Digimon with the fewest digivolution cards to the bottom of the deck.
[All Turns] [Once Per Turn] When any of your [Dracomon] or [Examon] text Digimon would leave the battle area, by suspending 1 of your such Digimon, they don't leave."###.to_string();
        c.inherited_text = r###"[All Turns] [Once Per Turn] When any of your [Dracomon] or [Examon] text Digimon would leave the battle area, by suspending 1 of your such Digimon, they don't leave."###.to_string();
    }
    for c in r
        .game
        .card_data
        .iter_mut()
        .filter(|c| c.card_id == "BT23-047")
    {
        c.effect_text = r###"＜Piercing＞ ＜Security A. +1＞ ＜Partition＞ (green Lv.5 & blue Lv.5)
[On Play] [When Digivolving] Suspend 5 of your opponent's Digimon or Tamers. None of your opponent's Digimon can unsuspend in their next unsuspend phase. Then, this Digimon may attack.
[Your Turn] [Once Per Turn] When your opponent's security stack is removed from, trash 1 of their Option cards in the battle area. Then, delete 1 of their suspended Digimon or Tamers."###.to_string();
        c.inherited_text = r###""###.to_string();
    }
}
fn fixture() -> DebugRunner {
    let mut green = make_test_card("GREEN5", "Green source");
    green.level = Some(5);
    green.colors = vec![CardColor::Green];
    let mut blue = make_test_card("BLUE5", "Blue source");
    blue.level = Some(5);
    blue.colors = vec![CardColor::Blue];
    let mut r = DebugRunner::builder()
        .dsl_card("EX13-024")
        .unwrap()
        .dsl_card("BT23-047")
        .unwrap()
        .add_card(green)
        .add_card(blue)
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .memory(3)
        .start();
    hydrate_printed(&mut r);
    r.skip_mulligan();
    r.place_stack(0, &["GREEN5", "BLUE5", "BT23-047"]);
    r.place_on_field(0, "EX13-024", None);
    r
}
fn act(r: &mut DebugRunner, a: u16) {
    let v = r.pending_selection_view().expect("required decision");
    assert_eq!(
        build_action_mask(&r.game, v.selecting_player)[a as usize],
        1.0
    );
    r.game.resolve_selection(v.selecting_player, a).unwrap();
}
fn order(r: &mut DebugRunner, text: &str) {
    let v = r.pending_selection_view().expect("order choice");
    assert_eq!(v.kind, SelectionKind::EffectChoice);
    let id = if text == "don't leave" {
        "EX13-024"
    } else if text == "Partition" {
        "BT23-047"
    } else {
        "NATIVE"
    };
    let handle = r
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == id)
        .unwrap()
        .top_card()
        .handle();
    let row = v
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .find(|e| e.source_card == Some(handle) && (id != "NATIVE" || e.label.contains(text)))
        .expect("candidate physical card identity");
    let a = row.action_id;
    act(r, a);
}

fn departure(r: &mut DebugRunner) {
    r.game.set_effect_source_player_for_test(Some(1));
    r.game
        .delete_permanent_with_effects(digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: 0,
        });
}
fn protect(r: &mut DebugRunner) {
    if r.pending_selection_view().unwrap().kind == SelectionKind::Replacement {
        act(r, REPLACEMENT_ACCEPT);
    }
    let v = r.pending_selection_view().unwrap();
    assert_eq!(v.kind, SelectionKind::OwnField);
    assert_eq!(
        v.valid_action_ids.len(),
        2,
        "both real text-qualified Digimon can pay"
    );
    let a = v.valid_action_ids[1];
    act(r, a);
}
fn ids(r: &DebugRunner) -> Vec<String> {
    r.game
        .player(0)
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}
fn partition_picks(r: &mut DebugRunner) {
    for _ in 0..2 {
        let Some(v) = r.pending_selection_view() else {
            break;
        };
        if !v.prompt.contains("<Partition>: select") {
            break;
        }
        assert!(matches!(
            v.kind,
            SelectionKind::CountCappedMultiSelect { min: 1, max: 1, .. }
        ));
        assert!(
            !v.valid_action_ids.contains(&PASS),
            "Partition requires a complete pair after consent"
        );
        // Choose the first exposed physical source; slot matching guarantees a complete pair.
        let a = v.valid_action_ids[0];
        act(r, a);
    }
    assert_eq!(
        r.game.player(0).battle_area[0].card_sources.len(),
        1,
        "both chosen source cards were extracted"
    );
}
fn protected(r: &DebugRunner) {
    assert!(ids(r).contains(&"BT23-047".into()));
    assert!(r
        .game
        .player(0)
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "EX13-024" && p.is_suspended));
    assert!(!r
        .game
        .player(0)
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "BT23-047"));
}
#[test]
fn official_q7274_prevention_then_partition_plays_both_sources_keeps_carrier() {
    let mut r = fixture();
    departure(&mut r);
    order(&mut r, "don't leave");
    protect(&mut r);
    let v = r
        .pending_selection_view()
        .expect("Q7274 says Partition remains available after prevention");
    assert_eq!(v.kind, SelectionKind::Replacement);
    assert!(v.prompt.contains("Partition"));
    act(&mut r, REPLACEMENT_ACCEPT);
    partition_picks(&mut r);
    protected(&r);
    let mut got = ids(&r);
    got.sort();
    let mut expected = vec!["BT23-047", "EX13-024", "GREEN5", "BLUE5"];
    expected.sort();
    assert_eq!(got, expected);
    assert_eq!(r.game.player(0).battle_area[0].card_sources.len(), 1);
    assert!(r.pending_selection().is_none());
}
#[test]
fn protected_partition_decline_keeps_sources_and_remembered_cancellation() {
    let mut r = fixture();
    departure(&mut r);
    order(&mut r, "don't leave");
    protect(&mut r);
    act(&mut r, PASS);
    protected(&r);
    assert_eq!(ids(&r), vec!["BT23-047", "EX13-024"]);
    assert_eq!(r.game.player(0).battle_area[0].card_sources.len(), 3);
    assert!(r.pending_selection().is_none());
}
#[test]
fn clone_after_cancellation_branches_partition_accept_and_decline() {
    let mut r = fixture();
    departure(&mut r);
    order(&mut r, "don't leave");
    protect(&mut r);
    assert!(r.pending_selection().is_some());
    let saved = r.game.clone();
    let digest = saved.verification_digest();
    act(&mut r, REPLACEMENT_ACCEPT);
    partition_picks(&mut r);
    protected(&r);
    assert_eq!(ids(&r).len(), 4);
    r.game = saved;
    assert_eq!(r.game.verification_digest(), digest);
    act(&mut r, PASS);
    protected(&r);
    assert_eq!(ids(&r).len(), 2);
}
#[test]
fn partition_first_then_prevention_also_keeps_carrier() {
    let mut r = fixture();
    departure(&mut r);
    order(&mut r, "Partition");
    assert_eq!(r.game.player(0).battle_area[0].card_sources.len(), 3);
    let v = r.pending_selection_view().expect("Partition consent must be offered");
    assert_eq!(v.kind, SelectionKind::Replacement);
    assert!(v.prompt.contains("Partition"));
    act(&mut r, REPLACEMENT_ACCEPT);
    partition_picks(&mut r);
    protect(&mut r);
    protected(&r);
    assert_eq!(ids(&r).len(), 4);
    assert!(r.pending_selection().is_none());
}
#[test]
fn decline_both_still_commits_original_departure_once() {
    let mut r = fixture();
    departure(&mut r);
    order(&mut r, "don't leave");
    act(&mut r, PASS);
    act(&mut r, PASS);
    assert_eq!(ids(&r), vec!["EX13-024"]);
    assert_eq!(
        r.game
            .player(0)
            .trash
            .iter()
            .filter(|c| c.card_id(&r.game.card_data) == "BT23-047")
            .count(),
        1
    );
}
#[test]
fn own_effect_never_offers_partition_after_prevention() {
    let mut r = fixture();
    r.game.set_effect_source_player_for_test(Some(0));
    r.game
        .delete_permanent_with_effects(digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: 0,
        });
    protect(&mut r);
    protected(&r);
    assert!(r.pending_selection().is_none());
    assert_eq!(ids(&r).len(), 2);
}
struct TwoBarriers;
impl digimon_engine::effect::CardEffect for TwoBarriers {
    fn effects(
        &self,
        c: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        let mut a = digimon_engine::cards::keyword_effects::keyword_to_auto_effect(
            digimon_engine::enums::Keyword::Barrier,
            c,
        );
        a.extend(
            digimon_engine::cards::keyword_effects::keyword_to_auto_effect(
                digimon_engine::enums::Keyword::Barrier,
                c,
            ),
        );
        a
    }
}
#[test]
fn rule_16_24_4_second_barrier_can_pay_after_first_prevents_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("DUAL", "Dual Barrier"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .security(0, &["FILL"; 5])
        .memory(3)
        .start();
    r.register_effect("DUAL", std::sync::Arc::new(TwoBarriers));
    r.skip_mulligan();
    let h = r.place_on_field(0, "DUAL", None);
    let n = r.game.player(0).security.len();
    assert!(n >= 2);
    r.game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(h),
        ReplacementCause::Battle,
        Some(Zone::Trash),
    );
    let v = r.pending_selection_view().unwrap();
    act(&mut r, v.valid_action_ids[0]);
    act(&mut r, REPLACEMENT_ACCEPT);
    assert_eq!(r.game.player(0).security.len(), n - 1);
    act(&mut r, REPLACEMENT_ACCEPT);
    assert_eq!(r.game.player(0).security.len(), n - 2);
    assert_eq!(ids(&r), vec!["DUAL"]);
    assert!(r.pending_selection().is_none());
}

#[test]
fn cancelled_window_never_transfers_partition_to_same_physical_card_reentry() {
    let mut r = fixture();
    departure(&mut r);
    order(&mut r, "don't leave");
    protect(&mut r);
    assert!(r.pending_selection().is_some());
    let old = r.game.players[0].battle_area.remove(0);
    let mut fresh =
        digimon_engine::permanent::Permanent::new(old.top_card().clone(), r.game.turn_count);
    fresh.card_sources = old.card_sources;
    r.game.players[0].battle_area.insert(0, fresh);
    act(&mut r, REPLACEMENT_ACCEPT);
    assert_eq!(ids(&r), vec!["BT23-047", "EX13-024"]);
    assert_eq!(r.game.player(0).battle_area[0].card_sources.len(), 3);
    assert!(r.pending_selection().is_none());
}
struct NativeWindow {
    redirect: bool,
}
impl digimon_engine::effect::CardEffect for NativeWindow {
    fn effects(
        &self,
        c: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        let redirect = self.redirect;
        vec![
            digimon_engine::effect::Effect::when_would_leave_battle_area(c)
                .name("First mandatory outcome")
                .replacement_process(move |ctx| {
                    if redirect {
                        ctx.redirect_to(Zone::Hand)
                    } else {
                        ctx.cancel()
                    }
                })
                .build(),
            digimon_engine::effect::Effect::when_would_leave_battle_area(c)
                .name("Later optional draw")
                .optional()
                .replacement_process(|ctx| {
                    ctx.effect.draw(0, 1);
                })
                .build(),
        ]
    }
}
fn native(redirect: bool) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("NATIVE", "Native"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .memory(3)
        .start();
    r.register_effect("NATIVE", std::sync::Arc::new(NativeWindow { redirect }));
    r.skip_mulligan();
    let h = r.place_on_field(0, "NATIVE", None);
    r.game.set_effect_source_player_for_test(Some(1));
    r.game.delete_permanent_with_effects(h);
    r
}
#[test]
fn mandatory_cancel_remains_when_later_nonreplacing_effect_completes() {
    let mut r = native(false);
    let hand = r.hand_size(0);
    order(&mut r, "First mandatory");
    act(&mut r, REPLACEMENT_ACCEPT);
    assert_eq!(r.hand_size(0), hand + 1);
    assert_eq!(ids(&r), vec!["NATIVE"]);
    assert!(r.pending_selection().is_none());
}
#[test]
fn redirect_still_commits_new_destination_without_running_remaining_old_window() {
    let mut r = native(true);
    let hand = r.hand_size(0);
    order(&mut r, "First mandatory");
    assert!(ids(&r).is_empty());
    assert_eq!(
        r.hand_size(0),
        hand + 1,
        "redirected card only; optional draw not activated"
    );
    assert!(r.pending_selection().is_none());
}

#[test]
fn native_cancel_clones_accept_or_decline_later_optional_without_departure() {
    let mut r = native(false);
    let hand = r.hand_size(0);
    order(&mut r, "First mandatory");
    assert_eq!(r.pending_selection_view().unwrap().kind, SelectionKind::Replacement);
    let saved = r.game.clone();
    for accept in [false, true] {
        r.game = saved.clone();
        act(&mut r, if accept { REPLACEMENT_ACCEPT } else { PASS });
        assert_eq!(r.hand_size(0), hand + usize::from(accept));
        assert_eq!(ids(&r), vec!["NATIVE"]);
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn cancelled_window_subject_departure_does_not_commit_again_or_target_neighbor() {
    let mut r = native(false);
    order(&mut r, "First mandatory");
    let old = r.game.players[0].battle_area.remove(0);
    r.game.players[0].trash.extend(old.card_sources);
    r.place_on_field(0, "FILL", None);
    let saved = r.game.clone();
    for accept in [false, true] {
        r.game = saved.clone();
        let hand = r.hand_size(0);
        act(&mut r, if accept { REPLACEMENT_ACCEPT } else { PASS });
        assert_eq!(ids(&r), vec!["FILL"]);
        assert_eq!(r.hand_size(0), hand, "departed source cannot activate the optional draw");
        assert_eq!(r.game.players[0].trash.iter().filter(|c| c.card_id(&r.game.card_data)=="NATIVE").count(), 1);
        assert!(r.pending_selection().is_none());
    }
}

// These are typed dispatcher-contract controls, not invented printed cards.
// A later immediate effect may independently perform overwrite processing
// (18-2-1/3/4); CustomHandled denotes processing already completed by its body.
struct CancelThenTerminal(u8);
impl digimon_engine::effect::CardEffect for CancelThenTerminal {
    fn effects(&self, c: digimon_engine::card_source::CardHandle) -> Vec<digimon_engine::effect::Effect> {
        let kind = self.0;
        vec![
            digimon_engine::effect::Effect::when_would_leave_battle_area(c)
                .name("First mandatory cancellation")
                .replacement_condition(|ctx, subject| matches!(subject, ReplacementSubject::Permanent(h) if Some(*h) == ctx.source_permanent))
                .replacement_process(|ctx| ctx.cancel()).build(),
            digimon_engine::effect::Effect::when_would_leave_battle_area(c)
                .name("Later terminal processing").optional()
                .replacement_condition(|ctx, subject| matches!(subject, ReplacementSubject::Permanent(h) if Some(*h) == ctx.source_permanent))
                .replacement_process(move |ctx| match kind {
                    0 => ctx.redirect_to(Zone::Hand),
                    1 => ctx.substitute(ReplacementSubject::Permanent(
                        digimon_engine::permanent::PermanentHandle {player: 0, index: 1})),
                    2 => {ctx.effect.draw(0, 1); ctx.handled();},
                    _ => unreachable!(),
                }).build(),
        ]
    }
}
fn terminal_contract(kind: u8) {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("NATIVE", "Native"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL";20]).deck(1, &["FILL";20]).memory(3).start();
    r.register_effect("NATIVE", std::sync::Arc::new(CancelThenTerminal(kind)));
    r.skip_mulligan();
    let h = r.place_on_field(0, "NATIVE", None);
    let other = r.place_on_field(0, "FILL", None);
    let other_card = r.game.player(0).battle_area[other.index as usize].top_card().handle();
    let main_card = r.game.player(0).battle_area[h.index as usize].top_card().handle();
    r.game.set_effect_source_player_for_test(Some(1));
    r.game.delete_permanent_with_effects(h);
    order(&mut r, "First mandatory");
    assert!(r.pending_selection_view().unwrap().is_optional);
    let saved = r.game.clone();
    for accept in [false, true] {
        r.game = saved.clone();
        let hand = r.hand_size(0);
        act(&mut r, if accept { REPLACEMENT_ACCEPT } else { PASS });
        assert!(r.pending_selection().is_none());
        if !accept {
            assert_eq!(ids(&r), vec!["NATIVE", "FILL"]);
            assert_eq!(r.hand_size(0), hand);
            assert!(r.game.player(0).trash.is_empty());
        } else {match kind {
            0 => {
                assert_eq!(ids(&r), vec!["FILL"]);
                assert_eq!(r.hand_size(0), hand+1);
                assert!(r.game.player(0).hand.iter().any(|c| c.handle()==main_card));
                assert!(r.game.player(0).trash.is_empty());
            },
            1 => {
                assert_eq!(ids(&r), vec!["NATIVE"]);
                assert_eq!(r.hand_size(0), hand);
                assert_eq!(r.game.player(0).trash.iter().filter(|c| c.handle()==other_card).count(), 1);
                assert!(!r.game.player(0).trash.iter().any(|c| c.handle()==main_card));
            },
            2 => {assert_eq!(ids(&r), vec!["NATIVE", "FILL"]); assert_eq!(r.hand_size(0),hand+1);},
            _ => unreachable!(),
        }}
    }
}
#[test]
fn prior_cancel_does_not_suppress_explicit_later_redirect_contract() { terminal_contract(0); }
#[test]
fn prior_cancel_does_not_suppress_separate_substituted_processing_contract() { terminal_contract(1); }
#[test]
fn prior_cancel_then_custom_handled_never_repeats_original_departure() { terminal_contract(2); }


// CustomHandled carries no replacement destination or new subject. Its body
// must not undo an already recorded cancellation of this attack declaration.
struct CancelThenHandledAttack;
impl digimon_engine::effect::CardEffect for CancelThenHandledAttack {
    fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
        use digimon_engine::{effect::EffectBuilder,enums::EffectTiming};
        vec![
            EffectBuilder::new(c,EffectTiming::WhenWouldAttack)
                .name("First attack cancellation")
                .replacement_condition(|ctx,subject| matches!(subject,ReplacementSubject::Permanent(h) if Some(*h)==ctx.source_permanent))
                .replacement_process(|ctx|ctx.cancel()).build(),
            EffectBuilder::new(c,EffectTiming::WhenWouldAttack)
                .name("Later handled draw").optional()
                .replacement_condition(|ctx,subject| matches!(subject,ReplacementSubject::Permanent(h) if Some(*h)==ctx.source_permanent))
                .replacement_process(|ctx|{ctx.effect.draw(0,1);ctx.handled();}).build(),
        ]
    }
}
#[test]
fn remembered_attack_cancellation_survives_later_handled_body_accept_and_decline() {
    use digimon_engine::action::space::{encode_attack,SECURITY_TARGET};
    let mut r=DebugRunner::builder().add_card(make_test_card("NATIVE","Attacker"))
        .add_card(make_test_card("FILL","Filler")).deck(0,&["FILL";20]).deck(1,&["FILL";20])
        .security(1,&["FILL";5]).memory(3).start();
    r.register_effect("NATIVE",std::sync::Arc::new(CancelThenHandledAttack));
    r.skip_mulligan();r.game.enter_main_phase();
    let h=r.place_on_field(0,"NATIVE",Some(0));
    let action=encode_attack(h.index as u16,SECURITY_TARGET);
    assert_eq!(digimon_engine::action::build_action_mask(&r.game,0)[action as usize],1.0);
    r.game.decode_action(action,0);
    order(&mut r,"First attack cancellation");
    assert!(r.pending_selection_view().unwrap().is_optional);
    let saved=r.game.clone();
    for accept in [false,true] {
        r.game=saved.clone();let hand=r.hand_size(0);
        act(&mut r,if accept{REPLACEMENT_ACCEPT}else{PASS});
        r.auto_resolve().unwrap();
        assert_eq!(r.hand_size(0),hand+usize::from(accept));
        assert_eq!(r.game.players[1].security.len(),5,"cancelled attack cannot resume after handled={accept}");
        assert!(r.game.pending_attack.is_none());
    }
}
