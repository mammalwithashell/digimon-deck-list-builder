//! Local deck storage for the desktop build. Decks are per-app JSON files
//! under `app_data_dir()/decks/<deck_id>.json`. Listing scans the dir.
//!
//! Shapes mirror `frontend/src/api/deckApi.ts::DeckResponse` so the TS
//! side can treat them identically to server-returned decks.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
    pub id: String,
    pub owner_id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub game_mode: String,
    pub main_deck: Vec<String>,
    pub egg_deck: Vec<String>,
    #[serde(default)]
    pub main_deck_alt_arts: Vec<bool>,
    #[serde(default)]
    pub egg_deck_alt_arts: Vec<bool>,
    #[serde(default)]
    pub commander_id: Option<String>,
    #[serde(default)]
    pub is_valid: bool,
    #[serde(default)]
    pub validation_errors: Vec<String>,
    #[serde(default)]
    pub is_public: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub meta_tier: Option<String>,
    #[serde(default)]
    pub meta_archetype: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckSummary {
    pub id: String,
    pub name: String,
    pub game_mode: String,
    pub is_valid: bool,
    pub is_public: bool,
    pub card_count: usize,
    #[serde(default)]
    pub meta_tier: Option<String>,
    #[serde(default)]
    pub meta_archetype: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn decks_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app_data_dir unavailable: {e}"))?;
    let dir = base.join("decks");
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("create decks dir: {e}"))?;
    }
    Ok(dir)
}

fn read_deck_file(path: &Path) -> Option<Deck> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[tauri::command]
pub fn decks_list(app: AppHandle) -> Result<Vec<DeckSummary>, String> {
    let dir = decks_dir(&app)?;
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("read decks dir: {e}"))? {
        let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
        // Only `.json` is considered — this also hides `.json.tmp` files left
        // behind by a crashed `decks_put` (their extension is `tmp`, not `json`).
        if entry.path().extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        match read_deck_file(&entry.path()) {
            Some(deck) => {
                out.push(DeckSummary {
                    id: deck.id,
                    name: deck.name,
                    game_mode: deck.game_mode,
                    is_valid: deck.is_valid,
                    is_public: deck.is_public,
                    card_count: deck.main_deck.len() + deck.egg_deck.len(),
                    meta_tier: deck.meta_tier,
                    meta_archetype: deck.meta_archetype,
                    created_at: deck.created_at,
                    updated_at: deck.updated_at,
                });
            }
            None => {
                eprintln!(
                    "deck_storage: skipping unreadable deck file: {}",
                    entry.path().display()
                );
            }
        }
    }
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
}

#[tauri::command]
pub fn decks_get(app: AppHandle, deck_id: String) -> Result<Deck, String> {
    let path = decks_dir(&app)?.join(format!("{deck_id}.json"));
    read_deck_file(&path).ok_or_else(|| format!("deck not found: {deck_id}"))
}

#[tauri::command]
pub fn decks_put(app: AppHandle, deck: Deck) -> Result<Deck, String> {
    let dir = decks_dir(&app)?;
    // Assign an ID for new decks.
    let mut deck = deck;
    if deck.id.is_empty() {
        deck.id = Uuid::new_v4().to_string();
    }
    let now = chrono::Utc::now().to_rfc3339();
    if deck.created_at.is_empty() {
        deck.created_at = now.clone();
    }
    deck.updated_at = now;
    let path = dir.join(format!("{}.json", deck.id));
    let tmp_path = dir.join(format!("{}.json.tmp", deck.id));
    let json = serde_json::to_vec_pretty(&deck).map_err(|e| format!("serialize deck: {e}"))?;
    // Crash-atomic write: write the full body to a sibling `.tmp` then
    // rename into place. `rename` is atomic on POSIX and on Windows when
    // source + dest sit on the same volume (same directory here). Matches
    // the pattern in `models.rs::download`.
    fs::write(&tmp_path, json).map_err(|e| format!("write deck: {e}"))?;
    fs::rename(&tmp_path, &path).map_err(|e| format!("rename deck: {e}"))?;
    Ok(deck)
}

#[tauri::command]
pub fn decks_delete(app: AppHandle, deck_id: String) -> Result<bool, String> {
    let path = decks_dir(&app)?.join(format!("{deck_id}.json"));
    match fs::remove_file(&path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("delete deck: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Mini harness: implement the IO against an explicit dir so we don't need a Tauri AppHandle.
    fn write_deck(dir: &Path, deck: &Deck) {
        let path = dir.join(format!("{}.json", deck.id));
        fs::write(path, serde_json::to_vec(deck).unwrap()).unwrap();
    }

    fn sample_deck(id: &str) -> Deck {
        Deck {
            id: id.into(),
            owner_id: "guest_abc".into(),
            name: format!("deck-{id}"),
            description: String::new(),
            game_mode: "standard".into(),
            main_deck: vec!["BT1-001".into(); 50],
            egg_deck: vec!["BT1-002".into(); 5],
            main_deck_alt_arts: vec![],
            egg_deck_alt_arts: vec![],
            commander_id: None,
            is_valid: true,
            validation_errors: vec![],
            is_public: false,
            tags: vec![],
            meta_tier: None,
            meta_archetype: None,
            created_at: "2026-04-18T00:00:00Z".into(),
            updated_at: "2026-04-18T00:00:00Z".into(),
        }
    }

    #[test]
    fn list_returns_empty_when_no_decks() {
        let tmp = TempDir::new().unwrap();
        let entries: Vec<_> = fs::read_dir(tmp.path()).unwrap().collect();
        assert!(entries.is_empty());
    }

    #[test]
    fn round_trip_single_deck() {
        let tmp = TempDir::new().unwrap();
        let deck = sample_deck("d1");
        write_deck(tmp.path(), &deck);
        let back = read_deck_file(&tmp.path().join("d1.json")).unwrap();
        assert_eq!(back.id, "d1");
        assert_eq!(back.main_deck.len(), 50);
    }

    #[test]
    fn malformed_json_returns_none_not_panic() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("broken.json"), b"{not json").unwrap();
        assert!(read_deck_file(&tmp.path().join("broken.json")).is_none());
    }

    #[test]
    fn write_is_crash_atomic_no_tmp_left_behind() {
        let tmp = TempDir::new().unwrap();
        let deck = sample_deck("d1");
        // Simulate a successful put by writing to a .tmp and renaming.
        let tmp_path = tmp.path().join("d1.json.tmp");
        let final_path = tmp.path().join("d1.json");
        fs::write(&tmp_path, serde_json::to_vec(&deck).unwrap()).unwrap();
        fs::rename(&tmp_path, &final_path).unwrap();
        assert!(final_path.exists());
        assert!(
            !tmp_path.exists(),
            "no temp file should linger after rename"
        );
    }

    #[test]
    fn listing_ignores_tmp_files() {
        // Leftover .tmp files from a crashed write should not appear in the
        // listing and should not try to parse as JSON.
        let tmp = TempDir::new().unwrap();
        let good = sample_deck("good");
        write_deck(tmp.path(), &good);
        fs::write(tmp.path().join("partial.json.tmp"), b"{incomplete").unwrap();

        let jsons: Vec<_> = fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
            .collect();
        assert_eq!(jsons.len(), 1);
        assert_eq!(jsons[0].path().file_name().unwrap(), "good.json");
    }
}
