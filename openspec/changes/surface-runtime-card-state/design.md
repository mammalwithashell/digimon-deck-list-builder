## Context

`digimon-engine/src/serialization.rs::to_ui_json` builds the frontend-facing per-permanent view. It already computes runtime values (DP and `dpBreakdown`, `keywords`/`keywordBreakdown`, per-source `dpContribution`, `isTop`, colors, level), but lines ~247-272 hard-code the effect-text fields:

```rust
"mainEffectText": "",       // per source
"inheritedEffectText": "",  // per source
...
"inheritedEffects": [],     // per permanent
```

The frontend `CardOverlay` stack inspector is already written to render `sources[].inheritedEffectText` and a permanent's `inheritedEffects`, so it currently shows the stack structure with blank effect text. The engine holds the printed text in `card_data[c.data_index]` (it already reads `card_id`/name from there). This is the frontend `to_ui_json` surface specifically — `live-game-surface` owns a separate tool-facing `view` module and is out of scope.

The companion change `add-ingame-card-preview` covers *printed* text via the metadata API for a right-click preview; this change is about the engine's own state being self-describing and **runtime-accurate** (e.g., which inherited effects are actually in force), which the metadata API cannot provide and which non-UI consumers (WS/`state_filter`, engine-state debugging) also benefit from.

## Goals / Non-Goals

**Goals:**
- Per-source `mainEffectText`/`inheritedEffectText` populated from engine card data.
- Permanent `inheritedEffects` reflects the inherited effects currently conferred by the stack (top card excluded), runtime-accurate where engine state alters them.
- No change to already-correct runtime fields (DP/keywords) or to the serialization's field shape.

**Non-Goals:**
- Any frontend change (the consumers already render these fields).
- The tool-facing `view` module / `FieldView` (`live-game-surface`).
- The card preview or the action log (separate changes).
- New effect-resolution semantics — only *surfacing* state that the engine already knows.

## Decisions

**1. Two layers, clearly separable so they can ship independently.**
- **Layer A (printed text from `card_data`)**: fill each source's `mainEffectText`/`inheritedEffectText` and the obvious case of `inheritedEffects` directly from the printed text the engine already has. Cheap, removes the stub, and is sufficient for "read what each source says."
- **Layer B (runtime accuracy)**: where engine state changes which inherited effects apply (substitution/removal — cf. `dsl-inherited-substitute-trash`), make `inheritedEffects` reflect the in-force set rather than printed text.
Layer A is the baseline acceptance; Layer B is the runtime-accurate completion. Splitting lets Layer A land immediately and Layer B follow if the substitution surface is non-trivial.

**2. Build `inheritedEffects` from the stack's non-top sources, attributed to each source.**
Inherited effects in Digimon TCG come from the digivolution-source cards beneath the top card. The serialized `inheritedEffects` enumerates those, each attributable to its source (so the UI can show "from <source>"). The top card contributes its main effect, not an inherited effect, and is excluded. *Alternative considered:* derive `inheritedEffects` purely on the frontend from `sources[]` — rejected because runtime substitution/removal (Layer B) requires engine state the frontend doesn't have, and because other `to_ui_json` consumers should get the same answer.

**3. Source of text is `card_data`, the same place ids/names already come from.**
No new lookup path or external dependency; the printed text fields already exist on the engine's card metadata loaded at startup.

**4. Treat as a parity alignment with the legacy Python serialization.**
The Python engine's `to_ui_json` populated this text; the Rust port stubbed it. Record the closure in `docs/RUST_PYTHON_PARITY.md` if it is currently listed as a divergence.

## Risks / Trade-offs

- **[Layer B (runtime-active inherited effects) is more involved than Layer A]** → Ship Layer A first (printed text un-stub) for immediate value; scope Layer B against the actual substitution/removal mechanisms present in the engine, and gate its acceptance scenario behind those mechanisms existing.
- **[Field-shape drift breaks consumers]** → Only the *contents* of existing fields change (`""`→text, `[]`→list); field names and types stay identical. A unit test pins the unchanged DP/keyword fields.
- **[Printed text vs engine behavior mismatch]** (e.g., `card_overrides.json`) → The engine should serialize the text it actually resolves from; where overrides apply, the engine's `card_data` is the authority, keeping the inspector consistent with engine behavior.
- **[Performance of building text per serialization]** → `to_ui_json` is already O(permanents × sources); adding string clones from `card_data` is negligible and only runs on UI serialization, not the RL hot path.

## Migration Plan

Additive content change to an existing serialization surface; no field added/removed, no migration. Requires rebuilding the PyO3 binding (`maturin develop`) for the hosted/desktop engine to pick up new text. Rollback = revert the serialization diff. Verify with Rust serialization unit tests over a constructed stacked permanent and a manual/Playwright check of the in-game inspector.

## Open Questions

- For Layer B, what is the complete set of engine mechanisms that alter which inherited effects apply (substitution, removal, suppression), and which are in-scope for v1 vs deferred?
- Should `inheritedEffects[]` entries carry structured fields (source card id, text, active flag) or just text? (Lean: structured — source id + text — so the UI can attribute and the data is future-proof.)
- Is this currently tracked as a `RUST_PYTHON_PARITY.md` divergence to close, or a net-new surface? (Confirm during implementation.)
