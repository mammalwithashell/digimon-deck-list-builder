# Archetype Faithfulness Campaign

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring all 17 priority archetypes to faithful card implementations by fixing all outstanding bugs from the 03-13 through 03-15 QA campaign, then re-QA to confirm.

**Architecture:** Three-phase approach: (1) Fix critical/crash bugs and engine gaps that block multiple archetypes, (2) Fix per-card script bugs organized by archetype, (3) Re-QA all 17 archetypes and update the `qa/archetype-qa/` docs with current verdicts.

**Tech Stack:** Python 3.11+ engine scripts, headless game engine, debug game API for verification.

---

## Current State

The 03-15 cross-archetype campaign ran 190+ automated games across 8 matchup pairs and found **23 distinct card-level issues + 2 systemic engine issues**. These findings were written to `qa/qa-reports/2026-03-15-*.md` but **were NOT backported to the `qa/archetype-qa/*.md` docs**. The archetype-qa docs still reflect the 03-14 implementation pass verdicts.

Additionally, the 03-13 individual archetype reviews found many more bugs (see `qa/qa-reports/INDEX.md` — 191 outstanding issues total across all reports).

### Outstanding Bug Inventory (from 03-15 campaign summary + earlier reports)

**Crashes (5):**
| Bug | Card | Impact | Fix Complexity |
|-----|------|--------|----------------|
| BT18-100 DelaySkill enum | Gospel of the Fallen Angel | Blocks Millenniummon + Galacticmon | 1-line |
| BT24-056 CANNOT_BE_RETURNED | Dezipmon | 45% crash in Zephagamon matchups, 25 scripts affected | Enum add or batch replace |
| BT15-069 is_player_one | Candlemon | 30-40% Dark Masters crash | 1-line |
| BT14-044 card_name vs card_names | Palmon | 50% random game crash | 1-line |
| 7 uppercase filenames | Various | Zero effects on 7 cards | git mv |

**Systemic Engine Issues (5):**
| Issue | Impact | Archetypes | Priority |
|-------|--------|------------|----------|
| BeforePayCost process callbacks never execute | Free cost reductions (BT23-057, BT13-086, BT18-073+) | Jesmon, RK, Millenniummon | HIGH |
| [Main] field effects unreachable (OnDeclaration) | 97+ scripts' [Main] effects can't activate | Jesmon, many others | HIGH |
| Cross-archetype deadlock after selection decline (#42) | Empty action mask after declining selection | Multi-archetype | MEDIUM |
| CANNOT_DIGIVOLVE modifier not checked in action mask (#43) | Digivolve actions still appear | Royal Knights | LOW |
| CANNOT_ADD_SECURITY modifier not enforced (#45) | Recovery/add-security ignores restriction | Royal Knights | LOW |

**Per-Card QA-FAIL (from 03-15 campaign):**
| Card | Archetype | Issues |
|------|-----------|--------|
| BT24-071 Raidramon | Chaos Control | SA+1 is no-op, On Deletion filter missing |
| BT24-079 Hadesmon | Chaos Control | No timing, link not implemented |
| EX10-054 VenomMyotismon | Chaos Control | Suspend 1 not 2, wrong target for cannot-unsuspend |
| BT20-073 MetalPhantomon | Chaos Control | Skips deletion cost, no level filter |
| EX8-057 DemiDevimon | Chaos/Rocks | Wrong zone, wrong filter, wrong count |
| BT24-097 Soul Fear | Chaos Control | Fabricated When Attacking effect |
| BT24-074 SkullSeadramon | Chaos Control | Wrong mechanic entirely |
| BT23-097 Seventh Penetration | Hudiemon | Missing level >= hand size filter |
| BT21-029 Medusamon | Medusamon | Petrification tokens never generated |
| BT23-076 Sistermon Blanc | Jesmon | On Play completely wrong |
| BT20-013 BaoHuckmon | Jesmon | [Main] unreachable (systemic) |
| P-216 WaruMonzaemon | Dark Masters | OnEnterFieldAnyone fires on ALL plays |
| BT13-086 Gizmon: XT | Royal Knights | Free cost reduction, overbroad filters |
| BT13-100 Yoshino Fujieda | Royal Knights | Wrong trigger type, wrong target |
| BT13-036 Liollmon | Royal Knights | Auto-selects DP target, missing condition |
| BT24-098 Invasion of Titans | TS Jupitermon | Trashes 1 not 2, missing Titan filter |
| BT18-073 Machinedramon | Millenniummon | Cost reduction without deletion cost |

**Per-Card bugs from 03-13 reviews (outstanding, not yet fixed):**
- DNA Omnimon: 13 outstanding issues (`2026-03-13-dna-omnimon.md`)
- ExMaquinamon: 14 outstanding issues (`2026-03-13-exmaquinamon.md`)
- Galacticmon: 27 outstanding issues (`2026-03-13-galacticmon.md`)
- Puppets: 32 outstanding issues (`2026-03-13-puppets.md`)
- Dark Masters: 18 outstanding issues (`2026-03-13-dark-masters.md`)
- Jesmon: 8 outstanding issues (`2026-03-13-jesmon.md`)
- TS Jupitermon: 5 outstanding issues (`2026-03-13-ts-jupitermon.md`)
- TS Neptunemon: 9 outstanding issues (`2026-03-03-ts-neptune-gameplay.md`)
- Medusamon: 7 outstanding issues (`2026-03-09-medusa-regression.md`)
- Royal Knights: 3 outstanding issues (`2026-03-03-royal-knights-script-audit.md`)

**Pre-03-13 outstanding issues (from older reports, still open):**
- Diaboromon: 2 issues — BT22-091 SwitchDefender (#7), Overclock EOT (#8) (`2026-03-01-diaboromon.md`)
- TS Neptunemon: Lanamon hand-card placement (#4), Divermon no DP (#7) (`2026-03-01-ts-neptune.md`)
- Royal Knights: BT20-056 Alphamon DP 3000 vs 11000 (#27) (`2026-03-03-royal-knights-gameplay.md`)
- Millenniummon: Batch fix incomplete constraint detection (#22) (`2026-03-03-millennium-retest.md`)
- TS Olympos: De-Digivolve uses attack action IDs (#65) (`2026-03-11-ts-olympos-vs-imperialdramon.md`)
- CS Hudiemon: 3 PARTIAL cards (engine limitations: BT3-103, EX1-068, EX1-071) (`2026-03-01-cs-hudiemon-partial-retest.md`)
- Medusamon: BT5-008 Gaossmon DP over-applied (#5) (`2026-03-01-medusa-partial-retest.md`)

---

## Phase 1: Critical Fixes (unblocks all archetypes)

### Task 1.1: Fix 5 Crash Bugs

These are independent 1-line fixes. Can be done in parallel.

**Files:**
- Modify: `digimon_gym/engine/data/scripts/bt18/bt18_100.py:87`
- Modify: `digimon_gym/engine/data/scripts/bt24/bt24_056.py:73,110`
- Modify: `digimon_gym/engine/data/scripts/bt15/bt15_069.py:42`
- Modify: `digimon_gym/engine/data/scripts/bt14/bt14_044.py:80`

- [ ] **Step 1: Fix BT18-100 DelaySkill**
  Replace `effect2.set_timing(EffectTiming.DelaySkill)` with:
  ```python
  effect2.set_timing(EffectTiming.OnStartMainPhase)
  effect2._is_delay_effect = True
  effect2._is_field_main = True
  ```
  Add condition for field presence and owner's turn.

- [ ] **Step 2: Fix BT24-056 CANNOT_BE_RETURNED**
  Decide: add `CANNOT_BE_RETURNED` to `ModifierType` enum, OR batch-replace all 25 scripts to use `CANNOT_BE_REMOVED`.
  Recommended: batch-replace since `CANNOT_BE_REMOVED` already has enforcement.
  ```bash
  grep -rl "CANNOT_BE_RETURNED" digimon_gym/engine/data/scripts/ | head -30
  ```
  Replace all occurrences with `CANNOT_BE_REMOVED`.

- [ ] **Step 3: Fix BT15-069 is_player_one**
  Replace `player.is_player_one` with `player.player_id == 1` (2 occurrences).

- [ ] **Step 4: Fix BT14-044 card_name → card_names**
  Replace `top_card.card_name` with `top_card.card_names[0]` on line 80.

- [ ] **Step 5: Verify fixes**
  Run a Jesmon vs Dark Masters smoke test (10 games greedy) to confirm 0 crashes.
  ```bash
  python -c "from digimon_gym.engine.runners.headless_game import HeadlessGame; ..."
  ```

- [ ] **Step 6: Commit**
  ```bash
  git add digimon_gym/engine/data/scripts/bt18/bt18_100.py \
          digimon_gym/engine/data/scripts/bt24/bt24_056.py \
          digimon_gym/engine/data/scripts/bt15/bt15_069.py \
          digimon_gym/engine/data/scripts/bt14/bt14_044.py
  git commit -m "fix: resolve 4 crash bugs (DelaySkill, CANNOT_BE_RETURNED, is_player_one, card_name)"
  ```

### Task 1.2: Fix 7 Uppercase Filenames

**Files:**
- Rename: `digimon_gym/engine/data/scripts/bt24/BT24_016.py` → `bt24_016.py`
- Rename: `digimon_gym/engine/data/scripts/bt24/BT24_082.py` → `bt24_082.py`
- Rename: `digimon_gym/engine/data/scripts/bt24/BT24_089.py` → `bt24_089.py`
- Rename: `digimon_gym/engine/data/scripts/bt21/BT21_072.py` → `bt21_072.py`
- Rename: `digimon_gym/engine/data/scripts/ex8/EX8_074.py` → `ex8_074.py`
- Rename: `digimon_gym/engine/data/scripts/ex9/EX9_013.py` → `ex9_013.py`
- Rename: `digimon_gym/engine/data/scripts/p/P_206.py` → `p_206.py`

- [ ] **Step 1: Rename all 7 files**
  ```bash
  git mv digimon_gym/engine/data/scripts/bt24/BT24_016.py digimon_gym/engine/data/scripts/bt24/bt24_016.py
  git mv digimon_gym/engine/data/scripts/bt24/BT24_082.py digimon_gym/engine/data/scripts/bt24/bt24_082.py
  # ... etc for all 7
  ```

- [ ] **Step 2: Verify scripts load**
  ```python
  python -c "from digimon_gym.engine.data.scripts.bt24 import bt24_016; print('OK')"
  ```

- [ ] **Step 3: Commit**
  ```bash
  git commit -m "fix: rename 7 uppercase script files to lowercase for import"
  ```

### Task 1.3: Fix P-216 WaruMonzaemon Mass Deletion

**Files:**
- Modify: `digimon_gym/engine/data/scripts/p/p_216.py`

- [ ] **Step 1: Read script and C# reference**
  Read `p_216.py` and `DCGO/Assets/Scripts/CardEffect/P/P_216.cs` (note: underscore convention per feedback).

- [ ] **Step 2: Fix condition2 to only fire on own play**
  The `OnEnterFieldAnyone` condition must check that `context.get('played_permanent')` is this card's permanent.

- [ ] **Step 3: Verify via debug game**
  Play P-216, then play another card — confirm no spurious deletion.

- [ ] **Step 4: Commit**

---

## Phase 2: Engine Fixes (systemic issues)

### Task 2.1: BeforePayCost Process Callbacks

**Context:** `calculate_play_cost()` reads `cost_reduction` but never calls `on_process_callback`. This means cards that should pay a cost for their discount (delete a Digimon, return cards from trash, etc.) get the discount for free.

**Files:**
- Modify: `digimon_gym/engine/game/game.py` (or wherever `calculate_play_cost` / `action_play_card` lives)

- [ ] **Step 1: Find the exact location**
  ```bash
  grep -n "calculate_play_cost\|action_play_card\|BeforePayCost" digimon_gym/engine/game/game.py
  ```

- [ ] **Step 2: Read the C# DCGO implementation**
  Check how DCGO handles BeforePayCost — it likely calls the process callback during play execution (after cost is confirmed, before paying).

- [ ] **Step 3: Implement process callback invocation**
  After `calculate_play_cost()` determines the reduction, and the player commits to the play action, call `execute_effects(EffectTiming.BeforePayCost)` which invokes `on_process_callback` on each matched effect.

- [ ] **Step 4: Test with BT23-057 Gankoomon**
  Debug game: play Gankoomon with 3 Huckmon/Sistermon in trash. Verify they get returned to deck as cost.

- [ ] **Step 5: Test with BT13-086 Gizmon: XT**
  Debug game: play Gizmon:XT with a Lv4 Digimon on field. Verify Lv4 is deleted as cost.

- [ ] **Step 6: Commit**

### Task 2.2: Selection Phase Deadlock (#42)

**Context:** After declining a selection phase (e.g., Medusa Elizamon On Play), the action mask returns empty despite Main phase and active Digimon. Reproduces across multiple cross-archetype games.

**Files:**
- Modify: `digimon_gym/engine/game/game.py` (selection decode / phase fallback logic)

- [ ] **Step 1: Reproduce the deadlock**
  Run Royal Knights vs Medusa debug game, trigger Elizamon On Play, decline selection.

- [ ] **Step 2: Trace the root cause**
  The `_decode_selection()` guard was added for Report 22 (#23) but the cross-archetype case still fails. Investigate why the phase doesn't fall back to Main after selection decline.

- [ ] **Step 3: Fix and test**
  Ensure declining any optional selection cleanly returns to Main phase with a valid action mask.

- [ ] **Step 4: Commit**

### Task 2.3: [Main] Field Effect Action Mask (OnDeclaration)

**Context:** Scripts with `_is_field_main = True` should expose [Main] activation in the action mask, but `OnDeclaration` timing isn't collected for field permanents.

**Files:**
- Modify: `digimon_gym/engine/game/action_mask.py` (or equivalent)

- [ ] **Step 1: Trace how `_is_field_main` actions (30-59) are masked**
  Read action_mask.py and understand why `_is_field_main` effects aren't surfaced.

- [ ] **Step 2: Read C# reference for [Main] field effects**
  Check DCGO's handling of `OnDeclaration` / field-activated [Main] effects.

- [ ] **Step 3: Implement fix**
  Likely need to scan field permanents for `_is_field_main` effects and expose actions 30-59 when conditions are met.

- [ ] **Step 4: Test with BT20-013 BaoHuckmon**
  Debug game: have BaoHuckmon on field with valid Sistermon/Gankoomon in hand. Verify [Main] action appears in mask.

- [ ] **Step 5: Commit**

### Task 2.4: CANNOT_DIGIVOLVE and CANNOT_ADD_SECURITY Enforcement (DEFERRED)

**Status:** DEFERRED — low priority. These modifiers are registered by scripts (BT13-007, BT9-103) but not enforced in the action mask / recovery path. Only 2 cards affected. Can be addressed after the main faithfulness campaign.

- Engine gap: `CANNOT_DIGIVOLVE` — action mask still shows digivolve actions
- Engine gap: `CANNOT_ADD_SECURITY` — `recovery()` doesn't check it

---

## Phase 3: Per-Archetype Script Fixes

Each task here fixes all outstanding bugs for one archetype. Tasks are independent and can run in parallel (via `/implement-archetype` or manual agent dispatch). Order is by priority from the campaign summary.

**IMPORTANT: ExMaquinamon and Galacticmon share EX11-series scripts.** Tasks 3.10 and 3.11 MUST run sequentially, not in parallel, to avoid merge conflicts. See assignment table below.

**For each archetype task, the agent MUST:**
1. Read the archetype's current QA doc (`qa/archetype-qa/{name}.md`)
2. Read ALL relevant qa-reports (03-13 initial + 03-15 campaign)
3. Read each buggy script + its C# reference via Pinecone MCP
4. Fix each script to faithfully match card text
5. Run 10-game smoke test
6. Run targeted debug games for each fixed card
7. **Output a structured verdict table** (Card ID | Verdict | Notes) for Phase 4 consumption

### Task 3.1: Chaos Control (7 cards)

**QA Reports:** `2026-03-15-rocks-chaos-control-qa.md`
**Cards to fix:**
| Card | Issue Summary |
|------|---------------|
| BT24-071 Raidramon | SA+1 is `pass`, On Deletion filter missing |
| BT24-079 Hadesmon | No timing on When Digivolving, link unimplemented |
| EX10-054 VenomMyotismon | Suspend 1→2, cannot-unsuspend target wrong |
| BT20-073 MetalPhantomon | Missing self-deletion cost, no level filter |
| EX8-057 DemiDevimon | Wrong zone, wrong filter, wrong count |
| BT24-097 Soul Fear | Fabricated When Attacking effect |
| BT24-074 SkullSeadramon | Wrong mechanic entirely |

- [ ] Fix each card (read C# ref, rewrite script)
- [ ] Smoke test: 20 games Chaos Control mirror
- [ ] Debug test: each fixed card individually
- [ ] Commit

### Task 3.2: Jesmon (3 cards + systemic dependency)

**QA Reports:** `2026-03-13-jesmon.md`, `2026-03-15-jesmon-dark-masters-qa.md`
**Depends on:** Task 2.1 (BeforePayCost) and Task 2.2 ([Main] field effects)
**Cards to fix:**
| Card | Issue Summary |
|------|---------------|
| BT23-076 Sistermon Blanc | On Play completely wrong (rewrite needed) |
| BT20-013 BaoHuckmon | [Main] unreachable (blocked by Task 2.2) |
| BT23-057 Gankoomon | Process callback never fires (blocked by Task 2.1) |

Plus 8 more from the 03-13 report — agent should read `2026-03-13-jesmon.md` for full list.

- [ ] Fix each card
- [ ] Smoke test: 20 games Jesmon mirror
- [ ] Debug test each fixed card
- [ ] Commit

### Task 3.3: Royal Knights (5 cards + 2 engine-gap notes)

**QA Reports:** `2026-03-03-royal-knights-script-audit.md`, `2026-03-03-royal-knights-gameplay.md`, `2026-03-15-ts-jupitermon-royal-knights-qa.md`
**Depends on:** Task 2.1 (BeforePayCost)
**Cards to fix:**
| Card | Issue Summary |
|------|---------------|
| BT13-086 Gizmon: XT | Free cost reduction (3 bugs) |
| BT13-100 Yoshino Fujieda | Wrong trigger + wrong target |
| BT13-036 Liollmon | Auto-selects target, missing condition |
| BT23-057 Gankoomon | Process callback (shared with Jesmon) |
| BT20-056 Alphamon | DP displays 3000 instead of 11000 (#27) — investigate DB vs script |

**Engine-gap notes (DEFERRED to Task 2.4):**
- CANNOT_DIGIVOLVE not enforced in mask (#43)
- CANNOT_ADD_SECURITY not enforced (#45)

- [ ] Fix each card
- [ ] Investigate Alphamon DP discrepancy
- [ ] Smoke test + debug test
- [ ] Commit

### Task 3.4: Dark Masters (18 from 03-13)

**QA Reports:** `2026-03-13-dark-masters.md`, `2026-03-15-jesmon-dark-masters-qa.md`
**Depends on:** Task 1.1 (crash fixes for BT15-069, BT14-044), Task 1.3 (P-216 fix)
**Note:** BT15-069, BT14-044, and P-216 are already fixed in Phase 1. This task covers the remaining 18 cards from the 03-13 report only.

- [ ] Fix 18 remaining cards from 03-13 report
- [ ] Smoke test + debug test
- [ ] Commit

### Task 3.5: Medusamon (petrification tokens + 7 from 03-09)

**QA Reports:** `2026-03-09-medusa-regression.md`, `2026-03-15-dna-omnimon-medusamon-qa.md`
**Cards to fix:**
| Card | Issue Summary |
|------|---------------|
| BT21-029 Medusamon | Petrification tokens never generated (2 sub-bugs) |
| BT24-017 Medusamon | DP scaling timing wrong, missing trash cost, missing Piercing |
| BT24-082 Owen Dreadnought | 4 bugs (digivolve trigger, targets, filter, self-return) |

Plus issues 52-58 from regression report.

- [ ] Fix each card
- [ ] Smoke test + debug test
- [ ] Commit

### Task 3.6: TS Jupitermon (5 from 03-13 + 4 from 03-15)

**QA Reports:** `2026-03-13-ts-jupitermon.md`, `2026-03-15-ts-jupitermon-royal-knights-qa.md`
**Cards to fix:**
| Card | Issue Summary |
|------|---------------|
| BT24-046 Garurumon | Suspend filter uses nonexistent `owner` attr |
| BT24-084 Inori Misono | Missing memory <= 4 condition |
| BT24-037 Silphymon | force_attack stub, SA+1 stub |
| P-213 Aegiochusmon | force_attack stub |
| BT24-098 Invasion of Titans | 4 bugs (trash count, trait filters) |

- [ ] Fix each card
- [ ] Smoke test + debug test
- [ ] Commit

### Task 3.7: TS Neptunemon (9 from 03-03)

**QA Reports:** `2026-03-03-ts-neptune-gameplay.md`
**Cards to fix:** Issues 33-41 from report (BT24-031, BT24-029, BT24-102, BT24-090, BT24-088, BT3-093, BT24-027/028/029, BT24-028).

- [ ] Fix each card
- [ ] Smoke test + debug test
- [ ] Commit

### Task 3.8: Millenniummon (BT18-100 + BT18-073)

**QA Reports:** `2026-03-15-ts-neptunemon-millenniummon-qa.md`
**Depends on:** Task 1.1 (BT18-100 crash fix), Task 2.1 (BeforePayCost for BT18-073)

- [ ] Fix BT18-073 deletion cost enforcement
- [ ] Fix any remaining 03-03 issues
- [ ] Smoke test + debug test
- [ ] Commit

### Task 3.9: DNA Omnimon (13 from 03-13)

**QA Reports:** `2026-03-13-dna-omnimon.md`
**Cards:** 13 outstanding issues — agent reads report for full list.

- [ ] Fix each card
- [ ] Smoke test + debug test
- [ ] Commit

### Task 3.10: ExMaquinamon + Shared EX11 Scripts (14 from 03-13)

**QA Reports:** `2026-03-13-exmaquinamon.md`
**Cards:** 14 outstanding issues. Note: 5 share the `effect_source_permanent` condition bug.
**MUST RUN BEFORE Task 3.11** — owns all shared EX11-series scripts.

**Shared EX11 scripts (owned by this task, also in Galacticmon report):**
EX11-006, EX11-027, EX11-029, EX11-033, EX11-036, EX11-045, EX11-062, EX11-070, EX11-073

- [ ] Fix systemic `effect_source_permanent` pattern across 5 scripts
- [ ] Fix remaining 9 cards (including all shared EX11 scripts)
- [ ] Smoke test + debug test
- [ ] Commit

### Task 3.11: Galacticmon (non-EX11 cards, ~18 remaining from 03-13)

**QA Reports:** `2026-03-13-galacticmon.md`
**Depends on:** Task 1.1 (BT18-100), **Task 3.10** (shared EX11 scripts)
**Cards:** ~18 outstanding issues after excluding the 9 EX11 scripts fixed in Task 3.10. Agent must read the Galacticmon report, cross-reference with Task 3.10 fixes, and fix only the non-overlapping cards.

- [ ] Fix non-EX11 Galacticmon cards
- [ ] Verify EX11 scripts from Task 3.10 work in Galacticmon context
- [ ] Smoke test + debug test
- [ ] Commit

### Task 3.12a: Puppets Batch 1 — Megas + Options (16 cards from 03-13)

**QA Reports:** `2026-03-13-puppets.md`
**Cards:** First 16 issues from the report (highest-level cards: megas, options, tamers). Agent reads full report and takes the first half.

- [ ] Fix 16 cards
- [ ] Smoke test: 10 games Puppets mirror
- [ ] Commit

### Task 3.12b: Puppets Batch 2 — Rookies + Champions + Ultimates (16 cards from 03-13)

**QA Reports:** `2026-03-13-puppets.md`
**Depends on:** Task 3.12a (to avoid merge conflicts on shared files)
**Cards:** Remaining 16 issues. Agent reads full report and takes the second half.

- [ ] Fix 16 cards
- [ ] Smoke test: 10 games Puppets mirror
- [ ] Debug test key cards
- [ ] Commit

### Task 3.13: Hudiemon (BT23-097 + remaining)

**QA Reports:** `2026-03-15-hudiemon-zephagamon-qa.md`
**Cards:** BT23-097 Seventh Penetration (missing filter) + engine-gap stubs.

- [ ] Fix BT23-097
- [ ] Audit remaining stubs against engine capabilities
- [ ] Smoke test + debug test
- [ ] Commit

### Task 3.14: Zephagamon (BT24-056 dependency)

**Depends on:** Task 1.1 (CANNOT_BE_RETURNED fix)
**QA Reports:** `2026-03-15-hudiemon-zephagamon-qa.md`

- [ ] After CANNOT_BE_RETURNED fix, re-run 20 games
- [ ] Fix any newly exposed bugs
- [ ] Commit

### Task 3.15: Diaboromon (2 outstanding)

**QA Reports:** `2026-03-01-diaboromon.md` (issues #7, #8)
**Cards:**
| Card | Issue Summary |
|------|---------------|
| BT22-091 | SwitchDefender attack redirect — engine gap? Check if Task 2.2 or redirect_attack resolves |
| Overclock | `_is_overclock` flag present but no EOT attack triggers |

**Note:** Both may be engine limitations (SwitchDefender, Overclock EOT). If so, mark BLOCKED in archetype-qa doc with justification.

- [ ] Investigate both issues against current engine capabilities
- [ ] Fix if possible, otherwise document as BLOCKED
- [ ] Commit

### Task 3.16: TS Olympos (2 outstanding from 03-11)

**QA Reports:** `2026-03-11-ts-olympos-vs-imperialdramon.md` (issues #64, #65)
**Cards:**
| Card | Issue Summary |
|------|---------------|
| BT24-041 Minervamon | De-Digivolve uses attack action IDs instead of target selection (#65) |
| SelectReveal | Action descriptions show wrong text (#64) — cosmetic, LOW |

- [ ] Fix BT24-041 de-digivolve target selection
- [ ] Note #64 as cosmetic WONTFIX or fix if straightforward
- [ ] Commit

### Task 3.17: Remaining Stable Archetypes (BG Imperial, Rocks)

These archetypes showed 0 new bugs in the 03-15 campaign and no outstanding pre-03-13 issues. Verify current verdicts are still accurate.

- [ ] BG Imperial: re-run 10 game smoke test, confirm stable
- [ ] Rocks: re-run 10 game smoke test, confirm stable

---

## Phase 4: Update Archetype QA Docs

### Task 4.1: Backport All Findings to archetype-qa Docs

For each of the 17 archetypes, update `qa/archetype-qa/{name}.md` with:
1. Current date
2. Revised card-by-card verdicts reflecting all fixes from Phases 1-3
3. Any remaining BLOCKED/PARTIAL items with engine-gap references
4. Smoke test results

**Format per doc:**
```markdown
# Archetype QA: {Name}
Date: 2026-03-XX (updated)
Total cards: N

## Summary
- PASS: X
- FIXED: Y (this campaign)
- BLOCKED: Z (engine gaps)
- PARTIAL: W

## Card-by-Card Verdicts
| Card ID | Name | Verdict | Notes |
...

## Outstanding Issues
...

## Smoke Test
- N/N games completed, 0 crashes
```

- [ ] Update all 17 archetype-qa docs
- [ ] Update `qa/archetype-qa/INDEX.md` with revised priority/status
- [ ] Update `qa/archetype-qa/engine-gaps.md` if any new gaps discovered
- [ ] Commit

### Task 4.2: Update qa-reports INDEX

- [ ] Mark all fixed issues in `qa/qa-reports/INDEX.md` as FIXED
- [ ] Update totals
- [ ] Commit

---

## Phase 5: Re-QA Cross-Archetype Campaign

### Task 5.1: Run Full 8-Matchup Campaign

Re-run the same 8 matchup pairs from the 03-15 campaign:

| # | Matchup |
|---|---------|
| 1 | TS Neptunemon vs Millenniummon |
| 2 | Hudiemon vs Zephagamon |
| 3 | Puppets vs TS Olympos |
| 4 | BG Imperial vs Galacticmon |
| 5 | Jesmon vs Dark Masters |
| 6 | Rocks vs Chaos Control |
| 7 | DNA Omnimon vs Medusamon |
| 8 | TS Jupitermon vs Royal Knights |

Each matchup: 20 automated games (10 random, 10 greedy) + 4-6 targeted debug games.

- [ ] Run all 8 matchups (can be parallel agents)
- [ ] Compile results into `qa/qa-reports/2026-03-XX-faithfulness-retest.md`
- [ ] Fix any newly discovered bugs
- [ ] Final update to archetype-qa docs

---

## Dependency Graph

```
Phase 1 (crashes + uppercase + P-216)
  ├── Task 1.1 (crash fixes) ──────────────┐
  ├── Task 1.2 (uppercase files)            │
  └── Task 1.3 (P-216)                     │
                                             v
Phase 2 (engine fixes)               Phase 3 unblocked
  ├── Task 2.1 (BeforePayCost) ─────> Tasks 3.2, 3.3, 3.8
  ├── Task 2.2 (selection deadlock) ─> cross-archetype stability
  └── Task 2.3 ([Main] mask) ───────> Task 3.2
                                             │
Phase 3 (per-archetype) ────────────────────┘
  Parallel group A (no deps beyond Phase 1):
    3.1 (Chaos), 3.5 (Medusa), 3.6 (TS Jupiter), 3.7 (TS Neptune),
    3.9 (DNA Omnimon), 3.13 (Hudiemon), 3.14 (Zephaga),
    3.15 (Diaboromon), 3.16 (TS Olympos), 3.17 (BG/Rocks verify)

  Sequential chain (Phase 2 deps):
    3.2 (Jesmon) ── needs 2.1 + 2.3
    3.3 (RK) ───── needs 2.1
    3.4 (DM) ───── needs 1.1 + 1.3
    3.8 (Mill) ─── needs 1.1 + 2.1

  Sequential chain (script overlap):
    3.10 (ExMaquinamon) ──> 3.11 (Galacticmon)  [shared EX11 scripts]
    3.12a (Puppets B1) ──> 3.12b (Puppets B2)
                                             │
Phase 4 (doc updates) <─────────────────────┘
  Tasks 4.1-4.2
                                             │
Phase 5 (re-QA) <───────────────────────────┘
  Task 5.1
```

## Execution Notes

- **Phase 1** tasks are all independent — run in parallel.
- **Phase 2** tasks are independent of each other but must complete before their Phase 3 dependents.
- **Phase 3 parallel group A** (10 tasks) can start immediately after Phase 1 — no Phase 2 dependencies.
- **Phase 3 sequential chains**: Tasks 3.2/3.3/3.8 wait for Phase 2. Tasks 3.10→3.11 and 3.12a→3.12b must run sequentially to avoid merge conflicts on shared files.
- **For each Phase 3 task**, use `/implement-archetype` or `/review-archetype` skills with Pinecone context packs for C# reference lookups.
- **Each Phase 3 agent must output a structured verdict table** for Phase 4 to consume.
- **Phase 4** is a documentation sweep after all fixes land.
- **Phase 5** is the final validation gate.

## Estimated Scale

- **Total outstanding bugs:** ~200 across all reports (191 from INDEX + ~10 pre-03-13 not tracked)
- **Crash bugs:** 5 (quick fixes)
- **Engine changes:** 3 (BeforePayCost, selection deadlock, [Main] mask)
- **Engine gaps deferred:** 2 (CANNOT_DIGIVOLVE, CANNOT_ADD_SECURITY)
- **Script rewrites:** ~30-40 cards need significant rewrites
- **Script fixes:** ~100+ cards need minor fixes
- **Verification games:** ~200+ automated + ~50 debug games
- **Parallel capacity:** Up to 10 agents simultaneously (Phase 3 group A)
