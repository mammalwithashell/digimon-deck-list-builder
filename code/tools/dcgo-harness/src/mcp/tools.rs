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
            "description": "Try a line WITHOUT committing a scenario file. sim_only \n                (the DEFAULT) lowers it in our engine only and CANNOT see DCGO's prompt \n                sequence; pass sim_only=false to ask the oracle, which is what returns \n                the prompt sequence DCGO actually walks. Use the oracle form while \n                composing: prompt sequence is where lines break, and sim-only is blind \n                to it.",
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
