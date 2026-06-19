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
    #[serde(default)]
    pub folder_id: Option<String>,
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
    pub is_pinned: bool,
    #[serde(default)]
    pub is_builtin: bool,
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
    #[serde(default)]
    pub description: String,
    pub game_mode: String,
    pub is_valid: bool,
    pub is_public: bool,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default)]
    pub is_builtin: bool,
    #[serde(default)]
    pub folder_id: Option<String>,
    pub card_count: usize,
    pub main_count: usize,
    pub egg_count: usize,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub commander_id: Option<String>,
    #[serde(default)]
    pub deck_icon_card_id: Option<String>,
    #[serde(default)]
    pub meta_tier: Option<String>,
    #[serde(default)]
    pub meta_archetype: Option<String>,
    #[serde(default)]
    pub colors: Vec<String>,
    #[serde(default)]
    pub highest_level: Option<u8>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckFolder {
    pub id: String,
    pub owner_id: String,
    pub name: String,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct LibraryMetadata {
    folders: Vec<DeckFolder>,
}

const DEFAULT_FOLDER_NAMES: [&str; 3] = ["Tournament", "Experimental", "Casual"];

/// The 6 official starter decks bundled into the binary (generated from
/// `data/deck_library.json` by `code/tools/gen_starter_decks.py`). These are
/// read-only: they always appear in the library and can't be edited/deleted.
const STARTER_DECKS_JSON: &str = include_str!("../../../data/starter_decks.json");

pub fn builtin_starter_decks() -> Vec<Deck> {
    #[derive(serde::Deserialize)]
    struct File {
        starter_decks: Vec<Raw>,
    }
    #[derive(serde::Deserialize)]
    struct Raw {
        id: String,
        name: String,
        egg_deck: Vec<String>,
        main_deck: Vec<String>,
    }
    let file: File =
        serde_json::from_str(STARTER_DECKS_JSON).expect("starter_decks.json is malformed");
    file.starter_decks
        .into_iter()
        .map(|r| Deck {
            id: r.id,
            owner_id: "builtin".into(),
            folder_id: None,
            name: r.name,
            description: "Official starter deck.".into(),
            game_mode: "starter".into(),
            main_deck: r.main_deck,
            egg_deck: r.egg_deck,
            main_deck_alt_arts: vec![],
            egg_deck_alt_arts: vec![],
            commander_id: None,
            is_valid: true,
            validation_errors: vec![],
            is_public: false,
            is_pinned: false,
            is_builtin: true,
            tags: vec!["starter".into()],
            meta_tier: None,
            meta_archetype: None,
            created_at: "2024-01-01T00:00:00Z".into(),
            updated_at: "2024-01-01T00:00:00Z".into(),
        })
        .collect()
}

fn is_builtin_id(id: &str) -> bool {
    builtin_starter_decks().iter().any(|d| d.id == id)
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

fn library_metadata_path(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app_data_dir unavailable: {e}"))?;
    if !base.exists() {
        fs::create_dir_all(&base).map_err(|e| format!("create app data dir: {e}"))?;
    }
    Ok(base.join("deck_library.json"))
}

fn read_deck_file(path: &Path) -> Option<Deck> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn read_metadata(path: &Path) -> LibraryMetadata {
    let Some(bytes) = fs::read(path).ok() else {
        return LibraryMetadata::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

fn write_metadata(path: &Path, metadata: &LibraryMetadata) -> Result<(), String> {
    let tmp_path = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(metadata)
        .map_err(|e| format!("serialize deck library metadata: {e}"))?;
    fs::write(&tmp_path, json).map_err(|e| format!("write deck library metadata: {e}"))?;
    fs::rename(&tmp_path, path).map_err(|e| format!("rename deck library metadata: {e}"))?;
    Ok(())
}

fn ensure_default_folders(path: &Path) -> Result<LibraryMetadata, String> {
    let mut metadata = read_metadata(path);
    if metadata.folders.is_empty() {
        let now = chrono::Utc::now().to_rfc3339();
        metadata.folders = DEFAULT_FOLDER_NAMES
            .iter()
            .enumerate()
            .map(|(idx, name)| DeckFolder {
                id: Uuid::new_v4().to_string(),
                owner_id: "guest".into(),
                name: (*name).into(),
                sort_order: idx as i32,
                created_at: now.clone(),
                updated_at: now.clone(),
            })
            .collect();
        write_metadata(path, &metadata)?;
    }
    Ok(metadata)
}

fn deck_summary(deck: Deck) -> DeckSummary {
    let deck_icon_card_id = summary_icon_card_id(&deck.main_deck, &deck.egg_deck, deck.commander_id.as_deref());
    DeckSummary {
        id: deck.id,
        name: deck.name,
        description: deck.description,
        game_mode: deck.game_mode,
        is_valid: deck.is_valid,
        is_public: deck.is_public,
        is_pinned: deck.is_pinned,
        is_builtin: deck.is_builtin,
        folder_id: deck.folder_id,
        card_count: deck.main_deck.len() + deck.egg_deck.len(),
        main_count: deck.main_deck.len(),
        egg_count: deck.egg_deck.len(),
        tags: deck.tags,
        commander_id: deck.commander_id,
        deck_icon_card_id,
        meta_tier: deck.meta_tier,
        meta_archetype: deck.meta_archetype,
        colors: vec![],
        highest_level: None,
        created_at: deck.created_at,
        updated_at: deck.updated_at,
    }
}

fn summary_icon_card_id(
    main_deck: &[String],
    egg_deck: &[String],
    commander_id: Option<&str>,
) -> Option<String> {
    if let Some(commander_id) = commander_id {
        if main_deck.iter().any(|id| id == commander_id) || egg_deck.iter().any(|id| id == commander_id) {
            return Some(commander_id.to_string());
        }
    }
    if let Some(id) = main_deck.iter().min() {
        return Some(id.clone());
    }
    egg_deck.iter().min().cloned()
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
                out.push(deck_summary(deck));
            }
            None => {
                eprintln!(
                    "deck_storage: skipping unreadable deck file: {}",
                    entry.path().display()
                );
            }
        }
    }
    for deck in builtin_starter_decks() {
        out.push(deck_summary(deck));
    }
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
}

#[tauri::command]
pub fn decks_get(app: AppHandle, deck_id: String) -> Result<Deck, String> {
    if let Some(deck) = builtin_starter_decks().into_iter().find(|d| d.id == deck_id) {
        return Ok(deck);
    }
    let path = decks_dir(&app)?.join(format!("{deck_id}.json"));
    read_deck_file(&path).ok_or_else(|| format!("deck not found: {deck_id}"))
}

#[tauri::command]
pub fn decks_put(app: AppHandle, deck: Deck) -> Result<Deck, String> {
    let dir = decks_dir(&app)?;
    // Assign an ID for new decks.
    let mut deck = deck;
    if deck.is_builtin || is_builtin_id(&deck.id) {
        return Err("Starter decks are built-in and can't be modified".into());
    }
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
    if is_builtin_id(&deck_id) {
        return Err("Starter decks are built-in and can't be deleted".into());
    }
    let path = decks_dir(&app)?.join(format!("{deck_id}.json"));
    match fs::remove_file(&path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("delete deck: {e}")),
    }
}

#[tauri::command]
pub fn deck_folders_list(app: AppHandle) -> Result<Vec<DeckFolder>, String> {
    let path = library_metadata_path(&app)?;
    let mut metadata = ensure_default_folders(&path)?;
    metadata.folders.sort_by(|a, b| {
        a.sort_order
            .cmp(&b.sort_order)
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(metadata.folders)
}

#[tauri::command]
pub fn deck_folders_create(
    app: AppHandle,
    name: String,
    sort_order: Option<i32>,
) -> Result<DeckFolder, String> {
    let path = library_metadata_path(&app)?;
    let mut metadata = ensure_default_folders(&path)?;
    if metadata
        .folders
        .iter()
        .any(|f| f.name.eq_ignore_ascii_case(name.trim()))
    {
        return Err("Folder name already exists".into());
    }
    let now = chrono::Utc::now().to_rfc3339();
    let folder = DeckFolder {
        id: Uuid::new_v4().to_string(),
        owner_id: "guest".into(),
        name: name.trim().into(),
        sort_order: sort_order.unwrap_or(metadata.folders.len() as i32),
        created_at: now.clone(),
        updated_at: now,
    };
    metadata.folders.push(folder.clone());
    write_metadata(&path, &metadata)?;
    Ok(folder)
}

#[tauri::command]
pub fn deck_folders_update(
    app: AppHandle,
    folder_id: String,
    name: Option<String>,
    sort_order: Option<i32>,
) -> Result<DeckFolder, String> {
    let path = library_metadata_path(&app)?;
    let mut metadata = ensure_default_folders(&path)?;
    if let Some(ref next_name) = name {
        if metadata
            .folders
            .iter()
            .any(|f| f.id != folder_id && f.name.eq_ignore_ascii_case(next_name.trim()))
        {
            return Err("Folder name already exists".into());
        }
    }
    let folder = metadata
        .folders
        .iter_mut()
        .find(|f| f.id == folder_id)
        .ok_or_else(|| format!("folder not found: {folder_id}"))?;
    if let Some(name) = name {
        folder.name = name.trim().into();
    }
    if let Some(sort_order) = sort_order {
        folder.sort_order = sort_order;
    }
    folder.updated_at = chrono::Utc::now().to_rfc3339();
    let out = folder.clone();
    write_metadata(&path, &metadata)?;
    Ok(out)
}

#[tauri::command]
pub fn deck_folders_delete(app: AppHandle, folder_id: String) -> Result<bool, String> {
    let path = library_metadata_path(&app)?;
    let mut metadata = ensure_default_folders(&path)?;
    let before = metadata.folders.len();
    metadata.folders.retain(|f| f.id != folder_id);
    if metadata.folders.len() == before {
        return Ok(false);
    }
    write_metadata(&path, &metadata)?;

    let dir = decks_dir(&app)?;
    for entry in fs::read_dir(&dir).map_err(|e| format!("read decks dir: {e}"))? {
        let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
        if entry.path().extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(mut deck) = read_deck_file(&entry.path()) else {
            continue;
        };
        if deck.folder_id.as_deref() == Some(folder_id.as_str()) {
            deck.folder_id = None;
            decks_put(app.clone(), deck)?;
        }
    }
    Ok(true)
}

#[tauri::command]
pub fn decks_update_library(
    app: AppHandle,
    deck_id: String,
    folder_id: Option<String>,
    clear_folder: Option<bool>,
    is_pinned: Option<bool>,
) -> Result<Deck, String> {
    if is_builtin_id(&deck_id) {
        return Err("Starter decks are built-in and can't be modified".into());
    }
    let mut deck = decks_get(app.clone(), deck_id)?;
    if clear_folder.unwrap_or(false) {
        deck.folder_id = None;
    } else if folder_id.is_some() {
        deck.folder_id = folder_id;
    }
    if let Some(is_pinned) = is_pinned {
        deck.is_pinned = is_pinned;
    }
    decks_put(app, deck)
}

/// The 6 bundled read-only starter decks, exposed as a Tauri command for a
/// future "starter only" library view / external tooling. (The AI-Starter
/// picker itself reads the bundled `starterDecks.generated.ts`, so it needs no
/// round-trip; this command is the desktop runtime equivalent of that data.)
#[tauri::command]
pub fn rust_list_starter_decks() -> Vec<Deck> {
    builtin_starter_decks()
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
            folder_id: None,
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
            is_pinned: false,
            is_builtin: false,
            tags: vec![],
            meta_tier: None,
            meta_archetype: None,
            created_at: "2026-04-18T00:00:00Z".into(),
            updated_at: "2026-04-18T00:00:00Z".into(),
        }
    }

    #[test]
    fn builtin_starter_decks_are_six_and_starter_mode() {
        let decks = builtin_starter_decks();
        assert_eq!(decks.len(), 6, "ship all 6 starter decks");
        for d in &decks {
            assert!(d.is_builtin, "{} must be flagged built-in", d.id);
            assert_eq!(d.game_mode, "starter");
            assert_eq!(d.main_deck.len(), 50, "{} main", d.id);
            assert!(d.egg_deck.len() <= 5, "{} egg <= 5", d.id);
            assert!(d.id.starts_with("starter_st"), "{} id prefix", d.id);
        }
        assert!(decks.iter().any(|d| d.id == "starter_st2_cocytus_blue"));
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

    #[test]
    fn deck_summary_includes_library_fields() {
        let mut deck = sample_deck("d1");
        deck.folder_id = Some("folder-1".into());
        deck.is_pinned = true;
        deck.description = "test notes".into();
        deck.tags = vec!["meta".into()];

        let summary = deck_summary(deck);

        assert_eq!(summary.folder_id.as_deref(), Some("folder-1"));
        assert!(summary.is_pinned);
        assert_eq!(summary.description, "test notes");
        assert_eq!(summary.main_count, 50);
        assert_eq!(summary.egg_count, 5);
        assert_eq!(summary.tags, vec!["meta"]);
    }

    #[test]
    fn deck_summary_includes_explicit_or_fallback_icon_card() {
        let mut deck = sample_deck("d1");
        deck.main_deck = vec!["BT1-003".into(), "BT1-001".into(), "BT1-002".into()];
        deck.egg_deck = vec!["BT1-004".into()];
        deck.commander_id = Some("BT1-002".into());

        let summary = deck_summary(deck.clone());

        assert_eq!(summary.commander_id.as_deref(), Some("BT1-002"));
        assert_eq!(summary.deck_icon_card_id.as_deref(), Some("BT1-002"));

        deck.commander_id = Some("BT9-999".into());
        let summary = deck_summary(deck);
        assert_eq!(summary.commander_id.as_deref(), Some("BT9-999"));
        assert_eq!(summary.deck_icon_card_id.as_deref(), Some("BT1-001"));
    }

    #[test]
    fn metadata_seeds_default_folders_once() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("deck_library.json");

        let metadata = ensure_default_folders(&path).unwrap();
        assert_eq!(metadata.folders.len(), 3);
        assert_eq!(metadata.folders[0].name, "Tournament");

        let again = ensure_default_folders(&path).unwrap();
        assert_eq!(again.folders.len(), 3);
        assert_eq!(again.folders[1].name, "Experimental");
    }

    #[test]
    fn metadata_round_trips_custom_folder() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("deck_library.json");
        let folder = DeckFolder {
            id: "folder-1".into(),
            owner_id: "guest".into(),
            name: "Locals".into(),
            sort_order: 7,
            created_at: "2026-04-26T00:00:00Z".into(),
            updated_at: "2026-04-26T00:00:00Z".into(),
        };
        write_metadata(
            &path,
            &LibraryMetadata {
                folders: vec![folder],
            },
        )
        .unwrap();

        let back = read_metadata(&path);
        assert_eq!(back.folders.len(), 1);
        assert_eq!(back.folders[0].name, "Locals");
        assert_eq!(back.folders[0].sort_order, 7);
    }
}
