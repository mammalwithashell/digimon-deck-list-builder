//! A scenario game must start from a LEGAL OPENING POSITION.
//!
//! `ScenarioAdapter` builds its game with `Game::new_with_ordered_decks`, which
//! is the *replay* constructor. That constructor deals starting hands but
//! deliberately does NOT lay security (`game/setup.rs`: "Security is
//! deliberately NOT laid here — it waits until mulligan finalizes"). The
//! recording adapters make up the difference in `relay_initial_state`, which
//! overwrites the zones from a recorded post-mulligan snapshot.
//!
//! A scenario has no such snapshot, so if nothing else lays security the exam
//! would run every line against a board with an empty security stack — a
//! position that cannot occur in a real game. Every attack would immediately
//! win, and the resulting "divergence" against DCGO would be an artifact of our
//! own setup rather than a finding about the card.
//!
//! That failure would be SILENT: the line still lowers, the game still steps,
//! and the run still reports clean. Hence this test.

use dcgo_harness::exam::projection::StateProjection;
use dcgo_harness::exam::scenario::Scenario;
use dcgo_harness::exam::ScenarioAdapter;

const LINE: &str = r#"
card: ST1-08
clause: ST1-08#effect#0
seed: 424242
decks:
  p0: { stack: [], rest: starter_st1_gaia_red }
  p1: { stack: [], rest: starter_st1_gaia_red }
steps:
  - actor: 0
    do: { pass: {} }
"#;

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repo root")
}

/// Build the scenario's opening game and project it, exactly as the exam does.
fn opening_projection() -> StateProjection {
    let root = repo_root();
    let cards = std::fs::read_to_string(root.join("data/cards.json")).expect("cards.json");
    let card_data = digimon_engine::CardData::load_from_str(&cards).expect("parse cards.json");

    let pool = std::fs::read_to_string(root.join("data/starter_decks.json")).expect("starter decks");
    let pool: serde_json::Value = serde_json::from_str(&pool).expect("parse starter decks");
    let deck = pool["starter_decks"]
        .as_array()
        .expect("starter_decks array")
        .iter()
        .find(|d| d["id"] == "starter_st1_gaia_red")
        .expect("gaia red present");
    let main: Vec<String> = deck["main_deck"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    let scenario = Scenario::from_yaml(LINE).expect("golden line parses");
    let adapter = ScenarioAdapter::from_scenario(&scenario, main.clone(), main, &card_data)
        .expect("adapter builds");

    let game = digimon_engine::runners::replay::RecordingSource::build_initial_game(
        &adapter, &card_data,
    )
    .expect("initial game builds");

    StateProjection::from_game(&game, 0)
}

#[test]
fn a_scenario_game_deals_both_players_an_opening_hand() {
    let p = opening_projection();
    assert!(
        !p.p0.hand.is_empty(),
        "p0 opened with an EMPTY HAND -- every scenario would be unable to play anything"
    );
    assert!(!p.p1.hand.is_empty(), "p1 opened with an empty hand");
}

#[test]
fn a_scenario_game_lays_security_for_both_players() {
    let p = opening_projection();
    assert!(
        p.p0.security > 0,
        "p0 opened with an EMPTY SECURITY STACK. That position cannot occur in a real \
         game: the first attack wins outright. Every exam run would then diverge from \
         DCGO for a reason caused by our own setup, not by the card under test. \
         ScenarioAdapter must lay security (Game::new_with_ordered_decks does not -- see \
         game/setup.rs)."
    );
    assert!(p.p1.security > 0, "p1 opened with an empty security stack");
}

#[test]
fn both_players_open_symmetrically() {
    // Same deck both seats, so anything asymmetric at step 0 is a setup bug.
    let p = opening_projection();
    assert_eq!(
        p.p0.hand.len(),
        p.p1.hand.len(),
        "asymmetric opening hands from identical decks"
    );
    assert_eq!(
        p.p0.security, p.p1.security,
        "asymmetric security from identical decks"
    );
}
