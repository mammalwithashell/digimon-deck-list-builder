use digimon_engine::{action::{build_action_mask, space::{PASS, REPLACEMENT_ACCEPT}}, card_choice_page::{CARD_CHOICE_NEXT, CARD_CHOICE_START}, card_source::CardHandle, debug_runner::{make_test_card, DebugRunner}, effect::{CardEffect, Effect}, enums::Keyword, replacement::ReplacementCause, resume::ResumeFrame};
use std::sync::Arc;
struct FragmentTwo;
impl CardEffect for FragmentTwo {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        digimon_engine::cards::keyword_effects::keyword_to_auto_effect(Keyword::Fragment(2), card)
    }
}
#[test]
fn keyword_fragment_deep_owned_page_and_required_prefix_survive_clone() {
    for (player, depth) in [0, 1].into_iter().flat_map(|player| [11usize, 12, 39, 300].into_iter().map(move |depth| (player, depth))) {
        let mut r = DebugRunner::builder().add_card(make_test_card("FRAG-SOURCE", "Source"))
            .add_card(make_test_card("FRAG-TOP", "Top"))
            .deck(0, &["FRAG-SOURCE"; 20]).deck(1, &["FRAG-SOURCE"; 20]).memory(3).start();
        r.skip_mulligan(); r.register_effect("FRAG-TOP", Arc::new(FragmentTwo));
        let mut stack = vec!["FRAG-SOURCE"; depth + 1]; stack.push("FRAG-TOP");
        let host = r.place_stack(player, &stack);
        let expected = r.game.player(player).battle_area[0].card_sources[depth].handle();
        r.game.delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
        r.game.resolve_selection(player, REPLACEMENT_ACCEPT).unwrap();
        for _ in 0..depth / 28 { r.game.resolve_selection(player, CARD_CHOICE_NEXT).unwrap(); }
        r.game.resolve_selection(player, CARD_CHOICE_START + (depth % 28) as u16).unwrap();
        let Some(state) = r.game.pending_selection_resume.as_ref().and_then(find_count_frame) else { panic!("required second keyword pick remains parked"); };
        assert_eq!(state.accum, vec![expected], "typed row must commit the physical source rather than decode its reused action alias");
        assert_eq!(build_action_mask(&r.game, player)[PASS as usize], 0.0);
        let mut clone = r.game.clone();
        for game in [&mut r.game, &mut clone] {
            assert!(game.pending_selection.as_ref().unwrap().card_choice.is_some(), "next required pick retains paging owner");
            game.resolve_selection(player, CARD_CHOICE_START).unwrap();
            assert!(game.pending_selection.is_none());
            assert_eq!(game.player(player).battle_area.len(), 1, "full Fragment payment cancels deletion");
            assert_eq!(game.player(player).battle_area[0].card_sources.len(), depth);
            assert!(game.player(player).trash.iter().any(|c| c.handle() == expected));
        }
    }
}

fn find_count_frame(stack: &digimon_engine::resume::ResumeStack) -> Option<&digimon_engine::resume::NonDslCountCappedState> {
    for frame in &stack.frames {
        match frame {
            ResumeFrame::NonDslCountCappedStep(state) => return Some(state),
            ResumeFrame::ReplacementBody { inner, .. } => if let Some(state) = find_count_frame(inner) { return Some(state); },
            _ => {}
        }
    }
    None
}

#[path = "keyword_phase_d/fragment_n.rs"]
mod existing_fragment_controls;
#[path = "material_save_keyword.rs"]
mod existing_material_save_controls;

#[path = "keyword_phase_d/helpers.rs"]
mod helpers;

struct RecipePartition;
impl CardEffect for RecipePartition {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        digimon_engine::cards::keyword_effects::keyword_to_auto_effect(Keyword::Partition, card)
    }
    fn partition_slots(&self, _: bool) -> Option<Arc<Vec<digimon_dsl::compiled::CompiledPredicate>>> {
        Some(Arc::new(vec![
            digimon_dsl::compiled::CompiledPredicate { name_is: Some("Recipe A".into()), ..Default::default() },
            digimon_dsl::compiled::CompiledPredicate { name_is: Some("Recipe B".into()), ..Default::default() },
        ]))
    }
}
#[test]
fn keyword_partition_recipe_reaches_deep_sources_and_clones_before_commit() {
    for (player, depth) in [(0, 39usize), (1, 300usize)] {
        let mut r = DebugRunner::builder().add_card(make_test_card("RECIPE-A", "Recipe A"))
            .add_card(make_test_card("RECIPE-B", "Recipe B"))
            .add_card(make_test_card("RECIPE-FILL", "Other"))
            .add_card(make_test_card("RECIPE-TOP", "Top"))
            .deck(0, &["RECIPE-FILL"; 20]).deck(1, &["RECIPE-FILL"; 20]).memory(3).start();
        r.skip_mulligan(); r.register_effect("RECIPE-TOP", Arc::new(RecipePartition));
        let mut stack = vec!["RECIPE-FILL"; depth + 1];
        stack[12] = "RECIPE-A"; stack[depth] = "RECIPE-A";
        stack.push("RECIPE-B"); stack.push("RECIPE-TOP");
        let host = r.place_stack(player, &stack);
        let expected = r.game.player(player).battle_area[0].card_sources[depth].handle();
        r.game.delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
        assert!(r.game.pending_selection.is_some(), "printed recipe at deep source positions must satisfy Partition gate");
        r.game.resolve_selection(player, REPLACEMENT_ACCEPT).unwrap();
        let pending = r.game.pending_selection.as_ref().expect("choose between both physical Recipe A sources and Recipe B");
        assert!(pending.card_choice.is_some());
        assert_eq!(build_action_mask(&r.game, player)[PASS as usize], 0.0);
        let mut clone = r.game.clone();
        for game in [&mut r.game, &mut clone] {
            game.resolve_selection(player, CARD_CHOICE_START + 1).unwrap();
            assert!(game.pending_selection.is_none());
            assert_eq!(game.player(player).battle_area.len(), 2);
            assert!(game.player(player).battle_area.iter().any(|p| p.top_card().handle() == expected));
            assert!(game.player(player).battle_area.iter().any(|p| p.top_card().card_id(&game.card_data) == "RECIPE-B"));
        }
    }
}

#[test]
fn keyword_fragment_lost_prefix_requires_replacement_picks_without_early_payment() {
    let mut r = DebugRunner::builder().add_card(make_test_card("FRAG-SOURCE", "Source"))
        .add_card(make_test_card("FRAG-TOP", "Top"))
        .deck(0, &["FRAG-SOURCE"; 20]).deck(1, &["FRAG-SOURCE"; 20]).memory(3).start();
    r.skip_mulligan(); r.register_effect("FRAG-TOP", Arc::new(FragmentTwo));
    let mut stack = vec!["FRAG-SOURCE"; 40]; stack.push("FRAG-TOP");
    let host = r.place_stack(0, &stack);
    r.game.delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    r.game.resolve_selection(0, REPLACEMENT_ACCEPT).unwrap();
    r.game.resolve_selection(0, CARD_CHOICE_NEXT).unwrap();
    r.game.resolve_selection(0, CARD_CHOICE_START + 11).unwrap();
    // A separate effect moved the first selected physical source before cost completion.
    let moved = r.game.players[0].battle_area[0].card_sources.remove(39);
    r.game.players[0].trash.push(moved);
    r.game.resolve_selection(0, CARD_CHOICE_START).unwrap();
    let state = r.game.pending_selection_resume.as_ref().and_then(find_count_frame)
        .expect("lost prefix must require another live original source instead of silently ending the choice");
    assert_eq!(state.accum.len(), 1);
    assert_eq!(r.game.player(0).trash.len(), 1, "incomplete payment must not trash accepted replacement picks");
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
    r.game.resolve_selection(0, CARD_CHOICE_START).unwrap();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.player(0).battle_area.len(), 1);
    assert_eq!(r.game.player(0).trash.len(), 3);
}

#[test]
fn keyword_partition_role_loss_exposes_only_finish_impossible_without_playing_sources() {
    use digimon_engine::{action::build_action_mask, owned_card_choice::CardChoiceOperation};
    for player in [0,1] {
        let mut r=DebugRunner::builder().add_card(make_test_card("ROLE-A","Recipe A"))
            .add_card(make_test_card("ROLE-B","Recipe B")).add_card(make_test_card("ROLE-TOP","Top"))
            .deck(0,&["ROLE-A";20]).deck(1,&["ROLE-A";20]).memory(3).start();
        r.skip_mulligan();r.game.turn_player_idx=player as usize;r.register_effect("ROLE-TOP",Arc::new(RecipePartition));
        let host=r.place_stack(player,&["ROLE-A","ROLE-A","ROLE-B","ROLE-B","ROLE-TOP"]);
        r.game.delete_permanent_with_cause(host,ReplacementCause::OpponentEffect);
        r.game.resolve_selection(player,REPLACEMENT_ACCEPT).unwrap();
        assert_eq!(build_action_mask(&r.game,player)[CARD_CHOICE_START as usize],1.0);
        // All four physical cards remain, but the two B cards lose their printed role.
        r.game.players[player as usize].battle_area[0].card_sources[2].face_down=true;
        r.game.players[player as usize].battle_area[0].card_sources[3].face_down=true;
        let mask=build_action_mask(&r.game,player);
        for action in CARD_CHOICE_START..CARD_CHOICE_START+4 {
            assert_eq!(mask[action as usize],0.0,"no physical count can substitute for a missing required printed role");
        }
        assert_eq!(mask[PASS as usize],1.0,"impossible required role has one explicit safe completion");
        let page=r.game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap().view(&r.game,player);
        assert_eq!(page.rows.len(),1);assert!(matches!(page.rows[0].operation,CardChoiceOperation::FinishImpossible));
        let mut clone=r.game.clone();
        for game in [&mut r.game,&mut clone] {
            let before=game.events.len();game.resolve_selection(player,PASS).unwrap();
            assert!(game.pending_selection.is_none());assert_eq!(game.player(player).battle_area.len(),0,"no partial Partition play succeeds");
            assert_eq!(game.memory,3);
            assert!(!game.events[before..].iter().any(|event|matches!(event,digimon_engine::events::GameEvent::Play{..})),"impossible-role recovery never pays successful play effects");
        }
    }
}
