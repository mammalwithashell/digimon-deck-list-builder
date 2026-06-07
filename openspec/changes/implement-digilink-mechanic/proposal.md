## Why

The `[Link]` keyword (the Appmon mechanic; ~47 cards across BT10, EX2, ST22, and BT21–BT25/EX10/EX11) is only half-implementable in the Rust engine today. A substrate audit (2026-06-06, DCGO as reference) found the engine has solid support for one of the keyword's two card shapes and no support for the other:

- **Shape A — Plug-In Options** (~10 cards, e.g. *Offensive Plug-In V* ST22-08): an Option card links sideways onto a host Digimon. This is **built**: `Permanent.linked_cards` storage, the `WhenWouldLink`/`OnLink`/`OnLinkedCardTrashed`/`OnUnlink` timings, `ChangeLinkCost`/`ChangeLinkMax` modifiers, host-selection → attach → cascade (host deletion / return-to-hand trash the linked card), and DSL `kind: link_requirement` + `scope: linked`. Initiation runs through `OptionSubtype::Link` in the Option play flow (`game_actions.rs:2415`).

- **Shape B — Appmon Link Digimon** (~34 cards, e.g. *Gatchmon* BT21-009, *Logimon* BT25-052): a **Digimon card** is linked sideways onto a host Digimon. This is **unbuilt**. DCGO models it as a player-activated `[Main]` ability (`CardEffectFactory.LinkEffect`), driven by a structured `card.linkCondition` (cost + Digimon filter from `AddSelfLinkConditionStaticEffect`), able to link a card from **hand, trash, under-stack, another host's linked area, or as a whole standing battle-area permanent** (`ILinkCard.LinkCard()`, root `Hand`/`Trash`/`DigivolutionCards`/`LinkedCards`/`None`). On attach it fires `WhenLinked` — used **317 times** across DCGO card scripts — and the linked Digimon grants DP and ESS keywords (e.g. `Raid`) to its host.

The engine's link-initiation path is fundamentally an Option-play branch; it cannot express a Digimon activating its own link ability or a standing permanent being absorbed as a link. Authoring any of the 34 Shape-B cards on the existing substrate is therefore blocked. This change adds the missing Shape-B initiation substrate and DSL vocabulary, reusing the Shape-A storage/timing/cascade machinery rather than duplicating it.

## What Changes

- Add a Rust engine **Digimon-link initiation path**: a player-activated link ability on a Digimon (and a from-hand link) that selects a host Digimon, fires `WhenWouldLink`, pays the (modifier-adjusted) link cost, and attaches the linked card via the existing `attach_linked_card` / `linked_cards` machinery.
- Add a **standing-permanent absorb** path (DCGO root `None` → `IPlacePermanentToLinkCards`): a Digimon already in play is removed from the battle area and placed, whole, into a host's linked cards.
- Add **per-source-zone** link origins beyond the just-played Option: hand, trash, under-stack (digivolution cards), and re-link from another host's linked area.
- Represent a Digimon card's **self link-condition** (cost + host filter) as card-level metadata usable by `kind: digimon` cards, not only as an `OptionMain` effect.
- Confirm and, if needed, wire the linked Digimon's **`WhenLinked` self-trigger** and its **ESS grant to host** (DP + keywords such as `Raid`) — the audit marked these PARTIAL pending confirming tests (see design D6/D7).
- Extend the YAML DSL so Shape-B cards declare a link condition, `WhenLinked` triggers, and linked-scope ESS grants declaratively.
- Add focused Rust behavioral tests for an initial acceptance pool (BT21-009 *Gatchmon*, BT25-052 *Logimon*, plus one standing-permanent-absorb card) as the no-approximations gate.
- Preserve the action-space and observation contracts. Any new player choice (link-activate, host pick, source-zone pick) uses existing pending-selection / action-mask machinery and existing action ranges (`FIELD_EFFECT` for on-field link-activate); no `ACTION_SPACE_SIZE` change.

## Capabilities

### New Capabilities

- `digilink-execution`: The Rust engine can resolve a Digimon being linked to a host Digimon as a first-class action — self link-condition evaluation, host selection, multi-zone source origins (including absorbing a standing permanent), cost payment, `WhenLinked` dispatch, and ESS grant to the host.

### Modified Capabilities

- `dsl-card-scripting-vocabulary`: YAML card specs can author a Digimon's self link-condition, `WhenLinked` triggers, and linked-scope ESS grants without raw Rust.

## Impact

- Affected code: `code/digimon-engine/src/game_actions.rs`, `code/digimon-engine/src/effect_context/`, `code/digimon-engine/src/option_lifecycle.rs`, `code/digimon-engine/src/card_data.rs` (self link-condition metadata), `code/digimon-engine/src/action/mask.rs` (link-activate masking), `code/digimon-dsl/`, and `code/digimon-engine/cards/`.
- Affected tests: Rust behavioral tests under `code/digimon-engine/tests/option_flow/` and `code/digimon-engine/tests/cards_behavioral/`, DSL lowering tests (`tests/dsl/link.rs`), and Appmon archetype QA fixtures.
- Affected docs / gap trackers: `docs/RUST_ENGINE_GAPS.md` (Option/Plug-In/Link entry), `qa/dsl-vocab-gaps.md`, and `docs/RUST_ENGINE_API.md` (link API surface).
- Compatibility: no `ACTION_SPACE_SIZE`, active tensor profile, PyO3 action contract, or frontend action-constant change. New choices reuse existing pending-selection masks (mirrors the Group 5 Link-registration contract note in `RUST_ENGINE_GAPS.md`).
- Scope boundary: this change defines the Shape-B substrate, DSL vocabulary, and the initial acceptance fixtures. It does **not** author all 34 Appmon Link cards.
