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

---

## Phase 1: Find Test Decks

Find competitive decklists where ALL cards have frozen script implementations.

```bash
python -m digimon_gym.engine.data.deck_finder --min-coverage 1.0 --max-results 20
```

If $ARGUMENTS contains an archetype name, look for that archetype specifically.
If no fully-playable decks are found, try `--min-coverage 0.95` and note the missing cards.

Pick 1-2 archetypes to test. Prefer decks with higher `meta_share` (more competitive).

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

### 4e. Manipulate game state for targeted testing

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
