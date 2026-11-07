//! Step a DCGO recording through the engine one recorded step at a time and
//! print, for each step, the recorded action, the engine's pending selection
//! and the board — the cheap stand-in for the engine MCP when hunting a
//! divergence. Usage: `cargo run -p dcgo-replay --example trace -- <rec.jsonl> [from] [to]`.

use std::fs;

use dcgo_replay::{load_card_data, parse_jsonl};
use digimon_engine::live_game::LiveGame;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("usage: trace <recording.jsonl> [from] [to]");
    let from: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    let to: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(u32::MAX);
    let text = fs::read_to_string(path).expect("read recording");
    let text = text.trim_start_matches('\u{feff}');
    let recording = parse_jsonl(text).expect("parse recording");
    let card_data = load_card_data(None).expect("cards.json");
    let mut game = LiveGame::from_dcgo_recording(recording, &card_data).expect("build game");
    let stack = std::thread::Builder::new();
    let _ = stack;
    loop {
        let view = match game.replay_step_view() {
            Ok(v) => v,
            Err(e) => {
                println!("view error: {e:?}");
                break;
            }
        };
        if view.is_complete || view.paused {
            println!("STOP cursor={} complete={} paused={} divergences={:#?}", view.cursor, view.is_complete, view.paused, view.divergences);
            break;
        }
        if view.cursor >= from && view.cursor <= to {
            println!("--- cursor {} recorded={:?}", view.cursor, view.recorded);
            if let Some(p) = game.pending_selection() {
                println!("    pending: {p:?}");
            }
            println!("    in play: {:?}", view.card_ids_in_play);
        }
        if view.cursor > to {
            break;
        }
        if let Err(e) = game.step_forward() {
            println!("step_forward error at cursor {}: {e:?}", view.cursor);
            break;
        }
    }
}
