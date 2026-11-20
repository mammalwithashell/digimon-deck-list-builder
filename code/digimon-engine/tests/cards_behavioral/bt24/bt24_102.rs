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

/// Accept Homeros's "By suspending this Tamer, you may ..." OptionalSkill
/// gate (DCGO BT24_102.cs:84 `SetUpActivateClass(..., true, ...)`).
fn accept_homeros_gate(runner: &mut DebugRunner) {
    let pending = runner
        .pending_selection()
        .expect("Homeros's optional activation gate");
    assert!(pending.is_optional, "the 'By suspending' gate is optional");
    assert_ne!(
        pending.kind,
        SelectionKind::OwnField,
        "the gate precedes the target pick (cost is paid before the pick)"
    );
    let accept = pending
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != digimon_engine::action::space::PASS)
        .expect("accept action");
    runner.execute_action(0, accept).expect("accept Homeros's gate");
}

/// §15-7-5 (digest.md: the "By X" cost may be paid even when the result
/// cannot happen) + DCGO BT24_102.cs:121-135: `CanActivateCondition` is only
/// "on the battle area and can pay the suspend cost" -- no eligible-target
/// check. With NO [Olympos XII] Digimon the gate is still offered, accepting
/// suspends Homeros, and no pick follows (SelectPermanentEffect with no
/// candidate opens nothing, SelectPermanentEffect.cs:181-199).
#[test]
fn bt24_102_gate_offered_without_olympos_target_and_suspends() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-102 YAML parses")
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();
    let homeros = runner.place_on_field(0, "BT24-102", Some(0));
    enqueue_homeros_eot(&mut runner, homeros);
    accept_homeros_gate(&mut runner);
    let _ = runner.auto_resolve();
    assert!(runner.game.players[0].battle_area[homeros.index as usize].is_suspended);
    assert!(runner.pending_selection().is_none(), "no target -> no pick");
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
    accept_homeros_gate(&mut runner);
    assert!(
        runner.game.players[0].battle_area[homeros.index as usize].is_suspended,
        "accepting the gate pays Homeros's suspend cost first (DCGO BT24_102.cs:137-138)"
    );

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

/// G-ENGINE-REFIRE-SPLITS-COMBINED-TIMING: a single printed
/// `[On Play] [When Digivolving]` effect (BT24-101's shape) is ONE effect, so
/// Homeros's "activate 1 [On Play] or [When Digivolving] effect" must not
/// offer it twice. With only that one effect on the target, no EffectChoice
/// surfaces and the effect resolves exactly once.
#[test]
fn bt24_102_refire_combined_on_play_when_digivolving_is_one_choice() {
    const TARGET_YAML: &str = r#"
card: TEST-OLY-COMBINED
name: Combined Olympos
kind: digimon
level: 6
color: [yellow]
cost: 10
dp: 12000
traits: [Olympos XII, TS]
effects:
  - when: [on_play, when_digivolving]
    summary: "[On Play] [When Digivolving] Gain 2 memory"
    process:
      - gain_memory: 2
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-102 YAML parses")
        .from_dsl_yaml(TARGET_YAML)
        .expect("target YAML parses")
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    let homeros = runner.place_on_field(0, "BT24-102", Some(0));
    let _target = runner.place_on_field(0, "TEST-OLY-COMBINED", Some(0));
    enqueue_homeros_eot(&mut runner, homeros);
    accept_homeros_gate(&mut runner);

    let pending = runner.pending_selection().expect("target selection");
    assert_eq!(pending.kind, SelectionKind::OwnField);
    let target_action = pending.valid_action_ids[0];
    runner
        .execute_action(0, target_action)
        .expect("choose Olympos XII target");

    if let Some(p) = runner.pending_selection() {
        assert_ne!(
            p.kind,
            SelectionKind::EffectChoice,
            "a combined [On Play][When Digivolving] effect is ONE effect, not two choices ({} offered)",
            p.valid_action_ids.len()
        );
    }
    assert_eq!(runner.memory(), 2, "the combined effect resolves exactly once");
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
        .filter(|m| matches!(m.modifier, digimon_engine::enums::ModifierType::ChangeDp))
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
    let battle_area = &lg
        .field(0, digimon_engine::view::Perspective::God)
        .battle_area;
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

// ---------------------------------------------------------------------------
// Homeros x every implemented [Olympos XII] Digimon with an [On Play] and/or
// [When Digivolving] effect (replay-hunt round 2, lane homeros-effect-pick).
//
// DCGO (BT24_102.cs:93-130, 173-250) builds the candidate list from
// `permanent.EffectList(OnEnterFieldAnyone).Filter(CanBeEffectCandidate)`: an
// effect object counts when its DESCRIPTION starts with "[On Play]" or
// "[When Digivolving]" (ICardEffect.IsOnPlay / IsWhenDigivolving). A printed
// `[On Play] [When Digivolving] X` effect is one object per tag
// (`CardEffectFactory.ActivateClassesForSharedEffects`, or two hand-written
// ActivateClass objects), so DCGO lists such a card TWICE and opens its
// "Select 1 effect to activate." prompt over the duplicates; every choice runs
// the same shared coroutine. The engine models the printed effect as ONE
// clause (`Effect::clause_group`), so it asks nothing and records that DCGO
// would have listed `dcgo_entries` of them (`Game::last_collapsed_refire`).
// ---------------------------------------------------------------------------

use digimon_engine::game::CollapsedRefire;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DcgoShape {
    /// DCGO: exactly one [On Play]/[When Digivolving] effect object. The
    /// engine runs it directly; nothing is collapsed and nothing is asked.
    Single,
    /// DCGO: ONE printed multi-timing effect built as `n` objects. The engine
    /// runs the one clause and records `dcgo_entries == n`.
    Collapsed(usize),
    /// DCGO: a collapsed combined effect PLUS a separate [When Digivolving]
    /// effect (Merukimon BT24-051). The engine keeps its own effect choice
    /// with `n` entries (one per distinct effect), where DCGO lists 3.
    OwnChoice(usize),
}

/// Every implemented [Olympos XII] Digimon with an [On Play] / [When
/// Digivolving] effect, with the shape DCGO gives it (read from DCGO's C#, see
/// the lane report's per-card table).
/// BT24-081 Titamon + SkullBaluchimon is NOT listed: its printed traits are
/// Shaman/Titan/TS (card image and the official DB); an `Olympos XII` entry in
/// `cards.json` / `card_overrides.json` was a data error and is removed.
const OLYMPOS_REFIRE_SHAPES: &[(&str, DcgoShape)] = &[
    ("BT10-042", DcgoShape::Single),          // Venusmon: [When Digivolving] only
    ("BT24-030", DcgoShape::Collapsed(2)),    // Neptunemon: two objects, shared coroutine
    ("BT24-040", DcgoShape::Collapsed(2)),    // Venusmon
    ("BT24-041", DcgoShape::Collapsed(2)),    // Minervamon (+ [On Deletion], not offered)
    ("BT24-051", DcgoShape::OwnChoice(2)),    // Merukimon: [OP][WD] + separate [WD][WA]
    ("BT24-101", DcgoShape::Collapsed(2)),    // Jupitermon
    ("BT25-018", DcgoShape::Collapsed(2)),    // Apollomon
    ("BT25-020", DcgoShape::Collapsed(2)),    // Marsmon ([WA] not offered)
    ("BT25-028", DcgoShape::Collapsed(2)),    // Dianamon ([All Turns] not offered)
    ("BT25-044", DcgoShape::Collapsed(2)),    // Junomon
    ("BT25-059", DcgoShape::Collapsed(2)),    // Ceresmon
    ("BT25-075", DcgoShape::Collapsed(2)),    // Vulcanusmon
    ("BT25-077", DcgoShape::Collapsed(2)),    // Bacchusmon
    ("BT25-103", DcgoShape::Single),          // GraceNovamon: [WD][WA]; only [WD] is a candidate
    ("BT26-032", DcgoShape::Single),          // Ceresmon (DUAL): [When Digivolving] only
    ("BT26-033", DcgoShape::Single),          // Jupitermon (DUAL)
    ("BT26-080", DcgoShape::Single),          // Bacchusmon (DUAL)
    ("BT26-081", DcgoShape::Collapsed(2)),    // Mervamon
    ("BT26-083", DcgoShape::Collapsed(2)),    // Junomon: Hysteric Mode ([On Deletion] not offered)
    ("BT26-103", DcgoShape::Single),          // Jupitermon: Wrath Mode: [WD][Counter]
    ("P-207", DcgoShape::Collapsed(2)),       // Minervamon
];

fn homeros_refire_runner(carrier_id: &str) -> (DebugRunner, CardHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-102")
        .expect("BT24-102 production YAML")
        .dsl_card(carrier_id)
        .unwrap_or_else(|e| panic!("{carrier_id} production YAML: {e}"))
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .security(0, &["FILL"; 5])
        .security(1, &["FILL"; 5])
        .memory(0)
        .start();
    let homeros = runner.place_on_field(0, "BT24-102", Some(0));
    let carrier = runner.place_on_field(0, carrier_id, Some(0));
    runner.place_on_field(1, "FILL", Some(0));
    let carrier_card = runner.top_card(carrier);
    enqueue_homeros_eot(&mut runner, homeros);
    accept_homeros_gate(&mut runner);
    (runner, carrier_card)
}

/// For every card: Homeros offers it, the engine's candidate count matches DCGO's
/// shape (an effect choice exactly when DCGO's list holds distinct effects), a
/// collapsed combined effect records how many entries DCGO would list, and the
/// refire runs the effect with the Digimon as carrier (no panic, no stuck prompt).
#[test]
fn bt24_102_refire_matches_dcgo_candidate_shape_for_every_olympos_xii_digimon() {
    for (card_id, shape) in OLYMPOS_REFIRE_SHAPES {
        let (mut runner, carrier_card) = homeros_refire_runner(card_id);
        let pending = runner
            .pending_selection()
            .unwrap_or_else(|| panic!("{card_id}: Homeros offers its [Olympos XII] pick"));
        assert_eq!(pending.kind, SelectionKind::OwnField, "{card_id}");
        assert_eq!(
            pending.valid_action_ids.len(),
            1,
            "{card_id}: only the card with a refireable effect is offered"
        );
        let pick = pending.valid_action_ids[0];
        runner
            .execute_action(0, pick)
            .unwrap_or_else(|e| panic!("{card_id}: choose the Olympos XII Digimon: {e:?}"));

        let own_choice = runner
            .pending_selection()
            .filter(|p| {
                p.kind == SelectionKind::EffectChoice
                    && p.prompt == "Choose an effect to activate"
                    && p.effect_choices
                        .iter()
                        .flatten()
                        .any(|e| e.source_card == Some(carrier_card))
            })
            .map(|p| p.valid_action_ids.len());
        let collapsed: Option<CollapsedRefire> = runner.game.last_collapsed_refire.clone();
        match *shape {
            DcgoShape::Single => {
                assert_eq!(own_choice, None, "{card_id}: one effect object, no choice");
                assert!(collapsed.is_none(), "{card_id}: nothing collapsed: {collapsed:?}");
            }
            DcgoShape::Collapsed(entries) => {
                assert_eq!(own_choice, None, "{card_id}: one printed effect is ONE effect, no choice");
                let c = collapsed.unwrap_or_else(|| panic!("{card_id}: the collapsed refire is recorded"));
                assert_eq!(c.dcgo_entries, entries, "{card_id}");
                assert_eq!(c.carrier_card, carrier_card, "{card_id}: the Digimon is the carrier");
                assert_eq!(c.carrier_card_id, *card_id);
                assert_eq!(c.controller, 0);
                assert_eq!(
                    runner.game.card_data_for_handle(c.grantor).map(|d| d.card_id.as_str()),
                    Some("BT24-102"),
                    "{card_id}: Homeros is the grantor"
                );
            }
            DcgoShape::OwnChoice(entries) => {
                assert_eq!(own_choice, Some(entries), "{card_id}: two distinct effects are a real choice");
                assert!(collapsed.is_none(), "{card_id}: a two-effect refire is not collapsed: {collapsed:?}");
            }
        }
        let _ = runner.auto_resolve();
    }
}

/// Eligibility (DCGO BT24_102.cs:93-98): only an [Olympos XII] Digimon whose
/// effect list holds an [On Play] / [When Digivolving] candidate is offered. A
/// Digimon with only [When Attacking] / [On Deletion] effects, one without the
/// trait, and a plain Digimon are not.
#[test]
fn bt24_102_pick_offers_only_olympos_xii_digimon_with_a_refireable_effect() {
    const ATTACK_ONLY: &str = r#"
card: TEST-OLY-ATTACK
name: Attack Only Olympos
kind: digimon
level: 6
color: [yellow]
cost: 10
dp: 12000
traits: [Olympos XII, TS]
effects:
  - when: when_attacking
    summary: "[When Attacking] Gain 2 memory"
    process:
      - gain_memory: 2
"#;
    const DELETION_ONLY: &str = r#"
card: TEST-OLY-DELETION
name: Deletion Only Olympos
kind: digimon
level: 6
color: [yellow]
cost: 10
dp: 12000
traits: [Olympos XII, TS]
effects:
  - when: on_deletion
    summary: "[On Deletion] Gain 2 memory"
    process:
      - gain_memory: 2
"#;
    const NOT_OLYMPOS: &str = r#"
card: TEST-NOT-OLYMPOS
name: Not Olympos
kind: digimon
level: 6
color: [yellow]
cost: 10
dp: 12000
traits: [TS]
effects:
  - when: on_play
    summary: "[On Play] Gain 2 memory"
    process:
      - gain_memory: 2
"#;
    let build = || {
        DebugRunner::builder()
            .from_dsl_yaml(YAML)
            .expect("BT24-102 YAML parses")
            .dsl_card("BT24-041")
            .expect("BT24-041 production YAML")
            .from_dsl_yaml(ATTACK_ONLY)
            .expect("attack-only YAML")
            .from_dsl_yaml(DELETION_ONLY)
            .expect("deletion-only YAML")
            .from_dsl_yaml(NOT_OLYMPOS)
            .expect("not-olympos YAML")
            .add_card(make_filler("FILL"))
            .deck(0, &["FILL"; 10])
            .deck(1, &["FILL"; 10])
            .memory(0)
            .start()
    };

    // One eligible Digimon among four: it alone is offered.
    let mut runner = build();
    let homeros = runner.place_on_field(0, "BT24-102", Some(0));
    runner.place_on_field(0, "TEST-OLY-ATTACK", Some(0));
    runner.place_on_field(0, "TEST-OLY-DELETION", Some(0));
    runner.place_on_field(0, "TEST-NOT-OLYMPOS", Some(0));
    let minervamon = runner.place_on_field(0, "BT24-041", Some(0));
    enqueue_homeros_eot(&mut runner, homeros);
    accept_homeros_gate(&mut runner);
    let pending = runner.pending_selection().expect("the [Olympos XII] pick");
    assert_eq!(pending.kind, SelectionKind::OwnField);
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "only Minervamon (BT24-041), whose [On Play] [When Digivolving] effect Homeros can activate, is offered"
    );
    assert!(pending.is_optional, "the pick stays declinable");
    let _ = minervamon;

    // None eligible: the pick opens nothing (DCGO: maxCount = Min(1, 0) opens no
    // prompt) and the cost was still paid.
    let mut runner = build();
    let homeros = runner.place_on_field(0, "BT24-102", Some(0));
    runner.place_on_field(0, "TEST-OLY-ATTACK", Some(0));
    runner.place_on_field(0, "TEST-OLY-DELETION", Some(0));
    enqueue_homeros_eot(&mut runner, homeros);
    accept_homeros_gate(&mut runner);
    assert!(
        runner.pending_selection().is_none(),
        "no [Olympos XII] Digimon has an [On Play]/[When Digivolving] effect: no pick opens"
    );
    assert!(
        runner.game.players[0].battle_area[homeros.index as usize].is_suspended,
        "the suspend cost is paid even though nothing can be activated (CR 4.3 15-7-5)"
    );
}

/// BT24-041 Minervamon end to end under Homeros's refire: the ONE printed
/// effect runs once with Minervamon as the carrier and Homeros as the source;
/// its optional "may play" pick stays optional and plays the Iliad card.
#[test]
fn bt24_102_refires_minervamons_single_effect_with_homeros_as_source() {
    let mut iliad = make_test_card("ILIAD-3", "Iliad Rookie");
    iliad.card_kind = CardKind::Digimon;
    iliad.level = Some(3);
    iliad.dp = Some(3000);
    iliad.play_cost = 3;
    iliad.traits = vec!["Iliad".to_string()];
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-102")
        .expect("BT24-102 production YAML")
        .dsl_card("BT24-041")
        .expect("BT24-041 production YAML")
        .add_card(iliad)
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .memory(0)
        .start();
    let homeros = runner.place_on_field(0, "BT24-102", Some(0));
    let minervamon = runner.place_on_field(0, "BT24-041", Some(0));
    runner.add_to_hand(0, "ILIAD-3");
    enqueue_homeros_eot(&mut runner, homeros);
    accept_homeros_gate(&mut runner);
    let pick = runner.pending_selection().expect("Olympos pick").valid_action_ids[0];
    runner.execute_action(0, pick).expect("choose Minervamon");

    let record = runner.game.last_collapsed_refire.clone().expect("one collapsed refire is recorded");
    assert_eq!(record.dcgo_entries, 2, "[On Play] and [When Digivolving]: two DCGO objects");
    assert_eq!(record.carrier, minervamon);
    let pending = runner.pending_selection().expect("Minervamon's own optional hand pick");
    assert_eq!(pending.kind, SelectionKind::Hand, "no effect choice precedes it");
    assert!(pending.is_optional, "'You may play 1 ...' stays optional");
    let play = pending
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != digimon_engine::action::space::PASS)
        .expect("the Iliad card can be played");
    runner.execute_action(0, play).expect("play the Iliad card");
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "ILIAD-3"),
        "the refired effect played the hand card"
    );
}
