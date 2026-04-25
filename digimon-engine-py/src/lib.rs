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

use ::digimon_engine::card_data::CardData;
use ::digimon_engine::card_registry::{CardRegistry as RustCardRegistry, REGISTRY_CAPACITY};
use ::digimon_engine::deck_tools;
use ::digimon_engine::enums::{CardKind as RustCardKind, GamePhase as RustGamePhase};
use ::digimon_engine::events::GameEvent;
use ::digimon_engine::rules::CardRestriction;
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
#[pyclass(module = "digimon_engine", name = "CardKind", eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CardKind {
    Digimon,
    Tamer,
    Option,
    DigiEgg,
    Token,
}

impl From<RustCardKind> for CardKind {
    fn from(k: RustCardKind) -> Self {
        match k {
            RustCardKind::Digimon => CardKind::Digimon,
            RustCardKind::Tamer => CardKind::Tamer,
            RustCardKind::Option => CardKind::Option,
            RustCardKind::DigiEgg => CardKind::DigiEgg,
            RustCardKind::Token => CardKind::Token,
        }
    }
}

/// Python-visible mirror of the Rust `GamePhase` enum. Variant names
/// match the **Python** convention (Start, End, SelectEffectChoice)
/// rather than the Rust internal names (Unsuspend, EndTurn, EffectChoice)
/// so callers post-Phase 3 cutover keep their existing identifiers.
#[pyclass(module = "digimon_engine", name = "GamePhase", eq, eq_int)]
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
        }
    }
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
fn out_of_set_cards(card_ids: Vec<String>) -> Vec<String> {
    deck_tools::out_of_set_cards(card_ids.iter().cloned()).into_iter().collect()
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

#[pyfunction]
fn get_models_dir() -> PathBuf {
    deck_tools::get_models_dir()
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
    #[pyo3(signature = (deck1_ids, deck2_ids, verbose = false, record_actions = false, record_tensors = false, seed = None))]
    fn new(
        deck1_ids: Vec<String>,
        deck2_ids: Vec<String>,
        verbose: bool,
        record_actions: bool,
        record_tensors: bool,
        seed: Option<u64>,
    ) -> PyResult<Self> {
        let db = card_db()?;
        let runner = HeadlessRunner::new(
            deck1_ids,
            deck2_ids,
            db,
            verbose,
            record_actions,
            record_tensors,
            seed,
        )
        .map_err(PyValueError::new_err)?;
        // Auto-keep both players' mulligan so the runner behaves like Python
        // `HeadlessGame`, which calls `game.start_game()` in its base class.
        // Callers who want explicit mulligan decisions should call
        // `accept_mulligan` before any `step`.
        let mut this = Self { inner: runner };
        while let Some(p) = this.inner.mulligan_current_player() {
            let _ = this.inner.accept_mulligan(p, true);
        }
        Ok(this)
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
    ) -> Bound<'py, PyArray1<f32>> {
        let rust_pid = player_id.map(|p| p.saturating_sub(1));
        let tensor = self.inner.get_board_tensor(rust_pid);
        PyArray1::from_vec_bound(py, tensor)
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
                d.set_item(
                    "selectingPlayer",
                    (v.selecting_player as i64) + 1,
                )?;
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
    fn get_events_since_last_step<'py>(
        &mut self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyList>> {
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

    #[getter]
    fn is_game_over(&self) -> bool {
        self.inner.is_game_over()
    }

    /// Winner as Python player_id (1 or 2), or `None` if no winner yet.
    #[getter]
    fn winner_id(&self) -> Option<u8> {
        to_python_pid(self.inner.winner_id())
    }

    /// Manual mulligan override. `pid` is the Python player_id (1 or 2).
    fn accept_mulligan(&mut self, pid: u8, keep: bool) -> PyResult<()> {
        let rust_pid = pid.checked_sub(1).ok_or_else(|| {
            PyValueError::new_err("player_id must be 1 or 2")
        })?;
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
        GameEvent::MemoryChange { player, delta, total, .. } => {
            d.set_item("player", py_pid(*player))?;
            let meta = PyDict::new_bound(py);
            meta.set_item("delta", *delta)?;
            meta.set_item("total", *total)?;
            d.set_item("meta", meta)?;
        }
        GameEvent::TurnStart { player, turn_count, .. } => {
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
        GameEvent::Play { player, card_id, field_index, .. } => {
            d.set_item("player", py_pid(*player))?;
            d.set_item("source_card_id", card_id.as_str())?;
            d.set_item("source_slot", *field_index)?;
        }
        GameEvent::Digivolve { player, top_card_id, field_index, from_stack_top, .. } => {
            d.set_item("player", py_pid(*player))?;
            d.set_item("source_card_id", top_card_id.as_str())?;
            d.set_item("source_slot", *field_index)?;
            let meta = PyDict::new_bound(py);
            meta.set_item("from_stack_top", from_stack_top.as_str())?;
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
            meta.set_item(
                "target_player",
                target_player.map(|p| py_pid(p)),
            )?;
            d.set_item("meta", meta)?;
        }
        GameEvent::Trash { player, card_id, .. } => {
            d.set_item("player", py_pid(*player))?;
            d.set_item("source_card_id", card_id.as_str())?;
        }
        GameEvent::Mill { player, card_id, .. } => {
            d.set_item("player", py_pid(*player))?;
            d.set_item("source_card_id", card_id.as_str())?;
        }
        GameEvent::SecurityReveal { defender, card_id, .. } => {
            d.set_item("player", py_pid(*defender))?;
            d.set_item("source_card_id", card_id.as_str())?;
        }
        GameEvent::GameOver { winner, .. } => {
            let meta = PyDict::new_bound(py);
            meta.set_item(
                "winner",
                winner.map(|w| py_pid(w)),
            )?;
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
    m.add_class::<PyDeckValidationResult>()?;
    m.add_function(wrap_pyfunction!(parse_tts, m)?)?;
    m.add_function(wrap_pyfunction!(parse_text, m)?)?;
    m.add_function(wrap_pyfunction!(parse_deck, m)?)?;
    m.add_function(wrap_pyfunction!(summarize_deck, m)?)?;
    m.add_function(wrap_pyfunction!(validate_deck, m)?)?;
    m.add_function(wrap_pyfunction!(out_of_set_cards, m)?)?;
    m.add_class::<PyCardRestriction>()?;
    m.add_function(wrap_pyfunction!(restricted_list, m)?)?;
    m.add_function(wrap_pyfunction!(expand_deck_dict, m)?)?;
    m.add_class::<CardKind>()?;
    m.add_class::<GamePhase>()?;
    m.add_function(wrap_pyfunction!(load_tested_cards, m)?)?;
    m.add_function(wrap_pyfunction!(is_card_tested, m)?)?;
    m.add_class::<CardRegistry>()?;
    m.add("REGISTRY_CAPACITY", REGISTRY_CAPACITY)?;
    m.add("EMBEDDING_DIM", EMBEDDING_DIM)?;
    m.add_function(wrap_pyfunction!(get_models_dir, m)?)?;
    m.add_function(wrap_pyfunction!(load_implemented_card_ids, m)?)?;
    Ok(())
}
