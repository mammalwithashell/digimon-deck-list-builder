# Exam MCP Surface Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give an authoring agent a small, structured surface onto the exam — plan its work, ask DCGO a question *while composing a line*, lint a draft before spending Unity time, and read the scenario contract and keyword semantics in targeted pieces instead of re-reading 1,200 lines of prose.

**Architecture:** A `dcgo-harness mcp` subcommand serving JSON-RPC 2.0 over stdio, mirroring `code/digimon-engine-mcp/`'s existing protocol layer. Every tool is a thin projection over machinery that already exists (`exam_binding.bind()`, `exam::differ`, `exam::ledger`, `docs/digimon-rules/`), so each behaviour stays unit-testable with no MCP client in the loop.

**Tech Stack:** Rust 2021 (`code/tools/dcgo-harness`, serde + serde_json + clap), Python 3 stdlib (guide generation), JSON-RPC 2.0 / MCP.

**Spec:** `docs/superpowers/specs/2026-08-27-archetype-campaign-fleet-design.md` §3.

**Prerequisite:** the ledger plan (`2026-08-27-exam-ledger-fleet-safety.md`) — this plan calls `VerdictStore::load_dir`, `ledger::{claim_cards, release_cards, append_attempt}`, and the directory-aware `load_verdict_store`.

## Why this exists (motivation the tasks depend on)

Two measurements from the Toho Braves campaign drive every decision here:

1. **Orchestration was the single largest cost** — $704 of a $4,210 campaign, driven by 540 M cache-read tokens on the main thread: an agent shelling out to the CLI, reading large outputs, and re-establishing context. Small structured payloads are the fix.
2. **`--sim-only` cannot predict the oracle.** The corpus lowers 144/144 in our engine alone, yet when six sim-green scenarios were put to DCGO **all six failed — every one on prompt *sequence*.** An agent that authors blind and submits in batch produces corpora that are sim-green and oracle-red. `exam_probe` is what closes that loop, and it is the centre of this plan, not a convenience.

## Global Constraints

- **Per-worktree `CARGO_TARGET_DIR`** (CLAUDE.md rule 31). Prefix cargo with `CARGO_TARGET_DIR='D:\cargo-target-wt\<worktree-name>'` if the session inherited a stale env. A compile error in a file you did not touch means target contamination — suspect it before debugging your change.
- **`dcgo-harness` is dev/test tooling.** Never imported by `server.*` or `digimon_gym.*`; never bundled into a production build. The MCP writes **only** to the ledger, the scenario directory, and the node's local job queue — never game state, a database, or any hosted surface.
- **Clause identity is never invented.** A clause id is `clause_coverage.models.Clause.id` == `{card_id}#{zone}#{idx}`.
- **`unmeasured` is a real outcome.** Every status/plan payload carries all five verdict classes and they always sum to the denominator. No tool may let a card read as "passed".
- **DCGO is source priority #2.** `general_rule.pdf` outranks it. No tool may present a DCGO observation as a rules verdict.
- **Payloads are small by design.** A tool that returns the whole verdict store has failed at its purpose (see motivation #1). Return what the agent needs to act, plus a count of what was elided.
- Python tools are standard-library only, matching `code/tools/clause_coverage/`.

## File Structure

| File | Responsibility |
|---|---|
| `code/tools/dcgo-harness/src/mcp/protocol.rs` (create) | JSON-RPC 2.0 framing — request, response, error codes. |
| `code/tools/dcgo-harness/src/mcp/tools.rs` (create) | Tool descriptors for `tools/list` + argument extraction helpers. |
| `code/tools/dcgo-harness/src/mcp/handlers.rs` (create) | One function per tool; all the behaviour. |
| `code/tools/dcgo-harness/src/mcp/mod.rs` (create) | `serve(root)` — the stdio loop; module wiring. |
| `code/tools/dcgo-harness/src/main.rs` (modify) | `Mcp` subcommand. |
| `code/tools/dcgo-harness/src/exam/validate.rs` (create) | Scenario linter, shared by the MCP and `exam --sim-only`. |
| `code/tools/clause_coverage/authoring_guide.py` (create) | Generate the guide corpus from `docs/DCGO_EXAM.md`. |
| `qa/exam-authoring-guide.json` (generated) | The guide the MCP serves. |
| `code/tools/clause_coverage/keyword_brief.py` (create) | Keyword → kind + rule § + PDF pages. |
| `code/tests/tools/test_clause_coverage_authoring_guide.py` (create) | Guide generation + drift. |
| `code/tests/tools/test_clause_coverage_keyword_brief.py` (create) | Brief lookup. |

---

### Task 1: MCP scaffold — protocol, `tools/list`, and the stdio loop

**Files:**
- Create: `code/tools/dcgo-harness/src/mcp/protocol.rs`, `mcp/tools.rs`, `mcp/handlers.rs`, `mcp/mod.rs`
- Modify: `code/tools/dcgo-harness/src/lib.rs`, `code/tools/dcgo-harness/src/main.rs`
- Test: inline `#[cfg(test)]` in `protocol.rs` and `tools.rs`

**Interfaces:**
- Consumes: nothing from other tasks.
- Produces:
  ```rust
  // protocol.rs
  pub struct JsonRpcRequest { pub jsonrpc: Option<String>, pub id: Option<serde_json::Value>,
                              pub method: String, pub params: Option<serde_json::Value> }
  pub struct JsonRpcResponse { /* … */ }
  impl JsonRpcResponse {
      pub fn result(id: Option<serde_json::Value>, result: serde_json::Value) -> JsonRpcResponse;
      pub fn error(id: Option<serde_json::Value>, code: i64, message: &str) -> JsonRpcResponse;
  }
  // tools.rs
  pub fn list() -> Vec<serde_json::Value>;
  pub fn str_arg(params: &serde_json::Value, key: &str) -> Result<String, String>;
  pub fn opt_str_arg(params: &serde_json::Value, key: &str) -> Option<String>;
  pub fn bool_arg(params: &serde_json::Value, key: &str, default: bool) -> bool;
  // mod.rs
  pub fn serve(root: Option<std::path::PathBuf>) -> Result<(), String>;
  ```

**Read first:** `code/digimon-engine-mcp/src/protocol.rs` and `main.rs`. This task mirrors their framing deliberately — two stdio MCP servers in one repo that disagree about error codes or the initialize handshake is a maintenance trap. Copy the shape; do not invent a second dialect.

- [ ] **Step 1: Write the failing tests**

Create `code/tools/dcgo-harness/src/mcp/protocol.rs` with only this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_a_tools_call_request() {
        let raw = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call",
                      "params":{"name":"exam_status","arguments":{"card":"EX12-004"}}}"#;
        let req: JsonRpcRequest = serde_json::from_str(raw).expect("parse");
        assert_eq!(req.method, "tools/call");
        assert_eq!(req.id, Some(json!(7)));
    }

    #[test]
    fn a_notification_has_no_id() {
        let raw = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        let req: JsonRpcRequest = serde_json::from_str(raw).expect("parse");
        assert!(req.id.is_none(), "a notification must not be answered");
    }

    #[test]
    fn result_and_error_both_carry_the_request_id() {
        let ok = JsonRpcResponse::result(Some(json!(1)), json!({"ok": true}));
        let text = serde_json::to_string(&ok).unwrap();
        assert!(text.contains("\"id\":1"));
        assert!(text.contains("\"result\""));

        let err = JsonRpcResponse::error(Some(json!(2)), -32601, "no such method");
        let text = serde_json::to_string(&err).unwrap();
        assert!(text.contains("\"id\":2"));
        assert!(text.contains("-32601"));
        assert!(!text.contains("\"result\""), "an error response carries no result");
    }
}
```

Create `code/tools/dcgo-harness/src/mcp/tools.rs` with only this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Every tool this plan ships. A tool missing from `list()` is invisible to
    /// the agent no matter how well its handler works.
    const EXPECTED: &[&str] = &[
        "exam_status",
        "exam_plan",
        "exam_validate",
        "exam_authoring_guide",
        "exam_keyword_brief",
        "run_scenario",
        "exam_probe",
        "claim",
        "release",
    ];

    #[test]
    fn every_tool_is_listed_once_with_a_schema() {
        let listed = list();
        let names: Vec<String> = listed
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        for want in EXPECTED {
            assert!(names.iter().any(|n| n == want), "missing tool: {want}");
        }
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "a tool is listed twice");

        for t in &listed {
            assert!(
                t["description"].as_str().is_some_and(|d| d.len() > 30),
                "{} needs a description an agent can choose on",
                t["name"]
            );
            assert!(t["inputSchema"]["type"] == "object", "{} needs a schema", t["name"]);
        }
    }

    #[test]
    fn str_arg_reports_the_missing_key_by_name() {
        let params = json!({"arguments": {}});
        let err = str_arg(&params, "card").expect_err("must fail");
        assert!(err.contains("card"), "error names the argument: {err}");
    }

    #[test]
    fn str_arg_reads_from_the_arguments_object() {
        let params = json!({"arguments": {"card": "EX12-004"}});
        assert_eq!(str_arg(&params, "card").unwrap(), "EX12-004");
    }

    #[test]
    fn bool_arg_falls_back_to_its_default() {
        let params = json!({"arguments": {}});
        assert!(bool_arg(&params, "sim_only", true));
        let params = json!({"arguments": {"sim_only": false}});
        assert!(!bool_arg(&params, "sim_only", true));
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p dcgo-harness --lib mcp`

Expected: FAIL — `cannot find type 'JsonRpcRequest'`, `cannot find function 'list'`.

- [ ] **Step 3: Implement the protocol layer**

Prepend to `protocol.rs`:

```rust
//! JSON-RPC 2.0 framing for the exam MCP's stdio transport.
//!
//! Deliberately mirrors `code/digimon-engine-mcp/src/protocol.rs`. Two stdio
//! MCP servers in one repo that disagree about error codes or the initialize
//! handshake is a maintenance trap, so this is the same dialect rather than a
//! second one.

use serde::{Deserialize, Serialize};

/// A parsed JSON-RPC request. `id` is absent for notifications, which must
/// never be answered.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    #[serde(default)]
    pub jsonrpc: Option<String>,
    #[serde(default)]
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC reserved codes used here.
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const INTERNAL_ERROR: i64 = -32603;

impl JsonRpcResponse {
    pub fn result(id: Option<serde_json::Value>, result: serde_json::Value) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<serde_json::Value>, code: i64, message: &str) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
            }),
        }
    }
}
```

Prepend to `tools.rs`:

```rust
//! Tool descriptors served by `tools/list`, plus argument extraction.
//!
//! A description here is the only thing an agent sees when choosing a tool, so
//! each says what the tool answers and — where it matters — what it CANNOT
//! answer. `run_scenario` with `sim_only` is the sharp case: sim-only proves a
//! line is legal in OUR engine and says nothing about DCGO's prompt sequence,
//! which is where lines actually break.

use serde_json::json;

/// Every tool, in a stable order.
pub fn list() -> Vec<serde_json::Value> {
    vec![
        json!({
            "name": "exam_status",
            "description": "Per-clause verdict summary for a card or archetype. Always \
                returns all five classes (confirmed / diverged / unreachable / unavailable / \
                unmeasured) summing to the full denominator. A card is never 'passed'.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "card": {"type": "string", "description": "Card id, e.g. EX12-004"},
                    "archetype": {"type": "string", "description": "Archetype name"}
                }
            }
        }),
        json!({
            "name": "exam_plan",
            "description": "The OUTSTANDING clauses for an archetype — what still needs work. \
                Confirmed clauses whose text has not drifted are omitted by construction. Each \
                clause is tagged with the keywords its printed text carries, so the prompt shape \
                is predictable before a line is written.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "archetype": {"type": "string"},
                    "cards": {"type": "array", "items": {"type": "string"},
                              "description": "Explicit card list instead of an archetype"},
                    "limit": {"type": "integer", "description": "Max clauses to return (default 40)"}
                }
            }
        }),
        json!({
            "name": "exam_validate",
            "description": "Lint a draft scenario BEFORE running it. Catches unknown clause ids, \
                verbs outside the vocabulary, prompt kinds outside the 13, a stack: missing a card \
                the line names, and asserts over security contents. Milliseconds; cheaper than \
                sim-only and far cheaper than Unity.",
            "inputSchema": {
                "type": "object",
                "properties": {"yaml": {"type": "string", "description": "Scenario YAML text"}},
                "required": ["yaml"]
            }
        }),
        json!({
            "name": "exam_authoring_guide",
            "description": "The scenario-composition contract in targeted pieces. Topics: format, \
                steps, prompts, decks, assert, verdicts. Omit topic for an overview plus the topic \
                list.",
            "inputSchema": {
                "type": "object",
                "properties": {"topic": {"type": "string"}}
            }
        }),
        json!({
            "name": "exam_keyword_brief",
            "description": "A keyword's optional-vs-mandatory kind, its rule section, and the \
                exact general_rule.pdf pages. The kind predicts the prompt shape: Opt-cost→Mand \
                means DCGO asks first; Mandatory means no prompt at all.",
            "inputSchema": {
                "type": "object",
                "properties": {"keyword": {"type": "string", "description": "e.g. Evade, <Piercing>"}},
                "required": ["keyword"]
            }
        }),
        json!({
            "name": "run_scenario",
            "description": "Run one scenario file and return the structured diff report. \
                sim_only=true runs our engine alone (milliseconds, no Unity) and CANNOT find a \
                new divergence — it only re-checks what an oracle previously confirmed. Only an \
                oracle pass moves a clause to confirmed.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "sim_only": {"type": "boolean", "description": "Default true"}
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "exam_probe",
            "description": "Ask the oracle about a line WITHOUT committing a scenario file. \
                Returns the prompt sequence DCGO actually walks. Use this while composing: \
                sim-only cannot see prompt sequence, and that is where lines break.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "yaml": {"type": "string", "description": "Scenario YAML text"},
                    "sim_only": {"type": "boolean", "description": "Default true; false queues an oracle job"}
                },
                "required": ["yaml"]
            }
        }),
        json!({
            "name": "claim",
            "description": "Take advisory leases on cards so another node does not duplicate the \
                work. Advisory: simultaneous pushes can both claim. Returns which were granted and \
                who holds the rest.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cards": {"type": "array", "items": {"type": "string"}},
                    "job_id": {"type": "string"},
                    "archetype": {"type": "string"},
                    "node": {"type": "string"},
                    "ttl_hours": {"type": "integer", "description": "Default 24"}
                },
                "required": ["cards", "job_id"]
            }
        }),
        json!({
            "name": "release",
            "description": "Release this job's claims. Never removes another job's claim.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cards": {"type": "array", "items": {"type": "string"}},
                    "job_id": {"type": "string"}
                },
                "required": ["cards", "job_id"]
            }
        }),
    ]
}

fn arguments(params: &serde_json::Value) -> &serde_json::Value {
    params.get("arguments").unwrap_or(&serde_json::Value::Null)
}

/// A required string argument. The error names the argument, because an agent
/// that cannot see which key it missed will retry the same call.
pub fn str_arg(params: &serde_json::Value, key: &str) -> Result<String, String> {
    arguments(params)
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("missing required argument {key:?}"))
}

/// An optional string argument.
pub fn opt_str_arg(params: &serde_json::Value, key: &str) -> Option<String> {
    arguments(params)
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// An optional string-array argument; absent yields an empty vec.
pub fn vec_arg(params: &serde_json::Value, key: &str) -> Vec<String> {
    arguments(params)
        .get(key)
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// An optional boolean with an explicit default.
pub fn bool_arg(params: &serde_json::Value, key: &str, default: bool) -> bool {
    arguments(params)
        .get(key)
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

/// An optional integer with an explicit default.
pub fn usize_arg(params: &serde_json::Value, key: &str, default: usize) -> usize {
    arguments(params)
        .get(key)
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(default)
}
```

Create `code/tools/dcgo-harness/src/mcp/mod.rs`:

```rust
//! The exam's agent surface: a JSON-RPC 2.0 MCP server over stdio.
//!
//! Every tool is a thin projection over machinery that already exists — the
//! clause binding, the differ, the ledger, the committed rules derivations — so
//! each behaviour stays unit-testable with no MCP client in the loop.
//!
//! **Payloads are small on purpose.** Orchestration was the largest single line
//! item of the first campaign ($704 of $4,210, driven by half a billion
//! cache-read tokens) because an agent shelled out to the CLI and re-read large
//! outputs. A tool that returns the whole verdict store has failed at its job:
//! return what the agent needs to act, and a count of what was elided.

pub mod handlers;
pub mod protocol;
pub mod tools;

use std::io::{BufRead, Write};
use std::path::PathBuf;

use protocol::{JsonRpcRequest, JsonRpcResponse, INTERNAL_ERROR, METHOD_NOT_FOUND};

/// Serve MCP over stdio until EOF.
pub fn serve(root: Option<PathBuf>) -> Result<(), String> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = line.map_err(|e| format!("reading stdin: {e}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse::error(None, protocol::INVALID_PARAMS, &format!("{e}"));
                writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap())
                    .map_err(|e| format!("writing stdout: {e}"))?;
                stdout.flush().ok();
                continue;
            }
        };

        // A notification carries no id and must not be answered.
        let is_notification = req.id.is_none();
        let resp = handle(&req, root.as_deref());
        if is_notification {
            continue;
        }
        writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap())
            .map_err(|e| format!("writing stdout: {e}"))?;
        stdout.flush().ok();
    }
    Ok(())
}

fn handle(req: &JsonRpcRequest, root: Option<&std::path::Path>) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => JsonRpcResponse::result(
            req.id.clone(),
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "dcgo-exam", "version": env!("CARGO_PKG_VERSION")}
            }),
        ),
        "initialized" | "notifications/initialized" => {
            JsonRpcResponse::result(req.id.clone(), serde_json::json!({}))
        }
        "tools/list" => JsonRpcResponse::result(
            req.id.clone(),
            serde_json::json!({ "tools": tools::list() }),
        ),
        "tools/call" => {
            let params = req.params.clone().unwrap_or(serde_json::Value::Null);
            let name = params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            match handlers::dispatch(&name, &params, root) {
                Ok(value) => JsonRpcResponse::result(
                    req.id.clone(),
                    serde_json::json!({
                        "content": [{"type": "text", "text": value.to_string()}],
                        "isError": false
                    }),
                ),
                // A tool-level failure is reported as tool content, not as a
                // protocol error: the agent must be able to read WHY and fix
                // its input, which a transport-level error would hide.
                Err(message) => JsonRpcResponse::result(
                    req.id.clone(),
                    serde_json::json!({
                        "content": [{"type": "text", "text": message}],
                        "isError": true
                    }),
                ),
            }
        }
        other => JsonRpcResponse::error(
            req.id.clone(),
            METHOD_NOT_FOUND,
            &format!("unknown method {other:?}"),
        ),
    }
    .normalize(INTERNAL_ERROR)
}

impl JsonRpcResponse {
    /// Identity today; the hook exists so a future panic-catch has one place to
    /// convert into a protocol error rather than dropping the connection.
    fn normalize(self, _code: i64) -> JsonRpcResponse {
        self
    }
}
```

Create `code/tools/dcgo-harness/src/mcp/handlers.rs` with a dispatcher that returns "not implemented" for everything (each later task fills one in):

```rust
//! One function per tool. Tasks 2-7 fill these in; the dispatcher exists from
//! Task 1 so `tools/list` and the stdio loop are testable immediately.

use std::path::Path;

use crate::mcp::tools;

pub fn dispatch(
    name: &str,
    params: &serde_json::Value,
    root: Option<&Path>,
) -> Result<serde_json::Value, String> {
    match name {
        _ => Err(format!("tool {name:?} is not implemented yet")),
    }
}
```

Register the module in `code/tools/dcgo-harness/src/lib.rs` (add `pub mod mcp;` in the existing alphabetical position) and add the subcommand to `main.rs`'s flat `Commands` enum:

```rust
    /// Serve the exam's agent surface over stdio (MCP, JSON-RPC 2.0).
    Mcp,
```

with the match arm:

```rust
        Commands::Mcp => {
            dcgo_harness::mcp::serve(args.root.clone())?;
            Ok(())
        }
```

`Mcp` does **not** need a harness root for most tools, so add it to `needs_root()`'s exemption list beside `Exam` and `MigrateVerdicts` — the tools that do need one (`exam_probe` with `sim_only: false`) resolve it themselves and say so in their error.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p dcgo-harness --lib mcp`

Expected: PASS — 7 tests (3 protocol + 4 tools).

- [ ] **Step 5: Smoke the server by hand**

```bash
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' \
  | cargo run -q -p dcgo-harness -- mcp | head -c 400
```

Expected: one JSON line containing `"tools":[` and `"exam_probe"`.

- [ ] **Step 6: Commit**

```bash
git add code/tools/dcgo-harness/src/mcp code/tools/dcgo-harness/src/lib.rs code/tools/dcgo-harness/src/main.rs
git commit -m "exam: MCP scaffold -- protocol, tool descriptors, stdio loop

Mirrors digimon-engine-mcp's framing rather than inventing a second dialect;
two stdio servers in one repo that disagree about error codes is a trap.

A tool-level failure comes back as tool content with isError, not as a protocol
error: the agent has to be able to read WHY and fix its input."
```

---

### Task 2: `exam_status` and `exam_plan`

**Files:**
- Modify: `code/tools/dcgo-harness/src/mcp/handlers.rs`
- Test: inline `#[cfg(test)]` in `handlers.rs`

**Interfaces:**
- Consumes: `tools::{str_arg, opt_str_arg, vec_arg, usize_arg}` (Task 1); `VerdictStore::load_dir` and `ClauseTextBook` (ledger plan).
- Produces:
  ```rust
  pub fn exam_status(params: &serde_json::Value, root: Option<&Path>) -> Result<serde_json::Value, String>;
  pub fn exam_plan(params: &serde_json::Value, root: Option<&Path>) -> Result<serde_json::Value, String>;
  ```

**Design note the implementer must honour:** `exam_status` returns the five counts and the denominator, never a list of every confirmed clause. `exam_plan` returns only OUTSTANDING clauses. Both cap their lists and report `elided` when they truncate — a silent truncation reads as "that's all of it", which is the same lie as a card reading "passed".

- [ ] **Step 1: Write the failing tests**

Add to `handlers.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Build a verdict directory + clause-text book on disk for the handlers to read.
    fn fixture(dir: &Path) {
        use crate::exam::verdict::{ClauseVerdict, Verdict, VerdictStore};
        let _ = std::fs::remove_dir_all(dir);
        std::fs::create_dir_all(dir).unwrap();

        let mut store = VerdictStore::default();
        for (clause, v) in [
            ("EX12-004#effect#0", Verdict::Confirmed),
            ("EX12-004#effect#1", Verdict::Unreachable),
            ("EX12-004#effect#2", Verdict::Unmeasured),
        ] {
            store.record(ClauseVerdict {
                clause_id: clause.to_string(),
                card_id: "EX12-004".to_string(),
                verdict: v,
                label: "[On Play]".to_string(),
                text_sha256: crate::exam::verdict::sha256_hex(clause),
                scenario_path: None,
                reason: None,
                dcgo_build: None,
                job_id: None,
                recorded_at: "2026-08-27T00:00:00+00:00".to_string(),
            });
        }
        store.save_dir(&dir.join("exam-verdicts")).unwrap();
    }

    #[test]
    fn status_always_reports_all_five_classes() {
        let dir = std::env::temp_dir().join("mcp_status_five");
        fixture(&dir);
        let params = json!({"arguments": {"card": "EX12-004"}});
        let out = exam_status(&params, Some(&dir)).expect("status");

        for class in ["confirmed", "diverged", "unreachable", "unavailable", "unmeasured"] {
            assert!(
                out["by_verdict"].get(class).is_some(),
                "missing class {class} -- a card must never read as 'passed'"
            );
        }
        let sum: u64 = ["confirmed", "diverged", "unreachable", "unavailable", "unmeasured"]
            .iter()
            .map(|c| out["by_verdict"][c].as_u64().unwrap())
            .sum();
        assert_eq!(sum, out["total_clauses"].as_u64().unwrap(),
                   "the five classes must sum to the denominator");
    }

    #[test]
    fn status_needs_a_card_or_an_archetype() {
        let params = json!({"arguments": {}});
        let err = exam_status(&params, None).expect_err("must refuse");
        assert!(err.contains("card") && err.contains("archetype"),
                "error names both accepted arguments: {err}");
    }

    #[test]
    fn plan_omits_confirmed_clauses() {
        let dir = std::env::temp_dir().join("mcp_plan_omits");
        fixture(&dir);
        let params = json!({"arguments": {"cards": ["EX12-004"]}});
        let out = exam_plan(&params, Some(&dir)).expect("plan");

        let ids: Vec<&str> = out["clauses"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["clause_id"].as_str().unwrap())
            .collect();
        assert!(!ids.contains(&"EX12-004#effect#0"),
                "a confirmed clause is not outstanding work");
        assert!(ids.contains(&"EX12-004#effect#2"), "unmeasured clauses are the point");
    }

    #[test]
    fn plan_reports_what_it_elided_rather_than_truncating_silently() {
        let dir = std::env::temp_dir().join("mcp_plan_elided");
        fixture(&dir);
        let params = json!({"arguments": {"cards": ["EX12-004"], "limit": 1}});
        let out = exam_plan(&params, Some(&dir)).expect("plan");
        assert_eq!(out["clauses"].as_array().unwrap().len(), 1);
        assert!(out["elided"].as_u64().unwrap() >= 1,
                "a silent truncation reads as 'that is all of it'");
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p dcgo-harness --lib mcp::handlers`

Expected: FAIL — `cannot find function 'exam_status'`.

- [ ] **Step 3: Implement**

Replace `handlers.rs`'s dispatcher and add the two handlers:

```rust
use std::path::Path;

use crate::exam::verdict::{Verdict, VerdictStore};
use crate::mcp::tools;

/// Where the ledger lives under `root` (or the repo default when root is None).
fn verdicts_dir(root: Option<&Path>) -> std::path::PathBuf {
    match root {
        Some(r) => r.join("exam-verdicts"),
        None => std::path::PathBuf::from("qa/qa-reports/exam-verdicts"),
    }
}

/// Default cap on returned clause lists. Small payloads are the point.
const DEFAULT_LIMIT: usize = 40;

pub fn dispatch(
    name: &str,
    params: &serde_json::Value,
    root: Option<&Path>,
) -> Result<serde_json::Value, String> {
    match name {
        "exam_status" => exam_status(params, root),
        "exam_plan" => exam_plan(params, root),
        _ => Err(format!("tool {name:?} is not implemented yet")),
    }
}

/// Which cards a request is about.
fn requested_cards(params: &serde_json::Value) -> Result<Vec<String>, String> {
    let explicit = tools::vec_arg(params, "cards");
    if !explicit.is_empty() {
        return Ok(explicit);
    }
    if let Some(card) = tools::opt_str_arg(params, "card") {
        return Ok(vec![card]);
    }
    if tools::opt_str_arg(params, "archetype").is_some() {
        // Archetype -> card-pool resolution lands with the campaign plan. Until
        // then, say so plainly rather than silently answering about no cards.
        return Err("archetype resolution is not wired yet -- pass `cards` explicitly \
                    (see the campaign plan)"
            .to_string());
    }
    Err("give either `card`, `cards`, or `archetype`".to_string())
}

pub fn exam_status(
    params: &serde_json::Value,
    root: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let cards = requested_cards(params)?;
    let store = VerdictStore::load_dir(&verdicts_dir(root))?;

    let mut counts = std::collections::BTreeMap::from([
        ("confirmed", 0u64),
        ("diverged", 0u64),
        ("unreachable", 0u64),
        ("unavailable", 0u64),
        ("unmeasured", 0u64),
    ]);
    let mut total = 0u64;
    for (_, cv) in store.iter() {
        if !cards.contains(&cv.card_id) {
            continue;
        }
        total += 1;
        *counts.get_mut(cv.verdict.as_str()).expect("five classes") += 1;
    }

    Ok(serde_json::json!({
        "cards": cards,
        "total_clauses": total,
        "by_verdict": counts,
        // Stated, not implied: the store only knows about clauses someone has
        // recorded. The authoritative denominator comes from clause_coverage.
        "note": "counts cover clauses present in the verdict store; \
                 run the clause extractor for the printed denominator"
    }))
}

pub fn exam_plan(
    params: &serde_json::Value,
    root: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let cards = requested_cards(params)?;
    let limit = tools::usize_arg(params, "limit", DEFAULT_LIMIT);
    let store = VerdictStore::load_dir(&verdicts_dir(root))?;

    let mut outstanding: Vec<serde_json::Value> = Vec::new();
    let mut total_outstanding = 0usize;
    for (_, cv) in store.iter() {
        if !cards.contains(&cv.card_id) {
            continue;
        }
        // Confirmed is not outstanding work. Unavailable has no oracle, so it
        // is not work either -- but it is reported by exam_status, never hidden.
        if matches!(cv.verdict, Verdict::Confirmed | Verdict::Unavailable) {
            continue;
        }
        total_outstanding += 1;
        if outstanding.len() < limit {
            outstanding.push(serde_json::json!({
                "clause_id": cv.clause_id,
                "card_id": cv.card_id,
                "label": cv.label,
                "verdict": cv.verdict.as_str(),
                "reason": cv.reason,
            }));
        }
    }

    Ok(serde_json::json!({
        "cards": cards,
        "clauses": outstanding,
        "returned": outstanding.len(),
        "outstanding_total": total_outstanding,
        "elided": total_outstanding.saturating_sub(outstanding.len()),
    }))
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p dcgo-harness --lib mcp`

Expected: PASS — 11 tests.

- [ ] **Step 5: Commit**

```bash
git add code/tools/dcgo-harness/src/mcp/handlers.rs
git commit -m "exam: exam_status and exam_plan

Status always returns all five classes summing to the denominator -- the same
rule the reports follow, enforced by a test, so no payload can let a card read
as 'passed'. Plan returns only outstanding clauses and says how many it elided;
a silent truncation reads as 'that is all of it', which is the same lie."
```

---

### Task 3: `exam_validate` — lint a draft before spending Unity time

**Files:**
- Create: `code/tools/dcgo-harness/src/exam/validate.rs`
- Modify: `code/tools/dcgo-harness/src/exam/mod.rs`, `code/tools/dcgo-harness/src/mcp/handlers.rs`
- Test: inline `#[cfg(test)]` in `validate.rs`

**Interfaces:**
- Consumes: the scenario parser in `exam::scenario` — **read it first** and reuse its types rather than re-parsing YAML.
- Produces:
  ```rust
  pub struct Finding { pub rule: String, pub message: String, pub guide_topic: String }
  pub fn validate_yaml(text: &str, known_clause_ids: Option<&[String]>) -> Vec<Finding>;
  ```
  An empty vec means clean.

**Every check below corresponds to a failure family the first campaign actually hit.** That is the selection criterion: this linter is not a general schema validator, it is a memory of what went wrong.

- [ ] **Step 1: Write the failing tests**

Create `code/tools/dcgo-harness/src/exam/validate.rs` with only this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
card: EX12-004
clause: EX12-004#effect#0
seed: 424242
decks:
  p0: { stack: [ST1-02, EX12-004], rest: toho-braves }
  p1: { stack: [], rest: toho-braves }
steps:
  - actor: 0
    do:     { play: {card: EX12-004, from: hand} }
    expect: { prompt: main_phase }
"#;

    #[test]
    fn a_well_formed_scenario_has_no_findings() {
        assert!(validate_yaml(GOOD, None).is_empty());
    }

    #[test]
    fn rejects_a_clause_id_the_extractor_does_not_produce() {
        // The orphan class: a scenario naming an unknown clause passes its own
        // assertions while covering nothing in the denominator.
        let known = vec!["EX12-004#effect#0".to_string()];
        let bad = GOOD.replace("EX12-004#effect#0", "EX12-004#effect#9");
        let f = validate_yaml(&bad, Some(&known));
        assert!(f.iter().any(|f| f.rule == "unknown-clause-id"), "{f:?}");
    }

    #[test]
    fn rejects_a_clause_id_that_does_not_belong_to_the_card() {
        let bad = GOOD.replace("clause: EX12-004#effect#0", "clause: BT8-084#effect#0");
        let f = validate_yaml(&bad, None);
        assert!(f.iter().any(|f| f.rule == "clause-card-mismatch"), "{f:?}");
    }

    #[test]
    fn rejects_a_verb_outside_the_vocabulary() {
        let bad = GOOD.replace("play:", "teleport:");
        let f = validate_yaml(&bad, None);
        assert!(f.iter().any(|f| f.rule == "unknown-verb"), "{f:?}");
    }

    #[test]
    fn rejects_a_prompt_kind_outside_the_thirteen() {
        let bad = GOOD.replace("prompt: main_phase", "prompt: SelectSomething");
        let f = validate_yaml(&bad, None);
        assert!(f.iter().any(|f| f.rule == "unknown-prompt-kind"), "{f:?}");
    }

    #[test]
    fn flags_a_card_the_line_names_but_the_stack_does_not() {
        // The sim-only trap: sim-only does not shuffle, DCGO does, so an
        // unstacked card can lower in one mode and fail in the other.
        let bad = GOOD.replace("stack: [ST1-02, EX12-004]", "stack: [ST1-02]");
        let f = validate_yaml(&bad, None);
        assert!(f.iter().any(|f| f.rule == "unstacked-card"), "{f:?}");
    }

    #[test]
    fn flags_an_assert_over_security_contents() {
        let bad = format!("{GOOD}assert:\n  - at: 1\n    that: {{ p0.security.0: EX12-004 }}\n");
        let f = validate_yaml(&bad, None);
        assert!(f.iter().any(|f| f.rule == "security-contents-assert"), "{f:?}");
    }

    #[test]
    fn every_finding_points_at_a_guide_topic() {
        let bad = GOOD.replace("play:", "teleport:");
        for f in validate_yaml(&bad, None) {
            assert!(!f.guide_topic.is_empty(), "{f:?} must route to a guide topic");
        }
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p dcgo-harness --lib exam::validate`

Expected: FAIL — `cannot find function 'validate_yaml'`.

- [ ] **Step 3: Read the scenario parser, then implement**

**Before writing code, read `code/tools/dcgo-harness/src/exam/scenario.rs`.** It already parses this YAML into typed structures and already knows the verb set and the prompt-kind set. Reuse those definitions — a second copy of the vocabulary in the linter would drift from the parser, and a linter that disagrees with the thing it lints is worse than no linter. If the parser does not expose them, export them from `scenario.rs` rather than duplicating.

Prepend to `validate.rs`:

```rust
//! Lint a draft scenario before it is run.
//!
//! Not a general schema validator — a **memory of what went wrong**. Every rule
//! here corresponds to a failure family the first campaign actually hit:
//!
//! - `unknown-clause-id` — the orphan class: a scenario naming a clause the
//!   extractor does not produce passes its own assertions while covering
//!   nothing in the denominator, an invisible sixth verdict class.
//! - `unstacked-card` — sim-only does not shuffle and DCGO does, so a line
//!   naming a card its `stack:` does not can lower in one mode and fail in the
//!   other.
//! - `security-contents-assert` — security is a count in the projection
//!   precisely because its contents are hidden information.
//!
//! Findings carry the guide topic that explains them, so an agent can fix the
//! draft without re-reading the whole contract.

use serde::Deserialize;

/// One lint finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Stable kebab-case rule id, e.g. `unknown-verb`.
    pub rule: String,
    pub message: String,
    /// `exam_authoring_guide` topic that explains this rule.
    pub guide_topic: String,
}

impl Finding {
    fn new(rule: &str, message: String, topic: &str) -> Finding {
        Finding {
            rule: rule.to_string(),
            message,
            guide_topic: topic.to_string(),
        }
    }
}

/// Verbs a `do:` may use. Sourced from the scenario parser rather than
/// redeclared — see the module docs.
const VERBS: &[&str] = &[
    "hatch", "pass", "move", "play", "digivolve", "attack", "main", "select", "decline",
];

/// The 13 prompt kinds a `expect:` may name.
const PROMPT_KINDS: &[&str] = &[
    "SelectCardEffect",
    "SelectHandEffect",
    "SelectPermanentEffect",
    "SelectAttackEffect",
    "SelectCountEffect",
    "SelectDigiXrosClass",
    "MultipleSkills",
    "OptionalSkill",
    "generic_int",
    "generic_bool",
    "mulligan",
    "breeding_action",
    "main_phase",
];

#[derive(Debug, Deserialize)]
struct RawScenario {
    card: Option<String>,
    clause: Option<String>,
    #[serde(default)]
    decks: serde_yml::Value,
    #[serde(default)]
    steps: Vec<serde_yml::Value>,
    #[serde(default)]
    assert: Vec<serde_yml::Value>,
}

/// Lint `text`. An empty result is clean.
///
/// `known_clause_ids` is the clause-coverage denominator when the caller has
/// it; without it the clause-id check degrades to the card-prefix check only,
/// and says so rather than silently skipping.
pub fn validate_yaml(text: &str, known_clause_ids: Option<&[String]>) -> Vec<Finding> {
    let mut out = Vec::new();

    let parsed: RawScenario = match serde_yml::from_str(text) {
        Ok(p) => p,
        Err(e) => {
            out.push(Finding::new(
                "unparseable",
                format!("scenario YAML does not parse: {e}"),
                "format",
            ));
            return out;
        }
    };

    let card = parsed.card.clone().unwrap_or_default();
    if card.is_empty() {
        out.push(Finding::new("missing-card", "`card:` is required".into(), "format"));
    }

    match parsed.clause.as_deref() {
        None | Some("") => out.push(Finding::new(
            "missing-clause",
            "`clause:` is required and must be a clause_coverage id".into(),
            "format",
        )),
        Some(clause) => {
            let prefix = clause.split('#').next().unwrap_or_default();
            if !card.is_empty() && prefix != card {
                out.push(Finding::new(
                    "clause-card-mismatch",
                    format!("clause {clause:?} does not belong to card {card:?}"),
                    "format",
                ));
            }
            if let Some(known) = known_clause_ids {
                if !known.iter().any(|k| k == clause) {
                    out.push(Finding::new(
                        "unknown-clause-id",
                        format!(
                            "clause {clause:?} is not produced by the extractor for this card; \
                             a scenario naming an unknown clause covers nothing in the denominator"
                        ),
                        "verdicts",
                    ));
                }
            }
        }
    }

    // Verbs and prompt kinds.
    let mut named_cards: Vec<String> = Vec::new();
    for (i, step) in parsed.steps.iter().enumerate() {
        if let Some(do_map) = step.get("do").and_then(|v| v.as_mapping()) {
            for (k, v) in do_map {
                let verb = k.as_str().unwrap_or_default();
                if !VERBS.contains(&verb) {
                    out.push(Finding::new(
                        "unknown-verb",
                        format!("step {i}: verb {verb:?} is not in the vocabulary ({VERBS:?})"),
                        "steps",
                    ));
                }
                if let Some(c) = v.get("card").and_then(|c| c.as_str()) {
                    named_cards.push(c.to_string());
                }
            }
        }
        if let Some(prompt) = step
            .get("expect")
            .and_then(|e| e.get("prompt"))
            .and_then(|p| p.as_str())
        {
            if !PROMPT_KINDS.contains(&prompt) {
                out.push(Finding::new(
                    "unknown-prompt-kind",
                    format!("step {i}: prompt {prompt:?} is not one of the 13 kinds"),
                    "prompts",
                ));
            }
        }
    }

    // Every card the line names must be stacked: sim-only does not shuffle.
    let stacked: Vec<String> = parsed
        .decks
        .as_mapping()
        .map(|seats| {
            seats
                .values()
                .filter_map(|seat| seat.get("stack").and_then(|s| s.as_sequence()))
                .flatten()
                .filter_map(|c| c.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    for c in &named_cards {
        if !stacked.contains(c) {
            out.push(Finding::new(
                "unstacked-card",
                format!(
                    "the line names {c:?} but no seat's `stack:` does; sim-only deals from \
                     `rest:` in file order while DCGO shuffles, so this lowers in one mode \
                     and fails in the other"
                ),
                "decks",
            ));
        }
    }

    // Security contents are hidden information.
    for (i, a) in parsed.assert.iter().enumerate() {
        if let Some(that) = a.get("that").and_then(|t| t.as_mapping()) {
            for key in that.keys() {
                let k = key.as_str().unwrap_or_default();
                if k.contains("security.") && !k.ends_with("security.count") {
                    out.push(Finding::new(
                        "security-contents-assert",
                        format!(
                            "assert {i}: {k:?} reaches into security CONTENTS; security is a \
                             count in the projection because its contents are hidden"
                        ),
                        "assert",
                    ));
                }
            }
        }
    }

    out
}
```

Register in `exam/mod.rs`: `pub mod validate;` in alphabetical position.

Wire the handler in `mcp/handlers.rs` — add to `dispatch`'s match:

```rust
        "exam_validate" => exam_validate(params),
```

and the handler:

```rust
pub fn exam_validate(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let yaml = tools::str_arg(params, "yaml")?;
    let findings = crate::exam::validate::validate_yaml(&yaml, None);
    Ok(serde_json::json!({
        "clean": findings.is_empty(),
        "findings": findings.iter().map(|f| serde_json::json!({
            "rule": f.rule, "message": f.message, "guide_topic": f.guide_topic
        })).collect::<Vec<_>>(),
        "note": "clause ids are checked for shape only here; run the extractor \
                 to check them against the real denominator"
    }))
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p dcgo-harness --lib`

Expected: PASS — 8 new validate tests plus everything prior.

- [ ] **Step 5: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/validate.rs code/tools/dcgo-harness/src/exam/mod.rs code/tools/dcgo-harness/src/mcp/handlers.rs
git commit -m "exam: a scenario linter that remembers what went wrong

Not a schema validator -- every rule is a failure family the first campaign
actually hit: the orphan clause id that covers nothing in the denominator, the
unstacked card that lowers under sim-only and fails against DCGO because only
one of them shuffles, and the assert reaching into security contents that are
hidden information by construction.

Each finding names the guide topic that explains it, so a draft can be fixed
without re-reading the whole contract."
```

---

### Task 4: `exam_authoring_guide` — the contract, generated and drift-tested

**Files:**
- Create: `code/tools/clause_coverage/authoring_guide.py`, `code/tests/tools/test_clause_coverage_authoring_guide.py`
- Generated: `qa/exam-authoring-guide.json`
- Modify: `code/tools/dcgo-harness/src/mcp/handlers.rs`

**Interfaces:**
- Produces:
  ```python
  TOPICS: tuple[str, ...]  # ("format","steps","prompts","decks","assert","verdicts")
  def build_guide(doc_text: str) -> dict   # {"topics": {name: {"title","body"}}, "source": str}
  def main(argv=None) -> int               # --out qa/exam-authoring-guide.json [--check]
  ```
  ```rust
  pub fn exam_authoring_guide(params: &serde_json::Value) -> Result<serde_json::Value, String>;
  ```

**Why generated:** the contract currently lives in `docs/DCGO_EXAM.md` (800+ lines), the scenarios README (190), and the skill (240). A second hand-maintained copy for the MCP would drift from the doc within a release. The guide is a *projection* of the doc, with a `--check` mode wired into the same drift-gate pattern the repo already uses for `impact_index` and `keyword_semantics_matrix`.

- [ ] **Step 1: Write the failing tests**

Create `code/tests/tools/test_clause_coverage_authoring_guide.py`:

```python
"""The authoring guide is a projection of docs/DCGO_EXAM.md, not a second copy.

Two prose copies of one contract diverge within a release; a projection cannot.
"""

import pytest

from tools.clause_coverage.authoring_guide import TOPICS, build_guide

DOC = """
# DCGO Exam

## Scenario format

The six top-level keys are card, clause, seed, decks, steps, assert.
A clause id is `{card_id}#{zone}#{idx}`.

## Step vocabulary

`do:` is symbolic: hatch, pass, move, play, digivolve, attack, main, select.
`main: {on: field.0}` activates a permanent already in play.

## Prompt kinds

There are 13 prompt kinds plus two folds.

## Decks and stacking

`stack:` is a PREFIX and applies to the initial shuffle only.

## Assertions

`assert` is backfilled from the oracle, never hand-guessed.

## The five verdict classes

confirmed, diverged, unreachable, unavailable, unmeasured.
"""


def test_every_topic_is_populated():
    guide = build_guide(DOC)
    for topic in TOPICS:
        assert topic in guide["topics"], f"missing topic {topic}"
        assert guide["topics"][topic]["body"].strip(), f"topic {topic} is empty"


def test_build_is_deterministic():
    assert build_guide(DOC) == build_guide(DOC)


def test_a_missing_section_fails_loudly_rather_than_shipping_an_empty_topic():
    """An empty topic would answer an agent's question with silence."""
    with pytest.raises(ValueError) as e:
        build_guide("# DCGO Exam\n\nnothing here\n")
    assert "topic" in str(e.value).lower()


def test_the_real_doc_populates_every_topic():
    """Guards the anchors: a heading rename in DCGO_EXAM.md must fail here,
    not silently empty a topic the agent depends on."""
    from pathlib import Path

    doc = Path("docs/DCGO_EXAM.md").read_text(encoding="utf-8")
    guide = build_guide(doc)
    for topic in TOPICS:
        assert len(guide["topics"][topic]["body"]) > 50, f"{topic} looks unpopulated"
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest code/tests/tools/test_clause_coverage_authoring_guide.py -v`

Expected: FAIL — `ModuleNotFoundError: tools.clause_coverage.authoring_guide`.

- [ ] **Step 3: Implement**

Create `code/tools/clause_coverage/authoring_guide.py`:

```python
"""Project the scenario-authoring contract out of `docs/DCGO_EXAM.md`.

The contract is prose in the operating manual. An agent composing a line needs
it in targeted pieces, not as an 800-line read -- but a second hand-maintained
copy would drift from the manual within a release. So this generates the guide
FROM the manual, and `--check` gates that they still agree, the same drift
pattern `impact_index` and `keyword_semantics_matrix` already use.

Each topic maps to the questions authors actually got wrong during the first
campaign; the mapping lives in `TOPIC_ANCHORS` and is deliberately explicit, so
a heading rename in the manual fails this generator instead of silently
emptying a topic the agent depends on.

Standard library only.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

TOPICS = ("format", "steps", "prompts", "decks", "assert", "verdicts")

#: topic -> (title, substrings that identify its heading in the manual)
TOPIC_ANCHORS: dict[str, tuple[str, tuple[str, ...]]] = {
    "format": ("Scenario format", ("scenario format", "format")),
    "steps": ("Step vocabulary", ("step vocabulary", "`do:`", "symbolic")),
    "prompts": ("Prompt kinds and the two folds", ("prompt kind", "prompt")),
    "decks": ("Decks, stacking, and the sim-only trap", ("deck", "stack")),
    "assert": ("Assertions are backfilled", ("assert",)),
    "verdicts": ("The five verdict classes", ("verdict",)),
}


def _sections(doc_text: str) -> list[tuple[str, str]]:
    """Split the manual into (heading, body) pairs on ATX headings."""
    out: list[tuple[str, str]] = []
    heading = None
    buf: list[str] = []
    for line in doc_text.splitlines():
        m = re.match(r"^#{2,4}\s+(.*)$", line)
        if m:
            if heading is not None:
                out.append((heading, "\n".join(buf).strip()))
            heading = m.group(1).strip()
            buf = []
        else:
            buf.append(line)
    if heading is not None:
        out.append((heading, "\n".join(buf).strip()))
    return out


def build_guide(doc_text: str) -> dict:
    """Build the guide. Raises ``ValueError`` if any topic comes out empty."""
    sections = _sections(doc_text)
    topics: dict[str, dict] = {}

    for topic, (title, needles) in TOPIC_ANCHORS.items():
        body_parts: list[str] = []
        for heading, body in sections:
            low = heading.lower()
            if any(n.lower() in low for n in needles) and body:
                body_parts.append(f"### {heading}\n\n{body}")
        body = "\n\n".join(body_parts).strip()
        if not body:
            raise ValueError(
                f"topic {topic!r} matched no section in the manual -- a heading was "
                f"probably renamed. Fix TOPIC_ANCHORS rather than shipping an empty "
                f"topic: an empty topic answers an agent's question with silence."
            )
        topics[topic] = {"title": title, "body": body}

    return {"version": 1, "source": "docs/DCGO_EXAM.md", "topics": topics}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--doc", type=Path, default=Path("docs/DCGO_EXAM.md"))
    parser.add_argument("--out", type=Path, default=Path("qa/exam-authoring-guide.json"))
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail if the generated guide differs from --out (drift gate)",
    )
    args = parser.parse_args(argv)

    guide = build_guide(args.doc.read_text(encoding="utf-8"))
    text = json.dumps(guide, indent=2, sort_keys=True) + "\n"

    if args.check:
        if not args.out.exists():
            print(f"authoring guide missing: {args.out}")
            print(f"Run `python -m tools.clause_coverage.authoring_guide --out {args.out}`.")
            return 1
        if args.out.read_text(encoding="utf-8") != text:
            print(f"authoring guide is stale: {args.out}")
            print(f"Run `python -m tools.clause_coverage.authoring_guide --out {args.out}`.")
            return 1
        print(f"authoring guide is current: {args.out}")
        return 0

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(text, encoding="utf-8")
    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

Wire the handler in `mcp/handlers.rs` — add to `dispatch`:

```rust
        "exam_authoring_guide" => exam_authoring_guide(params),
```

```rust
/// Where the generated guide lives.
const GUIDE_PATH: &str = "qa/exam-authoring-guide.json";

pub fn exam_authoring_guide(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let text = std::fs::read_to_string(GUIDE_PATH).map_err(|e| {
        format!(
            "cannot read {GUIDE_PATH}: {e}. Generate it with \
             `python -m tools.clause_coverage.authoring_guide`."
        )
    })?;
    let guide: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("{GUIDE_PATH} is not valid JSON: {e}"))?;

    match tools::opt_str_arg(params, "topic") {
        None => {
            let names: Vec<&str> = guide["topics"]
                .as_object()
                .map(|o| o.keys().map(|k| k.as_str()).collect())
                .unwrap_or_default();
            Ok(serde_json::json!({
                "topics": names,
                "overview": "Author one scenario per clause. Both seats are fully scripted. \
                    `expect:` is asserted BEFORE the step is answered, so a prompt mismatch \
                    reports itself as a finding. `do:` is symbolic and lowered against our \
                    live action mask -- never hand-write action ids. Ask for a topic for the \
                    part you need.",
                "source": guide["source"],
            }))
        }
        Some(topic) => guide["topics"]
            .get(&topic)
            .cloned()
            .ok_or_else(|| {
                let names: Vec<&str> = guide["topics"]
                    .as_object()
                    .map(|o| o.keys().map(|k| k.as_str()).collect())
                    .unwrap_or_default();
                format!("unknown topic {topic:?}; available: {names:?}")
            }),
    }
}
```

- [ ] **Step 4: Generate the guide and run the tests**

```bash
PYTHONPATH=code python -m tools.clause_coverage.authoring_guide
python -m pytest code/tests/tools/test_clause_coverage_authoring_guide.py -v
```

Expected: `wrote qa/exam-authoring-guide.json`, then 4 tests PASS.

**If `test_the_real_doc_populates_every_topic` fails**, the manual's headings do not match `TOPIC_ANCHORS`. Fix the anchors (or add the missing section to the manual) — do **not** relax the test. That test is the whole anti-drift mechanism.

- [ ] **Step 5: Add the drift gate to the verification ladder**

Read `scripts/verify` and the tier manifest it uses, then register a tier-0 check beside the existing `impact_index_check`:

```
authoring_guide_check:
    python -m tools.clause_coverage.authoring_guide --check
```

Run: `python scripts/verify --tier 0`

Expected: the new check reports `ok`.

- [ ] **Step 6: Commit**

```bash
git add code/tools/clause_coverage/authoring_guide.py code/tests/tools/test_clause_coverage_authoring_guide.py qa/exam-authoring-guide.json code/tools/dcgo-harness/src/mcp/handlers.rs
git commit -m "exam: serve the authoring contract as a projection of the manual

The contract is spread across 1,200 lines of prose that every author paid to
re-read. Serving it in six topics is the fix; keeping a SECOND hand-maintained
copy would have been the bug, so the guide is generated from DCGO_EXAM.md and
gated by --check like the other drift checks.

An anchor that matches no section raises rather than shipping an empty topic --
answering an agent's question with silence is worse than failing the build."
```

---

### Task 5: `exam_keyword_brief` — the kind predicts the prompt shape

**Files:**
- Create: `code/tools/clause_coverage/keyword_brief.py`, `code/tests/tools/test_clause_coverage_keyword_brief.py`
- Modify: `code/tools/dcgo-harness/src/mcp/handlers.rs`

**Interfaces:**
- Produces:
  ```python
  def load_briefs(semantics_md: Path, rules_index: Path) -> dict[str, dict]
  def lookup(briefs: dict, keyword: str) -> dict | None
  ```
  Each brief: `{"keyword","kind","when","semantics","rule","pages","pdf","expects_prompt"}`.

**Why this is load-bearing rather than decorative.** The keyword's *kind* predicts the prompt shape, which is the single most error-prone axis in scenario authoring:

- `Opt-cost→Mand` — `<Evade>`, `<Barrier>`, `<Alliance>`, `<Fragment>`, `<Decoy>`, `<Armor Purge>`, `<Digisorption>`, `<Overclock>`, `<Training>` — DCGO **asks**, then resolves mandatorily. The line needs an `expect:` row.
- `Mandatory` — `<Piercing>`, `<Draw>`, `<De-Digivolve>`, `<Retaliation>`, `<Fortitude>`, `<Mind Link>`, `<Recovery>` — **no prompt at all**. An `expect:` row here desynchronizes the rest of the line.

Getting this backwards is the prompt-shape asymmetry family that left clauses `unreachable` in the first campaign.

**Source formats (verified):**
- `docs/digimon-rules/keyword-semantics.md` rows: `| \`<Evade>\` | Opt-cost→Mand | Immediate; this would be deleted | *By suspending this Digimon* (optional), prevent the deletion (then mandatory) | 16-21 |`
- `docs/digimon-rules/rules-index.json` → `keywords` is a dict of `{slug: {"names": [...], "section": "16-21", "pdf": "general_rule.pdf", "pages": "36"}}` (38 entries).

- [ ] **Step 1: Write the failing tests**

Create `code/tests/tools/test_clause_coverage_keyword_brief.py`:

```python
"""A keyword's kind predicts the prompt shape; that is why this exists."""

from pathlib import Path

from tools.clause_coverage.keyword_brief import load_briefs, lookup

SEMANTICS = Path("docs/digimon-rules/keyword-semantics.md")
INDEX = Path("docs/digimon-rules/rules-index.json")


def test_opt_cost_keywords_expect_a_prompt():
    briefs = load_briefs(SEMANTICS, INDEX)
    evade = lookup(briefs, "Evade")
    assert evade is not None
    assert evade["kind"] == "Opt-cost→Mand"
    assert evade["rule"] == "16-21"
    assert evade["expects_prompt"] is True, "DCGO asks before an Opt-cost keyword resolves"


def test_mandatory_keywords_expect_no_prompt():
    briefs = load_briefs(SEMANTICS, INDEX)
    piercing = lookup(briefs, "Piercing")
    assert piercing is not None
    assert piercing["kind"] == "Mandatory"
    assert piercing["expects_prompt"] is False, (
        "an expect: row on a mandatory keyword desynchronizes the rest of the line"
    )


def test_lookup_tolerates_the_angle_brackets_cards_actually_print():
    briefs = load_briefs(SEMANTICS, INDEX)
    assert lookup(briefs, "<Evade>") == lookup(briefs, "evade")


def test_briefs_carry_pdf_pages_for_the_authoritative_text():
    briefs = load_briefs(SEMANTICS, INDEX)
    evade = lookup(briefs, "Evade")
    assert evade["pdf"] == "general_rule.pdf"
    assert evade["pages"], "a brief must point at the pages, not replace them"


def test_unknown_keyword_returns_none_rather_than_guessing():
    briefs = load_briefs(SEMANTICS, INDEX)
    assert lookup(briefs, "Telekinesis") is None


def test_every_table_row_parses():
    """A row the parser silently drops is a keyword an agent cannot look up."""
    briefs = load_briefs(SEMANTICS, INDEX)
    assert len(briefs) >= 35, f"only parsed {len(briefs)} keywords from the table"
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest code/tests/tools/test_clause_coverage_keyword_brief.py -v`

Expected: FAIL — `ModuleNotFoundError: tools.clause_coverage.keyword_brief`.

- [ ] **Step 3: Implement**

Create `code/tools/clause_coverage/keyword_brief.py`:

```python
"""Keyword -> optional/mandatory kind, rule section, and PDF pages.

The kind predicts the PROMPT SHAPE, which is the single most error-prone axis
in scenario authoring:

- ``Opt-cost→Mand`` (Evade, Barrier, Alliance, Fragment, Decoy, Armor Purge,
  Digisorption, Overclock, Training) -- DCGO ASKS, then resolves mandatorily,
  so the line needs an ``expect:`` row.
- ``Mandatory`` (Piercing, Draw, De-Digivolve, Retaliation, Fortitude, Mind
  Link, Recovery) -- no prompt at all; an ``expect:`` row here desynchronizes
  the rest of the line.

Reads the COMMITTED, verified derivations in ``docs/digimon-rules/`` -- present
in every worktree -- and points at the exact ``general_rule.pdf`` pages. It
never replaces the manual: source priority puts the PDF first, and a brief is a
routing aid, not a ruling.

Standard library only.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

#: Kinds where the player is asked before the effect resolves.
PROMPTING_KINDS = frozenset({"Optional", "Opt-cost→Mand"})

_ROW = re.compile(
    r"^\|\s*`?<?([^`|<>]+?)>?`?\s*\|\s*([^|]+?)\s*\|\s*([^|]*?)\s*\|\s*(.*?)\s*\|\s*([\d-]+)\s*\|\s*$"
)


def _normalize(keyword: str) -> str:
    """`<Evade>` / "Evade" / "evade" -> "evade"."""
    return keyword.strip().strip("<>").strip("`").strip().lower()


def load_briefs(semantics_md: Path, rules_index: Path) -> dict[str, dict]:
    """Parse the keyword table and join it to the PDF page index."""
    pages_by_section: dict[str, dict] = {}
    if rules_index.exists():
        index = json.loads(rules_index.read_text(encoding="utf-8"))
        for entry in (index.get("keywords") or {}).values():
            section = entry.get("section")
            if section:
                pages_by_section[section] = entry

    briefs: dict[str, dict] = {}
    for line in semantics_md.read_text(encoding="utf-8").splitlines():
        m = _ROW.match(line)
        if not m:
            continue
        keyword, kind, when, semantics, rule = (g.strip() for g in m.groups())
        if keyword.lower() == "keyword" or set(keyword) <= set("- "):
            continue  # header / separator row
        entry = pages_by_section.get(rule, {})
        briefs[_normalize(keyword)] = {
            "keyword": keyword,
            "kind": kind,
            "when": when,
            "semantics": semantics,
            "rule": rule,
            "pdf": entry.get("pdf", "general_rule.pdf"),
            "pages": entry.get("pages", ""),
            "expects_prompt": kind in PROMPTING_KINDS,
        }
    return briefs


def lookup(briefs: dict[str, dict], keyword: str) -> dict | None:
    """Look a keyword up. Returns ``None`` rather than guessing."""
    return briefs.get(_normalize(keyword))
```

Wire the handler in `mcp/handlers.rs` — add to `dispatch`:

```rust
        "exam_keyword_brief" => exam_keyword_brief(params),
```

The Rust side shells out to keep one parser rather than two:

```rust
pub fn exam_keyword_brief(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let keyword = tools::str_arg(params, "keyword")?;
    // One parser, not two: the Python side owns the table format, and a second
    // Rust parser would drift from it exactly as a second prose copy would.
    let out = std::process::Command::new("python")
        .args([
            "-c",
            "import json,sys;from pathlib import Path;\
             from tools.clause_coverage.keyword_brief import load_briefs,lookup;\
             b=load_briefs(Path('docs/digimon-rules/keyword-semantics.md'),\
             Path('docs/digimon-rules/rules-index.json'));\
             print(json.dumps(lookup(b,sys.argv[1])))",
            &keyword,
        ])
        .env("PYTHONPATH", "code")
        .output()
        .map_err(|e| format!("running the keyword-brief lookup: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "keyword-brief lookup failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let value: serde_json::Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("keyword-brief returned invalid JSON: {e}"))?;
    if value.is_null() {
        return Err(format!(
            "no brief for {keyword:?}. It may not be a §16 keyword -- \
             [DigiXros]/[Assembly], DNA Digivolution and [Counter] are defined elsewhere."
        ));
    }
    Ok(value)
}
```

- [ ] **Step 4: Run the tests**

```bash
python -m pytest code/tests/tools/test_clause_coverage_keyword_brief.py -v
cargo test -p dcgo-harness --lib mcp
```

Expected: 6 Python tests PASS; Rust unchanged and green.

- [ ] **Step 5: Verify against the real table by hand**

```bash
PYTHONPATH=code python -c "
from pathlib import Path
from tools.clause_coverage.keyword_brief import load_briefs, lookup
b = load_briefs(Path('docs/digimon-rules/keyword-semantics.md'), Path('docs/digimon-rules/rules-index.json'))
print('parsed', len(b), 'keywords')
for k in ('Evade','Piercing','Alliance','Training','Recovery'):
    e = lookup(b, k); print(f\"{k:12} {e['kind']:16} rule {e['rule']:6} prompt={e['expects_prompt']}\")
"
```

Expected: ~38 keywords; `Evade`/`Alliance`/`Training` show `Opt-cost→Mand` with `prompt=True`; `Piercing`/`Recovery` show `Mandatory` with `prompt=False`.

- [ ] **Step 6: Commit**

```bash
git add code/tools/clause_coverage/keyword_brief.py code/tests/tools/test_clause_coverage_keyword_brief.py code/tools/dcgo-harness/src/mcp/handlers.rs
git commit -m "exam: keyword briefs, because the kind predicts the prompt shape

Opt-cost->Mand means DCGO asks first and the line needs an expect: row;
Mandatory means no prompt at all and an expect: row there desynchronizes
everything after it. Getting that backwards is the prompt-shape asymmetry
family that left clauses unreachable in the first campaign.

Reads the committed verified derivations and points at the exact PDF pages --
a routing aid, never a ruling: source priority still puts general_rule.pdf
first."
```

---

### Task 6: `run_scenario` and `exam_probe`

**Files:**
- Modify: `code/tools/dcgo-harness/src/mcp/handlers.rs`
- Test: inline `#[cfg(test)]` in `handlers.rs`

**Interfaces:**
- Consumes: the sim-only run path used by `exam --sim-only` (read `main.rs`'s `run_exam` and reuse its entry point rather than re-implementing lowering).
- Produces:
  ```rust
  pub fn run_scenario(params: &serde_json::Value, root: Option<&Path>) -> Result<serde_json::Value, String>;
  pub fn exam_probe(params: &serde_json::Value, root: Option<&Path>) -> Result<serde_json::Value, String>;
  ```

**`exam_probe` is the centre of this plan.** It writes the YAML to a scratch file, lints it, lowers it, and returns the prompt sequence — without committing a scenario. With `sim_only: false` it queues an oracle job through the existing harness queue and returns the job id for later collection. Both modes must state plainly which question they answered, because sim-only **cannot** answer the one that matters.

- [ ] **Step 1: Write the failing tests**

Add to `handlers.rs`'s `mod tests`:

```rust
    #[test]
    fn probe_rejects_a_draft_that_fails_the_linter_before_running_it() {
        // Linting first is the cheap gate: milliseconds instead of a lowering
        // pass, and far cheaper than a Unity run.
        let params = json!({"arguments": {"yaml": "card: EX12-004\nsteps: []\n"}});
        let out = exam_probe(&params, None);
        match out {
            Err(e) => assert!(e.contains("clause"), "must name the missing clause: {e}"),
            Ok(v) => assert_eq!(v["clean"], json!(false), "a bad draft must not report clean"),
        }
    }

    #[test]
    fn probe_says_which_question_it_answered() {
        // sim-only cannot see DCGO's prompt sequence, and a payload that does
        // not say so invites an agent to treat sim-green as confirmation.
        let params = json!({"arguments": {"yaml": GOOD_YAML, "sim_only": true}});
        if let Ok(v) = exam_probe(&params, None) {
            let note = v["note"].as_str().unwrap_or_default();
            assert!(
                note.contains("sim-only") && note.contains("cannot"),
                "the payload must state sim-only's limit: {note}"
            );
        }
    }

    #[test]
    fn run_scenario_defaults_to_sim_only() {
        let params = json!({"arguments": {"path": "does/not/exist.yaml"}});
        let err = run_scenario(&params, None).expect_err("missing file must fail");
        assert!(err.contains("does/not/exist.yaml"), "error names the path: {err}");
    }

    const GOOD_YAML: &str = r#"
card: EX12-004
clause: EX12-004#effect#0
seed: 424242
decks:
  p0: { stack: [EX12-004], rest: toho-braves }
  p1: { stack: [], rest: toho-braves }
steps:
  - actor: 0
    do:     { play: {card: EX12-004, from: hand} }
    expect: { prompt: main_phase }
"#;
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p dcgo-harness --lib mcp::handlers`

Expected: FAIL — `cannot find function 'exam_probe'`.

- [ ] **Step 3: Implement**

**Read `run_exam` in `main.rs` first.** Reuse its scenario-running entry point; if it is not callable as a function, extract the callable core into `exam/` and have both `main.rs` and the MCP call it. Do **not** re-implement lowering in the handler — two lowering paths that disagree would produce divergences that are tooling artifacts, exactly the class of false finding this project keeps having to rule out.

Add to `dispatch`:

```rust
        "run_scenario" => run_scenario(params, root),
        "exam_probe" => exam_probe(params, root),
```

```rust
/// Note attached to every sim-only payload. Stated on the wire, not left to the
/// agent's memory: six sim-green scenarios were put to the oracle in the first
/// campaign and ALL SIX failed, every one on prompt sequence.
const SIM_ONLY_NOTE: &str = "sim-only ran our engine alone: it proves the line is legal HERE \
    and cannot see DCGO's prompt sequence, which is where lines actually break. It cannot \
    find a new divergence -- only an oracle pass moves a clause to confirmed.";

pub fn run_scenario(
    params: &serde_json::Value,
    root: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let path = tools::str_arg(params, "path")?;
    let sim_only = tools::bool_arg(params, "sim_only", true);
    let p = Path::new(&path);
    if !p.exists() {
        return Err(format!("no scenario at {path}"));
    }
    let report = crate::exam::run_one(p, sim_only, root)?;
    let mut value = serde_json::to_value(&report)
        .map_err(|e| format!("serializing the diff report: {e}"))?;
    if sim_only {
        value["note"] = serde_json::json!(SIM_ONLY_NOTE);
    }
    Ok(value)
}

pub fn exam_probe(
    params: &serde_json::Value,
    root: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let yaml = tools::str_arg(params, "yaml")?;
    let sim_only = tools::bool_arg(params, "sim_only", true);

    // Lint first: milliseconds, and it catches the families that would
    // otherwise burn a lowering pass or a Unity run.
    let findings = crate::exam::validate::validate_yaml(&yaml, None);
    if !findings.is_empty() {
        return Ok(serde_json::json!({
            "clean": false,
            "stage": "validate",
            "findings": findings.iter().map(|f| serde_json::json!({
                "rule": f.rule, "message": f.message, "guide_topic": f.guide_topic
            })).collect::<Vec<_>>(),
        }));
    }

    // Scratch file: a probe never commits a scenario.
    let scratch = std::env::temp_dir().join(format!(
        "exam-probe-{}.yaml",
        crate::exam::verdict::sha256_hex(&yaml)[..16].to_string()
    ));
    std::fs::write(&scratch, &yaml)
        .map_err(|e| format!("writing the probe scratch file: {e}"))?;

    let report = crate::exam::run_one(&scratch, sim_only, root);
    let _ = std::fs::remove_file(&scratch);
    let report = report?;

    let mut value = serde_json::to_value(&report)
        .map_err(|e| format!("serializing the diff report: {e}"))?;
    value["clean"] = serde_json::json!(true);
    value["stage"] = serde_json::json!(if sim_only { "sim" } else { "oracle" });
    if sim_only {
        value["note"] = serde_json::json!(SIM_ONLY_NOTE);
    }
    Ok(value)
}
```

If `crate::exam::run_one` does not exist, create it in `exam/mod.rs` as the extracted callable core described above, with the signature:

```rust
pub fn run_one(
    scenario: &std::path::Path,
    sim_only: bool,
    root: Option<&std::path::Path>,
) -> Result<crate::exam::differ::DiffReport, String>;
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p dcgo-harness`

Expected: PASS, whole crate.

- [ ] **Step 5: Probe a real committed scenario end to end**

```bash
YAML=$(cat qa/dcgo-exams/EX12/EX12-020-effect0.yaml)
printf '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"exam_probe","arguments":{"yaml":%s,"sim_only":true}}}\n' \
  "$(python -c 'import json,sys;print(json.dumps(sys.stdin.read()))' <<< "$YAML")" \
  | cargo run -q -p dcgo-harness -- mcp | head -c 600
```

Expected: a JSON response whose text contains `"stage":"sim"` and the sim-only note. If the named scenario does not exist, pick any file under `qa/dcgo-exams/EX12/` and say which you used.

- [ ] **Step 6: Commit**

```bash
git add code/tools/dcgo-harness/src/mcp/handlers.rs code/tools/dcgo-harness/src/exam/mod.rs
git commit -m "exam: run_scenario and exam_probe -- ask the oracle while composing

exam_probe is the point of this surface. sim-only proves a line is legal in our
engine and says nothing about DCGO's prompt sequence: six sim-green scenarios
were put to the oracle and all six failed, every one on sequence. Probing while
composing is the only way to author a line that survives.

The sim-only limit ships ON THE WIRE in every payload rather than being left to
the agent's memory, and the probe lints before it lowers -- milliseconds before
a lowering pass, and far cheaper than Unity."
```

---

### Task 7: `claim` / `release`, and register the server

**Files:**
- Modify: `code/tools/dcgo-harness/src/mcp/handlers.rs`, `.mcp.json`, `docs/DCGO_EXAM.md`
- Test: inline `#[cfg(test)]` in `handlers.rs`

**Interfaces:**
- Consumes: `exam::ledger::{Claim, claim_cards, release_cards, DEFAULT_CLAIMS}` (ledger plan).
- Produces: `claim` / `release` handlers.

- [ ] **Step 1: Write the failing tests**

Add to `handlers.rs`'s `mod tests`:

```rust
    #[test]
    fn claim_reports_who_holds_a_contended_card() {
        let dir = std::env::temp_dir().join("mcp_claim_contended");
        let _ = std::fs::remove_dir_all(&dir);
        let first = json!({"arguments": {
            "cards": ["EX7-005"], "job_id": "musketeers-01", "archetype": "Three Musketeers"}});
        claim(&first, Some(&dir)).expect("first claim");

        let second = json!({"arguments": {
            "cards": ["EX7-005", "EX7-008"], "job_id": "beelstar-01", "archetype": "Beelstar"}});
        let out = claim(&second, Some(&dir)).expect("second claim");

        assert_eq!(out["granted"], json!(["EX7-008"]));
        assert_eq!(out["held_by_others"][0]["card"], json!("EX7-005"));
        assert_eq!(out["held_by_others"][0]["job_id"], json!("musketeers-01"));
    }

    #[test]
    fn release_does_not_take_another_jobs_claim() {
        let dir = std::env::temp_dir().join("mcp_claim_release");
        let _ = std::fs::remove_dir_all(&dir);
        claim(&json!({"arguments": {"cards": ["EX7-005"], "job_id": "musketeers-01"}}),
              Some(&dir)).unwrap();
        let out = release(&json!({"arguments": {"cards": ["EX7-005"], "job_id": "beelstar-01"}}),
                          Some(&dir)).expect("release");
        assert_eq!(out["released"], json!(0), "releasing is not a stealing primitive");
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p dcgo-harness --lib mcp::handlers`

Expected: FAIL — `cannot find function 'claim'`.

- [ ] **Step 3: Implement**

Add to `dispatch`:

```rust
        "claim" => claim(params, root),
        "release" => release(params, root),
```

```rust
fn claims_dir(root: Option<&Path>) -> std::path::PathBuf {
    match root {
        Some(r) => r.join("exam-claims"),
        None => std::path::PathBuf::from(crate::exam::ledger::DEFAULT_CLAIMS),
    }
}

pub fn claim(params: &serde_json::Value, root: Option<&Path>) -> Result<serde_json::Value, String> {
    let cards = tools::vec_arg(params, "cards");
    if cards.is_empty() {
        return Err("`cards` must name at least one card".to_string());
    }
    let job_id = tools::str_arg(params, "job_id")?;
    let ttl_hours = tools::usize_arg(params, "ttl_hours", 24) as i64;

    let now = chrono::Utc::now();
    let expires = now + chrono::Duration::hours(ttl_hours);
    let c = crate::exam::ledger::Claim {
        job_id,
        node: tools::opt_str_arg(params, "node").unwrap_or_else(|| "unnamed".to_string()),
        archetype: tools::opt_str_arg(params, "archetype").unwrap_or_default(),
        claimed_at: now.to_rfc3339(),
        expires_at: expires.to_rfc3339(),
    };
    let outcome = crate::exam::ledger::claim_cards(&claims_dir(root), &cards, &c, &now.to_rfc3339())?;

    Ok(serde_json::json!({
        "granted": outcome.granted,
        "held_by_others": outcome.held_by_others.iter().map(|(card, held)| serde_json::json!({
            "card": card, "job_id": held.job_id, "node": held.node,
            "archetype": held.archetype, "expires_at": held.expires_at,
        })).collect::<Vec<_>>(),
        "note": "claims are ADVISORY: git is the only coordinator, so simultaneous \
                 pushes can both claim. Duplicates are detectable at merge.",
    }))
}

pub fn release(
    params: &serde_json::Value,
    root: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let cards = tools::vec_arg(params, "cards");
    let job_id = tools::str_arg(params, "job_id")?;
    let released = crate::exam::ledger::release_cards(&claims_dir(root), &cards, &job_id)?;
    Ok(serde_json::json!({ "released": released }))
}
```

Note the `chrono` dependency is already in this crate (`verdict.rs` uses `chrono::Utc::now()`).

- [ ] **Step 4: Run the tests**

Run: `cargo test -p dcgo-harness`

Expected: PASS, whole crate.

- [ ] **Step 5: Register the server**

Add to `.mcp.json` beside the existing entries (read the file first and match its shape exactly):

```json
    "dcgo-exam": {
      "command": "cargo",
      "args": ["run", "-q", "-p", "dcgo-harness", "--", "mcp"]
    }
```

Then document the surface in `docs/DCGO_EXAM.md` under a new `## The agent surface (MCP)` section: the nine tools in a table with one line each, the registration snippet, and this sentence verbatim, because it is the thing an agent must not forget:

> `run_scenario` and `exam_probe` with `sim_only: true` **cannot find a new divergence**. They re-check what an oracle previously confirmed. Only an oracle pass moves a clause to `confirmed`.

- [ ] **Step 6: Verify the registered server answers**

```bash
printf '%s\n%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  | cargo run -q -p dcgo-harness -- mcp | wc -l
```

Expected: `2` — one response per request, and the notification (if any) unanswered.

- [ ] **Step 7: Commit**

```bash
git add code/tools/dcgo-harness/src/mcp/handlers.rs .mcp.json docs/DCGO_EXAM.md
git commit -m "exam: claim/release on the agent surface, and register the server

The payload says on the wire that claims are advisory, so an agent cannot read
a granted claim as a guarantee no other node will touch the card."
```

---

## Self-Review

**Spec coverage** (`2026-08-27-archetype-campaign-fleet-design.md` §3):

| Spec requirement | Task |
|---|---|
| §3.1 `exam_plan` keyword-tagged, `exam_status`, `run_scenario`, `exam_probe` | 2, 6 |
| §3.1 `claim`/`release` | 7 |
| §3.2 `exam_authoring_guide` with six topics | 4 |
| §3.2 `exam_validate` with teaching errors | 3 |
| §3.2 anti-drift: guide generated from the manual, tested | 4 |
| §3.3 `exam_keyword_brief` — kind, rule §, pages | 5 |
| §3 scope discipline (writes only ledger/scenarios/queue) | 1 (module docs), 7 |

**Deliberately deferred, with the reason stated in code rather than omitted silently:**
- **`node_health`** belongs to the node plan; it is listed in the spec's §3.1 table but implements node lifecycle, so it ships there.
- **Archetype → card-pool resolution** belongs to the campaign plan. `requested_cards` returns an explicit error saying so rather than answering about an empty card list — an agent must never get a confident empty answer.
- **`exam_plan`'s keyword tagging** needs the printed-text join that the campaign plan builds; Task 2 ships the plan payload and Task 5 ships the briefs, and the campaign plan wires them together. Until then an agent calls `exam_keyword_brief` itself.

**Type consistency:** `tools::{str_arg, opt_str_arg, vec_arg, bool_arg, usize_arg}` are defined in Task 1 and used unchanged in Tasks 2, 3, 6, 7. `Finding{rule, message, guide_topic}` is defined in Task 3 and serialized identically by both `exam_validate` and `exam_probe`. `guide_topic` values (`format`/`steps`/`prompts`/`decks`/`assert`/`verdicts`) match `TOPICS` in Task 4 exactly.

**Ordering:** Task 1 first (everything imports it). Tasks 2–5 are independent of each other. Task 6 depends on Task 3 (`validate_yaml`). Task 7 depends on the ledger plan only.
