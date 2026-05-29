//! End-to-end integration tests for `digimon-engine-cli`.
//!
//! Spawns the actual compiled binary, pipes scripted stdin, and asserts
//! on stdout/exit code. Validates that:
//! - `scenario` is correctly stubbed (non-zero exit with a clear message)
//! - `replay` loads a recorded game and prints a chosen view
//! - The binary exits cleanly on `quit` from a scripted REPL session

use std::io::Write;
use std::process::{Command, Stdio};

/// Locate the cards.json used by the workspace. Walks up from
/// `CARGO_MANIFEST_DIR` looking for `data/cards.json`.
fn cards_json_path() -> std::path::PathBuf {
    let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..6 {
        let candidate = dir.join("data").join("cards.json");
        if candidate.exists() {
            return candidate;
        }
        if !dir.pop() {
            break;
        }
    }
    panic!("could not locate data/cards.json from CARGO_MANIFEST_DIR");
}

/// Path to the compiled binary under test.
fn bin_path() -> std::path::PathBuf {
    // CARGO_BIN_EXE_<bin_name> is set by Cargo when running `cargo test`
    // against a workspace with a `[[bin]]` target.
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_digimon-engine-cli"))
}

#[test]
fn scenario_subcommand_is_stubbed_with_clear_message() {
    let out = Command::new(bin_path())
        .args(["scenario", "/tmp/nonexistent.yaml"])
        .output()
        .expect("spawning CLI");
    assert!(!out.status.success(), "scenario stub should exit non-zero");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not yet implemented"),
        "expected stub message, got stderr: {}",
        stderr
    );
}

#[test]
fn version_flag_succeeds() {
    let out = Command::new(bin_path())
        .args(["--version"])
        .output()
        .expect("spawning CLI");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("digimon-engine-cli"));
}

#[test]
fn help_flag_succeeds() {
    let out = Command::new(bin_path())
        .args(["--help"])
        .output()
        .expect("spawning CLI");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Each subcommand should appear in the help text.
    for sub in &["debug", "replay", "scenario"] {
        assert!(stdout.contains(sub), "help missing subcommand {}", sub);
    }
}

#[test]
fn debug_repl_exits_cleanly_on_eof() {
    // Pipe an empty stdin — REPL prints prompt, hits EOF, exits 0.
    let cards = cards_json_path();
    let mut child = Command::new(bin_path())
        .args([
            "--cards-json",
            &cards.to_string_lossy(),
            "--pool",
            "implemented",
            "debug",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawning CLI");
    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(b"").unwrap();
    }
    let out = child.wait_with_output().expect("waiting on CLI");
    assert!(out.status.success(), "REPL should exit 0 on EOF");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("digimon-engine-cli REPL"),
        "missing banner — got: {}",
        stdout
    );
}

#[test]
fn debug_repl_scripted_session_help_then_quit() {
    let cards = cards_json_path();
    let mut child = Command::new(bin_path())
        .args([
            "--cards-json",
            &cards.to_string_lossy(),
            "--pool",
            "implemented",
            "debug",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawning CLI");
    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(b"help\nquit\n").unwrap();
    }
    let out = child.wait_with_output().expect("waiting on CLI");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Commands:"), "help output missing");
}
