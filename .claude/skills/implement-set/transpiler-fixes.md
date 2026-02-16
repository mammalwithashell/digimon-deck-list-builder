# Transpiler Fix Guide

When the review phase identifies issues, fix them in the transpiler — not in individual scripts. This document covers common patterns and where to fix them.

## Architecture Overview

The transpiler pipeline is a Python package at `tools/transpiler/`:

| Module | Role | Lines |
|--------|------|-------|
| `patterns.py` | Compiled regex patterns, TIMING_MAP, GAIN_KEYWORD_MAP | ~360 |
| `models.py` | `EffectBlock` dataclass (extracted effect metadata) | ~80 |
| `extractors.py` | C# parsing: timing blocks, factory effects, action scanning | ~850 |
| `generators.py` | Python code generation from EffectBlocks | ~940 |
| `validation.py` | Cross-validation against digimoncard.io | ~600 |
| `cli.py` | CLI entry point, file I/O, report generation | ~320 |

## Common Fix Categories

### 1. Missing Keyword Recognition

**Symptom**: Card has a keyword (e.g., `<Vortex>`) but the transpiled script doesn't set the flag.

**Fix location**: `tools/transpiler/patterns.py`

Add a new factory regex:
```python
RE_FACTORY_NEW_KEYWORD = re.compile(r'AddNewKeywordStaticEffect|NewKeywordClass')
```

Add to `GAIN_KEYWORD_MAP` if it uses `CardEffectCommons.Gain*()`:
```python
'GainNewKeyword': 'new_keyword',
```

Add a factory recognition entry in `extractors.py` `_scan_factory_patterns()`.

### 2. Missing Action Detection

**Symptom**: Card performs an action (e.g., "trash 2 from top of deck") but the callback is empty or missing.

**Fix location**: `tools/transpiler/patterns.py` and `extractors.py`

Add regex in `patterns.py`:
```python
RE_NEW_ACTION = re.compile(r'SomeActionMethod\(')
```

Add scanning in `extractors.py` `_scan_actions()`:
```python
if RE_NEW_ACTION.search(block):
    eb.actions.append("new_action")
```

Add code emission in `generators.py` `_emit_action()`:
```python
elif action == "new_action":
    lines.append(f"{indent}# New action implementation")
    lines.append(f"{indent}player.some_action()")
```

### 3. Wrong Timing Assignment

**Symptom**: Effect has wrong timing (e.g., `is_on_play` when card says `[When Digivolving]`).

**Fix location**: `tools/transpiler/patterns.py` `TIMING_MAP`

The C# timing enum maps to Python properties via `TIMING_MAP` and `TIMING_TO_PROPERTY`. If a new timing variant is found:
```python
TIMING_MAP['NewTimingEnumValue'] = 'EffectTiming.NewTiming'
```

### 4. Wrong Selection Direction

**Symptom**: Script uses `effect_select_own_permanent` when it should target opponent (or vice versa).

**Fix location**: `tools/transpiler/extractors.py`

The selection direction is determined by scanning for context clues in the C# source:
- `IsPermanentExistsOnOpponentBattleAreaDigimon` → opponent target
- `SelectPermanentEffect.Mode` + the lambda filter context

For keyword grants specifically, the `grant_target_is_opponent` field on `EffectBlock` controls direction. Check:
- `eb.grant_is_self` — set when no `SelectPermanentEffect` is found
- `eb.grant_target_is_opponent` — set when opponent area check is found
- `eb._has_select_permanent` — accumulated flag across all `_scan_actions` passes

### 5. No-Action Stub (pass-only callback)

**Symptom**: Process callback contains only `pass`.

**Investigation steps**:
1. Read the original C# source: `DCGO/Assets/Scripts/CardEffect/{SET}/CardEffect_{CARD_ID}.cs`
2. Identify what the C# `ActivateCoroutine` or `SharedActivateCoroutine` does
3. Determine if it matches an existing action pattern that the transpiler should recognize
4. If it's a new pattern, add detection + emission (categories 2-3 above)
5. If it's metadata-only (like jogress conditions), the callback should be skipped entirely

**Jogress condition stubs**: These are correctly handled — the transpiler now skips callback generation when the only actions are `jogress_condition`.

### 6. SharedActivateCoroutine Resolution

**Symptom**: Effect delegates to a shared method but the transpiler doesn't follow it.

**Fix location**: `tools/transpiler/extractors.py`

The transpiler resolves `SharedActivateCoroutine` by:
1. Detecting the coroutine reference in the timing block
2. Extracting the shared method body via `_extract_method_body()`
3. Recursively calling `_scan_actions()` on the shared body

If a new shared pattern isn't being resolved, check:
- Does `_extract_method_body()` regex match the method signature?
- Is the method name being detected in the timing block?
- Are `EffectBlock` flags being accumulated correctly across recursive calls?

### 7. Factory Condition Extraction

**Symptom**: Factory effect (keyword) has wrong conditions (e.g., always active when it should check traits).

**Fix location**: `tools/transpiler/extractors.py` `_extract_factory_conditions()`

Factory conditions are parsed from C# `CanActivateCondition` or `Condition` closures. Common patterns:
- `IsOwnerTurn` → `factory_cond_owner_turn`
- `DigivolutionCards.Count >= N` → `factory_cond_digi_count`
- `HasText("X")` → `factory_cond_has_text`
- `EqualsCardName("X")` → `factory_cond_source_name`
- `HasTrait("X")` → `factory_cond_source_trait`

## Fix Workflow

1. **Identify the issue** — which card, which effect, what's wrong
2. **Read the C# source** — understand what the effect should do
3. **Find the root cause** — which transpiler module is responsible
4. **Make the fix** — modify patterns/extractors/generators
5. **Re-transpile the affected set** — verify the fix works for the specific card
6. **Re-transpile ALL sets** — verify no regressions in other sets
7. **Run full test suite** — `python -m pytest tests/ -v`

```bash
# Re-transpile all sets after a fix
python tools/transpile_dcgo.py DCGO/Assets/Scripts/CardEffect/BT14 digimon_gym/engine/data/scripts/bt14
python tools/transpile_dcgo.py DCGO/Assets/Scripts/CardEffect/BT20 digimon_gym/engine/data/scripts/bt20
python tools/transpile_dcgo.py DCGO/Assets/Scripts/CardEffect/BT23 digimon_gym/engine/data/scripts/bt23
python tools/transpile_dcgo.py DCGO/Assets/Scripts/CardEffect/BT24 digimon_gym/engine/data/scripts/bt24

# Run tests
python -m pytest tests/ -v
```

## Engine Gap Escalation

If the review reveals that the engine itself lacks support for a mechanic:

1. Check `CLAUDE.md` "Known Gaps" — is it already tracked?
2. If not tracked, add it to the Known Gaps section
3. For the transpiled script, the transpiler should emit a `(descriptive-tagged)` marker so the effect is documented but not actively broken
4. File the engine gap as future work

**Currently unimplemented engine mechanics** (emit as descriptive tags):
- Cost reduction effects
- Redirect attack effects
- Effect immunity (partial — some implemented via Progress)
- Grant skill to other Digimon (complex targeting)
- Attack unsuspended Digimon
- Token play (partial)
- Forced attack
- SA modifier grants
- Effect disable
- Temp effect grants

## Validation Mode

After fixing transpiler issues, use validation mode to cross-check against the card API:

```bash
python tools/transpile_dcgo.py --validate DCGO/Assets/Scripts/CardEffect/{SET} digimon_gym/engine/data/scripts/{set_lower}
```

Or scan API coverage for a set:
```bash
python tools/transpile_dcgo.py --scan-api {SET_ID}
```

This reports:
- Forward mismatches (API mentions X, script missing)
- Reverse mismatches (script claims X, API doesn't mention)
- Timing mismatches
- Pattern coverage statistics
