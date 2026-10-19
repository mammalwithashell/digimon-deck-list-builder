use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::card_data::{CardData, DnaCost, DnaRequirement};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, CostDelta};
use digimon_engine::permanent::PermanentHandle;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
enum Change {
    DnaIncluded,
    DnaExcluded,
    Evolve,
    Devolve,
    Replay,
}
struct Rider(Change);
fn find(game: &digimon_engine::game::Game, id: &str) -> PermanentHandle {
    PermanentHandle {
        player: 0,
        index: game.players[0]
            .battle_area
            .iter()
            .position(|p| p.top_card().card_id(&game.card_data) == id)
            .unwrap() as u8,
    }
}
impl CardEffect for Rider {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let change = self.0;
        vec![Effect::when_would_be_deleted(card)
            .name("Change pending subject")
            .optional()
            .replacement_condition(|ctx, subject| subject.permanent() == ctx.source_permanent)
            .replacement_process(move |rctx| {
                let target = find(rctx.effect.game, "TARGET");
                match change {
                    Change::DnaIncluded | Change::DnaExcluded => {
                        let first = if matches!(change, Change::DnaIncluded) {
                            target
                        } else {
                            find(rctx.effect.game, "OTHER")
                        };
                        let second = find(rctx.effect.game, "PARTNER");
                        let result = rctx.effect.game.players[0].hand[0].handle();
                        assert!(rctx
                            .effect
                            .effect_initiated_dna_digivolve(first, second, result, 0, false)
                            .is_some());
                    }
                    Change::Evolve => {
                        assert!(rctx.effect.effect_initiated_digivolve_ignore_requirements(
                            0,
                            0,
                            target,
                            CostDelta::Free
                        ));
                    }
                    Change::Devolve => {
                        assert_eq!(rctx.effect.de_digivolve(target, None, Some(1)), 1);
                    }
                    Change::Replay => {
                        let source = rctx.effect.game.players[0].battle_area[target.index as usize]
                            .top_card()
                            .handle();
                        assert!(rctx.effect.game.return_to_hand(target).is_some());
                        assert!(rctx
                            .effect
                            .game
                            .play_card_from_effect_without_cost(0, source)
                            .is_some());
                    }
                }
            })
            .build()]
    }
}
fn card(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(5);
    c.dp = Some(6000);
    c
}
fn check(change: Change) {
    let mut result = card("RESULT", CardColor::Purple);
    result.level = Some(6);
    let req = |color| DnaRequirement {
        level: 5,
        card_colors: vec![color],
        name_contains: String::new(),
        text_contains: String::new(),
    };
    result.dna_costs = vec![DnaCost {
        requirement1: req(CardColor::Purple),
        requirement2: req(CardColor::Red),
        memory_cost: 0,
    }];
    let mut runner = DebugRunner::builder()
        .add_card(card("TARGET", CardColor::Purple))
        .add_card(card("OTHER", CardColor::Purple))
        .add_card(card("PARTNER", CardColor::Red))
        .add_card(card("BYSTANDER", CardColor::Blue))
        .add_card(result)
        .hand(0, &["RESULT"])
        .deck(0, &["BYSTANDER"; 4])
        .deck(1, &["BYSTANDER"; 4])
        .memory(10)
        .start();
    runner.register_effect("TARGET", Arc::new(Rider(change)));
    // Nontrivial compaction: the partner precedes the threatened subject.
    runner.place_on_field(0, "PARTNER", None);
    runner.place_on_field(0, "TARGET", None);
    runner.place_on_field(0, "OTHER", None);
    runner.place_on_field(0, "BYSTANDER", None);
    let target = find(&runner.game, "TARGET");
    if matches!(change, Change::Devolve) {
        let lower = runner.game.players[0]
            .battle_area
            .remove(2)
            .card_sources
            .pop()
            .unwrap();
        runner.game.players[0].battle_area[target.index as usize]
            .card_sources
            .insert(0, lower);
    }
    runner.game.delete_permanent_with_effects(target);
    let mut cloned = runner.game.clone();
    for game in [&mut runner.game, &mut cloned] {
        game.resolve_selection(0, REPLACEMENT_ACCEPT).unwrap();
        assert!(game.pending_selection.is_none(), "{change:?}");
        let tops: Vec<_> = game.players[0]
            .battle_area
            .iter()
            .map(|p| p.top_card().card_id(&game.card_data))
            .collect();
        match change {
            Change::DnaIncluded | Change::DnaExcluded => assert!(
                tops.contains(&"RESULT"),
                "new DNA Digimon must survive {change:?}: {tops:?}"
            ),
            Change::Evolve => assert!(
                !tops.contains(&"RESULT"),
                "ordinary evolution retains original subject"
            ),
            Change::Devolve => assert!(
                !tops.contains(&"OTHER"),
                "de-digivolution retains original subject"
            ),
            Change::Replay => assert!(
                tops.contains(&"TARGET"),
                "replayed physical card is a new permanent"
            ),
        }
        assert!(!game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&game.card_data) == "BYSTANDER"));
        if matches!(change, Change::DnaExcluded) {
            assert!(game.players[0]
                .trash
                .iter()
                .any(|c| c.card_id(&game.card_data) == "TARGET"));
        }
        // An abandoned old event must not contaminate a later independent one.
        let bystander = find(game, "BYSTANDER");
        game.delete_permanent_with_effects(bystander);
        assert!(game.pending_selection.is_none());
        assert!(game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&game.card_data) == "BYSTANDER"));
    }
}
#[test]
fn pending_deletion_does_not_follow_consumed_dna_material() {
    check(Change::DnaIncluded);
}
#[test]
fn pending_deletion_excluded_from_dna_commits_after_compaction() {
    check(Change::DnaExcluded);
}
#[test]
fn pending_deletion_follows_ordinary_evolution() {
    check(Change::Evolve);
}
#[test]
fn pending_deletion_does_not_follow_replayed_physical_card() {
    check(Change::Replay);
}

#[test]
fn pending_deletion_follows_de_digivolved_stack() {
    check(Change::Devolve);
}
