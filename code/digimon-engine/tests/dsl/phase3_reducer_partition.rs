use std::sync::{Arc, Mutex};

use digimon_dsl::compiled::{CompiledPredicate, CompiledScope};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::lower_partition;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::Keyword;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

fn run_partition_process(
    runner: &mut DebugRunner,
    permanent: PermanentHandle,
    active_when: Option<&CompiledPredicate>,
    sources: &[CompiledPredicate],
    exclude_cause: &[String],
) {
    let source_card = runner.game.players[permanent.player as usize].battle_area
        [permanent.index as usize]
        .top_card()
        .handle();
    let effect = lower_partition::lower(
        source_card,
        CompiledScope::FaceUp,
        active_when,
        sources,
        exclude_cause,
    );
    let process = effect.process.as_ref().expect("partition process");
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(permanent), 0);
    process(&mut ctx);
}

fn runner_with_partition_host() -> (DebugRunner, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TEST-PARTITION", "Partition Host"))
        .start();
    let permanent = runner.place_on_field(0, "TEST-PARTITION", Some(0));
    (runner, permanent)
}

fn add_source_under_top(runner: &mut DebugRunner, permanent: PermanentHandle, card_id: &str) {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|data| data.card_id == card_id)
        .expect("source card data");
    let card_index = runner.game.next_card_index();
    let source = CardSource::new(data_index, permanent.player, card_index);
    let stack = &mut runner.game.players[permanent.player as usize].battle_area
        [permanent.index as usize]
        .card_sources;
    let top = stack.pop().expect("partition host has a top card");
    stack.push(source);
    stack.push(top);
}

#[test]
fn partition_active_when_false_does_not_grant_keyword() {
    let never = CompiledPredicate {
        your_turn: Some(false),
        opponents_turn: Some(false),
        ..Default::default()
    };
    let (mut runner, permanent) = runner_with_partition_host();

    run_partition_process(&mut runner, permanent, Some(&never), &[], &[]);

    assert!(!runner.game.has_keyword(permanent, Keyword::Partition));
}

#[test]
fn partition_without_gates_still_grants_keyword() {
    let (mut runner, permanent) = runner_with_partition_host();

    run_partition_process(&mut runner, permanent, None, &[], &[]);

    assert!(runner.game.has_keyword(permanent, Keyword::Partition));
}

#[test]
fn partition_source_predicate_requires_matching_material() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TEST-PARTITION", "Partition Host"))
        .add_card(make_test_card("TEST-SOURCE", "WarGreymon"))
        .start();
    let permanent = runner.place_on_field(0, "TEST-PARTITION", Some(0));
    let source_filter = CompiledPredicate {
        name_is: Some("WarGreymon".to_string()),
        ..Default::default()
    };

    run_partition_process(
        &mut runner,
        permanent,
        None,
        std::slice::from_ref(&source_filter),
        &[],
    );
    assert!(!runner.game.has_keyword(permanent, Keyword::Partition));

    add_source_under_top(&mut runner, permanent, "TEST-SOURCE");
    run_partition_process(&mut runner, permanent, None, &[source_filter], &[]);
    assert!(runner.game.has_keyword(permanent, Keyword::Partition));
}

struct PartitionOnDeletion {
    excluded_causes: Vec<String>,
    observed_keyword: Arc<Mutex<Option<bool>>>,
}

impl CardEffect for PartitionOnDeletion {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let excluded_causes = self.excluded_causes.clone();
        let observed_keyword = self.observed_keyword.clone();
        vec![Effect::on_deletion(card)
            .name("Partition gate probe")
            .process(move |ctx| {
                let effect = lower_partition::lower(
                    card,
                    CompiledScope::FaceUp,
                    None,
                    &[],
                    &excluded_causes,
                );
                if let Some(process) = effect.process.as_ref() {
                    process(ctx);
                }
                let has_keyword = ctx
                    .source_permanent
                    .map(|handle| ctx.game.has_keyword(handle, Keyword::Partition))
                    .unwrap_or(false);
                *observed_keyword.lock().unwrap() = Some(has_keyword);
            })
            .build()]
    }
}

#[test]
fn partition_exclude_cause_battle_suppresses_grant_during_deletion() {
    let observed_keyword = Arc::new(Mutex::new(None));
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TEST-PARTITION", "Partition Host"))
        .start();
    runner.register_effect(
        "TEST-PARTITION",
        Arc::new(PartitionOnDeletion {
            excluded_causes: vec!["battle".to_string()],
            observed_keyword: observed_keyword.clone(),
        }),
    );
    let permanent = runner.place_on_field(0, "TEST-PARTITION", Some(0));

    runner
        .game
        .delete_permanent_with_cause(permanent, ReplacementCause::Battle);

    assert_eq!(*observed_keyword.lock().unwrap(), Some(false));
}
