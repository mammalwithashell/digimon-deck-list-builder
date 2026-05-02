# digimon-dsl

Leaf crate holding the card-scripting DSL.

## Surface

- `loader` — parse YAML into `CardSpec`
- `validator` — semantic validation
- `compile` — CardSpec → CompiledCard IR lowering
- `compiled` — the compiled IR types
- `pack` — bincode-serialized pack with manifest + compiled cards
- `registry` — CardRegistry holds the compiled cards keyed by card_id
- `pretty` — canonical YAML pretty-printer
- `schema` — JSON Schema export

## Consumers

- `digimon-engine` — depends on digimon-dsl as runtime + build dep
- `tools/dsl-lint` — CLI linter
- `tools/dsl-schema-export` — JSON Schema exporter

## Phase status

- Phase 0 — schema + parse + validate + round-trip
- Phase 1a — cleanup + real cards.json adapter + dsl-lint --cross-check
- Phase 1b — AOT pipeline: CardSpec → CompiledCard → CardPack → embedded blob
- Phase 1c — engine integration: lower CompiledCard → Effect closures (next)
