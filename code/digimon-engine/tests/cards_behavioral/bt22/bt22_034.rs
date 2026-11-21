//! BT22-034 Reppamon - Digimon, Lv.4, Yellow, DP 5000, Cost 5.
//! Traits: [Holy Beast] / [CS]. Attribute: Vaccine.
//!
//! # Card text (data/card_bundles/BT22-034.md - verbatim)
//!
//! ```text
//! [Digivolve] Lv.3 w/[CS] trait: Cost 2   (plus the standard Lv.3 yellow circle)
//! When effects trash this card from the security stack, you may play this
//!   card without paying the cost.
//! [On Play] [When Digivolving] 1 of your opponent's Digimon gets -3000 DP
//!   until their turn ends. By trashing your top security card, instead 1 of
//!   your opponent's Digimon gets -6000 DP until their turn ends.
//! Inherited: [When Attacking] [Once Per Turn] 1 of your opponent's Digimon
//!   gets -2000 DP for the turn.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT22/Yellow/BT22_034.cs`
//!   - OP / WD: mandatory. With >= 1 security card the controller is asked a
//!     Yes/No bool BEFORE the target pick; "yes" trashes the top security and
//!     the penalty is -6000 only if that card reached the trash; the target
//!     pick is mandatory. With no security the bool is skipped.
//!   - OnDiscardSecurity: `CanTriggerOnTrashSelfSecurity(cardEffect != null)`
//!     -> optional free play of the card from the trash.
//!   - Inherited [When Attacking] [OPT]: mandatory -2000 DP, for the turn.
//!
//! Official Q&A: the processing after "instead" replaces the standard
//! processing (so -6000 REPLACES -3000, they do not stack).
//!
//! Regression origin: a recorded PvP game (BT22-034 digivolved, the player
//! answered the "trash top security?" bool) stopped with "generic_bool met no
//! prompt" because the card had no spec. These tests use synthetic boards.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT22-034";

/// Effect-trash of the controller's top security (the trigger the card's
/// own security-zone clause keys on).
const TRASHER_YAML: &str = r#"
card: DSL-SEC-TRASHER
name: Sec Trasher
kind: digimon
level: 4
color: [yellow]
cost: 0
dp: 3000
effects:
  - when: on_play
    summary: "[On Play] Trash your top security card"
    process:
      - trash_top_security: { of: you }
"#;

fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 4;
    c
}

fn cs_lv3(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.colors = vec![color];
    c.traits = vec!["CS".to_string()];
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT22-034 YAML parses and compiles")
        .from_dsl_yaml(TRASHER_YAML)
        .expect("inline trasher fixture compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(opp_digimon("OPP-A", 8000))
        .add_card(opp_digimon("OPP-B", 9000))
        .add_card(cs_lv3("CS-RED", CardColor::Red))
        .add_card({
            let mut c = cs_lv3("PLAIN-RED", CardColor::Red);
            c.traits.clear();
            c
        })
        .deck(0, &["PAD"; 12])
        .deck(1, &["PAD"; 12])
}

/// Reppamon already on P0's field (no play-flow), then its [On Play] fired.
fn fire_on_play(runner: &mut DebugRunner) -> PermanentHandle {
    let h = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_play_event_triggers(0, h.index as usize, false, false);
    h
}

// ─── Structure ────────────────────────────────────────────────────────────────

#[test]
fn bt22_034_has_printed_metadata_and_cs_digivolve_box() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Reppamon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(5000));
    assert!(card.traits.iter().any(|t| t == "CS"));
    assert!(card
        .alt_paths
        .iter()
        .any(|p| p.kind == CompiledAltPathKind::Digivolve));
}

#[test]
fn bt22_034_digivolves_from_a_non_yellow_cs_lv3_for_cost_2() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    let cs = runner.place_on_field(0, "CS-RED", Some(0));
    let plain = runner.place_on_field(0, "PLAIN-RED", Some(0));
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[encode_digivolve(0, cs.index as u16) as usize],
        1.0,
        "[Digivolve] Lv.3 w/[CS] trait: Cost 2 is legal over a red [CS] Lv.3"
    );
    assert_eq!(
        mask[encode_digivolve(0, plain.index as u16) as usize],
        0.0,
        "a red Lv.3 without [CS] and without the yellow circle is not a legal base"
    );
}

// ─── [On Play] / [When Digivolving] ───────────────────────────────────────────

#[test]
fn bt22_034_on_play_with_security_asks_trash_choice_before_the_target_pick() {
    let mut runner = base()
        .security(0, &["PAD", "PAD", "PAD"])
        .memory(10)
        .start();
    let opp = runner.place_on_field(1, "OPP-A", Some(0));
    fire_on_play(&mut runner);

    let sel = runner.pending_selection().expect("the trash-security choice is parked");
    assert_eq!(sel.kind, SelectionKind::EffectChoice, "Yes/No gate comes first");
    assert_eq!(
        sel.effect_choices.as_ref().map(|c| c.len()),
        Some(2),
        "exactly two branches: trash / don't"
    );
    // Nothing has happened yet.
    assert_eq!(runner.security_count(0), 3);
    assert_eq!(runner.effective_dp(opp), Some(8000));
}

#[test]
fn bt22_034_trashing_top_security_gives_minus_6000_instead_of_minus_3000() {
    let mut runner = base()
        .security(0, &["PAD", "PAD", "PAD"])
        .memory(10)
        .start();
    let opp = runner.place_on_field(1, "OPP-A", Some(0));
    let trash_before = runner.trash_size(0);
    fire_on_play(&mut runner);

    runner.execute_branch(0).expect("accept: trash top security");
    runner.auto_resolve().expect("pick the only opposing Digimon");

    assert_eq!(runner.security_count(0), 2, "the top security card was trashed");
    assert_eq!(runner.trash_size(0), trash_before + 1);
    assert_eq!(
        runner.effective_dp(opp),
        Some(8000 - 6000),
        "-6000 REPLACES the standard -3000 (official Q&A on \"instead\")"
    );
}

#[test]
fn bt22_034_declining_the_trash_gives_minus_3000_and_keeps_security() {
    let mut runner = base()
        .security(0, &["PAD", "PAD", "PAD"])
        .memory(10)
        .start();
    let opp = runner.place_on_field(1, "OPP-A", Some(0));
    fire_on_play(&mut runner);

    runner.execute_branch(1).expect("decline");
    runner.auto_resolve().expect("pick the only opposing Digimon");

    assert_eq!(runner.security_count(0), 3, "no security was trashed");
    assert_eq!(runner.effective_dp(opp), Some(8000 - 3000));
}

#[test]
fn bt22_034_no_security_skips_the_choice_and_applies_minus_3000() {
    let mut runner = base().memory(10).start();
    let opp = runner.place_on_field(1, "OPP-A", Some(0));
    fire_on_play(&mut runner);

    // DCGO skips the bool when there is no security card to trash: the very
    // first decision is the mandatory target pick.
    let sel = runner.pending_selection().expect("target pick parked");
    assert_ne!(
        sel.kind,
        SelectionKind::EffectChoice,
        "no Yes/No gate is offered when there is nothing to trash"
    );
    runner.auto_resolve().expect("resolve target pick");
    assert_eq!(runner.effective_dp(opp), Some(8000 - 3000));
}

#[test]
fn bt22_034_target_pick_chooses_among_opposing_digimon() {
    let mut runner = base()
        .security(0, &["PAD", "PAD"])
        .memory(10)
        .start();
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    let b = runner.place_on_field(1, "OPP-B", Some(0));
    fire_on_play(&mut runner);

    runner.execute_branch(0).expect("accept");
    let sel = runner.pending_selection().expect("target pick");
    assert!(
        sel.valid_action_ids.len() >= 2,
        "both opposing Digimon are offered as targets (mandatory, no PASS needed)"
    );
    // Pick the LAST offered target (the second Digimon).
    let player = sel.selecting_player;
    let last = *sel.valid_action_ids.last().unwrap();
    runner.execute_action(player, last).expect("pick second target");
    runner.auto_resolve().ok();

    let dps = (runner.effective_dp(a), runner.effective_dp(b));
    assert!(
        dps == (Some(8000), Some(9000 - 6000)) || dps == (Some(8000 - 6000), Some(9000)),
        "exactly one Digimon took -6000: {dps:?}"
    );
}

// ─── When effects trash this card from security ───────────────────────────────

#[test]
fn bt22_034_effect_trash_from_security_offers_optional_free_play() {
    let mut runner = base()
        .hand(0, &["DSL-SEC-TRASHER"])
        .security(0, &["PAD", "PAD", CARD_ID])
        .memory(8)
        .start();

    runner.play(0, 0).expect("play the trasher");
    assert!(
        runner.pending_selection().is_some(),
        "effect-trashing Reppamon from security installs its optional free play"
    );
    assert!(runner.pending_is_optional(), "'you may play this card'");
    runner.accept_optional_trigger().expect("accept the free play");
    runner.auto_resolve().ok();

    let on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(on_field, "Reppamon is played from the trash without paying the cost");
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "and it left the trash"
    );
}

#[test]
fn bt22_034_declining_the_free_play_leaves_it_in_trash() {
    let mut runner = base()
        .hand(0, &["DSL-SEC-TRASHER"])
        .security(0, &["PAD", "PAD", CARD_ID])
        .memory(8)
        .start();
    runner.play(0, 0).expect("play the trasher");
    runner.decline_optional_trigger().expect("decline");
    runner.auto_resolve().ok();
    assert!(runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == CARD_ID));
}

// ─── Inherited [When Attacking] [Once Per Turn] ───────────────────────────────

#[test]
fn bt22_034_inherited_clause_is_a_mandatory_once_per_turn_when_attacking() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).unwrap();
    let inherited = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited [When Attacking] clause");
    assert!(inherited.once_per_turn);
    assert!(!inherited.optional, "no \"may\" in the printed text");
}

#[test]
fn bt22_034_inherited_when_attacking_minus_2000_once_per_turn() {
    let mut carrier = make_test_card("CARRIER", "Carrier");
    carrier.card_kind = CardKind::Digimon;
    carrier.level = Some(5);
    carrier.dp = Some(7000);
    let mut runner = base().add_card(carrier).memory(10).start();
    let opp = runner.place_on_field(1, "OPP-A", Some(0));
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(stack));
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("mandatory target pick");
    assert_eq!(runner.effective_dp(opp), Some(8000 - 2000), "-2000 for the turn");

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(stack));
    runner.game.drain_effect_queue();
    runner.auto_resolve().ok();
    assert_eq!(
        runner.effective_dp(opp),
        Some(8000 - 2000),
        "[Once Per Turn]: a second attack the same turn does not stack another -2000"
    );
}
