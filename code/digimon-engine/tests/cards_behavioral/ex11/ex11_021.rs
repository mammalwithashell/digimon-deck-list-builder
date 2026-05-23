//! EX11-021 Kokeshimon — Digimon, Lv.4, Yellow/Black.
//!
//! # Card text (cards.json)
//!
//! [When Digivolving] If you have 1 or fewer Tamers, you may play 1
//! [Mirai Kinosaki] from your hand without paying the cost.
//!
//! Inherited Effect [Opponent's Turn] [Once Per Turn]
//! When one of your opponent's Digimon attacks, by deleting 1 of your other
//! Digimon, end that attack.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX11/Yellow/EX11_021.cs
//!
//! # Patterns this test covers
//! - B-adjacent: [When Digivolving] optional free-play gated on a Tamer count
//! - E2 OPT + optional decline (the inherited cost-effect is declinable)
//! - F-adjacent: inherited [Opponent's Turn] attack-end by deleting an
//!   *other* own Digimon (EX11-020 Hanimon idiom — `other: true` filter)
//! - `on_opponent_attack` declared-attack observer timing (G-DSL-ON-OPPONENT-ATTACK
//!   / `OnAllyAttack`-`OnOpponentAttack` engine slice — both RESOLVED)
//! - Cost-firing clause: the deleted other Digimon is paid as a cost
//!
//! # Audit note
//! The inherited clause's printed cost is "by deleting 1 of your **other**
//! Digimon" — NOT the carrier itself. An earlier YAML revision deleted `this`
//! (copied from a sibling whose text genuinely says "this Digimon"); that was
//! corrected to a `select_own_permanent { other: true, kind: digimon }` cost,
//! matching EX11-020 Hanimon, whose printed text is identical.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledCost, CompiledPredicate,
    CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlaySource};
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Card builders ────────────────────────────────────────────────────

fn make_tamer(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card
}

/// A Lv.3 yellow Digimon usable as a digivolution base for EX11-021's
/// printed Lv.3-yellow alt-path.
fn make_lv3_yellow_base(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card.colors = vec![CardColor::Yellow];
    card
}

/// Recursively test whether a predicate tree contains `other: true`.
fn predicate_requires_other(p: &CompiledPredicate) -> bool {
    p.other == Some(true)
        || p.all_of.iter().any(predicate_requires_other)
        || p.any_of.iter().any(predicate_requires_other)
}

/// Recursively test whether a predicate tree constrains kind to Digimon.
fn predicate_requires_digimon(p: &CompiledPredicate) -> bool {
    p.kind == Some(CompiledCardKind::Digimon)
        || p.all_of.iter().any(predicate_requires_digimon)
        || p.any_of.iter().any(predicate_requires_digimon)
}

fn card_ids_on_field(runner: &DebugRunner, player: usize) -> Vec<String> {
    runner.game.players[player]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

// ─── SECTION 1 — Structural assertions ────────────────────────────────

#[test]
fn ex11_021_has_printed_metadata_and_evolution_paths() {
    let runner = DebugRunner::builder()
        .dsl_card("EX11-021")
        .expect("EX11-021 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("EX11-021")
        .expect("EX11-021 must be compiled");

    assert_eq!(compiled.name, "Kokeshimon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(4));
    assert_eq!(compiled.cost, Some(5));
    assert_eq!(compiled.dp, Some(6000));
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Yellow, CompiledColor::Purple],
        "Kokeshimon is dual-colour Yellow/Black (Black lowers to Purple)"
    );
    assert!(compiled.traits.iter().any(|t| t == "Puppet"));
    assert!(compiled.traits.iter().any(|t| t == "LIBERATOR"));

    // Printed standard digivolve: Lv.3 yellow.
    assert!(
        compiled.alt_paths.iter().any(|path| {
            matches!(path.cost, Some(CompiledCost::Literal(3)))
                && path.from.as_ref().is_some_and(|from| {
                    from.level_eq == Some(3) && from.color_is == Some(CompiledColor::Yellow)
                })
        }),
        "standard Lv.3 yellow digivolution should cost 3"
    );
    // Printed alternate digivolve: Lv.3 with [Puppet] trait → cost 2.
    assert!(
        compiled.alt_paths.iter().any(|path| {
            matches!(path.cost, Some(CompiledCost::Literal(2)))
                && path.from.as_ref().is_some_and(|from| {
                    from.all_of.iter().any(|p| p.level_eq == Some(3))
                        && from
                            .all_of
                            .iter()
                            .any(|p| p.trait_has.as_deref() == Some("Puppet"))
                })
        }),
        "alternate Lv.3 [Puppet] digivolution should cost 2"
    );
}

#[test]
fn ex11_021_has_when_digivolving_optional_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("EX11-021")
        .expect("EX11-021 YAML loads")
        .start();
    let compiled = runner.compiled_card("EX11-021").expect("compiled");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving) => {
                Some(t)
            }
            _ => None,
        })
        .expect("When Digivolving clause exists");

    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert_eq!(clause.when, vec![CompiledTiming::WhenDigivolving]);
    assert!(
        clause.optional,
        "the free play is optional (\"you may play\")"
    );
    assert!(
        !clause.once_per_turn,
        "When Digivolving free play is not Once Per Turn"
    );
    assert!(
        clause.condition.is_some(),
        "the clause is gated on the 1-or-fewer-Tamers count"
    );
    // Behavioral coverage of the select/play steps lives in Section 3.
}

#[test]
fn ex11_021_has_inherited_opponent_turn_opt_attack_end_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("EX11-021")
        .expect("EX11-021 YAML loads")
        .start();
    let compiled = runner.compiled_card("EX11-021").expect("compiled");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnOpponentAttack) => {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited on_opponent_attack clause exists");

    assert_eq!(
        clause.scope,
        CompiledScope::Inherited,
        "the attack-end clause is an inherited effect"
    );
    assert_eq!(clause.when, vec![CompiledTiming::OnOpponentAttack]);
    assert!(
        clause.once_per_turn,
        "the inherited clause is [Once Per Turn]"
    );
    assert!(
        clause.active_when.is_some(),
        "the clause is gated to the opponent's turn via active_when"
    );
    // Audit-critical structural facts (faithfulness regression guards):
    //  - the cost target must EXCLUDE the carrier ("1 of your OTHER Digimon");
    //  - the cost selection must be declinable ("by deleting ..." is a may);
    //  - the end-attack payoff must be gated behind an `if` so it only fires
    //    when the cost was actually paid.
    let select_step = clause
        .process
        .iter()
        .find_map(|s| match s {
            CompiledStep::SelectOwnPermanent {
                filter, optional, ..
            } => Some((filter, optional)),
            _ => None,
        })
        .expect("the cost selects one of your own permanents");
    assert!(
        *select_step.1,
        "the cost selection is declinable (\"by deleting ...\" may be passed up)"
    );
    assert!(
        predicate_requires_other(select_step.0),
        "the deletion target must EXCLUDE the carrier itself (\"other Digimon\")"
    );
    assert!(
        predicate_requires_digimon(select_step.0),
        "the deletion target must be a Digimon"
    );
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::If { .. })),
        "the end-attack payoff is gated behind an `if` so it only fires when \
         the deletion cost was actually paid"
    );
    // The end_attack outcome itself is covered behaviorally in Section 4.
}

// ─── SECTION 2 — Condition gating: When Digivolving Tamer count ───────

/// Positive: with 1 or fewer Tamers the When Digivolving free-play prompt
/// installs.
#[test]
fn ex11_021_when_digivolving_offers_play_with_one_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-021")
        .expect("EX11-021 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_tamer("MIRAI", "Mirai Kinosaki"))
        .add_card(make_tamer("ONE-TAMER", "A Tamer"))
        .hand(0, &["MIRAI"])
        .memory(10)
        .start();
    // Exactly one Tamer on the field — the 1-or-fewer gate passes.
    runner.place_on_field(0, "ONE-TAMER", Some(0));
    let koko = runner.place_stack(0, &["BASE", "EX11-021"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(koko),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("Mirai hand prompt installs at 1 Tamer");
    assert_eq!(view.kind, SelectionKind::Hand);
}

/// Negative: with 2 or more Tamers the condition blocks — no prompt at all.
#[test]
fn ex11_021_when_digivolving_blocked_with_two_tamers() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-021")
        .expect("EX11-021 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_tamer("MIRAI", "Mirai Kinosaki"))
        .add_card(make_tamer("TAMER-A", "Tamer A"))
        .add_card(make_tamer("TAMER-B", "Tamer B"))
        .hand(0, &["MIRAI"])
        .memory(10)
        .start();
    runner.place_on_field(0, "TAMER-A", Some(0));
    runner.place_on_field(0, "TAMER-B", Some(0));
    let koko = runner.place_stack(0, &["BASE", "EX11-021"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(koko),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection_view().is_none(),
        "2 Tamers fails the 1-or-fewer gate: no Mirai prompt installs"
    );
    assert_eq!(
        runner.hand_size(0),
        1,
        "Mirai stays in hand when the clause is gated out"
    );
}

// ─── SECTION 3 — Behavioral: When Digivolving free play ───────────────

/// The prompt is optional and can be declined cleanly — no hidden auto-play.
#[test]
fn ex11_021_when_digivolving_play_is_optional_and_declinable() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-021")
        .expect("EX11-021 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_tamer("MIRAI", "Mirai Kinosaki"))
        .hand(0, &["MIRAI"])
        .memory(10)
        .start();
    let koko = runner.place_stack(0, &["BASE", "EX11-021"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(koko),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_is_optional(),
        "play-Mirai selection is optional"
    );
    runner
        .execute_action(0, PASS)
        .expect("decline the free play");

    assert!(
        runner.pending_selection_view().is_none(),
        "declining resolves without a hidden auto-play"
    );
    assert_eq!(runner.hand_size(0), 1, "Mirai stays in hand on decline");
    assert!(
        !card_ids_on_field(&runner, 0).contains(&"MIRAI".to_string()),
        "no Mirai was played when the effect was declined"
    );
}

/// Positive behavioral: selecting Mirai actually plays it free into the
/// battle area without spending memory.
#[test]
fn ex11_021_when_digivolving_plays_mirai_free_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-021")
        .expect("EX11-021 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_tamer("MIRAI", "Mirai Kinosaki"))
        .add_card(make_test_card("NOT-MIRAI", "Some Other Card"))
        .hand(0, &["MIRAI", "NOT-MIRAI"])
        .memory(3)
        .start();
    let koko = runner.place_stack(0, &["BASE", "EX11-021"]);
    let memory_before = runner.game.memory;

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(koko),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("Mirai hand prompt installs");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only Mirai Kinosaki is an eligible target, not the unrelated card"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("select Mirai");
    runner.auto_resolve().expect("finish free play");

    assert!(
        card_ids_on_field(&runner, 0).contains(&"MIRAI".to_string()),
        "Mirai Kinosaki entered the battle area"
    );
    assert_eq!(
        runner.hand_size(0),
        1,
        "only Mirai left the hand; the unrelated card stays"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "Mirai is played WITHOUT paying its cost"
    );
}

/// Integrated path: a real digivolve onto a Lv.3 yellow base fires the
/// [When Digivolving] effect through the live digivolution flow.
#[test]
fn ex11_021_when_digivolving_fires_through_real_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-021")
        .expect("EX11-021 YAML loads")
        .add_card(make_lv3_yellow_base("BASE-LV3", "BaseLv3"))
        .add_card(make_tamer("MIRAI", "Mirai Kinosaki"))
        .hand(0, &["EX11-021", "MIRAI"])
        .memory(20)
        .start();
    let base = runner.place_on_field(0, "BASE-LV3", Some(0));
    runner.game.enter_main_phase();

    let koko_hand_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "EX11-021")
        .expect("EX11-021 in hand");
    let ok = runner.game.digivolve_from_hand(
        0,
        koko_hand_idx,
        base.index as usize,
        PlaySource::ByDigivolve,
    );
    assert!(
        ok,
        "digivolve EX11-021 onto the Lv.3 yellow base must succeed"
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("When Digivolving prompt installs through a real digivolve");
    assert_eq!(view.kind, SelectionKind::Hand);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick Mirai");
    runner.auto_resolve().expect("finish free play");

    assert!(
        card_ids_on_field(&runner, 0).contains(&"MIRAI".to_string()),
        "Mirai is played by the When Digivolving effect after a live digivolve"
    );
}

// ─── SECTION 4 — Behavioral: inherited [Opponent's Turn] attack end ───

/// Positive: on the opponent's turn, with another own Digimon present, the
/// inherited clause offers to delete that other Digimon and end the attack.
#[test]
fn ex11_021_inherited_deletes_other_digimon_to_end_opponent_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-021")
        .expect("EX11-021 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("OTHER", "Other Digimon"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("SECURITY", "Security"))
        .security(0, &["SECURITY"])
        .start();
    // EX11-021 is a digivolution source under CARRIER — its inherited
    // effect is active.
    runner.place_stack(0, &["EX11-021", "CARRIER"]);
    runner.place_on_field(0, "OTHER", Some(0));
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: opponent's turn");

    runner.attack_player(attacker, 0, false);

    let view = runner
        .pending_selection_view()
        .expect("other-Digimon cost selection installs");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(
        runner.pending_is_optional(),
        "the cost is declinable and must never auto-delete"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the OTHER Digimon — not the EX11-021 carrier stack — is eligible"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("delete the other Digimon and end the attack");
    runner.auto_resolve().expect("finish attack cancel");

    assert_eq!(
        runner.security_count(0),
        1,
        "the attack ended before the security check"
    );
    assert!(
        card_ids_on_field(&runner, 0).contains(&"CARRIER".to_string()),
        "the carrier stack stays — the printed cost deletes an OTHER Digimon"
    );
    assert!(
        !card_ids_on_field(&runner, 0).contains(&"OTHER".to_string()),
        "the selected other Digimon was deleted as the cost"
    );
    assert!(
        runner.game.pending_attack.is_none(),
        "the attack state is fully cleared"
    );
}

/// Negative (decline): the cost is declinable — passing leaves both Digimon
/// alive and lets the attack resolve normally.
#[test]
fn ex11_021_inherited_decline_keeps_digimon_and_resolves_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-021")
        .expect("EX11-021 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("OTHER", "Other Digimon"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("SECURITY", "Security"))
        .security(0, &["SECURITY"])
        .start();
    runner.place_stack(0, &["EX11-021", "CARRIER"]);
    runner.place_on_field(0, "OTHER", Some(0));
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();

    runner.attack_player(attacker, 0, false);
    runner.execute_action(0, PASS).expect("decline the cost");

    assert_eq!(
        runner.battle_area_size(0),
        2,
        "declining deletes neither the carrier nor the other Digimon"
    );
    assert_eq!(
        runner.security_count(0),
        0,
        "declining lets the attack proceed, so security is checked"
    );
}

/// Negative (no eligible target): a lone EX11-021 carrier with no OTHER
/// Digimon cannot pay the cost, so the attack is never cancelled.
#[test]
fn ex11_021_inherited_cannot_fire_without_another_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-021")
        .expect("EX11-021 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("SECURITY", "Security"))
        .security(0, &["SECURITY"])
        .start();
    // Only the EX11-021 carrier stack — no other Digimon to delete.
    runner.place_stack(0, &["EX11-021", "CARRIER"]);
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();

    runner.attack_player(attacker, 0, false);

    // With no eligible "other Digimon" the cost cannot be paid: either no
    // prompt installs, or a declinable prompt with zero real targets does.
    // Either way the carrier survives and the attack must resolve.
    if runner.pending_selection_view().is_some() {
        assert!(
            runner.pending_is_optional(),
            "any installed prompt must be declinable"
        );
        assert_eq!(
            runner.pending_action_count(),
            0,
            "no other Digimon is an eligible deletion target"
        );
        runner
            .execute_action(0, PASS)
            .expect("pass the unpayable cost");
    }

    assert!(
        card_ids_on_field(&runner, 0).contains(&"CARRIER".to_string()),
        "the lone carrier survives — there is no other Digimon to delete"
    );
    assert_eq!(
        runner.security_count(0),
        0,
        "the attack could not be ended, so the security check happened"
    );
}

/// Negative (your turn): the inherited clause is gated to the opponent's
/// turn — when YOUR Digimon attacks on YOUR turn it must not fire.
#[test]
fn ex11_021_inherited_does_not_fire_on_your_own_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-021")
        .expect("EX11-021 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("OTHER", "Other Digimon"))
        .add_card(make_test_card("SECURITY", "Security"))
        .security(1, &["SECURITY"])
        .start();
    runner.place_stack(0, &["EX11-021", "CARRIER"]);
    let mine = runner.place_on_field(0, "OTHER", Some(0));
    assert_eq!(runner.turn_player(), 0, "precondition: it is your turn");

    // Your own Digimon attacks on your own turn.
    runner.attack_player(mine, 1, false);

    assert!(
        runner.pending_selection_view().is_none(),
        "the [Opponent's Turn] inherited clause must not fire on your turn"
    );
    assert_eq!(
        runner.battle_area_size(0),
        2,
        "no Digimon was deleted by the inherited clause on your turn"
    );
}

/// OPT lockout: the inherited clause is [Once Per Turn] — after it ends the
/// first opponent attack, a second opponent attack the same turn must NOT
/// offer the cost again.
#[test]
fn ex11_021_inherited_once_per_turn_locks_out_second_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-021")
        .expect("EX11-021 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("OTHER-1", "Other Digimon 1"))
        .add_card(make_test_card("OTHER-2", "Other Digimon 2"))
        .add_card(make_test_card("ATTACKER-1", "Attacker 1"))
        .add_card(make_test_card("ATTACKER-2", "Attacker 2"))
        .add_card(make_test_card("SECURITY-A", "Security A"))
        .add_card(make_test_card("SECURITY-B", "Security B"))
        .security(0, &["SECURITY-A", "SECURITY-B"])
        .start();
    runner.place_stack(0, &["EX11-021", "CARRIER"]);
    runner.place_on_field(0, "OTHER-1", Some(0));
    runner.place_on_field(0, "OTHER-2", Some(0));
    let attacker1 = runner.place_on_field(1, "ATTACKER-1", Some(0));
    let attacker2 = runner.place_on_field(1, "ATTACKER-2", Some(0));
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: opponent's turn");

    // First opponent attack — the inherited clause fires; pay the cost.
    runner.attack_player(attacker1, 0, false);
    let view = runner
        .pending_selection_view()
        .expect("first attack offers the cost");
    assert_eq!(view.kind, SelectionKind::OwnField);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("delete an other Digimon and end the first attack");
    runner.auto_resolve().expect("finish first attack cancel");
    assert_eq!(
        runner.security_count(0),
        2,
        "first attack ended before its security check"
    );

    // Second opponent attack the same turn — OPT is spent, no prompt.
    runner.attack_player(attacker2, 0, false);
    assert!(
        runner.pending_selection_view().is_none(),
        "the inherited clause is Once Per Turn: no cost prompt on the 2nd attack"
    );
    assert_eq!(
        runner.security_count(0),
        1,
        "the 2nd attack is not cancelled and checks security"
    );
}

// ─── SECTION 5 — Cost-firing: the deleted other Digimon ───────────────

/// The deletion of the chosen other Digimon is genuinely paid as a cost:
/// exactly that one Digimon leaves the battle area, the carrier survives,
/// and the board count drops by one.
#[test]
fn ex11_021_inherited_delete_cost_removes_exactly_the_chosen_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-021")
        .expect("EX11-021 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("SACRIFICE", "Sacrifice"))
        .add_card(make_test_card("BYSTANDER", "Bystander"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("SECURITY", "Security"))
        .security(0, &["SECURITY"])
        .start();
    runner.place_stack(0, &["EX11-021", "CARRIER"]);
    runner.place_on_field(0, "SACRIFICE", Some(0));
    runner.place_on_field(0, "BYSTANDER", Some(0));
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();

    let board_before = runner.battle_area_size(0);
    let trash_before = runner.trash_size(0);
    assert_eq!(board_before, 3, "carrier + sacrifice + bystander on board");

    runner.attack_player(attacker, 0, false);

    let view = runner
        .pending_selection_view()
        .expect("cost selection installs");
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both non-carrier Digimon are eligible deletion targets"
    );
    // Pick the first eligible target as the paid cost.
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pay the deletion cost");
    runner.auto_resolve().expect("finish attack cancel");

    assert_eq!(
        runner.battle_area_size(0),
        board_before - 1,
        "exactly one Digimon left the battle area as the cost"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "the deleted Digimon went to the trash"
    );
    assert!(
        card_ids_on_field(&runner, 0).contains(&"CARRIER".to_string()),
        "the EX11-021 carrier itself was never the deletion target"
    );
    // Exactly one of the two non-carrier Digimon remains.
    let survivors: Vec<String> = card_ids_on_field(&runner, 0)
        .into_iter()
        .filter(|id| id == "SACRIFICE" || id == "BYSTANDER")
        .collect();
    assert_eq!(
        survivors.len(),
        1,
        "one of the two other Digimon was paid as the cost, the other survives"
    );
    assert!(
        runner.game.pending_attack.is_none(),
        "the attack ended after the cost was paid"
    );
}
