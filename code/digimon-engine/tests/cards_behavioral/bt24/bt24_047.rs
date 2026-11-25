//! BT24-047 Kokatorimon — Track D result-bound may-attack branch.

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::action::space::{decode_attack, encode_attack, SECURITY_TARGET};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const YAML: &str = include_str!("../../../cards/bt24/BT24-047.yaml");

fn compiled_bt24_047() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("BT24-047.yaml parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("BT24-047.yaml compiles");
    registry
        .lookup("BT24-047")
        .expect("BT24-047 in registry")
        .clone()
}

fn bird_card(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, "Bird Ally");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(6000);
    card.traits = vec!["Bird".to_string()];
    card
}

fn plain_card(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, "Plain Digimon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(3000);
    card.traits = vec!["Plain".to_string()];
    card
}

fn runner() -> DebugRunner {
    let mut security = make_test_card("SEC", "Security");
    security.card_kind = CardKind::Digimon;
    security.dp = Some(1000);

    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-047 YAML loads")
        .add_card(bird_card("BIRD"))
        .add_card(plain_card("OPP"))
        .add_card(security)
        .security(1, &["SEC"])
        .memory(10)
        .start()
}

fn fire_on_play(runner: &mut DebugRunner, source: digimon_engine::PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

#[test]
fn bt24_047_clause_gates_the_tail_on_an_actual_suspend_of_your_digimon() {
    let compiled = compiled_bt24_047();
    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT24-047 must have shared On Play/When Digivolving clause");

    let has_bound_branch = clause.process.iter().any(|step| match step {
        CompiledStep::If {
            condition, then, ..
        } => {
            // "If this effect suspended your Digimon": an actual suspend of
            // your Digimon, not merely choosing one (Q845).
            condition.effect_suspended_any_own_digimon == Some(true) && then
                .iter()
                .any(|tail| matches!(tail, CompiledStep::MayAttackNow { optional: true, .. }))
        }
        _ => false,
    });

    assert!(
        has_bound_branch,
        "the unsuspend/may-attack tail must be gated on this effect having suspended your Digimon"
    );
}

#[test]
fn bt24_047_suspending_own_bird_unsuspends_it_and_opens_may_attack() {
    let mut runner = runner();
    let source = runner.place_on_field(0, "BT24-047", Some(0));
    let bird = runner.place_on_field(0, "BIRD", Some(0));
    runner.game.turn_count = 1;
    let security_before = runner.security_count(1);

    fire_on_play(&mut runner, source);

    let suspend_prompt = runner
        .pending_selection_view()
        .expect("Kokatorimon should offer a Digimon to suspend");
    assert_eq!(suspend_prompt.kind, SelectionKind::AnyField);
    let bird_suspend_action = encode_attack(bird.player as u16, bird.index as u16);
    assert!(
        suspend_prompt
            .valid_action_ids
            .contains(&bird_suspend_action),
        "own unsuspended Bird should be a legal suspend choice"
    );
    runner
        .game
        .resolve_selection(suspend_prompt.selecting_player, bird_suspend_action)
        .expect("suspend choice resolves");

    let unsuspend_prompt = runner
        .pending_selection_view()
        .expect("suspending your Digimon should choose an eligible Digimon to unsuspend");
    assert_eq!(unsuspend_prompt.kind, SelectionKind::OwnField);
    let bird_field_action = encode_attack(0, bird.index as u16);
    assert!(
        unsuspend_prompt
            .valid_action_ids
            .contains(&bird_field_action),
        "the just-suspended Bird should be eligible for the result-bound unsuspend"
    );
    runner
        .game
        .resolve_selection(unsuspend_prompt.selecting_player, bird_field_action)
        .expect("unsuspend choice resolves");

    assert!(
        !runner.game.players[0].battle_area[bird.index as usize].is_suspended,
        "the selected Bird should be unsuspended before the follow-up attack prompt"
    );

    let attack_prompt = runner
        .pending_selection_view()
        .expect("the Digimon this effect unsuspended may attack");
    assert!(
        attack_prompt.is_optional,
        "printed 'may attack' must remain declineable"
    );
    let security_attack = attack_prompt
        .valid_action_ids
        .iter()
        .copied()
        .find(|action| decode_attack(*action).1 == SECURITY_TARGET)
        .expect("the normal attack flow should offer the opponent player");
    runner
        .game
        .resolve_selection(attack_prompt.selecting_player, security_attack)
        .expect("follow-up attack resolves");

    assert_eq!(
        runner.security_count(1),
        security_before - 1,
        "the follow-up attack should use the central attack/security flow"
    );
    assert!(
        runner.game.players[0].battle_area[bird.index as usize].is_suspended,
        "the attacker should pay the normal suspend cost after being unsuspended by the effect"
    );
}

/// Q845 / Q1782: an already-suspended Digimon may be chosen, but it is not
/// "suspended by this effect", so the "If this effect suspended your Digimon"
/// tail does not open.
#[test]
fn bt24_047_choosing_your_already_suspended_digimon_does_not_open_the_tail() {
    let mut runner = runner();
    let source = runner.place_on_field(0, "BT24-047", Some(0));
    let bird = runner.place_on_field(0, "BIRD", Some(0));
    runner.game.players[0].battle_area[bird.index as usize].is_suspended = true;
    runner.game.turn_count = 1;

    fire_on_play(&mut runner, source);

    let suspend_prompt = runner
        .pending_selection_view()
        .expect("Kokatorimon should offer a Digimon to suspend");
    let bird_action = encode_attack(bird.player as u16, bird.index as u16);
    assert!(
        suspend_prompt.valid_action_ids.contains(&bird_action),
        "an already-suspended Digimon is a legal choice (Q845)"
    );
    runner
        .game
        .resolve_selection(suspend_prompt.selecting_player, bird_action)
        .expect("suspend choice resolves");

    assert!(
        runner.pending_selection_view().is_none(),
        "choosing an already-suspended Digimon did not suspend it, so no unsuspend/may-attack follows"
    );
    assert!(runner.game.players[0].battle_area[bird.index as usize].is_suspended);
}

#[test]
fn bt24_047_suspending_opponent_digimon_does_not_unlock_own_unsuspend_tail() {
    let mut runner = runner();
    let source = runner.place_on_field(0, "BT24-047", Some(0));
    let bird = runner.place_on_field(0, "BIRD", Some(0));
    let opponent = runner.place_on_field(1, "OPP", Some(0));
    runner.game.players[0].battle_area[bird.index as usize].is_suspended = true;
    runner.game.turn_count = 1;

    fire_on_play(&mut runner, source);

    let suspend_prompt = runner
        .pending_selection_view()
        .expect("Kokatorimon should offer a Digimon to suspend");
    let opponent_action = encode_attack(opponent.player as u16, opponent.index as u16);
    assert!(
        suspend_prompt.valid_action_ids.contains(&opponent_action),
        "opponent Digimon should be a legal target for the initial suspend"
    );
    runner
        .game
        .resolve_selection(suspend_prompt.selecting_player, opponent_action)
        .expect("opponent suspend resolves");

    assert!(
        runner.game.pending_selection.is_none(),
        "suspending an opponent's Digimon must not open your unsuspend/may-attack tail"
    );
    assert!(
        runner.game.players[0].battle_area[bird.index as usize].is_suspended,
        "pre-existing suspended Bird should remain untouched"
    );
    assert!(
        runner.game.players[1].battle_area[opponent.index as usize].is_suspended,
        "opponent Digimon should still be suspended by the first effect"
    );
}

// ─── Colour-gated standard-circle alt path (W1 regression guard) ────────────
//
// The `kind: digivolve` alt path mirroring the printed "Green Lv.3" circle
// used to say `from: { level_eq: 3, color: green }`. `color` is not a
// DSL predicate key, so it was silently dropped and the path accepted a
// level-3 base of ANY colour. Now `color_is` — a wrong-colour base must be
// rejected; a right-colour base still digivolves.

fn bt24_047_color_gate_base(id: &str, color: digimon_engine::enums::CardColor) -> digimon_engine::card_data::CardData {
    let mut c = digimon_engine::debug_runner::make_test_card(id, "Wrongcolormon");
    c.card_kind = digimon_engine::enums::CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.traits = Vec::new();
    c
}

/// Try to digivolve BT24-047 (from hand) onto a level-3 base of `color`.
fn bt24_047_color_gate_try(color: digimon_engine::enums::CardColor) -> bool {
    let mut r = digimon_engine::debug_runner::DebugRunner::builder()
        .dsl_card("BT24-047")
        .expect("BT24-047 in embedded DSL pack")
        .add_card(bt24_047_color_gate_base("CG-BASE", color))
        .add_card(digimon_engine::debug_runner::make_test_card("CG-FILL", "CG-FILL"))
        .hand(0, &["BT24-047"])
        .deck(0, &["CG-FILL"; 5])
        .deck(1, &["CG-FILL"])
        .memory(10)
        .start();
    let base = r.place_on_field(0, "CG-BASE", Some(0));
    let hand_idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "BT24-047")
        .expect("BT24-047 in hand");
    r.game.digivolve_from_hand(
        0,
        hand_idx,
        base.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve,
    )
}

#[test]
fn bt24_047_standard_alt_path_rejects_wrong_color_base() {
    assert!(
        !bt24_047_color_gate_try(digimon_engine::enums::CardColor::Blue),
        "a blue level-3 base must NOT digivolve into BT24-047 via the Green Lv.3 alt path"
    );
}

#[test]
fn bt24_047_standard_alt_path_accepts_right_color_base() {
    assert!(
        bt24_047_color_gate_try(digimon_engine::enums::CardColor::Green),
        "a green level-3 base digivolves into BT24-047 via the Green Lv.3 alt path"
    );
}

/// "[Avian] or [Bird] in any of its traits" is a substring reading (DCGO
/// `HasBirdTraits` -> `ContainsTraits`): a [Giant Bird] / [Holy Bird] Digimon
/// is a legal unsuspend target, a [Plain] one is not.
#[test]
fn bt24_047_unsuspend_matches_bird_as_substring_of_any_trait() {
    for (traits, eligible) in [
        (&["Giant Bird"][..], true),
        (&["Holy Bird"][..], true),
        (&["Avian"][..], true),
        (&["Plain"][..], false),
    ] {
        let mut ally = bird_card("SUBSTR-ALLY");
        ally.traits = traits.iter().map(|t| t.to_string()).collect();
        let mut runner = DebugRunner::builder()
            .from_dsl_yaml(YAML)
            .expect("BT24-047 YAML loads")
            .add_card(ally)
            .add_card(plain_card("OPP"))
            .memory(10)
            .start();
        let source = runner.place_on_field(0, "BT24-047", Some(0));
        let ally = runner.place_on_field(0, "SUBSTR-ALLY", Some(0));
        runner.game.turn_count = 1;

        fire_on_play(&mut runner, source);
        let suspend_prompt = runner
            .pending_selection_view()
            .expect("Kokatorimon should offer a Digimon to suspend");
        let ally_action = encode_attack(ally.player as u16, ally.index as u16);
        assert!(suspend_prompt.valid_action_ids.contains(&ally_action));
        runner
            .game
            .resolve_selection(suspend_prompt.selecting_player, ally_action)
            .expect("suspend the ally");
        // The suspended ally is offered to the unsuspend step iff one of its
        // traits CONTAINS Avian / Bird.
        let offered = runner.pending_selection_view().is_some_and(|v| {
            v.kind == SelectionKind::OwnField
                && v.valid_action_ids
                    .contains(&encode_attack(0, ally.index as u16))
        });
        assert_eq!(offered, eligible, "{traits:?}: unsuspend eligibility");
    }
}

// ─── Inherited [All Turns][Once Per Turn] battle deletion → gain 1 memory ────
//
// Kokatorimon (BT24-047) card face inherited box; DCGO BT24_047.cs
// "All Turns - ESS" (OnEndBattle, inherited, maxCountPerTurn 1).

mod inherited_memory {
    use digimon_engine::debug_runner::{make_test_card, DebugRunner};
    use digimon_engine::enums::CardKind;

    fn digimon(id: &str, dp: i32) -> digimon_engine::CardData {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Digimon;
        c.level = Some(5);
        c.dp = Some(dp);
        c
    }

    fn runner() -> DebugRunner {
        let mut r = DebugRunner::builder()
            .dsl_card("BT24-047")
            .expect("BT24-047 YAML")
            .add_card(digimon("CARRIER", 9000))
            .add_card(digimon("WEAK-A", 1000))
            .add_card(digimon("WEAK-B", 1000))
            .add_card(digimon("STRONG", 15000))
            .add_card(make_test_card("PAD", "Pad"))
            .deck(0, &["PAD"; 8])
            .deck(1, &["PAD"; 8])
            .memory(3)
            .start();
        r.game.turn_count = 1;
        r
    }

    fn index_of(r: &DebugRunner, player: usize, id: &str) -> Option<usize> {
        r.game.players[player]
            .battle_area
            .iter()
            .position(|p| p.top_card().card_id(&r.game.card_data) == id)
    }

    /// The carrier deletes an opponent's Digimon in battle: +1 memory. A second
    /// battle deletion in the same turn gains nothing ([Once Per Turn]).
    #[test]
    fn bt24_047_inherited_battle_deletion_gains_one_memory_once_per_turn() {
        let mut r = runner();
        let carrier = r.place_stack(0, &["BT24-047", "CARRIER"]);
        let a = r.place_on_field(1, "WEAK-A", Some(0));
        r.place_on_field(1, "WEAK-B", Some(0));
        let mem = r.memory();
        r.attack_digimon(carrier, a, false);
        assert!(r.pending_selection().is_none(), "no decision is left");
        assert!(index_of(&r, 1, "WEAK-A").is_none(), "deleted in battle");
        assert_eq!(r.memory(), mem + 1, "gain 1 memory");

        let ci = index_of(&r, 0, "CARRIER").expect("carrier survives");
        r.game.players[0].battle_area[ci].is_suspended = false;
        let carrier = digimon_engine::permanent::PermanentHandle { player: 0, index: ci as u8 };
        let bi = index_of(&r, 1, "WEAK-B").expect("second target");
        let b = digimon_engine::permanent::PermanentHandle { player: 1, index: bi as u8 };
        r.attack_digimon(carrier, b, false);
        assert!(r.pending_selection().is_none(), "no decision is left");
        assert!(index_of(&r, 1, "WEAK-B").is_none(), "second deletion in battle");
        assert_eq!(r.memory(), mem + 1, "[Once Per Turn]: no second gain");
    }

    /// Losing the battle (the carrier is deleted) gains nothing.
    #[test]
    fn bt24_047_inherited_no_memory_when_the_carrier_loses() {
        let mut r = runner();
        let carrier = r.place_stack(0, &["BT24-047", "CARRIER"]);
        let s = r.place_on_field(1, "STRONG", Some(0));
        let mem = r.memory();
        r.attack_digimon(carrier, s, false);
        assert!(r.pending_selection().is_none(), "no decision is left");
        assert!(index_of(&r, 0, "CARRIER").is_none());
        assert_eq!(r.memory(), mem);
    }

    /// It is an inherited effect: Kokatorimon itself on top does not gain.
    #[test]
    fn bt24_047_face_up_kokatorimon_does_not_gain_from_its_inherited_box() {
        let mut r = runner();
        let own = r.place_on_field(0, "BT24-047", Some(0));
        // Kokatorimon has 4000 DP; WEAK-A has 1000.
        let a = r.place_on_field(1, "WEAK-A", Some(0));
        let mem = r.memory();
        r.attack_digimon(own, a, false);
        assert!(r.pending_selection().is_none(), "no decision is left");
        assert!(index_of(&r, 1, "WEAK-A").is_none());
        assert_eq!(r.memory(), mem);
    }
}
