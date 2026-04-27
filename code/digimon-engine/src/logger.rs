//! Game logger trait + default impls. Mirrors
//! `digimon_gym/engine/loggers.py::IGameLogger`.
//!
//! `SilentLogger` is the RL-training default (no allocations). The
//! engine instantiates it implicitly so hot loops never pay for log
//! buffering. Interactive callers can swap in `VerboseLogger` via
//! `Game::set_logger` to retrieve human-readable traces.
//!
//! Structured `GameEvent` support (Python's `EventLogger`) is deferred
//! until the event taxonomy is ported.

/// Abstract base for game logging. Parity with Python's `IGameLogger`.
pub trait GameLogger: Send + Sync + std::fmt::Debug {
    /// Append a user-facing log message.
    fn log(&mut self, message: &str);

    /// Append a verbose/debug-only message. `VerboseLogger` prefixes it
    /// with `[VERBOSE]` to match Python formatting.
    fn log_verbose(&mut self, message: &str);

    /// Return all accumulated log messages, oldest first. `SilentLogger`
    /// always returns an empty vec.
    fn get_logs(&self) -> Vec<String>;

    /// Drop all buffered messages.
    fn clear(&mut self);
}

/// Zero-overhead logger used by default. Drops every message.
#[derive(Debug, Default)]
pub struct SilentLogger;

impl GameLogger for SilentLogger {
    fn log(&mut self, _message: &str) {}
    fn log_verbose(&mut self, _message: &str) {}
    fn get_logs(&self) -> Vec<String> {
        Vec::new()
    }
    fn clear(&mut self) {}
}

/// Buffers every log message. Used by tests and interactive runners.
#[derive(Debug)]
pub struct VerboseLogger {
    logs: Vec<String>,
}

impl VerboseLogger {
    pub fn new() -> Self {
        Self { logs: Vec::new() }
    }
}

impl Default for VerboseLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl GameLogger for VerboseLogger {
    fn log(&mut self, message: &str) {
        self.logs.push(message.to_string());
    }
    fn log_verbose(&mut self, message: &str) {
        self.logs.push(format!("[VERBOSE] {}", message));
    }
    fn get_logs(&self) -> Vec<String> {
        self.logs.clone()
    }
    fn clear(&mut self) {
        self.logs.clear();
    }
}
