//! PyO3 bindings for `digimon-engine`.
//!
//! Exposes:
//! - `RustHeadlessGame` — 1:1 mirror of Python's
//!   `digimon_gym.engine.runners.headless_game.HeadlessGame`. Used by
//!   `DigimonEnv` (RL gym) and the server's game lifecycle.
//! - `CardDatabase` / `PyCard` — static cards.json loader and per-card
//!   metadata wrapper.
//! - `parse_deck`, `parse_tts`, `parse_text`, `summarize_deck`,
//!   `validate_deck`, `out_of_set_cards`, `expand_deck_dict`,
//!   `restricted_list` — deck parsing, validation, and configuration.
//! - `CardKind`, `GamePhase` — engine enums (variant names match Python).
//! - `load_tested_cards`, `is_card_tested` — alpha tested-cards allowlist.
//! - `CardRegistry`, `REGISTRY_CAPACITY`, `EMBEDDING_DIM` — stable
//!   card-index registry for tensor encoding.
//! - `get_models_dir` — ONNX models directory resolver.
//! - `load_implemented_card_ids` — set of card IDs with registered Rust
//!   `CardEffect` implementations.
//!
//! Conventions:
//! - Deck ids are `list[str]`. DigiEggs are auto-routed into each player's
//!   digitama deck inside `Game::new`.
//! - Player ids are 1/2 on the Python side (matching Python engine), 0/1
//!   inside Rust. This layer converts.
//! - Action and tensor arrays are returned as zero-copy numpy `float32`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use numpy::PyArray1;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde_json::Value;

use pyo3::wrap_pyfunction;

use ::digimon_engine::action::space::{
    ACTION_SPACE_SIZE, BREEDING_SOURCE_SELECT_END, BREEDING_SOURCE_SELECT_START, SOURCE_SELECT_END,
    SOURCE_SELECT_START,
};
use ::digimon_engine::card_data::CardData;
use ::digimon_engine::card_registry::{CardRegistry as RustCardRegistry, REGISTRY_CAPACITY};
use ::digimon_engine::deck_tools;
use ::digimon_engine::enums::{CardKind as RustCardKind, GamePhase as RustGamePhase};
use ::digimon_engine::events::GameEvent;
use ::digimon_engine::observation::{
    default_observation_profile, list_observation_profiles as rust_list_observation_profiles,
    observation_layout, parse_observation_profile,
};
use ::digimon_engine::policies::greedy_action as choose_greedy_action;
use ::digimon_engine::rules::CardRestriction;
use ::digimon_engine::tensor::{MAX_SOURCES, TENSOR_SIZE};
use ::digimon_engine::tensor_profiles::{
    all_profile_ids, default_profile, profile_by_id, TensorProfile as RustTensorProfile,
};
use ::digimon_engine::HeadlessRunner;

/// Embedding dimension for the warm-start card embedding table. Mirrors
/// the Python `digimon_gym.engine.data.card_registry.EMBEDDING_DIM`
/// constant. The Rust crate doesn't carry this value internally — the
/// tensor module bakes the dim into per-tensor offsets — so we expose
/// the canonical value here for callers that need it.
const EMBEDDING_DIM: u32 = 16;

/// Lazy-loaded card database. Loading the ~4000-card JSON is slow and
/// allocation-heavy; sharing a single parsed copy across runners cuts
/// per-episode construction time. The path can be overridden via the
/// `DIGIMON_CARDS_JSON` env var for tests / alternate databases.
fn card_db() -> PyResult<&'static HashMap<String, CardData>> {
    static DB: OnceLock<HashMap<String, CardData>> = OnceLock::new();
    if let Some(db) = DB.get() {
        return Ok(db);
    }
    let path = resolve_cards_path()?;
    let loaded = CardData::load_from_file(&path).map_err(|e| {
        PyRuntimeError::new_err(format!("failed to load cards.json from {:?}: {}", path, e))
    })?;
    // OnceLock::set can race; ignore the error — whichever thread loses just
    // discards its copy.
    let _ = DB.set(loaded);
    Ok(DB.get().expect("just set"))
}

fn resolve_cards_path() -> PyResult<PathBuf> {
    if let Ok(p) = std::env::var("DIGIMON_CARDS_JSON") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Ok(pb);
        }
    }
    // Default: walk up from CWD looking for `data/cards.json`. The
    // pre-2026 location (`digimon_gym/engine/data/cards.json`) is
    // checked as a fallback so a stale checkout or partially-migrated
    // environment still loads cleanly.
    let mut here = std::env::current_dir()
        .map_err(|e| PyRuntimeError::new_err(format!("cwd unavailable: {}", e)))?;
    for _ in 0..6 {
        for rel in &["data/cards.json", "digimon_gym/engine/data/cards.json"] {
            let candidate = here.join(rel);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
        if !here.pop() {
            break;
        }
    }
    Err(PyRuntimeError::new_err(
        "cards.json not found — set DIGIMON_CARDS_JSON or run from the project root",
    ))
}

/// Python-visible mirror of the Rust `CardKind` enum. Variants match
/// `digimon_gym.engine.data.enums.CardKind` plus `Token` (Rust-only
/// today; included so `From<RustCardKind>` is total).
#[pyclass(module = "digimon_engine", name = "CardKind", eq, eq_int, hash, frozen)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CardKind {
    Digimon,
    Tamer,
    Option,
    DigiEgg,
    Token,
    Dual,
}

impl From<RustCardKind> for CardKind {
    fn from(k: RustCardKind) -> Self {
        match k {
            RustCardKind::Digimon => CardKind::Digimon,
            RustCardKind::Tamer => CardKind::Tamer,
            RustCardKind::Option => CardKind::Option,
            RustCardKind::DigiEgg => CardKind::DigiEgg,
            RustCardKind::Token => CardKind::Token,
            RustCardKind::Dual => CardKind::Dual,
        }
    }
}

/// Python-visible mirror of the Rust `GamePhase` enum. Variant names
/// match the **Python** convention (Start, End, SelectEffectChoice)
/// rather than the Rust internal names (Unsuspend, EndTurn, EffectChoice)
/// so callers post-Phase 3 cutover keep their existing identifiers.
#[pyclass(
    module = "digimon_engine",
    name = "GamePhase",
    eq,
    eq_int,
    hash,
    frozen
)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum GamePhase {
    Mulligan,
    Start,
    Draw,
    Breeding,
    Main,
    End,
    SelectTarget,
    SelectMaterial,
    SelectTrash,
    SelectSource,
    SelectHand,
    SelectReveal,
    SelectSecurity,
    SelectEffectChoice,
    BlockTiming,
    CounterTiming,
    AllianceTiming,
    EndOfTurnAction,
    GameOver,
    SelectUnion,
    SelectPermutation,
    SelectBudgeted,
    SelectBreedingPermanent,
    SelectPlayOrder,
}

impl From<RustGamePhase> for GamePhase {
    fn from(p: RustGamePhase) -> Self {
        match p {
            RustGamePhase::Mulligan => GamePhase::Mulligan,
            RustGamePhase::Unsuspend => GamePhase::Start,
            RustGamePhase::Draw => GamePhase::Draw,
            RustGamePhase::Breeding => GamePhase::Breeding,
            RustGamePhase::Main => GamePhase::Main,
            RustGamePhase::EndTurn => GamePhase::End,
            RustGamePhase::SelectTarget => GamePhase::SelectTarget,
            RustGamePhase::SelectMaterial => GamePhase::SelectMaterial,
            RustGamePhase::SelectTrash => GamePhase::SelectTrash,
            RustGamePhase::SelectSource => GamePhase::SelectSource,
            RustGamePhase::SelectHand => GamePhase::SelectHand,
            RustGamePhase::SelectReveal => GamePhase::SelectReveal,
            RustGamePhase::SelectSecurity => GamePhase::SelectSecurity,
            RustGamePhase::EffectChoice => GamePhase::SelectEffectChoice,
            RustGamePhase::BlockTiming => GamePhase::BlockTiming,
            RustGamePhase::CounterTiming => GamePhase::CounterTiming,
            RustGamePhase::AllianceTiming => GamePhase::AllianceTiming,
            RustGamePhase::EndOfTurnAction => GamePhase::EndOfTurnAction,
            RustGamePhase::GameOver => GamePhase::GameOver,
            RustGamePhase::SelectUnion => GamePhase::SelectUnion,
            RustGamePhase::SelectPermutation => GamePhase::SelectPermutation,
            RustGamePhase::SelectBudgeted => GamePhase::SelectBudgeted,
            RustGamePhase::SelectBreedingPermanent => GamePhase::SelectBreedingPermanent,
            RustGamePhase::SelectPlayOrder => GamePhase::SelectPlayOrder,
        }
    }
}

/// Python-visible mirror of `card_data::EvoCost`.
#[pyclass(module = "digimon_engine", name = "PyEvoCost")]
#[derive(Clone)]
pub struct PyEvoCost {
    /// Required color of the source card. String form of `CardColor`.
    #[pyo3(get)]
    pub card_color: String,
    #[pyo3(get)]
    pub level: u8,
    #[pyo3(get)]
    pub memory_cost: u16,
}

/// Python-visible per-card metadata wrapper. Mirror of the subset of
/// `card_data::CardData` that callers need at the Python layer (the
/// full struct contains effect text and DSL-only fields the bindings
/// don't expose). Returned by `CardDatabase::get_card`.
#[pyclass(module = "digimon_engine", name = "PyCard")]
#[derive(Clone)]
pub struct PyCard {
    #[pyo3(get)]
    pub card_id: String,
    #[pyo3(get)]
    pub card_name: String,
    /// Typed enum matching `digimon_engine.CardKind`.
    #[pyo3(get)]
    pub card_kind: CardKind,
    #[pyo3(get)]
    pub level: Option<u8>,
    #[pyo3(get)]
    pub dp: Option<i32>,
    #[pyo3(get)]
    pub play_cost: u16,
    /// String form of each `CardColor` (e.g. "Red", "Blue", ...).
    #[pyo3(get)]
    pub colors: Vec<String>,
    #[pyo3(get)]
    pub traits: Vec<String>,
    #[pyo3(get)]
    pub evo_costs: Vec<PyEvoCost>,
    #[pyo3(get)]
    pub effect_text: String,
    #[pyo3(get)]
    pub inherited_text: String,
    #[pyo3(get)]
    pub security_text: String,
}

impl PyCard {
    fn from_card_data(card: &CardData) -> Self {
        Self {
            card_id: card.card_id.clone(),
            card_name: card.card_name.clone(),
            card_kind: CardKind::from(card.card_kind),
            level: card.level,
            dp: card.dp,
            play_cost: card.play_cost,
            colors: card.colors.iter().map(|c| format!("{:?}", c)).collect(),
            traits: card.traits.clone(),
            evo_costs: card
                .evo_costs
                .iter()
                .map(|ec| PyEvoCost {
                    card_color: match ec.card_color {
                        0 => "Red".to_string(),
                        1 => "Blue".to_string(),
                        2 => "Yellow".to_string(),
                        3 => "Green".to_string(),
                        4 => "White".to_string(),
                        5 => "Black".to_string(),
                        6 => "Purple".to_string(),
                        _ => format!("Unknown({})", ec.card_color),
                    },
                    level: ec.level,
                    memory_cost: ec.memory_cost,
                })
                .collect(),
            effect_text: card.effect_text.clone(),
            inherited_text: card.inherited_text.clone(),
            security_text: card.security_text.clone(),
        }
    }
}

/// Python-visible wrapper around `deck_tools::DeckValidationResult`.
#[pyclass(module = "digimon_engine", name = "PyDeckValidationResult")]
#[derive(Clone)]
pub struct PyDeckValidationResult {
    #[pyo3(get)]
    pub is_valid: bool,
    #[pyo3(get)]
    pub errors: Vec<String>,
    #[pyo3(get)]
    pub warnings: Vec<String>,
}

#[pyfunction]
fn parse_tts(raw: &str) -> PyResult<Vec<String>> {
    deck_tools::parse_tts(raw).map_err(PyValueError::new_err)
}

#[pyfunction]
fn parse_text(raw: &str) -> PyResult<Vec<String>> {
    deck_tools::parse_text(raw).map_err(PyValueError::new_err)
}

#[pyfunction]
fn parse_deck(raw: &str) -> PyResult<Vec<String>> {
    deck_tools::parse_deck(raw).map_err(PyValueError::new_err)
}

#[pyfunction]
fn summarize_deck(card_ids: Vec<String>) -> HashMap<String, u32> {
    deck_tools::summarize_deck(&card_ids)
}

#[pyfunction]
fn validate_deck(card_ids: Vec<String>) -> PyDeckValidationResult {
    let result = deck_tools::validate_deck(&card_ids);
    PyDeckValidationResult {
        is_valid: result.is_valid,
        errors: result.errors,
        warnings: result.warnings,
    }
}

#[pyfunction]
fn validate_deck_for_game_mode(
    card_ids: Vec<String>,
    game_mode: &str,
) -> PyResult<PyDeckValidationResult> {
    let result = deck_tools::validate_deck_for_game_mode(&card_ids, game_mode)
        .map_err(PyValueError::new_err)?;
    Ok(PyDeckValidationResult {
        is_valid: result.is_valid,
        errors: result.errors,
        warnings: result.warnings,
    })
}

#[pyfunction]
fn out_of_set_cards(card_ids: Vec<String>) -> Vec<String> {
    deck_tools::out_of_set_cards(card_ids.iter().cloned())
        .into_iter()
        .collect()
}

/// Python-visible wrapper around `rules::CardRestriction`.
#[pyclass(module = "digimon_engine", name = "PyCardRestriction")]
#[derive(Clone)]
pub struct PyCardRestriction {
    /// `card_id -> max copies allowed` (0 = banned, 1 = restricted to 1).
    #[pyo3(get)]
    pub card_limits: HashMap<String, u8>,
    /// List of `(group_a, group_b)` mutual-exclusivity pairs.
    #[pyo3(get)]
    pub choice_groups: Vec<(Vec<String>, Vec<String>)>,
}

impl From<CardRestriction> for PyCardRestriction {
    fn from(r: CardRestriction) -> Self {
        Self {
            card_limits: r.card_limits.into_iter().collect(),
            choice_groups: r.choice_groups,
        }
    }
}

#[pyfunction]
fn restricted_list() -> PyCardRestriction {
    CardRestriction::official_eng().into()
}

#[pyfunction]
fn eden_restricted_list() -> PyCardRestriction {
    CardRestriction::eden().into()
}

#[pyfunction]
fn expand_deck_dict(counts: HashMap<String, u32>) -> Vec<String> {
    deck_tools::expand_deck_dict(&counts)
}

#[pyfunction]
fn load_tested_cards() -> std::collections::HashSet<String> {
    deck_tools::tested_cards_set().clone()
}

#[pyfunction]
fn is_card_tested(card_id: &str) -> bool {
    deck_tools::is_card_tested(card_id)
}

#[pyclass(module = "digimon_engine", name = "TensorProfile")]
#[derive(Clone)]
pub struct TensorProfile {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub game_mode: String,
    #[pyo3(get)]
    pub version: u32,
    #[pyo3(get)]
    pub tensor_size: usize,
    #[pyo3(get)]
    pub field_slots: usize,
    #[pyo3(get)]
    pub slot_size: usize,
    #[pyo3(get)]
    pub max_sources: usize,
    #[pyo3(get)]
    pub card_id_slot_count: usize,
    #[pyo3(get)]
    pub scalar_slot_count: usize,
    #[pyo3(get)]
    pub card_id_positions: Vec<usize>,
    #[pyo3(get)]
    pub scalar_positions: Vec<usize>,
}

fn py_tensor_profile(profile: &RustTensorProfile) -> TensorProfile {
    let (card_id_positions, scalar_positions) = profile.positions();
    TensorProfile {
        id: profile.id.to_string(),
        game_mode: profile.game_mode.to_string(),
        version: profile.version,
        tensor_size: profile.tensor_size,
        field_slots: profile.field_slots,
        slot_size: profile.slot_size,
        max_sources: MAX_SOURCES,
        card_id_slot_count: profile.card_id_slot_count,
        scalar_slot_count: profile.scalar_slot_count,
        card_id_positions,
        scalar_positions,
    }
}

#[pyfunction]
#[pyo3(signature = (profile_id = None))]
fn get_tensor_profile(profile_id: Option<String>) -> PyResult<TensorProfile> {
    let profile = match profile_id {
        None => default_profile(),
        Some(id) => profile_by_id(&id)
            .ok_or_else(|| PyValueError::new_err(format!("unknown tensor profile: {id}")))?,
    };
    Ok(py_tensor_profile(&profile))
}

#[pyfunction]
fn list_tensor_profiles() -> Vec<String> {
    all_profile_ids().into_iter().map(str::to_string).collect()
}

fn profile_to_pydict(py: Python<'_>, profile: &RustTensorProfile) -> PyResult<PyObject> {
    let d = PyDict::new_bound(py);
    let (card_id_positions, scalar_positions) = profile.positions();

    d.set_item("profile_id", profile.id)?;
    d.set_item("game_mode", profile.game_mode)?;
    d.set_item("version", profile.version)?;
    d.set_item("tensor_version", profile.tensor_version)?;
    d.set_item("feature_schema_version", profile.feature_schema_version)?;
    d.set_item("tensor_size", profile.tensor_size)?;
    d.set_item("field_slots", profile.field_slots)?;
    d.set_item("slot_size", profile.slot_size)?;
    d.set_item("max_sources", profile.max_sources)?;
    d.set_item("card_id_slot_count", profile.card_id_slot_count)?;
    d.set_item("scalar_slot_count", profile.scalar_slot_count)?;
    d.set_item("card_id_positions", card_id_positions)?;
    d.set_item("scalar_positions", scalar_positions)?;
    d.set_item("layout_hash", profile.layout_hash)?;

    let sections = PyList::empty_bound(py);
    for section in profile.sections {
        let sd = PyDict::new_bound(py);
        sd.set_item("name", section.id)?;
        sd.set_item("offset", section.start)?;
        sd.set_item("size", section.len)?;
        sd.set_item("shape", section.shape.to_vec())?;
        sections.append(sd)?;
    }
    d.set_item("sections", sections)?;

    Ok(d.into_py(py))
}

#[pyfunction]
fn list_observation_profiles() -> Vec<String> {
    rust_list_observation_profiles()
        .into_iter()
        .map(str::to_string)
        .collect()
}

#[pyfunction]
#[pyo3(signature = (profile_id = None))]
fn get_observation_layout(py: Python<'_>, profile_id: Option<String>) -> PyResult<PyObject> {
    let parsed = match profile_id {
        None => default_observation_profile(),
        Some(raw) => parse_observation_profile(&raw).map_err(PyValueError::new_err)?,
    };
    let profile = observation_layout(parsed);
    profile_to_pydict(py, &profile)
}

/// Python-visible wrapper around the Rust `CardRegistry`.
#[pyclass(module = "digimon_engine", name = "CardRegistry")]
pub struct CardRegistry {
    inner: RustCardRegistry,
}

#[pymethods]
impl CardRegistry {
    /// Construct from the static cards.json database. Mirrors what the
    /// Python `CardRegistry` does at module-import time.
    #[new]
    fn new() -> PyResult<Self> {
        let db = card_db()?;
        Ok(Self {
            inner: RustCardRegistry::from_cards(db),
        })
    }

    /// Stable integer index for `card_id`, or `None` if unknown.
    /// (The underlying Rust API returns `0` for unknown — the Python
    /// wrapper translates to `Option` for ergonomics.)
    fn index_of(&self, card_id: &str) -> Option<u16> {
        let idx = self.inner.get_index(card_id);
        if idx == 0 {
            None
        } else {
            Some(idx)
        }
    }

    /// Normalized ID = index / REGISTRY_CAPACITY. `0.0` for unknown.
    fn norm_id_of(&self, card_id: &str) -> f32 {
        self.inner.get_norm_id(card_id)
    }

    /// Reverse lookup: card ID for a given integer index.
    fn id_of(&self, index: u16) -> Option<String> {
        self.inner.get_id(index).map(|s| s.to_string())
    }

    fn count(&self) -> usize {
        self.inner.count()
    }
}

/// Resolve the ONNX models directory. Returns a `pathlib.Path` so callers
/// can use the `/` operator (PyO3 0.22's default `PathBuf` conversion
/// produces `str`, which breaks `models_dir / "<file>"` patterns).
#[pyfunction]
fn get_models_dir(py: Python) -> PyResult<PyObject> {
    let path = deck_tools::get_models_dir();
    let path_str = path.to_string_lossy().to_string();
    let pathlib = py.import_bound("pathlib")?;
    let path_class = pathlib.getattr("Path")?;
    Ok(path_class.call1((path_str,))?.into_py(py))
}

#[pyfunction]
fn load_implemented_card_ids() -> std::collections::HashSet<String> {
    ::digimon_engine::cards::build_registry()
        .registered_card_ids()
        .into_iter()
        .collect()
}

/// Python-visible wrapper around the static cards.json database.
#[pyclass(module = "digimon_engine", name = "CardDatabase")]
pub struct CardDatabase {}

#[pymethods]
impl CardDatabase {
    #[new]
    fn new() -> PyResult<Self> {
        // Touch the static loader so construction surfaces a clear error
        // (rather than deferring it to the first lookup).
        let _ = card_db()?;
        Ok(Self {})
    }

    fn get_card(&self, card_id: &str) -> PyResult<Option<PyCard>> {
        let db = card_db()?;
        Ok(db.get(card_id).map(PyCard::from_card_data))
    }

    fn count(&self) -> PyResult<usize> {
        let db = card_db()?;
        Ok(db.len())
    }
}

/// RL-shaped game runner. Mirrors `HeadlessGame`'s public surface.
#[pyclass(module = "digimon_engine", name = "RustHeadlessGame")]
pub struct RustHeadlessGame {
    inner: HeadlessRunner,
}

#[pymethods]
impl RustHeadlessGame {
    #[new]
    #[pyo3(signature = (
        deck1_ids,
        deck2_ids,
        verbose = false,
        record_actions = false,
        record_tensors = false,
        seed = None,
        observation_profile = None
    ))]
    fn new(
        deck1_ids: Vec<String>,
        deck2_ids: Vec<String>,
        verbose: bool,
        record_actions: bool,
        record_tensors: bool,
        seed: Option<u64>,
        observation_profile: Option<String>,
    ) -> PyResult<Self> {
        let db = card_db()?;
        let profile = match observation_profile {
            None => default_observation_profile(),
            Some(raw) => parse_observation_profile(&raw).map_err(PyValueError::new_err)?,
        };
        let runner = HeadlessRunner::new_with_observation_profile(
            deck1_ids,
            deck2_ids,
            db,
            verbose,
            record_actions,
            record_tensors,
            seed,
            profile,
        )
        .map_err(PyValueError::new_err)?;
        Ok(Self { inner: runner })
    }

    /// Execute a single action for the current decision player. No-op after
    /// `game_over`. Matches `HeadlessGame.step`.
    fn step(&mut self, action_id: u16) {
        self.inner.step(action_id);
    }

    /// Return the action mask for the current decision player as a
    /// numpy `float32` array of shape `(ACTION_SPACE_SIZE,)`.
    fn get_action_mask<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f32>> {
        let mask = self.inner.get_action_mask();
        PyArray1::from_vec_bound(py, mask)
    }

    /// Return the board tensor for `player_id` (Python 1/2, default = current).
    #[pyo3(signature = (player_id = None))]
    fn get_board_tensor<'py>(
        &self,
        py: Python<'py>,
        player_id: Option<u8>,
    ) -> PyResult<Bound<'py, PyArray1<f32>>> {
        let rust_pid = match player_id {
            None => None,
            Some(pid) => Some(to_rust_pid(pid)?),
        };
        let tensor = self.inner.get_board_tensor(rust_pid);
        Ok(PyArray1::from_vec_bound(py, tensor))
    }

    fn get_rl_state(&self, py: Python) -> PyResult<PyObject> {
        let game = &self.inner.game;
        let current_player = self.inner.current_decision_player() + 1;
        let p1 = game.player(0);
        let p2 = game.player(1);

        let d = PyDict::new_bound(py);
        d.set_item("game_over", game.game_over)?;
        d.set_item("winner_id", to_python_pid(game.winner.unwrap_or(u8::MAX)))?;
        d.set_item(
            "terminal_reason",
            game.terminal_outcome_reason.map(|reason| reason.as_str()),
        )?;
        d.set_item(
            "terminal_result",
            game.terminal_outcome_reason.map(|reason| reason.result()),
        )?;
        d.set_item("current_player_id", current_player)?;
        d.set_item("phase", game.current_phase.py_name())?;
        d.set_item("memory", game.memory)?;
        d.set_item("p1_security", p1.security.len())?;
        d.set_item("p2_security", p2.security.len())?;
        d.set_item("p1_total_dp", p1.total_field_dp(&game.card_data))?;
        d.set_item("p2_total_dp", p2.total_field_dp(&game.card_data))?;
        // Digivolve reward-shaping counters. See
        // docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md.
        // p1 = Rust index 0, p2 = Rust index 1 (Python 1/2 convention).
        d.set_item("p1_digivolutions", game.n_digivolutions[0])?;
        d.set_item("p2_digivolutions", game.n_digivolutions[1])?;
        d.set_item("p1_dna_digivolutions", game.n_dna_digivolutions[0])?;
        d.set_item("p2_dna_digivolutions", game.n_dna_digivolutions[1])?;
        Ok(d.into_py(py))
    }

    fn greedy_action(&self) -> u16 {
        let mask = self.inner.get_action_mask();
        choose_greedy_action(&self.inner.game, &mask)
    }

    /// Run to conclusion. `policy_fn`, if provided, is
    /// `Callable[[game, mask], int]`. Returns Python player_id
    /// (1 or 2), or 0 for draw/timeout. Matches
    /// `HeadlessGame.run_until_conclusion`.
    #[pyo3(signature = (max_turns = 200, policy_fn = None))]
    fn run_until_conclusion(
        &mut self,
        py: Python,
        max_turns: u32,
        policy_fn: Option<PyObject>,
    ) -> PyResult<i32> {
        // We can't pass an `&Game` across the FFI boundary (PyO3 would need
        // a wrapper), but RL policies typically only need the mask — not the
        // full game — because the mask already encodes legal actions. The
        // Python default policy signature is `(game, mask) -> int`. For now
        // we pass `None, mask` when a policy is provided, which matches the
        // "action-mask-only" policies used in training; policies that need
        // to inspect game state should access it via `.game` on the runner.
        let winner = if let Some(cb) = policy_fn {
            let mut steps = 0u32;
            while !self.inner.is_game_over() && steps < max_turns {
                let mask = self.inner.get_action_mask();
                let mask_arr = PyArray1::from_slice_bound(py, &mask);
                let args = (py.None(), mask_arr);
                let result = cb.call1(py, args)?;
                let action: u16 = result.extract(py)?;
                self.inner.step(action);
                steps += 1;
            }
            if !self.inner.is_game_over() {
                self.inner.game.declare_winner(0);
            }
            self.inner.winner_id()
        } else {
            self.inner
                .run_until_conclusion::<fn(&_, &[f32]) -> u16>(max_turns, None)
        };
        Ok(to_python_pid(winner).unwrap_or(0) as i32)
    }

    /// Log buffer (empty until a future recording milestone ports the logger).
    fn get_last_log<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        PyList::empty_bound(py)
    }

    /// Full UI-state dict. Matches Python's
    /// `digimon_gym.engine.game.serialization.to_ui_json`. Consumed by
    /// `state_filter.py` and the React frontend.
    fn to_ui_json(&self, py: Python) -> PyResult<PyObject> {
        let value = ::digimon_engine::serialization::to_ui_json(&self.inner.game);
        json_to_pyobject(py, &value)
    }

    /// Snapshot of the currently installed `PendingSelection`, or `None`
    /// if no prompt is pending. Keys: `kind`, `phase`, `selectingPlayer`
    /// (Python 1/2 convention), `validIndices`, `isOptional`, `prompt`,
    /// optional `effectChoices`.
    fn get_pending_selection(&self, py: Python) -> PyResult<PyObject> {
        let game = &self.inner.game;
        match game.pending_selection.as_ref() {
            None => Ok(py.None()),
            Some(sel) => {
                let v = sel.view();
                let d = PyDict::new_bound(py);
                d.set_item("kind", v.kind_str())?;
                d.set_item("phase", v.previous_phase_str())?;
                d.set_item("selectingPlayer", (v.selecting_player as i64) + 1)?;
                d.set_item("validIndices", v.valid_action_ids.clone())?;
                d.set_item("isOptional", v.is_optional)?;
                d.set_item("prompt", v.prompt.clone())?;
                if let Some(choices) = v.effect_choices.as_ref() {
                    let list = PyList::empty_bound(py);
                    for c in choices {
                        let cd = PyDict::new_bound(py);
                        cd.set_item("label", c.label.as_str())?;
                        cd.set_item("actionId", c.action_id)?;
                        list.append(cd)?;
                    }
                    d.set_item("effectChoices", list)?;
                }
                Ok(d.into_py(py))
            }
        }
    }

    /// Drain accumulated `GameEvent`s since the last call. Each dict has
    /// `type`, `seq`, `player` (Python 1/2), `source_card_id`,
    /// `source_slot`, `target_card_id`, `target_slot`, `meta`. Matches
    /// Python `GameEvent.to_dict`.
    fn get_events_since_last_step<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let drained = self.inner.game.drain_events();
        let list = PyList::empty_bound(py);
        for ev in drained {
            list.append(event_to_pydict(py, &ev)?)?;
        }
        Ok(list)
    }

    /// Recording dict, or `None` if `record_actions=False` at
    /// construction. Shape matches Python `GameRecorder.to_dict`.
    fn get_recording(&self, py: Python) -> PyResult<PyObject> {
        match self.inner.get_recording() {
            None => Ok(py.None()),
            Some(v) => json_to_pyobject(py, &v),
        }
    }

    fn get_terminal_outcome(&self, py: Python) -> PyResult<PyObject> {
        let d = PyDict::new_bound(py);
        let reason = self.inner.terminal_outcome_reason();
        d.set_item("game_over", self.inner.is_game_over())?;
        d.set_item("winner_id", to_python_pid(self.inner.winner_id()))?;
        d.set_item("result", reason.map(|r| r.result()))?;
        d.set_item("reason", reason.map(|r| r.as_str()))?;
        // `win_reason` is a stable, Python-friendly alias for `reason`.
        // BO3 match training consumes this on the per-game recording
        // artifact (per the `bo3-match-training` spec) — recorders write
        // it verbatim into `outcome.win_reason`.
        d.set_item("win_reason", reason.map(|r| r.as_str()))?;
        Ok(d.into_py(py))
    }

    fn force_step_limit_winner(&mut self) -> Option<u8> {
        to_python_pid(self.inner.force_step_limit_winner())
    }

    /// Concede on behalf of `pid` (Python 1/2). The opponent is declared
    /// the winner with `win_reason = "concede"`. Always-legal at any agent
    /// decision point; the action mask reports action 93 as legal whenever
    /// the player has any other legal action. Used by the Python
    /// `MatchEnv` wrapper to handle BO3 match-training concedes.
    fn concede(&mut self, pid: u8) -> PyResult<Option<u8>> {
        let rust_pid = to_rust_pid(pid)?;
        let winner = self.inner.concede(rust_pid);
        Ok(to_python_pid(winner))
    }

    /// Install a `SelectPlayOrder` selection with `loser_pid` (Python 1/2)
    /// as the chooser. The mask then reports actions 94 (PLAY_FIRST) and
    /// 95 (PLAY_SECOND) legal for that player. Call this between games
    /// of a BO3 match; after the next `step()` resolves the selection,
    /// call `take_play_order_choice()` to read the result.
    fn request_play_order_selection(&mut self, loser_pid: u8) -> PyResult<()> {
        let rust_pid = to_rust_pid(loser_pid)?;
        self.inner.request_play_order_selection(rust_pid);
        Ok(())
    }

    /// Read-and-clear the most recent `SelectPlayOrder` outcome. Returns
    /// `"first"`, `"second"`, or `None` if no selection has been resolved
    /// since the slot was last taken. The Python `MatchEnv` wrapper uses
    /// the returned string to decide the first-player for the next game.
    fn take_play_order_choice(&mut self) -> Option<&'static str> {
        self.inner.take_play_order_choice().map(|p| match p {
            ::digimon_engine::selection::PlayOrder::First => "first",
            ::digimon_engine::selection::PlayOrder::Second => "second",
        })
    }

    #[getter]
    fn is_game_over(&self) -> bool {
        self.inner.is_game_over()
    }

    /// Winner as Python player_id (1 or 2), or `None` if no winner yet.
    #[getter]
    fn winner_id(&self) -> Option<u8> {
        to_python_pid(self.inner.winner_id())
    }

    #[getter]
    fn current_player_id(&self) -> u8 {
        self.inner.current_decision_player() + 1
    }

    #[getter]
    fn observation_profile_id(&self) -> &'static str {
        self.inner.observation_profile_id()
    }

    fn get_observation_layout(&self, py: Python<'_>) -> PyResult<PyObject> {
        let profile = self.inner.observation_layout();
        profile_to_pydict(py, &profile)
    }

    /// Manual mulligan override. `pid` is the Python player_id (1 or 2).
    fn accept_mulligan(&mut self, pid: u8, keep: bool) -> PyResult<()> {
        let rust_pid = to_rust_pid(pid)?;
        self.inner
            .accept_mulligan(rust_pid, keep)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Python player_id (1 or 2) of the player expected to submit the next
    /// mulligan decision, or `None` if mulligan is already complete.
    #[getter]
    fn mulligan_current_player(&self) -> Option<u8> {
        to_python_pid(self.inner.mulligan_current_player().unwrap_or(u8::MAX))
    }
}

fn to_rust_pid(py_pid: u8) -> PyResult<u8> {
    match py_pid {
        1 | 2 => Ok(py_pid - 1),
        _ => Err(PyValueError::new_err("player_id must be 1 or 2")),
    }
}

fn to_python_pid(rust_pid: u8) -> Option<u8> {
    if rust_pid == u8::MAX {
        None
    } else {
        Some(rust_pid + 1)
    }
}

/// Recursively convert a `serde_json::Value` into a Python object.
/// Objects become `PyDict`, arrays become `PyList`, numbers become
/// `int` or `float`, null becomes `None`.
fn json_to_pyobject(py: Python, v: &Value) -> PyResult<PyObject> {
    Ok(match v {
        Value::Null => py.None(),
        Value::Bool(b) => b.into_py(py),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_py(py)
            } else if let Some(u) = n.as_u64() {
                u.into_py(py)
            } else {
                n.as_f64().unwrap_or(0.0).into_py(py)
            }
        }
        Value::String(s) => s.into_py(py),
        Value::Array(a) => {
            let list = PyList::empty_bound(py);
            for item in a {
                list.append(json_to_pyobject(py, item)?)?;
            }
            list.into_py(py)
        }
        Value::Object(o) => {
            let dict = PyDict::new_bound(py);
            for (k, val) in o {
                dict.set_item(k.as_str(), json_to_pyobject(py, val)?)?;
            }
            dict.into_py(py)
        }
    })
}

/// Convert a single `GameEvent` into a dict matching Python's
/// `GameEvent.to_dict` shape — keys: `type`, `seq`, `player`,
/// `source_card_id`, `source_slot`, `target_card_id`, `target_slot`,
/// `meta`.
fn event_to_pydict<'py>(py: Python<'py>, ev: &GameEvent) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("type", ev.type_str())?;
    d.set_item("seq", ev.seq())?;
    d.set_item("meta", PyDict::new_bound(py))?;
    // defaults
    d.set_item("source_card_id", py.None())?;
    d.set_item("source_slot", py.None())?;
    d.set_item("target_card_id", py.None())?;
    d.set_item("target_slot", py.None())?;
    d.set_item("player", 0)?;

    let py_pid = |p: u8| -> i64 { (p as i64) + 1 };

    match ev {
        GameEvent::MemoryChange {
            player,
            delta,
            total,
            ..
        } => {
            d.set_item("player", py_pid(*player))?;
            let meta = PyDict::new_bound(py);
            meta.set_item("delta", *delta)?;
            meta.set_item("total", *total)?;
            d.set_item("meta", meta)?;
        }
        GameEvent::TurnStart {
            player, turn_count, ..
        } => {
            d.set_item("player", py_pid(*player))?;
            let meta = PyDict::new_bound(py);
            meta.set_item("turn_count", *turn_count)?;
            d.set_item("meta", meta)?;
        }
        GameEvent::PhaseChange { player, phase, .. } => {
            d.set_item("player", py_pid(*player))?;
            let meta = PyDict::new_bound(py);
            meta.set_item("phase", format!("{:?}", phase))?;
            d.set_item("meta", meta)?;
        }
        GameEvent::Play {
            player,
            card_id,
            field_index,
            cost_paid,
            cost_printed,
            via_alt_path,
            ..
        } => {
            d.set_item("player", py_pid(*player))?;
            d.set_item("source_card_id", card_id.as_str())?;
            d.set_item("source_slot", *field_index)?;
            // New per-play payload from the `add-reward-profiles` change
            // (capability `engine-event-emission`). Surfaces in meta so
            // the existing top-level keys stay stable.
            let meta = PyDict::new_bound(py);
            meta.set_item("cost_paid", *cost_paid)?;
            meta.set_item("cost_printed", *cost_printed)?;
            meta.set_item(
                "via_alt_path",
                via_alt_path
                    .as_ref()
                    .map(|s| s.as_str())
                    .map(|s| s.into_py(py))
                    .unwrap_or_else(|| py.None()),
            )?;
            d.set_item("meta", meta)?;
        }
        GameEvent::Digivolve {
            player,
            top_card_id,
            field_index,
            from_stack_top,
            was_dna,
            was_blast_dna,
            memory_paid,
            ..
        } => {
            d.set_item("player", py_pid(*player))?;
            d.set_item("source_card_id", top_card_id.as_str())?;
            d.set_item("source_slot", *field_index)?;
            // New payload from the `add-reward-profiles` change
            // (capability `engine-event-emission`). Surfaces in `meta`
            // alongside the existing `from_stack_top` so top-level keys
            // stay stable for downstream consumers that don't yet read
            // the new fields.
            let meta = PyDict::new_bound(py);
            meta.set_item("from_stack_top", from_stack_top.as_str())?;
            meta.set_item("was_dna", *was_dna)?;
            meta.set_item("was_blast_dna", *was_blast_dna)?;
            meta.set_item("memory_paid", *memory_paid)?;
            d.set_item("meta", meta)?;
        }
        GameEvent::Attack {
            player,
            attacker_field_index,
            target_field_index,
            target_player,
            ..
        } => {
            d.set_item("player", py_pid(*player))?;
            d.set_item("source_slot", *attacker_field_index)?;
            if let Some(t) = target_field_index {
                d.set_item("target_slot", *t)?;
            }
            let meta = PyDict::new_bound(py);
            meta.set_item("target_player", target_player.map(|p| py_pid(p)))?;
            d.set_item("meta", meta)?;
        }
        GameEvent::Trash {
            player, card_id, ..
        } => {
            d.set_item("player", py_pid(*player))?;
            d.set_item("source_card_id", card_id.as_str())?;
        }
        GameEvent::Mill {
            player, card_id, ..
        } => {
            d.set_item("player", py_pid(*player))?;
            d.set_item("source_card_id", card_id.as_str())?;
        }
        GameEvent::SecurityReveal {
            defender, card_id, ..
        } => {
            d.set_item("player", py_pid(*defender))?;
            d.set_item("source_card_id", card_id.as_str())?;
        }
        GameEvent::GameOver { winner, reason, .. } => {
            let meta = PyDict::new_bound(py);
            meta.set_item("winner", winner.map(|w| py_pid(w)))?;
            meta.set_item("reason", reason.as_str())?;
            meta.set_item("result", reason.result())?;
            d.set_item("meta", meta)?;
        }
        _ => {
            // Future variants: emit with defaults only (non_exhaustive guard)
        }
    }
    Ok(d)
}

#[pymodule]
fn digimon_engine(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add_class::<RustHeadlessGame>()?;
    m.add_class::<CardDatabase>()?;
    m.add_class::<PyCard>()?;
    m.add_class::<PyEvoCost>()?;
    m.add_class::<PyDeckValidationResult>()?;
    m.add_function(wrap_pyfunction!(parse_tts, m)?)?;
    m.add_function(wrap_pyfunction!(parse_text, m)?)?;
    m.add_function(wrap_pyfunction!(parse_deck, m)?)?;
    m.add_function(wrap_pyfunction!(summarize_deck, m)?)?;
    m.add_function(wrap_pyfunction!(validate_deck, m)?)?;
    m.add_function(wrap_pyfunction!(validate_deck_for_game_mode, m)?)?;
    m.add_function(wrap_pyfunction!(out_of_set_cards, m)?)?;
    m.add_class::<PyCardRestriction>()?;
    m.add_function(wrap_pyfunction!(restricted_list, m)?)?;
    m.add_function(wrap_pyfunction!(eden_restricted_list, m)?)?;
    m.add_function(wrap_pyfunction!(expand_deck_dict, m)?)?;
    m.add_class::<CardKind>()?;
    m.add_class::<GamePhase>()?;
    m.add_function(wrap_pyfunction!(load_tested_cards, m)?)?;
    m.add_function(wrap_pyfunction!(is_card_tested, m)?)?;
    m.add_class::<CardRegistry>()?;
    m.add("REGISTRY_CAPACITY", REGISTRY_CAPACITY)?;
    m.add("EMBEDDING_DIM", EMBEDDING_DIM)?;
    m.add_class::<TensorProfile>()?;
    m.add_function(wrap_pyfunction!(get_tensor_profile, m)?)?;
    m.add_function(wrap_pyfunction!(list_tensor_profiles, m)?)?;
    m.add("TENSOR_PROFILE_ID", default_profile().id)?;
    m.add_function(wrap_pyfunction!(list_observation_profiles, m)?)?;
    m.add_function(wrap_pyfunction!(get_observation_layout, m)?)?;
    m.add(
        "DEFAULT_OBSERVATION_PROFILE",
        default_observation_profile().as_str(),
    )?;
    m.add_function(wrap_pyfunction!(get_models_dir, m)?)?;
    m.add_function(wrap_pyfunction!(load_implemented_card_ids, m)?)?;
    m.add("ACTION_SPACE_SIZE", ACTION_SPACE_SIZE)?;
    // Source-selection sub-range boundaries — Python callers (digimon_gym)
    // pin ACTION_SOURCE_END to SOURCE_SELECT_END - 1, and use the breeding
    // constants for the appended breeding-carrier source-select range.
    m.add("SOURCE_SELECT_START", SOURCE_SELECT_START)?;
    m.add("SOURCE_SELECT_END", SOURCE_SELECT_END)?;
    m.add("BREEDING_SOURCE_SELECT_START", BREEDING_SOURCE_SELECT_START)?;
    m.add("BREEDING_SOURCE_SELECT_END", BREEDING_SOURCE_SELECT_END)?;
    m.add("TENSOR_SIZE", TENSOR_SIZE)?;
    Ok(())
}
