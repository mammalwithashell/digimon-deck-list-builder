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
            "description": "Per-clause verdict summary for a card (or an explicit list of \
                cards). Always returns all five classes (confirmed / diverged / unreachable / \
                unavailable / unmeasured) summing to the FULL PRINTED denominator — a \
                `clause_coverage extract` output, not just the clauses someone happened to \
                record a verdict for. A card is never 'passed'.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "card": {"type": "string", "description": "Card id, e.g. EX12-004"},
                    "cards": {"type": "array", "items": {"type": "string"},
                              "description": "Explicit card list instead of a single card"},
                    "clause_text_json": {"type": "string",
                        "description": "Path to a `clause_coverage extract` output supplying \
                            the printed denominator; defaults to the repo's tracked extract. \
                            When unreadable, the response degrades to a stored-rows-only count \
                            and says so via `denominator_source` — never a silent fallback."}
                }
            }
        }),
        json!({
            "name": "exam_plan",
            "description": "The OUTSTANDING clauses for a card (or an explicit list of cards) — \
                what still needs work, over the FULL PRINTED denominator. Confirmed clauses \
                whose text has not drifted, and unavailable clauses, are omitted by \
                construction; everything else is outstanding, including a clause with NO \
                stored verdict at all.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cards": {"type": "array", "items": {"type": "string"},
                              "description": "Explicit card list"},
                    "card": {"type": "string", "description": "Single card id instead of `cards`"},
                    "limit": {"type": "integer", "description": "Max clauses to return (default 40)"},
                    "clause_text_json": {"type": "string",
                        "description": "Path to a `clause_coverage extract` output supplying \
                            the printed denominator; defaults to the repo's tracked extract. \
                            When unreadable, the response degrades to a stored-rows-only count \
                            (a clause with no stored row cannot appear) and says so via \
                            `denominator_source`."}
                }
            }
        }),
        json!({
            "name": "exam_validate",
            "description": "Lint a draft scenario BEFORE running it. Catches unknown clause ids \
                (when a clause-text book is available — see `clause_text_json`), verbs outside \
                the vocabulary, prompt kinds outside the 13, a stack: missing a card the line \
                names, and asserts over security contents. Milliseconds; cheaper than sim-only \
                and far cheaper than Unity.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "yaml": {"type": "string", "description": "Scenario YAML text"},
                    "clause_text_json": {"type": "string",
                        "description": "Path to a `clause_coverage extract` output; defaults to \
                            the repo's tracked extract (see exam_status). Without one, the \
                            unknown-clause-id check degrades to a card-prefix check only."}
                },
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
                oracle pass moves a clause to confirmed. sim_only=false does NOT reach the \
                oracle today — it is refused with a clear error (same refusal as `exam_probe`; \
                there is no DCGO state sidecar behind a bare scenario path).",
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
            "description": "Try a line WITHOUT committing a scenario file. sim_only \n                (the DEFAULT, and today the ONLY working mode) lowers it in our engine \n                alone and CANNOT see DCGO's prompt sequence -- which is where lines \n                actually break, so a clean result here is NOT confirmation. sim_only=false \n                returns a clear error until an oracle node can be queued: to get a real \n                oracle answer today, submit the scenario through the harness queue and \n                diff against the sidecar it writes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "yaml": {"type": "string", "description": "Scenario YAML text"},
                    "sim_only": {"type": "boolean", "description": "Default true; false returns \
                        an error today (no oracle wiring yet) -- see the description above"}
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
        json!({
            "name": "node_health",
            "description": "Is this machine able to answer as an oracle? Reports every \
                preflight check with a remedy: the player, the action-space gate, whether the \
                harness is enabled, the queue, and whether a player is already running. Run \
                this BEFORE authoring -- a NO-GO discovered afterwards wastes the authoring.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "build": {"type": "string", "description": "Player build directory"}
                }
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
        "node_health",
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
    fn exam_status_and_exam_plan_do_not_advertise_archetype() {
        // Archetype resolution was never wired up (`requested_cards` in
        // `mcp::handlers` refuses it with a named error). Advertising
        // `archetype` in the schema while `exam_plan`'s description led with
        // it promised a capability the tool did not have; the fix is to stop
        // advertising it here, not to build resolution just to match the
        // schema. The handler still refuses an `archetype` argument passed
        // anyway (pinned in `mcp::handlers::tests`).
        let listed = list();
        for name in ["exam_status", "exam_plan"] {
            let tool = listed.iter().find(|t| t["name"] == name).unwrap();
            assert!(
                tool["inputSchema"]["properties"].get("archetype").is_none(),
                "{name} must not advertise `archetype` in its schema"
            );
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
