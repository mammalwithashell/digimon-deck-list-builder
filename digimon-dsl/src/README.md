# DSL — Phase 0

Parse + validate + round-trip + JSON Schema export. No engine integration.

## Entry points

- `loader::load_file(path)` / `loader::load_dir(dir)` / `loader::load_dir_ok(dir)`
- `loader::cross_check(spec, db)`
- `validator::validate(spec, ctx)`
- `pretty::format_spec(spec)`
- `schema::export_json_schema()`
- `raw_rust_registry::RawRustRegistry` trait + `StubRegistry` test impl

## Status

Phase 0 exit criteria met per `tests/dsl/phase0_exit.rs`:
- 15/15 worked examples parse, validate, cross-check, round-trip.
- JSON Schema is deterministic and non-empty.

Next: Phase 1 plan — AOT lowering to `Effect` closures + `build.rs` + rkyv
blob + `from_embedded()` (see spec §7a).
