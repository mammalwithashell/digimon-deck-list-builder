# Plan: Security Check UX + Effect Visibility (Phases 2–3)

## Current State

- `SecurityRevealOverlay` exists: shows revealed card for 2s with flip animation, click to dismiss
- `useEffectHighlight` hook exists: highlights source permanent with golden pulse for 1.5s on `effect_activate` events
- Backend emits `security_reveal`, `security_battle`, `keyword_trigger`, `effect_activate` events
- **Gap:** Security checks run atomically — all checks complete in one step, overlay only shows the LAST reveal. No DP comparison, no battle result, no effect text on the revealed card.

## Phase 2: Security Check UX

### Step 1: Enrich `security_reveal` event with card data

**File:** `code/engine_py_legacy/engine/core/player.py` (`security_attack` method, ~line 502)

Add to the `security_reveal` event meta:
- `main_effect_text`: the card's main effect text (for display)
- `inherited_effect_text`: inherited effect text
- `base_dp`: if Digimon, include base DP for comparison display
- `card_type`: "Digimon" / "Option" / "Tamer" (already have `is_digimon`)

Also enrich `security_battle` event meta with:
- `attacker_name`: for the DP overlay display
- `defender_name`: security card name
- `jamming`: whether Jamming prevented deletion

### Step 2: Queue-based security reveal on frontend

**File:** `code/frontend/src/components/board/SecurityRevealOverlay.tsx`

Currently shows only the latest event. Refactor to:
- Queue ALL security_reveal events (multiple checks per attack with SA+1, etc.)
- Show them one at a time, each with its own dismiss timer
- After reveal, if a `security_battle` event follows (same seq range), show DP comparison overlay before moving to next reveal
- Flow per check: reveal card (0.5s flip) → show card + effect text (hold) → if battle: show DP comparison result → dismiss → next check

### Step 3: DP comparison overlay

**File:** `code/frontend/src/components/board/SecurityRevealOverlay.tsx` (extend)

After showing the security card, if it's a Digimon, transition to a DP comparison view:
- Left: attacker card + DP value
- Right: security Digimon + DP value
- Result banner: "Attacker Wins!" / "Security Digimon Wins!" / "Jamming!"
- Color-coded (green for winner, red for loser)
- Auto-dismiss after 1.5s or click

### Step 4: Security break animation

**File:** `code/frontend/src/index.css`

Add CSS animation for when security is broken:
- Glass shatter / crack effect on the security stack (CSS-only, inspired by DCGO)
- Trigger via a new CSS class applied to `SecurityStack` component when `security_remaining` decreases
- Track previous count in `SecurityStack.tsx` to detect decrease

### Step 5: Security effect text display

**File:** `code/frontend/src/components/board/SecurityRevealOverlay.tsx`

When showing the revealed card:
- Display the card's security effect text below the card (e.g., "[Security] Play this card without paying the cost")
- Highlight "Security Effect" text in amber
- If the card has no security effect, show "No security effect" in gray

## Phase 3: Effect Activation Visibility

### Step 6: Add `active_effect` data to game state

**File:** `code/engine_py_legacy/engine/game.py` (in `execute_effects` / effect processing)

Enrich `effect_activate` events with:
- `effect_text`: the effect's description text
- `effect_name`: short name
- `source_card_name`: which card contributed this effect
- `is_inherited`: whether this is an inherited effect
- `inherited_from_index`: which card in the stack this effect comes from

### Step 7: Effect text popup component

**File:** `code/frontend/src/components/game/EffectPopup.tsx` (new)

- Subscribe to `effect_activate` events
- Show floating popup near the source permanent with:
  - Effect source card name (if inherited, show "Inherited from [CardName]")
  - Effect text (truncated if too long)
- Animate in (slide-up + fade), hold 1.5s, animate out
- Stack multiple simultaneous effects vertically

### Step 8: Sequential effect delay

**File:** `code/frontend/src/components/board/SecurityRevealOverlay.tsx` and `EffectPopup.tsx`

- When multiple effects resolve in sequence, add 300ms delay between showing each popup
- Use a queue system similar to the security reveal queue from Step 2

## Files Modified

### Backend (3 files):
1. `code/engine_py_legacy/engine/core/player.py` — enrich security events (Step 1)
2. `code/engine_py_legacy/engine/game.py` — enrich effect_activate events (Step 6)
3. `code/engine_py_legacy/engine/events.py` — no changes needed (meta dict is flexible)

### Frontend (5 files):
1. `code/frontend/src/components/board/SecurityRevealOverlay.tsx` — queue + DP overlay + effect text (Steps 2, 3, 5)
2. `code/frontend/src/components/board/SecurityStack.tsx` — security break animation trigger (Step 4)
3. `code/frontend/src/index.css` — new animations (Step 4)
4. `code/frontend/src/components/game/EffectPopup.tsx` — NEW: effect text popup (Step 7)
5. `code/frontend/src/pages/GamePage.tsx` — mount EffectPopup component (Step 7)

### Tests:
- Update `tests/test_security_flow.py` — verify enriched event metadata

## Implementation Order

1. Step 1 (backend: enrich events) — foundation for everything else
2. Step 2 (frontend: queue reveals) — fixes multi-check display bug
3. Step 3 (frontend: DP overlay) — highest visual impact
4. Step 4 (frontend: break animation) — polish
5. Step 5 (frontend: effect text on reveal) — information parity with DCGO
6. Step 6 (backend: enrich effect events) — foundation for Phase 3
7. Step 7 (frontend: effect popup) — Phase 3 core
8. Step 8 (frontend: sequential delays) — Phase 3 polish
