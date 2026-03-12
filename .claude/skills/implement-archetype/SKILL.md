---
name: implement-archetype
description: Implement or QA-review all card effects for a Digimon TCG archetype. Takes an archetype name (from deck_library.json) or a manual card list. Plans card categorization, dispatches parallel Sonnet agents with context packs, compiles QA index, runs smoke tests. Use when asked to implement an archetype, implement card effects, or QA-review archetype scripts.
argument-hint: <ARCHETYPE_NAME> [--cards CARD1,CARD2,...] [--qa-only] [--skip-smoke-test]
---

# Implement Archetype Card Effects

You are implementing all card effects for archetype **$ARGUMENTS** in the Digimon TCG game engine.

## Quick Reference

- **Engine API Reference**: `docs/archetype-qa/engine-api-reference.md` — the complete scripting reference (give this to every agent)
- **Design Spec**: `docs/superpowers/specs/2026-03-10-implement-archetype-design.md`
- **Card API**: `https://digimoncard.io/index.php/api-public/search?card=<CARD_ID>`
- **C# Scripts**: `DCGO/Assets/Scripts/CardEffect/{SET}/{COLOR}/{CARD_ID}.cs`
- **Python Scripts**: `digimon_gym/engine/data/scripts/{set_lower}/{set_lower}_{nnn}.py`
- **Frozen Manifest**: `digimon_gym/engine/data/scripts/_frozen_manifest.json`
- **Deck Library**: `digimon_gym/engine/data/deck_library.json`
- **Engine Gaps**: `docs/archetype-qa/engine-gaps.md`
- **Known Complex Cards**: `digimon_gym/engine/data/scripts/known_complex_cards.json`

---

## Phase 1: Resolve Card Pool

### 1a. Parse input

If `$ARGUMENTS` contains `--cards`, use the provided comma-separated card IDs.
Otherwise, look up the archetype name in `deck_library.json`:

```python
import json
from pathlib import Path
from collections import Counter

lib = json.loads(Path('digimon_gym/engine/data/deck_library.json').read_text())
archetype = lib['archetypes'].get('ARCHETYPE_NAME', {})

all_cards = set()
for dl in archetype.get('decklists', []):
    all_cards.update(json.loads(dl['decklist']))

print(f'Unique cards: {len(all_cards)}')
for card_id in sorted(all_cards):
    print(f'  {card_id}')
```

### 1b. Build card manifest

For each card ID, determine:

1. **Existing script status**: Check `_frozen_manifest.json` for a frozen entry. If present → QA-only. If only in `generated/` → Implement (use generated as starting point). If missing → Implement from scratch.

2. **C# source availability**: Search `DCGO/Assets/Scripts/CardEffect/` for `{CARD_ID}.cs`. The directory structure is `{SET}/{COLOR}/{CARD_ID}.cs`. Use glob/find to locate since color subdirectory varies.

3. **Card metadata**: Fetch from DigimonCard.io API or local `cards.json`:
   ```
   https://digimoncard.io/index.php/api-public/search?card=CARD_ID
   ```
   Extract: name, kind, level, colors, traits, DP, play cost, effect text, inherited text, security text.

4. **Complexity check**: Cards with 4+ distinct effects, DNA digivolve, or listed in `known_complex_cards.json` are categorized as Complex.

### 1c. Categorize cards

| Category | Criteria | Batch Size |
|----------|----------|------------|
| QA-only | Frozen script exists | 8-10 per agent |
| Implement | No frozen script | 3-5 per agent |
| Complex | 4+ effects, DNA, multi-step, or known complex | 1-3 per agent |

---

## Phase 2: Present Plan for Approval

Show the user a summary table:

```
Archetype: {name}
Total unique cards: N
  QA-only:  N (existing frozen scripts to review)
  Implement: N (new scripts to write)
  Complex:  N (need extra context/care)

Cards missing C# source: N
  {list card IDs}

Proposed batches: N agents
  Batch 1 (QA): [CARD-001, CARD-002, ...] (8 cards)
  Batch 2 (Implement): [CARD-010, CARD-011, ...] (4 cards)
  ...

Estimated time: ~N minutes
```

**Grouping rules:**
- Cards that reference each other by name/trait should be in the same batch
- Tamers that buff specific Digimon should be batched with those Digimon
- Option cards used by the archetype should be batched with Digimon they target

**Wait for user approval before proceeding.**

---

## Phase 3: Assemble Context Packs and Dispatch Agents

### 3a. Prepare compact engine reference

Read only the **Script Structure** and **Anti-Patterns** sections from `docs/archetype-qa/engine-api-reference.md` (~150 lines). Agents will use Pinecone MCP to look up full API details on demand.

### 3b. Read C# scripts

For each card with a C# source, read the file contents. Include inline in the agent prompt.

### 3c. Read existing Python scripts

For QA-only cards, read the frozen script. Include inline for the agent to review.

### 3d. Select few-shot examples via Pinecone

For each implementation batch (especially Complex cards), use Pinecone MCP to search the `card-scripts` namespace with filter `{is_frozen: true}` for 2-3 similar frozen scripts. Include the best matches inline in the agent prompt as few-shot examples.

### 3e. Dispatch agents

Use the Agent tool with `model: "sonnet"` and `isolation: "worktree"` for implementation agents.
Use `model: "sonnet"` without worktree for QA-only agents (they don't write files).

**Dispatch in parallel** — use a single message with multiple Agent tool calls.

#### QA Agent Prompt Template

```
You are QA-reviewing existing card effect scripts for the Digimon TCG simulator.

## Your Task
For each card below, compare the existing Python script against the card's official text
and C# reference implementation. Report one of:
- PASS: Script correctly implements all effects
- QA-FAIL: Script has specific issues (list them with line numbers)

## Engine Quick Reference
{Script Structure + Anti-Patterns sections from engine-api-reference.md}

## Dynamic Context (Pinecone MCP)
You have access to Pinecone for searching the engine knowledge base. The index is "digimon-engine".

- Engine API details: search namespace "engine-api" for methods, patterns, or concepts
- Similar implementations: search namespace "card-scripts" with filter {is_frozen: true}
- Card interactions: search namespace "card-metadata" for cards referenced by name in effects

Examples:
- Find Blocker implementation patterns: search "engine-api" for "Blocker keyword"
- Find frozen scripts with Rush: search "card-scripts" for "Rush grant keyword"
- Look up a card's text: search "card-metadata" for the card ID

## Self-Recovery
If you encounter an unfamiliar engine pattern or are unsure about a method:
1. Search Pinecone "engine-api" namespace for the relevant method or concept
2. Search Pinecone "card-scripts" namespace for frozen scripts with similar effects
3. If still unsure, flag it in your review rather than guessing

## Cards to Review

### Card 1: {CARD_ID} — {card_name}
**Card Text:** {effect_text}
**Inherited Text:** {inherited_text}
**Security Text:** {security_text}
**Kind:** {kind} | **Level:** {level} | **Colors:** {colors} | **Traits:** {traits}

**C# Reference:**
```csharp
{c# file contents}
```

**Current Python Script:**
```python
{python file contents}
```

### Card 2: ...
(repeat for each card)

## Output Format
For each card, output exactly:
```
CARD_ID: PASS
```
or
```
CARD_ID: QA-FAIL
- Issue 1: {description} (line {N})
- Issue 2: {description} (line {N})
```
```

#### Implementation Agent Prompt Template

```
You are implementing card effect scripts for the Digimon TCG simulator.

## Your Task
For each card below, write a complete Python card effect script. Follow the engine API
reference exactly. Use the C# reference as behavioral guidance and the card text as the
source of truth.

CRITICAL RULES:
1. Do NOT stub or approximate any effect. If you cannot faithfully implement an effect,
   report BLOCKED with details of what's missing.
2. Every BeforePayCost condition MUST start with: if context.get('card_source') is not card: return False
3. Effects below the inheritance line need separate ICardEffect instances with is_inherited_effect = True
4. Use the exact boilerplate and patterns from the engine API reference.

## Engine Quick Reference
{Script Structure + Anti-Patterns sections from engine-api-reference.md}

## Dynamic Context (Pinecone MCP)
You have access to Pinecone for searching the engine knowledge base. The index is "digimon-engine".

- Engine API details: search namespace "engine-api" for methods, patterns, or concepts
- Similar implementations: search namespace "card-scripts" with filter {is_frozen: true}
- Card interactions: search namespace "card-metadata" for cards referenced by name in effects

Examples:
- Find how to grant Blocker: search "engine-api" for "Blocker keyword grant"
- Find frozen scripts with Rush: search "card-scripts" for "Rush grant keyword"
- Look up a card referenced in effect text: search "card-metadata" for the card name

## Self-Recovery
If you encounter an unfamiliar engine pattern or get stuck:
1. Search Pinecone "engine-api" namespace for the relevant method or concept
2. Search Pinecone "card-scripts" namespace for frozen scripts with similar effects
3. If still blocked, report BLOCKED with details rather than guessing

## Few-Shot Examples
{2-3 similar frozen scripts selected via Pinecone search, if available}

## Cards to Implement

### Card 1: {CARD_ID} — {card_name}
**Card Text:** {effect_text}
**Inherited Text:** {inherited_text}
**Security Text:** {security_text}
**Kind:** {kind} | **Level:** {level} | **Colors:** {colors} | **Traits:** {traits} | **DP:** {dp} | **Play Cost:** {cost}

**C# Reference:**
```csharp
{c# file contents}
```

### Card 2: ...

## Output Format
For each card:
1. Write the complete script file to: digimon_gym/engine/data/scripts/{set}/{set}_{nnn}.py
2. After writing, output the verdict:
```
CARD_ID: IMPLEMENTED
```
or
```
CARD_ID: BLOCKED
- Missing mechanic: {what the engine doesn't support}
- Effect text: "{the specific effect text that can't be implemented}"
- Suggested engine change: {brief description}
```
```

---

## Phase 4: Compile QA Index

After all agents return, merge verdicts into `docs/archetype-qa/{archetype_name}.md`:

```markdown
# Archetype QA: {name}
Date: {today}
Total cards: N

## Summary
- PASS: N
- IMPLEMENTED: N
- QA-FAIL: N
- BLOCKED: N

## QA Failures
### {CARD_ID} {card_name}
- Issue: {description}
- Line: {N}
- Severity: high|medium|low

## Blocked Cards
### {CARD_ID} {card_name}
- Effect text: "{...}"
- Missing mechanic: {description}
- Suggested engine change: {description}

## Implementation Notes
{any cross-card notes from agents}
```

**Update engine gaps tracker**: Append any new BLOCKED items to `docs/archetype-qa/engine-gaps.md`.

---

## Phase 5: Fix QA Failures

For each QA-FAIL card:
1. Read the issue details from the agent
2. Apply the fix to the script
3. Verify the fix addresses the reported issue

Skip this phase if `--qa-only` flag was passed (just report, don't fix).

### 5b. Update Pinecone with new scripts

After fixing QA failures and writing new scripts, ingest updated card scripts into Pinecone
so the next archetype run benefits from them:

```bash
python tools/ingest_pinecone.py --namespace card-scripts --set {set_id}
```

---

## Phase 6: Verification

Skip if `--skip-smoke-test` flag was passed.

### 6a. Smoke test

Pick a deck list from the archetype and run 50 mirror-match episodes:

```python
import json
from pathlib import Path

lib = json.loads(Path('digimon_gym/engine/data/deck_library.json').read_text())
archetype = lib['archetypes']['ARCHETYPE_NAME']
deck = json.loads(archetype['decklists'][0]['decklist'])

# Run via engine directly
from digimon_gym.engine.game import HeadlessGame

crashes = 0
for i in range(50):
    try:
        game = HeadlessGame(deck1=deck, deck2=deck)
        while not game.game_over:
            mask = game.action_mask()
            valid = [i for i, v in enumerate(mask) if v]
            if not valid:
                break
            action = valid[0]  # greedy first valid action
            game.step(action)
    except Exception as e:
        crashes += 1
        print(f'Game {i} crashed: {e}')

print(f'{50 - crashes}/50 games completed successfully')
```

If crashes occur, identify the failing script from the stack trace and fix it. Rerun until 50 clean games.

### 6b. Targeted effect tests

For Complex cards and fixed QA-FAIL cards, write pytest cases in `tests/test_archetype_{name}.py`.

Each test should:
1. Set up a specific board state using HeadlessGame
2. Trigger the effect
3. Assert the outcome

```python
def test_bt23_057_cost_reduction():
    """Gankoomon cost should be reduced by 5 when 3+ qualifying cards in trash."""
    # Setup: create game, put qualifying cards in trash, verify cost reduction
    ...

def test_bt23_077_blocker_keyword():
    """Sistermon Ciel should have Blocker keyword on field."""
    ...
```

Run: `python -m pytest tests/test_archetype_{name}.py -v`

---

## Phase 7: Final Report

Present to the user:

```
## Archetype Implementation Complete: {name}

### Results
- Total cards: N
- PASS (existing, verified): N
- IMPLEMENTED (new scripts): N
- QA-FAIL (found and fixed): N
- BLOCKED (needs engine work): N

### Smoke Test
- 50/50 games completed successfully

### Targeted Tests
- N tests written, all passing

### New/Modified Files
- digimon_gym/engine/data/scripts/{set}/{files}...
- tests/test_archetype_{name}.py
- docs/archetype-qa/{name}.md

### Engine Gaps Found
- {list any BLOCKED items requiring engine changes}
```

---

## Flags

- `--qa-only`: Only QA-review existing scripts, don't implement missing ones, don't fix QA failures
- `--skip-smoke-test`: Skip the 50-game smoke test
- `--cards CARD1,CARD2,...`: Override card pool with explicit list instead of deck_library.json lookup
