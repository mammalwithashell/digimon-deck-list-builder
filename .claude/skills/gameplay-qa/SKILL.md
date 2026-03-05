---
name: gameplay-qa
description: QA test the Digimon TCG simulator by playing games with a critical eye for correct play costs, game flow, card effects, and keywords. Finds fully-implemented decks, creates deterministic test games, plays through them via API + Playwright, validates correctness against official rules, and files issues for bugs found.
argument-hint: [ARCHETYPE_NAME] [--focus play_costs|digivolution|keywords|effects|all]
---

# Gameplay QA Testing

You are a QA tester for a Digimon TCG game simulator. Your job is to play games,
critically evaluate the engine's behavior against official rules, and file issues
for any bugs you find.

## Prerequisites

- Backend server running at http://localhost:8000 with `DEBUG_MODE=1`
- Frontend running at http://localhost:5173

## Quick Reference

- **RULES_CONTEXT.md** — Official Digimon TCG rules reference (read this first!)
- **Official Rules PDFs** (if RULES_CONTEXT.md is insufficient):
  - https://world.digimoncard.com/rule/pdf/general_rule.pdf
  - https://world.digimoncard.com/rule/pdf/manual.pdf
- **Card API**: https://digimoncard.io/index.php/api-public/search?card=CARD_ID
- **QA Index**: `docs/qa-reports/INDEX.md` — tracks all QA issues and their resolution status
- **QA Reports**: `docs/qa-reports/YYYY-MM-DD-{archetype}.md`
- **Validated Cards**: `docs/qa-reports/validated_cards.json` — cards confirmed working through QA

---

## Phase 0: Review Prior QA Work

Before starting a new QA session, check what has already been tested.

1. Read `docs/qa-reports/INDEX.md` to see:
   - Which archetypes have been tested
   - Outstanding issues that may still affect gameplay
   - Won't-fix issues to be aware of during testing
2. Read `docs/qa-reports/validated_cards.json` to see which individual cards are already validated:
   - `PASS` cards can be skipped unless doing regression testing
   - `PARTIAL` cards should be prioritized for deeper testing
   - `FAIL` cards need retesting after fixes
3. Skim any relevant existing reports in `docs/qa-reports/` for the archetype you plan to test
4. If re-testing a previously tested archetype (regression test), note which prior issues were fixed and verify them

---

## Phase 1: Find Test Decks and Identify Cards to Validate

### 1a. Pick an archetype

If $ARGUMENTS contains an archetype name, use that. Otherwise, find eligible archetypes:

```bash
python -m digimon_gym.engine.data.deck_finder --min-coverage 1.0 --max-results 20
```

If no fully-playable decks are found, try `--min-coverage 0.95` and note the missing cards.
Prefer archetypes with higher `meta_share` (more competitive).

### 1b. Collect ALL unique cards for the archetype

Look up the archetype in `deck_library.json` and gather every unique card across all its decklists:

```bash
python -c "
import json
from pathlib import Path

lib = json.loads(Path('digimon_gym/engine/data/deck_library.json').read_text())
validated = json.loads(Path('docs/qa-reports/validated_cards.json').read_text()) if Path('docs/qa-reports/validated_cards.json').exists() else {'cards': {}}

archetype = lib['archetypes'].get('ARCHETYPE_NAME', {})
all_cards = set()
for dl in archetype.get('decklists', []):
    all_cards.update(json.loads(dl['decklist']))

validated_ids = {k for k, v in validated.get('cards', {}).items() if v['status'] == 'PASS'}
partial_ids = {k for k, v in validated.get('cards', {}).items() if v['status'] == 'PARTIAL'}
unvalidated = sorted(all_cards - validated_ids - partial_ids)
print(f'Total unique cards: {len(all_cards)}')
print(f'Already validated (PASS): {len(all_cards & validated_ids)}')
print(f'Partially validated: {len(all_cards & partial_ids)}')
print(f'Need testing: {len(unvalidated)}')
for c in unvalidated:
    print(f'  {c}')
"
```

### 1c. Plan test games to cover all unvalidated cards

- Arrange deck order so unvalidated cards appear in the opening hand (first 5 non-egg cards)
- Create multiple games if needed to cover all unvalidated cards
- Prioritize PARTIAL cards for deeper testing, then untested cards
- Use the archetype's actual decklist as the base (pick the highest-placement list)

---

## Phase 2: Study Rules & Cards

Before playing, understand what each card should do:

1. Read `RULES_CONTEXT.md` to understand core game rules
2. For each key card in the selected deck, look up its data:
   ```bash
   python -c "
   from digimon_gym.engine.data.card_database import CardDatabase
   db = CardDatabase()
   card = db.get_card('CARD_ID')
   if card:
       print(f'Name: {card.card_name_eng}')
       print(f'Kind: {card.card_kind.name}')
       print(f'Level: {card.level}')
       print(f'Play Cost: {card.play_cost}')
       print(f'DP: {card.dp}')
       print(f'Colors: {[c.name for c in card.card_colors]}')
       print(f'Type: {card.type_eng}')
       print(f'Effect: {card.effect_description_eng}')
       print(f'Inherited: {card.inherited_effect_description_eng}')
       print(f'Security: {card.security_effect_description_eng}')
       print(f'Evo Costs: {[(e.card_color.name, e.level, e.memory_cost) for e in card.evo_costs]}')
   "
   ```

3. Note expected behaviors for key cards:
   - Play cost and what memory change to expect
   - Effect triggers (On Play, When Digivolving, When Attacking, etc.)
   - Keywords and their mechanics
   - Digivolution requirements and costs

4. **Optional/Cost Effects Rule** — Critically check each card's text for these patterns:
   - **"may"** → The effect MUST be optional (`is_optional=True`), triggering a player/agent selection to use or decline it.
   - **"by [doing X]"** → The stated action is a **cost** the player must pay to activate the effect. The cost targets the card's OWN permanent (e.g., "by suspending this Tamer" means `perm.suspend()` on this card, NOT selecting an opponent's permanent). The effect should be `is_optional=True` (player chooses whether to pay), and the cost must execute before the reward.
   - **"or"** → When card text says "do X, or you may do Y", this is a **mutually exclusive choice** — the player picks one branch, not both.
   - Scripts that apply "by" costs to opponent targets, skip "may" optionality, or execute both branches of an "or" choice are **bugs**.

---

## Phase 3: Create Test Game

Use the debug API to create a deterministic test game.

### 3a. Arrange the deck for targeted testing

Organize card IDs so cards you want to test appear in specific positions.
The first 5 non-egg cards will be drawn as the opening hand (if skip_shuffle=true).

### 3b. Create the game

```bash
curl -s -X POST http://localhost:8000/debug/games \
  -H "Content-Type: application/json" \
  -d '{
    "deck1": [<CARD_IDS>],
    "deck2": [<CARD_IDS>],
    "player1_type": "human",
    "player2_type": "human",
    "first_player": 1,
    "skip_shuffle": true,
    "auto_mulligan": "keep",
    "initial_memory": 0,
    "agent_action_delay_ms": 0
  }'
```

Save the `game_id` from the response.

### 3c. Verify initial state

```bash
curl -s http://localhost:8000/debug/games/GAME_ID/internal-state | python -m json.tool
```

Confirm:
- Hand cards match expected (first 5 non-egg cards from deck1)
- Library order is preserved
- Memory is at expected value
- Phase is correct (Main after auto-mulligan, or Mulligan if manual)

---

## Phase 4: Play Game (API-Driven)

Drive gameplay through the API. For each turn:

### 4a. Check available actions

```bash
curl -s http://localhost:8000/games/GAME_ID/actions | python -m json.tool
```

This returns human-readable action descriptions for every legal action.

### 4b. Execute an action

```bash
curl -s -X POST http://localhost:8000/games/GAME_ID/actions \
  -H "Content-Type: application/json" \
  -d '{"action": ACTION_ID}'
```

The response includes:
- `action_context.memory_before` and `action_context.memory_after` — verify cost deduction
- `state` — full game state after action
- `logs` — engine log messages describing what happened
- `action_descriptions` — next available actions

### 4c. Validate after each action

**Optional/Cost Effect Validation** (check these for EVERY effect that triggers):
- If card text says **"may"**: Verify a selection/choice is presented to the player. If the effect fires automatically without choice, it's a bug.
- If card text says **"by [doing X]"**: Verify the cost action targets the correct entity (e.g., "by suspending this Tamer" must suspend the card's own permanent, not an opponent's). Verify the cost is paid BEFORE the reward.
- If card text says **"X, or you may Y"**: Verify only one branch executes, not both.

**For PLAY actions (0-29)**:
- Memory should decrease by the card's `play_cost`
- Card should appear in battle area (Digimon/Tamer) or trash (Option after resolving)
- On Play effects should trigger (check logs)

**For DIGIVOLVE actions (400-999)**:
- Memory should decrease by the evo cost (NOT play cost)
- Player should draw 1 card (digivolution bonus)
- When Digivolving effects should trigger
- Inherited effects from sources should become available

**For ATTACK actions (100-399)**:
- No memory cost for attacking
- Opponent should get block timing if they have Blocker digimon
- Security checks should proceed correctly
- When Attacking effects should trigger

**For HATCH (60)**:
- Top egg from digitama deck goes to breeding area
- No memory cost

**For MOVE (61)**:
- Breeding area digimon moves to battle area
- No memory cost

**For PASS (62)**:
- Turn passes, memory moves to opponent's side

### 4d. Inspect internal state for verification

```bash
curl -s http://localhost:8000/debug/games/GAME_ID/internal-state | python -m json.tool
```

Use this to verify deck order, security stack, and hidden information.

### 4e. Track card coverage

After each game, track which cards were played/tested. Continue creating new games with different deck arrangements until all unvalidated cards from Phase 1b have been covered. Use `inject-card` to add specific cards to hand if they aren't appearing naturally.

### 4f. Manipulate game state for targeted testing

Set memory to test edge cases:
```bash
curl -s -X POST http://localhost:8000/debug/games/GAME_ID/set-memory \
  -H "Content-Type: application/json" \
  -d '{"memory": 5}'
```

Inject a card to test specific scenarios:
```bash
curl -s -X POST http://localhost:8000/debug/games/GAME_ID/inject-card \
  -H "Content-Type: application/json" \
  -d '{"player_id": 1, "card_id": "BT24-015", "zone": "hand"}'
```

---

## Phase 5: Visual Verification (Playwright)

After key game actions, verify the web UI renders correctly.

1. Navigate to `http://localhost:5173/game/GAME_ID`
2. Take a browser snapshot to read the accessibility tree
3. Verify:
   - Memory gauge shows correct value
   - Phase indicator matches expected phase
   - Cards appear in correct zones
   - Prompt text is appropriate for the current phase
   - Action buttons appear when expected

**Key UI elements to check:**
- Memory gauge / counter display
- Phase banner text
- Prompt bar text
- Action buttons (Keep/Mulligan/Hatch/Move/Pass)
- Card positions in hand, field, breeding area
- Selection panel during selection phases

---

## Phase 6: Produce QA Report

After testing, create a structured QA report.

Save to: `docs/qa-reports/YYYY-MM-DD-{archetype}.md`

### Report Template:

```markdown
# Gameplay QA Report — {Archetype Name}

## Test Setup
- **Date**: YYYY-MM-DD
- **Archetype**: {name}
- **Deck ID**: {deck_id}
- **Game ID(s)**: {game_ids}
- **Total Turns Played**: N
- **Focus Areas**: play costs, digivolution, keywords, effects

## Summary
- **Total Issues Found**: N
- Critical: N | High: N | Medium: N | Low: N

## Detailed Findings

### Issue 1: {brief title}
- **Card(s)**: {card_id(s)} — {card_name(s)}
- **Severity**: critical|high|medium|low
- **Category**: play_cost | digivolution | keyword | effect | ui | memory | game_flow
- **Expected**: {what should happen per official rules}
- **Actual**: {what actually happened in the engine}
- **Steps to Reproduce**:
  1. Create game with deck X
  2. Play card Y (action ID Z)
  3. Observe memory change / board state / etc.
- **Evidence**: memory_before=X, memory_after=Y, expected_cost=Z
- **Rules Reference**: {which rule this violates}

## Cards Tested Successfully
- {card_id}: {card_name} — play cost correct, effects fired correctly

## Areas Not Covered
- {mechanic or card not tested in this session}
```

### Update the QA Index

After saving the report, update `docs/qa-reports/INDEX.md`:

1. Add a row to the **Summary** table:
   ```markdown
   | [archetype](YYYY-MM-DD-archetype.md) | N | N | N | N |
   ```

2. Add a new **Report section** at the bottom with the issue table:
   ```markdown
   ## Report N: {Archetype} (YYYY-MM-DD)

   N issues found across M cards. {summary of resolution status}.

   | # | Issue | Sev | Status | Fix |
   |---|-------|-----|--------|-----|
   | 1 | {brief description} | crit/high/med/low | FIXED/WONTFIX/OUTSTANDING | {fix description} |
   ```

3. Update the **Last updated** date at the top

4. Update the **Total** row in the summary table

**Status values**: `FIXED`, `WONTFIX`, `OUTSTANDING`

### Update the Validated Cards Index

After saving the report, update `docs/qa-reports/validated_cards.json`:

1. For each card in the "Cards Tested" table:
   - **PASS** cards: add/update entry with `"status": "PASS"`
   - **PARTIAL** cards: add/update entry with `"status": "PARTIAL"` and notes explaining gaps
   - **FAIL** cards: add/update entry with `"status": "FAIL"`
2. Set `validated_date` to today and `report` to the report filename
3. Increment `version` and update `last_updated`

Cards that were previously FAIL but are now fixed should be updated to PASS after verifying the fix.

### Completion Criteria

Testing for an archetype is complete when **ALL unique cards** in the archetype's decklists (from Phase 1b) have an entry in `validated_cards.json` with status `PASS` or `PARTIAL`. Cards that cannot be tested in gameplay (e.g., situational cards) should be marked `PARTIAL` with `"notes": "static analysis only"`.

---

## Phase 7: File Issues

For each finding, check for duplicates first, then file:

### Check for existing issues
```bash
curl -s "http://localhost:8000/issues?card_id=CARD_ID&status=new" \
  -H "Authorization: Bearer $TOKEN"
```

### File new issue
```bash
curl -s -X POST http://localhost:8000/issues \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "card_id": "CARD_ID",
    "description": "EXPECTED vs ACTUAL behavior with rules reference and reproduction steps",
    "source": "system",
    "severity": "high"
  }'
```

### Update index when issues are resolved

When bugs from a QA report are fixed (in the same session or later), update `docs/qa-reports/INDEX.md`:
- Change the issue's status from `OUTSTANDING` to `FIXED`
- Add a brief fix description in the Fix column
- Update the summary table counts (Fixed/Outstanding columns)
- When all issues are resolved, add "All outstanding issues resolved." below the summary table

### Severity Guidelines
- **critical**: Game crashes, infinite loops, wrong winner declared, game state corruption
- **high**: Incorrect play costs, wrong memory changes, effects not triggering at all, security check errors
- **medium**: Keywords partially wrong, minor timing issues, effect activating with wrong parameters
- **low**: Cosmetic issues, edge cases unlikely in normal play, minor UI mismatches

### Issue Description Format
Always include:
1. **Expected behavior** (with rules reference if possible)
2. **Actual behavior** observed
3. **Game state context** (turn, memory, phase, board state)
4. **Action that triggered** the issue (action ID and description)
5. **Reproduction steps** (deck arrangement, game setup)

---

## Testing Strategies

### Play Cost Testing
1. Set memory to exactly the card's play cost
2. Verify the card CAN be played (appears in action mask)
3. Play the card
4. Verify memory changed by exactly the play cost amount
5. Set memory to 1 less than needed (memory + 10 < cost)
6. Verify the card CANNOT be played (absent from action mask)

### Digivolution Testing
1. Place a valid base on the field (play a Rookie)
2. Check that valid digivolve actions appear for matching Champions in hand
3. Digivolve and verify:
   - Evo cost deducted (not play cost)
   - Drew 1 bonus card
   - When Digivolving effect triggered (check logs)
   - Inherited effects from source accessible

### Keyword Testing
- **Blocker**: Attack opponent, verify BlockTiming phase appears if opponent has Blocker
- **Rush**: Play a digimon with Rush, verify it can attack same turn
- **Piercing**: Attack security with Piercing digimon, verify excess damage effect
- **Security Attack +N**: Check security checks count matches expected
- **Reboot**: Verify unsuspend during opponent's unsuspend phase

### Effect Timing Testing
- **[On Play]**: Play card, verify effect fires immediately
- **[When Digivolving]**: Digivolve, verify effect fires
- **[When Attacking]**: Attack, verify effect fires
- **[On Deletion]**: Get a digimon deleted, verify effect fires
- **[Start of Your Turn]**: Pass to opponent and back, verify start-of-turn effects
