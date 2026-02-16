---
name: implement-set
description: Implement a Digimon TCG card set by ingesting card metadata, transpiling C# scripts, reviewing transpiled output against official rules, and running validation tests. Use when asked to add a new card set, implement a set, or transpile cards.
argument-hint: <SET_ID> [--review-only]
allowed-tools: Bash, Read, Write, Edit, Glob, Grep, WebFetch, Task, TodoWrite
---

# Implement a Digimon TCG Card Set

You are implementing card set **$ARGUMENTS** for the Digimon TCG game engine. Follow this workflow precisely.

## Quick Reference

- **CLAUDE.md** — project overview, architecture, common commands
- **RULES_CONTEXT.md** — official Digimon TCG rules (keyword mechanics, effect timing, processing conditions)
- **Official Rule PDFs** (for ambiguous rules):
  - Comprehensive Rules: https://world.digimoncard.com/rule/pdf/general_rule.pdf?20251225
  - Rule Manual: https://world.digimoncard.com/rule/pdf/manual.pdf?20250711
- **Card API**: `https://digimoncard.io/index.php/api-public/search?card=<CARD_ID>`
- For detailed review procedures, see [review-checklist.md](review-checklist.md)
- For common transpiler issues and fixes, see [transpiler-fixes.md](transpiler-fixes.md)

---

## Phase 1: Ingest Card Metadata

**Goal**: Ensure all cards in the set exist in `cards.json`.

```bash
python tools/ingest_cards.py --set $ARGUMENTS
```

**Verify**: Check output for card count. Cross-reference with the expected set size by looking up the set on digimoncard.io:
```
https://digimoncard.io/index.php/api-public/search?pack=BT-{N}:%20{SetName}
```

If the set name isn't known, search the API for the set ID first. Confirm the card count matches expectations (typically 100-115 cards per booster set).

---

## Phase 2: Transpile C# Scripts

**Goal**: Convert DCGO C# CardEffect scripts to Python CardScript files.

```bash
python tools/transpile_dcgo.py DCGO/Assets/Scripts/CardEffect/$ARGUMENTS digimon_gym/engine/data/scripts/{set_lower}
```

Where `{set_lower}` is the lowercase set ID (e.g., `BT24` → `bt24`).

**Post-transpile checks**:
1. Read `TRANSPILE_REPORT.md` in the output directory
2. Note the counts: total scripts, effects (factory vs activate)
3. Identify any `no-action` entries — these are stubs that need investigation
4. Identify `(descriptive-tagged)` entries — these are known limitations

---

## Phase 3: Validate Transpiled Scripts

**Goal**: Ensure all scripts are importable and produce correct effect counts.

### 3a. Check for syntax errors
```bash
python -c "
import importlib, os, sys
sys.path.insert(0, '.')
script_dir = 'digimon_gym/engine/data/scripts/{set_lower}'
errors = []
for f in sorted(os.listdir(script_dir)):
    if f.startswith('{set_lower}_') and f.endswith('.py'):
        mod_name = f'digimon_gym.engine.data.scripts.{set_lower}.{f[:-3]}'
        try:
            importlib.import_module(mod_name)
        except Exception as e:
            errors.append((f, str(e)))
print(f'{len(errors)} errors' if errors else 'All scripts import cleanly')
for f, e in errors:
    print(f'  {f}: {e}')
"
```

### 3b. Write parametrized test file
Create `tests/test_{set_lower}_scripts.py` following the pattern in existing test files (e.g., `tests/test_bt24_scripts.py`). The test file should include:
- `test_{set}_script_imports` — each script is importable with `get_card_effects`
- `test_{set}_script_returns_effects` — each script returns a non-empty effect list
- `test_{set}_script_count` — correct number of scripts exist
- `test_{set}_cards_in_database` — all cards exist in `cards.json`

### 3c. Run tests
```bash
python -m pytest tests/test_{set_lower}_scripts.py -v
python -m pytest tests/ -v  # Full suite for regression check
```

---

## Phase 4: Review Against Official Rules

**This is the most critical phase.** Review transpiled output against official Digimon TCG rules.

For detailed procedures, see [review-checklist.md](review-checklist.md).

### High-level review process:

1. **Sample 10-15 cards** across different card types (Digimon, Tamer, Option) and complexity levels
2. **For each sampled card**:
   a. Fetch actual effect text from digimoncard.io API
   b. Read the transpiled Python script
   c. Compare extracted effects against the card text
   d. Verify keyword mechanics match RULES_CONTEXT.md definitions
   e. Check timing assignments against the card text timing indicators
3. **Check for systemic issues**: If a pattern error is found in one card, grep for the same pattern across the whole set
4. **Verify stubs**: For any `no-action` entries in TRANSPILE_REPORT.md, determine if the stub is:
   - Correct (metadata-only effect like jogress condition)
   - Fixable (transpiler pattern gap that should be addressed)
   - Known limitation (complex multi-step sequence not yet supported)

### Keyword verification priorities:
Review these keywords carefully as they have nuanced rules:
- **Blocker** — can only block while unsuspended; suspends when blocking (RULES_CONTEXT.md Section 12)
- **Piercing** — only triggers when the attacked Digimon is deleted by the battle (not by effects)
- **Retaliation** — only triggers when the Digimon with Retaliation is deleted in battle
- **Security Attack +/-** — stacks across multiple sources (card effects, inherited, granted)
- **Rush** — bypasses summoning sickness but can only attack Digimon (not players) unless it also has other keywords
- **Progress** — immune to opponent effects while attacking (blocks DP debuffs, security effects, deletion)
- **De-Digivolve** — trashes cards from top of digivolution cards, not deletion

### Timing verification:
Check these common timing mappings:
- `[On Play]` → `is_on_play = True`
- `[When Digivolving]` → `is_when_digivolving = True`
- `[When Attacking]` → timing at attack declaration
- `[On Deletion]` → `is_on_deletion = True`
- `[Start of Your Main Phase]` → `OnStartMainPhase` + owner turn check
- `[All Turns]` → no turn restriction in condition
- `[Your Turn]` → owner turn check in condition
- `[Opponent's Turn]` → opponent turn check

---

## Phase 5: Fix Issues

If Phase 4 reveals issues:

1. **Transpiler pattern gaps** → Fix in `tools/transpiler/` (see [transpiler-fixes.md](transpiler-fixes.md))
   - Then re-transpile: `python tools/transpile_dcgo.py DCGO/Assets/Scripts/CardEffect/$ARGUMENTS digimon_gym/engine/data/scripts/{set_lower}`
   - Re-run tests to verify no regressions
2. **Engine gaps** → Add to Known Gaps in CLAUDE.md if not immediately fixable
3. **Individual script issues** → **Do NOT manually edit transpiled scripts**. Fix the transpiler instead (transpiler-first policy per CLAUDE.md)

---

## Phase 6: Update Documentation

1. Update `CLAUDE.md` Known Gaps section with new stub/descriptive-tag counts
2. Update the repository structure section if new directories were created
3. Ensure `__init__.py` exists in the new scripts directory

---

## Output Summary

When complete, provide:
- Card count ingested
- Scripts transpiled (total, with effects)
- Effect counts (factory, activate)
- Stub count (no-action) and descriptive-tagged count
- Test results (pass/fail counts)
- Any issues found during review and whether they were fixed
- Any remaining known gaps
