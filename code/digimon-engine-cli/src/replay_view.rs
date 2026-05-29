//! `replay` subcommand — single-shot recording viewer.
//!
//! Loads a recording, seeks to the requested step, prints the chosen
//! view from the chosen perspective. Optional `--verify` mode walks
//! the recording from step 0 to the target and prints any divergences
//! encountered along the way.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use digimon_engine::card_data::CardData;
use digimon_engine::live_game::LiveGame;
use digimon_engine::runners::replay::ReplayRunner;

use crate::{parse_perspective, print_json, read_json_path};

pub fn run(
    recording: PathBuf,
    step: u32,
    view: &str,
    show: &str,
    player: u8,
    verify: bool,
    card_data: HashMap<String, CardData>,
) -> ExitCode {
    let recording_value = match read_json_path(&recording) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(2);
        }
    };

    if verify {
        // Verify mode: walk from step 0 to `step`, accumulate divergences.
        let mut runner = match ReplayRunner::new(recording_value.clone(), &card_data, true) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("error constructing ReplayRunner: {}", e);
                return ExitCode::from(2);
            }
        };
        let target = step.min(runner.total_steps());
        let mut had_divergence = false;
        for _ in 0..target {
            let r = runner.step();
            if !r.divergences.is_empty() {
                had_divergence = true;
                eprintln!("DIVERGENCE at step {}: {:?}", r.step_number, r.divergences);
            }
        }
        emit_view(&LiveGame::from_game(runner.game), view, show, player);
        return if had_divergence {
            ExitCode::from(3)
        } else {
            ExitCode::SUCCESS
        };
    }

    // Non-verify: construct LiveGame directly at the requested step.
    let lg = match LiveGame::from_recording_at_step(recording_value, step, &card_data) {
        Ok(lg) => lg,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(2);
        }
    };
    emit_view(&lg, view, show, player);
    ExitCode::SUCCESS
}

fn emit_view(lg: &LiveGame, view: &str, show: &str, player: u8) {
    let perspective = parse_perspective(view);
    match show {
        "state" => print_json(&serde_json::to_value(lg.state(perspective)).unwrap()),
        "hand" => print_json(&serde_json::to_value(lg.hand(player, perspective)).unwrap()),
        "field" => print_json(&serde_json::to_value(lg.field(player, perspective)).unwrap()),
        "security" => print_json(&serde_json::to_value(lg.security(player, perspective)).unwrap()),
        "pending" => match lg.pending_selection() {
            Some(p) => print_json(&serde_json::to_value(p).unwrap()),
            None => println!("(no pending selection)"),
        },
        "queue" => print_json(&serde_json::to_value(lg.effect_queue()).unwrap()),
        "events" => print_json(&serde_json::to_value(lg.events(None)).unwrap()),
        "actions" => print_json(&serde_json::to_value(lg.legal_actions(player)).unwrap()),
        other => {
            eprintln!(
                "unknown view '{}' — try one of: state, hand, field, security, pending, queue, events, actions",
                other
            );
        }
    }
}
