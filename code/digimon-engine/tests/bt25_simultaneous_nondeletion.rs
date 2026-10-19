//! Source-first behavioral regressions for Q6307 simultaneous leaves.
//! Real printed producers: BT20-045 DNA all-highest; BT24-091 all-lowest.
use digimon_engine::{
    action::{
        build_action_mask,
        space::{DNA_DIGIVOLVE_START, PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT},
    },
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardColor, GamePhase},
    selection::SelectionKind,
};
fn act(r: &mut DebugRunner, a: u16) {
    let p = r
        .game
        .pending_selection
        .as_ref()
        .expect("actual pending choice")
        .selecting_player;
    assert_eq!(build_action_mask(&r.game, p)[a as usize], 1.0);
    r.game.resolve_selection(p, a).unwrap();
}
fn settle_optional(r: &mut DebugRunner) {
    for _ in 0..12 {
        let Some(v) = r.pending_selection_view() else {
            return;
        };
        assert!(
            r.game.pending_selection.as_ref().unwrap().is_optional,
            "unexpected required tail {:?}: {}",
            v.kind,
            v.prompt
        );
        act(r, PASS);
    }
    panic!("optional tail did not finish");
}
fn field(r: &DebugRunner, p: usize) -> Vec<CardHandle> {
    r.game.players[p]
        .battle_area
        .iter()
        .map(|x| x.top_card().handle())
        .collect()
}
fn base(producer: &str) -> DebugRunner {
    let mut b = DebugRunner::builder();
    for id in ["BT25-039", "BT25-033", "BT25-059"] {
        b = b.dsl_card(id).unwrap();
    }
    if producer == "TEST-SEQUENTIAL" {
        b = b.from_dsl_yaml(SEQUENTIAL).unwrap();
    } else {
        b = b.dsl_card(producer).unwrap();
    }
    for (id, color) in [("GROUP-G", CardColor::Green), ("GROUP-B", CardColor::Blue)] {
        let mut c = make_test_card(id, id);
        c.colors = vec![color];
        c.level = Some(6);
        c.dp = Some(6000);
        b = b.add_card(c);
    }
    let mut no = make_test_card("GROUP-NO", "Nonmatching test body");
    no.level = Some(4);
    no.dp = Some(12000);
    no.traits.clear();
    b = b.add_card(no);
    let mut r = b
        .hand(0, &[producer])
        .deck(0, &["GROUP-G"; 20])
        .deck(1, &["GROUP-B"; 20])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r
}
fn threatened(
    r: &mut DebugRunner,
    id: &str,
    nonmatching: bool,
) -> (Vec<CardHandle>, CardHandle, Option<CardHandle>) {
    let mut group = Vec::new();
    for _ in 0..2 {
        let h = r.place_on_field(1, id, None);
        group.push(
            r.game.players[1].battle_area[h.index as usize]
                .top_card()
                .handle(),
        );
    }
    let h = r.place_on_field(1, "BT25-039", None);
    let payer = r.game.players[1].battle_area[h.index as usize]
        .top_card()
        .handle();
    let other = if nonmatching {
        let h = r.place_on_field(1, "GROUP-NO", None);
        Some(
            r.game.players[1].battle_area[h.index as usize]
                .top_card()
                .handle(),
        )
    } else {
        None
    };
    (group, payer, other)
}
fn dna(r: &mut DebugRunner) {
    r.place_on_field(0, "GROUP-G", None);
    r.place_on_field(0, "GROUP-B", None);
    assert_eq!(
        build_action_mask(&r.game, 0)[DNA_DIGIVOLVE_START as usize],
        1.0
    );
    r.game.decode_action(DNA_DIGIVOLVE_START, 0);
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids,
        vec![0, 1]
    );
    act(r, 0);
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids,
        vec![1]
    );
    act(r, 1);
    assert_eq!(r.pending_selection_view().unwrap().selecting_player, 1);
    assert_eq!(
        r.pending_selection_view().unwrap().kind,
        SelectionKind::Replacement
    );
}
fn tidal(r: &mut DebugRunner) {
    r.place_on_field(0, "BT25-033", None);
    r.game.players[0].battle_area[0].is_suspended = true;
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    if r.pending_selection_view()
        .is_some_and(|v| v.selecting_player == 0 && v.kind == SelectionKind::EffectChoice)
    {
        let v = r.pending_selection_view().unwrap();
        let rows = v.effect_choices.as_ref().unwrap();
        let a = rows
            .iter()
            .find(|c| c.label.contains("Standard") || c.label.contains("[Main]"))
            .expect("explicit ordinary Option-use mode")
            .action_id;
        act(r, a);
    }
    assert_eq!(r.pending_selection_view().unwrap().selecting_player, 1);
    assert_eq!(
        r.pending_selection_view().unwrap().kind,
        SelectionKind::Replacement
    );
}
#[test]
fn examon_dna_one_paid_siren_preserves_both_exact_highest_bodies_and_clones() {
    let mut r = base("BT20-045");
    let (group, payer, _) = threatened(&mut r, "BT25-059", false);
    dna(&mut r);
    let saved = r.game.clone();
    let mut digest = None;
    for _ in 0..2 {
        r.game = saved.clone();
        act(&mut r, REPLACEMENT_ACCEPT);
        settle_optional(&mut r);
        let now = field(&r, 1);
        assert!(
            group.iter().all(|h| now.contains(h)),
            "one successful payment protects the complete simultaneous matching set"
        );
        assert!(!now.contains(&payer));
        assert!(group
            .iter()
            .all(|h| !r.game.players[1].deck.iter().any(|c| c.handle() == *h)));
        let d = r.game.verification_digest();
        if let Some(old) = digest {
            assert_eq!(d, old);
        }
        digest = Some(d);
    }
}
#[test]
fn examon_dna_plural_prevention_still_bottomdecks_nonmatching_highest_body() {
    let mut r = base("BT20-045");
    let (group, payer, other) = threatened(&mut r, "BT25-059", true);
    dna(&mut r);
    act(&mut r, REPLACEMENT_ACCEPT);
    settle_optional(&mut r);
    let now = field(&r, 1);
    assert!(group.iter().all(|h| now.contains(h)));
    assert!(!now.contains(&payer));
    assert!(!now.contains(&other.unwrap()));
    assert!(r.game.players[1]
        .deck
        .iter()
        .any(|c| c.handle() == other.unwrap()));
}
#[test]
fn tidal_paid_plural_preserves_both_and_does_not_count_as_returned() {
    let mut r = base("BT24-091");
    let (group, payer, _) = threatened(&mut r, "BT25-033", false);
    tidal(&mut r);
    let saved = r.game.clone();
    let mut digest = None;
    for _ in 0..2 {
        r.game = saved.clone();
        act(&mut r, REPLACEMENT_ACCEPT);
        assert!(group.iter().all(|h| field(&r, 1).contains(h)));
        assert!(!field(&r, 1).contains(&payer));
        assert!(group
            .iter()
            .all(|h| !r.game.players[1].hand.iter().any(|c| c.handle() == *h)));
        assert!(r.game.players[0].battle_area[0].is_suspended);
        let v = r
            .pending_selection_view()
            .expect("then optional link still offered");
        assert_eq!(v.selecting_player, 0);
        assert!(
            r.game.pending_selection.as_ref().unwrap().is_optional,
            "no mandatory returned-card unsuspend after every return was prevented"
        );
        act(&mut r, PASS);
        settle_optional(&mut r);
        assert!(
            group.iter().all(|h| field(&r, 1).contains(h)),
            "settled bodies remain protected"
        );
        let d = r.game.verification_digest();
        if let Some(old) = digest {
            assert_eq!(d, old);
        }
        digest = Some(d);
    }
}
#[test]
fn tidal_declining_the_single_group_protection_returns_both_without_reoffering() {
    let mut r = base("BT24-091");
    let (group, payer, _) = threatened(&mut r, "BT25-033", false);
    tidal(&mut r);
    let saved = r.game.clone();
    for _ in 0..2 {
        r.game = saved.clone();
        act(&mut r, PASS);
        assert!(
            group
                .iter()
                .all(|h| r.game.players[1].hand.iter().any(|c| c.handle() == *h)),
            "one declined simultaneous activation does not reoffer per member"
        );
        assert!(field(&r, 1).contains(&payer));
        let v = r.pending_selection_view().unwrap();
        assert_eq!(v.selecting_player, 0);
        assert_eq!(v.kind, SelectionKind::OwnField);
        assert!(
            !r.game.pending_selection.as_ref().unwrap().is_optional,
            "actual returned cards require the printed unsuspend choice"
        );
    }
}

#[test]
fn examon_declined_group_exposes_both_bottom_orderings_to_causing_player() {
    let mut r = base("BT20-045");
    let (group, payer, _) = threatened(&mut r, "BT25-059", false);
    dna(&mut r);
    act(&mut r, PASS);
    let v = r
        .pending_selection_view()
        .expect("causing player chooses public bottom-deck order");
    assert_eq!(v.selecting_player, 0);
    assert!(!r.game.pending_selection.as_ref().unwrap().is_optional);
    assert_eq!(v.valid_action_ids.len(), 2);
    let saved = r.game.clone();
    for first in [0usize, 1] {
        r.game = saved.clone();
        let action = digimon_engine::action::space::encode_attack(1, first as u16);
        assert!(r
            .pending_selection_view()
            .unwrap()
            .valid_action_ids
            .contains(&action));
        act(&mut r, action);
        if let Some(v) = r.pending_selection_view() {
            if v.selecting_player == 0 && !r.game.pending_selection.as_ref().unwrap().is_optional {
                assert_eq!(v.valid_action_ids.len(), 2);
                act(&mut r, v.valid_action_ids[0]);
            }
        }
        assert_eq!(field(&r, 1), vec![payer]);
        let bottom: Vec<_> = r.game.players[1]
            .deck
            .iter()
            .take(2)
            .map(|c| c.handle())
            .collect();
        assert_eq!(
            bottom,
            vec![group[first], group[1 - first]],
            "choice states explicitly label bottommost-first order"
        );
    }
}

const SEQUENTIAL: &str = r#"
card: TEST-SEQUENTIAL
name: Explicit sequential loop test
kind: digimon
color: [blue]
level: 3
cost: 0
dp: 2000
effects:
  - when: on_play
    process:
      - for_each:
          over: { of: opponent, zone: [battle_area], kind: digimon, level_eq: 4 }
          bind_as: sequential_target
          body:
            - return_to_hand: { target: sequential_target }
      - draw: { of: you, count: 1 }
"#;
#[test]
fn explicitly_sequential_for_each_keeps_separate_leave_events() {
    let mut r = base("TEST-SEQUENTIAL");
    let (group, payer, _) = threatened(&mut r, "BT25-033", false);
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    assert_eq!(
        r.pending_selection_view().unwrap().kind,
        SelectionKind::Replacement
    );
    act(&mut r, REPLACEMENT_ACCEPT);
    assert_eq!(
        group.iter().filter(|h| field(&r, 1).contains(h)).count(),
        1,
        "an explicitly imperative loop is not silently rewritten as simultaneous"
    );
    assert_eq!(
        group
            .iter()
            .filter(|h| r.game.players[1].hand.iter().any(|c| c.handle() == **h))
            .count(),
        1
    );
    assert!(!field(&r, 1).contains(&payer));
}

#[test]
fn sequential_decline_reaches_next_physical_member_before_exactly_once_tail_with_clones() {
    let mut r = base("TEST-SEQUENTIAL");
    let (group, payer, _) = threatened(&mut r, "BT25-033", false);
    r.game.decode_action(PLAY_HAND_START, 0);
    let saved = r.game.clone();
    let own_hand = r.game.players[0].hand.len();
    for _ in 0..2 {
        r.game = saved.clone();
        act(&mut r, PASS);
        assert!(r.game.players[1]
            .hand
            .iter()
            .any(|c| c.handle() == group[0]));
        assert!(r.game.pending_selection.is_some(), "second sequential event absent: field={:?} hand={:?} own_hand={} expected={} fired={:?}",field(&r,1),r.game.players[1].hand.iter().map(|c|c.handle()).collect::<Vec<_>>(),r.game.players[0].hand.len(),own_hand,r.game.replacement_fired);
        let v = r.pending_selection_view().unwrap();
        assert_eq!(v.kind, SelectionKind::Replacement);
        assert_eq!(v.selecting_player, 1);
        assert_eq!(
            r.game.players[0].hand.len(),
            own_hand,
            "outer draw waits for second event"
        );
        act(&mut r, PASS);
        assert!(group
            .iter()
            .all(|h| r.game.players[1].hand.iter().any(|c| c.handle() == *h)));
        assert!(field(&r, 1).contains(&payer));
        assert_eq!(r.game.players[0].hand.len(), own_hand + 1);
    }
}

const PAYMENT_GUARD: &str = r#"
card: GROUP-PAY-GUARD
name: Payment guard
kind: tamer
color: [blue]
cost: 3
effects:
 - kind: replacement
   trigger: when_would_be_deleted
   optional: true
   active_when:
     all_of:
       - replacement_subject_is_mine: true
       - replacement_cause: own_effect
       - name_is: Sirenmon
   process:
     - select_effect_choice:
         labels: [Complete prevention, Confirm prevention]
         prompt: Complete payment prevention
     - cancel_replacement: {}
"#;
#[test]
fn plural_group_waits_for_parked_payment_and_keeps_causing_tail_before_on_deletion() {
    let mut b = DebugRunner::builder().from_dsl_yaml(PAYMENT_GUARD).unwrap();
    for id in ["BT25-039", "BT25-033", "BT24-091"] {
        b = b.dsl_card(id).unwrap();
    }
    let mut r = b
        .hand(0, &["BT24-091"])
        .deck(0, &["BT25-033"; 20])
        .deck(1, &["BT25-033"; 20])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    let (group, payer, _) = threatened(&mut r, "BT25-033", false);
    r.place_on_field(1, "GROUP-PAY-GUARD", None);
    tidal(&mut r);
    act(&mut r, REPLACEMENT_ACCEPT);
    assert_eq!(
        r.pending_selection_view().unwrap().kind,
        SelectionKind::Replacement
    );
    assert!(field(&r, 1).contains(&payer));
    let saved = r.game.clone();
    for prevent in [false, true] {
        let mut prior = None;
        for _ in 0..2 {
            r.game = saved.clone();
            act(&mut r, if prevent { REPLACEMENT_ACCEPT } else { PASS });
            if prevent {
                let v = r
                    .pending_selection_view()
                    .expect("accepted immediate body parks");
                assert_eq!(v.kind, SelectionKind::EffectChoice);
                assert_eq!(v.selecting_player, 1);
                assert_eq!(v.valid_action_ids.len(), 2);
                act(&mut r, v.valid_action_ids[0]);
            }
            assert_eq!(
                group.iter().filter(|h| field(&r, 1).contains(h)).count(),
                if prevent { 0 } else { 2 }
            );
            assert_eq!(field(&r, 1).contains(&payer), prevent);
            let v = r
                .pending_selection_view()
                .expect("original causing effect still has its own tail");
            assert_eq!(v.selecting_player, 0);
            if !prevent {
                assert!(
                    r.game.players[1].trash.iter().any(|c| c.handle() == payer),
                    "OnDeletion cannot place Siren in security before optional Link tail completes"
                );
                assert!(r.game.pending_selection.as_ref().unwrap().is_optional);
            }
            let digest = r.game.verification_digest();
            if let Some(prior) = prior {
                assert_eq!(digest, prior);
            }
            prior = Some(digest);
        }
    }
}

struct WholeGroupLeaveWitness(Vec<CardHandle>);
impl digimon_engine::effect::CardEffect for WholeGroupLeaveWitness {
    fn effects(&self, c: CardHandle) -> Vec<digimon_engine::effect::Effect> {
        let expected = self.0.clone();
        vec![digimon_engine::effect::EffectBuilder::new(
            c,
            digimon_engine::enums::EffectTiming::OnLeaveField,
        )
        .condition(move |ctx| {
            assert!(
                expected
                    .iter()
                    .all(|h| ctx.game.players[1].hand.iter().any(|c| c.handle() == *h)),
                "every group member has moved before any leave observer condition is evaluated"
            );
            assert!(ctx.game.players[1]
                .battle_area
                .iter()
                .all(|p| !expected.contains(&p.top_card().handle())));
            true
        })
        .process(|ctx| {
            ctx.draw(1, 1);
        })
        .build()]
    }
}
#[test]
fn grouped_return_commits_all_members_before_each_leave_observer_condition() {
    let mut r = base("BT24-091");
    let (group, _, _) = threatened(&mut r, "BT25-033", false);
    r.register_effect(
        "GROUP-G",
        std::sync::Arc::new(WholeGroupLeaveWitness(group.clone())),
    );
    r.place_on_field(1, "GROUP-G", None);
    tidal(&mut r);
    let hand = r.game.players[1].hand.len();
    act(&mut r, PASS);
    let v = r
        .pending_selection_view()
        .expect("printed unsuspend follows real returns");
    assert_eq!(v.kind, SelectionKind::OwnField);
    act(&mut r, v.valid_action_ids[0]);
    r.auto_resolve().unwrap();
    assert!(group
        .iter()
        .all(|h| r.game.players[1].hand.iter().any(|c| c.handle() == *h)));
    assert_eq!(
        r.game.players[1].hand.len(),
        hand + 4,
        "two actual returned cards and one observer activation per departure"
    );
}


fn slayer_play_choice(r: &mut DebugRunner, yes: bool) {
    assert_eq!(build_action_mask(&r.game,0)[PLAY_HAND_START as usize],1.0);
    r.game.decode_action(PLAY_HAND_START,0);
    let v=r.pending_selection_view().expect("printed optional return choice");
    assert_eq!(v.selecting_player,0);
    assert_eq!(v.kind,SelectionKind::EffectChoice);
    let wanted=if yes {"Return all"} else {"Don't return"};
    let a=v.effect_choices.as_ref().unwrap().iter().find(|x|x.label.contains(wanted)).unwrap().action_id;
    act(r,a);
    if yes { assert!(r.game.pending_selection.is_some(), "after printed yes: actor={} phase={:?} field={:?} hand={:?} trash={:?}", r.game.turn_player(),r.game.current_phase,field(r,1),r.game.players[1].hand.iter().map(|c|c.card_id(&r.game.card_data)).collect::<Vec<_>>(),r.game.players[1].trash.iter().map(|c|c.card_id(&r.game.card_data)).collect::<Vec<_>>()); }
}
fn slayer_targets(r: &mut DebugRunner) -> (Vec<CardHandle>,CardHandle) {
    let mut group=Vec::new();
    for _ in 0..2 {
        let h=r.place_stack(1,&["GROUP-G","BT25-033"]);
        group.push(r.game.players[1].battle_area[h.index as usize].top_card().handle());
    }
    let h=r.place_stack(1,&["GROUP-G","GROUP-B","BT25-039"]);
    (group,r.game.players[1].battle_area[h.index as usize].top_card().handle())
}
#[test]
fn slayer_real_on_play_one_paid_plural_protects_the_complete_fewest_source_group() {
    let mut r=base("EX13-024");
    let (group,payer)=slayer_targets(&mut r);
    slayer_play_choice(&mut r,true);
    assert_eq!(r.pending_selection_view().unwrap().selecting_player,1);
    let mut cloned=r.game.clone();
    act(&mut r,REPLACEMENT_ACCEPT);
    settle_optional(&mut r);
    assert!(group.iter().all(|h|field(&r,1).contains(h)));
    assert!(!field(&r,1).contains(&payer));
    assert!(group.iter().all(|h|!r.game.players[1].deck.iter().any(|c|c.handle()==*h)));
    let expected=r.game.verification_digest();
    std::mem::swap(&mut r.game,&mut cloned);
    act(&mut r,REPLACEMENT_ACCEPT);
    settle_optional(&mut r);
    assert_eq!(r.game.verification_digest(),expected,"original and clone resume identically");
}
#[test]
fn slayer_real_on_play_declined_group_offers_both_public_deck_orders_once() {
    for first in [0usize,1] {
        let mut r=base("EX13-024");
        let (group,payer)=slayer_targets(&mut r);
        slayer_play_choice(&mut r,true);
        act(&mut r,PASS);
        let v=r.pending_selection_view().expect("simultaneous public order");
        assert_eq!(v.selecting_player,0);
        assert_eq!(v.valid_action_ids.len(),2);
        act(&mut r,digimon_engine::action::space::encode_attack(1,first as u16));
        assert_eq!(field(&r,1),vec![payer]);
        assert_eq!(r.game.players[1].deck.iter().take(2).map(|c|c.handle()).collect::<Vec<_>>(),vec![group[first],group[1-first]]);
        assert_eq!(r.game.players[1].trash.len(),2,"both returned bodies' sources go to trash");
    }
}
#[test]
fn slayer_real_on_play_declining_return_does_not_offer_or_consume_siren_payment() {
    let mut r=base("EX13-024");
    let (group,payer)=slayer_targets(&mut r);
    slayer_play_choice(&mut r,false);
    assert!(r.game.pending_selection.is_none());
    assert!(group.iter().all(|h|field(&r,1).contains(h)));
    assert!(field(&r,1).contains(&payer));
    assert!(r.game.players[1].trash.is_empty());
}


#[test]
fn queued_real_siren_deletion_does_not_keep_expired_plural_trait_eligibility() {
    use digimon_engine::{modifiers::{ModifierEntry,ModifierPayload},enums::{Expiry,ModifierType}};
    for expires in [false,true] {
        let mut r=base("BT24-091");
        let victim=r.place_on_field(1,"GROUP-NO",None);
        let victim_card=r.game.players[1].battle_area[0].top_card().handle();
        let ally=r.place_on_field(1,"BT25-033",None);
        let ally_card=r.game.players[1].battle_area[ally.index as usize].top_card().handle();
        let h=r.place_on_field(1,"BT25-039",None);
        let payer=r.game.players[1].battle_area[h.index as usize].top_card().handle();
        r.game.modifiers.add(victim,ModifierEntry::simple(ModifierType::ChangeTraits,0,Expiry::UntilCondition,1)
            .with_payload(ModifierPayload::Traits{add:vec!["Shaman".into()],replace:false})
            .with_until_condition(std::sync::Arc::new(move|g,_|!expires||g.players[1].battle_area.iter().any(|p|p.top_card().handle()==payer))));
        tidal(&mut r);
        let mut original=r.game.clone();
        let mut expected=None;
        for pass in 0..2 {
            if pass==1 {std::mem::swap(&mut r.game,&mut original);}
            act(&mut r,REPLACEMENT_ACCEPT);
            assert!(field(&r,1).contains(&ally_card));
            assert!(!field(&r,1).contains(&payer));
            assert_eq!(field(&r,1).contains(&victim_card),!expires);
            assert_eq!(r.game.players[1].hand.iter().any(|c|c.handle()==victim_card),expires);
            if expires {
                let v=r.pending_selection_view().expect("actual return reaches causing Option's required unsuspend before queued OnDeletion");
                assert_eq!(v.selecting_player,0);
                assert_eq!(v.kind,SelectionKind::OwnField);
                assert!(!r.game.pending_selection.as_ref().unwrap().is_optional);
                assert!(r.game.players[0].battle_area[0].is_suspended);
                act(&mut r,digimon_engine::action::space::encode_attack(0,0));
            }
            settle_optional(&mut r);
            assert_eq!(r.game.players[0].battle_area[0].is_suspended,!expires);
            let digest=r.game.verification_digest();
            if let Some(prior)=expected {assert_eq!(digest,prior);} else {expected=Some(digest);}
        }
    }
}
