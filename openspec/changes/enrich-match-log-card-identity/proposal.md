## Why

QA reads the human-readable match log to file bugs, but the log identifies cards inconsistently and often anonymously: attacks render as `attacked YOU with slot 0`, trashed/revealed cards show a bare ID (`trashed BT25-061`), some plays/digivolves show the ID with no name, memory swings from tamers are unattributed (`YOU gained 1 memory`), and effect targeting and non-security reveals produce no log line at all. The root cause is that the frontend formatter reconstructs card names from **live board state at render time** — which has moved on by the time the log is read — instead of from the event. The engine knows the card's id and name at the instant each event fires; that information is discarded and then unreliably rebuilt. Every QA report keys off this log, so making it precise is leverage on the whole QA effort.

## What Changes

- **Uniform `[CARD-ID: Name]` rendering** for every card-bearing log line, sourced from the event payload (not board state), with board lookup demoted to a last-resort fallback.
- **`Attack` events carry attacker and target identity** (id + name; target is the defending Digimon or `security`), eliminating `slot N`.
- **`Play` / `Digivolve` / `Trash` / `Mill` / `SecurityReveal` events carry `card_name`** alongside the id they already emit.
- **`MemoryChange` events carry an optional effect-source** (id + name) for effect-driven changes. Motivating case: tamer start-of-turn `+memory`; applies to all effect-sourced changes (gains and losses), not just tamers. Cost-payment and structural memory changes remain anonymous (the card is already named on the Play/Digivolve line; passing has no card source).
- **New generic `EffectTarget` event** — emitted at selection commit with the source effect card and the chosen target card(s). Fires for **all** targets, including forced / single-legal-target selections.
- **New reveal events** for the three reveal sites that have no event today (mirroring the DCGO recorder chokepoints, CLAUDE.md rule 27): reveal-deck-top, trash-from-deck-top reveal, and reveal-hand. (`SecurityReveal` already exists and is covered by the enrichment above.)
- **Both adapter wires updated in lockstep** — `event_to_dto` (desktop) and `event_to_pydict` (browser/server) pass the new fields/variants through identically, closing the recurring desktop-vs-browser DTO drift.

All `GameEvent` additions are additive (`#[non_exhaustive]` enum; new variants use the established default-skip pattern in existing consumers).

## Capabilities

### New Capabilities
- `match-log-card-identity`: The human-readable match-log rendering contract — every card-bearing line renders `[CARD-ID: Name]` from event-carried identity (not live board state), across the desktop adapter and the frontend formatter, with a defined fallback when identity is absent.

### Modified Capabilities
- `engine-event-emission`: Event payloads gain card identity — `Attack` carries attacker/target id+name; `Play`/`Digivolve`/`Trash`/`Mill`/`SecurityReveal` carry `card_name`; `MemoryChange` carries an optional effect-source id+name. Two new event families are added: a generic `EffectTarget` event (source effect + chosen targets) and reveal events for reveal-deck-top, trash-from-deck-top, and reveal-hand. The PyO3 binding surfaces all additions.

## Impact

- **Engine** (`code/digimon-engine/src/`): `events.rs` (variant fields + new variants); emission sites in `combat/mod.rs` (Attack identity, reveals), `game/memory.rs` + `effect_context/action/lifecycle.rs` (MemoryChange source threading), `game/mod.rs` + `combat/deletion.rs` (Trash/Mill name), `effect_context/selections.rs` (EffectTarget at selection commit), and the reveal sites.
- **Adapters**: `code/src-tauri/src/engine_commands.rs::event_to_dto` and `code/digimon-engine-py/src/lib.rs::event_to_pydict` — pass new fields/variants through; frontend `GameEvent` type (`code/frontend/src/types/game.ts`) gains the fields.
- **Frontend**: `code/frontend/src/utils/gameLogFormat.ts` (+ `gameEvents.ts` type aliases) render `[CARD-ID: Name]` and the new event lines.
- **Tests**: `code/digimon-engine/tests/event_emission/*` (new/extended emission assertions), `code/frontend/src/utils/gameLogFormat.test.ts` (rendering).
- **Open question (resolve in design)**: whether the DCGO recording schema (`docs/DCGO_RECORDING_SCHEMA.md`) and replay consumers should also carry the new event fields, or treat them as log-only and ignore them on replay.
