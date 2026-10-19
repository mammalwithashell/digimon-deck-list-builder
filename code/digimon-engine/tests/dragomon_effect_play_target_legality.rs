//! Official BT8-097 Q4663/Q4664: effect controller owns the prohibition;
//! destination owner owns field capacity. DCGO EX5_060.cs filters targets with
//! CanPlayAsNewPermanent(cardSource, false, activateClass) before prompting.
use digimon_engine::action::{build_action_mask, space::PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{EffectTiming, Expiry, GamePhase, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

fn setup(actor: u8) -> (DebugRunner, PermanentHandle) {
    let mut data = CardData::load_from_str(include_str!("../../../data/cards.json")).unwrap();
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../cards/EX5/EX5-060.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/BT8/BT8-097.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/ST1/ST1-06.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/ST1/ST1-09.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/ST1/ST1-11.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/ST1/ST1-02.yaml")).unwrap();
    for id in ["EX5-060", "BT8-097", "ST1-06", "ST1-09", "ST1-11", "ST1-02"] {
        b = b.add_card(data.remove(id).unwrap());
    }
    let mut r = b.hand(0, &["BT8-097"]).hand(1, &["BT8-097"])
        .deck(0, &["ST1-02"; 12]).deck(1, &["ST1-02"; 12])
        .security(0, &["ST1-02"; 5]).security(1, &["ST1-02"; 5])
        .memory(10).start();
    r.skip_mulligan(); r.set_first_player(actor); r.game.current_phase = GamePhase::Main;
    let dragomon = r.place_on_field(actor, "EX5-060", None);
    r.inject_trash(1 - actor, "ST1-06");
    r.inject_trash(1 - actor, "ST1-09"); // level 5 is ineligible independently of locks
    (r, dragomon)
}
fn fire(r: &mut DebugRunner, source: PermanentHandle, timing: EffectTiming) {
    r.game.enqueue_triggered(timing, TriggerSource::Permanent(source));
    r.game.drain_effect_queue();
}
fn crimson(r: &mut DebugRunner, owner: u8) {
    r.set_first_player(owner); r.game.current_phase = GamePhase::Main;
    r.place_on_field(owner, "ST1-11", None); // real red source, survives own sweep
    assert!(build_action_mask(&r.game, owner)[PLAY_HAND_START as usize] > 0.0);
    r.game.decode_action(PLAY_HAND_START, owner);
    assert!(r.pending_selection().is_none());
    assert!(r.game.modifiers.player_has(1-owner, ModifierType::CannotPlayDigimonByEffect));
}
fn choose_legal_in_both_clones(r: &mut DebugRunner, actor: u8, before: usize) {
    let v = r.pending_selection_view().expect("unrestricted effect has legal target");
    assert_eq!(v.selecting_player, 1-actor);
    assert!(!v.is_optional);
    assert_eq!(v.valid_action_ids.len(), 1, "only real level4, never level5");
    let action = v.valid_action_ids[0]; let snapshot = r.game.clone();
    for cloned in [false, true] {
        if cloned { r.game = snapshot.clone(); }
        assert!(build_action_mask(&r.game, 1-actor)[action as usize] > 0.0);
        r.execute_action(1-actor, action).unwrap();
        assert!(r.pending_selection().is_none());
        assert_eq!(r.game.players[(1-actor) as usize].battle_area.len(), before+1);
        let played = r.game.players[(1-actor) as usize].battle_area.last().unwrap();
        assert_eq!(played.top_card().card_id(&r.game.card_data), "ST1-06");
        assert!(played.is_suspended);
        assert_eq!(r.trash_size(1-actor), 1);
    }
}
#[test]
fn restricted_dragomon_controller_has_no_impossible_target_on_both_triggers() {
    for actor in [0,1] { for timing in [EffectTiming::OnPlay, EffectTiming::WhenDigivolving] {
        let (mut r, source) = setup(actor); crimson(&mut r, 1-actor);
        let before = r.battle_area_size(1-actor); let snapshot = r.game.clone();
        for cloned in [false,true] {
            if cloned { r.game = snapshot.clone(); }
            fire(&mut r, source, timing);
            assert!(r.pending_selection().is_none(), "Q4664: prohibited play must not advertise a mandatory trash target");
            assert_eq!(r.battle_area_size(1-actor), before);
            assert_eq!(r.trash_size(1-actor), 3); // two Digimon plus that player's used Crimson Blaze
        }
    }}
}
#[test]
fn restricted_recipient_does_not_block_unrestricted_dragomon_controller() {
    for actor in [0,1] {
        let (mut r, source) = setup(actor); crimson(&mut r, actor);
        let before = r.battle_area_size(1-actor);
        fire(&mut r, source, EffectTiming::OnPlay);
        choose_legal_in_both_clones(&mut r, actor, before);
    }
}
#[test]
fn full_destination_field_hides_unpayable_play_choice() {
    for actor in [0,1] {
        let (mut r, source) = setup(actor);
        while r.battle_area_size(1-actor) < r.game.rules.field_slots as usize {
            r.place_on_field(1-actor, "ST1-02", None);
        }
        fire(&mut r, source, EffectTiming::OnPlay);
        assert!(r.pending_selection().is_none(), "receiver has no field slot");
        assert_eq!(r.trash_size(1-actor), 2);
    }
}
#[test]
fn full_effect_controllers_field_does_not_hide_receivers_legal_choice() {
    for actor in [0,1] {
        let (mut r, source) = setup(actor);
        while r.battle_area_size(actor) < r.game.rules.field_slots as usize {
            r.place_on_field(actor, "ST1-02", None);
        }
        fire(&mut r, source, EffectTiming::OnPlay);
        choose_legal_in_both_clones(&mut r, actor, 0);
    }
}
#[test]
fn tamer_only_play_prohibition_does_not_hide_digimon_target() {
    for actor in [0,1] {
        let (mut r, source) = setup(actor);
        r.game.modifiers.add_player_modifier(actor, PlayerModifierEntry::simple(
            ModifierType::CannotPlayTamerByEffect, 0, Expiry::Permanent, None, actor));
        fire(&mut r, source, EffectTiming::OnPlay);
        choose_legal_in_both_clones(&mut r, actor, 0);
    }
}
