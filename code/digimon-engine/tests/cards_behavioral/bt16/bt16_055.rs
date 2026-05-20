//! BT16-055 Namakemon — Digimon, Lv.4, Black, Puppet.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [On Play] [When Digivolving] If you have 3 or more security cards, until
//! the end of your opponent's turn, 1 of your Digimon can't have its DP
//! reduced by your opponent's effects and isn't affected by <De-Digivolve>
//! effects. If you have 3 or fewer, 1 of your Digimon gains <Blocker> and
//! <Reboot> until the end of your opponent's turn.
//!
//! Inherited: [All Turns] While this Digimon has [Pulsemon] in its text, it
//! gets +1000 DP.
//! ```
//!
//! # Legacy reference
//!
//! DCGO reference unavailable in this checkout.
//!
//! # Coverage
//!
//! - Metadata and normal/[Pulsemon] digivolution paths.
//! - Faithful active low-security branch: security <= 3 selects 1 own Digimon
//!   and grants <Blocker> + <Reboot> until the opponent's turn ends.
//! - Gap-routed high-security protection and inherited text-gated DP aura.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDpConstraint,
    CompiledTiming,
};
use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn bt16_055_metadata_paths_and_supported_clause_shape_match_printed_text() {
    let runner = namakemon_runner().start();
    let compiled = runner.compiled_card("BT16-055").expect("BT16-055 compiles");

    assert_eq!(compiled.name, "Namakemon");
    assert_eq!(compiled.level, Some(4));
    assert_eq!(compiled.cost, Some(4));
    assert_eq!(compiled.dp, Some(4000));
    assert_eq!(compiled.color, vec![CompiledColor::Black]);
    assert!(compiled
        .traits
        .iter()
        .any(|trait_name| trait_name == "Puppet"));

    let digivolve_paths: Vec<_> = compiled
        .alt_paths
        .iter()
        .filter(|path| path.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert!(
        digivolve_paths.iter().any(|path| {
            path.cost == Some(CompiledCost::Literal(2))
                && path.from.as_ref().is_some_and(|from| {
                    from.level_eq == Some(3) && from.color_is == Some(CompiledColor::Black)
                })
        }),
        "normal black Lv.3 digivolve path must cost 2; paths={digivolve_paths:?}"
    );
    assert!(
        digivolve_paths.iter().any(|path| {
            path.cost == Some(CompiledCost::Literal(2))
                && path
                    .from
                    .as_ref()
                    .is_some_and(|from| from.name_is.as_deref() == Some("Pulsemon"))
        }),
        "[Pulsemon] digivolve path must cost 2; paths={digivolve_paths:?}"
    );

    let supported_low_security = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .filter(|triggered| {
            triggered.when.contains(&CompiledTiming::OnPlay)
                && triggered.when.contains(&CompiledTiming::WhenDigivolving)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        supported_low_security.len(),
        1,
        "only the faithful <=3 security keyword-grant slice should be active"
    );
    assert_eq!(
        supported_low_security[0]
            .condition
            .as_ref()
            .and_then(|predicate| predicate.security_count_lte.clone()),
        Some(CompiledDpConstraint::Literal(3)),
        "active slice must require your security count <= 3"
    );
    assert!(
        !compiled.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(triggered)
                if triggered
                    .condition
                    .as_ref()
                    .and_then(|predicate| predicate.security_count_gte.clone())
                    == Some(CompiledDpConstraint::Literal(3))
        )),
        "omit the >=3 protection slice until narrow anti-DP-reduction and anti-De-Digivolve are expressible"
    );
}

#[test]
fn bt16_055_on_play_low_security_grants_blocker_and_reboot_to_selected_digimon() {
    let mut runner = namakemon_runner()
        .add_card(make_digimon("ALLY", 3, 2000))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["BT16-055"])
        .security(0, &["FILLER", "FILLER"])
        .memory(10)
        .start();
    let ally = runner.place_on_field(0, "ALLY", Some(0));

    runner.play(0, 0).expect("play Namakemon");

    let view = runner
        .pending_selection_view()
        .expect("low-security own Digimon selection");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(
        view.valid_action_ids.contains(&encode_permanent(ally)),
        "ally must be a legal recipient"
    );
    assert!(
        !runner.pending_is_optional(),
        "printed text is mandatory once the <=3 condition is met"
    );
    runner
        .execute_action(0, encode_permanent(ally))
        .expect("choose ally");
    runner.auto_resolve().expect("finish Namakemon effect");

    assert_has_low_security_keywords(&runner, ally);
}

#[test]
fn bt16_055_when_digivolving_uses_same_low_security_keyword_grant() {
    let mut runner = namakemon_runner()
        .add_card(make_digimon("BASE", 3, 1000))
        .add_card(make_test_card("FILLER", "Filler"))
        .security(0, &["FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();
    let namakemon = runner.place_stack(0, &["BASE", "BT16-055"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(namakemon),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("low-security own Digimon selection");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert_eq!(
        view.valid_action_ids,
        vec![encode_permanent(namakemon)],
        "only Namakemon's own stack is on field"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose Namakemon stack");
    runner.auto_resolve().expect("finish Namakemon effect");

    assert_has_low_security_keywords(&runner, namakemon);
}

#[test]
fn bt16_055_low_security_keywords_expire_after_opponents_turn_ends() {
    let mut runner = namakemon_runner()
        .add_card(make_digimon("ALLY", 3, 2000))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["BT16-055"])
        .security(0, &["FILLER"])
        .deck(0, &["FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER"])
        .memory(10)
        .start();
    let ally = runner.place_on_field(0, "ALLY", Some(0));

    runner.play(0, 0).expect("play Namakemon");
    runner
        .execute_action(0, encode_permanent(ally))
        .expect("choose ally");
    runner.auto_resolve().expect("finish Namakemon effect");

    assert_has_low_security_keywords(&runner, ally);
    runner.end_turn();
    assert_has_low_security_keywords(&runner, ally);
    runner.end_turn();
    assert!(
        !runner.game.has_keyword(ally, Keyword::Blocker),
        "Blocker expires when the opponent's turn ends"
    );
    assert!(
        !runner.game.has_keyword(ally, Keyword::Reboot),
        "Reboot expires when the opponent's turn ends"
    );
}

#[test]
fn bt16_055_does_not_prompt_low_security_branch_above_three_security() {
    let mut runner = namakemon_runner()
        .add_card(make_digimon("ALLY", 3, 2000))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["BT16-055"])
        .security(0, &["FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();
    runner.place_on_field(0, "ALLY", Some(0));

    runner.play(0, 0).expect("play Namakemon");

    assert!(
        runner.pending_selection().is_none(),
        "the <=3 keyword branch must not fire at 4 security; the >=3 protection branch is intentionally omitted"
    );
}

#[test]
#[ignore = "BLOCKED: PUPPETS-G024 — current DSL can only grant broad effect immunity or simple modifiers, not the printed narrow DP-reduction/opponent <De-Digivolve> protection bundle"]
fn bt16_055_high_security_selects_one_digimon_for_dp_reduction_and_de_digivolve_protection() {
    let mut runner = namakemon_runner()
        .add_card(make_digimon("ALLY", 3, 2000))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["BT16-055"])
        .security(0, &["FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();
    let ally = runner.place_on_field(0, "ALLY", Some(0));

    runner.play(0, 0).expect("play Namakemon");
    let view = runner
        .pending_selection_view()
        .expect("high-security protection target selection");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(view.valid_action_ids.contains(&encode_permanent(ally)));
}

#[test]
fn bt16_055_inherited_gives_1000_dp_only_when_carrier_text_contains_pulsemon() {
    let mut runner = namakemon_runner()
        .add_card(make_digimon_with_effect_text(
            "PULSEMON-TEXT-CARRIER",
            4,
            4000,
            "This card mentions [Pulsemon].",
        ))
        .add_card(make_digimon_with_effect_text(
            "NO-PULSEMON-TEXT-CARRIER",
            4,
            4000,
            "This card does not mention the required name.",
        ))
        .start();
    let pulsemon_text_carrier = runner.place_stack(0, &["BT16-055", "PULSEMON-TEXT-CARRIER"]);
    let non_pulsemon_text_carrier =
        runner.place_stack(0, &["BT16-055", "NO-PULSEMON-TEXT-CARRIER"]);

    assert_eq!(runner.effective_dp(pulsemon_text_carrier), Some(5000));
    assert_eq!(runner.effective_dp(non_pulsemon_text_carrier), Some(4000));
}

fn namakemon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT16-055")
        .expect("BT16-055 YAML loads")
}

fn encode_permanent(handle: PermanentHandle) -> u16 {
    encode_attack(handle.player as u16, handle.index as u16)
}

fn assert_has_low_security_keywords(runner: &DebugRunner, handle: PermanentHandle) {
    assert!(
        runner.game.has_keyword(handle, Keyword::Blocker),
        "selected Digimon gains Blocker"
    );
    assert!(
        runner.game.has_keyword(handle, Keyword::Reboot),
        "selected Digimon gains Reboot"
    );
}

fn make_digimon(id: &str, level: u8, dp: i32) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

fn make_digimon_with_effect_text(
    id: &str,
    level: u8,
    dp: i32,
    effect_text: &str,
) -> digimon_engine::card_data::CardData {
    let mut card = make_digimon(id, level, dp);
    card.effect_text = effect_text.to_string();
    card
}
