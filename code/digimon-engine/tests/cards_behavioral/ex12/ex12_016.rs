use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};
use digimon_engine::selection::SelectionKind;

use super::support::{
    field_contains, hand_index, plain_digimon, select_first_non_pass, DebugRunner,
};

const CARD_ID: &str = "EX12-016";

#[test]
fn ex12_016_has_security_attack_plus_and_decode_on_face_and_inherited() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-016 YAML loads")
        .start();
    let metalgreymon = runner.place_on_field(0, CARD_ID, Some(0));
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-016");

    assert!(runner
        .game
        .has_keyword(metalgreymon, Keyword::SecurityAttackPlus(1)));

    let decode_replacements = card
        .effects
        .iter()
        .filter(|clause| {
            matches!(
                clause,
                CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { .. })
            )
        })
        .count();
    assert_eq!(
        decode_replacements, 2,
        "MetalGreymon has Decode on its face and inherited text"
    );
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
            scope: CompiledScope::Inherited,
            ..
        })
    )));
}

#[test]
fn ex12_016_on_play_deletes_6000_dp_or_less_and_grants_forced_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-016 YAML loads")
        .add_card(plain_digimon("OPP-HIGH", CardColor::Red, 4, 7000))
        .add_card(plain_digimon("OPP-LOW", CardColor::Red, 4, 6000))
        .add_card(plain_digimon("SEC", CardColor::Red, 3, 2000))
        .add_card(plain_digimon("FILL-016", CardColor::Red, 3, 2000))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL-016", "FILL-016", "FILL-016"])
        .deck(1, &["FILL-016", "FILL-016", "FILL-016"])
        .security(0, &["SEC"])
        .memory(10)
        .start();
    let high = runner.place_on_field(1, "OPP-HIGH", None);
    runner.place_on_field(1, "OPP-LOW", None);

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-016");

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "first prompt chooses the <=6000 DP delete target"
    );
    select_first_non_pass(&mut runner);
    assert!(!field_contains(&runner, 1, "OPP-LOW"));
    assert!(field_contains(&runner, 1, "OPP-HIGH"));

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "then choose one of their Digimon to gain the forced attack"
    );
    select_first_non_pass(&mut runner);
    assert_eq!(
        runner
            .game
            .modifiers
            .granted_triggered_for_timing(high, EffectTiming::StartOfYourMainPhase)
            .len(),
        1,
        "selected opponent Digimon should carry the Start-of-Main forced attack"
    );

    runner.end_turn();
    let prompt = runner
        .pending_selection()
        .expect("forced attack should choose a target at opponent main phase");
    assert_eq!(prompt.kind, SelectionKind::Target);
    assert_eq!(prompt.selecting_player, 1);
    assert!(!prompt.is_optional);
    assert!(prompt
        .valid_action_ids
        .contains(&encode_attack(high.index as u16, SECURITY_TARGET)));
}

#[test]
fn ex12_016_shared_on_play_when_digivolving_clause_grants_forced_attack_body() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-016 YAML loads")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-016");

    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnPlay)
                    && triggered.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("shared OP/WD clause exists");

    assert!(clause.process.iter().any(|step| matches!(
        step,
        CompiledStep::GrantTriggeredEffect { body, .. }
            if body.iter().any(|nested| matches!(nested, CompiledStep::ForceAttack { .. }))
    )));
}

// ── <Assembly> on an effect play (CR 4.3 7-3-2-10) ──────────────────────
//
// MetalGreymon (EX12-016) prints `<Assembly -2: 1 Lv.4 or lower [Agumon]/
// [Greymon] name or [ME]/[VB] trait card>`. CR 4.3 7-3-2-10: "Assembly can be
// performed even when an effect is used to play a Digimon card with Assembly
// requirements." DCGO asks it on every non-digivolve play, effect plays
// included: `PlayCardClass.PlayCard` runs `SelectAssemblyClass.Select` for a
// single played card with `HasAssembly && !isEvolution` (CardController.cs:
// 926-933); the pick is a trash `SelectCardEffect` with `canNoSelect: true`
// and no separate yes/no gate (SelectAssemblyClass.cs:166-201, 215-245); the
// picks go to the bottom of the new Digimon's stack before its [On Play]
// (SelectAssemblyClass.cs:283-302, CardController.cs:1868-1869).
//
// Real game: raw-20261113T025510Z#g4 file step 127 — MetalGarurumon (EX12-035)
// was being deleted by Amaterasumon (EX12-047); its <Decode> played MetalGreymon
// (EX12-016) from its digivolution cards, and DCGO then asked P1 to pick
// Gammamon (EX12-007) from the trash for MetalGreymon's Assembly.

/// MetalGarurumon whose only digivolution card is MetalGreymon, with one
/// Assembly-eligible card (Lv.3 [VB]) in its owner's trash.
fn decode_into_metalgreymon_runner() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-016 YAML loads")
        .dsl_card("EX12-035")
        .expect("EX12-035 YAML loads")
        .add_card(super::support::vb_digimon("VB-ASM", CardColor::Blue, 3, 2000))
        .add_card(plain_digimon("FILL-016A", CardColor::Red, 3, 2000))
        .deck(0, &["FILL-016A", "FILL-016A", "FILL-016A"])
        .deck(1, &["FILL-016A", "FILL-016A", "FILL-016A"])
        .memory(10)
        .start();
    let metalgarurumon = runner.place_stack(0, &[CARD_ID, "EX12-035"]);
    super::support::push_to_trash(&mut runner, 0, "VB-ASM");
    (runner, metalgarurumon)
}

/// Leave MetalGarurumon by effect and accept its own Decode on MetalGreymon,
/// answering only the Decode prompts. Stops at the first prompt that is not
/// part of the Decode (or when nothing is pending).
fn decode_metalgreymon_out(runner: &mut DebugRunner, metalgarurumon: digimon_engine::permanent::PermanentHandle) {
    runner.game.return_to_hand_from_effect(metalgarurumon, 1);
    for _ in 0..6 {
        let Some(view) = runner.pending_selection_view() else { return };
        if view.prompt.contains("Assembly") {
            return;
        }
        select_first_non_pass(runner);
        if field_contains(runner, 0, CARD_ID) {
            return;
        }
    }
}

#[test]
fn ex12_016_assembly_is_offered_when_decode_plays_it_from_digivolution_cards() {
    let (mut runner, metalgarurumon) = decode_into_metalgreymon_runner();
    decode_metalgreymon_out(&mut runner, metalgarurumon);

    let view = runner
        .pending_selection_view()
        .unwrap_or_else(|| {
            panic!(
                "CR 7-3-2-10: the effect play of MetalGreymon must ask its Assembly \
                 (MetalGreymon on field without it: {}, VB-ASM still in trash: {})",
                field_contains(&runner, 0, CARD_ID),
                super::support::trash_contains(&runner, 0, "VB-ASM")
            )
        });
    assert!(
        view.prompt.contains("Assembly"),
        "the next prompt is MetalGreymon's Assembly trash pick, got {view:?}"
    );
    assert_eq!(view.selecting_player, 0);
    assert!(
        view.is_optional || view.valid_action_ids.contains(&PASS),
        "Assembly is not mandatory (CR 7-3-2-9; DCGO canNoSelect: true): {view:?}"
    );

    select_first_non_pass(&mut runner);
    if runner
        .pending_selection_view()
        .is_some_and(|v| v.prompt.contains("Assembly"))
    {
        // A count-capped pick may need an explicit confirm after the exact count.
        let v = runner.pending_selection_view().unwrap();
        if v.valid_action_ids.contains(&PASS) {
            runner.execute_action(v.selecting_player, PASS).expect("confirm pick");
        }
    }

    assert!(
        field_contains(&runner, 0, CARD_ID),
        "Decode still plays MetalGreymon"
    );
    let slot = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("MetalGreymon on field");
    assert_eq!(
        super::support::source_count(&runner, 0, slot),
        1,
        "the Assembly card is placed under MetalGreymon (CR 7-3-3-3)"
    );
    assert_eq!(super::support::bottom_source_id(&runner, 0, slot), "VB-ASM");
    assert!(!super::support::trash_contains(&runner, 0, "VB-ASM"));
}

#[test]
fn ex12_016_assembly_on_decode_play_may_be_declined() {
    let (mut runner, metalgarurumon) = decode_into_metalgreymon_runner();
    decode_metalgreymon_out(&mut runner, metalgarurumon);

    let view = runner
        .pending_selection_view()
        .expect("CR 7-3-2-10: the effect play of MetalGreymon must ask its Assembly");
    assert!(view.prompt.contains("Assembly"), "got {view:?}");
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline Assembly");

    assert!(field_contains(&runner, 0, CARD_ID), "declining still plays MetalGreymon");
    let slot = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("MetalGreymon on field");
    assert_eq!(super::support::source_count(&runner, 0, slot), 0);
    assert!(super::support::trash_contains(&runner, 0, "VB-ASM"));
}
