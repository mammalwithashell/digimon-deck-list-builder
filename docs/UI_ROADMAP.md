# UI Roadmap — DCGO Parity for Players

**Date:** 2026-06-10. Derived from a survey of DCGO's Unity client (`$BASE_DCGO/Assets/Scripts/Script/`) against our React frontend (`code/frontend/src/`). Audience priority: **players first**, with QA and RL-game playback served by the same information features. Living doc — update phase status as changes land/archive.

## Where we already match or beat DCGO

- **Board rendering**: all zones, suspend rotation, DP/level/keyword badges, source-count badges, FLIP transitions; memory gauge incl. DCGO-style preview trail (DCGO: `MemoryPredictionLine.cs`).
- **Drag-and-drop**: hand→field play/digivolve with validity glow, attack arrow (DCGO: `Draggable_*.cs`, `TargetArrow.cs`).
- **Stack/modifier inspection**: `CardOverlay` (richer than DCGO's `PermanentDetail.cs` once `add-permanent-stack-inspector` lands).
- **Deck builder**: search/filters/stats/import-export/validation/tested-pool gating ≈ parity (DCGO: `EditDeck.cs`, `FilterCardList.cs`, `CardDistribution.cs`).
- **Lobby**: matchmaking queue + lobby codes + spectator + bot picker — ahead of DCGO's Photon rooms.

## Gap analysis (DCGO has, we lack)

| Gap | DCGO source | Size |
|---|---|---|
| Bot actions imperceptible (ours renders whole bot turn at once; DCGO paces via animations/RPC cadence) | n/a (we created this) | **Critical** |
| Per-card command panel (click card → its legal actions, named) | `CommandPanel.cs`, `CommandButton.cs` | **Big** |
| Gameplay options / auto-processing (auto-order, auto min/max digivolve cost, auto-hatch, confirm-before-end, cut-in skip, suspend-rotation toggle) | `GameplayOption.cs`, `AutoProcessing.cs`, `AutomaticOrder/` | **Big** |
| Resolving-effect context popup (which card's which effect is resolving, 5.5s auto-close) | `ShowEffectDiscriptionObject.cs` | Moderate |
| Zone viewers: security stack (own, when legal), revealed-deck views | `CheckCardPanel.cs`, `SecurityObject.cs` | Moderate |
| Hand ergonomics: hover-to-expand zoom, evo-cost icons on card | `HandCard.cs` | Moderate |
| Ordering-selection UI (`SelectPermutation` falls back to generic panel) | `SelectCardPanel.cs` ordering mode | Moderate |
| Sound: BGM, decision/cancel SFX, battle/evolution SFX, volume panel | `ContinuousController.cs`, `VolumePanel.cs` | Total gap (pure additive) |
| Field-state affordances: summoning-sickness overlay, link icon on field card | `FieldPermanentCard.cs` | Small |
| Specialized evolution cut-ins (Burst/Jogress/DigiXros, screen shake) + skip toggle | `*EffectObject.cs`, `CutInProcess.cs` | Polish |

**Deliberately not chasing**: localization, profanity filter, Photon-style rooms, reverse-opponent-cards. **Optional cheap win**: DCGO deck-code import (`DeckCodeUtility.cs`) for community-list interop.

## Phases

### Phase 0 — Land what's in flight
- `add-permanent-stack-inspector` (25/27) and `add-desktop-resolution-presets` (35/41): finish + archive.
- `add-ingame-action-log` (0/12) and `add-ingame-card-preview` (0/15): implement — these close "I can't tell what happened / what cards do" for players and QA alike.

### Phase 1 — Interaction parity (proposed 2026-06-10)
- **`add-bot-action-pacing`** — paced agent stepping on both wires + frontend driver + persisted speed setting. Fixes the critical perception problem; RL paths untouched. The request-driven pacing is also the stepping stone toward replay playback.
- **`add-per-card-command-panel`** — mask-derived contextual action menu (DCGO `CommandPanel` parity); makes the action space legible; subsumes generic "Effect N" buttons.
- **`add-gameplay-options-auto-processing`** — DCGO's option set as UI-side auto-submit + presentation toggles; engine choice surface (no-approximations) untouched.

### Phase 2 — Information access & ergonomics (not yet proposed)
- Resolving-effect context popup (seed: `EffectPopup.tsx`).
- Security-stack viewer + revealed-deck viewers (extend `TrashViewer` pattern).
- Hover-zoom hand cards with evo-cost icons.
- Proper `SelectPermutation` ordering UI (drag-to-reorder list).
- On-field affordances: summoning sickness, link icon, can't-attack dimming.

### Phase 3 — Feel (not yet proposed)
- Sound: BGM + SFX + volume options (respecting gameplay-options panel).
- Specialized evolution cut-ins with skip toggle; attack/battle feedback polish.
- Replay playback UI (scrubber over action traces / recordings; builds on the pacing driver's beat-by-beat rendering).

## Constraints that bind all UI work

- **No-approximations (CLAUDE.md rule 17)**: automation is UI-side auto-submit only; the engine surfaces every choice.
- **Desktop DTO parity**: every wire-visible field lands on BOTH the browser DTO (`serialization.rs`) and desktop DTO (`engine_commands.rs`) in the same change (recurring failure mode).
- **Verify via the scenario substrate**: `add-ui-scenario-test-substrate` is complete — new UI behavior gets Playwright scenario specs, not just manual checks.
