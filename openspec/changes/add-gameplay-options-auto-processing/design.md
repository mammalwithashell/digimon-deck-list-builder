# Design: add-gameplay-options-auto-processing

## Context

DCGO's `GameplayOption.cs` is the battle-tested spec for which choices players want automated in a faithful Digimon TCG client: auto-order effects / deck operations (bottom draw, top draw, min/max digivolution cost, auto hatch), check-before-ending-selection, show-cut-in-animation, rotate-suspended-cards, reverse-opponent-cards. Our engine, by the no-approximations policy (CLAUDE.md rule 17), surfaces *every* choice through `pending_selection`/the action mask — correct for RL, tedious for humans. The frontend already has: persisted `uiStore` (resolution presets pattern), `SelectionPanel` for multi-selects, `KeywordPromptDialog`, transient animation components subscribing to `store.events`, and suspend rotation hardcoded in `Card.tsx`.

## Goals / Non-Goals

**Goals:**
- DCGO-parity gameplay toggles, persisted, controlling UI-side automation and presentation.
- Automation that never hides information, never changes the engine-visible action space, and is always auditable in the log.
- One small, unit-tested module that decides "is this pending selection auto-resolvable under current options?"

**Non-Goals:**
- Engine- or server-side auto-resolution of any kind (would violate no-approximations and leak into the RL action space).
- DCGO's "reverse opponent's cards" (our board renders opponent cards upright already) and banlist toggle (deck legality is engine-owned) — explicitly dropped from the copied spec.
- Sound options (no audio engine yet; separate roadmap item).

## Decisions

### D1: Automation is a pure UI-side auto-submit layer

A single module `code/frontend/src/utils/autoResolve.ts` exports `classifyAutoResolve(pendingSelection, mask, options) -> { action: number[] } | null`. `GamePage` runs it whenever a pending selection arrives; a non-null result is submitted through the normal `sendAction` path after a short visual beat (so the log/ticker entry is perceivable). The engine and wires are untouched. Alternative (server-side option flags) rejected: splits the policy across processes and risks the RL contract.

### D2: Classification rules, from strictest to optional

1. **Single legal action** (auto-order trivial effects): exactly one legal action in the mask for the pending selection, and it is not a yes/no keyword prompt (declining must stay human unless covered by rule 3). Safe by construction — the player had no choice.
2. **Order-only selections without hidden information** (deck-bottom/top placement order): pending selection kind is a permutation/ordering over cards the player already sees, where order affects only zone sequencing the player could not strategically exploit beyond defaults. Auto-submit identity order. Gated behind its own toggle; the classification is per selection-kind allowlist, not heuristic.
3. **Min/max digivolve cost**: when the same digivolve is offered at multiple costs, auto-pick the minimum (toggle; DCGO offers min and max — we ship min, max follows if requested).
4. **Auto-hatch**: in breeding phase with hatch legal and no other meaningful action (per mask), auto-submit hatch (toggle).

Anything not matching an explicit rule is never auto-resolved. Rules 2–4 ship default-on to match DCGO feel; rule 1 default-on; all individually toggleable.

### D3: Confirm-before-end-selection

When enabled, `SelectionPanel`'s submit becomes a two-step (Confirm overlay listing the chosen cards). Implemented inside `SelectionPanel` so every multi-select flow inherits it. Default off (DCGO default).

### D4: Presentation toggles

- **Show animations** (DCGO "show cut-in"): one boolean consumed by `PhaseBanner`, `DigivolveBanner`, `BattleEffect`, and `SecurityRevealOverlay` (collapse dwell times to manual-advance/minimal). Event subscription stays (lastSeqRef tracking per CLAUDE.md rule 15) — components render nothing/instantly rather than unsubscribing.
- **Rotate suspended cards**: thread an option read into `Card.tsx`/`PermanentSlot.tsx`; off replaces rotation with a "SUSPENDED" corner tag so state stays visible.

### D5: Options UI placement

A `GameplayOptionsPanel` component rendered (a) as a section on the settings page next to graphics settings, and (b) in-game via a gear affordance, since most toggles matter mid-game. State lives in a persisted `uiStore` slice (`gameplayOptions`), following the resolution-preset persistence pattern.

### D6: Auditability

Every auto-submitted action appends a distinguishable log/ticker entry ("(auto)" suffix) so a player or QA reviewer can attribute outcomes to automation. The toggle panel links to this behavior ("auto-resolved choices appear in the log").

## Risks / Trade-offs

- [Misclassifying a strategic selection as trivial] → rules are explicit allowlists keyed on selection kind, each with unit tests; no fuzzy heuristics; rule 2's kind-allowlist starts minimal (deck-bottom order) and grows case-by-case.
- [Auto-submit loops / re-entrancy (auto action produces another pending selection)] → driver processes one classification per state response, runs through the normal request cycle, with a depth guard + escape hatch (any error disables automation for the session and surfaces a notice).
- [Optional-keyword prompts auto-declined by rule 1 when "decline" is the only legal action] → that is genuinely forced and fine; but a one-legal-action *use* of an optional keyword must still auto-submit the use, not synthesize a decline — classification operates only on mask-legal actions.
- [Interaction with `add-bot-action-pacing`] → auto-resolved human prompts during the opponent's paced sequence should respect the pacing beat; both read `uiStore`, no shared logic otherwise.
- [DSL pitfall: over-exposed PASS on forced selections (known issue)] → automation must not paper over engine bugs: rule 1 requires exactly one legal action; if an illegal PASS is over-exposed the selection has two actions and stays manual (and the bug stays visible).

## Open Questions

- Should rule 3 offer "always ask / always min / always max" tri-state to fully match DCGO? Ship min-toggle first; extend if requested.
