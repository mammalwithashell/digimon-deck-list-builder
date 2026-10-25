use digimon_engine::{action::space::PASS, card_choice_page::CARD_CHOICE_START, debug_runner::{make_test_card,DebugRunner}, enums::CardKind};
fn fixture() -> &'static str { r#"
card: DX-OWNED
name: Owned DigiXros
kind: digimon
level: 4
color: [red]
cost: 6
dp: 6000
alt_paths:
  - kind: digixros
    materials:
      - filter: {name_is: Piece}
        repeat: {min: 1, max: 2}
        zones: [material]
        cost_delta: -1
    cost: 6
"# }
#[test]
fn digixros_under_tamer_reaches_deep_physical_sources_and_clones_prefix() {
    for (player, depth) in [0,1].into_iter().flat_map(|player| [12usize,39,300,11].into_iter().map(move |depth| (player,depth))) {
        let mut tamer=make_test_card("DX-TAMER","Tamer"); tamer.card_kind=CardKind::Tamer;tamer.level=None;tamer.dp=None;
        let mut r=DebugRunner::builder().from_dsl_yaml(fixture()).unwrap()
            .add_card(make_test_card("DX-PIECE","Piece")).add_card(make_test_card("DX-FILL","Other"))
            .add_card(tamer).hand(player,&["DX-OWNED","DX-PIECE"]).memory(10).start();
        r.skip_mulligan(); r.game.turn_player_idx = player as usize;
        let mut stack=vec!["DX-FILL";depth];stack.extend(["DX-PIECE","DX-PIECE","DX-TAMER"]);
        let host=r.place_stack(player,&stack);
        let expected=r.game.player(player).battle_area[0].card_sources[depth].handle();
        // An ally battle top and opponent sources are not in this recipe's allowed zone.
        r.place_stack(player,&["DX-PIECE"]);r.place_stack(1-player,&["DX-PIECE","DX-TAMER"]);
        r.play(player,0);
        let pending=r.game.pending_selection.as_ref().expect("deep source candidates must install a DigiXros material prompt");
        let owner=pending.card_choice.as_ref().expect("full DigiXros menu must use physical owned choices");
        let view=owner.view(&r.game,player);
        assert_eq!(view.rows.iter().filter(|row|row.card_id.is_some()).count(),2,"only own below-Tamer recipe pieces are eligible");
        assert_eq!(view.rows[0].position.as_deref(),Some(depth.to_string().as_str()));
        r.game.resolve_selection(player,CARD_CHOICE_START).unwrap();
        let mut clone=r.game.clone();
        for game in [&mut r.game,&mut clone] {
            assert_eq!(game.player(player).battle_area[host.index as usize].card_sources.len(), depth + 3,
                "selected prefix stays physically unpaid until transaction commit");
            game.resolve_selection(player,CARD_CHOICE_START).unwrap();
            game.resolve_selection(player,PASS).unwrap();
            let played=game.player(player).battle_area.iter().find(|p|p.top_card().card_id(&game.card_data)=="DX-OWNED").expect("paid DigiXros plays target");
            assert_eq!(played.card_sources.len(),3);
            assert!(played.card_sources.iter().any(|c|c.handle()==expected));
            assert_eq!(game.player(player).battle_area[host.index as usize].card_sources.len(),depth+1);
        }
    }
}

#[test]
fn digixros_source_prefix_follows_carrier_and_source_compaction_until_payment() {
    use digimon_engine::card_choice_page::CARD_CHOICE_NEXT;
    let mut tamer=make_test_card("DX-TAMER","Tamer");tamer.card_kind=CardKind::Tamer;tamer.level=None;tamer.dp=None;
    let mut r=DebugRunner::builder().from_dsl_yaml(fixture()).unwrap()
        .add_card(make_test_card("DX-PIECE","Piece")).add_card(make_test_card("DX-FILL","Other"))
        .add_card(tamer).hand(0,&["DX-OWNED"]).memory(10).start();
    r.skip_mulligan();r.place_stack(0,&["DX-FILL"]);
    let mut stack=vec!["DX-PIECE";40];stack.push("DX-TAMER");let host=r.place_stack(0,&stack);
    let expected=r.game.player(0).battle_area[host.index as usize].card_sources[39].handle();
    r.play(0,0);
    r.game.resolve_selection(0,CARD_CHOICE_NEXT).unwrap();
    r.game.resolve_selection(0,CARD_CHOICE_START+11).unwrap();
    r.game.players[0].battle_area.remove(0);
    r.game.players[0].battle_area[0].card_sources.remove(0);
    r.game.resolve_selection(0,CARD_CHOICE_START+1).unwrap();
    r.game.resolve_selection(0,PASS).unwrap();
    let played=r.game.player(0).battle_area.iter().find(|p|p.top_card().card_id(&r.game.card_data)=="DX-OWNED").unwrap();
    assert_eq!(played.card_sources.len(),3,"both physical materials must be paid after compaction");
    assert!(played.card_sources.iter().any(|c|c.handle()==expected));
    assert_eq!(r.game.memory,6,"cost reduction requires both physically paid cards");
}

#[path = "digixros_conditional_trash_zone.rs"]
mod existing_conditional_zone_controls;

#[test]
fn digixros_complete_menu_preserves_deep_hand_trash_and_battle_top_payment() {
    use digimon_engine::{card_source::CardSource, owned_card_choice::CardChoiceZone};
    let yaml=fixture().replace("zones: [material]", "zones: [hand, battle_area, trash]");
    let mut hand=vec!["DX-OWNED"];hand.extend(["DX-FILL";65]);hand.push("DX-HAND");
    let mut r=DebugRunner::builder().from_dsl_yaml(&yaml).unwrap()
        .add_card(make_test_card("DX-HAND","Piece")).add_card(make_test_card("DX-FIELD","Piece"))
        .add_card(make_test_card("DX-TRASH","Piece")).add_card(make_test_card("DX-FILL","Other"))
        .hand(0,&hand).memory(10).start();
    r.skip_mulligan();
    let field=r.place_stack(0,&["DX-FIELD"]);
    let field_card=r.game.player(0).battle_area[field.index as usize].top_card().handle();
    for id in std::iter::repeat("DX-FILL").take(65).chain(std::iter::once("DX-TRASH")) {
        let data=r.game.card_data.iter().position(|card|card.card_id==id).unwrap();
        let physical=r.game.next_card_index();r.game.players[0].trash.push(CardSource::new(data,0,physical));
    }
    let trash_card=r.game.player(0).trash[65].handle();
    r.play(0,0);
    let view=r.game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap().view(&r.game,0);
    assert_eq!(view.rows.iter().filter(|row|row.card_id.is_some()).count(),3,"all eligible zones remain selectable without legacy caps");
    assert!(matches!(view.rows[0].zone,Some(CardChoiceZone::Hand)));
    assert_eq!(view.rows[0].position.as_deref(),Some("66"));
    assert!(matches!(view.rows[1].zone,Some(CardChoiceZone::BattleTop)));
    assert!(matches!(view.rows[2].zone,Some(CardChoiceZone::Trash)));
    assert_eq!(view.rows[2].position.as_deref(),Some("65"));
    r.game.resolve_selection(0,CARD_CHOICE_START+1).unwrap();
    let mut clone=r.game.clone();
    for game in [&mut r.game,&mut clone] {
        game.resolve_selection(0,CARD_CHOICE_START+1).unwrap();
        game.resolve_selection(0,PASS).unwrap();
        let played=game.player(0).battle_area.iter().find(|p|p.top_card().card_id(&game.card_data)=="DX-OWNED").unwrap();
        assert_eq!(played.card_sources.len(),3);
        assert!(played.card_sources.iter().any(|c|c.handle()==field_card));
        assert!(played.card_sources.iter().any(|c|c.handle()==trash_card));
        assert!(game.player(0).hand.iter().any(|c|c.card_id(&game.card_data)=="DX-HAND"));
        assert_eq!(game.memory,6);
    }
}

#[test]
fn digixros_named_recipe_cannot_use_face_down_source_printed_identity() {
    use digimon_engine::owned_card_choice::CardChoiceOperation;
    let mut tamer=make_test_card("DX-TAMER","Tamer");tamer.card_kind=CardKind::Tamer;tamer.level=None;tamer.dp=None;
    let mut r=DebugRunner::builder().from_dsl_yaml(fixture()).unwrap()
        .add_card(make_test_card("DX-PIECE","Piece")).add_card(tamer)
        .hand(0,&["DX-OWNED"]).memory(10).start();
    r.skip_mulligan();let host=r.place_stack(0,&["DX-PIECE","DX-PIECE","DX-TAMER"]);
    r.game.players[0].battle_area[host.index as usize].card_sources[0].face_down=true;
    r.play(0,0);
    let view=r.game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap().view(&r.game,0);
    assert_eq!(view.rows.iter().filter(|row|matches!(row.operation,CardChoiceOperation::CommitCard)).count(),1,
        "a face-down source has no printed name and cannot satisfy a named DigiXros recipe");
    assert_eq!(view.rows[0].position.as_deref(),Some("1"));
}

#[test]
fn digixros_unqualified_source_allowance_keeps_face_down_cards_legal_and_hidden() {
    use digimon_engine::{action::build_action_mask, owned_card_choice::CardChoiceOperation};
    let yaml=fixture().replace("filter: {name_is: Piece}", "filter: {}");
    let mut tamer=make_test_card("DX-TAMER","Tamer");tamer.card_kind=CardKind::Tamer;tamer.level=None;tamer.dp=None;
    let mut r=DebugRunner::builder().from_dsl_yaml(&yaml).unwrap()
        .add_card(make_test_card("DX-PIECE","Piece")).add_card(tamer)
        .hand(0,&["DX-OWNED"]).memory(10).start();
    r.skip_mulligan();let host=r.place_stack(0,&["DX-PIECE","DX-PIECE","DX-TAMER"]);
    r.game.players[0].battle_area[host.index as usize].card_sources[0].face_down=true;
    let expected=r.game.player(0).battle_area[0].card_sources[0].handle();
    r.play(0,0);
    let view=r.game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap().view(&r.game,0);
    assert_eq!(view.rows.iter().filter(|row|matches!(row.operation,CardChoiceOperation::CommitCard)).count(),2);
    assert!(view.rows[0].card_id.is_none(),"unqualified selection must not reveal hidden printed identity");
    assert_eq!(build_action_mask(&r.game,0)[CARD_CHOICE_START as usize],1.0);
    r.game.resolve_selection(0,CARD_CHOICE_START).unwrap();r.game.resolve_selection(0,PASS).unwrap();
    let played=r.game.player(0).battle_area.iter().find(|p|p.top_card().card_id(&r.game.card_data)=="DX-OWNED").unwrap();
    assert_eq!(played.card_sources.len(),2);
    assert_eq!(played.card_sources[0].handle(),expected);assert!(played.card_sources[0].face_down);
    assert_eq!(r.game.memory,5);
}

#[test]
fn digixros_named_material_mask_rechecks_face_down_identity_after_prompt_install() {
    use digimon_engine::action::build_action_mask;
    let mut tamer=make_test_card("DX-TAMER","Tamer");tamer.card_kind=CardKind::Tamer;tamer.level=None;tamer.dp=None;
    let mut r=DebugRunner::builder().from_dsl_yaml(fixture()).unwrap()
        .add_card(make_test_card("DX-PIECE","Piece")).add_card(tamer)
        .hand(0,&["DX-OWNED"]).memory(10).start();
    r.skip_mulligan();let host=r.place_stack(0,&["DX-PIECE","DX-PIECE","DX-TAMER"]);
    r.play(0,0);
    assert_eq!(build_action_mask(&r.game,0)[CARD_CHOICE_START as usize],1.0);
    r.game.players[0].battle_area[host.index as usize].card_sources[0].face_down=true;
    assert_eq!(build_action_mask(&r.game,0)[CARD_CHOICE_START as usize],0.0,"stale printed-name eligibility must leave the mask immediately");
    assert_eq!(build_action_mask(&r.game,0)[(CARD_CHOICE_START+1) as usize],1.0);
}
