//! BT12-028 Paildramon — Digimon, Lv.5, Blue/Green, DP 8000, Cost 8.
//! Traits: Dragonkin.
//! Evo: Lv4 Blue / Cost 4 (printed evo_costs).
//! DNA: Lv4 Blue + Lv4 Green / Cost 0.
//!
//! # Card text (cards.json)
//!
//! ```text
//! Effect:
//! [When Digivolving] Trash the top 3 digivolution cards of all of your
//! opponent's Digimon. Then, if DNA digivolving, 2 of your opponent's
//! Digimon with no digivolution cards can't attack until the end of your
//! opponent's turn.
//!
//! Inherited Effect:
//! [End of Attack] If this Digimon has [Imperialdramon] in its name or
//! [Free] trait, gain 1 memory.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT12/Blue/BT12_028.cs
//!
//! # Clause coverage
//! - Clause 0a ([When Digivolving] trash top 3 from all opp Digimon):
//!     REUSABLE VERB LANDED — `trash_top_n_digivolution_cards_of_each`; the
//!     card remains load-only until the DNA follow-up and inherited predicate
//!     blockers are wired into the full clause.
//! - Clause 0b ([When Digivolving] DNA → CannotAttack 2 targets):
//!     IMPLEMENTED via DNA-origin gating
//! - Clause 1  ([Inherited][End of Attack] gain memory if carrier name/trait):
//!     IMPLEMENTED via source-name predicate support
//!
//! # Patterns covered
//! - G2: DNA digivolve alt-path (dna_digivolve with two materials)
//! - Track E reusable source-trashing verb is covered by the DSL zone-movement tests.
//! - Inherited End of Attack conditional memory gain (implemented)

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Runner factory ──────────────────────────────────────────────────────────

fn make_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card
}

fn make_named_digimon(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(8000);
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn paildramon() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT12-028")
        .expect("BT12-028 in embedded DSL pack")
        .add_card(make_digimon("MAT-BLUE"))
        .add_card(make_digimon("MAT-GREEN"))
        .add_card(make_digimon("OWN-BASE"))
        .add_card(make_digimon("OWN-SRC"))
        .add_card(make_digimon("OPP-A"))
        .add_card(make_digimon("OPP-A1"))
        .add_card(make_digimon("OPP-A2"))
        .add_card(make_digimon("OPP-A3"))
        .add_card(make_digimon("OPP-A4"))
        .add_card(make_digimon("OPP-B"))
        .add_card(make_digimon("OPP-C"))
        .add_card(make_digimon("OPP-C1"))
        .add_card(make_digimon("OPP-C2"))
        .memory(8)
        .start()
}

fn stack_ids(runner: &DebugRunner, handle: PermanentHandle) -> Vec<String> {
    runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .card_sources
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn trash_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .trash
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle, dna_origin: bool) {
    let card = runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .top_card()
        .handle();
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Digivolved {
            player: handle.player,
            permanent: handle,
            card,
            effect_initiated: false,
            dna_origin,
        },
    );
    runner.game.drain_effect_queue();
}

// ─── §1 Structural assertions ─────────────────────────────────────────────────

#[test]
fn bt12_028_has_dna_digivolve_alt_path() {
    let runner = paildramon();
    let compiled = runner.compiled_card("BT12-028").expect("BT12-028 compiled");

    let dna = compiled
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::DnaDigivolve);
    assert!(
        dna.is_some(),
        "BT12-028 must declare a DnaDigivolve alt-path (Lv4-Blue + Lv4-Green)"
    );

    let dna = dna.unwrap();
    assert_eq!(
        dna.materials.len(),
        2,
        "DNA alt-path must list exactly 2 material predicates"
    );
}

#[test]
fn bt12_028_compiles_implemented_main_and_inherited_clauses() {
    let runner = paildramon();
    let compiled = runner.compiled_card("BT12-028").expect("BT12-028 compiled");
    assert_eq!(
        compiled.effects.len(),
        2,
        "BT12-028 should compile the main When Digivolving clause plus inherited EndOfAttack clause"
    );
}

// ─── §2 Behavioral — Clause 0a: trash top 3 divi-cards (deferred card wiring) ─

#[test]
fn bt12_028_when_digivolving_trashes_top_3_source_cards_from_each_opponent_digimon() {
    let mut runner = paildramon();
    let own = runner.place_stack(0, &["OWN-SRC", "OWN-BASE"]);
    let paildramon = runner.place_on_field(0, "BT12-028", Some(0));
    let opp_with_four_sources =
        runner.place_stack(1, &["OPP-A1", "OPP-A2", "OPP-A3", "OPP-A4", "OPP-A"]);
    let opp_without_sources = runner.place_stack(1, &["OPP-B"]);

    fire_when_digivolving(&mut runner, paildramon, false);
    runner.auto_resolve().expect("non-DNA clause resolves");

    assert_eq!(
        stack_ids(&runner, opp_with_four_sources),
        vec!["OPP-A1", "OPP-A"],
        "top 3 digivolution cards are trashed, leaving the deepest source plus top card"
    );
    assert_eq!(
        stack_ids(&runner, opp_without_sources),
        vec!["OPP-B"],
        "opponent Digimon with no digivolution cards is unchanged"
    );
    assert_eq!(
        stack_ids(&runner, own),
        vec!["OWN-SRC", "OWN-BASE"],
        "own Digimon stacks are not touched"
    );
    assert_eq!(
        trash_ids(&runner, 1),
        vec!["OPP-A4", "OPP-A3", "OPP-A2"],
        "trashed sources go to the opponent owner's trash in top-source-first order"
    );
    assert!(
        runner.pending_selection().is_none(),
        "standard digivolve must not park the DNA-only target selection"
    );
}

// ─── §3 Behavioral — Clause 0b: DNA sub-condition ───────────────────────────

#[test]
fn bt12_028_dna_digivolve_applies_cannot_attack_to_two_digimon_with_no_sources() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-028")
        .expect("BT12-028 in embedded DSL pack")
        .add_card(make_digimon("MAT-BLUE"))
        .add_card(make_digimon("MAT-GREEN"))
        .add_card(make_digimon("OWN-NO-SRC"))
        .add_card(make_digimon("OPP-A"))
        .add_card(make_digimon("OPP-B"))
        .add_card(make_digimon("OPP-C"))
        .add_card(make_digimon("OPP-C1"))
        .add_card(make_digimon("OPP-C2"))
        .hand(0, &["BT12-028"])
        .memory(8)
        .start();

    let material_a = runner.place_on_field(0, "MAT-BLUE", Some(0));
    let material_b = runner.place_on_field(0, "MAT-GREEN", Some(0));
    let own_no_sources = runner.place_on_field(0, "OWN-NO-SRC", Some(0));
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));
    let opp_c = runner.place_stack(1, &["OPP-C1", "OPP-C2", "OPP-C"]);
    let hand_card = runner.game.player(0).hand[0].handle();

    {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card, None, 0);
        ctx.effect_initiated_dna_digivolve(material_a, material_b, hand_card, 0, true)
            .expect("BT12-028 DNA digivolves from hand");
    }

    assert_eq!(
        stack_ids(&runner, opp_c),
        vec!["OPP-C"],
        "trash step resolves before target selection, making the former stacked Digimon eligible"
    );

    let view = runner
        .pending_selection_view()
        .expect("DNA branch must park a visible target selection");
    assert_eq!(
        view.kind,
        SelectionKind::OppField,
        "DNA branch selects up to 2 opponent Digimon"
    );
    assert_eq!(
        view.selecting_player, 0,
        "BT12-028's controller chooses targets"
    );
    assert!(
        !view.is_optional,
        "the first DNA target is mandatory when eligible targets exist"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        3,
        "only opponent Digimon with no digivolution cards are legal targets"
    );

    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("select first no-source opponent Digimon");
    let view = runner
        .pending_selection_view()
        .expect("second DNA target selection remains pending");
    assert_eq!(
        view.kind,
        SelectionKind::OppField
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("select second no-source opponent Digimon");
    runner.auto_resolve().expect("DNA branch resolves");

    assert!(runner.game.modifiers.has(opp_a, ModifierType::CannotAttack));
    assert!(runner.game.modifiers.has(opp_b, ModifierType::CannotAttack));
    assert!(
        !runner.game.modifiers.has(opp_c, ModifierType::CannotAttack),
        "unselected eligible opponent Digimon is not locked"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(own_no_sources, ModifierType::CannotAttack),
        "own Digimon are excluded from the opponent target selection"
    );
}

#[test]
fn bt12_028_standard_digivolve_does_not_trigger_dna_sub_condition() {
    let mut runner = paildramon();
    let paildramon = runner.place_on_field(0, "BT12-028", Some(0));
    let opp = runner.place_on_field(1, "OPP-A", Some(0));

    fire_when_digivolving(&mut runner, paildramon, false);
    runner.auto_resolve().expect("standard digivolve resolves");

    assert!(
        !runner.game.modifiers.has(opp, ModifierType::CannotAttack),
        "standard digivolve must not apply the DNA-only CannotAttack rider"
    );
    assert!(
        runner.pending_selection().is_none(),
        "standard digivolve must not install a DNA-only selection"
    );
}

// ─── §4 Behavioral — Clause 1: Inherited [End of Attack] ────────────────────

fn fire_end_of_attack(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfAttack, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

#[test]
fn bt12_028_inherited_end_of_attack_gains_memory_when_carrier_has_imperialdramon_in_name() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-028")
        .expect("BT12-028 in embedded DSL pack")
        .add_card(make_named_digimon(
            "IMPERIAL-CARRIER",
            "Imperialdramon Fighter Mode",
            &[],
        ))
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["BT12-028", "IMPERIAL-CARRIER"]);
    fire_end_of_attack(&mut runner, carrier);

    assert_eq!(
        runner.memory(),
        1,
        "carrier with Imperialdramon in name should gain 1 memory from BT12-028 inherited effect"
    );
}

#[test]
fn bt12_028_inherited_end_of_attack_gains_memory_when_carrier_has_free_trait() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-028")
        .expect("BT12-028 in embedded DSL pack")
        .add_card(make_named_digimon("FREE-CARRIER", "Paildramon", &["Free"]))
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["BT12-028", "FREE-CARRIER"]);
    fire_end_of_attack(&mut runner, carrier);

    assert_eq!(
        runner.memory(),
        1,
        "carrier with Free trait should gain 1 memory from BT12-028 inherited effect"
    );
}

#[test]
fn bt12_028_inherited_end_of_attack_does_not_fire_for_unrelated_carrier() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-028")
        .expect("BT12-028 in embedded DSL pack")
        .add_card(make_named_digimon("UNRELATED", "Tyrannomon", &[]))
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["BT12-028", "UNRELATED"]);
    fire_end_of_attack(&mut runner, carrier);

    assert_eq!(
        runner.memory(),
        0,
        "carrier without Imperialdramon name or Free trait should not gain memory"
    );
}

// ─── §5 Inherited scope structural check ────────────────────────────────────

/// Inherited clause shape: scope=Inherited, when=EndOfAttack, not optional,
/// no OPT flag.
#[test]
fn bt12_028_inherited_clause_is_end_of_attack_non_optional() {
    use digimon_dsl::compiled::CompiledClause;

    let runner = paildramon();
    let compiled = runner.compiled_card("BT12-028").expect("BT12-028 compiled");

    let inherited: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        inherited.len(),
        1,
        "BT12-028 has exactly one inherited clause"
    );
    let clause = &inherited[0];
    assert!(
        clause.when.contains(&CompiledTiming::EndOfAttack),
        "inherited clause fires at EndOfAttack"
    );
    assert!(
        !clause.optional,
        "inherited clause is mandatory (no 'you may')"
    );
    assert!(
        !clause.once_per_turn,
        "inherited clause has no OPT restriction"
    );
}
