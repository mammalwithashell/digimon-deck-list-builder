//! Regenerates the `closed_cloud` decision-prefix fixtures when an engine rules
//! fix makes a recorded decision illegal.
//!
//! Each fixture is `{source, game_seed, cards, actions}`: a seeded mirror game
//! and the decisions that lead to one mid-game state. For an entry that no
//! longer replays, the generator keeps the seed, the decks and the still-legal
//! prefix, then plays on with a seeded random policy (never conceding) until
//! the state is the kind the test needs and the test's own checks pass:
//!
//! - `security-observer-contexts.json`: a security-removal observer
//!   (`OnOwnSecurityRemoved` / `OnOpponentSecurityRemoved`) is in the effect
//!   queue.
//! - `link-continuations.json`: a parked selection continuation
//!   (`pending_selection_resume`) whose frames come from a link effect.
//!
//! Entries that still replay are left untouched. Run (writes the files):
//!
//! ```text
//! DIGIMON_REGEN_FIXTURES=1 RUST_MIN_STACK=268435456 cargo test \
//!   --manifest-path code/digimon-engine/Cargo.toml --test determinization \
//!   fixture_regen -- --ignored --nocapture
//! ```
//!
//! Without the variable it only reports what it would change.
use super::closed_cloud::{check_closed_fixture, replay_fixture};
use super::support::*;
use digimon_engine::enums::EffectTiming;
use digimon_engine::Game;

const FIXTURE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/determinization/fixtures"
);

/// Replay the longest legal prefix of the recorded decisions.
fn legal_prefix(fixture: &serde_json::Value) -> (Game, Vec<(u8, u16)>) {
    let cards: Vec<String> = serde_json::from_value(fixture["cards"].clone()).unwrap();
    let refs: Vec<_> = cards.iter().map(String::as_str).collect();
    let mut root = new_game_with(fixture["game_seed"].as_u64().unwrap(), &refs, &refs);
    root.enable_belief_knowledge().unwrap();
    root.turn_count = 1;
    let mut kept = Vec::new();
    for step in fixture["actions"].as_array().unwrap() {
        let p = step[0].as_u64().unwrap() as u8;
        let a = step[1].as_u64().unwrap() as u16;
        if decision_player(&root) != p || digimon_engine::build_action_mask(&root, p)[a as usize] <= 0.5
        {
            break;
        }
        root.decode_action(a, p);
        kept.push((p, a));
    }
    (root, kept)
}

/// A queued security-removal observer whose event card (the removed security
/// card) now sits in the hand of the viewer's opponent, i.e. a private card
/// the viewer's closed world must not depend on.
fn security_observer_queued(game: &Game) -> bool {
    let viewer = decision_player(game);
    let opponent_hand = &game.players[(1 - viewer) as usize].hand;
    game.effect_queue.iter().any(|q| {
        matches!(
            q.timing,
            EffectTiming::OnOwnSecurityRemoved | EffectTiming::OnOpponentSecurityRemoved
        ) && q
            .trigger_context
            .as_ref()
            .and_then(|t| t.event_card.read())
            .is_some_and(|h| opponent_hand.iter().any(|c| c.handle() == h))
    })
}

fn link_continuation_parked(game: &Game) -> bool {
    game.pending_selection.is_some()
        && game
            .pending_selection_resume
            .as_ref()
            .is_some_and(|frames| format!("{frames:?}").contains("Link"))
}

fn passes_checks(game: &Game, cards: &[String], label: &str) -> bool {
    let game = game.clone();
    let cards = cards.to_vec();
    let label = label.to_string();
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        check_closed_fixture(&game, cards, &label)
    }))
    .is_ok()
}

/// Continue from the legal prefix until `wanted` holds and the checks pass.
fn regenerate(
    fixture: &serde_json::Value,
    wanted: fn(&Game) -> bool,
    label: &str,
) -> Option<Vec<(u8, u16)>> {
    let cards: Vec<String> = serde_json::from_value(fixture["cards"].clone()).unwrap();
    let (prefix_game, prefix) = legal_prefix(fixture);
    let seed = fixture["game_seed"].as_u64().unwrap();
    for attempt in 0..64u64 {
        let mut game = prefix_game.clone();
        let mut actions = prefix.clone();
        let mut lcg = Lcg(seed ^ (attempt.wrapping_mul(0x9E37_79B9_7F4A_7C15) + 1));
        for _ in 0..600 {
            if game.game_over {
                break;
            }
            if wanted(&game) && passes_checks(&game, &cards, label) {
                return Some(actions);
            }
            let p = decision_player(&game);
            let mut mask = digimon_engine::build_action_mask(&game, p);
            mask[digimon_engine::action::CONCEDE_GAME as usize] = 0.0;
            let Some(a) = lcg.pick(&mask) else { break };
            game.decode_action(a, p);
            actions.push((p, a));
        }
    }
    None
}

fn regenerate_file(name: &str, wanted: fn(&Game) -> bool, label: &str, write: bool) {
    let path = format!("{FIXTURE_DIR}/{name}");
    let mut fixtures: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let mut changed = false;
    for fixture in fixtures.as_array_mut().unwrap() {
        let recorded = fixture["actions"].as_array().unwrap().len();
        let (game, prefix) = legal_prefix(fixture);
        let cards: Vec<String> = serde_json::from_value(fixture["cards"].clone()).unwrap();
        if prefix.len() == recorded && passes_checks(&game, &cards, label) {
            eprintln!(
                "{name} {}: still valid (target state: {})",
                fixture["source"],
                wanted(&game)
            );
            continue;
        }
        let actions = regenerate(fixture, wanted, label)
            .unwrap_or_else(|| panic!("{name} {}: no qualifying state found", fixture["source"]));
        eprintln!(
            "{name} {}: kept {} of {recorded} recorded decisions, {} total",
            fixture["source"],
            prefix.len(),
            actions.len()
        );
        fixture["actions"] = serde_json::json!(actions
            .iter()
            .map(|(p, a)| serde_json::json!([p, a]))
            .collect::<Vec<_>>());
        let source = fixture["source"].as_str().unwrap().to_string();
        if !source.contains("regenerated") {
            fixture["source"] = serde_json::json!(format!(
                "{source} (regenerated 2026-11-11 after the CR 11-1-2 / DUAL / play-gate fixes)"
            ));
        }
        // The regenerated prefix must replay through the strict path.
        let root = replay_fixture(fixture);
        assert!(wanted(&root));
        changed = true;
    }
    if changed && write {
        std::fs::write(&path, serde_json::to_string(&fixtures).unwrap() + "\n").unwrap();
        eprintln!("wrote {path}");
    }
}

#[test]
#[ignore = "fixture generator; run explicitly with DIGIMON_REGEN_FIXTURES=1 to write"]
fn regenerate_closed_cloud_fixtures() {
    with_big_stack(|| {
        let write = std::env::var_os("DIGIMON_REGEN_FIXTURES").is_some();
        regenerate_file(
            "security-observer-contexts.json",
            security_observer_queued,
            "security-observer-regression",
            write,
        );
        regenerate_file(
            "link-continuations.json",
            link_continuation_parked,
            "link-continuation-regression",
            write,
        );
    });
}
