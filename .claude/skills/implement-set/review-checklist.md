# Card Effect Review Checklist

Detailed procedures for reviewing transpiled card scripts against official Digimon TCG rules.

## Per-Card Review Process

For each card being reviewed:

### Step 1: Fetch Official Effect Text

```bash
# Fetch card data from digimoncard.io API
curl -s "https://digimoncard.io/index.php/api-public/search?card={CARD_ID}" | python -m json.tool
```

Or use WebFetch:
```
https://digimoncard.io/index.php/api-public/search?card={CARD_ID}
```

Note the `solesource_effect` and `inherited_effect` fields.

### Step 2: Read Transpiled Script

```bash
cat digimon_gym/engine/data/scripts/{set_lower}/{card_id_lower}.py
```

### Step 3: Compare Effects

For each effect in the card text, verify:

- [ ] **Effect exists in script** — every `[Timing]` block in card text has a corresponding effect in the script
- [ ] **Timing is correct** — the effect's timing property matches the card text timing indicator
- [ ] **Keywords are correct** — all keyword flags (`_is_blocker`, `_is_rush`, etc.) match the card text
- [ ] **Actions are correct** — the process callback performs the right game actions
- [ ] **Conditions are correct** — turn restrictions, trait checks, etc. match the card text
- [ ] **Inherited flag is correct** — inherited effects from card text are flagged `is_inherited_effect = True`
- [ ] **Optional flag is correct** — optional effects (`you may`) have `is_optional = True`
- [ ] **Once-per-turn is correct** — `[Once Per Turn]` effects have `set_max_count_per_turn(1)`

---

## Keyword Mechanics Verification

When a keyword appears on a card, verify it matches the official rules. Reference: `RULES_CONTEXT.md` Section 12-14.

### Combat Keywords

| Keyword | Rule Section | Key Requirement | Script Flag |
|---------|-------------|-----------------|-------------|
| Blocker | 16-2 | Activation-type; unsuspend → suspend to block | `_is_blocker` |
| Piercing | 16-8 | After winning battle vs Digimon → check security | `_is_piercing` |
| Jamming | 16-6 | Persistent; survives security battle loss | `_is_jamming` |
| Retaliation | 16-10 | When deleted in battle → delete winner too | `_is_retaliation` |
| Security Attack +1/-1 | 16-12 | Modifies security check count | `_security_attack_modifier` |

### Movement Keywords

| Keyword | Rule Section | Key Requirement | Script Flag |
|---------|-------------|-----------------|-------------|
| Rush | 16-5 | Bypasses summoning sickness; Digimon-only attack on turn played | `_is_rush` |
| Blitz | 16-14 | When digivolved, can attack even if opponent has memory | `_is_blitz` |
| Raid | 16-13 | Attack highest-DP unsuspended opponent Digimon | `_is_raid` |

### Defensive Keywords

| Keyword | Rule Section | Key Requirement | Script Flag |
|---------|-------------|-----------------|-------------|
| Armor Purge | 16-15 | Trash top digi source to survive deletion | `_is_armor_purge` |
| Evade | 16-16 | Suspend to survive deletion (must be unsuspended) | `_is_evade` |
| Barrier | 16-17 | Trash top security to survive battle deletion | `_is_barrier` |
| Fortitude | 16-18 | When deleted, if had digi cards, replay top card | `_is_fortitude` |
| Save | 16-19 | After deletion, place under a Tamer | `_is_save` |
| Decoy | 16-23 | Delete this Digimon instead of target (opponent effects only) | `_is_decoy` |
| Material Save | 16-26 | Before deletion, place 1 digi source under Tamer | `_is_material_save` |

### Phase Keywords

| Keyword | Rule Section | Key Requirement | Script Flag |
|---------|-------------|-----------------|-------------|
| Reboot | 16-4 | Unsuspends during opponent's unsuspend phase | `_is_reboot` |
| Training | 16-40 | Suspend to place deck top at bottom of digi cards | `_is_training` |
| Progress | 16-36 | Immune to opponent effects while attacking | `_is_progress` |

### Digivolution Keywords

| Keyword | Rule Section | Key Requirement | Script Flag |
|---------|-------------|-----------------|-------------|
| Blast Digivolve | 16-31 | Counter-timing digivolution | `is_counter_effect` |
| De-Digivolve | 16-22 | Trash X from top of digi cards | `degen_count` |

### Restriction Keywords

| Keyword | Script Flag | Effect |
|---------|-------------|--------|
| Cannot Attack | `_is_cannot_attack` | `can_attack()` returns False |
| Cannot Attack Player | `_is_cannot_attack_player` | `can_attack_player()` returns False |
| Cannot Block | `_is_cannot_block` | `can_block()` returns False |
| Cannot Be Blocked | `_is_cannot_be_blocked` | Blockers can't block this |
| Cannot Unsuspend | `_is_cannot_unsuspend` | Skipped in `unsuspend_all()` |
| Cannot Return To Hand | `_is_cannot_return_to_hand` | `bounce_permanent_to_hand()` blocked |
| Cannot Return To Deck | `_is_cannot_return_to_deck` | `return_permanent_to_deck_bottom()` blocked |

---

## Timing Verification Table

Map from card text timing indicators to expected script properties:

| Card Text | Expected Property | Condition Check |
|-----------|-------------------|-----------------|
| `[On Play]` | `is_on_play = True` | permanent exists |
| `[When Digivolving]` | `is_when_digivolving = True` | permanent exists |
| `[When Attacking]` | timing at attack declaration | permanent exists |
| `[On Deletion]` | `is_on_deletion = True` | (no permanent check — it was just deleted) |
| `[Start of Your Main Phase]` | `OnStartMainPhase` | `card.owner.is_my_turn` |
| `[Start of Your Turn]` | `OnStartTurn` | `card.owner.is_my_turn` |
| `[End of Your Turn]` | `OnEndTurn` | `card.owner.is_my_turn` |
| `[Your Turn]` | (passive) | `card.owner.is_my_turn` |
| `[Opponent's Turn]` | (passive) | `not card.owner.is_my_turn` |
| `[All Turns]` | (passive) | no turn restriction |
| `[Once Per Turn]` | `set_max_count_per_turn(1)` | hash string set |
| `[Security]` | `is_security_effect = True` | security timing |
| `[Counter]` | `is_counter_effect = True` | counter timing |
| `[When Linking]` | `WhenLinked` timing | permanent exists |
| `[Main]` (Option) | `OptionSkill` timing | option main effect |

---

## Action Verification Table

Map from card text actions to expected callback code:

| Card Text Action | Expected Code |
|-----------------|---------------|
| "draw X card(s)" | `player.draw_cards(X)` |
| "gain X memory" | `player.add_memory(X)` |
| "+X000 DP" / "-X000 DP" | `perm.change_dp(X000)` / `perm.change_dp(-X000)` |
| "delete 1 of your opponent's Digimon" | `game.effect_select_opponent_permanent(player, delete_cb, ...)` |
| "return ... to hand" | `game.effect_select_opponent_permanent(player, bounce_cb, ...)` |
| "suspend" | `target_perm.suspend()` |
| "unsuspend" | `target_perm.unsuspend()` |
| "<Recovery +1 (Deck)>" | `player.recovery(1)` |
| "trash X cards from top of deck" | `player.mill(X)` |
| "gains <Keyword>" | `target_perm.grant_keyword('_is_keyword')` |
| "play ... without paying the cost" | play from zone with cost 0 |
| "this Digimon can't be returned to hands or decks" | `grant_keyword('_is_cannot_return_to_hand')` + `grant_keyword('_is_cannot_return_to_deck')` |

---

## Systemic Issue Detection

When you find an issue in one card, check if it's systemic:

```bash
# Search all transpiled scripts for the same pattern
grep -rn "PATTERN" digimon_gym/engine/data/scripts/{set_lower}/

# Check if other sets have the same issue
grep -rn "PATTERN" digimon_gym/engine/data/scripts/bt*/

# Check the transpiler for the root cause
grep -n "PATTERN" tools/transpiler/generators.py tools/transpiler/extractors.py
```

If the issue is systemic, fix the transpiler — never hand-edit individual scripts.

---

## Common Review Findings

### Acceptable Differences
- Descriptive-tagged effects (cost reduction, redirect attack, effect immunity) — these are known limitations
- Missing trait/name filters in target selection — the transpiler extracts what it can; perfect filtering is ongoing work
- `target_filter` returning `p.is_digimon` when card text specifies additional constraints — filter enhancement is tracked

### Issues That Need Fixing
- Wrong timing (e.g., `is_on_play` when card says `[When Digivolving]`)
- Missing keywords (e.g., card grants Blocker but script doesn't have `_is_blocker`)
- Wrong selection direction (e.g., `effect_select_own_permanent` when card targets opponent)
- `pass`-only callbacks (no-action stubs) — investigate if transpiler can handle the pattern
- Incorrect condition logic (e.g., missing owner turn check for `[Your Turn]` effects)
