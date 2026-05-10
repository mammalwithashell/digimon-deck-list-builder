//! BT24-102 Homeros — Track K cross-card effect refiring fixture.
//!
//! The key printed clause is:
//!   "[End of Your Turn] By suspending this Tamer, you may activate 1 [On Play]
//!    or [When Digivolving] effect of 1 of your [Olympos XII] trait Digimon."
//!
//! This fixture pins the YAML path. The lowerer must surface the Olympos XII
//! target pick and, when that target has both eligible timings, the effect
//! choice pick. The chosen effect then runs with carrier = target Digimon and
//! source attribution = Homeros.

use std::sync::{Arc, Mutex};

use digimon_dsl::compiled::{CompiledClause, CompiledColor, CompiledStep, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};
use digimon_engine::{CardEffect, CardHandle, Effect};

const YAML: &str = include_str!("../../../cards/bt24/BT24-102.yaml");

#[derive(Clone)]
struct OlymposRefireTarget {
    seen: Arc<Mutex<Vec<(CardHandle, Option<PermanentHandle>)>>>,
}

impl CardEffect for OlymposRefireTarget {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let on_play_seen = Arc::clone(&self.seen);
        let when_digivolving_seen = Arc::clone(&self.seen);
        vec![
            Effect::on_play(card)
                .name("homeros target on play")
                .process(move |ctx| {
                    ctx.gain_memory(2);
                    on_play_seen
                        .lock()
                        .unwrap()
                        .push((ctx.source_card, ctx.source_permanent));
                })
                .build(),
            Effect::when_digivolving(card)
                .name("homeros target when digivolving")
                .process(move |ctx| {
                    ctx.gain_memory(5);
                    when_digivolving_seen
                        .lock()
                        .unwrap()
                        .push((ctx.source_card, ctx.source_permanent));
                })
                .build(),
        ]
    }
}

fn make_olympos_target() -> CardData {
    let mut card = make_test_card("OLYMPOS-TARGET", "Olympos Target");
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(7000);
    card.traits = vec!["Olympos XII".to_string(), "TS".to_string()];
    card
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn enqueue_homeros_eot(runner: &mut DebugRunner, homeros: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::Permanent(homeros),
    );
    runner.game.drain_effect_queue();
}

#[test]
fn bt24_102_yaml_compiles_with_refire_clause() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-102 YAML parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-102")
        .expect("BT24-102 compiled card present");

    assert_eq!(compiled.card, "BT24-102");
    assert_eq!(compiled.name, "Homeros");
    assert_eq!(
        compiled.kind,
        digimon_dsl::compiled::CompiledCardKind::Tamer
    );
    assert_eq!(compiled.cost, Some(5));
    assert_eq!(compiled.color, vec![CompiledColor::White]);
    assert!(compiled.traits.iter().any(|t| t == "Iliad"));
    assert!(compiled.traits.iter().any(|t| t == "TS"));

    let eot = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::EndOfYourTurn) => {
                Some(t)
            }
            _ => None,
        })
        .expect("EndOfYourTurn refire clause exists");
    assert!(eot.optional, "printed 'may activate' must be optional");
    assert!(
        eot.process.iter().any(|step| matches!(
            step,
            CompiledStep::RefireEffect { timing, .. }
                if timing == "on_play_or_when_digivolving"
        )),
        "EndOfYourTurn process must lower the Track K timing filter"
    );
}

#[test]
fn bt24_102_end_of_turn_refire_surfaces_target_then_effect_choice() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-102 YAML parses")
        .add_card(make_olympos_target())
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();
    runner.register_effect(
        "OLYMPOS-TARGET",
        Arc::new(OlymposRefireTarget {
            seen: Arc::clone(&seen),
        }),
    );

    let homeros = runner.place_on_field(0, "BT24-102", Some(0));
    let target = runner.place_on_field(0, "OLYMPOS-TARGET", Some(0));
    let homeros_card = runner.top_card(homeros);

    enqueue_homeros_eot(&mut runner, homeros);

    let pending = runner
        .pending_selection()
        .expect("Olympos XII target selection");
    assert_eq!(pending.kind, SelectionKind::OwnField);
    assert!(
        pending.is_optional,
        "Homeros's printed 'may' must be declinable at the target-choice prompt"
    );
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "only the Olympos XII Digimon should be targetable"
    );
    let target_action = pending.valid_action_ids[0];
    runner
        .execute_action(0, target_action)
        .expect("choose Olympos XII target");

    assert!(
        runner.game.players[0].battle_area[homeros.index as usize].is_suspended,
        "choosing a refire target pays Homeros's suspend cost"
    );

    let pending = runner
        .pending_selection()
        .expect("refired effect choice selection");
    assert_eq!(pending.kind, SelectionKind::EffectChoice);
    assert_eq!(
        pending.valid_action_ids.len(),
        2,
        "target has both On Play and When Digivolving effects"
    );
    let on_play_action = pending
        .effect_choices
        .as_ref()
        .expect("effect-choice metadata")
        .iter()
        .find(|choice| choice.timing == Some(EffectTiming::OnPlay))
        .map(|choice| choice.action_id)
        .expect("On Play choice is visible");
    runner
        .execute_action(0, on_play_action)
        .expect("choose target On Play effect");

    assert_eq!(runner.memory(), 2, "chosen On Play effect must resolve");
    assert_eq!(
        *seen.lock().unwrap(),
        vec![(homeros_card, Some(target))],
        "refired effect must use Homeros as source and selected Digimon as carrier"
    );
    assert!(
        runner.pending_selection().is_none(),
        "all Homeros refire choices should be resolved"
    );
}
