//! EX6-063: actual printed image + official bundle + DCGO EX6_063.cs.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_attack, PASS},
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{PendingSecurity, SelectionKind, TriggerSource};
const ID: &str = "EX6-063";
fn body(id: &str, color: CardColor, kind: CardKind, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![color];
    c.card_kind = kind;
    c.level = if kind == CardKind::Digimon {
        Some(4)
    } else {
        None
    };
    c.dp = if kind == CardKind::Digimon {
        Some(5000)
    } else {
        None
    };
    c.traits = traits.iter().map(|s| s.to_string()).collect();
    c
}
fn builder() -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder()
        .dsl_card(ID)
        .expect("EX6-063 production YAML");
    for (id, color, kind, traits) in [
        ("ANGEL", CardColor::Yellow, CardKind::Digimon, vec!["Angel"]),
        (
            "ARCH",
            CardColor::Yellow,
            CardKind::Digimon,
            vec!["Archangel"],
        ),
        (
            "THREE",
            CardColor::Purple,
            CardKind::Digimon,
            vec!["Three Great Angels"],
        ),
        (
            "FALLEN",
            CardColor::Yellow,
            CardKind::Digimon,
            vec!["Fallen Angel"],
        ),
        ("PLAIN", CardColor::Yellow, CardKind::Digimon, vec!["Data"]),
        ("TAMER", CardColor::Yellow, CardKind::Tamer, vec!["Angel"]),
    ] {
        b = b.add_card(body(id, color, kind, &traits));
    }
    b.deck(0, &["PLAIN"; 8]).deck(1, &["PLAIN"; 8]).memory(3)
}
fn setup() -> (DebugRunner, PermanentHandle) {
    let mut r = builder().start();
    r.skip_mulligan();
    let t = r.place_on_field(0, ID, Some(0));
    (r, t)
}
fn grant(r: &mut DebugRunner, t: PermanentHandle, timing: EffectTiming) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(t));
    r.game.drain_effect_queue();
}
fn event(r: &mut DebugRunner, h: PermanentHandle, evolved: bool) {
    if evolved {
        let card = r.game.players[h.player as usize].battle_area[h.index as usize]
            .top_card()
            .handle();
        r.game.enqueue_triggered(
            EffectTiming::OnDigivolve,
            TriggerSource::Digivolved {
                player: h.player,
                permanent: h,
                card,
                effect_initiated: false,
                dna_origin: false,
            },
        );
        r.game.drain_effect_queue();
    } else {
        r.fire_play_event_triggers(h.player, h.index as usize, false, false);
    }
}
fn optional_mask(r: &DebugRunner, expected: SelectionKind) {
    let v = r
        .pending_selection_view()
        .expect("optional suspend payment");
    assert!(v.is_optional);
    assert_eq!(v.kind, expected);
    let m = build_action_mask(&r.game, v.selecting_player);
    assert_eq!(m[PASS as usize], 1.0);
    assert!(!v.valid_action_ids.is_empty());
    for a in v.valid_action_ids {
        assert_eq!(m[a as usize], 1.0);
    }
}
#[test]
fn ex6_063_both_grant_timings_offer_only_own_yellow_digimon_and_expire() {
    for timing in [EffectTiming::OnPlay, EffectTiming::StartOfYourMainPhase] {
        let (mut r, t) = setup();
        let a = r.place_on_field(0, "ANGEL", Some(0));
        let b = r.place_on_field(0, "PLAIN", Some(0));
        let purple = r.place_on_field(0, "THREE", Some(0));
        r.place_on_field(0, "TAMER", Some(0));
        r.place_on_field(1, "ANGEL", Some(0));
        grant(&mut r, t, timing);
        let v = r.pending_selection_view().expect("mandatory yellow target");
        assert!(!v.is_optional);
        let m = build_action_mask(&r.game, 0);
        assert_eq!(m[PASS as usize], 0.0);
        let aa = encode_attack(0, a.index as u16);
        let ab = encode_attack(0, b.index as u16);
        assert!(v.valid_action_ids.contains(&aa));
        assert!(v.valid_action_ids.contains(&ab));
        assert_eq!(v.valid_action_ids.len(), 2);
        for id in &v.valid_action_ids {
            assert_eq!(m[*id as usize], 1.0);
        }
        r.execute_action(0, ab).unwrap();
        assert!(r.game.has_keyword(b, Keyword::Barrier));
        assert!(!r.game.has_keyword(a, Keyword::Barrier));
        assert!(!r.game.has_keyword(purple, Keyword::Barrier));
        r.game.modifiers.expire_end_of_turn(0);
        assert!(r.game.has_keyword(b, Keyword::Barrier));
        r.game.modifiers.expire_end_of_turn(1);
        assert!(!r.game.has_keyword(b, Keyword::Barrier));
    }
}
#[test]
fn ex6_063_no_yellow_digimon_has_no_target_prompt() {
    let (mut r, t) = setup();
    r.place_on_field(0, "THREE", Some(0));
    r.place_on_field(0, "TAMER", Some(0));
    grant(&mut r, t, EffectTiming::OnPlay);
    assert!(r.pending_selection_view().is_none());
}
#[test]
fn ex6_063_own_play_and_evolution_of_each_exact_angel_trait_may_pay() {
    for evolved in [false, true] {
        for id in ["ANGEL", "ARCH", "THREE"] {
            let (mut r, t) = setup();
            let h = r.place_on_field(0, id, Some(0));
            let memory = r.game.memory;
            event(&mut r, h, evolved);
            optional_mask(&r, SelectionKind::TriggerOrder);
            assert!(!r.game.players[0].battle_area[t.index as usize].is_suspended);
            assert_eq!(r.game.memory, memory);
            r.accept_optional_trigger().unwrap();
            assert!(r.game.players[0].battle_area[t.index as usize].is_suspended);
            assert_eq!(r.game.memory, memory + 1);
        }
    }
}
#[test]
fn ex6_063_decline_does_not_suspend_or_gain_memory() {
    let (mut r, t) = setup();
    let h = r.place_on_field(0, "ANGEL", Some(0));
    let memory = r.game.memory;
    event(&mut r, h, false);
    optional_mask(&r, SelectionKind::TriggerOrder);
    r.decline_optional_trigger().unwrap();
    assert_eq!(r.game.memory, memory);
    assert!(!r.game.players[0].battle_area[t.index as usize].is_suspended);
}
#[test]
fn ex6_063_rejects_other_owner_non_digimon_nonmatching_traits_and_opponent_turn() {
    for (player, id, turn) in [
        (1, "ANGEL", 0),
        (0, "TAMER", 0),
        (0, "PLAIN", 0),
        (0, "FALLEN", 0),
        (0, "ANGEL", 1),
    ] {
        for evolved in [false, true] {
            let (mut r, _) = setup();
            r.game.turn_player_idx = turn as usize;
            let h = r.place_on_field(player, id, Some(0));
            let memory = r.game.memory;
            event(&mut r, h, evolved);
            assert!(
                r.pending_selection_view().is_none(),
                "{player} {id} turn{turn} evolved{evolved}"
            );
            assert_eq!(r.game.memory, memory);
        }
    }
}
#[test]
fn ex6_063_suspended_tamer_cannot_pay_and_unsuspension_allows_second_use_same_turn() {
    let (mut r, t) = setup();
    let h = r.place_on_field(0, "ANGEL", Some(0));
    event(&mut r, h, false);
    r.accept_optional_trigger().unwrap();
    let memory = r.game.memory;
    event(&mut r, h, true);
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.game.memory, memory);
    r.game.players[0].battle_area[t.index as usize].is_suspended = false;
    event(&mut r, h, true);
    optional_mask(&r, SelectionKind::TriggerOrder);
    r.accept_optional_trigger().unwrap();
    assert_eq!(r.game.memory, memory + 1);
}
#[test]
fn ex6_063_trait_checks_the_evolved_digimon_not_its_old_source() {
    let (mut r, _) = setup();
    let h = r.place_stack(0, &["ANGEL", "FALLEN"]);
    event(&mut r, h, true);
    assert!(r.pending_selection_view().is_none());
    let h = r.place_stack(0, &["FALLEN", "ARCH"]);
    event(&mut r, h, true);
    optional_mask(&r, SelectionKind::TriggerOrder);
}
#[test]
fn ex6_063_security_plays_free_and_runs_on_play_grant() {
    let mut r = builder().security(0, &[ID]).start();
    r.skip_mulligan();
    let target = r.place_on_field(0, "PLAIN", Some(0));
    let memory = r.game.memory;
    let card = r.game.players[0].security.pop().unwrap();
    let handle = card.handle();
    r.game.pending_security = Some(PendingSecurity {
        defender: 0,
        card,
        played: false,
    });
    r.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::SecurityRevealed {
            defender: 0,
            card: handle,
        },
    );
    r.game.drain_effect_queue();
    let v = r.pending_selection_view().expect("on-play Barrier target");
    let a = encode_attack(0, target.index as u16);
    assert!(v.valid_action_ids.contains(&a));
    r.execute_action(0, a).unwrap();
    assert_eq!(r.game.memory, memory);
    assert!(r.game.has_keyword(target, Keyword::Barrier));
    assert!(r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == ID));
}

#[test]
fn ex6_063_printed_tamer_metadata() {
    let (r, _) = setup();
    let c = r.compiled_card(ID).unwrap();
    assert_eq!(
        c.kind,
        digimon_engine::dsl::compiled::CompiledCardKind::Tamer
    );
    assert_eq!(c.cost, Some(4));
    assert_eq!(
        c.color,
        vec![digimon_engine::dsl::compiled::CompiledColor::Yellow]
    );
    assert_eq!(c.name, "T.K. Takaishi & Kari Kamiya");
}

fn barrier_battle_setup() -> (DebugRunner, PermanentHandle) {
    let mut r = builder().security(0, &["ANGEL"]).start();
    r.skip_mulligan();
    let t = r.place_on_field(0, ID, Some(0));
    let h = r.place_on_field(0, "PLAIN", Some(0));
    grant(&mut r, t, EffectTiming::OnPlay);
    r.execute_action(0, encode_attack(0, h.index as u16))
        .unwrap();
    assert!(r.game.has_keyword(h, Keyword::Barrier));
    (r, h)
}
#[test]
fn ex6_063_granted_barrier_pays_security_to_prevent_battle_deletion_or_may_decline() {
    for accept in [false, true] {
        let (mut r, h) = barrier_battle_setup();
        r.game
            .delete_permanent_with_cause(h, digimon_engine::replacement::ReplacementCause::Battle);
        optional_mask(&r, SelectionKind::Replacement);
        if accept {
            r.accept_optional_trigger().unwrap();
        } else {
            r.decline_optional_trigger().unwrap();
        }
        assert!(r.pending_selection_view().is_none());
        let survived = r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "PLAIN");
        assert_eq!(survived, accept);
        assert_eq!(r.game.players[0].security.len(), usize::from(!accept));
    }
}
struct OptionalSecurityGuard;
impl digimon_engine::effect::CardEffect for OptionalSecurityGuard {
    fn effects(
        &self,
        card: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        vec![digimon_engine::effect::Effect::when_would_be_trashed(card)
            .optional()
            .replacement_condition(|_, subject| {
                matches!(
                    subject,
                    digimon_engine::replacement::ReplacementSubject::Card(
                        _,
                        digimon_engine::enums::Zone::Security
                    )
                )
            })
            .replacement_process(|ctx| ctx.cancel())
            .build()]
    }
}
#[test]
fn ex6_063_granted_barrier_interrupted_payment_uses_physical_result_after_clone() {
    let (mut r, h) = barrier_battle_setup();
    r.register_effect("TAMER", std::sync::Arc::new(OptionalSecurityGuard));
    r.place_on_field(0, "TAMER", Some(0));
    r.game
        .delete_permanent_with_cause(h, digimon_engine::replacement::ReplacementCause::Battle);
    optional_mask(&r, SelectionKind::Replacement);
    r.accept_optional_trigger().unwrap();
    optional_mask(&r, SelectionKind::Replacement);
    let (mut canceled, _) = setup();
    canceled.game = r.game.clone();
    canceled.accept_optional_trigger().unwrap();
    assert_eq!(canceled.game.players[0].security.len(), 1);
    assert!(
        !canceled.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&canceled.game.card_data) == "PLAIN"),
        "unpaid Barrier cannot prevent deletion"
    );
    r.decline_optional_trigger().unwrap();
    assert!(r.pending_selection_view().is_none());
    assert!(r.game.players[0].security.is_empty());
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "PLAIN"),
        "paid Barrier prevents deletion after nested decline"
    );
}
