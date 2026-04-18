# Running Learnings from Mastemon Batch Fixes (Batches 1–3)

## Environment Constraints (CRITICAL)

- **DCGO submodule in agent worktree is EMPTY.** C# files must be read via absolute main-tree path:
  `C:/Users/james/Documents/digimon-deck-list-builder-1/DCGO/Assets/Scripts/CardEffect/{SET}/{COLOR}/{CARD}.cs`
- **Untracked files (like `qa/archetype-qa/mastemon-tribal/...` meta JSONs) do NOT exist in the worktree.** Card text will be provided inline in your prompt.
- `digimon_gym/engine/data/cards.json` IS tracked and available in the worktree — you can cross-reference it if needed.
- On Windows, always prefix Python/pytest commands with `PYTHONIOENCODING=utf-8`.

## Bugs Found So Far (Common Patterns — Watch For These)

### 1. Wrong alt-digi restrictions
- Transpiler emits `_alt_digi_color = CardColor.X` spuriously. C# alt-digi closures often only check `IsLevelN && HasXTraits` with no color restriction.
- Always compare against the C# `PermanentCondition` closure exactly. Add `_alt_digi_trait = "CS"` (or similar) when card text says `[CS]` trait.

### 2. Exact-name matching for EqualsCardName
- `contains_card_name()` does substring matching → wrongly matches "LadyDevimon (X Antibody)" for "LadyDevimon".
- When C# uses `EqualsCardName`, use exact list membership: `"LadyDevimon" in perm.top_card.card_names`.
- Also gate on kind: `perm.is_digimon` / `perm.is_tamer` to avoid cross-kind matches.

### 3. Inherited trait gates must check TOP CARD only
- `for cs in perm.card_sources: ...` scans the whole stack → wrongly grants auras when a below-top card matches.
- Use `perm.top_card.card_traits` only. C# uses `TopCard.CardTraits.Contains(...)`.

### 4. value_fn arity bug in register_modifier
- SILENT FAILURE: `value_fn=lambda: -6000` (0-arg) raises TypeError caught by bare `except Exception: pass` in `ModifierRegistry.get_int_modifier`. DP/SA modifier never applies.
- Correct: `value_fn=lambda cur, t, c: cur - 6000`.
- Same applies to `CHANGE_SECURITY_ATTACK` (listed in engine-gaps.md fix 24).

### 5. OnLoseSecurity / similar events need owner gate
- Without `event_player is player` check in condition, your effect fires when OPPONENT loses security too.
- C# uses `CanTriggerWhenLoseSecurity(hashtable, player => player == card.Owner)`.

### 6. "If you do" gates and nested selections
- If clause 2 depends on clause 1 succeeding, the clause 2 logic must run INSIDE clause 1's success callback, not synchronously after scheduling the selection.
- Nested pattern: `effect_select_hand_card(..., on_trashed=lambda: game.request_selection(GamePhase.SelectTrash, ...))`.

### 7. No auto-selection — preserve RL agent choice
- Never use `min(...)`, `[0]`, `sorted(...)[0]` to pick targets. Every "1 of your" / "any 1" must go through a selection phase exposing ALL valid indices.
- Top/Bottom security choice: `game.effect_choose_branch(2, callback, branch_labels=["Top of security", "Bottom of security"])`.

### 8. CardSource has `base_dp`, NOT `dp`
- `.dp` is only on `Permanent`. Filtering cards in hand/trash by DP → `getattr(c, 'base_dp', None)`.

### 9. put_permanent_to_security ownership
- `player.put_permanent_to_security(target_perm)` silently no-ops for opponent permanents.
- Resolve `target_owner = target_perm.owner` and call `target_owner.put_permanent_to_security(...)`.

### 10. Raw security_cards list ops miss events
- `security_cards.pop(0)` / `append(...)` bypass `OnDiscardSecurity` / `OnLoseSecurity`.
- Use `player.trash_security_card()` / `player.add_security_card()` / `effect_choose_deck_placement`.

### 11. Recovery / rider step gated inside selection callback
- "Then, <do Y>" often needs to fire unconditionally AFTER the selection, not only if the selection picked a target.
- Extract Y into a helper and call it after scheduling the selection (or in the selection's fallthrough path).

### 12. No stubs allowed
- `pass` is not an implementation. Every effect must have a real process callback.
- "Descriptive-tagged" comments don't count as implementation.

### 13. has_keyword self-inherit path ignores _keyword_permanent_condition
- Known engine behavior: aura path checks the filter, self-inherit path does not.
- Workaround: gate via `can_use_condition` closure using `context.get('permanent').top_card` trait.

### 14. Dead-code transpiler bloat
- Transpiler emits a hidden `NoTiming` ChangeCostClass duplicate alongside every `BeforePayCost` effect — inert in Python engine. Remove for clarity.

## Reminders

- Run the 16-item checklist from the prompt against every card.
- Write tests BEFORE fixing. Tests encode what the card SHOULD do.
- One revision round maximum. If stuck, report PARTIAL/BLOCKED rather than guess.
