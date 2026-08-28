//! One function per tool. Tasks 2-7 fill these in; the dispatcher exists from
//! Task 1 so `tools/list` and the stdio loop are testable immediately.

use std::path::Path;

#[allow(unused_imports)]
use crate::mcp::tools;

pub fn dispatch(
    name: &str,
    _params: &serde_json::Value,
    _root: Option<&Path>,
) -> Result<serde_json::Value, String> {
    match name {
        _ => Err(format!("tool {name:?} is not implemented yet")),
    }
}
