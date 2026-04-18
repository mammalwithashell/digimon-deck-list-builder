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
use pyo3::types::PyList;

use ::digimon_engine::card_data::CardData;
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

    /// Recording dict, or `None` until a future milestone ports `GameRecorder`.
    fn get_recording(&self, py: Python) -> PyObject {
        py.None()
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

#[pymodule]
fn digimon_engine(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add_class::<RustHeadlessGame>()?;
    Ok(())
}
