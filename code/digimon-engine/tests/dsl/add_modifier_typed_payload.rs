//! G-DSL-ADD-MODIFIER-NAME-COLOR-PAYLOAD — typed `add_modifier` payloads.
//!
//! `add_modifier` used to accept a structured payload only for
//! `TreatAsDigimon` (`synth_identity:`), so "change the base name / color / DP"
//! (EX13-031 KingSukamon) and trait grants could not be installed on a bound
//! target. The general shape is
//!
//! ```yaml
//! - add_modifier: { target: self, modifier: ChangeBaseCardName,  value: 0, payload: { name: Sukamon },   expiry: end_of_opponents_turn }
//! - add_modifier: { target: self, modifier: ChangeBaseCardColor, value: 0, payload: { colors: [white] }, expiry: end_of_opponents_turn }
//! - add_modifier: { target: self, modifier: ChangeOriginDP,      value: 0, payload: { dp: 3000 },        expiry: end_of_opponents_turn }
//! ```
//!
//! lowering to `ModifierPayload::{Name, Colors, Dp, Traits}`, read by
//! `Permanent::synth_identity` (rules name / colors / base DP) and so by
//! `Game::effective_dp` and every predicate overlay.

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledColor, CompiledModifierPayload, CompiledModifierTarget,
    CompiledModifierValue, CompiledStep,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_step;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, Expiry, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;

fn step(modifier: &str, payload: CompiledModifierPayload, expiry: &str) -> CompiledStep {
    CompiledStep::AddModifier {
        target: CompiledModifierTarget::Binding(CompiledBindingRef::Named("tgt".into())),
        modifier: modifier.into(),
        value: CompiledModifierValue::Literal(0),
        expiry: expiry.into(),
        synth_identity: None,
        payload: Some(payload),
        continuous: false,
    }
}

/// Player 0 owns the effect source; player 1 owns a red 6000 DP "Victim".
fn setup() -> (DebugRunner, PermanentHandle) {
    let mut src = make_test_card("SRC", "Source");
    src.colors = vec![CardColor::Yellow];
    let mut victim = make_test_card("VIC", "Victimmon");
    victim.colors = vec![CardColor::Red];
    victim.dp = Some(6000);
    victim.traits = vec!["Dragon".into()];
    let mut runner = DebugRunner::builder()
        .add_card(src)
        .add_card(victim)
        .build();
    runner.place_on_field(0, "SRC", None);
    let vic = runner.place_on_field(1, "VIC", None);
    (runner, vic)
}

fn run(runner: &mut DebugRunner, steps: &[CompiledStep], target: PermanentHandle) {
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", target);
    let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
    for s in steps {
        run_step(s, &mut ctx, &mut bindings);
    }
}

fn identity(
    runner: &DebugRunner,
    h: PermanentHandle,
) -> digimon_engine::permanent::SynthIdentity {
    runner.game.players[h.player as usize].battle_area[h.index as usize].synth_identity(
        &runner.game.card_data,
        &runner.game.modifiers,
        h,
    )
}

fn kingsukamon_steps() -> Vec<CompiledStep> {
    vec![
        step(
            "ChangeBaseCardName",
            CompiledModifierPayload::Name("Sukamon".into()),
            "end_of_opponents_turn",
        ),
        step(
            "ChangeBaseCardColor",
            CompiledModifierPayload::Colors(vec![CompiledColor::White]),
            "end_of_opponents_turn",
        ),
        step(
            "ChangeOriginDP",
            CompiledModifierPayload::Dp(3000),
            "end_of_opponents_turn",
        ),
    ]
}

#[test]
fn name_color_and_base_dp_payloads_replace_the_rules_identity() {
    let (mut runner, vic) = setup();
    run(&mut runner, &kingsukamon_steps(), vic);
    let id = identity(&runner, vic);
    assert_eq!(id.card_name, "Sukamon");
    assert_eq!(id.card_names, vec!["Sukamon".to_string()], "the old name is gone");
    assert_eq!(id.colors, vec![CardColor::White], "red is replaced, not added to");
    assert_eq!(id.dp, Some(3000));
    assert_eq!(runner.effective_dp(vic), Some(3000));
}

/// "Base DP" (DCGO `ChangeBaseDPClass`): the base becomes 3000 and the +/- DP
/// effects still apply on top of it.
#[test]
fn base_dp_payload_is_a_base_other_dp_modifiers_stack_on_top() {
    let (mut runner, vic) = setup();
    runner.game.modifiers.add(
        vic,
        ModifierEntry::simple(ModifierType::ChangeDp, -1000, Expiry::EndOfTurn, 1),
    );
    assert_eq!(runner.effective_dp(vic), Some(5000));
    run(&mut runner, &kingsukamon_steps(), vic);
    assert_eq!(runner.effective_dp(vic), Some(2000), "3000 base − 1000");
}

/// `end_of_opponents_turn` from player 0's effect: survives player 0's own turn
/// end, expires at the end of player 1's turn ("until their turn ends").
#[test]
fn typed_payload_modifiers_honor_their_expiry() {
    let (mut runner, vic) = setup();
    run(&mut runner, &kingsukamon_steps(), vic);
    runner.game.modifiers.expire_end_of_turn(0);
    assert_eq!(identity(&runner, vic).card_name, "Sukamon", "still on through your turn end");
    runner.game.modifiers.expire_end_of_turn(1);
    let id = identity(&runner, vic);
    assert_eq!(id.card_name, "Victimmon");
    assert_eq!(id.colors, vec![CardColor::Red]);
    assert_eq!(runner.effective_dp(vic), Some(6000));
}

#[test]
fn traits_payload_adds_or_replaces_traits() {
    let (mut runner, vic) = setup();
    run(
        &mut runner,
        &[step(
            "ChangeTraits",
            CompiledModifierPayload::Traits {
                add: vec!["Holy".into()],
                replace: false,
            },
            "end_of_turn",
        )],
        vic,
    );
    assert_eq!(identity(&runner, vic).traits, vec!["Dragon".to_string(), "Holy".into()]);
    run(
        &mut runner,
        &[step(
            "ChangeTraits",
            CompiledModifierPayload::Traits {
                add: vec!["Mutant".into()],
                replace: true,
            },
            "end_of_turn",
        )],
        vic,
    );
    assert_eq!(identity(&runner, vic).traits, vec!["Mutant".to_string()]);
}

/// The YAML surface compiles to the typed payload (parse → validate → compile).
#[test]
fn yaml_payload_compiles_to_typed_compiled_payload() {
    let yaml = r#"
card: X-1
name: Test
kind: digimon
level: 5
color: [yellow]
cost: 7
dp: 7000
effects:
  - when: on_play
    process:
      - add_modifier: { target: self, modifier: ChangeBaseCardName, value: 0, payload: { name: Sukamon }, expiry: end_of_opponents_turn }
      - add_modifier: { target: self, modifier: ChangeBaseCardColor, value: 0, payload: { colors: [white] }, expiry: end_of_opponents_turn }
      - add_modifier: { target: self, modifier: ChangeOriginDP, value: 0, payload: { dp: 3000 }, expiry: end_of_opponents_turn }
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse");
    let card = digimon_dsl::compile::compile(&spec).expect("compile");
    let digimon_dsl::compiled::CompiledClause::Triggered(t) = &card.effects[0] else {
        panic!("triggered clause");
    };
    let payloads: Vec<_> = t
        .process
        .iter()
        .filter_map(|s| match s {
            CompiledStep::AddModifier { payload, .. } => payload.clone(),
            _ => None,
        })
        .collect();
    assert_eq!(
        payloads,
        vec![
            CompiledModifierPayload::Name("Sukamon".into()),
            CompiledModifierPayload::Colors(vec![CompiledColor::White]),
            CompiledModifierPayload::Dp(3000),
        ]
    );
}
