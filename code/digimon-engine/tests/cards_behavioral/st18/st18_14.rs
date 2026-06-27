//! ST18-14 Shoto Kazama — Track D attack-target redirect fixture.

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::selection::SelectionKind;

const YAML: &str = include_str!("../../../cards/st18/ST18-14.yaml");

fn compiled_st18_14() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("ST18-14.yaml parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("ST18-14.yaml compiles");
    registry
        .lookup("ST18-14")
        .expect("ST18-14 in registry")
        .clone()
}

fn shoto_runner() -> DebugRunner {
    let mut attacker = make_test_card("ATK", "Attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut original = make_test_card("DEF-A", "Original Defender");
    original.card_kind = CardKind::Digimon;
    original.level = Some(4);
    original.dp = Some(1000);

    let mut redirect = make_test_card("DEF-B", "Redirect Defender");
    redirect.card_kind = CardKind::Digimon;
    redirect.level = Some(4);
    redirect.dp = Some(1000);

    let mut security = make_test_card("SEC", "Security");
    security.card_kind = CardKind::Digimon;
    security.dp = Some(1000);

    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("ST18-14 YAML loads")
        .add_card(attacker)
        .add_card(original)
        .add_card(redirect)
        .add_card(security)
        .security(1, &["SEC"])
        .memory(2)
        .start()
}

#[test]
fn st18_14_redirect_clause_contains_prompt_target_selection() {
    let compiled = compiled_st18_14();
    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAllyAttack) => {
                Some(t)
            }
            _ => None,
        })
        .expect("ST18-14 must have an on_ally_attack clause");

    assert!(
        clause.process.iter().any(|step| matches!(
            step,
            CompiledStep::RedirectAttackTarget {
                new_target: None,
                player: None,
                optional: true,
                ..
            }
        )),
        "Shoto's target-change branch must open a redirect target prompt"
    );
}

#[test]
fn st18_14_suspends_to_redirect_attack_to_player() {
    let mut runner = shoto_runner();
    runner.game.turn_count = 1;
    let shoto = runner.place_on_field(0, "ST18-14", Some(0));
    let attacker = runner.place_on_field(0, "ATK", Some(0));
    let original = runner.place_on_field(1, "DEF-A", Some(0));
    let redirect = runner.place_on_field(1, "DEF-B", Some(0));
    let security_before = runner.security_count(1);

    let _ = runner.attack_digimon(attacker, original, false);

    // The [Your Turn] clause is optional ("by suspending this Tamer, you may
    // change the attack target") and its body's first step is a mandatory
    // suspend, so an outer accept/decline prompt installs first
    // (G-OUTER-OPTIONAL-NOT-INSTALLED). Accept it.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "an outer accept/decline prompt must install for the optional clause"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the outer optional-trigger prompt");

    let pending = runner
        .pending_selection_view()
        .expect("Shoto should offer a redirect target prompt during the attack");
    assert_eq!(pending.kind, SelectionKind::Target);
    assert!(pending.is_optional, "printed target change is optional");
    assert!(
        build_action_mask(&runner.game, 0)[PASS as usize] > 0.0,
        "optional redirect prompt must expose PASS"
    );

    let original_action = encode_attack(attacker.index as u16, original.index as u16);
    let redirect_action = encode_attack(attacker.index as u16, redirect.index as u16);
    let player_action = encode_attack(attacker.index as u16, SECURITY_TARGET);
    assert!(
        !pending.valid_action_ids.contains(&original_action),
        "the current target must not be offered as 'another' target"
    );
    assert!(
        pending.valid_action_ids.contains(&redirect_action),
        "another opponent Digimon must be a legal redirect target"
    );
    assert!(
        pending.valid_action_ids.contains(&player_action),
        "the opponent player must be a legal redirect target"
    );

    runner
        .game
        .resolve_selection(pending.selecting_player, player_action)
        .expect("player redirect resolves");

    assert!(
        runner.game.players[0].battle_area[shoto.index as usize].is_suspended,
        "Shoto should pay the printed suspend cost"
    );
    assert_eq!(
        runner.security_count(1),
        security_before - 1,
        "redirecting to the player should continue into the normal security flow"
    );
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        2,
        "redirecting to the player should not battle either Digimon target"
    );
}

/// CLONE-FAITHFULNESS (make-engine-cloneable): the attack-target redirect prompt
/// is driven by the resumable data VM, so a `Game::clone` taken AT the prompt —
/// mid-attack, with `pending_attack` live — is faithful. Resolving the clone's
/// redirect-to-player continues into the security check (−1 opponent security)
/// without touching the original; the original then replays identically. Before
/// the flip the redirect prompt was closure-only, so the clone would hit the
/// panic-stub `PendingSelection::clone`.
#[test]
fn st18_14_redirect_target_clones_faithfully_at_the_prompt() {
    let mut runner = shoto_runner();
    runner.game.turn_count = 1;
    let shoto = runner.place_on_field(0, "ST18-14", Some(0));
    let attacker = runner.place_on_field(0, "ATK", Some(0));
    let original = runner.place_on_field(1, "DEF-A", Some(0));
    let _redirect = runner.place_on_field(1, "DEF-B", Some(0));
    let security_before = runner.game.players[1].security.len();

    let _ = runner.attack_digimon(attacker, original, false);

    // Accept the outer optional-trigger prompt (a separate closure, not this flip).
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Replacement));
    runner
        .accept_optional_trigger()
        .expect("accept the outer optional-trigger prompt");

    // The redirect-target prompt — resume-driven after the flip (clone-safe).
    let pending = runner
        .pending_selection_view()
        .expect("redirect target prompt");
    assert_eq!(pending.kind, SelectionKind::Target);
    assert!(
        runner.game.pending_selection_resume.is_some(),
        "redirect-target prompt must be resume-driven (clone-safe)"
    );
    let player_action = encode_attack(attacker.index as u16, SECURITY_TARGET);
    assert!(pending.valid_action_ids.contains(&player_action));

    // Clone, then resolve redirect-to-player on the CLONE only.
    let mut clone = runner.game.clone();
    clone
        .resolve_selection(pending.selecting_player, player_action)
        .expect("clone redirect resolves");
    assert_eq!(
        clone.players[1].security.len(),
        security_before - 1,
        "clone: redirect-to-player continued into the security check (−1 security)"
    );
    assert!(
        clone.players[0].battle_area[shoto.index as usize].is_suspended,
        "clone: Shoto paid the suspend cost"
    );

    // INDEPENDENCE: the original is still at the prompt, security unchanged.
    assert!(
        runner.game.pending_selection.is_some(),
        "original still at the redirect prompt after cloning + resolving the clone"
    );
    assert_eq!(
        runner.game.players[1].security.len(),
        security_before,
        "original opponent security untouched by the clone's resolution"
    );

    // REPLAYS IDENTICALLY.
    runner
        .game
        .resolve_selection(pending.selecting_player, player_action)
        .expect("original redirect resolves");
    assert_eq!(
        runner.game.players[1].security.len(),
        security_before - 1,
        "original reaches the same −1 security as the clone"
    );
}

#[test]
fn st18_14_does_not_prompt_when_attack_already_targets_player() {
    let mut runner = shoto_runner();
    runner.game.turn_count = 1;
    let _shoto = runner.place_on_field(0, "ST18-14", Some(0));
    let attacker = runner.place_on_field(0, "ATK", Some(0));

    let _ = runner.attack_player(attacker, 1, false);

    assert!(
        runner.pending_selection_view().is_none(),
        "Shoto only triggers when an ally attacks an opponent Digimon"
    );
    assert_eq!(
        runner.security_count(1),
        0,
        "direct player attack should resolve normally without Shoto interrupt"
    );
}
