//! BT27-032: official EN image/category522040, Q7666-68. No admission claim.
#[path = "../bt26/support.rs"]
mod support;
use digimon_engine::action::{
    mask::build_action_mask,
    space::{encode_digivolve, PASS},
};
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, EffectSourceKind, EffectTiming, Expiry, ModifierType};
use digimon_engine::modifiers::{EffectControllerFilter, EffectImmunityFilter, ModifierEntry};
use digimon_engine::selection::TriggerSource;
use digimon_engine::trigger_context::EventCause;
use support::*;
const ID: &str = "BT27-032";
fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT27-032 YAML required")
        .add_card(filler("F"))
        .add_card(digimon("L3", "Three", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("L5", "Five", CardColor::Blue, 5, 7, &[]))
        .add_card(digimon("L6", "Six", CardColor::Red, 6, 10, &[]))
        .add_card(digimon(
            "SQUAD",
            "Squad",
            CardColor::Red,
            5,
            7,
            &["DATA SQUAD"],
        ))
        .add_card(digimon("GAO", "MachGaogamon", CardColor::Red, 5, 7, &[]))
        .add_card(tamer("T", "Tamer", CardColor::Red, &[]))
        .add_card(tamer("ST", "Squad tamer", CardColor::Red, &["DATA SQUAD"]))
        .deck(0, &["F"; 20])
        .deck(1, &["F"; 20])
        .memory(10)
}
fn setup() -> DebugRunner {
    let mut r = base().start();
    r.set_first_player(0);
    r
}
fn gain(r: &mut DebugRunner, p: u8, effect: bool) {
    r.game.enqueue_triggered(
        EffectTiming::OnAddToHand,
        TriggerSource::HandGained {
            player: p,
            effect_initiated: effect,
        },
    );
    r.game.drain_effect_queue();
}
fn accept(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("choice exposed");
    let a = *v.valid_action_ids.iter().find(|a| **a != PASS).unwrap();
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    r.execute_action(0, a).unwrap();
}
fn drain(r: &mut DebugRunner) {
    for _ in 0..12 {
        if r.pending_selection().is_none() {
            return;
        }
        accept(r);
    }
    panic!("unexpected choice loop");
}
#[test]
fn bt27_032_wd_hand_buckets_and_kind_filters() {
    for n in [0, 3, 4, 7, 8, 9, 12] {
        let mut r = base().hand(1, &vec!["F"; n]).start();
        let m = r.place_on_field(0, ID, None);
        let a = r.place_on_field(1, "L3", None);
        let b = r.place_on_field(1, "T", None);
        r.place_on_field(0, "T", None);
        fire(&mut r, EffectTiming::WhenDigivolving, m);
        let count = (n / 4).min(2);
        for _ in 0..count {
            assert!(!r.pending_is_optional());
            accept(&mut r);
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(
            [a, b]
                .into_iter()
                .filter(|h| r.game.modifiers.has(*h, ModifierType::CannotSuspend))
                .count(),
            count
        );
    }
}
#[test]
fn bt27_032_actual_evolution_routes() {
    for id in ["L5", "SQUAD", "GAO"] {
        let mut r = base().hand(0, &[ID]).start();
        let h = r.place_on_field(0, id, Some(0));
        let a = encode_digivolve(0, h.index as u16);
        assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
        r.game.decode_action(a, 0);
        drain(&mut r);
        assert_eq!(top_id(&r, h), ID);
        assert_eq!(r.memory(), if id == "L5" { 6 } else { 7 });
    }
}
#[test]
fn bt27_032_wrong_level_cannot_evolve_or_play() {
    let mut r = base().hand(0, &[ID]).start();
    let h = r.place_on_field(0, "L3", Some(0));
    assert_eq!(
        build_action_mask(&r.game, 0)[encode_digivolve(0, h.index as u16) as usize],
        0.0
    );
    assert!(r.play(0, 0).is_none());
}
#[test]
fn bt27_032_effect_gain_targets_only_highest_ties() {
    let mut r = setup();
    r.place_on_field(0, ID, None);
    r.place_on_field(1, "L3", None);
    r.place_on_field(1, "L6", None);
    r.place_on_field(1, "L6", None);
    r.place_on_field(1, "T", None);
    gain(&mut r, 1, true);
    accept(&mut r);
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids.len(),
        2
    );
    accept(&mut r);
    assert_eq!(hand_ids(&r, 1), vec!["L6"]);
    assert!(field_ids(&r, 1).contains(&"L3".into()));
    assert_eq!(r.memory(), 10);
    assert!(r.game.pending_option.is_none());
}
#[test]
fn bt27_032_decline_keeps_target() {
    let mut r = setup();
    r.place_on_field(0, ID, None);
    r.place_on_field(1, "L6", None);
    gain(&mut r, 1, true);
    pass(&mut r, 0);
    assert_eq!(field_ids(&r, 1), vec!["L6"]);
}
#[test]
fn bt27_032_own_gain_and_rule_draw_do_not_trigger() {
    for (p, e) in [(0, true), (1, false)] {
        let mut r = setup();
        r.place_on_field(0, ID, None);
        r.place_on_field(1, "L6", None);
        gain(&mut r, p, e);
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn bt27_032_option_effect_attribution() {
    for (kind, bounced) in [
        (EffectSourceKind::Digimon, true),
        (EffectSourceKind::Option, false),
    ] {
        let mut r = setup();
        r.place_on_field(0, ID, None);
        let t = r.place_on_field(1, "L6", None);
        let mut x = ModifierEntry::simple(ModifierType::CannotBeAffected, 0, Expiry::Permanent, 0);
        x.effect_immunity_filter = Some(EffectImmunityFilter {
            source_kind: Some(kind),
            controller: EffectControllerFilter::OpponentOnly,
        });
        r.game.modifiers.add(t, x);
        gain(&mut r, 1, true);
        drain(&mut r);
        assert_eq!(hand_ids(&r, 1).contains(&"L6".into()), bounced);
    }
}
#[test]
fn bt27_032_real_option_use_pays_and_offers_arts() {
    let mut r = base().hand(0, &[ID]).start();
    let uses = std::sync::Arc::new(std::sync::Mutex::new(0));
    r.register_effect("ST", std::sync::Arc::new(OptionUseWitness(uses.clone())));
    r.place_on_field(0, "ST", None);
    r.place_on_field(0, "SQUAD", Some(0));
    r.place_on_field(1, "L6", None);
    let _ = r.game.play_option_from_hand(0, 0);
    accept(&mut r);
    assert_eq!(r.memory(), 5);
    assert_eq!(hand_ids(&r, 1), vec!["L6"]);
    assert!(
        r.pending_selection().is_some(),
        "Arts choice after real use"
    );
    drain(&mut r);
    assert!(field_ids(&r, 0).contains(&ID.into()));
    assert_eq!(*uses.lock().unwrap(), 1, "actual use emits OnUseOption");
}
fn source_event(
    r: &mut DebugRunner,
    owner: u8,
    tamer_host: bool,
    departed: bool,
    cause: EventCause,
) {
    let h = r.place_stack(owner, &["F", if tamer_host { "T" } else { "L5" }]);
    let host_card = r.game.players[owner as usize].battle_area[h.index as usize]
        .top_card()
        .handle();
    let card =
        r.game.players[owner as usize].battle_area[h.index as usize].card_sources[0].handle();
    if departed {
        r.place_on_field(owner, if tamer_host { "L5" } else { "T" }, None);
        // Preserve actual card objects in a real zone after the host departs.
        let removed = r.game.players[owner as usize]
            .battle_area
            .remove(h.index as usize);
        for source in removed.card_sources {
            r.game.players[source.owner as usize].put_in_trash(source);
        }
    }
    r.game.enqueue_triggered(
        EffectTiming::OnDigivolutionCardTrashed,
        TriggerSource::SourceTrashedFromStack {
            player: owner,
            host: h,
            host_card,
            card,
            cause,
        },
    );
    r.game.drain_effect_queue();
}
#[test]
fn bt27_032_source_trash_effect_owner_and_kind_filters() {
    for (owner, t, yes) in [(0, true, true), (1, true, false), (0, false, false)] {
        let mut r = setup();
        r.place_on_field(0, ID, None);
        r.place_on_field(1, "L6", None);
        source_event(&mut r, owner, t, false, EventCause::OwnEffect);
        assert_eq!(r.pending_selection().is_some(), yes);
    }
}
#[test]
fn bt27_032_departed_tamer_uses_captured_host_identity() {
    let mut r = setup();
    r.place_on_field(0, ID, None);
    r.place_on_field(1, "L6", None);
    source_event(&mut r, 0, true, true, EventCause::OwnEffect);
    assert!(
        r.pending_selection().is_some(),
        "captured Tamer remains qualifying after departure and compaction"
    );
    drain(&mut r);
    assert!(hand_ids(&r, 1).contains(&"L6".into()));
}
#[test]
fn bt27_032_departed_digimon_does_not_borrow_tamer_identity() {
    let mut r = setup();
    r.place_on_field(0, ID, None);
    r.place_on_field(1, "L6", None);
    source_event(&mut r, 0, false, true, EventCause::OwnEffect);
    assert!(r.pending_selection().is_none());
}
#[test]
fn bt27_032_rule_cleanup_is_not_effect_trash() {
    let mut r = setup();
    r.place_on_field(0, ID, None);
    r.place_on_field(1, "L6", None);
    source_event(&mut r, 0, true, false, EventCause::Rule);
    assert!(r.pending_selection().is_none());
}
#[test]
fn bt27_032_opt_shared_between_hand_and_source_events() {
    let mut r = setup();
    r.place_on_field(0, ID, None);
    r.place_on_field(1, "L6", None);
    r.place_on_field(1, "L6", None);
    gain(&mut r, 1, true);
    drain(&mut r);
    source_event(&mut r, 0, true, false, EventCause::Cost);
    assert!(r.pending_selection().is_none());
    assert_eq!(hand_ids(&r, 1).len(), 1);
}
#[test]
fn bt27_032_lock_expiry_and_reboot() {
    let mut r = base().hand(1, &["F"; 4]).start();
    let m = r.place_on_field(0, ID, Some(0));
    let t = r.place_on_field(1, "L3", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, m);
    accept(&mut r);
    r.game.players[0].battle_area[m.index as usize].is_suspended = true;
    r.end_turn();
    assert!(
        !r.game.players[0].battle_area[m.index as usize].is_suspended,
        "Reboot during opponent unsuspend phase"
    );
    assert!(r.game.modifiers.has(t, ModifierType::CannotSuspend));
    r.end_turn();
    assert!(!r.game.modifiers.has(t, ModifierType::CannotSuspend));
}
#[test]
fn bt27_032_printed_keywords_metadata() {
    use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause};
    let r = setup();
    let c = r.compiled_card(ID).unwrap();
    assert_eq!(c.dp, Some(12000));
    assert_eq!(c.level, Some(6));
    for name in ["Reboot", "Blocker", "Evade"] {
        assert!(c.effects.iter().any(|e|matches!(e,CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword{keyword,..}) if keyword==name)));
    }
}
struct OptionUseWitness(std::sync::Arc<std::sync::Mutex<u32>>);
impl digimon_engine::effect::CardEffect for OptionUseWitness {
    fn effects(
        &self,
        card: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        let n = self.0.clone();
        vec![digimon_engine::effect::Effect::on_play(card)
            .timing(EffectTiming::OnUseOption)
            .process(move |_| {
                *n.lock().unwrap() += 1;
            })
            .build()]
    }
}
struct DrawOpponent;
impl digimon_engine::effect::CardEffect for DrawOpponent {
    fn effects(
        &self,
        card: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        vec![digimon_engine::effect::Effect::on_play(card)
            .process(|ctx| {
                ctx.draw(1, 1);
            })
            .build()]
    }
}
#[test]
fn bt27_032_actual_draw_effect_direct_main_does_not_use_option() {
    let mut r = setup();
    let uses = std::sync::Arc::new(std::sync::Mutex::new(0));
    r.register_effect("T", std::sync::Arc::new(OptionUseWitness(uses.clone())));
    r.register_effect("L3", std::sync::Arc::new(DrawOpponent));
    r.place_on_field(0, ID, None);
    r.place_on_field(0, "T", None);
    r.place_on_field(1, "L6", None);
    let drawer = r.place_on_field(0, "L3", None);
    r.fire_on_play(0, drawer.index as usize);
    drain(&mut r);
    assert_eq!(hand_ids(&r, 1), vec!["F", "L6"]);
    assert_eq!(*uses.lock().unwrap(), 0);
    assert!(r.game.pending_option.is_none());
    assert_eq!(r.memory(), 10);
}
