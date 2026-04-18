//! PyO3 bindings for `digimon-engine`.
//!
//! Exposes `RustHeadlessGame`, a 1:1 mirror of Python's
//! `digimon_gym.engine.runners.headless_game.HeadlessGame`. `DigimonEnv`
//! swaps between the two via the `DIGIMON_BACKEND` env var.
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

use ::digimon_engine::card_data::CardData;
use ::digimon_engine::events::GameEvent;
use ::digimon_engine::HeadlessRunner;

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
    // Default: walk up from CWD looking for digimon_gym/engine/data/cards.json.
    let mut here = std::env::current_dir()
        .map_err(|e| PyRuntimeError::new_err(format!("cwd unavailable: {}", e)))?;
    for _ in 0..6 {
        let candidate = here.join("digimon_gym/engine/data/cards.json");
        if candidate.exists() {
            return Ok(candidate);
        }
        if !here.pop() {
            break;
        }
    }
    Err(PyRuntimeError::new_err(
        "cards.json not found — set DIGIMON_CARDS_JSON or run from the project root",
    ))
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
    Ok(())
}
