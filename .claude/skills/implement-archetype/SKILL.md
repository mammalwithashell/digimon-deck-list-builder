---
name: implement-archetype
description: Implement or QA-review all card effects for a Digimon TCG archetype. Takes an archetype name (from deck_library.json) or a manual card list. Plans card categorization, dispatches parallel Sonnet agents with context packs, compiles QA index, runs smoke tests. Use when asked to implement an archetype, implement card effects, or QA-review archetype scripts.
argument-hint: <ARCHETYPE_NAME> [--cards CARD1,CARD2,...] [--qa-only] [--skip-smoke-test]
---

# Implement Archetype Card Effects

You are implementing all card effects for archetype **$ARGUMENTS** in the Digimon TCG game engine.

## Quick Reference

- **Engine API Reference**: `qa/archetype-qa/engine-api-reference.md` — the complete scripting reference (give this to every agent)
- **Design Spec**: `docs/superpowers/specs/2026-03-10-implement-archetype-design.md`
- **Card API**: `https://digimoncard.io/index.php/api-public/search?card=<CARD_ID>`
- **C# Scripts**: `DCGO/Assets/Scripts/CardEffect/{SET}/{COLOR}/{CARD_ID}.cs`
- **Python Scripts**: `code/engine_py_legacy/engine/data/scripts/{set_lower}/{set_lower}_{nnn}.py`
- **Frozen Manifest**: `code/engine_py_legacy/engine/data/scripts/_frozen_manifest.json`
- **Deck Library**: `digimon_gym/engine/data/deck_library.json`
- **Engine Gaps**: `qa/archetype-qa/engine-gaps.md`
- **Known Complex Cards**: `code/engine_py_legacy/engine/data/scripts/known_complex_cards.json`

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
  - `script_status` — `"frozen"` (QA-only), `"generated"` (implement from generated), `"missing"` (implement from scratch)
  - `script_path` — relative path to existing script, or `None`
  - `csharp_path` — relative path to C# reference, or `None`
  - `deck_frequency` — how many decklists include this card
- `manifest.coverage_pct`, `manifest.frozen_count`, `manifest.generated_count`, `manifest.missing_count`
- `manifest.missing_cards` — card IDs with no script at all
- `manifest.best_decklist` — single best deck list (for smoke test in Phase 6)
- `manifest.meta_share`, `manifest.total_decklists`
- `deck_pool.json` is auto-written to `qa/archetype-qa/{slug}/`

### 1b. Build card manifest

**This is handled by `resolve_archetype()` above.** Each `CardEntry` in `manifest.unique_cards` already contains:
1. **Script status**: `card.script_status` — `"frozen"` (→ QA-only), `"generated"` (→ implement from generated), `"missing"` (→ implement from scratch)
2. **C# source**: `card.csharp_path` — path to C# reference, or `None`
3. **Card metadata**: all fields populated from `cards.json`
4. **Complexity check**: Cards with 4+ distinct effects, DNA digivolve, or listed in `known_complex_cards.json` are categorized as Complex (done in Phase 1c below)

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

## Phase 3: Assemble Context, Dispatch Agents, Review & Fix

### Phase 3A: Spawn Tech Lead Agent

Spawn a dedicated Opus agent that owns research, review, and simple fixes. This agent runs with a clean context window focused on card correctness, separate from orchestration overhead.

Use the Agent tool with `model: "opus"`.

#### Tech Lead Prompt Template (Phase: Research)

```
You are the tech lead for a Digimon TCG card effect implementation batch.

## Your Responsibilities
1. RESEARCH: Query Pinecone to build curated context for implementation agents
2. REVIEW: After implementations arrive (you will be resumed), QA each script for correctness AND faithfulness
3. FIX: Apply simple fixes directly; send complex fixes back with specific instructions

## Current Phase: Research
For the cards below, prepare curated engine context that implementation agents will use.

### Step 1: Identify mechanics
Scan all card effect text below. Build a deduplicated list of mechanics needed
(e.g., Blocker, Rush, Piercing, cost reduction, alt-digi, DNA, Delay, suspend,
de-digivolve, trash recovery, reveal-and-play, tokens, etc.)

### Step 2: Query Pinecone engine-api
For each mechanic identified, search namespace "engine-api" in index "digimon-engine".
Extract: method signatures, timing enums, required arguments, usage patterns.
Format as labeled snippets grouped by mechanic.

### Step 3: Query Pinecone card-scripts
For each implementation batch, search namespace "card-scripts" with filter {is_frozen: true}
using the most distinctive card's effect text. Select top 2-3 frozen scripts as few-shot examples.
Prefer scripts that demonstrate the same mechanic combinations.

### Step 4: Query Pinecone card-metadata
For cards whose effect text references other cards by name (e.g., "when X is in play"),
search namespace "card-metadata" to fetch the referenced card's metadata.

### Step 5: Check engine gaps
Cross-reference card effects against the Known Engine Gaps section below.
Flag any cards that will be BLOCKED before implementation begins.
Also flag any effects that look like they might hit a gap not yet documented.

### Output Format
Return your research as a structured context pack:

```
## Curated Engine Context

### Mechanic: {mechanic_name}
**API:** `game.method_name(args)` — description
**Timing:** EffectTiming.{value} ({int_value})
**Pattern:**
```python
{usage snippet}
```
**Gotchas:** {any pitfalls from anti-patterns or post-mortems}

(repeat for each mechanic)

### Few-Shot Examples
#### Example 1: {CARD_ID} — {description of what it demonstrates}
```python
{frozen script}
```
(repeat for 2-3 examples per batch)

### Cross-Card References
- {CARD_ID} references "{other_card_name}" — {metadata summary}

### Pre-Flagged BLOCKED Cards
- {CARD_ID}: {reason — which gap it hits}
```

## Cards
{full card manifest: card ID, name, kind, level, colors, traits, DP, play cost,
 effect text, inherited text, security text, C# source if available}

## Engine Quick Reference
{~350 lines from engine-api-reference.md: sections 1 (Script Structure), 5 (EffectTiming Enum),
 10 (Common Patterns), 11 (Anti-Patterns), and 4 (Modifier System summary)}

## Known Engine Gaps
{contents of qa/archetype-qa/engine-gaps.md}

## Error Checklist (save for review phase — you will use this when resumed)
1. BeforePayCost condition MUST start with: if context.get('card_source') is not card: return False
2. [When Attacking] uses EffectTiming.OnUseAttack (28), NOT OnAllyAttack (32)
3. No stubs — every effect has a complete process callback with real logic
4. Inherited effects have is_inherited_effect = True on separate ICardEffect instances
5. Alt-digi includes ALL qualifying traits/names from card text (check for alternatives like Sea Animal, Aqua)
6. Tamer [Start of Your Turn] effects check memory <= N gate where card text specifies it
7. register_modifier args: game.register_modifier(target_perm, ModifierType.X, value, condition=, expiry=)
8. Option main effect uses EffectTiming.OptionSkill; security effect uses EffectTiming.SecuritySkill
9. "Ignore color requirements" conditions check specific context, NOT return True unconditionally
10. Reveal flows use game.effect_reveal_from_deck(), NOT manual list operations or trash_cards.pop()
11. Suspend/target selections offer ALL valid targets unless card text explicitly says "opponent's" or "your"
12. Piercing grants use game.effect_grant_piercing_factory(), not manual flag setting
13. OnTappedAnyone callbacks verify the suspended Digimon is the correct one (self or own field), not any Digimon
14. DP modification uses game.register_modifier(perm, ModifierType.CHANGE_DP, ...) with proper expiry, not perm.change_dp()
15. Field presence: conditions on field effects check card.permanent_of_this_card() is not None
16. Use player.battle_area, NEVER player.field_cards
```

The orchestrator reads the engine quick reference by extracting sections 1, 4, 5, 10, and 11 from `qa/archetype-qa/engine-api-reference.md` (~350 lines total).

The tech lead returns: **curated context pack** + **pre-flagged BLOCKED cards**.

### Phase 3B: Dispatch Implementation & QA Agents

#### 3B-i. Read C# scripts

For each card with a C# source, read the file contents. Include inline in the agent prompt.

#### 3B-ii. Read existing Python scripts

For QA-only cards, read the frozen script. Include inline for the agent to review.

#### 3B-iii. Dispatch agents

Use the Agent tool with `model: "sonnet"` and `isolation: "worktree"` for implementation agents.
Use `model: "sonnet"` without worktree for QA-only agents (they don't write files).

**Dispatch in parallel** — use a single message with multiple Agent tool calls.

The key change from prior workflow: agents receive the **tech lead's curated context** as their primary reference, with Pinecone MCP retained only as a fallback for edge cases.

#### QA Agent Prompt Template

```
You are QA-reviewing existing card effect scripts for the Digimon TCG simulator.

## Your Task
For each card below, compare the existing Python script against the card's official text
and C# reference implementation. Report one of:
- PASS: Script correctly implements all effects
- QA-FAIL: Script has specific issues (list them with line numbers)

## Curated Engine Context
{paste the tech lead's curated context pack here — mechanic snippets, few-shot examples, cross-references}

## Engine Quick Reference
{~350 lines from engine-api-reference.md: Script Structure + EffectTiming Enum + Common Patterns + Anti-Patterns + Modifier System summary}

## Error Checklist — Verify EACH of these for every script
1. BeforePayCost condition starts with: if context.get('card_source') is not card: return False
2. [When Attacking] uses EffectTiming.OnUseAttack (28), NOT OnAllyAttack (32)
3. No stubs — every effect has a complete process callback
4. Inherited effects have is_inherited_effect = True
5. Alt-digi includes ALL qualifying traits/names from card text
6. Tamer [Start of Your Turn] checks memory <= N gate
7. register_modifier args: game.register_modifier(target_perm, ModifierType.X, value, condition=, expiry=)
8. Option: main=OptionSkill, security=SecuritySkill
9. "Ignore color" conditions check specific context, not return True
10. Reveal flows use game.effect_reveal_from_deck(), not manual list ops
11. Suspend/target: ALL valid targets unless card text restricts scope
12. Piercing: game.effect_grant_piercing_factory()
13. OnTappedAnyone: verify the suspended Digimon is the correct target
14. DP modification: register_modifier with CHANGE_DP + expiry, not change_dp()
15. Field presence: check card.permanent_of_this_card() is not None
16. Use player.battle_area, NEVER player.field_cards

## Pinecone MCP (fallback only)
The curated context above covers the main patterns. Use Pinecone only for edge cases not
covered above. Index: "digimon-engine".

- Engine API details: search namespace "engine-api"
- Similar implementations: search namespace "card-scripts" with filter {is_frozen: true}
- Card interactions: search namespace "card-metadata"

## Self-Recovery
If you encounter an unfamiliar engine pattern or are unsure about a method:
1. Check the Curated Engine Context above first
2. Search Pinecone "engine-api" namespace for the relevant method or concept
3. Search Pinecone "card-scripts" namespace for frozen scripts with similar effects
4. If still unsure, flag it in your review rather than guessing

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
5. Before submitting each script, verify it against the Error Checklist below.

## Curated Engine Context
{paste the tech lead's curated context pack here — mechanic snippets, few-shot examples, cross-references}

## Engine Quick Reference
{~350 lines from engine-api-reference.md: Script Structure + EffectTiming Enum + Common Patterns + Anti-Patterns + Modifier System summary}

## Error Checklist — Verify EACH script against this before submitting
1. BeforePayCost condition starts with: if context.get('card_source') is not card: return False
2. [When Attacking] uses EffectTiming.OnUseAttack (28), NOT OnAllyAttack (32)
3. No stubs — every effect has a complete process callback
4. Inherited effects have is_inherited_effect = True
5. Alt-digi includes ALL qualifying traits/names from card text
6. Tamer [Start of Your Turn] checks memory <= N gate
7. register_modifier args: game.register_modifier(target_perm, ModifierType.X, value, condition=, expiry=)
8. Option: main=OptionSkill, security=SecuritySkill
9. "Ignore color" conditions check specific context, not return True
10. Reveal flows use game.effect_reveal_from_deck(), not manual list ops
11. Suspend/target: ALL valid targets unless card text restricts scope
12. Piercing: game.effect_grant_piercing_factory()
13. OnTappedAnyone: verify the suspended Digimon is the correct target
14. DP modification: register_modifier with CHANGE_DP + expiry, not change_dp()
15. Field presence: check card.permanent_of_this_card() is not None
16. Use player.battle_area, NEVER player.field_cards

## Pinecone MCP (fallback only)
The curated context above covers the main patterns. Use Pinecone only for edge cases not
covered above. Index: "digimon-engine".

- Engine API details: search namespace "engine-api"
- Similar implementations: search namespace "card-scripts" with filter {is_frozen: true}
- Card interactions: search namespace "card-metadata"

## Self-Recovery
If you encounter an unfamiliar engine pattern or get stuck:
1. Check the Curated Engine Context above first
2. Search Pinecone "engine-api" namespace for the relevant method or concept
3. Search Pinecone "card-scripts" namespace for frozen scripts with similar effects
4. If still blocked, report BLOCKED with details rather than guessing

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
1. Write the complete script file to: code/engine_py_legacy/engine/data/scripts/{set}/{set}_{nnn}.py
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

### Phase 3C: Tech Lead Review + QA

After all Sonnet agents return, **resume the tech lead agent** (using its agent ID) with the implementation results.

#### Tech Lead Resume Prompt (Phase: Review)

```
## Phase: Review + QA

The implementation agents have returned. Review each new/modified script below.
Skip QA-only cards that were marked PASS — those are trusted.

### Instructions

For each IMPLEMENTED script:

1. **Structural Review** — Run the 16-item Error Checklist (from your initial prompt) against the script:
   - Check every BeforePayCost for leak guard
   - Verify all timing enums match the card's trigger text
   - Confirm no stubs, no manual list ops, no wrong API usage
   - Verify modifier argument order and expiry types
   - Check alt-digi completeness, memory gates, target scope

2. **Faithfulness QA** — Compare the script against the card's official effect text:
   - Does the script handle ALL effects mentioned in card text?
   - Are timing enums correct for each effect type?
   - Are conditions faithful (target restrictions, color requirements, trait filters)?
   - Do optional effects have is_optional = True?
   - Is "once per turn" enforced where card text says it?
   - Are inherited effects separated with is_inherited_effect = True?
   - Does security text get a SecuritySkill-timed effect?

3. **Verdict** per script — one of:
   - **CLEAN**: Script passes both structural and faithfulness checks
   - **SIMPLE-FIX**: Wrong enum, missing guard, wrong argument order, minor condition error
     → Include the exact fix (file, line, old code → new code)
   - **COMPLEX-FIX**: Wrong effect logic, missing entire effect, needs significant rewrite
     → Include: card ID, file path, what's wrong, correct pattern/approach
   - **BLOCKED**: Hits a known engine gap → log to engine-gaps.md

### Scripts to Review

{For each IMPLEMENTED card, include:}

#### {CARD_ID} — {card_name}
**Card Text:** {effect_text}
**Inherited Text:** {inherited_text}
**Security Text:** {security_text}
**File:** code/engine_py_legacy/engine/data/scripts/{set}/{set}_{nnn}.py

```python
{script contents written by Sonnet agent}
```

### Output Format

For each script:
```
CARD_ID: CLEAN
```
or
```
CARD_ID: SIMPLE-FIX
- Fix 1: Line {N}: `{old_code}` → `{new_code}` (reason)
- Fix 2: Line {N}: `{old_code}` → `{new_code}` (reason)
```
or
```
CARD_ID: COMPLEX-FIX
- Problem: {description of what's wrong}
- Correct approach: {how to fix it, including patterns/examples}
- Affected lines: {line range}
```
or
```
CARD_ID: BLOCKED
- Engine gap: {description}
- Card text: "{the specific effect text}"
```
```

The orchestrator collects the tech lead's verdicts for Phase 3D.

### Phase 3D: Hybrid Fix Round

Based on tech lead verdicts from Phase 3C:

#### SIMPLE-FIX scripts

The **tech lead applies fixes directly**. Resume the tech lead agent one more time with instructions to apply its own SIMPLE-FIX edits in the worktree. The tech lead reads each file, applies the line-level fixes it identified, and confirms the fix.

#### COMPLEX-FIX scripts

The orchestrator **resumes the original Sonnet agent** (using its stored agent ID) with the tech lead's specific fix instructions:

```
## Revision Required

The tech lead reviewed your implementation and found issues that need fixing.
Apply these fixes and resubmit. One revision round only.

### {CARD_ID}
**Problem:** {tech lead's description}
**Correct approach:** {tech lead's instructions}
**Affected lines:** {line range}

Re-read the card text and curated context, then fix the script.
Output the corrected script and confirm: CARD_ID: REVISED
```

One revision round maximum per Sonnet agent.

#### BLOCKED scripts

Log to `qa/archetype-qa/engine-gaps.md`. No further action.

#### Final spot-check

After fixes are applied, resume the tech lead one last time for a quick spot-check on any SIMPLE-FIX or COMPLEX-FIX scripts that were revised. Verdict: CLEAN or STILL-BROKEN (with notes for Phase 5).

---

## Phase 4: Compile QA Index

After Phase 3D completes, merge all verdicts into `qa/archetype-qa/{archetype_name}.md`:

```markdown
# Archetype QA: {name}
Date: {today}
Total cards: N

## Summary
- PASS: N (existing frozen, QA-verified)
- IMPLEMENTED: N total
  - CLEAN (first pass): N
  - SIMPLE-FIX (tech lead fixed): N
  - COMPLEX-FIX (Sonnet revised): N
  - STILL-BROKEN (needs Phase 5): N
- QA-FAIL: N (existing scripts with issues)
- BLOCKED: N (engine gaps)

## Tech Lead Review
### SIMPLE-FIX Applied
| Card ID | Fix Summary |
|---------|-------------|
| {ID} | {one-line description} |

### COMPLEX-FIX Revised
| Card ID | Problem | Resolution |
|---------|---------|------------|
| {ID} | {problem} | {fixed/still-broken} |

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

**Update engine gaps tracker**: Append any new BLOCKED items to `qa/archetype-qa/engine-gaps.md`.

---

## Phase 5: Fix Remaining Issues

This phase handles only scripts that survived as STILL-BROKEN after Phase 3D, plus any QA-FAIL cards from QA-only agents. Most fixes should already be handled by the tech lead in Phase 3D.

For each remaining issue:
1. Read the issue details and tech lead notes (if any)
2. Apply the fix to the script
3. Verify the fix addresses the reported issue

Skip this phase if `--qa-only` flag was passed (just report, don't fix).

### 5b. Update Pinecone with new scripts

After fixing QA failures and writing new scripts, ingest updated card scripts into Pinecone
so the next archetype run benefits from them:

```bash
python code/tools/ingest_pinecone.py --namespace card-scripts --set {set_id}
```

---

## Phase 6: Verification

Skip if `--skip-smoke-test` flag was passed.

### 6a. Smoke test

Pick a deck list from the archetype and run 50 mirror-match episodes:

```python
# manifest.best_decklist is already populated from Phase 1
deck = manifest.best_decklist

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

For Complex cards and fixed QA-FAIL cards, write pytest cases in `code/engine_py_legacy/tests/behavioral/test_{archetype_name}.py`.

Each test should use `DebugRunner` (via the `debug_runner` fixture from `code/engine_py_legacy/tests/conftest.py`):
1. Set up a specific board state with real cards
2. Trigger the effect
3. Assert the outcome via snapshots

```python
import pytest

@pytest.mark.behavioral
class TestGankoomonEffects:
    def test_bt23_057_cost_reduction(self, debug_runner):
        """Gankoomon cost should be reduced by 5 when 3+ qualifying cards in trash."""
        runner = debug_runner(archetype1="Jesmon GX (Gankoomon)", initial_memory=10)
        runner.inject_card(1, "BT23-057", "hand")
        # Put qualifying cards in trash
        for cid in ["BT23-060", "BT23-061", "BT23-062"]:
            runner.inject_card(1, cid, "trash")
        action = runner.find_action("Play Gankoomon")
        result = runner.execute(action)
        runner.auto_resolve()
        # Verify reduced cost was applied
        assert result.after.memory > result.before.memory - 7  # Cost reduced from 12

    def test_bt23_077_blocker_keyword(self, debug_runner):
        """Sistermon Ciel should have Blocker keyword on field."""
        runner = debug_runner(archetype1="Jesmon GX (Gankoomon)", initial_memory=10)
        runner.place_on_field(1, ["BT23-077"])
        snap = runner.snapshot()
        assert "blocker" in snap.p1_field[0].keywords
```

Run: `python -m pytest code/engine_py_legacy/tests/behavioral -v`

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
- code/engine_py_legacy/engine/data/scripts/{set}/{files}...
- code/engine_py_legacy/tests/test_archetype_{name}.py
- qa/archetype-qa/{name}.md

### Engine Gaps Found
- {list any BLOCKED items requiring engine changes}
```

---

## Flags

- `--qa-only`: Only QA-review existing scripts, don't implement missing ones, don't fix QA failures
- `--skip-smoke-test`: Skip the 50-game smoke test
- `--cards CARD1,CARD2,...`: Override card pool with explicit list instead of deck_library.json lookup
