---
name: batch-fix-cards
description: Archetype-scoped batch faithfulness pipeline. Resolves full card pool, processes in batches of 4. Each sub-agent writes DebugRunner tests from card text FIRST, then implements or fixes scripts to pass them. A review agent audits each batch before continuing. Tracks in validated_cards.json and Notion.
argument-hint: <ARCHETYPE_NAME> [--cards CARD1,CARD2,...] [--batch-size N] [--report-only] [--skip-tests] [--skip-notion]
---

# Batch Fix Cards — Archetype-Scoped Test-First Faithfulness Pipeline

Process an entire archetype's card scripts through a test-driven faithfulness pipeline. Cards are processed in batches of 4 with parallel Opus sub-agents. Each agent writes behavioral tests from card text first, then **implements new scripts from scratch or fixes existing ones** to pass them. A separate review agent audits each batch. Cards without existing scripts are implemented; cards with scripts are reviewed and fixed.

## When to Use

- Systematically implementing, verifying, and fixing all card scripts for an archetype
- Running a test-driven faithfulness pass across a deck list
- Combining the depth of `/fix-card` with the scope of `/implement-archetype`

**Not for:** Single card fixes (use `/fix-card`). Not for gameplay testing (use `/gameplay-qa`).

## Flags

- `--batch-size N`: Override batch size (default: 4)
- `--report-only`: Analyze only, no edits or tests
- `--skip-tests`: Fix scripts but skip DebugRunner test creation
- `--skip-notion`: Skip Notion board updates
- `--cards CARD1,CARD2,...`: Override card pool with explicit comma-separated list

## Quick Reference

| Resource | Path / ID |
|----------|-----------|
| Engine API Ref | `qa/archetype-qa/engine-api-reference.md` |
| Engine Gaps | `qa/archetype-qa/engine-gaps.md` |
| C# Scripts | `DCGO/Assets/Scripts/CardEffect/{SET}/{COLOR}/{CLASS_NAME}.cs` |
| Python Scripts | `code/engine_py_legacy/engine/data/scripts/{set_lower}/{set_lower}_{nnn}.py` |
| Card Metadata | `data/cards.json` |
| Deck Library | `data/deck_library.json` |
| Validated Cards | `qa/qa-reports/validated_cards.json` |
| Archetype QA | `qa/archetype-qa/{archetype_name}.md` |
| Pinecone Index | `digimon-engine` (namespaces: engine-api, card-scripts, card-metadata) |
| Notion Archetype Tracker | data source `collection://6af6bd9a-6ef6-4e23-b476-7f7c77c2f2aa` |
| Notion PM Board | data source `31f97972-7634-80d0-97eb-000b817cdae1` |

---

## Phase 1: Resolve Card Pool

### 1a. Resolve card pool and build manifest

Use the `resolve_deck` tool to resolve the full card pool with enriched metadata:

```python
import sys; sys.path.insert(0, '.')
from tools.resolve_deck import resolve_archetype

# If $ARGUMENTS contains --cards, pass as override:
# manifest = resolve_archetype('ARCHETYPE_NAME', cards_override=['CARD1', 'CARD2', ...])
# Otherwise:
manifest = resolve_archetype('ARCHETYPE_NAME')
```

The `manifest` object provides:
- `manifest.unique_cards` — list of `CardEntry` objects, each with:
  - `card_id`, `card_name`, `card_kind`, `level`, `colors`, `traits`, `dp`, `play_cost`, `evo_costs`
  - `effect_text`, `inherited_text`, `security_text`
  - `script_status` — `"frozen"`, `"generated"`, or `"missing"`
  - `script_path` — relative path to existing script, or `None`
  - `csharp_path` — relative path to C# reference, or `None`
  - `deck_frequency` — how many decklists include this card
- `manifest.coverage_pct`, `manifest.frozen_count`, `manifest.generated_count`, `manifest.missing_count`
- `manifest.missing_cards` — card IDs with no script at all
- `manifest.best_decklist` — single best deck list
- `manifest.meta_share`, `manifest.total_decklists`
- `deck_pool.json` is auto-written to `qa/archetype-qa/{slug}/`

### 1b. Filter to processable cards

Cards with `script_status == "missing"` go on a skip list — report to user but do not process (no script to fix). Only cards with `"frozen"` or `"generated"` scripts are processed.

```python
processable = [c for c in manifest.unique_cards if c.script_status != "missing"]
skipped = [c for c in manifest.unique_cards if c.script_status == "missing"]
```

### 1c. Build cross-archetype reverse map

For shared-card Notion tracking, build a reverse map: card ID → all archetypes containing it:

```python
reverse_map = {}  # card_id -> set of archetype names
for arch_name, arch_data in lib['archetypes'].items():
    for dl in arch_data.get('decklists', []):
        for card_id in json.loads(dl['decklist']):
            reverse_map.setdefault(card_id, set()).add(arch_name)
```

This is used in Phase 4D for cross-archetype Notion updates.

---

## Phase 2: Build Batches & Present Plan

### 2a. Group cards into batches of 4

Default batch size is 4, configurable via `--batch-size N`.

**Grouping rules** (apply before splitting into batches):
- Cards that reference each other by name in their effect text → same batch
- Tamers that buff specific Digimon → batch with those Digimon
- Option cards → batch with the Digimon they target
- Remaining cards fill batches in card ID order

### 2b. Present plan for approval

```
## Batch Fix Plan: {archetype_name}
Total cards: N (M to fix, K to implement)
Batches of {batch_size}: {ceil(N/batch_size)}

Batch 1: [CARD-001 (fix), CARD-002 (implement), CARD-003 (implement), CARD-004 (fix)]
Batch 2: [CARD-005 (implement), CARD-006 (implement), CARD-007 (implement), CARD-008 (implement)]
...
```

**Wait for user approval before proceeding.**

---

## Phase 3: Pre-Read Shared Context

Before dispatching any agents, the orchestrator reads and caches these once:

### 3a. Engine API Reference excerpt

Read `qa/archetype-qa/engine-api-reference.md`. Extract sections:
- Section 1: Script Structure
- Section 4: Modifier System
- Section 5: EffectTiming Enum
- Section 10: Common Patterns
- Section 11: Anti-Patterns

This is approximately 350 lines. Include verbatim in every agent prompt.

### 3b. Engine gaps

Read `qa/archetype-qa/engine-gaps.md` in full. Include in every agent prompt.

### 3c. Pre-create directories

For each set that appears in the card pool, ensure both test and script directories exist. This prevents agents from conflicting on directory creation.

```bash
# Test directories
mkdir -p code/engine_py_legacy/tests/behavioral/{set_lower}
touch code/engine_py_legacy/tests/behavioral/{set_lower}/__init__.py

# Script directories (for IMPLEMENT cards that need new scripts)
mkdir -p code/engine_py_legacy/engine/data/scripts/{set_lower}
touch code/engine_py_legacy/engine/data/scripts/{set_lower}/__init__.py
```

### 3d. Initialize Notion tracker

Skip if `--skip-notion`.

Search the Archetype Verification Tracker for the current archetype name:
- `notion-search query="{archetype_name}" data_source_url="collection://6af6bd9a-6ef6-4e23-b476-7f7c77c2f2aa"`

If not found: create a new row:
- Archetype: `{archetype_name}`
- Status: "In Progress"
- Total Cards: N
- Faithful: 0, Implemented: 0, Fixed: 0, Deferred: 0, Engine Gaps: 0

If found: update Status to "In Progress".

Track the Notion page URL for subsequent updates.

---

## Phase 4: Batch Loop

Repeat for each batch of 4 cards. Maintain running totals: `faithful_count`, `implemented_count`, `fixed_count`, `deferred_count`, `blocked_count`.

### Phase 4A: Gather Per-Card Context (orchestrator)

For each card in the current batch, the orchestrator reads:

1. **Card metadata** from `data/cards.json` — extract entry for this card ID. Key fields: `card_name_eng`, `effect_description_eng`, `inherited_effect_description_eng`, `card_kind`, `level`, `dp`, `play_cost`, `card_colors`, `type_eng` (traits), `evo_costs`

2. **Current Python script** from `code/engine_py_legacy/engine/data/scripts/{set}/{set}_{nnn}.py` — if the card is categorized as IMPLEMENT (no script exists), set to `null` in the agent prompt

3. **C# reference** — glob for `DCGO/Assets/Scripts/CardEffect/{SET}/*/{CLASS_NAME}.cs` (color subdirectory varies; C# class name uses underscores: BT17-001 → BT17_001.cs). The C# source is the **behavioral source of truth** for implementation. If not found, note "C# reference not available — use card text as sole source of truth".

4. **Prior QA status** from `qa/qa-reports/validated_cards.json`

### Phase 4B: Dispatch 4 Parallel Fix Agents

Dispatch 4 Agent tool calls **in a single message** (parallel execution):

```
Agent tool:
  model: "opus"
  isolation: "worktree"
  prompt: {per-card fix prompt — see Fix Agent Prompt Template below}
```

**IMPORTANT:** All agents must run at high effort. Each agent handles exactly ONE card.

#### Fix/Implement Agent Prompt Template

The prompt adapts based on the card's category (FIX or IMPLEMENT):

```
You are performing a test-driven {MODE: "faithfulness review and fix" | "implementation"} for a single Digimon TCG card script. You must work at high effort — be thorough and precise.

## Mode: {FIX | IMPLEMENT}
{If FIX: "An existing script is provided below. Review it for faithfulness, write tests, and fix any discrepancies."}
{If IMPLEMENT: "No script exists for this card. Write tests from card text first, then implement the script from scratch using the C# reference as behavioral guide."}

## Your Card

**Card ID:** {CARD_ID}
**Name:** {card_name}
**Kind:** {kind} | **Level:** {level} | **Colors:** {colors} | **Traits:** {traits} | **DP:** {dp} | **Play Cost:** {cost}

**Effect Text:**
{effect_description_eng}

**Inherited Effect:**
{inherited_effect_description_eng}

## Current Python Script
{If FIX:}
File: `code/engine_py_legacy/engine/data/scripts/{set}/{set}_{nnn}.py`
```python
{script contents}
```
{If IMPLEMENT:}
No existing script. You will create: `code/engine_py_legacy/engine/data/scripts/{set}/{set}_{nnn}.py`

## C# Reference Implementation (Behavioral Source of Truth)
File: `{csharp_path}`
```csharp
{C# contents, or "Not available — use card text as sole source of truth"}
```

## Prior QA Status
{existing validated_cards.json entry, or "No prior QA"}

---

## Your Workflow — TEST FIRST

### Step 1: Decompose Card Text into Clauses
Break the card text into numbered clauses. For each clause identify:
- Type: Trigger / Condition / Action / Target / Optionality / Duration / Frequency / Inheritance / Security / Cost / Keyword / Alt-Digi
- The exact card text
- The expected engine behavior (timing enum, API call, parameters)
- How the C# reference handles this clause (if available)

### Step 2: Write DebugRunner Tests FIRST
BEFORE reading or modifying any script, write tests that encode what the card SHOULD do based on the card text (source of truth).

Create `code/engine_py_legacy/tests/behavioral/{set_lower}/test_{set_lower}_{nnn}.py`:

```python
import pytest

@pytest.mark.behavioral
class Test{CLASS_NAME}{CardName}:
    """Tests for {CARD_ID} {card_name}."""

    def test_{clause_description}(self, debug_runner):
        """{Exact card text clause being tested}."""
        runner = debug_runner(initial_memory=N)
        runner.set_phase("Main")
        runner.inject_card(1, "{CARD_ID}", "hand")
        # Set up board state as needed
        runner.place_on_field(2, ["{TARGET_CARD}"])

        action = runner.find_action("Play {card_name}")
        assert action is not None
        result = runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Assert expected state per card text
```

**Coverage rules:**
- One test method per distinct effect clause
- For conditional effects: at least one positive + one negative case
- Tests should verify specific state changes described in the card text
- Use the `debug_runner` fixture from `code/engine_py_legacy/tests/conftest.py`

### Step 3: Run Tests
```bash
python -m pytest code/engine_py_legacy/tests/behavioral/{set_lower}/test_{set_lower}_{nnn}.py -v
```
{If FIX: "Failures reveal discrepancies between card text and current script."}
{If IMPLEMENT: "Tests will fail since no script exists yet. This confirms your test expectations before implementation."}

### Step 4: {FIX: "Faithfulness Analysis" | IMPLEMENT: "Implement the Script"}

{If FIX:}
Read the script carefully. Compare each clause against:
- The Python script implementation
- The C# reference (for behavioral ambiguities)
- Run the 16-item Error Checklist below

{If IMPLEMENT:}
Create `code/engine_py_legacy/engine/data/scripts/{set}/{set}_{nnn}.py` from scratch:
- Use the C# reference as behavioral guide for timing, conditions, and flow
- Follow patterns from the Engine API Reference excerpt below
- Translate C# patterns to Python engine API (see Error Checklist for correct API usage)
- Ensure the script directory exists: `code/engine_py_legacy/engine/data/scripts/{set}/` (create `__init__.py` if needed)

### Step 5: {FIX: "Fix the Script" | IMPLEMENT: "Refine the Script"}
For each failing test / MISMATCH clause:
- Edit the script to correctly implement the card text
- Follow patterns from the Engine API Reference
- Verify importability:
  ```bash
  python -c "from engine_py_legacy.engine.data.scripts.{set}.{set}_{nnn} import {CLASS_NAME}; print('OK')"
  ```
- If a clause hits an engine gap: add comment `# ENGINE GAP: {description}` and report BLOCKED for that clause

### Step 6: Re-run Tests
```bash
python -m pytest code/engine_py_legacy/tests/behavioral/{set_lower}/test_{set_lower}_{nnn}.py -v
```
If tests fail: analyze → fix script or test → rerun. ONE revision round maximum.

### Step 7: Report Verdict
- **FAITHFUL**: (FIX only) No script changes needed, all tests pass
- **IMPLEMENTED**: (IMPLEMENT only) New script created, all tests pass
- **FIXED**: (FIX only) Script corrected, all tests now pass
- **PARTIAL**: Some clauses working, some blocked by engine gaps
- **BLOCKED**: Cannot faithfully implement due to engine gaps

### Step 8: Flag New Patterns
If you used an engine API pattern not documented in the Engine API Reference excerpt below, report it in `### New Patterns Discovered` so the orchestrator can update the reference.

---

## Error Checklist — Verify ALL 16 items

1. `BeforePayCost` condition MUST start with: `if context.get('card_source') is not card: return False`
2. `[When Attacking]` uses `EffectTiming.OnUseAttack` (28), NOT `OnAllyAttack` (32)
3. No stubs — every effect has a complete process callback (no `pass`)
4. Inherited effects have `is_inherited_effect = True` on SEPARATE `ICardEffect` instances
5. Alt-digi includes ALL qualifying traits/names from card text
6. Tamer `[Start of Your Turn]` checks `memory <= N` gate where card text specifies
7. `register_modifier` arg order: `game.register_modifier(target_perm, ModifierType.X, value, condition=, expiry=)`
8. Option main = `EffectTiming.OptionSkill`; security = `EffectTiming.SecuritySkill`
9. "Ignore color requirements" conditions check specific context, NOT `return True`
10. Reveal flows use `game.effect_reveal_from_deck()`, NOT manual list ops
11. Target selections offer ALL valid targets; NO auto-selection (`min(...)`, `[0]`, etc.)
12. Piercing: `game.effect_grant_piercing_factory()`
13. `OnTappedAnyone` callbacks verify the suspended Digimon is the correct target
14. DP modification: `register_modifier` with `CHANGE_DP` + expiry, NOT `perm.change_dp()`
15. Field presence: conditions check `card.permanent_of_this_card() is not None`
16. Use `player.battle_area`, NEVER `player.field_cards`

## Critical Common Bugs

- **Name vs Trait**: Card names → `contains_card_name()`; Traits → `has_trait()`. Wrong API silently fails.
- **Multi-Step Selections**: Two separate player choices → TWO selection phases, not one.
- **Player Agency**: "1 of your" or "any 1" → player MUST choose via selection phase. No auto-selection.
- **"By" Costs**: "By deleting 1 of your Digimon" → deletion is a COST, happens first, not skippable.
- **Wrong Zone**: "from your trash" ≠ "from your hand". Verify zone.

## Engine API Reference (Excerpt)
{~350 lines: sections 1, 4, 5, 10, 11 from engine-api-reference.md}

## Engine Gaps
{contents of engine-gaps.md}

## Pinecone MCP (fallback)
The above context covers the main patterns. Use Pinecone only for edge cases. Index: "digimon-engine".
- Engine API details: search namespace "engine-api"
- Similar frozen scripts: search namespace "card-scripts" with filter {is_frozen: true}
- Card metadata/cross-references: search namespace "card-metadata"

## Self-Recovery
If unsure about a method or pattern:
1. Check Engine API Reference above first
2. Search Pinecone "engine-api" namespace
3. Search Pinecone "card-scripts" for frozen examples with similar mechanics
4. If still unsure, flag as UNCERTAIN and report PARTIAL rather than guessing

---

## Output Format

Report your results in this exact format:

```
## {CARD_ID} — {card_name}

### Verdict: {FAITHFUL|IMPLEMENTED|FIXED|PARTIAL|BLOCKED}

### Clause Analysis
Clause 1 ({type}): "{exact card text}"
Script: {what the script does for this clause}
Result: MATCH | IMPLEMENTED | FIXED | BLOCKED
{If IMPLEMENTED: description of how the clause was implemented}
{If FIXED: description of what was wrong and how it was corrected}

Clause 2 ...

### Tests Written
- code/engine_py_legacy/tests/behavioral/{set}/test_{set}_{nnn}.py — N tests
  - test_{name}: {what it verifies} — {PASS|FAIL}
  ...

### Script Changes
- {description of each change applied}
{or "No changes needed" if FAITHFUL}
{or "New script created from scratch" if IMPLEMENTED}

### New Patterns Discovered
- {pattern name}: {description of engine API usage not in the reference}
{or "None" if all patterns were documented}

### Test Output
{full pytest -v output}
```
```

### Phase 4C: Dispatch Review Agent

After all 4 fix agents return, dispatch a **separate Opus review agent** at **high effort**:

```
Agent tool:
  model: "opus"
  prompt: {review prompt — see Review Agent Prompt Template below}
```

No worktree needed — the reviewer is read-only.

#### Review Agent Prompt Template

```
You are the review agent for a batch of 4 Digimon TCG card script implementations. You must work at high effort — be thorough and precise. Your job is to verify that each implementation faithfully matches the card text.

## Your Task

For each card below, review:
1. The fix agent's clause analysis and verdict
2. The modified Python script
3. The DebugRunner tests written
4. Cross-reference against the card text (source of truth) and C# reference

## Review Criteria

For each card:
1. **Completeness**: Does the script handle ALL effects in the card text? Any missed clauses?
2. **Correctness**: Do the timings, conditions, targets, and actions match the card text exactly?
3. **Error Checklist**: Verify all 16 items (listed below) against each script
4. **Test Coverage**: Do the tests adequately cover each clause? Any missing edge cases?
5. **C# Alignment**: For ambiguous card text, does the Python match the C# behavioral reference?

## Error Checklist
1. BeforePayCost condition starts with: if context.get('card_source') is not card: return False
2. [When Attacking] uses EffectTiming.OnUseAttack (28), NOT OnAllyAttack (32)
3. No stubs — every effect has a complete process callback
4. Inherited effects have is_inherited_effect = True on separate ICardEffect instances
5. Alt-digi includes ALL qualifying traits/names from card text
6. Tamer [Start of Your Turn] checks memory <= N gate
7. register_modifier args: game.register_modifier(target_perm, ModifierType.X, value, condition=, expiry=)
8. Option: main=OptionSkill, security=SecuritySkill
9. "Ignore color" conditions check specific context, not return True
10. Reveal flows use game.effect_reveal_from_deck(), not manual list ops
11. Target selections offer ALL valid targets; no auto-selection
12. Piercing: game.effect_grant_piercing_factory()
13. OnTappedAnyone: verify the suspended Digimon is the correct target
14. DP modification: register_modifier with CHANGE_DP + expiry, not change_dp()
15. Field presence: check card.permanent_of_this_card() is not None
16. Use player.battle_area, NEVER player.field_cards

## Cards to Review

### Card 1: {CARD_ID} — {card_name}
**Card Text:** {effect_description_eng}
**Inherited Text:** {inherited_effect_description_eng}
**C# Reference:** {C# contents or "Not available"}

**Fix Agent Verdict:** {verdict}
**Fix Agent Clause Analysis:**
{clause analysis from fix agent}

**Modified Script:**
```python
{script contents after fix agent's changes}
```

**Tests Written:**
```python
{test file contents}
```

### Card 2: ...
(repeat for all 4 cards in batch)

## Output Format

For each card, output exactly:

```
{CARD_ID}: APPROVED
```
or
```
{CARD_ID}: NEEDS-FIX
- Issue 1: {description} — Fix: {specific line-level fix instruction}
- Issue 2: {description} — Fix: {specific line-level fix instruction}
```
```

### Phase 4D: Merge & Track (orchestrator)

After the review agent returns:

#### 4D-i. Copy files from worktrees

For each agent's worktree, copy:
- Script: `code/engine_py_legacy/engine/data/scripts/{set}/{set}_{nnn}.py` (modified for FIX cards, newly created for IMPLEMENT cards)
- New test file: `code/engine_py_legacy/tests/behavioral/{set_lower}/test_{set_lower}_{nnn}.py`

#### 4D-ii. Apply review fixes

For any card the review agent marked NEEDS-FIX, apply the specific fix instructions to the script in the main working tree.

#### 4D-iii. Run all batch tests together

```bash
python -m pytest code/engine_py_legacy/tests/behavioral/{set_lower}/test_{set_lower}_{nnn1}.py code/engine_py_legacy/tests/behavioral/{set_lower}/test_{set_lower}_{nnn2}.py code/engine_py_legacy/tests/behavioral/{set_lower}/test_{set_lower}_{nnn3}.py code/engine_py_legacy/tests/behavioral/{set_lower}/test_{set_lower}_{nnn4}.py -v
```

If any tests fail: one targeted fix round by the orchestrator. Diagnose and fix the specific issue.

#### 4D-iv. Update validated_cards.json

Read `qa/qa-reports/validated_cards.json`, then for each card in the batch, add/update:
```json
"{CARD_ID}": {
  "card_name": "{name}",
  "validated_date": "{YYYY-MM-DD}",
  "report": "batch-fix-cards",
  "status": "{PASS|FIXED|PARTIAL|BLOCKED}",
  "notes": "{one-line summary}"
}
```
Increment `version` once, update `last_updated` once for the entire batch.

Map verdicts: FAITHFUL → PASS, IMPLEMENTED → IMPLEMENTED, FIXED → FIXED, PARTIAL → PARTIAL, BLOCKED → BLOCKED.

#### 4D-v. Update running totals

```python
# Update counts based on this batch's verdicts
for card in batch:
    if verdict == "FAITHFUL": faithful_count += 1
    elif verdict == "IMPLEMENTED": implemented_count += 1
    elif verdict == "FIXED": fixed_count += 1
    elif verdict == "PARTIAL": deferred_count += 1
    elif verdict == "BLOCKED": blocked_count += 1
```

#### 4D-vi. Update Notion Archetype Verification Tracker

Skip if `--skip-notion`.

Update the archetype's row in the tracker with cumulative totals:
- Faithful: `{faithful_count}`
- Implemented: `{implemented_count}`
- Fixed: `{fixed_count}`
- Deferred: `{deferred_count}`
- Engine Gaps: `{blocked_count}`

**Cross-archetype updates:**
For each card processed in this batch, check the reverse map (from Phase 1c) for other archetypes containing this card. For each such archetype that already has a row in the Notion tracker:
1. Search: `notion-search query="{other_archetype}" data_source_url="collection://6af6bd9a-6ef6-4e23-b476-7f7c77c2f2aa"`
2. If found: increment the appropriate count (Faithful or Fixed) for the shared card

This ensures fixing a shared card propagates progress to all archetypes that use it.

#### 4D-vii. Update PM Board

Skip if `--skip-notion`.

For each card in the batch, update the PM board (`31f97972-7634-80d0-97eb-000b817cdae1`):
1. Search for card ID in the board
2. If found: update Status (Done if PASS/IMPLEMENTED/FIXED, In progress if PARTIAL, Not started if BLOCKED)
3. If not found: create new page — Name: `{CARD_ID} {card_name}`, Status: Done/In progress, Priority: Medium, Category: QA, Effort: S

#### 4D-viii. Update engine API reference

If any fix agent reported new patterns in `### New Patterns Discovered`, append them to `qa/archetype-qa/engine-api-reference.md` in the appropriate section (Common Patterns or the relevant API section). This keeps the reference growing for subsequent batches and future runs.

#### 4D-ix. Update archetype QA doc

If `qa/archetype-qa/{archetype_name}.md` exists, update per-card verdicts. If not, create it (see Phase 5 template).

#### 4D-x. Present batch summary

```
## Batch {N}/{total} Complete

| Card ID | Name | Verdict | Review | Tests |
|---------|------|---------|--------|-------|
| {ID} | {name} | {verdict} | {APPROVED/NEEDS-FIX} | {N/N pass} |
| ... | ... | ... | ... | ... |

Running totals: {faithful} faithful, {implemented} implemented, {fixed} fixed, {deferred} deferred, {blocked} blocked
Batch tests: all passing
```

### Phase 4E: Continue to Next Batch

The orchestrator automatically proceeds to the next batch. The user can interrupt between batches if needed.

---

## Phase 5: Final Report

After all batches complete:

```markdown
# Batch Fix Report: {archetype_name}
Date: {today}
Total cards: N processed across {K} batches

## Summary
| Verdict | Count |
|---------|-------|
| FAITHFUL | {faithful_count} |
| IMPLEMENTED | {implemented_count} |
| FIXED | {fixed_count} |
| PARTIAL | {deferred_count} |
| BLOCKED | {blocked_count} |

## Per-Card Results
| Card ID | Name | Verdict | Review | Tests | Notes |
|---------|------|---------|--------|-------|-------|
| ... | ... | ... | ... | ... | ... |

## Script Changes
### {CARD_ID} {card_name}
- {description of each fix}

## Blocked Cards & Engine Gaps
### {CARD_ID} {card_name}
- Engine gap: {description}
- Effect text: "{text}"
- Added to engine-gaps.md: {yes/no}

## Tests Created
- {N} test files, {M} total test methods
- All passing: {yes/no}

## Tracking Updates
- validated_cards.json: version {old} -> {new}, {N} cards updated
- Notion Archetype Tracker: {archetype_name} — Status: {Complete/In Progress}
- Notion PM Board: {N} tasks updated/created
- Engine API Reference: {N} new patterns added
```

### 5a. Finalize Notion tracker

Skip if `--skip-notion`.

Update the archetype's row:
- Status: "Complete" (if no PARTIAL/BLOCKED) or "In Progress" (if any remain)
- Final counts: Faithful, Implemented, Fixed, Deferred, Engine Gaps
- Notes: one-line summary (e.g., "28/53 implemented, 5 faithful, 3 fixed, 1 blocked (token piercing gap)")

### 5b. Update archetype QA doc

Write or update `qa/archetype-qa/{archetype_name}.md`:

```markdown
# Archetype QA: {name}
Date: {today}
Total cards: N
Pipeline: batch-fix-cards

## Summary
- FAITHFUL: {N} (no changes needed)
- IMPLEMENTED: {N} (new script created from scratch)
- FIXED: {N} (existing script corrected)
- PARTIAL: {N} (some clauses blocked)
- BLOCKED: {N} (engine gaps)

## Per-Card Verdicts
| Card ID | Name | Verdict | Review | Tests | Notes |
|---------|------|---------|--------|-------|-------|
| ... | ... | ... | ... | ... | ... |

## Fixes Applied
### {CARD_ID} {card_name}
- {fix description}

## Blocked Cards
### {CARD_ID} {card_name}
- Effect text: "{text}"
- Engine gap: {description}

## New Patterns Discovered
- {patterns added to engine-api-reference.md}
```

### 5c. Append new engine gaps

If any cards were BLOCKED due to new engine gaps, append to `qa/archetype-qa/engine-gaps.md`:

```markdown
### {Gap Title}
- **Discovered in:** {archetype} ({date})
- **Card(s):** {CARD_ID} — {card_name}
- **Effect text:** "{...}"
- **What's missing:** {description}
- **Workaround:** {if any, otherwise "None — BLOCKED"}
```
