//! Q1921: exposing an Option by De-Digivolve trashes it and its stack, not deletion.
//! Official BT9-109 observation e7d98de2f242c2f3626fd9ab538063629890092bbcfa8fe39ac4329b4c01fea0.
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{GamePhase, DelayTrigger};
use digimon_engine::permanent::OptionState;
use digimon_engine::permanent::PermanentHandle;

fn fixture() -> DebugRunner {
    let mut metadata = digimon_engine::card_data::CardData::load_from_str(include_str!("../../../data/cards.json")).unwrap();
    // Diagnostic observer only: not a replacement implementation of any printed card.
    // Its positive control distinguishes rule trash from a deletion dispatch.
    let mut builder = DebugRunner::builder().dsl_card("BT25-075").unwrap().dsl_card("BT25-014").unwrap().from_dsl_yaml(r#"
card: Q1921-DELETE-OBSERVER
name: Deletion dispatch witness
kind: tamer
color: [white]
cost: 0
effects:
  - when: on_any_deletion
    process:
      - gain_memory: 1
"#).unwrap().from_dsl_yaml(include_str!("../cards/BT9/BT9-109.yaml")).unwrap();
    for id in ["BT9-109", "BT9-062", "ST5-15", "ST1-03", "BT1-085", "P-035", "BT20-045"] {
        builder = builder.add_card(metadata.remove(id).unwrap());
    }
    let mut r = builder.deck(0, &["ST1-03";12]).deck(1, &["ST1-03";12]).memory(0).start();
    r.skip_mulligan(); r.game.current_phase=GamePhase::Main;
    r.place_on_field(0,"Q1921-DELETE-OBSERVER",None);
    r
}
fn stack(r: &mut DebugRunner, cards: &[&str]) -> PermanentHandle {
    let handle=r.place_on_field(1,cards[0],None);
    for id in &cards[1..] {
        let index=r.add_to_hand(1,id);
        let card=r.game.players[1].hand.remove(index);
        r.game.players[1].battle_area[handle.index as usize].card_sources.push(card);
    }
    r.game.tick_declarative_effects(); handle
}
fn degenerate(r:&mut DebugRunner, target:PermanentHandle) {
    let index=r.add_to_hand(0,"ST5-15");let card=r.game.players[0].hand[index].handle();
    assert_eq!(EffectContext::new(&mut r.game,card,None,0).de_digivolve(target,Some(3),Some(1)),1);
    r.game.drain_effect_queue();
}
#[test]
fn exposed_x_antibody_is_trashed_without_deletion_in_both_clones() {
    let mut r=fixture();let target=stack(&mut r,&["BT9-109","BT9-062"]);let frozen=r.game.clone();
    for clone in [false,true] {
        if clone {r.game=frozen.clone();}
        degenerate(&mut r,target);
        assert!(r.game.players[1].battle_area.is_empty(),"Option exposed by De-Digivolve cannot remain");
        let mut trash:Vec<_>=r.game.players[1].trash.iter().map(|c|c.card_id(&r.game.card_data)).collect();trash.sort();
        assert_eq!(trash,vec!["BT9-062","BT9-109"]);
        assert_eq!(r.game.memory,0,"rule trash must not dispatch On Deletion");
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn exposed_option_trashes_remaining_lower_sources_too() {
    let mut r=fixture();let target=stack(&mut r,&["ST1-03","BT9-109","BT9-062"]);
    degenerate(&mut r,target);assert!(r.game.players[1].battle_area.is_empty());
    let mut trash:Vec<_>=r.game.players[1].trash.iter().map(|c|c.card_id(&r.game.card_data)).collect();trash.sort();
    assert_eq!(trash,vec!["BT9-062","BT9-109","ST1-03"]);assert_eq!(r.game.memory,0);
}
#[test]
fn exposed_tamer_stays_and_unrelated_option_permanent_is_preserved() {
    let mut r=fixture();let target=stack(&mut r,&["BT1-085","BT9-062"]);
    let option=r.place_on_field(1,"P-035",None);
    r.game.players[1].battle_area[option.index as usize].option_state=OptionState::Delayed {owner:1, trash_on_turn:u16::MAX, trigger:DelayTrigger::MainPhaseActivated, placed_on_turn:0};
    degenerate(&mut r,target);
    let ids:Vec<_>=r.game.players[1].battle_area.iter().map(|p|p.top_card().card_id(&r.game.card_data)).collect();
    assert_eq!(ids,vec!["BT1-085","P-035"]);assert_eq!(r.game.memory,0);
}
#[test]
fn deletion_witness_control_observes_real_deletion() {
    let mut r=fixture();let target=stack(&mut r,&["ST1-03"]);
    r.game.delete_permanent_with_effects(target);r.game.drain_effect_queue();
    assert!(r.game.players[1].battle_area.is_empty());assert_eq!(r.game.memory,1);
}

#[test]
fn rule_trash_clears_removed_slot_and_rekeys_survivor_modifiers() {
    use digimon_engine::enums::{Expiry,ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
    let mut r=fixture();let target=stack(&mut r,&["BT9-109","BT9-062"]);
    let survivor=r.place_on_field(1,"ST1-03",None);
    r.game.modifiers.add(target,ModifierEntry::simple(ModifierType::CannotAttack,0,Expiry::Permanent,0));
    r.game.modifiers.add(survivor,ModifierEntry::simple(ModifierType::CannotBlock,0,Expiry::Permanent,0));
    degenerate(&mut r,target);
    let shifted=PermanentHandle{player:1,index:0};
    assert_eq!(r.game.players[1].battle_area.len(),1);
    assert!(!r.game.modifiers.has(shifted,ModifierType::CannotAttack));
    assert!(r.game.modifiers.has(shifted,ModifierType::CannotBlock));
}

#[test]
fn rule_trash_applies_overflow_to_lower_source_and_linked_card_once() {
    let mut r=fixture();let target=stack(&mut r,&["BT20-045","BT9-109","BT9-062"]);
    let index=r.add_to_hand(1,"BT20-045");let linked=r.game.players[1].hand.remove(index);
    r.game.players[1].battle_area[target.index as usize].linked_cards.push(linked);
    degenerate(&mut r,target);
    assert!(r.game.players[1].battle_area.is_empty());
    assert_eq!(r.game.players[1].trash.iter().filter(|c|c.card_id(&r.game.card_data)=="BT20-045").count(),2);
    assert_eq!(r.game.memory,10,"P1-owned lower and linked ACE each lose 5, on P0 turn");
}
#[test]
fn legitimate_field_option_states_are_not_caught_by_exposed_option_rule() {
    let states=[
        OptionState::Delayed {owner:1,trash_on_turn:u16::MAX,trigger:DelayTrigger::MainPhaseActivated,placed_on_turn:0},
        OptionState::OrdinaryFieldOption,
        OptionState::Training {owner:1,trained:None},
        OptionState::Linked {host:PermanentHandle{player:0,index:0}},
        OptionState::OrphanedPlugIn {last_carrier_owner:1},
    ];
    for state in states {
        let mut r=fixture();let option=r.place_on_field(1,"P-035",None);
        r.game.players[1].battle_area[option.index as usize].option_state=state;
        r.game.drain_effect_queue();
        assert_eq!(r.game.players[1].battle_area.len(),1,"unrelated existing OptionState must retain its own lifecycle: {state:?}");
        assert_eq!(r.game.memory,0);
    }
}

#[test]
fn rule_trash_overflow_preserves_compacted_survivor_aura_in_clones() {
    use digimon_engine::enums::{Expiry,Keyword,ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
    let mut r=fixture();r.place_on_field(1,"BT25-075",None);
    let target=stack(&mut r,&["BT20-045","BT9-109","BT9-062"]);
    let survivor=r.place_on_field(1,"BT25-014",None);
    let physical=r.game.players[1].battle_area[survivor.index as usize].top_card().handle();
    r.game.modifiers.add(survivor,ModifierEntry::simple(ModifierType::ChangeDp,1000,Expiry::Permanent,1));
    r.game.tick_declarative_effects();assert!(r.game.has_keyword(survivor,Keyword::Rush));
    let expected_dp=r.game.effective_dp(survivor);let frozen=r.game.clone();
    for clone in [false,true] {
        if clone {r.game=frozen.clone();}
        degenerate(&mut r,target);
        let shifted=PermanentHandle{player:1,index:1};
        assert_eq!(r.game.players[1].battle_area[1].top_card().handle(),physical);
        assert_eq!(r.game.memory,5);
        for _ in 0..2 {
            assert!(r.game.has_keyword(shifted,Keyword::Rush));
            assert!(r.game.modifiers.has(shifted,ModifierType::ChangeLinkMax));
            assert_eq!(r.game.effective_dp(shifted),expected_dp);
            r.game.tick_declarative_effects();
        }
    }
}
