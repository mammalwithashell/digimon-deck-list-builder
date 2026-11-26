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
//! `chronomon-security-history.json` is one long recorded game rather than a
//! list of prefixes, and has its own generator below
//! (`regenerate_chronomon_history_fixture`, same command): it keeps as much of
//! the recorded game as still replays and plays on to the recorded length.
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

/// Why the entries this run regenerates were regenerated. Name the engine change that made each
/// regenerated entry stop replaying; refine the stamp per entry afterwards if they differ. History:
/// 2026-11-11 the CR 11-1-2 / DUAL / play-gate fixes; 2026-11-12 this branch's DCGO-playback fixes
/// (0de06211d, c6eda5678; per-entry reasons are in each regenerated `source`); 2026-11-15 the select_hand
/// playability filters; 2026-11-21 d76249e94 (no DigiXros stop prompt once no candidate remains);
/// 2026-11-23 f3224fb0d (Erlangmon (EX12-034): an accepted leave replacement spends its [Once Per Turn]);
/// 2026-11-26 262228452 (PR #808 live DCGO parity fixes for EX12 triggers/conditions).
const REGEN_NOTE: &str = "regenerated 2026-11-26 after 262228452: PR #808 EX12 trigger/condition parity fixes (EX12-002 outer_prompt, EX12-015 condition, EX12-020/022/025/026 processing-condition hand check, EX12-048 select_materials)";

/// The same for `chronomon-security-history.json`.
const CHRONOMON_REGEN_NOTE: &str = "regenerated 2026-11-21 after the {Trash} liveness gate (CR 4.3 15-4-4-3, bug scan 2026-11): recorded decisions 58 and 159 accepted a Cherubimon BT26-078 trigger whose card had already been returned to the deck; decision 58 is dropped, the 158 decisions before the second one are the original trace, the rest is a seeded continuation";

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
        // Stamp why this entry was regenerated (update REGEN_NOTE with the
        // engine change each time the generator is run), replacing an older stamp.
        let source = fixture["source"].as_str().unwrap();
        let base = source.split(" (regenerated").next().unwrap();
        fixture["source"] = serde_json::json!(format!("{base} ({REGEN_NOTE})"));
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

// --- chronomon-security-history.json ---------------------------------------

/// `REPLACEMENT_ACCEPT`: the "yes" to an optional prompt.
const ACCEPT: u16 = digimon_engine::action::space::REPLACEMENT_ACCEPT;

fn replays(game: &Game, step: &serde_json::Value) -> bool {
    let p = step[0].as_u64().unwrap() as u8;
    let a = step[1].as_u64().unwrap() as u16;
    decision_player(game) == p && digimon_engine::build_action_mask(game, p)[a as usize] > 0.5
}

/// The recorded decisions that still replay. A recorded ACCEPT of a prompt the
/// engine no longer offers is dropped when the recorded game goes on from
/// there (the effect it accepted simply does not happen any more); any other
/// recorded decision that is not legal ends the prefix.
fn chronomon_prefix(fixture: &serde_json::Value) -> (Game, Vec<(u8, u16)>, Vec<usize>) {
    let cards: Vec<String> = serde_json::from_value(fixture["cards"].clone()).unwrap();
    let refs: Vec<_> = cards.iter().map(String::as_str).collect();
    let mut game = new_game_with(fixture["game_seed"].as_u64().unwrap(), &refs, &refs);
    game.enable_belief_knowledge().unwrap();
    game.turn_count = 1;
    let mut kept = Vec::new();
    let mut dropped = Vec::new();
    let recorded = fixture["actions"].as_array().unwrap();
    for (i, step) in recorded.iter().enumerate() {
        if !replays(&game, step) {
            let stale_accept = step[1].as_u64() == Some(ACCEPT as u64)
                && recorded.get(i + 1).is_some_and(|next| replays(&game, next));
            if stale_accept {
                dropped.push(i);
                continue;
            }
            break;
        }
        let p = step[0].as_u64().unwrap() as u8;
        let a = step[1].as_u64().unwrap() as u16;
        game.decode_action(a, p);
        super::closed_cloud::assert_belief_positions_match_zones(
            &game,
            &format!("prefix, after recorded decision {i} {step}"),
        );
        kept.push((p, a));
    }
    (game, kept, dropped)
}

/// One seeded continuation of the prefix to `length` decisions. Returns the
/// decisions and how many positioned belief facts the continuation checked,
/// or `None` when the game ended first or the final state has no closed world.
/// A belief position that does not match its zone is an engine bug, not a
/// reason to try another seed: it panics.
fn chronomon_continuation(
    prefix_game: &Game,
    prefix: &[(u8, u16)],
    cards: &[String],
    length: usize,
    seed: u64,
) -> Option<(Vec<(u8, u16)>, usize)> {
    let mut game = prefix_game.clone();
    let mut actions = prefix.to_vec();
    let mut lcg = Lcg(seed);
    let mut checked = 0;
    while actions.len() < length {
        if game.game_over {
            return None;
        }
        let p = decision_player(&game);
        let mut mask = digimon_engine::build_action_mask(&game, p);
        mask[digimon_engine::action::CONCEDE_GAME as usize] = 0.0;
        let a = lcg.pick(&mask)?;
        game.decode_action(a, p);
        actions.push((p, a));
        checked += super::closed_cloud::assert_belief_positions_match_zones(
            &game,
            &format!(
                "continuation seed {seed}, after decision {} [{p},{a}]",
                actions.len() - 1
            ),
        );
    }
    if game.game_over {
        return None;
    }
    match super::closed_cloud::materialize_chronomon_history_world(&game, cards) {
        Ok(_) => Some((actions, checked)),
        Err(e) => {
            eprintln!("  seed {seed}: no closed world at the final state ({e})");
            None
        }
    }
}

/// Keep the recorded game as far as it replays, then play on with a seeded
/// random policy (never conceding) to the recorded length. Of 16 seeds, the
/// continuation that checks the most positioned belief facts is kept: those
/// facts are what the test is about.
#[test]
#[ignore = "fixture generator; run explicitly with DIGIMON_REGEN_FIXTURES=1 to write"]
fn regenerate_chronomon_history_fixture() {
    with_big_stack(|| {
        let write = std::env::var_os("DIGIMON_REGEN_FIXTURES").is_some();
        let path = format!("{FIXTURE_DIR}/chronomon-security-history.json");
        let mut fixture: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let cards: Vec<String> = serde_json::from_value(fixture["cards"].clone()).unwrap();
        let length = fixture["actions"].as_array().unwrap().len();
        let (prefix_game, prefix, dropped) = chronomon_prefix(&fixture);
        if prefix.len() == length {
            eprintln!("chronomon-security-history.json: still valid");
            return;
        }
        eprintln!(
            "chronomon-security-history.json: {} of {length} recorded decisions replay (stale accepts dropped at {dropped:?})",
            prefix.len()
        );
        let seed = fixture["game_seed"].as_u64().unwrap();
        let mut best: Option<(Vec<(u8, u16)>, usize, u64)> = None;
        for attempt in 0..16u64 {
            let attempt_seed = seed ^ (attempt.wrapping_mul(0x9E37_79B9_7F4A_7C15) + 1);
            let Some((actions, checked)) =
                chronomon_continuation(&prefix_game, &prefix, &cards, length, attempt_seed)
            else {
                eprintln!("  attempt {attempt}: no qualifying final state");
                continue;
            };
            eprintln!("  attempt {attempt}: {checked} positioned belief facts checked");
            if best.as_ref().map_or(true, |(_, most, _)| checked > *most) {
                best = Some((actions, checked, attempt));
            }
        }
        let (actions, checked, attempt) =
            best.expect("chronomon-security-history.json: no continuation reached a closed world");
        eprintln!("chronomon-security-history.json: keeping attempt {attempt} ({checked} facts)");
        // The base name is the text before any earlier stamp or remark.
        let source = fixture["source"].as_str().unwrap().to_string();
        let base = source
            .split(" (regenerated")
            .next()
            .unwrap()
            .split(';')
            .next()
            .unwrap();
        fixture["source"] = serde_json::json!(format!("{base} ({CHRONOMON_REGEN_NOTE})"));
        if write {
            // Same layout as the committed file: one card / one decision per line.
            let mut out = String::from("{\n");
            out += &format!("  \"source\": {},\n", fixture["source"]);
            out += &format!("  \"game_seed\": {},\n", fixture["game_seed"]);
            out += "  \"cards\": [\n";
            let rows: Vec<String> = cards.iter().map(|c| format!("    \"{c}\"")).collect();
            out += &rows.join(",\n");
            out += "\n  ],\n  \"actions\": [\n";
            let rows: Vec<String> = actions
                .iter()
                .map(|(p, a)| format!("    [{p}, {a}]"))
                .collect();
            out += &rows.join(",\n");
            out += "\n  ]\n}\n";
            std::fs::write(&path, out).unwrap();
            eprintln!("wrote {path}");
        }
    });
}
