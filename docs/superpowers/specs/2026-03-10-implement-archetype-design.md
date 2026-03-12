# Design: Archetype-Level Card Effect Implementation Pipeline

**Date:** 2026-03-10
**Status:** Approved

## Problem

Previous approaches to implementing Digimon TCG card effects (full-set transpilation, set-level AI pipeline) struggled with detail accuracy. Card text is ambiguous; agents need C# reference implementations to disambiguate. Working at set level overwhelms context windows and makes QA intractable.

## Solution

An archetype-scoped workflow that:
1. Takes a deck-list-based card pool as input
2. Plans all cards with categorization and batching
3. Dispatches parallel Sonnet agents with curated context packs
4. Enforces a hard "no stubs" policy — agents report BLOCKED rather than approximate
5. Verifies with smoke tests + targeted effect tests
6. Accumulates engine gap reports across archetypes

## Input

An archetype is defined by a name and a list of card IDs. Sources:
- `deck_library.json` — 139 archetypes with 176 deck lists, card IDs deduplicated across all lists for the archetype
- Manual card list — user provides card IDs directly

## Workflow Phases

### Phase 1: Resolve Card Pool

1. Parse archetype name from `deck_library.json` or accept manual card list
2. Deduplicate card IDs across all deck lists in the archetype
3. For each card ID:
   - Check for existing frozen script in `_frozen_manifest.json`
   - Locate C# source at `DCGO/Assets/Scripts/CardEffect/{SET}/{COLOR}/{CARD_ID}.cs`
   - Fetch card text from DigimonCard.io API (or local card database)

### Phase 2: Categorize Cards

| Category | Criteria | Agent Task |
|----------|----------|------------|
| QA-only | Frozen script exists | Verify script against card text + C# |
| Implement | No script or generated-only | Write new script from scratch |
| Complex | 4+ effects, DNA digivolve, multi-step selections, or in `known_complex_cards.json` | Implement with extra example context |

### Phase 3: Present Plan for Approval

Show the user:
- Total unique cards, breakdown by category
- Batch groupings (cards that interact grouped together)
- Cards missing C# source (will rely on card text + similar examples)
- Estimated agent count and batch sizes

Wait for user approval before proceeding.

### Phase 4: Assemble Context Packs

Each agent batch receives:
1. **Engine API reference** — `docs/archetype-qa/engine-api-reference.md` (stable, pre-written)
2. **Card manifest** — for each assigned card: ID, name, kind, level, colors, traits, DP, full card text
3. **C# scripts** — raw contents inline (for cards that have them)
4. **Existing Python scripts** — for QA-only cards, current frozen script inline
5. **Similar frozen examples** — for Complex cards, 2-3 similar frozen scripts as few-shot examples

### Phase 5: Dispatch Parallel Agents

- **Agent model:** Sonnet (cost-effective for implementation, Opus plans)
- **Isolation:** Each agent runs in an isolated git worktree
- **Batch sizes:**
  - QA-only: 8-10 cards per agent
  - Implement: 3-5 cards per agent
  - Complex: 1-3 cards per agent
- **Grouping:** Cards that reference each other's traits/names are batched together
- **Agent instructions:**
  - For QA-only: read existing script, compare against card text + C#, report PASS or QA-FAIL with specific issues
  - For Implement: write new script following engine API reference patterns, use C# as behavioral reference
  - **Hard rule:** If an effect cannot be faithfully implemented (missing engine mechanic, ambiguous behavior, no C# source and card text is unclear), report BLOCKED with details. Do NOT stub, approximate, or use `pass # TODO`.

### Phase 6: Compile QA Index

Merge all agent verdicts into `docs/archetype-qa/{archetype_name}.md`:

```
# Archetype QA: {name}
Date: YYYY-MM-DD
Total cards: N

## Summary
- PASS: N
- IMPLEMENTED: N
- QA-FAIL: N
- BLOCKED: N

## QA Failures
### CARD-ID Card Name
- Issue: description
- Line: N
- Severity: high/medium/low

## Blocked Cards
### CARD-ID Card Name
- Effect text: "..."
- Missing mechanic: description
- Suggested engine change: description

## Implementation Notes
- Any cross-card interaction notes
- Patterns discovered during implementation
```

Update `docs/archetype-qa/engine-gaps.md` with any new BLOCKED items.

### Phase 7: Verification

**Smoke test:**
- Pick a deck list from the archetype's `deck_library.json` entry
- Run 50 mirror-match episodes with greedy/random policy
- Any crash = identify failing script from stack trace, fix, rerun
- Pass = 50 clean games

**Targeted effect tests:**
- For each Complex card and each QA-FAIL (after fix), write a pytest case
- Tests in `tests/test_archetype_{name}.py`
- Each test sets up a board state, triggers the effect, asserts outcome
- Run with `python -m pytest tests/test_archetype_{name}.py -v`

### Phase 8: Final Report

Present to user:
- QA index summary
- Smoke test results
- Test results
- BLOCKED items requiring engine work
- List of new/modified script files

## Persistent Artifacts

| Artifact | Location | Lifecycle |
|----------|----------|-----------|
| Engine API reference | `docs/archetype-qa/engine-api-reference.md` | Written once, updated as engine evolves |
| Per-archetype QA report | `docs/archetype-qa/{name}.md` | Created per archetype run |
| Engine gap tracker | `docs/archetype-qa/engine-gaps.md` | Accumulates across archetypes |
| Archetype tests | `tests/test_archetype_{name}.py` | Created per archetype run |
| Card scripts | `digimon_gym/engine/data/scripts/{set}/` | Created/modified per card |

## Agent Output Contract

Each card gets exactly one verdict:

| Verdict | Meaning | Agent Action |
|---------|---------|-------------|
| PASS | Existing script is correct | No code changes |
| IMPLEMENTED | New script written successfully | Script file created |
| QA-FAIL | Existing script has issues | Issues documented with specifics |
| BLOCKED | Cannot faithfully implement | Detailed report of what's missing |

## Key Constraints

1. **No stubs or approximations** — BLOCKED is always preferred over partial implementation
2. **Leak guard mandatory** — every BeforePayCost condition must check `card_source is not card`
3. **Inherited effects** — must be separate ICardEffect instances with `is_inherited_effect = True`
4. **C# as reference, card text as truth** — if C# and card text disagree, card text wins
5. **Engine API reference is the contract** — agents must only use documented methods
6. **Worktree isolation** — parallel agents cannot conflict on file writes

## DCGO C# Script Location

```
C:\Users\james\Documents\digimon-deck-list-builder-1\DCGO\Assets\Scripts\CardEffect\{SET}\{COLOR}\{CARD_ID}.cs
```

Where SET is uppercase (BT24, EX11, etc.) and COLOR is the card's primary color (Red, Blue, etc.).
