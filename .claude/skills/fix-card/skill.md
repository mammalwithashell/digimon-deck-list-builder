---
name: fix-card
description: Review a single card script for faithfulness against official card text, fix discrepancies, add DebugRunner tests, and track in Notion + validated_cards.json. Use for card-by-card QA walkthrough of an archetype. Accepts card ID (BT24-102), script path, or IDE-selected file.
argument-hint: <CARD_ID or SCRIPT_PATH> [--report-only] [--skip-tests] [--skip-notion]
---

# Fix Card — Single-Card Faithfulness Review, Fix & Test

Review, fix, test, and track a single Digimon TCG card script for faithfulness against official card text.

## When to Use

- Working through an archetype card-by-card to fix faithfulness issues
- Fixing a specific card reported as DISCREPANCY or QA-FAIL
- Verifying a card that was recently implemented or modified
- User has a card script open in VS Code and wants it reviewed

**Not for:** Batch archetype review (use `/review-archetype`). Not for exploratory gameplay testing (use `/gameplay-qa`). Not for implementing cards from scratch (use `/implement-archetype`).

## Flags

- `--report-only`: Analyze only, no edits or tests
- `--skip-tests`: Fix script but skip test creation/execution
- `--skip-notion`: Skip Notion board update

## Quick Reference

| Resource | Path |
|----------|------|
| Engine API Ref | `qa/archetype-qa/engine-api-reference.md` |
| Engine Gaps | `qa/archetype-qa/engine-gaps.md` |
| C# Scripts | `DCGO/Assets/Scripts/CardEffect/{SET}/{COLOR}/{CLASS_NAME}.cs` |
| Python Scripts | `code/engine_py_legacy/engine/data/scripts/{set_lower}/{set_lower}_{nnn}.py` |
| Card Metadata | `data/cards.json` |
| Validated Cards | `qa/qa-reports/validated_cards.json` |
| Archetype QA | `qa/archetype-qa/{archetype_name}.md` |
| Pinecone Index | `digimon-engine` (namespaces: engine-api, card-scripts, card-metadata) |
| Notion PM Board | data source `31f97972-7634-80d0-97eb-000b817cdae1` |

---

## Phase 1: Resolve Card Identity

Accept input in any form:
- **File path**: `code/engine_py_legacy/engine/data/scripts/bt24/bt24_102.py` or IDE-selected file
- **Card ID (hyphen)**: `BT24-102`
- **Card ID (underscore)**: `bt24_102` or `BT24_102`

Normalize to:
- `card_id`: `BT24-102` (canonical — uppercase set, hyphen)
- `set_lower`: `bt24`
- `card_number`: `102`
- `class_name`: `BT24_102` (underscore, for Python class and C# filename)
- `python_path`: `code/engine_py_legacy/engine/data/scripts/{set_lower}/{set_lower}_{card_number}.py`
- `csharp_glob`: `DCGO/Assets/Scripts/CardEffect/{SET_UPPER}/*/{CLASS_NAME}.cs`

Verify the Python script exists. If missing: report "MISSING — no script to review" and stop.

---

## Phase 2: Gather Context

Read all of these. Parallelize where possible.

### 2a. Card metadata
Read `data/cards.json`, extract entry for this card ID. Key fields:
- `card_name_eng` — display name
- `effect_description_eng` — **source of truth** for what the script must do
- `inherited_effect_description_eng` — inherited effect text
- `card_kind` — 0=Digimon, 1=Tamer, 2=Option, 3=Digitama
- `level`, `dp`, `play_cost`, `card_colors`, `type_eng` (traits), `evo_costs`

### 2b. Current Python script
Read `code/engine_py_legacy/engine/data/scripts/{set}/{set}_{nnn}.py`

### 2c. C# reference implementation
Glob for `DCGO/Assets/Scripts/CardEffect/{SET}/*/{CLASS_NAME}.cs`. Color subdirectory varies — use glob. If not found, note "C# reference not available" and proceed with card text only.

### 2d. Engine API reference
Read `qa/archetype-qa/engine-api-reference.md` — the scripting reference for card effect implementation.

### 2e. Engine gaps
Read the Remaining Gaps section of `qa/archetype-qa/engine-gaps.md`. Check if any gap blocks this card.

### 2f. Prior QA status
- Check `qa/qa-reports/validated_cards.json` for existing entry
- Grep `qa/archetype-qa/*.md` for this card ID to find prior verdicts

### 2g. Pinecone context (as needed)
Use Pinecone MCP `search-records` with index `digimon-engine`:
1. **Similar frozen scripts**: Search namespace `card-scripts` with filter `{is_frozen: true}` for 2-3 scripts with similar mechanics
2. **Mechanics lookup**: Search namespace `engine-api` for mechanics not covered in the API reference
3. **Cross-card references**: If card text names other cards, search namespace `card-metadata`

---

## Phase 3: Faithfulness Analysis

### 3a. Decompose card text into clauses

For each effect text block, identify every clause:

| Type | Pattern | Maps To |
|------|---------|---------|
| Trigger | `[On Play]`, `[When Digivolving]`, `[When Attacking]` | `EffectTiming` enum |
| Condition | "if...", "when..." | condition callback |
| Action | "delete", "return", "play", "trash", "draw" | `game.effect_*` API |
| Target | "1 of your opponent's Digimon", "all of your" | selection API + filter |
| Optionality | "you may" | `is_optional=True` |
| Duration | "for the turn", "until end of opponent's turn" | expiry parameter |
| Frequency | `[Once Per Turn]` | `set_max_count_per_turn(1)` |
| Inheritance | below inheritance line | `is_inherited_effect = True` |
| Security | `[Security]` | `EffectTiming.SecuritySkill` |
| Cost | "By [doing X]" | cost-first execution |
| Keyword | `<Blocker>`, `<Rush>`, `<Piercing>` | keyword mechanism |
| Alt-Digi | special digivolution | `_alt_digi_*` attributes |

### 3b. Compare each clause against the Python script

For each clause, verify correct timing, condition logic, action API, target scope, optionality, and duration.

### 3c. Run the 16-item Error Checklist

**Verify ALL 16 items against the script:**

1. `BeforePayCost` condition starts with: `if context.get('card_source') is not card: return False`
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

### 3d. Cross-reference C# implementation

When card text is ambiguous, use C# as behavioral source of truth:
- Multi-step selection flows (how many selections, what order)
- Exact filter logic (which conditions apply where)
- Timing of sub-effects (before vs after selection resolves)
- Edge cases (what happens when no valid targets)

### 3e. Critical common bugs to check

- **Name vs Trait**: Card names → `contains_card_name()`; Traits → `has_trait()`. Wrong API silently fails.
- **Multi-Step Selections**: Two separate player choices → TWO selection phases, not one.
- **Player Agency**: "1 of your" or "any 1" → player MUST choose via selection phase. No auto-selection.
- **"By" Costs**: "By deleting 1 of your Digimon" → deletion is a COST, happens first, not skippable.
- **Wrong Zone**: "from your trash" ≠ "from your hand". Verify zone.

### 3f. Produce analysis

For each clause:
```
Clause {N} ({type}): "{exact card text}"
Script: {what the script does}
Verdict: MATCH | MISMATCH
{If MISMATCH: Expected: {correct behavior}, Severity: critical|high|medium|low}
```

Overall verdict: **FAITHFUL** | **DISCREPANCY** | **BLOCKED**

---

## Phase 4: Fix Discrepancies

Skip if `--report-only`. Skip if FAITHFUL.

### 4a. Apply fixes
For each MISMATCH: edit the specific function/section. Follow patterns from Engine API ref and frozen scripts.

### 4b. Verify importability
```bash
python -c "from engine_py_legacy.engine.data.scripts.{set}.{set}_{nnn} import {CLASS_NAME}; print('OK')"
```

### 4c. Handle BLOCKED clauses
- Add comment: `# ENGINE GAP: {description} — see engine-gaps.md`
- If gap is NEW, append to `qa/archetype-qa/engine-gaps.md`

---

## Phase 5: Write and Run Tests

Skip if `--skip-tests` or `--report-only`.

### 5a. Write DebugRunner test

Create `code/engine_py_legacy/tests/behavioral/{set_lower}/test_{set_lower}_{nnn}.py`. Tests are organized by set subdirectory, mirroring the card script layout. Create the `{set_lower}/` directory and `__init__.py` if they don't exist. Use the `debug_runner` fixture from `code/engine_py_legacy/tests/conftest.py`.

```python
import pytest

@pytest.mark.behavioral
class Test{CLASS_NAME}{CardName}:
    """Tests for {CARD_ID} {card_name}."""

    def test_{trigger}_{expected_behavior}(self, debug_runner):
        """{Card text clause being tested}."""
        runner = debug_runner(initial_memory=N)
        runner.set_phase("Main")
        runner.inject_card(1, "{CARD_ID}", "hand")
        # Place targets as needed
        runner.place_on_field(2, ["{TARGET_CARD}"])

        action = runner.find_action("Play {card_name}")
        assert action is not None
        result = runner.execute(action)
        runner.auto_resolve()

        snap = runner.snapshot()
        # Assertions per card text...
```

**Coverage rules:**
- One test method per distinct effect clause
- Verify specific state changes the clause describes
- For conditional effects: at least one positive + one negative case if feasible

### 5b. Run tests
```bash
python -m pytest code/engine_py_legacy/tests/behavioral/test_{set_lower}_{nnn}.py -v
```

### 5c. Handle failures
If tests fail: analyze → fix script or test → rerun. ONE revision round. If still failing, report with diagnostics.

---

## Phase 6: Track Progress

### 6a. Update validated_cards.json

Read `qa/qa-reports/validated_cards.json`, add/update entry:
```json
"{CARD_ID}": {
  "card_name": "{name}",
  "validated_date": "{YYYY-MM-DD}",
  "report": "fix-card",
  "status": "{PASS|PARTIAL|BLOCKED|FAIL}",
  "notes": "{one-line summary}"
}
```
Increment `version`, update `last_updated`.

### 6b. Update archetype QA doc

Grep `qa/archetype-qa/*.md` for this card ID. If found, update the verdict row.

### 6c. Update Notion PM board

Skip if `--skip-notion`.

1. Search: `notion-search query="{CARD_ID}" data_source_url="collection://31f97972-7634-80d0-97eb-000b817cdae1"`
2. If found: `notion-update-page` → Status: Done (if PASS), In progress (if PARTIAL), Not started (if BLOCKED)
3. If not found: `notion-create-pages` with parent `data_source_id: 31f97972-7634-80d0-97eb-000b817cdae1`:
   - Name: `Fix: {CARD_ID} {card_name}`
   - Status: Done/In progress/Not started
   - Priority: Medium (or Critical if key archetype card)
   - Category: QA
   - Effort: S

### 6d. Present summary

```
## Fix Card: {CARD_ID} {card_name}
Verdict: {FAITHFUL / FIXED / PARTIAL / BLOCKED}

### Analysis
- Clauses: N total, N matched, N fixed, N blocked

### Changes
- {clause}: {what changed and why}

### Tests
- code/engine_py_legacy/tests/behavioral/test_{set}_{nnn}.py — N tests, all passing

### Tracking
- validated_cards.json: {status}
- Notion: {created/updated}
```
