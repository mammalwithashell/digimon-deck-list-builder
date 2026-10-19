//! BT16-025 printed WD ordering and lasting player-wide DNA restriction.
//! Source: actual BT16-025 image; DCGO BT16/Blue/BT16_025.cs WD coroutine.
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::SelectionKind;
fn make_digimon(id:&str,name:&str,level:u8,dp:i32)->CardData {
    let mut c=make_test_card(id,name);
    c.card_kind=CardKind::Digimon;c.level=Some(level);c.dp=Some(dp);c
}

/// Printed BT16-025 has one WD effect: suspend, THEN the DNA-only lock.
/// Image and official text; DCGO BT16_025.cs applies IsJogress in the same coroutine.
#[test]
fn bt16_025_dna_wd_resolves_both_parts_without_artificial_order_choice() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-025").unwrap()
        .add_card(make_digimon("ONE-A", "MaterialA", 4, 4000))
        .add_card(make_digimon("ONE-B", "MaterialB", 4, 4000))
        .add_card(make_digimon("ONE-OPP", "Opponent", 4, 4000))
        .add_card(make_test_card("ONE-DRAW", "Draw"))
        .hand(0, &["BT16-025"]).deck(0, &["ONE-DRAW"])
        .memory(10).start();
    let a=runner.place_on_field(0,"ONE-A",None);
    let b=runner.place_on_field(0,"ONE-B",None);
    let opp=runner.place_on_field(1,"ONE-OPP",None);
    let card=runner.game.player(0).hand[0].handle();
    EffectContext::new(&mut runner.game,card,None,0)
        .effect_initiated_dna_digivolve(a,b,card,0,true);
    assert!(runner.game.pending_selection.is_none(),
        "one printed mandatory WD must not create a choice between its suspend and Then DNA parts");
    assert!(runner.game.players[1].battle_area[opp.index as usize].is_suspended);
    runner.game.unsuspend_with_cause(opp,true);
    assert!(runner.game.players[1].battle_area[opp.index as usize].is_suspended,
        "DNA rider prevents effect unsuspension after the suspend part");
}

/// A real observer competes with the complete WD, never with its two halves.
#[test]
fn bt16_025_dna_wd_and_tamer_offer_two_real_effects_with_clone_safe_origin() {
    use digimon_engine::action::mask::build_action_mask;
    use digimon_engine::action::space::PASS;
    let mut runner=DebugRunner::builder().dsl_card("BT16-025").unwrap()
        .dsl_card("BT16-085").unwrap()
        .add_card(make_digimon("JOINT-A","MaterialA",4,4000))
        .add_card(make_digimon("JOINT-B","MaterialB",4,4000))
        .add_card(make_digimon("JOINT-OPP","Opponent",4,4000))
        .add_card(make_test_card("JOINT-DRAW","Draw"))
        .hand(0,&["BT16-025"]).deck(0,&["JOINT-DRAW"])
        .memory(10).start();
    runner.place_on_field(0,"BT16-085",None);
    let a=runner.place_on_field(0,"JOINT-A",None);
    let b=runner.place_on_field(0,"JOINT-B",None);
    let opp=runner.place_on_field(1,"JOINT-OPP",None);
    let card=runner.game.player(0).hand[0].handle();
    EffectContext::new(&mut runner.game,card,None,0).effect_initiated_dna_digivolve(a,b,card,0,true);
    let pending=runner.game.pending_selection.as_ref().expect("WD plus Tamer trigger order");
    let choices=pending.effect_choices.as_ref().expect("effect identities");
    assert_eq!(choices.len(),2,"only the complete WD and Davis/Ken are independent effects");
    assert!(!choices.iter().any(|c|c.timing==Some(EffectTiming::OnDnaDigivolve)));
    let first=choices.iter().find(|c|c.timing==Some(EffectTiming::WhenDigivolving)).unwrap().action_id;
    for mut game in [runner.game.clone(), runner.game] {
        assert!(build_action_mask(&game,0)[first as usize]>0.5);
        game.resolve_selection(0,first).unwrap();
        let optional=game.pending_selection.as_ref().expect("Davis/Ken optional cost remains");
        assert_eq!(optional.kind,SelectionKind::Replacement);
        assert!(build_action_mask(&game,0)[PASS as usize]>0.5);
        game.resolve_selection(0,PASS).unwrap();
        game.drain_effect_queue();
        assert!(game.pending_selection.is_none());
        assert!(game.players[1].battle_area[opp.index as usize].is_suspended);
        game.unsuspend_with_cause(opp,true);
        assert!(game.players[1].battle_area[opp.index as usize].is_suspended,
            "DNA context must survive choosing the full WD first from a cloned real ordering prompt");
    }
}

/// Printed "none of their Digimon" creates a lasting player-wide restriction.
/// DCGO BT16_025.cs:177 uses GainCanNotUnsuspendPlayerEffect with a live
/// opponent battle-area Digimon predicate, including cards that arrive later.
#[test]
fn bt16_025_dna_lock_covers_later_arrival_but_not_tamers_and_expires() {
    let mut tamer=make_test_card("LATE-TAMER","Late Tamer");
    tamer.card_kind=CardKind::Tamer;
    let mut runner=DebugRunner::builder().dsl_card("BT16-025").unwrap()
        .add_card(make_digimon("LATE-A","MaterialA",4,4000))
        .add_card(make_digimon("LATE-B","MaterialB",4,4000))
        .add_card(make_digimon("LATE-DIGI","Later Digimon",3,2000))
        .add_card(tamer).add_card(make_test_card("LATE-FILL","Draw"))
        .hand(0,&["BT16-025"])
        .deck(0,&["LATE-FILL","LATE-FILL","LATE-FILL"])
        .deck(1,&["LATE-FILL","LATE-FILL","LATE-FILL"])
        .memory(10).start();
    let a=runner.place_on_field(0,"LATE-A",None);
    let b=runner.place_on_field(0,"LATE-B",None);
    let card=runner.game.player(0).hand[0].handle();
    EffectContext::new(&mut runner.game,card,None,0).effect_initiated_dna_digivolve(a,b,card,0,true);
    runner.auto_resolve().expect("finish WD before new arrivals");
    let late=runner.place_on_field(1,"LATE-DIGI",None);
    let tamer=runner.place_on_field(1,"LATE-TAMER",None);
    runner.game.players[1].battle_area[late.index as usize].is_suspended=true;
    runner.game.players[1].battle_area[tamer.index as usize].is_suspended=true;
    runner.game.unsuspend_with_cause(late,true);
    runner.game.unsuspend_with_cause(tamer,true);
    assert!(runner.game.players[1].battle_area[late.index as usize].is_suspended,
        "a Digimon entering after DNA resolution cannot unsuspend during the player-wide duration");
    assert!(!runner.game.players[1].battle_area[tamer.index as usize].is_suspended,
        "the printed restriction applies to Digimon, not Tamers");
    runner.end_turn();
    assert!(runner.game.players[1].battle_area[late.index as usize].is_suspended,
        "the opponent's unsuspend phase is inside the duration");
    runner.end_turn();
    runner.game.unsuspend_with_cause(late,true);
    assert!(!runner.game.players[1].battle_area[late.index as usize].is_suspended,
        "the restriction expires at the opponent's turn end");
}
