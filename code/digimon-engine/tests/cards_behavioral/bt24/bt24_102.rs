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
use digimon_engine::live_game::LiveGame;
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

/// Regression: Homeros's `[All Turns] +1000 DP to your TS Digimon` filter
/// aura must install on the SAME `LiveGame::play()` call that lands
/// Homeros. Before the `live_game.rs` tick-discipline fix, the aura was
/// only materialized on the next action that went through
/// `decode_action` (e.g., a follow-up `pass_turn`). This test fails on
/// that pre-fix code and passes after `LiveGame::play()` calls
/// `tick_declarative_effects()` post-action.
#[test]
fn bt24_102_aura_installs_on_same_play_call() {
    // Synthetic TS-trait Lv3 Digimon that Homeros's aura should buff.
    // Built inline so the test owns the trait shape independently of any
    // particular printed card pool.
    let mut ts_probe = make_test_card("TS-LV3-PROBE", "TS Lv3 Probe");
    ts_probe.card_kind = CardKind::Digimon;
    ts_probe.level = Some(3);
    ts_probe.dp = Some(1000);
    ts_probe.traits = vec!["TS".to_string()];

    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-102 YAML parses")
        .add_card(ts_probe)
        .add_card(make_test_card("FILL", "filler"))
        .hand(0, &["BT24-102"])
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .memory(5)
        .start();

    let mut runner = runner;
    // Place the TS-trait probe on P0's battle area so Homeros's aura has
    // an in-zone target the instant Homeros itself enters play.
    let ts_handle = runner.place_on_field(0, "TS-LV3-PROBE", Some(0));

    // Sanity: no aura modifiers before Homeros lands. The aura is keyed
    // off Homeros's own presence, so the TS probe sees zero ChangeDp
    // entries while only fillers / itself are on the field.
    let mods_before = runner
        .game
        .modifiers
        .permanent_modifiers_iter(ts_handle)
        .filter(|m| {
            matches!(
                m.modifier,
                digimon_engine::enums::ModifierType::ChangeDp
            )
        })
        .count();
    assert_eq!(
        mods_before, 0,
        "no ChangeDp modifier expected before Homeros enters play"
    );

    // Move the configured Game into LiveGame and play Homeros via the
    // public wrapper that the MCP / CLI / PyO3 bindings all consume.
    let mut lg = LiveGame::from_game(runner.game);
    let result = lg.play(0, 0);
    assert!(
        result.ok,
        "Homeros play should succeed (memory 5, cost 5): {:?}",
        result.error
    );

    // Same-tick assertion: the aura modifier MUST be present without any
    // follow-up action forcing a tick. This is the contract the
    // `live-game-surface` spec delta added in
    // `fix-qa-bugs-aura-tick-reveal-picks`.
    let mods_view = lg.modifiers(ts_handle);
    let dp_mods: Vec<_> = mods_view
        .modifiers
        .iter()
        .filter(|m| m.modifier_type == "ChangeDp")
        .collect();
    assert_eq!(
        dp_mods.len(),
        1,
        "Homeros aura must install exactly one ChangeDp modifier on the TS Digimon \
         on the same LiveGame::play() call (no follow-up action). \
         Got modifiers: {:?}",
        mods_view.modifiers
    );
    assert_eq!(
        dp_mods[0].value, 1000,
        "Homeros aura is +1000 DP for TS Digimon"
    );
}

/// Regression: `LiveGame::move_from_breeding()` must also tick
/// declarative effects post-action so a moved breeding-area carrier
/// either gets aura-buffed or becomes the aura source on the same call.
#[test]
fn bt24_102_aura_installs_on_move_from_breeding() {
    // Same TS probe shape as the play() test.
    let mut ts_probe = make_test_card("TS-LV3-PROBE", "TS Lv3 Probe");
    ts_probe.card_kind = CardKind::Digimon;
    ts_probe.level = Some(3);
    ts_probe.dp = Some(1000);
    ts_probe.traits = vec!["TS".to_string()];

    // Breeding-area carrier carries the TS trait so it becomes Homeros's
    // aura target when it moves into the battle area.
    let mut bred_carrier = make_test_card("TS-BRED", "TS Bred Carrier");
    bred_carrier.card_kind = CardKind::Digimon;
    bred_carrier.level = Some(3);
    bred_carrier.dp = Some(1000);
    bred_carrier.traits = vec!["TS".to_string()];

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-102 YAML parses")
        .add_card(ts_probe)
        .add_card(bred_carrier)
        .add_card(make_test_card("FILL", "filler"))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .memory(5)
        .start();

    // Stage: place Homeros (aura source) on P0's field, place an
    // existing TS Digimon on field (already-targeted by the aura), and
    // place the to-be-moved carrier in P0's breeding area.
    let _homeros = runner.place_on_field(0, "BT24-102", Some(0));
    let _existing = runner.place_on_field(0, "TS-LV3-PROBE", Some(0));
    runner.place_in_breeding(0, "TS-BRED");

    // Force a tick so the existing target picks up its current aura
    // entry — this isolates the move_from_breeding tick from any earlier
    // staging side effects.
    runner.game.tick_declarative_effects();

    // Move the breeding carrier to the battle area through the LiveGame
    // wrapper.
    let mut lg = LiveGame::from_game(runner.game);
    let result = lg.move_from_breeding(0);
    assert!(
        result.ok,
        "move_from_breeding should succeed when breeding has a carrier and battle area has room: {:?}",
        result.error
    );

    // After the move, the just-moved TS carrier (a Homeros aura target)
    // must already carry the +1000 ChangeDp modifier — the
    // `tick_declarative_effects` call inside the LiveGame wrapper is the
    // only thing that installs it.
    let battle_area = &lg.field(0, digimon_engine::view::Perspective::God).battle_area;
    let moved = battle_area
        .iter()
        .find(|p| p.top_card_id == "TS-BRED")
        .map(|p| p.handle)
        .expect("moved carrier appears in battle area after move_from_breeding");
    let mods_view = lg.modifiers(moved);
    let dp_mods: Vec<_> = mods_view
        .modifiers
        .iter()
        .filter(|m| m.modifier_type == "ChangeDp")
        .collect();
    assert_eq!(
        dp_mods.len(),
        1,
        "Homeros aura must install ChangeDp on the moved carrier on the same \
         LiveGame::move_from_breeding() call. Got modifiers: {:?}",
        mods_view.modifiers
    );
    assert_eq!(dp_mods[0].value, 1000);
}
