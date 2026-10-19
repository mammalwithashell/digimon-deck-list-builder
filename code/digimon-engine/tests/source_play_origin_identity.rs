//! Two chosen source cards must retain remaining plays across replacement parking.
use std::sync::Arc;
use digimon_engine::action::{build_action_mask, space::{PASS,REPLACEMENT_ACCEPT}};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card,DebugRunner};
use digimon_engine::effect::{CardEffect,Effect};
use digimon_engine::enums::{EffectTiming,GamePhase};
use digimon_engine::selection::{SelectionKind,TriggerSource};
const DRIVER:&str=r#"
card: BATCH-CARRIER
name: Batch Carrier
kind: digimon
level: 6
color: [black]
cost: 10
dp: 10000
effects:
  - when: on_play
    process:
      - select_materials:
          of_permanent: source
          max: 2
          uniqueness: name
          bind_as: picked
          prompt: Choose source pair
      - play_from_materials:
          target: source
          source_index: picked
          cost_delta: free
      - draw: { of: you, count: 1 }
"#;
struct Gate;
impl CardEffect for Gate {
    fn effects(&self,card:CardHandle)->Vec<Effect>{vec![Effect::when_permanent_would_play(card)
        .optional().name("Refuse selected source play")
        .replacement_condition(|ctx,subject|match subject{
            digimon_engine::replacement::ReplacementSubject::Card(h,_)=>ctx.game.card_data_for_handle(*h).is_some_and(|d|d.card_id=="BATCH-A" || d.card_id=="BATCH-B"),_=>false})
        .replacement_process(|ctx|ctx.cancel()).build()]}
}
fn act(r:&mut DebugRunner,actor:u8,action:u16){
    assert!(build_action_mask(&r.game,actor)[action as usize]>0.0);r.game.resolve_selection(actor,action).unwrap();
}

// This fixture creates no links or reveal cards. Count every physical-card zone
// it can use, so adversarial host replacement cannot discard a selected source.
fn physical_occurrences(r: &DebugRunner, handle: CardHandle) -> usize {
    r.game.players.iter().map(|p| {
        p.hand.iter().chain(p.deck.iter()).chain(p.trash.iter())
            .chain(p.security.iter()).chain(p.digitama_deck.iter())
            .filter(|c| c.handle() == handle).count()
        + p.battle_area.iter().map(|body| body.card_sources.iter()
            .filter(|c| c.handle() == handle).count()).sum::<usize>()
        + p.breeding_area.as_ref().map_or(0, |body| body.card_sources.iter()
            .filter(|c| c.handle() == handle).count())
    }).sum()
}

#[test]
fn refused_source_returns_to_captured_host_after_compaction_but_not_fresh_same_card_host(){
    for fresh in [false,true]{
        let mut r=DebugRunner::builder().from_dsl_yaml(&DRIVER.replace("max: 2","max: 1")).unwrap()
            .add_card(make_test_card("BATCH-A","Batch Alpha"))
            .add_card(make_test_card("BATCH-GATE","Batch Gate"))
            .add_card(make_test_card("BATCH-FILL","Batch Filler"))
            .deck(0,&["BATCH-FILL";20]).deck(1,&["BATCH-FILL";20]).memory(10).start();
        r.skip_mulligan();r.set_first_player(0);r.game.current_phase=GamePhase::Main;
        r.place_on_field(0,"BATCH-FILL",None);
        let host=r.place_stack(0,&["BATCH-A","BATCH-CARRIER"]);
        assert_eq!(host.index,1);
        r.register_effect("BATCH-GATE",Arc::new(Gate));r.place_on_field(0,"BATCH-GATE",None);
        let expected=r.game.player(0).battle_area[1].card_sources.iter().map(|c|c.handle()).collect::<Vec<_>>();
        r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(host));r.game.drain_effect_queue();
        let action=r.pending_selection_view().unwrap().valid_action_ids[0];act(&mut r,0,action);
        assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::Replacement);
        // Preparation leaves the selected physical source under its carrier;
        // it is not a hand-staged card waiting for refusal rollback.
        assert_eq!(r.game.player(0).battle_area[1].card_sources.iter()
            .map(|c| c.handle()).collect::<Vec<_>>(), expected);
        assert!(!r.game.player(0).hand.iter().any(|c| c.handle() == expected[0]));
        for handle in &expected { assert_eq!(physical_occurrences(&r, *handle), 1); }
        // Controlled board lifecycle changes while the genuine source play is
        // pending, retaining physical card IDs so only permanent identity differs.
        let lower=r.game.player_mut(0).battle_area.remove(0);r.game.player_mut(0).trash.extend(lower.card_sources);
        if fresh{
            let mut old=r.game.player_mut(0).battle_area.remove(0);
            let top=old.card_sources.pop().unwrap();
            assert_eq!(top.handle(), expected[1]);
            assert_eq!(old.card_sources.iter().map(|c| c.handle()).collect::<Vec<_>>(),
                vec![expected[0]], "selected source still belongs to the departed carrier");
            // Adversarial lifecycle fixture: dispose the departed carrier's
            // lower cards instead of dropping them from the Game in `old`.
            r.game.player_mut(0).trash.extend(old.card_sources);
            r.game.player_mut(0).battle_area.insert(0,digimon_engine::permanent::Permanent::new(top,1));
        }
        for handle in &expected { assert_eq!(physical_occurrences(&r, *handle), 1); }
        let snapshot=r.game.clone();
        for cloned in [false,true]{
            if cloned{r.game=snapshot.clone();}
            act(&mut r,0,REPLACEMENT_ACCEPT);
            assert!(r.pending_selection_view().is_none());
            let restored=r.game.player(0).battle_area[0].card_sources.iter().map(|c|c.handle()).collect::<Vec<_>>();
            if fresh{assert_eq!(restored,vec![expected[1]],"fresh same-card host cannot receive old source");}
            else{assert_eq!(restored,expected,"source follows captured host through compaction");}
            assert_eq!(r.game.player(0).battle_area[1].card_sources.len(),1,"unrelated gate cannot receive source");
            assert_eq!(r.game.player(0).trash.iter().filter(|c|c.card_id(&r.game.card_data)=="BATCH-A").count(),if fresh{1}else{0});
            assert_eq!(r.game.player(0).trash.iter().filter(|c| c.handle() == expected[0]).count(),
                if fresh { 1 } else { 0 }, "exact selected source remains in its physical destination");
            for handle in &expected { assert_eq!(physical_occurrences(&r, *handle), 1); }
        }
    }
}
